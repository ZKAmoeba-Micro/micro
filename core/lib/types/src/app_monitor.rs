use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QueryStatus {
    pub app_name: String,
    pub start_time: i64,
    pub end_time: i64,
    pub page: usize,
    pub page_size: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FilterStatus {
    pub ip: String,
    pub query: QueryStatus,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ShowStatus {
    pub id: i64,
    pub app_name: String,
    pub ip: String,
    pub start_time: i64,
    pub heartbeat_update_at: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Status {
    pub app_name: String,
    pub start_time: i64,
    pub heartbeat_update_at: i64,
}
