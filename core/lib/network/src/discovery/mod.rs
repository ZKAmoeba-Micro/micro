//! The discovery sub-behaviour.
//!
//! This module creates a libp2p dummy-behaviour built around the discv5 protocol. It handles
//! queries and manages access to the discovery routing table.

pub(crate) mod enr;
pub mod enr_ext;

// Allow external use of the ENR builder
use crate::{error, Enr, NetworkConfig, NetworkGlobals};
use crate::{metrics, ClearDialError};
use discv5::{enr::NodeId, Discv5, Discv5Event};
pub use enr::{
    build_enr, create_enr_builder_from_config, load_enr_from_disk, use_or_load_enr, CombinedKey,
};
pub use enr_ext::{peer_id_to_node_id, CombinedKeyExt, EnrExt};
pub use libp2p::identity::{Keypair, PublicKey};

use futures::prelude::*;
use futures::stream::FuturesUnordered;
use libp2p::multiaddr::Protocol;
use libp2p::swarm::behaviour::{DialFailure, FromSwarm};
use libp2p::swarm::THandlerInEvent;
pub use libp2p::{
    core::{ConnectedPoint, Multiaddr},
    identity::PeerId,
    swarm::{
        dummy::ConnectionHandler, ConnectionId, DialError, NetworkBehaviour, NotifyHandler,
        PollParameters, SubstreamProtocol, ToSwarm,
    },
};
use lru::LruCache;
use slog::{crit, debug, error, info, trace, warn};
use std::{
    collections::{HashMap, VecDeque},
    net::{IpAddr, SocketAddr},
    path::Path,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::{Duration, Instant},
};
use tokio::sync::mpsc;

/// Local ENR storage filename.
pub const ENR_FILENAME: &str = "enr.dat";
/// Target number of peers to search for given a grouped subnet query.
const TARGET_PEERS_FOR_GROUPED_QUERY: usize = 6;
/// Number of times to attempt a discovery request.
const MAX_DISCOVERY_RETRY: usize = 3;
/// The maximum number of concurrent subnet discovery queries.
/// Note: we always allow a single FindPeers query, so we would be
/// running a maximum of `MAX_CONCURRENT_SUBNET_QUERIES + 1`
/// discovery queries at a time.
const MAX_CONCURRENT_SUBNET_QUERIES: usize = 2;
/// The max number of subnets to search for in a single subnet discovery query.
const MAX_SUBNETS_IN_QUERY: usize = 3;
/// The number of closest peers to search for when doing a regular peer search.
///
/// We could reduce this constant to speed up queries however at the cost of security. It will
/// make it easier to peers to eclipse this node. Kademlia suggests a value of 16.
pub const FIND_NODE_QUERY_CLOSEST_PEERS: usize = 16;
/// The threshold for updating `min_ttl` on a connected peer.
const DURATION_DIFFERENCE: Duration = Duration::from_millis(1);

/// A query has completed. This result contains a mapping of discovered peer IDs to the `min_ttl`
/// of the peer if it is specified.
#[derive(Debug)]
pub struct DiscoveredPeers {
    pub peers: HashMap<PeerId, Option<Instant>>,
}

#[derive(Debug, Clone, PartialEq)]
enum QueryType {
    /// We are searching for more peers without ENR or time constraints.
    FindPeers,
}

/// The result of a query.
struct QueryResult {
    query_type: QueryType,
    result: Result<Vec<Enr>, discv5::QueryError>,
}

// Awaiting the event stream future
enum EventStream {
    /// Awaiting an event stream to be generated. This is required due to the poll nature of
    /// `Discovery`
    Awaiting(
        Pin<
            Box<
                dyn Future<Output = Result<mpsc::Receiver<Discv5Event>, discv5::Discv5Error>>
                    + Send,
            >,
        >,
    ),
    /// The future has completed.
    Present(mpsc::Receiver<Discv5Event>),
    // The future has failed or discv5 has been disabled. There are no events from discv5.
    InActive,
}

/// The main discovery service. This can be disabled via CLI arguements. When disabled the
/// underlying processes are not started, but this struct still maintains our current ENR.
pub struct Discovery {
    /// A collection of seen live ENRs for quick lookup and to map peer-id's to ENRs.
    cached_enrs: LruCache<PeerId, Enr>,

