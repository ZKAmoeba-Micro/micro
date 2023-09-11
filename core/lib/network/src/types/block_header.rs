use serde_derive::{Deserialize, Serialize};
use ssz_derive::{Decode, Encode};
use tree_hash::{Hash256, TreeHash};
use tree_hash_derive::TreeHash;

use super::{signing_data::SignedRoot, slot_epoch::Slot};

/// A header of a `BeaconBlock`.
///
#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize, Encode, Decode, TreeHash)]
pub struct BlockHeader {
    pub slot: Slot,
    #[serde(with = "serde_utils::quoted_u64")]
    pub proposer_index: u64,
    pub parent_root: Hash256,
    pub state_root: Hash256,
    pub body_root: Hash256,
}

impl SignedRoot for BlockHeader {}
