//! **The peer hop** (round ROAD-3, audit/ROAD-BRIEF.md T6): two machines
//! with no direct road meet through a third khor machine that reaches
//! both. The middle splices two streams and sees only ciphertext: the two
//! ends run iroh's own handshake through it, as themselves, over the
//! carried transport (`khor_net::carry`, a link of `Carrier::Hop`).
//!
//! # The asker (X), the middle (B), the far end (Y)
//!
//! - **X**, about to dial Y with no link to it and no recent direct road
//!   ([`Node::hop_ahead`]), picks a middle from the roadbook ([`middles_with_cost`])
//!   and opens a stream on its tunnel wire to B asking `hop <Y>`.
//! - **B** checks the gate (the switch, both ends on its roster and not
//!   paused, its own road to Y not itself a hop), opens a stream on its
//!   tunnel wire to Y saying `hop-in <X>`, answers X, and copies bytes
//!   both ways until either end hangs up.
//! - **Y** checks the same gate and makes the stream its link to X.
//!
//! X then dials Y with the carried address beside the others. The relay
//! may answer first; the carried path is opened anyway (vendor/iroh
//! KHOR.md, patch 6) and `RoadOrder` takes it in the opening race and
//! keeps it. Direct may take over at any time (iroh's hole punch); once
//! it has held, X lets the hop go so the middle carries nothing.
//!
//! **Why the tunnel ALPN.** The hop is a long-lived duplex stream with a
//! one-byte answer before any payload: exactly the exit's shape
//! (`crate::tunnel`), which already has its wires, its gate per stream
//! and its status byte. The two asks are words a `host:port` can never
//! be (they have a space), so an exit never mistakes one for a target;
//! an older khor tries to connect to it as a host, fails, and answers
//! `NO_ROUTE`.
//!
//! **One hop only.** A middle whose own road to Y is a hop is never
//! picked and refuses if asked; X never picks a middle it reaches through
//! a carried road. The second of the owner's two hops is the LAN
//! entrance (`khor pipe --via`), not another of these.

use std::sync::Arc;
use std::time::Duration;

use iroh::EndpointId;
use iroh::endpoint::{RecvStream, SendStream};
use khor_core::{Road, RoadKind};
use khor_net::carry::Carrier;

use crate::Node;

#[path = "hop/stream_trace.rs"]
mod stream_trace;
use crate::tunnel::{NO_ROUTE, OK, REFUSED};
use crate::wires::{CarrierComparison, Proto};

/// X to B: splice me to this machine.
const ASK: &str = "hop ";
/// B to Y: this stream is a hop from this machine.
const HAND: &str = "hop-in ";
/// Explicitly understood at both forwarding endpoints before parallel commit.
const PREPARE: &str = "hop-ready ";
const PREPARE_HAND: &str = "hop-ready-in ";

/// What X gives the hop before dialling without it: a middle's wire
/// that has to be dialled first, plus its own stream to Y.
pub(crate) const HOP_WITHIN: Duration = Duration::from_secs(3);

/// How long B gives its own wire to Y: inside X's [`HOP_WITHIN`], so B
/// stops dialling for X before X stops waiting (net-dial B3: it was 5 s,
/// and every hop to a machine that was off cost B two seconds of a dial
/// nobody was waiting for any more).
const MIDDLE_WITHIN: Duration = HOP_WITHIN.saturating_sub(Duration::from_millis(500));

/// Middles X tries, best first, before it dials without a hop.
const TRIES: usize = 2;

/// How often X looks whether direct has taken over from its hop.
const LOOK_EVERY: Duration = Duration::from_secs(5);

/// How often the resident holds each relay wire against the hops
/// ([`Node::compare_roads`]).
const COMPARE_EVERY: std::time::Duration = Duration::from_secs(30);

/// How long a middle whose hop lost to the relay is not tried again for
/// that machine: the legs' written round trips said it would win and the
/// measurement end to end said not, so asking again at once would flap.
const LOST_FOR: Duration = Duration::from_secs(10 * 60);

/// How long a hop to a machine that no middle could reach is not asked
/// for again, after the first such miss: doubling with each miss in a
/// row up to [`LOST_FOR`], and forgotten when a hop to it opens.
///
/// net-dial B3: X asked for a hop before **every** dial to a machine that
/// was down (`wants_hop`), up to [`TRIES`] middles and [`HOP_WITHIN`] each
/// time, and each middle dialled the machine again for it — four machines
/// spent most of a day's failed dials that way on one machine that was
/// asleep.
const MISSED_FIRST: Duration = Duration::from_secs(30);

/// Per (this machine, far machine): hops that found no middle in a row,
/// and when the last did.
#[derive(Clone)]
struct MissEpoch {
    network: std::sync::Weak<()>,
    candidates: std::sync::Weak<()>,
}

struct MissObservation {
    network: std::sync::Weak<()>,
    order: u64,
}

impl MissObservation {
    fn new(network: std::sync::Weak<()>) -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self {
            network,
            order: NEXT.fetch_add(1, Ordering::Relaxed),
        }
    }
}

struct HopWire {
    id: usize,
    addrs: Vec<String>,
    weak: iroh::endpoint::WeakConnectionHandle,
}

impl HopWire {
    fn observed(conn: &iroh::endpoint::Connection) -> Option<Self> {
        let mut addrs: Vec<_> = conn
            .paths()
            .iter()
            .filter(|path| path.is_ip() || path.is_relay())
            .map(|path| format!("{:?}", path.remote_addr()))
            .collect();
        addrs.sort_unstable();
        addrs.dedup();
        (!addrs.is_empty() && conn.close_reason().is_none()).then(|| Self {
            id: conn.stable_id(),
            addrs,
            weak: conn.weak_handle(),
        })
    }

    fn matches(&self, other: &Self) -> bool {
        self.id == other.id
            && self.addrs == other.addrs
            && self
                .weak
                .upgrade()
                .is_some_and(|conn| conn.close_reason().is_none())
    }
}

/// A demand with no target road may recheck a live middle. Background
/// discovery and periodic quality comparisons keep their failure backoff.
#[derive(Clone, Copy, PartialEq, Eq)]
enum HopUse {
    Dial,
    Recovery,
    Comparison,
}

struct HopCandidate {
    id: String,
    live: bool,
    wires: Vec<HopWire>,
    hints: Vec<String>,
    missed: Option<(u32, std::time::Instant)>,
    attempted_at: Option<std::time::Instant>,
}

impl HopCandidate {
    fn cooling(&self) -> bool {
        self.missed
            .is_some_and(|(count, at)| at.elapsed() < missed_wait(count))
    }

    fn wait_for(&self, purpose: HopUse) -> Duration {
        let backoff = self.missed.map_or(Duration::ZERO, |(count, at)| {
            missed_wait(count).saturating_sub(at.elapsed())
        });
        if purpose == HopUse::Recovery
            && !backoff.is_zero()
            && self.live
            && self.wires.iter().any(|wire| {
                wire.weak
                    .upgrade()
                    .is_some_and(|conn| conn.close_reason().is_none())
            })
        {
            // Reuse the existing hop budget as a minimum interval between
            // actual asks. Concurrent/UI requests cannot turn one reachable
            // first leg into unbounded probes of an unavailable second leg.
            return self
                .attempted_at
                .map_or(Duration::ZERO, |at| HOP_WITHIN.saturating_sub(at.elapsed()));
        }
        if purpose == HopUse::Comparison {
            return backoff.max(self.attempted_at.map_or(Duration::ZERO, |at| {
                COMPARE_EVERY.saturating_sub(at.elapsed())
            }));
        }
        backoff
    }
}

fn same_candidates(a: &[HopCandidate], b: &[HopCandidate]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(a, b)| {
            a.id == b.id
                && a.live == b.live
                && a.hints == b.hints
                && a.wires.len() == b.wires.len()
                && a.wires.iter().zip(&b.wires).all(|(a, b)| a.matches(b))
        })
}

struct HopMiss {
    count: u32,
    at: std::time::Instant,
    epoch: Option<MissEpoch>,
    candidates: Vec<HopCandidate>,
    target_hints: Vec<String>,
    observed: u64,
    // Only this cache entry owns the candidate token. In-flight asks hold
    // weak tokens, so retiring a context also retires its late answers.
    _current: Option<Arc<()>>,
}

impl HopMiss {
    fn new(epoch: Option<MissEpoch>) -> Self {
        Self {
            count: 0,
            at: std::time::Instant::now(),
            epoch,
            candidates: Vec::new(),
            target_hints: Vec::new(),
            observed: 0,
            _current: None,
        }
    }
}

static MISSED: std::sync::LazyLock<
    std::sync::Mutex<std::collections::HashMap<(String, String), HopMiss>>,
> = std::sync::LazyLock::new(Default::default);

/// How long after `n` misses in a row a hop is not asked for again.
fn missed_wait(n: u32) -> Duration {
    MISSED_FIRST
        .saturating_mul(1u32 << n.saturating_sub(1).min(8))
        .min(LOST_FOR)
}

fn same_miss_epoch(a: Option<&MissEpoch>, b: Option<&MissEpoch>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => a.network.ptr_eq(&b.network) && a.candidates.ptr_eq(&b.candidates),
        (None, None) => true,
        _ => false,
    }
}

fn current_miss_epoch(epoch: Option<&MissEpoch>) -> bool {
    epoch.is_none_or(|epoch| {
        epoch.network.strong_count() != 0 && epoch.candidates.strong_count() != 0
    })
}

