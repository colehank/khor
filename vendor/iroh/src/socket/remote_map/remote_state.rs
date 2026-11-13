use std::{
    collections::{BTreeSet, VecDeque},
    net::SocketAddr,
    pin::Pin,
    sync::Arc,
    task::Poll,
};

use iroh_base::{EndpointId, TransportAddr};
use n0_error::StackResultExt;
use n0_future::{
    FuturesUnordered, FuturesUnorderedBounded, MaybeFuture, MergeUnbounded, Stream, StreamExt,
    boxed::BoxStream,
    future::{Boxed, now_or_never},
    task::JoinSet,
    time::{self, Duration, Instant},
};
use n0_watcher::Watcher;
use noq::{Closed, PathStats, PathStatus, WeakConnectionHandle};
use noq_proto::{PathError, PathEvent as NoqPathEvent, PathId, n0_nat_traversal};
use rustc_hash::FxHashMap;
use smallvec::{SmallVec, smallvec};
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;
use tracing::{Instrument, Level, Span, debug, error, event, info_span, instrument, trace, warn};

use self::path_state::RemotePathState;
pub(crate) use self::path_watcher::PathStateReceiver;
pub use self::{
    path_watcher::{
        Path, PathEvent, PathEventStream, PathList, PathListIter, PathListStream, PathMeasurement,
    },
    remote_info::{RemoteInfo, TransportAddrInfo, TransportAddrUsage},
};
use super::{MappedAddrs, Source};
use crate::{
    address_lookup::{AddressLookupFailed, AddressLookupServices, Item as AddressLookupItem},
    endpoint::DirectAddr,
    socket::{
        Metrics as SocketMetrics, RELAY_PATH_MAX_IDLE_TIMEOUT,
        remote_map::remote_state::path_watcher::PathStateSender,
        transports::{self, OwnedTransmit, TransportsSender},
    },
};

mod path_state;
mod path_watcher;
mod remote_info;

/// How often to attempt holepunching.
///
/// If there have been no changes to the NAT address candidates, holepunching will not be
/// attempted more frequently than at this interval.
const HOLEPUNCH_ATTEMPTS_INTERVAL: Duration = Duration::from_secs(5);

/// The latency at or under which we don't try to upgrade to a better path.
const GOOD_ENOUGH_LATENCY: Duration = Duration::from_millis(10);

// TODO: use this
// /// How long since the last activity we try to keep an established endpoint peering alive.
// ///
// /// It's also the idle time at which we stop doing QAD queries to keep NAT mappings alive.
// pub(super) const SESSION_ACTIVE_TIMEOUT: Duration = Duration::from_secs(45);

/// How often we try to upgrade to a better path.
///
/// Even if we have some non-relay route that works.
const UPGRADE_INTERVAL: Duration = Duration::from_secs(60);

/// The time after which an idle [`RemoteStateActor`] stops.
///
/// The actor only enters the idle state if no connections are active and no inbox senders exist
/// apart from the one stored in the endpoint map. Stopping and restarting the actor in this state
/// is not an issue; a timeout here serves the purpose of not stopping-and-recreating actors
/// in a high frequency, and to keep data about previous path around for subsequent connections.
const ACTOR_MAX_IDLE_TIMEOUT: Duration = Duration::from_secs(60);

/// khor patch 17 (vendor/iroh/KHOR.md): how often, while a connection is open, the selector
/// is run again on what the paths measure now.
///
/// Upstream selects only when a path opens or is abandoned, so a path that slows down or goes
/// silent keeps carrying the connection until noq abandons it at its idle timeout (15 s).  The
/// tick sends nothing: it reads each path's stats and moves the selection when they say so.
const RESELECT_INTERVAL: Duration = Duration::from_millis(250);

/// khor patch 17/30: the shared path-liveness observation. The transport's
/// in-flight retransmission and the selector must agree about a failed
/// path even while its last measured RTT is still fast. This retains the
/// previous two-PTO / max(300 ms, 3 PTO) / recent-packet guards.
pub(crate) fn unanswered_for(stats: &PathStats, now: std::time::Instant) -> Option<Duration> {
    stats.unanswered_for(now)
}

/// khor patch 17: how long a road that stopped answering has to answer again before it is a
/// candidate again ([`Silence`]).  A road that answers off and on would otherwise be left and
/// taken again on every other tick.
///
/// carry (10-23): a road far faster than the one in use, or the one in use itself, is a
/// candidate again at its first round trip, unless it stopped answering once already within
/// [`REHOLD`] of being back ([`Measured::mark_silent`]).
const SILENT_HOLD: Duration = Duration::from_secs(1);

/// khor patch 17 (carry, 10-23): a road that goes silent again within this long of being back
/// is one that answers off and on, and waits out [`SILENT_HOLD`] whatever it reads.
const REHOLD: Duration = Duration::from_secs(30);

/// khor patch 19: how much the selected direct path's round trip has to rise over the lowest it
/// read since it was selected before a hole-punching round is started at once, rather than at
/// the next [`UPGRADE_INTERVAL`] check (the floor's half, at least this).
const DEGRADED_BY: Duration = Duration::from_millis(5);

/// khor patch 19: how long the selected path is pinged on every measurement tick once a
/// round trip sample came in well over its floor: about eight samples, so the smoothed RTT
/// says within two seconds whether the path slowed, where an idle connection has one sample
/// per keepalive (5 s) and the smoothed RTT takes five of them to move half way.
const PROBE_FOR: Duration = Duration::from_secs(2);

/// khor patch 19: at most one such probe per this long, so a path whose samples jump often
/// (Wi-Fi) costs at most eight PINGs and their ACKs per half minute.
const PROBE_SPACING: Duration = Duration::from_secs(30);

/// khor patch 17: how long a move off a path that still answers is held while both paths are
/// measured, on every tick, before it is made ([`RemoteStateActor::confirm_move`]).
const CONFIRM_FOR: Duration = Duration::from_secs(2);

/// khor patch 17 (10-21): how long the selected road keeps a packet waiting before the other
/// roads are pinged ([`RemoteStateActor::probe_others`]): half of the 300 ms unanswered floor.
const PROBE_OTHERS_AFTER: Duration = Duration::from_millis(150);

/// khor patch 17 (10-21): a move whose two paths did not both take a sample in this long is
/// given up: every tick of it pings them, and one that never answers held the move, and the
/// pinging, for as long as the selector wanted it.
const CONFIRM_MAX: Duration = Duration::from_secs(4);

/// khor patch 17 (10-21): and the same move is not tried again for this long.
const CONFIRM_REST: Duration = Duration::from_secs(30);

/// khor patch 17 (flap, 10-23): the slack in [`far_faster`], so that two fast roads a few
/// milliseconds apart are not far apart. All transports use this comparison; a slow sample
/// alone must not bypass fresh confirmation on an otherwise fast road (netlab p1s).
const FAR_FASTER_BY: Duration = Duration::from_millis(10);

/// khor patch 17 (flap, 10-23): whether a path of round trip `to` is far faster than one of
/// `from`: twice it, and [`FAR_FASTER_BY`], is still no more.  Twice: were the faster one's
/// reading off by as much as itself, it would still be the faster, so there is nothing for the
/// confirmation to find out.
fn far_faster(to: Duration, from: Duration) -> bool {
    to.saturating_mul(2).saturating_add(FAR_FASTER_BY) <= from
}

/// khor patch 17 (carry, 10-23): of the copies in a probe timeout's backoff (`backed_off`: the
/// copy, its road, when the packet it waits on went out), those whose road heard something on
/// any copy after that packet went out (`heard`: each road's last receipt).
fn to_wake<'a>(
    backed_off: &'a [(ConnId, PathId, transports::FourTuple, std::time::Instant)],
    heard: &'a [(transports::FourTuple, std::time::Instant)],
) -> impl Iterator<Item = (ConnId, PathId)> + 'a {
    backed_off
        .iter()
        .filter(|(_, _, tuple, sent)| heard.iter().any(|(t, at)| t == tuple && at > sent))
        .map(|(conn_id, path_id, _, _)| (*conn_id, *path_id))
}

/// khor patch 19 (10-21): the rounds a selected path's degrading starts are at least this far
/// apart, however often the selection moves.
const PUNCH_SPACING: Duration = Duration::from_secs(30);

/// khor patch 19 (10-21): a selected direct path that kept the connection waiting this long
/// starts a round; a shorter stall is ridden out or left by the selector.
const PUNCH_SILENT_AFTER: Duration = Duration::from_secs(1);

/// khor patch 19 (10-21): how long after the remote's candidates change a round starts, so the
/// ADD_ADDRESS frames of one update (a new connection hears every address the remote has, one
/// frame each) are one round, not one each.
const ADDR_SETTLE: Duration = Duration::from_millis(100);

/// khor patch 18: the keepalive of a direct path that carries nothing, a spare (selected paths
/// keep [`crate::socket::HEARTBEAT_INTERVAL`]): a third of the packets, and still well inside
/// a home router's 30 s UDP mapping.
const SPARE_KEEP_ALIVE: Duration = Duration::from_secs(15);

/// khor patch 18: the idle timeout that goes with [`SPARE_KEEP_ALIVE`], three of them.
const SPARE_IDLE_TIMEOUT: Duration = Duration::from_secs(45);

/// How long to wait before retrying the addresses in [`State::pending_open_paths`].
const OPEN_PATH_RETRY_DELAY: Duration = Duration::from_millis(333);

/// The most addresses [`State::pending_open_paths`] holds.
///
/// Opening a path can fail on every live connection to the remote, on every retry tick, so a
/// queue that grew with each failure would grow geometrically in the number of connections
/// until the allocator failed.  A remote is never worth retrying on this many addresses; at the
/// bound the oldest entry is dropped, being the one least likely to still be reachable.
const MAX_PENDING_OPEN_PATHS: usize = 64;

/// khor patch 8 (vendor/iroh/KHOR.md): one hole-punching round this
/// endpoint started toward `remote`, with the candidates on both sides.
/// Upstream says so only as the tracing event `iroh::_events::qnt::init`.
#[derive(Debug, Clone)]
pub struct HolepunchRound {
    /// The remote endpoint.
    pub remote: EndpointId,
    /// This endpoint's candidates, as offered to the remote.
    pub local: Vec<SocketAddr>,
    /// The remote's candidates, as it offered them.
    pub remote_candidates: Vec<SocketAddr>,
}

/// khor patch 10: this endpoint's addresses as remotes observed them on
/// direct paths, with when; shared by every remote's actor and read where
/// the direct addresses are assembled (`socket.rs`).
pub(crate) type PeerObserved =
    Arc<std::sync::Mutex<std::collections::BTreeMap<SocketAddr, std::time::Instant>>>;

/// khor patch 10: how long a remote's observation is offered after it was
/// last made.
pub(crate) const PEER_OBSERVED_FOR: Duration = Duration::from_secs(10 * 60);

/// khor patch 10: the most observed addresses kept.
pub(crate) const PEER_OBSERVED_MAX: usize = 8;
// QUIC retransmits dropped Initials; a blocked transport must not retain them indefinitely.
const DATAGRAM_SEND_TIMEOUT: Duration = Duration::from_secs(3);
const MAX_DATAGRAM_SEND_TASKS: usize = 16;

/// A stream of events from all paths for all connections.
///
/// The connection is identified using [`ConnId`].  The event `Err` variant happens when the
/// actor has lagged processing the events, which is rather critical for us.
type PathEvents = MergeUnbounded<
    Pin<Box<dyn Stream<Item = (ConnId, Result<NoqPathEvent, noq::Lagged>)> + Send + Sync>>,
>;

/// A stream of events of announced NAT traversal candidate addresses for all connections.
///
/// The connection is identified using [`ConnId`].
type AddrEvents = MergeUnbounded<
    Pin<
        Box<
            dyn Stream<Item = (ConnId, Result<n0_nat_traversal::Event, noq::Lagged>)> + Send + Sync,
        >,
    >,
>;

/// The state we need to know about a single remote endpoint.
///
/// This actor manages all connections to the remote endpoint.  It will trigger holepunching
/// and select the best path etc.
pub(super) struct RemoteStateActor {
    /// All connections we have to this remote endpoint.
    connections: FxHashMap<ConnId, ConnectionState>,
    diagnostic_actor_serial: u64,
    diagnostic_registry_epoch: u64,
    /// State of the actor and hooks into the rest of the remote endpoint.
    ///
    /// This is on a separate struct so that we can have parallel mutable borrows to
    /// `connections` and `state`.
    state: State,
}

/// State of the [`RemoteStateActor`] and hooks into the rest of the remote endpoint.
struct State {
    /// The endpoint ID of the remote endpoint.
    endpoint_id: EndpointId,

    // Hooks into the rest of the Socket.
    //
    /// Metrics.
    metrics: Arc<SocketMetrics>,
    /// Our local addresses.
    ///
    /// These are our local addresses and any reflexive transport addresses.
    local_direct_addrs: n0_watcher::Direct<BTreeSet<DirectAddr>>,
    /// The mapped addresses for this endpoint.
    mapped_addrs: MappedAddrs,
    /// Address lookup service, cloned from the socket.
    address_lookup: AddressLookupServices,

    // Internal state - Noq Connections we are managing.
    //
    /// Notifications when connections are closed.
    connections_close: FuturesUnordered<OnClosed>,
    /// Events emitted by Noq about path changes, for all paths, all connections.
    path_events: PathEvents,
    /// A stream of events of announced NAT traversal candidate addresses for all connections.
    addr_events: AddrEvents,

    // Internal state - Holepunching and path state.
    //
    /// All possible paths we are aware of.
    ///
    /// These paths might be entirely impossible to use, since they are added by Address Lookup
    /// mechanisms.  The are only potentially usable.
    paths: RemotePathState,
    /// Information about the last holepunching attempt.
    last_holepunch: Option<HolepunchAttempt>,

    /// The path we currently consider the preferred path to the remote endpoint.
    ///
    /// **We expect this path to work.** If we become aware this path is broken then it is
    /// set back to `None`.  Having a selected path does not mean we may not be able to get
    /// a better path: e.g. when the selected path is a relay path we still need to trigger
    /// holepunching regularly.
    ///
    /// We only select a path once the path is functional in Noq.
    selected_path: Option<transports::FourTuple>,
    /// khor patch 18: the direct path the selector keeps open beside the selected one, so a
    /// failing selected path has somewhere to go at once.  `None`: none is kept, and every
    /// unselected direct path is closed as upstream does.
    backup_path: Option<transports::FourTuple>,
    /// khor patch 17: when the selected path was selected.
    selected_at: Option<Instant>,
    /// khor patch 19/29: the selected path, watched for slowing down or going silent.
    watch: Watch,
    /// khor patch 17: the move being confirmed ([`RemoteStateActor::confirm_move`]).
    moving: Moving,
    /// khor patch 17: what the measurement ticks read.
    measured: Measured,
    /// A bounded refresh of existing alternatives while STREAM traffic is active.
    refresh: ActiveRefresh,
    /// khor patch 20: the candidates last given to the connections.  A change of the direct
    /// addresses' sources alone (an address both on an interface and learnt by QAD) is no new
    /// candidate and starts no round.
    last_local_candidates: Option<BTreeSet<SocketAddr>>,
    /// Time at which we should schedule the next holepunch attempt.
    scheduled_holepunch: Option<Instant>,
    /// When to next attempt opening paths in [`Self::pending_open_paths`].
    scheduled_open_path: Option<Instant>,
    /// Paths which we still need to open.
    ///
    /// They failed to open because we did not have enough CIDs issued by the remote.
    ///
    /// Each address appears at most once, and there are at most [`MAX_PENDING_OPEN_PATHS`] of
    /// them: see [`State::queue_pending_open_path`].
    pending_open_paths: VecDeque<transports::FourTuple>,

    // Internal state - address lookup
    //
    /// Stream of Address Lookup results, or always pending if Address Lookup is not running.
    address_lookup_stream: Option<BoxStream<Result<AddressLookupItem, AddressLookupFailed>>>,

    /// The path selector used to pick the preferred path among the candidates.
    path_selector: Arc<dyn PathSelector>,

    /// khor patch 8: every round started, told to `Endpoint::holepunch_rounds`.
    rounds: tokio::sync::broadcast::Sender<HolepunchRound>,
    /// khor patch 10: the endpoint's addresses as remotes observed them.
    peer_observed: PeerObserved,
}

