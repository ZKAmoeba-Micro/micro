use crate::{SqlxError, StorageProcessor};
use micro_types::assignment_user_summary::{AssignmentUserSummaryInfo, UserStatus};

#[derive(Debug)]
pub struct AssignmentUserSummaryDal<'a, 'c> {
    pub(crate) storage: &'a mut StorageProcessor<'c>,
}

impl AssignmentUserSummaryDal<'_, '_> {
    pub async fn add_assignment_user_summary_info(
        &mut self,
        param_info: AssignmentUserSummaryInfo,
        status: UserStatus,
    ) -> Result<(), SqlxError> {
        tracing::info!(
            "add_assignment_user_summary_info param_info:{:?}",
            param_info
        );

        sqlx::query!(
            "
            INSERT INTO assignment_user_summary (verification_address,status,base_score,last_batch_number,created_at,update_at)
            VALUES ($1, $2, $3, $4, now(), now())
            ON CONFLICT(verification_address)
            DO UPDATE SET update_at=now()
            ",
            param_info.verification_address.as_bytes(),
            status.to_string(),
            param_info.base_score as i32,
            param_info.last_batch_number.0 as i64
        )
        .execute(self.storage.conn())
        .await?;
        Ok(())
    }
}
