use std::time::Duration;

use futures::{
    channel::{mpsc, oneshot},
    StreamExt,
};
use micro_config::configs::{api::Web3JsonRpcConfig, eth_sender::SenderConfig};
use micro_dal::ConnectionPool;
use micro_eth_signer::{EthereumSigner, PrivateKeySigner, TransactionParameters};
use micro_types::{
    api, l2::L2Tx, transaction_request::CallRequest, L2ChainId, EIP_1559_TX_TYPE, H256, U256,
    USED_BOOTLOADER_MEMORY_BYTES,
};
use micro_web3_decl::error::Web3Error;
use tokio::sync::watch;

use self::caller::Data;
use crate::{
    api_server::{
        execution_sandbox::{BlockArgs, VmConcurrencyBarrier},
        tx_sender::TxSender,
    },
    l1_gas_price::L1GasPriceProvider,
};

const GRACEFUL_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(15);

pub mod caller;

#[derive(Debug)]
pub struct L2SenderConfig {
    sender_config: SenderConfig,
    pub estimate_gas_scale_factor: f64,
    pub estimate_gas_acceptable_overestimation: u32,
    pub max_tx_size: usize,
    pub chain_id: L2ChainId,
}

impl L2SenderConfig {
    pub fn new(
        sender_config: SenderConfig,
        web3_json_rpc_config: Web3JsonRpcConfig,
        chain_id: L2ChainId,
    ) -> Self {
        Self {
            sender_config,
            estimate_gas_scale_factor: web3_json_rpc_config.estimate_gas_scale_factor,
            estimate_gas_acceptable_overestimation: web3_json_rpc_config
                .estimate_gas_acceptable_overestimation,
            max_tx_size: web3_json_rpc_config.max_tx_size,
            chain_id,
        }
    }

    fn private_key(&self) -> Option<H256> {
        self.sender_config.private_key()
    }
}

#[derive(Debug)]
pub struct L2Sender<G: L1GasPriceProvider> {
    config: L2SenderConfig,

    pool: ConnectionPool,

    tx_sender: TxSender<G>,
    vm_barrier: VmConcurrencyBarrier,

    eth_signer: PrivateKeySigner,

    msg_receiver: mpsc::UnboundedReceiver<caller::Data>,
    msg_sender: mpsc::UnboundedSender<caller::Data>,
}

impl<G: L1GasPriceProvider> L2Sender<G> {
    pub fn new(
        config: L2SenderConfig,
        pool: ConnectionPool,
        tx_sender: TxSender<G>,
        vm_barrier: VmConcurrencyBarrier,
    ) -> Self {
        let (tx, rx) = mpsc::unbounded();

        let operator_private_key = config
            .private_key()
            .expect("Operator private key is required for signing client");
        let eth_signer = PrivateKeySigner::new(operator_private_key);

        L2Sender {
            config,
            pool,

            tx_sender,
            vm_barrier,

            eth_signer,

            msg_receiver: rx,
            msg_sender: tx,
        }
    }

    pub fn get_caller(&self) -> caller::Caller {
        caller::Caller::new(self.msg_sender.clone())
    }

