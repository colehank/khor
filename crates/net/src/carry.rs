//! **Roads khor carries itself** (round ROAD-2, audit/ROAD-BRIEF.md T5 and
//! T6): an iroh path over a byte stream somebody else already holds.
//!
//! Two streams ride it, and iroh cannot tell them apart:
//!
//! - **khor inside ssh** (T5): the stdin and stdout of an ssh session
//!   whose far end is `khor carry-in`, bridged into that machine's
//!   resident. The machine that holds the ssh session (the carrier) is the
//!   only one with this road; a machine that ssh reaches but that reaches
//!   neither the internet nor a relay becomes a full peer through it.
//! - **the peer hop** (T6): a stream through a third khor machine that
//!   splices it to the far end. The middle forwards ciphertext; both ends
//!   authenticate end to end as themselves.
//!
//! One link is one stream to one machine. A frame is a QUIC datagram with
//! its length in front (`[u16 len][payload]`); the link knows whose it is,
//! so a frame carries no address. Everything is a datagram network to
//! QUIC: a full queue drops, never blocks (`spike: no frame dropped`).
//!
//! **The selector** ([`RoadOrder`]) is the owner's order: direct, then a
//! carried road, then the relay — and between a carried road and the
//! relay no move mid-connection unless the one in use breaks. iroh's own
//! selector puts a custom path level with direct and moves on a few
//! milliseconds (audit/road-spike.md, "mixed endpoint").
//!
//! ⚠️ The API is iroh's `unstable-custom-transports`. We vendor iroh, so it
//! changes only when we re-vendor; re-vendoring must re-run the spike.

use std::collections::HashMap;
use std::io;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use bytes::Bytes;
use iroh::endpoint::transports::{
    AddrKind, CustomEndpoint, CustomSender, CustomTransport, FourTuple, PathSelection, PathSelectionContext,
    PathSelector, RecvInfo, Transmit,
};
use iroh::{EndpointId, TransportAddr};
use iroh_base::CustomAddr;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::{mpsc, oneshot};

/// The custom transport's id in every [`CustomAddr`] khor makes: "khor".
pub const CARRY: u64 = 0x6b68_6f72;

/// Frames waiting to go out on one link, and frames waiting for iroh.
const QUEUE: usize = 1024;

/// A machine's address on the carried transport: its id. Which link
/// reaches it is this machine's business, not the address's.
pub fn addr_of(id: &EndpointId) -> CustomAddr {
    CustomAddr::from((CARRY, &id.as_bytes()[..]))
}

/// The machine a carried address names.
pub fn id_of(addr: &CustomAddr) -> Option<EndpointId> {
    if addr.id() != CARRY {
        return None;
    }
    let bytes: &[u8; 32] = addr.data().try_into().ok()?;
    EndpointId::from_bytes(bytes).ok()
}

/// The carried address as a dial candidate.
pub fn transport_addr(id: &EndpointId) -> TransportAddr {
    TransportAddr::Custom(addr_of(id))
}

/// What a link is a stream of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Carrier {
    /// An ssh session (T5): this machine runs it, or the far one does
    /// and this end is its `khor carry-in`.
    Ssh,
    /// A stream through `via`, which splices it to the far end (T6).
    Hop { via: EndpointId },
}

#[derive(Debug)]
struct Link {
    out: mpsc::Sender<Bytes>,
    carrier: Carrier,
    serial: u64,
}

#[derive(Debug)]
struct Inner {
    me: EndpointId,
    links: Mutex<HashMap<EndpointId, Link>>,
    /// Where readers hand frames to iroh: replaced at every bind, so a
    /// node that gets fresh ears keeps its links.
    inbox: Mutex<Option<mpsc::Sender<(EndpointId, Bytes)>>>,
    addrs: n0_watcher::Watchable<Vec<CustomAddr>>,
    serial: AtomicU64,
    dropped: AtomicU64,
}

/// **This machine's carried links**, one per far machine. Cheap to clone;
/// every clone is the same table.
#[derive(Debug, Clone)]
pub struct Links(Arc<Inner>);

/// Ends when its link does: the far end hung up, the stream broke, or a
/// newer link to the same machine took its place.
#[derive(Debug)]
pub struct Attached(oneshot::Receiver<()>);

impl Attached {
    pub async fn ended(self) {
        let _ = self.0.await;
    }
}

impl Links {
    pub fn new(me: EndpointId) -> Links {
        Links(Arc::new(Inner {
            me,
            links: Mutex::default(),
            inbox: Mutex::default(),
            addrs: n0_watcher::Watchable::new(vec![addr_of(&me)]),
            serial: AtomicU64::new(1),
            dropped: AtomicU64::new(0),
        }))
    }

