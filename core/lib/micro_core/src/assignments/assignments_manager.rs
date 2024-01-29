use std::time::Duration;

use async_trait::async_trait;
use micro_contracts::{sys_assignment_contract, sys_deposit_contract};
use micro_dal::ConnectionPool;
use micro_prover_utils::periodic_job::PeriodicJob;
use micro_system_constants::{ASSIGNMENT_ADDRESS, DEPOSIT_ADDRESS};
use micro_types::{
    api::GetLogsFilter,
    ethabi::{Contract, Token},
    l2::{
        assignment_batch::{AssignmentBatch, ASSIGNMENT_BATCH},
        event_map::EventMapBuilder,
        penalize::{Penalize, PENALIZE},
    },
    transaction_request::CallRequest,
    Bytes, L1BatchNumber, MiniblockNumber, H256, U256,
};

use crate::{api_server::web3::backend_jsonrpc::error::internal_error, l2_sender::caller::Caller};

#[derive(Debug)]
pub struct AssignmentsManager {
    pool: ConnectionPool,
    processing_timeout: Duration,
    retry_interval_ms: u64,
    l2_sender: Caller,
    deposit_abi: Contract,
    event_signatures: Vec<H256>,
    from_block: u32,
}

impl AssignmentsManager {
    pub fn new(
        processing_timeout: Duration,
        retry_interval_ms: u64,
        pool: ConnectionPool,
        l2_sender: Caller,
    ) -> Self {
        let deposit_abi = sys_deposit_contract();
        let contract_abi = sys_assignment_contract();

        let mut event_signatures = Vec::new();
        let assignment_signature = contract_abi
            .event(ASSIGNMENT_BATCH)
            .expect("missing ASSIGNMENT_BATCH abi")
            .signature();
        let penalize_signature = deposit_abi
            .event(PENALIZE)
            .expect("missing PENALIZE abi")
            .signature();
        event_signatures.push(assignment_signature);
        event_signatures.push(penalize_signature);
        Self {
            retry_interval_ms,
            processing_timeout,
            pool,
            l2_sender,
            deposit_abi,
            event_signatures,
            from_block: 0_u32,
        }
    }

