use serde_derive::{Deserialize, Serialize};
use ssz_derive::{Decode, Encode};
use tree_hash::{Hash256, TreeHash};
use tree_hash_derive::TreeHash;

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, Encode, Decode, TreeHash)]
pub struct SigningData {
    pub object_root: Hash256,
    pub domain: Hash256,
}

pub trait SignedRoot: TreeHash {
    fn signing_root(&self, domain: Hash256) -> Hash256 {
        SigningData {
            object_root: self.tree_hash_root(),
            domain,
        }
        .tree_hash_root()
    }
}
