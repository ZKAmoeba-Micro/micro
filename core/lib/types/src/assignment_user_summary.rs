use chrono::{DateTime, Utc};
use micro_basic_types::Address;
use micro_utils::UnsignedRatioSerializeAsDecimal;
use num::{rational::Ratio, BigUint};
use serde::{Deserialize, Serialize};

#[derive(Debug, strum::Display, strum::EnumString, strum::AsRefStr)]
pub enum UserStatus {
    #[strum(serialize = "enable")]
    Enable,
    #[strum(serialize = "disable")]
    Disable,
}

#[derive(Debug)]
pub struct AssignmentUserSummaryInfo {
    pub verification_address: Address,
    pub status: UserStatus,
    //The number of user engagements, which becomes zero when the user is assigned a proof task, and keeps increasing otherwise.
    //The larger the value the higher the chance of being selected
    pub participations_num: u32,
    //Number of proof tasks completed by users
    pub completion_num: u32,
    //Total value of time for the number of times the verifier completes the proof task
    //completion_prove_cumulative_time/completion_num = p/h
    pub completion_prove_cumulative_time: u64,
    //Number of proof tasks where the user has timed out
    //Completed proof tasks plus timeout proof tasks equal the total number of tasks assigned to the current user
    pub timeout_num: u32,
    //User Reputation Score
    pub score: u16,
    //When the authenticated user address was first created
    pub created_at: u64,
    pub update_at: u64,
    //Number of tokens pledged by the node user in the contract
    pub deposit_amount: u64,
}
