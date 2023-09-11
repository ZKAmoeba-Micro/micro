//! Domain-specific tests. Taken almost verbatim from the previous tree implementation.

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_with::{hex::Hex, serde_as};
use tempfile::TempDir;

use std::{fs::File, io::BufReader, num::NonZeroU32, slice};

use micro_config::constants::ACCOUNT_CODE_STORAGE_ADDRESS;
use micro_crypto::hasher::blake2::Blake2Hasher;
use micro_merkle_tree2::domain::MicroTree;
use micro_merkle_tree2::HashTree;
use micro_storage::db::Database;
use micro_storage::RocksDB;
use micro_types::{
    proofs::StorageLogMetadata, AccountTreeId, Address, L1BatchNumber, StorageKey, StorageLog,
    WitnessStorageLog, H256,
};
use micro_utils::u32_to_h256;

fn gen_storage_logs() -> Vec<WitnessStorageLog> {
    let addrs = vec![
        "4b3af74f66ab1f0da3f2e4ec7a3cb99baf1af7b2",
        "ef4bb7b21c5fe7432a7d63876cc59ecc23b46636",
        "89b8988a018f5348f52eeac77155a793adf03ecc",
        "782806db027c08d36b2bed376b4271d1237626b3",
        "b2b57b76717ee02ae1327cc3cf1f40e76f692311",
    ]
    .into_iter()
    .map(|s| s.parse::<Address>().unwrap());

    let proof_keys = addrs.flat_map(|addr| {
        (0..20).map(move |i| StorageKey::new(AccountTreeId::new(addr), u32_to_h256(i)))
    });
    let proof_values = (0..100).map(u32_to_h256);

    proof_keys
        .zip(proof_values)
        .map(|(proof_key, proof_value)| {
            let storage_log = StorageLog::new_write_log(proof_key, proof_value);
            WitnessStorageLog {
                storage_log,
                previous_value: H256::zero(),
            }
        })
        .collect()
}

fn convert_logs(logs: impl Iterator<Item = StorageLog>) -> Vec<WitnessStorageLog> {
    logs.map(|storage_log| WitnessStorageLog {
        storage_log,
        previous_value: H256::zero(),
    })
    .collect()
}

#[test]
fn basic_workflow() {
    let temp_dir = TempDir::new().expect("failed get temporary directory for RocksDB");
    let logs = gen_storage_logs();
    let block_number = NonZeroU32::new(1).unwrap();

    let (metadata, expected_root_hash) = {
        let db = RocksDB::new(Database::MerkleTree, temp_dir.as_ref(), false);
        let mut tree = MicroTree::new_lightweight(db);
        let metadata = tree.process_block(&logs);
        tree.save();
        tree.verify_consistency(block_number);
        (metadata, tree.root_hash())
    };

    assert_eq!(metadata.root_hash, expected_root_hash);
    assert_eq!(metadata.rollup_last_leaf_index, 101);
    assert_eq!(metadata.initial_writes.len(), logs.len());
    for (write, log) in metadata.initial_writes.iter().zip(&logs) {
        assert_eq!(write.value, log.storage_log.value);
    }
    assert!(metadata.repeated_writes.is_empty());

    assert_eq!(
        expected_root_hash,
        H256([
            125, 25, 107, 171, 182, 155, 32, 70, 138, 108, 238, 150, 140, 205, 193, 39, 90, 92,
            122, 233, 118, 238, 248, 201, 160, 55, 58, 206, 244, 216, 188, 10
        ]),
    );

    let db = RocksDB::new(Database::MerkleTree, temp_dir.as_ref(), false);
    let tree = MicroTree::new_lightweight(db);
    tree.verify_consistency(block_number);
    assert_eq!(tree.root_hash(), expected_root_hash);
    assert_eq!(tree.block_number(), block_number.get());
}

#[test]
fn basic_workflow_multiblock() {
    let temp_dir = TempDir::new().expect("failed get temporary directory for RocksDB");
    let logs = gen_storage_logs();
    let blocks = logs.chunks(9);

    let expected_root_hash = {
        let db = RocksDB::new(Database::MerkleTree, temp_dir.as_ref(), false);
        let mut tree = MicroTree::new_lightweight(db);
        tree.use_dedicated_thread_pool(2);
        for block in blocks {
            tree.process_block(block);
        }
        tree.save();
        tree.root_hash()
    };

    assert_eq!(
        expected_root_hash,
        H256([
            125, 25, 107, 171, 182, 155, 32, 70, 138, 108, 238, 150, 140, 205, 193, 39, 90, 92,
            122, 233, 118, 238, 248, 201, 160, 55, 58, 206, 244, 216, 188, 10
        ]),
    );

    let db = RocksDB::new(Database::MerkleTree, temp_dir.as_ref(), false);
    let tree = MicroTree::new_lightweight(db);
    assert_eq!(tree.root_hash(), expected_root_hash);
    assert_eq!(tree.block_number(), 12);
}

