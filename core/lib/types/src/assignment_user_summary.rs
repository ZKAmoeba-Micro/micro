use micro_basic_types::Address;
use serde::{Deserialize, Serialize};

// #[derive(Debug, strum::Display, strum::EnumString, strum::AsRefStr)]
// pub enum UserStatus {
//     #[strum(serialize = "undeposit")]
//     UnDeposit,
//     #[strum(serialize = "normal")]
//     Normal,
//     #[strum(serialize = "frozon")]
//     Frozon,
//     #[strum(serialize = "applying")]
//     Applying,
//     #[strum(serialize = "unknow")]
//     Unknow,
// }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentUserSummaryInfo {
    pub verification_address: Address,
    //User Reputation base_score
    pub base_score: u16,
    pub last_time: i64,
}

impl AssignmentUserSummaryInfo {
    pub fn new(verification_address: Address, base_score: u16, last_time: i64) -> Self {
        Self {
            verification_address: verification_address,
            base_score: base_score,
            last_time: last_time,
        }
    }
}
