use std::time::Duration;

use micro_config::configs::FriProverTaskApplyConfig;
use micro_types::web3::types::BlockNumber;
use tokio::sync::watch;

// Local deps
use crate::client::MicroClient;
use crate::{caller::Caller, client::RETRY_LIMIT, error::TaskApplyError};

#[derive(Debug)]
pub struct EthWatch<W: MicroClient + Sync> {
    client: W,
    poll_interval: Duration,

    last_processed_micro_block: u64,
    caller: Caller,
}

impl<W: MicroClient + Sync> EthWatch<W> {
    pub async fn new(
        client: W,
        task_apply_config: FriProverTaskApplyConfig,
        caller: Caller,
    ) -> Self {
        let last_processed_micro_block = client.finalized_block_number().await.unwrap();
        let poll_interval = task_apply_config.poll_duration();

        Self {
            client,
            poll_interval,
            last_processed_micro_block,
            caller,
        }
    }

    pub async fn run(&mut self, stop_receiver: watch::Receiver<bool>) -> anyhow::Result<()> {
        let mut timer = tokio::time::interval(self.poll_interval);
        loop {
            if *stop_receiver.borrow() {
                tracing::info!("Stop signal received, eth_watch is shutting down");
                break;
            }
            timer.tick().await;

            if let Err(error) = self.loop_iteration().await {
                // This is an error because otherwise we could potentially miss a priority operation
                // thus entering priority mode, which is not desired.
                tracing::error!("Failed to process new blocks {}", error);
                self.last_processed_micro_block = self
                    .client
                    .finalized_block_number()
                    .await
                    .expect("cannot initialize task_apply: cannot get current Micro block");
            }
        }
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn loop_iteration(&mut self) -> Result<(), TaskApplyError> {
        let to_block = self.client.finalized_block_number().await?;

        tracing::info!(
            "micro_watch loop_iteration  from:{:?}   to:{:?} ",
            self.last_processed_micro_block,
            to_block
        );

        if to_block <= self.last_processed_micro_block {
            return Ok(());
        }

        let events = self
            .client
            .get_events(
                BlockNumber::Number(self.last_processed_micro_block.into()),
                BlockNumber::Number(to_block.into()),
                RETRY_LIMIT,
            )
            .await?;

        self.caller.scan_event_apply(events).await;

        self.last_processed_micro_block = to_block;
        Ok(())
    }
}