/// Candidate membership and actual first-leg availability define a miss's
/// context. Cost and ordering deliberately do not: noisy RTT is no new route.
fn miss_context(
    me: &str,
    to: &str,
    observation: MissObservation,
    facts: impl FnOnce() -> (Vec<String>, Vec<HopCandidate>),
) -> Option<MissEpoch> {
    let mut all = MISSED.lock().unwrap_or_else(|p| p.into_inner());
    if observation.network.strong_count() == 0 {
        return None;
    }
    all.retain(|_, miss| current_miss_epoch(miss.epoch.as_ref()));
    let miss = all
        .entry((me.to_owned(), to.to_owned()))
        .or_insert_with(|| HopMiss::new(None));
    // A delayed selection is not evidence that a newer route disappeared.
    // The sequence is captured before reading the roster and roadbook.
    if observation.order < miss.observed {
        return None;
    }
    // Snapshot live wires under the same routing lock as registration.
    // A CPU-delayed registry snapshot cannot replace fresher evidence.
    // Wires never takes MISSED while holding its registries; no await here.
    let (target_hints, mut candidates) = facts();
    candidates.sort_unstable_by(|a, b| a.id.cmp(&b.id));
    candidates.dedup_by(|a, b| a.id == b.id);
    miss.observed = observation.order;
    if miss.target_hints == target_hints
        && same_candidates(&miss.candidates, &candidates)
        && let Some(epoch) = &miss.epoch
        && epoch.network.ptr_eq(&observation.network)
    {
        return Some(epoch.clone());
    }
    // A new candidate may get admission without retrying unchanged
    // middles that already refused. Only the same live wire in the same
    // network can carry its own failure forward into the new context.
    if miss.target_hints == target_hints
        && miss
            .epoch
            .as_ref()
            .is_some_and(|epoch| epoch.network.ptr_eq(&observation.network))
    {
        for candidate in &mut candidates {
            if let Some(previous) = miss.candidates.iter().find(|previous| {
                same_candidates(
                    std::slice::from_ref(previous),
                    std::slice::from_ref(candidate),
                )
            }) {
                candidate.missed = previous.missed;
                candidate.attempted_at = previous.attempted_at;
            }
        }
    }
    let current = Arc::new(());
    let epoch = MissEpoch {
        network: observation.network,
        candidates: Arc::downgrade(&current),
    };
    *miss = HopMiss {
        candidates,
        target_hints,
        observed: observation.order,
        _current: Some(current),
        ..HopMiss::new(Some(epoch.clone()))
    };
    Some(epoch)
}

/// Atomically claim actual work under the existing route owner. Recovery
/// shares its evidence with comparisons but is not delayed by their period.
fn begin_middle(
    me: &str,
    to: &str,
    epoch: &MissEpoch,
    id: &str,
    purpose: HopUse,
    by: tokio::time::Instant,
) -> bool {
    let mut all = MISSED.lock().unwrap_or_else(|p| p.into_inner());
    let Some(miss) = all.get_mut(&(me.to_owned(), to.to_owned())) else {
        return false;
    };
    if crate::deadline::expired(Some(by))
        || !current_miss_epoch(Some(epoch))
        || !same_miss_epoch(miss.epoch.as_ref(), Some(epoch))
    {
        return false;
    }
    let Some(middle) = miss.candidates.iter_mut().find(|middle| middle.id == id) else {
        return false;
    };
    if !middle.wait_for(purpose).is_zero() {
        return false;
    }
    middle.attempted_at = Some(std::time::Instant::now());
    true
}

fn used_middle(me: &str, to: &str, epoch: &MissEpoch, id: &str, wire: HopWire) {
    let mut all = MISSED.lock().unwrap_or_else(|p| p.into_inner());
    let Some(miss) = all.get_mut(&(me.to_owned(), to.to_owned())) else {
        return;
    };
    if current_miss_epoch(Some(epoch))
        && same_miss_epoch(miss.epoch.as_ref(), Some(epoch))
        && let Some(middle) = miss.candidates.iter_mut().find(|middle| middle.id == id)
    {
        // Record the wire this attempt actually established, before asking.
        // Its own successful dial is not an external candidate change that
        // should forgive a NO_ROUTE returned on that very same wire.
        middle.live = true;
        middle.wires = vec![wire];
    }
}

fn missed_in_epoch(me: &str, to: &str, epoch: Option<MissEpoch>, attempted: &[&str]) {
    let mut all = MISSED.lock().unwrap_or_else(|p| p.into_inner());
    // Check under the same lock as the write. Neither a network change nor
    // a new candidate context can be overwritten by an old completion.
    if !current_miss_epoch(epoch.as_ref()) {
        return;
    }
    let key = (me.to_owned(), to.to_owned());
    if epoch.is_some()
        && !all
            .get(&key)
            .is_some_and(|e| same_miss_epoch(e.epoch.as_ref(), epoch.as_ref()))
    {
        return;
    }
    let e = all
        .entry(key)
        .or_insert_with(|| HopMiss::new(epoch.clone()));
    if !same_miss_epoch(e.epoch.as_ref(), epoch.as_ref()) {
        *e = HopMiss::new(epoch);
    }
    e.count = e.count.saturating_add(1);
    e.at = std::time::Instant::now();
    for middle in &mut e.candidates {
        if attempted.contains(&middle.id.as_str()) {
            let count = middle
                .missed
                .map_or(1, |(count, _)| count.saturating_add(1));
            middle.missed = Some((count, e.at));
        }
    }
}

/// Commit while the existing routing owner still owns this answer. A
/// rejected publication does not clear waits or retire a newer context.
fn commit_in_epoch<T>(
    me: &str,
    to: &str,
    epoch: Option<&MissEpoch>,
    by: Option<tokio::time::Instant>,
    commit: impl FnOnce() -> Option<T>,
) -> Option<T> {
    let mut all = MISSED.lock().unwrap_or_else(|p| p.into_inner());
    let key = (me.to_owned(), to.to_owned());
    if crate::deadline::expired(by) || !current_miss_epoch(epoch) {
        return None;
    }
    let miss = all.get_mut(&key)?;
    if !same_miss_epoch(miss.epoch.as_ref(), epoch) {
        return None;
    }
    let committed = commit()?;
    if let Some(epoch) = epoch {
        // Success clears the wait, not the newest observation. Replacing
        // its token also retires every answer still using the old one.
        let current = Arc::new(());
        miss.count = 0;
        for middle in &mut miss.candidates {
            middle.missed = None;
        }
        miss.epoch = Some(MissEpoch {
            network: epoch.network.clone(),
            candidates: Arc::downgrade(&current),
        });
        miss._current = Some(current);
    } else {
        all.remove(&key);
    }
    Some(committed)
}

/// The unchanged backoff applies only to the context that observed it.
fn missed_lately_in_epoch(me: &str, to: &str, epoch: Option<&MissEpoch>) -> bool {
    let all = MISSED.lock().unwrap_or_else(|p| p.into_inner());
    current_miss_epoch(epoch)
        && all.get(&(me.to_owned(), to.to_owned())).is_some_and(|e| {
            same_miss_epoch(e.epoch.as_ref(), epoch)
                && if epoch.is_some() {
                    !e.candidates.is_empty() && e.candidates.iter().all(HopCandidate::cooling)
                } else {
                    e.count > 0 && e.at.elapsed() < missed_wait(e.count)
                }
        })
}

// The original pure backoff contract exercises the same production core
// without a Wires owner. Actual Node tests exercise owned network epochs.
#[cfg(test)]
fn missed(me: &str, to: &str) {
    missed_in_epoch(me, to, None, &[]);
}

#[cfg(test)]
fn made(me: &str, to: &str) {
    let _ = commit_in_epoch(me, to, None, None, || Some(()));
}

#[cfg(test)]
fn missed_lately(me: &str, to: &str) -> bool {
    missed_lately_in_epoch(me, to, None)
}

/// Per (machine, middle): when a hop through it lost to the relay.
static LOST: std::sync::LazyLock<
    std::sync::Mutex<std::collections::HashMap<(String, String), std::time::Instant>>,
> = std::sync::LazyLock::new(Default::default);

fn lost(to: &str, via: &str) {
    LOST.lock()
        .unwrap_or_else(|p| p.into_inner())
        .insert((to.to_owned(), via.to_owned()), std::time::Instant::now());
}

fn was_lost(to: &str, via: &str) -> bool {
    LOST.lock()
        .unwrap_or_else(|p| p.into_inner())
        .get(&(to.to_owned(), via.to_owned()))
        .is_some_and(|t| t.elapsed() < LOST_FOR)
}

/// Reclaims a carrier after two looks. Only an actual comparison loss
/// suppresses its middle; disappearing paths merely make it unused.
fn release_hop(
    links: &khor_net::carry::Links,
    id: &str,
    peer: EndpointId,
    via: EndpointId,
    attachment: u64,
    road: Option<khor_net::path::PathKind>,
    looks: u8,
    comparison: CarrierComparison,
) {
    if !links.detach_attached(&peer, Carrier::Hop { via }, attachment) {
        khor_net::carry::observe_lifecycle(
            id,
            "hop_release_not_current",
            || serde_json::json!({ "attachment": attachment, "via": via.to_string() }),
        );
        return;
    }
    let phase = match comparison {
        CarrierComparison::Unused => "hop_unused_release",
        _ => "hop_measured_release",
    };
    khor_net::carry::observe_lifecycle(id, phase, || {
        serde_json::json!({
            "attachment": attachment, "via": via.to_string(), "road": format!("{road:?}"), "looks": looks,
        })
    });
    if comparison == CarrierComparison::Outperformed
        && road == Some(khor_net::path::PathKind::Relay)
    {
        lost(id, &via.to_string());
    }
}

/// Per (this machine, far machine): hops this machine asked for, opening
/// or up ([`Asking`]).
static ASKING: std::sync::LazyLock<
    std::sync::Mutex<std::collections::HashMap<(String, String), u32>>,
> = std::sync::LazyLock::new(Default::default);

/// **One hop this machine asked for**, held from the ask until the hop
/// ends. Both ends of a slow relay measure it, so both may ask at once,
/// through one middle or two; each end would then take the other's hop
/// in place of its own and end both. Of two hops between the same two
/// machines, the one the lower id asked for stands, at both ends: the
/// lower refuses the other's while it asks ([`Node::serve_hop`]), the
/// higher keeps the other's that came in meanwhile ([`Node::open_hop_in_epoch`]).
struct Asking((String, String));

impl Asking {
    fn new(me: &str, to: &str) -> Asking {
        let key = (me.to_owned(), to.to_owned());
        *ASKING
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .entry(key.clone())
            .or_default() += 1;
        Asking(key)
    }
}

impl Drop for Asking {
    fn drop(&mut self) {
        let mut asking = ASKING.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(n) = asking.get_mut(&self.0) {
            *n -= 1;
            if *n == 0 {
                asking.remove(&self.0);
            }
        }
    }
}

