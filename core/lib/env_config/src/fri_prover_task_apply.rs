use micro_config::configs::FriProverTaskApplyConfig;

use crate::{envy_load, FromEnv};

impl FromEnv for FriProverTaskApplyConfig {
    fn from_env() -> anyhow::Result<Self> {
        envy_load("fri_prover_task_apply", "FRI_PROVER_TASK_APPLY_")
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
            call_contract_duration_secs: 1800,
            contract_apply_count: Some(3),
            chain_id: 270,
            app_monitor_url: Some("http://127.0.0.1:3000".to_string()),
            retry_interval_ms: Some(30000),
        }
    }

    #[test]
    fn from_env() {
        let config = r#"
            FRI_PROVER_TASK_APPLY_RPC_URL="http://private-dns-for-server"
            FRI_PROVER_TASK_APPLY_POLL_DURATION_SECS="100"
            FRI_PROVER_TASK_APPLY_CALL_CONTRACT_DURATION_SECS="1800"
            FRI_PROVER_TASK_APPLY_CONFIRMATIONS_FOR_ETH_EVENT="10"
            FRI_PROVER_TASK_APPLY_CONTRACT_APPLY_COUNT="3"
            FRI_PROVER_TASK_APPLY_CHAIN_ID="270"
            FRI_PROVER_TASK_APPLY_APP_MONITOR_URL="http://127.0.0.1:3000"
            FRI_PROVER_TASK_APPLY_RETRY_INTERVAL_MS=30000

        "#;
        let mut lock = MUTEX.lock();
        lock.set_env(config);
        let actual = FriProverTaskApplyConfig::from_env().unwrap();
        assert_eq!(actual, expected_config());
    }
}
