use micro_config::configs::FriProofCompressorConfig;

use crate::{envy_load, FromEnv};

impl FromEnv for FriProofCompressorConfig {
    fn from_env() -> anyhow::Result<Self> {
        envy_load("fri_proof_compressor", "FRI_PROOF_COMPRESSOR_")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::EnvMutex;

    static MUTEX: EnvMutex = EnvMutex::new();

    fn expected_config() -> FriProofCompressorConfig {
        FriProofCompressorConfig {
            compression_mode: 1,
            prometheus_listener_port: 3316,
            prometheus_pushgateway_url: "http://127.0.0.1:9091".to_string(),
            prometheus_push_interval_ms: Some(100),
            generation_timeout_in_secs: 3000,
            max_attempts: 5,
            universal_setup_path: "keys/setup/setup_2^26.key".to_string(),
            universal_setup_download_url:
                "https://storage.googleapis.com/zkamoeba-setup-keys-us/setup-keys/setup_2^26.key"
                    .to_string(),
            verify_wrapper_proof: false,
            app_monitor_url: Some("http://127.0.0.1:3000".to_string()),
            retry_interval_ms: Some(30000),
        }
    }

    #[test]
    fn from_env() {
        let mut lock = MUTEX.lock();
        let config = r#"
            FRI_PROOF_COMPRESSOR_COMPRESSION_MODE=1
            FRI_PROOF_COMPRESSOR_PROMETHEUS_LISTENER_PORT=3316
            FRI_PROOF_COMPRESSOR_PROMETHEUS_PUSHGATEWAY_URL="http://127.0.0.1:9091"
            FRI_PROOF_COMPRESSOR_PROMETHEUS_PUSH_INTERVAL_MS=100
            FRI_PROOF_COMPRESSOR_GENERATION_TIMEOUT_IN_SECS=3000
            FRI_PROOF_COMPRESSOR_MAX_ATTEMPTS=5
            FRI_PROOF_COMPRESSOR_UNIVERSAL_SETUP_PATH="keys/setup/setup_2^26.key"
            FRI_PROOF_COMPRESSOR_UNIVERSAL_SETUP_DOWNLOAD_URL="https://storage.googleapis.com/zkamoeba-setup-keys-us/setup-keys/setup_2^26.key"
            FRI_PROOF_COMPRESSOR_VERIFY_WRAPPER_PROOF=false
            FRI_PROOF_COMPRESSOR_APP_MONITOR_URL="http://127.0.0.1:3000"
            FRI_PROOF_COMPRESSOR_RETRY_INTERVAL_MS=30000
        "#;
        lock.set_env(config);

        let actual = FriProofCompressorConfig::from_env().unwrap();
        assert_eq!(actual, expected_config());
    }
}