/// A prepared stream belongs to its opening future until the watcher
/// accepts it. A live wire using the offer also transfers that ownership.
struct PendingHop {
    links: khor_net::carry::Links,
    wires: std::sync::Arc<crate::wires::Wires>,
    id: String,
    peer: EndpointId,
    via: EndpointId,
    attachment: u64,
    asked: Option<Asking>,
    handoff: Option<tokio::sync::oneshot::Sender<HopHandoff>>,
    committed: bool,
}

struct HopHandoff {
    asked: Asking,
    pending: Option<tokio::sync::watch::Receiver<()>>,
    cancelled: bool,
}

impl PendingHop {
    fn hand_off(&mut self, cancelled: bool) -> bool {
        let (Some(handoff), Some(asked)) = (self.handoff.take(), self.asked.take()) else {
            return false;
        };
        self.committed = handoff
            .send(HopHandoff {
                asked,
                pending: self.wires.pending_dial_done(&self.id),
                cancelled,
            })
            .is_ok();
        self.committed
    }
}

impl Drop for PendingHop {
    fn drop(&mut self) {
        if !self.committed {
            #[cfg(test)]
            self.wires.hold_pending_drop_for_test(self.attachment);
            let withdrawn = self.wires.withdraw_carrier_if_idle(&self.id, || {
                self.links.detach_attached(
                    &self.peer,
                    Carrier::Hop { via: self.via },
                    self.attachment,
                );
            });
            if !matches!(withdrawn, Ok(true)) {
                self.hand_off(true);
            }
        }
    }
}

fn asking(me: &str, to: &str) -> bool {
    ASKING
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .contains_key(&(me.to_owned(), to.to_owned()))
}

/// Whether a prospective hop's two-leg estimate pays the original preparation
/// cost hurdle. Already open paths use measured eligibility and confirmation.
pub(crate) fn beats(hop_ms: u32, relay_ms: u32) -> bool {
    let current = Duration::from_millis(u64::from(relay_ms));
    let candidate = Duration::from_millis(u64::from(hop_ms));
    let cost = khor_net::carry::switch_margin(current, false).max(current / 4);
    candidate.saturating_add(cost) <= current
}

/// A tunnel stream's first words, when they ask for a hop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Ask {
    /// From the asker: splice me to this machine.
    To(EndpointId),
    /// From a middle: this stream comes from this machine.
    From(EndpointId),
    /// Same encrypted transport, with receiver publication delayed until payload.
    PrepareTo(EndpointId),
    PrepareFrom(EndpointId),
}

impl Ask {
    pub(crate) fn of(target: &str) -> Option<Ask> {
        if let Some(id) = target.strip_prefix(PREPARE_HAND) {
            return id.parse().ok().map(Ask::PrepareFrom);
        }
        if let Some(id) = target.strip_prefix(PREPARE) {
            return id.parse().ok().map(Ask::PrepareTo);
        }
        if let Some(id) = target.strip_prefix(HAND) {
            return id.parse().ok().map(Ask::From);
        }
        target.strip_prefix(ASK)?.parse().ok().map(Ask::To)
    }

    fn words(self) -> String {
        match self {
            Ask::To(id) => format!("{ASK}{id}"),
            Ask::From(id) => format!("{HAND}{id}"),
            Ask::PrepareTo(id) => format!("{PREPARE}{id}"),
            Ask::PrepareFrom(id) => format!("{PREPARE_HAND}{id}"),
        }
    }
}

/// Which legs a middle may stand on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Legs {
    /// **Before a dial** ([`Node::hop_ahead`]): nothing measures the relay
    /// to `to` yet, so a relay leg is taken only to a middle that carries
    /// `to` over ssh. A relay to a middle that reaches `to` directly is
    /// presumed no better than the relay to `to`: one more try before
    /// every dial for a guess.
    Dial,
    /// No live road to the target: a configured relay is no evidence it
    /// answers here. Any non-recursive two-leg road may restore access,
    /// and a missing second-leg hint may be checked by bounded admission.
    Recovery,
    /// **Against the relay in use** ([`Node::compare_roads`]): either leg
    /// may be the relay — `to`'s home may be a far relay where the
    /// middle's is near (hinton on aps1, 10-20). The legs' sum is held
    /// against the relay's measured round trip, and iroh's end to end
    /// measurement has the last word ([`Node::open_hop_in_epoch`]).
    Measured,
    /// **This machine has no relay** (C7, `egress`): the first leg must be
    /// one it has without one — direct — and the second may be the
    /// middle's relay, which is the relay this machine cannot reach.
    NoRelay,
    /// **The far end says it has no relay** (C7, its row's `no_relay`):
    /// the second leg must be direct, and the first may be this
    /// machine's relay — the mirror of [`Legs::NoRelay`].
    ToNoRelay,
}

/// Eligible intermediates and their total two-leg round trips, in milliseconds.
/// Neither endpoint, satellites, paused machines, nor recursive hops qualify.
/// The leg policy controls relay eligibility; every policy sorts by cost.
/// A direct first leg only breaks equal-cost ties outside measured comparisons.
/// An unmeasured leg costs
/// `u16::MAX` for comparison by [`worth_a_hop`].
pub(crate) fn middles_with_cost(
    me: &str,
    to: &str,
    rows: &[(String, bool, bool)],
    mine: impl Fn(&str) -> Option<Road>,
    theirs: impl Fn(&str) -> Option<Road>,
    legs: Legs,
) -> Vec<(String, u32)> {
    let mut found: Vec<(u32, bool, String)> = rows
        .iter()
        .filter(|(id, sat, paused)| id != me && id != to && !sat && !paused)
        .filter_map(|(b, _, _)| {
            let first = mine(b).filter(|r| r.via.is_none())?;
            let second = match theirs(b) {
                Some(second) if second.via.is_none() => second,
                None if legs == Legs::Recovery => Road::own(to, RoadKind::None, None, None),
                _ => return None,
            };
            let usable = match (first.road, second.road) {
                (RoadKind::Direct, RoadKind::Direct | RoadKind::Ssh)
                | (RoadKind::Relay, RoadKind::Ssh) => true,
                (RoadKind::Direct, RoadKind::Relay) if legs == Legs::NoRelay => true,
                (RoadKind::Relay, RoadKind::Direct) if legs == Legs::ToNoRelay => true,
                (RoadKind::Direct | RoadKind::Relay, RoadKind::None) if legs == Legs::Recovery => {
                    true
                }
                (RoadKind::Direct | RoadKind::Relay, RoadKind::Direct | RoadKind::Relay) => {
                    matches!(legs, Legs::Measured | Legs::Recovery)
                }
                _ => false,
            };
            let ms =
                u32::from(first.ms.unwrap_or(u16::MAX)) + u32::from(second.ms.unwrap_or(u16::MAX));
            usable.then(|| {
                (
                    ms,
                    legs != Legs::Measured && first.road != RoadKind::Direct,
                    b.clone(),
                )
            })
        })
        .collect();
    found.sort();
    found.into_iter().map(|(ms, _, b)| (b, ms)).collect()
}

/// **The middles worth a hop to `to` against its relay, measured at
/// `relay_ms`** (pure, for [`Node::compare_roads`]): those of
/// [`middles_with_cost`] on [`Legs::Measured`] whose two legs beat the
/// relay ([`beats`]) and
/// through which no hop to `to` lost lately (`lost`, [`LOST_FOR`]).
pub(crate) fn worth_a_hop(
    me: &str,
    to: &str,
    relay_ms: u32,
    rows: &[(String, bool, bool)],
    mine: impl Fn(&str) -> Option<Road>,
    theirs: impl Fn(&str) -> Option<Road>,
    lost: impl Fn(&str) -> bool,
) -> Vec<String> {
    middles_with_cost(me, to, rows, mine, theirs, Legs::Measured)
        .into_iter()
        .filter(|(b, ms)| beats(*ms, relay_ms) && !lost(b))
        .map(|(b, _)| b)
        .collect()
}

/// The same effective IP/Relay/nearby candidates as the dialer. Invalid
/// input, list order, duplicates and source provenance are not new roads.
fn effective_hop_hints(device: &crate::DeviceInfo) -> Vec<String> {
    let Ok(addr) = khor_net::endpoint::dial_addr(&device.id, &device.addrs, &[]) else {
        return Vec::new();
    };
    let mut hints: Vec<_> = addr.addrs.iter().map(|addr| format!("{addr:?}")).collect();
    hints.sort_unstable();
    hints.dedup();
    hints
}

type PreparedHop = (
    EndpointId,
    usize,
    iroh::endpoint::WeakConnectionHandle,
    SendStream,
    RecvStream,
);

impl Node {
    async fn prepare_hop_middle(
        &self,
        ep: &iroh::Endpoint,
        target: &crate::DeviceInfo,
        middle: &crate::DeviceInfo,
        epoch: &MissEpoch,
        prepared: bool,
    ) -> Option<PreparedHop> {
        let via = middle.id.parse::<EndpointId>().ok()?;
        let peer = target.id.parse::<EndpointId>().ok()?;
        let line = Box::pin(crate::journal::BY.scope(
            crate::journal::By::Hop,
            self.line_to_inner(ep, middle, Proto::Tunnel, None, false),
        ))
        .await
        .ok()?;
        if let Some(wire) = HopWire::observed(&line.conn) {
            used_middle(self.device_str(), &target.id, epoch, &middle.id, wire);
        }
        let ask = if prepared {
            Ask::PrepareTo(peer)
        } else {
            Ask::To(peer)
        };
        let (send, recv) = crate::tunnel::dial(&line.conn, &ask.words()).await.ok()?;
        Some((
            via,
            line.conn.stable_id(),
            line.conn.weak_handle(),
            send,
            recv,
        ))
    }

