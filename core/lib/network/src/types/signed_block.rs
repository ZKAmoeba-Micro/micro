use derivative::Derivative;
use serde_derive::{Deserialize, Serialize};
use ssz_derive::{Decode, Encode};

/// A `BeaconBlock` and a signature from its proposer.
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode, Derivative)]
#[derivative(PartialEq, Hash)]
pub struct SignedBlock {
    pub slot: u64,
}
