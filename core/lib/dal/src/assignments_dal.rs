use std::{str::FromStr, time::Duration};

use micro_types::{Address, L1BatchNumber, MiniblockNumber, H256};
use strum::{Display, EnumString};

use crate::{
    instrument::InstrumentExt, time_utils::pg_interval_from_duration, SqlxError, StorageProcessor,
};
#[derive(Debug)]
pub struct AssignmentsDal<'a, 'c> {
    pub(crate) storage: &'a mut StorageProcessor<'c>,
}

#[derive(Debug, EnumString, Display)]
pub enum ProverResultStatus {
    #[strum(serialize = "assigned_not_certified")]
    AssignedNotCertified,
    #[strum(serialize = "be_punished")]
    BePunished,
    #[strum(serialize = "picked_by_prover")]
    PickedByProver,
    #[strum(serialize = "successful")]
    Successful,
    #[strum(serialize = "failed")]
    Failed,
    #[strum(serialize = "rollbacked")]
    Rollbacked,
}

impl AssignmentsDal<'_, '_> {
    pub async fn insert_and_update_assignments(
        &mut self,
        verification_address: Address,
        block_number: L1BatchNumber,
        miniblock_number: MiniblockNumber,
        storage_index: u64,
    ) -> Result<(), SqlxError> {
        let mut transaction = self.storage.start_transaction().await.unwrap();

        tracing::info!("insert_and_update_assignments verification_address:{verification_address},block_number:{block_number},miniblock_number:{miniblock_number},event_index:{storage_index}");

        let batch_hash = sqlx::query!(
            "SELECT hash FROM l1_batches WHERE number = $1 ORDER BY number DESC  LIMIT 1",
            block_number.0 as i64
        )
        .fetch_one(transaction.conn())
        .await?
        .hash;

        let hash = match batch_hash {
            Some(h) => H256::from_slice(&h),
            None => H256::zero(),
        };

        sqlx::query!("INSERT INTO assignments (verification_address,l1_batch_number,miniblock_number,storage_index,status,created_at,updated_at,batch_hash) VALUES ($1,$2,$3,$4,'assigned_not_certified', now(), now(),$5) ON CONFLICT(verification_address,l1_batch_number,storage_index,batch_hash) DO UPDATE  SET updated_at=now()",
             verification_address.as_bytes(),
             block_number.0 as i64,
             miniblock_number.0 as i64,
             storage_index as i64,
             hash.as_bytes(),
        )
        .execute(transaction.conn())
        .await?;
        transaction.commit().await.unwrap();
        Ok(())
    }

    pub async fn update_assigments_status_for_time(
        &mut self,
        processing_timeout: Duration,
    ) -> Result<(), SqlxError> {
        // let time_str: &str = if !m_time.is_empty() { m_time } else { "'60 M'" };

        let processing_timeout = pg_interval_from_duration(processing_timeout);
        let count = sqlx::query!("select count(1) as num from assignments where now()>created_at + $1::interval and status in('assigned_not_certified','picked_by_prover')",&processing_timeout)
        .fetch_one(self.storage.conn())
        .await?
        .num;
        tracing::info!("update_assigments_status_for_time count:{:?}", count);
        if count != Some(0) {
            let mut transaction = self.storage.start_transaction().await.unwrap();

            sqlx::query!(
                "update assignments \
            set status ='be_punished', updated_at = now() \
            where id in (select id \
                from assignments \
                where now()>created_at + $1::interval and status in('assigned_not_certified','picked_by_prover') \
            ) and tx_hash is null",
                &processing_timeout,
            )
            .execute(transaction.conn())
            .await?;

            sqlx::query!(
                "UPDATE proof_generation_details \
            SET status='ready_to_be_proven', updated_at = now() \
            WHERE status='picked_by_prover' \
            and l1_batch_number in (select  l1_batch_number \
            from assignments \
            where  status='be_punished')"
            )
            .execute(transaction.conn())
            .await?;
            transaction.commit().await.unwrap();
        }
        Ok(())
    }

    pub async fn update_assigments_status_be_punished(
        &mut self,
        verification_address: Address,
        l1_batch_number: L1BatchNumber,
    ) -> Result<(), SqlxError> {
        let mut transaction = self.storage.start_transaction().await.unwrap();

        sqlx::query!(
            "UPDATE assignments \
             SET status ='be_punished', updated_at = now() \
             WHERE verification_address = $1 AND l1_batch_number = $2 AND status = 'successful'",
            verification_address.as_bytes(),
            l1_batch_number.0 as i64,
        )
        .execute(transaction.conn())
        .await?;

        sqlx::query!(
            "UPDATE proof_generation_details \
             SET status = 'ready_to_be_proven', updated_at = now() \
             WHERE status = 'generated' AND l1_batch_number = $1",
            l1_batch_number.0 as i64
        )
        .execute(transaction.conn())
        .await?;
        transaction.commit().await.unwrap();

        Ok(())
    }

    pub async fn update_assigments_status_by_punished(
        &mut self,
        storage_index: u64,
        verification_address: Address,
        block_number: L1BatchNumber,
        tx_hash: H256,
    ) -> Result<(), SqlxError> {
        tracing::info!("update_assigments_status verification_address:{verification_address},block_number:{block_number},tx_hash:{tx_hash}");

        sqlx::query!("UPDATE assignments SET status='failed',tx_hash=$1,updated_at=now() WHERE storage_index=$2 and verification_address = $3 AND l1_batch_number = $4  and status='be_punished' and tx_hash is null",
            tx_hash.as_bytes(),
            storage_index as i64,
            verification_address.as_bytes(),
            block_number.0 as i64,
        )
        .execute(self.storage.conn())
        .await?;
        Ok(())
    }

