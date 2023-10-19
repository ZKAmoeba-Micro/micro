use std::convert::TryFrom;
use std::sync::Arc;

use micro_dal::ConnectionPool;
use micro_state::PostgresStorageCaches;
use micro_types::{
    api,
    ethabi::Contract,
    l2::{
        system_contracts::{NodeInfo, NodeInfoList},
        L2Tx,
    },
    transaction_request::TransactionRequest,
    AccountTreeId, L2ChainId, DEPOSIT_ADDRESS, USED_BOOTLOADER_MEMORY_BYTES,
};

use micro_web3_decl::error::Web3Error;

use crate::api_server::execution_sandbox::execute_tx_eth_call;
use vm::{constants::BLOCK_GAS_LIMIT, ExecutionResult};

use super::{
    execution_sandbox::{BlockArgs, TxSharedArgs, VmConcurrencyLimiter},
    tx_sender::ApiContracts,
    web3::backend_jsonrpc::error::internal_error,
};

#[derive(Debug)]
pub struct SystemContractCaller {
    pool: ConnectionPool,
    fair_l2_gas_price: u64,
    vm_execution_cache_misses_limit: Option<usize>,
    deposit_abi: Contract,
    storage_caches: PostgresStorageCaches,
    api_contracts: ApiContracts,
    chain_id: L2ChainId,
    vm_concurrency_limiter: Arc<VmConcurrencyLimiter>,
}

impl SystemContractCaller {
    pub async fn new(
        connection_pool: ConnectionPool,
        fair_l2_gas_price: u64,
        chain_id: L2ChainId,
        storage_caches: PostgresStorageCaches,
        vm_execution_cache_misses_limit: Option<usize>,
        vm_concurrency_limiter: Arc<VmConcurrencyLimiter>,
    ) -> SystemContractCaller {
        let api_contracts = ApiContracts::load_from_disk();

        SystemContractCaller {
            pool: connection_pool,
            fair_l2_gas_price,
            vm_execution_cache_misses_limit,
            deposit_abi: micro_contracts::sys_deposit_contract(),
            storage_caches,
            api_contracts,
            chain_id,
            vm_concurrency_limiter,
        }
    }

    pub async fn deposit_node_list(&self) -> Result<Vec<NodeInfo>, Web3Error> {
        tracing::warn!("deposit_node_list start");
        let func = self.deposit_abi.function("getNodeInfo")?;

        let mut tx: TransactionRequest = TransactionRequest::default();
        tx.to = Some(DEPOSIT_ADDRESS);
        tx.input = func.encode_input(&[])?.into();

        let result = self.call(tx).await?;

        // parse result
        let res = func.decode_output(&result)?;
        let res = NodeInfoList::try_from(res)?;
        let last_res = res.list;
        // let check_node = get_all_addresses();
        // tracing::warn!(
        //     "deposit_node_list nodeCheck deposit_node_list:{:?}",
        //     check_node
        // );
        // tracing::warn!("deposit_node_list nodeCheck last_res:{:?}", last_res);

        // if !check_node.is_empty() {
        //     if !last_res.is_empty() {
        //         let filtered_node_info_vec: Vec<NodeInfo> = last_res
        //             .into_iter()
        //             .filter(|node_info| {
        //                 check_node
        //                     .iter()
        //                     .any(|address| address == &node_info.node_address)
        //             })
        //             .collect();
        //         tracing::warn!(
        //             "deposit_node_list nodeCheck filtered_node_info_vec:{:?}",
        //             filtered_node_info_vec
        //         );
        //         return Ok(filtered_node_info_vec);
        //     }
        // }

        Ok(last_res)
    }

    async fn call(&self, request: TransactionRequest) -> Result<Vec<u8>, Web3Error> {
        let mut connection = self.pool.access_storage_tagged("api").await.unwrap();
        let block_args = BlockArgs::new(
            &mut connection,
            api::BlockId::Number(api::BlockNumber::Latest),
        )
        .await
        .map_err(|err| internal_error("system_contract_caller", err))?
        .ok_or(Web3Error::NoBlock)?;

        let tx = L2Tx::from_request(request.into(), USED_BOOTLOADER_MEMORY_BYTES)?;

        let vm_permit = self.vm_concurrency_limiter.acquire().await;
        let vm_permit = vm_permit.ok_or(Web3Error::InternalError)?;

        let vm_result = execute_tx_eth_call(
            vm_permit,
            self.shared_args(),
            self.pool.clone(),
            tx,
            block_args,
            self.vm_execution_cache_misses_limit,
            vec![],
        )
        .await;
        let res = match vm_result.result {
            ExecutionResult::Success { output } => output,
            ExecutionResult::Revert { output } => output.encoded_data(),
            ExecutionResult::Halt { .. } => vec![],
        };
        Ok(res)
    }

    fn shared_args(&self) -> TxSharedArgs {
        TxSharedArgs {
            operator_account: AccountTreeId::default(),
            l1_gas_price: self.fair_l2_gas_price,
            fair_l2_gas_price: self.fair_l2_gas_price,
            base_system_contracts: self.api_contracts.eth_call.clone(),
            caches: self.storage_caches.clone(),
            validation_computational_gas_limit: BLOCK_GAS_LIMIT,
            chain_id: self.chain_id,
        }
    }
}
