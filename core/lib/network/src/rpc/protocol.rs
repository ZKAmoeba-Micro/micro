use super::methods::*;
use crate::{
    rpc::{
        methods::{MaxErrorLen, ResponseTermination, MAX_ERROR_LEN},
        MaxRequestBlocks, MAX_REQUEST_BLOCKS,
    },
    types::{block::Block, signed_block::SignedBlock},
};
use futures::future::BoxFuture;
use futures::prelude::{AsyncRead, AsyncWrite};
use futures::{FutureExt, StreamExt};
use libp2p::core::{InboundUpgrade, UpgradeInfo};
use ssz::Encode;
use ssz_types::VariableList;
use std::io;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;
use strum::{AsRefStr, Display, EnumString, IntoStaticStr};
use tokio_io_timeout::TimeoutStream;
use tokio_util::{
    codec::Framed,
    compat::{Compat, FuturesAsyncReadCompatExt},
};
use tree_hash::Hash256;

lazy_static! {
    pub static ref BLOCKS_BY_ROOT_REQUEST_MIN: usize =
        VariableList::<Hash256, MaxRequestBlocks>::from(Vec::<Hash256>::new())
            .as_ssz_bytes()
            .len();
    pub static ref BLOCKS_BY_ROOT_REQUEST_MAX: usize =
        VariableList::<Hash256, MaxRequestBlocks>::from(vec![
            Hash256::zero();
            MAX_REQUEST_BLOCKS as usize
        ])
        .as_ssz_bytes()
        .len();
    pub static ref ERROR_TYPE_MIN: usize = VariableList::<u8, MaxErrorLen>::from(Vec::<u8>::new())
        .as_ssz_bytes()
        .len();
    pub static ref ERROR_TYPE_MAX: usize =
        VariableList::<u8, MaxErrorLen>::from(vec![0u8; MAX_ERROR_LEN as usize])
            .as_ssz_bytes()
            .len();
}

/// The protocol prefix the RPC protocol id.
const PROTOCOL_PREFIX: &str = "/micro/chain/req";
/// The number of seconds to wait for the first bytes of a request once a protocol has been
/// established before the stream is terminated.
const REQUEST_TIMEOUT: u64 = 15;

/// Returns the maximum bytes that can be sent across the RPC.
pub fn max_rpc_size(max_chunk_size: usize) -> usize {
    max_chunk_size
}

/// Protocol names to be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumString, AsRefStr, Display)]
#[strum(serialize_all = "snake_case")]
pub enum Protocol {
    /// The Status protocol name.
    Status,
    /// The Goodbye protocol name.
    Goodbye,
    /// The `BlocksByRange` protocol name.
    #[strum(serialize = "beacon_blocks_by_range")]
    BlocksByRange,
    /// The `BlocksByRoot` protocol name.
    #[strum(serialize = "beacon_blocks_by_root")]
    BlocksByRoot,
    /// The `Ping` protocol name.
    Ping,
    /// The `MetaData` protocol name.
    #[strum(serialize = "metadata")]
    MetaData,
}

/// RPC Encondings supported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Encoding {
    SSZSnappy,
}

/// All valid protocol name and version combinations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SupportedProtocol {
    StatusV1,
    GoodbyeV1,
    BlocksByRangeV1,
    BlocksByRootV1,
    PingV1,
    MetaDataV1,
}

impl SupportedProtocol {
    pub fn version_string(&self) -> &'static str {
        match self {
            SupportedProtocol::StatusV1 => "1",
            SupportedProtocol::GoodbyeV1 => "1",
            SupportedProtocol::BlocksByRangeV1 => "1",
            SupportedProtocol::BlocksByRootV1 => "1",
            SupportedProtocol::PingV1 => "1",
            SupportedProtocol::MetaDataV1 => "1",
        }
    }

    pub fn protocol(&self) -> Protocol {
        match self {
            SupportedProtocol::StatusV1 => Protocol::Status,
            SupportedProtocol::GoodbyeV1 => Protocol::Goodbye,
            SupportedProtocol::BlocksByRangeV1 => Protocol::BlocksByRange,
            SupportedProtocol::BlocksByRootV1 => Protocol::BlocksByRoot,
            SupportedProtocol::PingV1 => Protocol::Ping,
            SupportedProtocol::MetaDataV1 => Protocol::MetaData,
        }
    }

    fn currently_supported() -> Vec<ProtocolId> {
        vec![
            ProtocolId::new(Self::StatusV1, Encoding::SSZSnappy),
            ProtocolId::new(Self::GoodbyeV1, Encoding::SSZSnappy),
            ProtocolId::new(Self::BlocksByRangeV1, Encoding::SSZSnappy),
            ProtocolId::new(Self::BlocksByRootV1, Encoding::SSZSnappy),
            ProtocolId::new(Self::PingV1, Encoding::SSZSnappy),
            ProtocolId::new(Self::MetaDataV1, Encoding::SSZSnappy),
        ]
    }
}

impl std::fmt::Display for Encoding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let repr = match self {
            Encoding::SSZSnappy => "ssz_snappy",
        };
        f.write_str(repr)
    }
}

