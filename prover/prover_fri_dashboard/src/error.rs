use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

use micro_dal::SqlxError;

pub enum DashboardError {
    DatabaseError(SqlxError),
    RpcError(String),
}

impl IntoResponse for DashboardError {
    fn into_response(self) -> Response {
        let (status_code, message) = match self {
            DashboardError::DatabaseError(err) => {
                tracing::error!("database error: {:?}", err);
                (StatusCode::BAD_REQUEST, err.to_string())
            }
            DashboardError::RpcError(err) => {
                tracing::error!("rpc error: {:?}", err);
                (StatusCode::BAD_REQUEST, err.to_string())
            }
        };
        tracing::info!("response {} {}", status_code, message);
        (status_code, message).into_response()
    }
}
