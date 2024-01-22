use std::time::Duration;

use micro_basic_types::H256;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct FriProverTaskApplyConfig {
    pub rpc_url: String,
    pub poll_duration_secs: u16,
    pub call_contract_duration_secs: u64,
    pub contract_apply_count: Option<u32>,
    pub confirmations_for_eth_event: u64,
    pub chain_id: u64,
    pub app_monitor_url: Option<String>,
    pub retry_interval_ms: Option<u64>,
}

impl FriProverTaskApplyConfig {
    // Don't load private key, if it's not required.
    pub fn prover_private_key(&self) -> Option<H256> {
        std::env::var("FRI_PROVER_TASK_APPLY_PROVER_PRIVATE_KEY")
            .ok()
            .map(|pk| pk.parse().unwrap())
    }

    pub fn poll_duration(&self) -> Duration {
        Duration::from_secs(self.poll_duration_secs as u64)
    }

    pub fn call_contract_duration_secs(&self) -> Duration {
        Duration::from_secs(self.call_contract_duration_secs)
    }
}
