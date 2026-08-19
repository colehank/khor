//! A road to a machine only somebody else can reach.
//!
//! # What this is for
//!
//! A machine that lives on one LAN and nowhere else — no route out, no
//! way to reach any relay — cannot be met halfway: a relay only works
//! when *both* ends can reach it, and there is no such machine. What
//! there is, is a third machine that can reach both: it sits on that
//! LAN and can also get out. This module makes that machine a road.
//!
//! # Why at the datagram layer
//!
//! The obvious version forwards khor's own frames, and it costs two
//! things that are easy to miss: the middle machine ends up **inside
//! the conversation** rather than under it, and khor ends up writing the
//! path selection, failover and loop prevention that iroh already
//! writes. Handing iroh a transport instead keeps both where they were:
//! what crosses the middle is QUIC encrypted to an identity it does not
//! hold, and the road is simply one more candidate iroh probes in
//! parallel and keeps or drops on the measurement (`iroh` picks by RTT).
//!
//! **The far machine takes no part in this.** The exit delivers by plain
//! UDP from its own socket, so the far machine sees an ordinary peer at
//! an ordinary address and answers there. That is also what makes the
//! hop count exactly one: what leaves the exit is addressed to a host,
//! not to another khor, so there is nothing to chain and no loop to
//! prevent.
//!
//! # The shape of an address
//!
//! `via/<exit endpoint id>/<far endpoint id>` — who forwards, and **who
//! for**. Deliberately not "and to which socket address": the asking
//! machine is the one that cannot reach the far machine, so it is the
//! worst placed of the three to say where it is. The exit resolves that
//! itself, and because it does, one mechanism serves both directions —
//! a far machine on the exit's LAN is looked up in the device table, and
//! one out on the internet is looked up in the path the exit already
//! holds open to it (`iroh::Endpoint::remote_info`).

use std::{
    io,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};

use iroh::endpoint::transports::{CustomEndpoint, CustomSender, CustomTransport, RecvInfo, Transmit};
use iroh_base::CustomAddr;

/// This transport's id inside `CustomAddr`. Unregistered on purpose —
/// iroh keeps a table for transports that want to interoperate between
/// implementations, and this one only ever talks to another khor.
pub const TRANSPORT_ID: u64 = u64::from_be_bytes(*b"khor-via");

/// Bigger than any QUIC packet iroh sends and smaller than anything a
/// peer could use to make us buffer.
const DATAGRAM_MAX: usize = 2048;

/// How many datagrams may wait for a road that is still being built.
/// Small on purpose: this is UDP, and a dropped datagram is a retransmit
/// rather than an error. A deep queue would only add delay to packets
/// QUIC has already given up on.
const QUEUE: usize = 64;

/// Where a datagram is going: through `exit`, to `far`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Road {
    pub exit: iroh::EndpointId,
    pub far: iroh::EndpointId,
}

impl Road {
    /// The `CustomAddr` iroh carries for this road.
    pub fn addr(&self) -> CustomAddr {
        CustomAddr::from_parts(TRANSPORT_ID, self.to_string().as_bytes())
    }

    /// Reads one back. `None` for anything that is not ours — another
    /// transport's address, or a peer that made one up.
    pub fn of(addr: &CustomAddr) -> Option<Road> {
        if addr.id() != TRANSPORT_ID {
            return None;
        }
        let text = std::str::from_utf8(addr.data()).ok()?;
        let (exit, far) = text.strip_prefix("via/")?.split_once('/')?;
        Some(Road { exit: exit.parse().ok()?, far: far.parse().ok()? })
    }
}

impl std::fmt::Display for Road {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "via/{}/{}", self.exit, self.far)
    }
}

/// What the transport needs from khor to build a road: a stream to the
/// exit that is already speaking the tunnel handshake.
///
/// **A trait rather than a call into `khor_node`** because the dependency
/// runs the other way: this crate is the substrate, and the borrow lives
/// a layer up where the device table and the pairing rules are.
pub trait Exits: Send + Sync + 'static {
    /// Opens a datagram road to `far` through `exit`, returning the
    /// halves to pipe. Called off the send path, so it may take as long
    /// as a dial takes.
    fn open(
        &self,
        road: Road,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<(Sink, Source), String>> + Send>,
    >;
}

