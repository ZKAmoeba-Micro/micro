//! This module aims to provide a genesis setup for the Micro network.
//! It initializes the Merkle tree with the basic setup (such as fields of special service accounts),
//! setups the required databases, and outputs the data required to initialize a smart contract.

use crate::sync_layer::genesis::fetch_base_system_contracts;
use tempfile::TempDir;
use vm::zk_evm::aux_structures::{LogQuery, Timestamp};

use micro_contracts::{BaseSystemContracts, BaseSystemContractsHashes};
use micro_dal::StorageProcessor;
use micro_merkle_tree::MicroTree;
use micro_storage::db::Database;
use micro_storage::RocksDB;
use micro_types::{
    block::DeployedContract,
    block::{BlockGasCount, L1BatchHeader, MiniblockHeader},
    commitment::{BlockCommitment, BlockMetadata},
    get_code_key, get_system_context_init_logs,
    system_contracts::get_system_smart_contracts,
    tokens::{TokenInfo, TokenMetadata, ETHEREUM_ADDRESS},
    zkevm_test_harness::witness::sort_storage_access::sort_storage_access_queries,
    Address, L1BatchNumber, L2ChainId, MiniblockNumber, StorageLog, StorageLogKind, H256,
};
use micro_utils::{be_words_to_bytes, bytecode::hash_bytecode, h256_to_u256, miniblock_hash};
use micro_web3_decl::{jsonrpsee::http_client::HttpClientBuilder, namespaces::ZksNamespaceClient};

#[derive(Debug, Clone)]
pub enum GenesisParams {
    MainNode {
        first_validator: Address,
    },
    ExternalNode {
        main_node_url: String,
        base_system_contracts_hashes: BaseSystemContractsHashes,
    },
}

pub async fn ensure_genesis_state(
    storage: &mut StorageProcessor<'_>,
    micro_chain_id: L2ChainId,
    genesis_params: GenesisParams,
) -> H256 {
    let temp_dir = TempDir::new().expect("failed get temporary directory for RocksDB");
    let db = RocksDB::new(Database::MerkleTree, temp_dir.as_ref(), false);
    let mut tree = MicroTree::new(db);

    let mut transaction = storage.start_transaction_blocking();

    // return if genesis block was already processed
    if !transaction.blocks_dal().is_genesis_needed() {
        vlog::debug!("genesis is not needed!");
        return transaction
            .blocks_dal()
            .get_block_state_root(L1BatchNumber(0))
            .expect("genesis block hash is empty");
    }

    vlog::info!("running regenesis");

    // If running main node, load base system contracts from disk.
    // If running external node, request contracts and first validator address from the main node.
    let (base_system_contracts, first_validator_address) = match genesis_params {
        GenesisParams::ExternalNode {
            main_node_url,
            base_system_contracts_hashes,
        } => {
            if main_node_url.contains("mainnet") || main_node_url.contains("testnet") {
                panic!("EN is still WIP. Can't run genesis for mainnet/testnet");
            }

            // These have to be *initial* base contract hashes of main node
            // (those that were used during genesis), not necessarily the current ones.
            let contracts =
                fetch_base_system_contracts(&main_node_url, base_system_contracts_hashes)
                    .await
                    .expect("Failed to fetch base system contracts from main node");

            let client = HttpClientBuilder::default().build(main_node_url).unwrap();
            let first_validator = client
                .get_block_details(MiniblockNumber(0))
                .await
                .ok()
                .flatten()
                .expect("Failed to fetch genesis miniblock")
                .operator_address;

            (contracts, first_validator)
        }
        GenesisParams::MainNode { first_validator } => {
            (BaseSystemContracts::load_from_disk(), first_validator)
        }
    };

    let base_system_contracts_hashes = base_system_contracts.hashes();

    create_genesis_block(
        &mut transaction,
        first_validator_address,
        micro_chain_id,
        base_system_contracts,
    );
    vlog::info!("chain_schema_genesis is complete");

    let storage_logs =
        crate::metadata_calculator::get_logs_for_l1_batch(&mut transaction, L1BatchNumber(0));
    let metadata = tree.process_block(storage_logs.unwrap().storage_logs);
    let genesis_root_hash = metadata.root_hash;
    let rollup_last_leaf_index = metadata.rollup_last_leaf_index;

    let block_commitment = BlockCommitment::new(
        vec![],
        rollup_last_leaf_index,
        genesis_root_hash,
        vec![],
        vec![],
        base_system_contracts_hashes.bootloader,
        base_system_contracts_hashes.default_aa,
    );

    save_genesis_block_metadata(
        &mut transaction,
        &block_commitment,
        genesis_root_hash,
        rollup_last_leaf_index,
    );
    vlog::info!("operations_schema_genesis is complete");

    transaction.commit_blocking();

    // We need to `println` this value because it will be used to initialize the smart contract.
    println!("CONTRACTS_GENESIS_ROOT={:?}", genesis_root_hash);
    println!(
        "CONTRACTS_GENESIS_BLOCK_COMMITMENT={:?}",
        block_commitment.hash().commitment
    );
    println!(
        "CONTRACTS_GENESIS_ROLLUP_LEAF_INDEX={}",
        rollup_last_leaf_index
    );
    println!(
        "CHAIN_STATE_KEEPER_BOOTLOADER_HASH={:?}",
        base_system_contracts_hashes.bootloader
    );
    println!(
        "CHAIN_STATE_KEEPER_DEFAULT_AA_HASH={:?}",
        base_system_contracts_hashes.default_aa
    );

    genesis_root_hash
}

