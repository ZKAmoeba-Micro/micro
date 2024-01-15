use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FilterStatus {
    pub app_name: String,
    pub ip: String,
    pub start_time: i32,
    pub end_time: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Status {
    pub id: i32,
    pub app_name: String,
    pub ip: String,
    pub start_time: i32,
    pub heartbeat_update_at: i32,
}
