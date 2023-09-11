use crate::StorageProcessor;
use micro_types::{
    get_code_key, AccountTreeId, Address, L1BatchNumber, MiniblockNumber, StorageKey, StorageLog,
    FAILED_CONTRACT_DEPLOYMENT_BYTECODE_HASH, H256,
};
use sqlx::types::chrono::Utc;
use std::collections::HashMap;

#[derive(Debug)]
pub struct StorageLogsDal<'a, 'c> {
    pub storage: &'a mut StorageProcessor<'c>,
}

impl StorageLogsDal<'_, '_> {
    pub fn insert_storage_logs(
        &mut self,
        block_number: MiniblockNumber,
        logs: &[(H256, Vec<StorageLog>)],
    ) {
        async_std::task::block_on(async {
            let mut copy = self
            .storage
            .conn()
            .copy_in_raw(
                "COPY storage_logs (hashed_key, address, key, value, operation_number, tx_hash, miniblock_number, created_at, updated_at)
                FROM STDIN WITH (DELIMITER '|')",
            )
            .await
            .unwrap();

            let mut bytes: Vec<u8> = Vec::new();
            let now = Utc::now().naive_utc().to_string();
            let mut operation_number = 0u32;
            for (tx_hash, logs) in logs {
                let tx_hash_str = format!("\\\\x{}", hex::encode(tx_hash.0));
                for log in logs {
                    let hashed_key_str = format!("\\\\x{}", hex::encode(log.key.hashed_key().0));
                    let address_str = format!("\\\\x{}", hex::encode(log.key.address().0));
                    let key_str = format!("\\\\x{}", hex::encode(log.key.key().0));
                    let value_str = format!("\\\\x{}", hex::encode(log.value.0));
                    let row = format!(
                        "{}|{}|{}|{}|{}|{}|{}|{}|{}\n",
                        hashed_key_str,
                        address_str,
                        key_str,
                        value_str,
                        operation_number,
                        tx_hash_str,
                        block_number,
                        now,
                        now
                    );
                    bytes.extend_from_slice(row.as_bytes());

                    operation_number += 1;
                }
            }
            copy.send(bytes).await.unwrap();
            copy.finish().await.unwrap();
        })
    }

    pub fn append_storage_logs(
        &mut self,
        block_number: MiniblockNumber,
        logs: &[(H256, Vec<StorageLog>)],
    ) {
        async_std::task::block_on(async {
            let mut operation_number = sqlx::query!(
                r#"SELECT COUNT(*) as "count!" FROM storage_logs WHERE miniblock_number = $1"#,
                block_number.0 as i64
            )
            .fetch_one(self.storage.conn())
            .await
            .unwrap()
            .count as u32;

            let mut copy = self
                .storage
                .conn()
                .copy_in_raw(
                    "COPY storage_logs (hashed_key, address, key, value, operation_number, tx_hash, miniblock_number, created_at, updated_at)
                FROM STDIN WITH (DELIMITER '|')",
                )
                .await
                .unwrap();

            let mut bytes: Vec<u8> = Vec::new();
            let now = Utc::now().naive_utc().to_string();
            for (tx_hash, logs) in logs {
                let tx_hash_str = format!("\\\\x{}", hex::encode(tx_hash.0));
                for log in logs {
                    let hashed_key_str = format!("\\\\x{}", hex::encode(log.key.hashed_key().0));
                    let address_str = format!("\\\\x{}", hex::encode(log.key.address().0));
                    let key_str = format!("\\\\x{}", hex::encode(log.key.key().0));
                    let value_str = format!("\\\\x{}", hex::encode(log.value.0));
                    let row = format!(
                        "{}|{}|{}|{}|{}|{}|{}|{}|{}\n",
                        hashed_key_str,
                        address_str,
                        key_str,
                        value_str,
                        operation_number,
                        tx_hash_str,
                        block_number,
                        now,
                        now
                    );
                    bytes.extend_from_slice(row.as_bytes());

                    operation_number += 1;
                }
            }
            copy.send(bytes).await.unwrap();
            copy.finish().await.unwrap();
        })
    }

