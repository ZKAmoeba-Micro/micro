use std::time::Duration;

use micro::{signer::Signer, wallet::Wallet};
use micro_config::configs::FriProverTaskApplyConfig;
use micro_contracts::sys_deposit_contract;
use micro_eth_signer::{EthereumSigner, PrivateKeySigner};
use micro_system_constants::DEPOSIT_ADDRESS;
use micro_types::{
    ethabi::{Contract, Token},
    l2::new_batch::NewBatch,
    web3::types::{BlockNumber as Web3BlockNumber, Log},
    L2ChainId,
};
use tokio::sync::watch;

// Local deps
use crate::client::RETRY_LIMIT;
use crate::client::{Error, MicroClient};

#[derive(Debug)]
pub struct EthWatch<W: MicroClient + Sync> {
    client: W,
    poll_interval: Duration,
    wallet: Wallet<PrivateKeySigner, micro::HttpClient>,
    last_processed_micro_block: u64,
    contract_abi: Contract,
}

impl<W: MicroClient + Sync> EthWatch<W> {
    pub async fn new(client: W, config: FriProverTaskApplyConfig) -> Self {
        let contract_abi = sys_deposit_contract();
        let operator_private_key = config
            .prover_private_key()
            .expect("Operator private key is required for signing client");
        let eth_signer = PrivateKeySigner::new(operator_private_key);
        let address = eth_signer.get_address().await.unwrap();
        let chain_id = L2ChainId::try_from(config.chain_id).unwrap();
        let signer = Signer::new(eth_signer, address, chain_id);
        let wallet: Wallet<PrivateKeySigner, micro::HttpClient> =
            Wallet::with_http_client(&config.rpc_url, signer).unwrap();
        let last_processed_micro_block = client.finalized_block_number().await.unwrap();
        let poll_interval = config.poll_duration();
        Self {
            client,
            poll_interval,
            wallet,
            last_processed_micro_block,
            contract_abi,
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
    async fn loop_iteration(&mut self) -> Result<(), Error> {
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
                Web3BlockNumber::Number(self.last_processed_micro_block.into()),
                Web3BlockNumber::Number(to_block.into()),
                RETRY_LIMIT,
            )
            .await?;

        //todo  send transaction
        if !events.is_empty() {
            for event in events {
                let _ = self.task_apply(event).await;
            }
        }

        self.last_processed_micro_block = to_block;
        Ok(())
    }

    async fn task_apply(&mut self, event: Log) -> Result<(), Error> {
        let batch = NewBatch::try_from(event.clone()).unwrap();
        tracing::info!("task_apply  batch :{:?}", batch);
        let data = self
            .contract_abi
            .function("proofApply")
            .unwrap()
            .encode_input(&[Token::Uint(batch.batch_number)])
            .unwrap();

        if let Err(error) = self
            .wallet
            .start_execute_contract()
            .contract_address(DEPOSIT_ADDRESS)
            .calldata(data)
            .send()
            .await
        {
            tracing::error!("Failed to apply new batch {:?}  error:{}", &batch, error);
        };

        Ok(())
    }
}
