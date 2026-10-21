//! **Whether this machine has a relay to go out by** (2026-10-21, C7).
//!
//! lecun, 2026-10-20: 24 hours without one line of 「家中继」 and without
//! one public address told by a relay, and every one of its 213 failed
//! dials still named Beijing, hk and pk among the roads tried. It talked
//! to the others all day, directly and through turing or hinton, but it
//! had no idea that it had no relay: every round tried the relays first
//! and waited out the dial before a hop was asked for, and a relay that
//! was down for a minute and a network with no way to any relay looked
//! the same to its backoff.
//!
//! # The rule
//!
//! - **No relay** is entered when, for [`ENTER_AFTER`] straight, the ears
//!   have had no home relay *and* the net report has answered from no
//!   relay at all — no HTTPS, no QAD. A home that is only connecting
//!   counts as none. hinton, with v6 alone and n0's relays behind ours, has
//!   a home on aps1 or euc1: it has a relay, only a far one, and is never
//!   in this state.
//! - It is left the moment either comes back: any home, or any relay in
//!   a report. One answer is enough; the way in is slow on purpose, the
//!   way out is not.
//! - **New ears start over** (`Node::set_ears`): a new network may have
//!   what the old one did not.
//!
//! What the state does is elsewhere: the machine writes it into its own
//! row (`khor_sync::devices` `F_NO_RELAY`) for the others to read, and a
//! dial from it asks for a hop first and gives the relays a short wait
//! (`link::Node::dial`, `hop::Node::wants_hop`).
//!
//! Why not lower the net report's own pace: that is iroh's, and a report
//! that finds nothing costs this machine three probes in the background;
//! what cost lecun was the dials that waited on relays in the foreground.

use std::time::{Duration, Instant};

/// How long with no home and no relay answering before this machine
/// says it has no relay: two of iroh's full reports (5 min each) and a
/// little — one report that missed everything is a blip.
pub(crate) const ENTER_AFTER: Duration = Duration::from_secs(11 * 60);

/// What a beat saw of the relays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Seen {
    /// The ears have a home relay, connected.
    pub home: bool,
    /// The last net report heard from some relay (HTTPS or QAD).
    pub answered: bool,
}

/// The state, fed once a beat.
#[derive(Debug, Default)]
pub(crate) struct Egress {
    /// Since when nothing was seen, `None` while something is.
    dark_since: Option<Instant>,
    /// What the machine said: `None` until it knows either way — a row
    /// left saying 「no relay」 by the last run is said again, or taken
    /// back, once this run knows.
    none: Option<bool>,
}

impl Egress {
    /// One beat's look; `Some(state)` when the state is known and differs
    /// from what was said.
    pub(crate) fn saw(&mut self, seen: Seen, now: Instant) -> Option<bool> {
        let none = if seen.home || seen.answered {
            self.dark_since = None;
            false
        } else {
            let since = *self.dark_since.get_or_insert(now);
            if now.duration_since(since) < ENTER_AFTER {
                return None;
            }
            true
        };
        (self.none != Some(none)).then(|| {
            self.none = Some(none);
            none
        })
    }

    /// Whether this machine has said it has no relay.
    pub(crate) fn none(&self) -> bool {
        self.none == Some(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DARK: Seen = Seen { home: false, answered: false };

    /// lecun's day: nothing, beat after beat, until [`ENTER_AFTER`]; out
    /// at the first answer.
    #[test]
    fn no_relay_is_entered_slowly_and_left_at_once() {
        let t0 = Instant::now();
        let mut e = Egress::default();
        let beat = Duration::from_secs(5);
        let mut said = None;
        let mut t = t0;
        while t.duration_since(t0) <= ENTER_AFTER + beat {
            if let Some(s) = e.saw(DARK, t) {
                said = Some((s, t.duration_since(t0)));
            }
            t += beat;
        }
        let (s, at) = said.expect("said once");
        assert!(s && at >= ENTER_AFTER && at < ENTER_AFTER + beat, "{at:?}");
        assert!(e.none());
        assert_eq!(e.saw(Seen { home: false, answered: true }, t), Some(false), "one answer is enough");
        assert_eq!(e.saw(DARK, t + beat), None, "and the clock starts again");
    }

    /// A run that hears a relay says so once — the row may still say
    /// 「no relay」 from the last run — and then nothing more.
    #[test]
    fn the_first_answer_is_said_once() {
        let t0 = Instant::now();
        let mut e = Egress::default();
        assert_eq!(e.saw(DARK, t0), None, "not yet known");
        let lit = Seen { home: true, answered: true };
        assert_eq!(e.saw(lit, t0 + Duration::from_secs(5)), Some(false));
        assert_eq!(e.saw(lit, t0 + Duration::from_secs(10)), None);
    }

    /// A relay down for a while is not a network with none: an answer
    /// inside the window starts it over.
    #[test]
    fn a_blip_is_not_no_relay() {
        let t0 = Instant::now();
        let mut e = Egress::default();
        assert_eq!(e.saw(DARK, t0), None);
        assert_eq!(e.saw(DARK, t0 + ENTER_AFTER - Duration::from_secs(1)), None);
        assert_eq!(e.saw(Seen { home: true, answered: false }, t0 + ENTER_AFTER), Some(false));
        assert_eq!(e.saw(DARK, t0 + ENTER_AFTER + Duration::from_secs(5)), None);
        assert!(!e.none());
    }

    /// hinton: v6 alone, no relay of ours answers, n0's aps1 is its home.
    /// It has a relay out — only a far one.
    #[test]
    fn a_home_on_the_fallback_tier_is_a_relay() {
        let t0 = Instant::now();
        let mut e = Egress::default();
        for i in 0..400 {
            let seen = Seen { home: true, answered: i % 2 == 0 };
            assert_eq!(e.saw(seen, t0 + Duration::from_secs(5 * i)), (i == 0).then_some(false));
        }
        assert!(!e.none());
    }
}

impl crate::Node {
    /// One beat's look at the relays (`link`'s beat): the home on these
    /// ears, and whether the last net report heard from any relay. When
    /// the state changes, it is written to the journal and to this
    /// machine's own row.
    pub(crate) fn look_for_relays(&self, ep: &iroh::Endpoint) {
        use iroh::Watcher;
        let home = ep.home_relay_status().get().iter().any(|r| r.is_connected());
        let answered = ep.net_report().get().is_some_and(|r| r.relay_latency.iter().next().is_some());
        let changed = self.egress.lock().unwrap_or_else(|p| p.into_inner()).saw(Seen { home, answered }, Instant::now());
        let Some(none) = changed else { return };
        self.wires.note(None, crate::journal::What::NoRelay { none });
        // The roads to try changed: a machine waiting out a dial that the
        // relays failed is dialled again at once, the other way.
        self.wires.forgive_all();
        if let Ok(loaded) = self.devices_loaded() {
            let mut store = loaded.store;
            if loaded.doc.set_no_relay(self.device_str(), none).is_ok() {
                let _ = store.flush(&loaded.doc);
            }
        }
    }

    /// Whether this machine has said it has no relay to go out by.
    pub(crate) fn no_relay(&self) -> bool {
        self.egress.lock().unwrap_or_else(|p| p.into_inner()).none()
    }
}
