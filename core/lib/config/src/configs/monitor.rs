use std::time::Duration;

use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct MonitorConfig {
    pub timeout_in_secs: u16,
    //Task polling interval, in milliseconds
    pub retry_interval_ms: u64,
}

impl MonitorConfig {
    pub fn monitor_transactions_timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_in_secs as u64)
    }
}
