// Built-in uses
use std::time::Duration;
// External uses
use serde::Deserialize;
// Local uses
use crate::envy_load;

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct ContractVerifierConfig {
    /// Max time of a single compilation (in s).
    pub compilation_timeout: u64,
    /// Interval between polling db for verification requests (in ms).
    pub polling_interval: Option<u64>,
    /// Port to which the Prometheus exporter server is listening.
    pub prometheus_port: u16,
}

impl ContractVerifierConfig {
    pub fn from_env() -> Self {
        envy_load!("contract_verifier", "CONTRACT_VERIFIER_")
    }

    pub fn compilation_timeout(&self) -> Duration {
        Duration::from_secs(self.compilation_timeout)
    }

    pub fn polling_interval(&self) -> Duration {
        Duration::from_millis(self.polling_interval.unwrap_or(1000))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::configs::test_utils::set_env;

    fn expected_config() -> ContractVerifierConfig {
        ContractVerifierConfig {
            compilation_timeout: 30,
            polling_interval: Some(1000),
            prometheus_port: 3314,
        }
    }

    #[test]
    fn from_env() {
        let config = r#"
            CONTRACT_VERIFIER_COMPILATION_TIMEOUT=30
            CONTRACT_VERIFIER_POLLING_INTERVAL=1000
            CONTRACT_VERIFIER_PROMETHEUS_PORT=3314
        "#;
        set_env(config);

        let actual = ContractVerifierConfig::from_env();
        assert_eq!(actual, expected_config());
    }
}
