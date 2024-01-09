use std::time::Duration;

use anyhow::Context;
use micro_dal::ConnectionPool;
use tokio::{sync::watch, time::sleep};
#[derive(Debug)]
pub struct MonitorTransactions {
    pool: ConnectionPool,
    processing_timeout: Duration,
    retry_interval_ms: u64,
}

impl MonitorTransactions {
    const SERVICE_NAME: &'static str = "MonitorTransactions";

    pub fn new(pool: ConnectionPool, processing_timeout: Duration, retry_interval_ms: u64) -> Self {
        Self {
            pool,
            processing_timeout,
            retry_interval_ms,
        }
    }
    async fn time_check(&mut self) {
        let mut connection = self.pool.access_storage().await.unwrap();
        let reqs = connection
            .transactions_dal()
            .get_last_transactions_time()
            .await;
        tracing::info!("MonitorTransactions reqs:{:?}", &reqs);

        if reqs > self.processing_timeout.as_secs().try_into().unwrap() {
            tracing::error!(
                "MonitorTransactions: No transaction for a long time diff_time:{:?},time:{:?}",
                reqs,
                self.processing_timeout
            );
        }
    }

    pub async fn run(mut self, stop_receiver: watch::Receiver<bool>) -> anyhow::Result<()>
    where
        Self: Sized,
    {
        tracing::info!(
            "Starting periodic job: {} with frequency: {} ms",
            Self::SERVICE_NAME,
            self.polling_interval_ms()
        );
        loop {
            if *stop_receiver.borrow() {
                tracing::warn!(
                    "Stop signal received, shutting down {} component while waiting for a new job",
                    Self::SERVICE_NAME
                );
                return Ok(());
            }
            self.run_routine_task()
                .await
                .context("run_routine_task()")?;
            sleep(Duration::from_millis(self.polling_interval_ms())).await;
        }
    }

    async fn run_routine_task(&mut self) -> anyhow::Result<()> {
        self.time_check().await;
        Ok(())
    }
    fn polling_interval_ms(&self) -> u64 {
        self.retry_interval_ms
    }
}
