//! Scenario f: the hop (NET-DIRECT step 4) on real NATs. X and Y sit
//! behind endpoint-dependent NATs and cannot punch to each other; M has
//! a public address and reaches both directly. The relay is far (200 ms
//! round trip), so X ↔ Y through M at its two legs beats it, and the
//! 30 s compare (`hop::compare_roads`) should move the wire there. With
//! M frozen, the wire goes back to the relay without going down
//! (`crates/cli/tests/hop.rs` measured 13 s on loopback).
//!
//! Scenario f3 (C7): a machine with no relay at all goes out through a
//! middle that has one (lecun, 2026-10-20). See its doc.
//!
//! Scenario f2: a hop with a relay leg (hinton, 2026-10-20). Y's home is
//! a far relay (its v4 reaches nothing else: n0's aps1, 300–1000 ms); X
//! reaches B only through the near relay; B and Y share a LAN. Through B
//! is about a tenth of Y's relay, so either end's compare should move the
//! wire there, and it should stay there.

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

/// Drops every packet between `dev` and `ip`, both ways, with nft.
fn cut(dev: &patchbay::Device, ip: std::net::Ipv4Addr) {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let rules = format!(
        "table inet netlab_cut {{\n chain cut_in {{ type filter hook input priority -10; policy accept;\n  ip saddr {ip} drop\n }}\n chain cut_out {{ type filter hook output priority -10; policy accept;\n  ip daddr {ip} drop\n }}\n}}\n"
    );
    let mut cmd = Command::new("nft");
    cmd.args(["-f", "-"]).stdin(Stdio::piped());
    let mut child = dev.spawn_command_sync(cmd).unwrap();
    child.stdin.take().unwrap().write_all(rules.as_bytes()).unwrap();
    assert!(child.wait().unwrap().success(), "nft took the cut:\n{rules}");
}

/// How long f2 watches the settled wire for moves.
const SETTLED: Duration = Duration::from_secs(120);

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
async fn f2_a_relay_leg_through_a_middle_beats_a_far_home() {
    let s = "f2";
    let lab = Lab::new().await.unwrap();
    let near = relay(&lab, s, "near", None).await;
    // 200 ms each way on the relay's own link: X ↔ Y through it is about
    // 800 ms, aps1 from Beijing on a bad evening.
    let far = relay(&lab, s, "far", Some(Duration::from_millis(400))).await;
    let (rx, rl) = (router(&lab, "natx", Net::Strict).await, router(&lab, "lan", Net::Strict).await);
    let dx = lab.add_device("x").uplink(rx.id()).build().await.unwrap();
    let db = lab.add_device("b").uplink(rl.id()).build().await.unwrap();
    let dy = lab.add_device("y").uplink(rl.id()).build().await.unwrap();
    for d in [&dx, &db, &dy] {
        slow(d, LEG).await;
    }
    // hinton's v4 reaches nothing outside but n0: Y never reaches the
    // near relay, so X meets it on the far one only.
    cut(&dy, near.dev.ip().expect("the near relay has IPv4"));
    let x = node(s, dx, "x", &[&near], &[]).await;
    let b = node(s, db, "b", &[&near], &[]).await;
    let y = node(s, dy, "y", &[&far], &[]).await;
    tokio::time::sleep(Duration::from_secs(8)).await;
    mesh(&[&b, &x, &y]).await;

    // The legs: X to B on the near relay, B to Y on the LAN.
    let legs = [
        x.until(&b, Duration::from_secs(120), |l| l.hop == Hop::Relay && l.rtt_ms.is_some()).await,
        b.until(&y, Duration::from_secs(120), |l| l.hop == Hop::Direct && l.rtt_ms.is_some()).await,
    ];
    let leg_words: Vec<String> = legs
        .iter()
        .map(|l| l.as_ref().map_or("never".into(), |(t, l)| format!("{} after {t:.1?}", word(l))))
        .collect();
    let relay_first = x.until(&y, Duration::from_secs(120), |l| l.hop == Hop::Relay && l.rtt_ms.is_some()).await;
    let hopped = x.until(&y, Duration::from_secs(150), |l| l.hop == Hop::Forwarded).await;
    let Some((took, through)) = hopped else {
        let now = x.link(&y).await;
        let back = y.link(&x).await;
        result(
            s,
            false,
            format!(
                "legs x–b {}, b–y {}; x→y on the relay {}; never through b in 150 s (x→y now {}, y→x now {})",
                leg_words[0],
                leg_words[1],
                relay_first.as_ref().map_or("never".into(), |(_, l)| word(l)),
                now.as_ref().map_or("nothing".into(), word),
                back.as_ref().map_or("nothing".into(), word),
            ),
        );
        panic!("f2: x reaches y through b");
    };

    // Settled: read both ends every second, count the moves.
    let began = std::time::Instant::now();
    let (mut moves, mut last, mut worst) = (0u32, (Hop::Forwarded, through.via.clone()), 0u32);
    let mut rtts = vec![];
    while began.elapsed() < SETTLED {
        tokio::time::sleep(Duration::from_secs(1)).await;
        let Some(l) = x.link(&y).await else { continue };
        if (l.hop, l.via.clone()) != last {
            moves += 1;
            last = (l.hop, l.via.clone());
        }
        if let Some(ms) = l.rtt_ms {
            worst = worst.max(ms);
            rtts.push(ms);
        }
    }
    rtts.sort_unstable();
    let median = rtts.get(rtts.len() / 2).copied();
    let there = x.link(&y).await.expect("a link");
    let back = y.link(&x).await.expect("a link back");
    // Who asked: both ends compare the same relay, and of two hops
    // between them the lower id's stands (`hop::Asking`).
    let asks = |n: &crate::lab::LabNode, peer: &crate::lab::LabNode| {
        let all: Vec<_> = n.journal(Some(peer)).into_iter().filter(|e| e["e"] == "hop").collect();
        (all.iter().filter(|e| e["via"].is_string()).count(), all.len())
    };
    let (xa, ya) = (asks(&x, &y), asks(&y, &x));
    let lower = if x.id < y.id { "x" } else { "y" };
    let ok = through.via.as_deref() == Some(b.id.as_str())
        && there.hop == Hop::Forwarded
        && back.hop == Hop::Forwarded
        && moves == 0;
    result(
        s,
        ok,
        format!(
            "legs x–b {}, b–y {}; relay {}; through b after {took:.1?}; over {SETTLED:?} {moves} moves, median {} ms, worst {worst} ms; now x→y {}, y→x {}; hops up/asked x {}/{}, y {}/{} (lower id {lower})",
            leg_words[0],
            leg_words[1],
            relay_first.as_ref().map_or("never".into(), |(_, l)| word(l)),
            median.map_or("?".into(), |m| m.to_string()),
            word(&there),
            word(&back),
            xa.0,
            xa.1,
            ya.0,
            ya.1,
        ),
    );
    assert_eq!(through.via.as_deref(), Some(b.id.as_str()), "the middle is b");
    assert_eq!(there.hop, Hop::Forwarded, "x → y holds on the hop");
    assert_eq!(back.hop, Hop::Forwarded, "y → x rides the same hop");
    assert_eq!(moves, 0, "no moves once through b");
}