    pub async fn run(mut self, mut stop_receiver: watch::Receiver<bool>) -> anyhow::Result<()> {
        loop {
            if *stop_receiver.borrow_and_update() {
                tracing::info!("Stop signal received, l2_sender is shutting down");
                self.vm_barrier.close();
                break;
            }

            tokio::select! {
                _ = stop_receiver.changed() => {
                    tracing::info!("Stop signal received, l2_sender is shutting down");
                    self.vm_barrier.close();
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

        Self::wait_for_vm(self.vm_barrier).await;

        Ok(())
    }

    async fn process(&self, data: Data) -> anyhow::Result<()> {
        match data {
            caller::Data::Call(data, callback) => self.call(data, callback).await,
            caller::Data::Send(data, callback) => self.send(data, callback).await,
        }
    }

    async fn call(
        &self,
        data: CallRequest,
        callback: oneshot::Sender<Result<Vec<u8>, Web3Error>>,
    ) -> anyhow::Result<()> {
        let mut connection = self.pool.access_storage_tagged("api").await.unwrap();
        let block_args = BlockArgs::new(
            &mut connection,
            api::BlockId::Number(api::BlockNumber::Latest),
        )
        .await
        .map_err(|err| internal_error("l2 sender", err))?
        .ok_or(Web3Error::NoBlock)?;

        let tx = L2Tx::from_request(data.into(), USED_BOOTLOADER_MEMORY_BYTES)?;

        let call_result = self.tx_sender.eth_call(block_args, tx).await;
        let call_result = call_result
            .map_err(|err| Web3Error::SubmitTransactionError(err.to_string(), err.data()));

        callback
            .send(call_result)
            .map_err(|_| Web3Error::InternalError)?;

        Ok(())
    }

    async fn send(
        &self,
        mut data: CallRequest,
        callback: oneshot::Sender<Result<H256, Web3Error>>,
    ) -> anyhow::Result<()> {
        data.from = Some(self.eth_signer.get_address().await.unwrap());
        data.nonce = Some(self.get_nonce().await);
        data.transaction_type = Some(EIP_1559_TX_TYPE.into());

        let tx = L2Tx::from_request(data.clone().into(), self.config.max_tx_size)?;

        let scale_factor = self.config.estimate_gas_scale_factor;
        let acceptable_overestimation = self.config.estimate_gas_acceptable_overestimation;

        let fee = self
            .tx_sender
            .get_txs_fee_in_wei(tx.into(), scale_factor, acceptable_overestimation)
            .await
            .map_err(|err| Web3Error::SubmitTransactionError(err.to_string(), err.data()));

        let fee = match fee {
            Ok(f) => f,
            Err(e) => {
                callback
                    .send(Err(e))
                    .map_err(|_| Web3Error::InternalError)?;

                return Ok(());
            }
        };

        let tx = TransactionParameters {
            nonce: data.nonce.unwrap_or_default(),
            to: data.to,
            gas: fee.gas_limit,
            value: data.value.unwrap_or_default(),
            data: data.data.unwrap_or_default().0,
            chain_id: self.config.chain_id.as_u64(),
            max_priority_fee_per_gas: fee.max_priority_fee_per_gas,
            gas_price: None,
            transaction_type: data.transaction_type,
            access_list: None,
            max_fee_per_gas: fee.max_fee_per_gas,
        };
        let signed_tx = self.eth_signer.sign_transaction(tx).await?;

        let (tx_request, hash) =
            api::TransactionRequest::from_bytes(&signed_tx, self.config.chain_id)?;
        let mut l2_tx = L2Tx::from_request(tx_request, self.config.max_tx_size)?;
        l2_tx.set_input(signed_tx, hash);

        let submit_result = self.tx_sender.submit_tx(l2_tx).await;
        let submit_result = submit_result.map(|_| hash).map_err(|err| {
            tracing::debug!("Send raw transaction error: {err}");
            metrics::counter!(
                "l2.submit_tx_error",
                1,
                "reason" => err.prom_error_code()
            );
            Web3Error::SubmitTransactionError(err.to_string(), err.data())
        });

        callback
            .send(submit_result)
            .map_err(|_| Web3Error::InternalError)?;

        Ok(())
    }

    async fn wait_for_vm(vm_barrier: VmConcurrencyBarrier) {
        let wait_for_vm =
            tokio::time::timeout(GRACEFUL_SHUTDOWN_TIMEOUT, vm_barrier.wait_until_stopped());
        if wait_for_vm.await.is_err() {
            tracing::warn!(
                "VM execution on l2 sender server didn't stop after {GRACEFUL_SHUTDOWN_TIMEOUT:?}; \
                 forcing shutdown anyway"
            );
        } else {
            tracing::info!("VM execution on l2 sender server stopped");
        }
    }

    async fn get_nonce(&self) -> U256 {
        let mut connection = self.pool.access_storage_tagged("api").await.unwrap();

        connection
            .transactions_web3_dal()
            .next_nonce_by_initiator_account(self.eth_signer.get_address().await.unwrap())
            .await
            .unwrap()
    }
}

pub fn internal_error(method_name: &str, error: impl ToString) -> Web3Error {
    tracing::error!(
        "Internal error in method {}: {}",
        method_name,
        error.to_string(),
    );
    metrics::counter!("l2.sender.internal_errors", 1, "method" => method_name.to_string());

    Web3Error::InternalError
}
