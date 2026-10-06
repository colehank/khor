//! **What this machine writes about its own roads** (round NET-12, the
//! 我的 khor dial): for each other machine, how this one reaches it —
//! direct, by which relay, or not at all — and the round trip, kept in
//! the device table so every machine can draw the pairs it is not part
//! of.
//!
//! # The whole point is what it does not write
//!
//! A road read every beat and written every beat would be a block every
//! five seconds that every machine copies forever. The rules (the owner,
//! 2026-10-06, 「这会累积数据吗」):
//!
//! - **A change of kind is written once it has held [`SETTLE`]**: a
//!   direct/relay flap inside that window writes nothing. A machine that
//!   falls unreachable writes `none` once.
//! - **`none` is unreachable, not idle** (round NET-13): the resident's
//!   last attempt to reach that machine failed and nothing was heard
//!   since (`Link::down_ms`). A wire that closed idle — nothing asked of
//!   it for a while — writes nothing at all, and a change it interrupted
//!   starts settling again from the next live reading.
//! - **The round trip is written only when it moved by more than a
//!   quarter** of the last written value (38 → 41 no, 38 → 50 yes), and
//!   by at least [`MS_LEAST`], and then at most once per [`MS_FLOOR`] for
//!   that machine.
//! - **A machine never written is written `none` only after
//!   [`FIRST_NONE`]**: just paired or just started, it has no wire yet.
//! - **What is already there is not written again**: the book starts from
//!   this machine's own row as the table holds it.
//!
//! The numbers in the table are for the *other* pairs. A face draws this
//! machine's own roads from its live wires (`Link::rtt_ms`), never from
//! its own row read back.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use khor_core::{Road, RoadKind};

/// How long a new kind of road must hold before it is written.
pub(crate) const SETTLE: Duration = Duration::from_secs(12);

/// The least time between two round-trip writes for one machine.
pub(crate) const MS_FLOOR: Duration = Duration::from_secs(10 * 60);

/// The least a round trip must move, besides a quarter: on a LAN, 1 →
/// 2 ms is a hundred per cent and nothing at all (measured on the
/// four-machine mesh, `tests/roads_cost.rs`).
pub(crate) const MS_LEAST: u16 = 5;

/// How long a road never written must stay `none` before `none` is
/// written for it: a machine just paired, or just started, has no wire
/// for the first round or two, and that is not a machine unreachable.
pub(crate) const FIRST_NONE: Duration = Duration::from_secs(120);

/// What a beat reads off the wire to one machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Seen {
    pub road: RoadKind,
    pub relay: Option<String>,
    pub ms: Option<u16>,
}

#[derive(Debug, Clone)]
struct Written {
    road: RoadKind,
    relay: Option<String>,
    ms: Option<u16>,
    /// When `ms` was last written. A value found in the table at start
    /// counts as written then, so a restart does not rewrite every round
    /// trip at once.
    ms_at: Instant,
}

/// The book: what is in the table, and what is waiting to settle.
#[derive(Debug, Default)]
pub(crate) struct RoadBook {
    written: HashMap<String, Written>,
    waiting: HashMap<String, (RoadKind, Option<String>, Instant)>,
    started: bool,
}

impl RoadBook {
    /// Starts from what the table holds for this machine, once.
    pub(crate) fn start_from(&mut self, held: &[Road], now: Instant) {
        if self.started {
            return;
        }
        self.started = true;
        for r in held {
            self.written.insert(
                r.to.clone(),
                Written { road: r.road, relay: r.relay.clone(), ms: r.ms, ms_at: now },
            );
        }
    }

    /// One beat's reading of the road to `to`; the road to write, when
    /// one is due.
    pub(crate) fn note(&mut self, to: &str, seen: Seen, now: Instant) -> Option<Road> {
        let road = |s: &Seen| Road { to: to.to_owned(), road: s.road, relay: s.relay.clone(), ms: s.ms };
        let Some(w) = self.written.get(to) else {
            // Never written: the first reading that holds is the first
            // write, by the same settling rule — but `none` only once it
            // has held [`FIRST_NONE`].
            let hold = if seen.road == RoadKind::None { FIRST_NONE } else { SETTLE };
            return self.settle(to, &seen, now, hold).then(|| self.wrote(to, &seen, now, road(&seen)));
        };
        if w.road != seen.road || w.relay != seen.relay {
            return self.settle(to, &seen, now, SETTLE).then(|| self.wrote(to, &seen, now, road(&seen)));
        }
        self.waiting.remove(to);
        let moved = match (w.ms, seen.ms) {
            (Some(was), Some(now_ms)) => {
                let by = was.abs_diff(now_ms);
                u32::from(by) * 4 > u32::from(was) && by >= MS_LEAST
            }
            (None, Some(_)) => true,
            _ => false,
        };
        if moved && now.duration_since(w.ms_at) >= MS_FLOOR {
            return Some(self.wrote(to, &seen, now, road(&seen)));
        }
        None
    }

