use crate::models::storage_contract::StorageContractSource;
use crate::StorageProcessor;
use micro_contracts::{BaseSystemContracts, SystemContractCode};
use micro_types::{
    vm_trace::ContractSourceDebugInfo, Address, MiniblockNumber, StorageKey, StorageLog,
    StorageValue, H256, U256,
};
use micro_utils::{bytes_to_be_words, bytes_to_chunks};
use std::collections::{HashMap, HashSet};
use std::time::Instant;

#[derive(Debug)]
pub struct StorageDal<'a, 'c> {
    pub storage: &'a mut StorageProcessor<'c>,
}

impl StorageDal<'_, '_> {
    pub fn insert_factory_deps(
        &mut self,
        block_number: MiniblockNumber,
        factory_deps: HashMap<H256, Vec<u8>>,
    ) {
        async_std::task::block_on(async {
            let (bytecode_hashes, bytecodes): (Vec<_>, Vec<_>) = factory_deps
                .into_iter()
                .map(|dep| (dep.0.as_bytes().into(), dep.1))
                .unzip();

            // Copy from stdin can't be used here because of 'ON CONFLICT'.
            sqlx::query!(
                "INSERT INTO factory_deps
                (bytecode_hash, bytecode, miniblock_number, created_at, updated_at)
            SELECT u.bytecode_hash, u.bytecode, $3, now(), now()
            FROM UNNEST($1::bytea[], $2::bytea[])
                AS u(bytecode_hash, bytecode)
            ON CONFLICT (bytecode_hash) DO NOTHING
            ",
                &bytecode_hashes,
                &bytecodes,
                block_number.0 as i64,
            )
            .execute(self.storage.conn())
            .await
            .unwrap();
        })
    }

    pub fn get_factory_dep(&mut self, hash: H256) -> Option<Vec<u8>> {
        async_std::task::block_on(async {
            sqlx::query!(
                "SELECT bytecode FROM factory_deps WHERE bytecode_hash = $1",
                &hash.0.to_vec(),
            )
            .fetch_optional(self.storage.conn())
            .await
            .unwrap()
            .map(|row| row.bytecode)
        })
    }

    pub fn get_base_system_contracts(
        &mut self,
        bootloader_hash: H256,
        default_aa_hash: H256,
    ) -> BaseSystemContracts {
        async_std::task::block_on(async {
            let bootloader_bytecode = self
                .get_factory_dep(bootloader_hash)
                .expect("Bootloader code should be present in the database");
            let bootloader_code = SystemContractCode {
                code: bytes_to_be_words(bootloader_bytecode),
                hash: bootloader_hash,
            };

            let default_aa_bytecode = self
                .get_factory_dep(default_aa_hash)
                .expect("Default account code should be present in the database");

            let default_aa_code = SystemContractCode {
                code: bytes_to_be_words(default_aa_bytecode),
                hash: default_aa_hash,
            };
            BaseSystemContracts {
                bootloader: bootloader_code,
                default_aa: default_aa_code,
            }
        })
    }

    pub fn get_factory_deps(&mut self, hashes: &HashSet<H256>) -> HashMap<U256, Vec<[u8; 32]>> {
        let hashes_as_vec_u8: Vec<Vec<u8>> = hashes.iter().map(|hash| hash.0.to_vec()).collect();

        async_std::task::block_on(async {
            sqlx::query!(
                "SELECT bytecode, bytecode_hash FROM factory_deps WHERE bytecode_hash = ANY($1)",
                &hashes_as_vec_u8,
            )
            .fetch_all(self.storage.conn())
            .await
            .unwrap()
            .into_iter()
            .map(|row| {
                (
                    U256::from_big_endian(&row.bytecode_hash),
                    bytes_to_chunks(&row.bytecode),
                )
            })
            .collect()
        })
    }

    pub fn get_factory_deps_for_revert(&mut self, block_number: MiniblockNumber) -> Vec<H256> {
        async_std::task::block_on(async {
            sqlx::query!(
                "SELECT bytecode_hash FROM factory_deps WHERE miniblock_number > $1",
                block_number.0 as i64
            )
            .fetch_all(self.storage.conn())
            .await
            .unwrap()
            .into_iter()
            .map(|row| H256::from_slice(&row.bytecode_hash))
            .collect()
        })
    }