    /// This machine's road to `b` for picking a middle: its live wire
    /// when one is up (a carried one is no first leg), else what it last
    /// wrote in its own row.
    fn road_for_hop(&self, b: &str, mine: &[Road]) -> Option<Road> {
        let r = self.wires.link(b);
        let ms = r
            .rtt
            .map(|t| t.as_millis().min(u128::from(u16::MAX)) as u16);
        match r.road {
            Some(khor_net::path::PathKind::Direct) => {
                return Some(Road::own(b, RoadKind::Direct, None, ms));
            }
            Some(khor_net::path::PathKind::Relay) => {
                return Some(Road::own(b, RoadKind::Relay, r.relay, ms));
            }
            Some(khor_net::path::PathKind::Carried) => return None,
            _ => {}
        }
        if self.carry.carrier(&b.parse().ok()?).is_some() {
            return None;
        }
        for conn in self.wires.incoming_for_hop(b) {
            let kind = match khor_net::path::classify(&conn) {
                khor_net::path::PathKind::Direct => RoadKind::Direct,
                khor_net::path::PathKind::Relay => RoadKind::Relay,
                _ => continue,
            };
            let ms = crate::wires::rtt_of(&conn)
                .as_millis()
                .min(u128::from(u16::MAX)) as u16;
            return Some(Road::own(
                b,
                kind,
                khor_net::path::relay_name(&conn),
                Some(ms),
            ));
        }
        if r.down.is_some() {
            return None;
        }
        mine.iter().find(|w| w.to == b).cloned()
    }

    /// Whether X should look for a hop to `y` before dialling it: the
    /// switch is on, no link to `y` is up, `y` is a member, and its last
    /// road from here was not direct — or the last try at it failed.
    fn wants_hop(&self, y: &crate::DeviceInfo, mine: &[Road]) -> bool {
        let Ok(peer) = y.id.parse::<EndpointId>() else {
            return false;
        };
        if !self.hop_on()
            || self.carrying().is_none()
            || self.carry.has(&peer)
            || y.sponsor.is_some()
        {
            return false;
        }
        let r = self.wires.link(&y.id);
        // No relay here, or none there (C7): every road but a direct
        // one is a hop's.
        if r.road.is_none() || r.down.is_some() || self.no_relay() || y.no_relay {
            return true;
        }
        let direct = |k: Option<RoadKind>| k == Some(RoadKind::Direct);
        let last = match r.last_road {
            Some(khor_net::path::PathKind::Direct) => Some(RoadKind::Direct),
            _ => None,
        };
        !direct(last)
            && !direct(
                mine.iter()
                    .find(|w| w.to == y.id && w.via.is_none())
                    .map(|w| w.road),
            )
    }

