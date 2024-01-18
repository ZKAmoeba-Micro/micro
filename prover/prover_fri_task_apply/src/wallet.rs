use futures::{channel::mpsc, StreamExt};
use micro::{signer::Signer, types::U256, wallet::Wallet, EthNamespaceClient, HttpClient};
use micro_config::configs::FriProverTaskApplyConfig;
use micro_contracts::sys_assignment_contract;
use micro_eth_signer::{EthereumSigner, PrivateKeySigner};
use micro_system_constants::ASSIGNMENT_ADDRESS;
use micro_types::{
    api::{BlockIdVariant, BlockNumber},
    ethabi::{Contract, Token},
    l2::new_batch::NewBatch,
    transaction_request::CallRequest,
    web3::types::Log,
    L2ChainId, Nonce,
};
use tokio::sync::watch;

use crate::{caller, caller::Data, error::TaskApplyError};

#[derive(Debug)]
pub struct TaskApplyWallet {
    wallet: Wallet<PrivateKeySigner, HttpClient>,
    contract_abi: Contract,
    nonce: u32,
    msg_receiver: mpsc::UnboundedReceiver<caller::Data>,
    msg_sender: mpsc::UnboundedSender<caller::Data>,
}

impl TaskApplyWallet {
    pub async fn new(config: FriProverTaskApplyConfig) -> Result<Self, TaskApplyError> {
        let contract_abi = sys_assignment_contract();
        let operator_private_key = config
            .prover_private_key()
            .expect("Operator private key is required for signing client");
        let eth_signer = PrivateKeySigner::new(operator_private_key);
        let address = eth_signer
            .get_address()
            .await
            .map_err(|e| TaskApplyError::WalletError(e.to_string()))?;
        let chain_id = L2ChainId::try_from(config.chain_id)
            .map_err(|e| TaskApplyError::WalletError(e.to_string()))?;
        let signer = Signer::new(eth_signer, address, chain_id);
        let wallet: Wallet<PrivateKeySigner, HttpClient> =
            Wallet::with_http_client(&config.rpc_url, signer)
                .map_err(|e| TaskApplyError::ClientError(e.to_string()))?;
        let nonce = wallet
            .get_nonce()
            .await
            .map_err(|e| TaskApplyError::ClientError(e.to_string()))?;
        let (tx, rx) = mpsc::unbounded();

        Ok(Self {
            wallet,
            contract_abi,
            nonce,
            msg_receiver: rx,
            msg_sender: tx,
        })
    }

    pub fn get_caller(&self) -> caller::Caller {
        caller::Caller::new(self.msg_sender.clone())
    }

    pub async fn run(mut self, mut stop_receiver: watch::Receiver<bool>) -> anyhow::Result<()> {
        loop {
            if *stop_receiver.borrow() {
                tracing::info!("Stop signal received, task_apply is shutting down");
                break;
            }

            tokio::select! {
                _ = stop_receiver.changed() => {
                    tracing::info!("Stop signal received, task_apply is shutting down");
                    break;
                }
                data = self.msg_receiver.next() => {
                    if let Some(data) = data {
                        let res = self.process(data).await;
                        if let Err(e) = res {
                            tracing::error!("process l2 sender tx failed {:?}", e);
                        }
                    }
                }
            }
        }

        Ok(())
    }
    async fn process(&mut self, data: Data) -> anyhow::Result<(), TaskApplyError> {
        match data {
            caller::Data::QueryApply(data) => self.query_and_apply(data).await,
            caller::Data::EventApply(data) => self.scan_event_apply(data).await,
        }
    }

    async fn query_and_apply(&mut self, query_count: u32) -> anyhow::Result<(), TaskApplyError> {
        tracing::info!("wallet query_and_send query_count:{:?}", query_count);
        let contract_function = self
            .contract_abi
            .function("getBatchNumberList")
            .expect("failed to get `getBatchNumberList` function");
        let data = contract_function
            .encode_input(&[Token::Uint(0.into()), Token::Uint(query_count.into())])
            .expect("failed to encode parameters");
        let req = CallRequest {
            to: Some(ASSIGNMENT_ADDRESS),
            data: Some(data.into()),
            ..Default::default()
        };
        let block = Some(BlockIdVariant::BlockNumber(BlockNumber::Latest));

        let bytes = self
            .wallet
            .provider
            .call(req, block)
            .await
            .map_err(|e| TaskApplyError::WalletError(e.to_string()))?;

        let mut result = contract_function
            .decode_output(&bytes.0)
            .map_err(|e| TaskApplyError::WalletError(e.to_string()))?;

        let arr = result.remove(0).into_array().unwrap();

        if arr.len() > 0 {
            let mut nonce = self.get_nonce().await?;
            for token in arr {
                let batch_number = token.into_uint().unwrap();
                let res = self.task_apply(nonce, batch_number).await;
                match res {
                    Ok(_) => {
                        nonce += 1;
                        self.nonce = nonce;
                    }
                    Err(error) => {
                        tracing::error!("Task Apply error {:?}", error);
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    async fn scan_event_apply(&mut self, events: Vec<Log>) -> anyhow::Result<(), TaskApplyError> {
        tracing::info!("wallet scan_event_apply events:{:?}", events.len());
        if events.len() > 0 {
            let mut nonce = self.get_nonce().await?;
            for event in events {
                let new_batch = NewBatch::try_from(event.clone())
                    .map_err(|e| TaskApplyError::WalletError(e.to_string()))?;
                let res = self.task_apply(nonce, new_batch.batch_number).await;
                match res {
                    Ok(_) => {
                        nonce += 1;
                        self.nonce = nonce;
                    }
                    Err(error) => {
                        tracing::error!("Task Apply error {:?}", error);
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    async fn task_apply(
        &mut self,
        nonce: u32,
        batch_number: U256,
    ) -> anyhow::Result<(), TaskApplyError> {
        tracing::info!("task_apply  nonce:{:?} batch :{:?} ", nonce, batch_number);

        let apply_function = self
            .contract_abi
            .function("proofApply")
            .map_err(|e| TaskApplyError::WalletError(e.to_string()))?;
        let data = apply_function
            .encode_input(&[Token::Uint(batch_number)])
            .map_err(|e| TaskApplyError::WalletError(e.to_string()))?;

        let nonce = Nonce::from(nonce);

        if let Err(error) = self
            .wallet
            .start_execute_contract()
            .contract_address(ASSIGNMENT_ADDRESS)
            .calldata(data)
            .nonce(nonce)
            .send()
            .await
        {
            tracing::error!(
                "task_apply Failed to apply new batch {:?}  error:{}",
                batch_number,
                error
            );
            self.nonce = self
                .wallet
                .get_nonce()
                .await
                .map_err(|e| TaskApplyError::ClientError(e.to_string()))?;
            return Err(TaskApplyError::ClientError(error.to_string()));
        };

        Ok(())
    }

    async fn get_nonce(&mut self) -> Result<u32, TaskApplyError> {
        let mut nonce = self
            .wallet
            .get_nonce()
            .await
            .map_err(|e| TaskApplyError::ClientError(e.to_string()))?;
        if self.nonce > nonce {
            nonce = self.nonce;
        }
        Ok(nonce)
    }
}
