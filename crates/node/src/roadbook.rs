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

use khor_core::{Leg, LegKind, Road, RoadKind};

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
    /// Which leg broke, on a sponsor's `none` to its satellite (round
    /// ROAD-1): part of the kind, so a jump that comes back and a target
    /// that goes down is a change.
    pub broken: Option<Leg>,
}

#[derive(Debug, Clone)]
struct Written {
    road: RoadKind,
    relay: Option<String>,
    ms: Option<u16>,
    broken: Option<Leg>,
    /// When `ms` was last written. A value found in the table at start
    /// counts as written then, so a restart does not rewrite every round
    /// trip at once.
    ms_at: Instant,
}

/// The book: what is in the table, and what is waiting to settle.
#[derive(Debug, Default)]
pub(crate) struct RoadBook {
    written: HashMap<String, Written>,
    waiting: HashMap<String, (RoadKind, Option<String>, Option<Leg>, Instant)>,
    started: bool,
    /// What ssh's words said broke on each satellite's last failed
    /// sample (`satellite::broken`), by satellite id; cleared by a sample
    /// that came back.
    sat_broken: HashMap<String, Option<crate::satellite::Broken>>,
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
                Written { road: r.road, relay: r.relay.clone(), ms: r.ms, broken: r.broken.clone(), ms_at: now },
            );
        }
    }

    /// One beat's reading of the road to `to`; the road to write, when
    /// one is due.
    pub(crate) fn note(&mut self, to: &str, seen: Seen, now: Instant) -> Option<Road> {
        let road = |s: &Seen| Road { broken: s.broken.clone(), ..Road::own(to, s.road, s.relay.clone(), s.ms) };
        let Some(w) = self.written.get(to) else {
            // Never written: the first reading that holds is the first
            // write, by the same settling rule — but `none` only once it
            // has held [`FIRST_NONE`].
            let hold = if seen.road == RoadKind::None { FIRST_NONE } else { SETTLE };
            return self.settle(to, &seen, now, hold).then(|| self.wrote(to, &seen, now, road(&seen)));
        };
        if w.road != seen.road || w.relay != seen.relay || w.broken != seen.broken {
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

    /// A satellite's sample came back (`None`) or failed, with what
    /// ssh's words said broke (`Some(reading)`).
    pub(crate) fn sampled(&mut self, sat: &str, failed: Option<Option<crate::satellite::Broken>>) {
        match failed {
            None => self.sat_broken.remove(sat),
            Some(b) => self.sat_broken.insert(sat.to_owned(), b),
        };
    }

    /// No live wire to `to` and nothing failed: no road to write, and a
    /// change that was settling starts over from the next reading.
    pub(crate) fn idle(&mut self, to: &str) {
        self.waiting.remove(to);
    }

    /// Whether `seen`'s kind has now held for `hold`.
    fn settle(&mut self, to: &str, seen: &Seen, now: Instant, hold: Duration) -> bool {
        match self.waiting.get(to) {
            Some((road, relay, broken, since))
                if *road == seen.road && *relay == seen.relay && *broken == seen.broken =>
            {
                now.duration_since(*since) >= hold
            }
            _ => {
                self.waiting.insert(to.to_owned(), (seen.road, seen.relay.clone(), seen.broken.clone(), now));
                false
            }
        }
    }

    fn wrote(&mut self, to: &str, seen: &Seen, now: Instant, road: Road) -> Road {
        self.waiting.remove(to);
        self.written.insert(
            to.to_owned(),
            Written { road: seen.road, relay: seen.relay.clone(), ms: seen.ms, broken: seen.broken.clone(), ms_at: now },
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
            // **A sponsor's road to each of its satellites** (round ROAD-1):
            // its ssh, up while the last sample is fresh. Never sampled
            // yet is no reading, as an idle wire is.
            for d in all.iter().filter(|d| d.sponsor.as_deref() == Some(me.as_str())) {
                match self.satellite_road_in(&book, &d.id) {
                    Some(r) => {
                        let seen = Seen { road: r.road, relay: None, ms: None, broken: r.broken };
                        if let Some(w) = book.note(&d.id, seen, now) {
                            due.push(w);
                        }
                    }
                    None => book.idle(&d.id),
                }
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
                    broken: None,
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

    /// This sponsor's road to satellite `sat`: `ssh` while its last
    /// sample is younger than [`SAT_FRESH`], `none` once older — with
    /// the leg that broke, as ssh's words on the last failed sample read
    /// (`satellite::broken`); `None` before the first sample.
    pub fn satellite_road(&self, sat: &str) -> Option<Road> {
        let book = self.roadbook.lock().unwrap_or_else(|p| p.into_inner());
        self.satellite_road_in(&book, sat)
    }

    fn satellite_road_in(&self, book: &RoadBook, sat: &str) -> Option<Road> {
        let (_, age_ms) = self.vitals_of(sat)?;
        if Duration::from_millis(age_ms) < SAT_FRESH {
            return Some(Road::own(sat, RoadKind::Ssh, None, None));
        }
        let me = Some(self.device_str().to_owned());
        let them = Some(sat.to_owned());
        use crate::satellite::Broken;
        let leg = match book.sat_broken.get(sat).copied().flatten() {
            Some(Broken::ToJump) => Leg { from: me, to: None, leg: LegKind::ToJump },
            Some(Broken::JumpToTarget) => Leg { from: None, to: them, leg: LegKind::JumpToTarget },
            Some(Broken::PasswordOnly) => Leg { from: me, to: them, leg: LegKind::Auth },
            None => Leg { from: me, to: them, leg: LegKind::Ssh },
        };
        Some(Road { broken: Some(leg), ..Road::own(sat, RoadKind::None, None, None) })
    }

    /// **This machine's roads to its own satellites**, read live (the
    /// sampler's age), as [`Self::satellite_road`] says them.
    pub fn my_satellite_roads(&self) -> Vec<Road> {
        let me = self.device_str();
        self.devices()
            .unwrap_or_default()
            .iter()
            .filter(|d| d.sponsor.as_deref() == Some(me))
            .filter_map(|d| self.satellite_road(&d.id))
            .collect()
    }

    /// **`x`'s roads with its roads to satellites added** (round ROAD-1):
    /// `x_roads` are `x`'s own (live for this machine, the table's for
    /// another); a satellite sponsored by another machine is reached
    /// through that sponsor ([`through_sponsors`]).
    pub fn with_satellite_roads(&self, x: &str, x_roads: Vec<Road>) -> Vec<Road> {
        let all = self.devices().unwrap_or_default();
        let me = self.device_str();
        let sats: Vec<(String, String)> = all
            .iter()
            .filter_map(|d| Some((d.id.clone(), d.sponsor.clone()?)))
            .collect();
        through_sponsors(x, x_roads, &sats, |sponsor, sat| {
            if sponsor == me {
                return self.satellite_road(sat);
            }
            all.iter()
                .find(|d| d.id == sponsor)
                .and_then(|d| roads_in(d).into_iter().find(|r| r.to == sat))
        })
    }

    /// The roads `d` reports, as the table holds them (round NET-12): for
    /// another machine's row. This machine's own roads are its live
    /// wires (`Node::links`), never this.
    pub fn roads_of(&self, d: &khor_sync::devices::DeviceInfo) -> Vec<Road> {
        roads_in(d)
    }
}

/// How old a satellite's last sample may be and its ssh still count as
/// up: two and a half of the sampler's rounds, so one slow round is not
/// a broken road.
pub const SAT_FRESH: Duration = Duration::from_millis(crate::satellite::SAMPLE_EVERY.as_millis() as u64 * 5 / 2);

/// **Roads to satellites through their sponsors**, added to `x`'s own.
/// For each satellite `s` whose sponsor `p` is not `x`: the first leg is
/// `x`'s road to `p`, the second `p`'s ssh road to `s` (`second`). A
/// road is written only when both legs are known, or one is known
/// broken: a leg nobody has a reading for is no word, as an idle wire is.
pub fn through_sponsors(
    x: &str,
    mut x_roads: Vec<Road>,
    sats: &[(String, String)],
    second: impl Fn(&str, &str) -> Option<Road>,
) -> Vec<Road> {
    for (s, p) in sats {
        if p == x || x_roads.iter().any(|r| &r.to == s) {
            continue;
        }
        let first = x_roads.iter().find(|r| &r.to == p).cloned();
        let second = second(p, s);
        let broken = |leg: Leg| Road {
            via: Some(p.clone()),
            broken: Some(leg),
            ..Road::own(s, RoadKind::None, None, None)
        };
        let road = match (first, second) {
            (Some(f), _) if f.road == RoadKind::None => {
                broken(Leg { from: Some(x.to_owned()), to: Some(p.clone()), leg: LegKind::ToSponsor })
            }
            // The sponsor's own reading of its ssh says which leg; a
            // sponsor that wrote none says only that its ssh broke.
            (_, Some(r)) if r.road == RoadKind::None => broken(
                r.broken.unwrap_or(Leg { from: Some(p.clone()), to: Some(s.clone()), leg: LegKind::Ssh }),
            ),
            (Some(f), Some(r)) if r.road == RoadKind::Ssh => Road { to: s.clone(), via: Some(p.clone()), ..f },
            _ => continue,
        };
        x_roads.push(road);
    }
    x_roads
}

fn roads_in(d: &khor_sync::devices::DeviceInfo) -> Vec<Road> {
    d.roads.iter().filter_map(|(to, json)| from_json(to, json)).collect()
}

/// A road as its register holds it (`khor_sync::devices`' `F_ROAD`):
/// the object without `to`, which is the key.
pub(crate) fn to_json(r: &Road) -> String {
    serde_json::json!({ "road": r.road, "relay": r.relay, "ms": r.ms, "broken": r.broken }).to_string()
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
        Seen { road, relay: None, ms, broken: None }
    }

    /// A flap inside the settling window writes nothing; a kind that
    /// holds is written once.
    #[test]
    fn a_kind_is_written_once_it_has_held() {
        let t0 = Instant::now();
        let mut b = RoadBook::default();
        b.start_from(&[Road::own("a", RoadKind::Direct, None, Some(20))], t0);
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
        b.start_from(&[Road::own("a", RoadKind::Direct, None, Some(38))], t0);
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
        b.start_from(&[Road::own("a", RoadKind::Direct, None, Some(20))], t0);
        assert_eq!(b.note("a", seen(RoadKind::Relay, Some(90)), s(1)), None);
        // The wire closes idle for a minute: nothing is noted, so nothing
        // can be written — and the relay reading 60 s ago held nothing.
        b.idle("a");
        assert_eq!(b.note("a", seen(RoadKind::Relay, Some(90)), s(61)), None, "settles from the new reading");
        assert!(b.note("a", seen(RoadKind::Relay, Some(90)), s(73)).is_some(), "and is written once it held");
    }

    #[test]
    fn a_register_reads_back_as_the_road_it_was() {
        let r = Road::own("ab", RoadKind::Relay, Some("relay.khor.cloud".into()), Some(180));
        assert_eq!(from_json("ab", &to_json(&r)), Some(r));
        let ssh = Road::own("ab", RoadKind::Ssh, None, None);
        assert_eq!(from_json("ab", &to_json(&ssh)), Some(ssh), "a sponsor's road to its satellite");
        let auth = Road {
            broken: Some(Leg { from: Some("p".into()), to: Some("ab".into()), leg: LegKind::Auth }),
            ..Road::own("ab", RoadKind::None, None, None)
        };
        assert_eq!(from_json("ab", &to_json(&auth)), Some(auth), "the leg that broke rides the register");
        assert_eq!(from_json("ab", "{\"road\":\"sideways\"}"), None, "a kind this build does not know");
    }

    /// A satellite is reached through its sponsor: both legs up is the
    /// first leg's road with `via`; either leg down names that leg; a
    /// leg nobody has read is no road at all.
    #[test]
    fn a_satellite_is_reached_through_its_sponsor_and_a_break_names_its_leg() {
        let sats = vec![("s".to_owned(), "p".to_owned())];
        let up = |_: &str, _: &str| Some(Road::own("s", RoadKind::Ssh, None, None));
        let down = |_: &str, _: &str| Some(Road::own("s", RoadKind::None, None, None));
        let unread = |_: &str, _: &str| None;
        let to_p = |k| vec![Road::own("p", k, None, Some(9))];

        let r = through_sponsors("x", to_p(RoadKind::Relay), &sats, up);
        let s = r.iter().find(|r| r.to == "s").expect("a road to the satellite");
        assert_eq!((s.road, s.via.as_deref(), s.ms, &s.broken), (RoadKind::Relay, Some("p"), Some(9), &None));

        let r = through_sponsors("x", to_p(RoadKind::None), &sats, up);
        let s = r.iter().find(|r| r.to == "s").unwrap();
        assert_eq!(s.road, RoadKind::None);
        assert_eq!(s.broken, Some(Leg { from: Some("x".into()), to: Some("p".into()), leg: LegKind::ToSponsor }));

        let r = through_sponsors("x", to_p(RoadKind::Direct), &sats, down);
        let s = r.iter().find(|r| r.to == "s").unwrap();
        assert_eq!(s.broken, Some(Leg { from: Some("p".into()), to: Some("s".into()), leg: LegKind::Ssh }));
        // The sponsor's own reading names a jump's leg, whose far end is
        // no device.
        let jump = Leg { from: Some("p".into()), to: None, leg: LegKind::ToJump };
        let at_jump = |_: &str, _: &str| Some(Road { broken: Some(jump.clone()), ..Road::own("s", RoadKind::None, None, None) });
        let r = through_sponsors("x", to_p(RoadKind::Direct), &sats, at_jump);
        assert_eq!(r.iter().find(|r| r.to == "s").unwrap().broken, Some(jump));

        assert!(through_sponsors("x", to_p(RoadKind::Direct), &sats, unread).iter().all(|r| r.to != "s"));
        assert!(through_sponsors("x", vec![], &sats, up).iter().all(|r| r.to != "s"), "no road to the sponsor read");
        assert!(through_sponsors("p", vec![], &sats, up).is_empty(), "the sponsor's own road is its ssh, not this");
    }
}