#[test]
fn revert_blocks() {
    let temp_dir = TempDir::new().expect("failed get temporary directory for RocksDB");
    let storage = RocksDB::new(Database::MerkleTree, temp_dir.as_ref(), false);

    // Generate logs and save them to DB.
    // Produce 4 blocks with distinct values and 1 block with modified values from first block
    let block_size: usize = 25;
    let address: Address = "4b3af74f66ab1f0da3f2e4ec7a3cb99baf1af7b2".parse().unwrap();
    let proof_keys =
        (0..100).map(move |i| StorageKey::new(AccountTreeId::new(address), u32_to_h256(i)));
    let proof_values = (0..100).map(u32_to_h256);

    // Add couple of blocks of distinct keys/values
    let mut logs: Vec<_> = convert_logs(
        proof_keys
            .zip(proof_values)
            .map(|(proof_key, proof_value)| StorageLog::new_write_log(proof_key, proof_value)),
    );
    // Add a block with repeated keys
    let mut extra_logs = convert_logs((0..block_size).map(move |i| {
        StorageLog::new_write_log(
            StorageKey::new(AccountTreeId::new(address), u32_to_h256(i as u32)),
            u32_to_h256((i + 1) as u32),
        )
    }));
    logs.append(&mut extra_logs);

    let mirror_logs = logs.clone();
    let tree_metadata: Vec<_> = {
        let mut tree = MicroTree::new_lightweight(storage);
        let metadata = logs.chunks(block_size).map(|chunk| {
            let metadata = tree.process_block(chunk);
            tree.save();
            metadata
        });
        metadata.collect()
    };

    assert_eq!(tree_metadata.len(), 5);
    // 4 first blocks must contain only insert ops, while the last one must contain
    // only the update ops.
    for (i, metadata) in tree_metadata.iter().enumerate() {
        let expected_leaf_index = if i == 4 {
            assert!(metadata.initial_writes.is_empty());
            assert_eq!(metadata.repeated_writes.len(), block_size);
            for (write, idx) in metadata.repeated_writes.iter().zip(1_u64..) {
                assert_eq!(write.index, idx);
                assert_eq!(write.value, H256::from_low_u64_be(idx));
            }
            block_size * 4 + 1
        } else {
            assert!(metadata.repeated_writes.is_empty());
            assert_eq!(metadata.initial_writes.len(), block_size);
            block_size * (i + 1) + 1
        };
        assert_eq!(metadata.rollup_last_leaf_index, expected_leaf_index as u64);
    }

    // Revert the last block.
    let storage = RocksDB::new(Database::MerkleTree, temp_dir.as_ref(), false);
    {
        let mut tree = MicroTree::new_lightweight(storage);
        assert_eq!(tree.root_hash(), tree_metadata.last().unwrap().root_hash);
        tree.revert_logs(L1BatchNumber(3));
        assert_eq!(tree.root_hash(), tree_metadata[3].root_hash);
        tree.save();
    }

    // Revert two more blocks.
    let storage = RocksDB::new(Database::MerkleTree, temp_dir.as_ref(), false);
    {
        let mut tree = MicroTree::new_lightweight(storage);
        tree.revert_logs(L1BatchNumber(1));
        assert_eq!(tree.root_hash(), tree_metadata[1].root_hash);
        tree.save();
    }

    // Revert two more blocks second time; the result should be the same
    let storage = RocksDB::new(Database::MerkleTree, temp_dir.as_ref(), false);
    {
        let mut tree = MicroTree::new_lightweight(storage);
        tree.revert_logs(L1BatchNumber(1));
        assert_eq!(tree.root_hash(), tree_metadata[1].root_hash);
        tree.save();
    }

    // Reapply one of the reverted logs
    let storage = RocksDB::new(Database::MerkleTree, temp_dir.as_ref(), false);
    {
        let storage_log = mirror_logs.get(3 * block_size).unwrap();
        let mut tree = MicroTree::new_lightweight(storage);
        tree.process_block(slice::from_ref(storage_log));
        tree.save();
    }

    // check saved block number
    let storage = RocksDB::new(Database::MerkleTree, temp_dir.as_ref(), false);
    let tree = MicroTree::new_lightweight(storage);
    assert_eq!(tree.block_number(), 3);
}

