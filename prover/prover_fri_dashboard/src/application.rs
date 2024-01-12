use axum::Json;
use serde::{Deserialize, Serialize};

use crate::error::DashboardError;

#[derive(Debug, Serialize, Deserialize)]
pub struct Status {
    app_name: String,
    start_time: i32,
    ip: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {}

pub async fn get() -> Result<Json<Response>, DashboardError> {
    // TODO
    Ok(Json(Response {}))
}

pub async fn add() {}

pub async fn update() {}
