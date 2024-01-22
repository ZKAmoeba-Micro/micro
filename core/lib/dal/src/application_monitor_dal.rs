use micro_types::app_monitor::{QueryStatus, ShowStatus};
use sqlx::Row;

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
        heartbeat_time: i32,
    ) -> Result<(), SqlxError> {
        sqlx::query!(
            r#"INSERT INTO application_monitor 
        (app_name,ip,start_at,heartbeat_update_at,heartbeat_time,created_at,updated_at) 
        VALUES ($1,$2,$3,$4,$5,now(),now())"#,
            app_name,
            ip,
            start_time,
            start_time,
            heartbeat_time
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

    pub async fn get_count(&mut self, filter: QueryStatus) -> Result<Option<u32>, SqlxError> {
        let (where_sql, _arg_index) = self.build_where_clause(&filter);
        let query = format!(
            r#"select count(1) AS "total" from (
            SELECT count(1)  FROM application_monitor
            WHERE {}
            group by ip,app_name) b
            "#,
            where_sql
        );
        let mut query = sqlx::query(&query);
        match &filter.ip {
            Some(ip) => {
                let r = format!("{}%", ip);
                query = query.bind(r);
            }
            None => {}
        };
        match &filter.app_name {
            Some(app_name) => {
                query = query.bind(app_name);
            }
            None => {}
        };
        match filter.start_time {
            Some(start_time) => {
                query = query.bind(start_time);
            }
            None => {}
        };
        match filter.end_time {
            Some(end_time) => {
                query = query.bind(end_time);
            }
            None => {}
        };
        let result = query
            .instrument("get_count")
            .with_arg("filter", &filter)
            .fetch_optional(self.storage.conn())
            .await?;
        Ok(result.map(|row| row.get::<i64, _>("total") as u32))
    }

    pub async fn get_app_monitors(
        &mut self,
        filter: QueryStatus,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<ShowStatus>, SqlxError> {
        let (where_sql, arg_index) = self.build_where_clause(&filter);
        let query = format!(
            r#"WITH Re AS (  
                SELECT   
                    app_name, ip, start_at as start_time, heartbeat_update_at,heartbeat_time,
                    ROW_NUMBER() OVER(PARTITION BY app_name, ip ORDER BY id DESC) as rn  
                FROM   
                    application_monitor
                where {}
            )  
            SELECT   
                app_name, ip, start_time, heartbeat_update_at,heartbeat_time
            FROM   
                Re  
            WHERE   
                rn = 1
            ORDER BY start_time desc
            OFFSET ${} LIMIT ${}
            "#,
            where_sql,
            arg_index,
            arg_index + 1
        );

        let mut query = sqlx::query_as(&query);
        match &filter.ip {
            Some(ip) => {
                let r = format!("{}%", ip);
                query = query.bind(r);
            }
            None => {}
        };

        match &filter.app_name {
            Some(app_name) => {
                query = query.bind(app_name);
            }
            None => {}
        };
        match filter.start_time {
            Some(start_time) => {
                query = query.bind(start_time);
            }
            None => {}
        };
        match filter.end_time {
            Some(end_time) => {
                query = query.bind(end_time);
            }
            None => {}
        };
        query = query.bind(offset);
        query = query.bind(limit);

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

    fn build_where_clause(&self, filter: &QueryStatus) -> (String, u8) {
        let mut arg_index = 1;
        let mut where_sql = format!("(1=1)");

        match &filter.ip {
            Some(_) => {
                where_sql += &format!(" AND (ip like ${})", arg_index);
                arg_index += 1;
            }
            None => {}
        };
        match &filter.app_name {
            Some(_) => {
                where_sql += &format!(" AND (app_name = ${})", arg_index);
                arg_index += 1;
            }
            None => {}
        };
        match filter.start_time {
            Some(_) => {
                where_sql += &format!(" AND (start_at >= ${})", arg_index);
                arg_index += 1;
            }
            None => {}
        };
        match filter.end_time {
            Some(_) => {
                where_sql += &format!(" AND (start_at <= ${})", arg_index);
                arg_index += 1;
            }
            None => {}
        };
        (where_sql, arg_index)
    }
}