impl RemoteStateActor {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        endpoint_id: EndpointId,
        local_direct_addrs: n0_watcher::Direct<BTreeSet<DirectAddr>>,
        mapped_addrs: MappedAddrs,
        metrics: Arc<SocketMetrics>,
        address_lookup: AddressLookupServices,
        path_selector: Arc<dyn PathSelector>,
        rounds: tokio::sync::broadcast::Sender<HolepunchRound>,
        peer_observed: PeerObserved,
    ) -> Self {
        let diagnostic_actor_serial = if tracing::enabled!(target: "khor_net_samples", tracing::Level::DEBUG)
        {
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        } else {
            0
        };
        Self {
            connections: FxHashMap::default(),
            diagnostic_actor_serial,
            diagnostic_registry_epoch: 0,
            state: State {
                endpoint_id,
                metrics: metrics.clone(),
                local_direct_addrs,
                mapped_addrs,
                address_lookup,
                connections_close: Default::default(),
                path_events: Default::default(),
                addr_events: Default::default(),
                paths: RemotePathState::new(metrics),
                last_holepunch: None,
                selected_path: Default::default(),
                backup_path: None,
                selected_at: None,
                watch: Watch::default(),
                moving: Moving::default(),
                measured: Default::default(),
                refresh: Default::default(),
                last_local_candidates: None,
                scheduled_holepunch: None,
                scheduled_open_path: None,
                pending_open_paths: VecDeque::new(),
                address_lookup_stream: None,
                path_selector,
                rounds,
                peer_observed,
            },
        }
    }

    pub(super) fn start(
        self,
        initial_msgs: Vec<RemoteStateMessage>,
        tasks: &mut JoinSet<(EndpointId, Vec<RemoteStateMessage>)>,
        shutdown_token: CancellationToken,
        parent_span: Span,
    ) -> mpsc::Sender<RemoteStateMessage> {
        let (tx, rx) = mpsc::channel(16);
        let endpoint_id = self.state.endpoint_id;

        // Ideally we'd use the endpoint span as parent.  We'd have to plug that span into
        // here somehow.  Instead we have no parent and explicitly set the me attribute.  If
        // we don't explicitly set a span we get the spans from whatever call happens to
        // first create the actor, which is often very confusing as it then keeps those
        // spans for all logging of the actor.
        tasks.spawn(
            self.run(initial_msgs, rx, shutdown_token)
                .instrument(info_span!(
                    parent: parent_span,
                    "RemoteStateActor",
                    remote = %endpoint_id.fmt_short(),
                )),
        );
        tx
    }

    /// Runs the main loop of the actor.
    async fn run(
        mut self,
        initial_msgs: Vec<RemoteStateMessage>,
        mut inbox: mpsc::Receiver<RemoteStateMessage>,
        shutdown_token: CancellationToken,
    ) -> (EndpointId, Vec<RemoteStateMessage>) {
        trace!("actor started");
        let mut send_tasks = FuturesUnorderedBounded::new(MAX_DATAGRAM_SEND_TASKS);
        for msg in initial_msgs {
            self.handle_message(msg, &mut send_tasks);
        }
        let idle_timeout = time::sleep(ACTOR_MAX_IDLE_TIMEOUT);
        n0_future::pin!(idle_timeout);

        let check_connections = time::interval(UPGRADE_INTERVAL);
        n0_future::pin!(check_connections);

        // khor patch 17: the selection again, on measurement.
        let reselect = time::interval(RESELECT_INTERVAL);
        n0_future::pin!(reselect);

        loop {
            let scheduled_path_open = match self.state.scheduled_open_path {
                Some(when) => MaybeFuture::Some(time::sleep_until(when)),
                None => MaybeFuture::None,
            };
            n0_future::pin!(scheduled_path_open);
            let scheduled_hp = match self.state.scheduled_holepunch {
                Some(when) => MaybeFuture::Some(time::sleep_until(when)),
                None => MaybeFuture::None,
            };
            n0_future::pin!(scheduled_hp);
            // khor patch 40: only pending decisions own a timer. A tick just
            // before an exact boundary must not defer it another full tick.
            let next_selection = [
                self.state.moving.deadline(Instant::now()),
                self.state.measured.recovery_deadline(),
            ]
            .into_iter()
            .flatten()
            .min();
            let scheduled_selection = match next_selection {
                Some(when) => MaybeFuture::Some(time::sleep_until(when)),
                None => MaybeFuture::None,
            };
            n0_future::pin!(scheduled_selection);
            if !self.is_idle(&inbox) {
                idle_timeout
                    .as_mut()
                    .reset(Instant::now() + ACTOR_MAX_IDLE_TIMEOUT);
            }

            tokio::select! {
                biased;

                _ = shutdown_token.cancelled() => {
                    trace!("actor cancelled");
                    break;
                }
                _ = &mut scheduled_selection, if !self.connections.is_empty() => {
                    self.reselect();
                }
                Some(()) = send_tasks.next(), if !send_tasks.is_empty() => {}
                msg = inbox.recv() => {
                    match msg {
                        Some(msg) => self.handle_message(msg, &mut send_tasks),
                        None => break,
                    }
                }
                Some((id, evt)) = self.state.path_events.next() => {
                    self.handle_path_event(id, evt);
                }
                Some((id, evt)) = self.state.addr_events.next() => {
                    // khor patch 19 (10-21): the frames of one update are one round.
                    trace!(?id, ?evt, "remote addrs updated, holepunching shortly");
                    self.state.punch_soon(Instant::now() + ADDR_SETTLE);
                }
                Some((conn_id, closed)) = self.state.connections_close.next(), if !self.state.connections_close.is_empty() => {
                    self.handle_connection_close(conn_id, closed);
                }
                res = self.state.local_direct_addrs.updated() => {
                    if let Err(n0_watcher::Disconnected) = res {
                        trace!("direct address watcher disconnected, shutting down");
                        break;
                    }
                    // khor patch 20: only new candidates are worth a round.
                    if self.update_local_direct_address() {
                        trace!("local addrs updated, triggering holepunching");
                        self.trigger_holepunching();
                    }
                }
                _ = &mut scheduled_path_open => {
                    trace!("triggering scheduled path_open");
                    self.state.scheduled_open_path = None;
                    let mut addrs = std::mem::take(&mut self.state.pending_open_paths);
                    while let Some(addr) = addrs.pop_front() {
                        self.open_path_on_all_conns(&addr);
                    }
                }
                _ = &mut scheduled_hp => {
                    trace!("triggering scheduled holepunching");
                    self.state.scheduled_holepunch = None;
                    self.trigger_holepunching();
                }
                Some(item) = maybe_next(self.state.address_lookup_stream.as_mut()), if self.state.address_lookup_stream.is_some() => {
                    self.state.handle_address_lookup_item(item);
                }
                _ = check_connections.tick() => {
                    self.check_connections();
                }
                _ = reselect.tick(), if !self.connections.is_empty() => {
                    self.reselect();
                }
                _ = &mut idle_timeout => {
                    if self.is_idle(&inbox) {
                        trace!("idle timeout expired and still idle: terminate actor");
                        break;
                    } else {
                        // Seems like we weren't really idle, so we reset
                        idle_timeout.as_mut().reset(Instant::now() + ACTOR_MAX_IDLE_TIMEOUT);
                    }
                }
            }
        }

        inbox.close();
        // There might be a race between checking `inbox.is_empty()` and `inbox.close()`,
        // so we pull out all messages that are left over.
        let mut leftover_msgs = Vec::with_capacity(inbox.len());
        inbox.recv_many(&mut leftover_msgs, inbox.len()).await;

        trace!("actor terminating");
        (self.state.endpoint_id, leftover_msgs)
    }

    /// Returns `true` if the actor is fully idle.
    fn is_idle(&self, inbox: &mpsc::Receiver<RemoteStateMessage>) -> bool {
        self.connections.is_empty()
            && inbox.is_empty()
            && self.state.paths.resolve_requests_is_empty()
    }

    /// Handles an actor message.
    #[instrument(skip(self))]
    fn handle_message(
        &mut self,
        msg: RemoteStateMessage,
        send_tasks: &mut FuturesUnorderedBounded<Boxed<()>>,
    ) {
        match msg {
            RemoteStateMessage::SendDatagram(sender, transmit) => {
                self.state
                    .handle_msg_send_datagram(sender, transmit, send_tasks);
            }
            RemoteStateMessage::AddConnection(handle, tx) => {
                self.handle_msg_add_connection(handle, tx);
            }
            RemoteStateMessage::ResolveRemote(addrs, tx) => {
                // khor patch 12: a custom (carried) address handed in while
                // connections are live is opened on them now. Upstream
                // only records it, and a connection that settled on the
                // relay never learnt of a road khor carried afterwards.
                // `open_path_ensure` leaves a path that is already open.
                let custom: Vec<transports::FourTuple> =
                    to_transports_addr(self.state.endpoint_id, addrs.iter().cloned())
                        .filter(|a| matches!(a, transports::Addr::Custom(_)))
                        .map(transports::FourTuple::from_remote)
                        .collect();
                self.state.handle_msg_resolve_remote(addrs, tx);
                if !custom.is_empty() {
                    if std::env::var_os("KHOR_NET_DIAGNOSTICS").is_some() {
                        let live = self.connections.values().any(|state| {
                            state
                                .handle
                                .upgrade()
                                .is_some_and(|conn| conn.close_reason().is_none())
                        });
                        let selected = self.state.selected_path.clone();
                        eprintln!(
                            "[khor-pending-carrier] remote={} live={} registered={} previous={selected:?}",
                            self.state.endpoint_id,
                            live,
                            self.connections.len(),
                        );
                    }
                }
                for open_addr in &custom {
                    self.open_path_on_all_conns(open_addr);
                }
            }
            RemoteStateMessage::RemoteInfo(tx) => {
                let addrs = self.state.paths.to_remote_addrs();
                let info = RemoteInfo {
                    endpoint_id: self.state.endpoint_id,
                    addrs,
                };
                tx.send(info).ok();
            }
            RemoteStateMessage::NetworkChange { is_major } => {
                self.handle_msg_network_change(is_major);
            }
            RemoteStateMessage::DiscardIpHints { before, tx } => {
                // Recheck at execution, not enqueue: a newly established or
                // still-pending connection must keep its current addresses.
                let live = self.connections.values().any(|state| {
                    state
                        .handle
                        .upgrade()
                        .is_some_and(|conn| conn.close_reason().is_none())
                });
                let removed = if live {
                    0
                } else {
                    self.state.paths.discard_ip_hints_before(before)
                };
                if !live
                    && removed > 0
                    && self.state.selected_path.as_ref().is_some_and(|path| {
                        matches!(path.remote(), transports::Addr::Ip(_))
                            && !self.state.paths.addrs().any(|addr| addr == &path.remote())
                    })
                {
                    self.state.selected_path = None;
                }
                let _ = tx.send(removed);
            }
            RemoteStateMessage::CloseCustom(still_gone) => {
                if still_gone.is_some_and(|check| !check()) {
                    return;
                }
                // khor patch 13: the road under every custom (carried)
                // path to this remote is gone, so close them now. Upstream
                // leaves a path until its idle timeout (15 s) and the
                // selector keeps choosing it on its last round trip; the
                // close emits `Abandoned`, which re-runs the selection.
                for conn_state in self.connections.values() {
                    let Some(conn) = conn_state.handle.upgrade() else {
                        continue;
                    };
                    for (path_id, tuple) in conn_state.paths.iter() {
                        if !matches!(tuple.remote(), transports::Addr::Custom(_)) {
                            continue;
                        }
                        if let Some(path) = conn.path(*path_id) {
                            // The last open path is refused: the
                            // connection keeps it and idles out as before.
                            let _ = path.close();
                        }
                    }
                }
            }
        }
    }

    /// Handles [`RemoteStateMessage::AddConnection`].
    ///
    /// Error returns are fatal and kill the actor.
    fn handle_msg_add_connection(
        &mut self,
        conn: noq::Connection,
        tx: oneshot::Sender<PathStateReceiver>,
    ) {
        let (path_state_sender, path_state_receiver) = PathStateSender::new();
        self.state.metrics.num_conns_opened.inc();
        // Remove any conflicting stable_ids from the local state.
        let conn_id = ConnId(conn.stable_id());
        if tracing::enabled!(target: "khor_net_samples", tracing::Level::DEBUG) {
            self.diagnostic_registry_epoch = self.diagnostic_registry_epoch.saturating_add(1);
        }
        self.connections.remove(&conn_id);

        // Hook up paths, NAT addresses and connection closed event streams.
        self.state
            .path_events
            .push(Box::pin(conn.path_events().map(move |evt| (conn_id, evt))));
        self.state.addr_events.push(Box::pin(
            conn.nat_traversal_updates().map(move |evt| (conn_id, evt)),
        ));
        self.state.connections_close.push(OnClosed::new(&conn));

        // Add local addrs to the connection
        let local_addrs = self.state.local_candidates();
        update_qnt_candidates(&conn, &local_addrs);

        // Store the connection
        let conn_state = self
            .connections
            .entry(conn_id)
            .insert_entry(ConnectionState {
                handle: conn.weak_handle(),
                path_state: path_state_sender,
                paths: Default::default(),
                has_been_direct: false,
                path_ids: PathIds::default(),
            })
            .into_mut();

        // Store PathId(0), set path_status and select best path, check if holepunching
        // is needed.
        if let Some(path) = conn.path(PathId::ZERO) {
            let path_remote = self
                .state
                .register_and_configure_path(conn_id, conn_state, &path);

            if let Some(path_remote) = path_remote
                && conn.side().is_client()
            {
                // We may have raced this with a relay address.  Try and add any
                // relay addresses we have back.
                //
                // khor patch 6 (vendor/iroh/KHOR.md): and any custom transport
                // address, whichever path won the race.  Upstream opens only
                // relays here, so a custom address dialled together with a
                // relay was never opened once the relay answered first.
                let path0_relay = path_remote.is_relay();
                let relays = self
                    .state
                    .paths
                    .addrs()
                    .filter(|addr| {
                        (addr.is_relay() && !path0_relay)
                            || matches!(addr, transports::Addr::Custom(_))
                    })
                    .map(|addr| transports::FourTuple::from_remote(addr.clone()))
                    .collect::<Vec<_>>();
                for open_addr in relays {
                    let outcome = self
                        .state
                        .open_path_on_conn(conn_id, conn_state, &conn, &open_addr);
                    if outcome == OpenPathOutcome::Retry {
                        self.state.queue_pending_open_path(&open_addr);
                    }
                }
            }
        }
        self.trigger_holepunching();
        self.select_path();
        tx.send(path_state_receiver).ok();
    }

    /// Handles [`RemoteStateMessage::NetworkChange`].
    fn handle_msg_network_change(&mut self, is_major: bool) {
        // Ping all the paths so loss-detection starts ASAP.
        for conn in self.connections.values() {
            if let Some(noq_conn) = conn.handle.upgrade() {
                for (path_id, addr) in &conn.paths {
                    if let Some(path) = noq_conn.path(*path_id) {
                        // Ping the current path
                        if let Err(err) = path.ping() {
                            warn!(%err, %path_id, ?addr, "failed to ping path");
                        }
                    }
                }
            }
        }

        if is_major {
            self.trigger_holepunching();
        }
    }

    fn handle_connection_close(&mut self, conn_id: ConnId, closed: Closed) {
        event!(
            target: "iroh::_events::conn::closed",
            Level::DEBUG,
            %conn_id,
            remote_id = %self.state.endpoint_id.fmt_short(),
            reason=?closed.reason,
        );

        if let Some(conn_state) = self.connections.remove(&conn_id) {
            if tracing::enabled!(target: "khor_net_samples", tracing::Level::DEBUG) {
                self.diagnostic_registry_epoch = self.diagnostic_registry_epoch.saturating_add(1);
            }
            self.state.metrics.num_conns_closed.inc();
            conn_state.path_state.close(closed);
        }
        if self.connections.is_empty() {
            trace!("last connection closed - clearing selected_path");
            self.state.selected_path = None;
        }
    }

    /// Updates the local [`DirectAddr`]s to all connections.
    ///
    /// Each connection needs to have the local direct addresses to use as QNT address
    /// candidates.
    ///
    /// khor patch 20: answers whether the candidates changed.  The watcher fires on any change
    /// of the direct addresses, their sources included; an address that is an interface's and
    /// also what QAD reports read as one in one report and the other in the next (2026-10-16,
    /// a machine whose global IPv6 is its interface's), and every such flip was a round of
    /// hole punching on every connection.
    fn update_local_direct_address(&mut self) -> bool {
        let local_addrs = self.state.local_candidates();
        if self.state.last_local_candidates.as_ref() == Some(&local_addrs) {
            return false;
        }
        for conn in self.connections.values().filter_map(|s| s.handle.upgrade()) {
            update_qnt_candidates(&conn, &local_addrs);
        }
        self.state.last_local_candidates = Some(local_addrs);
        true
    }

    /// Triggers holepunching to the remote endpoint.
    ///
    /// This will manage the entire process of holepunching with the remote endpoint.
    ///
    /// - Holepunching happens on the Connection with the lowest [`ConnId`] which is a
    ///   client.
    ///   - Both endpoints may initiate holepunching if both have a client connection.
    ///   - Any opened paths are opened on all other connections without holepunching.
    /// - If there are no changes in local or remote candidate addresses since the
    ///   last attempt **and** there was a recent attempt, a trigger_holepunching call
    ///   will be scheduled instead.
    fn trigger_holepunching(&mut self) {
        if self.connections.is_empty() {
            trace!("not holepunching: no connections");
            return;
        }

        let Some(conn) = self
            .connections
            .iter()
            .filter_map(|(id, state)| state.handle.upgrade().map(|conn| (*id, conn)))
            .filter(|(_, conn)| conn.side().is_client())
            .min_by_key(|(id, _)| *id)
            .map(|(_, conn)| conn)
        else {
            trace!("not holepunching: no client connection");
            return;
        };
        let remote_candidates = match conn.get_remote_nat_traversal_addresses() {
            Ok(addrs) => BTreeSet::from_iter(addrs),
            Err(err) => {
                warn!("failed to get nat candidate addresses: {err:#}");
                return;
            }
        };
        let local_candidates = self.state.local_candidates();
        let new_candidates = self
            .state
            .last_holepunch
            .as_ref()
            .map(|last_hp| {
                // Addrs are allowed to disappear, but if there are new ones we need to
                // holepunch again.
                trace!(
                    ?last_hp,
                    ?local_candidates,
                    ?remote_candidates,
                    "candidates to holepunch?"
                );
                !remote_candidates.is_subset(&last_hp.remote_candidates)
                    || !local_candidates.is_subset(&last_hp.local_candidates)
            })
            .unwrap_or(true);
        if !new_candidates && let Some(ref last_hp) = self.state.last_holepunch {
            let next_hp = last_hp.when + HOLEPUNCH_ATTEMPTS_INTERVAL;
            let now = Instant::now();
            if next_hp > now {
                trace!(scheduled_in = ?(next_hp - now), "not holepunching: no new addresses");
                self.state.scheduled_holepunch = Some(next_hp);
                return;
            }
        }

        self.state.do_holepunching(conn);
    }

    #[instrument(skip(self))]
    fn handle_path_event(&mut self, conn_id: ConnId, event: Result<NoqPathEvent, noq::Lagged>) {
        let Ok(event) = event else {
            warn!("missed a PathEvent, RemoteStateActor lagging");
            // TODO: Is it possible to recover using the sync APIs to figure out what the
            //    state of the connection and it's paths are?
            return;
        };
        let Some(conn_state) = self.connections.get_mut(&conn_id) else {
            trace!("event for removed connection");
            return;
        };
        let Some(conn) = conn_state.handle.upgrade() else {
            trace!("event for closed connection");
            return;
        };
        trace!("path event");
        match event {
            NoqPathEvent::Established { id: path_id, .. } => {
                let Some(path) = conn.path(path_id) else {
                    trace!("path open event for unknown path");
                    return;
                };

                self.state
                    .register_and_configure_path(conn_id, conn_state, &path);
                self.select_path();
            }
            NoqPathEvent::Abandoned { id, reason, .. } => {
                // khor patch 1 (10-21): a path id is about to be given back.
                for addr in conn_state.path_ids.freed() {
                    self.state.queue_pending_open_path(&addr);
                }
                // Remove abandoned path from the conn state.
                let Some(network_path) = conn_state.remove_path(&id, &conn) else {
                    debug!(%id, "path not in path_id_map");
                    return;
                };

                // We track all known remote addresses for the peer in `State::paths`. The
                // paths are tracked by remote address only (we ignore the local
                // IP). Therefore, we mark a remote addr as abandoned in the remote-global
                // state only once no connections have any path to that remote addr.
                if !conn_state
                    .paths
                    .values()
                    .any(|tuple| tuple.remote() == network_path.remote())
                {
                    self.state.paths.abandoned_path(&network_path.remote());
                }

                event!(
                    target: "iroh::_events::path::abandoned",
                    Level::DEBUG,
                    remote = %self.state.endpoint_id.fmt_short(),
                    %conn_id,
                    path_id = %id,
                    %network_path,
                    ?reason
                );

                // If the remote closed our selected path, select a new one.
                self.select_path();
            }
            NoqPathEvent::Discarded { id, path_stats, .. } => {
                trace!(%id, ?path_stats, "path discarded");
                // khor patch 1 (10-21): by now the remote has raised the limit, as a rule.
                for addr in conn_state.path_ids.freed() {
                    self.state.queue_pending_open_path(&addr);
                }
            }
            NoqPathEvent::ObservedAddr { id, addr, .. } => {
                // khor patch 10: on a direct path, what the remote sees is
                // this endpoint's address on the internet for that road —
                // the same fact QAD asks a relay for. On a relay or carried
                // path it is a mapped address and means nothing.
                let direct = conn
                    .path(id)
                    .and_then(|p| p.network_path().ok())
                    .and_then(|np| self.state.mapped_addrs.to_transport_tuple(&np))
                    .is_some_and(|t| t.is_ip());
                if direct {
                    self.state.peer_observed(addr);
                }
            }
            NoqPathEvent::RemoteStatus { .. } => {
                // Nothing to do for these events.
            }
            _ => {
                // We expect to keep noq and iroh in sync in all test setups, but in production it's totally possible
                // that iroh itself is linked against a newer version of noq with additional events we don't yet
                // know how to handle.
                #[cfg(test)]
                panic!("Unhandled path event: {event:?}");
            }
        }
    }

    /// Selects the preferred path by invoking the configured [`PathSelector`].
    ///
    /// The selected path is added to any connections which do not yet have it.  Any unused
    /// direct paths are closed for all connections.
    #[instrument(skip_all)]
    fn select_path(&mut self) {
        self.select_path_on(false);
    }

    /// [`Self::select_path`]; on a measurement `tick` (khor patch 17) the selection is only
    /// applied when it, or the backup, changed.  Every move, a tick's or an event's, goes
    /// through [`Self::confirm_move`]: a move it holds keeps the selection and the backup as
    /// they are.
    fn select_path_on(&mut self, tick: bool) {
        let now = Instant::now();
        let (selected_addr, backup) = {
            let held_for = self
                .state
                .selected_at
                .map(|at| now.saturating_duration_since(at));
            let ctx = PathSelectionContext::new(
                self.state.selected_path.as_ref(),
                self.state.backup_path.as_ref(),
                held_for,
                &self.connections,
                &self.state.measured,
            );
            let selection = self.state.path_selector.select(&ctx);
            (selection.selected().cloned(), selection.backup().cloned())
        };
        let was_moving = self.state.moving.to().is_some();
        if !self.confirm_move(selected_addr.as_ref()) {
            if !tick {
                self.apply_selected_path();
            }
            return;
        }

        // A move given up leaves the path it was to open: closed again unless it is the backup.
        let mut changed = was_moving && self.state.moving.to().is_none();
        if let Some(addr) = selected_addr
            && self.state.selected_path.as_ref() != Some(&addr)
        {
            let prev_remote = self.state.selected_path.replace(addr.clone());
            self.state.selected_at = Some(now);
            // khor patch 19: a new selection is measured from its own floor.  A direct road
            // left because it went silent for one that is not direct has the other direct roads
            // looked for at once: without a backup the client closed them.
            self.state.watch.reset();
            if prev_remote
                .as_ref()
                .is_some_and(|p| p.is_ip() && self.state.measured.silent.contains_key(p))
                && !addr.is_ip()
                && self.state.watch.may_punch(now)
            {
                debug!("left a silent direct path, triggering holepunching");
                self.trigger_holepunching();
            }
            changed = true;
            event!(
                target: "iroh::_events::path::selected",
                Level::DEBUG,
                remote = %self.state.endpoint_id.fmt_short(),
                network_path = %addr,
                prev_network_path = %prev_remote.map(|p| format!("{p}")).unwrap_or("None".to_string()),
            );
        } else {
            trace!(current_path = ?self.state.selected_path, "keeping current path");
        }
        // khor patch 18: the backup is whatever the selector names, none included; it is
        // never the selected path.
        let backup = backup.filter(|b| Some(b) != self.state.selected_path.as_ref());
        if backup != self.state.backup_path {
            self.state.backup_path = backup;
            changed = true;
        }

        if tick && !changed {
            return;
        }
        self.apply_selected_path();
    }

    /// khor patch 17: whether the selection may move to `to` now.
    ///
    /// The selector compares smoothed RTTs, and a path that carries nothing has one sample per
    /// keepalive, or none at all on the side that only acknowledges its peer's: under load
    /// (netlab p1, 20 scenarios side by side) the road in use read 8.5 ms against the other's
    /// 3.4 ms of three minutes before, and the connection moved between two 1 ms roads; one
    /// sample from a stalled machine (100 ms) did the same.  So a move off a path that still
    /// answers is held for [`CONFIRM_FOR`], the two paths are pinged on every tick that read
    /// no new sample of them, and it is made only if the selector still wants it on every tick
    /// and both took a sample in that time.  A path that stopped answering is left at once.
    ///
    /// The moves an event asks for are held the same way (10-17, fourth leg): a hole-punching
    /// round opens paths on every connection, each opening ran the selector at once on its
    /// handshake's RTT against the road in use, and under load p1 moved 20 times in ten
    /// minutes, three in one second. Every move from an open path now goes through
    /// confirmation (feat/net): khor compares RTT across transport kinds, so moving onto
    /// direct is no longer an unconditional upgrade.
    ///
    /// A move whose two paths do not both take a sample within [`CONFIRM_MAX`] is given up,
    /// and not tried again for [`CONFIRM_REST`] ([`Moving`]).
    ///
    /// 10-21 (net-flap): a path that stopped answering is left for one that answered since it
    /// stopped, pinged every tick until it does, or after [`CONFIRM_MAX`] for whatever the
    /// selector names ([`Moving::leave`]).  A machine that stalls stalls on every road, and the
    /// other road, idle, has nothing in flight to show it: netlab p1s (300 ms spikes on both of
    /// alpha's roads) left the road in use for the other on the spike and came back.
    ///
    /// feat/net: all transports use the two paths' readings in [`Moving::step`]. A much
    /// faster candidate can leave early with samples sent after the move began; meanwhile
    /// the current road is also probed so one scheduling spike can be corrected. A current
    /// road slower than the confirmation itself is not waited on for a sample.
    fn confirm_move(&mut self, to: Option<&transports::FourTuple>) -> bool {
        let Some(to) = to.filter(|to| self.state.selected_path.as_ref() != Some(*to)) else {
            self.state.moving.clear();
            return true;
        };
        let Some(from) = self.state.selected_path.clone() else {
            debug!(target: "khor_net_samples", ?to, "select without a current road");
            self.state.moving.clear();
            return true;
        };
        let open = self
            .connections
            .values()
            .any(|c| c.paths.values().any(|t| *t == from));
        if !open {
            debug!(target: "khor_net_samples", ?from, ?to, "select after current road closed");
            self.state.moving.clear();
            return true;
        }
        let silence = self
            .state
            .measured
            .silent
            .get(&from)
            .filter(|s| s.answered.is_none())
            .copied();
        let now = Instant::now();
        let starting = self.state.moving.to() != Some(to);
        let since = self
            .state
            .moving
            .to
            .as_ref()
            .filter(|(road, _)| road == to)
            .map_or(now, |(_, since)| *since);
        let measured = &self.state.measured;
        let sampled =
            |road: &transports::FourTuple, after: Instant| measured.samples_after(road, after) >= 2;
        let mut current_copy_hint = None;
        let step = match silence {
            Some(silence) => {
                let check_current = self.connections.values().any(|c| {
                    let Some(conn) = c.handle.upgrade() else {
                        return false;
                    };
                    c.paths.iter().any(|(path_id, road)| {
                        road == &from
                            && conn
                                .path_stats(*path_id)
                                .and_then(|stats| stats.last_received)
                                .is_some_and(|at| {
                                    at > silence.began.into_std()
                                        && now.into_std().saturating_duration_since(at)
                                            < RESELECT_INTERVAL
                                })
                    })
                });
                current_copy_hint = Some(check_current);
                self.state
                    .moving
                    .leave(to, now, silence.began, check_current, |after| {
                        measured.samples_after(to, after) >= 1
                    })
            }
            // feat/net: every transport compares fresh measured readings. A direct
            // candidate gets neither an unconditional upgrade nor a different gain rule.
            None => self.state.moving.step(
                to,
                now,
                measured.road(&from).map_or(Duration::ZERO, |(rtt, _)| rtt),
                measured.road(to).map_or(Duration::MAX, |(rtt, _)| rtt),
                |after| sampled(&from, after),
                |after| sampled(to, after),
            ),
        };
        if step == Step::Go || starting && matches!(step, Step::Hold { .. }) {
            debug!(
                target: "khor_net_samples",
                ?from,
                ?to,
                ?now,
                ?since,
                failed = silence.is_some(),
                check_current = ?current_copy_hint,
                ?step,
                readings = ?measured.rtts,
                silent = ?measured.silent,
                "move decision"
            );
        }
        let ping = match step {
            Step::Go => return true,
            Step::Rest => return false,
            Step::Hold { from: f, to: t } => [(f, true, from), (t, false, to.clone())],
        };
        for (_, current, road) in ping.into_iter().filter(|(p, _, _)| *p) {
            // An idle copy can be the only one in PTO while another copy of
            // the same road answers. Probe the suspect copy, not an arbitrary
            // healthy one; the candidate instead uses its least-backed-off copy.
            let road = &road;
            let copy = self
                .connections
                .iter()
                .flat_map(|(conn_id, c)| {
                    c.paths.iter().filter_map(move |(path_id, tuple)| {
                        if tuple != road {
                            return None;
                        }
                        let conn = c.handle.upgrade()?;
                        let pto_count = conn.path_stats(*path_id)?.pto_count;
                        let priority = if current {
                            pto_count
                        } else {
                            u32::MAX - pto_count
                        };
                        Some((
                            priority,
                            *conn_id,
                            *path_id,
                            pto_count,
                            conn.path(*path_id)?,
                        ))
                    })
                })
                .max_by_key(|(priority, ..)| *priority);
            if let Some((_, conn_id, path_id, pto_count, path)) = copy {
                let result = path.ping();
                debug!(target: "khor_net_samples", ?conn_id, ?path_id, current, pto_count, ?result, "confirmation probe");
            }
        }
        false
    }

    /// khor patch 17: one measurement tick. Watches the selected path for going
    /// silent or slowing down, then runs the selector on the paths' stats as they are now.
    fn reselect(&mut self) {
        let tuples_changed = self.refresh_path_tuples();
        let (sample, waiting, recovery) = self.read_paths();
        self.watch_selected(sample);
        self.probe_others(waiting);
        self.refresh_candidates();
        self.select_path_on(!tuples_changed);
        self.probe_recovering(recovery);
    }

    /// khor patch 39: a receipt can reset PTO without sampling a packet sent
    /// after the silence judgment. Confirm that known road within a bounded
    /// episode instead of waiting for its next idle keepalive.
    fn probe_recovering(&mut self, probes: Vec<RecoveryProbe>) {
        for probe in probes {
            let road = &probe.road;
            let selected = self.state.selected_path.as_ref() == Some(road);
            let confirming = self
                .state
                .moving
                .to()
                .is_some_and(|to| to == road || selected);
            let available = !self.state.measured.silent.contains_key(road);
            if confirming || selected && available {
                debug!(target: "khor_net_samples", road = ?probe.road, epoch = ?probe.epoch,
                    reserved = ?probe.at, attempt = probe.attempt, "recovery probe merged with selection");
                continue;
            }
            let preferred = self
                .state
                .measured
                .recovery
                .get(road)
                .and_then(|check| check.copy);
            let copy = self
                .connections
                .iter()
                .flat_map(|(conn_id, c)| {
                    c.paths.iter().filter_map(move |(path_id, tuple)| {
                        if tuple != road {
                            return None;
                        }
                        let conn = c.handle.upgrade()?;
                        let stats = conn.path_stats(*path_id)?;
                        Some((
                            stats.pto_count,
                            stats.last_received,
                            *conn_id,
                            *path_id,
                            conn.path(*path_id)?,
                        ))
                    })
                })
                .min_by(|a, b| {
                    recovery_copy_rank(preferred, (a.2, a.3), a.0)
                        .cmp(&recovery_copy_rank(preferred, (b.2, b.3), b.0))
                        .then_with(|| b.1.cmp(&a.1))
                        .then_with(|| a.2.0.cmp(&b.2.0))
                });
            if let Some((pto_count, _, conn_id, path_id, path)) = copy {
                if let Some(check) = self.state.measured.recovery.get_mut(road) {
                    check.copy = Some((conn_id, path_id));
                }
                let result = path.ping();
                debug!(target: "khor_net_samples", road = ?probe.road, epoch = ?probe.epoch,
                    reserved = ?probe.at, attempt = probe.attempt, ?conn_id, ?path_id, pto_count, ?result,
                    "recovery confirmation probe");
            } else {
                debug!(target: "khor_net_samples", road = ?probe.road, epoch = ?probe.epoch,
                    reserved = ?probe.at, attempt = probe.attempt, "recovery probe unavailable");
            }
        }
    }

    /// A passive QUIC migration can change a path's tuple without opening a
    /// new path ID. Reconcile the cached identity before attributing samples
    /// or applying the peer's selection to its individual connections.
    fn refresh_path_tuples(&mut self) -> bool {
        let mut retired = Vec::new();
        for (conn_id, conn_state) in self.connections.iter_mut() {
            let Some(conn) = conn_state.handle.upgrade() else {
                continue;
            };
            let changes: Vec<_> = conn_state
                .paths
                .iter()
                .filter_map(|(id, cached)| {
                    let path = conn.path(*id)?;
                    let actual = self
                        .state
                        .mapped_addrs
                        .to_transport_tuple(&path.network_path().ok()?)?;
                    (actual != *cached).then_some((cached.clone(), actual, path))
                })
                .collect();
            for (cached, actual, path) in changes {
                // Old samples name the old road, even when QUIC preserves its
                // RTT estimate for a port-only rebind. Other copies stay intact.
                self.state.measured.rtts.remove(&(*conn_id, path.id()));
                self.state
                    .register_and_configure_path(*conn_id, conn_state, &path);
                retired.push(cached.remote());
                if std::env::var_os("KHOR_NET_DIAGNOSTICS").is_some() {
                    eprintln!(
                        "[khor-path-tuple-refreshed] peer={} conn={} path={} cached={:?} actual={:?}",
                        self.state.endpoint_id,
                        conn_id,
                        path.id(),
                        cached,
                        actual,
                    );
                }
            }
        }
        let changed = !retired.is_empty();
        for remote in retired {
            if !self
                .connections
                .values()
                .any(|c| c.paths.values().any(|p| p.remote() == remote))
            {
                self.state.paths.abandoned_path(&remote);
            }
        }
        changed
    }

    /// STREAM traffic makes discovery responsive. At most one existing
    /// alternative is pinged per five seconds per remote, across all
    /// connections; the probes' ACKs cannot extend the activity window.
    fn refresh_candidates(&mut self) {
        let now = Instant::now();
        let diagnose = tracing::enabled!(target: "khor_net_samples", tracing::Level::DEBUG);
        let mut stream_counters = Vec::new();
        let frames = self
            .connections
            .iter()
            .filter_map(|(id, c)| c.handle.upgrade().map(|c| (id, c)))
            .map(|(id, c)| {
                let s = c.stats();
                if diagnose {
                    stream_counters.push((id.0, s.frame_tx.stream, s.frame_rx.stream));
                }
                s.frame_tx.stream.saturating_add(s.frame_rx.stream)
            })
            .fold(0u64, |n, frames| n.saturating_add(frames));
        if diagnose {
            static BEGAN: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
            let monotonic_us = BEGAN
                .get_or_init(std::time::Instant::now)
                .elapsed()
                .as_micros() as u64;
            let unix_us = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_micros() as u64;
            stream_counters.sort_unstable_by_key(|row| row.0);
            debug!(target: "khor_net_samples", peer = %self.state.endpoint_id,
                monotonic_us, unix_us, actor_serial = self.diagnostic_actor_serial,
                registry_epoch = self.diagnostic_registry_epoch,
                counters = ?stream_counters, "khor_stream_snapshot_v1");
        }
        self.state.refresh.observe(frames, now);
        if !self.state.refresh.ready(now) {
            return;
        }
        let mut alternatives: Vec<(transports::FourTuple, noq::Path)> = Vec::new();
        for c in self.connections.values() {
            let Some(conn) = c.handle.upgrade() else {
                continue;
            };
            for (id, tuple) in &c.paths {
                if Some(tuple) == self.state.selected_path.as_ref()
                    || alternatives.iter().any(|(seen, _)| seen == tuple)
                {
                    continue;
                }
                if let Some(path) = conn.path(*id) {
                    alternatives.push((tuple.clone(), path));
                }
            }
        }
        if let Some(index) = self.state.refresh.take(alternatives.len(), now) {
            let _ = alternatives[index].1.ping();
        }
    }

    /// khor patch 17 (10-21): the selected road has kept a packet waiting `waiting` (after a
    /// probe timeout); from [`PROBE_OTHERS_AFTER`] on, every other road that took no sample
    /// since that packet went out is pinged once a tick, so that when the selected road is
    /// judged silent the road to leave for has already answered or not ([`Moving::leave`]):
    /// leaving does not wait a further tick for it (netlab p3: 0.9 s worst echo, 0.6 s with).
    fn probe_others(&mut self, waiting: Option<Duration>) {
        let (Some(waiting), Some(selected)) = (waiting, self.state.selected_path.as_ref()) else {
            return;
        };
        if waiting < PROBE_OTHERS_AFTER {
            return;
        }
        let now = Instant::now();
        let since = now.checked_sub(waiting).unwrap_or(now);
        let mut pinged: Vec<&transports::FourTuple> = Vec::new();
        for conn_state in self.connections.values() {
            let Some(conn) = conn_state.handle.upgrade() else {
                continue;
            };
            for (path_id, road) in conn_state.paths.iter() {
                if road == selected
                    || pinged.contains(&road)
                    || self
                        .state
                        .measured
                        .road(road)
                        .is_some_and(|(_, at)| at > since)
                {
                    continue;
                }
                if let Some(path) = conn.path(*path_id) {
                    let _ = path.ping();
                    pinged.push(road);
                }
            }
        }
    }

    /// khor patch 17: reads every path's stats on every connection.  Two machines often have
    /// two connections, and a road is a path on each; the copy that last took an RTT sample
    /// is the one that says how fast the road is now (a copy that carries nothing keeps the
    /// RTT of its last keepalive), and any copy that stopped answering makes the road silent
    /// until a copy of it takes a new sample after that. Returns the largest new ACK-delay-corrected
    /// sample observed on the selected road (khor patch 19/29), independently of smoothing.
    ///
    /// Also returns how long the selected road has kept a packet waiting, if it has after a
    /// probe timeout ([`Self::probe_others`]).
    fn read_paths(&mut self) -> (Duration, Option<Duration>, Vec<RecoveryProbe>) {
        let now = Instant::now();
        let diagnose_tuples = std::env::var_os("KHOR_NET_DIAGNOSTICS").is_some();
        let selected = self.state.selected_path.as_ref();
        let measured = &mut self.state.measured;
        let mut sample = Duration::ZERO;
        let mut present = Vec::new();
        let mut silent_now: Vec<(transports::FourTuple, Instant)> = Vec::new();
        let mut waiting = None;
        // carry (10-23): the copies in a probe timeout's backoff, with the packet they wait on,
        // and when each road last heard anything on any copy ([`Self::wake`]).
        let mut backed_off: Vec<(ConnId, PathId, transports::FourTuple, std::time::Instant)> =
            Vec::new();
        let mut heard: Vec<(transports::FourTuple, std::time::Instant)> = Vec::new();
        for (conn_id, conn_state) in self.connections.iter() {
            let Some(conn) = conn_state.handle.upgrade() else {
                continue;
            };
            for (path_id, tuple) in conn_state.paths.iter() {
                let Some(stats) = conn.path_stats(*path_id) else {
                    continue;
                };
                if diagnose_tuples
                    && let Some(actual) = conn
                        .path(*path_id)
                        .and_then(|path| path.network_path().ok())
                        .and_then(|path| self.state.mapped_addrs.to_transport_tuple(&path))
                    && actual != *tuple
                {
                    eprintln!(
                        "[khor-path-tuple-mismatch] peer={} conn={} path={} cached={:?} actual={:?}",
                        self.state.endpoint_id, conn_id, path_id, tuple, actual,
                    );
                }
                if diagnose_tuples
                    && let Some(silence) = measured.silent.get(tuple)
                    && stats
                        .last_received
                        .is_some_and(|at| at > silence.judged.into_std())
                    && stats
                        .latest_rtt_sent
                        .is_none_or(|sent| sent <= silence.judged.into_std())
                {
                    debug!(target: "khor_net_samples", ?conn_id, ?path_id, ?tuple, ?now,
                        ?silence, pto_count = stats.pto_count, last_received = ?stats.last_received,
                        unacked_since = ?stats.unacked_since, latest_rtt_sent = ?stats.latest_rtt_sent,
                        latest_rtt_received = ?stats.latest_rtt_received,
                        "recovery lacks a post-judgment round trip");
                }
                let key = (*conn_id, *path_id);
                present.push(key);
                if let Some(waited) = unanswered_for(&stats, now.into_std()) {
                    debug!(target: "khor_net_samples", ?conn_id, ?path_id, ?tuple, ?now,
                        pto_count = stats.pto_count, last_received = ?stats.last_received,
                        unacked_since = ?stats.unacked_since, pto = ?stats.pto,
                        "silent path copy");
                    silent_now.push((tuple.clone(), now.checked_sub(waited).unwrap_or(now)));
                }
                if stats.pto_count > 0
                    && let Some(sent) = stats.unacked_since
                {
                    backed_off.push((*conn_id, *path_id, tuple.clone(), sent));
                }
                if let Some(at) = stats.last_received {
                    match heard.iter_mut().find(|(t, _)| t == tuple) {
                        Some((_, last)) => *last = (*last).max(at),
                        None => heard.push((tuple.clone(), at)),
                    }
                }
                if Some(tuple) == selected
                    && stats.pto_count > 0
                    && let Some(sent) = stats.unacked_since
                {
                    let waited = now.into_std().saturating_duration_since(sent);
                    waiting = Some(waiting.map_or(waited, |w: Duration| w.max(waited)));
                }
                let previous_quality = measured
                    .recovery
                    .contains_key(tuple)
                    .then(|| measured.road(tuple).map(|(rtt, _)| rtt))
                    .flatten();
                let observed = match measured.rtts.get_mut(&key) {
                    Some(r) => {
                        let observed = r.observe(&stats, now);
                        if observed && Some(tuple) == selected {
                            sample = sample.max(stats.latest_rtt);
                        }
                        observed
                    }
                    None => {
                        let since = measured
                            .rtts
                            .values()
                            .filter(|r| r.road == *tuple)
                            .filter_map(|r| r.quality_since)
                            .max();
                        let reading = match since {
                            Some(since) => {
                                Reading::with_epoch(tuple.clone(), &stats, now, Some(since))
                            }
                            None => Reading::new(tuple.clone(), &stats, now),
                        };
                        measured.rtts.insert(key, reading);
                        false
                    }
                };
                if observed && let Some(previous) = previous_quality {
                    measured.recovery_quality_step(key, stats.latest_rtt, previous, now);
                }
                // Opt-in diagnosis: a successful PING request does not show
                // which copy supplied the road's subsequent quality sample.
                if tracing::enabled!(target: "khor_net_samples", tracing::Level::DEBUG)
                    && !tuple.is_ip()
                    && !tuple.is_relay()
                {
                    debug!(target: "khor_net_samples", ?conn_id, ?path_id, ?tuple, ?now,
                        ?stats, observed, reading = ?measured.rtts.get(&key),
                        recovery = ?measured.recovery.get(tuple),
                        moving = ?self.state.moving,
                        "carried recovery copy measurement");
                }
            }
        }
        measured.rtts.retain(|key, r| {
            r.expire(now);
            present.contains(key) || !r.changed.is_empty()
        });
        for conn in self.connections.values() {
            conn.path_state
                .record_measurements(conn.paths.iter().filter_map(|(id, road)| {
                    measured
                        .observation(road)
                        .map(|observation| (*id, observation))
                }));
        }
        // Reachability is independent of quality and transport type. A first
        // post-judgment round trip restores it; repeated faults retain the hold.
        // The historical copy-waking policy is separate and unchanged here.
        let off_direct = selected.is_some_and(|s| !s.is_ip());
        if !silent_now.is_empty() {
            debug!(target: "khor_net_samples", ?now, ?selected, ?backed_off, ?heard,
                wake_allowed = off_direct, "road recovery inputs");
        }
        let connections = &self.connections;
        let open = |tuple: &transports::FourTuple| {
            connections
                .values()
                .any(|c| c.paths.values().any(|t| t == tuple))
        };
        let recovery = measured.complete_read(now, &silent_now, &heard, open);
        if off_direct {
            self.wake(&backed_off, &heard);
        }
        (sample, waiting, recovery)
    }

    /// carry (10-23): pings every copy of a road that is in a probe timeout's backoff while the
    /// road heard something, on any copy, after the packet that copy waits on went out.
    ///
    /// The road answers again, and the copy would only learn so at its next probe timeout,
    /// which a stall of seconds backed off to seconds as well: netlab f4 (x–m black for 3 s),
    /// m's busy connection to x took a round trip a few hundred ms after the leg came back,
    /// an idle one only 2 s later, and the road was as silent as that idle copy
    /// ([`Measured::mark_silent`]); m's connection to x stayed on the relay, 1.7 s late, and
    /// what y sent x through m with it.  The PING is answered in one round trip, and its ACK
    /// ends the backoff (RFC 9002: an acknowledgement resets the probe count).  Nothing is
    /// pinged on a road that heard nothing (a blackhole) or on a copy with nothing waiting;
    /// at most once a tick each, and only until the copy's own round trip comes back.
    ///
    /// Only while the connection is off direct (on the relay or a carried road), as with the
    /// rest of the 10-23 rules: between two direct roads in the same stall (netlab p1s, 300 ms
    /// spikes on both), a PING woken mid-spike took a spike-long sample on the one road, its
    /// smoothed RTT rose an eighth of 300 ms over the other's, and the connection moved and
    /// came back (three runs in six went over p1s's one move; none before).
    fn wake(
        &self,
        backed_off: &[(ConnId, PathId, transports::FourTuple, std::time::Instant)],
        heard: &[(transports::FourTuple, std::time::Instant)],
    ) {
        for (conn_id, path_id) in to_wake(backed_off, heard) {
            let Some(path) = self
                .connections
                .get(&conn_id)
                .and_then(|c| c.handle.upgrade())
                .and_then(|c| c.path(path_id))
            else {
                continue;
            };
            let _ = path.ping();
        }
    }

    /// khor patch 19/34: a selected path that stopped answering, or whose round trip
    /// stays well past its floor with fresh evidence, starts a hole-punching round:
    /// the other direct roads may have been closed (only one direct path is kept per
    /// connection without a backup) and only a round opens them again.  Upstream checks every
    /// [`UPGRADE_INTERVAL`] (60 s), and only whether some direct path is under 10 ms.  One
    /// round per episode; the round's own spacing ([`HOLEPUNCH_ATTEMPTS_INTERVAL`]) holds.
    ///
    /// A path that is idle but for its keepalive has one RTT sample every 5 s, and the
    /// transport estimator moves an eighth of the way per sample. One ACK-delay-corrected sample well
    /// over the floor has the path pinged every tick for [`PROBE_FOR`], once per episode:
    /// the recent sample median then distinguishes sustained slowing from a single spike.
    /// Normal quality must stay high for that window and have two post-decision samples;
    /// quality above the entire window retains immediate exploration rather than waiting
    /// for round trips that cannot fit. The existing probe spacing and budget do not change.
    ///
    /// A road that stopped answering starts a round only once it kept the connection waiting
    /// [`PUNCH_SILENT_AFTER`] (a shorter stall the selector rides out on the road, or leaves
    /// for one that is not direct, which starts a round then: `select_path_on`), and the rounds
    /// this starts are at least [`PUNCH_SPACING`] apart however often the selection moves
    /// ([`Watch`]).  Each move used to start the episode afresh: a road judged silent and
    /// back within a second was a round every time, ~10 a minute between MacLab and lecun.
    fn watch_selected(&mut self, sample: Duration) {
        let Some(selected) = self.state.selected_path.clone() else {
            return;
        };
        let Some((rtt, _)) = self.state.measured.road(&selected) else {
            return;
        };
        let now = Instant::now();
        let stall = self
            .state
            .measured
            .silent
            .get(&selected)
            .filter(|s| s.answered.is_none())
            .map(|s| s.stall(now));
        let measured = &self.state.measured;
        let watched = self.state.watch.tick(rtt, sample, stall, now, |since| {
            measured.samples_after(&selected, since) >= 2
        });
        if watched.probe {
            // One copy is enough: the road's RTT is the copy's that takes the samples.
            let copy = self.connections.values().find_map(|c| {
                let path_id = c.paths.iter().find(|(_, t)| **t == selected)?.0;
                c.handle.upgrade()?.path(*path_id)
            });
            if let Some(path) = copy {
                let _ = path.ping();
            }
        }
        if watched.punch {
            debug!(
                ?rtt,
                ?stall,
                "selected path degraded, triggering holepunching"
            );
            self.trigger_holepunching();
        }
    }

    /// Propagates a change of [`State::selected_path`] to noq.
    ///
    /// Iterates over all connections and applies the selected path as follows:
    /// - Closes non-selected IP paths (but keeps one IP path open still)
    /// - Sets all non-selected paths to [`PathStatus::Backup`]
    /// - Opens the selected path if it does not exist on the connection
    /// - Sets the selected path to [`PathStatus::Available`]
    fn apply_selected_path(&mut self) {
        let backup = self.state.backup_path.clone();
        // khor patch 17: the path a move is being confirmed to stays open too.
        let moving = self.state.moving.to().cloned();
        let Some(selected) = self.state.selected_path.clone() else {
            // We can't open the selected path on all paths if we don't have one yet.  And
            // we can't close all "unselected" paths either, because we don't know which one
            // is selected.
            return;
        };

        let mut retry_selected = false;
        for (conn_id, conn_state) in self.connections.iter_mut() {
            let Some(conn) = conn_state.handle.upgrade() else {
                continue;
            };

            // Open path if it doesn't exist yet.
            let outcome = self
                .state
                .open_path_on_conn(*conn_id, conn_state, &conn, &selected);
            retry_selected |= outcome == OpenPathOutcome::Retry;

            for (path_id, path_fourtuple) in conn_state.paths.iter() {
                let Some(path) = conn.path(*path_id) else {
                    continue;
                };

                // Closes redundant IP paths so that at most one remains per connection.
                //
                // Relay and custom paths are kept open. Only the client closes paths,
                // to avoid the client and server independently closing different paths
                // and racing to abandon the last one.
                //
                // khor patch 18: and the backup the selector named is kept too, so at most
                // two remain.  Still only the client closes.
                if conn.side().is_client()
                    && path_fourtuple.is_ip()
                    && path_fourtuple != &selected
                    && Some(path_fourtuple) != backup.as_ref()
                    && Some(path_fourtuple) != moving.as_ref()
                    && conn_state.paths.values().filter(|a| a.is_ip()).count() > 1
                {
                    trace!(?path_fourtuple, %conn_id, %path_id, "closing direct path");
                    match path.close() {
                        Err(noq_proto::ClosePathError::MultipathNotNegotiated) => {
                            error!("multipath not negotiated");
                        }
                        Err(noq_proto::ClosePathError::LastOpenPath) => {
                            error!("could not close last open path");
                        }
                        Err(noq_proto::ClosePathError::ClosedPath) => {
                            // We already closed this.
                        }
                        Ok(()) => {}
                    }
                    continue;
                }

                // Set path status: The selected path becomes Available, all other paths
                // become Backup.
                self.state.set_path_status(*conn_id, &path, path_fourtuple);
            }

            // Record the new selected path in the path watcher.
            conn_state.path_state.record_selected(&selected);
        }

        if retry_selected {
            self.state.queue_pending_open_path(&selected);
        }
    }

    /// Opens `open_addr` as a new path on every connection to the remote.
    ///
    /// However many connections fail to open it, the address is queued for retry at most once:
    /// it is the address that is retried, not the connection.
    fn open_path_on_all_conns(&mut self, open_addr: &transports::FourTuple) {
        let mut retry = false;
        for (conn_id, conn_state) in self.connections.iter_mut() {
            let Some(conn) = conn_state.handle.upgrade() else {
                continue;
            };
            let outcome = self
                .state
                .open_path_on_conn(*conn_id, conn_state, &conn, open_addr);
            retry |= outcome == OpenPathOutcome::Retry;
        }
        if retry {
            self.state.queue_pending_open_path(open_addr);
        }
    }

    /// Handles regularly checking if any paths need hole punching currently
    ///
    /// Currently we need to have 1 IP path, with a good enough latency.
    ///
    /// khor patch 19 (10-21): one such path on any connection is enough, read as the
    /// measurement ticks read the road ([`Measured::road`]).  Upstream wanted one on every
    /// connection, by each copy's own RTT: a remote's second connection that carries nothing
    /// keeps the RTT of its last keepalive, or never had the direct path (the remote opened
    /// it, and only the client opens paths), and either was a round every 60 s however good
    /// the road in use.
    fn check_connections(&mut self) {
        let mut direct = Vec::new();
        for conn_state in self.connections.values() {
            let Some(conn) = conn_state.handle.upgrade() else {
                continue;
            };
            for (path_id, addr) in conn_state.paths.iter().filter(|(_, a)| a.is_ip()) {
                let rtt = match self.state.measured.road(addr) {
                    Some((rtt, _)) => Some(rtt),
                    None => conn.path_stats(*path_id).map(|stats| stats.rtt),
                };
                direct.extend(rtt);
            }
        }
        if !good_enough(direct) {
            debug!("connections are not good enough, triggering holepunching");
            self.trigger_holepunching();
        }
    }
}