#[derive(Debug, Clone)]
pub struct RPCProtocol {
    pub max_rpc_size: usize,
    pub ttfb_timeout: Duration,
}

impl UpgradeInfo for RPCProtocol {
    type Info = ProtocolId;
    type InfoIter = Vec<Self::Info>;

    /// The list of supported RPC protocols for Micro.
    fn protocol_info(&self) -> Self::InfoIter {
        let supported_protocols = SupportedProtocol::currently_supported();

        supported_protocols
    }
}

/// Represents the ssz length bounds for RPC messages.
#[derive(Debug, PartialEq)]
pub struct RpcLimits {
    pub min: usize,
    pub max: usize,
}

impl RpcLimits {
    pub fn new(min: usize, max: usize) -> Self {
        Self { min, max }
    }

    /// Returns true if the given length is greater than `max_rpc_size` or out of
    /// bounds for the given ssz type, returns false otherwise.
    pub fn is_out_of_bounds(&self, length: usize, max_rpc_size: usize) -> bool {
        length > std::cmp::min(self.max, max_rpc_size) || length < self.min
    }
}

/// Tracks the types in a protocol id.
#[derive(Clone, Debug)]
pub struct ProtocolId {
    /// The protocol name and version
    pub versioned_protocol: SupportedProtocol,

    /// The encoding of the RPC.
    pub encoding: Encoding,

    /// The protocol id that is formed from the above fields.
    protocol_id: String,
}

impl AsRef<str> for ProtocolId {
    fn as_ref(&self) -> &str {
        self.protocol_id.as_ref()
    }
}

impl ProtocolId {
    /// Returns min and max size for messages of given protocol id requests.
    pub fn rpc_request_limits(&self) -> RpcLimits {
        match self.versioned_protocol.protocol() {
            Protocol::Status => RpcLimits::new(
                <StatusMessage as Encode>::ssz_fixed_len(),
                <StatusMessage as Encode>::ssz_fixed_len(),
            ),
            Protocol::Goodbye => RpcLimits::new(
                <GoodbyeReason as Encode>::ssz_fixed_len(),
                <GoodbyeReason as Encode>::ssz_fixed_len(),
            ),
            Protocol::BlocksByRange => RpcLimits::new(
                <BlocksByRangeRequest as Encode>::ssz_fixed_len(),
                <BlocksByRangeRequest as Encode>::ssz_fixed_len(),
            ),
            Protocol::BlocksByRoot => {
                RpcLimits::new(*BLOCKS_BY_ROOT_REQUEST_MIN, *BLOCKS_BY_ROOT_REQUEST_MAX)
            }
            Protocol::Ping => RpcLimits::new(
                <Ping as Encode>::ssz_fixed_len(),
                <Ping as Encode>::ssz_fixed_len(),
            ),
            Protocol::MetaData => RpcLimits::new(0, 0), // Metadata requests are empty
        }
    }

    /// Returns `true` if the given `ProtocolId` should expect `context_bytes` in the
    /// beginning of the stream, else returns `false`.
    pub fn has_context_bytes(&self) -> bool {
        match self.versioned_protocol {
            SupportedProtocol::StatusV1
            | SupportedProtocol::BlocksByRootV1
            | SupportedProtocol::BlocksByRangeV1
            | SupportedProtocol::PingV1
            | SupportedProtocol::MetaDataV1
            | SupportedProtocol::GoodbyeV1 => false,
        }
    }
}