/// The two halves of an opened road, as the caller hands them over.
pub type Sink = tokio::sync::mpsc::Sender<Vec<u8>>;
/// Datagrams coming back, already stripped of their length prefix.
pub type Source = tokio::sync::mpsc::Receiver<Vec<u8>>;

/// The transport as iroh sees it.
#[derive(Debug)]
pub struct Via {
    exits: Arc<dyn Exits>,
}

impl Via {
    pub fn new(exits: Arc<dyn Exits>) -> Self {
        Via { exits }
    }
}

impl std::fmt::Debug for dyn Exits {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Exits")
    }
}

impl CustomTransport for Via {
    fn bind(&self) -> io::Result<Box<dyn CustomEndpoint>> {
        let (tx, rx) = tokio::sync::mpsc::channel(QUEUE);
        Ok(Box::new(ViaEndpoint {
            exits: self.exits.clone(),
            inbox: rx,
            inbox_tx: tx,
            roads: Arc::new(Mutex::new(std::collections::HashMap::new())),
            none: n0_watcher::Watchable::new(Vec::new()),
        }))
    }
}

/// One datagram that came back, and which road it came back on.
type Heard = (Road, Vec<u8>);

#[derive(Debug)]
struct ViaEndpoint {
    exits: Arc<dyn Exits>,
    inbox: tokio::sync::mpsc::Receiver<Heard>,
    inbox_tx: tokio::sync::mpsc::Sender<Heard>,
    /// Roads already built or being built, so a burst of datagrams to
    /// one machine does not open a road per packet.
    roads: Arc<Mutex<std::collections::HashMap<Road, Option<Sink>>>>,
    /// Held only so `watch_local_addrs` has something to hand back; it
    /// never changes, because this transport has no address of its own.
    none: n0_watcher::Watchable<Vec<CustomAddr>>,
}

impl CustomEndpoint for ViaEndpoint {
    /// **Nothing.** This transport dials and never answers: a road is
    /// something the asking machine builds, and there is no address on
    /// it that anyone else could send to.
    fn watch_local_addrs(&self) -> n0_watcher::Direct<Vec<CustomAddr>> {
        self.none.watch()
    }

    fn create_sender(&self) -> Arc<dyn CustomSender> {
        Arc::new(ViaSender {
            exits: self.exits.clone(),
            inbox: self.inbox_tx.clone(),
            roads: self.roads.clone(),
        })
    }

    fn poll_recv(
        &mut self,
        cx: &mut Context,
        bufs: &mut [io::IoSliceMut<'_>],
        metas: &mut [noq_udp::RecvMeta],
        recv_infos: &mut [RecvInfo],
    ) -> Poll<io::Result<usize>> {
        let mut filled = 0;
        while filled < bufs.len() {
            match self.inbox.poll_recv(cx) {
                Poll::Ready(Some((road, bytes))) => {
                    let n = bytes.len().min(bufs[filled].len());
                    bufs[filled][..n].copy_from_slice(&bytes[..n]);
                    // `addr` is the socket address a normal transport
                    // would report; ours has none, and iroh reads the
                    // remote from `RecvInfo` for custom transports. The
                    // rest of the struct is deliberately left at its
                    // "arbitrary, overwrite me" default.
                    // Built by assignment because the struct is
                    // `non_exhaustive`: a literal would stop compiling
                    // the day iroh's udp crate grows a field, which is
                    // exactly the day we would want it to keep working.
                    // `addr` stays at the default — ours has none, and
                    // iroh reads the remote from `RecvInfo` for custom
                    // transports.
                    let mut meta = noq_udp::RecvMeta::default();
                    meta.len = n;
                    meta.stride = n;
                    metas[filled] = meta;
                    recv_infos[filled] = RecvInfo::new(road.addr(), None);
                    filled += 1;
                }
                // Nothing more right now: hand up what we have, or
                // register the wake-up if there was nothing at all.
                Poll::Pending => {
                    return if filled == 0 { Poll::Pending } else { Poll::Ready(Ok(filled)) };
                }
                Poll::Ready(None) => {
                    return if filled == 0 {
                        Poll::Ready(Err(io::Error::other("via: inbox closed")))
                    } else {
                        Poll::Ready(Ok(filled))
                    };
                }
            }
        }
        Poll::Ready(Ok(filled))
    }
}

#[derive(Debug)]
struct ViaSender {
    exits: Arc<dyn Exits>,
    inbox: tokio::sync::mpsc::Sender<Heard>,
    roads: Arc<Mutex<std::collections::HashMap<Road, Option<Sink>>>>,
}

impl CustomSender for ViaSender {
    fn is_valid_send_addr(&self, addr: &CustomAddr) -> bool {
        Road::of(addr).is_some()
    }

