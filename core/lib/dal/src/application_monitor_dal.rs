use micro_types::app_monitor::{FilterStatus, ShowStatus};

use crate::{
    instrument::InstrumentExt, models::storage_app_monitor::AppMonitorStatus, SqlxError,
    StorageProcessor,
};
#[derive(Debug)]
pub struct ApplicationMonitorDal<'a, 'c> {
    pub(crate) storage: &'a mut StorageProcessor<'c>,
}

impl ApplicationMonitorDal<'_, '_> {
    pub async fn insert(
        &mut self,
        app_name: String,
        ip: String,
        start_time: i64,
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
        update_at: i64,
        app_name: String,
        ip: String,
        start_time: i64,
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

    pub async fn get_app_monitors(
        &mut self,
        filter: FilterStatus,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<ShowStatus>, SqlxError> {
        let (where_sql, arg_index) = self.build_where_clause(&filter);
        let query = format!(
            r#"SELECT id, app_name, ip, start_at as start_time, heartbeat_update_at FROM application_monitor
            WHERE {}
            ORDER BY start_time desc
            OFFSET ${} LIMIT ${}
            "#,
            where_sql,
            arg_index,
            arg_index + 1
        );
        let mut query = sqlx::query_as(&query);

        if !filter.query.app_name.is_empty() {
            query = query.bind(&filter.query.app_name);
        }
        if filter.query.start_time > 0 {
            query = query.bind(filter.query.start_time);
        }
        if filter.query.end_time > 0 {
            query = query.bind(filter.query.end_time);
        }
        query = query.bind(offset as i32);
        query = query.bind(limit as i32);

        let db_results: Vec<AppMonitorStatus> = query
            .instrument("get_app_monitors")
            .report_latency()
            .with_arg("filter", &filter)
            .with_arg("limit", &limit)
            .fetch_all(self.storage.conn())
            .await?;

        let results = db_results.into_iter().map(Into::into).collect();
        Ok(results)
    }

    fn build_where_clause(&self, filter: &FilterStatus) -> (String, u8) {
        let mut arg_index = 1;
        let mut where_sql = format!("(ip = {})", filter.ip);
        if !filter.query.app_name.is_empty() {
            where_sql += &format!(" AND (app_name = ${})", arg_index);
            arg_index += 1;
        }
        if filter.query.start_time > 0 {
            where_sql += &format!(" AND (start_at >= ${})", arg_index);
            arg_index += 1;
        }
        if filter.query.end_time > 0 {
            where_sql += &format!(" AND (start_at <= ${})", arg_index);
            arg_index += 1;
        }
        (where_sql, arg_index)
    }
}