// Default account and bootloader are not a regular system contracts
// they have never been actually deployed anywhere,
// They are the initial code that is fed into the VM upon its start.
// Both are rather parameters of a block and not system contracts.
// The code of the bootloader should not be deployed anywhere anywhere in the kernel space (i.e. addresses below 2^16)
// because in this case we will have to worry about protecting it.
fn insert_base_system_contracts_to_factory_deps(
    storage: &mut StorageProcessor<'_>,
    contracts: BaseSystemContracts,
) {
    let factory_deps = vec![contracts.bootloader, contracts.default_aa]
        .iter()
        .map(|c| (c.hash, be_words_to_bytes(&c.code)))
        .collect();

    storage
        .storage_dal()
        .insert_factory_deps(MiniblockNumber(0), factory_deps);
}

fn insert_system_contracts(
    storage: &mut StorageProcessor<'_>,
    contracts: Vec<DeployedContract>,
    chain_id: L2ChainId,
) {
    let system_context_init_logs = (H256::default(), get_system_context_init_logs(chain_id));

    let storage_logs: Vec<(H256, Vec<StorageLog>)> = contracts
        .clone()
        .into_iter()
        .map(|contract| {
            let hash = hash_bytecode(&contract.bytecode);
            let code_key = get_code_key(contract.account_id.address());

            (
                Default::default(),
                vec![StorageLog::new_write_log(code_key, hash)],
            )
        })
        .chain(Some(system_context_init_logs))
        .collect();

    let mut transaction = storage.start_transaction_blocking();

    transaction
        .storage_logs_dal()
        .insert_storage_logs(MiniblockNumber(0), &storage_logs);

    // we don't produce proof for the genesis block,
    // but we still need to populate the table
    // to have the correct initial state of the merkle tree
    let log_queries: Vec<LogQuery> = storage_logs
        .iter()
        .enumerate()
        .flat_map(|(tx_index, (_, storage_logs))| {
            storage_logs
                .iter()
                .enumerate()
                .map(move |(log_index, storage_log)| {
                    LogQuery {
                        // Monotonically increasing Timestamp. Normally it's generated by the VM, but we don't have a VM in the genesis block.
                        timestamp: Timestamp(((tx_index << 16) + log_index) as u32),
                        tx_number_in_block: tx_index as u16,
                        aux_byte: 0,
                        shard_id: 0,
                        address: *storage_log.key.address(),
                        key: h256_to_u256(*storage_log.key.key()),
                        read_value: h256_to_u256(H256::zero()),
                        written_value: h256_to_u256(storage_log.value),
                        rw_flag: storage_log.kind == StorageLogKind::Write,
                        rollback: false,
                        is_service: false,
                    }
                })
                .collect::<Vec<LogQuery>>()
        })
        .collect();

    let (_, deduped_log_queries) = sort_storage_access_queries(&log_queries);

    let (deduplicated_writes, protective_reads): (Vec<_>, Vec<_>) = deduped_log_queries
        .into_iter()
        .partition(|log_query| log_query.rw_flag);
    transaction
        .storage_logs_dedup_dal()
        .insert_protective_reads(L1BatchNumber(0), &protective_reads);
    transaction
        .storage_logs_dedup_dal()
        .insert_initial_writes(L1BatchNumber(0), &deduplicated_writes);

    transaction.storage_dal().apply_storage_logs(&storage_logs);

    let factory_deps = contracts
        .into_iter()
        .map(|c| (hash_bytecode(&c.bytecode), c.bytecode))
        .collect();
    transaction
        .storage_dal()
        .insert_factory_deps(MiniblockNumber(0), factory_deps);

    transaction.commit_blocking();
}

