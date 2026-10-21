//! The wires this machine holds to other machines (docs/NET.md 连接):
//! **one long-lived QUIC connection per far machine and protocol**,
//! opened on first need, kept warm by iroh's keepalive, watched for
//! silence, retired when it stops answering, and opened again with
//! backoff.
//!
//! # Why one long-lived wire
//!
//! iroh keeps the hole it punched only while some connection to that
//! machine is open: when the last one closes it forgets the selected
//! path, and the next connection sprays its first packet at every known
//! address, the relay included, and punches again (`vendor/iroh`,
//! `remote_state.rs`, `handle_connection_close` and
//! `handle_msg_send_datagram`). Before this module the sync pump dialled
//! every machine every five seconds and dropped the connection after
//! the round — measured 13–14 fresh dials a minute per peer — so the
//! punched path was thrown away twelve times a minute and the row's word
//! followed the dial cycle (audit/net.md F1–F3).
//!
//! # Lifecycle
//!
//! - **Owner**: the node's `Wires`, keyed by (device id, protocol). In
//!   the resident it lives as long as the endpoint; a one-shot verb's
//!   node holds its wires for the one breath it lives.
//! - **Opened** by whoever needs it first ([`crate::Node::line_to`]),
//!   **one dial at a time per key**: a second asker waits for the first
//!   dial rather than racing it, and an asker for another machine never
//!   waits at all.
//! - **Kept** with iroh's own keepalive (a PING every 5 s per path), so
//!   the NAT mapping and the selected path stay warm while idle.
//! - **Heard**: every datagram the far side sends on it — answers, and
//!   the acks of those keepalives — moves `heard`. A healthy wire is
//!   therefore heard every few seconds without any traffic of ours.
//! - **Retired** (closed, forgotten) when: nothing has been heard for
//!   its silence limit; a request was sent and nothing at all came back
//!   within its ack limit; QUIC closed it; the network changed and it
//!   stayed quiet past its limit after the change; or the endpoint under
//!   it was replaced. **Every limit scales with that wire's own round
//!   trip** ([`Patience`]): a slow road that answers is never a dead one. **In-flight requests on a retired wire fail
//!   at once** with the connection's close — a reading is asked again
//!   on a fresh wire, anything else reports that the machine was not
//!   reached and lets the person decide.
//! - **Backoff**: a dial that fails makes the pump wait 5 s, then 10,
//!   20, 40, and 60 s at most before it dials that machine again. A
//!   person's own request is never refused by backoff — only the
//!   background pump honours it.
//!
//! Each side dials its own wire: a connection somebody else opened to
//! this machine is answered, never borrowed for this machine's own
//! questions — the far side may be a one-shot verb with nobody serving
//! its end. Both wires ride the one path state iroh keeps per machine.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use iroh::endpoint::Connection;
use khor_net::path::PathKind;

/// **How long to wait for a wire, measured from its own round trip.**
///
/// ⚠️ 墓碑 (round NET-2): these were constants — 4 s for a request's
/// first ack, 15 s of silence, 3 s after a network change — chosen on
/// loopback. Measured through a relay with 3 s of added round trip and
/// 2 % stalls (`crates/cli/tests/slow_relay.rs`), they retired four live
/// wires a minute and every listing failed; the owner's Mac measured its
/// Hong Kong relay at 2040 ms, which is the same regime.
///
/// Now each limit is a floor (the old constant, still right for a fast
/// path) or a multiple of the wire's smoothed RTT, whichever is longer,
/// under a ceiling. The multiple follows QUIC's own loss detection (RFC
/// 9002 §6.2): a probe timeout is `smoothed_rtt + max(4·rttvar, 1 ms) +
/// max_ack_delay`, about three RTTs when the variance is half the RTT,
/// and QUIC declares persistent congestion only after three of those —
/// so nine RTTs of hearing nothing is the point where QUIC itself would
/// give up on the path, and anything sooner is khor being less patient
/// than its transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Patience {
    /// A request sent and nothing at all heard back.
    pub(crate) ack: Duration,
    /// Nothing heard on a wire, request or not.
    pub(crate) silent: Duration,
    /// After a network change or a wake, how long a wire has to be heard
    /// before it is presumed to have died with the old network (iroh
    /// pings every path on a change).
    pub(crate) after_change: Duration,
    /// Long enough that a wire not heard for this long missed one of
    /// iroh's 5 s keepalives.
    pub(crate) keepalive_missed: Duration,
}

/// RTTs of hearing nothing before a wire is judged dead (three probe
/// timeouts; see [`Patience`]).
const RTTS: u32 = 9;

/// **A wire that carries a hop gives up sooner**: 10 s of silence
/// instead of 15 (the lead, 2026-10-09). A middle that freezes holds the
/// two ends on a road that carries nothing until the wire under it is
/// judged dead, and only then does iroh take the relay back; the
/// three-node hop test (`crates/cli/tests/hop.rs`) measured 20 s from a
/// frozen middle to the relay at 15 s. Every other wire keeps 15: it has
/// a relay of its own under it and nothing else waits on its verdict.
const HOP_SILENT: Duration = Duration::from_secs(10);

impl Patience {
    /// For a wire one or more hops ride ([`Wires::hop_held`]): the
    /// silence floor is [`HOP_SILENT`], the rest as [`Self::for_rtt`].
    pub(crate) fn for_hop_rtt(rtt: Duration) -> Patience {
        let k = rtt.saturating_mul(RTTS);
        Patience {
            silent: (HOP_SILENT + k).min(Duration::from_secs(90)),
            ..Patience::for_rtt(rtt)
        }
    }

    pub(crate) fn for_rtt(rtt: Duration) -> Patience {
        let k = rtt.saturating_mul(RTTS);
        Patience {
            ack: Duration::from_secs(4).max(k).min(Duration::from_secs(60)),
            silent: (Duration::from_secs(15) + k).min(Duration::from_secs(90)),
            after_change: Duration::from_secs(3).max(k).min(Duration::from_secs(30)),
            keepalive_missed: (Duration::from_secs(6) + k).min(Duration::from_secs(60)),
        }
    }

    /// For this connection, now.
    pub(crate) fn of(conn: &Connection) -> Patience {
        Patience::for_rtt(rtt_of(conn))
    }
}

/// The round trip QUIC has smoothed for the path carrying this wire's
/// bytes (`PathStats::rtt` via `Connection::paths`): the selected path's,
/// or the longest of the open ones while none is selected. Zero when the
/// connection has no path to say — the floors then apply.
pub(crate) fn rtt_of(conn: &Connection) -> Duration {
    let paths = conn.paths();
    let mut longest = Duration::ZERO;
    for p in paths.iter() {
        if p.is_selected() {
            return p.rtt();
        }
        longest = longest.max(p.rtt());
    }
    longest
}

/// **Asks every path of `conn` whether it is there** (vendor/iroh patch
/// 21: a PING on each), before a wire is judged dead; answers how long to
/// give the answers: three of the slowest open path's round trips, at
/// least a second.
///
/// A wire is judged on the connection's datagrams, every path's together,
/// but a path that carries nothing is heard only at its keepalive (5 s, a
/// spare 15 s). So when the path in use went black, the others were quiet
/// by design, not dead, and the wire was retired at its ack limit (4 s)
/// while iroh still had a road: 2026-10-16 23:13:42, turing judged its wire
/// to the Mac dead and the Mac's wire to it had been heard 367 ms before.
/// An answer on any path moves the count, and iroh's selector leaves the
/// path that stopped answering (patch 17).
pub(crate) fn ask_every_path(conn: &Connection) -> Duration {
    let mut slowest = Duration::ZERO;
    for p in conn.paths().iter() {
        if p.ping() {
            slowest = slowest.max(p.rtt());
        }
    }
    Duration::from_secs(1).max(slowest.saturating_mul(3)).min(Duration::from_secs(30))
}

