//! **The peer hop** (round ROAD-3, audit/ROAD-BRIEF.md T6): two machines
//! with no direct road meet through a third khor machine that reaches
//! both. The middle splices two streams and sees only ciphertext: the two
//! ends run iroh's own handshake through it, as themselves, over the
//! carried transport (`khor_net::carry`, a link of `Carrier::Hop`).
//!
//! # The asker (X), the middle (B), the far end (Y)
//!
//! - **X**, about to dial Y with no link to it and no recent direct road
//!   ([`Node::hop_ahead`]), picks a middle from the roadbook ([`middles`])
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
use crate::tunnel::{NO_ROUTE, OK, REFUSED};
use crate::wires::Proto;

/// X to B: splice me to this machine.
const ASK: &str = "hop ";
/// B to Y: this stream is a hop from this machine.
const HAND: &str = "hop-in ";

/// What X gives the hop before dialling without it: a middle's wire
/// that has to be dialled first, plus its own stream to Y.
const HOP_WITHIN: Duration = Duration::from_secs(3);

/// How long B gives its own wire to Y.
const MIDDLE_WITHIN: Duration = Duration::from_secs(5);

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

static LAST_COMPARE: std::sync::Mutex<Option<std::time::Instant>> = std::sync::Mutex::new(None);

/// Per (machine, middle): when a hop through it lost to the relay.
static LOST: std::sync::LazyLock<std::sync::Mutex<std::collections::HashMap<(String, String), std::time::Instant>>> =
    std::sync::LazyLock::new(Default::default);

fn lost(to: &str, via: &str) {
    LOST.lock().unwrap_or_else(|p| p.into_inner()).insert((to.to_owned(), via.to_owned()), std::time::Instant::now());
}

fn was_lost(to: &str, via: &str) -> bool {
    LOST.lock()
        .unwrap_or_else(|p| p.into_inner())
        .get(&(to.to_owned(), via.to_owned()))
        .is_some_and(|t| t.elapsed() < LOST_FOR)
}

/// Whether a hop of `hop_ms` (its legs added) is worth opening against a
/// relay measured at `relay_ms`: faster by [`khor_net::carry::SWITCH_MARGIN`].
pub(crate) fn beats(hop_ms: u32, relay_ms: u32) -> bool {
    let margin = u32::try_from(khor_net::carry::SWITCH_MARGIN.as_millis()).unwrap_or(10);
    hop_ms.saturating_add(margin) <= relay_ms
}

/// A tunnel stream's first words, when they ask for a hop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Ask {
    /// From the asker: splice me to this machine.
    To(EndpointId),
    /// From a middle: this stream comes from this machine.
    From(EndpointId),
}

impl Ask {
    pub(crate) fn of(target: &str) -> Option<Ask> {
        if let Some(id) = target.strip_prefix(HAND) {
            return id.parse().ok().map(Ask::From);
        }
        target.strip_prefix(ASK)?.parse().ok().map(Ask::To)
    }

    fn words(self) -> String {
        match self {
            Ask::To(id) => format!("{ASK}{id}"),
            Ask::From(id) => format!("{HAND}{id}"),
        }
    }
}

/// **The middles for a hop from `me` to `to`, best first** (pure, for the
/// roadbook rules): a machine that is not either end, not a satellite and
/// not paused here, whose own written road to `to` is direct or its ssh
/// (it carries `to`, round ROAD-2), and that `me` reaches itself.
///
/// - `me` must reach the middle directly — or through the relay when the
///   middle carries `to` over ssh: then nothing else reaches `to` at all,
///   whereas the relay to a middle that reaches `to` directly is no
///   better than the relay to `to`.
/// - Neither leg may be a hop itself (`via`): one hop only.
/// - Order: a direct first leg first, then the two legs' round trips.
///
/// `rows` are (id, satellite, paused); `mine(b)` is this machine's road
/// to `b`; `theirs(b)` is `b`'s written road to `to`.
pub(crate) fn middles(
    me: &str,
    to: &str,
    rows: &[(String, bool, bool)],
    mine: impl Fn(&str) -> Option<Road>,
    theirs: impl Fn(&str) -> Option<Road>,
) -> Vec<String> {
    middles_with_cost(me, to, rows, mine, theirs).into_iter().map(|(b, _)| b).collect()
}

