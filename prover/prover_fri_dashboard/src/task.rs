use std::sync::Arc;

use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{dashboard::Dashboard, error::DashboardError};

#[derive(Debug, Serialize, Deserialize)]
pub struct Task {
    pub batch_number: u64,
    pub prove_status: String,       // data: completed number and all number
    pub compression_status: String, // data: queued, in_progress, successful, failed, sent_to_server
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Params {
    pub page: u32,
    pub page_size: u32,
}

pub async fn get(
    Query(params): Query<Params>,
    State(state): State<Arc<Dashboard>>,
) -> Result<Json<Vec<Task>>, DashboardError> {
    let mut connection = state.pool.access_storage().await.unwrap();

    let list = connection
        .fri_prover_jobs_dal()
        .get_job_status_details((params.page - 1) * params.page_size, params.page_size)
        .await;

    let result = list
        .into_iter()
        .map(|item| Task {
            batch_number: item.l1_batch_number.0 as u64,
            prove_status: format!(
                "{} / {}",
                item.prover_status_successful_count, item.prover_status_all_count
            ),
            compression_status: item.compression_status,
        })
        .collect();

    Ok(Json(result))
}
