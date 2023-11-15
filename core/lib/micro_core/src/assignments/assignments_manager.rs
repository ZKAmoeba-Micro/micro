use crate::l1_gas_price::L1GasPriceProvider;
use crate::l2_sender::L2Sender;
use async_trait::async_trait;
use micro_config::constants::DEPOSIT_ADDRESS;
use micro_contracts::sys_deposit_contract;
use micro_dal::ConnectionPool;
use micro_prover_utils::periodic_job::PeriodicJob;
use micro_types::{
    assignment_user_summary::UserStatus,
    ethabi::{Contract, Token},
    transaction_request::CallRequest,
    Bytes, L1BatchNumber, H256, U256,
};
use std::ops::Add;
use std::time::Duration;

#[derive(Debug)]
pub struct AssignmentsManager<G: L1GasPriceProvider> {
    pool: ConnectionPool,
    processing_timeout: Duration,
    retry_interval_ms: u64,
    l2_sender: L2Sender<G>,
    deposit_abi: Contract,
}

impl<G: L1GasPriceProvider> AssignmentsManager<G> {
    pub fn new(
        processing_timeout: Duration,
        retry_interval_ms: u64,
        pool: ConnectionPool,
        l2_sender: L2Sender<G>,
    ) -> Self {
        Self {
            retry_interval_ms,
            processing_timeout,
            pool,
            l2_sender,
            deposit_abi: sys_deposit_contract(),
        }
    }

    pub async fn assign_proof_tasks(&mut self, block_number: L1BatchNumber, score: u16) {
        let mut connection = self.pool.access_storage().await.unwrap();
        let mut user_list = connection
            .assignment_user_summary_dal()
            .select_normal_user_list(UserStatus::Normal)
            .await;
        let user_count = user_list.len();
        tracing::info!("assign_proof_tasks list size: {:?}", user_count);
        if user_count > 0 {
            for user in &mut user_list {
                if let Some(sub_value) = &block_number.0.checked_sub(user.last_batch_number.0) {
                    if let Some(dyn_score) = sub_value.checked_mul(score as u32) {
                        user.base_score = user.base_score.add(dyn_score as u16);
                    }
                }
            }
            user_list.sort_by(|a, b| a.base_score.cmp(&b.base_score));
            tracing::info!("assign_proof_tasks list: {:?}", user_list);
            if let Some(top_user) = user_list.first() {
                let address = top_user.verification_address;
                //TODO error
                let _ = connection
                    .assignments_dal()
                    .insert_assignments(address.clone(), block_number.clone())
                    .await;

                let abi_data = self
                    .deposit_abi
                    .function("assignment")
                    .unwrap()
                    .encode_input(&[
                        Token::Address(address.clone()),
                        Token::Uint(U256::from(block_number.0)),
                    ])
                    .unwrap();
                let data = CallRequest {
                    to: Some(DEPOSIT_ADDRESS),
                    data: Some(Bytes(abi_data)),
                    ..Default::default()
                };
                match self.l2_sender.get_caller().send(data).await {
                    Ok(tx_hash) => {
                        tracing::info!("assign_proof_tasks tx is success address: {:?},block_number:{:?},tx_hash:{:?}", &address,&block_number,tx_hash);
                    }
                    _ => {
                        tracing::info!(
                            "assign_proof_tasks tx is fail address: {:?},block_number:{:?}",
                            &address,
                            &block_number
                        );
                    }
                };
            }
        }
    }

    pub async fn time_out_check(&mut self) {
        let mut connection = self.pool.access_storage().await.unwrap();
        //TODO error
        let _ = connection
            .assignments_dal()
            .update_assigments_status_for_time(self.processing_timeout)
            .await;
    }

    pub async fn send_be_punished_tx(&mut self) {
        let mut connection = self.pool.access_storage().await.unwrap();
        let res = connection
            .assignments_dal()
            .get_punished_address_list()
            .await;
        for (address, l1_batch_number) in res {
            let abi_data = self
                .deposit_abi
                .function("penalize")
                .unwrap()
                .encode_input(&[Token::Address(address.clone())])
                .unwrap();

            let data = CallRequest {
                to: Some(DEPOSIT_ADDRESS),
                data: Some(Bytes(abi_data)),
                ..Default::default()
            };
            match self.l2_sender.get_caller().send(data).await {
                Ok(tx_hash) => {
                    let hash = H256::from_slice(&tx_hash);
                    //todo error
                    let _ = connection
                        .assignments_dal()
                        .update_assigments_status_by_punished(
                            address.clone(),
                            l1_batch_number,
                            hash,
                        )
                        .await;
                }
                _ => {
                    tracing::info!("send_be_punished_tx is fail address: {:?}", &address);
                }
            };
        }
    }

    pub fn monitor_change_event() {}
}
// /// Prove task assigned to verification node periodically.
// #[async_trait]
// impl <G: L1GasPriceProvider>  PeriodicJob for AssignmentsManager<G>{
//     const SERVICE_NAME: &'static str = "AssignmentsManager";

//     async fn run_routine_task(&mut self) -> anyhow::Result<()> {
//         Ok(())
//     }

//     fn polling_interval_ms(&self) -> u64 {
//         self.retry_interval_ms
//     }
// }