    pub async fn get_punished_address_list(&mut self) -> Vec<(i32, Address, L1BatchNumber, u64)> {
        let result = sqlx::query!(
        "SELECT id,verification_address,l1_batch_number,storage_index \
        FROM ( \
            SELECT id,verification_address,l1_batch_number,storage_index,status,tx_hash,COUNT(*) OVER (PARTITION BY verification_address) AS total_count \
            FROM assignments \
            WHERE status IN ('failed', 'be_punished') ORDER BY created_at ) AS subquery WHERE total_count > 0 AND status = 'be_punished' and tx_hash is null")
        .fetch_all(self.storage.conn())
        .await
        .unwrap();

        result
            .into_iter()
            .map(|row| {
                (
                    row.id,
                    Address::from_slice(&row.verification_address),
                    L1BatchNumber(row.l1_batch_number as u32),
                    row.storage_index as u64,
                )
            })
            .collect()
    }

    pub async fn get_next_block_to_be_proven(&mut self, prover: Address) -> Option<L1BatchNumber> {
        let mut transaction = self.storage.start_transaction().await.unwrap();

        let result: Option<L1BatchNumber> = sqlx::query!(
            "UPDATE assignments \
             SET status = 'picked_by_prover', updated_at = now() \
             WHERE id = ( \
                 SELECT id \
                 FROM assignments \
                 WHERE status = 'assigned_not_certified' \
                 AND verification_address = $1 \
                 ORDER BY id ASC \
                 LIMIT 1 \
                 FOR UPDATE \
                 SKIP LOCKED \
             ) \
             RETURNING assignments.l1_batch_number",
            prover.as_bytes(),
        )
        .fetch_optional(transaction.conn())
        .await
        .unwrap()
        .map(|row| L1BatchNumber(row.l1_batch_number as u32));

        if let Some(l1_batch_number) = &result {
            sqlx::query!(
                "UPDATE proof_generation_details \
                 SET status = 'picked_by_prover', updated_at = now(), prover_taken_at = now() \
                 WHERE l1_batch_number = $1",
                l1_batch_number.0 as i64,
            )
            .execute(transaction.conn())
            .await
            .unwrap();
        }

        transaction.commit().await.unwrap();

        result
    }

    pub async fn get_job_details(
        &mut self,
        prover: Address,
        l1_batch_number: L1BatchNumber,
    ) -> Result<Option<(ProverResultStatus, i64)>, SqlxError> {
        let status_and_created_at = sqlx::query!(
            "SELECT status, created_at FROM assignments WHERE verification_address = $1 AND l1_batch_number = $2 ORDER BY created_at DESC LIMIT 1",
            prover.as_bytes(),
            l1_batch_number.0 as i64
        )
        .fetch_optional(self.storage.conn())
        .await?
        .map(|row| (ProverResultStatus::from_str(&row.status).unwrap(), row.created_at.timestamp()));

        Ok(status_and_created_at)
    }

    pub async fn get_verification_address(
        &mut self,
        l1_batch_number: L1BatchNumber,
    ) -> Result<Address, SqlxError> {
        let address = sqlx::query!(
            "SELECT verification_address FROM assignments WHERE l1_batch_number = $1 AND status = 'successful' LIMIT 1",
            l1_batch_number.0 as i64
        )
        .fetch_one(self.storage.conn())
        .await?
        .verification_address;

        Ok(Address::from_slice(&address))
    }

    pub async fn save_proof_artifacts_metadata(
        &mut self,
        block_number: L1BatchNumber,
        proof_blob_url: &str,
    ) -> Result<(), SqlxError> {
        let mut transaction = self.storage.start_transaction().await.unwrap();

        sqlx::query!("UPDATE assignments SET status = 'successful', updated_at = now() WHERE l1_batch_number = $1 AND status = 'picked_by_prover'",
            block_number.0 as i64,
        )
        .execute(transaction.conn())
        .await?
        .rows_affected()
        .eq(&1)
        .then_some(())
        .ok_or(sqlx::Error::RowNotFound)?;

        sqlx::query!(
            "UPDATE proof_generation_details \
             SET status='generated', proof_blob_url = $1, updated_at = now() \
             WHERE l1_batch_number = $2",
            proof_blob_url,
            block_number.0 as i64,
        )
        .execute(transaction.conn())
        .await?
        .rows_affected()
        .eq(&1)
        .then_some(())
        .ok_or(sqlx::Error::RowNotFound)?;

        transaction.commit().await.unwrap();

        Ok(())
    }

    pub async fn get_max_mini_number(&mut self) -> Result<MiniblockNumber, sqlx::Error> {
        let number = sqlx::query!(
            "select max(miniblock_number) number from assignments where status != 'rollbacked'"
        )
        .instrument("get_max_mini_number")
        .report_latency()
        .fetch_one(self.storage.conn())
        .await?
        .number
        .unwrap_or(0);
        Ok(MiniblockNumber(number as u32))
    }

    pub async fn rollback_assigments(
        &mut self,
        last_l1_batch_to_keep: L1BatchNumber,
        last_miniblock_to_keep: MiniblockNumber,
    ) -> Result<(), SqlxError> {
        tracing::info!("rollback_assigments last_l1_batch_to_keep: {last_l1_batch_to_keep}, last_miniblock_to_keep: {last_miniblock_to_keep}");

        sqlx::query!("UPDATE assignments SET status = 'rollbacked', updated_at = now() WHERE l1_batch_number > $1", last_l1_batch_to_keep.0 as i64)
        .execute(self.storage.conn())
        .await?;

        sqlx::query!("UPDATE assignments SET miniblock_number = $1, updated_at = now() WHERE miniblock_number > $1", last_miniblock_to_keep.0 as i64)
        .execute(self.storage.conn())
        .await?;
        Ok(())
    }
}
