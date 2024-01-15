use std::sync::Arc;

use axum::{
    extract::{Query, State},
    Json,
};
use micro_types::app_monitor::{FilterStatus, Status};

use crate::{
    application_monitor::{add_record, get_app_monitors, update_record},
    dashboard::Dashboard,
    error::DashboardError,
};

pub async fn get(
    Query(params): Query<FilterStatus>,
    State(state): State<Arc<Dashboard>>,
) -> Result<Json<Vec<Status>>, DashboardError> {
    let offset = (params.page - 1) * params.page_size;
    let limit = params.page_size;
    let list = get_app_monitors(&state.pool, params, offset, limit).await;

    match list {
        Ok(result) => Ok(Json(result)),
        Err(e) => {
            tracing::error!("app monitor database error: {:?}", e);
            Err(DashboardError::DatabaseError(e))
        }
    }
}

pub async fn add(
    State(state): State<Arc<Dashboard>>,
    Json(data): Json<Status>,
) -> Result<Json<bool>, DashboardError> {
    let result = add_record(&state.pool, data.app_name, data.start_time, data.ip).await;
    match result {
        Ok(result) => Ok(Json(result)),
        Err(e) => Err(DashboardError::DatabaseError(e)),
    }
}

pub async fn update(
    State(state): State<Arc<Dashboard>>,
    Json(data): Json<Status>,
) -> Result<Json<bool>, DashboardError> {
    let result = update_record(
        &state.pool,
        data.heartbeat_update_at,
        data.app_name,
        data.ip,
        data.start_time,
    )
    .await;
    match result {
        Ok(result) => Ok(Json(result)),
        Err(e) => Err(DashboardError::DatabaseError(e)),
    }
}