    /// The transport to register at bind. One per endpoint; binding again
    /// (fresh ears) moves the links to the new endpoint.
    pub fn transport(&self) -> Arc<dyn CustomTransport> {
        Arc::new(self.clone())
    }

    /// Whether a link to `peer` is up now.
    pub fn has(&self, peer: &EndpointId) -> bool {
        self.lock().contains_key(peer)
    }

    /// What the link to `peer` is a stream of, when one is up.
    pub fn carrier(&self, peer: &EndpointId) -> Option<Carrier> {
        self.lock().get(peer).map(|l| l.carrier)
    }

    /// Every machine with a link up, and what it rides.
    pub fn up(&self) -> Vec<(EndpointId, Carrier)> {
        self.lock().iter().map(|(id, l)| (*id, l.carrier)).collect()
    }

    /// Frames dropped on a full queue since this table was made.
    pub fn dropped(&self) -> u64 {
        self.0.dropped.load(Relaxed)
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<EndpointId, Link>> {
        self.0.links.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// **Makes `r`/`w` the link to `peer`**, replacing any link to it. Runs
    /// on its own task until either half ends.
    pub fn attach<R, W>(&self, peer: EndpointId, carrier: Carrier, r: R, w: W) -> Attached
    where
        R: AsyncRead + Unpin + Send + 'static,
        W: AsyncWrite + Unpin + Send + 'static,
    {
        let (out, out_rx) = mpsc::channel::<Bytes>(QUEUE);
        let serial = self.0.serial.fetch_add(1, Relaxed);
        self.lock().insert(peer, Link { out, carrier, serial });
        let (done, gone) = oneshot::channel();
        let links = self.clone();
        tokio::spawn(async move {
            tokio::select! {
                () = write_frames(out_rx, w) => {}
                () = links.read_frames(peer, r) => {}
            }
            let mut map = links.lock();
            if map.get(&peer).is_some_and(|l| l.serial == serial) {
                map.remove(&peer);
            }
            drop(map);
            let _ = done.send(());
        });
        Attached(gone)
    }

    async fn read_frames<R: AsyncRead + Unpin>(&self, peer: EndpointId, mut r: R) {
        let mut len = [0u8; 2];
        loop {
            if r.read_exact(&mut len).await.is_err() {
                return;
            }
            let mut frame = vec![0u8; usize::from(u16::from_be_bytes(len))];
            if r.read_exact(&mut frame).await.is_err() {
                return;
            }
            let inbox = self.0.inbox.lock().unwrap_or_else(|p| p.into_inner()).clone();
            // No endpoint bound yet, or one that closed: a datagram to a
            // socket nobody reads.
            let Some(inbox) = inbox else { continue };
            if inbox.try_send((peer, Bytes::from(frame))).is_err() {
                self.0.dropped.fetch_add(1, Relaxed);
            }
        }
    }
}

/// Drains a link's queue onto its stream, flushing once the queue is
/// empty: a burst of datagrams goes out as one write.
async fn write_frames<W: AsyncWrite + Unpin>(mut rx: mpsc::Receiver<Bytes>, w: W) {
    let mut w = tokio::io::BufWriter::with_capacity(256 << 10, w);
    while let Some(f) = rx.recv().await {
        if w.write_all(&f).await.is_err() {
            return;
        }
        while let Ok(f) = rx.try_recv() {
            if w.write_all(&f).await.is_err() {
                return;
            }
        }
        if w.flush().await.is_err() {
            return;
        }
    }
}

impl CustomTransport for Links {
    fn bind(&self) -> io::Result<Box<dyn CustomEndpoint>> {
        let (tx, inbox) = mpsc::channel(QUEUE);
        *self.0.inbox.lock().unwrap_or_else(|p| p.into_inner()) = Some(tx);
        Ok(Box::new(Bound {
            links: self.clone(),
            inbox,
        }))
    }
}

#[derive(Debug)]
struct Bound {
    links: Links,
    inbox: mpsc::Receiver<(EndpointId, Bytes)>,
}

impl CustomEndpoint for Bound {
    fn watch_local_addrs(&self) -> n0_watcher::Direct<Vec<CustomAddr>> {
        self.links.0.addrs.watch()
    }

    fn create_sender(&self) -> Arc<dyn CustomSender> {
        Arc::new(self.links.clone())
    }

    fn poll_recv(
        &mut self,
        cx: &mut Context,
        bufs: &mut [io::IoSliceMut<'_>],
        metas: &mut [noq_udp::RecvMeta],
        infos: &mut [RecvInfo],
    ) -> Poll<io::Result<usize>> {
        let me = addr_of(&self.links.0.me);
        let mut n = 0;
        while n < bufs.len() {
            let (from, data) = match self.inbox.poll_recv(cx) {
                Poll::Ready(Some(p)) => p,
                Poll::Ready(None) => return Poll::Ready(Err(io::Error::other("carried links closed"))),
                Poll::Pending if n > 0 => break,
                Poll::Pending => return Poll::Pending,
            };
            if bufs[n].len() < data.len() {
                self.links.0.dropped.fetch_add(1, Relaxed);
                continue;
            }
            bufs[n][..data.len()].copy_from_slice(&data);
            metas[n].len = data.len();
            metas[n].stride = data.len();
            infos[n] = RecvInfo::new(addr_of(&from), Some(me.clone()));
            n += 1;
        }
        Poll::Ready(Ok(n))
    }

    fn max_transmit_segments(&self) -> NonZeroUsize {
        NonZeroUsize::new(16).expect("non-zero")
    }
}

impl CustomSender for Links {
    fn is_valid_send_addr(&self, addr: &CustomAddr) -> bool {
        addr.id() == CARRY
    }

    fn poll_send(
        &self,
        _cx: &mut Context,
        dst: &CustomAddr,
        _src: Option<&CustomAddr>,
        t: &Transmit<'_>,
    ) -> Poll<io::Result<()>> {
        let Some(to) = id_of(dst) else {
            return Poll::Ready(Err(io::Error::other("not a carried address")));
        };
        let out = match self.lock().get(&to) {
            Some(l) => l.out.clone(),
            // No link: like a datagram to a host that is not there.
            None => return Poll::Ready(Ok(())),
        };
        let seg = t.segment_size.unwrap_or(t.contents.len()).max(1);
        for chunk in t.contents.chunks(seg) {
            let Ok(len) = u16::try_from(chunk.len()) else {
                self.0.dropped.fetch_add(1, Relaxed);
                continue;
            };
            let mut f = Vec::with_capacity(2 + chunk.len());
            f.extend_from_slice(&len.to_be_bytes());
            f.extend_from_slice(chunk);
            if out.try_send(Bytes::from(f)).is_err() {
                self.0.dropped.fetch_add(1, Relaxed);
            }
        }
        Poll::Ready(Ok(()))
    }
}

/// A path's place in the owner's order. Lower is preferred.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    Direct,
    Carried,
    Relay,
}

impl Tier {
    fn of(path: &FourTuple) -> Tier {
        match path.addr_kind() {
            AddrKind::IpV4 | AddrKind::IpV6 => Tier::Direct,
            AddrKind::Relay => Tier::Relay,
            AddrKind::Custom(_) => Tier::Carried,
        }
    }
}

/// How long after a path is first selected a better tier may still take
/// over from it without the path breaking: the opening race, where the
/// relay often answers a few ms before a carried road does. Past it, a
/// connection on the relay stays on the relay.
pub const OPENING: Duration = Duration::from_secs(3);

/// Within the direct tier, the margin a path must beat the one in use by
/// before the selection moves (iroh's own `RTT_SWITCHING_MIN`).
const DIRECT_MARGIN: Duration = Duration::from_millis(5);

/// Paths remembered for [`OPENING`]; past this many the memory starts
/// over, which at worst lets one connection's relay give way to a
/// carried road once more.
const REMEMBERED: usize = 4096;

/// **khor's path selector** (round ROAD-2): direct, then carried, then
/// relay; moving up to direct whenever a direct path is open (iroh's hole
/// punch); between a carried road and the relay, only in the opening
/// moments or when the path in use breaks.
#[derive(Debug, Default)]
pub struct RoadOrder {
    first_seen: Mutex<HashMap<FourTuple, Instant>>,
}

impl PathSelector for RoadOrder {
    fn select(&self, ctx: &PathSelectionContext<'_>) -> PathSelection {
        let mut candidates = Vec::new();
        let mut current = None;
        for p in ctx.paths() {
            let Some(stats) = p.stats() else { continue };
            let tier = Tier::of(p.network_path());
            if Some(p.network_path()) == ctx.current() {
                current = Some(match current {
                    Some((t, rtt)) if rtt <= stats.rtt => (t, rtt),
                    _ => (tier, stats.rtt),
                });
            }
            candidates.push((p, tier, stats.rtt));
        }
        let age = ctx.current().map(|c| {
            let mut seen = self.first_seen.lock().unwrap_or_else(|p| p.into_inner());
            if seen.len() >= REMEMBERED && !seen.contains_key(c) {
                seen.clear();
            }
            seen.entry(c.clone()).or_insert_with(Instant::now).elapsed()
        });
        let ranked: Vec<(Tier, Duration)> = candidates.iter().map(|(_, t, r)| (*t, *r)).collect();
        let mut selection = PathSelection::none();
        if let Some(i) = choose(current, age.unwrap_or_default(), &ranked) {
            selection.set(&candidates[i].0);
        }
        selection
    }
}

/// The rule, apart from iroh's types: which candidate to move to, or
/// `None` to stay. `current` is the path in use with its round trip, or
/// `None` when nothing is selected or the selected path broke (it is no
/// longer among the open ones); `age` is how long it has been in use.
pub fn choose(current: Option<(Tier, Duration)>, age: Duration, candidates: &[(Tier, Duration)]) -> Option<usize> {
    let best = candidates
        .iter()
        .enumerate()
        .min_by_key(|(_, (tier, rtt))| (*tier, *rtt))
        .map(|(i, _)| i)?;
    let (best_tier, best_rtt) = candidates[best];
    let Some((tier, rtt)) = current else {
        return Some(best);
    };
    match (tier, best_tier) {
        // Up to direct at any time; within direct, only past the margin.
        (Tier::Direct, Tier::Direct) => (best_rtt + DIRECT_MARGIN <= rtt).then_some(best),
        (_, Tier::Direct) => Some(best),
        // Off direct only when it broke, and then `current` is `None`.
        (Tier::Direct, _) => None,
        // The relay gives way to a carried road only while opening.
        (Tier::Relay, Tier::Carried) => (age < OPENING).then_some(best),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MS: fn(u64) -> Duration = Duration::from_millis;

    #[test]
    fn the_order_is_direct_then_carried_then_relay() {
        let all = [(Tier::Relay, MS(5)), (Tier::Carried, MS(40)), (Tier::Direct, MS(90))];
        assert_eq!(choose(None, Duration::ZERO, &all), Some(2), "direct, however slow");
        assert_eq!(choose(None, Duration::ZERO, &all[..2]), Some(1), "a carried road over a faster relay");
        assert_eq!(choose(None, Duration::ZERO, &all[..1]), Some(0));
        assert_eq!(choose(None, Duration::ZERO, &[]), None);
    }

    /// The owner's 「只在断的时候换路」: a few ms is never a reason.
    #[test]
    fn a_carried_road_and_the_relay_hold_once_opened() {
        let long = OPENING + Duration::from_secs(1);
        let both = [(Tier::Relay, MS(5)), (Tier::Carried, MS(40))];
        assert_eq!(
            choose(Some((Tier::Carried, MS(40))), long, &both),
            None,
            "never down to the relay for being faster"
        );
        assert_eq!(
            choose(Some((Tier::Relay, MS(5))), long, &both),
            None,
            "a connection on the relay stays there"
        );
        assert_eq!(
            choose(Some((Tier::Relay, MS(5))), MS(200), &both),
            Some(1),
            "the opening race: the relay answered first"
        );
        // Broken: the one in use is gone, so there is no current.
        assert_eq!(choose(None, long, &both[..1]), Some(0), "the carried road broke: the relay");
    }

    #[test]
    fn direct_takes_over_at_any_time_and_is_left_only_when_it_breaks() {
        let long = Duration::from_secs(600);
        let open = [(Tier::Carried, MS(40)), (Tier::Direct, MS(60))];
        assert_eq!(choose(Some((Tier::Carried, MS(40))), long, &open), Some(1));
        assert_eq!(choose(Some((Tier::Relay, MS(40))), long, &[(Tier::Relay, MS(40)), (Tier::Direct, MS(60))]), Some(1));
        assert_eq!(choose(Some((Tier::Direct, MS(60))), long, &open), None);
        let two = [(Tier::Direct, MS(60)), (Tier::Direct, MS(57))];
        assert_eq!(choose(Some((Tier::Direct, MS(60))), long, &two), None, "within the margin");
        let two = [(Tier::Direct, MS(60)), (Tier::Direct, MS(20))];
        assert_eq!(choose(Some((Tier::Direct, MS(60))), long, &two), Some(1));
    }

    #[test]
    fn a_carried_address_names_its_machine() {
        let id = iroh::SecretKey::generate().public();
        assert_eq!(id_of(&addr_of(&id)), Some(id));
        assert_eq!(id_of(&CustomAddr::from((7u64, &id.as_bytes()[..]))), None, "another transport's");
        assert_eq!(id_of(&CustomAddr::from((CARRY, &[1u8, 2][..]))), None);
    }
}
