use std::time::Duration;

use micro_config::configs::FriProverTaskApplyConfig;
use micro_dal::ConnectionPool;
use tokio::sync::watch;

use crate::caller::Caller;

#[derive(Debug)]
pub struct TaskApply {
    call_contract_interval: Duration,
    pool: ConnectionPool,
    contract_apply_count: Option<u32>,
    caller: Caller,
}

impl TaskApply {
    pub async fn new(
        task_apply_config: FriProverTaskApplyConfig,
        pool: ConnectionPool,
        caller: Caller,
    ) -> Self {
        let poll_interval = task_apply_config.call_contract_duration_secs();
        let contract_apply_count = task_apply_config.contract_apply_count;

        Self {
            call_contract_interval: poll_interval,
            pool,
            contract_apply_count,
            caller,
        }
    }

    pub async fn run(&mut self, stop_receiver: watch::Receiver<bool>) -> anyhow::Result<()> {
        let mut timer = tokio::time::interval(self.call_contract_interval);
        loop {
            if *stop_receiver.borrow() {
                tracing::info!("Stop signal received, task_apply is shutting down");
                break;
            }

            timer.tick().await;
            let _ = self.loop_iteration().await;
        }
        Ok(())
    }

    pub async fn loop_iteration(&mut self) -> anyhow::Result<()> {
        let queue_count = self
            .pool
            .access_storage()
            .await
            .unwrap()
            .fri_proof_compressor_dal()
            .queued_count()
            .await;
        if queue_count > 0u32 {
            tracing::info!("task_apply queue_count greeter than 0");
            return Ok(());
        }

        let query_count;

        match self.contract_apply_count {
            Some(value) => {
                query_count = value;
            }
            None => {
                query_count = self
                    .pool
                    .access_storage()
                    .await
                    .unwrap()
                    .fri_gpu_prover_queue_dal()
                    .fri_task_count()
                    .await;
            }
        }

        if query_count == 0 {
            tracing::info!("task_apply query_count is 0");
            return Ok(());
        }
        self.caller.query_and_apply(query_count).await;

        Ok(())
    }
}
