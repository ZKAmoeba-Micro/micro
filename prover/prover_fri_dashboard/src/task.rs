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

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub total_page: u32,
    pub list: Vec<Task>,
}

pub async fn get(
    Query(params): Query<Params>,
    State(state): State<Arc<Dashboard>>,
) -> Result<Json<Response>, DashboardError> {
    let mut connection = state.pool.access_storage().await.unwrap();

    let count = connection
        .fri_prover_jobs_dal()
        .get_job_count()
        .await
        .map_err(|e| DashboardError::DatabaseError(e))?;

    match count {
        Some(c) => {
            let list = connection
                .fri_prover_jobs_dal()
                .get_job_status_details((params.page - 1) * params.page_size, params.page_size)
                .await
                .map_err(|e| DashboardError::DatabaseError(e))?;

            let list: Vec<Task> = list
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

            let mut total_page = c / params.page_size;
            if c % params.page_size != 0 {
                total_page += 1;
            }

            Ok(Json(Response { total_page, list }))
        }
        None => Ok(Json(Response {
            total_page: 0,
            list: vec![],
        })),
    }
}