    /// **X's half**: before dialling `y`, open a hop to it through a
    /// middle when the roadbook names one. Quietly nothing when no hop is
    /// wanted, none can be had, or it takes past [`HOP_WITHIN`]: the dial
    /// goes on without it.
    pub(crate) fn hop_ahead<'a>(
        &'a self,
        ep: &'a iroh::Endpoint,
        y: &'a crate::DeviceInfo,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
        // This explicit cold-path boundary also breaks Send inference through
        // hop -> middle dial -> hop, while held-wire requests stay unboxed.
        Box::pin(async move {
            // The cheap answers first: this runs before every dial.
            if !self.hop_on() || y.id.parse().is_ok_and(|p| self.carry.has(&p)) {
                return;
            }
            let observation = MissObservation::new(self.wires.hop_epoch());
            let Ok(all) = self.devices() else { return };
            let me = self.device_str().to_owned();
            let mine = all
                .iter()
                .find(|d| d.id == me)
                .map(|d| self.roads_of(d))
                .unwrap_or_default();
            if !self.wants_hop(y, &mine) {
                return;
            }
            let rows: Vec<(String, bool, bool)> = all
                .iter()
                .map(|d| {
                    (
                        d.id.clone(),
                        d.sponsor.is_some(),
                        crate::pause::peer_on(self.root(), &d.id),
                    )
                })
                .collect();
            // Relay availability reports cannot exclude bounded recovery when
            // there is no usable target wire. The middle checks its second leg.
            let legs = match (self.no_relay(), y.no_relay) {
                _ if self.wires.link(&y.id).road.is_none() => Legs::Recovery,
                (true, _) => Legs::NoRelay,
                (false, true) => Legs::ToNoRelay,
                (false, false) => Legs::Dial,
            };
            let mut picked: Vec<String> = middles_with_cost(
                &me,
                &y.id,
                &rows,
                |b| self.road_for_hop(b, &mine),
                |b| {
                    all.iter()
                        .find(|d| d.id == b)
                        .and_then(|d| self.roads_of(d).into_iter().find(|r| r.to == y.id))
                },
                legs,
            )
            .into_iter()
            .map(|(b, _)| b)
            .collect();
            if legs == Legs::Recovery {
                // Recovery uses a road we hold before spending its finite
                // attempts on cached reachability. Stable sorting keeps the
                // existing latency order inside each group; a live relay
                // gets the same admission preference as a live IP wire.
                picked.sort_by_key(|b| self.road_for_hop(b, &[]).is_none());
            }
            #[cfg(test)]
            self.wires.hold_hop_selection_for_test().await;
            let Some(epoch) = self.hop_miss_epoch_for(y, &all, &picked, observation) else {
                return;
            };
            let purpose = if legs == Legs::Recovery
                && crate::deadline::current().is_some()
                && matches!(
                    crate::journal::by_now(),
                    crate::journal::By::Ask | crate::journal::By::Forced
                ) {
                HopUse::Recovery
            } else {
                HopUse::Dial
            };
            if purpose != HopUse::Recovery && missed_lately_in_epoch(&me, &y.id, Some(&epoch)) {
                return;
            }
            // Keep the cold preparation state out of every enclosing dial and
            // resident request's future, including requests that need no hop.
            Box::pin(self.open_hop_in_epoch(ep, y, &all, &picked, epoch, purpose)).await;
        })
    }

    #[cfg(test)]
    fn hop_miss_epoch(
        &self,
        to: &str,
        picked: &[String],
        observation: MissObservation,
    ) -> Option<MissEpoch> {
        let all = self.devices().ok()?;
        let target = all.iter().find(|device| device.id == to)?;
        self.hop_miss_epoch_for(target, &all, picked, observation)
    }

    fn hop_miss_epoch_for(
        &self,
        target: &crate::DeviceInfo,
        all: &[crate::DeviceInfo],
        picked: &[String],
        observation: MissObservation,
    ) -> Option<MissEpoch> {
        miss_context(self.device_str(), &target.id, observation, || {
            // Use the roster read after this observation began, not a
            // caller's older DeviceInfo. No document I/O under MISSED.
            let target = all
                .iter()
                .find(|device| device.id == target.id)
                .unwrap_or(target);
            let candidates = picked
                .iter()
                .map(|id| {
                    let mut conns: Vec<_> = self
                        .wires
                        .live(id, Proto::Tunnel)
                        .or_else(|| self.wires.live(id, Proto::Main))
                        .into_iter()
                        .collect();
                    if conns.is_empty() {
                        conns = self.wires.incoming_for_hop(id);
                    }
                    let mut wires: Vec<_> = conns.iter().filter_map(HopWire::observed).collect();
                    wires.sort_unstable_by_key(|wire| wire.id);
                    wires.dedup_by_key(|wire| wire.id);
                    HopCandidate {
                        id: id.clone(),
                        live: self.road_for_hop(id, &[]).is_some(),
                        wires,
                        hints: all
                            .iter()
                            .find(|device| &device.id == id)
                            .map(effective_hop_hints)
                            .unwrap_or_default(),
                        missed: None,
                        attempted_at: None,
                    }
                })
                .collect();
            (effective_hop_hints(target), candidates)
        })
    }

    #[cfg(test)]
    async fn open_hop(
        &self,
        ep: &iroh::Endpoint,
        y: &crate::DeviceInfo,
        all: &[crate::DeviceInfo],
        picked: &[String],
    ) -> bool {
        let observation = MissObservation::new(self.wires.hop_epoch());
        let Some(epoch) = self.hop_miss_epoch_for(y, all, picked, observation) else {
            return false;
        };
        self.open_hop_in_epoch(ep, y, all, picked, epoch, HopUse::Dial)
            .await
    }

    /// **Opens a hop to `y` through the first of `picked` that answers**
    /// (at most [`TRIES`], within [`HOP_WITHIN`]), hands iroh the carried
    /// address (vendor/iroh patch 12: opened at once on a live connection)
    /// and watches it: the hop is let go once direct, or the relay, has
    /// carried the connection for two looks — the selector
    /// (`carry::RoadOrder`) found it no faster. `true` when a hop is up.
    async fn open_hop_in_epoch(
        &self,
        ep: &iroh::Endpoint,
        y: &crate::DeviceInfo,
        all: &[crate::DeviceInfo],
        picked: &[String],
        epoch: MissEpoch,
        purpose: HopUse,
    ) -> bool {
        let caller = crate::deadline::current();
        let hop_by = tokio::time::Instant::now() + HOP_WITHIN;
        let by = caller.map_or(hop_by, |caller| caller.min(hop_by));
        if crate::deadline::expired(Some(by)) {
            return false;
        }
        let Ok(peer) = y.id.parse::<EndpointId>() else {
            return false;
        };
        let me = self.device_str().to_owned();
        let eligible = loop {
            let (eligible, retry_in) = {
                let all = MISSED.lock().unwrap_or_else(|p| p.into_inner());
                let Some(miss) = all.get(&(me.clone(), y.id.clone())) else {
                    return false;
                };
                if !current_miss_epoch(Some(&epoch))
                    || !same_miss_epoch(miss.epoch.as_ref(), Some(&epoch))
                {
                    return false;
                }
                let mut ready = Vec::with_capacity(TRIES);
                let mut retry_in: Option<Duration> = None;
                for id in picked {
                    let Some(middle) = miss.candidates.iter().find(|middle| &middle.id == id)
                    else {
                        continue;
                    };
                    let wait = middle.wait_for(purpose);
                    if wait.is_zero() {
                        ready.push(id);
                        if ready.len() == TRIES {
                            break;
                        }
                    } else if purpose == HopUse::Recovery && middle.live && wait < HOP_WITHIN {
                        retry_in = Some(retry_in.map_or(wait, |previous| previous.min(wait)));
                    }
                }
                (ready, retry_in)
            };
            if !eligible.is_empty() {
                break eligible;
            }
            let Some(wait) = retry_in else { return false };
            // This is a caller-owned wait, inside the SAME hop/caller deadline.
            // Cancellation drops it. Recheck the epoch and atomically claim
            // each actual attempt below after any other demand woke first.
            if crate::deadline::wait(Some(by), tokio::time::sleep(wait))
                .await
                .is_err()
            {
                return false;
            }
        };
        let asked = Asking::new(&me, &y.id);
        let mut attempted = Vec::with_capacity(TRIES);
        let tries = async {
            if y.hop_staged && eligible.len() > 1 {
                // The row is only a hint. Every positive preparation must
                // use words understood by both B and Y; an old endpoint
                // cannot accidentally grant a concurrent legacy admission.
                // Reserve half of the SAME deadline for an unknown or
                // downgraded peer's bounded legacy fallback.
                let now = tokio::time::Instant::now();
                let prepared_by = now + by.saturating_duration_since(now) / 2;
                let preparing = async {
                    use futures::StreamExt;
                    let mut pending = futures::stream::FuturesUnordered::new();
                    for b in &eligible {
                        let Some(middle) = all.iter().find(|d| &d.id == *b) else {
                            continue;
                        };
                        if middle.id.parse::<EndpointId>().is_err() {
                            continue;
                        }
                        if !begin_middle(&me, &y.id, &epoch, b, purpose, prepared_by) {
                            continue;
                        }
                        attempted.push(b.as_str());
                        pending.push(self.prepare_hop_middle(ep, y, middle, &epoch, true));
                    }
                    while let Some(result) = pending.next().await {
                        if result.is_some() {
                            // Dropping the remaining futures cancels their
                            // request streams before any carried publication.
                            return result;
                        }
                    }
                    None
                };
                if let Ok(Some(prepared)) = crate::deadline::wait(
                    Some(prepared_by),
                    crate::deadline::scope(prepared_by, preparing),
                )
                .await
                {
                    return Some(prepared);
                }
            }
            let slots = eligible.len();
            for (index, b) in eligible.into_iter().enumerate() {
                let Some(middle) = all.iter().find(|d| &d.id == b) else {
                    continue;
                };
                if middle.id.parse::<EndpointId>().is_err() {
                    continue;
                }
                if !attempted.contains(&b.as_str()) {
                    if !begin_middle(&me, &y.id, &epoch, b, purpose, by) {
                        continue;
                    }
                    attempted.push(b.as_str());
                }
                // Share the existing deadline. A silent first admission
                // cannot consume the opportunity of every other middle.
                // Expiring this preparation does not retire its outer wire.
                let now = tokio::time::Instant::now();
                let left = by.saturating_duration_since(now);
                let slice = left / u32::try_from(slots - index).unwrap_or(u32::MAX);
                let attempt_by = now + slice;
                if let Ok(Some(prepared)) = crate::deadline::wait(
                    Some(attempt_by),
                    crate::deadline::scope(
                        attempt_by,
                        self.prepare_hop_middle(ep, y, middle, &epoch, false),
                    ),
                )
                .await
                {
                    return Some(prepared);
                }
            }
            None
        };
        let result = crate::deadline::wait(Some(by), crate::deadline::scope(by, tries)).await;
        let tried = u32::try_from(attempted.len()).unwrap_or(u32::MAX);
        let Ok(Some((via, outer_wire, outer, send, recv))) = result else {
            if tried > 0 && !crate::deadline::expired(caller) {
                self.wires
                    .note(Some(&y.id), crate::journal::What::Hop { via: None, tried });
                missed_in_epoch(&me, &y.id, Some(epoch), &attempted);
            }
            return false;
        };
        // Lock order is network generation -> routing owner -> Links.
        // No await or document I/O: forgiveness, a new candidate context,
        // or a protected carrier cannot split validation from publication.
        let attached = self.wires.with_hop_epoch(&epoch.network, || {
            commit_in_epoch(&me, &y.id, Some(&epoch), Some(by), || {
                stream_trace::attach_if(
                    &self.carry,
                    peer,
                    Carrier::Hop { via },
                    Some(outer_wire),
                    "initiator",
                    (send, recv),
                    |current| {
                        !crate::deadline::expired(Some(by))
                            && match current {
                                Some(Carrier::Ssh) => false,
                                Some(Carrier::Hop { .. }) if y.id < me => false,
                                _ => true,
                            }
                    },
                )
            })
        });
        let Some(attached) = attached.flatten() else {
            return false;
        };
        self.wires.note(
            Some(&y.id),
            crate::journal::What::Hop {
                via: Some(via.to_string()),
                tried,
            },
        );
        let attachment = attached.serial();
        let (handoff, handed) = tokio::sync::oneshot::channel::<HopHandoff>();
        let mut pending = PendingHop {
            links: self.carry.clone(),
            wires: self.wires.clone(),
            id: y.id.clone(),
            peer,
            via,
            attachment,
            asked: Some(asked),
            handoff: Some(handoff),
            committed: false,
        };
        khor_net::carry::observe_lifecycle(&y.id, "hop_outer_bound", || {
            serde_json::json!({
                "role": "initiator", "attachment": attachment,
                "via": via.to_string(), "outer_wire": outer_wire,
            })
        });
        let (links, wires, id, node) = (
            self.carry.clone(),
            self.wires.clone(),
            y.id.clone(),
            self.weak(),
        );
        // The wire to the middle is judged on a hop's patience while the
        // hop lives (`wires::HOP_SILENT`): a frozen middle hands back sooner.
        let riding = wires.hop_held(&via.to_string());
        tokio::spawn(async move {
            let _riding = riding;
            let ended = attached.ended();
            tokio::pin!(ended);
            // Register the existing watcher before the first cancellable
            // await. It owns path cleanup even if the opening future stops
            // during offer_addrs; Asking transfers only after that offer.
            let handed = tokio::select! {
                () = &mut ended => {
                    crate::carry::let_path_go(&node, &links, peer).await;
                    return;
                }
                received = handed => received.ok(),
            };
            let Some(handoff) = handed else {
                ended.await;
                crate::carry::let_path_go(&node, &links, peer).await;
                return;
            };
            let _asked = handoff.asked;
            let mut pending = handoff.pending;
            loop {
                if let Some(mut waiting) = pending.take() {
                    tokio::select! {
                        () = &mut ended => {
                            crate::carry::let_path_go(&node, &links, peer).await;
                            return;
                        }
                        _ = waiting.changed() => {}
                    }
                }
                #[cfg(test)]
                let reviewed = wires.hold_pending_carrier_for_test(attachment).await;
                let withdrawn = if handoff.cancelled {
                    wires.withdraw_carrier_if_idle(&id, || {
                        links.detach_attached(&peer, Carrier::Hop { via }, attachment);
                    })
                } else {
                    Ok(false)
                };
                #[cfg(test)]
                if let Some(reviewed) = reviewed {
                    let _ = reviewed.send(());
                }
                match withdrawn {
                    Err(successor) => pending = Some(successor),
                    Ok(true) => {
                        crate::carry::let_path_go(&node, &links, peer).await;
                        return;
                    }
                    Ok(false) => break,
                }
            }
            let mut held = 0u8;
            loop {
                tokio::select! {
                    () = &mut ended => {
                        khor_net::carry::observe_lifecycle(&id, "hop_attachment_ended", || serde_json::json!({
                            "attachment": attachment, "via": via.to_string(),
                        }));
                        // The road under the carried path is gone: iroh
                        // chooses again now, not when the path idles out.
                        crate::carry::let_path_go(&node, &links, peer).await;
                        return;
                    }
                    () = tokio::time::sleep(LOOK_EVERY) => {}
                }
                // The selected direct or relay road beats the hop on
                // measurement for two looks: the middle is let go, so it
                // carries nothing it need not. **The relay keeps it only on
                // measurement** (carry, 2026-10-23): one taken while the
                // carried road stalls is not the hop losing. The phone's leg
                // to turing black for 3 s read 「relay」 at two looks, the
                // middle was let go as lost for ten minutes, its path closed,
                // and the connection, back on the hop, fell to the relay for
                // good (netlab f4).
                khor_net::carry::observe_lifecycle(
                    &id,
                    "hop_outer_look",
                    || serde_json::json!({"role": "initiator", "attachment": attachment, "outer": outer_facts(&outer)}),
                );
                let road = wires.link(&id).road;
                // No outgoing selected road can also mean the last
                // consumer closed. Incoming wires and unknown path facts
                // still participate in this same conservative comparison.
                let comparison = wires.compare_carrier(&id);
                held = if comparison != CarrierComparison::Needed {
                    held + 1
                } else {
                    0
                };
                if held >= 2 {
                    #[cfg(test)]
                    let reviewed = wires.hold_normal_release_for_test(attachment).await;
                    let _ = wires.withdraw_carrier_if_idle(&id, || {
                        let current_road = wires.link(&id).road;
                        let current_comparison = wires.compare_carrier(&id);
                        if current_road == road && current_comparison == comparison {
                            release_hop(&links, &id, peer, via, attachment, road, held, comparison);
                        }
                    });
                    // A pending consumer or a changed measurement starts
                    // the existing two-look comparison again after it ends.
                    held = 0;
                    #[cfg(test)]
                    if let Some(reviewed) = reviewed {
                        let _ = reviewed.send(());
                    }
                }
                #[cfg(test)]
                wires.note_hop_look_for_test(attachment);
            }
        });
        let offer = async {
            #[cfg(test)]
            self.wires.hold_hop_offer_for_test(attachment).await;
            if let Some(addr) = self.carried_addr(&y.id) {
                ep.offer_addrs(iroh::EndpointAddr::from_parts(peer, [addr]))
                    .await;
            }
            #[cfg(test)]
            self.wires.hold_hop_offered_for_test(attachment).await;
        };
        if crate::deadline::wait(Some(by), offer).await.is_err() {
            return false;
        }
        pending.hand_off(false)
    }

    /// **The road choice by measured legs** (NET-DIRECT step 4), every
    /// resident beat, with [`COMPARE_EVERY`] per unchanged middle context:
    /// a real network/candidate/first-leg change can be compared at once.
    /// For each machine whose
    /// live wire uses a direct or relay path, the best middle's two legs added
    /// ([`middles_with_cost`]: this machine's live round trip to the
    /// middle, the middle's written one onward) against the relay's
    /// measured round trip. A hop that meets the selector's gain margin
    /// is opened; iroh then measures it end to end and the selector
    /// keeps whichever is faster, and a hop that loses is let go
    /// ([`Node::open_hop_in_epoch`]) and not tried through that middle again for
    /// [`LOST_FOR`]. Direct needs nothing here: iroh's punching runs under
    /// every road and takes over when it opens.
    pub(crate) async fn compare_roads(&self, ep: &iroh::Endpoint) {
        if !self.hop_on() || self.carrying().is_none() {
            return;
        }
        let observation = MissObservation::new(self.wires.hop_epoch());
        let Ok(all) = self.devices() else { return };
        let me = self.device_str().to_owned();
        let mine = all
            .iter()
            .find(|d| d.id == me)
            .map(|d| self.roads_of(d))
            .unwrap_or_default();
        let rows: Vec<(String, bool, bool)> = all
            .iter()
            .map(|d| {
                (
                    d.id.clone(),
                    d.sponsor.is_some(),
                    crate::pause::peer_on(self.root(), &d.id),
                )
            })
            .collect();
        for y in all.iter().filter(|d| d.id != me && d.sponsor.is_none()) {
            let Ok(peer) = y.id.parse::<EndpointId>() else {
                continue;
            };
            if self.carry.has(&peer) || crate::pause::peer_on(self.root(), &y.id) {
                continue;
            }
            let r = self.wires.link(&y.id);
            let (
                Some(khor_net::path::PathKind::Direct | khor_net::path::PathKind::Relay),
                Some(rtt),
            ) = (r.road, r.rtt)
            else {
                continue;
            };
            let relay_ms = u32::try_from(rtt.as_millis()).unwrap_or(u32::MAX);
            let picked = worth_a_hop(
                &me,
                &y.id,
                relay_ms,
                &rows,
                |b| self.road_for_hop(b, &mine),
                |b| {
                    all.iter()
                        .find(|d| d.id == b)
                        .and_then(|d| self.roads_of(d).into_iter().find(|r| r.to == y.id))
                },
                |b| was_lost(&y.id, b),
            );
            if !picked.is_empty() {
                let observation = MissObservation {
                    network: observation.network.clone(),
                    order: observation.order,
                };
                let Some(epoch) = self.hop_miss_epoch_for(y, &all, &picked, observation) else {
                    continue;
                };
                Box::pin(self.open_hop_in_epoch(ep, y, &all, &picked, epoch, HopUse::Comparison))
                    .await;
            }
        }
    }

    /// **One hop ask on a tunnel stream** (B's or Y's half), after the
    /// stream's first words named it. `paired` is the tunnel's own gate
    /// for the machine on the other end of this stream.
    pub(crate) async fn serve_hop(
        self: Arc<Self>,
        ep: Arc<iroh::Endpoint>,
        from: &str,
        incoming: iroh::endpoint::WeakConnectionHandle,
        ask: Ask,
        mut send: SendStream,
        recv: RecvStream,
        paired: bool,
    ) {
        let incoming_wire = incoming.upgrade().map(|c| c.stable_id());
        let refuse = |mut send: SendStream, byte: u8| async move {
            let _ = send.write_all(&[byte]).await;
            let _ = send.finish();
        };
        let Ok(from_id) = from.parse::<EndpointId>() else {
            return;
        };
        if !paired || !self.hop_on() || self.carrying().is_none() {
            return refuse(send, REFUSED).await;
        }
        let me = self.device_str().to_owned();
        let member = |id: &str| {
            id != me
                && id != from
                && !crate::pause::peer_on(self.root(), id)
                && self.admit(id).is_ok()
                && self
                    .devices()
                    .is_ok_and(|all| all.iter().any(|d| d.id == id && d.sponsor.is_none()))
        };
        let prepared = matches!(ask, Ask::PrepareTo(_) | Ask::PrepareFrom(_));
        match ask {
            // B: splice `from` (X) to `to` (Y).
            Ask::To(to) | Ask::PrepareTo(to) => {
                let y = to.to_string();
                if !member(&y) || matches!(self.carry.carrier(&to), Some(Carrier::Hop { .. })) {
                    return refuse(send, REFUSED).await;
                }
                let Some(d) = self
                    .devices()
                    .ok()
                    .and_then(|all| all.into_iter().find(|d| d.id == y))
                else {
                    return refuse(send, REFUSED).await;
                };
                // **B's own word on Y first** (net-dial B3): a machine B is
                // waiting out a failed dial to, that its relay said left,
                // or that B found unreachable and has not heard since, is
                // no road for X either — said at once, not after a dial.
                if self.wires.live(&y, Proto::Tunnel).is_none() && self.wires.out_of_reach(&y) {
                    return refuse(send, NO_ROUTE).await;
                }
                let by = std::time::Instant::now() + MIDDLE_WITHIN;
                // A dial for X, not for B (`journal::By::Onward`): written
                // as such, and its failure is not B's — no backoff, no
                // 「不通」, no closing what Y opened to B.
                let onward = crate::journal::BY.scope(crate::journal::By::Onward, async {
                    let line =
                        Box::pin(self.line_to_inner(&ep, &d, Proto::Tunnel, Some(by), false))
                            .await
                            .ok()?;
                    let wire = line.conn.stable_id();
                    let hand = if prepared {
                        Ask::PrepareFrom(from_id)
                    } else {
                        Ask::From(from_id)
                    };
                    crate::tunnel::dial(&line.conn, &hand.words())
                        .await
                        .ok()
                        .map(|(send, recv)| (wire, line.conn.weak_handle(), send, recv))
                });
                let Ok(Some((onward_wire, onward, ys, yr))) =
                    tokio::time::timeout(MIDDLE_WITHIN, onward).await
                else {
                    return refuse(send, NO_ROUTE).await;
                };
                if send.write_all(&[OK]).await.is_err() {
                    return;
                }
                khor_net::carry::observe_lifecycle(from, "hop_splice_bound", || {
                    serde_json::json!({
                        "to": y, "incoming_wire": incoming_wire, "onward_wire": onward_wire,
                    })
                });
                let _riding = (self.wires.hop_held(from), self.wires.hop_held(&y));
                let copying = stream_trace::splice(
                    (send, recv),
                    (ys, yr),
                    from_id,
                    (incoming_wire, onward_wire),
                );
                if std::env::var_os("KHOR_NET_DIAGNOSTICS").is_some() {
                    tokio::pin!(copying);
                    let mut look = tokio::time::interval(LOOK_EVERY);
                    loop {
                        tokio::select! {
                            () = &mut copying => break,
                            _ = look.tick() => {
                                khor_net::carry::observe_lifecycle(from, "hop_outer_look", || {
                                    serde_json::json!({"role": "middle", "to": y,
                                        "incoming": outer_facts(&incoming), "onward": outer_facts(&onward)})
                                });
                            }
                        }
                    }
                } else {
                    copying.await;
                }
                khor_net::carry::observe_lifecycle(from, "hop_splice_ended", || {
                    serde_json::json!({
                        "to": y, "incoming_wire": incoming_wire, "onward_wire": onward_wire,
                    })
                });
            }
            // Y: this stream is the link to `from`'s asker (X), via B.
            Ask::From(x) | Ask::PrepareFrom(x) => {
                let asker = x.to_string();
                if !member(&asker) || matches!(self.carry.carrier(&x), Some(Carrier::Ssh)) {
                    return refuse(send, REFUSED).await;
                }
                // This end asked for a hop to the asker too, and its id is
                // the lower: its own stands ([`Asking`]).
                if me < asker && asking(&me, &asker) {
                    return refuse(send, REFUSED).await;
                }
                if send.write_all(&[OK]).await.is_err() {
                    return;
                }
                // OK admits a preparation, not a replacement carrier.
                // Only the chosen initiator publishes its transport and
                // writes a complete framed datagram. A status-only loser
                // can therefore finish late without replacing live traffic.
                let mut recv = recv;
                let Ok(Some(prefix)) =
                    tokio::time::timeout(HOP_WITHIN, stream_trace::first_frame(&mut recv)).await
                else {
                    return;
                };
                if !self.hop_on()
                    || self.carrying().is_none()
                    || !member(&asker)
                    || matches!(self.carry.carrier(&x), Some(Carrier::Ssh))
                    || (me < asker && asking(&me, &asker))
                {
                    return;
                }
                let attached = stream_trace::attach_prefixed(
                    &self.carry,
                    x,
                    Carrier::Hop { via: from_id },
                    incoming_wire,
                    "receiver",
                    (send, recv),
                    prefix,
                );
                khor_net::carry::observe_lifecycle(&asker, "hop_outer_bound", || {
                    serde_json::json!({
                        "role": "receiver", "attachment": attached.serial(),
                        "via": from, "outer_wire": incoming_wire,
                    })
                });
                if let Some(addr) = self.carried_addr(&asker) {
                    ep.offer_addrs(iroh::EndpointAddr::from_parts(x, [addr]))
                        .await;
                }
                self.wires.forgive(&asker);
                let _riding = self.wires.hop_held(from);
                let ended = attached.ended();
                if std::env::var_os("KHOR_NET_DIAGNOSTICS").is_some() {
                    tokio::pin!(ended);
                    let mut look = tokio::time::interval(LOOK_EVERY);
                    loop {
                        tokio::select! {
                            () = &mut ended => break,
                            _ = look.tick() => {
                                khor_net::carry::observe_lifecycle(&asker, "hop_outer_look", || {
                                    serde_json::json!({"role": "receiver", "outer": outer_facts(&incoming)})
                                });
                            }
                        }
                    }
                } else {
                    ended.await;
                }
                crate::carry::let_path_go(&self.weak(), &self.carry, x).await;
            }
        }
    }
}

