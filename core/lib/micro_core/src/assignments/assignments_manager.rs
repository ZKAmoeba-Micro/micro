use std::ops::Add;

use async_trait::async_trait;

use micro_dal::ConnectionPool;
use micro_prover_utils::periodic_job::PeriodicJob;
use micro_types::{
    assignment_user_summary::{AssignmentUserSummaryInfo, UserStatus},
    Address, L1BatchNumber,
};
use std::{time::Duration};
#[derive(Debug)]
pub struct AssignmentsManager {
    pool: ConnectionPool,
    processing_timeout: Duration,
    retry_interval_ms: u64,
}

impl AssignmentsManager {
    pub fn new(processing_timeout: Duration,retry_interval_ms: u64, pool: ConnectionPool) -> Self {
        Self {
            retry_interval_ms,
            processing_timeout,
            pool,
        }
    }

    pub async fn assign_proof_tasks(&mut self, block_number: L1BatchNumber, score: u16) {
        let mut connection = self.pool.access_storage().await.unwrap();
        let mut user_list = connection
            .assignment_user_summary_dal()
            .select_normal_user_list(UserStatus::Normal)
            .await;
        let user_count = user_list.len();
        tracing::info!("assign_proof_tasks list size: {:?}", user_count);
        if user_count > 0 {
            for user in &mut user_list {
                if let Some(sub_value) = block_number.0.checked_sub(user.last_batch_number.0) {
                    if let Some(dyn_score) = sub_value.checked_mul(score as u32) {
                        user.base_score = user.base_score.add(dyn_score as u16);
                    }
                }
            }
            user_list.sort_by(|a, b| a.base_score.cmp(&b.base_score));
            tracing::info!("assign_proof_tasks list: {:?}", user_list);
            if let Some(top_user) = user_list.first() {
                //TODO error
                let _ = connection
                    .assignments_dal()
                    .insert_assignments(top_user.verification_address, block_number)
                    .await;
            }
        }
    }

    pub async fn time_out_check(&mut self) {
        let mut connection = self.pool.access_storage().await.unwrap();
        connection.assignments_dal().update_assigments_status_for_time(self.processing_timeout).await;
    }

    pub fn monitor_change_event() {}
}
/// Prove task assigned to verification node periodically.
#[async_trait]
impl PeriodicJob for AssignmentsManager {
    const SERVICE_NAME: &'static str = "AssignmentsManager";

    async fn run_routine_task(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn polling_interval_ms(&self) -> u64 {
        self.retry_interval_ms
    }
}
