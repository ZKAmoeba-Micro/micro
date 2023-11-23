use crate::{api_server::web3::backend_jsonrpc::error::internal_error, l2_sender::caller::Caller};
use async_trait::async_trait;
use chrono::Utc;
use micro_config::constants::DEPOSIT_ADDRESS;
use micro_contracts::sys_deposit_contract;
use micro_dal::{assignments_dal::ProverResultStatus, ConnectionPool};
use micro_prover_utils::periodic_job::PeriodicJob;
use micro_types::{
    api::{BlockNumber, GetLogsFilter},
    assignment_user_summary::AssignmentUserSummaryInfo,
    ethabi::{Contract, Token},
    l2::score_update::{ScoreUpdate, Status},
    transaction_request::CallRequest,
    Bytes, MiniblockNumber, H256,
};
use std::{collections::HashMap, convert::TryFrom, ops::Add, time::Duration};
#[derive(Debug)]

pub struct AssignmentsManager {
    pool: ConnectionPool,
    processing_timeout: Duration,
    retry_interval_ms: u64,
    //The number of points for participating once
    once_score: u32,
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
        once_score: u32,
        l2_sender: Caller,
    ) -> Self {
        let contract_abi = sys_deposit_contract();
        let events = vec!["ScoreUpdate"];
        let mut event_signatures = Vec::new();
        for name in events {
            let msg = format!("{} event is missing in abi", name);
            let signature = contract_abi.event(name).expect(msg.as_str()).signature();
            event_signatures.push(signature);
        }
        Self {
            retry_interval_ms,
            processing_timeout,
            pool,
            once_score,
            l2_sender,
            deposit_abi: contract_abi,
            event_signatures,
            from_block: 0_u32,
        }
    }

    async fn assign_proof_tasks(&mut self) {
        let mut connection = self.pool.access_storage().await.unwrap();
        let mut user_list = connection
            .assignment_user_summary_dal()
            .select_normal_user_list(Status::Normal)
            .await;
        let user_count = user_list.len();
        if user_count > 0 {
            tracing::info!("assign_proof_tasks list size: {:?}", user_count);
            let batch_numbers = connection
                .proof_generation_dal()
                .get_batch_number_list()
                .await;
            tracing::info!("assign_proof_tasks list: {:?}", user_list);
            tracing::info!("assign_proof_tasks batch_numbers: {:?}", batch_numbers);
            let now: i64 = Utc::now().timestamp_millis() / 1000;

            for user in &mut user_list {
                let sub_value = now - user.last_time;
                if let Some(dyn_score) = sub_value.checked_mul(self.once_score.into()) {
                    user.base_score = user.base_score.add(dyn_score as u16);
                }
            }
            user_list.sort_by(|a, b| b.base_score.cmp(&a.base_score));
            tracing::info!("assign_proof_tasks user_list: {:?}", user_list);

            for (index, batch_number) in batch_numbers.iter().enumerate() {
                let top_user = user_list.get(index % user_count).unwrap();

                let address = top_user.verification_address;
                let status = connection
                    .assignments_dal()
                    .get_last_status(address.clone(), batch_number.clone())
                    .await;

                match status {
                    Ok(s) => {
                        if s.to_string() != ProverResultStatus::Failed.to_string() {
                            continue;
                        }
                    }
                    Err(e) => tracing::error!("assign_proof_tasks status e:{:?}", e),
                }
                let res = connection
                    .assignments_dal()
                    .insert_and_update_assignments(address.clone(), batch_number.clone(), now)
                    .await;
                match res {
                    Err(r) => tracing::error!("assign_proof_tasks status res:{:?}", r),
                    _ => {}
                }
                let abi_data = self
                    .deposit_abi
                    .function("assignment")
                    .unwrap()
                    .encode_input(&[Token::Address(address.clone())])
                    .unwrap();
                let data = CallRequest {
                    to: Some(DEPOSIT_ADDRESS),
                    data: Some(Bytes(abi_data)),
                    ..Default::default()
                };

                match self.l2_sender.send(data).await {
                    Ok(tx_hash) => {
                        tracing::info!("assign_proof_tasks tx is success address: {:?},block_number:{:?},tx_hash:{:?}", &address,&batch_number,tx_hash);
                    }
                    Err(r) => {
                        //TODO
                        tracing::error!(
                            "assign_proof_tasks tx is fail address: {:?},block_number:{:?},err:{}",
                            &address,
                            &batch_number,
                            r
                        );
                    }
                };
            }
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
            match self.l2_sender.send(data).await {
                Ok(Ok(hash)) => {
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
        let latest_mini_block_number = connection
            .assignment_user_summary_dal()
            .get_max_miniblock_number()
            .await
            .unwrap_or(MiniblockNumber(self.from_block));
        tracing::warn!(
            "monitor_change_event latest_mini_block_number: {:?},from:{}",
            &latest_mini_block_number,
            self.from_block
        );
        if latest_mini_block_number.0 == 0 {
            self.from_block = self.from_block;
        } else if latest_mini_block_number.0 != 0 && latest_mini_block_number.0 > self.from_block {
            self.from_block = latest_mini_block_number.0;
        } else {
            self.from_block = self.from_block + 1;
        }
        // if self.from_block==0 || (latest_mini_block_number.0!=0 && latest_mini_block_number.0 >self.from_block){
        //     self.from_block=latest_mini_block_number.0;
        // }
        // if latest_mini_block_number.0 == 0 {
        //     latest_mini_block_number = connection
        //         .blocks_web3_dal()
        //         .get_sealed_miniblock_number()
        //         .await
        //         .unwrap();
        // }

        let next_number = (self.from_block as i32) + 99;
        //Multiple parameter lists for one event
        let mut topics = Vec::new();
        //Verification address parameter of the event
        for signal in &self.event_signatures {
            let h256 = vec![*signal];
            let topic = (1, h256);
            topics.push(topic);
        }

        let filter = GetLogsFilter {
            from_block: MiniblockNumber(self.from_block),
            to_block: Some(BlockNumber::Number(next_number.into())),
            addresses: vec![DEPOSIT_ADDRESS],
            topics: topics,
        };
        tracing::warn!("monitor_change_event filter: {:?}", &filter);
        let logs = connection
            .events_web3_dal()
            .get_logs(filter, i32::MAX as usize)
            .await
            .map_err(|err| internal_error(METHOD_NAME, err))
            .unwrap();

        if logs.is_empty() {
            tracing::warn!("monitor_change_event logs is null");
            return;
        }
        let mut scores_map = HashMap::new();
        for log in logs {
            let score_update = ScoreUpdate::try_from(log).unwrap();
            scores_map.insert(score_update.prover, score_update);
        }
        tracing::warn!("monitor_change_event scores_map:{:?}", scores_map);

        let scores: Vec<ScoreUpdate> = scores_map.values().cloned().collect();
        for score in scores {
            let status = score.status;
            if status == Status::Unknow {
                continue;
            }
            let param_info = AssignmentUserSummaryInfo::new(
                score.prover,
                score.base_score.as_u32() as u16,
                score.latest_assignment_time.as_u64() as i64,
            );
            let _ = connection
                .assignment_user_summary_dal()
                .add_or_update_assignment_user_summary_info(
                    param_info,
                    status,
                    MiniblockNumber(score.mini_block_number),
                )
                .await;
        }

        // for log in logs {
        //     let address = h256_to_account_address(&log.topics[1]);
        //     let status = h256_to_u32(log.topics[2]);
        //     let base_score = h256_to_u32(log.topics[3]) as u16;
        //     let batch_number = L1BatchNumber(h256_to_u32(H256::from_slice(&log.data.0)));

        //     let user_status = match status {
        //         0 => UserStatus::UnDeposit,
        //         1 => UserStatus::Normal,
        //         3 => UserStatus::Applying,
        //         _ => UserStatus::Frozon,
        //     };
        //     let block_number = log.block_number.unwrap();
        //     let param_info = AssignmentUserSummaryInfo::new(address, base_score, batch_number);
        //     let _ = connection
        //         .assignment_user_summary_dal()
        //         .add_or_update_assignment_user_summary_info(
        //             param_info,
        //             user_status,
        //             MiniblockNumber(U64::as_u32(&block_number)),
        //         )
        //         .await;
        // }
    }
}
// /// Prove task assigned to verification node periodically.
#[async_trait]
impl PeriodicJob for AssignmentsManager {
    const SERVICE_NAME: &'static str = "AssignmentsManager";
    async fn run_routine_task(&mut self) -> anyhow::Result<()> {
        self.assign_proof_tasks().await;
        self.time_out_check().await;
        self.send_be_punished_tx().await;
        self.monitor_change_event().await;
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
