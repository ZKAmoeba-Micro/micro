use std::{net::SocketAddr, sync::Arc};

use axum::{
    extract::{ConnectInfo, Query, State},
    Json,
};
use micro_types::app_monitor::{FilterStatus, QueryStatus, ShowStatus, Status};

use crate::{
    application_monitor::{add_record, get_app_monitors, update_record},
    dashboard::Dashboard,
    error::DashboardError,
};

pub async fn get(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Query(params): Query<QueryStatus>,
    State(state): State<Arc<Dashboard>>,
) -> Result<Json<Vec<ShowStatus>>, DashboardError> {
    let offset = (params.page - 1) * params.page_size;
    let limit = params.page_size;
    let ip = addr.ip().to_string();

    let filter = FilterStatus {
        ip: ip,
        query: params,
    };
    let list = get_app_monitors(&state.pool, filter, offset, limit).await;
    match list {
        Ok(result) => Ok(Json(result)),
        Err(e) => {
            tracing::error!("app monitor database error: {:?}", e);
            Err(DashboardError::DatabaseError(e))
        }
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
