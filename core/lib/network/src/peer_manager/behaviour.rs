use crate::discovery::Discovery;
use crate::peer_manager::PeerManager;

use crate::types::SnappyTransform;

use libp2p::gossipsub;
use libp2p::identify;
use libp2p::swarm::NetworkBehaviour;

pub type SubscriptionFilter =
    gossipsub::MaxCountSubscriptionFilter<gossipsub::WhitelistSubscriptionFilter>;
pub type Gossipsub = gossipsub::Behaviour<SnappyTransform, SubscriptionFilter>;
