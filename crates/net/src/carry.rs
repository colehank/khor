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
    AddrKind, CustomEndpoint, CustomSender, CustomTransport, FourTuple, PathSelection,
    PathSelectionContext, PathSelector, RecvInfo, Transmit,
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
        self.carrier(peer).or_else(|| {
            self.0
                .last
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .get(peer)
                .copied()
        })
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
        self.lock().insert(
            peer,
            Link {
                out,
                carrier,
                serial,
            },
        );
        self.0
            .last
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .insert(peer, carrier);
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
            let inbox = self
                .0
                .inbox
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .clone();
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
                Poll::Ready(None) => {
                    return Poll::Ready(Err(io::Error::other("carried links closed")));
                }
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
/// before the selection moves (iroh's own `RTT_SWITCHING_MIN`), or a
/// quarter of the one in use's round trip if that is more ([`direct_margin`]).
const DIRECT_MARGIN: Duration = Duration::from_millis(5);

/// **How long a move between direct paths holds** (2026-10-21, net-flap):
/// for this long after the selection moved, while the path in use answers,
/// the other must beat it by twice the margin, or half its round trip
/// ([`direct_margin`]). The machines' two LAN roads (0–1 ms) traded places
/// 230–330 times a day, one every 22–60 s, most of them within 5 s of the
/// 30 s sync: the sync's burst lifts the road in use to 5–8 ms while the
/// other, idle, reads 1, the burst moves with the connection, and 30 s
/// later the same again. A road that really slowed (netlab p2: 1 → 21 ms)
/// still moves at once.
pub const DIRECT_DWELL: Duration = Duration::from_secs(60);

/// **The margin a direct path must beat the direct one in use by**, whose
/// round trip is `rtt`: [`DIRECT_MARGIN`] or a quarter of `rtt`, whichever
/// is more; within [`DIRECT_DWELL`] of the last move (`settling`), twice
/// that margin or half of `rtt`.
pub fn direct_margin(rtt: Duration, settling: bool) -> Duration {
    if settling {
        (DIRECT_MARGIN * 2).max(rtt / 2)
    } else {
        DIRECT_MARGIN.max(rtt / 4)
    }
}

/// **Between a carried road and the relay, the margin the faster must
/// beat the one in use by** (NET-DIRECT step 4, the owner 2026-10-09:
/// 「可以，一定要把iroh用好呀」 on Tailscale's peer-relay rule): the
/// lowest round trip carries the connection, and noise of a few ms moves
/// nothing.
pub const SWITCH_MARGIN: Duration = Duration::from_millis(10);

/// **Whether a second direct path is kept open beside the one in use**
/// (vendor/iroh/KHOR.md patch 18), as a spare to move to the moment the
/// one in use stops answering. Without one, a direct path that fails falls
/// back to the relay, and iroh punches again at once (patch 17) to find
/// the other direct road.
///
/// A spare costs its keepalive: a PING and its ACK every 15 s (patch 18's
/// `SPARE_KEEP_ALIVE`, a third of a selected path's 5 s). noq's per-path
/// keepalive timers are independent, so on a phone those wake the radio on
/// their own.
///
/// `KHOR_SPARE_ROAD`: `off`, `near` or `on`. Default `near`; `off` on iOS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Spare {
    /// Never: one direct path per connection, as upstream iroh keeps.
    Off,
    /// Only a direct path within [`SPARE_NEAR`] of the one in use — two
    /// roads of about one speed (a LAN and a second network), where losing
    /// one should not mean the relay.
    Near,
    /// The fastest other direct path, whatever its speed.
    On,
}

/// How much slower than the path in use a direct path may be and still be
/// kept as the spare under [`Spare::Near`].
pub const SPARE_NEAR: Duration = Duration::from_millis(10);

/// `KHOR_SPARE_ROAD` (see [`Spare`]).
pub const SPARE_ENV: &str = "KHOR_SPARE_ROAD";

