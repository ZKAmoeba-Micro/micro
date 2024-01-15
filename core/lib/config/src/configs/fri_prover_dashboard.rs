use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct FriProverDashboardConfig {
    pub host: String,
    pub port: u16,
    pub health_check_interval: u64,
}
