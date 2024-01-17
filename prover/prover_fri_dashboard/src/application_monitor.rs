use micro_dal::{ConnectionPool, SqlxError};
use micro_types::app_monitor::{FilterStatus, ShowStatus};

pub(crate) async fn add_record(
    pool: &ConnectionPool,
    app_name: String,
    start_time: i64,
    ip: String,
    heartbeat_time: u32,
) -> Result<bool, SqlxError> {
    let mut connection = pool.access_storage().await.unwrap();
    let result = connection
        .application_monitor_dal()
        .insert(
            app_name.clone(),
            ip.clone(),
            start_time,
            heartbeat_time as i32,
        )
        .await;
    match result {
        Ok(_) => Ok(true),
        Err(e) => {
            tracing::error!("Adding the Application Monitor record failed.  app_name:{app_name},ip:{ip},start_time:{start_time},e:{e}");
            Err(e)
        }
    }
}
pub(crate) async fn update_record(
    pool: &ConnectionPool,
    update_at: i64,
    app_name: String,
    ip: String,
    start_time: i64,
) -> Result<bool, SqlxError> {
    let mut connection = pool.access_storage().await.unwrap();
    let result = connection
        .application_monitor_dal()
        .update(update_at, app_name.clone(), ip.clone(), start_time)
        .await;
    match result {
        Ok(_) => Ok(true),
        Err(e) => {
            tracing::error!("Update the Application Monitor record failed. update_at:{update_at}, app_name:{app_name},ip:{ip},start_time:{start_time},e:{e}");
            Err(e)
        }
    }
}

pub(crate) async fn get_app_monitors(
    pool: &ConnectionPool,
    filter: FilterStatus,
    offset: u32,
    limit: u32,
) -> Result<Vec<ShowStatus>, SqlxError> {
    let mut connection = pool.access_storage().await.unwrap();
    let result = connection
        .application_monitor_dal()
        .get_app_monitors(filter.clone(), offset, limit)
        .await;
    result
}

pub(crate) async fn get_count(pool: &ConnectionPool, filter: FilterStatus) -> u32 {
    let mut connection = pool.access_storage().await.unwrap();
    let result = connection
        .application_monitor_dal()
        .get_count(filter.clone())
        .await;
    match result {
        Ok(res) => match res {
            Some(r) => r,
            None => 0,
        },
        Err(e) => {
            tracing::error!(
                "Get the Application Monitor record failed. FilterStatus:{:?},e:{e}",
                filter
            );
            0
        }
    }
}
