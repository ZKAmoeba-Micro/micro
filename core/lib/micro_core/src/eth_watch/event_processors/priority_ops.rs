use std::{collections::HashMap, convert::TryFrom};

use itertools::Itertools;
use micro_contracts::micro_contract;
use micro_dal::StorageProcessor;
use micro_types::{
    l1::{L1FactoryDep, L1Tx},
    web3::types::Log,
    PriorityOpId, H256,
};

use crate::{
    eth_watch::{
        client::{Error, EthClient},
        event_processors::EventProcessor,
        metrics::{PollStage, METRICS},
    },
    metrics::{TxStage, APP_METRICS},
};

/// Responsible for saving new priority L1 transactions to the database.
#[derive(Debug)]
pub struct PriorityOpsEventProcessor {
    next_expected_priority_id: PriorityOpId,
    new_priority_request_signature: H256,
    new_priority_request_factory_deps_signature: H256,
}

impl PriorityOpsEventProcessor {
    pub fn new(next_expected_priority_id: PriorityOpId) -> Self {
        Self {
            next_expected_priority_id,
            new_priority_request_signature: micro_contract()
                .event("NewPriorityRequest")
                .expect("NewPriorityRequest event is missing in abi")
                .signature(),
            new_priority_request_factory_deps_signature: micro_contract()
                .event("NewPriorityRequestFactoryDeps")
                .expect("NewPriorityRequestFactoryDeps event is missing in abi")
                .signature(),
        }
    }
}

#[async_trait::async_trait]
impl<W: EthClient + Sync> EventProcessor<W> for PriorityOpsEventProcessor {
    async fn process_events(
        &mut self,
        storage: &mut StorageProcessor<'_>,
        _client: &W,
        events: Vec<Log>,
    ) -> Result<(), Error> {
        let mut priority_ops = Vec::new();
        let mut splitted_deps: Vec<L1FactoryDep> = Vec::new();
        for event in events {
            if event.topics[0] == self.new_priority_request_signature {
                let tx =
                    L1Tx::try_from(event).map_err(|err| Error::LogParse(format!("{}", err)))?;
                priority_ops.push(tx);
            } else if event.topics[0] == self.new_priority_request_factory_deps_signature {
                let dep = L1FactoryDep::try_from(event)
                    .map_err(|err| Error::LogParse(format!("{}", err)))?;
                splitted_deps.push(dep);
            }
        }

        let mut deps_result: HashMap<micro_types::PriorityOpId, Vec<Vec<u8>>> = HashMap::new();

        let mapped_deps = splitted_deps.into_iter().into_group_map_by(|d| d.serial_id);
        for (serial_id, list) in mapped_deps {
            let mapped_dep = list.into_iter().into_group_map_by(|d| d.dep_index);

            let mut temp_deps: Vec<Vec<u8>> = vec![];
            temp_deps.resize(mapped_dep.len(), vec![]);

            for (dep_index, list) in mapped_dep {
                let mut temp_dep = vec![];

                let deps: Vec<L1FactoryDep> = list
                    .into_iter()
                    .unique_by(|x| x.splitted_index)
                    .sorted_by_key(|x| x.splitted_index)
                    .collect();

                for mut d in deps {
                    temp_dep.append(&mut d.bytes);
                }
                temp_deps[dep_index as usize] = temp_dep;
            }

            deps_result.insert(serial_id, temp_deps);
        }

        let priority_ops: Vec<L1Tx> = priority_ops
            .into_iter()
            .unique_by(|x| x.serial_id())
            .update(|x| {
                x.execute.factory_deps = match deps_result.remove(&x.serial_id()) {
                    Some(d) => Some(d),
                    None => Some(vec![]),
                }
            })
            .sorted_by_key(|event| event.serial_id())
            .collect();

        if priority_ops.is_empty() {
            return Ok(());
        }

        let first = &priority_ops[0];
        let last = &priority_ops[priority_ops.len() - 1];
        tracing::debug!(
            "Received priority requests with serial ids: {} (block {}) - {} (block {})",
            first.serial_id(),
            first.eth_block(),
            last.serial_id(),
            last.eth_block(),
        );
        assert_eq!(
            last.serial_id().0 - first.serial_id().0 + 1,
            priority_ops.len() as u64,
            "There is a gap in priority ops received"
        );

        let new_ops: Vec<_> = priority_ops
            .into_iter()
            .skip_while(|tx| tx.serial_id() < self.next_expected_priority_id)
            .collect();
        if new_ops.is_empty() {
            return Ok(());
        }

        let first_new = &new_ops[0];
        let last_new = new_ops[new_ops.len() - 1].clone();
        assert_eq!(
            first_new.serial_id(),
            self.next_expected_priority_id,
            "priority transaction serial id mismatch"
        );

        let stage_latency = METRICS.poll_eth_node[&PollStage::PersistL1Txs].start();
        APP_METRICS.processed_txs[&TxStage::added_to_mempool()].inc();
        APP_METRICS.processed_l1_txs[&TxStage::added_to_mempool()].inc();
        for new_op in new_ops {
            let eth_block = new_op.eth_block();
            storage
                .transactions_dal()
                .insert_transaction_l1(new_op, eth_block)
                .await;
        }
        stage_latency.observe();
        self.next_expected_priority_id = last_new.serial_id().next();
        Ok(())
    }

    fn relevant_topic(&self) -> Vec<H256> {
        vec![
            self.new_priority_request_signature,
            self.new_priority_request_factory_deps_signature,
        ]
    }
}
