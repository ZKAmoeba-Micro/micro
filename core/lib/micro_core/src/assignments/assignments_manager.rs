use crate::{api_server::web3::backend_jsonrpc::error::internal_error, l2_sender::caller::Caller};
use async_trait::async_trait;
use micro_config::constants::DEPOSIT_ADDRESS;
use micro_contracts::sys_deposit_contract;
use micro_dal::ConnectionPool;
use micro_prover_utils::periodic_job::PeriodicJob;
use micro_types::l2::event_map::EventMapBuilder;
use micro_types::{
    api::{BlockNumber, GetEventLogsFilter, GetLogsFilter},
    ethabi::{Contract, Token},
    l2::{
        assignment_batch::{AssignmentBatch, EVENTNAME},
        penalize::{Penalize, PENALIZE},
    },
    transaction_request::CallRequest,
    Bytes, L1BatchNumber, MiniblockNumber, H256, U256,
};
use std::time::Duration;
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
        let contract_abi = sys_deposit_contract();
        let events = vec![EVENTNAME, PENALIZE];
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
            l2_sender,
            deposit_abi: contract_abi,
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
        let res = connection
            .assignments_dal()
            .get_punished_address_list()
            .await;
        for (_id, address, l1_batch_number, event_index) in res {
            let abi_data = self
                .deposit_abi
                .function("penalize")
                .unwrap()
                .encode_input(&[
                    Token::Address(address.clone()),
                    Token::Uint(U256::from(l1_batch_number.0)),
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
        let latest_mini_block_number = connection
            .assignments_dal()
            .get_max_mini_number()
            .await
            .unwrap_or(MiniblockNumber(self.from_block));
        let sealed_mini_number = connection
            .blocks_web3_dal()
            .get_sealed_miniblock_number()
            .await
            .unwrap();

        tracing::warn!(
            "monitor_change_event get_next_number: {:?},from:{}",
            &latest_mini_block_number,
            self.from_block
        );
        if latest_mini_block_number.0 == 0 {
            self.from_block = self.from_block;
        } else if latest_mini_block_number.0 != 0 && latest_mini_block_number.0 > self.from_block {
            self.from_block = latest_mini_block_number.0;
        } else if self.from_block > sealed_mini_number.0 {
            self.from_block = sealed_mini_number.0;
        } else {
            self.from_block = self.from_block + 1;
        }
        let next_number = (self.from_block as i32) + 1024;
        //Multiple parameter lists for one event
        let mut event_names = Vec::new();
        //Verification address parameter of the event
        for signal in &self.event_signatures {
            let h256 = vec![*signal];
            //let topic = (1, h256);
            event_names.push(h256);
        }
        let filter = GetLogsFilter {
            from_block: MiniblockNumber(self.from_block),
            to_block: Some(BlockNumber::Number(next_number.into())),
            addresses: vec![DEPOSIT_ADDRESS],
            topics: vec![],
        };
        let filter = GetEventLogsFilter {
            log_filter: filter,
            event_names: event_names,
        };
        tracing::warn!("monitor_change_event filter: {:?}", &filter);
        let logs: Vec<micro_types::api::Log> = connection
            .events_web3_dal()
            .get_logs(filter, i32::MAX as usize)
            .await
            .map_err(|err| internal_error(METHOD_NAME, err))
            .unwrap();

        if logs.is_empty() {
            tracing::warn!("monitor_change_event logs is null");
            return;
        }

        let at_map = AssignmentBatch::build_map(logs.clone());
        let penalize_map = Penalize::build_map(logs.clone());
        tracing::warn!("monitor_change_event at_map:{:?}", at_map);
        tracing::warn!("monitor_change_event penalize_map:{:?}", penalize_map);
        if !&at_map.is_empty() {
            let assignments_list: Vec<AssignmentBatch> = at_map.values().cloned().collect();
            for assignment in assignments_list {
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
            let assignments_list: Vec<Penalize> = penalize_map.values().cloned().collect();
            for assignment in assignments_list {
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
