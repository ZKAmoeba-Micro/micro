use axum::Json;
use bigdecimal::{BigDecimal, Zero};
use serde::{Deserialize, Serialize};

use crate::error::DashboardError;

#[derive(Debug, Serialize, Deserialize)]
pub enum Status {
    UnDeposit,
    Normal,
    Frozen,
    Applying,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub in_white_list: bool,
    pub amount: String,
    pub status: Status,
}

pub async fn get() -> Result<Json<Response>, DashboardError> {
    // TODO
    Ok(Json(Response {
        in_white_list: false,
        amount: BigDecimal::zero().to_string(),
        status: Status::UnDeposit,
    }))
}
