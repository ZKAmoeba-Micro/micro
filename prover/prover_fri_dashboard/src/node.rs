use std::sync::Arc;

use axum::{extract::State, Json};
use micro_types::U64;
use micro_web3_decl::namespaces::EthNamespaceClient;
use serde::{Deserialize, Serialize};

use crate::{dashboard::Dashboard, error::DashboardError};

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub local_block_number: u64,
    pub latest_block_number: u64,
}

pub async fn get(State(state): State<Arc<Dashboard>>) -> Result<Json<Response>, DashboardError> {
    let mut connection = state.pool.access_storage().await.unwrap();

    let local_block_number = connection
        .blocks_web3_dal()
        .get_sealed_miniblock_number()
        .await
        .map(|n| U64::from(n.0))
        .map_err(|e| DashboardError::DatabaseError(e))?;

    let latest_block_number = state
        .client
        .get_block_number()
        .await
        .map_err(|_| DashboardError::RpcError("".to_string()))?;

    Ok(Json(Response {
        local_block_number: local_block_number.as_u64(),
        latest_block_number: latest_block_number.as_u64(),
    }))
}
