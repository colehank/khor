//! How this machine's recent dials went, per far machine (docs/
//! CONNECT.md 度量与验收: 首包时间、直连占比) — the resident's memory
//! behind [`khor_core::RoadTally`].
//!
//! **Recorded at the one door every dial goes through** (`Node::
//! dial_with`), so no caller has to remember to report: the sync tick,
//! every verb, every tunnel. Only dials this machine *started* are
//! counted — a connection somebody else opened has no「发起」 to time
//! from, and counting its road without its time would make the two
//! numbers describe different sets of connections.
//!
//! **Where a dial settled is judged after it had a chance to settle.**
//! A connection usually starts on the relay and moves to a direct path
//! once hole punching lands, so reading it at connect time would count
//! nearly every dial as 中继. It is watched for [`SETTLE`]; the first
//! moment a direct path carries it, it is 直连; if none does within the
//! window, it is whatever road it was last seen on. A connection that
//! closes before any road was reported is not counted at all — nothing
//! was learned from it.
//!
//! In memory only. A restart starts every tally again, which is the
//! honest answer for a network that may have changed under the restart.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use khor_core::RoadTally;
use khor_net::path::{PathKind, classify};

/// How many settled dials per machine the tally counts.
pub(crate) const REMEMBERED: usize = 20;

/// How long a fresh connection is watched for a direct path before its
/// road is taken as final. Hole punching that lands at all lands in
/// well under this on the networks measured (NET.md 走的是哪条路).
pub(crate) const SETTLE: Duration = Duration::from_secs(10);

/// One far machine's history.
#[derive(Debug, Default)]
pub(crate) struct RoadLog {
    first_reply_ms: Option<u64>,
    settled: VecDeque<PathKind>,
}

/// Every far machine's history, keyed by device id.
pub(crate) type Roads = Arc<Mutex<HashMap<String, RoadLog>>>;

/// A dial to `id` came back after `took`.
pub(crate) fn replied(roads: &Roads, id: &str, took: Duration) {
    if let Ok(mut map) = roads.lock() {
        map.entry(id.to_owned()).or_default().first_reply_ms =
            Some(u64::try_from(took.as_millis()).unwrap_or(u64::MAX));
    }
}

/// A dial to `id` settled on `kind`. `Unknown` is dropped here, not
/// counted as either — it is the absence of a reading.
pub(crate) fn settled(roads: &Roads, id: &str, kind: PathKind) {
    if kind == PathKind::Unknown {
        return;
    }
    if let Ok(mut map) = roads.lock() {
        let log = map.entry(id.to_owned()).or_default();
        log.settled.push_back(kind);
        while log.settled.len() > REMEMBERED {
            log.settled.pop_front();
        }
    }
}