    /// **Never waits and never fails on a road that is not up yet.**
    /// Building one means dialing the exit, which takes as long as a
    /// dial takes; blocking iroh's send path on that would stall every
    /// other path it is probing at the same time. So the first datagram
    /// starts the road and is dropped, and QUIC — which is built for a
    /// lossy road — sends it again.
    fn poll_send(
        &self,
        _cx: &mut Context,
        dst: &CustomAddr,
        _src: Option<&CustomAddr>,
        transmit: &Transmit<'_>,
    ) -> Poll<io::Result<()>> {
        let Some(road) = Road::of(dst) else {
            return Poll::Ready(Err(io::Error::other("via: not one of ours")));
        };
        if transmit.contents.len() > DATAGRAM_MAX {
            return Poll::Ready(Err(io::Error::other("via: datagram too long")));
        }
        let sink = {
            let mut roads = self.roads.lock().unwrap_or_else(|e| e.into_inner());
            match roads.get(&road) {
                Some(Some(sink)) => Some(sink.clone()),
                // Already being built. Drop, as above.
                Some(None) => None,
                None => {
                    roads.insert(road.clone(), None);
                    let (exits, inbox, all) =
                        (self.exits.clone(), self.inbox.clone(), self.roads.clone());
                    let building = road.clone();
                    tokio::spawn(async move {
                        build(exits, inbox, all, building).await;
                    });
                    None
                }
            }
        };
        if let Some(sink) = sink {
            // A full queue is a road slower than the sender; the same
            // rule applies — drop rather than wait.
            let _ = sink.try_send(transmit.contents.to_vec());
        }
        Poll::Ready(Ok(()))
    }
}

/// Builds one road and pumps what comes back into the endpoint's inbox.
/// On failure the road is forgotten, so the next datagram tries again
/// rather than the pair being written off forever.
async fn build(
    exits: Arc<dyn Exits>,
    inbox: tokio::sync::mpsc::Sender<Heard>,
    all: Arc<Mutex<std::collections::HashMap<Road, Option<Sink>>>>,
    road: Road,
) {
    let opened = exits.open(road.clone()).await;
    let (sink, mut source) = match opened {
        Ok(halves) => halves,
        Err(_) => {
            all.lock().unwrap_or_else(|e| e.into_inner()).remove(&road);
            return;
        }
    };
    all.lock().unwrap_or_else(|e| e.into_inner()).insert(road.clone(), Some(sink));
    while let Some(bytes) = source.recv().await {
        if inbox.send((road.clone(), bytes)).await.is_err() {
            break;
        }
    }
    all.lock().unwrap_or_else(|e| e.into_inner()).remove(&road);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The address is what one machine tells another to do, so it has to
    /// survive the trip verbatim — and anything that is not ours has to
    /// be refused rather than half-read.
    #[test]
    fn a_road_survives_being_written_down_and_read_back() {
        // Real keys, because an endpoint id is a curve point and not
        // any 32 bytes: a hand-written literal parsed for one value and
        // not for the next, which this test caught on its first run.
        let exit = iroh::SecretKey::generate().public();
        let far = iroh::SecretKey::generate().public();
        let road = Road { exit, far };
        let read = Road::of(&road.addr()).expect("our own address must read back");
        assert_eq!(read, road);
    }

    /// Somebody else's transport, and a malformed one of ours. The first
    /// is the reason `id` exists at all; the second is what a peer that
    /// made an address up would look like.
    #[test]
    fn an_address_that_is_not_ours_is_refused_rather_than_guessed() {
        assert!(Road::of(&CustomAddr::from_parts(TRANSPORT_ID + 1, b"via/x/y")).is_none());
        for bad in ["", "via/", "via//x", "via/notanid/alsonot", "no-prefix"] {
            assert!(
                Road::of(&CustomAddr::from_parts(TRANSPORT_ID, bad.as_bytes())).is_none(),
                "{bad:?} must not read as a road"
            );
        }
    }
}
