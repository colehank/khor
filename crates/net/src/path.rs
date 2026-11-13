//! Which road a connection is on, right now.
//!
//! iroh keeps several paths open at once — after hole-punching the relay
//! stays as fallback — and selects one by RTT. This module only reports
//! that choice; path selection belongs to the QUIC layer, which sees
//! congestion and loss where we see a ping.

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PathKind {
    Direct,
    Relay,
    /// A road khor carries itself (round ROAD-2, [`crate::carry`]): an ssh
    /// session or a hop through a third machine. Which of the two is the
    /// node's to say; the path only knows it is not an IP and not a relay.
    Carried,
    /// The default. "Not reported yet" is the only value that claims
    /// nothing; defaulting to Relay or Direct would be making it up.
    #[default]
    Unknown,
}

/// The selector's quality and the evidence behind it, copied from iroh.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathQuality {
    pub rtt: Duration,
    pub age: Duration,
    pub samples: usize,
}

/// Transport statistics and the independent quality used by selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathFact {
    pub relay: bool,
    /// A carried road ([`PathKind::Carried`]).
    pub carried: bool,
    /// Whether application data rides this path right now. With several
    /// open, exactly one is.
    pub selected: bool,
    /// QUIC's smoothed estimate, retained for transport diagnostics.
    #[serde(deserialize_with = "deserialize_rtt_millis")]
    pub rtt_ms: u128,
    pub quality: Option<PathQuality>,
    /// Diagnostic data only — never drawn on screen.
    pub addr: String,
}

// Journal events are internally tagged, so serde buffers their fields.
// That buffer supports the u64 JSON numbers used for measured milliseconds,
// but cannot deserialize u128 directly. Preserve PathFact's arithmetic type.
fn deserialize_rtt_millis<'de, D>(deserializer: D) -> Result<u128, D::Error>
where
    D: serde::Deserializer<'de>,
{
    u64::deserialize(deserializer).map(u128::from)
}

/// The rule: the **selected** path decides; none selected = Unknown, not
/// a guess. A merely-existing direct candidate is not "direct" — while
/// unselected, the bytes are still on the relay, and reporting Direct
/// then puts a confident falsehood on screen. Split from [`classify`] so
/// this half is testable without a live QUIC connection.
pub fn kind_of(facts: &[PathFact]) -> PathKind {
    match facts.iter().find(|f| f.selected) {
        Some(f) if f.relay => PathKind::Relay,
        Some(f) if f.carried => PathKind::Carried,
        Some(_) => PathKind::Direct,
        None => PathKind::Unknown,
    }
}

/// The current road of a live connection. Instantaneous fact, never
/// cached: relay one second, direct the next is normal mid-upgrade.
pub fn classify(conn: &iroh::endpoint::Connection) -> PathKind {
    kind_of(&path_facts(conn))
}

/// **The relay the connection is going through, by name** (round NET-8,
/// for UX-16's map): the selected path's relay, as its host — and port,
/// when it is not the scheme's own (`relay.khor.cloud`,
/// `101.200.80.77:3340`). `None` when the selected path is direct, or
/// none is selected yet.
pub fn relay_name(conn: &iroh::endpoint::Connection) -> Option<String> {
    let paths = conn.paths();
    let p = paths.iter().find(|p| p.is_selected() && p.is_relay())?;
    match p.remote_addr() {
        iroh::TransportAddr::Relay(url) => {
            let host = url.host_str()?;
            Some(match url.port() {
                Some(port) => format!("{host}:{port}"),
                None => host.to_owned(),
            })
        }
        _ => None,
    }
}

/// Every path right now, the selected one first.
pub fn path_facts(conn: &iroh::endpoint::Connection) -> Vec<PathFact> {
    let mut facts: Vec<PathFact> = conn
        .paths()
        .iter()
        .map(|p| PathFact {
            relay: p.is_relay(),
            carried: matches!(p.remote_addr(), iroh::TransportAddr::Custom(_)),
            selected: p.is_selected(),
            rtt_ms: p.rtt().as_millis(),
            quality: p.measurement().map(|m| PathQuality {
                rtt: m.rtt(),
                age: m.age(),
                samples: m.samples(),
            }),
            addr: format!("{:?}", p.remote_addr()),
        })
        .collect();
    facts.sort_by_key(|f| !f.selected);
    facts
}

