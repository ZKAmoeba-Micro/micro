use std::time::Duration;

use micro_basic_types::H256;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct FriProverGatewayConfig {
    pub api_url: String,
    pub api_poll_duration_secs: u16,

    /// Configurations for prometheus
    pub prometheus_listener_port: u16,
    pub prometheus_pushgateway_url: String,
    pub prometheus_push_interval_ms: Option<u64>,

    pub app_monitor_url: Option<String>,
    pub retry_interval_ms: Option<u64>,
}

impl FriProverGatewayConfig {
    // Don't load private key, if it's not required.
    pub fn prover_private_key(&self) -> Option<H256> {
        std::env::var("FRI_PROVER_GATEWAY_PROVER_PRIVATE_KEY")
            .ok()
            .map(|pk| pk.parse().unwrap())
    }

    pub fn api_poll_duration(&self) -> Duration {
        Duration::from_secs(self.api_poll_duration_secs as u64)
    }
}
