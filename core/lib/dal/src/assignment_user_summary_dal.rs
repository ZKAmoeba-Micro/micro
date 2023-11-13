use crate::{SqlxError, StorageProcessor};
use micro_types::{
    assignment_user_summary::{AssignmentUserSummaryInfo, UserStatus},
    Address, L1BatchNumber,
};
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

    pub async fn select_normal_user_list(
        &mut self,
        status: UserStatus,
    ) -> Vec<AssignmentUserSummaryInfo> {
        let result = sqlx::query!("select verification_address,base_score,last_batch_number from assignment_user_summary where status=$1",
            status.to_string()
        )
        .fetch_all(self.storage.conn())
        .await
        .unwrap();

        result
            .into_iter()
            .map(|row| AssignmentUserSummaryInfo {
                verification_address: Address::from_slice(&row.verification_address),
                base_score: row.base_score as u16,
                last_batch_number: L1BatchNumber::from(row.last_batch_number as u32),
            })
            .collect()
    }
}