/// khor patch 19 (10-21): whether some direct road, of these round trips, is good enough that
/// no round is needed.
fn good_enough(direct: impl IntoIterator<Item = Duration>) -> bool {
    direct.into_iter().any(|rtt| rtt <= GOOD_ENOUGH_LATENCY)
}

impl State {
    /// Handles [`RemoteStateMessage::SendDatagram`].
    fn handle_msg_send_datagram(
        &mut self,
        sender: Box<TransportsSender>,
        transmit: OwnedTransmit,
        send_tasks: &mut FuturesUnorderedBounded<Boxed<()>>,
    ) {
        // Sending datagrams might fail, e.g. because we don't have the right transports set
        // up to handle sending this owned transmit to.
        // After all, we try every single path that we know (relay URL, IP address), even
        // though we might not have a relay transport or ip-capable transport set up.
        // So these errors must not be fatal for this actor (or even this operation).

        let targets = if let Some(addr) = self.selected_path.as_ref() {
            trace!(?addr, "sending datagram to selected path");

            // TODO(Frando): We might want to include a local IP here in the future, if we confidently
            // know that it is the correct one.
            // See https://github.com/n0-computer/iroh/issues/4280.
            smallvec![transports::FourTuple::from_remote(addr.remote())]
        } else {
            trace!(
                paths = ?self.paths.addrs().collect::<Vec<_>>(),
                "sending datagram to all known paths",
            );
            if self.paths.is_empty() {
                warn!("Cannot send datagrams: No paths to remote endpoint known");
            }

            let mut targets = SmallVec::new();
            for addr in self.paths.addrs() {
                // We never want to send to our local addresses.
                // The local address set is updated in the main loop so we can use `peek` here.
                if let transports::Addr::Ip(sockaddr) = addr
                    && self
                        .local_direct_addrs
                        .peek()
                        .iter()
                        .any(|a| a.addr == *sockaddr)
                {
                    trace!(%sockaddr, "not sending datagram to our own address");

                // TODO(Frando): We might want to include a local IP here in the future, if we confidently
                // know that it is the correct one.
                // See https://github.com/n0-computer/iroh/issues/4280.
                } else {
                    targets.push(transports::FourTuple::from_remote(addr.clone()));
                }
            }
            // This message is received *before* a connection is added.  So we do
            // not yet have a connection to holepunch.  Instead we trigger
            // holepunching when AddConnection is received.
            targets
        };

        if targets.is_empty() {
            return;
        }
        let send = Box::pin(
            async move {
                if time::timeout(
                    DATAGRAM_SEND_TIMEOUT,
                    send_datagram_to_targets(sender, transmit, targets),
                )
                .await
                .is_err()
                {
                    debug!("Datagram send timed out");
                }
            }
            .instrument(Span::current()),
        );
        if send_tasks.try_push(send).is_err() {
            debug!("dropping datagram: send task limit reached");
        }
    }