    pub fn rollback_storage(&mut self, block_number: MiniblockNumber) {
        async_std::task::block_on(async {
            vlog::info!("fetching keys that were changed after given block number");
            let modified_keys: Vec<H256> = sqlx::query!(
                "SELECT DISTINCT ON (hashed_key) hashed_key FROM
                (SELECT * FROM storage_logs WHERE miniblock_number > $1) inn",
                block_number.0 as i64
            )
            .fetch_all(self.storage.conn())
            .await
            .unwrap()
            .into_iter()
            .map(|row| H256::from_slice(&row.hashed_key))
            .collect();
            vlog::info!("loaded {:?} keys", modified_keys.len());

            for key in modified_keys {
                let previous_value: Option<H256> = sqlx::query!(
                    "select value from storage_logs where hashed_key = $1 and miniblock_number <= $2 order by miniblock_number desc, operation_number desc limit 1",
                    key.as_bytes(),
                    block_number.0 as i64
                )
                .fetch_optional(self.storage.conn())
                .await
                .unwrap()
                .map(|r| H256::from_slice(&r.value));
                match previous_value {
                    None => {
                        sqlx::query!("delete from storage where hashed_key = $1", key.as_bytes(),)
                            .execute(self.storage.conn())
                            .await
                            .unwrap()
                    }
                    Some(val) => sqlx::query!(
                        "update storage set value = $1 where hashed_key = $2",
                        val.as_bytes(),
                        key.as_bytes(),
                    )
                    .execute(self.storage.conn())
                    .await
                    .unwrap(),
                };
            }
        })
    }

    pub fn rollback_storage_logs(&mut self, block_number: MiniblockNumber) {
        async_std::task::block_on(async {
            sqlx::query!(
                "DELETE FROM storage_logs WHERE miniblock_number > $1",
                block_number.0 as i64
            )
            .execute(self.storage.conn())
            .await
            .unwrap();
        })
    }

    pub fn is_contract_deployed_at_address(&mut self, address: Address) -> bool {
        let hashed_key = get_code_key(&address).hashed_key();
        async_std::task::block_on(async {
            let count = sqlx::query!(
                r#"
                    SELECT COUNT(*) as "count!"
                    FROM (
                        SELECT * FROM storage_logs
                        WHERE storage_logs.hashed_key = $1
                        ORDER BY storage_logs.miniblock_number DESC, storage_logs.operation_number DESC
                        LIMIT 1
                    ) sl
                    WHERE sl.value != $2
                "#,
                hashed_key.as_bytes(),
                FAILED_CONTRACT_DEPLOYMENT_BYTECODE_HASH.as_bytes(),
            )
            .fetch_one(self.storage.conn())
            .await
            .unwrap()
            .count;
            count > 0
        })
    }

    pub fn get_touched_slots_for_l1_batch(
        &mut self,
        l1_batch_number: L1BatchNumber,
    ) -> HashMap<StorageKey, H256> {
        async_std::task::block_on(async {
            let storage_logs = sqlx::query!(
                "
                SELECT address, key, value
                FROM storage_logs
                WHERE miniblock_number BETWEEN (SELECT MIN(number) FROM miniblocks WHERE l1_batch_number = $1)
                    AND (SELECT MAX(number) FROM miniblocks WHERE l1_batch_number = $1)
                ORDER BY miniblock_number, operation_number
                ",
                l1_batch_number.0 as i64
            )
                .fetch_all(self.storage.conn())
                .await
                .unwrap();

            let mut touched_slots = HashMap::new();
            for storage_log in storage_logs.into_iter() {
                touched_slots.insert(
                    StorageKey::new(
                        AccountTreeId::new(Address::from_slice(&storage_log.address)),
                        H256::from_slice(&storage_log.key),
                    ),
                    H256::from_slice(&storage_log.value),
                );
            }
            touched_slots
        })
    }

