use micro_dal::ConnectionPool;

#[derive(Debug)]
pub struct ApplicationMonitor {
    pool: ConnectionPool,
}

impl ApplicationMonitor {
    pub fn new(pool: ConnectionPool) -> Self {
        Self { pool: pool }
    }

    async fn add(&mut self, app_name: String, start_time: i32, ip: String) {
        let mut connection = self.pool.access_storage().await.unwrap();
        let result = connection
            .application_monitor_dal()
            .insert(app_name.clone(), ip.clone(), start_time)
            .await;
        match result {
            Ok(_) => {}
            Err(e) => {
                tracing::error!("Adding the Application Monitor record failed.app_name:{app_name},ip:{ip},start_time:{start_time},e:{e}");
            }
        }
    }
}
