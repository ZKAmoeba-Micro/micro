mod signing_data;

#[macro_use]
pub mod slot_epoch_macros;

pub mod error;
mod globals;
mod pubsub;
pub mod slot_epoch;
mod sync_state;
mod topics;

pub mod block;
pub mod block_body;
pub mod block_header;
pub mod signed_block;
pub mod signed_block_header;

pub type Enr = discv5::enr::Enr<discv5::enr::CombinedKey>;

pub use globals::NetworkGlobals;
pub use pubsub::{PubsubMessage, SnappyTransform};
pub use sync_state::{BackFillState, SyncState};
pub use topics::{core_topics_to_subscribe, GossipEncoding, GossipKind, GossipTopic};