    pub fn set_contract_source(&mut self, address: Address, source: ContractSourceDebugInfo) {
        async_std::task::block_on(async {
            sqlx::query!(
                "INSERT INTO contract_sources (address, assembly_code, pc_line_mapping, created_at, updated_at)
                VALUES ($1, $2, $3, now(), now())
                ON CONFLICT (address)
                DO UPDATE SET assembly_code = $2, pc_line_mapping = $3, updated_at = now()
                ",
                address.as_bytes(),
                source.assembly_code,
                serde_json::to_value(source.pc_line_mapping).unwrap()
            )
            .execute(self.storage.conn())
            .await
            .unwrap();
        })
    }

    pub fn get_contract_source(&mut self, address: Address) -> Option<ContractSourceDebugInfo> {
        async_std::task::block_on(async {
            let source = sqlx::query_as!(
                StorageContractSource,
                "SELECT assembly_code, pc_line_mapping FROM contract_sources WHERE address = $1",
                address.as_bytes()
            )
            .fetch_optional(self.storage.conn())
            .await
            .unwrap();
            source.map(Into::into)
        })
    }

    // we likely don't need `storage` table at all, as we have `storage_logs` table
    // Returns the list of unique storage updates for block
    pub fn apply_storage_logs(
        &mut self,
        updates: &[(H256, Vec<StorageLog>)],
    ) -> Vec<(StorageKey, (H256, StorageValue))> {
        async_std::task::block_on(async {
            let mut unique_updates: HashMap<StorageKey, (H256, StorageValue)> = HashMap::new();
            for (tx_hash, storage_logs) in updates {
                for storage_log in storage_logs {
                    unique_updates.insert(storage_log.key, (*tx_hash, storage_log.value));
                }
            }
            let unique_updates: Vec<(StorageKey, (H256, StorageValue))> =
                unique_updates.into_iter().collect();

            let hashed_keys: Vec<Vec<u8>> = unique_updates
                .iter()
                .map(|(key, _)| key.hashed_key().0.to_vec())
                .collect();

            let addresses: Vec<_> = unique_updates
                .iter()
                .map(|(key, _)| key.address().0.to_vec())
                .collect();
            let keys: Vec<_> = unique_updates
                .iter()
                .map(|(key, _)| key.key().0.to_vec())
                .collect();
            let values: Vec<Vec<u8>> = unique_updates
                .iter()
                .map(|(_, (_, value))| value.as_bytes().to_vec())
                .collect();

            let tx_hashes: Vec<Vec<u8>> = unique_updates
                .iter()
                .map(|(_, (tx_hash, _))| tx_hash.0.to_vec())
                .collect();

            // Copy from stdin can't be used here because of 'ON CONFLICT'.
            sqlx::query!(
                "INSERT INTO storage (hashed_key, address, key, value, tx_hash, created_at, updated_at)
                SELECT u.hashed_key, u.address, u.key, u.value, u.tx_hash, now(), now()
                    FROM UNNEST ($1::bytea[], $2::bytea[], $3::bytea[], $4::bytea[], $5::bytea[])
                    AS u(hashed_key, address, key, value, tx_hash)
                ON CONFLICT (hashed_key)
                DO UPDATE SET tx_hash = excluded.tx_hash, value = excluded.value, updated_at = now()
                ",
                &hashed_keys,
                &addresses,
                &keys,
                &values,
                &tx_hashes,
            )
            .execute(self.storage.conn())
            .await
            .unwrap();

            unique_updates
        })
    }

    pub fn get_by_key(&mut self, key: &StorageKey) -> Option<H256> {
        async_std::task::block_on(async {
            let started_at = Instant::now();

            let result = sqlx::query!(
                "SELECT value FROM storage WHERE hashed_key = $1",
                &key.hashed_key().0.to_vec()
            )
            .fetch_optional(self.storage.conn())
            .await
            .unwrap()
            .map(|row| H256::from_slice(&row.value));
            metrics::histogram!("dal.request", started_at.elapsed(), "method" => "get_by_key");

            result
        })
    }

    pub fn rollback_factory_deps(&mut self, block_number: MiniblockNumber) {
        async_std::task::block_on(async {
            sqlx::query!(
                "DELETE FROM factory_deps WHERE miniblock_number > $1",
                block_number.0 as i64
            )
            .execute(self.storage.conn())
            .await
            .unwrap();
        })
    }
}
