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
//! Scenario f4 (flap, 2026-10-23): the carried road in use stalls for
//! longer than the relay answers, the phone changing networks. See its doc.
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
    cut_as(dev, ip, "netlab_cut");
}

/// [`cut`] in a table of its own name, which [`uncut`] takes away.
fn cut_as(dev: &patchbay::Device, ip: std::net::Ipv4Addr, table: &str) {
    let rules = format!(
        "table inet {table} {{\n chain cut_in {{ type filter hook input priority -10; policy accept;\n  ip saddr {ip} drop\n }}\n chain cut_out {{ type filter hook output priority -10; policy accept;\n  ip daddr {ip} drop\n }}\n}}\n"
    );
    nft(dev, &rules);
}

/// Takes away a [`cut_as`].
fn uncut(dev: &patchbay::Device, table: &str) {
    nft(dev, &format!("delete table inet {table}\n"));
}

fn nft(dev: &patchbay::Device, rules: &str) {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let mut cmd = Command::new("nft");
    cmd.args(["-f", "-"]).stdin(Stdio::piped());
    let mut child = dev.spawn_command_sync(cmd).unwrap();
    child.stdin.take().unwrap().write_all(rules.as_bytes()).unwrap();
    assert!(child.wait().unwrap().success(), "nft took:\n{rules}");
}

/// Runs `tc` with `args` in `dev`'s namespace.
fn tc(dev: &patchbay::Device, args: &str) {
    let mut cmd = std::process::Command::new("tc");
    cmd.args(args.split_whitespace());
    assert!(dev.spawn_command_sync(cmd).unwrap().wait().unwrap().success(), "tc {args}");
}

