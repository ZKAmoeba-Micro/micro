use micro_config::configs::MonitorConfig;

use crate::{envy_load, FromEnv};

impl FromEnv for MonitorConfig {
    fn from_env() -> anyhow::Result<Self> {
        envy_load("monitor_transactions", "MONITOR_TRANSACTIONS_")
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::test_utils::EnvMutex;

    static MUTEX: EnvMutex = EnvMutex::new();

    fn expected_config() -> MonitorConfig {
        MonitorConfig {
            timeout_in_secs: 3600,
            retry_interval_ms: 1000,
        }
    }

    #[test]
    fn from_env() {
        let config = r#"
            MONITOR_TRANSACTIONS_TIMEOUT_IN_SECS="3600"
            MONITOR_TRANSACTIONS_RETRY_INTERVAL_MS="1000"
        "#;
        let mut lock = MUTEX.lock();
        lock.set_env(config);
        let actual = MonitorConfig::from_env().unwrap();
        assert_eq!(actual, expected_config());
    }
}
