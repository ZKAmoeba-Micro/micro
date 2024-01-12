use axum::Json;
use serde::{Deserialize, Serialize};

use crate::error::DashboardError;

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub local_block_number: u64,
    pub latest_block_number: u64,
}

pub async fn get() -> Result<Json<Response>, DashboardError> {
    // TODO
    Ok(Json(Response {
        local_block_number: 0,
        latest_block_number: 0,
    }))
}