/// [`middles`] with each one's cost: its two legs' round trips added, in
/// ms (`u16::MAX` for a leg with none written) — the measure a hop is
/// held against the relay by ([`Node::compare_roads`]).
pub(crate) fn middles_with_cost(
    me: &str,
    to: &str,
    rows: &[(String, bool, bool)],
    mine: impl Fn(&str) -> Option<Road>,
    theirs: impl Fn(&str) -> Option<Road>,
) -> Vec<(String, u32)> {
    let mut found: Vec<(bool, u32, String)> = rows
        .iter()
        .filter(|(id, sat, paused)| id != me && id != to && !sat && !paused)
        .filter_map(|(b, _, _)| {
            let second = theirs(b).filter(|r| r.via.is_none())?;
            let first = mine(b).filter(|r| r.via.is_none())?;
            let usable = match (first.road, second.road) {
                (RoadKind::Direct, RoadKind::Direct | RoadKind::Ssh) => true,
                (RoadKind::Relay, RoadKind::Ssh) => true,
                _ => false,
            };
            let ms = u32::from(first.ms.unwrap_or(u16::MAX)) + u32::from(second.ms.unwrap_or(u16::MAX));
            usable.then(|| (first.road != RoadKind::Direct, ms, b.clone()))
        })
        .collect();
    found.sort();
    found.into_iter().map(|(_, ms, b)| (b, ms)).collect()
}

impl Node {
    /// This machine's road to `b` for picking a middle: its live wire
    /// when one is up (a carried one is no first leg), else what it last
    /// wrote in its own row.
    fn road_for_hop(&self, b: &str, mine: &[Road]) -> Option<Road> {
        let r = self.wires.link(b);
        let ms = r.rtt.map(|t| t.as_millis().min(u128::from(u16::MAX)) as u16);
        match r.road {
            Some(khor_net::path::PathKind::Direct) => return Some(Road::own(b, RoadKind::Direct, None, ms)),
            Some(khor_net::path::PathKind::Relay) => return Some(Road::own(b, RoadKind::Relay, r.relay, ms)),
            Some(khor_net::path::PathKind::Carried) => return None,
            _ => {}
        }
        if r.down.is_some() || self.carry.carrier(&b.parse().ok()?).is_some() {
            return None;
        }
        mine.iter().find(|w| w.to == b).cloned()
    }

    /// Whether X should look for a hop to `y` before dialling it: the
    /// switch is on, no link to `y` is up, `y` is a member, and its last
    /// road from here was not direct — or the last try at it failed.
    fn wants_hop(&self, y: &crate::DeviceInfo, mine: &[Road]) -> bool {
        let Ok(peer) = y.id.parse::<EndpointId>() else { return false };
        if !self.hop_on() || self.carrying().is_none() || self.carry.has(&peer) || y.sponsor.is_some() {
            return false;
        }
        let r = self.wires.link(&y.id);
        if r.down.is_some() {
            return true;
        }
        let direct = |k: Option<RoadKind>| k == Some(RoadKind::Direct);
        let last = match r.last_road {
            Some(khor_net::path::PathKind::Direct) => Some(RoadKind::Direct),
            _ => None,
        };
        !direct(last) && !direct(mine.iter().find(|w| w.to == y.id && w.via.is_none()).map(|w| w.road))
    }

    /// **X's half**: before dialling `y`, open a hop to it through a
    /// middle when the roadbook names one. Quietly nothing when no hop is
    /// wanted, none can be had, or it takes past [`HOP_WITHIN`]: the dial
    /// goes on without it.
    pub(crate) async fn hop_ahead(&self, ep: &iroh::Endpoint, y: &crate::DeviceInfo) {
        // The cheap answers first: this runs before every dial.
        if !self.hop_on() || y.id.parse().is_ok_and(|p| self.carry.has(&p)) {
            return;
        }
        let Ok(all) = self.devices() else { return };
        let me = self.device_str().to_owned();
        let mine = all.iter().find(|d| d.id == me).map(|d| self.roads_of(d)).unwrap_or_default();
        if !self.wants_hop(y, &mine) {
            return;
        }
        let rows: Vec<(String, bool, bool)> = all
            .iter()
            .map(|d| (d.id.clone(), d.sponsor.is_some(), crate::pause::peer_on(self.root(), &d.id)))
            .collect();
        let picked = middles(
            &me,
            &y.id,
            &rows,
            |b| self.road_for_hop(b, &mine),
            |b| all.iter().find(|d| d.id == b).and_then(|d| self.roads_of(d).into_iter().find(|r| r.to == y.id)),
        );
        self.open_hop(ep, y, &all, &picked).await;
    }

