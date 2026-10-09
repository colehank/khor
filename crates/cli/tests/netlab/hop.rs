//! Scenario f: the hop (NET-DIRECT step 4) on real NATs. X and Y sit
//! behind endpoint-dependent NATs and cannot punch to each other; M has
//! a public address and reaches both directly. The relay is far (200 ms
//! round trip), so X ↔ Y through M at its two legs beats it, and the
//! 30 s compare (`hop::compare_roads`) should move the wire there. With
//! M frozen, the wire goes back to the relay without going down
//! (`crates/cli/tests/hop.rs` measured 13 s on loopback).

use std::time::Duration;

use khor_core::Hop;
use patchbay::Lab;

use crate::lab::{mesh, node, relay, result, slow, word};
use crate::nat::{Net, router};

/// Each machine's own link: 10 ms each way, so the legs are measurable.
const LEG: Duration = Duration::from_millis(20);

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
async fn f_two_strict_nats_meet_through_a_public_middle_and_fall_back_when_it_freezes() {
    let s = "f";
    let lab = Lab::new().await.unwrap();
    let r = relay(&lab, s, "relay", Some(Duration::from_millis(200))).await;
    let (rx, ry, rm) = (
        router(&lab, "natx", Net::Strict).await,
        router(&lab, "naty", Net::Strict).await,
        router(&lab, "pub", Net::Public).await,
    );
    let dx = lab.add_device("x").uplink(rx.id()).build().await.unwrap();
    let dy = lab.add_device("y").uplink(ry.id()).build().await.unwrap();
    let dm = lab.add_device("m").uplink(rm.id()).build().await.unwrap();
    for d in [&dx, &dy, &dm] {
        slow(d, LEG).await;
    }
    let x = node(s, dx, "x", &[&r], &[]).await;
    let y = node(s, dy, "y", &[&r], &[]).await;
    let m = node(s, dm, "m", &[&r], &[]).await;
    // Over a slow relay the first probe takes a while.
    tokio::time::sleep(Duration::from_secs(8)).await;
    mesh(&[&m, &x, &y]).await;

    // The legs: M reaches each directly.
    let legs = [
        x.until(&m, Duration::from_secs(120), |l| l.hop == Hop::Direct).await,
        y.until(&m, Duration::from_secs(120), |l| l.hop == Hop::Direct).await,
    ];
    let leg_words: Vec<String> = legs
        .iter()
        .map(|l| l.as_ref().map_or("never direct".into(), |(t, l)| format!("{} after {t:.1?}", word(l))))
        .collect();
    let relay_first = x.until(&y, Duration::from_secs(90), |l| l.hop == Hop::Relay && l.rtt_ms.is_some()).await;
    let hopped = x.until(&y, Duration::from_secs(150), |l| l.hop == Hop::Forwarded).await;
    let Some((took, through)) = hopped else {
        let now = x.link(&y).await;
        result(
            s,
            false,
            format!(
                "legs x–m {}, y–m {}; x→y on the relay {}; never through m in 150 s (now {})",
                leg_words[0],
                leg_words[1],
                relay_first.as_ref().map_or("never".into(), |(_, l)| word(l)),
                now.as_ref().map_or("nothing".into(), word),
            ),
        );
        panic!("f: x reaches y through m");
    };
    tokio::time::sleep(Duration::from_secs(5)).await;
    let settled = x.link(&y).await.expect("a link");

    m.freeze();
    let back = x.until(&y, Duration::from_secs(60), |l| l.hop == Hop::Relay).await;
    let after = x.link(&y).await;
    m.thaw();
    let ok = through.via.as_deref() == Some(m.id.as_str())
        && settled.hop == Hop::Forwarded
        && back.as_ref().is_some_and(|(t, _)| *t < Duration::from_secs(16))
        && after.as_ref().is_some_and(|l| l.down_ms.is_none());
    result(
        s,
        ok,
        format!(
            "legs x–m {}, y–m {}; relay {}; through m after {took:.1?}, settled {}; m frozen → relay {}",
            leg_words[0],
            leg_words[1],
            relay_first.as_ref().map_or("never".into(), |(_, l)| word(l)),
            word(&settled),
            back.as_ref().map_or("never in 60 s".into(), |(t, l)| format!("after {t:.1?} ({})", word(l))),
        ),
    );
    assert_eq!(through.via.as_deref(), Some(m.id.as_str()), "the middle is m");
    assert_eq!(settled.hop, Hop::Forwarded, "the hop holds once open");
    assert!(back.is_some_and(|(t, _)| t < Duration::from_secs(16)), "a frozen middle hands back to the relay in under 16 s");
    assert!(after.is_some_and(|l| l.down_ms.is_none()), "the wire never went down");
}
