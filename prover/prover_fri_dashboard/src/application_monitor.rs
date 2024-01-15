use micro_dal::ConnectionPool;
use micro_types::app_monitor::{FilterStatus, Status};
#[derive(Debug)]
pub struct ApplicationMonitor {
    pool: ConnectionPool,
}

impl ApplicationMonitor {
    pub fn new(pool: ConnectionPool) -> Self {
        Self { pool: pool }
    }

    async fn add_record(&mut self, app_name: String, start_time: i32, ip: String) {
        let mut connection = self.pool.access_storage().await.unwrap();
        let result = connection
            .application_monitor_dal()
            .insert(app_name.clone(), ip.clone(), start_time)
            .await;
        match result {
            Ok(_) => {}
            Err(e) => {
                tracing::error!("Adding the Application Monitor record failed.  app_name:{app_name},ip:{ip},start_time:{start_time},e:{e}");
            }
        }
    }

    pub async fn update_record(
        &mut self,
        update_at: i32,
        app_name: String,
        ip: String,
        start_time: i32,
    ) {
        let mut connection = self.pool.access_storage().await.unwrap();
        let result = connection
            .application_monitor_dal()
            .update(update_at, app_name.clone(), ip.clone(), start_time)
            .await;
        match result {
            Ok(_) => {}
            Err(e) => {
                tracing::error!("Update the Application Monitor record failed. update_at:{update_at}, app_name:{app_name},ip:{ip},start_time:{start_time},e:{e}");
            }
        }
    }

    pub async fn get_app_monitors(
        &mut self,
        filter: FilterStatus,
        limit: usize,
    ) -> Option<Vec<Status>> {
        let mut connection = self.pool.access_storage().await.unwrap();
        let result = connection
            .application_monitor_dal()
            .get_app_monitors(filter.clone(), limit)
            .await;
        match result {
            Ok(res) => {
                return Some(res);
            }
            Err(e) => {
                tracing::error!(
                    "Get the Application Monitor record failed. filter:{:?},e:{e}",
                    filter
                );
                return None;
            }
        }
    }
}