/// How long a far session's open is given on top of the request's own
/// budget for the road it travels: the same nine RTTs.
pub(crate) fn budget_for(base: Duration, conn: &Connection) -> Duration {
    base + rtt_of(conn)
        .saturating_mul(RTTS)
        .min(Duration::from_secs(120))
}

/// The longest any wire waits after a change before its verdict.
const CHANGE_JUDGED_WITHIN: Duration = Duration::from_secs(30);

/// How often the resident looks at its wires: liveness and road.
pub(crate) const TICK: Duration = Duration::from_secs(1);

/// Samples a new road must hold before the word changes. A path being
/// re-chosen passes through a moment with none selected or the other one
/// selected; one sample of that is not a change of road.
const SETTLE: u8 = 2;

/// The pump's wait after a failed dial, doubling per failure.
///
/// **Up to five minutes** (round NET-6; it was one). A failed dial
/// through a relay costs about 9 KB (the diagnosis probe `idle_cost.rs`:
/// four of them in a minute were 35 KB towards the relay), so a machine
/// that is off cost 9 KB a minute for as long as it stayed off. One that
/// comes back dials this one on its own first round, a person's own
/// request dials at once whatever the wait, and a change of this
/// machine's network forgives every wait (`Wires::network_changed`) —
/// so the longer wait is only ever the pump's.
const BACKOFF_FIRST: Duration = Duration::from_secs(5);
const BACKOFF_MAX: Duration = Duration::from_secs(300);

/// Which protocol a wire speaks: requests, or borrow pipes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Proto {
    Main,
    Tunnel,
}

impl Proto {
    pub(crate) fn alpn(self) -> &'static [u8] {
        match self {
            Proto::Main => khor_net::endpoint::ALPN,
            Proto::Tunnel => khor_net::endpoint::TUNNEL_ALPN,
        }
    }
}

/// Why a wire was retired — what the counters say and what the far side
/// reads as the close reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Retired {
    Silent,
    Unanswered,
    Closed,
    Network,
    NewEars,
    Paused,
    Suspended,
    /// The relay said the far machine left it (`Wires::departed`).
    Departed,
    /// The far machine closed it going to the background or pausing
    /// (`suspended`, `paused`): it is resting, not dead (net-dial B4).
    Asleep,
}

impl Retired {
    /// The journal's key for it (`journal::What::Retired`).
    fn key(self) -> &'static str {
        match self {
            Retired::Silent => "silent",
            Retired::Unanswered => "unanswered",
            Retired::Closed => "closed",
            Retired::Network => "network",
            Retired::NewEars => "replaced",
            Retired::Paused => "paused",
            Retired::Suspended => "suspended",
            Retired::Departed => "departed",
            Retired::Asleep => "asleep",
        }
    }

    fn reason(self) -> &'static [u8] {
        match self {
            Retired::Silent => b"silent",
            Retired::Unanswered => b"unanswered",
            Retired::Closed => b"closed",
            Retired::Network => b"network changed",
            Retired::NewEars => b"new endpoint",
            Retired::Paused => b"paused",
            Retired::Suspended => b"suspended",
            Retired::Departed => b"departed",
            Retired::Asleep => b"closed",
        }
    }
}

/// **Whether a connection's end says the far machine is resting** (net-dial
/// B4): it closed the connection going to the background or pausing (the
/// reasons [`Retired::Suspended`] and [`Retired::Paused`] put on the wire),
/// or it refused a new one (a dropped noq `Incoming` is CONNECTION_REFUSED,
/// what a paused or backgrounded machine does with every dial). Either way
/// it will dial when it is back; dialling it meanwhile only costs the
/// relay.
pub(crate) fn resting_close(e: &iroh::endpoint::ConnectionError) -> bool {
    use iroh::endpoint::ConnectionError;
    match e {
        ConnectionError::ApplicationClosed(c) => {
            c.reason.as_ref() == Retired::Suspended.reason() || c.reason.as_ref() == Retired::Paused.reason()
        }
        ConnectionError::ConnectionClosed(c) => c.error_code == iroh::endpoint::TransportErrorCode::CONNECTION_REFUSED,
        _ => false,
    }
}

#[derive(Default)]
struct Slot {
    conn: Option<Connection>,
    /// Datagrams received on `conn` as last counted.
    rx: u64,
    /// When the far side was last heard on any wire this slot held.
    /// Outlives the wire, so a retired machine's age keeps growing.
    heard: Option<Instant>,
    /// The settled road, and a different one being considered.
    word: Option<PathKind>,
    /// When `word` became what it is (UX-48: a machine row says how long
    /// its direct or relay road has held).
    word_at: Option<Instant>,
    /// The last road this slot settled on, kept past the wire: an idle
    /// machine's row says which road it last used and how long ago that
    /// road settled (`word_at`). `None` until a wire first settles.
    last_word: Option<PathKind>,
    leaning: Option<(PathKind, u8)>,
    failures: u32,
    retry_at: Option<Instant>,
    /// Since this resident started: dials that failed, and wires retired
    /// unanswered — the per-machine half of [`Counts`].
    failed_total: u64,
    unanswered_total: u64,
    /// The last round trip measured on this key's wire, kept after the
    /// wire goes: the next dial to that machine is given time for it.
    rtt: Duration,
    /// The last thing that went wrong with this key, in words, and when.
    last_failure: Option<(String, Instant)>,
    /// UDP bytes sent and received on the wires this slot held before
    /// the one it holds now (round NET-3: what a sync round costs).
    bytes_past: (u64, u64),
    /// When the wire held now was adopted, for the journal's account of
    /// how long it lived.
    since: Option<Instant>,
}

/// UDP bytes a wire has sent and received, as QUIC counts them —
/// handshake, acks and keepalives included, so what the road carried.
fn bytes_of(conn: &Connection) -> (u64, u64) {
    let s = conn.stats();
    (s.udp_tx.bytes, s.udp_rx.bytes)
}

/// Every wire this node holds, and the counters behind `khor doctor`'s
/// account of them.
#[derive(Default)]
pub(crate) struct Wires {
    slots: Mutex<HashMap<(String, Proto), Slot>>,
    dialing: Mutex<HashMap<(String, Proto), DialLock>>,
    /// A network change not yet judged: wires not heard since it are
    /// retired once their [`Patience::after_change`] has passed.
    changed: Mutex<Option<Instant>>,
    /// The connections other machines opened to this one, weakly, and
    /// when each was last heard ([`Self::serving`]).
    served: Mutex<Vec<Served>>,
    /// Machines a relay said left it, and when (`Wires::departed`):
    /// until one is heard from again, a dial to it is given a short time.
    gone: Mutex<HashMap<String, Instant>>,
    /// **Machines unreachable now, and when the attempt that said so
    /// failed** (round NET-13): set by a dial that failed
    /// ([`Self::failed`]) and by a request unanswered on its wire
    /// ([`Self::retire`] with [`Retired::Unanswered`]) — the attempts the
    /// pump and a person's ask already make, no probe of its own —
    /// and cleared by anything heard from that machine. A wire that
    /// closes for any other reason (silent, closed, a network change, a
    /// relay's word that it left, let go on purpose) sets nothing: no
    /// attempt failed, so the machine is idle, not unreachable.
    down: Mutex<HashMap<String, Instant>>,
    /// Machines whose wire a hop rides, and how many ([`Self::hop_held`]):
    /// those wires are judged on [`Patience::for_hop_rtt`].
    hops: Mutex<HashMap<String, usize>>,
    pub(crate) counts: Counts,
    /// The connection journal, set by the resident only
    /// (`crate::journal`: one writer per home).
    pub(crate) journal: std::sync::OnceLock<Arc<crate::journal::Journal>>,
}