    /// The directory where the ENR is stored.
    enr_dir: String,

    /// The handle for the underlying discv5 Server.
    ///
    /// This is behind a Reference counter to allow for futures to be spawned and polled with a
    /// static lifetime.
    discv5: Discv5,

    /// A collection of network constants that can be read from other threads.
    network_globals: Arc<NetworkGlobals>,

    /// Indicates if we are actively searching for peers. We only allow a single FindPeers query at
    /// a time, regardless of the query concurrency.
    find_peer_active: bool,

    /// Active discovery queries.
    active_queries: FuturesUnordered<std::pin::Pin<Box<dyn Future<Output = QueryResult> + Send>>>,

    /// The discv5 event stream.
    event_stream: EventStream,

    /// Indicates if the discovery service has been started. When the service is disabled, this is
    /// always false.
    pub started: bool,

    /// This keeps track of whether an external UDP port change should also indicate an internal
    /// TCP port change. As we cannot detect our external TCP port, we assume that the external UDP
    /// port is also our external TCP port. This assumption only holds if the user has not
    /// explicitly set their ENR TCP port via the CLI config. The first indicates tcp4 and the
    /// second indicates tcp6.
    update_tcp_port: (bool, bool),

    /// Logger for the discovery behaviour.
    log: slog::Logger,
}

impl Discovery {
    /// NOTE: Creating discovery requires running within a tokio execution environment.
    pub async fn new(
        local_key: Keypair,
        config: &NetworkConfig,
        network_globals: Arc<NetworkGlobals>,
        log: &slog::Logger,
    ) -> error::Result<Self> {
        let log = log.clone();

        let enr_dir = match config.network_dir.to_str() {
            Some(path) => String::from(path),
            None => String::from(""),
        };

        let local_enr = network_globals.local_enr.read().clone();
        let local_node_id = local_enr.node_id();

        info!(log, "ENR Initialised"; "enr" => local_enr.to_base64(), "seq" => local_enr.seq(), "id"=> %local_enr.node_id(),
              "ip4" => ?local_enr.ip4(), "udp4"=> ?local_enr.udp4(), "tcp4" => ?local_enr.tcp4(), "tcp6" => ?local_enr.tcp6(), "udp6" => ?local_enr.udp6()
        );

        // convert the keypair into an ENR key
        let enr_key: CombinedKey = CombinedKey::from_libp2p(local_key)?;

        let mut discv5 = Discv5::new(local_enr, enr_key, config.discv5_config.clone())
            .map_err(|e| format!("Discv5 service failed. Error: {:?}", e))?;

        // Add bootnodes to routing table
        for bootnode_enr in config.boot_nodes_enr.clone() {
            if bootnode_enr.node_id() == local_node_id {
                // If we are a boot node, ignore adding it to the routing table
                continue;
            }
            debug!(
                log,
                "Adding node to routing table";
                "node_id" => %bootnode_enr.node_id(),
                "peer_id" => %bootnode_enr.peer_id(),
                "ip" => ?bootnode_enr.ip4(),
                "udp" => ?bootnode_enr.udp4(),
                "tcp" => ?bootnode_enr.tcp4()
            );
            let repr = bootnode_enr.to_string();
            let _ = discv5.add_enr(bootnode_enr).map_err(|e| {
                error!(
                    log,
                    "Could not add peer to the local routing table";
                    "addr" => repr,
                    "error" => e.to_string(),
                )
            });
        }

        // Start the discv5 service and obtain an event stream
        let event_stream = if !config.disable_discovery {
            discv5.start().map_err(|e| e.to_string()).await?;
            debug!(log, "Discovery service started");
            EventStream::Awaiting(Box::pin(discv5.event_stream()))
        } else {
            EventStream::InActive
        };

        if !config.boot_nodes_multiaddr.is_empty() {
            info!(log, "Contacting Multiaddr boot-nodes for their ENR");
        }

        // get futures for requesting the Enrs associated to these multiaddr and wait for their
        // completion
        let mut fut_coll = config
            .boot_nodes_multiaddr
            .iter()
            .map(|addr| addr.to_string())
            // request the ENR for this multiaddr and keep the original for logging
            .map(|addr| {
                futures::future::join(
                    discv5.request_enr(addr.clone()),
                    futures::future::ready(addr),
                )
            })
            .collect::<FuturesUnordered<_>>();

        while let Some((result, original_addr)) = fut_coll.next().await {
            match result {
                Ok(enr) => {
                    debug!(
                        log,
                        "Adding node to routing table";
                        "node_id" => %enr.node_id(),
                        "peer_id" => %enr.peer_id(),
                        "ip" => ?enr.ip4(),
                        "udp" => ?enr.udp4(),
                        "tcp" => ?enr.tcp4()
                    );
                    let _ = discv5.add_enr(enr).map_err(|e| {
                        error!(
                            log,
                            "Could not add peer to the local routing table";
                            "addr" => original_addr.to_string(),
                            "error" => e.to_string(),
                        )
                    });
                }
                Err(e) => {
                    error!(log, "Error getting mapping to ENR"; "multiaddr" => original_addr.to_string(), "error" => e.to_string())
                }
            }
        }

        let update_tcp_port = (
            config.enr_tcp4_port.is_none(),
            config.enr_tcp6_port.is_none(),
        );

        Ok(Self {
            cached_enrs: LruCache::new(50),
            network_globals,
            find_peer_active: false,
            active_queries: FuturesUnordered::new(),
            discv5,
            event_stream,
            started: !config.disable_discovery,
            update_tcp_port,
            log,
            enr_dir,
        })
    }