impl Spare {
    /// The platform's default, unless [`SPARE_ENV`] says otherwise.
    pub fn from_env() -> Spare {
        match std::env::var(SPARE_ENV).ok().as_deref().map(str::trim) {
            Some("off") => Spare::Off,
            Some("near") => Spare::Near,
            Some("on") => Spare::On,
            _ if cfg!(target_os = "ios") => Spare::Off,
            _ => Spare::Near,
        }
    }
}

/// **khor's path selector** (round ROAD-2, rule of NET-DIRECT step 4):
/// direct whenever a direct path is open (iroh's hole punch keeps going
/// under every other road); otherwise the carried road or the relay,
/// whichever has the lower measured round trip, moving only when the
/// other beats the one in use by [`SWITCH_MARGIN`]. A carried road through
/// a middle machine is measured end to end — its two legs and the middle.
///
/// **A path that stopped answering is no candidate** (vendor/iroh patch
/// 17, `PathSelectionData::answering`): two probe timeouts in a row with
/// nothing acknowledged, on any connection it carries, its oldest packet
/// unanswered for 300 ms or three probe timeouts. **Unless it is the
/// direct path in use, and it stalled for less than the fastest other
/// road's round trip** (2026-10-21, net-flap): a packet sent on that road
/// now would arrive no sooner than the answer the stalled one still owes.
/// A 2 ms road whose ACK came 100 ms late went to the 500 ms relay and came
/// back within a second (MacLab↔hinton, 212 times a day). Each path's ACKs
/// come back on that path (vendor/noq-proto patch 4), so that is its own
/// round trip, not the road the peer sends on. Its round trip is the last
/// one it measured, so by RTT alone a dead path looks as good as ever;
/// iroh runs this selector every 250 ms on what the paths measure, so the
/// connection leaves it within one tick of noq's second probe timeout.
///
/// ⚠️ 墓碑: until 2026-10-09 the carried road always beat the relay in the
/// first 3 s of a connection and the two never traded places after
/// (「只在断的时候换路」): a hop slower than the relay kept carrying, and a
/// connection that had settled on the relay never took a faster hop.
/// ⚠️ 墓碑 (2026-10-16): the selector ran only when a path opened or was
/// abandoned. A direct path that slowed kept the connection until noq
/// abandoned it 15 s after its last packet, and one that went black kept
/// it all that time: khor judged the wire dead first (4 s) and redialled.
#[derive(Debug)]
pub struct RoadOrder {
    pub spare: Spare,
}

impl Default for RoadOrder {
    fn default() -> Self {
        RoadOrder {
            spare: Spare::from_env(),
        }
    }
}

impl PathSelector for RoadOrder {
    fn select(&self, ctx: &PathSelectionContext<'_>) -> PathSelection {
        let mut candidates = Vec::new();
        let mut current = None;
        let mut stalled = Vec::new();
        for p in ctx.paths() {
            // The road's round trip as the copy that takes its samples
            // measured it (vendor/iroh patch 17): the same road on the
            // other connection, idle, still reads its last keepalive's.
            let Some(rtt) = p.rtt() else { continue };
            let tier = Tier::of(p.network_path());
            let in_use = Some(p.network_path()) == ctx.current();
            if let Some(stall) = p.unanswered_for() {
                if in_use && tier == Tier::Direct {
                    stalled.push((p, stall, rtt));
                }
                continue;
            }
            if in_use {
                current = Some(match current {
                    Some((t, r)) if r <= rtt => (t, r),
                    _ => (tier, rtt),
                });
            }
            candidates.push((p, tier, rtt));
        }
        // The direct path in use, stalled: held while the stall is shorter
        // than any other road's round trip.
        let fastest = candidates.iter().map(|(_, _, r)| *r).min();
        if let Some(stall) = stalled.iter().map(|(_, s, _)| *s).max()
            && rides_out(stall, fastest)
        {
            for (p, _, rtt) in stalled {
                current = Some(match current {
                    Some((t, r)) if r <= rtt => (t, r),
                    _ => (Tier::Direct, rtt),
                });
                candidates.push((p, Tier::Direct, rtt));
            }
        }
        let ranked: Vec<(Tier, Duration)> = candidates.iter().map(|(_, t, r)| (*t, *r)).collect();
        let mut selection = PathSelection::none();
        let settling = ctx.held_for().is_some_and(|held| held < DIRECT_DWELL);
        let chosen = choose_settling(current, &ranked, settling);
        if let Some(i) = chosen {
            selection.set(&candidates[i].0);
        }
        // The path that will carry the connection: the new choice, or the
        // one in use staying.
        let carrying = match chosen {
            Some(i) => Some((
                candidates[i].0.network_path().clone(),
                candidates[i].1,
                candidates[i].2,
            )),
            None => current.and_then(|(t, r)| ctx.current().map(|c| (c.clone(), t, r))),
        };
        if let Some((path, tier, rtt)) = carrying
            && tier == Tier::Direct
        {
            let others: Vec<(bool, Duration)> = candidates
                .iter()
                .map(|(p, t, r)| (*t == Tier::Direct && *p.network_path() != path, *r))
                .collect();
            let held = candidates
                .iter()
                .position(|(p, _, _)| Some(p.network_path()) == ctx.backup());
            if let Some(i) = spare(self.spare, rtt, &others, held) {
                selection.set_backup(&candidates[i].0);
            }
        }
        selection
    }
}