/// Whether the selected road keeps a carried road out on measurement.
/// Uses the same steady-state gain as selection, including for a direct
/// road. A temporary fallback must not tear down a faster hop while its
/// recovery/confirmation is still running. Unknown carried RTTs do not
/// claim that the carried road is faster.
pub fn selected_beats_carried(facts: &[PathFact]) -> bool {
    let Some(selected) = facts.iter().find(|f| f.selected && !f.carried) else {
        return false;
    };
    let rtt = |f: &PathFact| {
        f.quality.as_ref().map_or_else(
            || Duration::from_millis(f.rtt_ms.min(u128::from(u64::MAX)) as u64),
            |q| q.rtt,
        )
    };
    !facts.iter().any(|f| {
        f.carried
            && (f.quality.is_some() || f.rtt_ms > 0)
            && crate::carry::improves(rtt(f), rtt(selected), false)
    })
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

    fn carried(selected: bool, rtt_ms: u128) -> PathFact {
        PathFact {
            relay: false,
            carried: true,
            selected,
            rtt_ms,
            quality: (rtt_ms > 0).then(|| quality(rtt_ms)),
            addr: "Custom(khor)".into(),
        }
    }

    fn quality(rtt_ms: u128) -> PathQuality {
        PathQuality {
            rtt: Duration::from_millis(rtt_ms as u64),
            age: Duration::ZERO,
            samples: 2,
        }
    }

    #[test]
    fn cleanup_does_not_remove_a_hop_that_current_quality_would_select() {
        let mut selected = path(true, true, 20);
        selected.quality = Some(quality(300));
        let mut hop = carried(false, 100);
        hop.quality = Some(quality(5));
        assert!(!selected_beats_carried(&[selected, hop]));
    }

    #[test]
    fn cleanup_keeps_the_measured_twenty_percent_faster_hop_for_confirmation() {
        assert!(!selected_beats_carried(&[
            path(true, true, 131),
            carried(false, 104)
        ]));
    }

    #[test]
    fn cleanup_does_not_keep_a_hop_because_of_old_transport_history() {
        let mut selected = path(false, true, 500);
        selected.quality = Some(quality(5));
        let mut hop = carried(false, 5);
        hop.quality = Some(quality(300));
        assert!(selected_beats_carried(&[selected, hop]));
    }

    #[test]
    fn a_measured_submillisecond_hop_is_not_an_unknown_zero() {
        let mut hop = carried(false, 0);
        hop.quality = Some(PathQuality {
            rtt: Duration::from_micros(500),
            age: Duration::ZERO,
            samples: 2,
        });
        assert!(!selected_beats_carried(&[path(true, true, 10), hop]));
    }

    #[test]
    fn hop_cleanup_uses_the_same_gain_as_selection() {
        assert!(
            !selected_beats_carried(&[path(true, true, 30), carried(false, 22)]),
            "a hop 8 ms faster on a 30 ms relay can win selection; keep it"
        );
        assert!(
            !selected_beats_carried(&[path(true, true, 500), carried(false, 450)]),
            "a measured 50 ms gain can enter fresh confirmation; keep it"
        );
    }

    /// The relay keeps a connection off its hop only when the hop, measured
    /// end to end, is no faster: a stalled carried road reads what it read
    /// before the stall (netlab f4: 100 ms against a relay 1.7 s late), and a
    /// relay taken over it is not the hop losing.
    #[test]
    fn the_relay_beats_the_hop_only_on_measurement() {
        assert!(
            !selected_beats_carried(&[path(true, true, 856), carried(false, 100)]),
            "a stalled hop"
        );
        assert!(
            selected_beats_carried(&[path(true, true, 200), carried(false, 300)]),
            "a slower hop"
        );
        assert!(
            !selected_beats_carried(&[path(true, true, 200), carried(false, 195)]),
            "5 ms reaches the steady eligibility floor"
        );
        assert!(
            selected_beats_carried(&[path(true, true, 200), carried(false, 196)]),
            "4 ms remains below the steady eligibility floor"
        );
        assert!(
            selected_beats_carried(&[path(true, true, 200)]),
            "no carried path"
        );
        assert!(
            selected_beats_carried(&[path(true, true, 200), carried(false, 0)]),
            "a carried path never measured"
        );
        assert!(
            !selected_beats_carried(&[path(true, false, 200), carried(true, 300)]),
            "on the hop now"
        );
        assert!(
            !selected_beats_carried(&[path(false, true, 90), carried(false, 5)]),
            "a direct fallback must keep a faster recovering hop"
        );
        assert!(selected_beats_carried(&[
            path(false, true, 5),
            carried(false, 90),
        ]));
    }

    fn path(relay: bool, selected: bool, rtt_ms: u128) -> PathFact {
        PathFact {
            relay,
            carried: false,
            selected,
            rtt_ms,
            quality: Some(quality(rtt_ms)),
            addr: if relay {
                "http://relay:3340".into()
            } else {
                "10.0.0.2:11204".into()
            },
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
        // A carried road is neither (round ROAD-2).
        let carried = PathFact {
            carried: true,
            ..path(false, true, 9)
        };
        assert_eq!(
            kind_of(&[carried, path(true, false, 61)]),
            PathKind::Carried
        );
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
        assert_eq!(
            serde_json::to_string(&PathKind::Direct).unwrap(),
            r#""direct""#
        );
        assert_eq!(
            serde_json::to_string(&PathKind::Relay).unwrap(),
            r#""relay""#
        );
        assert_eq!(
            serde_json::to_string(&PathKind::Unknown).unwrap(),
            r#""unknown""#
        );
    }
}
