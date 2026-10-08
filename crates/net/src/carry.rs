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
//! **The selector** ([`RoadOrder`]): direct whenever it is open, else the
//! faster of a carried road and the relay by at least
//! [`SWITCH_MARGIN`] (NET-DIRECT step 4). iroh's own selector puts a
//! custom path level with direct and moves on a few milliseconds
//! (audit/road-spike.md, "mixed endpoint").
//!
//! ⚠️ The API is iroh's `unstable-custom-transports`. We vendor iroh, so it
//! changes only when we re-vendor; re-vendoring must re-run the spike.

use std::collections::HashMap;
use std::io;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::Duration;

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
    /// The carrier each machine's link was last attached on, kept after
    /// the link ends ([`Links::last_carrier`]).
    last: Mutex<HashMap<EndpointId, Carrier>>,
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
            last: Mutex::default(),
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

    /// **What the link to `peer` rides, or rode when it ended**: the word
    /// for a connection iroh still has on its carried path between the
    /// link ending and iroh choosing another road. Read as [`carrier`]
    /// was, a hop whose middle stopped answering read as ssh for those
    /// two seconds, on the face and in the row (net-e, 2026-10-09).
    ///
    /// [`carrier`]: Links::carrier
    pub fn last_carrier(&self, peer: &EndpointId) -> Option<Carrier> {
        self.carrier(peer)
            .or_else(|| self.0.last.lock().unwrap_or_else(|p| p.into_inner()).get(peer).copied())
    }

    /// Every machine with a link up, and what it rides.
    pub fn up(&self) -> Vec<(EndpointId, Carrier)> {
        self.lock().iter().map(|(id, l)| (*id, l.carrier)).collect()
    }

    /// Ends the link to `peer` if it is still a stream of `carrier`: its
    /// queue closes, so its writer and then its task end. A newer link of
    /// another kind is left alone.
    pub fn detach(&self, peer: &EndpointId, carrier: Carrier) {
        let mut map = self.lock();
        if map.get(peer).is_some_and(|l| l.carrier == carrier) {
            map.remove(peer);
        }
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
        self.0.last.lock().unwrap_or_else(|p| p.into_inner()).insert(peer, carrier);
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

/// Within the direct tier, the margin a path must beat the one in use by
/// before the selection moves (iroh's own `RTT_SWITCHING_MIN`).
const DIRECT_MARGIN: Duration = Duration::from_millis(5);

/// **Between a carried road and the relay, the margin the faster must
/// beat the one in use by** (NET-DIRECT step 4, the owner 2026-10-09:
/// 「可以，一定要把iroh用好呀」 on Tailscale's peer-relay rule): the
/// lowest round trip carries the connection, and noise of a few ms moves
/// nothing.
pub const SWITCH_MARGIN: Duration = Duration::from_millis(10);

/// **khor's path selector** (round ROAD-2, rule of NET-DIRECT step 4):
/// direct whenever a direct path is open (iroh's hole punch keeps going
/// under every other road); otherwise the carried road or the relay,
/// whichever has the lower measured round trip, moving only when the
/// other beats the one in use by [`SWITCH_MARGIN`]. A carried road through
/// a middle machine is measured end to end — its two legs and the middle.
///
/// ⚠️ 墓碑: until 2026-10-09 the carried road always beat the relay in the
/// first 3 s of a connection and the two never traded places after
/// (「只在断的时候换路」): a hop slower than the relay kept carrying, and a
/// connection that had settled on the relay never took a faster hop.
#[derive(Debug, Default)]
pub struct RoadOrder;

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
        let ranked: Vec<(Tier, Duration)> = candidates.iter().map(|(_, t, r)| (*t, *r)).collect();
        let mut selection = PathSelection::none();
        if let Some(i) = choose(current, &ranked) {
            selection.set(&candidates[i].0);
        }
        selection
    }
}

