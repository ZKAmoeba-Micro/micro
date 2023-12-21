use std::convert::TryFrom;

use micro_basic_types::{
    ethabi::{decode, ParamType},
    H256,
};
use micro_contracts::sys_assignment_contract;
use serde::{Deserialize, Serialize};

use crate::{api::Log, l2::event_map::EventMapBuilder, Address, U256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignmentBatch {
    pub prover: Address,
    pub batch_number: U256,
    pub storage_index: U256,
    pub assignment_time: U256,
    pub mini_block_number: u32,
}

pub const ASSIGNMENT_BATCH: &'static str = "AssignmentBatch";

impl AssignmentBatch {
    pub(crate) fn is_assignment_batch() -> H256 {
        let msg = format!(
            "AssignmentBatch {} event is missing in abi",
            ASSIGNMENT_BATCH
        );
        let signature: micro_basic_types::H256 = sys_assignment_contract()
            .event(ASSIGNMENT_BATCH)
            .expect(msg.as_str())
            .signature();
        signature
    }
}

impl TryFrom<Log> for AssignmentBatch {
    type Error = crate::ethabi::Error;

    fn try_from(event: Log) -> Result<Self, Self::Error> {
        // let topics = event.topics;
        // let prover = h256_to_account_address(topics.get(1).unwrap());
        // let status = h256_to_u256(*topics.get(2).unwrap()).as_u32();
        // let base_score = h256_to_u256(*topics.get(3).unwrap());
        // let status = match status {
        //     0 => Status::UnDeposit,
        //     1 => Status::Normal,
        //     2 => Status::Frozen,
        //     3 => Status::Applying,
        //     _ => Status::Unknow,
        // };
        // let mut decoded = decode(&[ParamType::Uint(256)], &event.data.0).unwrap();
        // let latest_assignment_time = decoded.remove(0).into_uint().unwrap();
        let mini_block_number = event.block_number.unwrap().as_u32();
        let mut decoded = decode(
            &[
                ParamType::Address,
                ParamType::Uint(256),
                ParamType::Uint(256),
                ParamType::Uint(256),
            ],
            &event.data.0,
        )
        .unwrap();
        let prover = decoded.remove(0).into_address().unwrap();
        let batch_number = decoded.remove(0).into_uint().unwrap();
        let storage_index = decoded.remove(0).into_uint().unwrap();
        let assignment_time = decoded.remove(0).into_uint().unwrap();

        Ok(AssignmentBatch {
            prover,
            batch_number,
            storage_index,
            assignment_time,
            mini_block_number,
        })
    }
}

impl EventMapBuilder for AssignmentBatch {
    type A = AssignmentBatch;
    fn build_map(logs: Vec<Log>) -> Vec<AssignmentBatch> {
        let mut as_map: Vec<AssignmentBatch> = Vec::new();
        for log in logs {
            let tops = log.clone().topics;
            let event_name = tops.get(0).unwrap();
            if event_name == &AssignmentBatch::is_assignment_batch() {
                let at_add: AssignmentBatch = AssignmentBatch::try_from(log).unwrap();
                as_map.push(at_add);
            }
        }
        as_map
    }
}