/// An RPC protocol ID.
impl ProtocolId {
    pub fn new(versioned_protocol: SupportedProtocol, encoding: Encoding) -> Self {
        let protocol_id = format!(
            "{}/{}/{}/{}",
            PROTOCOL_PREFIX,
            versioned_protocol.protocol(),
            versioned_protocol.version_string(),
            encoding
        );

        ProtocolId {
            versioned_protocol,
            encoding,
            protocol_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum InboundRequest {
    Status(StatusMessage),
    Goodbye(GoodbyeReason),
    BlocksByRange(BlocksByRangeRequest),
    BlocksByRoot(BlocksByRootRequest),
    Ping(Ping),
    MetaData(MetadataRequest),
}

/// Implements the encoding per supported protocol for `RPCRequest`.
impl InboundRequest {
    /* These functions are used in the handler for stream management */

    /// Number of responses expected for this request.
    pub fn expected_responses(&self) -> u64 {
        match self {
            InboundRequest::Status(_) => 1,
            InboundRequest::Goodbye(_) => 0,
            InboundRequest::BlocksByRange(req) => req.count,
            InboundRequest::BlocksByRoot(req) => req.block_roots.len() as u64,
            InboundRequest::Ping(_) => 1,
            InboundRequest::MetaData(_) => 1,
        }
    }

    /// Gives the corresponding `SupportedProtocol` to this request.
    pub fn versioned_protocol(&self) -> SupportedProtocol {
        match self {
            InboundRequest::Status(_) => SupportedProtocol::StatusV1,
            InboundRequest::Goodbye(_) => SupportedProtocol::GoodbyeV1,
            InboundRequest::BlocksByRange(_) => SupportedProtocol::BlocksByRangeV1,
            InboundRequest::BlocksByRoot(_) => SupportedProtocol::BlocksByRootV1,
            InboundRequest::Ping(_) => SupportedProtocol::PingV1,
            InboundRequest::MetaData(_) => SupportedProtocol::MetaDataV1,
        }
    }

    /// Returns the `ResponseTermination` type associated with the request if a stream gets
    /// terminated.
    pub fn stream_termination(&self) -> ResponseTermination {
        match self {
            // this only gets called after `multiple_responses()` returns true. Therefore, only
            // variants that have `multiple_responses()` can have values.
            InboundRequest::BlocksByRange(_) => ResponseTermination::BlocksByRange,
            InboundRequest::BlocksByRoot(_) => ResponseTermination::BlocksByRoot,
            InboundRequest::Status(_) => unreachable!(),
            InboundRequest::Goodbye(_) => unreachable!(),
            InboundRequest::Ping(_) => unreachable!(),
            InboundRequest::MetaData(_) => unreachable!(),
        }
    }
}

/// Error in RPC Encoding/Decoding.
#[derive(Debug, Clone, PartialEq, IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum RPCError {
    /// Error when decoding the raw buffer from ssz.
    // NOTE: in the future a ssz::DecodeError should map to an InvalidData error
    #[strum(serialize = "decode_error")]
    SSZDecodeError(ssz::DecodeError),
    /// IO Error.
    IoError(String),
    /// The peer returned a valid response but the response indicated an error.
    ErrorResponse(RPCResponseErrorCode, String),
    /// Timed out waiting for a response.
    StreamTimeout,
    /// Peer does not support the protocol.
    UnsupportedProtocol,
    /// Stream ended unexpectedly.
    IncompleteStream,
    /// Peer sent invalid data.
    InvalidData(String),
    /// An error occurred due to internal reasons. Ex: timer failure.
    InternalError(&'static str),
    /// Negotiation with this peer timed out.
    NegotiationTimeout,
    /// Handler rejected this request.
    HandlerRejected,
    /// We have intentionally disconnected.
    Disconnected,
}

impl From<ssz::DecodeError> for RPCError {
    #[inline]
    fn from(err: ssz::DecodeError) -> Self {
        RPCError::SSZDecodeError(err)
    }
}
impl From<tokio::time::error::Elapsed> for RPCError {
    fn from(_: tokio::time::error::Elapsed) -> Self {
        RPCError::StreamTimeout
    }
}

impl From<io::Error> for RPCError {
    fn from(err: io::Error) -> Self {
        RPCError::IoError(err.to_string())
    }
}

// Error trait is required for `ProtocolsHandler`
impl std::fmt::Display for RPCError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            RPCError::SSZDecodeError(ref err) => write!(f, "Error while decoding ssz: {:?}", err),
            RPCError::InvalidData(ref err) => write!(f, "Peer sent unexpected data: {}", err),
            RPCError::IoError(ref err) => write!(f, "IO Error: {}", err),
            RPCError::ErrorResponse(ref code, ref reason) => write!(
                f,
                "RPC response was an error: {} with reason: {}",
                code, reason
            ),
            RPCError::StreamTimeout => write!(f, "Stream Timeout"),
            RPCError::UnsupportedProtocol => write!(f, "Peer does not support the protocol"),
            RPCError::IncompleteStream => write!(f, "Stream ended unexpectedly"),
            RPCError::InternalError(ref err) => write!(f, "Internal error: {}", err),
            RPCError::NegotiationTimeout => write!(f, "Negotiation timeout"),
            RPCError::HandlerRejected => write!(f, "Handler rejected the request"),
            RPCError::Disconnected => write!(f, "Gracefully Disconnected"),
        }
    }
}

impl std::error::Error for RPCError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            // NOTE: this does have a source
            RPCError::SSZDecodeError(_) => None,
            RPCError::IoError(_) => None,
            RPCError::StreamTimeout => None,
            RPCError::UnsupportedProtocol => None,
            RPCError::IncompleteStream => None,
            RPCError::InvalidData(_) => None,
            RPCError::InternalError(_) => None,
            RPCError::ErrorResponse(_, _) => None,
            RPCError::NegotiationTimeout => None,
            RPCError::HandlerRejected => None,
            RPCError::Disconnected => None,
        }
    }
}

impl std::fmt::Display for InboundRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InboundRequest::Status(status) => write!(f, "Status Message: {}", status),
            InboundRequest::Goodbye(reason) => write!(f, "Goodbye: {}", reason),
            InboundRequest::BlocksByRange(req) => write!(f, "Blocks by range: {:?}", req),
            InboundRequest::BlocksByRoot(req) => write!(f, "Blocks by root: {:?}", req),
            InboundRequest::Ping(ping) => write!(f, "Ping: {}", ping.data),
            InboundRequest::MetaData(_) => write!(f, "MetaData request"),
        }
    }
}

impl RPCError {
    /// Get a `str` representation of the error.
    /// Used for metrics.
    pub fn as_static_str(&self) -> &'static str {
        match self {
            RPCError::ErrorResponse(ref code, ..) => code.into(),
            e => e.into(),
        }
    }
}