    /// Handles [`RemoteStateMessage::ResolveRemote`].
    fn handle_msg_resolve_remote(
        &mut self,
        addrs: BTreeSet<TransportAddr>,
        tx: oneshot::Sender<Result<(), AddressLookupFailed>>,
    ) {
        let addrs = to_transports_addr(self.endpoint_id, addrs);
        self.paths.insert_multiple(addrs, Source::App);
        self.paths.resolve_remote(tx);
        // Start Address Lookup if we have no selected path.
        self.trigger_address_lookup();
    }

    /// Triggers Address Lookup for the remote endpoint, if needed.
    ///
    /// Does not start Address Lookup if we have a selected path or if Address Lookup is
    /// currently running.
    fn trigger_address_lookup(&mut self) {
        if self.selected_path.is_some() || self.address_lookup_stream.is_some() {
            return;
        }
        let stream = self.address_lookup.resolve(self.endpoint_id);
        let stream = stream.filter_map(|item| match item {
            // We don't care about errors from individual services, we just continue.
            // Individual errors are buffered into the final error by `AddressLookupServices::resolve`,
            // and if the lookup fails we return them upstream with the final `AddressLookupFailed` error.
            Ok(Err(_err)) => None,
            Ok(Ok(item)) => Some(Ok(item)),
            Err(err) => Some(Err(err)),
        });
        self.address_lookup_stream = Some(Box::pin(stream));
    }

    /// Handles an address lookup result.
    ///
    /// All address lookup results end up being sent here. It takes care of updating the
    /// [`RemotePathState`] with the results.
    fn handle_address_lookup_item(
        &mut self,
        item: Option<Result<AddressLookupItem, AddressLookupFailed>>,
    ) {
        match item {
            None => {
                self.paths.address_lookup_finished(Ok(()));
                self.address_lookup_stream = None;
            }
            Some(Err(err)) => {
                if let AddressLookupFailed::NoServiceConfigured { .. } = err {
                    trace!("Address Lookup not configured");
                } else {
                    debug!("Address Lookup failed: {err:#}");
                }
                self.paths.address_lookup_finished(Err(err));
                self.address_lookup_stream = None;
            }
            Some(Ok(item)) => {
                if item.endpoint_id() != self.endpoint_id {
                    warn!(
                        ?item,
                        "Address Lookup emitted item for wrong remote endpoint"
                    );
                } else {
                    let source = Source::AddressLookup {
                        name: item.provenance().to_string(),
                    };
                    let addrs =
                        to_transports_addr(self.endpoint_id, item.into_endpoint_addr().addrs);
                    self.paths.insert_multiple(addrs, source);
                }
            }
        }
    }

    /// Unconditionally perform holepunching.
    #[instrument(skip_all)]
    fn do_holepunching(&mut self, conn: noq::Connection) {
        self.metrics.holepunch_attempts.inc();
        let local_candidates = self.local_candidates();
        match conn.initiate_nat_traversal_round() {
            Ok(remote_candidates) => {
                let remote_candidates = remote_candidates
                    .iter()
                    .map(|addr| SocketAddr::new(addr.ip().to_canonical(), addr.port()))
                    .collect::<BTreeSet<SocketAddr>>();
                event!(
                    target: "iroh::_events::qnt::init",
                    Level::DEBUG,
                    remote = %self.endpoint_id.fmt_short(),
                    ?local_candidates,
                    ?remote_candidates,
                );
                // khor patch 8: no subscriber is no error.
                let _ = self.rounds.send(HolepunchRound {
                    remote: self.endpoint_id,
                    local: local_candidates.iter().copied().collect(),
                    remote_candidates: remote_candidates.iter().copied().collect(),
                });
                self.last_holepunch = Some(HolepunchAttempt {
                    when: Instant::now(),
                    local_candidates,
                    remote_candidates,
                });
            }
            Err(err) => {
                debug!("failed to initiate NAT traversal: {err:#}");
                use noq_proto::n0_nat_traversal::Error;
                match err {
                    Error::Closed
                    | Error::TooManyAddresses
                    | Error::WrongConnectionSide
                    | Error::ExtensionNotNegotiated => {
                        // Fatal, no need to retry for now
                    }
                    Error::Multipath(_) | Error::NotEnoughAddresses => {
                        // Retry in a bit
                        let now = Instant::now();
                        let next_hp = now + Duration::from_millis(100);
                        trace!(scheduled_in = ?(next_hp - now), "holepunching retry");
                        self.scheduled_holepunch = Some(next_hp);
                    }
                }
            }
        }
    }

    /// Register a path with our state and configure path-specific settings.
    ///
    /// This inserts the path in the [`ConnectionState`] and [`Self::paths`].
    ///
    /// It configures the path with the correct path status (see [`Self::set_path_status`]),
    /// and applies path-type-specific settings:
    /// Relay paths get a longer idle timeout to accommodate transparent reconnection
    /// by the relay actor (see [`RELAY_PATH_MAX_IDLE_TIMEOUT`]).
    fn register_and_configure_path(
        &mut self,
        conn_id: ConnId,
        conn_state: &mut ConnectionState,
        path: &noq::Path,
    ) -> Option<transports::FourTuple> {
        let network_path = self
            .mapped_addrs
            .to_transport_tuple(&path.network_path().ok()?)?;
        event!(
            target: "iroh::_events::path::open",
            Level::DEBUG,
            remote = %self.endpoint_id.fmt_short(),
            %conn_id,
            path_id=%path.id(),
            %network_path,
        );
        conn_state.add_open_path(network_path.clone(), path.id(), &self.metrics);
        // khor patch 18: a direct path opened as a backup gets the spare's pulse; one whose
        // status then changes is set again in `set_path_status`.
        pulse(
            path,
            &network_path,
            self.path_status_for_addr(&network_path),
        );
        if network_path.is_relay()
            && let Err(e) = path.set_max_idle_timeout(Some(RELAY_PATH_MAX_IDLE_TIMEOUT))
        {
            debug!(?e, "failed to set relay path idle timeout");
        }

        self.set_path_status(conn_id, path, &network_path);
        self.paths
            .insert_open_path(network_path.remote(), Source::Connection);
        Some(network_path)
    }

    fn set_path_status(
        &mut self,
        conn_id: ConnId,
        path: &noq::Path,
        network_path: &transports::FourTuple,
    ) {
        let status = self.path_status_for_addr(network_path);
        match path.set_status(status) {
            Err(error) => warn!(?error, ?network_path, ?status, "set_status failed"),
            Ok(prev_status) if prev_status != status => {
                pulse(path, network_path, status);
                event!(
                    target: "iroh::_events::path::set_status",
                    Level::DEBUG,
                    remote = %self.endpoint_id.fmt_short(),
                    %conn_id,
                    path_id=%path.id(),
                    %network_path,
                    ?status,
                    ?prev_status,
                );
            }
            Ok(_) => {}
        }
    }

    /// Opens `open_4tuple` as a new path on one connection.
    ///
    /// The returned [`OpenPathOutcome`] says whether the address is worth retrying later; the
    /// caller is what queues that retry, once for the address rather than once per connection.
    fn open_path_on_conn(
        &mut self,
        conn_id: ConnId,
        conn_state: &mut ConnectionState,
        conn: &noq::Connection,
        open_4tuple: &transports::FourTuple,
    ) -> OpenPathOutcome {
        // Only the client opens paths; the server receives them via
        // QUIC frames and reacts to PathOpened events.
        if conn.side().is_server() {
            return OpenPathOutcome::Done;
        }
        // Already open on this connection; nothing to do.
        if conn_state.paths.values().any(|a| a == open_4tuple) {
            return OpenPathOutcome::Done;
        }
        // This connection can carry no further path until one of its paths is abandoned.
        if conn_state.path_ids.exhausted() {
            conn_state.path_ids.wait(open_4tuple);
            return OpenPathOutcome::Done;
        }

        let mapped_4tuple = self.mapped_addrs.to_mapped_tuple(open_4tuple);
        let path_status = self.path_status_for_addr(open_4tuple);

        let fut = conn.open_path_ensure(mapped_4tuple, path_status);
        match fut.path_id() {
            Some(path_id) => {
                trace!(%conn_id, %path_id, ?path_status, "opening new path");
                OpenPathOutcome::Done
            }
            None => {
                let ret = now_or_never(fut);
                match ret {
                    Some(Err(PathError::RemoteCidsExhausted)) => {
                        trace!(?open_4tuple, ?ret, "scheduling open_path");
                        OpenPathOutcome::Retry
                    }
                    Some(Err(PathError::MaxPathIdReached)) => {
                        // khor patch 1 (10-21): retrying at once only meets the limit again,
                        // so the address waits for this connection's next abandoned path,
                        // after which the remote raises the limit (`PathIds`).
                        trace!(
                            %conn_id,
                            ?open_4tuple,
                            "max path id reached, opening when a path is abandoned"
                        );
                        conn_state.path_ids.wait(open_4tuple);
                        OpenPathOutcome::Done
                    }
                    _ => {
                        warn!(?ret, "Opening path failed");
                        OpenPathOutcome::Done
                    }
                }
            }
        }
    }

    /// khor patch 19 (10-21): a round no later than `at`, however many ask for one.
    fn punch_soon(&mut self, at: Instant) {
        self.scheduled_holepunch = Some(self.scheduled_holepunch.map_or(at, |s| s.min(at)));
    }

    /// Queues `open_addr` to be opened again after [`OPEN_PATH_RETRY_DELAY`].
    ///
    /// The queue holds each address at most once and never more than
    /// [`MAX_PENDING_OPEN_PATHS`] of them, so a remote that keeps refusing paths costs a
    /// bounded amount of memory however many connections and addresses it is reached on.
    fn queue_pending_open_path(&mut self, open_addr: &transports::FourTuple) {
        self.scheduled_open_path = Some(Instant::now() + OPEN_PATH_RETRY_DELAY);
        if self.pending_open_paths.contains(open_addr) {
            return;
        }
        if self.pending_open_paths.len() >= MAX_PENDING_OPEN_PATHS {
            self.pending_open_paths.pop_front();
        }
        self.pending_open_paths.push_back(open_addr.clone());
    }

    /// Returns the [`PathStatus`] for `addr`.
    ///
    /// Returns [`PathStatus::Available`] if `addr` is the currently-selected path,
    /// or [`PathStatus::Backup`] otherwise.
    fn path_status_for_addr(&self, addr: &transports::FourTuple) -> PathStatus {
        if Some(addr) == self.selected_path.as_ref() {
            PathStatus::Available
        } else {
            PathStatus::Backup
        }
    }

    /// Returns the current set of local direct addresses.
    ///
    /// khor patch 9: only those the endpoint's address filter lets through.
    /// Upstream offers every direct address to the remote, a proxy's
    /// fake-IP TUN address (198.18.0.0/15) among them, which no peer can
    /// reach and a peer running the same proxy reaches on itself.
    fn local_candidates(&mut self) -> BTreeSet<SocketAddr> {
        let all = self
            .local_direct_addrs
            .get()
            .iter()
            .map(|d| TransportAddr::Ip(d.addr))
            .collect();
        self.address_lookup
            .filtered(all)
            .into_iter()
            .filter_map(|a| match a {
                TransportAddr::Ip(sa) => Some(sa),
                _ => None,
            })
            .collect()
    }

    /// khor patch 10: an address the remote says it sees this endpoint's
    /// packets come from, on a direct path. Kept for [`PEER_OBSERVED_FOR`]
    /// and offered like a QAD address from the next direct-address update
    /// on; at most [`PEER_OBSERVED_MAX`], the oldest giving way.
    fn peer_observed(&self, addr: SocketAddr) {
        let addr = SocketAddr::new(addr.ip().to_canonical(), addr.port());
        // Two endpoints on one machine see each other on loopback: that is
        // no address on the internet.
        if addr.ip().is_loopback() || addr.ip().is_unspecified() || addr.port() == 0 {
            return;
        }
        let mut seen = self.peer_observed.lock().expect("poisoned");
        seen.insert(addr, std::time::Instant::now());
        while seen.len() > PEER_OBSERVED_MAX {
            let Some(oldest) = seen.iter().min_by_key(|(_, t)| **t).map(|(a, _)| *a) else {
                break;
            };
            seen.remove(&oldest);
        }
    }
}

/// khor patch 18: the keepalive and idle timeout of a direct path for its status.  The
/// selected path keeps iroh's own (5 s, 15 s); an unselected one, a spare that carries
/// nothing, is pinged every [`SPARE_KEEP_ALIVE`].  A relay path behind a direct one is pinged
/// as seldom: each of its pings costs an ACK on the relay too (vendor/noq-proto patch 4), and
/// the relay connection has its own ping.  Custom paths are left as configured.  Set only
/// when the status changes: setting the idle timeout rearms its timer.
fn pulse(path: &noq::Path, network_path: &transports::FourTuple, status: PathStatus) {
    if network_path.is_relay() {
        // Only the keepalive: the relay path keeps its own idle timeout
        // (`RELAY_PATH_MAX_IDLE_TIMEOUT`).
        let keep = match status {
            PathStatus::Available => crate::socket::HEARTBEAT_INTERVAL,
            PathStatus::Backup => SPARE_KEEP_ALIVE,
        };
        let _ = path.set_keep_alive_interval(Some(keep));
        return;
    }
    if !network_path.is_ip() {
        return;
    }
    let (keep, idle) = match status {
        PathStatus::Available => (
            crate::socket::HEARTBEAT_INTERVAL,
            crate::socket::PATH_MAX_IDLE_TIMEOUT,
        ),
        PathStatus::Backup => (SPARE_KEEP_ALIVE, SPARE_IDLE_TIMEOUT),
    };
    let _ = path.set_keep_alive_interval(Some(keep));
    let _ = path.set_max_idle_timeout(Some(idle));
}

/// Updates QNT's candidate addresses to be the current set of direct addresses.
///
/// `direct_addrs` must be a set of addresses extracted from the endpoint's current
/// [`DirectAddr`]s.
fn update_qnt_candidates(conn: &noq::Connection, direct_addrs: &BTreeSet<SocketAddr>) {
    let noq_candidates = match conn.get_local_nat_traversal_addresses() {
        Ok(addrs) => BTreeSet::from_iter(addrs),
        Err(err) => {
            warn!("failed to get local nat candidates: {err:#}");
            return;
        }
    };
    for addr in direct_addrs.difference(&noq_candidates) {
        if let Err(err) = conn.add_nat_traversal_address(*addr) {
            warn!("failed adding local addr: {err:#}",);
        }
    }
    for addr in noq_candidates.difference(direct_addrs) {
        if let Err(err) = conn.remove_nat_traversal_address(*addr) {
            warn!("failed removing local addr: {err:#}");
        }
    }
    trace!(?direct_addrs, "updated local QNT addresses");
}

fn send_datagram<'a>(
    sender: &'a mut TransportsSender,
    addr: transports::FourTuple,
    owned_transmit: OwnedTransmit,
) -> impl Future<Output = n0_error::Result<()>> + 'a {
    std::future::poll_fn(move |cx| {
        let transmit = transports::Transmit {
            ecn: owned_transmit.ecn,
            contents: owned_transmit.contents.as_ref(),
            segment_size: owned_transmit.segment_size,
        };

        Pin::new(&mut *sender)
            .poll_send(cx, &addr, &transmit)
            .map(|res| res.with_context(|_| format!("failed to send datagram to {:?}", addr)))
    })
}

async fn send_datagram_to_targets(
    sender: Box<TransportsSender>,
    transmit: OwnedTransmit,
    targets: SmallVec<[transports::FourTuple; 8]>,
) {
    let mut sends = targets
        .into_iter()
        .map(|target| {
            let mut sender = sender.clone();
            let transmit = transmit.clone();
            async move {
                if let Err(err) = send_datagram(&mut sender, target.clone(), transmit).await {
                    debug!(?target, "failed to send datagram: {err:#}");
                }
            }
        })
        .collect::<FuturesUnordered<_>>();
    while sends.next().await.is_some() {}
}

/// Messages to send to the [`RemoteStateActor`].
#[derive(derive_more::Debug)]
pub(crate) enum RemoteStateMessage {
    /// Sends a datagram to all known paths.
    ///
    /// Used to send QUIC Initial packets.  If there is no working direct path this will
    /// trigger holepunching.
    ///
    /// This is not acceptable to use on the normal send path, as it is an async send
    /// operation with a bunch more copying.  So it should only be used for sending QUIC
    /// Initial packets.
    #[debug("SendDatagram(..)")]
    SendDatagram(Box<TransportsSender>, OwnedTransmit),
    /// Adds an active connection to this remote endpoint.
    ///
    /// The actor will downgrade the connection to a [`noq::WeakConnectionHandle`] as soon
    /// as it processes the message. It will keep hold of the weak handle until it closes,
    /// but only update to a strong [`noq::Connection`] for brief moments.
    ///
    /// The actor will actively manage paths on the connection and start holepunching as needed.
    #[debug("AddConnection({})", _0.stable_id())]
    AddConnection(noq::Connection, oneshot::Sender<PathStateReceiver>),
    /// Asks if there is any possible path that could be used.
    ///
    /// This adds the provided transport addresses to the list of potential paths for this
    /// remote and starts Address Lookup if needed.
    ///
    /// Sends back `Ok` immediately if the provided address list is non-empy or we have are
    /// other known paths.  Otherwise sends back `Ok` once Address Lookup produces a result,
    /// or the Address Lookup error if Address Lookup fails or produces no results,
    #[debug("ResolveRemote(..)")]
    ResolveRemote(
        BTreeSet<TransportAddr>,
        oneshot::Sender<Result<(), AddressLookupFailed>>,
    ),
    /// Returns information about the remote.
    ///
    /// This currently only includes a list of all known transport addresses for the remote.
    RemoteInfo(oneshot::Sender<RemoteInfo>),
    /// The network status has changed in some way
    NetworkChange { is_major: bool },
    /// Retire only old, unconnected IP hints for a cold carrier retry.
    DiscardIpHints {
        before: Instant,
        tx: oneshot::Sender<usize>,
    },
    /// khor patch 13: closes every custom path to this remote on every
    /// connection; the road under them is gone.
    #[debug("CloseCustom(..)")]
    CloseCustom(Option<Box<dyn Fn() -> bool + Send + Sync>>),
}

/// Information about a holepunch attempt.
///
/// Addresses are always stored in canonical form.
#[derive(Debug)]
struct HolepunchAttempt {
    when: Instant,
    /// The set of local addresses which could take part in holepunching.
    ///
    /// This does not mean every address here participated in the holepunching.  E.g. we
    /// could have tried only a sub-set of the addresses because a previous attempt already
    /// covered part of the range.
    ///
    /// We do not store this as a [`DirectAddr`] because this is checked for equality and we
    /// do not want to compare the sources of these addresses.
    local_candidates: BTreeSet<SocketAddr>,
    /// The set of remote addresses which could take part in holepunching.
    ///
    /// Like [`Self::local_candidates`] we may not have used them.
    remote_candidates: BTreeSet<SocketAddr>,
}

/// Newtype to track Connections.
///
/// The wrapped value is the [`noq::Connection::stable_id`] value, and is thus only valid
/// for active connections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::Display)]
#[display("{_0}")]
struct ConnId(usize);

/// A connection managed by the [`RemoteStateActor`].
///
/// - A handle to the connection.
/// - The paths we know about.
/// - Some stuff to make observers happy.
#[derive(Debug)]
struct ConnectionState {
    /// Weak handle to the connection.
    handle: WeakConnectionHandle,
    /// Writer-side handle for the connection's path observation state.
    ///
    /// The matching [`PathStateReceiver`] is held by the [`Connection`].
    ///
    /// [`Connection`]: crate::endpoint::Connection
    path_state: PathStateSender,
    /// The open paths that exist on this connection.
    ///
    /// This might be lagging from the connection state inside noq itself as this can only
    /// be updated once we received the event from noq.
    ///
    /// IP paths *should* have the local IP address filled in by the time we receive the
    /// event. The noq established event is only emitted once at least one datagram is
    /// received from the peer on the path.
    paths: FxHashMap<PathId, transports::FourTuple>,
    /// Whether this connection has ever had a direct path.
    ///
    /// Used for recording metrics.
    has_been_direct: bool,
    /// Whether this connection has run out of path ids, and what waits for one ([`PathIds`]).
    path_ids: PathIds,
}

/// khor patch 1 (10-21): a connection that ran out of path ids, and the addresses waiting to be
/// opened on it.
///
/// [`PathError::MaxPathIdReached`] lasts until a path is abandoned: the peer raises its limit
/// by one for each PATH_ABANDON it receives (noq-proto `connection/mod.rs`, `Frame::PathAbandon`
/// → `set_max_path_id(local_max_path_id + 1)`; noq's own NAT traversal treats the error as
/// temporary and queues the path again).  Upstream PR #4522 marked the connection for good,
/// and a connection that met the limit once, in one round of hole punching on a remote with
/// many candidates, never opened a path again: not a carried road offered later
/// (`Endpoint::offer_addrs`), not the selected path on it.
#[derive(Debug, Default)]
struct PathIds {
    exhausted: bool,
    /// At most [`MAX_PENDING_OPEN_PATHS`], the oldest dropped first.
    waiting: VecDeque<transports::FourTuple>,
}

impl PathIds {
    fn exhausted(&self) -> bool {
        self.exhausted
    }

    /// Opening `addr` met the limit, or would.
    fn wait(&mut self, addr: &transports::FourTuple) {
        self.exhausted = true;
        if self.waiting.contains(addr) {
            return;
        }
        if self.waiting.len() >= MAX_PENDING_OPEN_PATHS {
            self.waiting.pop_front();
        }
        self.waiting.push_back(addr.clone());
    }

    /// A path of the connection was abandoned or discarded: the limit is about to rise, so
    /// the addresses waiting are tried again.
    fn freed(&mut self) -> Vec<transports::FourTuple> {
        self.exhausted = false;
        self.waiting.drain(..).collect()
    }
}

/// The outcome of trying to open one path on one connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OpenPathOutcome {
    /// Nothing further to do: the path is opening, is already open, or failed permanently.
    Done,
    /// The remote had no connection ids left, so this address is worth trying again later.
    Retry,
}

impl ConnectionState {
    /// Tracks an open path for the connection.
    fn add_open_path(
        &mut self,
        network_path: transports::FourTuple,
        path_id: PathId,
        metrics: &Arc<SocketMetrics>,
    ) {
        match network_path {
            transports::FourTuple::Ip { .. } => metrics.paths_direct.inc(),
            transports::FourTuple::Relay { .. } => metrics.paths_relay.inc(),
            transports::FourTuple::Custom { .. } => metrics.paths_custom.inc(),
        };
        if !self.has_been_direct && network_path.is_ip() {
            self.has_been_direct = true;
            metrics.num_conns_direct.inc();
        }

        self.paths.insert(path_id, network_path.clone());
        if let Some(conn) = self.handle.upgrade()
            && let Some(path) = conn.path(path_id)
        {
            let handle = path.weak_handle();
            self.path_state.record_opened(handle, network_path);
        }
    }

    /// Removes a path from this connection.
    fn remove_path(
        &mut self,
        path_id: &PathId,
        conn: &noq::Connection,
    ) -> Option<transports::FourTuple> {
        let addr = self.paths.remove(path_id)?;
        self.path_state.record_abandoned(*path_id, conn);
        Some(addr)
    }
}

/// State of the endpoint relevant for path selection.
///
/// Constructed by the endpoint and passed to [`PathSelector::select`].  Borrows from
/// the endpoint's internal data.
#[derive(Debug)]
#[cfg_attr(not(feature = "unstable-custom-transports"), allow(unreachable_pub))]
pub struct PathSelectionContext<'a> {
    current: Option<&'a transports::FourTuple>,
    /// khor patch 18: the backup kept beside `current`, if any.
    backup: Option<&'a transports::FourTuple>,
    /// khor patch 17: the paths that stopped answering on some connection, each with how long
    /// it has kept the connection waiting ([`PathSelectionData::answering`]).
    silent: Vec<(transports::FourTuple, Duration)>,
    /// khor patch 17 (10-21): how long `current` has been the selection.
    held_for: Option<Duration>,
    /// khor patch 17: each road's RTT as the copy that took the most samples lately measured it
    /// ([`PathSelectionData::rtt`], `Measured::road`).
    fresh: Vec<(transports::FourTuple, Duration)>,
    source: PathsSource<'a>,
}