/// Out of `dev`'s first interface: `extra` more one way on what goes to
/// `ip`, `base` on the rest (`slow`'s half), with a prio qdisc and a u32
/// filter; `None` puts back `base` alone.
async fn delay_to(dev: &patchbay::Device, ip: std::net::Ipv4Addr, base: u32, extra: Option<u32>) {
    let iface = dev.default_iface().expect("an interface");
    let ifname = iface.name().to_owned();
    match extra {
        Some(extra) => {
            tc(dev, &format!("qdisc del dev {ifname} root"));
            tc(dev, &format!("qdisc add dev {ifname} root handle 1: prio bands 3 priomap 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1"));
            tc(dev, &format!("qdisc add dev {ifname} parent 1:2 handle 20: netem delay {base}ms limit 10000"));
            tc(dev, &format!("qdisc add dev {ifname} parent 1:3 handle 30: netem delay {}ms limit 10000", base + extra));
            tc(dev, &format!("filter add dev {ifname} parent 1:0 protocol ip prio 1 u32 match ip dst {ip}/32 flowid 1:3"));
        }
        None => {
            iface
                .set_condition(patchbay::LinkCondition::new().latency_ms(base), patchbay::LinkDirection::Egress)
                .await
                .unwrap();
        }
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_millis() as u64
}

/// `n`'s path lines about `peer` since `since` (ms since the epoch): when,
/// the change (`selected`, `started`, `opened`), the road.
fn path_lines(n: &crate::lab::LabNode, peer: &crate::lab::LabNode, since: u64) -> Vec<(u64, String, String)> {
    n.journal(Some(peer))
        .into_iter()
        .filter(|e| e["e"] == "path" && e["at"].as_u64().is_some_and(|t| t >= since))
        .map(|e| {
            (
                e["at"].as_u64().unwrap(),
                e["change"].as_str().unwrap_or("?").to_owned(),
                e["road"].as_str().unwrap_or("?").to_owned(),
            )
        })
        .collect()
}

/// The relay's one way, as f4 measures it: 22 ms (hinton to turing's
/// relay; the phone's carried road through turing is about 20 ms).
const F4_RELAY_ONE_WAY: u32 = 22;

/// Three machines for f4: X (the phone) and Y (MacLab) behind strict NATs,
/// M (turing) public; the relay 22 ms each way. Returns once Y rides the
/// carried road through M to X and held it for 10 s.
async fn f4_lab(
    s: &str,
) -> (Lab, crate::lab::Relay, patchbay::Router, crate::lab::LabNode, crate::lab::LabNode, crate::lab::LabNode) {
    let lab = Lab::new().await.unwrap();
    let r = relay(&lab, s, "relay", Some(Duration::from_millis(2 * F4_RELAY_ONE_WAY as u64))).await;
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
    tokio::time::sleep(Duration::from_secs(5)).await;
    mesh(&[&m, &x, &y]).await;
    let hopped = y.until(&x, Duration::from_secs(180), |l| l.hop == Hop::Forwarded).await;
    if hopped.is_none() {
        result(s, false, format!("y never through m to x in 180 s (now {})", y.link(&x).await.as_ref().map_or("nothing".into(), word)));
        panic!("{s}: y reaches x through m");
    }
    // Held: 10 s on the hop, both ways.
    let began = std::time::Instant::now();
    while began.elapsed() < Duration::from_secs(10) {
        tokio::time::sleep(Duration::from_millis(500)).await;
        let held = y.link(&x).await.is_some_and(|l| l.hop == Hop::Forwarded)
            && x.link(&y).await.is_some_and(|l| l.hop == Hop::Forwarded);
        if !held {
            y.until(&x, Duration::from_secs(60), |l| l.hop == Hop::Forwarded).await;
            x.until(&y, Duration::from_secs(60), |l| l.hop == Hop::Forwarded).await;
        }
    }
    (lab, r, rx, x, y, m)
}

/// f4 (flap, 2026-10-23): **the carried road in use stalls, the relay is
/// slower still**. The phone (X) changes networks while MacLab (Y) rides
/// the road through turing (M) to it, about 20 ms: X's leg to M goes black
/// for 3 s; its leg to the relay goes black for 1.5 s and what the relay
/// sends it is 1.7 s late until 1.5 s after M is back (the relay read
/// 1743, 3468, 4994 ms in the phone's journal). Y leaves the stalled
/// carried road for the relay (the carried road in use is not ridden out),
/// and the relay, a spare keepalive'd every 15 s, looks as if it answers.
/// Once the road through M answers again (M hears X on their leg) Y
/// should be back on it within 1.5 s (the 1 s silent hold and a tick or
/// two), never resting on the relay (a stretch of 20 s): the move off it
/// took the 2 s confirm and a sample of the relay, which one round trip of
/// 3.4 s could not give, and gave up for 30 s.
#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
async fn f4_a_stalled_carried_road_is_taken_again_at_once_off_a_slow_relay() {
    let s = "f4";
    let (_lab, r, rx, x, y, m) = f4_lab(s).await;
    let relay_ip = r.dev.ip().expect("the relay has IPv4");
    let m_ip = m.dev.ip().expect("m has IPv4");
    let x_out = rx.uplink_ip().expect("x's NAT has an address outside");

    let t = now_ms();
    // Y types on X's terminal throughout (an echo every 200 ms): an idle
    // connection has nothing in flight for 3 s of black to show, and the
    // phone's own network change pings every path.
    let typing_until = std::time::Instant::now() + Duration::from_secs(12);
    let stall = async {
        cut_as(&x.dev, m_ip, "flap_m");
        cut_as(&x.dev, relay_ip, "flap_relay");
        delay_to(&r.dev, x_out, F4_RELAY_ONE_WAY, Some(1700)).await;
        tokio::time::sleep(Duration::from_millis(1500)).await;
        uncut(&x.dev, "flap_relay");
        tokio::time::sleep(Duration::from_millis(1500)).await;
        uncut(&x.dev, "flap_m");
        let uncut_at = now_ms();
        tokio::time::sleep(Duration::from_millis(1500)).await;
        delay_to(&r.dev, x_out, F4_RELAY_ONE_WAY, None).await;
        uncut_at
    };
    // When the carried road answers again: its leg, m's wire to x, heard
    // again after the cut (the hop itself is read only by y's iroh).
    let leg_back = async {
        tokio::time::sleep(Duration::from_millis(3000)).await;
        let began = std::time::Instant::now();
        loop {
            if m.link(&x).await.and_then(|l| l.heard_ms).is_some_and(|h| h < 250) {
                break Some(now_ms());
            }
            if began.elapsed() > Duration::from_secs(30) {
                break None;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    };
    let mut seen = vec![];
    let watch = async {
        // Long enough to see a 30 s rest on the relay.
        let began = std::time::Instant::now();
        while began.elapsed() < Duration::from_secs(40) {
            if let Some(l) = y.link(&x).await {
                let w = format!("{:?}", l.hop);
                if seen.last().map(|(_, s)| s) != Some(&w) {
                    seen.push(((now_ms() - t) as f64 / 1000.0, w));
                }
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    };
    let (uncut_at, slowest, (), leg) = tokio::join!(stall, typing(&y, &x, typing_until), watch, leg_back);
    let answers = leg.unwrap_or(uncut_at);

    let lines: Vec<(u64, String, String)> =
        path_lines(&y, &x, t).into_iter().filter(|(_, c, _)| c == "selected").collect();
    let on_relay = lines.iter().find(|(_, _, r)| r == "relay").map(|(at, _, _)| *at);
    let back = on_relay.and_then(|left| lines.iter().find(|(at, _, r)| *at >= left && r == "carried").map(|(at, _, _)| *at));
    // The longest stretch on the relay, to the end of the watch if it never left.
    let end = now_ms();
    let mut longest = 0u64;
    let mut since: Option<u64> = None;
    for (at, _, road) in &lines {
        match (road.as_str(), since) {
            ("relay", None) => since = Some(*at),
            ("relay", Some(_)) => {}
            (_, Some(s0)) => {
                longest = longest.max(at - s0);
                since = None;
            }
            _ => {}
        }
    }
    if let Some(s0) = since {
        longest = longest.max(end - s0);
    }
    let after_answer = back.map(|b| b.saturating_sub(answers));
    let now = y.link(&x).await;
    let x_lines: Vec<String> = path_lines(&x, &y, t)
        .into_iter()
        .filter(|(_, c, _)| c == "selected")
        .map(|(at, _, r)| format!("{r}@{:.1}s", at.saturating_sub(t) as f64 / 1000.0))
        .collect();
    let ok = now.as_ref().is_some_and(|l| l.hop == Hop::Forwarded)
        && longest < 20_000
        && (on_relay.is_none() || after_answer.is_some_and(|ms| ms <= 1500));
    let secs = |at: u64| format!("{:.1}s", at.saturating_sub(t) as f64 / 1000.0);
    result(
        s,
        ok,
        format!(
            "y→x: relay at {}, back on the carried road at {} ({} after the leg x–m is heard again, at {}), longest on the relay {:.1}s; selections {:?}; links {seen:?}; slowest keystroke {slowest:.1?}; now {}; x's selections {x_lines:?}",
            on_relay.map_or("never".into(), secs),
            back.map_or("never".into(), secs),
            after_answer.map_or("—".into(), |ms| format!("{:.1}s", ms as f64 / 1000.0)),
            leg.map_or(format!("never; uncut at {}", secs(uncut_at)), secs),
            longest as f64 / 1000.0,
            lines.iter().map(|(at, _, r)| format!("{r}@{}", secs(*at))).collect::<Vec<_>>(),
            now.as_ref().map_or("nothing".into(), word),
        ),
    );
    assert!(longest < 20_000, "no rest on the relay: {:.1}s", longest as f64 / 1000.0);
    if on_relay.is_some() {
        assert!(after_answer.is_some_and(|ms| ms <= 1500), "back within 1.5 s of the leg answering: {after_answer:?} ms");
    }
    assert_eq!(now.map(|l| l.hop), Some(Hop::Forwarded), "y ends on the carried road");
}

/// Echoes from `n` to `peer` (a listing of /tmp, as a keystroke's round
/// trip) one after another, 200 ms apart, until `until`; the slowest.
async fn typing(n: &crate::lab::LabNode, peer: &crate::lab::LabNode, until: std::time::Instant) -> Duration {
    let mut slowest = Duration::ZERO;
    while std::time::Instant::now() < until {
        let began = std::time::Instant::now();
        let to = peer.name.clone();
        let _ = tokio::time::timeout(
            Duration::from_secs(30),
            n.with(move |node| async move { node.ls_of(&to, "/tmp").await.map(|_| ()) }),
        )
        .await;
        slowest = slowest.max(began.elapsed());
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    slowest
}

/// f4i (flap): **a new connection's first road is not a move**. The phone
/// back in front lets its wires go and visits every machine; the new
/// connection to Y begins on the relay while the machine's road is the one
/// through M, and its first line read 「改走中继」, the next 「改为通过 m」:
/// a flap in the journal that never happened to the machine. The phone's
/// resume (`network_change_major`) is the app's call into its own
/// resident, which a lab resident has no door for; both ends letting their
/// wires go, X out of reach for a minute and more, back on the relay
/// before m, and X visiting Y makes the same new connection.
#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
async fn f4i_a_new_connection_does_not_write_its_first_road_as_a_move() {
    let s = "f4i";
    let (_lab, r, _rx, x, y, m) = f4_lab(s).await;
    let t = now_ms();
    // Both ends let their wires go, so the next round is a new connection.
    let yname = y.name.clone();
    x.with(move |n| async move { n.let_go_of(&yname).await.ok() }).await.expect("x lets go of y");
    let xname = x.name.clone();
    y.with(move |n| async move { n.let_go_of(&xname).await.ok() }).await.expect("y lets go of x");
    // As long as a phone in the background, and as unreachable: iroh's
    // actor for the remote stops after a minute with no connection
    // (`ACTOR_MAX_IDLE_TIMEOUT`), and the next connection has no carried
    // path to begin on. Reachable, the two redial within 15 s and the new
    // connection begins where the actor still is.
    cut_as(&x.dev, m.dev.ip().expect("m has IPv4"), "away_m");
    cut_as(&x.dev, r.dev.ip().expect("the relay has IPv4"), "away_relay");
    tokio::time::sleep(Duration::from_secs(75)).await;
    // Back with the relay first and m a little later (the phone's new
    // network reaches the relay before the hop is found): the new
    // connection begins on the relay, then moves through m.
    uncut(&x.dev, "away_relay");
    let u = now_ms();
    let yname = y.name.clone();
    x.with(move |n| async move { n.visit(&yname).await.ok() }).await.expect("x visits y");
    let on_relay = x.until(&y, Duration::from_secs(60), |l| l.hop == Hop::Relay && l.rtt_ms.is_some()).await;
    eprintln!("[netlab] f4i on the relay {:?}", on_relay.as_ref().map(|(t, _)| t));
    uncut(&x.dev, "away_m");
    let back = x.until(&y, Duration::from_secs(120), |l| l.hop == Hop::Forwarded && l.rtt_ms.is_some()).await;
    tokio::time::sleep(Duration::from_secs(15)).await;
    let (xl, yl) = (path_lines(&x, &y, t), path_lines(&y, &x, t));
    let fmt = |l: &[(u64, String, String)]| {
        l.iter().map(|(at, c, r)| format!("{c} {r}@{:.1}s", at.saturating_sub(t) as f64 / 1000.0)).collect::<Vec<_>>()
    };
    let since_t = |n: &crate::lab::LabNode, peer: &crate::lab::LabNode| {
        n.journal(Some(peer))
            .into_iter()
            .filter(|e| e["at"].as_u64().is_some_and(|a| a >= t))
            .map(|e| format!("{}@{:.1}s", e["e"].as_str().unwrap_or("?"), e["at"].as_u64().unwrap_or(t).saturating_sub(t) as f64 / 1000.0))
            .collect::<Vec<_>>()
    };
    eprintln!("[netlab] f4i x's lines {:?}; y's {:?}", since_t(&x, &y), since_t(&y, &x));
    // Back in reach: the new connection's first road line, on either end,
    // says where it began, and no line after `u` is a move to the relay.
    let after: Vec<(u64, String, String)> =
        xl.iter().chain(&yl).filter(|(at, c, _)| *at >= u && c != "opened").cloned().collect();
    let first = after.iter().min_by_key(|(at, _, _)| *at).cloned();
    let to_relay = after.iter().filter(|(_, c, r)| c == "selected" && r == "relay").count();
    let ok = back.is_some() && first.as_ref().is_some_and(|(_, c, _)| c == "started") && to_relay == 0;
    result(
        s,
        ok,
        format!(
            "both let go, x away 75 s, x visited y: back through m {}; first line back {}; x's lines {:?}, y's {:?}",
            back.as_ref().map_or("never in 120 s".into(), |(t, l)| format!("after {t:.1?} ({})", word(l))),
            first.as_ref().map_or("none".into(), |(at, c, r)| format!("{c} {r}@{:.1}s", at.saturating_sub(t) as f64 / 1000.0)),
            fmt(&xl),
            fmt(&yl),
        ),
    );
    assert!(back.is_some(), "x is back through m");
    let (_, change, _) = first.expect("a road line for the new connection");
    assert_eq!(change, "started", "the new connection's first road is where it began, not a move: x {:?}, y {:?}", fmt(&xl), fmt(&yl));
    assert_eq!(to_relay, 0, "no 「改走中继」 once back: x {:?}, y {:?}", fmt(&xl), fmt(&yl));
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