#[test]
fn reset_tree() {
    let temp_dir = TempDir::new().expect("failed get temporary directory for RocksDB");
    let storage = RocksDB::new(Database::MerkleTree, temp_dir.as_ref(), false);
    let logs = gen_storage_logs();
    let mut tree = MicroTree::new_lightweight(storage);
    let empty_root_hash = tree.root_hash();

    logs.chunks(5)
        .into_iter()
        .fold(empty_root_hash, |hash, chunk| {
            tree.process_block(chunk);
            tree.reset();
            assert_eq!(tree.root_hash(), hash);

            tree.process_block(chunk);
            tree.save();
            tree.root_hash()
        });
}

#[test]
fn read_logs() {
    let temp_dir = TempDir::new().expect("failed get temporary directory for RocksDB");
    let mut logs = gen_storage_logs();
    logs.truncate(5);

    let write_metadata = {
        let db = RocksDB::new(Database::MerkleTree, temp_dir.as_ref(), false);
        let mut tree = MicroTree::new_lightweight(db);
        let metadata = tree.process_block(&logs);
        tree.save();
        metadata
    };

    let db = RocksDB::new(Database::MerkleTree, temp_dir.as_ref(), false);
    let mut tree = MicroTree::new_lightweight(db);
    let read_logs = logs
        .into_iter()
        .map(|log| StorageLog::new_read_log(log.storage_log.key, log.storage_log.value));
    let read_metadata = tree.process_block(&convert_logs(read_logs));

    assert_eq!(read_metadata.root_hash, write_metadata.root_hash);
}

fn create_write_log(
    address: Address,
    address_storage_key: [u8; 32],
    value: [u8; 32],
) -> WitnessStorageLog {
    WitnessStorageLog {
        storage_log: StorageLog::new_write_log(
            StorageKey::new(AccountTreeId::new(address), H256(address_storage_key)),
            H256(value),
        ),
        previous_value: H256::zero(),
    }
}

fn subtract_from_max_value(diff: u8) -> [u8; 32] {
    let mut value = [255_u8; 32];
    value[31] -= diff;
    value
}

#[test]
fn root_hash_compatibility() {
    let temp_dir = TempDir::new().expect("failed get temporary directory for RocksDB");
    let db = RocksDB::new(Database::MerkleTree, temp_dir.as_ref(), false);
    let mut tree = MicroTree::new_lightweight(db);
    assert_eq!(
        tree.root_hash(),
        H256([
            152, 164, 142, 78, 209, 115, 97, 136, 56, 74, 232, 167, 157, 210, 28, 77, 102, 135,
            229, 253, 34, 202, 24, 20, 137, 6, 215, 135, 54, 192, 216, 106
        ]),
    );

    let storage_logs = vec![
        create_write_log(ACCOUNT_CODE_STORAGE_ADDRESS, [0; 32], [1; 32]),
        create_write_log(
            Address::from_low_u64_be(9223372036854775808),
            [254; 32],
            subtract_from_max_value(1),
        ),
        create_write_log(
            Address::from_low_u64_be(9223372036854775809),
            [253; 32],
            subtract_from_max_value(2),
        ),
        create_write_log(
            Address::from_low_u64_be(9223372036854775810),
            [252; 32],
            subtract_from_max_value(3),
        ),
        create_write_log(
            Address::from_low_u64_be(9223372036854775811),
            [251; 32],
            subtract_from_max_value(4),
        ),
        create_write_log(
            Address::from_low_u64_be(9223372036854775812),
            [250; 32],
            subtract_from_max_value(5),
        ),
    ];

    let metadata = tree.process_block(&storage_logs);
    assert_eq!(
        metadata.root_hash,
        H256([
            35, 191, 235, 50, 17, 223, 143, 160, 240, 38, 139, 111, 221, 156, 42, 29, 72, 90, 196,
            198, 72, 13, 219, 88, 59, 250, 94, 112, 221, 3, 44, 171
        ])
    );
}

#[test]
fn process_block_idempotency_check() {
    let temp_dir = TempDir::new().expect("failed to get temporary directory for RocksDB");
    let rocks_db = RocksDB::new(Database::MerkleTree, temp_dir.as_ref(), false);
    let mut tree = MicroTree::new_lightweight(rocks_db);
    let logs = gen_storage_logs();
    let tree_metadata = tree.process_block(&logs);

    // Simulate server restart by calling `process_block` again on the same tree
    tree.reset();
    let repeated_tree_metadata = tree.process_block(&logs);
    assert_eq!(repeated_tree_metadata.root_hash, tree_metadata.root_hash);
    assert_eq!(
        repeated_tree_metadata.initial_writes,
        tree_metadata.initial_writes
    );
    assert_eq!(
        repeated_tree_metadata.repeated_writes,
        tree_metadata.repeated_writes
    );
}