/// The rule, apart from iroh's types: which candidate to move to, or
/// `None` to stay. `current` is the path in use with its round trip, or
/// `None` when nothing is selected or the selected path broke (it is no
/// longer among the open ones).
pub fn choose(current: Option<(Tier, Duration)>, candidates: &[(Tier, Duration)]) -> Option<usize> {
    let best = candidates
        .iter()
        .enumerate()
        .min_by_key(|(_, (tier, rtt))| (*tier != Tier::Direct, *rtt))
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
        // A carried road and the relay: the faster, past the margin.
        _ => (best_rtt + SWITCH_MARGIN <= rtt).then_some(best),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MS: fn(u64) -> Duration = Duration::from_millis;

    #[test]
    fn direct_first_then_the_faster_of_carried_and_relay() {
        let all = [(Tier::Relay, MS(5)), (Tier::Carried, MS(40)), (Tier::Direct, MS(90))];
        assert_eq!(choose(None, &all), Some(2), "direct, however slow");
        assert_eq!(choose(None, &all[..2]), Some(0), "the relay, faster than the carried road");
        let hop = [(Tier::Relay, MS(58)), (Tier::Carried, MS(29))];
        assert_eq!(choose(None, &hop), Some(1), "the hop, faster than the relay");
        assert_eq!(choose(None, &all[..1]), Some(0));
        assert_eq!(choose(None, &[]), None);
    }

    /// NET-DIRECT step 4: the faster of a carried road and the relay, by
    /// at least [`SWITCH_MARGIN`] — never on a few ms of noise.
    #[test]
    fn a_carried_road_and_the_relay_trade_places_by_ten_ms() {
        let iphone_maclab = [(Tier::Relay, MS(58)), (Tier::Carried, MS(29))];
        assert_eq!(choose(Some((Tier::Relay, MS(58))), &iphone_maclab), Some(1), "29 ms through turing beats 58 ms");
        assert_eq!(choose(Some((Tier::Carried, MS(29))), &iphone_maclab), None, "and holds");
        let close = [(Tier::Relay, MS(30)), (Tier::Carried, MS(22))];
        assert_eq!(choose(Some((Tier::Relay, MS(30))), &close), None, "8 ms is not enough");
        assert_eq!(choose(Some((Tier::Carried, MS(22))), &close), None);
        let degraded = [(Tier::Relay, MS(30)), (Tier::Carried, MS(90))];
        assert_eq!(choose(Some((Tier::Carried, MS(90))), &degraded), Some(0), "a hop whose leg degraded gives way");
        // Broken: the one in use is gone, so there is no current.
        assert_eq!(choose(None, &degraded[..1]), Some(0), "the carried road broke: the relay");
    }

    #[test]
    fn direct_takes_over_at_any_time_and_is_left_only_when_it_breaks() {
        let open = [(Tier::Carried, MS(40)), (Tier::Direct, MS(60))];
        assert_eq!(choose(Some((Tier::Carried, MS(40))), &open), Some(1));
        assert_eq!(choose(Some((Tier::Relay, MS(40))), &[(Tier::Relay, MS(40)), (Tier::Direct, MS(60))]), Some(1));
        assert_eq!(choose(Some((Tier::Direct, MS(60))), &open), None);
        let two = [(Tier::Direct, MS(60)), (Tier::Direct, MS(57))];
        assert_eq!(choose(Some((Tier::Direct, MS(60))), &two), None, "within the margin");
        let two = [(Tier::Direct, MS(60)), (Tier::Direct, MS(20))];
        assert_eq!(choose(Some((Tier::Direct, MS(60))), &two), Some(1));
    }

    #[test]
    fn a_carried_address_names_its_machine() {
        let id = iroh::SecretKey::generate().public();
        assert_eq!(id_of(&addr_of(&id)), Some(id));
        assert_eq!(id_of(&CustomAddr::from((7u64, &id.as_bytes()[..]))), None, "another transport's");
        assert_eq!(id_of(&CustomAddr::from((CARRY, &[1u8, 2][..]))), None);
    }

    /// A hop whose link ended still reads as the hop until iroh leaves the
    /// carried path, never as ssh (net-e: the three-node test read 「Ssh」
    /// for two seconds after the middle stopped).
    #[tokio::test]
    async fn an_ended_link_is_remembered_by_what_it_rode() {
        let links = Links::new(iroh::SecretKey::generate().public());
        let (peer, via) = (iroh::SecretKey::generate().public(), iroh::SecretKey::generate().public());
        assert_eq!(links.last_carrier(&peer), None);
        let (here, there) = tokio::io::duplex(64);
        let (r, w) = tokio::io::split(here);
        let attached = links.attach(peer, Carrier::Hop { via }, r, w);
        assert_eq!(links.last_carrier(&peer), Some(Carrier::Hop { via }));
        drop(there);
        attached.ended().await;
        assert_eq!(links.carrier(&peer), None, "the link is gone");
        assert_eq!(links.last_carrier(&peer), Some(Carrier::Hop { via }), "and is remembered as the hop");
    }
}
