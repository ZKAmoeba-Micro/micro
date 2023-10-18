use micro_basic_types::U64;
use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct StatiticsInfo {
    pub tx_mempool: u32,
    pub last_verified_block: U64,
    pub next_block_id: U64,
    pub un_verified_blocks: U64,
    pub latest_proof_time: i64,
}

impl StatiticsInfo {}
