use crate::types::block_body::BlockBody;
use crate::types::slot_epoch::Slot;
use crate::*;
use derivative::Derivative;
use serde_derive::{Deserialize, Serialize};
use ssz::{Decode, DecodeError};
use ssz_derive::{Decode, Encode};
use std::marker::PhantomData;
use superstruct::superstruct;
use tree_hash::Hash256;
use tree_hash::TreeHash;
use tree_hash_derive::TreeHash;

/// A block of the `BeaconChain`.
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode, TreeHash, Derivative)]
#[derivative(PartialEq, Hash)]
pub struct Block {
    pub slot: Slot,
    #[serde(with = "serde_utils::quoted_u64")]
    pub proposer_index: u64,
    pub parent_root: Hash256,
    pub state_root: Hash256,
    pub body: BlockBody,
}
