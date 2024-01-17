use std::{net::SocketAddr, sync::Arc};

use axum::{
    extract::{ConnectInfo, Query, State},
    Json,
};
use micro_types::app_monitor::{FilterStatus, QueryStatus, ShowStatus, Status};
use serde::{Deserialize, Serialize};

use crate::{
    application_monitor::{add_record, get_app_monitors, get_count, update_record},
    dashboard::Dashboard,
    error::DashboardError,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub total_page: u32,
    pub list: Vec<ShowStatus>,
}

pub async fn get(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Query(params): Query<QueryStatus>,
    State(state): State<Arc<Dashboard>>,
) -> Result<Json<Response>, DashboardError> {
    let offset = (params.page - 1) * params.page_size;
    let limit = params.page_size;
    let ip = addr.ip().to_string();

    let filter = FilterStatus {
        ip: ip,
        query: params,
    };
    let count = get_count(&state.pool, filter.clone()).await;
    if count > 0 {
        let list = get_app_monitors(&state.pool, filter, offset, limit).await;
        match list {
            Ok(result) => {
                let mut total_page = count / limit;
                if count % limit != 0 {
                    total_page += 1;
                }
                Ok(Json(Response {
                    total_page,
                    list: result,
                }))
            }
            Err(e) => {
                tracing::error!("app get monitor database error: {:?}", e);
                // Err(DashboardError::DatabaseError(e));
                Ok(Json(Response {
                    total_page: 0,
                    list: vec![],
                }))
            }
        }
    } else {
        Ok(Json(Response {
            total_page: 0,
            list: vec![],
        }))
    }
}

pub async fn add(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<Arc<Dashboard>>,
    Json(data): Json<Status>,
) -> Result<Json<bool>, DashboardError> {
    let result = add_record(
        &state.pool,
        data.app_name,
        data.start_time,
        addr.ip().to_string(),
        data.heartbeat_time,
    )
    .await;
    match result {
        Ok(result) => Ok(Json(result)),
        Err(e) => Err(DashboardError::DatabaseError(e)),
    }
}

pub async fn update(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<Arc<Dashboard>>,
    Json(data): Json<Status>,
) -> Result<Json<bool>, DashboardError> {
    let result = update_record(
        &state.pool,
        data.heartbeat_update_at,
        data.app_name,
        addr.ip().to_string(),
        data.start_time,
    )
    .await;
    match result {
        Ok(result) => Ok(Json(result)),
        Err(e) => Err(DashboardError::DatabaseError(e)),
    }
}