/// How long a relay's word that a machine left counts, unheard since.
const GONE_FOR: Duration = Duration::from_secs(120);

/// How long the pump leaves a machine its relay said left, unheard
/// since (round NET-6): it is not dialled at all, because it will dial —
/// a resident rounds with everyone the moment it starts or resumes, and
/// that round is what clears the word. Before, a phone whose app was
/// closed was dialled on every beat its backoff allowed, about 9 KB
/// through the relay each time: the term lane measured 25–34 KB a minute
/// from a desk with nobody attached. Ten minutes caps the trust in one
/// word, for a machine whose own first round never arrived.
const LEFT_ALONE_FOR: Duration = Duration::from_secs(600);

/// The lock that makes one key's dials one at a time.
type DialLock = Arc<tokio::sync::Mutex<()>>;

/// A connection another machine opened to this one.
struct Served {
    conn: iroh::endpoint::WeakConnectionHandle,
    id: String,
    rx: u64,
    heard: Instant,
    rtt: Duration,
}

/// A hop riding a machine's wire ([`Wires::hop_held`]); dropping it
/// lets the wire go back to the ordinary patience.
pub(crate) struct HopHeld {
    wires: Arc<Wires>,
    id: String,
}

impl Drop for HopHeld {
    fn drop(&mut self) {
        let mut hops = self.wires.hops.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(n) = hops.get_mut(&self.id) {
            *n -= 1;
            if *n == 0 {
                hops.remove(&self.id);
            }
        }
    }
}

/// The counters, as atomics. [`Counts::snapshot`] is what crosses ipc.
#[derive(Default)]
pub(crate) struct Counts {
    pub(crate) dialed: AtomicU64,
    pub(crate) dial_failed: AtomicU64,
    pub(crate) retired_silent: AtomicU64,
    pub(crate) retired_unanswered: AtomicU64,
    pub(crate) retired_closed: AtomicU64,
    pub(crate) retired_network: AtomicU64,
    pub(crate) retired_new_ears: AtomicU64,
    pub(crate) answers_late: AtomicU64,
    pub(crate) reads_retried: AtomicU64,
    pub(crate) read_direct: AtomicU64,
    pub(crate) read_relay: AtomicU64,
    pub(crate) read_idle: AtomicU64,
    pub(crate) read_down: AtomicU64,
    pub(crate) network_changes: AtomicU64,
    pub(crate) ops_late: AtomicU64,
    pub(crate) pump_rounds: AtomicU64,
    pub(crate) punch_rounds: AtomicU64,
}

impl Counts {
    pub(crate) fn snapshot(&self) -> khor_core::WireCounts {
        khor_core::WireCounts {
            dialed: self.dialed.load(Relaxed),
            dial_failed: self.dial_failed.load(Relaxed),
            retired_silent: self.retired_silent.load(Relaxed),
            retired_unanswered: self.retired_unanswered.load(Relaxed),
            retired_closed: self.retired_closed.load(Relaxed),
            retired_network: self.retired_network.load(Relaxed),
            retired_new_ears: self.retired_new_ears.load(Relaxed),
            answers_late: self.answers_late.load(Relaxed),
            reads_retried: self.reads_retried.load(Relaxed),
            read_direct: self.read_direct.load(Relaxed),
            read_relay: self.read_relay.load(Relaxed),
            read_idle: self.read_idle.load(Relaxed),
            network_changes: self.network_changes.load(Relaxed),
            ops_late: self.ops_late.load(Relaxed),
            by_peer: Vec::new(),
            sent_bytes: 0,
            received_bytes: 0,
            rounds: self.pump_rounds.load(Relaxed),
            read_down: self.read_down.load(Relaxed),
            punch_rounds: self.punch_rounds.load(Relaxed),
        }
    }

    fn retired(&self, why: Retired) {
        match why {
            Retired::Silent => &self.retired_silent,
            Retired::Unanswered => &self.retired_unanswered,
            // Gone from the relay is the far side gone: closed, from there.
            Retired::Closed | Retired::Departed | Retired::Asleep => &self.retired_closed,
            Retired::Network => &self.retired_network,
            // A pause and a suspension are asked for, not failures; they
            // count with the endpoint's own replacement as 「let go on
            // purpose」.
            Retired::NewEars | Retired::Paused | Retired::Suspended => &self.retired_new_ears,
        }
        .fetch_add(1, Relaxed);
    }
}

/// Datagrams the far side has sent on this connection, ever. Moves with
/// answers and with the acks of iroh's keepalives alike.
pub(crate) fn heard_count(conn: &Connection) -> u64 {
    conn.stats().udp_rx.datagrams
}

/// The road a live connection is on now. `Unknown` while no path is
/// selected — a moment, mid-upgrade or right after the handshake — and
/// the settling below never takes that moment for a road.
fn road_now(conn: &Connection) -> PathKind {
    khor_net::path::classify(conn)
}

fn backoff(failures: u32) -> Duration {
    let doubled = BACKOFF_FIRST.saturating_mul(1u32 << failures.saturating_sub(1).min(8));
    doubled.min(BACKOFF_MAX)
}

/// One sample of the road: the first one settles at once, a different
/// one only after [`SETTLE`] in a row.
fn settle(slot: &mut Slot, kind: PathKind) {
    if kind == PathKind::Unknown {
        return;
    }
    match slot.word {
        None => {
            slot.word = Some(kind);
            slot.last_word = Some(kind);
            slot.word_at = Some(Instant::now());
            slot.leaning = None;
        }
        Some(w) if w == kind => slot.leaning = None,
        Some(_) => {
            let n = match slot.leaning {
                Some((k, n)) if k == kind => n + 1,
                _ => 1,
            };
            if n >= SETTLE {
                slot.word = Some(kind);
                slot.last_word = Some(kind);
                slot.word_at = Some(Instant::now());
                slot.leaning = None;
            } else {
                slot.leaning = Some((kind, n));
            }
        }
    }
}

/// What [`Wires::link`] knows about one machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Reading {
    /// The settled road of a live wire; `None` when none is live.
    pub(crate) road: Option<PathKind>,
    pub(crate) heard: Option<Duration>,
    /// How long since `last_road` settled: the live wire's road, or,
    /// with none live, the road the last wire was on.
    pub(crate) since: Option<Duration>,
    /// The road the last settled wire was on, live or gone; `None` when
    /// no wire to it ever settled since the resident started.
    pub(crate) last_road: Option<PathKind>,
    /// The live wire's smoothed round trip ([`rtt_of`]).
    pub(crate) rtt: Option<Duration>,
    /// The relay the live wire goes through, by name; `None` when direct.
    pub(crate) relay: Option<String>,
    /// Without a live wire: how long since the attempt that found the
    /// machine unreachable ([`Wires::down`]); `None` when none did.
    pub(crate) down: Option<Duration>,
    /// The last round trip measured on any wire to it, live or gone.
    pub(crate) last_rtt: Option<Duration>,
}