    /// **Opens a hop to `y` through the first of `picked` that answers**
    /// (at most [`TRIES`], within [`HOP_WITHIN`]), hands iroh the carried
    /// address (vendor/iroh patch 12: opened at once on a live connection)
    /// and watches it: the hop is let go once direct, or the relay, has
    /// carried the connection for two looks — the selector
    /// (`carry::RoadOrder`) found it no faster. `true` when a hop is up.
    async fn open_hop(&self, ep: &iroh::Endpoint, y: &crate::DeviceInfo, all: &[crate::DeviceInfo], picked: &[String]) -> bool {
        let Ok(peer) = y.id.parse::<EndpointId>() else { return false };
        let tries = async {
            for b in picked.iter().take(TRIES) {
                let Some(middle) = all.iter().find(|d| &d.id == b) else { continue };
                let Ok(via) = b.parse::<EndpointId>() else { continue };
                // Boxed: the middle's wire is a dial, and a dial may hop.
                let line = Box::pin(crate::journal::BY.scope(
                    crate::journal::By::Hop,
                    self.line_to_inner(ep, middle, Proto::Tunnel, None, false),
                ));
                let Ok(line) = line.await else {
                    continue;
                };
                if let Ok((send, recv)) = crate::tunnel::dial(&line.conn, &Ask::To(peer).words()).await {
                    return Some((via, send, recv));
                }
            }
            None
        };
        let tried = u32::try_from(picked.len().min(TRIES)).unwrap_or(u32::MAX);
        let Ok(Some((via, send, recv))) = tokio::time::timeout(HOP_WITHIN, tries).await else {
            if tried > 0 {
                self.wires.note(Some(&y.id), crate::journal::What::Hop { via: None, tried });
            }
            return false;
        };
        self.wires.note(
            Some(&y.id),
            crate::journal::What::Hop {
                via: Some(via.to_string()),
                tried,
            },
        );
        let attached = self.carry.attach(peer, Carrier::Hop { via }, recv, send);
        if let Some(addr) = self.carried_addr(&y.id) {
            ep.offer_addrs(iroh::EndpointAddr::from_parts(peer, [addr])).await;
        }
        let (links, wires, id, ep) = (self.carry.clone(), self.wires.clone(), y.id.clone(), ep.clone());
        // The wire to the middle is judged on a hop's patience while the
        // hop lives (`wires::HOP_SILENT`): a frozen middle hands back sooner.
        let riding = wires.hop_held(&via.to_string());
        tokio::spawn(async move {
            let _riding = riding;
            let ended = attached.ended();
            tokio::pin!(ended);
            let mut held = 0u8;
            loop {
                tokio::select! {
                    () = &mut ended => {
                        // The road under the carried path is gone: iroh
                        // chooses again now, not when the path idles out
                        // (vendor/iroh patch 13). A newer link to the same
                        // machine rides that path, so it is left.
                        if !links.has(&peer) {
                            ep.close_carried(peer).await;
                        }
                        return;
                    }
                    () = tokio::time::sleep(LOOK_EVERY) => {}
                }
                // Direct took over, or the relay kept it (the hop is no
                // faster), for two looks: the middle is let go, so it
                // carries nothing it need not.
                let road = wires.link(&id).road;
                held = if matches!(road, Some(khor_net::path::PathKind::Direct | khor_net::path::PathKind::Relay)) {
                    held + 1
                } else {
                    0
                };
                if held >= 2 {
                    if road == Some(khor_net::path::PathKind::Relay) {
                        lost(&id, &via.to_string());
                    }
                    links.detach(&peer, Carrier::Hop { via });
                }
            }
        });
        true
    }