pub(crate) fn create_genesis_block(
    storage: &mut StorageProcessor<'_>,
    first_validator_address: Address,
    chain_id: L2ChainId,
    base_system_contracts: BaseSystemContracts,
) {
    let mut zero_block_header = L1BatchHeader::new(
        L1BatchNumber(0),
        0,
        first_validator_address,
        base_system_contracts.hashes(),
    );
    zero_block_header.is_finished = true;

    let zero_miniblock_header = MiniblockHeader {
        number: MiniblockNumber(0),
        timestamp: 0,
        hash: miniblock_hash(MiniblockNumber(0)),
        l1_tx_count: 0,
        l2_tx_count: 0,
        base_fee_per_gas: 0,
        l1_gas_price: 0,
        l2_fair_gas_price: 0,
        base_system_contracts_hashes: base_system_contracts.hashes(),
    };

    let mut transaction = storage.start_transaction_blocking();

    transaction
        .blocks_dal()
        .insert_l1_batch(zero_block_header, BlockGasCount::default());
    transaction
        .blocks_dal()
        .insert_miniblock(zero_miniblock_header);
    transaction
        .blocks_dal()
        .mark_miniblocks_as_executed_in_l1_batch(L1BatchNumber(0));

    insert_base_system_contracts_to_factory_deps(&mut transaction, base_system_contracts);

    let contracts = get_system_smart_contracts();
    insert_system_contracts(&mut transaction, contracts, chain_id);

    add_eth_token(&mut transaction);

    transaction.commit_blocking();
}

pub(crate) fn add_eth_token(storage: &mut StorageProcessor<'_>) {
    let eth_token = TokenInfo {
        l1_address: ETHEREUM_ADDRESS,
        l2_address: ETHEREUM_ADDRESS,
        metadata: TokenMetadata {
            name: "FileCoin".to_string(),
            symbol: "FIL".to_string(),
            decimals: 18,
        },
    };

    let mut transaction = storage.start_transaction_blocking();

    transaction.tokens_dal().add_tokens(vec![eth_token.clone()]);
    transaction
        .tokens_dal()
        .update_well_known_l1_token(&ETHEREUM_ADDRESS, eth_token.metadata);

    transaction.commit_blocking();
}

pub(crate) fn save_genesis_block_metadata(
    storage: &mut StorageProcessor<'_>,
    block_commitment: &BlockCommitment,
    genesis_root_hash: H256,
    rollup_last_leaf_index: u64,
) {
    let block_commitment_hash = block_commitment.hash();

    let metadata = BlockMetadata {
        root_hash: genesis_root_hash,
        rollup_last_leaf_index,
        merkle_root_hash: genesis_root_hash,
        initial_writes_compressed: vec![],
        repeated_writes_compressed: vec![],
        commitment: block_commitment_hash.commitment,
        l2_l1_messages_compressed: vec![],
        l2_l1_merkle_root: Default::default(),
        block_meta_params: block_commitment.meta_parameters(),
        aux_data_hash: block_commitment_hash.aux_output,
        meta_parameters_hash: block_commitment_hash.meta_parameters,
        pass_through_data_hash: block_commitment_hash.pass_through_data,
    };
    storage
        .blocks_dal()
        .save_block_metadata(L1BatchNumber(0), metadata);
}