    /// Return the nodes local ENR.
    pub fn local_enr(&self) -> Enr {
        self.discv5.local_enr()
    }

    /// Return the cached enrs.
    pub fn cached_enrs(&self) -> impl Iterator<Item = (&PeerId, &Enr)> {
        self.cached_enrs.iter()
    }

    /// Removes a cached ENR from the list.
    pub fn remove_cached_enr(&mut self, peer_id: &PeerId) -> Option<Enr> {
        self.cached_enrs.pop(peer_id)
    }

    /// This adds a new `FindPeers` query to the queue if one doesn't already exist.
    /// The `target_peers` parameter informs discovery to end the query once the target is found.
    /// The maximum this can be is 16.
    pub fn discover_peers(&mut self, target_peers: usize) {
        // If the discv5 service isn't running or we are in the process of a query, don't bother queuing a new one.
        if !self.started || self.find_peer_active {
            return;
        }
        // Immediately start a FindNode query
        let target_peers = std::cmp::min(FIND_NODE_QUERY_CLOSEST_PEERS, target_peers);
        debug!(self.log, "Starting a peer discovery request"; "target_peers" => target_peers );
        self.find_peer_active = true;
        self.start_query(QueryType::FindPeers, target_peers, |_| true);
    }

    /// Add an ENR to the routing table of the discovery mechanism.
    pub fn add_enr(&mut self, enr: Enr) {
        // add the enr to seen caches
        self.cached_enrs.put(enr.peer_id(), enr.clone());

        if let Err(e) = self.discv5.add_enr(enr) {
            debug!(
                self.log,
                "Could not add peer to the local routing table";
                "error" => %e
            )
        }
    }

    /// Returns an iterator over all enr entries in the DHT.
    pub fn table_entries_enr(&self) -> Vec<Enr> {
        self.discv5.table_entries_enr()
    }

    /// Returns the ENR of a known peer if it exists.
    pub fn enr_of_peer(&mut self, peer_id: &PeerId) -> Option<Enr> {
        // first search the local cache
        if let Some(enr) = self.cached_enrs.get(peer_id) {
            return Some(enr.clone());
        }
        // not in the local cache, look in the routing table
        if let Ok(node_id) = enr_ext::peer_id_to_node_id(peer_id) {
            self.discv5.find_enr(&node_id)
        } else {
            None
        }
    }

    /// Updates the local ENR TCP port.
    /// There currently isn't a case to update the address here. We opt for discovery to
    /// automatically update the external address.
    ///
    /// If the external address needs to be modified, use `update_enr_udp_socket.
    pub fn update_enr_tcp_port(&mut self, port: u16) -> Result<(), String> {
        self.discv5
            .enr_insert("tcp", &port)
            .map_err(|e| format!("{:?}", e))?;

        // replace the global version
        *self.network_globals.local_enr.write() = self.discv5.local_enr();
        // persist modified enr to disk
        enr::save_enr_to_disk(Path::new(&self.enr_dir), &self.local_enr(), &self.log);
        Ok(())
    }

