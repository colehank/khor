//! Which road a connection is on, right now.
//!
//! iroh keeps several paths open at once — after hole-punching the relay
//! stays as fallback — and selects one by RTT. This module only reports
//! that choice; path selection belongs to the QUIC layer, which sees
//! congestion and loss where we see a ping.

use std::time::{Duration, Instant};

use serde::Serialize;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PathKind {
    Direct,
    Relay,
    /// The default. "Not reported yet" is the only value that claims
    /// nothing; defaulting to Relay or Direct would be making it up.
    #[default]
    Unknown,
}

/// One path's current facts, copied verbatim from iroh's observations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PathFact {
    pub relay: bool,
    /// Whether application data rides this path right now. With several
    /// open, exactly one is.
    pub selected: bool,
    pub rtt_ms: u128,
    /// Diagnostic data only — never drawn on screen.
    pub addr: String,
}

/// The rule: the **selected** path decides; none selected = Unknown, not
/// a guess. A merely-existing direct candidate is not "direct" — while
/// unselected, the bytes are still on the relay, and reporting Direct
/// then puts a confident falsehood on screen. Split from [`classify`] so
/// this half is testable without a live QUIC connection.
pub fn kind_of(facts: &[PathFact]) -> PathKind {
    match facts.iter().find(|f| f.selected) {
        Some(f) if f.relay => PathKind::Relay,
        Some(_) => PathKind::Direct,
        None => PathKind::Unknown,
    }
}

/// The current road of a live connection. Instantaneous fact, never
/// cached: relay one second, direct the next is normal mid-upgrade.
pub fn classify(conn: &iroh::endpoint::Connection) -> PathKind {
    kind_of(&path_facts(conn))
}

/// Every path right now, the selected one first.
pub fn path_facts(conn: &iroh::endpoint::Connection) -> Vec<PathFact> {
    let mut facts: Vec<PathFact> = conn
        .paths()
        .iter()
        .map(|p| PathFact {
            relay: p.is_relay(),
            selected: p.is_selected(),
            rtt_ms: p.rtt().as_millis(),
            addr: format!("{:?}", p.remote_addr()),
        })
        .collect();
    facts.sort_by_key(|f| !f.selected);
    facts
}

/// Watches a connection settle into direct or relay. Hole-punching is
/// async — connections usually start on the relay and upgrade — so
/// judging at connect time is wrong; give it a window.
pub async fn observe_path(
    conn: &iroh::endpoint::Connection,
    window: Duration,
) -> (PathKind, Option<u128>) {
    let start = Instant::now();
    let deadline = start + window;
    let mut last = PathKind::Unknown;

    while Instant::now() < deadline {
        let kind = classify(conn);
        if kind == PathKind::Direct {
            return (PathKind::Direct, Some(start.elapsed().as_millis()));
        }
        if kind != PathKind::Unknown {
            last = kind;
        }
        // The sampling interval is the measurement precision: at 250ms,
        // sub-tick upgrades all read as "250ms". 25ms resolves sub-second
        // upgrades and only reads paths().
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    (last, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(relay: bool, selected: bool, rtt_ms: u128) -> PathFact {
        PathFact {
            relay,
            selected,
            rtt_ms,
            addr: if relay { "http://relay:3340".into() } else { "10.0.0.2:11204".into() },
        }
    }

    /// After hole-punching two paths are open at once, and only one
    /// carries bytes. Pins a judgment that once lied: "any non-relay
    /// path exists" reported Direct while the bytes were still on the
    /// relay — and the diagnosis screen then declared, confidently and
    /// falsely, that the relay was unused.
    #[test]
    fn the_selected_path_decides_not_the_mere_existence_of_a_direct_one() {
        // Mid-upgrade: a direct candidate exists, bytes still on relay.
        assert_eq!(
            kind_of(&[path(true, true, 61), path(false, false, 30)]),
            PathKind::Relay,
            "a direct candidate is not the same as taking it"
        );
        // Upgrade done: direct selected, relay kept as fallback.
        assert_eq!(
            kind_of(&[path(false, true, 30), path(true, false, 61)]),
            PathKind::Direct
        );
        // Relay only.
        assert_eq!(kind_of(&[path(true, true, 61)]), PathKind::Relay);
        // No paths, or none selected: both are "no fact available".
        assert_eq!(kind_of(&[]), PathKind::Unknown);
        assert_eq!(
            kind_of(&[path(true, false, 61), path(false, false, 30)]),
            PathKind::Unknown
        );
    }

    /// The three strings are pinned snake_case: stored measurement data
    /// and cross-layer mappings key on them, so changing one silently
    /// corrupts every recorded run.
    #[test]
    fn path_kind_serializes_as_snake_case() {
        assert_eq!(serde_json::to_string(&PathKind::Direct).unwrap(), r#""direct""#);
        assert_eq!(serde_json::to_string(&PathKind::Relay).unwrap(), r#""relay""#);
        assert_eq!(serde_json::to_string(&PathKind::Unknown).unwrap(), r#""unknown""#);
    }
}