impl Wires {
    /// One line in the connection journal, when this node keeps one.
    pub(crate) fn note(&self, peer: Option<&str>, what: crate::journal::What) {
        if let Some(j) = self.journal.get() {
            j.note(peer, what);
        }
    }

    /// Takes and lets go of every lock these wires keep — the serve
    /// loop's beat (`crate::reactor::beat`) comes after it, so a lock
    /// somebody never let go of stops the beat (net-dial B1).
    pub(crate) fn pulse(&self) {
        drop(self.slots.lock().unwrap_or_else(|p| p.into_inner()));
        drop(self.dialing.lock().unwrap_or_else(|p| p.into_inner()));
        drop(self.changed.lock().unwrap_or_else(|p| p.into_inner()));
        drop(self.served.lock().unwrap_or_else(|p| p.into_inner()));
        drop(self.gone.lock().unwrap_or_else(|p| p.into_inner()));
        drop(self.down.lock().unwrap_or_else(|p| p.into_inner()));
        drop(self.hops.lock().unwrap_or_else(|p| p.into_inner()));
    }

    /// The failure count and the pump's wait that one more failed dial
    /// to this key would leave ([`Self::failed`]) — what the journal
    /// writes beside the dial that is about to be counted.
    pub(crate) fn next_backoff(&self, id: &str, proto: Proto) -> (u32, Duration) {
        let slots = self.slots.lock().unwrap_or_else(|p| p.into_inner());
        let n = slots
            .get(&(id.to_owned(), proto))
            .map_or(0, |s| s.failures)
            .saturating_add(1);
        drop(slots);
        (n, self.wait_after(id, n))
    }

    /// The pump's wait after the `n`th failed dial in a row to `id`:
    /// [`backoff`], **or the longest wait for a machine its relay said
    /// left, or that said it was resting, and that has not been heard
    /// since** (net-dial B4). Its ten minutes alone ([`LEFT_ALONE_FOR`])
    /// already were the wait; starting again from five seconds when they
    /// ran out redialled a sleeping phone six times in its first minute.
    fn wait_after(&self, id: &str, n: u32) -> Duration {
        if self.gone.lock().unwrap_or_else(|p| p.into_inner()).contains_key(id) {
            return BACKOFF_MAX;
        }
        backoff(n)
    }

    /// [`drop_wire`], written in the journal: why, how long the wire had
    /// been quiet, how long it lived.
    fn drop_noted(&self, id: &str, proto: Proto, slot: &mut Slot, why: Retired) {
        if slot.conn.is_some() {
            let now = Instant::now();
            let since = |t: Option<Instant>| t.map_or(0, |t| crate::journal::ms(now.saturating_duration_since(t)));
            self.note(
                Some(id),
                crate::journal::What::Retired {
                    why: why.key().to_owned(),
                    tunnel: proto == Proto::Tunnel,
                    quiet_ms: since(slot.heard),
                    age_ms: since(slot.since),
                },
            );
        }
        drop_wire(slot, why);
    }

    /// The live wire for this key, if there is one.
    pub(crate) fn live(&self, id: &str, proto: Proto) -> Option<Connection> {
        let slots = self.slots.lock().unwrap_or_else(|p| p.into_inner());
        slots
            .get(&(id.to_owned(), proto))
            .and_then(|s| s.conn.clone())
            .filter(|c| c.close_reason().is_none())
    }

    /// The lock that makes a key's dials one at a time.
    pub(crate) fn dial_lock(&self, id: &str, proto: Proto) -> Arc<tokio::sync::Mutex<()>> {
        self.dialing
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .entry((id.to_owned(), proto))
            .or_default()
            .clone()
    }

    /// A freshly dialled wire becomes the one this key holds.
    pub(crate) fn adopt(&self, id: &str, proto: Proto, conn: Connection) {
        self.back(id);
        let now = Instant::now();
        let mut slots = self.slots.lock().unwrap_or_else(|p| p.into_inner());
        let slot = slots.entry((id.to_owned(), proto)).or_default();
        if let Some(old) = slot.conn.take()
            && old.stable_id() != conn.stable_id()
        {
            let (tx, rx) = bytes_of(&old);
            slot.bytes_past.0 += tx;
            slot.bytes_past.1 += rx;
            old.close(0u32.into(), Retired::NewEars.reason());
        }
        slot.rx = heard_count(&conn);
        slot.heard = Some(now);
        let rtt = rtt_of(&conn);
        if !rtt.is_zero() {
            slot.rtt = rtt;
        }
        slot.word = None;
        slot.leaning = None;
        settle(slot, road_now(&conn));
        slot.failures = 0;
        slot.retry_at = None;
        slot.since = Some(now);
        slot.conn = Some(conn);
        self.counts.dialed.fetch_add(1, Relaxed);
    }

    /// A dial for this key failed: the pump waits before the next one.
    pub(crate) fn failed(&self, id: &str, proto: Proto, why: String) {
        let (_, wait) = self.next_backoff(id, proto);
        let mut slots = self.slots.lock().unwrap_or_else(|p| p.into_inner());
        let slot = slots.entry((id.to_owned(), proto)).or_default();
        slot.last_failure = Some((why, Instant::now()));
        slot.failures = slot.failures.saturating_add(1);
        slot.failed_total += 1;
        slot.retry_at = Some(Instant::now() + wait);
        self.counts.dial_failed.fetch_add(1, Relaxed);
        drop(slots);
        self.went_down(id);
        // A connection that machine opened here and that has missed a
        // keepalive may be what steered this dial wrong (`Self::serving`):
        // it goes, and the next dial sprays every road again.
        let now = Instant::now();
        let mut served = self.served.lock().unwrap_or_else(|p| p.into_inner());
        served.retain(|w| {
            if w.id != id
                || now.saturating_duration_since(w.heard)
                    < Patience::for_rtt(w.rtt).keepalive_missed
            {
                return true;
            }
            if let Some(conn) = w.conn.upgrade() {
                conn.close(0u32.into(), Retired::Silent.reason());
            }
            false
        });
    }

    /// How long the pump should still leave this machine alone; `None`
    /// when it may dial now (or holds a live wire already).
    pub(crate) fn backing_off(&self, id: &str) -> Option<Duration> {
        if self.live(id, Proto::Main).is_some() {
            return None;
        }
        let slots = self.slots.lock().unwrap_or_else(|p| p.into_inner());
        let at = slots.get(&(id.to_owned(), Proto::Main))?.retry_at?;
        at.checked_duration_since(Instant::now())
    }

    /// **Whether this machine has its own reason to think `id` is out of
    /// reach now** (net-dial B3, a middle asked for a hop to it): a dial to
    /// it failed and its wait has not run out, on either protocol; its
    /// relay said it left; or an attempt found it unreachable and nothing
    /// has been heard from it since.
    pub(crate) fn out_of_reach(&self, id: &str) -> bool {
        let now = Instant::now();
        let waiting = {
            let slots = self.slots.lock().unwrap_or_else(|p| p.into_inner());
            [Proto::Main, Proto::Tunnel].iter().any(|p| {
                slots
                    .get(&(id.to_owned(), *p))
                    .and_then(|s| s.retry_at)
                    .is_some_and(|at| at > now)
            })
        };
        waiting
            || self.gone_lately(id)
            || self.down.lock().unwrap_or_else(|p| p.into_inner()).contains_key(id)
    }