    /// Updates the local ENR UDP socket.
    ///
    /// This is with caution. Discovery should automatically maintain this. This should only be
    /// used when automatic discovery is disabled.
    pub fn update_enr_udp_socket(&mut self, socket_addr: SocketAddr) -> Result<(), String> {
        const IS_TCP: bool = false;
        if self.discv5.update_local_enr_socket(socket_addr, IS_TCP) {
            // persist modified enr to disk
            enr::save_enr_to_disk(Path::new(&self.enr_dir), &self.local_enr(), &self.log);
        }
        *self.network_globals.local_enr.write() = self.discv5.local_enr();
        Ok(())
    }

    // Bans a peer and it's associated seen IP addresses.
    pub fn ban_peer(&mut self, peer_id: &PeerId, ip_addresses: Vec<IpAddr>) {
        // first try and convert the peer_id to a node_id.
        if let Ok(node_id) = peer_id_to_node_id(peer_id) {
            // If we could convert this peer id, remove it from the DHT and ban it from discovery.
            self.discv5.ban_node(&node_id, None);
            // Remove the node from the routing table.
            self.discv5.remove_node(&node_id);
        }

        for ip_address in ip_addresses {
            self.discv5.ban_ip(ip_address, None);
        }
    }

    /// Unbans the peer in discovery.
    pub fn unban_peer(&mut self, peer_id: &PeerId, ip_addresses: Vec<IpAddr>) {
        // first try and convert the peer_id to a node_id.
        if let Ok(node_id) = peer_id_to_node_id(peer_id) {
            self.discv5.ban_node_remove(&node_id);
        }

        for ip_address in ip_addresses {
            self.discv5.ban_ip_remove(&ip_address);
        }
    }

    ///  Marks node as disconnected in the DHT, freeing up space for other nodes, this also removes
    ///  nodes from the cached ENR list.
    pub fn disconnect_peer(&mut self, peer_id: &PeerId) {
        if let Ok(node_id) = peer_id_to_node_id(peer_id) {
            self.discv5.disconnect_node(&node_id);
        }
        // Remove the peer from the cached list, to prevent redialing disconnected
        // peers.
        self.cached_enrs.pop(peer_id);
    }

    /* Internal Functions */

    // Returns a boolean indicating if we are currently processing the maximum number of
    // concurrent subnet queries or not.
    fn at_capacity(&self) -> bool {
        self.active_queries
            .len()
            .saturating_sub(self.find_peer_active as usize) // We only count active subnet queries
            >= MAX_CONCURRENT_SUBNET_QUERIES
    }

    /// Search for a specified number of new peers using the underlying discovery mechanism.
    ///
    /// This can optionally search for peers for a given predicate. Regardless of the predicate
    /// given, this will only search for peers on the same enr_fork_id as specified in the local
    /// ENR.
    fn start_query(
        &mut self,
        query: QueryType,
        target_peers: usize,
        additional_predicate: impl Fn(&Enr) -> bool + Send + 'static,
    ) {
        // Generate a random target node id.
        let random_node = NodeId::random();

        // General predicate
        let predicate: Box<dyn Fn(&Enr) -> bool + Send> =
            Box::new(move |enr: &Enr| additional_predicate(enr));

        // Build the future
        let query_future = self
            .discv5
            .find_node_predicate(random_node, predicate, target_peers)
            .map(|v| QueryResult {
                query_type: query,
                result: v,
            });

        // Add the future to active queries, to be executed.
        self.active_queries.push(Box::pin(query_future));
    }

    /// Process the completed QueryResult returned from discv5.
    fn process_completed_queries(
        &mut self,
        query: QueryResult,
    ) -> Option<HashMap<PeerId, Option<Instant>>> {
        match query.query_type {
            QueryType::FindPeers => {
                self.find_peer_active = false;
                match query.result {
                    Ok(r) if r.is_empty() => {
                        debug!(self.log, "Discovery query yielded no results.");
                    }
                    Ok(r) => {
                        debug!(self.log, "Discovery query completed"; "peers_found" => r.len());
                        let mut results: HashMap<_, Option<Instant>> = HashMap::new();
                        r.iter().for_each(|enr| {
                            // cache the found ENR's
                            self.cached_enrs.put(enr.peer_id(), enr.clone());
                            results.insert(enr.peer_id(), None);
                        });
                        return Some(results);
                    }
                    Err(e) => {
                        warn!(self.log, "Discovery query failed"; "error" => %e);
                    }
                }
            }
        }
        None
    }

