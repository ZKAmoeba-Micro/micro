use crate::{SqlxError, StorageProcessor};
// use serde::{Deserialize, Serialize};

// #[derive(Debug, Serialize, Deserialize)]
// pub struct Status {
//     app_name: String,
//     start_time: i32,
//     ip: String,
//     heartbeat_update_at: i32,
// }

#[derive(Debug)]
pub struct ApplicationMonitorDal<'a, 'c> {
    pub(crate) storage: &'a mut StorageProcessor<'c>,
}

impl ApplicationMonitorDal<'_, '_> {
    pub async fn insert(
        &mut self,
        app_name: String,
        ip: String,
        start_time: i32,
    ) -> Result<(), SqlxError> {
        sqlx::query!(
            r#"INSERT INTO application_monitor 
        (app_name,ip,start_at,heartbeat_update_at,created_at,updated_at) 
        VALUES ($1,$2,$3,$4,now(),now())"#,
            app_name,
            ip,
            start_time,
            start_time
        )
        .execute(self.storage.conn())
        .await?;
        Ok(())
    }

    pub async fn update(
        &mut self,
        update_at: i32,
        app_name: String,
        ip: String,
        start_time: i32,
    ) -> Result<(), SqlxError> {
        sqlx::query!("UPDATE application_monitor set heartbeat_update_at=$1,updated_at=now() where app_name=$2 and ip=$3 and start_at=$4",
        update_at,
        app_name,
        ip,
        start_time,
        )
        .execute(self.storage.conn())
        .await?;
        Ok(())
    }
}
