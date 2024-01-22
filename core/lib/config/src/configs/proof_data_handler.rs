use std::time::Duration;

use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, Copy, PartialEq)]
pub enum ProtocolVersionLoadingMode {
    FromDb,
    FromEnvVar,
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct ProofDataHandlerConfig {
    pub http_port: u16,
    pub proof_generation_timeout_in_secs: u64,
    pub protocol_version_loading_mode: ProtocolVersionLoadingMode,
    pub fri_protocol_version_id: u16,
    //Task polling interval, in milliseconds
    pub retry_interval_ms: u64,
    //The number of points for participating once
    pub once_score: u32,
}

impl ProofDataHandlerConfig {
    pub fn proof_generation_timeout(&self) -> Duration {
        Duration::from_secs(self.proof_generation_timeout_in_secs as u64)
    }
}
