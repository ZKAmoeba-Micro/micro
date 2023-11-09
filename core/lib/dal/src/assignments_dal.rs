use crate::{SqlxError, StorageProcessor};
use micro_types::{Address, L1BatchNumber};
use strum::{Display, EnumString};

#[derive(Debug)]
pub struct AssignmentsDal<'a, 'c> {
    pub(crate) storage: &'a mut StorageProcessor<'c>,
}

#[derive(Debug, EnumString, Display)]
pub enum ProverResultStatus {
    #[strum(serialize = "ready_to_be_proven")]
    ReadyToBeProven,
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

        sqlx::query!("UPDATE proof_generation_details SET status='picked_by_prover', updated_at = now(),prover_taken_at = now() WHERE  l1_batch_number = $1",
             block_number.0 as i64,
        )
        .execute(transaction.conn())
        .await?;

        sqlx::query!("INSERT INTO assignments (verification_address,l1_batch_number, status, created_at, updated_at) VALUES ($1,$2, 'ready_to_be_proven', now(), now()) ON CONFLICT (verification_address,l1_batch_number) DO NOTHING",
             verification_address.as_bytes(),
             block_number.0 as i64,
        )
        .execute(transaction.conn())
        .await?;

        transaction.commit().await.unwrap();
        Ok(())
    }

    pub async fn update_assigments_return_status_and_time(
        &mut self,
        verification_address: Address,
        block_number: L1BatchNumber,
        is_result: bool,
    ) -> Result<(), SqlxError> {
        tracing::info!("update_assigments_return_status_and_time verification_address:{verification_address},block_number:{block_number},is_result:{is_result}");

        let mut transaction = self.storage.start_transaction().await.unwrap();

        let result_status = if is_result {
            sqlx::query!("UPDATE proof_generation_details SET status='generated', updated_at = now() WHERE  l1_batch_number = $1",
                 block_number.0 as i64,
            )
            .execute(transaction.conn())
            .await?;

            ProverResultStatus::Successful.to_string()
        } else {
            sqlx::query!("UPDATE proof_generation_details SET status='ready_to_be_proven', updated_at = now() WHERE l1_batch_number = $1",
                 block_number.0 as i64,
            )
            .execute(transaction.conn())
            .await?;

            ProverResultStatus::Failed.to_string()
        };
        sqlx::query!("UPDATE assignments SET status=$1,updated_at=now() WHERE verification_address=$2 and l1_batch_number = $3",
            result_status,
            verification_address.as_bytes(),
            block_number.0 as i64,
        )
        .execute(transaction.conn())
        .await?;

        transaction.commit().await.unwrap();
        Ok(())
    }
}