    /// **The road choice by measured legs** (NET-DIRECT step 4), every
    /// [`COMPARE_EVERY`] from the resident's beat: for each machine whose
    /// live wire is on the relay, the best middle's two legs added
    /// ([`middles_with_cost`]: this machine's live round trip to the
    /// middle, the middle's written one onward) against the relay's
    /// measured round trip. A hop at least [`khor_net::carry::SWITCH_MARGIN`]
    /// faster is opened; iroh then measures it end to end and the selector
    /// keeps whichever is faster, and a hop that loses is let go
    /// ([`Node::open_hop`]) and not tried through that middle again for
    /// [`LOST_FOR`]. Direct needs nothing here: iroh's punching runs under
    /// every road and takes over when it opens.
    pub(crate) async fn compare_roads(&self, ep: &iroh::Endpoint) {
        {
            let mut last = LAST_COMPARE.lock().unwrap_or_else(|p| p.into_inner());
            if last.is_some_and(|t| t.elapsed() < COMPARE_EVERY) {
                return;
            }
            *last = Some(std::time::Instant::now());
        }
        if !self.hop_on() || self.carrying().is_none() {
            return;
        }
        let Ok(all) = self.devices() else { return };
        let me = self.device_str().to_owned();
        let mine = all.iter().find(|d| d.id == me).map(|d| self.roads_of(d)).unwrap_or_default();
        let rows: Vec<(String, bool, bool)> = all
            .iter()
            .map(|d| (d.id.clone(), d.sponsor.is_some(), crate::pause::peer_on(self.root(), &d.id)))
            .collect();
        for y in all.iter().filter(|d| d.id != me && d.sponsor.is_none()) {
            let Ok(peer) = y.id.parse::<EndpointId>() else { continue };
            if self.carry.has(&peer) || crate::pause::peer_on(self.root(), &y.id) {
                continue;
            }
            let r = self.wires.link(&y.id);
            let (Some(khor_net::path::PathKind::Relay), Some(rtt)) = (r.road, r.rtt) else { continue };
            let relay_ms = u32::try_from(rtt.as_millis()).unwrap_or(u32::MAX);
            let picked: Vec<String> = middles_with_cost(
                &me,
                &y.id,
                &rows,
                |b| self.road_for_hop(b, &mine),
                |b| all.iter().find(|d| d.id == b).and_then(|d| self.roads_of(d).into_iter().find(|r| r.to == y.id)),
            )
            .into_iter()
            .filter(|(b, ms)| {
                beats(*ms, relay_ms) && !was_lost(&y.id, b)
            })
            .map(|(b, _)| b)
            .collect();
            if !picked.is_empty() {
                self.open_hop(ep, y, &all, &picked).await;
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
        ask: Ask,
        mut send: SendStream,
        recv: RecvStream,
        paired: bool,
    ) {
        let refuse = |mut send: SendStream, byte: u8| async move {
            let _ = send.write_all(&[byte]).await;
            let _ = send.finish();
        };
        let Ok(from_id) = from.parse::<EndpointId>() else { return };
        if !paired || !self.hop_on() || self.carrying().is_none() {
            return refuse(send, REFUSED).await;
        }
        let me = self.device_str().to_owned();
        let member = |id: &str| {
            id != me
                && id != from
                && !crate::pause::peer_on(self.root(), id)
                && self.admit(id).is_ok()
                && self.devices().is_ok_and(|all| all.iter().any(|d| d.id == id && d.sponsor.is_none()))
        };
        match ask {
            // B: splice `from` (X) to `to` (Y).
            Ask::To(to) => {
                let y = to.to_string();
                if !member(&y) || matches!(self.carry.carrier(&to), Some(Carrier::Hop { .. })) {
                    return refuse(send, REFUSED).await;
                }
                let Some(d) = self.devices().ok().and_then(|all| all.into_iter().find(|d| d.id == y)) else {
                    return refuse(send, REFUSED).await;
                };
                let by = std::time::Instant::now() + MIDDLE_WITHIN;
                let onward = async {
                    let line = Box::pin(self.line_to_inner(&ep, &d, Proto::Tunnel, Some(by), false)).await.ok()?;
                    crate::tunnel::dial(&line.conn, &Ask::From(from_id).words()).await.ok()
                };
                let Ok(Some((ys, yr))) = tokio::time::timeout(MIDDLE_WITHIN, onward).await else {
                    return refuse(send, NO_ROUTE).await;
                };
                if send.write_all(&[OK]).await.is_err() {
                    return;
                }
                let _riding = (self.wires.hop_held(from), self.wires.hop_held(&y));
                splice((send, recv), (ys, yr)).await;
            }
            // Y: this stream is the link to `from`'s asker (X), via B.
            Ask::From(x) => {
                let asker = x.to_string();
                if !member(&asker) || matches!(self.carry.carrier(&x), Some(Carrier::Ssh)) {
                    return refuse(send, REFUSED).await;
                }
                if send.write_all(&[OK]).await.is_err() {
                    return;
                }
                let attached = self.carry.attach(x, Carrier::Hop { via: from_id }, recv, send);
                if let Some(addr) = self.carried_addr(&asker) {
                    ep.offer_addrs(iroh::EndpointAddr::from_parts(x, [addr])).await;
                }
                self.wires.forgive(&asker);
                let _riding = self.wires.hop_held(from);
                attached.ended().await;
                if !self.carry.has(&x) {
                    ep.close_carried(x).await;
                }
            }
        }
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
mod tests {
    use super::*;

    fn road(to: &str, k: RoadKind, ms: u16) -> Road {
        Road::own(to, k, None, Some(ms))
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

    /// The roadbook rules: both legs known, neither a hop; direct first
    /// leg first; the relay as a first leg only to a middle that carries
    /// the far end over ssh.
    /// NET-DIRECT step 4, MacLab's night of 10-09: the iPhone on the relay
    /// at 58 ms, turing reached directly at 3 ms and writing a direct road
    /// to the iPhone at 26 ms — the hop costs 29 ms and is opened; at a
    /// relay of 35 ms it would save under 10 ms and is not.
    #[test]
    fn a_hop_is_worth_opening_when_its_legs_beat_the_relay_by_ten_ms() {
        let rows: Vec<(String, bool, bool)> =
            ["maclab", "iphone", "turing", "hinton"].iter().map(|s| (s.to_string(), false, false)).collect();
        let mine = |b: &str| match b {
            "turing" => Some(Road::own(b, RoadKind::Direct, None, Some(3))),
            "hinton" => Some(Road::own(b, RoadKind::Direct, None, Some(2))),
            _ => None,
        };
        let theirs = |b: &str| match b {
            "turing" => Some(Road::own("iphone", RoadKind::Direct, None, Some(26))),
            "hinton" => Some(Road::own("iphone", RoadKind::Relay, Some("101.200.80.77:3340".into()), Some(37))),
            _ => None,
        };
        let found = middles_with_cost("maclab", "iphone", &rows, mine, theirs);
        assert_eq!(found, vec![("turing".to_owned(), 29)], "hinton reaches the iPhone only by relay: no middle");
        assert!(beats(29, 58));
        assert!(!beats(29, 35), "6 ms is not worth a middle");
        assert!(beats(29, 39));
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
        assert_eq!(middles("x", "y", &rows, mine, theirs), vec!["b", "c"], "40 ms before 55; d only by relay");

        let carries = |b: &str| (b == "d").then(|| road("y", RoadKind::Ssh, 3));
        assert_eq!(middles("x", "y", &rows, mine, carries), vec!["d"], "the relay to the machine that carries y");

        let forwarded = |b: &str| Some(Road { via: Some("z".into()), ..road("y", RoadKind::Direct, 1) }).filter(|_| b == "b");
        assert!(middles("x", "y", &rows, mine, forwarded).is_empty(), "a middle whose own road is a hop");
        let relayed = |_: &str| Some(road("y", RoadKind::Relay, 1));
        assert!(middles("x", "y", &rows, mine, relayed).is_empty(), "a middle on the relay adds nothing");
        assert!(middles("x", "y", &rows, |_| None, theirs).is_empty(), "no road of mine to it");
    }
}