/// Read an actual carrier connection only inside an enabled diagnostic closure.
/// A weak handle cannot keep an otherwise finished outer connection alive.
fn outer_facts(outer: &iroh::endpoint::WeakConnectionHandle) -> serde_json::Value {
    match outer.upgrade() {
        Some(conn) => serde_json::json!({
            "wire": conn.stable_id(), "closed": conn.close_reason().is_some(),
            "measurements": khor_net::path::path_facts(&conn),
            "transport": format!("{:?}", conn.stats()),
            "transport_paths": conn.paths().iter().map(|p| serde_json::json!({
                "addr": format!("{:?}", p.remote_addr()), "selected": p.is_selected(),
                "stats": format!("{:?}", p.stats()),
            })).collect::<Vec<_>>(),
        }),
        None => serde_json::json!({"dropped": true}),
    }
}

/// Copies bytes both ways between two tunnel streams until both
/// directions end, passing each half-close on. The middle never reads a
/// frame: what it copies is QUIC ciphertext between the two ends.
async fn splice(a: (SendStream, RecvStream), b: (SendStream, RecvStream)) {
    let ((mut a_send, mut a_recv), (mut b_send, mut b_recv)) = (a, b);
    let there = async {
        let _ = tokio::io::copy(&mut a_recv, &mut b_send).await;
        let _ = b_send.finish();
    };
    let back = async {
        let _ = tokio::io::copy(&mut b_recv, &mut a_send).await;
        let _ = a_send.finish();
    };
    // Either direction ending ends the hop: a link is both halves.
    tokio::select! {
        () = there => {}
        () = back => {}
    }
}

