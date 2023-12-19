use micro_config::configs::FriProverTaskApplyConfig;

use crate::{envy_load, FromEnv};

impl FromEnv for FriProverTaskApplyConfig {
    fn from_env() -> anyhow::Result<Self> {
        envy_load("fri_prover_gateway", "FRI_PROVER_GATEWAY_")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::EnvMutex;

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