/// Watches a fresh connection settle, without keeping it alive: the
/// handle is weak, so a caller that drops its connection closes it on
/// time — the watch never becomes the reason a pipe stays open.
///
/// With a `journal`, where it settled is written there too: the road, the
/// relay, the round trip, and every path iroh held at that moment.
pub(crate) fn watch(
    roads: &Roads,
    id: &str,
    conn: &iroh::endpoint::Connection,
    journal: Option<Arc<crate::journal::Journal>>,
    carry: &khor_net::carry::Links,
) {
    if let Some(j) = journal.clone() {
        follow(id, conn.path_events(), conn.weak_handle(), j, carry.clone());
    }
    let (roads, id, weak) = (roads.clone(), id.to_owned(), conn.weak_handle());
    tokio::spawn(async move {
        let deadline = tokio::time::Instant::now() + SETTLE;
        let mut last = PathKind::Unknown;
        // The facts behind the verdict, read off the connection the
        // moment it is reached.
        let mut facts: Option<(Option<String>, u64, Vec<String>)> = None;
        let read = |conn: &iroh::endpoint::Connection| {
            let paths = khor_net::path::path_facts(conn)
                .into_iter()
                .map(|f| {
                    let mark = if f.selected { "*" } else { "" };
                    format!("{mark}{} {}ms", f.addr, f.rtt_ms)
                })
                .collect();
            (
                khor_net::path::relay_name(conn),
                crate::journal::ms(crate::wires::rtt_of(conn)),
                paths,
            )
        };
        while tokio::time::Instant::now() < deadline {
            let Some(conn) = weak.upgrade() else { break };
            if conn.close_reason().is_some() {
                break;
            }
            match classify(&conn) {
                PathKind::Direct => {
                    last = PathKind::Direct;
                    facts = Some(read(&conn));
                    break;
                }
                PathKind::Unknown => {}
                kind => {
                    last = kind;
                    facts = Some(read(&conn));
                }
            }
            drop(conn);
            // The sampling interval is the precision of "when did it go
            // direct", which nothing here reports — only whether. A
            // quarter second is plenty and costs nothing.
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        settled(&roads, &id, last);
        if let (Some(j), Some((relay, rtt_ms, paths))) = (journal, facts) {
            j.note(
                Some(&id),
                crate::journal::What::Settled {
                    road: crate::journal::road_key(last).to_owned(),
                    relay,
                    rtt_ms,
                    paths,
                },
            );
        }
    });
}

/// **A connection another machine opened to this one, followed too**
/// ([`follow`]): it rides the same paths as this machine's own (iroh keeps
/// one selection per machine), and a move made while only it was open —
/// a phone dialling in, a wire of this side's retired — was written
/// nowhere. The same lines, deduplicated across the two.
pub(crate) fn follow_inbound(
    id: &str,
    conn: &iroh::endpoint::Connection,
    journal: Option<Arc<crate::journal::Journal>>,
    carry: &khor_net::carry::Links,
) {
    if let Some(j) = journal {
        follow(id, conn.path_events(), conn.weak_handle(), j, carry.clone());
    }
}

/// Every machine's tally, in no particular order.
/// The last path written per machine and kind of change ([`follow`]): a
/// machine's two connections (one each way) report the same selection, and
/// one line says it.
static WRITTEN: std::sync::LazyLock<Mutex<HashMap<(String, &'static str), String>>> =
    std::sync::LazyLock::new(Default::default);

/// **A direct path is written when it opens, once** (net-i): per machine
/// and address, how many paths to it are open across all its connections,
/// and when its opening was last written. Before this, 「直连路径已打通」
/// came every ~5 s for the same two addresses (turing, 2026-10-09: 61
/// lines an hour): each of a machine's connections opens its own path to
/// the same address, and every punching round opens more. An opening is
/// written when the first path to that address opens and none was written
/// in the last [`REOPEN_QUIET`]. An address with no path open is forgotten
/// once that quiet is up (net-flap, 2026-10-21: every address ever punched
/// stayed, a machine's whole history of candidates).
static OPEN: std::sync::LazyLock<Mutex<Openings>> = std::sync::LazyLock::new(Default::default);

/// How long an address that closed and opened again stays unwritten.
const REOPEN_QUIET: Duration = Duration::from_secs(10 * 60);

/// Per machine and address: paths open, and when its opening was last written ([`OPEN`]).
#[derive(Default)]
struct Openings(HashMap<(String, String), (u32, std::time::Instant)>);

impl Openings {
    fn opened(&mut self, id: &str, addr: &str, now: std::time::Instant) -> bool {
        self.0.retain(|_, (count, last)| *count > 0 || now.duration_since(*last) < REOPEN_QUIET);
        match self.0.get_mut(&(id.to_owned(), addr.to_owned())) {
            Some((count, last)) => {
                *count += 1;
                let news = *count == 1 && now.duration_since(*last) >= REOPEN_QUIET;
                if news {
                    *last = now;
                }
                news
            }
            None => {
                self.0.insert((id.to_owned(), addr.to_owned()), (1, now));
                true
            }
        }
    }

    fn closed(&mut self, id: &str, addr: &str, now: std::time::Instant) {
        let key = (id.to_owned(), addr.to_owned());
        if let Some((count, last)) = self.0.get_mut(&key) {
            *count = count.saturating_sub(1);
            if *count == 0 && now.duration_since(*last) >= REOPEN_QUIET {
                self.0.remove(&key);
            }
        }
    }
}

/// Counts a direct path to `addr` opening; whether it is news.
fn opened_is_news(id: &str, addr: &str) -> bool {
    OPEN.lock().unwrap_or_else(|p| p.into_inner()).opened(id, addr, std::time::Instant::now())
}

/// Counts a direct path to `addr` closing (or its connection ending).
fn closed(id: &str, addr: &str) {
    OPEN.lock().unwrap_or_else(|p| p.into_inner()).closed(id, addr, std::time::Instant::now());
}

/// **Every path change of a connection, for its whole life** (NET-DIRECT):
/// a direct path opening is a hole punch that landed, a selection is the
/// road moving — relay to direct, direct back to relay, onto a carried road.
/// [`watch`]'s verdict covers only the first [`SETTLE`]; iroh keeps
/// punching every 60 s after it, and what that finds was written nowhere.
/// The stream ends when the connection closes; nothing is polled.
fn follow(
    id: &str,
    events: iroh::endpoint::PathEventStream,
    weak: iroh::endpoint::WeakConnectionHandle,
    journal: Arc<crate::journal::Journal>,
    carry: khor_net::carry::Links,
) {
    let id = id.to_owned();
    tokio::spawn(async move {
        use n0_future::StreamExt;
        use iroh::endpoint::PathEvent;
        tokio::pin!(events);
        // This connection's open direct paths, handed back when it ends.
        let mut mine: HashMap<iroh::endpoint::PathId, String> = HashMap::new();
        // The selected path as it stands, read off the connection.
        let selected_now = || {
            let conn = weak.upgrade()?;
            let paths = conn.paths();
            let p = paths.iter().find(|p| p.is_selected())?;
            Some(("selected", p.id(), p.remote_addr().clone()))
        };
        // **The road it is on when the watch begins**: iroh selects the
        // first path while the connection is being added, before anyone can
        // subscribe, so a connection that never moved wrote no road at all
        // (2026-10-16). **Written as where it began, not as a move**
        // (flap, 2026-10-23): a new connection begins on the relay while
        // the machine's other connection rides the hop, and its first line
        // read 「改走中继」 against the machine's last 「改为通过 turing」 —
        // a move back and forth the machine never made.
        let mut first = selected_now().map(|(_, pid, remote)| ("started", pid, remote));
        loop {
            let (change, path_id, remote) = if let Some(now) = first.take() {
                now
            } else {
                let Some(event) = events.next().await else { break };
                match event {
                    // **Dropped events leave a trace** (vendor/iroh patch
                    // 21): how many, then the selection as it stands, so the
                    // road it moved to is not lost with them.
                    PathEvent::Lagged { missed, .. } => {
                        journal.note(Some(&id), crate::journal::What::PathsMissed { missed });
                        let Some(now) = selected_now() else { continue };
                        now
                    }
                    PathEvent::Opened { id: pid, remote_addr, .. } if remote_addr.is_ip() => ("opened", pid, remote_addr),
                    PathEvent::Selected { id: pid, remote_addr, .. } => ("selected", pid, remote_addr),
                    PathEvent::Closed { id: pid, .. } => {
                        if let Some(addr) = mine.remove(&pid) {
                            closed(&id, &addr);
                        }
                        continue;
                    }
                    _ => continue,
                }
            };
            let (road, addr) = match &remote {
                iroh::TransportAddr::Ip(sa) => ("direct", sa.to_string()),
                iroh::TransportAddr::Relay(url) => (
                    "relay",
                    url.as_str()
                        .trim_end_matches('/')
                        .split("://")
                        .last()
                        .unwrap_or_default()
                        .to_owned(),
                ),
                // A carried road names what carries it (net-e): the
                // middle's id for a peer hop, `ssh` for ssh — the line
                // said 「改走转连：」 with nothing after the colon.
                _ => (
                    "carried",
                    match id.parse().ok().and_then(|p| carry.last_carrier(&p)) {
                        Some(khor_net::carry::Carrier::Hop { via }) => via.to_string(),
                        Some(khor_net::carry::Carrier::Ssh) => "ssh".to_owned(),
                        None => String::new(),
                    },
                ),
            };
            if change == "opened" {
                mine.insert(path_id, addr.clone());
                if !opened_is_news(&id, &addr) {
                    continue;
                }
            } else {
                // Where a connection began and where it moved are the one
                // road of the machine's, deduplicated together.
                let mut written = WRITTEN.lock().unwrap_or_else(|p| p.into_inner());
                let key = (id.clone(), if change == "started" { "selected" } else { change });
                if written.get(&key) == Some(&addr) {
                    continue;
                }
                written.insert(key, addr.clone());
            }
            // A carried path just selected has no sample yet and reads 0:
            // said as nothing rather than 「往返 0 ms」.
            let rtt_ms = weak
                .upgrade()
                .and_then(|c| c.rtt(path_id))
                .map(crate::journal::ms)
                .filter(|ms| road != "carried" || *ms > 0);
            journal.note(
                Some(&id),
                crate::journal::What::Path {
                    change: change.to_owned(),
                    road: road.to_owned(),
                    addr,
                    rtt_ms,
                },
            );
        }
        for addr in mine.values() {
            closed(&id, addr);
        }
    });
}

pub(crate) fn tally(roads: &Roads) -> Vec<(String, RoadTally)> {
    let Ok(map) = roads.lock() else {
        return Vec::new();
    };
    map.iter()
        .map(|(id, log)| {
            let count = |k: PathKind| {
                u32::try_from(log.settled.iter().filter(|s| **s == k).count()).unwrap_or(u32::MAX)
            };
            (
                id.clone(),
                RoadTally {
                    first_reply_ms: log.first_reply_ms,
                    direct: count(PathKind::Direct),
                    relay: count(PathKind::Relay),
                },
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// net-i: a machine's second connection opening a path to the same
    /// address is not written again, nor is the address closing and
    /// opening again within the quiet; a new address is.
    #[test]
    fn a_direct_path_is_written_once_per_address() {
        let id = "net-i-once";
        assert!(opened_is_news(id, "10.0.0.1:1"), "the first opening is news");
        assert!(!opened_is_news(id, "10.0.0.1:1"), "the other connection's path to it is not");
        closed(id, "10.0.0.1:1");
        closed(id, "10.0.0.1:1");
        assert!(!opened_is_news(id, "10.0.0.1:1"), "closed and opened again at once is not");
        assert!(opened_is_news(id, "[2001:db8::1]:1"), "another address is");
    }

    /// net-flap: an address with nothing open is forgotten once the quiet
    /// is up, and its next opening is news as before.
    #[test]
    fn an_address_closed_for_the_quiet_is_forgotten() {
        let mut open = Openings::default();
        let t0 = std::time::Instant::now();
        assert!(open.opened("m", "10.0.0.1:1", t0));
        open.closed("m", "10.0.0.1:1", t0 + Duration::from_secs(1));
        assert_eq!(open.0.len(), 1, "within the quiet it is kept");
        let later = t0 + REOPEN_QUIET + Duration::from_secs(1);
        assert!(open.opened("m", "10.0.0.2:1", later), "another address");
        assert_eq!(open.0.len(), 1, "the closed one is gone");
        assert!(open.opened("m", "10.0.0.1:1", later), "and is news again");
        open.closed("m", "10.0.0.2:1", later + REOPEN_QUIET);
        assert_eq!(open.0.len(), 1, "a close after the quiet removes it at once");
    }

    /// The tally keeps the last [`REMEMBERED`] settled dials, drops the
    /// unknowns, and keeps the most recent reply time.
    #[test]
    fn the_tally_counts_the_recent_settled_dials_and_the_last_reply() {
        let roads: Roads = Arc::default();
        replied(&roads, "a", Duration::from_millis(900));
        replied(&roads, "a", Duration::from_millis(42));
        for _ in 0..REMEMBERED {
            settled(&roads, "a", PathKind::Relay);
        }
        settled(&roads, "a", PathKind::Unknown);
        for _ in 0..3 {
            settled(&roads, "a", PathKind::Direct);
        }
        let got = tally(&roads);
        assert_eq!(
            got,
            vec![(
                "a".to_string(),
                RoadTally {
                    first_reply_ms: Some(42),
                    direct: 3,
                    relay: REMEMBERED as u32 - 3,
                }
            )],
            "the oldest three relay dials fall off, the unknown is not counted"
        );
    }
}
