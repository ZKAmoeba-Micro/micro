use axum::Json;
use serde::{Deserialize, Serialize};

use crate::error::DashboardError;

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {}

pub async fn get() -> Result<Json<Response>, DashboardError> {
    // TODO
    Ok(Json(Response {}))
}