/// khor patch 19: the selected path as the measurement ticks watch it
/// ([`RemoteStateActor::watch_selected`]).
#[derive(Debug, Default)]
struct Watch {
    /// The lowest round trip it read since it was selected.
    floor: Option<Duration>,
    /// Whether a round was already started for its rising past the floor, or its going silent.
    degraded: bool,
    /// When the selected road's quality most recently rose above its floor margin.
    slowing_since: Option<Instant>,
    /// When it was last probed ([`PROBE_FOR`], [`PROBE_SPACING`]), and whether it was already
    /// for this rise (once per episode, like `degraded`).
    probed_at: Option<Instant>,
    probed: bool,
    /// When this watch last started a round ([`PUNCH_SPACING`]); kept across selections.
    punched_at: Option<Instant>,
}

/// khor patch 19: what one tick of [`Watch`] decided.
#[derive(Debug, Default, PartialEq, Eq)]
struct Watched {
    /// Ping the selected path this tick.
    probe: bool,
    /// Start a hole-punching round.
    punch: bool,
}

impl Watch {
    /// A new selection is measured from its own floor.
    fn reset(&mut self) {
        self.floor = None;
        self.degraded = false;
        self.slowing_since = None;
        self.probed = false;
    }

    /// Whether a round may start now, and if so marks it started.
    fn may_punch(&mut self, now: Instant) -> bool {
        let may = self.punched_at.is_none_or(|at| now >= at + PUNCH_SPACING);
        if may {
            self.punched_at = Some(now);
        }
        may
    }

    /// One tick: the selected road's round trip `rtt`, the largest RTT sample it took since the
    /// last tick, and how long it has kept the connection waiting if it is silent.
    /// `sampled_after` requires two independent observations of packets sent after its marker.
    fn tick(
        &mut self,
        rtt: Duration,
        sample: Duration,
        stall: Option<Duration>,
        now: Instant,
        sampled_after: impl FnOnce(Instant) -> bool,
    ) -> Watched {
        let floor = self.floor.map_or(rtt, |f| f.min(rtt));
        self.floor = Some(floor);
        let by = DEGRADED_BY.max(floor / 2);
        // Only a sample past "good enough" is worth it: below, nothing would be done.
        if sample >= floor + by
            && sample > GOOD_ENOUGH_LATENCY
            && !self.probed
            && !self.degraded
            && self.probed_at.is_none_or(|at| now >= at + PROBE_SPACING)
        {
            self.probed = true;
            self.probed_at = Some(now);
        }
        let probe = self.probed_at.is_some_and(|at| now < at + PROBE_FOR);
        let slowing = rtt > GOOD_ENOUGH_LATENCY && rtt >= floor + by;
        if slowing {
            self.slowing_since.get_or_insert(now);
        } else {
            self.slowing_since = None;
        }
        let confirmed = self
            .slowing_since
            .is_some_and(|since| now >= since + PROBE_FOR && sampled_after(since));
        // Normal quality needs post-decision round trips, not a single spike or a stale
        // reading after the actor resumes. A road slower than the probe window cannot
        // supply those in time, so keep its existing fast exploration, like a true silence.
        let worse = stall.is_some_and(|s| s >= PUNCH_SILENT_AFTER)
            || (slowing && (rtt > PROBE_FOR || confirmed));
        let mut punch = false;
        if worse && !self.degraded {
            // Within the spacing the episode waits for it, still degraded.
            if self.may_punch(now) {
                self.degraded = true;
                punch = true;
            }
        } else if stall.is_none() && rtt < floor + by / 2 {
            self.degraded = false;
            self.probed = false;
        }
        Watched { probe, punch }
    }
}

/// khor patch 17: a move off a path that still answers, being confirmed
/// ([`RemoteStateActor::confirm_move`]).
#[derive(Debug, Default)]
struct Moving {
    /// The path the selection would move to, and since when.
    to: Option<(transports::FourTuple, Instant)>,
    /// A move given up ([`CONFIRM_MAX`]), and until when it is not tried again.
    rest: Option<(transports::FourTuple, Instant)>,
}

/// khor patch 17: what one tick of confirming a move decided.
#[derive(Debug, PartialEq, Eq)]
enum Step {
    /// Move now.
    Go,
    /// Not yet; ping the path moved from, the path moved to, where they took no sample in the
    /// last tick.
    Hold { from: bool, to: bool },
    /// Not now: the move was given up lately.
    Rest,
}

impl Moving {
    /// Wake at a finite confirmation boundary, without a timer at rest. A wake
    /// only re-reads evidence; elapsed time alone never supplies a sample.
    fn deadline(&self, now: Instant) -> Option<Instant> {
        let (_, since) = self.to.as_ref()?;
        [*since + CONFIRM_FOR, *since + CONFIRM_MAX]
            .into_iter()
            .find(|at| now < *at)
    }

    /// One tick of the selection wanting to leave a path that stopped answering at `began` for
    /// `to`: a sample after `began` proves a backup answered during the fault. If another
    /// current copy is still receiving, first probe the suspect copy and obtain a backup
    /// sample after this decision, so an idle copy's PTO cannot immediately move a healthy
    /// road. Either case retains [`CONFIRM_MAX`] as its upper bound.
    fn leave(
        &mut self,
        to: &transports::FourTuple,
        now: Instant,
        began: Instant,
        check_current: bool,
        to_sampled: impl Fn(Instant) -> bool,
    ) -> Step {
        let since = match &self.to {
            Some((moving, since)) if moving == to => *since,
            _ => {
                self.to = Some((to.clone(), now));
                now
            }
        };
        let proof_after = if check_current {
            since.max(began)
        } else {
            began
        };
        if to_sampled(proof_after) || now >= since + CONFIRM_MAX {
            self.to = None;
            return Step::Go;
        }
        let tick = now.checked_sub(RESELECT_INTERVAL).unwrap_or(now).max(began);
        let tick = if check_current { tick.max(since) } else { tick };
        Step::Hold {
            from: check_current,
            to: !to_sampled(tick),
        }
    }

    /// The path a move is being confirmed to.
    fn to(&self) -> Option<&transports::FourTuple> {
        self.to.as_ref().map(|(to, _)| to)
    }

    fn clear(&mut self) {
        self.to = None;
    }

    /// One tick of the selection wanting to move to `to`; `from_rtt`, `to_rtt`: the two paths'
    /// round trips as read now; `from_sampled(t)`, `to_sampled(t)`: whether the path moved from,
    /// to, took a sample after `t`.
    ///
    /// flap (2026-10-23): a move to a path [`far_faster`] is made as soon as `to` took a sample
    /// since the move began: the confirmation is there for two readings within noise of each
    /// other, and the phone's carried road through turing (20 ms) waited it out against a relay
    /// that read 1743 ms.  And a path moved from whose round trip is longer than [`CONFIRM_FOR`]
    /// is not asked for a sample, nor pinged for one: it cannot take one in the confirmation,
    /// and a relay that read 4994 ms had the move off it given up and rest 30 s.
    ///
    /// feat/net: candidate packets must have been sent after this decision began, including
    /// on the fast branch. While waiting, also probe the current road within the existing
    /// confirmation budget; a single scheduling spike must be allowed to correct itself.
    fn step(
        &mut self,
        to: &transports::FourTuple,
        now: Instant,
        from_rtt: Duration,
        to_rtt: Duration,
        from_sampled: impl Fn(Instant) -> bool,
        to_sampled: impl Fn(Instant) -> bool,
    ) -> Step {
        if let Some((rest, until)) = &self.rest {
            if now < *until {
                if rest == to {
                    self.to = None;
                    return Step::Rest;
                }
            } else {
                self.rest = None;
            }
        }
        let since = match &self.to {
            Some((moving, since)) if moving == to => *since,
            _ => {
                self.to = Some((to.clone(), now));
                now
            }
        };
        let far = far_faster(to_rtt, from_rtt);
        let from_slow = from_rtt > CONFIRM_FOR;
        let from_needed = !from_slow;
        if far && to_sampled(since) {
            self.to = None;
            return Step::Go;
        }
        if now >= since + CONFIRM_FOR && (!from_needed || from_sampled(since)) && to_sampled(since)
        {
            self.to = None;
            return Step::Go;
        }
        if now >= since + CONFIRM_MAX {
            self.to = None;
            self.rest = Some((to.clone(), now + CONFIRM_REST));
            return Step::Rest;
        }
        let tick = now.checked_sub(RESELECT_INTERVAL).unwrap_or(now).max(since);
        Step::Hold {
            from: from_needed && !from_sampled(tick),
            to: !to_sampled(tick),
        }
    }
}

/// Activity is evidence of STREAM frames, not PING/ACK traffic. This
/// budget controls exploration only; failure recovery retains its own
/// bounded probes. No new timer is installed on an idle connection.
#[derive(Debug, Default)]
struct ActiveRefresh {
    frames: u64,
    active_until: Option<Instant>,
    next: Option<Instant>,
    cursor: usize,
}

impl ActiveRefresh {
    fn observe(&mut self, frames: u64, now: Instant) {
        if frames > self.frames {
            self.active_until = Some(now + Duration::from_secs(5));
        }
        self.frames = frames;
    }

    fn ready(&self, now: Instant) -> bool {
        self.active_until.is_some_and(|until| now < until) && self.next.is_none_or(|at| now >= at)
    }

    fn take(&mut self, alternatives: usize, now: Instant) -> Option<usize> {
        if alternatives == 0 || !self.ready(now) {
            return None;
        }
        let index = self.cursor % alternatives;
        self.cursor = index + 1;
        self.next = Some(now + Duration::from_secs(5));
        Some(index)
    }
}

/// khor patch 17: what the actor's measurement ticks read ([`RemoteStateActor::read_paths`]).
#[derive(Debug, Default)]
struct Measured {
    /// Each path's smoothed RTT on each connection as last read, and when it last changed.
    rtts: FxHashMap<(ConnId, PathId), Reading>,
    /// The roads that stopped answering.
    silent: FxHashMap<transports::FourTuple, Silence>,
    /// carry (10-23): when each road last answered again after a silence, for [`REHOLD`].
    back: FxHashMap<transports::FourTuple, Instant>,
    /// khor patch 39: bounded recovery episodes; spent entries prevent a new
    /// receipt from minting another budget for the same unresolved silence.
    recovery: FxHashMap<transports::FourTuple, RecoveryCheck>,
}

#[derive(Debug)]
struct RecoveryCheck {
    epoch: Instant,
    began: Instant,
    attempts: u8,
    last: Option<Instant>,
    next: Option<Instant>,
    copy: Option<(ConnId, PathId)>,
    quality_steps: u8,
    quality_best: Option<Duration>,
    /// Last observed quality before this fault, never an initial RTT estimate.
    prior_quality: Option<Duration>,
}

#[derive(Debug)]
struct RecoveryProbe {
    road: transports::FourTuple,
    epoch: Instant,
    attempt: u8,
    at: Instant,
}

const RECOVERY_PROBES: u8 = 3;

/// Rebuild one copy's median; switch copies if the old one closes or backs off.
fn recovery_copy_rank(
    preferred: Option<(ConnId, PathId)>,
    copy: (ConnId, PathId),
    pto_count: u32,
) -> (u32, bool) {
    (pto_count, preferred != Some(copy))
}

/// khor patch 17/35: a road that stopped answering ([`unanswered_for`]), until a new round
/// trip; a repeated fault additionally waits [`SILENT_HOLD`].
#[derive(Debug, Clone, Copy)]
struct Silence {
    /// When the packet it left unanswered went out: the stall began then.
    began: Instant,
    /// The last tick that found it silent.
    judged: Instant,
    /// Since when it answers again: a copy took a round trip after `judged`, and none is
    /// silent now.
    answered: Option<Instant>,
    /// carry (10-23): it went silent within [`REHOLD`] of last being back, so it waits out
    /// [`SILENT_HOLD`] however fast it reads.
    rehold: bool,
    /// A pre-fault observation, held only by this existing silence episode.
    prior_quality: Option<Duration>,
}

impl Silence {
    /// How long the road kept the connection waiting: to now, or to when it answered again.
    fn stall(&self, now: Instant) -> Duration {
        self.answered
            .unwrap_or(now)
            .saturating_duration_since(self.began)
    }
}

/// khor patch 17: one copy of a road as the measurement ticks read it.
#[derive(Debug)]
struct Reading {
    road: transports::FourTuple,
    rtt: Duration,
    samples: u64,
    quality: Duration,
    recent: VecDeque<(Instant, Duration)>,
    /// Sampled packet send and ACK processing instants for fresh observation ticks.
    /// This counts ticks rather than every ACK received between ticks.
    changed: VecDeque<(Instant, Instant)>,
    /// khor patch 41: packets from before a verified recovery quality step
    /// cannot refresh the new quality window, including on another copy.
    quality_since: Option<Instant>,
    /// Send time of the latest sampled packet; absent for an initial estimate.
    sent: Option<Instant>,
    /// When the latest sample's ACK was processed, not when stats were read.
    at: Instant,
}

impl Reading {
    fn new(road: transports::FourTuple, stats: &PathStats, now: Instant) -> Self {
        Self::with_epoch(road, stats, now, None)
    }

    fn with_epoch(
        road: transports::FourTuple,
        stats: &PathStats,
        now: Instant,
        quality_since: Option<Instant>,
    ) -> Self {
        let mut reading = Self {
            road,
            rtt: stats.rtt,
            samples: 0,
            quality: stats.rtt,
            recent: VecDeque::new(),
            changed: VecDeque::new(),
            quality_since,
            sent: None,
            at: now,
        };
        reading.observe(stats, now);
        reading
    }

    fn restart_quality(&mut self, since: Instant, now: Instant) {
        let since = self.quality_since.map_or(since, |old| old.max(since));
        let latest = self
            .recent
            .back()
            .copied()
            .filter(|(at, _)| *at == self.at && self.sent.is_some_and(|sent| sent >= since));
        self.quality_since = Some(since);
        self.recent.clear();
        if let Some((at, sample)) = latest {
            self.record(sample, at, now);
        }
    }

    fn record(&mut self, sample: Duration, received: Instant, now: Instant) {
        self.recent
            .retain(|(at, _)| now.saturating_duration_since(*at) < CONFIRM_FOR);
        if now.saturating_duration_since(received) < CONFIRM_FOR {
            self.recent.push_back((received, sample));
        }
        while self.recent.len() > 8 {
            self.recent.pop_front();
        }
        self.quality = sample;
        self.recalculate();
    }

    fn recalculate(&mut self) {
        if self.recent.is_empty() {
            return;
        }
        let mut samples: Vec<_> = self.recent.iter().map(|(_, rtt)| *rtt).collect();
        samples.sort_unstable();
        self.quality = samples[samples.len() / 2];
    }

    fn expire(&mut self, now: Instant) {
        while self
            .changed
            .front()
            .is_some_and(|(_, at)| now >= *at + CONFIRM_FOR)
        {
            self.changed.pop_front();
        }
        let before = self.recent.len();
        self.recent
            .retain(|(at, _)| now.saturating_duration_since(*at) < CONFIRM_FOR);
        if self.recent.len() != before {
            self.recalculate();
        }
    }

    fn observe(&mut self, stats: &PathStats, now: Instant) -> bool {
        let new_sample = self.samples != stats.rtt_samples;
        self.rtt = stats.rtt;
        self.samples = stats.rtt_samples;
        if stats.rtt_samples == 0 {
            self.quality = stats.rtt;
            return false;
        }
        if !new_sample {
            return false;
        }
        let Some((sent, received)) = stats
            .latest_rtt_sent
            .zip(stats.latest_rtt_received)
            .filter(|(sent, received)| sent <= received)
            .map(|(sent, received)| (Instant::from_std(sent), Instant::from_std(received)))
            .filter(|(_, received)| *received <= now)
        else {
            return false;
        };
        if self.quality_since.is_some_and(|since| sent < since) {
            return false;
        }
        self.sent = Some(sent);
        self.at = received;
        self.record(stats.latest_rtt, received, now);
        let fresh = now.saturating_duration_since(received) < CONFIRM_FOR;
        if fresh {
            self.changed.push_back((sent, received));
        }
        fresh
    }
}

impl Measured {
    /// Complete one observation tick using the existing reachability and probe rules.
    fn complete_read(
        &mut self,
        now: Instant,
        silent_now: &[(transports::FourTuple, Instant)],
        heard: &[(transports::FourTuple, std::time::Instant)],
        open: impl Fn(&transports::FourTuple) -> bool,
    ) -> Vec<RecoveryProbe> {
        let probes = self.recovery_probes(now, silent_now, heard, &open);
        self.mark_silent(now, silent_now, open);
        probes
    }

    /// A recovered road can improve in steps as its substrate leaves a slow
    /// fallback. Restart only on a fresh, substantial, monotonically lower step;
    /// ordinary healthy medians and the subsequent Moving confirmation stay intact.
    fn recovery_quality_step(
        &mut self,
        key: (ConnId, PathId),
        sample: Duration,
        previous: Duration,
        now: Instant,
    ) {
        let Some(reading) = self.rtts.get(&key) else {
            return;
        };
        let Some(sent) = reading.sent else { return };
        let road = reading.road.clone();
        let Some(check) = self.recovery.get_mut(&road) else {
            return;
        };
        // khor patch 46: returning to an actually observed pre-fault regime
        // starts one fresh window even if the transient median is less than
        // 25% slower. This supplies no time or samples to Moving confirmation.
        let returned = check.quality_steps == 0
            && sample < previous
            && check.prior_quality.is_some_and(|prior| sample <= prior);
        if now >= check.began + CONFIRM_FOR
            || sent <= check.began
            || check.quality_steps >= RECOVERY_PROBES
            || !returned && sample.saturating_mul(4) > previous.saturating_mul(3)
            || check
                .quality_best
                .is_some_and(|best| sample.saturating_mul(4) > best.saturating_mul(3))
        {
            return;
        }
        check.quality_steps += 1;
        check.quality_best = Some(sample);
        debug!(target: "khor_net_samples", ?road, ?sent, ?previous, ?sample,
            step = check.quality_steps, "recovery quality window restarted");
        for reading in self.rtts.values_mut().filter(|r| r.road == road) {
            reading.restart_quality(sent, now);
        }
    }

    fn recovery_deadline(&self) -> Option<Instant> {
        self.recovery.values().filter_map(|check| check.next).min()
    }
    /// Requests, including failures or merging with an existing confirmation,
    /// consume the episode's budget. No extra timer or connection owner is kept.
    fn recovery_probes(
        &mut self,
        now: Instant,
        silent_now: &[(transports::FourTuple, Instant)],
        heard: &[(transports::FourTuple, std::time::Instant)],
        open: impl Fn(&transports::FourTuple) -> bool,
    ) -> Vec<RecoveryProbe> {
        let silent = &self.silent;
        self.recovery.retain(|road, check| {
            open(road)
                && match silent.get(road) {
                    Some(silence) => silence.began == check.epoch,
                    None => now < check.began + CONFIRM_FOR,
                }
        });
        for (road, silence) in &self.silent {
            if silence.answered.is_some()
                || silent_now.iter().any(|(t, _)| t == road)
                || !heard
                    .iter()
                    .any(|(t, at)| t == road && *at > silence.judged.into_std())
                || !self
                    .rtts
                    .values()
                    .any(|r| r.road == *road && r.samples > 0 && r.sent.is_some())
                || !open(road)
            {
                continue;
            }
            self.recovery.entry(road.clone()).or_insert(RecoveryCheck {
                epoch: silence.began,
                began: now,
                attempts: 0,
                last: None,
                next: None,
                copy: None,
                quality_steps: 0,
                quality_best: None,
                prior_quality: silence.prior_quality,
            });
        }
        let roads: Vec<_> = self.recovery.keys().cloned().collect();
        let mut probes = Vec::new();
        for road in roads {
            let began = self.recovery[&road].began;
            let sampled = self.samples_after(&road, began) >= usize::from(RECOVERY_PROBES);
            debug!(target: "khor_net_samples", ?road, ?now, ?began, sampled,
                representative = ?self.representative(&road),
                check = ?self.recovery.get(&road), "recovery road measurement");
            let check = self
                .recovery
                .get_mut(&road)
                .expect("known recovery episode");
            check.next = None;
            if sampled {
                check.attempts = RECOVERY_PROBES;
            }
            if check.attempts >= RECOVERY_PROBES
                || now >= check.began + CONFIRM_FOR
                || silent_now.iter().any(|(t, _)| t == &road)
            {
                continue;
            }
            if let Some(next) = check
                .last
                .map(|last| last + RESELECT_INTERVAL)
                .filter(|next| now < *next)
            {
                check.next = (next < check.began + CONFIRM_FOR).then_some(next);
                continue;
            }
            check.attempts += 1;
            check.last = Some(now);
            if check.attempts < RECOVERY_PROBES
                && now + RESELECT_INTERVAL < check.began + CONFIRM_FOR
            {
                check.next = Some(now + RESELECT_INTERVAL);
            }
            probes.push(RecoveryProbe {
                road,
                epoch: check.epoch,
                attempt: check.attempts,
                at: now,
            });
        }
        probes
    }

    /// khor patch 17: one tick's silences: `silent_now` the roads a copy of which does not
    /// answer, each with when the packet it left waiting went out; `open` whether a road is
    /// still a path on some connection.  A road no connection has any more is forgotten; one
    /// a copy of which took a round trip after the latest silence judgment, and none of
    /// which is silent now, is available again. Quality is evaluated by selection/Moving,
    /// not by this reachability gate. A fault within [`REHOLD`] of a previous recovery
    /// still requires [`SILENT_HOLD`] of answering so an intermittent road cannot flap.
    fn mark_silent(
        &mut self,
        now: Instant,
        silent_now: &[(transports::FourTuple, Instant)],
        open: impl Fn(&transports::FourTuple) -> bool,
    ) {
        self.back.retain(|_, at| now < *at + REHOLD);
        for (tuple, began) in silent_now {
            let rehold = self.back.contains_key(tuple);
            // Do not save a transient post-fault sample as the old regime.
            // Use the same active-copy priority as ordinary road measurements.
            let prior_quality = self
                .rtts
                .values()
                .filter(|r| r.road == *tuple && r.samples > 0 && r.sent.is_some() && r.at <= *began)
                .max_by_key(|r| (r.recent.len(), r.at))
                .map(|r| r.quality);
            let silence = self.silent.entry(tuple.clone()).or_insert(Silence {
                began: *began,
                judged: now,
                answered: None,
                rehold,
                prior_quality,
            });
            silence.judged = now;
            silence.answered = None;
        }
        for (tuple, silence) in self.silent.iter_mut() {
            if silent_now.iter().any(|(t, _)| t == tuple) || !open(tuple) {
                continue;
            }
            if silence.answered.is_none()
                && self
                    .rtts
                    .values()
                    .any(|r| r.road == *tuple && r.sent.is_some_and(|sent| sent > silence.judged))
            {
                silence.answered = Some(now);
                debug!(target: "khor_net_samples", ?tuple, ?now, ?silence, "road answered after judgment");
            }
        }
        let back = &mut self.back;
        self.silent.retain(|tuple, silence| {
            if silent_now.iter().any(|(t, _)| t == tuple) {
                return true;
            }
            if !open(tuple) {
                return false;
            }
            let held = silence
                .answered
                .is_none_or(|at| silence.rehold && now < at + SILENT_HOLD);
            if !held {
                debug!(target: "khor_net_samples", ?tuple, ?now, ?silence, immediate = !silence.rehold, "road recovery hold ended");
                back.insert(tuple.clone(), now);
            }
            held
        });
    }

    /// khor patch 17/29: a road's quality and its representative copy's sample time. The
    /// copy that took the most samples within [`CONFIRM_FOR`] speaks for the road, the freshest
    /// of those on a tie, and a copy whose path closed still does until that time is up.  A
    /// copy that carries nothing has a sample per keepalive, which moves its smoothed RTT an
    /// eighth of the way: when the freshest copy spoke (2026-10-17, p2 under load), one such
    /// sample had a road that slowed to 21 ms read 5 ms, the selector stopped wanting the move
    /// for a tick and the confirmation began again, three times; and when the copy that
    /// carried the road closed, the idle one left read 4 ms.
    fn road(&self, road: &transports::FourTuple) -> Option<(Duration, Instant)> {
        let speaks = self.representative(road)?;
        Some((speaks.quality, speaks.at))
    }

    fn representative(&self, road: &transports::FourTuple) -> Option<&Reading> {
        self.rtts
            .values()
            .filter(|r| r.road == *road)
            .max_by_key(|r| (r.recent.len(), r.at))
    }