    /// The far side was heard just now (an answer came back).
    pub(crate) fn heard(&self, id: &str, proto: Proto, conn: &Connection) {
        let mut slots = self.slots.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(slot) = slots.get_mut(&(id.to_owned(), proto))
            && slot
                .conn
                .as_ref()
                .is_some_and(|c| c.stable_id() == conn.stable_id())
        {
            slot.rx = heard_count(conn);
            slot.heard = Some(Instant::now());
        }
        drop(slots);
        self.back(id);
    }

    /// Retires `conn` if it is still the wire this key holds: closed,
    /// forgotten, counted. Answers whether it was.
    pub(crate) fn retire(&self, id: &str, proto: Proto, conn: &Connection, why: Retired) -> bool {
        let mut slots = self.slots.lock().unwrap_or_else(|p| p.into_inner());
        let Some(slot) = slots.get_mut(&(id.to_owned(), proto)) else {
            return false;
        };
        if slot
            .conn
            .as_ref()
            .is_none_or(|c| c.stable_id() != conn.stable_id())
        {
            return false;
        }
        self.drop_noted(id, proto, slot, why);
        if why == Retired::Unanswered {
            slot.unanswered_total += 1;
            slot.last_failure = Some((khor_catalog::msg::wire_unanswered().to_owned(), Instant::now()));
        }
        self.counts.retired(why);
        drop(slots);
        if why == Retired::Unanswered {
            self.went_down(id);
        }
        if why == Retired::Asleep {
            self.asleep(id);
        }
        if matches!(why, Retired::Unanswered | Retired::Silent) {
            self.let_go_served_by(id, why);
        }
        true
    }

    /// Retires this key's wire if it has missed a keepalive — what a face
    /// asking to attach again (a pipe that ended, a manual reload) does
    /// first, so the new attachment goes out on a fresh wire rather than
    /// waiting out its ack limit on a dead one. A wire heard within the
    /// keepalive interval is kept: it is alive, and redialling it would
    /// only cost a handshake.
    pub(crate) fn retire_if_quiet(&self, id: &str, proto: Proto) -> bool {
        let quiet = {
            let slots = self.slots.lock().unwrap_or_else(|p| p.into_inner());
            match slots.get(&(id.to_owned(), proto)) {
                Some(slot) if slot.conn.is_some() => slot
                    .heard
                    .is_none_or(|h| h.elapsed() >= Patience::for_rtt(slot.rtt).keepalive_missed)
                    .then(|| slot.conn.clone())
                    .flatten(),
                _ => None,
            }
        };
        match quiet {
            Some(conn) => self.retire(id, proto, &conn, Retired::Silent),
            None => false,
        }
    }

    /// The last round trip measured to this machine on any wire, for the
    /// next dial's budget. Zero when none was ever measured.
    pub(crate) fn last_rtt(&self, id: &str) -> Duration {
        let slots = self.slots.lock().unwrap_or_else(|p| p.into_inner());
        [Proto::Main, Proto::Tunnel]
            .iter()
            .filter_map(|p| slots.get(&(id.to_owned(), *p)))
            .map(|s| s.rtt)
            .max()
            .unwrap_or_default()
    }

    /// Lets go of every wire to one machine (a pause bites on them).
    pub(crate) fn retire_peer(&self, id: &str, why: Retired) {
        let mut slots = self.slots.lock().unwrap_or_else(|p| p.into_inner());
        for proto in [Proto::Main, Proto::Tunnel] {
            if let Some(slot) = slots.get_mut(&(id.to_owned(), proto))
                && slot.conn.is_some()
            {
                self.drop_noted(id, proto, slot, why);
                self.counts.retired(why);
            }
        }
    }

    /// Lets go of every wire: the endpoint under them is being replaced,
    /// or the app is going to the background.
    /// Retires this machine's wires, both protocols, on request — the
    /// next ask dials afresh. What `ipc::Op::LetGo` does: a diagnostic,
    /// and the measurement of what a redial costs (round NET-3).
    pub(crate) fn let_go(&self, id: &str) {
        let mut slots = self.slots.lock().unwrap_or_else(|p| p.into_inner());
        for proto in [Proto::Main, Proto::Tunnel] {
            if let Some(slot) = slots.get_mut(&(id.to_owned(), proto))
                && slot.conn.is_some()
            {
                self.drop_noted(id, proto, slot, Retired::NewEars);
                self.counts.retired(Retired::NewEars);
            }
        }
    }

    pub(crate) fn clear(&self, why: Retired) {
        let mut slots = self.slots.lock().unwrap_or_else(|p| p.into_inner());
        for ((id, proto), slot) in slots.iter_mut() {
            if slot.conn.is_some() {
                self.drop_noted(id, *proto, slot, why);
                self.counts.retired(why);
            }
        }
    }

    /// A connection another machine opened to this one, being answered.
    ///
    /// **Watched for silence like this machine's own wires**, and for a
    /// reason that is not tidiness: iroh keeps one path state per far
    /// machine, shared by every connection to it, and while any of them
    /// is open a new dial sends its first packet **only** down the path
    /// that one selected (`remote_state.rs`, `handle_msg_send_datagram`).
    /// A far machine that restarted leaves its old connection to this one
    /// open here until QUIC's own idle timeout, and every dial back to it
    /// meanwhile goes to the dead process's port and times out — measured
    /// in this file's own test, a second write after a restart failing
    /// after a full 10 s dial.
    pub(crate) fn serving(&self, conn: &Connection) {
        self.back(&conn.remote_id().to_string());
        self.note(Some(&conn.remote_id().to_string()), crate::journal::What::Inbound);
        let mut served = self.served.lock().unwrap_or_else(|p| p.into_inner());
        served.retain(|w| w.conn.upgrade().is_some_and(|c| c.close_reason().is_none()));
        served.push(Served {
            conn: conn.weak_handle(),
            id: conn.remote_id().to_string(),
            rx: heard_count(conn),
            heard: Instant::now(),
            rtt: rtt_of(conn),
        });
    }

    /// Closes every connection other machines opened to this one: what a
    /// phone going to the background owes them, so their wires to it end
    /// now instead of after their silence limit.
    pub(crate) fn let_go_served(&self, why: Retired) {
        let served = std::mem::take(&mut *self.served.lock().unwrap_or_else(|p| p.into_inner()));
        for conn in served.iter().filter_map(|w| w.conn.upgrade()) {
            conn.close(0u32.into(), why.reason());
        }
    }

