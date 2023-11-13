use std::str::FromStr;

use crate::{SqlxError, StorageProcessor};
use micro_types::{Address, L1BatchNumber};
use strum::{Display, EnumString};

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
}

impl AssignmentsDal<'_, '_> {
    pub async fn insert_assignments(
        &mut self,
        verification_address: Address,
        block_number: L1BatchNumber,
    ) -> Result<(), SqlxError> {
        let mut transaction = self.storage.start_transaction().await.unwrap();

        tracing::info!("insert_assignments verification_address:{verification_address},block_number:{block_number}");

        sqlx::query!("INSERT INTO assignments (verification_address,l1_batch_number, status, created_at, updated_at) VALUES ($1,$2, 'assigned_not_certified', now(), now()) ON CONFLICT (verification_address,l1_batch_number) DO NOTHING",
             verification_address.as_bytes(),
             block_number.0 as i64,
        )
        .execute(transaction.conn())
        .await?;

        sqlx::query!("UPDATE assignment_user_summary SET last_batch_number=$1, update_at = now() WHERE  verification_address = $2",
        block_number.0 as i64,
        verification_address.as_bytes(),
        )
        .execute(transaction.conn())
        .await?;

        transaction.commit().await.unwrap();
        Ok(())
    }

    pub async fn update_assigments_status_for_time(
        &mut self,
        verification_address: Address,
        block_number: L1BatchNumber,
    ) -> Result<(), SqlxError> {
        tracing::info!("update_assigments_status_for_time verification_address:{verification_address},block_number:{block_number}");

        let mut transaction = self.storage.start_transaction().await.unwrap();
        sqlx::query!("UPDATE proof_generation_details SET status='ready_to_be_proven', updated_at = now() WHERE l1_batch_number = $1",
                block_number.0 as i64,
        )
        .execute(transaction.conn())
        .await?;
        sqlx::query!("UPDATE assignments SET status='be_punished',updated_at=now() WHERE status='assigned_not_certified' and verification_address=$1 and l1_batch_number = $2",
            verification_address.as_bytes(),
            block_number.0 as i64,
        )
        .execute(transaction.conn())
        .await?;
        transaction.commit().await.unwrap();
        Ok(())
    }

    pub async fn update_assigments_status_by_punished(
        &mut self,
        verification_address: Address,
        block_number: L1BatchNumber,
    ) -> Result<(), SqlxError> {
        tracing::info!("update_assigments_status verification_address:{verification_address},block_number:{block_number}");
        sqlx::query!("UPDATE assignments SET status='failed',updated_at=now() WHERE status='be_punished' and verification_address=$1 and l1_batch_number = $2",
            verification_address.as_bytes(),
            block_number.0 as i64,
        )
        .execute(self.storage.conn())
        .await?;
        Ok(())
    }

    pub async fn get_next_block_to_be_proven(&mut self, prover: Address) -> Option<L1BatchNumber> {
        let mut transaction = self.storage.start_transaction().await.unwrap();

        let result: Option<L1BatchNumber> = sqlx::query!(
            "UPDATE assignments \
             SET status = 'picked_by_prover', updated_at = now() \
             WHERE l1_batch_number = ( \
                 SELECT l1_batch_number \
                 FROM assignments \
                 WHERE status = 'assigned_not_certified' \
                 AND verification_address = $1 \
                 ORDER BY l1_batch_number ASC \
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
            "SELECT status, created_at FROM assignments WHERE verification_address = $1 AND l1_batch_number = $2 LIMIT 1",
            prover.as_bytes(),
            l1_batch_number.0 as i64
        )
        .fetch_optional(self.storage.conn())
        .await?
        .map(|row| (ProverResultStatus::from_str(&row.status).unwrap(), row.created_at.timestamp_millis()));

        Ok(status_and_created_at)
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
}
