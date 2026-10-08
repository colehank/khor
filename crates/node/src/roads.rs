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
) {
    if let Some(j) = journal.clone() {
        follow(id, conn.path_events(), conn.weak_handle(), j);
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

/// Every machine's tally, in no particular order.
/// The last path written per machine and kind of change ([`follow`]): a
/// machine's two connections (one each way) report the same selection, and
/// one line says it.
static WRITTEN: std::sync::LazyLock<Mutex<HashMap<(String, &'static str), String>>> =
    std::sync::LazyLock::new(Default::default);

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
) {
    let id = id.to_owned();
    tokio::spawn(async move {
        use n0_future::StreamExt;
        use iroh::endpoint::PathEvent;
        tokio::pin!(events);
        while let Some(event) = events.next().await {
            let (change, path_id, remote) = match event {
                PathEvent::Opened { id: pid, remote_addr, .. } if remote_addr.is_ip() => ("opened", pid, remote_addr),
                PathEvent::Selected { id: pid, remote_addr, .. } => ("selected", pid, remote_addr),
                _ => continue,
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
                _ => ("carried", String::new()),
            };
            {
                let mut written = WRITTEN.lock().unwrap_or_else(|p| p.into_inner());
                let key = (id.clone(), change);
                if written.get(&key) == Some(&addr) {
                    continue;
                }
                written.insert(key, addr.clone());
                // A new selection is news for the next opening too.
                if change == "selected" {
                    written.remove(&(id.clone(), "opened"));
                }
            }
            let rtt_ms = weak
                .upgrade()
                .and_then(|c| c.rtt(path_id))
                .map(crate::journal::ms);
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