    async fn time_out_check(&mut self) {
        let mut connection = self.pool.access_storage().await.unwrap();
        //TODO error
        let _ = connection
            .assignments_dal()
            .update_assigments_status_for_time(self.processing_timeout)
            .await;
    }
    async fn send_be_punished_tx(&mut self) {
        let mut connection = self.pool.access_storage().await.unwrap();
        let mut res = connection
            .assignments_dal()
            .get_punished_address_list()
            .await;

        res.reverse();

        for (_id, address, l1_batch_number, event_index) in res {
            let abi_data = self
                .deposit_abi
                .function("penalize")
                .unwrap()
                .encode_input(&[
                    Token::Uint(U256::from(l1_batch_number.0)),
                    Token::Address(address.clone()),
                    Token::Uint(U256::from(event_index)),
                ])
                .unwrap();

            let data = CallRequest {
                to: Some(DEPOSIT_ADDRESS),
                data: Some(Bytes(abi_data)),
                ..Default::default()
            };
            match self.l2_sender.send(data).await {
                Ok(Ok(hash)) => {
                    tracing::warn!(
                        "send_be_punished_tx send address: {:?},l1_batch_number:{:?},hash:{}",
                        address.clone(),
                        l1_batch_number.clone(),
                        hash
                    );
                }
                e => {
                    tracing::error!(
                        "send_be_punished_tx is fail address: {:?}, error {:?}",
                        &address,
                        e
                    );
                }
            };
        }
    }
    async fn monitor_change_event(&mut self) {
        const METHOD_NAME: &str = "monitor_change_event";
        let mut connection = self.pool.access_storage().await.unwrap();
        if self.from_block == 0 {
            self.from_block = connection
                .assignments_dal()
                .get_max_mini_number()
                .await
                .unwrap_or(MiniblockNumber(0))
                .0;
        }

        let sealed_mini_number = connection
            .blocks_web3_dal()
            .get_sealed_miniblock_number()
            .await
            .unwrap();

        let mut to_block = self.from_block + 1024;
        if to_block > sealed_mini_number.0 {
            to_block = sealed_mini_number.0;
        }

        tracing::warn!(
            "monitor_change_event from: {}, to: {}",
            self.from_block,
            to_block
        );

        let topic = (1, self.event_signatures.clone());
        let filter = GetLogsFilter {
            from_block: MiniblockNumber(self.from_block),
            to_block: MiniblockNumber(to_block),
            addresses: vec![DEPOSIT_ADDRESS, ASSIGNMENT_ADDRESS],
            topics: vec![topic],
        };
        tracing::warn!("monitor_change_event filter: {:?}", &filter);
        let logs: Vec<micro_types::api::Log> = connection
            .events_web3_dal()
            .get_logs(filter, i32::MAX as usize)
            .await
            .map_err(|err| internal_error(METHOD_NAME, err))
            .unwrap();

        self.from_block = to_block;

        if logs.is_empty() {
            tracing::warn!("monitor_change_event logs is null");
            return;
        }

        let at_map = AssignmentBatch::build_map(logs.clone());
        let penalize_map = Penalize::build_map(logs.clone());
        tracing::warn!("monitor_change_event at_map:{:?}", at_map);
        tracing::warn!("monitor_change_event penalize_map:{:?}", penalize_map);
        if !&at_map.is_empty() {
            for assignment in at_map {
                let res = connection
                    .assignments_dal()
                    .insert_and_update_assignments(
                        assignment.prover,
                        L1BatchNumber(assignment.batch_number.as_u32()),
                        MiniblockNumber(assignment.mini_block_number),
                        assignment.storage_index.as_u64(),
                    )
                    .await;
                match res {
                    Ok(_) => {
                        continue;
                    }
                    Err(e) => {
                        tracing::error!("monitor_change_event at_map address:{:?}, batch_number:{:?},mini_block_number:{:?},storage_index:{:?}, e:{:?}",
                    assignment.prover,assignment.batch_number,assignment.mini_block_number,assignment.storage_index, e)
                    }
                }
            }
        }

        if !&penalize_map.is_empty() {
            for assignment in penalize_map {
                let hash = match assignment.transaction_hash {
                    Some(value) => value,
                    None => H256::default(),
                };
                if hash == H256::default() {
                    tracing::error!("monitor_change_event penalize_map address:{:?}, batch_number:{:?},mini_block_number:{:?},storage_index:{:?}, hash is error",
                    assignment.prover,assignment.batch_number,assignment.mini_block_number,assignment.storage_index);
                    continue;
                }

                let res = connection
                    .assignments_dal()
                    .update_assigments_status_by_punished(
                        assignment.storage_index.as_u64(),
                        assignment.prover,
                        L1BatchNumber(assignment.batch_number.as_u32()),
                        hash,
                    )
                    .await;
                match res {
                    Ok(_) => {
                        continue;
                    }
                    Err(e) => {
                        tracing::error!("monitor_change_event penalize_map address:{:?}, batch_number:{:?},mini_block_number:{:?},storage_index:{:?}, e:{:?}",
                    assignment.prover,assignment.batch_number,assignment.mini_block_number,assignment.storage_index, e)
                    }
                }
            }
        }
    }
}
// /// Prove task assigned to verification node periodically.
#[async_trait]
impl PeriodicJob for AssignmentsManager {
    const SERVICE_NAME: &'static str = "AssignmentsManager";
    async fn run_routine_task(&mut self) -> anyhow::Result<()> {
        self.monitor_change_event().await;
        self.time_out_check().await;
        self.send_be_punished_tx().await;
        Ok(())
    }
    fn polling_interval_ms(&self) -> u64 {
        self.retry_interval_ms
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_group_logs() {}
}