    /// Closes the connections one far machine opened to this one: its
    /// wire here was just found dead, so those are too, and left open
    /// they would steer the next dial to it down the dead path.
    /// **A relay said this machine left it** (`Endpoint::relay_departures`,
    /// vendor/iroh/KHOR.md patch 4): its relay connection ended, so a wire
    /// to it through that relay is dead now, not after its silence limit.
    /// A wire on a direct road is left alone — the relay is not that
    /// road. Until it is heard again, a dial to it is given a short time
    /// ([`Wires::gone_lately`]).
    ///
    /// Before (round NET-5's probe, 200 ms relay road): a machine whose
    /// resident stopped kept its word 中继 for 14 s, and the first ask to
    /// it waited out an ack and a full dial to say so.
    pub(crate) fn departed(&self, id: &str) {
        self.note(Some(id), crate::journal::What::Departed);
        self.gone
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .insert(id.to_owned(), Instant::now());
        let mut any = false;
        {
            let mut slots = self.slots.lock().unwrap_or_else(|p| p.into_inner());
            for proto in [Proto::Main, Proto::Tunnel] {
                if let Some(slot) = slots.get_mut(&(id.to_owned(), proto))
                    && slot.conn.is_some()
                    && slot.word != Some(PathKind::Direct)
                {
                    self.drop_noted(id, proto, slot, Retired::Departed);
                    self.counts.retired(Retired::Departed);
                    any = true;
                }
            }
        }
        if any {
            self.let_go_served_by(id, Retired::Departed);
        }
    }

    /// Whether a relay said this machine left, and nothing has been heard
    /// from it since.
    ///
    /// The word is kept until the machine is heard ([`Self::back`]), not
    /// forgotten when [`LEFT_ALONE_FOR`] runs out: the pump's next failure
    /// to it waits the longest ([`Self::wait_after`]).
    pub(crate) fn gone_lately(&self, id: &str) -> bool {
        self.gone
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get(id)
            .is_some_and(|at| at.elapsed() < GONE_FOR)
    }

    /// **The far machine said it is resting** (net-dial B4): it refused a
    /// dial, or closed a wire going to the background or pausing
    /// ([`resting_close`]). Left alone as one its relay said left — not
    /// down, no backoff counted: it dials when it is back, and that
    /// connection clears the word ([`Self::serving`]) and is answered with
    /// a round at once.
    pub(crate) fn asleep(&self, id: &str) {
        self.gone
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .insert(id.to_owned(), Instant::now());
    }

    /// Whether the pump should leave this machine alone because its relay
    /// said it left and nothing has been heard from it since
    /// ([`LEFT_ALONE_FOR`]).
    pub(crate) fn left_alone(&self, id: &str) -> bool {
        self.gone
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get(id)
            .is_some_and(|at| at.elapsed() < LEFT_ALONE_FOR)
    }

    /// Heard from: whatever a relay said about it is old news, and so is
    /// an attempt that failed to reach it.
    fn back(&self, id: &str) {
        self.gone.lock().unwrap_or_else(|p| p.into_inner()).remove(id);
        self.down.lock().unwrap_or_else(|p| p.into_inner()).remove(id);
    }

    /// An attempt to reach this machine failed just now.
    fn went_down(&self, id: &str) {
        self.down
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .insert(id.to_owned(), Instant::now());
    }

    /// **Except one still talking**: a connection that far machine opened
    /// here and that was heard within its keepalive is alive, whatever this
    /// machine's own wire to it found — the two ride different paths when
    /// the two sides selected differently. 2026-10-16 23:13:42: turing's
    /// wire to the Mac was judged dead and the Mac's wire to turing, heard
    /// 367 ms before, was closed with it.
    fn let_go_served_by(&self, id: &str, why: Retired) {
        let now = Instant::now();
        let mut served = self.served.lock().unwrap_or_else(|p| p.into_inner());
        served.retain(|w| {
            if w.id != id {
                return true;
            }
            if now.saturating_duration_since(w.heard) < Patience::for_rtt(w.rtt).keepalive_missed {
                return true;
            }
            if let Some(conn) = w.conn.upgrade() {
                conn.close(0u32.into(), why.reason());
            }
            false
        });
    }

    /// A new road to this machine came up (round ROAD-2: a carried link):
    /// the pump's wait on it is forgiven, so the next round dials.
    pub(crate) fn forgive(&self, id: &str) {
        let mut slots = self.slots.lock().unwrap_or_else(|p| p.into_inner());
        for ((who, _), slot) in slots.iter_mut() {
            if who == id {
                slot.failures = 0;
                slot.retry_at = None;
            }
        }
    }

    /// **The home relay is back after being down since `at`** (NET-STABLE
    /// 第二步): the waits of the machines whose dials failed since are
    /// forgiven — they may have failed only because the relay was gone.
    /// A machine whose wait began before is left to it: a relay that
    /// drops for a moment is not a reason to redial every machine that
    /// is off (NET-6's cost). Answers how many it forgave.
    pub(crate) fn forgive_failed_since(&self, at: Instant) -> usize {
        let mut n = 0;
        for slot in self.slots.lock().unwrap_or_else(|p| p.into_inner()).values_mut() {
            if slot.retry_at.is_some() && slot.last_failure.as_ref().is_some_and(|(_, when)| *when >= at) {
                slot.failures = 0;
                slot.retry_at = None;
                n += 1;
            }
        }
        n
    }

    /// Every wait the pump was keeping is forgiven: the dials that failed
    /// may have failed on the old network.
    pub(crate) fn forgive_all(&self) {
        for slot in self.slots.lock().unwrap_or_else(|p| p.into_inner()).values_mut() {
            slot.failures = 0;
            slot.retry_at = None;
        }
    }

    /// The network under this machine changed, or it woke from sleep:
    /// any wire not heard within its [`Patience::after_change`] from now is
    /// presumed to have died with the old network ([`Self::tick`] judges).
    pub(crate) fn network_changed(&self) {
        self.forgive_all();
        *self.changed.lock().unwrap_or_else(|p| p.into_inner()) = Some(Instant::now());
        self.counts.network_changes.fetch_add(1, Relaxed);
    }

    /// Marks `id`'s wire as one a hop rides, until the guard drops: the
    /// side that opened the hop (its wire to the middle), the middle (its
    /// wires to both ends) and the far end (the middle's wire to it).
    pub(crate) fn hop_held(self: &Arc<Self>, id: &str) -> HopHeld {
        *self
            .hops
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .entry(id.to_owned())
            .or_default() += 1;
        HopHeld {
            wires: Arc::clone(self),
            id: id.to_owned(),
        }
    }

