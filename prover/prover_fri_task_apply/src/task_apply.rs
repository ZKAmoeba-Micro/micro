use std::time::Duration;

use micro::{signer::Signer, types::U256, wallet::Wallet, EthNamespaceClient};
use micro_config::configs::FriProverTaskApplyConfig;
use micro_contracts::sys_assignment_contract;
use micro_dal::ConnectionPool;
use micro_eth_signer::{EthereumSigner, PrivateKeySigner};
use micro_system_constants::ASSIGNMENT_ADDRESS;
use micro_types::{
    api::{BlockIdVariant, BlockNumber},
    ethabi::{Contract, Token},
    transaction_request::CallRequest,
    L2ChainId, Nonce,
};
use tokio::sync::watch;

#[derive(Debug)]
pub struct TaskApply {
    poll_interval: Duration,
    pool: ConnectionPool,
    wallet: Wallet<PrivateKeySigner, micro::HttpClient>,
    contract_abi: Contract,
}

impl TaskApply {
    pub async fn new(config: FriProverTaskApplyConfig, pool: ConnectionPool) -> Self {
        let contract_abi = sys_assignment_contract();
        let operator_private_key = config
            .prover_private_key()
            .expect("Operator private key is required for signing client");
        let eth_signer = PrivateKeySigner::new(operator_private_key);
        let address = eth_signer.get_address().await.unwrap();
        let chain_id = L2ChainId::try_from(config.chain_id).unwrap();
        let signer = Signer::new(eth_signer, address, chain_id);
        let wallet: Wallet<PrivateKeySigner, micro::HttpClient> =
            Wallet::with_http_client(&config.rpc_url, signer).unwrap();
        let poll_interval = config.poll_duration();
        Self {
            poll_interval,
            pool,
            wallet,
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
            return Ok(());
        }

        let mut query_count = self
            .pool
            .access_storage()
            .await
            .unwrap()
            .fri_gpu_prover_queue_dal()
            .fri_task_count()
            .await;
        if query_count == 0 {
            return Ok(());
        }
        let arr = self.list(query_count).await.unwrap();

        if arr.len() > 0 {
            let mut nonce = self.wallet.get_nonce().await.unwrap();
            for token in arr {
                let batch_number: U256 = token.into_uint().unwrap();
                let _ = self.apply(nonce, batch_number).await;
                nonce += 1;
            }
        }

        Ok(())
    }

    async fn list(&mut self, query_count: u32) -> anyhow::Result<Vec<Token>> {
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

        Ok(arr)
    }

    async fn apply(&mut self, nonce: u32, batch_number: U256) -> anyhow::Result<()> {
        tracing::info!("task_apply nonce:{:?} batch :{:?} ", nonce, batch_number);
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
        };

        Ok(())
    }
}