    fn observation(&self, road: &transports::FourTuple) -> Option<PathMeasurement> {
        let speaks = self.representative(road)?;
        (speaks.samples > 0 && speaks.sent.is_some())
            .then(|| PathMeasurement::new(speaks.quality, speaks.at, speaks.recent.len()))
    }

    /// A numerical EWMA change is not a sample counter. Require two
    /// separate observation ticks of packets sent after the move began;
    /// a backlogged ACK must not confirm a post-stall round trip.
    fn samples_after(&self, road: &transports::FourTuple, after: Instant) -> usize {
        // khor patch 47: confirmation belongs to the same copy as road quality.
        // A different copy's ACKs cannot make the representative's old quality fresh.
        self.representative(road).map_or(0, |r| {
            r.changed.iter().filter(|(sent, _)| *sent > after).count()
        })
    }
}

/// Either a reference to live connection state, or a synthesized list of paths
/// (for unit-testing selectors).
#[derive(Debug)]
enum PathsSource<'a> {
    Live(&'a FxHashMap<ConnId, ConnectionState>),
    #[cfg(test)]
    Test(Vec<PathSelectionData<'a>>),
}

#[cfg_attr(not(feature = "unstable-custom-transports"), allow(unreachable_pub))]
impl<'a> PathSelectionContext<'a> {
    fn new(
        current: Option<&'a transports::FourTuple>,
        backup: Option<&'a transports::FourTuple>,
        held_for: Option<Duration>,
        connections: &'a FxHashMap<ConnId, ConnectionState>,
        measured: &Measured,
    ) -> Self {
        // khor patch 17: a path is as silent as its worst copy, and stays so until a copy
        // answers again ([`RemoteStateActor::read_paths`]).  The same road on another
        // connection that has nothing in flight reads as answering, and it hid a dead copy
        // (2026-10-17, p3: the connection stayed on a blackholed road for 15 s).  Its RTT is
        // the one the copy that took the most samples lately measured (`Measured::road`): a
        // copy that carries nothing keeps the RTT of its last keepalive, and hid a road
        // slowing under typing for 10 s (p2).
        let now = Instant::now();
        let mut silent: Vec<(transports::FourTuple, Duration)> = measured
            .silent
            .iter()
            .map(|(tuple, silence)| (tuple.clone(), silence.stall(now)))
            .collect();
        let mut fresh: Vec<(transports::FourTuple, Duration)> = Vec::new();
        for state in connections.values() {
            let Some(conn) = state.handle.upgrade() else {
                continue;
            };
            for (path_id, tuple) in state.paths.iter() {
                if let Some(waited) = conn
                    .path_stats(*path_id)
                    .and_then(|s| unanswered_for(&s, now.into_std()))
                {
                    match silent.iter_mut().find(|(t, _)| t == tuple) {
                        Some((_, stall)) => *stall = (*stall).max(waited),
                        None => silent.push((tuple.clone(), waited)),
                    }
                }
                if !fresh.iter().any(|(t, _)| t == tuple)
                    && let Some((rtt, _)) = measured.road(tuple)
                {
                    fresh.push((tuple.clone(), rtt));
                }
            }
        }
        Self {
            current,
            backup,
            silent,
            held_for,
            fresh,
            source: PathsSource::Live(connections),
        }
    }

    /// Constructs a context with synthetic path data for testing.
    #[cfg(test)]
    pub(crate) fn for_test(
        current: Option<&'a transports::FourTuple>,
        paths: Vec<PathSelectionData<'a>>,
    ) -> Self {
        Self {
            current,
            backup: None,
            silent: Vec::new(),
            held_for: None,
            fresh: Vec::new(),
            source: PathsSource::Test(paths),
        }
    }

    /// The path currently considered the preferred path to the remote endpoint, if any.
    pub fn current(&self) -> Option<&transports::FourTuple> {
        self.current
    }

    /// khor patch 18: the direct path currently kept open as the backup, if any
    /// ([`PathSelection::set_backup`]).
    pub fn backup(&self) -> Option<&transports::FourTuple> {
        self.backup
    }

    /// khor patch 17 (10-21): how long [`Self::current`] has been the selection, if it is one.
    pub fn held_for(&self) -> Option<Duration> {
        self.held_for
    }

    /// Iterator over candidate paths.
    ///
    /// The same address may appear more than once when it is a path on multiple
    /// connections to the remote.  Selectors that care should aggregate as appropriate.
    pub fn paths(&self) -> Box<dyn Iterator<Item = PathSelectionData<'a>> + '_> {
        let (silent, fresh) = (&self.silent, &self.fresh);
        match &self.source {
            PathsSource::Live(connections) => Box::new(
                connections
                    .values()
                    .filter_map(|state| state.handle.upgrade().map(|conn| (state, conn)))
                    .flat_map(move |(state, conn)| {
                        state.paths.iter().map(move |(path_id, addr)| {
                            let silent = silent.iter().find(|(t, _)| t == addr).map(|(_, s)| *s);
                            let rtt = fresh.iter().find(|(t, _)| t == addr).map(|(_, rtt)| *rtt);
                            PathSelectionData::live(addr, *path_id, conn.clone(), silent, rtt)
                        })
                    }),
            ),
            #[cfg(test)]
            PathsSource::Test(paths) => Box::new(paths.iter().cloned()),
        }
    }
}

/// Data the selector sees about one candidate path.
//
// In production this borrows from a live connection and looks up stats from noq on
// demand.  In `#[cfg(test)]` builds it can also wrap synthesized stats so selectors
// can be unit-tested without standing up real connections.
#[cfg_attr(not(feature = "unstable-custom-transports"), allow(unreachable_pub))]
#[derive(derive_more::Debug, Clone)]
pub struct PathSelectionData<'a> {
    network_path: &'a transports::FourTuple,
    /// khor patch 17: this path stopped answering on some connection, and how long it has kept
    /// the connection waiting.
    silent: Option<Duration>,
    /// khor patch 17: the road's RTT as the copy that took the most samples lately measured it,
    /// if a tick read it.
    fresh_rtt: Option<Duration>,
    #[debug(skip)]
    source: StatsSource,
}

#[derive(Clone)]
enum StatsSource {
    Live {
        path_id: PathId,
        conn: noq::Connection,
    },
    /// Boxed so `PathStats` (100+ bytes, 14 fields) doesn't inflate the enum's
    /// size in production where only the `Live` variant is ever constructed.
    #[cfg(test)]
    Test(Option<Box<PathStats>>),
}

#[cfg_attr(not(feature = "unstable-custom-transports"), allow(unreachable_pub))]
impl<'a> PathSelectionData<'a> {
    fn live(
        network_path: &'a transports::FourTuple,
        path_id: PathId,
        conn: noq::Connection,
        silent: Option<Duration>,
        fresh_rtt: Option<Duration>,
    ) -> Self {
        Self {
            network_path,
            silent,
            fresh_rtt,
            source: StatsSource::Live { path_id, conn },
        }
    }

    /// Constructs a [`PathSelectionData`] with synthetic stats for testing.
    ///
    /// `PathStats` is `#[non_exhaustive]` so callers build it via
    /// `let mut s = PathStats::default(); s.rtt = ...;`.
    #[cfg(test)]
    pub(crate) fn for_test(
        network_path: &'a transports::FourTuple,
        stats: Option<PathStats>,
    ) -> Self {
        Self {
            network_path,
            silent: None,
            fresh_rtt: None,
            source: StatsSource::Test(stats.map(Box::new)),
        }
    }

    /// The network path of the candidate path.
    pub fn network_path(&self) -> &transports::FourTuple {
        self.network_path
    }

    /// Returns path statistics if available.
    pub fn stats(&self) -> Option<PathStats> {
        match &self.source {
            StatsSource::Live { path_id, conn } => conn.path_stats(*path_id),
            #[cfg(test)]
            StatsSource::Test(stats) => stats.as_deref().copied(),
        }
    }

    /// khor patch 17: whether the path answers: on no connection two probe
    /// timeouts in a row with nothing heard for 300 ms.  A path that sent and
    /// heard nothing back keeps the round trip it last measured, so its RTT alone never says
    /// it is gone; noq abandons it only at its idle timeout.  Each path's ACKs come back on
    /// that path (vendor/noq-proto patch 4), so its probe timeouts are its own round trip's,
    /// not the path the peer sends on.
    pub fn answering(&self) -> bool {
        self.unanswered_for().is_none()
    }

    /// khor patch 17 (10-21): how long the path has kept the connection waiting, if it is not
    /// answering ([`Self::answering`]): since the oldest packet it left unacknowledged went out.
    /// A selector can hold a road that stalled for less than another road's round trip: a
    /// packet sent on the other now would arrive no sooner.
    pub fn unanswered_for(&self) -> Option<Duration> {
        let here = self
            .stats()
            .and_then(|s| unanswered_for(&s, std::time::Instant::now()));
        match (self.silent, here) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (a, b) => a.or(b),
        }
    }

    /// khor patch 17/29: the road's recent ACK-sample median from the copy taking the most
    /// samples lately (`Measured::road`), else this copy's transport estimate. `None` when
    /// its stats can't be read. Migration separately requires new evidence on both paths;
    /// repeatedly reading the same estimate does not make it fresh.
    pub fn rtt(&self) -> Option<Duration> {
        let stats = self.stats()?;
        Some(self.fresh_rtt.unwrap_or(stats.rtt))
    }
}

/// Trait to configure path selection.
///
/// Most users do not need to provide their own selector.
#[cfg_attr(not(feature = "unstable-custom-transports"), allow(unreachable_pub))]
pub trait PathSelector: Send + Sync + std::fmt::Debug + 'static {
    /// Pick the selected path to carry application data among the currently
    /// open network paths to the remote endpoint.
    ///
    /// Build the result by starting from [`PathSelection::none`] and calling
    /// [`PathSelection::set`] for the path the selector wants active.
    ///
    /// Returning an empty [`PathSelection`] keeps the current selection unchanged.
    fn select(&self, ctx: &PathSelectionContext<'_>) -> PathSelection;
}

/// The set of paths a [`PathSelector`] has chosen.
///
/// Today this holds at most one path.  Build via [`PathSelection::none`] +
/// [`PathSelection::set`].
#[derive(Debug, Clone)]
#[cfg_attr(not(feature = "unstable-custom-transports"), allow(unreachable_pub))]
pub struct PathSelection {
    selection: Option<transports::FourTuple>,
    /// khor patch 18.
    backup: Option<transports::FourTuple>,
}

#[cfg_attr(not(feature = "unstable-custom-transports"), allow(unreachable_pub))]
impl PathSelection {
    /// An empty selection.
    pub fn none() -> Self {
        Self {
            selection: None,
            backup: None,
        }
    }

    /// khor patch 18: keeps this direct path open beside the selected one, as a backup.
    ///
    /// Upstream closes every unselected direct path on the client (one direct path per
    /// connection), so when the selected path fails the connection falls back to the relay
    /// until a hole-punching round opens a direct one again.  A backup stays open, as a
    /// [`PathStatus::Backup`] path with a slower keepalive, and the selector can move to it
    /// on the next measurement tick.  Not naming one (the default) is upstream's behaviour.
    /// Only direct paths are kept this way; relay and custom paths are never closed anyway.
    pub fn set_backup(&mut self, path: &PathSelectionData<'_>) {
        if path.network_path.is_ip() {
            self.backup = Some(path.network_path.clone());
        }
    }

    /// khor patch 18: the backup named, if any.
    pub(crate) fn backup(&self) -> Option<&transports::FourTuple> {
        self.backup.as_ref()
    }

    /// Sets the path as the selected path.
    ///
    /// This discards any previously selected path and sets this one as a single selected
    /// path.
    pub fn set(&mut self, path: &PathSelectionData<'_>) {
        if self.selection.is_some() {
            tracing::warn!(
                path = %path.network_path(),
                "PathSelection already contains a path; ignoring additional path"
            );
            return;
        }
        self.selection = Some(path.network_path.clone());
    }

    /// The selected path: the one data should be sent on. This is not public so
    /// we can later allow for selecting multiple paths without changing the
    /// public API of `PathSelection`.
    ///
    /// Returns `None` when nothing has been selected.
    pub(crate) fn selected(&self) -> Option<&transports::FourTuple> {
        self.selection.as_ref()
    }
}

/// Future that resolves to the `conn_id` once a connection is closed.
///
/// This uses [`noq::Connection::on_closed`], which does not keep the connection alive
/// while awaiting the future.
struct OnClosed {
    conn_id: ConnId,
    inner: noq::OnClosed,
}

impl OnClosed {
    fn new(conn: &noq::Connection) -> Self {
        Self {
            conn_id: ConnId(conn.stable_id()),
            inner: conn.on_closed(),
        }
    }
}

impl Future for OnClosed {
    type Output = (ConnId, Closed);

    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let closed = std::task::ready!(Pin::new(&mut self.inner).poll(cx));
        Poll::Ready((self.conn_id, closed))
    }
}

/// Converts an iterator of [`TransportAddr'] into an iterator of [`transports::Addr`].
fn to_transports_addr(
    endpoint_id: EndpointId,
    addrs: impl IntoIterator<Item = TransportAddr>,
) -> impl Iterator<Item = transports::Addr> {
    addrs.into_iter().filter_map(move |addr| match addr {
        TransportAddr::Relay(relay_url) => Some(transports::Addr::from((relay_url, endpoint_id))),
        TransportAddr::Ip(sockaddr) => Some(transports::Addr::from(sockaddr)),
        TransportAddr::Custom(custom_addr) => Some(transports::Addr::from(custom_addr)),
        _ => {
            warn!(?addr, "Unsupported TransportAddr");
            None
        }
    })
}

/// Returns the next item if `maybe_stream` is `Some`, or `None` otherwise.
async fn maybe_next<S: Stream + Unpin>(maybe_stream: Option<&mut S>) -> Option<Option<S::Item>> {
    match maybe_stream {
        None => None,
        Some(s) => Some(s.next().await),
    }
}

#[cfg(test)]
#[path = "remote_state/confirmation_identity_tests.rs"]
mod tests_confirmation_identity;