/// f3: a machine with no relay (C7, lecun on 2026-10-20). X reaches no
/// relay at all; it shares a LAN with B, and B reaches Y only by the
/// relay. X should say it has no relay (its journal, and its row where
/// Y reads it) within `egress::ENTER_AFTER` and a minute, and then reach
/// Y through B — the middle's relay is X's way out — with dials that no
/// longer wait out the relays.
#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
async fn f3_a_machine_with_no_relay_goes_out_through_a_middle() {
    let s = "f3";
    let lab = Lab::new().await.unwrap();
    let r = relay(&lab, s, "relay", None).await;
    let (rl, ry) = (router(&lab, "lan", Net::Strict).await, router(&lab, "naty", Net::Strict).await);
    let dx = lab.add_device("x").uplink(rl.id()).build().await.unwrap();
    let db = lab.add_device("b").uplink(rl.id()).build().await.unwrap();
    let dy = lab.add_device("y").uplink(ry.id()).build().await.unwrap();
    for d in [&dx, &db, &dy] {
        slow(d, LEG).await;
    }
    cut(&dx, r.dev.ip().expect("the relay has IPv4"));
    let x = node(s, dx, "x", &[&r], &[]).await;
    let b = node(s, db, "b", &[&r], &[]).await;
    let y = node(s, dy, "y", &[&r], &[]).await;
    tokio::time::sleep(Duration::from_secs(8)).await;
    mesh(&[&b, &x, &y]).await;

    let began = std::time::Instant::now();
    let (said, said_at) = loop {
        let at = x.journal(None).into_iter().find(|e| e["e"] == "no-relay" && e["none"] == true);
        if at.is_some() || began.elapsed() > Duration::from_secs(13 * 60) {
            break (at.as_ref().map(|_| began.elapsed()), at.and_then(|e| e["at"].as_u64()).unwrap_or(u64::MAX));
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    };
    let hopped = x.until(&y, Duration::from_secs(60), |l| l.hop == Hop::Forwarded).await;
    let x_id = x.id.clone();
    let seen_by_y = y
        .with(move |n| async move { n.devices().ok()?.into_iter().find(|d| d.id == x_id).map(|d| d.no_relay) })
        .await;
    // X's failed dials to Y once it knew: none waits out the relays.
    let late: Vec<u64> = x
        .journal(Some(&y))
        .into_iter()
        .filter(|e| e["e"] == "dial" && e["won"].is_null() && e["at"].as_u64().is_some_and(|t| t > said_at))
        .filter_map(|e| e["ms"].as_u64())
        .collect();
    let ok = said.is_some()
        && hopped.as_ref().is_some_and(|(_, l)| l.via.as_deref() == Some(b.id.as_str()))
        && seen_by_y == Some(true)
        && late.iter().all(|ms| *ms <= 5_000);
    result(
        s,
        ok,
        format!(
            "no relay said after {}; y reads it {seen_by_y:?}; x→y through b {}; failed dials since {late:?} ms",
            said.map_or("never in 13 min".into(), |t| format!("{t:.0?}")),
            hopped.as_ref().map_or("never in 60 s".into(), |(t, l)| format!("after {t:.1?} ({})", word(l))),
        ),
    );
    assert!(said.is_some(), "x says it has no relay");
    assert_eq!(seen_by_y, Some(true), "y reads it in x's row");
    let (_, l) = hopped.expect("x reaches y through b");
    assert_eq!(l.via.as_deref(), Some(b.id.as_str()), "the middle is b");
    assert!(late.iter().all(|ms| *ms <= 5_000), "no dial waits out the relays: {late:?}");
}