    /// No live wire to `to` and nothing failed: no road to write, and a
    /// change that was settling starts over from the next reading.
    pub(crate) fn idle(&mut self, to: &str) {
        self.waiting.remove(to);
    }

    /// Whether `seen`'s kind has now held for `hold`.
    fn settle(&mut self, to: &str, seen: &Seen, now: Instant, hold: Duration) -> bool {
        match self.waiting.get(to) {
            Some((road, relay, since)) if *road == seen.road && *relay == seen.relay => {
                now.duration_since(*since) >= hold
            }
            _ => {
                self.waiting.insert(to.to_owned(), (seen.road, seen.relay.clone(), now));
                false
            }
        }
    }

    fn wrote(&mut self, to: &str, seen: &Seen, now: Instant, road: Road) -> Road {
        self.waiting.remove(to);
        self.written.insert(
            to.to_owned(),
            Written { road: seen.road, relay: seen.relay.clone(), ms: seen.ms, ms_at: now },
        );
        road
    }
}

/// Roads written into the table by this process — what the cost probe
/// counts (`tests/roads_cost.rs`).
static WRITTEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub fn roads_written() -> u64 {
    WRITTEN.load(std::sync::atomic::Ordering::Relaxed)
}

impl crate::Node {
    /// **One beat of the book** (the resident's tick): every wire read,
    /// what is due written in one flush. A satellite has no road of its
    /// own (its sponsor speaks for it) and is left out.
    pub(crate) fn note_roads(&self) {
        let Ok(all) = self.devices() else { return };
        let me = self.device_str().to_owned();
        let now = Instant::now();
        let mut due = Vec::new();
        {
            let mut book = self.roadbook.lock().unwrap_or_else(|p| p.into_inner());
            if let Some(mine) = all.iter().find(|d| d.id == me) {
                book.start_from(&roads_in(mine), now);
            }
            for d in all.iter().filter(|d| d.id != me && d.sponsor.is_none()) {
                let r = self.wires.link(&d.id);
                let road = match r.road {
                    Some(khor_net::path::PathKind::Direct) => RoadKind::Direct,
                    Some(khor_net::path::PathKind::Relay) => RoadKind::Relay,
                    _ if r.down.is_some() => RoadKind::None,
                    _ => {
                        book.idle(&d.id);
                        continue;
                    }
                };
                let seen = Seen {
                    road,
                    relay: if road == RoadKind::Relay { r.relay } else { None },
                    ms: if road == RoadKind::None {
                        None
                    } else {
                        r.rtt.map(|t| t.as_micros().div_ceil(1000).min(u128::from(u16::MAX)) as u16)
                    },
                };
                if let Some(w) = book.note(&d.id, seen, now) {
                    due.push(w);
                }
            }
        }
        if due.is_empty() {
            return;
        }
        let Ok(loaded) = self.devices_loaded() else { return };
        let mut wrote = 0;
        for w in &due {
            if loaded.doc.set_road(&me, &w.to, &to_json(w)).unwrap_or(false) {
                wrote += 1;
            }
        }
        let mut store = loaded.store;
        if store.flush(&loaded.doc).is_ok() {
            WRITTEN.fetch_add(wrote, std::sync::atomic::Ordering::Relaxed);
        }
    }

    /// The roads `d` reports, as the table holds them (round NET-12): for
    /// another machine's row. This machine's own roads are its live
    /// wires (`Node::links`), never this.
    pub fn roads_of(&self, d: &khor_sync::devices::DeviceInfo) -> Vec<Road> {
        roads_in(d)
    }
}

fn roads_in(d: &khor_sync::devices::DeviceInfo) -> Vec<Road> {
    d.roads.iter().filter_map(|(to, json)| from_json(to, json)).collect()
}

/// A road as its register holds it (`khor_sync::devices`' `F_ROAD`):
/// the object without `to`, which is the key.
pub(crate) fn to_json(r: &Road) -> String {
    serde_json::json!({ "road": r.road, "relay": r.relay, "ms": r.ms }).to_string()
}