#[cfg(test)]
#[path = "remote_state/recovery_baseline_tests.rs"]
mod tests_recovery_baseline;

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use iroh_base::SecretKey;
    use n0_watcher::Watchable;

    use super::*;
    use crate::socket::biased_rtt_path_selector::BiasedRttPathSelector;

    /// A [`State`] with no connections, which is all the pending-open-path queue needs.
    ///
    /// The returned guard keeps the direct-address watchable alive: the watcher in the state
    /// disconnects when it drops.
    fn test_state() -> (State, impl Sized) {
        let metrics = Arc::new(SocketMetrics::default());
        let watchable: Watchable<BTreeSet<DirectAddr>> = Watchable::new(BTreeSet::new());
        let state = State {
            endpoint_id: SecretKey::from_bytes(&[0u8; 32]).public(),
            metrics: metrics.clone(),
            local_direct_addrs: watchable.watch(),
            mapped_addrs: MappedAddrs::default(),
            address_lookup: AddressLookupServices::default(),
            connections_close: Default::default(),
            path_events: Default::default(),
            addr_events: Default::default(),
            paths: RemotePathState::new(metrics),
            last_holepunch: None,
            selected_path: None,
            backup_path: None,
            selected_at: None,
            watch: Watch::default(),
            moving: Moving::default(),
            measured: Default::default(),
            refresh: Default::default(),
            last_local_candidates: None,
            scheduled_holepunch: None,
            scheduled_open_path: None,
            pending_open_paths: VecDeque::new(),
            address_lookup_stream: None,
            path_selector: Arc::new(BiasedRttPathSelector::default()),
            rounds: tokio::sync::broadcast::channel(1).0,
            peer_observed: Default::default(),
        };
        (state, watchable)
    }

    fn addr(port: u16) -> transports::FourTuple {
        transports::FourTuple::Ip {
            remote: SocketAddr::from(([127, 0, 0, 1], port)),
            local: None,
        }
    }

    /// One retry tick against a remote that has no connection ids left: the actor drains the
    /// queue and tries every address it held on every connection, each of which fails.
    fn failing_retry_tick(state: &mut State, conns: usize) {
        let addrs = std::mem::take(&mut state.pending_open_paths);
        for open_addr in addrs {
            for _ in 0..conns {
                state.queue_pending_open_path(&open_addr);
            }
        }
    }

    /// With several connections to one remote, a failing address must not multiply.
    ///
    /// Every connection failing to open the same address used to push it back onto the queue,
    /// so each retry tick multiplied the queue by the number of connections and the backing
    /// `VecDeque` doubled until the allocator aborted the process.
    #[test]
    fn pending_open_paths_stays_bounded_across_retries() {
        let (mut state, _guard) = test_state();
        state.queue_pending_open_path(&addr(1));

        for tick in 0..8 {
            failing_retry_tick(&mut state, 4);
            assert!(
                state.pending_open_paths.len() <= MAX_PENDING_OPEN_PATHS,
                "queue grew to {} entries after {} ticks",
                state.pending_open_paths.len(),
                tick + 1,
            );
        }
        assert_eq!(state.pending_open_paths.len(), 1);
        assert!(state.scheduled_open_path.is_some());
    }

    /// The bound holds even when the addresses are all different.
    #[test]
    fn pending_open_paths_is_bounded_and_keeps_the_newest() {
        let (mut state, _guard) = test_state();
        for port in 0..(MAX_PENDING_OPEN_PATHS as u16 + 10) {
            state.queue_pending_open_path(&addr(port));
        }
        assert_eq!(state.pending_open_paths.len(), MAX_PENDING_OPEN_PATHS);
        assert_eq!(state.pending_open_paths.front(), Some(&addr(10)));
        assert_eq!(
            state.pending_open_paths.back(),
            Some(&addr(MAX_PENDING_OPEN_PATHS as u16 + 9))
        );
    }

    /// A queued address is not queued twice.
    #[test]
    fn pending_open_paths_holds_each_address_once() {
        let (mut state, _guard) = test_state();
        state.queue_pending_open_path(&addr(1));
        state.queue_pending_open_path(&addr(2));
        state.queue_pending_open_path(&addr(1));
        assert_eq!(
            Vec::from_iter(state.pending_open_paths.iter().cloned()),
            vec![addr(1), addr(2)]
        );
    }

    // khor patches 17–19 and 1 (10-21, net-flap): the measurement ticks' logic on synthetic
    // readings.

    const MS: fn(u64) -> Duration = Duration::from_millis;

    /// A path's stats: `rtt`, its PTO, `ptos` probe timeouts in a row, the last packet heard
    /// `heard` ago and the oldest unacknowledged one out `waiting` ago.
    fn stats(
        now: std::time::Instant,
        rtt: Duration,
        pto: Duration,
        ptos: u32,
        heard: Option<Duration>,
        waiting: Option<Duration>,
    ) -> PathStats {
        let mut s = PathStats::default();
        s.rtt = rtt;
        s.pto = pto;
        s.pto_count = ptos;
        s.last_received = heard.map(|h| now - h);
        s.unacked_since = waiting.map(|w| now - w);
        s
    }

    /// The red case of 10-21: an idle 2 ms road heard its last keepalive's ACK 4 s ago; the
    /// next keepalive went out 100 ms ago and its ACK is late (Wi-Fi power save, a stalled
    /// machine), two probe timeouts already.  It answers: the wait is 100 ms, not 4 s.
    #[test]
    fn an_idle_road_whose_ack_is_late_still_answers() {
        let now = std::time::Instant::now();
        let late = stats(
            now,
            MS(2),
            MS(30),
            2,
            Some(Duration::from_secs(4)),
            Some(MS(100)),
        );
        assert_eq!(
            unanswered_for(&late, now),
            None,
            "a 100 ms wait is no silence"
        );
        let gone = stats(
            now,
            MS(2),
            MS(30),
            2,
            Some(Duration::from_secs(4)),
            Some(MS(350)),
        );
        assert_eq!(
            unanswered_for(&gone, now),
            Some(MS(350)),
            "a 350 ms wait is"
        );
    }

    /// The wait is at least three probe timeouts: a 400 ms road's PTO is past 1 s.
    #[test]
    fn a_slow_road_waits_three_probe_timeouts() {
        let now = std::time::Instant::now();
        let s = |waiting| stats(now, MS(400), MS(450), 2, Some(MS(2000)), Some(waiting));
        assert_eq!(unanswered_for(&s(MS(1000)), now), None);
        assert_eq!(unanswered_for(&s(MS(1400)), now), Some(MS(1400)));
    }

    /// Two probe timeouts, something heard lately, or nothing in flight: it answers.
    #[test]
    fn a_road_answers_without_two_ptos_or_while_heard() {
        let now = std::time::Instant::now();
        let one_pto = stats(now, MS(2), MS(30), 1, Some(MS(900)), Some(MS(900)));
        assert_eq!(unanswered_for(&one_pto, now), None);
        let heard = stats(now, MS(2), MS(30), 3, Some(MS(50)), Some(MS(900)));
        assert_eq!(unanswered_for(&heard, now), None);
        let idle = stats(now, MS(2), MS(30), 3, Some(MS(900)), None);
        assert_eq!(unanswered_for(&idle, now), None);
        let never_heard = stats(now, MS(2), MS(30), 2, None, Some(MS(400)));
        assert_eq!(unanswered_for(&never_heard, now), Some(MS(400)));
    }

    #[test]
    fn an_initial_estimate_change_is_not_a_fresh_ack() {
        let t = Instant::now();
        let mut stats = PathStats::default();
        stats.rtt = MS(300);
        let mut reading = Reading::new(addr(1), &stats, t);
        stats.rtt = MS(1);
        assert!(!reading.observe(&stats, t + MS(250)));
        assert_eq!(
            reading.quality,
            MS(1),
            "an initial estimate may still guide exploration"
        );
        assert!(
            reading.changed.is_empty(),
            "PATH_RESPONSE is not new ACK evidence"
        );
    }

    #[test]
    fn a_delayed_stats_read_does_not_make_an_old_ack_new() {
        let t = Instant::now();
        let mut stats = PathStats::default();
        stats.rtt = MS(5);
        stats.latest_rtt = MS(5);
        stats.rtt_samples = 1;
        stats.latest_rtt_sent = Some(t.into_std());
        stats.latest_rtt_received = Some((t + MS(5)).into_std());
        let road = addr(1);
        let reading = Reading::new(road.clone(), &stats, t + MS(3000));
        assert_eq!(
            reading.at,
            t + MS(5),
            "age begins when the ACK was processed"
        );
        let mut measured = Measured::default();
        measured.rtts.insert((ConnId(0), PathId::ZERO), reading);
        assert_eq!(measured.samples_after(&road, t + MS(2000)), 0);
        assert_eq!(measured.observation(&road).unwrap().samples(), 0);
    }

    #[test]
    fn a_backlogged_ack_cannot_confirm_a_post_stall_round_trip() {
        let t = Instant::now();
        let after = t + MS(1000);
        let road = addr(1);
        let mut stats = PathStats::default();
        stats.rtt = MS(5);
        stats.latest_rtt = MS(5);
        stats.rtt_samples = 1;
        stats.latest_rtt_sent = Some(t.into_std());
        stats.latest_rtt_received = Some((after + MS(100)).into_std());
        let mut reading = Reading::new(road.clone(), &stats, after + MS(100));
        stats.rtt_samples = 2;
        stats.latest_rtt_sent = Some((t + MS(500)).into_std());
        stats.latest_rtt_received = Some((after + MS(350)).into_std());
        reading.observe(&stats, after + MS(350));
        let mut measured = Measured::default();
        measured.rtts.insert((ConnId(0), PathId::ZERO), reading);
        assert_eq!(
            measured.samples_after(&road, after),
            0,
            "both packets were sent before the stall"
        );
        for (count, sent) in [(3, 600), (4, 850)] {
            stats.rtt_samples = count;
            stats.latest_rtt_sent = Some((after + MS(sent)).into_std());
            stats.latest_rtt_received = Some((after + MS(sent + 5)).into_std());
            measured
                .rtts
                .get_mut(&(ConnId(0), PathId::ZERO))
                .unwrap()
                .observe(&stats, after + MS(sent + 50));
            assert_eq!(measured.samples_after(&road, after), (count - 2) as usize);
        }
    }

    #[test]
    fn equal_rtt_samples_are_fresh_and_recovery_does_not_use_the_old_average() {
        let now = Instant::now();
        let mut stats = PathStats::default();
        stats.rtt = MS(300);
        stats.latest_rtt = MS(300);
        stats.rtt_samples = 1;
        sample_time(&mut stats, now);
        let mut r = Reading::new(addr(1), &stats, now);
        let recovered = now + Duration::from_secs(3);
        stats.rtt = MS(260);
        stats.latest_rtt = MS(1);
        stats.rtt_samples = 2;
        sample_time(&mut stats, recovered);
        assert!(r.observe(&stats, recovered));
        assert_eq!(r.quality, MS(1));
        assert_eq!(r.rtt, MS(260), "transport smoothing remains separate");
        stats.rtt_samples = 3;
        sample_time(&mut stats, recovered + MS(250));
        assert!(
            r.observe(&stats, recovered + MS(250)),
            "equal values still give fresh evidence"
        );
        assert_eq!(r.at, recovered + MS(250));
        assert!(
            !r.observe(&stats, recovered + MS(500)),
            "a repeated read is not a new sample"
        );
        assert_eq!(r.quality, MS(1));
        assert!(r.recent.len() <= 8);
    }

    #[test]
    fn expiring_an_old_sample_updates_quality_without_creating_an_ack() {
        let t = Instant::now();
        let road = addr(1);
        let mut stats = PathStats::default();
        stats.rtt = MS(300);
        stats.latest_rtt = MS(300);
        stats.rtt_samples = 1;
        sample_time(&mut stats, t);
        let mut reading = Reading::new(road.clone(), &stats, t);
        stats.latest_rtt = MS(1);
        stats.rtt_samples = 2;
        sample_time(&mut stats, t + MS(1500));
        reading.observe(&stats, t + MS(1500));
        assert_eq!(
            reading.quality,
            MS(300),
            "the upper median still includes the slow sample"
        );
        reading.expire(t + MS(2250));
        let mut measured = Measured::default();
        measured.rtts.insert((ConnId(0), PathId::ZERO), reading);
        let observation = measured.observation(&road).unwrap();
        assert_eq!(observation.samples(), 1);
        assert_eq!(
            observation.rtt(),
            MS(1),
            "only the fast sample remains in this same window"
        );
        assert_eq!(measured.road(&road), Some((MS(1), t + MS(1500))));
        assert_eq!(
            measured.samples_after(&road, t + MS(2000)),
            0,
            "expiry is not a new ACK"
        );
    }

    #[test]
    fn active_refresh_is_bounded_rotates_and_does_not_sustain_itself() {
        let t = Instant::now();
        let mut refresh = ActiveRefresh::default();
        assert_eq!(refresh.take(2, t), None);
        refresh.observe(1, t);
        assert_eq!(refresh.take(2, t), Some(0));
        refresh.observe(2, t + MS(4000));
        assert_eq!(refresh.take(2, t + MS(4000)), None);
        assert_eq!(refresh.take(2, t + MS(5000)), Some(1));
        refresh.observe(2, t + MS(8000));
        assert_eq!(
            refresh.take(2, t + MS(10000)),
            None,
            "PINGs are not STREAM activity"
        );
        refresh.observe(3, t + MS(20000));
        assert_eq!(refresh.take(0, t + MS(20000)), None);
        assert_eq!(refresh.take(2, t + MS(20000)), Some(0));
    }

    #[test]
    fn shared_quality_and_age_come_from_the_same_representative_copy() {
        let now = Instant::now();
        let road = addr(1);
        let mut stats = PathStats::default();
        stats.rtt = MS(50);
        stats.latest_rtt = MS(5);
        stats.rtt_samples = 1;
        sample_time(&mut stats, now - MS(1000));
        let mut busy = Reading::new(road.clone(), &stats, now - MS(1000));
        stats.rtt_samples = 2;
        sample_time(&mut stats, now - MS(750));
        busy.observe(&stats, now - MS(750));
        stats.rtt_samples = 3;
        sample_time(&mut stats, now - MS(500));
        busy.observe(&stats, now - MS(500));
        stats.rtt = MS(20);
        stats.latest_rtt = MS(20);
        stats.rtt_samples = 1;
        sample_time(&mut stats, now);
        let quiet = Reading::new(road.clone(), &stats, now);
        let mut measured = Measured::default();
        measured.rtts.insert((ConnId(0), PathId::ZERO), busy);
        measured.rtts.insert((ConnId(1), PathId::ZERO), quiet);
        let observation = measured.observation(&road).unwrap();
        assert_eq!(observation.rtt(), MS(5));
        assert_eq!(observation.samples(), 3);
        assert!(observation.age() >= MS(500));
        assert_eq!(measured.road(&road), Some((MS(5), now - MS(500))));
        let other = addr(2);
        measured.rtts.insert(
            (ConnId(2), PathId::ZERO),
            Reading::new(other.clone(), &PathStats::default(), now),
        );
        assert!(
            measured.observation(&other).is_none(),
            "an initial estimate is not observed quality"
        );
    }

    fn sample_time(stats: &mut PathStats, received: Instant) {
        stats.latest_rtt_sent = Some((received - stats.latest_rtt).into_std());
        stats.latest_rtt_received = Some(received.into_std());
    }

    fn reading(road: &transports::FourTuple, at: Instant) -> Reading {
        Reading {
            road: road.clone(),
            rtt: MS(1),
            samples: 0,
            quality: MS(1),
            recent: VecDeque::new(),
            changed: VecDeque::from([(at, at)]),
            quality_since: None,
            sent: Some(at),
            at,
        }
    }

    fn recovery_case(t: Instant) -> (Measured, transports::FourTuple) {
        let road = addr(1);
        let mut m = Measured::default();
        let mut old = reading(&road, t - MS(500));
        old.samples = 1;
        m.rtts.insert((ConnId(0), PathId::ZERO), old);
        m.mark_silent(t, &[(road.clone(), t - MS(300))], |_| true);
        (m, road)
    }

    fn recovery_sample(m: &mut Measured, key: (ConnId, PathId), sample: Duration, at: Instant) {
        let road = m.rtts[&key].road.clone();
        let previous = m.road(&road).unwrap().0;
        let mut stats = PathStats::default();
        stats.rtt_samples = m.rtts[&key].samples + 1;
        stats.rtt = sample;
        stats.latest_rtt = sample;
        sample_time(&mut stats, at);
        if m.rtts.get_mut(&key).unwrap().observe(&stats, at) {
            m.recovery_quality_step(key, sample, previous, at);
        }
    }

    fn recovered_quality_case(t: Instant) -> (Measured, transports::FourTuple) {
        let (mut m, road) = recovery_case(t);
        let heard = [(road.clone(), (t + MS(1)).into_std())];
        assert_eq!(
            m.recovery_probes(t + MS(250), &[], &heard, |_| true).len(),
            1
        );
        let key = (ConnId(0), PathId::ZERO);
        recovery_sample(&mut m, key, MS(250), t + MS(600));
        recovery_sample(&mut m, key, MS(131), t + MS(900));
        recovery_sample(&mut m, key, MS(85), t + MS(1150));
        (m, road)
    }

    #[test]
    fn a_first_recovery_ack_keeps_the_bounded_quality_episode() {
        let t = Instant::now();
        let key = (ConnId(0), PathId::ZERO);
        let (mut measured, road) = recovery_case(t);
        let mut probes = Vec::new();
        // The first receive hint is already a fresh RTT. It can establish
        // reachability before the actor has created a recovery episode.
        for (rtt, after) in [(250, 600), (131, 900), (85, 1150)] {
            let now = t + MS(after);
            recovery_sample(&mut measured, key, MS(rtt), now);
            probes.extend(measured.complete_read(
                now,
                &[],
                &[(road.clone(), now.into_std())],
                |_| true,
            ));
        }
        assert_eq!(
            measured.road(&road).unwrap().0,
            MS(85),
            "a transient first ACK must not outlive the bounded recovery episode"
        );
        assert!(!measured.silent.contains_key(&road));
        let check = &measured.recovery[&road];
        assert_eq!(check.began, t + MS(600));
        assert_eq!(check.epoch, t - MS(300));
        assert_eq!(check.attempts, RECOVERY_PROBES);
        assert_eq!(probes.len(), usize::from(RECOVERY_PROBES));
        assert!(
            measured
                .complete_read(
                    t + MS(1400),
                    &[],
                    &[(road, (t + MS(1400)).into_std())],
                    |_| true,
                )
                .is_empty(),
            "the same recovery cannot refill its probe budget"
        );
    }

    #[test]
    fn a_known_recovery_can_start_a_new_quality_window_without_changing_healthy_medians() {
        let t = Instant::now();
        let key = (ConnId(0), PathId::ZERO);
        let (mut healthy, road) = recovery_case(t);
        healthy.silent.clear();
        for (rtt, at) in [(250, 600), (131, 900), (85, 1150)] {
            recovery_sample(&mut healthy, key, MS(rtt), t + MS(at));
        }
        assert_eq!(healthy.road(&road).unwrap().0, MS(131));
        let (m, road) = recovered_quality_case(t);
        assert_eq!(m.road(&road).unwrap().0, MS(85));
        assert_eq!(m.recovery[&road].quality_steps, 2);
        assert_eq!(
            m.recovery[&road].attempts, 1,
            "new quality windows do not mint probe budgets"
        );
        let reading = &m.rtts[&key];
        assert_eq!(reading.samples, 4);
        assert_eq!(
            reading.changed.len(),
            4,
            "one real observation per ACK update"
        );
        assert_eq!(reading.at, t + MS(1150));
        assert_eq!(reading.sent, Some(t + MS(1065)));
    }

    #[test]
    fn late_pre_epoch_packets_cannot_refresh_quality_age_or_confirm_a_new_epoch() {
        let t = Instant::now();
        let key = (ConnId(0), PathId::ZERO);
        let (mut m, road) = recovered_quality_case(t);
        let count = m.rtts[&key].samples;
        let changed = m.rtts[&key].changed.len();
        recovery_sample(&mut m, key, MS(500), t + MS(1400));
        let r = &m.rtts[&key];
        assert_eq!(
            r.samples,
            count + 1,
            "transport counter remains the real counter"
        );
        assert_eq!(r.rtt, MS(500), "transport SRTT is still read unchanged");
        assert_eq!(r.quality, MS(85));
        assert_eq!(r.at, t + MS(1150));
        assert_eq!(r.sent, Some(t + MS(1065)));
        assert_eq!(r.changed.len(), changed);
        assert_eq!(m.recovery[&road].quality_steps, 2);
    }

    #[test]
    fn another_copy_inherits_the_recovery_window_before_reading_an_old_ack() {
        let t = Instant::now();
        let (mut m, road) = recovered_quality_case(t);
        let epoch = m.rtts.values().filter_map(|r| r.quality_since).max();
        let mut stats = PathStats::default();
        stats.rtt_samples = 1;
        stats.latest_rtt = MS(300);
        sample_time(&mut stats, t + MS(1300));
        let copy = Reading::with_epoch(road.clone(), &stats, t + MS(1300), epoch);
        assert!(copy.sent.is_none());
        assert!(copy.recent.is_empty());
        let key = (ConnId(1), PathId::ZERO);
        m.rtts.insert(key, copy);
        assert_eq!(m.road(&road).unwrap().0, MS(85));
        recovery_sample(&mut m, key, MS(89), t + MS(1500));
        assert_eq!(m.road(&road).unwrap().0, MS(89));
        assert_eq!(m.rtts[&key].quality_since, epoch);
    }

    #[test]
    fn recovery_noise_cannot_repeatedly_restart_a_window_or_exceed_the_step_budget() {
        let t = Instant::now();
        let key = (ConnId(0), PathId::ZERO);
        let (mut m, road) = recovered_quality_case(t);
        recovery_sample(&mut m, key, MS(131), t + MS(1500));
        assert_eq!(
            m.road(&road).unwrap().0,
            MS(131),
            "a slow follow-up remains evidence"
        );
        recovery_sample(&mut m, key, MS(85), t + MS(1750));
        assert_eq!(m.recovery[&road].quality_steps, 2);
        assert!(m.rtts[&key].recent.iter().any(|(_, rtt)| *rtt == MS(131)));
        recovery_sample(&mut m, key, MS(50), t + MS(1900));
        assert_eq!(m.recovery[&road].quality_steps, 3);
        let epoch = m.rtts[&key].quality_since;
        recovery_sample(&mut m, key, MS(25), t + MS(2100));
        assert_eq!(m.recovery[&road].quality_steps, 3);
        assert_eq!(m.rtts[&key].quality_since, epoch);
    }

    #[test]
    fn an_early_measurement_tick_preserves_the_exact_pending_recovery_deadline() {
        let t = Instant::now();
        let (mut m, road) = recovery_case(t);
        let heard = [(road.clone(), (t + MS(1)).into_std())];
        assert!(m.recovery_deadline().is_none());
        assert_eq!(
            m.recovery_probes(t + MS(250), &[], &heard, |_| true).len(),
            1
        );
        assert_eq!(m.recovery_deadline(), Some(t + MS(500)));
        let early = t + MS(500) - Duration::from_nanos(1);
        assert!(m.recovery_probes(early, &[], &heard, |_| true).is_empty());
        assert_eq!(
            m.recovery_deadline(),
            Some(t + MS(500)),
            "do not wait a further interval"
        );
        assert_eq!(
            m.recovery_probes(t + MS(500), &[], &heard, |_| true).len(),
            1
        );
        assert_eq!(m.recovery_deadline(), Some(t + MS(750)));
        assert_eq!(
            m.recovery_probes(t + MS(750), &[], &heard, |_| true).len(),
            1
        );
        assert!(
            m.recovery_deadline().is_none(),
            "a spent episode has no timer"
        );
    }

    #[test]
    fn silent_closed_and_expired_recovery_episodes_disarm_their_due_wake() {
        let t = Instant::now();
        for condition in 0..3 {
            let (mut m, road) = recovery_case(t);
            let heard = [(road.clone(), (t + MS(1)).into_std())];
            assert_eq!(
                m.recovery_probes(t + MS(250), &[], &heard, |_| true).len(),
                1
            );
            let (now, silent_now) = if condition == 0 {
                (t + MS(500), vec![(road, t)])
            } else if condition == 1 {
                (t + MS(500), Vec::new())
            } else {
                (t + MS(250) + CONFIRM_FOR, Vec::new())
            };
            assert!(
                m.recovery_probes(now, &silent_now, &heard, |_| condition != 1)
                    .is_empty()
            );
            assert!(
                m.recovery_deadline().is_none(),
                "past-due wakes cannot spin while ineligible"
            );
        }
    }

    #[test]
    fn recovery_samples_stay_on_one_live_copy_unless_another_has_less_backoff() {
        let a = (ConnId(0), PathId::ZERO);
        let b = (ConnId(1), PathId::ZERO);
        assert!(recovery_copy_rank(Some(a), a, 0) < recovery_copy_rank(Some(a), b, 0));
        assert!(recovery_copy_rank(Some(a), b, 0) < recovery_copy_rank(Some(a), a, 1));
        assert_eq!(
            recovery_copy_rank(None, a, 0),
            recovery_copy_rank(None, b, 0)
        );
    }

    #[test]
    fn confirmation_deadlines_keep_the_window_and_fresh_sample_requirements() {
        let t = Instant::now();
        let road = addr(2);
        let mut moving = Moving::default();
        assert!(moving.deadline(t).is_none());
        assert!(matches!(
            moving.step(&road, t, MS(100), MS(60), |_| true, |_| true),
            Step::Hold { .. }
        ));
        let early = t + CONFIRM_FOR - Duration::from_nanos(1);
        assert_eq!(moving.deadline(early), Some(t + CONFIRM_FOR));
        assert!(matches!(
            moving.step(&road, early, MS(100), MS(60), |_| true, |_| true),
            Step::Hold { .. }
        ));
        assert!(matches!(
            moving.step(
                &road,
                t + CONFIRM_FOR,
                MS(100),
                MS(60),
                |_| false,
                |_| false
            ),
            Step::Hold { .. }
        ));
        assert_eq!(moving.deadline(t + CONFIRM_FOR), Some(t + CONFIRM_MAX));
        assert_eq!(
            moving.step(&road, t + CONFIRM_FOR, MS(100), MS(60), |_| true, |_| true),
            Step::Go
        );
        assert!(moving.deadline(t + CONFIRM_FOR).is_none());
        moving.to = Some((road, t));
        assert!(
            moving.deadline(t + CONFIRM_MAX).is_none(),
            "no immediate past-due wake loop"
        );
    }

    #[test]
    fn recovery_probes_need_a_known_road_and_a_hint_after_judgment() {
        let t = Instant::now();
        let (mut m, road) = recovery_case(t);
        assert!(m.recovery_probes(t, &[], &[], |_| true).is_empty());
        assert!(
            m.recovery_probes(t, &[], &[(road.clone(), t.into_std())], |_| true)
                .is_empty()
        );
        let heard = [(road.clone(), (t + MS(1)).into_std())];
        assert!(
            m.recovery_probes(t + MS(250), &[(road.clone(), t)], &heard, |_| true)
                .is_empty()
        );
        m.rtts.get_mut(&(ConnId(0), PathId::ZERO)).unwrap().samples = 0;
        assert!(
            m.recovery_probes(t + MS(250), &[], &heard, |_| true)
                .is_empty()
        );
        m.rtts.get_mut(&(ConnId(0), PathId::ZERO)).unwrap().samples = 1;
        let probes = m.recovery_probes(t + MS(250), &[], &heard, |_| true);
        assert_eq!(probes.len(), 1);
        assert_eq!(probes[0].road, road);
        assert_eq!(probes[0].epoch, t - MS(300));
        assert_eq!(probes[0].attempt, 1);
        assert!(
            m.silent.contains_key(&road),
            "the hint itself never establishes reachability"
        );
        let mut healthy = Measured::default();
        healthy.rtts = m.rtts;
        assert!(
            healthy
                .recovery_probes(t + MS(250), &[], &heard, |_| true)
                .is_empty()
        );
    }

    #[test]
    fn recovery_probe_requests_are_paced_and_cannot_refill_the_same_epoch() {
        let t = Instant::now();
        let (mut m, road) = recovery_case(t);
        let heard = [(road.clone(), (t + MS(1)).into_std())];
        for attempt in 1..=RECOVERY_PROBES {
            let now = t + MS(250 * u64::from(attempt));
            let probes = m.recovery_probes(now, &[], &heard, |_| true);
            assert_eq!(probes.len(), 1);
            assert_eq!(probes[0].attempt, attempt);
            // No send result is reported: failures and merging consume the same budget.
            assert!(
                m.recovery_probes(now + MS(1), &[], &heard, |_| true)
                    .is_empty()
            );
        }
        let later_hint = [(road.clone(), (t + MS(5000)).into_std())];
        assert!(
            m.recovery_probes(t + MS(5000), &[], &later_hint, |_| true)
                .is_empty()
        );
        assert_eq!(m.recovery[&road].attempts, RECOVERY_PROBES);
        assert!(
            m.silent.contains_key(&road),
            "unanswered probes cannot clear the fault"
        );
    }

    #[test]
    fn recovery_probing_has_a_finite_window_even_with_budget_remaining() {
        let t = Instant::now();
        let (mut m, road) = recovery_case(t);
        let heard = [(road.clone(), (t + MS(1)).into_std())];
        assert_eq!(
            m.recovery_probes(t + MS(250), &[], &heard, |_| true).len(),
            1
        );
        assert!(
            m.recovery_probes(t + MS(250) + CONFIRM_FOR, &[], &heard, |_| true)
                .is_empty()
        );
        assert!(
            m.recovery_probes(t + MS(4000), &[], &heard, |_| true)
                .is_empty()
        );
        assert_eq!(
            m.recovery[&road].attempts, 1,
            "an expired silence retains a spent tombstone"
        );
    }

    #[test]
    fn a_new_round_trip_can_end_silence_without_discarding_the_probe_budget() {
        let t = Instant::now();
        let (mut m, road) = recovery_case(t);
        let heard = [(road.clone(), (t + MS(1)).into_std())];
        assert_eq!(
            m.recovery_probes(t + MS(250), &[], &heard, |_| true).len(),
            1
        );
        let mut fresh = reading(&road, t + MS(300));
        fresh.samples = 2;
        m.rtts.insert((ConnId(0), PathId::ZERO), fresh);
        m.mark_silent(t + MS(500), &[], |_| true);
        assert!(!m.silent.contains_key(&road));
        assert_eq!(
            m.recovery_probes(t + MS(500), &[], &heard, |_| true).len(),
            1
        );
        assert_eq!(
            m.recovery_probes(t + MS(750), &[], &heard, |_| true).len(),
            1
        );
        assert!(
            m.recovery_probes(t + MS(1000), &[], &heard, |_| true)
                .is_empty()
        );
        assert!(
            m.recovery_probes(t + MS(2500), &[], &heard, |_| true)
                .is_empty()
        );
        assert!(
            m.recovery.is_empty(),
            "recovered episodes release their state after the window"
        );
    }

    #[test]
    fn natural_fresh_samples_cancel_remaining_recovery_probes() {
        let t = Instant::now();
        let (mut m, road) = recovery_case(t);
        let heard = [(road.clone(), (t + MS(1)).into_std())];
        assert_eq!(
            m.recovery_probes(t + MS(250), &[], &heard, |_| true).len(),
            1
        );
        let r = m.rtts.get_mut(&(ConnId(0), PathId::ZERO)).unwrap();
        for ms in [300, 350, 400] {
            r.changed.push_back((t + MS(ms), t + MS(ms)));
        }
        assert!(
            m.recovery_probes(t + MS(500), &[], &heard, |_| true)
                .is_empty()
        );
        assert_eq!(m.recovery[&road].attempts, RECOVERY_PROBES);
    }

    #[test]
    fn closed_roads_drop_recovery_state_and_an_independent_fault_gets_a_new_epoch() {
        let t = Instant::now();
        let (mut m, road) = recovery_case(t);
        let heard = [(road.clone(), (t + MS(1)).into_std())];
        assert_eq!(
            m.recovery_probes(t + MS(250), &[], &heard, |_| true).len(),
            1
        );
        assert!(
            m.recovery_probes(t + MS(500), &[], &heard, |_| false)
                .is_empty()
        );
        assert!(m.recovery.is_empty());
        m.mark_silent(t + MS(500), &[], |_| false);
        m.mark_silent(t + MS(1000), &[(road.clone(), t + MS(600))], |_| true);
        let heard = [(road.clone(), (t + MS(1001)).into_std())];
        let probes = m.recovery_probes(t + MS(1250), &[], &heard, |_| true);
        assert_eq!(probes.len(), 1);
        assert_eq!(probes[0].epoch, t + MS(600));
        assert_eq!(probes[0].attempt, 1);
    }

    #[test]
    fn an_old_packet_acked_after_a_stall_does_not_end_the_silence() {
        let road = addr(1);
        let t = Instant::now();
        let open = |_: &transports::FourTuple| true;
        let mut m = Measured::default();
        m.mark_silent(t, &[(road.clone(), t - MS(300))], open);
        let mut old = reading(&road, t + MS(250));
        old.sent = Some(t - MS(500));
        m.rtts.insert((ConnId(0), PathId::ZERO), old);
        m.mark_silent(t + MS(250), &[], open);
        assert!(
            m.silent.contains_key(&road),
            "the ACK's packet predates the last silence judgment"
        );
        m.rtts
            .insert((ConnId(0), PathId::ZERO), reading(&road, t + MS(500)));
        m.mark_silent(t + MS(500), &[], open);
        assert!(
            !m.silent.contains_key(&road),
            "a new round trip can end the silence"
        );
    }

    /// A silent road answers again only after a round trip and [`SILENT_HOLD`] of answering;
    /// going silent in between starts the hold again.
    #[test]
    fn a_repeatedly_silent_road_answers_again_after_the_hold() {
        let road = addr(1);
        let open = |_: &transports::FourTuple| true;
        let mut m = Measured::default();
        let t0 = Instant::now();
        m.back.insert(road.clone(), t0 - MS(100));
        m.mark_silent(t0, &[(road.clone(), t0 - MS(300))], open);
        assert_eq!(
            m.silent[&road].stall(t0 + MS(100)),
            MS(400),
            "the stall counts from the packet"
        );
        m.mark_silent(t0 + MS(250), &[], open);
        assert!(m.silent[&road].answered.is_none(), "no round trip yet");
        m.rtts
            .insert((ConnId(0), PathId::ZERO), reading(&road, t0 + MS(400)));
        m.mark_silent(t0 + MS(500), &[], open);
        assert_eq!(m.silent[&road].answered, Some(t0 + MS(500)));
        assert_eq!(
            m.silent[&road].stall(t0 + MS(900)),
            MS(800),
            "the stall ends when it answered"
        );
        m.mark_silent(t0 + MS(1200), &[], open);
        assert!(m.silent.contains_key(&road), "held for a second");
        m.mark_silent(t0 + MS(1300), &[(road.clone(), t0 + MS(1000))], open);
        assert!(
            m.silent[&road].answered.is_none(),
            "silent again: the hold starts over"
        );
        m.rtts
            .insert((ConnId(0), PathId::ZERO), reading(&road, t0 + MS(1400)));
        m.mark_silent(t0 + MS(1500), &[], open);
        m.mark_silent(t0 + MS(2400), &[], open);
        assert!(m.silent.contains_key(&road));
        m.mark_silent(t0 + MS(2500), &[], open);
        assert!(!m.silent.contains_key(&road), "a second of answering");
        m.mark_silent(t0 + MS(2600), &[(road.clone(), t0 + MS(2300))], open);
        m.mark_silent(t0 + MS(2700), &[], |_| false);
        assert!(m.silent.is_empty(), "a road no connection has is forgotten");
    }

    #[test]
    fn a_first_post_judgment_round_trip_restores_availability_without_a_quality_gate() {
        let road = addr(1);
        let t0 = Instant::now();
        let open = |_: &transports::FourTuple| true;
        let mut measured = Measured::default();
        measured.mark_silent(t0, &[(road.clone(), t0 - MS(300))], open);
        let mut recovered = reading(&road, t0 + MS(400));
        // The old outage may still dominate quality. A new round trip
        // restores reachability; Moving separately decides whether to use it.
        recovered.quality = MS(5000);
        measured.rtts.insert((ConnId(0), PathId::ZERO), recovered);
        measured.mark_silent(t0 + MS(500), &[], open);
        assert!(
            !measured.silent.contains_key(&road),
            "one first fault cannot impose a second quality-dependent recovery hold"
        );
    }

    /// A first recovery is independent of quality. Repeated faults within [`REHOLD`]
    /// wait out [`SILENT_HOLD`]; once that history expires, one new round trip suffices.
    #[test]
    fn a_one_off_fault_returns_at_its_first_round_trip_and_repeated_faults_hold() {
        let road = addr(1);
        let open = |_: &transports::FourTuple| true;
        let mut m = Measured::default();
        let t0 = Instant::now();
        m.mark_silent(t0, &[(road.clone(), t0 - MS(300))], open);
        m.mark_silent(t0 + MS(250), &[], open);
        assert!(
            m.silent.contains_key(&road),
            "no round trip yet: still silent"
        );
        m.rtts
            .insert((ConnId(0), PathId::ZERO), reading(&road, t0 + MS(400)));
        m.mark_silent(t0 + MS(500), &[], open);
        assert!(
            !m.silent.contains_key(&road),
            "back at its first round trip"
        );
        // Silent again five seconds on: off and on, so held.
        let t1 = t0 + MS(5500);
        m.mark_silent(t1, &[(road.clone(), t1 - MS(300))], open);
        m.rtts
            .insert((ConnId(0), PathId::ZERO), reading(&road, t1 + MS(400)));
        m.mark_silent(t1 + MS(500), &[], open);
        assert!(
            m.silent.contains_key(&road),
            "silent again within REHOLD: held"
        );
        m.mark_silent(t1 + MS(1400), &[], open);
        assert!(m.silent.contains_key(&road), "held for a second");
        m.mark_silent(t1 + MS(1500), &[], open);
        assert!(!m.silent.contains_key(&road), "a second of answering");
        // Long after it was last back, a stall is one stall again.
        let t2 = t1 + MS(1500) + REHOLD;
        m.mark_silent(t2, &[(road.clone(), t2 - MS(300))], open);
        m.rtts
            .insert((ConnId(0), PathId::ZERO), reading(&road, t2 + MS(400)));
        m.mark_silent(t2 + MS(500), &[], open);
        assert!(!m.silent.contains_key(&road), "quick again after REHOLD");
        // Quality does not gate availability for a different first-time fault either.
        let other = addr(2);
        m.mark_silent(t2 + MS(600), &[(other.clone(), t2 + MS(300))], open);
        m.rtts
            .insert((ConnId(1), PathId::ZERO), reading(&other, t2 + MS(700)));
        m.mark_silent(t2 + MS(800), &[], open);
        assert!(
            !m.silent.contains_key(&other),
            "a first-time response restores availability"
        );
    }

    /// carry (10-23): a copy in a probe timeout's backoff is woken when its road heard anything
    /// after the packet it waits on went out, on it or on another copy; not on a road that
    /// heard nothing since (a blackhole).
    #[test]
    fn a_backed_off_copy_is_woken_when_its_road_is_heard_again() {
        let (a, b) = (addr(1), addr(2));
        let t0 = std::time::Instant::now();
        let ms = |n: u64| t0 + MS(n);
        let backed_off = vec![
            // An idle copy of `a` whose keepalive went into the stall.
            (ConnId(0), PathId::ZERO, a.clone(), ms(100)),
            // The busy copy of `a`, its last probe out at 2.9 s.
            (ConnId(1), PathId::ZERO, a.clone(), ms(2900)),
            // `b`, still black.
            (ConnId(1), PathId::from(1u32), b.clone(), ms(100)),
        ];
        let heard = vec![(a.clone(), ms(3100)), (b.clone(), ms(50))];
        let woken: Vec<_> = to_wake(&backed_off, &heard).collect();
        assert_eq!(
            woken,
            vec![(ConnId(0), PathId::ZERO), (ConnId(1), PathId::ZERO)]
        );
        let before: Vec<_> = to_wake(&backed_off, &[(a.clone(), ms(2000))]).collect();
        assert_eq!(
            before,
            vec![(ConnId(0), PathId::ZERO)],
            "heard before the busy copy's probe: only the idle one"
        );
        assert_eq!(
            to_wake(&backed_off, &[]).count(),
            0,
            "nothing heard: nothing woken"
        );
    }

    /// A short stall of the selected road starts no round; a long one does, once; and the
    /// rounds this starts stay [`PUNCH_SPACING`] apart across selections.
    #[test]
    fn the_watch_punches_on_a_long_stall_once_per_spacing() {
        let t0 = Instant::now();
        let mut w = Watch::default();
        assert_eq!(
            w.tick(MS(2), MS(0), None, t0, |_| false),
            Watched::default()
        );
        assert!(
            !w.tick(MS(2), MS(0), Some(MS(400)), t0 + MS(250), |_| false)
                .punch,
            "a 400 ms stall: no round"
        );
        assert!(
            w.tick(MS(2), MS(0), Some(MS(1000)), t0 + MS(500), |_| false)
                .punch,
            "a second: a round"
        );
        assert!(
            !w.tick(MS(2), MS(0), Some(MS(1250)), t0 + MS(750), |_| false)
                .punch,
            "once per episode"
        );
        // The selection moves away and back, the road stalls again: within the spacing, no round.
        w.reset();
        assert!(!w.tick(MS(2), MS(0), None, t0 + MS(2000), |_| false).punch);
        assert!(
            !w.tick(MS(2), MS(0), Some(MS(1500)), t0 + MS(3000), |_| false)
                .punch,
            "within 30 s"
        );
        assert!(
            w.tick(
                MS(2),
                MS(0),
                Some(MS(1500)),
                t0 + PUNCH_SPACING + MS(500),
                |_| false
            )
            .punch,
            "the episode still degraded once the spacing is up"
        );
        assert!(
            !w.may_punch(t0 + PUNCH_SPACING + MS(600)),
            "and the spacing counts from it"
        );
    }

    #[test]
    fn a_single_slow_quality_sample_is_probed_before_holepunching() {
        let t0 = Instant::now();
        let mut watch = Watch::default();
        watch.tick(MS(1), MS(1), None, t0, |_| false);
        let spike = watch.tick(MS(150), MS(150), None, t0 + MS(250), |_| false);
        assert_eq!(
            spike,
            Watched {
                probe: true,
                punch: false
            }
        );
        let recovered = watch.tick(MS(2), MS(2), None, t0 + MS(500), |_| true);
        assert!(
            !recovered.punch,
            "the probe corrected the isolated slow reading"
        );
    }

    #[test]
    fn slow_quality_without_post_decision_samples_cannot_punch_after_a_pause() {
        let t0 = Instant::now();
        let mut watch = Watch::default();
        watch.tick(MS(1), MS(1), None, t0, |_| false);
        let began = t0 + MS(250);
        watch.tick(MS(150), MS(150), None, began, |_| false);
        let late = began + PROBE_FOR + MS(100);
        let old = watch.tick(MS(150), MS(0), None, late, |since| {
            assert_eq!(since, began);
            false
        });
        assert!(
            !old.punch,
            "elapsed time and a cached reading are not new evidence"
        );
        let fresh = watch.tick(MS(150), MS(150), None, late + MS(500), |since| {
            assert_eq!(since, began);
            true
        });
        assert!(
            fresh.punch,
            "fresh sustained slowing can still discover other roads"
        );
    }

    #[test]
    fn a_recovered_quality_restarts_confirmation_without_resetting_probe_spacing() {
        let t0 = Instant::now();
        let mut watch = Watch::default();
        watch.tick(MS(1), MS(1), None, t0, |_| false);
        watch.tick(MS(150), MS(150), None, t0 + MS(250), |_| false);
        watch.tick(MS(1), MS(1), None, t0 + MS(500), |_| false);
        let again = t0 + MS(750);
        watch.tick(MS(150), MS(150), None, again, |_| false);
        assert!(
            !watch
                .tick(MS(150), MS(150), None, t0 + MS(2250), |_| true)
                .punch
        );
        let sustained = watch.tick(MS(150), MS(150), None, again + PROBE_FOR, |since| {
            assert_eq!(since, again);
            true
        });
        assert!(sustained.punch);
        assert!(
            !sustained.probe,
            "foreground samples do not start another probe burst"
        );
    }

    #[test]
    fn quality_slower_than_the_probe_window_does_not_wait_for_new_round_trips() {
        let t0 = Instant::now();
        let mut watch = Watch::default();
        watch.tick(MS(1), MS(1), None, t0, |_| false);
        assert!(
            watch
                .tick(MS(2500), MS(2500), None, t0 + MS(250), |_| false)
                .punch
        );
        assert!(
            !watch
                .tick(MS(2500), MS(0), None, t0 + MS(500), |_| false)
                .punch
        );
    }

    /// A road slowing past its floor still starts a round, and one sample well over it has the
    /// road probed (patch 19).
    #[test]
    fn the_watch_punches_on_a_slowed_road_and_probes_a_slow_sample() {
        let t0 = Instant::now();
        let mut w = Watch::default();
        w.tick(MS(1), MS(0), None, t0, |_| false);
        let probed = w.tick(MS(3), MS(17), None, t0 + MS(250), |_| false);
        assert_eq!(
            probed,
            Watched {
                probe: true,
                punch: false
            }
        );
        assert!(!w.tick(MS(21), MS(21), None, t0 + MS(2500), |_| false).punch);
        assert!(
            w.tick(MS(21), MS(21), None, t0 + MS(4500), |since| {
                assert_eq!(since, t0 + MS(2500));
                true
            })
            .punch,
            "sustained quality with two post-decision observations"
        );
        assert!(!w.tick(MS(1), MS(0), None, t0 + MS(5000), |_| false).punch);
        assert!(!w.degraded, "back under the floor's half margin");
    }

    /// A move is made once both paths took a sample after it began and [`CONFIRM_FOR`] passed;
    /// until then the paths that took none in the last tick are pinged.
    #[test]
    fn a_move_goes_once_both_roads_sampled_for_two_seconds() {
        let t0 = Instant::now();
        let to = addr(2);
        let mut m = Moving::default();
        let yes = |_| true;
        let no = |_| false;
        assert_eq!(
            m.step(&to, t0, MS(8), MS(3), no, yes),
            Step::Hold {
                from: true,
                to: false
            }
        );
        assert_eq!(m.to(), Some(&to));
        assert_eq!(
            m.step(&to, t0 + MS(1000), MS(8), MS(3), yes, yes),
            Step::Hold {
                from: false,
                to: false
            }
        );
        assert_eq!(
            m.step(&to, t0 + CONFIRM_FOR, MS(8), MS(3), yes, yes),
            Step::Go
        );
        assert_eq!(m.to(), None);
    }

    /// A move whose roads never both answer is given up after [`CONFIRM_MAX`], and the same
    /// move is not tried for [`CONFIRM_REST`]; another one is.
    #[test]
    fn a_move_never_confirmed_is_given_up_and_rests() {
        let t0 = Instant::now();
        let (to, other) = (addr(2), addr(3));
        let mut m = Moving::default();
        let yes = |_| true;
        let no = |_| false;
        let mut pinged = 0;
        let mut t = t0;
        loop {
            match m.step(&to, t, MS(8), MS(3), yes, no) {
                Step::Hold { to: true, .. } => pinged += 1,
                Step::Rest => break,
                step => panic!("{step:?} at {:?}", t - t0),
            }
            t += RESELECT_INTERVAL;
        }
        assert_eq!(t - t0, CONFIRM_MAX, "given up after four seconds");
        assert_eq!(pinged, 16, "a ping a tick until then");
        assert_eq!(
            m.step(&to, t + MS(10_000), MS(8), MS(3), yes, yes),
            Step::Rest,
            "resting"
        );
        assert!(
            matches!(
                m.step(&other, t + MS(10_000), MS(8), MS(3), yes, no),
                Step::Hold { .. }
            ),
            "another move is not"
        );
        assert!(
            matches!(
                m.step(&to, t + CONFIRM_REST, MS(8), MS(3), yes, yes),
                Step::Hold { .. }
            ),
            "tried again after the rest"
        );
    }

    /// 10-21: a road that stopped answering is left for one that answered since, at once if it
    /// did; one that has not is pinged until it does, or for [`CONFIRM_MAX`].
    #[test]
    fn a_silent_road_uses_a_new_backup_round_trip_before_leaving() {
        let t0 = Instant::now();
        let began = t0 - MS(300);
        let to = addr(2);
        let mut m = Moving::default();
        assert_eq!(
            m.leave(&to, t0, began, true, |after| after < t0),
            Step::Hold {
                from: true,
                to: true
            },
            "the spare's old answer predates the decision"
        );
        assert_eq!(
            m.leave(&to, t0, began, true, |_| false),
            Step::Hold {
                from: true,
                to: true
            },
            "probe both roads while waiting"
        );
        assert_eq!(
            m.leave(&to, t0 + MS(250), began, true, |after| after < t0 + MS(100)),
            Step::Go,
            "now it did"
        );
        let mut m = Moving::default();
        assert!(matches!(
            m.leave(&to, t0, began, true, |_| false),
            Step::Hold { .. }
        ));
        assert_eq!(
            m.leave(&to, t0 + CONFIRM_MAX, began, true, |_| false),
            Step::Go,
            "nothing answers: go all the same"
        );
    }

    #[test]
    fn a_far_faster_move_cannot_be_confirmed_by_packets_sent_before_it_began() {
        let t0 = Instant::now();
        let to = addr(2);
        let mut moving = Moving::default();
        let sent = [t0 - MS(200), t0 - MS(100)];
        let sampled = |after: Instant| sent.iter().filter(|sent| **sent > after).count() >= 2;
        let step = moving.step(&to, t0, MS(300), MS(2), |_| false, sampled);
        assert_ne!(
            step,
            Step::Go,
            "both candidate packets predate the decision"
        );
    }

    #[test]
    fn a_fault_move_probes_the_current_road_and_requires_a_post_decision_backup_sample() {
        let now = Instant::now();
        let to = addr(2);
        let mut moving = Moving::default();
        let old_backup_packet = now - MS(100);
        let step = moving.leave(&to, now, now - MS(500), true, |after| {
            old_backup_packet > after
        });
        assert_eq!(
            step,
            Step::Hold {
                from: true,
                to: true
            },
            "probe the suspected current copy as well; a pre-decision backup ACK cannot end confirmation"
        );
        let mut older_move = Moving::default();
        let _ = older_move.step(&to, now - MS(1000), MS(30), MS(20), |_| false, |_| false);
        let before_fault = now - MS(600);
        assert_eq!(
            older_move.leave(&to, now, now - MS(500), true, |after| before_fault > after),
            Step::Hold {
                from: true,
                to: true
            },
            "an older move cannot reuse a pre-fault sample"
        );
    }

    #[test]
    fn a_fault_with_no_receiving_copy_can_use_a_backup_that_answered_after_the_fault() {
        let now = Instant::now();
        let to = addr(2);
        let mut moving = Moving::default();
        let backup_packet = now - MS(100);
        assert_eq!(
            moving.leave(&to, now, now - MS(500), false, |after| backup_packet
                > after),
            Step::Go,
            "without any current-copy receipt, retain fast failover to a post-fault backup"
        );
        let mut stale = Moving::default();
        let before_fault = now - MS(600);
        assert_eq!(
            stale.leave(&to, now, now - MS(500), false, |after| before_fault > after),
            Step::Hold {
                from: false,
                to: true
            },
            "a pre-fault sample still cannot prove the backup"
        );
        assert_eq!(
            stale.leave(&to, now + CONFIRM_MAX, now - MS(500), false, |_| false),
            Step::Go,
            "the original upper bound still applies when no road answers"
        );
    }

    #[test]
    fn a_far_faster_move_probes_the_current_road_after_a_slow_sample() {
        let t0 = Instant::now();
        let to = addr(2);
        let mut moving = Moving::default();
        let step = moving.step(&to, t0, MS(300), MS(2), |_| false, |_| false);
        assert_eq!(
            step,
            Step::Hold {
                from: true,
                to: true
            }
        );
    }

    /// flap (10-23): a move to a road far faster than the one in use (the carried road through
    /// turing at 20 ms, off a relay that read 1743 ms) is made once the road moved to took a
    /// sample since the move began: no [`CONFIRM_FOR`], and no sample required of the slow one.
    /// A move between roads of about one speed is confirmed as before.
    #[test]
    fn a_far_faster_road_is_taken_without_the_two_second_confirm() {
        let t0 = Instant::now();
        let to = addr(2);
        let mut m = Moving::default();
        let no = |_| false;
        assert!(
            matches!(
                m.step(&to, t0, MS(1743), MS(20), no, no),
                Step::Hold { to: true, .. }
            ),
            "ping the fast one"
        );
        let tick = t0 + RESELECT_INTERVAL;
        assert_eq!(
            m.step(&to, tick, MS(1743), MS(20), no, |after| after < tick),
            Step::Go,
            "it answered: go"
        );
        let mut m = Moving::default();
        let yes = |_| true;
        assert!(matches!(
            m.step(&to, t0, MS(30), MS(20), yes, yes),
            Step::Hold { .. }
        ));
        assert!(
            matches!(
                m.step(&to, tick, MS(30), MS(20), yes, yes),
                Step::Hold { .. }
            ),
            "one speed: the two seconds"
        );
        assert!(
            matches!(
                m.step(&to, tick, MS(300), MS(150), yes, yes),
                Step::Hold { .. }
            ),
            "twice and 10 ms is not"
        );
        assert_eq!(
            m.step(&to, tick, MS(309), MS(116), yes, yes),
            Step::Go,
            "the relay at 309 ms, the hop at 116 ms"
        );
        // feat/net: packets from before the decision cannot confirm the fast branch.
        let mut m = Moving::default();
        let last_tick = |after: Instant| after < t0 - MS(100);
        assert_eq!(
            m.step(&to, t0, MS(856), MS(174), no, last_tick),
            Step::Hold {
                from: true,
                to: true
            }
        );
        let answered = t0 + MS(500);
        assert_eq!(
            m.step(&to, answered, MS(856), MS(174), no, |after| after
                < answered),
            Step::Go
        );
        let mut m = Moving::default();
        let older = |after: Instant| after < t0 - MS(400);
        assert!(
            matches!(
                m.step(&to, t0, MS(856), MS(174), no, older),
                Step::Hold { to: true, .. }
            ),
            "an older one is not"
        );
    }

    /// flap (10-23): a move off a road whose round trip is longer than [`CONFIRM_FOR`] (a relay
    /// that read 4994 ms) is not given up for that road taking no sample: it cannot within the
    /// confirmation.  It was, and the connection stayed on the relay [`CONFIRM_REST`] more.
    #[test]
    fn a_move_off_a_road_slower_than_the_confirm_is_not_given_up() {
        let t0 = Instant::now();
        let to = addr(2);
        let no = |_| false;
        let sampled_at = |t: Instant| move |after: Instant| after < t;
        let mut m = Moving::default();
        let _ = m.step(&to, t0, MS(4994), MS(54), no, no);
        let late = t0 + CONFIRM_MAX;
        assert_ne!(
            m.step(&to, late, MS(4994), MS(54), no, sampled_at(late)),
            Step::Rest,
            "far faster"
        );
        // Not far faster, both slow: the slow one's sample is not asked for.
        let mut m = Moving::default();
        let _ = m.step(&to, t0, MS(2500), MS(1400), no, no);
        let first = t0 + MS(500);
        assert!(matches!(
            m.step(&to, first, MS(2500), MS(1400), no, sampled_at(first)),
            Step::Hold { .. }
        ));
        let done = t0 + CONFIRM_FOR;
        assert_eq!(
            m.step(&to, done, MS(2500), MS(1400), no, sampled_at(first)),
            Step::Go,
            "after the two seconds"
        );
        // And the slow one is not pinged for it.
        let mut m = Moving::default();
        assert_eq!(
            m.step(&to, t0, MS(2500), MS(1400), no, no),
            Step::Hold {
                from: false,
                to: true
            }
        );
    }

    /// One direct road under 10 ms on any connection is good enough.
    #[test]
    fn one_good_direct_road_is_good_enough() {
        assert!(
            good_enough([MS(95), MS(2)]),
            "the idle copy's stale 95 ms does not count"
        );
        assert!(!good_enough([MS(95), MS(30)]));
        assert!(!good_enough([]), "no direct road");
    }

    /// The ADD_ADDRESS frames of one update are one round: the soonest asked for.
    #[test]
    fn rounds_asked_for_together_are_one() {
        let (mut state, _guard) = test_state();
        let t0 = Instant::now();
        state.punch_soon(t0 + MS(100));
        state.punch_soon(t0 + MS(120));
        assert_eq!(state.scheduled_holepunch, Some(t0 + MS(100)));
        state.scheduled_holepunch = Some(t0 + Duration::from_secs(5));
        state.punch_soon(t0 + MS(100));
        assert_eq!(
            state.scheduled_holepunch,
            Some(t0 + MS(100)),
            "sooner than the spacing's"
        );
    }

    /// patch 1 (10-21): a connection out of path ids keeps what waits, and tries it again when
    /// one of its paths is abandoned.
    #[test]
    fn path_ids_run_out_until_a_path_is_abandoned() {
        let mut ids = PathIds::default();
        assert!(!ids.exhausted());
        ids.wait(&addr(1));
        ids.wait(&addr(2));
        ids.wait(&addr(1));
        assert!(ids.exhausted());
        assert_eq!(ids.freed(), vec![addr(1), addr(2)], "each address once");
        assert!(!ids.exhausted(), "the limit rises with the abandoned path");
        assert!(ids.freed().is_empty());
        for port in 0..(MAX_PENDING_OPEN_PATHS as u16 + 5) {
            ids.wait(&addr(port));
        }
        assert_eq!(ids.freed().len(), MAX_PENDING_OPEN_PATHS, "bounded");
    }
}