    pub fn get_storage_logs_for_revert(
        &mut self,
        l1_batch_number: L1BatchNumber,
    ) -> Vec<(H256, Option<H256>)> {
        async_std::task::block_on(async {
            let miniblock_number = match self
                .storage
                .blocks_dal()
                .get_miniblock_range_of_l1_batch(l1_batch_number)
            {
                None => return Vec::new(),
                Some((_, number)) => number,
            };

            vlog::info!("fetching keys that were changed after given block number");
            let modified_keys: Vec<H256> = sqlx::query!(
                "SELECT DISTINCT ON (hashed_key) hashed_key FROM
                (SELECT * FROM storage_logs WHERE miniblock_number > $1) inn",
                miniblock_number.0 as i64
            )
            .fetch_all(self.storage.conn())
            .await
            .unwrap()
            .into_iter()
            .map(|row| H256::from_slice(&row.hashed_key))
            .collect();
            vlog::info!("loaded {:?} keys", modified_keys.len());

            let mut result: Vec<(H256, Option<H256>)> = vec![];

            for key in modified_keys {
                let initially_written_at: Option<L1BatchNumber> = sqlx::query!(
                    "
                        SELECT l1_batch_number FROM initial_writes
                        WHERE hashed_key = $1
                    ",
                    key.as_bytes(),
                )
                .fetch_optional(self.storage.conn())
                .await
                .unwrap()
                .map(|row| L1BatchNumber(row.l1_batch_number as u32));
                match initially_written_at {
                    // Key isn't written to the storage - nothing to rollback.
                    None => continue,
                    // Key was initially written, it's needed to remove it.
                    Some(initially_written_at) if initially_written_at > l1_batch_number => {
                        result.push((key, None));
                    }
                    // Key was rewritten, it's needed to restore the previous value.
                    Some(_) => {
                        let previous_value: Vec<u8> = sqlx::query!(
                            "
                            SELECT value FROM storage_logs
                            WHERE hashed_key = $1 AND miniblock_number <= $2
                            ORDER BY miniblock_number DESC, operation_number DESC
                            LIMIT 1
                            ",
                            key.as_bytes(),
                            miniblock_number.0 as i64
                        )
                        .fetch_one(self.storage.conn())
                        .await
                        .unwrap()
                        .value;
                        result.push((key, Some(H256::from_slice(&previous_value))));
                    }
                }
                if result.len() % 1000 == 0 {
                    vlog::info!("processed {:?} values", result.len());
                }
            }

            result
        })
    }

    pub fn get_previous_storage_values(
        &mut self,
        hashed_keys: Vec<H256>,
        l1_batch_number: L1BatchNumber,
    ) -> HashMap<H256, H256> {
        async_std::task::block_on(async {
            let hashed_keys: Vec<_> = hashed_keys.into_iter().map(|key| key.0.to_vec()).collect();
            let (miniblock_number, _) = self
                .storage
                .blocks_dal()
                .get_miniblock_range_of_l1_batch(l1_batch_number)
                .unwrap();
            sqlx::query!(
                r#"
                    SELECT u.hashed_key as "hashed_key!",
                        (SELECT value FROM storage_logs
                        WHERE hashed_key = u.hashed_key AND miniblock_number < $2
                        ORDER BY miniblock_number DESC, operation_number DESC LIMIT 1) as "value?"
                    FROM UNNEST($1::bytea[]) AS u(hashed_key)
                "#,
                &hashed_keys,
                miniblock_number.0 as i64
            )
            .fetch_all(self.storage.conn())
            .await
            .unwrap()
            .into_iter()
            .map(|row| {
                (
                    H256::from_slice(&row.hashed_key),
                    row.value
                        .map(|value| H256::from_slice(&value))
                        .unwrap_or_else(H256::zero),
                )
            })
            .collect()
        })
    }
}
