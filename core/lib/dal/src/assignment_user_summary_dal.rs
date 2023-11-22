use crate::{instrument::InstrumentExt, SqlxError, StorageProcessor};
use micro_types::{
    assignment_user_summary::AssignmentUserSummaryInfo, l2::score_update::Status, Address,
    MiniblockNumber,
};
#[derive(Debug)]
pub struct AssignmentUserSummaryDal<'a, 'c> {
    pub(crate) storage: &'a mut StorageProcessor<'c>,
}

impl AssignmentUserSummaryDal<'_, '_> {
    pub async fn add_or_update_assignment_user_summary_info(
        &mut self,
        param_info: AssignmentUserSummaryInfo,
        status: Status,
        miniblock_number: MiniblockNumber,
    ) -> Result<(), SqlxError> {
        tracing::info!(
            "add_assignment_user_summary_info param_info:{:?}",
            param_info
        );
        sqlx::query!(
            "INSERT INTO assignment_user_summary (verification_address,status,base_score,last_time,miniblock_number,created_at,update_at)
            VALUES ($1, $2, $3, $4,$5, now(), now())
            ON CONFLICT(verification_address)
            DO UPDATE  SET status=excluded.status,base_score=excluded.base_score,last_time=excluded.last_time,miniblock_number=excluded.miniblock_number,update_at=now()",
            param_info.verification_address.as_bytes(),
            status.to_string(),
            param_info.base_score as i32,
            param_info.last_time,
            miniblock_number.0 as i64,

        )
        .execute(self.storage.conn())
        .await?;
        Ok(())
    }

    pub async fn select_normal_user_list(
        &mut self,
        status: Status,
    ) -> Vec<AssignmentUserSummaryInfo> {
        let result = sqlx::query!("select verification_address,base_score,last_time from assignment_user_summary where status=$1",
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
                last_time: row.last_time,
            })
            .collect()
    }

    pub async fn get_max_miniblock_number(&mut self) -> Result<MiniblockNumber, sqlx::Error> {
        let number =
            sqlx::query!("select max(miniblock_number) number from assignment_user_summary")
                .instrument("get_max_miniblock_number")
                .report_latency()
                .fetch_one(self.storage.conn())
                .await?
                .number
                .unwrap_or(0);
        Ok(MiniblockNumber(number as u32))
    }
}