    /// One look at every wire: counts what was heard, settles the road,
    /// and retires the silent. Returns how many it retired, which is the
    /// pump's cue to visit early.
    pub(crate) fn tick(&self) -> usize {
        let now = Instant::now();
        // A change is judged per wire, each on its own patience; it stays
        // pending until the slowest could have answered.
        let change = {
            let mut changed = self.changed.lock().unwrap_or_else(|p| p.into_inner());
            match *changed {
                Some(at) if now >= at + CHANGE_JUDGED_WITHIN => changed.take(),
                other => other,
            }
        };
        let gone_with_change = |heard: Option<Instant>, rtt: Duration| {
            change.is_some_and(|at| {
                heard.is_none_or(|h| h < at) && now >= at + Patience::for_rtt(rtt).after_change
            })
        };
        // Taken apart from the other locks, so no order between them.
        let hops: Vec<String> = self
            .hops
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .keys()
            .cloned()
            .collect();
        let silent = |id: &str, rtt: Duration| {
            if hops.iter().any(|h| h == id) {
                Patience::for_hop_rtt(rtt).silent
            } else {
                Patience::for_rtt(rtt).silent
            }
        };
        let mut retired = 0;
        let mut dead: Vec<String> = Vec::new();
        let mut heard: Vec<String> = Vec::new();
        let mut resting: Vec<String> = Vec::new();
        {
            let mut served = self.served.lock().unwrap_or_else(|p| p.into_inner());
            served.retain_mut(|w| {
                let Some(conn) = w.conn.upgrade() else {
                    return false;
                };
                if let Some(why) = conn.close_reason() {
                    if resting_close(&why) {
                        resting.push(w.id.clone());
                    }
                    return false;
                }
                let rx = heard_count(&conn);
                if rx != w.rx {
                    w.rx = rx;
                    w.heard = now;
                    heard.push(w.id.clone());
                }
                let rtt = rtt_of(&conn);
                if !rtt.is_zero() {
                    w.rtt = rtt;
                }
                if now.saturating_duration_since(w.heard) >= silent(&w.id, w.rtt)
                    || gone_with_change(Some(w.heard), w.rtt)
                {
                    conn.close(0u32.into(), Retired::Silent.reason());
                    return false;
                }
                true
            });
        }
        let mut slots = self.slots.lock().unwrap_or_else(|p| p.into_inner());
        for ((id, proto), slot) in slots.iter_mut() {
            let Some(conn) = slot.conn.clone() else {
                continue;
            };
            if let Some(end) = conn.close_reason() {
                let why = if resting_close(&end) {
                    resting.push(id.clone());
                    Retired::Asleep
                } else {
                    Retired::Closed
                };
                self.drop_noted(id, *proto, slot, why);
                slot.conn = None;
                slot.word = None;
                slot.leaning = None;
                self.counts.retired_closed.fetch_add(1, Relaxed);
                retired += 1;
                continue;
            }
            let rx = heard_count(&conn);
            if rx != slot.rx {
                slot.rx = rx;
                slot.heard = Some(now);
                heard.push(id.clone());
            }
            let rtt = rtt_of(&conn);
            if !rtt.is_zero() {
                slot.rtt = rtt;
            }
            let quiet = slot
                .heard
                .map_or(Duration::MAX, |h| now.saturating_duration_since(h));
            let stale_after_change = gone_with_change(slot.heard, slot.rtt);
            // A hop rides only a tunnel wire; the request wire to the
            // same machine keeps its own patience.
            let limit = if *proto == Proto::Tunnel {
                silent(id, slot.rtt)
            } else {
                Patience::for_rtt(slot.rtt).silent
            };
            if quiet >= limit || stale_after_change {
                let why = if stale_after_change {
                    Retired::Network
                } else {
                    Retired::Silent
                };
                self.drop_noted(id, *proto, slot, why);
                slot.last_failure = Some((
                    if why == Retired::Network {
                        khor_catalog::msg::wire_network_changed()
                    } else {
                        khor_catalog::msg::wire_went_silent()
                    }
                    .to_owned(),
                    now,
                ));
                self.counts.retired(why);
                dead.push(id.clone());
                retired += 1;
                continue;
            }
            settle(slot, road_now(&conn));
        }
        drop(slots);
        for id in heard {
            self.back(&id);
        }
        for id in resting {
            self.asleep(&id);
        }
        for id in dead {
            self.let_go_served_by(&id, Retired::Silent);
        }
        retired
    }

    /// What this node can say about one machine right now: the settled
    /// road of its live wire (requests first, a borrow wire otherwise)
    /// and how long since it was last heard on either.
    pub(crate) fn link(&self, id: &str) -> Reading {
        let now = Instant::now();
        let slots = self.slots.lock().unwrap_or_else(|p| p.into_inner());
        let mut road = None;
        let mut since = None;
        let mut rtt = None;
        let mut relay = None;
        let mut heard: Option<Instant> = None;
        let mut last_rtt: Option<Duration> = None;
        // With no live wire: the newest road either protocol settled on.
        let mut last: Option<(PathKind, Instant)> = None;
        for proto in [Proto::Main, Proto::Tunnel] {
            let Some(slot) = slots.get(&(id.to_owned(), proto)) else {
                continue;
            };
            if !slot.rtt.is_zero() {
                last_rtt = Some(last_rtt.map_or(slot.rtt, |was| was.max(slot.rtt)));
            }
            if road.is_none()
                && let Some(conn) = slot.conn.as_ref().filter(|c| c.close_reason().is_none())
            {
                road = slot.word;
                since = slot.word.and(slot.word_at).map(|at| now.saturating_duration_since(at));
                rtt = Some(rtt_of(conn)).filter(|r| !r.is_zero());
                relay = khor_net::path::relay_name(conn);
            }
            if let Some(h) = slot.heard {
                heard = Some(heard.map_or(h, |was| was.max(h)));
            }
            if let (Some(w), Some(at)) = (slot.last_word, slot.word_at)
                && last.is_none_or(|(_, was)| at > was)
            {
                last = Some((w, at));
            }
        }
        drop(slots);
        let (last_road, since) = match road {
            Some(r) => (Some(r), since),
            None => (last.map(|(w, _)| w), last.map(|(_, at)| now.saturating_duration_since(at))),
        };
        // A live wire is the machine reached, whatever an attempt on the
        // other protocol said.
        let down = road
            .is_none()
            .then(|| self.down.lock().unwrap_or_else(|p| p.into_inner()).get(id).copied())
            .flatten()
            .map(|at| now.saturating_duration_since(at));
        Reading {
            road,
            since,
            last_road,
            heard: heard.map(|h| now.saturating_duration_since(h)),
            rtt,
            relay,
            down,
            last_rtt: rtt.or(last_rtt),
        }
    }

    /// The counters, with the per-machine failures filled in.
    pub(crate) fn counts_now(&self) -> khor_core::WireCounts {
        let mut c = self.counts.snapshot();
        let slots = self.slots.lock().unwrap_or_else(|p| p.into_inner());
        let mut by: std::collections::BTreeMap<String, (u64, u64)> = Default::default();
        for ((id, _), slot) in slots.iter() {
            let (tx, rx) = slot.conn.as_ref().map(bytes_of).unwrap_or_default();
            c.sent_bytes += slot.bytes_past.0 + tx;
            c.received_bytes += slot.bytes_past.1 + rx;
            if slot.failed_total + slot.unanswered_total > 0 {
                let e = by.entry(id.clone()).or_default();
                e.0 += slot.failed_total;
                e.1 += slot.unanswered_total;
            }
        }
        c.by_peer = by.into_iter().map(|(id, (f, u))| (id, f, u)).collect();
        c
    }

    /// One machine as the connections page shows it: the held wire's
    /// round trip, and the last failure on either protocol with its age.
    pub(crate) fn peer_view(&self, id: &str) -> (Option<Duration>, Option<(String, Duration)>, u64, u64) {
        let slots = self.slots.lock().unwrap_or_else(|p| p.into_inner());
        let (mut rtt, mut failure, mut failed, mut unanswered) = (None, None::<(String, Instant)>, 0, 0);
        for proto in [Proto::Main, Proto::Tunnel] {
            let Some(slot) = slots.get(&(id.to_owned(), proto)) else {
                continue;
            };
            if slot.conn.as_ref().is_some_and(|c| c.close_reason().is_none()) && rtt.is_none() {
                rtt = Some(slot.rtt);
            }
            if let Some((why, at)) = &slot.last_failure
                && failure.as_ref().is_none_or(|(_, was)| at > was)
            {
                failure = Some((why.clone(), *at));
            }
            failed += slot.failed_total;
            unanswered += slot.unanswered_total;
        }
        (rtt, failure.map(|(w, at)| (w, at.elapsed())), failed, unanswered)
    }