/// **Whether the direct path in use rides out a stall** of `stall` (how
/// long its oldest unanswered packet has waited) rather than leave for the
/// fastest other road, of round trip `fastest`: while the stall is shorter.
pub fn rides_out(stall: Duration, fastest: Option<Duration>) -> bool {
    fastest.is_some_and(|fastest| stall < fastest)
}

/// **Which direct path to keep as the spare** beside one of round trip
/// `rtt` in use: `others` are the candidates, each whether it is another
/// direct path and its round trip; `held` the spare kept now, if it is
/// among them. The one held stays while it qualifies and is no more than
/// [`DIRECT_MARGIN`] behind the fastest that does (no trading places on
/// noise); otherwise the fastest that qualifies.
pub fn spare(
    policy: Spare,
    rtt: Duration,
    others: &[(bool, Duration)],
    held: Option<usize>,
) -> Option<usize> {
    let ok = |r: Duration| match policy {
        Spare::Off => false,
        Spare::Near => r <= rtt + SPARE_NEAR,
        Spare::On => true,
    };
    let best = others
        .iter()
        .enumerate()
        .filter(|(_, (direct, r))| *direct && ok(*r))
        .min_by_key(|(_, (_, r))| *r)
        .map(|(i, _)| i)?;
    match held {
        Some(h)
            if others[h].0 && ok(others[h].1) && others[h].1 <= others[best].1 + DIRECT_MARGIN =>
        {
            Some(h)
        }
        _ => Some(best),
    }
}

/// The rule, apart from iroh's types: which candidate to move to, or
/// `None` to stay. `current` is the path in use with its round trip, or
/// `None` when nothing is selected or the selected path broke (it is no
/// longer among the open ones).
pub fn choose(current: Option<(Tier, Duration)>, candidates: &[(Tier, Duration)]) -> Option<usize> {
    choose_settling(current, candidates, false)
}

