//! The Ethereum 2.0 Wire Protocol
//!
//! This protocol is a purpose built Ethereum 2.0 libp2p protocol. It's role is to facilitate
//! direct peer-to-peer communication primarily for sending/receiving chain information for
//! syncing.

use futures::future::FutureExt;
use libp2p::swarm::{
    handler::ConnectionHandler, ConnectionId, NetworkBehaviour, NotifyHandler, PollParameters,
    ToSwarm,
};
use libp2p::swarm::{FromSwarm, SubstreamProtocol, THandlerInEvent};
use libp2p::PeerId;
pub(crate) use methods::{MetaData, Ping, RPCCodedResponse, RPCResponse};
pub(crate) use protocol::InboundRequest;
use slog::{crit, debug, o};
use std::marker::PhantomData;
use std::task::{Context, Poll};
use std::time::Duration;

pub use methods::{
    BlocksByRangeRequest, BlocksByRootRequest, GoodbyeReason, MaxRequestBlocks,
    RPCResponseErrorCode, ResponseTermination, StatusMessage, MAX_REQUEST_BLOCKS,
};
pub(crate) use outbound::OutboundRequest;
pub use protocol::{max_rpc_size, Protocol, RPCError};

pub mod methods;
mod outbound;
mod protocol;

/// Composite trait for a request id.
pub trait ReqId: Send + 'static + std::fmt::Debug + Copy + Clone {}
impl<T> ReqId for T where T: Send + 'static + std::fmt::Debug + Copy + Clone {}

/// Identifier of inbound and outbound substreams from the handler's perspective.
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub struct SubstreamId(usize);

/// RPC events sent from Micro.
#[derive(Debug, Clone)]
pub enum RPCSend<Id> {
    /// A request sent from Micro.
    ///
    /// The `Id` is given by the application making the request. These
    /// go over *outbound* connections.
    Request(Id, OutboundRequest),
    /// A response sent from Micro.
    ///
    /// The `SubstreamId` must correspond to the RPC-given ID of the original request received from the
    /// peer. The second parameter is a single chunk of a response. These go over *inbound*
    /// connections.
    Response(SubstreamId, RPCCodedResponse),
    /// Micro has requested to terminate the connection with a goodbye message.
    Shutdown(Id, GoodbyeReason),
}

/// RPC events received from outside Micro.
#[derive(Debug, Clone)]
pub enum RPCReceived<Id> {
    /// A request received from the outside.
    ///
    /// The `SubstreamId` is given by the `RPCHandler` as it identifies this request with the
    /// *inbound* substream over which it is managed.
    Request(SubstreamId, InboundRequest),
    /// A response received from the outside.
    ///
    /// The `Id` corresponds to the application given ID of the original request sent to the
    /// peer. The second parameter is a single chunk of a response. These go over *outbound*
    /// connections.
    Response(Id, RPCResponse),
    /// Marks a request as completed
    EndOfStream(Id, ResponseTermination),
}

impl<Id: std::fmt::Debug> std::fmt::Display for RPCSend<Id> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RPCSend::Request(id, req) => write!(f, "RPC Request(id: {:?}, {})", id, req),
            RPCSend::Response(id, res) => write!(f, "RPC Response(id: {:?}, {})", id, res),
            RPCSend::Shutdown(_id, reason) => write!(f, "Sending Goodbye: {}", reason),
        }
    }
}