    /// The machines this node holds a live wire to, for tests and for
    /// whatever diagnostic wants it — names are the caller's business.
    pub(crate) fn held(&self) -> Vec<String> {
        let slots = self.slots.lock().unwrap_or_else(|p| p.into_inner());
        let mut ids: Vec<String> = slots
            .iter()
            .filter(|(_, s)| s.conn.as_ref().is_some_and(|c| c.close_reason().is_none()))
            .map(|((id, _), _)| id.clone())
            .collect();
        ids.sort();
        ids.dedup();
        ids
    }
}

fn drop_wire(slot: &mut Slot, why: Retired) {
    if let Some(conn) = slot.conn.take() {
        let (tx, rx) = bytes_of(&conn);
        slot.bytes_past.0 += tx;
        slot.bytes_past.1 += rx;
        conn.close(0u32.into(), why.reason());
    }
    slot.word = None;
    slot.leaning = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A different road is taken only after it held for two samples;
    /// the moment with no path selected is never a road.
    #[test]
    fn a_road_settles_after_two_samples_and_a_blank_moment_is_not_one() {
        let mut s = Slot::default();
        settle(&mut s, PathKind::Relay);
        assert_eq!(
            s.word,
            Some(PathKind::Relay),
            "the first reading stands at once"
        );
        settle(&mut s, PathKind::Unknown);
        assert_eq!(
            s.word,
            Some(PathKind::Relay),
            "no path selected is not a change"
        );
        settle(&mut s, PathKind::Direct);
        assert_eq!(
            s.word,
            Some(PathKind::Relay),
            "one sample of direct is a flicker"
        );
        settle(&mut s, PathKind::Relay);
        settle(&mut s, PathKind::Direct);
        assert_eq!(
            s.word,
            Some(PathKind::Relay),
            "an interrupted lean starts over"
        );
        settle(&mut s, PathKind::Direct);
        assert_eq!(s.word, Some(PathKind::Direct), "two in a row is a change");
    }

    /// **The road's age restarts when the road changes, and only then**
    /// (UX-48: a machine row's 「直连 · 3 小时」 is how long that word has
    /// held).
    #[test]
    fn the_roads_age_restarts_on_a_change_and_not_on_the_same_word() {
        let mut s = Slot::default();
        settle(&mut s, PathKind::Relay);
        let first = s.word_at.expect("the first road is dated");
        std::thread::sleep(Duration::from_millis(5));
        settle(&mut s, PathKind::Relay);
        settle(&mut s, PathKind::Direct);
        assert_eq!(s.word_at, Some(first), "the same word and a flicker keep its age");
        settle(&mut s, PathKind::Direct);
        assert!(s.word_at.is_some_and(|at| at > first), "a settled change is a new age");
    }

    /// **An idle machine keeps the road it last used** (UX-48: an idle
    /// row reads direct or relay over how long ago that road settled): the
    /// road and its date outlive the wire; never connected says neither;
    /// the newer of the two protocols' roads is the one said.
    #[test]
    fn an_idle_machine_keeps_the_road_it_last_used_and_when_it_settled() {
        let w = Wires::default();
        let r = w.link("m");
        assert_eq!((r.road, r.last_road, r.since), (None, None, None), "never connected: nothing to say");
        {
            let mut slots = w.slots.lock().unwrap();
            let s = slots.entry(("m".to_owned(), Proto::Main)).or_default();
            settle(s, PathKind::Relay);
            settle(s, PathKind::Direct);
            settle(s, PathKind::Direct);
            drop_wire(s, Retired::Silent);
        }
        let r = w.link("m");
        assert_eq!(r.road, None, "no live wire");
        assert_eq!(r.last_road, Some(PathKind::Direct), "the road it last settled on");
        let first = r.since.expect("dated by when that road settled");
        std::thread::sleep(Duration::from_millis(5));
        let aged = w.link("m").since.expect("still dated");
        assert!(aged > first, "and it keeps ageing");
        {
            let mut slots = w.slots.lock().unwrap();
            let s = slots.entry(("m".to_owned(), Proto::Tunnel)).or_default();
            settle(s, PathKind::Relay);
            drop_wire(s, Retired::Closed);
        }
        let r = w.link("m");
        assert_eq!(r.last_road, Some(PathKind::Relay), "the newer road of the two protocols");
        assert!(r.since.is_some_and(|s| s < aged), "dated by that road");
        assert_eq!(r.down, None, "idle is not unreachable");
    }

    #[test]
    fn backoff_doubles_from_five_seconds_to_five_minutes() {
        let secs: Vec<u64> = (1..=9).map(|n| backoff(n).as_secs()).collect();
        assert_eq!(secs, vec![5, 10, 20, 40, 80, 160, 300, 300, 300]);
    }

    /// **Unreachable is an attempt that failed, until anything is heard**
    /// (round NET-13): a dial's failure says it, letting a machine go
    /// does not, and hearing from it clears it.
    #[test]
    fn a_failed_dial_is_down_until_heard_and_letting_go_is_not() {
        let w = Wires::default();
        assert_eq!(w.link("m").down, None, "never dialled: idle");
        w.retire_peer("m", Retired::Paused);
        w.clear(Retired::Suspended);
        w.departed("m");
        assert_eq!(w.link("m").down, None, "let go of, or gone from its relay: idle");
        w.failed("m", Proto::Tunnel, "no".into());
        assert!(w.link("m").down.is_some(), "a failed dial on either wire");
        w.network_changed();
        assert!(w.link("m").down.is_some(), "a new network forgives the wait, not the fact");
        w.back("m");
        assert_eq!(w.link("m").down, None, "heard from");
    }

    /// **A machine left alone is not dialled again from five seconds**
    /// (net-dial B4): its relay said it left, or it said it was resting,
    /// and nothing was heard since — the pump's next failed dial to it
    /// waits the longest. Heard from, the backoff starts over.
    #[test]
    fn a_failure_after_being_left_alone_waits_the_longest() {
        let w = Wires::default();
        w.asleep("m");
        assert!(w.left_alone("m"), "resting: left alone");
        assert_eq!(w.link("m").down, None, "and not unreachable");
        w.failed("m", Proto::Main, "no".into());
        let wait = w.backing_off("m").expect("a wait");
        assert!(wait > BACKOFF_MAX - Duration::from_secs(5), "the longest wait, not five seconds: {wait:?}");
        w.back("m");
        w.forgive("m");
        w.failed("m", Proto::Main, "no".into());
        let wait = w.backing_off("m").expect("a wait");
        assert!(wait <= BACKOFF_FIRST, "heard from since: from five seconds again: {wait:?}");
    }

    /// **A middle's own word on the far machine** (net-dial B3): a wait
    /// on either protocol, a relay's word that it left, or an attempt
    /// that failed — each is out of reach; heard from, none is.
    #[test]
    fn out_of_reach_is_a_wait_a_departure_or_a_failure_until_heard() {
        let w = Wires::default();
        assert!(!w.out_of_reach("y"), "never dialled");
        w.failed("y", Proto::Tunnel, "no".into());
        assert!(w.out_of_reach("y"), "a tunnel dial failed and its wait runs");
        w.back("y");
        w.forgive("y");
        assert!(!w.out_of_reach("y"), "heard, and the wait forgiven");
        w.departed("y");
        assert!(w.out_of_reach("y"), "its relay said it left");
        w.back("y");
        assert!(!w.out_of_reach("y"), "heard from since");
    }
}
