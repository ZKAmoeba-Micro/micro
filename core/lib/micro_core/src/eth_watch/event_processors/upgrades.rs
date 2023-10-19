use crate::eth_watch::{
    client::{Error, EthClient},
    event_processors::EventProcessor,
};
use micro_contracts::{governance_contract, micro_contract};
use micro_dal::StorageProcessor;
use micro_types::{
    protocol_version::GovernanceOperation, web3::types::Log, Address, ProtocolUpgrade,
    ProtocolVersionId, H256,
};
use std::convert::TryFrom;
use std::time::Instant;

/// Responsible for saving new protocol upgrade proposals to the database.
#[derive(Debug)]
pub struct UpgradesEventProcessor {
    diamond_proxy_address: Address,
    last_seen_version_id: ProtocolVersionId,
    upgrade_proposal_signature: H256,
    execute_upgrade_short_signature: [u8; 4],
}

impl UpgradesEventProcessor {
    pub fn new(diamond_proxy_address: Address, last_seen_version_id: ProtocolVersionId) -> Self {
        Self {
            diamond_proxy_address,
            last_seen_version_id,
            upgrade_proposal_signature: governance_contract()
                .event("TransparentOperationScheduled")
                .expect("TransparentOperationScheduled event is missing in abi")
                .signature(),
            execute_upgrade_short_signature: micro_contract()
                .function("executeUpgrade")
                .unwrap()
                .short_signature(),
        }
    }
}

#[async_trait::async_trait]
impl<W: EthClient + Sync> EventProcessor<W> for UpgradesEventProcessor {
    async fn process_events(
        &mut self,
        storage: &mut StorageProcessor<'_>,
        client: &W,
        events: Vec<Log>,
    ) -> Result<(), Error> {
        let mut upgrades = Vec::new();
        for event in events
            .into_iter()
            .filter(|event| event.topics[0] == self.upgrade_proposal_signature)
        {
            let governance_operation = GovernanceOperation::try_from(event)
                .map_err(|err| Error::LogParse(format!("{:?}", err)))?;
            // Some calls can target other contracts than Diamond proxy, skip them.
            for call in governance_operation
                .calls
                .into_iter()
                .filter(|call| call.target == self.diamond_proxy_address)
            {
                if call.data.len() < 4 || &call.data[..4] != &self.execute_upgrade_short_signature {
                    continue;
                }

                let upgrade = ProtocolUpgrade::try_from(call)
                    .map_err(|err| Error::LogParse(format!("{:?}", err)))?;

                // Scheduler VK is not present in proposal event. It is hardcoded in verifier contract.
                let scheduler_vk_hash = if let Some(address) = upgrade.verifier_address {
                    Some(client.scheduler_vk_hash(address).await?)
                } else {
                    None
                };
                upgrades.push((upgrade, scheduler_vk_hash));
            }
        }

        if upgrades.is_empty() {
            return Ok(());
        }

        let ids_str: Vec<_> = upgrades
            .iter()
            .map(|(u, _)| format!("{}", u.id as u16))
            .collect();
        tracing::debug!("Received upgrades with ids: {}", ids_str.join(", "));

        let new_upgrades: Vec<_> = upgrades
            .into_iter()
            .skip_while(|(v, _)| v.id as u16 <= self.last_seen_version_id as u16)
            .collect();
        if new_upgrades.is_empty() {
            return Ok(());
        }

        let last_id = new_upgrades.last().unwrap().0.id;
        let stage_start = Instant::now();
        for (upgrade, scheduler_vk_hash) in new_upgrades {
            let previous_version = storage
                .protocol_versions_dal()
                .load_previous_version(upgrade.id)
                .await
                .expect("Expected previous version to be present in DB");
            let new_version = previous_version.apply_upgrade(upgrade, scheduler_vk_hash);
            storage
                .protocol_versions_dal()
                .save_protocol_version_with_tx(new_version)
                .await;
        }
        metrics::histogram!("eth_watcher.poll_eth_node", stage_start.elapsed(), "stage" => "persist_upgrades");

        self.last_seen_version_id = last_id;

        Ok(())
    }

    fn relevant_topic(&self) -> Vec<H256> {
        vec![self.upgrade_proposal_signature]
    }
}