/// [`choose`], `settling` within [`DIRECT_DWELL`] of the last move: a move
/// between direct paths then needs the larger margin ([`direct_margin`]).
pub fn choose_settling(
    current: Option<(Tier, Duration)>,
    candidates: &[(Tier, Duration)],
    settling: bool,
) -> Option<usize> {
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
        (Tier::Direct, Tier::Direct) => {
            (best_rtt + direct_margin(rtt, settling) <= rtt).then_some(best)
        }
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
        let all = [
            (Tier::Relay, MS(5)),
            (Tier::Carried, MS(40)),
            (Tier::Direct, MS(90)),
        ];
        assert_eq!(choose(None, &all), Some(2), "direct, however slow");
        assert_eq!(
            choose(None, &all[..2]),
            Some(0),
            "the relay, faster than the carried road"
        );
        let hop = [(Tier::Relay, MS(58)), (Tier::Carried, MS(29))];
        assert_eq!(
            choose(None, &hop),
            Some(1),
            "the hop, faster than the relay"
        );
        assert_eq!(choose(None, &all[..1]), Some(0));
        assert_eq!(choose(None, &[]), None);
    }

    /// NET-DIRECT step 4: the faster of a carried road and the relay, by
    /// at least [`SWITCH_MARGIN`] — never on a few ms of noise.
    #[test]
    fn a_carried_road_and_the_relay_trade_places_by_ten_ms() {
        let iphone_maclab = [(Tier::Relay, MS(58)), (Tier::Carried, MS(29))];
        assert_eq!(
            choose(Some((Tier::Relay, MS(58))), &iphone_maclab),
            Some(1),
            "29 ms through turing beats 58 ms"
        );
        assert_eq!(
            choose(Some((Tier::Carried, MS(29))), &iphone_maclab),
            None,
            "and holds"
        );
        let close = [(Tier::Relay, MS(30)), (Tier::Carried, MS(22))];
        assert_eq!(
            choose(Some((Tier::Relay, MS(30))), &close),
            None,
            "8 ms is not enough"
        );
        assert_eq!(choose(Some((Tier::Carried, MS(22))), &close), None);
        let degraded = [(Tier::Relay, MS(30)), (Tier::Carried, MS(90))];
        assert_eq!(
            choose(Some((Tier::Carried, MS(90))), &degraded),
            Some(0),
            "a hop whose leg degraded gives way"
        );
        // Broken: the one in use is gone, so there is no current.
        assert_eq!(
            choose(None, &degraded[..1]),
            Some(0),
            "the carried road broke: the relay"
        );
    }

    #[test]
    fn direct_takes_over_at_any_time_and_is_left_only_when_it_breaks() {
        let open = [(Tier::Carried, MS(40)), (Tier::Direct, MS(60))];
        assert_eq!(choose(Some((Tier::Carried, MS(40))), &open), Some(1));
        assert_eq!(
            choose(
                Some((Tier::Relay, MS(40))),
                &[(Tier::Relay, MS(40)), (Tier::Direct, MS(60))]
            ),
            Some(1)
        );
        assert_eq!(choose(Some((Tier::Direct, MS(60))), &open), None);
        let two = [(Tier::Direct, MS(60)), (Tier::Direct, MS(57))];
        assert_eq!(
            choose(Some((Tier::Direct, MS(60))), &two),
            None,
            "within the margin"
        );
        let two = [(Tier::Direct, MS(60)), (Tier::Direct, MS(20))];
        assert_eq!(choose(Some((Tier::Direct, MS(60))), &two), Some(1));
    }

    /// net-flap (2026-10-21): a direct road that stalled for less than the
    /// relay's round trip is ridden out; past it, or with a direct spare
    /// faster than the stall, it is left.
    #[test]
    fn a_stall_shorter_than_the_other_road_is_ridden_out() {
        assert!(
            rides_out(MS(300), Some(MS(500))),
            "300 ms stalled, the relay 500 ms away: stay"
        );
        assert!(
            !rides_out(MS(500), Some(MS(500))),
            "the relay's round trip: go"
        );
        assert!(!rides_out(MS(300), Some(MS(9))), "a spare at 9 ms: go");
        assert!(!rides_out(MS(300), None), "nowhere to go: nothing to hold");
    }

    /// net-flap (2026-10-21): two LAN roads, the one in use lifted to 8 ms
    /// by the sync's burst while the other reads 1: within a minute of the
    /// last move nothing moves; a road that really slowed (p2, 21 ms) moves
    /// at once.
    #[test]
    fn a_move_between_direct_roads_holds_for_a_minute() {
        let loaded = [(Tier::Direct, MS(8)), (Tier::Direct, MS(1))];
        assert_eq!(
            choose_settling(Some((Tier::Direct, MS(8))), &loaded, true),
            None,
            "settling: 7 ms is not enough"
        );
        assert_eq!(
            choose_settling(Some((Tier::Direct, MS(8))), &loaded, false),
            Some(1),
            "settled: it is"
        );
        let slowed = [(Tier::Direct, MS(21)), (Tier::Direct, MS(1))];
        assert_eq!(
            choose_settling(Some((Tier::Direct, MS(21))), &slowed, true),
            Some(1),
            "p2: at once"
        );
        assert_eq!(direct_margin(MS(1), false), MS(5));
        assert_eq!(
            direct_margin(MS(60), false),
            MS(15),
            "a quarter of a slow road"
        );
        assert_eq!(direct_margin(MS(60), true), MS(30));
        let far = [(Tier::Direct, MS(60)), (Tier::Direct, MS(48))];
        assert_eq!(
            choose(Some((Tier::Direct, MS(60))), &far),
            None,
            "12 ms on 60 is noise"
        );
    }

    /// vendor/iroh patch 18: the spare beside a direct path in use.
    #[test]
    fn a_spare_is_a_direct_road_of_about_one_speed_and_stays() {
        // (another direct path?, rtt): the relay, a second network, a far one.
        let others = [(false, MS(20)), (true, MS(3)), (true, MS(40))];
        assert_eq!(
            spare(Spare::Off, MS(1), &others, None),
            None,
            "off keeps none"
        );
        assert_eq!(
            spare(Spare::Near, MS(1), &others, None),
            Some(1),
            "the second network, 2 ms behind"
        );
        assert_eq!(
            spare(Spare::Near, MS(1), &others[..1], None),
            None,
            "the relay is never the spare"
        );
        assert_eq!(
            spare(Spare::Near, MS(1), &[(true, MS(40))], None),
            None,
            "39 ms behind is no spare"
        );
        assert_eq!(
            spare(Spare::On, MS(1), &[(true, MS(40))], None),
            Some(0),
            "unless asked for any"
        );
        let two = [(true, MS(4)), (true, MS(2))];
        assert_eq!(
            spare(Spare::Near, MS(1), &two, Some(0)),
            Some(0),
            "the one held stays within the margin"
        );
        let two = [(true, MS(9)), (true, MS(2))];
        assert_eq!(
            spare(Spare::Near, MS(1), &two, Some(0)),
            Some(1),
            "and gives way past it"
        );
    }

    #[test]
    fn a_carried_address_names_its_machine() {
        let id = iroh::SecretKey::generate().public();
        assert_eq!(id_of(&addr_of(&id)), Some(id));
        assert_eq!(
            id_of(&CustomAddr::from((7u64, &id.as_bytes()[..]))),
            None,
            "another transport's"
        );
        assert_eq!(id_of(&CustomAddr::from((CARRY, &[1u8, 2][..]))), None);
    }

    /// A hop whose link ended still reads as the hop until iroh leaves the
    /// carried path, never as ssh (net-e: the three-node test read 「Ssh」
    /// for two seconds after the middle stopped).
    #[tokio::test]
    async fn an_ended_link_is_remembered_by_what_it_rode() {
        let links = Links::new(iroh::SecretKey::generate().public());
        let (peer, via) = (
            iroh::SecretKey::generate().public(),
            iroh::SecretKey::generate().public(),
        );
        assert_eq!(links.last_carrier(&peer), None);
        let (here, there) = tokio::io::duplex(64);
        let (r, w) = tokio::io::split(here);
        let attached = links.attach(peer, Carrier::Hop { via }, r, w);
        assert_eq!(links.last_carrier(&peer), Some(Carrier::Hop { via }));
        drop(there);
        attached.ended().await;
        assert_eq!(links.carrier(&peer), None, "the link is gone");
        assert_eq!(
            links.last_carrier(&peer),
            Some(Carrier::Hop { via }),
            "and is remembered as the hop"
        );
    }
}