/// A register back into a road; `None` for one this build cannot read.
pub(crate) fn from_json(to: &str, json: &str) -> Option<Road> {
    let mut v: serde_json::Value = serde_json::from_str(json).ok()?;
    v.as_object_mut()?.insert("to".into(), serde_json::Value::String(to.to_owned()));
    serde_json::from_value(v).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seen(road: RoadKind, ms: Option<u16>) -> Seen {
        Seen { road, relay: None, ms }
    }

    /// A flap inside the settling window writes nothing; a kind that
    /// holds is written once.
    #[test]
    fn a_kind_is_written_once_it_has_held() {
        let t0 = Instant::now();
        let mut b = RoadBook::default();
        b.start_from(&[Road { to: "a".into(), road: RoadKind::Direct, relay: None, ms: Some(20) }], t0);
        let s = |n: u64| t0 + Duration::from_secs(n);
        assert_eq!(b.note("a", seen(RoadKind::Relay, Some(90)), s(1)), None);
        assert_eq!(b.note("a", seen(RoadKind::Direct, Some(20)), s(5)), None, "flapped back: nothing");
        assert_eq!(b.note("a", seen(RoadKind::Relay, Some(90)), s(6)), None);
        assert_eq!(b.note("a", seen(RoadKind::Relay, Some(90)), s(12)), None, "held 6 s");
        let w = b.note("a", seen(RoadKind::Relay, Some(90)), s(18)).expect("held 12 s: written");
        assert_eq!((w.road, w.ms), (RoadKind::Relay, Some(90)));
        assert_eq!(b.note("a", seen(RoadKind::Relay, Some(90)), s(40)), None, "once");
        // Unreachable: `none` once.
        assert_eq!(b.note("a", seen(RoadKind::None, None), s(41)), None);
        assert!(b.note("a", seen(RoadKind::None, None), s(53)).is_some());
        assert_eq!(b.note("a", seen(RoadKind::None, None), s(600)), None);
    }

    /// The round trip moves the table only by more than a quarter, and at
    /// most once in ten minutes.
    #[test]
    fn a_round_trip_is_written_by_a_quarter_at_most_every_ten_minutes() {
        let t0 = Instant::now();
        let mut b = RoadBook::default();
        b.start_from(&[Road { to: "a".into(), road: RoadKind::Direct, relay: None, ms: Some(38) }], t0);
        let m = |n: u64| t0 + Duration::from_secs(n * 60);
        assert_eq!(b.note("a", seen(RoadKind::Direct, Some(50)), m(5)), None, "38 -> 50 inside the floor: no");
        assert_eq!(b.note("a", seen(RoadKind::Direct, Some(41)), m(11)), None, "38 -> 41: no");
        let w = b.note("a", seen(RoadKind::Direct, Some(50)), m(11)).expect("38 -> 50 after it: yes");
        assert_eq!(w.ms, Some(50));
        assert_eq!(b.note("a", seen(RoadKind::Direct, Some(200)), m(15)), None, "the floor again");
        assert!(b.note("a", seen(RoadKind::Direct, Some(200)), m(22)).is_some());
    }

    /// A machine never written is not written `none` for the round or
    /// two before its first wire; and a LAN's 1 → 2 ms moves nothing.
    #[test]
    fn a_new_machine_is_not_written_unreachable_and_a_lan_does_not_jitter() {
        let t0 = Instant::now();
        let s = |n: u64| t0 + Duration::from_secs(n);
        let mut b = RoadBook::default();
        b.start_from(&[], t0);
        assert_eq!(b.note("new", seen(RoadKind::None, None), s(0)), None);
        assert_eq!(b.note("new", seen(RoadKind::None, None), s(30)), None, "30 s without a wire: not yet");
        assert!(b.note("new", seen(RoadKind::Direct, Some(1)), s(31)).is_none());
        assert!(b.note("new", seen(RoadKind::Direct, Some(1)), s(43)).is_some(), "its first road");
        assert_eq!(b.note("new", seen(RoadKind::Direct, Some(2)), s(43 + 3600)), None, "1 -> 2 ms: nothing");
        let mut lone = RoadBook::default();
        lone.start_from(&[], t0);
        lone.note("gone", seen(RoadKind::None, None), s(0));
        assert!(lone.note("gone", seen(RoadKind::None, None), s(120)).is_some(), "none, once it held two minutes");
    }

    /// An idle wire is no road (round NET-13): the beat notes nothing for
    /// it, and a change it interrupted settles again from scratch.
    #[test]
    fn idle_writes_nothing_and_restarts_a_settling_change() {
        let t0 = Instant::now();
        let s = |n: u64| t0 + Duration::from_secs(n);
        let mut b = RoadBook::default();
        b.start_from(&[Road { to: "a".into(), road: RoadKind::Direct, relay: None, ms: Some(20) }], t0);
        assert_eq!(b.note("a", seen(RoadKind::Relay, Some(90)), s(1)), None);
        // The wire closes idle for a minute: nothing is noted, so nothing
        // can be written — and the relay reading 60 s ago held nothing.
        b.idle("a");
        assert_eq!(b.note("a", seen(RoadKind::Relay, Some(90)), s(61)), None, "settles from the new reading");
        assert!(b.note("a", seen(RoadKind::Relay, Some(90)), s(73)).is_some(), "and is written once it held");
    }

    #[test]
    fn a_register_reads_back_as_the_road_it_was() {
        let r = Road { to: "ab".into(), road: RoadKind::Relay, relay: Some("relay.khor.cloud".into()), ms: Some(180) };
        assert_eq!(from_json("ab", &to_json(&r)), Some(r));
        assert_eq!(from_json("ab", "{\"road\":\"sideways\"}"), None, "a kind this build does not know");
    }
}