    /// Drives the queries returning any results from completed queries.
    fn poll_queries(&mut self, cx: &mut Context) -> Option<HashMap<PeerId, Option<Instant>>> {
        while let Poll::Ready(Some(query_result)) = self.active_queries.poll_next_unpin(cx) {
            let result = self.process_completed_queries(query_result);
            if result.is_some() {
                return result;
            }
        }
        None
    }
}

/* NetworkBehaviour Implementation */

impl NetworkBehaviour for Discovery {
    // Discovery is not a real NetworkBehaviour...
    type ConnectionHandler = ConnectionHandler;
    type ToSwarm = DiscoveredPeers;

    fn handle_established_inbound_connection(
        &mut self,
        _connection_id: ConnectionId,
        _peer: PeerId,
        _local_addr: &Multiaddr,
        _remote_addr: &Multiaddr,
    ) -> Result<libp2p::swarm::THandler<Self>, libp2p::swarm::ConnectionDenied> {
        // TODO: we might want to check discovery's banned ips here in the future.
        Ok(ConnectionHandler)
    }

    fn handle_established_outbound_connection(
        &mut self,
        _connection_id: ConnectionId,
        _peer: PeerId,
        _addr: &Multiaddr,
        _role_override: libp2p::core::Endpoint,
    ) -> Result<libp2p::swarm::THandler<Self>, libp2p::swarm::ConnectionDenied> {
        Ok(ConnectionHandler)
    }

    fn on_connection_handler_event(
        &mut self,
        _peer_id: PeerId,
        _connection_id: ConnectionId,
        _event: void::Void,
    ) {
    }

    fn handle_pending_outbound_connection(
        &mut self,
        _connection_id: ConnectionId,
        maybe_peer: Option<PeerId>,
        _addresses: &[Multiaddr],
        _effective_role: libp2p::core::Endpoint,
    ) -> Result<Vec<Multiaddr>, libp2p::swarm::ConnectionDenied> {
        if let Some(enr) = maybe_peer.and_then(|peer_id| self.enr_of_peer(&peer_id)) {
            // ENR's may have multiple Multiaddrs. The multi-addr associated with the UDP
            // port is removed, which is assumed to be associated with the discv5 protocol (and
            // therefore irrelevant for other libp2p components).
            Ok(enr.multiaddr_tcp())
        } else {
            Ok(vec![])
        }
    }