/// Serializable version of [`StorageLogMetadata`] that hex-encodes byte arrays.
#[serde_as]
#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct StorageLogMetadataSnapshot {
    #[serde_as(as = "Hex")]
    root_hash: [u8; 32],
    is_write: bool,
    first_write: bool,
    #[serde_as(as = "Vec<Hex>")]
    merkle_paths: Vec<[u8; 32]>,
    #[serde_as(as = "Hex")]
    leaf_hashed_key: [u8; 32],
    leaf_enumeration_index: u64,
    #[serde_as(as = "Hex")]
    value_written: [u8; 32],
    #[serde_as(as = "Hex")]
    value_read: [u8; 32],
}

impl From<StorageLogMetadata> for StorageLogMetadataSnapshot {
    fn from(metadata: StorageLogMetadata) -> Self {
        let mut leaf_hashed_key = [0_u8; 32];
        metadata.leaf_hashed_key.to_big_endian(&mut leaf_hashed_key);

        Self {
            root_hash: metadata.root_hash,
            is_write: metadata.is_write,
            first_write: metadata.first_write,
            merkle_paths: metadata.merkle_paths,
            leaf_hashed_key,
            leaf_enumeration_index: metadata.leaf_enumeration_index,
            value_written: metadata.value_written,
            value_read: metadata.value_read,
        }
    }
}

fn load_json<T: DeserializeOwned>(path: &str) -> T {
    let file = File::open(path).unwrap_or_else(|err| {
        panic!("Cannot open snapshot at `{path}`: {err}");
    });
    serde_json::from_reader(BufReader::new(file)).expect("cannot deserialize snapshot")
}

// Snapshots are taken from the old tree implementation.
#[test]
fn witness_workflow() {
    let temp_dir = TempDir::new().expect("failed get temporary directory for RocksDB");
    let logs = gen_storage_logs();
    let (first_chunk, _) = logs.split_at(logs.len() / 2);

    let db = RocksDB::new(Database::MerkleTree, temp_dir.as_ref(), false);
    let mut tree = MicroTree::new(db);
    let metadata = tree.process_block(first_chunk);
    let job = metadata.witness.unwrap();
    assert_eq!(job.next_enumeration_index(), 1);
    let merkle_paths: Vec<_> = job.into_merkle_paths().collect();

    assert!(merkle_paths.iter().all(|log| log.merkle_paths.len() == 256));

    let mut witnesses: Vec<_> = merkle_paths
        .into_iter()
        .map(StorageLogMetadataSnapshot::from)
        .collect();

    // The working dir for integration tests is set to the crate dir, so specifying relative paths
    // should be OK.
    let last_witness: StorageLogMetadataSnapshot =
        load_json("./tests/integration/snapshots/log-metadata-full.json");
    assert_eq!(*witnesses.last().unwrap(), last_witness);

    for witness in &mut witnesses {
        // Leave only 8 hashes closest to the tree root
        witness.merkle_paths = witness.merkle_paths.split_off(248);
    }
    let truncated_witnesses: Vec<StorageLogMetadataSnapshot> =
        load_json("./tests/integration/snapshots/log-metadata-list-short.json");
    assert_eq!(witnesses, truncated_witnesses);
}

#[test]
fn witnesses_with_multiple_blocks() {
    let temp_dir = TempDir::new().expect("failed get temporary directory for RocksDB");
    let logs = gen_storage_logs();

    let db = RocksDB::new(Database::MerkleTree, temp_dir.as_ref(), false);
    let mut tree = MicroTree::new(db);
    let empty_tree_hashes: Vec<_> = (0..256)
        .map(|i| Blake2Hasher.empty_subtree_hash(i))
        .collect();

    let non_empty_levels_by_block = logs.chunks(10).map(|block| {
        let metadata = tree.process_block(block);
        let witness = metadata.witness.unwrap();

        let non_empty_levels = witness.into_merkle_paths().map(|log| {
            assert_eq!(log.merkle_paths.len(), 256);
            log.merkle_paths
                .iter()
                .zip(&empty_tree_hashes)
                .position(|(hash, empty_hash)| *hash != empty_hash.0)
                .unwrap_or(256)
        });
        non_empty_levels.collect::<Vec<_>>()
    });
    let non_empty_levels_by_block: Vec<_> = non_empty_levels_by_block.collect();

    // For at least some blocks, the non-empty level for the first log can be lower
    // than for the following logs (meaning that additional empty subtree hashes
    // are included in the Merkle path for the following log).
    let has_following_log_with_greater_level =
        non_empty_levels_by_block.iter().any(|block_levels| {
            block_levels[1..]
                .iter()
                .any(|&level| level > block_levels[0])
        });
    assert!(
        has_following_log_with_greater_level,
        "{non_empty_levels_by_block:?}"
    );
}
