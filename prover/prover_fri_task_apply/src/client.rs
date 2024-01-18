use std::fmt::Debug;

use micro_contracts::sys_assignment_contract;
use micro_eth_client::{types::Error as MicroClientError, EthInterface};
use micro_system_constants::ASSIGNMENT_ADDRESS;
use micro_types::{
    l2::new_batch::NEW_BATCH,
    web3::{
        self,
        types::{BlockId, BlockNumber, FilterBuilder, Log},
    },
    Address, H256,
};

use crate::error::TaskApplyError;

#[async_trait::async_trait]
pub trait MicroClient {
    /// Returns events in a given block range.
    async fn get_events(
        &self,
        from: BlockNumber,
        to: BlockNumber,
        retries_left: usize,
    ) -> Result<Vec<Log>, TaskApplyError>;
    /// Returns finalized L1 block number.
    async fn finalized_block_number(&self) -> Result<u64, TaskApplyError>;
}

pub const RETRY_LIMIT: usize = 5;
const TOO_MANY_RESULTS_INFURA: &str = "query returned more than";
const TOO_MANY_RESULTS_ALCHEMY: &str = "response size exceeded";

#[derive(Debug)]
pub struct MicroHttpQueryClient<E> {
    client: E,
    topics: Vec<H256>,
    contract_addr: Address,
    confirmations_for_eth_event: Option<u64>,
}

impl<E: EthInterface> MicroHttpQueryClient<E> {
    pub fn new(client: E, confirmations_for_eth_event: Option<u64>) -> Self {
        let contract_abi = sys_assignment_contract();
        let events = vec![NEW_BATCH];
        let mut topics = Vec::new();
        for name in events {
            let msg = format!("{} event is missing in abi", name);
            let signature = contract_abi.event(name).expect(msg.as_str()).signature();
            topics.push(signature);
        }

        Self {
            client,
            topics,
            contract_addr: ASSIGNMENT_ADDRESS,
            confirmations_for_eth_event,
        }
    }

    async fn get_filter_logs(
        &self,
        from: BlockNumber,
        to: BlockNumber,
        topics: Vec<H256>,
    ) -> Result<Vec<Log>, TaskApplyError> {
        let filter = FilterBuilder::default()
            .address(vec![self.contract_addr])
            .from_block(from)
            .to_block(to)
            .topics(Some(topics), None, None, None)
            .build();

        self.client
            .logs(filter, "task_apply")
            .await
            .map_err(Into::into)
    }
}

#[async_trait::async_trait]
impl<E: EthInterface + Send + Sync + 'static> MicroClient for MicroHttpQueryClient<E> {
    async fn get_events(
        &self,
        from: BlockNumber,
        to: BlockNumber,
        retries_left: usize,
    ) -> Result<Vec<Log>, TaskApplyError> {
        tracing::info!(
            "get_events   from:{:?}  to:{:?}  retries_left:{:?}",
            from,
            to,
            retries_left
        );
        let mut result = self.get_filter_logs(from, to, self.topics.clone()).await;

        // This code is compatible with both Infura and Alchemy API providers.
        // Note: we don't handle rate-limits here - assumption is that we're never going to hit them.
        if let Err(TaskApplyError::MicroClient(MicroClientError::EthereumGateway(err))) = &result {
            tracing::warn!("Provider returned error message: {:?}", err);
            let err_message = err.to_string();
            let err_code = if let web3::Error::Rpc(err) = err {
                Some(err.code.code())
            } else {
                None
            };

            let should_retry = |err_code, err_message: String| {
                // All of these can be emitted by either API provider.
                err_code == Some(-32603)             // Internal error
                    || err_message.contains("failed")    // Server error
                    || err_message.contains("timed out") // Time-out error
            };

            // check whether the error is related to having too many results
            if err_message.contains(TOO_MANY_RESULTS_INFURA)
                || err_message.contains(TOO_MANY_RESULTS_ALCHEMY)
            {
                // get the numeric block ids
                let from_number = match from {
                    BlockNumber::Number(num) => num,
                    _ => {
                        // invalid variant
                        return result;
                    }
                };
                let to_number = match to {
                    BlockNumber::Number(num) => num,
                    BlockNumber::Latest => self.client.block_number("task_apply").await?,
                    _ => {
                        // invalid variant
                        return result;
                    }
                };

                // divide range into two halves and recursively fetch them
                let mid = (from_number + to_number) / 2;

                // safety check to prevent infinite recursion (quite unlikely)
                if from_number >= mid {
                    return Err(TaskApplyError::ClientError(
                        "Infinite recursion caused by too many responses".to_string(),
                    ));
                }
                tracing::warn!(
                    "Splitting block range in half: {:?} - {:?} - {:?}",
                    from,
                    mid,
                    to
                );
                let mut first_half = self
                    .get_events(from, BlockNumber::Number(mid), RETRY_LIMIT)
                    .await?;
                let mut second_half = self
                    .get_events(BlockNumber::Number(mid + 1u64), to, RETRY_LIMIT)
                    .await?;

                first_half.append(&mut second_half);
                result = Ok(first_half);
            } else if should_retry(err_code, err_message) && retries_left > 0 {
                tracing::warn!("Retrying. Retries left: {:?}", retries_left);
                result = self.get_events(from, to, retries_left - 1).await;
            }
        }

        result
    }

    async fn finalized_block_number(&self) -> Result<u64, TaskApplyError> {
        if let Some(confirmations) = self.confirmations_for_eth_event {
            let latest_block_number = self.client.block_number("task_apply").await?.as_u64();
            Ok(latest_block_number.saturating_sub(confirmations))
        } else {
            self.client
                .block(BlockId::Number(BlockNumber::Finalized), "task_apply")
                .await
                .map_err(Into::into)
                .map(|res| {
                    res.expect("Finalized block must be present on L1")
                        .number
                        .expect("Finalized block must contain number")
                        .as_u64()
                })
        }
    }
}
