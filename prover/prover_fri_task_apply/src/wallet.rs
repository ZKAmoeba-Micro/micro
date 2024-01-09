use futures::{
    channel::{mpsc, oneshot},
    StreamExt,
};
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
    L2ChainId, Nonce, H256,
};
use tokio::sync::watch;

use crate::{caller, caller::Data, client::Error};

#[derive(Debug)]
pub struct TaskApplyWallet {
    wallet: Wallet<PrivateKeySigner, HttpClient>,
    contract_abi: Contract,
    nonce: u32,
    msg_receiver: mpsc::UnboundedReceiver<caller::Data>,
    msg_sender: mpsc::UnboundedSender<caller::Data>,
}

impl TaskApplyWallet {
    pub async fn new(config: FriProverTaskApplyConfig) -> Self {
        let contract_abi = sys_assignment_contract();
        let operator_private_key = config
            .prover_private_key()
            .expect("Operator private key is required for signing client");
        let eth_signer = PrivateKeySigner::new(operator_private_key);
        let address = eth_signer.get_address().await.unwrap();
        let chain_id = L2ChainId::try_from(config.chain_id).unwrap();
        let signer = Signer::new(eth_signer, address, chain_id);
        let wallet: Wallet<PrivateKeySigner, HttpClient> =
            Wallet::with_http_client(&config.rpc_url, signer).unwrap();
        let nonce = wallet.get_nonce().await.unwrap();
        let (tx, rx) = mpsc::unbounded();

        Self {
            wallet,
            contract_abi,
            nonce,
            msg_receiver: rx,
            msg_sender: tx,
        }
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
    async fn process(&mut self, data: Data) -> anyhow::Result<()> {
        match data {
            caller::Data::List(data, callback) => self.query_and_send(data, callback).await,
            caller::Data::Apply(data, callback) => self.scan_event_apply(data, callback).await,
        }
    }

    async fn query_and_send(
        &mut self,
        query_count: u32,
        _callback: oneshot::Sender<Result<H256, Error>>,
    ) -> anyhow::Result<()> {
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

        let bytes = self.wallet.provider.call(req, block).await.unwrap();

        let mut result = contract_function.decode_output(&bytes.0).unwrap();

        let arr = result.remove(0).into_array().unwrap();
        let len = arr.len() as u32;
        if len > 0 {
            let mut nonce = self.get_nonce().await;
            self.nonce += len;
            for token in arr {
                let batch_number = token.into_uint().unwrap();
                let _ = self.task_apply(nonce, batch_number).await;
                nonce += 1;
            }
        }

        Ok(())
    }

    async fn scan_event_apply(
        &mut self,
        events: Vec<Log>,
        _callback: oneshot::Sender<Result<H256, Error>>,
    ) -> anyhow::Result<()> {
        let len = events.len() as u32;
        tracing::info!("wallet scan_event_apply events:{:?}", len);
        if len > 0 {
            let mut nonce = self.get_nonce().await;
            self.nonce += len;
            for event in events {
                let batch = NewBatch::try_from(event.clone()).unwrap();
                let _ = self.task_apply(nonce, batch.batch_number).await;
                nonce += 1;
            }
        }

        Ok(())
    }

    async fn task_apply(&mut self, nonce: u32, batch_number: U256) -> anyhow::Result<()> {
        tracing::info!("task_apply  nonce:{:?} batch :{:?} ", nonce, batch_number);
        let data = self
            .contract_abi
            .function("proofApply")
            .unwrap()
            .encode_input(&[Token::Uint(batch_number)])
            .unwrap();

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

            self.nonce = self.wallet.get_nonce().await.unwrap();
        };

        Ok(())
    }

    async fn get_nonce(&mut self) -> u32 {
        let mut nonce = self.wallet.get_nonce().await.unwrap();
        if self.nonce > nonce {
            nonce = self.nonce;
        } else {
            self.nonce = nonce;
        }
        nonce
    }
}
