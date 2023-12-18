use std::time::Duration;

use micro_basic_types::H256;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct FriProverTaskApplyConfig {
    pub rpc_url: String,
    pub poll_duration_secs: u16,
    pub confirmations_for_eth_event: u64,
    pub chain_id: u64,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::configs::test_utils::EnvMutex;

    static MUTEX: EnvMutex = EnvMutex::new();

    fn expected_config() -> FriProverTaskApplyConfig {
        FriProverTaskApplyConfig {
            rpc_url: "http://private-dns-for-server".to_string(),
            poll_duration_secs: 100,
            confirmations_for_eth_event: 10,
            chain_id: 270,
        }
    }

    #[test]
    fn from_env() {
        let config = r#"
            FRI_PROVER_TASK_APPLY_RPC_URL="http://private-dns-for-server"
            FRI_PROVER_TASK_APPLY_POLL_DURATION_SECS="100"
            FRI_PROVER_TASK_APPLY_CONFIRMATIONS_FOR_ETH_EVENT="10"
            FRI_PROVER_TASK_APPLY_CHAIN_ID="270"

        "#;
        let mut lock = MUTEX.lock();
        lock.set_env(config);
        let actual = FriProverTaskApplyConfig::from_env().unwrap();
        assert_eq!(actual, expected_config());
    }
}
