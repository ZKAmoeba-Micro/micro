use derivative::Derivative;
use serde_derive::{Deserialize, Serialize};
use ssz_derive::{Decode, Encode};
use tree_hash_derive::TreeHash;

/// The body of a `BeaconChain` block, containing operations.
#[derive(Debug, Clone, Encode, Decode, TreeHash, Serialize, Deserialize, Derivative)]
#[derivative(PartialEq, Hash)]
pub struct BlockBody {}