#[cfg(test)]
#[path = "hop/offer_tests.rs"]
mod tests_offer;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measured_path_eligibility_does_not_lower_new_middle_preparation_cost() {
        let current = Duration::from_millis(131);
        let candidate = Duration::from_millis(104);
        assert!(khor_net::carry::improves(candidate, current, false));
        assert!(!beats(104, 131));
    }

    /// An old watcher can finish a measured look after replacement. Its
    /// completed attachment never owns a later stream through the same middle.
    #[tokio::test]
    async fn a_finished_hop_watcher_cannot_release_its_live_replacement() {
        use khor_net::carry::Links;
        use khor_net::path::PathKind;

        let me = iroh::SecretKey::generate().public();
        let via = iroh::SecretKey::generate().public();
        // Positive controls: the current owner still releases a losing hop;
        // only an actual loss to the relay suppresses that middle afterward.
        for road in [PathKind::Relay, PathKind::Direct] {
            let peer = iroh::SecretKey::generate().public();
            let links = Links::new(me);
            let (here, _remote) = tokio::io::duplex(64);
            let (r, w) = tokio::io::split(here);
            let current = links.attach(peer, Carrier::Hop { via }, r, w);
            let serial = current.serial();
            assert!(links.has(&peer));
            release_hop(
                &links,
                &peer.to_string(),
                peer,
                via,
                serial,
                Some(road),
                2,
                CarrierComparison::Outperformed,
            );
            assert!(!links.has(&peer), "the current owner releases its own link");
            assert_eq!(
                was_lost(&peer.to_string(), &via.to_string()),
                road == PathKind::Relay
            );
            tokio::time::timeout(Duration::from_secs(1), current.ended())
                .await
                .expect("released current link ends");
        }

        let peer = iroh::SecretKey::generate().public();
        let links = Links::new(me);
        let (old_here, _old_remote) = tokio::io::duplex(64);
        let (old_r, old_w) = tokio::io::split(old_here);
        let old = links.attach(peer, Carrier::Hop { via }, old_r, old_w);
        let stale = old.serial();
        let (new_here, _new_remote) = tokio::io::duplex(64);
        let (new_r, new_w) = tokio::io::split(new_here);
        let replacement = links.attach(peer, Carrier::Hop { via }, new_r, new_w);
        let current = replacement.serial();
        assert_ne!(stale, current, "distinct attachment generations");
        tokio::time::timeout(Duration::from_secs(1), old.ended())
            .await
            .expect("the replaced stream has actually ended");
        let ended = replacement.ended();
        tokio::pin!(ended);
        assert!(
            tokio::time::timeout(Duration::from_millis(20), &mut ended)
                .await
                .is_err()
        );
        assert_eq!(links.carrier(&peer), Some(Carrier::Hop { via }));
        assert!(!was_lost(&peer.to_string(), &via.to_string()));

        release_hop(
            &links,
            &peer.to_string(),
            peer,
            via,
            stale,
            Some(PathKind::Relay),
            2,
            CarrierComparison::Outperformed,
        );
        let still_live = links.has(&peer);
        let wrongly_lost = was_lost(&peer.to_string(), &via.to_string());
        eprintln!(
            "[hop-owner-contract] old={stale} current={current} old_ended=true replacement_alive_before=true alive_after={still_live} lost_after={wrongly_lost}"
        );
        assert!(
            still_live && !wrongly_lost,
            "a stale watcher cannot release or suppress its replacement"
        );
    }

    /// A carrier can be unused after its paths disappear during a network
    /// move. Reclaim the owned stream, without calling that a measured loss.
    #[tokio::test]
    async fn an_unused_hop_is_released_without_suppressing_its_middle() {
        use khor_net::carry::Links;
        use khor_net::path::PathKind;

        let me = iroh::SecretKey::generate().public();
        let peer = iroh::SecretKey::generate().public();
        let via = iroh::SecretKey::generate().public();
        let links = Links::new(me);
        let (stream, _remote) = tokio::io::duplex(64);
        let (r, w) = tokio::io::split(stream);
        let current = links.attach(peer, Carrier::Hop { via }, r, w);
        let serial = current.serial();
        assert!(links.has(&peer));
        release_hop(
            &links,
            &peer.to_string(),
            peer,
            via,
            serial,
            Some(PathKind::Relay),
            2,
            CarrierComparison::Unused,
        );
        let suppressed = was_lost(&peer.to_string(), &via.to_string());
        assert!(!links.has(&peer), "an unused carrier is actually reclaimed");
        tokio::time::timeout(Duration::from_secs(1), current.ended())
            .await
            .expect("the reclaimed owned stream ends");
        eprintln!("[hop-unused-contract] serial={serial} released=true suppressed={suppressed}");
        assert!(
            !suppressed,
            "no carried path comparison is no evidence that its middle lost"
        );
    }

    fn middles(
        me: &str,
        to: &str,
        rows: &[(String, bool, bool)],
        mine: impl Fn(&str) -> Option<Road>,
        theirs: impl Fn(&str) -> Option<Road>,
    ) -> Vec<String> {
        middles_with_cost(me, to, rows, mine, theirs, Legs::Dial)
            .into_iter()
            .map(|(id, _)| id)
            .collect()
    }

    fn road(to: &str, k: RoadKind, ms: u16) -> Road {
        Road::own(to, k, None, Some(ms))
    }

    /// Both candidates are already eligible before a dial. A direct
    /// first leg must not put a 120 ms hop ahead of a 5 ms relay/SSH hop.
    #[test]
    fn cold_hop_candidates_rank_by_the_measured_two_leg_cost() {
        let rows: Vec<_> = ["unknown", "slow-direct", "fast-relay"]
            .into_iter()
            .map(|id| (id.to_owned(), false, false))
            .collect();
        let mine = |id: &str| match id {
            "unknown" => Some(Road::own(id, RoadKind::Direct, None, None)),
            "slow-direct" => Some(road(id, RoadKind::Direct, 100)),
            "fast-relay" => Some(road(id, RoadKind::Relay, 2)),
            _ => None,
        };
        let theirs = |id: &str| match id {
            "unknown" => Some(road("to", RoadKind::Ssh, 1)),
            "slow-direct" => Some(road("to", RoadKind::Ssh, 20)),
            "fast-relay" => Some(road("to", RoadKind::Ssh, 3)),
            _ => None,
        };
        for policy in [Legs::Measured, Legs::Dial] {
            assert_eq!(
                middles_with_cost("me", "to", &rows, mine, theirs, policy),
                vec![
                    ("fast-relay".to_owned(), 5),
                    ("slow-direct".to_owned(), 120),
                    ("unknown".to_owned(), u32::from(u16::MAX) + 1),
                ],
                "known two-leg cost is primary under {policy:?}"
            );
        }
    }

    /// **A hop that found no middle is not asked for before every dial**
    /// (net-dial B3): 30 s after the first miss, doubling to ten minutes,
    /// and asked again at once after a hop to it opened.
    #[test]
    fn a_hop_that_found_no_middle_waits_before_it_is_asked_again() {
        let waits: Vec<u64> = (1..=7).map(|n| missed_wait(n).as_secs()).collect();
        assert_eq!(waits, vec![30, 60, 120, 240, 480, 600, 600]);
        let (me, to) = ("missed-x", "missed-y");
        assert!(!missed_lately(me, to), "never missed: asked");
        missed(me, to);
        assert!(
            missed_lately(me, to),
            "missed just now: not asked before the next dial"
        );
        assert!(
            !missed_lately(me, "missed-z"),
            "another machine is its own question"
        );
        made(me, to);
        assert!(!missed_lately(me, to), "a hop to it opened: asked again");
    }

    /// The middle's leg is inside the asker's wait: the asker never gives
    /// up on a middle that is still dialling for it.
    #[test]
    fn the_middles_dial_ends_inside_the_askers_wait() {
        assert!(MIDDLE_WITHIN < HOP_WITHIN);
    }

    #[test]
    fn an_ask_is_never_a_host_and_port() {
        let id = iroh::SecretKey::generate().public();
        assert_eq!(Ask::of(&Ask::To(id).words()), Some(Ask::To(id)));
        assert_eq!(Ask::of(&Ask::From(id).words()), Some(Ask::From(id)));
        assert_eq!(Ask::of("example.com:22"), None);
        assert_eq!(Ask::of("hop:22"), None, "a host called hop");
        assert_eq!(Ask::of("hop nonsense"), None);
    }

    /// The roadbook rules: both legs known, neither a hop; lower total
    /// measured cost first; the relay as a first leg only to a middle that carries
    /// the far end over ssh.
    /// NET-DIRECT step 4, MacLab's night of 10-09: the iPhone on the relay
    /// at 58 ms, turing reached directly at 3 ms and writing a direct road
    /// to the iPhone at 26 ms — the hop costs 29 ms and is opened; at a
    /// relay of 35 ms it would save under 10 ms and is not.
    #[test]
    fn a_hop_is_worth_opening_when_the_selector_would_use_it() {
        let rows: Vec<(String, bool, bool)> = ["maclab", "iphone", "turing", "hinton"]
            .iter()
            .map(|s| (s.to_string(), false, false))
            .collect();
        let mine = |b: &str| match b {
            "turing" => Some(Road::own(b, RoadKind::Direct, None, Some(3))),
            "hinton" => Some(Road::own(b, RoadKind::Direct, None, Some(2))),
            _ => None,
        };
        let theirs = |b: &str| match b {
            "turing" => Some(Road::own("iphone", RoadKind::Direct, None, Some(26))),
            "hinton" => Some(Road::own(
                "iphone",
                RoadKind::Relay,
                Some("101.200.80.77:3340".into()),
                Some(37),
            )),
            _ => None,
        };
        let found = middles_with_cost("maclab", "iphone", &rows, mine, theirs, Legs::Dial);
        assert_eq!(
            found,
            vec![("turing".to_owned(), 29)],
            "hinton reaches the iPhone only by relay: no middle"
        );
        // Against the relay in use, hinton's relay leg is measured too:
        // 39 ms, behind turing.
        let found = middles_with_cost("maclab", "iphone", &rows, mine, theirs, Legs::Measured);
        assert_eq!(
            found,
            vec![("turing".to_owned(), 29), ("hinton".to_owned(), 39)]
        );
        assert!(beats(29, 58));
        assert!(!beats(29, 35), "6 ms is not worth a middle");
        assert!(beats(29, 39));
        assert!(beats(22, 30), "8 ms on 30 ms wins selection");
        assert!(!beats(450, 500), "50 ms does not pay new-middle cost");
    }

    /// **lecun's day of 10-20** (C7): its relays never answered; it
    /// reaches turing and hinton directly, and they reach localhost only
    /// by the Beijing relay. Before a dial nothing is a middle for it
    /// (the second legs are relays); with no relay of its own, the
    /// middle's relay is the one road there is — turing, nearer, first.
    #[test]
    fn with_no_relay_a_middle_may_reach_the_far_end_by_its_relay() {
        let rows = the_fleet();
        let mine = |b: &str| match b {
            "turing" => Some(Road::own(b, RoadKind::Direct, None, Some(2))),
            "hinton" => Some(Road::own(b, RoadKind::Direct, None, Some(3))),
            "maclab" => Some(Road::own(b, RoadKind::Relay, beijing(), Some(30))),
            _ => None,
        };
        let theirs = |b: &str| match b {
            "turing" => Some(Road::own("localhost", RoadKind::Relay, beijing(), Some(42))),
            "hinton" => Some(Road::own("localhost", RoadKind::Relay, beijing(), Some(60))),
            "maclab" => Some(Road::own("localhost", RoadKind::Relay, beijing(), Some(20))),
            _ => None,
        };
        assert!(
            middles_with_cost("lecun", "localhost", &rows, mine, theirs, Legs::Dial).is_empty()
        );
        let found = middles_with_cost("lecun", "localhost", &rows, mine, theirs, Legs::NoRelay);
        assert_eq!(
            found,
            vec![("turing".to_owned(), 44), ("hinton".to_owned(), 63)],
            "a relay first leg is no leg here"
        );
        // The other way: localhost reaches turing and maclab by the relay,
        // turing and hinton reach lecun directly.
        let mine = |b: &str| match b {
            "turing" => Some(Road::own(b, RoadKind::Relay, beijing(), Some(42))),
            "maclab" => Some(Road::own(b, RoadKind::Relay, beijing(), Some(20))),
            _ => None,
        };
        let theirs = |b: &str| match b {
            "turing" => Some(Road::own("lecun", RoadKind::Direct, None, Some(2))),
            "maclab" => Some(Road::own("lecun", RoadKind::Relay, beijing(), Some(20))),
            _ => None,
        };
        assert!(
            middles_with_cost("localhost", "lecun", &rows, mine, theirs, Legs::Dial).is_empty()
        );
        assert_eq!(
            middles_with_cost("localhost", "lecun", &rows, mine, theirs, Legs::ToNoRelay),
            vec![("turing".to_owned(), 44)],
            "to a machine with no relay, the middle's leg must be direct"
        );
    }

    /// The five machines of 10-20.
    fn the_fleet() -> Vec<(String, bool, bool)> {
        ["localhost", "maclab", "turing", "hinton", "lecun"]
            .iter()
            .map(|s| (s.to_string(), false, false))
            .collect()
    }

    fn beijing() -> Option<String> {
        Some("101.200.80.77:3340".to_owned())
    }

    /// hinton's day of 10-20: its v4 reaches nothing outside, so its home
    /// is n0's aps1 and localhost meets it there at 300–1000 ms
    /// (hinton's journal: 改走中继：aps1…，往返 374 ms). localhost reaches
    /// turing on the Beijing relay at 42 ms and turing reaches hinton
    /// directly at 2 ms: the hop costs 44 ms and is opened, from either
    /// end — a relay leg is no bar once the relay it would replace is
    /// measured.
    #[test]
    fn a_relay_first_leg_is_a_middle_when_the_relay_it_replaces_is_slower() {
        let rows = the_fleet();
        let mine = |b: &str| match b {
            "turing" => Some(Road::own(b, RoadKind::Relay, beijing(), Some(42))),
            _ => None,
        };
        let theirs = |b: &str| match b {
            "turing" => Some(Road::own("hinton", RoadKind::Direct, None, Some(2))),
            _ => None,
        };
        assert_eq!(
            worth_a_hop("localhost", "hinton", 400, &rows, mine, theirs, |_| false),
            vec!["turing"]
        );

        // Both legs on the Beijing relay still beat aps1; a forwarded
        // second leg never does (one hop only).
        let relayed = |b: &str| {
            (b == "turing").then(|| Road::own("hinton", RoadKind::Relay, beijing(), Some(40)))
        };
        assert_eq!(
            worth_a_hop("localhost", "hinton", 400, &rows, mine, relayed, |_| false),
            vec!["turing"]
        );
        let forwarded = |b: &str| {
            (b == "turing").then(|| Road {
                via: Some("maclab".into()),
                ..Road::own("hinton", RoadKind::Direct, None, Some(2))
            })
        };
        assert!(
            worth_a_hop("localhost", "hinton", 400, &rows, mine, forwarded, |_| {
                false
            })
            .is_empty()
        );
    }

    /// The same hop seen from hinton.
    #[test]
    fn a_relay_second_leg_is_a_middle_when_the_relay_it_replaces_is_slower() {
        let rows = the_fleet();
        let none = |_: &str| false;
        let mine = |b: &str| match b {
            "turing" => Some(Road::own(b, RoadKind::Direct, None, Some(2))),
            _ => None,
        };
        let theirs = |b: &str| match b {
            "turing" => Some(Road::own("localhost", RoadKind::Relay, beijing(), Some(42))),
            _ => None,
        };
        assert_eq!(
            worth_a_hop("hinton", "localhost", 400, &rows, mine, theirs, none),
            vec!["turing"]
        );
        assert!(
            worth_a_hop("hinton", "localhost", 50, &rows, mine, theirs, none).is_empty(),
            "6 ms is not worth a middle"
        );
        assert!(
            worth_a_hop("hinton", "localhost", 400, &rows, mine, theirs, |b| b
                == "turing")
            .is_empty(),
            "lost lately"
        );
        assert!(
            middles("hinton", "localhost", &rows, mine, theirs).is_empty(),
            "before a dial nothing is measured: no relay leg"
        );
    }

    /// The mark both ends read when they ask at once: held from the ask
    /// to the hop's end, per pair, counted.
    #[test]
    fn an_ask_is_marked_until_its_hop_ends() {
        assert!(!asking("ask-lo", "ask-hi"));
        let one = Asking::new("ask-lo", "ask-hi");
        let two = Asking::new("ask-lo", "ask-hi");
        assert!(asking("ask-lo", "ask-hi"));
        assert!(
            !asking("ask-hi", "ask-lo"),
            "the other end's ask is its own"
        );
        drop(one);
        assert!(asking("ask-lo", "ask-hi"), "two asks, one ended");
        drop(two);
        assert!(!asking("ask-lo", "ask-hi"));
    }

    #[test]
    fn a_middle_reaches_both_ends_and_is_one_hop() {
        let rows: Vec<(String, bool, bool)> = ["x", "y", "b", "c", "d", "sat", "off"]
            .iter()
            .map(|id| (id.to_string(), *id == "sat", *id == "off"))
            .collect();
        let mine = |b: &str| match b {
            "b" => Some(road(b, RoadKind::Direct, 30)),
            "c" => Some(road(b, RoadKind::Direct, 5)),
            "d" => Some(road(b, RoadKind::Relay, 80)),
            "sat" | "off" => Some(road(b, RoadKind::Direct, 1)),
            _ => None,
        };
        let theirs = |b: &str| match b {
            "b" => Some(road("y", RoadKind::Direct, 10)),
            "c" => Some(road("y", RoadKind::Direct, 50)),
            "d" => Some(road("y", RoadKind::Direct, 1)),
            _ => Some(road("y", RoadKind::Direct, 1)),
        };
        assert_eq!(
            middles("x", "y", &rows, mine, theirs),
            vec!["b", "c"],
            "40 ms before 55; d only by relay"
        );

        let carries = |b: &str| (b == "d").then(|| road("y", RoadKind::Ssh, 3));
        assert_eq!(
            middles("x", "y", &rows, mine, carries),
            vec!["d"],
            "the relay to the machine that carries y"
        );

        let forwarded = |b: &str| {
            Some(Road {
                via: Some("z".into()),
                ..road("y", RoadKind::Direct, 1)
            })
            .filter(|_| b == "b")
        };
        assert!(
            middles("x", "y", &rows, mine, forwarded).is_empty(),
            "a middle whose own road is a hop"
        );
        let relayed = |_: &str| Some(road("y", RoadKind::Relay, 1));
        assert!(
            middles("x", "y", &rows, mine, relayed).is_empty(),
            "a middle on the relay adds nothing"
        );
        assert!(
            middles("x", "y", &rows, |_| None, theirs).is_empty(),
            "no road of mine to it"
        );
    }
}
