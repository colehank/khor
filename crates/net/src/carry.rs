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
//! **The selector** ([`RoadOrder`]): the measured fastest answering road,
//! regardless of transport. A move must beat [`switch_margin`], and iroh
//! confirms it with fresh samples. Transport kind describes a road; it
//! does not rank it above a faster road (feat/net, 2026-11-09).
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
    /// Dropping this attachment stops its I/O, including a blocked flush.
    _stop: oneshot::Sender<()>,
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
pub struct Attached {
    serial: u64,
    ended: oneshot::Receiver<()>,
}

impl Attached {
    /// Attachment identity within the Links table that created it.
    pub fn serial(&self) -> u64 {
        self.serial
    }

    pub async fn ended(self) {
        let _ = self.ended.await;
    }
}

/// Opt-in lifecycle facts for isolated network diagnosis. Values are built
/// only when the same diagnostic switch as the native entry point is set.
/// This does not change the connection journal or its human-facing words.
pub fn observe_lifecycle(
    peer: impl std::fmt::Display,
    phase: &str,
    details: impl FnOnce() -> serde_json::Value,
) {
    if std::env::var_os("KHOR_NET_DIAGNOSTICS").is_none() {
        return;
    }
    let at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64);
    eprintln!(
        "[khor-carried-lifecycle] {}",
        serde_json::json!({
            "schema": 1, "at": at, "pid": std::process::id(),
            "peer": peer.to_string(), "phase": phase, "details": details(),
        })
    );
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
        self.detach_matching(peer, carrier, None);
    }

    /// Ends only the attachment this owner created. A later stream of the
    /// same kind, even through the same middle, belongs to its new owner.
    /// Returns whether the current link was removed.
    pub fn detach_attached(&self, peer: &EndpointId, carrier: Carrier, serial: u64) -> bool {
        self.detach_matching(peer, carrier, Some(serial))
    }

    fn detach_matching(&self, peer: &EndpointId, carrier: Carrier, serial: Option<u64>) -> bool {
        let mut map = self.lock();
        if !map.get(peer).is_some_and(|l| {
            l.carrier == carrier && serial.is_none_or(|expected| l.serial == expected)
        }) {
            return false;
        }
        observe_lifecycle(peer, "link_detached", || {
            serde_json::json!({
                "serial": map.get(peer).map(|l| l.serial), "carrier": format!("{carrier:?}"),
            })
        });
        map.remove(peer);
        true
    }

    /// Frames dropped on a full queue since this table was made.
    pub fn dropped(&self) -> u64 {
        self.0.dropped.load(Relaxed)
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<EndpointId, Link>> {
        self.0.links.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// **Makes `r`/`w` the link to `peer`**, replacing any link to it. Runs
    /// on its own task until either half ends or this attachment is removed.
    pub fn attach<R, W>(&self, peer: EndpointId, carrier: Carrier, r: R, w: W) -> Attached
    where
        R: AsyncRead + Unpin + Send + 'static,
        W: AsyncWrite + Unpin + Send + 'static,
    {
        self.attach_if(peer, carrier, r, w, |_| true)
            .expect("unconditional attachment")
    }

    /// Checks the current carrier and replaces it under the same registry
    /// lock. A rejected stream is dropped without changing the live owner.
    /// `allow` must not reenter this Links table.
    pub fn attach_if<R, W>(
        &self,
        peer: EndpointId,
        carrier: Carrier,
        r: R,
        w: W,
        allow: impl FnOnce(Option<Carrier>) -> bool,
    ) -> Option<Attached>
    where
        R: AsyncRead + Unpin + Send + 'static,
        W: AsyncWrite + Unpin + Send + 'static,
    {
        let (out_rx, stopped, serial) = {
            let mut map = self.lock();
            if !allow(map.get(&peer).map(|link| link.carrier)) {
                return None;
            }
            let (out, out_rx) = mpsc::channel::<Bytes>(QUEUE);
            let (stop, stopped) = oneshot::channel();
            let serial = self.0.serial.fetch_add(1, Relaxed);
            observe_lifecycle(peer, "link_attached", || {
                serde_json::json!({
                    "serial": serial, "carrier": format!("{carrier:?}"),
                })
            });
            let previous_serial = map.get(&peer).map(|l| l.serial);
            map.insert(
                peer,
                Link {
                    out,
                    carrier,
                    serial,
                    _stop: stop,
                },
            );
            observe_lifecycle(peer, "link_attachment_committed", || {
                serde_json::json!({
                    "serial": serial, "previous_serial": previous_serial,
                    "carrier": format!("{carrier:?}"),
                })
            });
            self.0
                .last
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .insert(peer, carrier);
            (out_rx, stopped, serial)
        };
        let (done, gone) = oneshot::channel();
        let links = self.clone();
        tokio::spawn(async move {
            let ended = tokio::select! {
                () = write_frames(out_rx, w) => "writer_ended",
                () = links.read_frames(peer, r) => "reader_ended",
                _ = stopped => "attachment_removed",
            };
            let mut map = links.lock();
            observe_lifecycle(peer, "link_io_ended", || {
                serde_json::json!({
                    "serial": serial, "current_serial": map.get(&peer).map(|l| l.serial),
                    "carrier": format!("{carrier:?}"), "side": ended,
                })
            });
            if map.get(&peer).is_some_and(|l| l.serial == serial) {
                map.remove(&peer);
            }
            drop(map);
            let _ = done.send(());
        });
        Some(Attached {
            serial,
            ended: gone,
        })
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

/// A path's transport, used for reporting and spare-path management.
/// Selection compares measurements, not this enum's declaration order.
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

/// The absolute eligibility floor. Moving between live paths additionally
/// requires iroh's independent fresh-sample confirmation.
const PATH_MARGIN: Duration = Duration::from_millis(5);

/// For one minute after a move, require at least 10 ms of gain. This
/// suppresses the 1 -> 8 ms LAN burst oscillation without making a large
/// improvement on a high-RTT road wait for the dwell to expire.
pub const PATH_DWELL: Duration = Duration::from_secs(60);

/// The same absolute eligibility floor for every transport. During
/// settling it doubles; confidence belongs to fresh-sample confirmation.
pub fn switch_margin(_rtt: Duration, settling: bool) -> Duration {
    if settling {
        PATH_MARGIN * 2
    } else {
        PATH_MARGIN
    }
}

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

/// **khor's path selector**: the answering path with the lowest measured
/// RTT, changing only past [`switch_margin`]. The measurement of a
/// carried road includes both legs and the middle. Initial connection
/// and later upgrades use the same transport-independent ordering.
///
/// **A path that stopped answering is no candidate** (vendor/iroh patch
/// 17, `PathSelectionData::answering`): two probe timeouts in a row with
/// nothing acknowledged, on any connection it carries, its oldest packet
/// unanswered for 300 ms or three probe timeouts. **Unless it is the
/// path in use, and it stalled for less than the fastest other
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
                if in_use {
                    stalled.push((p, tier, stall, rtt));
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
        // The path in use, stalled: held while the stall is shorter
        // than any other road's round trip.
        let fastest = candidates.iter().map(|(_, _, r)| *r).min();
        if let Some(stall) = stalled.iter().map(|(_, _, s, _)| *s).max()
            && rides_out(stall, fastest)
        {
            for (p, tier, _, rtt) in stalled {
                current = Some(match current {
                    Some((t, r)) if r <= rtt => (t, r),
                    _ => (tier, rtt),
                });
                candidates.push((p, tier, rtt));
            }
        }
        let ranked: Vec<(Tier, Duration)> = candidates.iter().map(|(_, t, r)| (*t, *r)).collect();
        let mut selection = PathSelection::none();
        let settling = ctx.held_for().is_some_and(|held| held < PATH_DWELL);
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

/// **Whether the path in use rides out a stall** of `stall` (how
/// long its oldest unanswered packet has waited) rather than leave for the
/// fastest other road, of round trip `fastest`: while the stall is shorter.
pub fn rides_out(stall: Duration, fastest: Option<Duration>) -> bool {
    fastest.is_some_and(|fastest| stall < fastest)
}

/// **Which direct path to keep as the spare** beside one of round trip
/// `rtt` in use: `others` are the candidates, each whether it is another
/// direct path and its round trip; `held` the spare kept now, if it is
/// among them. The one held stays while it qualifies and is no more than
/// [`PATH_MARGIN`] behind the fastest that does (no trading places on
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
            if others[h].0 && ok(others[h].1) && others[h].1 <= others[best].1 + PATH_MARGIN =>
        {
            Some(h)
        }
        _ => Some(best),
    }
}

/// Whether a candidate improves on the current path enough to switch.
/// Selection and carried-road cleanup share this rule so a watcher cannot
/// remove a path the selector would use. Preparing a new middle pays its
/// separate, existing cost hurdle.
pub fn improves(candidate: Duration, current: Duration, settling: bool) -> bool {
    candidate.saturating_add(switch_margin(current, settling)) <= current
}

/// The rule, apart from iroh's types: which candidate to move to, or
/// `None` to stay. `current` is the path in use with its round trip, or
/// `None` when nothing is selected or the selected path broke (it is no
/// longer among the open ones).
pub fn choose(current: Option<(Tier, Duration)>, candidates: &[(Tier, Duration)]) -> Option<usize> {
    choose_settling(current, candidates, false)
}

/// [`choose`], `settling` within [`PATH_DWELL`] of the last move: a move
/// requires the settling noise floor ([`switch_margin`]).
pub fn choose_settling(
    current: Option<(Tier, Duration)>,
    candidates: &[(Tier, Duration)],
    settling: bool,
) -> Option<usize> {
    let best = candidates
        .iter()
        .enumerate()
        .min_by_key(|(_, (_, rtt))| *rtt)
        .map(|(i, _)| i)?;
    let (_, best_rtt) = candidates[best];
    let Some((_, rtt)) = current else {
        return Some(best);
    };
    improves(best_rtt, rtt, settling).then_some(best)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MS: fn(u64) -> Duration = Duration::from_millis;

    /// Product rule: a road's transport cannot overrule a large measured
    /// improvement. Cover starting, leaving a live road and settling in
    /// both directions for every transport pairing.
    #[test]
    fn measured_speed_wins_for_every_transport_pair() {
        let kinds = [Tier::Direct, Tier::Carried, Tier::Relay];
        for slow in kinds {
            for fast in kinds {
                let roads = [(slow, MS(90)), (fast, MS(5))];
                assert_eq!(choose(None, &roads), Some(1), "start: {roads:?}");
                for settling in [false, true] {
                    assert_eq!(
                        choose_settling(Some(roads[0]), &roads, settling),
                        Some(1),
                        "leave slow: {roads:?}, settling {settling}"
                    );
                    assert_eq!(
                        choose_settling(Some(roads[1]), &roads, settling),
                        None,
                        "hold fast: {roads:?}, settling {settling}"
                    );
                }
            }
        }
    }

    #[test]
    fn settling_filters_noise_without_trapping_a_materially_slower_road() {
        for kind in [Tier::Direct, Tier::Carried, Tier::Relay] {
            let improved = [(kind, MS(500)), (Tier::Relay, MS(300))];
            assert_eq!(
                choose_settling(Some(improved[0]), &improved, true),
                Some(1),
                "40 percent faster is useful even just after a move"
            );
            let noise = [(kind, MS(8)), (Tier::Relay, MS(1))];
            assert_eq!(
                choose_settling(Some(noise[0]), &noise, true),
                None,
                "a 7 ms fluctuation does not restart settling"
            );
        }
    }

    #[test]
    fn a_measured_twenty_percent_gain_can_enter_fresh_confirmation() {
        for kind in [Tier::Direct, Tier::Carried, Tier::Relay] {
            let roads = [(Tier::Relay, MS(131)), (kind, MS(104))];
            for settling in [false, true] {
                assert_eq!(
                    choose_settling(Some(roads[0]), &roads, settling),
                    Some(1),
                    "a real 27 ms gain must reach the existing fresh-sample confirmation"
                );
            }
        }
    }

    #[test]
    fn the_fastest_measured_road_starts_the_connection() {
        let all = [
            (Tier::Relay, MS(5)),
            (Tier::Carried, MS(40)),
            (Tier::Direct, MS(90)),
        ];
        assert_eq!(choose(None, &all), Some(0), "the relay is fastest");
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

    /// Relative and absolute gain have the same meaning for relay and
    /// carried roads as for direct roads.
    #[test]
    fn a_carried_road_and_the_relay_use_the_same_gain_margin() {
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
            Some(1),
            "8 ms on 30 ms exceeds the quarter-RTT margin"
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
    fn a_live_direct_road_competes_by_measurement_in_both_directions() {
        let open = [(Tier::Carried, MS(40)), (Tier::Direct, MS(60))];
        assert_eq!(choose(Some((Tier::Carried, MS(40))), &open), None);
        assert_eq!(
            choose(
                Some((Tier::Relay, MS(40))),
                &[(Tier::Relay, MS(40)), (Tier::Direct, MS(60))]
            ),
            None
        );
        assert_eq!(choose(Some((Tier::Direct, MS(60))), &open), Some(0));
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
        assert_eq!(switch_margin(MS(1), false), MS(5));
        assert_eq!(
            switch_margin(MS(60), false),
            MS(5),
            "eligibility does not repeat the live path confirmation barrier"
        );
        assert_eq!(switch_margin(MS(60), true), MS(10));
        let far = [(Tier::Direct, MS(60)), (Tier::Direct, MS(48))];
        assert_eq!(
            choose(Some((Tier::Direct, MS(60))), &far),
            Some(1),
            "a measured 12 ms gain can enter fresh confirmation"
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_rejected_carrier_keeps_the_protected_stream_and_its_owner() {
        let links = Links::new(iroh::SecretKey::generate().public());
        let peer = iroh::SecretKey::generate().public();
        let via = iroh::SecretKey::generate().public();
        let (here, mut remote) = tokio::io::duplex(64);
        let (r, w) = tokio::io::split(here);
        let original = links.attach(peer, Carrier::Ssh, r, w);
        let (loser, mut rejected) = tokio::io::duplex(64);
        let (r, w) = tokio::io::split(loser);
        assert!(
            links
                .attach_if(peer, Carrier::Hop { via }, r, w, |current| {
                    current != Some(Carrier::Ssh)
                })
                .is_none()
        );
        assert_eq!(links.lock().get(&peer).unwrap().serial, original.serial());
        assert_eq!(links.last_carrier(&peer), Some(Carrier::Ssh));
        let mut byte = [0; 1];
        assert_eq!(rejected.read(&mut byte).await.unwrap(), 0);
        links
            .lock()
            .get(&peer)
            .unwrap()
            .out
            .try_send(Bytes::from_static(b"protected stream"))
            .unwrap();
        let mut bytes = [0; 16];
        tokio::time::timeout(Duration::from_secs(1), remote.read_exact(&mut bytes))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(&bytes, b"protected stream");
        assert!(links.detach_attached(&peer, Carrier::Ssh, original.serial()));
        tokio::time::timeout(Duration::from_secs(1), original.ended())
            .await
            .unwrap();
        let (next, next_remote) = tokio::io::duplex(64);
        let (r, w) = tokio::io::split(next);
        let accepted = links
            .attach_if(peer, Carrier::Hop { via }, r, w, |current| {
                current.is_none()
            })
            .unwrap();
        assert_eq!(links.carrier(&peer), Some(Carrier::Hop { via }));
        drop(next_remote);
        tokio::time::timeout(Duration::from_secs(1), accepted.ended())
            .await
            .unwrap();
    }

    /// Detaching or replacing a link must stop its owned I/O even when
    /// the peer does not drain an already-blocked write. The replacement
    /// remains usable when the old task completes.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_blocked_carrier_stops_on_detach_or_replacement() {
        let mut stopped_before_remote_closed = Vec::new();
        for replace in [false, true] {
            let links = Links::new(iroh::SecretKey::generate().public());
            let peer = iroh::SecretKey::generate().public();
            let via = iroh::SecretKey::generate().public();
            let carrier = Carrier::Hop { via };
            let (here, mut remote) = tokio::io::duplex(1);
            let (r, w) = tokio::io::split(here);
            let old = links.attach(peer, carrier, r, w);
            let serial = old.serial();
            links
                .lock()
                .get(&peer)
                .unwrap()
                .out
                .try_send(Bytes::from_static(b"blocked carrier frame"))
                .unwrap();
            let mut byte = [0u8; 1];
            tokio::time::timeout(Duration::from_secs(1), remote.read_exact(&mut byte))
                .await
                .expect("the real writer reached the one-byte pipe")
                .unwrap();
            assert_eq!(byte, [b'b']);
            let replacement = if replace {
                let (next, next_remote) = tokio::io::duplex(64);
                let (r, w) = tokio::io::split(next);
                let attached = links.attach(peer, carrier, r, w);
                assert_ne!(serial, attached.serial());
                Some((attached, next_remote))
            } else {
                assert!(links.detach_attached(&peer, carrier, serial));
                None
            };
            let ended = old.ended();
            tokio::pin!(ended);
            let stopped = tokio::time::timeout(Duration::from_secs(1), &mut ended)
                .await
                .is_ok();
            // Clean up the baseline's blocked task before asserting the
            // failure; closing this peer must not be what makes it pass.
            drop(remote);
            if !stopped {
                tokio::time::timeout(Duration::from_secs(1), &mut ended)
                    .await
                    .expect("baseline I/O ends after its blocked peer closes");
            }
            if let Some((attached, next_remote)) = replacement {
                assert_eq!(links.carrier(&peer), Some(carrier));
                assert!(
                    links.has(&peer),
                    "old completion cannot remove the replacement"
                );
                drop(next_remote);
                tokio::time::timeout(Duration::from_secs(1), attached.ended())
                    .await
                    .expect("the healthy replacement has its own lifetime");
            } else {
                assert!(!links.has(&peer));
            }
            eprintln!(
                "[carrier-blocked-owner-contract] replace={replace} old_serial={serial} blocked_write_seen=true stopped_before_remote_closed={stopped}"
            );
            stopped_before_remote_closed.push(stopped);
        }
        assert_eq!(
            stopped_before_remote_closed,
            vec![true, true],
            "both detach and replacement stop the blocked old owner without the peer's help"
        );
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
