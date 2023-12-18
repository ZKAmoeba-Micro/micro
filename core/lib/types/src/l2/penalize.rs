use std::convert::TryFrom;

use micro_basic_types::H256;
use micro_contracts::sys_deposit_contract;
use micro_utils::{h256_to_account_address, h256_to_u256};
use serde::{Deserialize, Serialize};

use crate::{api::Log, l2::event_map::EventMapBuilder, Address, U256};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Penalize {
    pub prover: Address,
    pub batch_number: U256,
    pub storage_index: U256,
    pub mini_block_number: u32,
    pub transaction_hash: Option<H256>,
}
pub const PENALIZE: &'static str = "Penalize";
impl Penalize {
    pub(crate) fn is_penalize() -> H256 {
        let msg = format!("Penalize {} event is missing in abi", PENALIZE);
        let signature: micro_basic_types::H256 = sys_deposit_contract()
            .event(PENALIZE)
            .expect(msg.as_str())
            .signature();
        signature
    }
}
impl TryFrom<Log> for Penalize {
    type Error = crate::ethabi::Error;

    fn try_from(event: Log) -> Result<Self, Self::Error> {
        let topics = event.topics;
        let prover = h256_to_account_address(topics.get(1).unwrap());
        let batch_number = h256_to_u256(*topics.get(2).unwrap());
        let storage_index = h256_to_u256(*topics.get(3).unwrap());
        let mini_block_number = event.block_number.unwrap().as_u32();
        let transaction_hash = event.transaction_hash;

        // let status = match status {
        //     0 => Status::UnDeposit,
        //     1 => Status::Normal,
        //     2 => Status::Frozen,
        //     3 => Status::Applying,
        //     _ => Status::Unknow,
        // };
        // let mut decoded = decode(&[ParamType::Uint(256)], &event.data.0).unwrap();
        // let latest_assignment_time = decoded.remove(0).into_uint().unwrap();
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

        Ok(Penalize {
            transaction_hash,
            prover,
            batch_number,
            storage_index,
            mini_block_number,
        })
    }
}

impl EventMapBuilder for Penalize {
    type A = Penalize;
    fn build_map(logs: Vec<Log>) -> Vec<Penalize> {
        let mut as_map: Vec<Penalize> = Vec::new();
        for log in logs {
            let tops = log.clone().topics;
            let event_name = tops.get(0).unwrap();
            if event_name == &Penalize::is_penalize() {
                let at_add: Penalize = Penalize::try_from(log).unwrap();
                as_map.push(at_add);
            }
        }
        as_map
    }
}