    // Main execution loop to drive the behaviour
    fn poll(
        &mut self,
        cx: &mut Context,
        _: &mut impl PollParameters,
    ) -> Poll<ToSwarm<Self::ToSwarm, THandlerInEvent<Self>>> {
        if !self.started {
            return Poll::Pending;
        }

        // Drive the queries and return any results from completed queries
        if let Some(peers) = self.poll_queries(cx) {
            // return the result to the peer manager
            return Poll::Ready(ToSwarm::GenerateEvent(DiscoveredPeers { peers }));
        }

        // Process the server event stream
        match self.event_stream {
            EventStream::Awaiting(ref mut fut) => {
                // Still awaiting the event stream, poll it
                if let Poll::Ready(event_stream) = fut.poll_unpin(cx) {
                    match event_stream {
                        Ok(stream) => {
                            debug!(self.log, "Discv5 event stream ready");
                            self.event_stream = EventStream::Present(stream);
                        }
                        Err(e) => {
                            slog::crit!(self.log, "Discv5 event stream failed"; "error" => %e);
                            self.event_stream = EventStream::InActive;
                        }
                    }
                }
            }
            EventStream::InActive => {} // ignore checking the stream
            EventStream::Present(ref mut stream) => {
                while let Poll::Ready(Some(event)) = stream.poll_recv(cx) {
                    match event {
                        // We filter out unwanted discv5 events here and only propagate useful results to
                        // the peer manager.
                        Discv5Event::Discovered(_enr) => {
                            // Peers that get discovered during a query but are not contactable or
                            // don't match a predicate can end up here. For debugging purposes we
                            // log these to see if we are unnecessarily dropping discovered peers
                            /*
                            if enr.eth2() == self.local_enr().eth2() {
                                trace!(self.log, "Peer found in process of query"; "peer_id" => format!("{}", enr.peer_id()), "tcp_socket" => enr.tcp_socket());
                            } else {
                            // this is temporary warning for debugging the DHT
                            warn!(self.log, "Found peer during discovery not on correct fork"; "peer_id" => format!("{}", enr.peer_id()), "tcp_socket" => enr.tcp_socket());
                            }
                            */
                        }
                        Discv5Event::SocketUpdated(socket_addr) => {
                            info!(self.log, "Address updated"; "ip" => %socket_addr.ip(), "udp_port" => %socket_addr.port());
                            metrics::inc_counter(&metrics::ADDRESS_UPDATE_COUNT);
                            metrics::check_nat();
                            // Discv5 will have updated our local ENR. We save the updated version
                            // to disk.

                            if (self.update_tcp_port.0 && socket_addr.is_ipv4())
                                || (self.update_tcp_port.1 && socket_addr.is_ipv6())
                            {
                                // Update the TCP port in the ENR
                                self.discv5.update_local_enr_socket(socket_addr, true);
                            }
                            let enr = self.discv5.local_enr();
                            enr::save_enr_to_disk(Path::new(&self.enr_dir), &enr, &self.log);
                            // update  network globals
                            *self.network_globals.local_enr.write() = enr;
                            // A new UDP socket has been detected.
                            // Build a multiaddr to report to libp2p
                            let addr = match socket_addr.ip() {
                                IpAddr::V4(v4_addr) => {
                                    self.network_globals.listen_port_tcp4().map(|tcp4_port| {
                                        Multiaddr::from(v4_addr).with(Protocol::Tcp(tcp4_port))
                                    })
                                }
                                IpAddr::V6(v6_addr) => {
                                    self.network_globals.listen_port_tcp6().map(|tcp6_port| {
                                        Multiaddr::from(v6_addr).with(Protocol::Tcp(tcp6_port))
                                    })
                                }
                            };

                            if let Some(address) = addr {
                                // NOTE: This doesn't actually track the external TCP port. More sophisticated NAT handling
                                // should handle this.
                                return Poll::Ready(ToSwarm::NewExternalAddrCandidate(address));
                            }
                        }
                        Discv5Event::EnrAdded { .. }
                        | Discv5Event::TalkRequest(_)
                        | Discv5Event::NodeInserted { .. }
                        | Discv5Event::SessionEstablished { .. } => {} // Ignore all other discv5 server events
                    }
                }
            }
        }
        Poll::Pending
    }

    fn on_swarm_event(&mut self, event: FromSwarm<Self::ConnectionHandler>) {
        match event {
            FromSwarm::DialFailure(DialFailure { peer_id, error, .. }) => {
                self.on_dial_failure(peer_id, error)
            }
            FromSwarm::ConnectionEstablished(_)
            | FromSwarm::ConnectionClosed(_)
            | FromSwarm::AddressChange(_)
            | FromSwarm::ListenFailure(_)
            | FromSwarm::NewListener(_)
            | FromSwarm::NewListenAddr(_)
            | FromSwarm::ExpiredListenAddr(_)
            | FromSwarm::ListenerError(_)
            | FromSwarm::ListenerClosed(_)
            | FromSwarm::NewExternalAddrCandidate(_)
            | FromSwarm::ExternalAddrExpired(_)
            | FromSwarm::ExternalAddrConfirmed(_) => {
                // Ignore events not relevant to discovery
            }
        }
    }
}

impl Discovery {
    fn on_dial_failure(&mut self, peer_id: Option<PeerId>, error: &DialError) {
        if let Some(peer_id) = peer_id {
            match error {
                DialError::LocalPeerId { .. }
                | DialError::Denied { .. }
                | DialError::NoAddresses
                | DialError::Transport(_)
                | DialError::WrongPeerId { .. } => {
                    // set peer as disconnected in discovery DHT
                    debug!(self.log, "Marking peer disconnected in DHT"; "peer_id" => %peer_id, "error" => %ClearDialError(error));
                    self.disconnect_peer(&peer_id);
                }
                DialError::DialPeerConditionFalse(_) | DialError::Aborted => {}
            }
        }
    }
}