#[cfg(all(test, not(wasm_browser)))]
mod tests_upstream_datagram_send {
    use super::*;

    #[tokio::test]
    async fn blocked_relay_does_not_delay_direct_initial() {
        let receiver = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let (mut sender, _relay_receiver) = TransportsSender::with_bounded_relay(
            1,
            [transports::IpConfig::V4 {
                ip_net: "127.0.0.1/8".parse().unwrap(),
                port: 0,
                is_required: true,
                is_default: true,
            }]
            .into_iter(),
        );
        let relay = transports::FourTuple::Relay {
            url: "https://relay.example.invalid".parse().unwrap(),
            endpoint_id: iroh_base::SecretKey::from_bytes(&[1; 32]).public(),
        };
        let transmit = OwnedTransmit {
            ecn: None,
            contents: bytes::Bytes::from_static(b"initial"),
            segment_size: None,
        };
        send_datagram(&mut sender, relay.clone(), transmit.clone())
            .await
            .unwrap();
        let sends = send_datagram_to_targets(
            Box::new(sender),
            transmit,
            smallvec![
                relay,
                transports::FourTuple::from_remote(transports::Addr::Ip(
                    receiver.local_addr().unwrap(),
                )),
            ],
        );
        let mut buf = [0; 64];
        // The Relay queue stays full while the later Direct target receives its Initial.
        tokio::select! {
            _ = sends => panic!("blocked Relay send unexpectedly completed"),
            received = time::timeout(Duration::from_secs(1), receiver.recv(&mut buf)) => {
                let len = received.expect("Direct send blocked behind Relay").unwrap();
                assert_eq!(&buf[..len], b"initial");
            }
        }
    }
}
