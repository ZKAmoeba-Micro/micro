use std::convert::TryFrom;

use crate::{api::Log, Address, U256};

use micro_basic_types::ethabi::{decode, ParamType};
use micro_utils::{h256_to_account_address, h256_to_u256};
use serde::{Deserialize, Serialize};

#[derive(
    Debug, strum::Display, strum::EnumString, Clone, Serialize, Deserialize, PartialEq, Eq,
)]
pub enum Status {
    #[strum(serialize = "undeposit")]
    UnDeposit,
    #[strum(serialize = "normal")]
    Normal,
    #[strum(serialize = "frozen")]
    Frozen,
    #[strum(serialize = "applying")]
    Applying,
    #[strum(serialize = "unknow")]
    Unknow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreUpdate {
    pub prover: Address,
    pub status: Status,
    pub base_score: U256,
    pub latest_assignment_time: U256,
    pub mini_block_number: u32,
}

impl TryFrom<Log> for ScoreUpdate {
    type Error = crate::ethabi::Error;

    fn try_from(event: Log) -> Result<Self, Self::Error> {
        let topics = event.topics;
        let mini_block_number = event.block_number.unwrap().as_u32();

        let prover = h256_to_account_address(topics.get(1).unwrap());
        let status = h256_to_u256(*topics.get(2).unwrap()).as_u32();
        let base_score = h256_to_u256(*topics.get(3).unwrap());

        let status = match status {
            0 => Status::UnDeposit,
            1 => Status::Normal,
            2 => Status::Frozen,
            3 => Status::Applying,
            _ => Status::Unknow,
        };

        let mut decoded = decode(&[ParamType::Uint(256)], &event.data.0).unwrap();

        let latest_assignment_time = decoded.remove(0).into_uint().unwrap();

        // //todo  put info to data,not topics
        // let mut decoded = decode(
        //     &[
        //         ParamType::Address,
        //         ParamType::Uint(256),
        //         ParamType::Uint(256),
        //         ParamType::Uint(256),
        //     ],
        //     &event.data.0,
        // )
        // .unwrap();
        // let prover = decoded.remove(0).into_address().unwrap();
        // let status = decoded.remove(0).into_uint().unwrap().as_u32();

        // let status = match status {
        //     0 => Status::UnDeposit,
        //     1 => Status::Normal,
        //     2 => Status::Frozen,
        //     3 => Status::Applying,
        //     _ => return Err(Self::Error::InvalidData),
        // };
        // let base_score = decoded.remove(0).into_uint().unwrap();
        // let latest_assignment_time = decoded.remove(0).into_uint().unwrap();

        Ok(ScoreUpdate {
            prover,
            status,
            base_score,
            latest_assignment_time,
            mini_block_number,
        })
    }
}
