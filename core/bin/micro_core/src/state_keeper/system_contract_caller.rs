use std::convert::{TryFrom, TryInto};

use micro_contracts::{BaseSystemContracts, SystemContractCode, PLAYGROUND_BLOCK_BOOTLOADER_CODE};
use micro_dal::ConnectionPool;
use micro_types::{
    api,
    ethabi::Contract,
    l2::system_contracts::{NodeInfo, NodeInfoList},
    transaction_request::TransactionRequest,
    DEPOSIT_ADDRESS, H256,
};
use micro_utils::bytes_to_be_words;
use micro_web3_decl::error::Web3Error;

use crate::api_server::execution_sandbox::execute_tx_eth_call;
use crate::node_check::get_all_addresses;

#[derive(Debug)]
pub struct SystemContractCaller {
    pool: ConnectionPool,
    fair_l2_gas_price: u64,
    vm_execution_cache_misses_limit: Option<usize>,
    playground_base_system_contracts: BaseSystemContracts,

    deposit_abi: Contract,
}

impl SystemContractCaller {
    pub fn new(
        connection_pool: ConnectionPool,
        fair_l2_gas_price: u64,
        default_aa_hash: H256,
        vm_execution_cache_misses_limit: Option<usize>,
    ) -> SystemContractCaller {
        let mut storage = connection_pool.access_storage_blocking();
        let default_aa_bytecode = storage
            .storage_dal()
            .get_factory_dep(default_aa_hash)
            .expect("Default AA hash must be present in the database");
        drop(storage);

        let default_aa_contract = SystemContractCode {
            code: bytes_to_be_words(default_aa_bytecode),
            hash: default_aa_hash,
        };

        let playground_base_system_contracts = BaseSystemContracts {
            default_aa: default_aa_contract.clone(),
            bootloader: PLAYGROUND_BLOCK_BOOTLOADER_CODE.clone(),
        };

        SystemContractCaller {
            pool: connection_pool,
            fair_l2_gas_price,
            vm_execution_cache_misses_limit,
            playground_base_system_contracts,

            deposit_abi: micro_contracts::sys_deposit_contract(),
        }
    }

    pub fn deposit_node_list(&self) -> Result<Vec<NodeInfo>, Web3Error> {
        let func = self.deposit_abi.function("getNodeInfo")?;

        let mut tx: TransactionRequest = TransactionRequest::default();
        tx.to = Some(DEPOSIT_ADDRESS);
        tx.input = func.encode_input(&[])?.into();

        let result = self.call(tx)?;

        // parse result
        let res = func.decode_output(&result)?;
        let res = NodeInfoList::try_from(res)?;
        let last_res = res.list;
        let check_node = get_all_addresses();
        vlog::warn!(
            "deposit_node_list nodeCheck deposit_node_list:{:?}",
            check_node
        );
        vlog::warn!("deposit_node_list nodeCheck last_res:{:?}", last_res);

        if !check_node.is_empty() {
            if !last_res.is_empty() {
                let filtered_node_info_vec: Vec<NodeInfo> = last_res
                    .into_iter()
                    .filter(|node_info| {
                        check_node
                            .iter()
                            .any(|address| address == &node_info.node_address)
                    })
                    .collect();
                vlog::warn!(
                    "deposit_node_list nodeCheck filtered_node_info_vec:{:?}",
                    filtered_node_info_vec
                );
                return Ok(filtered_node_info_vec);
            }
        }

        Ok(last_res)
    }

    fn call(&self, tx: TransactionRequest) -> Result<Vec<u8>, Web3Error> {
        let result = execute_tx_eth_call(
            &self.pool,
            tx.try_into()?,
            api::BlockId::Number(api::BlockNumber::Latest),
            self.fair_l2_gas_price,
            self.fair_l2_gas_price,
            Some(self.fair_l2_gas_price),
            &self.playground_base_system_contracts,
            self.vm_execution_cache_misses_limit,
            false,
        )?;
        let res = match result.revert_reason {
            Some(result) => result.original_data,
            None => result
                .return_data
                .into_iter()
                .flat_map(|val| {
                    let bytes: [u8; 32] = val.into();
                    bytes.to_vec()
                })
                .collect::<Vec<_>>(),
        };
        Ok(res)
    }
}
