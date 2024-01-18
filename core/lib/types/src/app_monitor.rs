use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QueryStatus {
    pub ip: Option<String>,
    pub app_name: Option<String>,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ShowStatus {
    pub app_name: String,
    pub ip: String,
    pub start_time: i64,
    pub heartbeat_update_at: i64,
    pub heartbeat_time: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Status {
    pub app_name: String,
    pub start_time: i64,
    pub heartbeat_update_at: i64,
    pub heartbeat_time: u32,
}
