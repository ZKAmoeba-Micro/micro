use micro_basic_types::{Address, L1BatchNumber};
use serde::{Deserialize, Serialize};

#[derive(Debug, strum::Display, strum::EnumString, strum::AsRefStr)]
pub enum UserStatus {
    #[strum(serialize = "undeposit")]
    UnDeposit,
    #[strum(serialize = "normal")]
    Normal,
    #[strum(serialize = "frozon")]
    Frozon,
    #[strum(serialize = "applying")]
    Applying,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentUserSummaryInfo {
    pub verification_address: Address,
    //User Reputation base_score
    pub base_score: u16,
    pub last_batch_number: L1BatchNumber,
}

impl AssignmentUserSummaryInfo {
    pub fn new(
        verification_address: Address,
        base_score: u16,
        last_batch_number: L1BatchNumber,
    ) -> Self {
        Self {
            verification_address: verification_address,
            base_score: base_score,
            last_batch_number: last_batch_number,
        }
    }
}
