use std::convert::TryFrom;

use micro_utils::h256_to_u256;
use serde::{Deserialize, Serialize};

use crate::{web3::types::Log, U256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewBatch {
    pub batch_number: U256,
}

pub const NEW_BATCH: &'static str = "NewBatch";

impl TryFrom<Log> for NewBatch {
    type Error = crate::ethabi::Error;

    fn try_from(event: Log) -> Result<Self, Self::Error> {
        let topics = event.topics;
        let batch_number = h256_to_u256(*topics.get(1).unwrap());
        Ok(NewBatch { batch_number })
    }
}
