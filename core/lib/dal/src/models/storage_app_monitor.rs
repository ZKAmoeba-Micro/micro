use micro_types::app_monitor::ShowStatus;

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct AppMonitorStatus {
    pub app_name: String,
    pub ip: String,
    pub start_time: i64,
    pub heartbeat_update_at: i64,
    pub heartbeat_time: i32,
}

impl From<AppMonitorStatus> for ShowStatus {
    fn from(tx: AppMonitorStatus) -> ShowStatus {
        ShowStatus {
            app_name: tx.app_name,
            ip: tx.ip,
            start_time: tx.start_time,
            heartbeat_update_at: tx.heartbeat_update_at,
            heartbeat_time: tx.heartbeat_time,
        }
    }
}
