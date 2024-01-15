use micro_types::app_monitor::Status;

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct AppMonitorStatus {
    pub id: i32,
    pub app_name: String,
    pub ip: String,
    pub start_time: i32,
    pub heartbeat_update_at: i32,
}

impl From<AppMonitorStatus> for Status {
    fn from(tx: AppMonitorStatus) -> Status {
        Status {
            id: tx.id,
            app_name: tx.app_name,
            ip: tx.ip,
            start_time: tx.start_time,
            heartbeat_update_at: tx.heartbeat_update_at,
        }
    }
}
