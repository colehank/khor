//! Scenarios p1–p4: **two direct roads between the same two machines**,
//! and whether the connection rides the better one (the owner,
//! 2026-10-16: 「为什么明明有最优路径却没有走」).
//!
//! Each machine has a leg on two networks: a LAN router with no delay
//! (`eth0`, also the way to the relay) and a second network on another
//! subnet (`eth1`) — any second direct road, an overlay or a second
//! cable; nothing here knows or cares which. Both roads reach the other
//! machine directly, so iroh holds two candidate direct paths.
//!
//! - p1 **same speed**: both roads 1 ms, ten minutes. The connection
//!   should pick one and stay on it: at most one move. p1j the same at
//!   about 20 ms with jitter, past iroh's 10 ms "good enough", where it
//!   punches every 60 s.
//! - p2 **the road in use slows**: the connection on the second road,
//!   which then goes to about 21 ms round trip, while alpha types (an
//!   echo every 200 ms, as a terminal's keystrokes); back on the LAN
//!   within 10 s. p2i the same with nothing typed, the connection idle
//!   but for its keepalives: reported, not asserted (the owner's lag is
//!   typing; an idle connection has a sample every 5 s at best).
//! - p3 **the road in use goes black**: every packet on it dropped, an
//!   echo every 200 ms; no echo waits more than 1 s, and neither side's
//!   wire is retired.
//! - p4 **a false interface change**: the interfaces never move, but the
//!   machine's public addresses are both its interface's and what a QAD
//!   relay reports (turing's global IPv6, no NAT in front; here v4 and v6
//!   both, on a dual-stack network with a QAD stand-in on each family).
//!   Ten minutes, no 「本机网卡地址变化」.
//!
//! - p1s **one speed, stalls and a busy CPU** (net-flap, 2026-10-21): p1,
//!   with every 30 s a 300 ms latency spike on alpha's two legs and a
//!   keystroke in it, and both residents pinned to one CPU beside two busy
//!   loops: the machines' own stalls (turing on NFS). Counts the moves
//!   between the two roads; at most one.
//! - p5 **a short stall rides out** (net-flap): one LAN road and a relay
//!   500 ms away, the connection idle; every 30 s the LAN drops everything
//!   for 150, 300 or 600 ms while a keystroke is on the wire. A stall
//!   shorter than the relay's round trip moves nothing to the relay and
//!   starts no round of hole punching.
//!
//! The clocks are real: ten minutes are ten minutes. Shortening iroh's
//! timers (keepalive, the 60 s upgrade check, the net report's 20–26 s
//! rhythm) would change the very ratios the flapping comes from, so the
//! two long scenarios run side by side instead of faster.

use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime};

use khor_core::Hop;
use patchbay::{Device, IpSupport, Lab, LinkCondition, LinkDirection, Nat, Router, RouterPreset};

use crate::lab::{LabNode, mesh, node, qad_relays_dual, relay, result};
use crate::scratch::Reaped;

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

/// A network of its own subnet behind a home router (v4).
async fn network(lab: &Lab, name: &str) -> Router {
    lab.add_router(name)
        .nat(Nat::Moderate)
        .ip_support(IpSupport::V4Only)
        .build()
        .await
        .unwrap()
}

/// A machine with a leg on each network: `eth0` on `lan` (the default
/// route, so the relay is reached that way), `eth1` on `second`.
async fn two_legs(lab: &Lab, name: &str, lan: &Router, second: &Router) -> Device {
    lab.add_device(name)
        .iface("eth0", lan.id())
        .iface("eth1", second.id())
        .default_via("eth0")
        .build()
        .await
        .unwrap()
}

/// `one_way` ms on every packet out of `ifname` (and into it, with `Both`).
async fn delay(dev: &Device, ifname: &str, one_way: u32, dir: LinkDirection) {
    let iface = dev.iface(ifname).expect("the leg");
    if one_way == 0 {
        iface.clear_condition(dir).await.unwrap();
    } else {
        iface
            .set_condition(LinkCondition::new().latency_ms(one_way), dir)
            .await
            .unwrap();
    }
}

/// The journal's selections on `n`'s connections to `peer` since `since`
/// (ms since the epoch): when, the road, the address.
fn selections(n: &LabNode, peer: &LabNode, since: u64) -> Vec<(u64, String, String)> {
    n.journal(Some(peer))
        .into_iter()
        .filter(|e| {
            e["e"] == "path"
                && e["change"] == "selected"
                && e["at"].as_u64().is_some_and(|t| t >= since)
        })
        .map(|e| {
            (
                e["at"].as_u64().unwrap(),
                e["road"].as_str().unwrap_or("?").to_owned(),
                e["addr"].as_str().unwrap_or("?").to_owned(),
            )
        })
        .collect()
}

/// Which of `dev`'s legs an address names: `lan`, `second`, or the
/// address itself (a relay).
fn leg(dev: &Device, addr: &str) -> String {
    let ip = addr.rsplit_once(':').map_or(addr, |(ip, _)| ip);
    for (ifname, name) in [("eth0", "lan"), ("eth1", "second")] {
        if dev
            .iface(ifname)
            .and_then(|i| i.ip())
            .is_some_and(|v4| v4.to_string() == ip)
        {
            return name.to_owned();
        }
    }
    addr.to_owned()
}

/// `n`'s last selection toward `peer`, as a leg of the peer's: the latest
/// `path` selection line, or the selected path of a later `settled` line
/// (`*Ip(10.0.1.3:4242) 1ms`; a connection that never moved wrote no
/// selection before the watch read the road it began on).
fn current(n: &LabNode, peer: &LabNode) -> Option<String> {
    let selected = selections(n, peer, 0).pop();
    let settled = n
        .journal(Some(peer))
        .into_iter()
        .filter(|e| e["e"] == "settled")
        .filter_map(|e| {
            let first = e["paths"].as_array()?.first()?.as_str()?.to_owned();
            let addr = first.strip_prefix("*Ip(")?.split(')').next()?.to_owned();
            Some((e["at"].as_u64()?, "direct".to_owned(), addr))
        })
        .last();
    let (_, road, addr) = match (selected, settled) {
        (Some(a), Some(b)) => {
            if b.0 > a.0 {
                b
            } else {
                a
            }
        }
        (a, b) => a.or(b)?,
    };
    Some(if road == "direct" {
        leg(&peer.dev, &addr)
    } else {
        road
    })
}

/// Selections since `since`, as legs with seconds from `since`.
fn moves(n: &LabNode, peer: &LabNode, since: u64) -> Vec<String> {
    selections(n, peer, since)
        .into_iter()
        .map(|(at, road, addr)| {
            let what = if road == "direct" {
                leg(&peer.dev, &addr)
            } else {
                road
            };
            format!("{what}@{:.0}s", (at - since) as f64 / 1000.0)
        })
        .collect()
}

/// Journal lines of kind `e` on `n` (about `peer`, or its own) since `since`.
fn lines(n: &LabNode, peer: Option<&LabNode>, e: &str, since: u64) -> Vec<serde_json::Value> {
    n.journal(peer)
        .into_iter()
        .filter(|l| l["e"] == e && l["at"].as_u64().is_some_and(|t| t >= since))
        .collect()
}

/// Packets and bytes through every interface of `dev` but loopback, both
/// ways (`/proc/net/dev` read inside its namespace).
fn wire_traffic(dev: &Device) -> (u64, u64) {
    let mut cmd = Command::new("cat");
    cmd.arg("/proc/net/dev").stdout(Stdio::piped());
    let out = dev
        .spawn_command_sync(cmd)
        .unwrap()
        .wait_with_output()
        .unwrap();
    let (mut packets, mut bytes) = (0, 0);
    for l in String::from_utf8_lossy(&out.stdout).lines().skip(2) {
        let Some((name, cols)) = l.split_once(':') else {
            continue;
        };
        if name.trim() == "lo" {
            continue;
        }
        let n: Vec<u64> = cols
            .split_whitespace()
            .filter_map(|c| c.parse().ok())
            .collect();
        // rx bytes, rx packets … (8 columns), tx bytes, tx packets.
        if n.len() >= 10 {
            bytes += n[0] + n[8];
            packets += n[1] + n[9];
        }
    }
    (packets, bytes)
}

/// The rounds of hole punching this machine has started, toward anyone.
async fn punch_rounds(n: &LabNode) -> u64 {
    n.with(|node| async move {
        node.wire_counts()
            .await
            .map(|c| c.punch_rounds)
            .unwrap_or(0)
    })
    .await
}

/// Two machines with a leg on each of two networks, on one plain relay,
/// paired, and direct to each other on both sides.
struct Pair {
    _lab: Lab,
    _relay: crate::lab::Relay,
    a: LabNode,
    b: LabNode,
}

async fn pair(s: &str, before: impl AsyncFnOnce(&Device, &Device)) -> Pair {
    let lab = Lab::new().await.unwrap();
    let r = relay(&lab, s, "relay", None).await;
    let lan = network(&lab, "lan").await;
    let second = network(&lab, "second").await;
    let da = two_legs(&lab, "a", &lan, &second).await;
    let db = two_legs(&lab, "b", &lan, &second).await;
    before(&da, &db).await;
    let a = node(s, da, "alpha", &[&r], &[]).await;
    let b = node(s, db, "beta", &[&r], &[]).await;
    tokio::time::sleep(Duration::from_secs(3)).await;
    mesh(&[&a, &b]).await;
    for (x, y) in [(&a, &b), (&b, &a)] {
        x.until(y, Duration::from_secs(90), |l| l.hop == Hop::Direct)
            .await
            .unwrap_or_else(|| panic!("{} goes direct to {}", x.name, y.name));
    }
    Pair {
        _lab: lab,
        _relay: r,
        a,
        b,
    }
}

/// Ten minutes; `NETLAB_LONG_S` shortens it for a look at the setup.
fn long() -> Duration {
    Duration::from_secs(
        std::env::var("NETLAB_LONG_S")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10 * 60),
    )
}

/// Both roads `one_way` ms out of each machine (`jitter` ms either side),
/// ten minutes: at most one move, no wire retired.
async fn one_speed(s: &str, one_way: u32, jitter: u32) {
    one_speed_stalling(s, one_way, jitter, false).await;
}

/// [`one_speed`]; `stalls`: every 30 s a 300 ms spike of 300 ms on both
/// of alpha's legs with a keystroke in it, both residents on one CPU with
/// two busy loops (p1s).
async fn one_speed_stalling(s: &str, one_way: u32, jitter: u32, stalls: bool) {
    let p = pair(s, async |da, db| {
        for d in [da, db] {
            for i in ["eth0", "eth1"] {
                let cond = LinkCondition::new().latency_ms(one_way).jitter_ms(jitter);
                d.iface(i)
                    .unwrap()
                    .set_condition(cond, LinkDirection::Egress)
                    .await
                    .unwrap();
            }
        }
    })
    .await;
    // Settled: the first minute is the dial's.
    tokio::time::sleep(Duration::from_secs(30)).await;
    let since = now_ms();
    let (rounds0, traffic0) = (punch_rounds(&p.a).await, wire_traffic(&p.a.dev));
    let (mut spikes, mut slowest) = (0, Duration::ZERO);
    if stalls {
        let _busy = busy_beside(&[&p.a, &p.b]);
        let began = Instant::now();
        while began.elapsed() < long() {
            tokio::time::sleep(Duration::from_secs(30)).await;
            let cond = LinkCondition::new()
                .latency_ms(one_way + 300)
                .jitter_ms(jitter);
            for i in ["eth0", "eth1"] {
                p.a.dev
                    .iface(i)
                    .unwrap()
                    .set_condition(cond, LinkDirection::Egress)
                    .await
                    .unwrap();
            }
            let typed = echo(&p.a);
            let calm = async {
                tokio::time::sleep(Duration::from_millis(300)).await;
                let cond = LinkCondition::new().latency_ms(one_way).jitter_ms(jitter);
                for i in ["eth0", "eth1"] {
                    p.a.dev
                        .iface(i)
                        .unwrap()
                        .set_condition(cond, LinkDirection::Egress)
                        .await
                        .unwrap();
                }
            };
            let ((took, _), ()) = tokio::join!(typed, calm);
            slowest = slowest.max(took);
            spikes += 1;
        }
    } else {
        tokio::time::sleep(long()).await;
    }
    let (ma, mb) = (moves(&p.a, &p.b, since), moves(&p.b, &p.a, since));
    let rounds = punch_rounds(&p.a).await - rounds0;
    let traffic = wire_traffic(&p.a.dev);
    let retired = lines(&p.a, Some(&p.b), "retired", since).len()
        + lines(&p.b, Some(&p.a), "retired", since).len();
    let changes = lines(&p.a, None, "network", since).len();
    let worst = ma.len().max(mb.len());
    let ok = worst <= 1 && retired == 0;
    result(
        s,
        ok,
        format!(
            "{}±{} ms each way{}, over {:?}: alpha moved {} {ma:?}, beta {} {mb:?}; retired {retired}; alpha: {changes} network changes, {rounds} punch rounds, {} packets {} bytes",
            one_way,
            jitter,
            if stalls {
                format!(
                    ", {spikes} spikes of 300 ms (slowest keystroke {slowest:.1?}), one busy CPU"
                )
            } else {
                String::new()
            },
            long(),
            ma.len(),
            mb.len(),
            traffic.0 - traffic0.0,
            traffic.1 - traffic0.1,
        ),
    );
    assert!(
        worst <= 1,
        "at most one move in ten minutes: alpha {ma:?}, beta {mb:?}"
    );
    assert_eq!(retired, 0, "no wire retired");
}

/// p1: two LAN-fast roads (1 ms). iroh does not punch again while a
/// direct path is under 10 ms (`GOOD_ENOUGH_LATENCY`), so nothing opens
/// and nothing moves: the guard.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn p1_two_roads_of_one_speed_hold_still() {
    one_speed("p1", 1, 0).await;
}

/// p1j: two roads of one speed past 10 ms (about 20 ms round trip, a few
/// ms of jitter): the campus or the internet between two sites, where
/// iroh punches every 60 s and every round opens the other road again.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn p1j_two_jittery_roads_of_one_speed_hold_still() {
    one_speed("p1j", 10, 2).await;
}

/// p1s: p1 with the machines' own stalls (net-flap, 2026-10-21): turing
/// on NFS stalls, its ACKs come late, and the two LAN roads traded places
/// 230–330 times a day.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn p1s_two_roads_of_one_speed_hold_still_through_stalls() {
    one_speed_stalling("p1s", 1, 0, true).await;
}

/// Pins `nodes`' residents to one CPU and runs two busy loops on it, until
/// dropped: a machine short of CPU (p1s). The CPU is picked by this
/// process's pid, so scenarios side by side pick different ones.
fn busy_beside(nodes: &[&LabNode]) -> Vec<Reaped> {
    let cpus = std::thread::available_parallelism().map_or(1, |n| n.get());
    let cpu = (std::process::id() as usize % cpus).to_string();
    for n in nodes {
        let _ = Command::new("taskset")
            .args(["-a", "-p", "-c", &cpu, &n.pid().to_string()])
            .stdout(Stdio::null())
            .status();
    }
    (0..2)
        .filter_map(|_| {
            Command::new("taskset")
                .args(["-c", &cpu, "sh", "-c", "while :; do :; done"])
                .spawn()
                .ok()
                .map(Reaped)
        })
        .collect()
}

/// p5 (net-flap, 2026-10-21): one direct road and a relay 500 ms away;
/// every 30 s the road drops everything for 150, 300, then 600 ms, four
/// times each, while a keystroke is on the wire (the connection otherwise
/// idle). While the stall is shorter than the relay's round trip the
/// connection should ride it out: no move to the relay, no round of hole
/// punching. MacLab↔hinton left a 2 ms road for the 500 ms relay and came
/// back within a second 212 times a day.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn p5_a_short_stall_rides_out_and_punches_nothing() {
    let s = "p5";
    let lab = Lab::new().await.unwrap();
    let r = relay(&lab, s, "relay", Some(Duration::from_millis(500))).await;
    let lan = network(&lab, "lan").await;
    let da = lab.add_device("a").uplink(lan.id()).build().await.unwrap();
    let db = lab.add_device("b").uplink(lan.id()).build().await.unwrap();
    let a = node(s, da, "alpha", &[&r], &[]).await;
    let b = node(s, db, "beta", &[&r], &[]).await;
    tokio::time::sleep(Duration::from_secs(5)).await;
    mesh(&[&a, &b]).await;
    for (x, y) in [(&a, &b), (&b, &a)] {
        x.until(y, Duration::from_secs(120), |l| l.hop == Hop::Direct)
            .await
            .unwrap_or_else(|| panic!("{} goes direct to {}", x.name, y.name));
    }
    tokio::time::sleep(Duration::from_secs(30)).await;
    let ip_b = b.dev.ip().unwrap().to_string();
    let drops: u32 = std::env::var("NETLAB_P5_DROPS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(4);
    let mut short_ok = true;
    let mut said = vec![];
    for stall in [150u64, 300, 600] {
        let since = now_ms();
        let rounds0 = (punch_rounds(&a).await, punch_rounds(&b).await);
        let mut slowest = Duration::ZERO;
        for _ in 0..drops {
            tokio::time::sleep(Duration::from_secs(30)).await;
            let typed = echo(&a);
            let cut = async {
                drop_between(&a.dev, &ip_b, true);
                tokio::time::sleep(Duration::from_millis(stall)).await;
                drop_between(&a.dev, &ip_b, false);
            };
            let ((took, _), ()) = tokio::join!(typed, cut);
            slowest = slowest.max(took);
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
        let relayed = |n: &LabNode, peer: &LabNode| {
            selections(n, peer, since)
                .into_iter()
                .filter(|(_, road, _)| road == "relay")
                .count()
        };
        let (ra, rb) = (relayed(&a, &b), relayed(&b, &a));
        let rounds = (
            punch_rounds(&a).await - rounds0.0,
            punch_rounds(&b).await - rounds0.1,
        );
        if stall < 500 {
            short_ok &= ra == 0 && rb == 0 && rounds == (0, 0);
        }
        said.push(format!(
            "{stall} ms ×{drops}: to the relay alpha {ra} beta {rb}, punch rounds alpha {} beta {}, slowest keystroke {slowest:.1?}; alpha {:?}",
            rounds.0,
            rounds.1,
            moves(&a, &b, since),
        ));
    }
    result(s, short_ok, said.join("; "));
    assert!(
        short_ok,
        "a stall shorter than the relay's round trip moves nothing and punches nothing: {said:?}"
    );
}

/// Drops (`on`) or stops dropping everything between `dev` and `ip` (nft,
/// in `dev`).
fn drop_between(dev: &Device, ip: &str, on: bool) {
    let rules = if on {
        format!(
            "table inet netlab_flap {{\n chain flap_in {{ type filter hook input priority -10; policy accept;\n  ip saddr {ip} drop\n }}\n chain flap_out {{ type filter hook output priority -10; policy accept;\n  ip daddr {ip} drop\n }}\n}}\n"
        )
    } else {
        "delete table inet netlab_flap\n".to_owned()
    };
    let mut cmd = Command::new("nft");
    cmd.args(["-f", "-"]).stdin(Stdio::piped());
    let mut child = dev.spawn_command_sync(cmd).unwrap();
    use std::io::Write;
    child
        .stdin
        .take()
        .unwrap()
        .write_all(rules.as_bytes())
        .unwrap();
    assert!(child.wait().unwrap().success(), "nft took:\n{rules}");
}

/// A listing of beta's /tmp from alpha: one request and its answer on
/// the wire, a keystroke's worth. How long it took, or why it failed.
async fn echo(a: &LabNode) -> (Duration, Option<String>) {
    let t = Instant::now();
    let got = tokio::time::timeout(
        Duration::from_secs(30),
        a.with(|n| async move { n.ls_of("beta", "/tmp").await.map(|_| ()) }),
    )
    .await;
    let error = match got {
        Ok(Ok(())) => None,
        Ok(Err(e)) => Some(e),
        Err(_) => Some("no answer in 30 s".to_owned()),
    };
    (t.elapsed(), error)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn p2_the_road_in_use_slows_and_the_lan_takes_over() {
    slows("p2", true).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn p2i_the_road_in_use_slows_while_idle() {
    slows("p2i", false).await;
}

/// p2 (`typing`: an echo every 200 ms from alpha) and p2i (nothing).
async fn slows(s: &str, typing: bool) {
    // The LAN slow at first (30 ms), so the connection lands on the
    // second road.
    let p = pair(s, async |da, _| {
        delay(da, "eth0", 15, LinkDirection::Both).await
    })
    .await;
    let began = Instant::now();
    while [current(&p.a, &p.b), current(&p.b, &p.a)]
        .iter()
        .any(|c| c.as_deref() != Some("second"))
    {
        assert!(
            began.elapsed() < Duration::from_secs(90),
            "both on the second road: {:?} {:?}",
            current(&p.a, &p.b),
            current(&p.b, &p.a)
        );
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    // Both roads the same again; the connection may stay where it is.
    delay(&p.a.dev, "eth0", 0, LinkDirection::Both).await;
    tokio::time::sleep(Duration::from_secs(20)).await;
    let held = [current(&p.a, &p.b), current(&p.b, &p.a)];
    // Now the road in use slows to about 21 ms.
    let since = now_ms();
    let slowed = Instant::now();
    delay(&p.a.dev, "eth1", 10, LinkDirection::Both).await;
    delay(&p.b.dev, "eth1", 1, LinkDirection::Egress).await;
    let (mut back, mut echoes) = (None, 0);
    while slowed.elapsed() < Duration::from_secs(90) {
        if [current(&p.a, &p.b), current(&p.b, &p.a)]
            .iter()
            .all(|c| c.as_deref() == Some("lan"))
        {
            back = Some(slowed.elapsed());
            break;
        }
        if typing {
            let (_, error) = echo(&p.a).await;
            assert_eq!(error, None, "every echo answers");
            echoes += 1;
            tokio::time::sleep(Duration::from_millis(200)).await;
        } else {
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }
    let retired = lines(&p.a, Some(&p.b), "retired", since).len()
        + lines(&p.b, Some(&p.a), "retired", since).len();
    let ok = back.is_some_and(|b| b <= Duration::from_secs(10)) && retired == 0;
    result(
        s,
        ok,
        format!(
            "held {held:?} at equal speed; {} after the slowdown both on the LAN {}; alpha {:?}, beta {:?}; retired {retired}",
            if typing {
                format!("typing ({echoes} echoes),")
            } else {
                "idle,".to_owned()
            },
            back.map_or("not within 90 s".into(), |b| format!("after {b:.1?}")),
            moves(&p.a, &p.b, since),
            moves(&p.b, &p.a, since),
        ),
    );
    assert_eq!(retired, 0, "no wire retired");
    if typing {
        assert!(
            back.is_some_and(|b| b <= Duration::from_secs(10)),
            "back on the LAN within 10 s, took {back:?}"
        );
    }
}

/// Drops every packet between `dev` and `ip`, both ways (nft, in `dev`).
fn blackhole(dev: &Device, ip: &str) {
    let rules = format!(
        "table inet netlab_black {{\n chain black_in {{ type filter hook input priority -10; policy accept;\n  ip saddr {ip} drop\n }}\n chain black_out {{ type filter hook output priority -10; policy accept;\n  ip daddr {ip} drop\n }}\n}}\n"
    );
    let mut cmd = Command::new("nft");
    cmd.args(["-f", "-"]).stdin(Stdio::piped());
    let mut child = dev.spawn_command_sync(cmd).unwrap();
    use std::io::Write;
    child
        .stdin
        .take()
        .unwrap()
        .write_all(rules.as_bytes())
        .unwrap();
    assert!(
        child.wait().unwrap().success(),
        "nft took the blackhole:\n{rules}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn p3_the_road_in_use_goes_black_and_nothing_waits() {
    let s = "p3";
    // The second road 8 ms slower, past the 5 ms it takes to move between
    // direct roads, so the LAN is the road in use; within the 10 ms a
    // spare may be behind, so the second road is kept as the spare.
    let p = pair(s, async |da, _| {
        delay(da, "eth1", 4, LinkDirection::Both).await
    })
    .await;
    let began = Instant::now();
    while current(&p.a, &p.b).as_deref() != Some("lan") {
        assert!(
            began.elapsed() < Duration::from_secs(90),
            "alpha on the LAN: {:?}",
            current(&p.a, &p.b)
        );
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    tokio::time::sleep(Duration::from_secs(10)).await;
    let mut before = vec![];
    for _ in 0..10 {
        let (took, error) = echo(&p.a).await;
        assert_eq!(error, None, "the echo answers before the blackhole");
        before.push(took);
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    let lan_b = p.b.dev.iface("eth0").unwrap().ip().unwrap().to_string();
    let since = now_ms();
    blackhole(&p.a.dev, &lan_b);
    let blacked = Instant::now();
    let (mut waits, mut failed, mut why) = (vec![], 0, None);
    while blacked.elapsed() < Duration::from_secs(60) {
        let (took, error) = echo(&p.a).await;
        waits.push(took);
        failed += usize::from(error.is_some());
        why = why.or(error);
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    let worst = waits.iter().max().copied().unwrap_or_default();
    let slow: Vec<String> = waits
        .iter()
        .filter(|w| **w > Duration::from_millis(300))
        .map(|w| format!("{w:.1?}"))
        .collect();
    let retired: Vec<String> = lines(&p.a, Some(&p.b), "retired", since)
        .into_iter()
        .map(|l| format!("alpha:{}", l["why"]))
        .chain(
            lines(&p.b, Some(&p.a), "retired", since)
                .into_iter()
                .map(|l| format!("beta:{}", l["why"])),
        )
        .collect();
    let ok = worst <= Duration::from_secs(1) && failed == 0 && retired.is_empty();
    result(
        s,
        ok,
        format!(
            "echo before {:.1?} at most; after the blackhole {} echoes, worst {worst:.1?}, {failed} failed (first: {why:?}), over 300 ms {slow:?}; retired {retired:?}; alpha {:?}, beta {:?}",
            before.iter().max().unwrap(),
            waits.len(),
            moves(&p.a, &p.b, since),
            moves(&p.b, &p.a, since),
        ),
    );
    assert!(
        worst <= Duration::from_secs(1),
        "no echo waits more than 1 s: worst {worst:?}"
    );
    assert_eq!(failed, 0, "every echo answered");
    assert!(retired.is_empty(), "no wire retired: {retired:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn p4_an_address_both_interface_and_qad_is_no_network_change() {
    let s = "p4";
    let lab = Lab::new().await.unwrap();
    let r = relay(&lab, s, "relay", None).await;
    let qad = qad_relays_dual(&lab, s, &["qad"]).await;
    // Public routers, no NAT: each machine's public addresses are its
    // interface's own, and the QAD stand-in reports those same addresses.
    let (ra, rb) = (
        lab.add_router("left")
            .preset(RouterPreset::Public)
            .build()
            .await
            .unwrap(),
        lab.add_router("right")
            .preset(RouterPreset::Public)
            .build()
            .await
            .unwrap(),
    );
    let da = lab.add_device("a").uplink(ra.id()).build().await.unwrap();
    let db = lab.add_device("b").uplink(rb.id()).build().await.unwrap();
    let ca = qad.ca.to_string_lossy().into_owned();
    let env = [
        (khor_net::endpoint::PUBLIC_RELAYS_ENV, qad.env.as_str()),
        (khor_net::endpoint::RELAY_CA_ENV, ca.as_str()),
    ];
    let a = node(s, da, "alpha", &[&r], &env).await;
    let b = node(s, db, "beta", &[&r], &env).await;
    tokio::time::sleep(Duration::from_secs(3)).await;
    mesh(&[&a, &b]).await;
    a.until(&b, Duration::from_secs(90), |l| l.hop == Hop::Direct)
        .await
        .expect("alpha goes direct");
    tokio::time::sleep(Duration::from_secs(30)).await;
    let since = now_ms();
    let (rounds0, traffic0) = (punch_rounds(&a).await, wire_traffic(&a.dev));
    tokio::time::sleep(long()).await;
    let changed = |n: &LabNode| {
        lines(n, None, "network", since)
            .into_iter()
            .filter(|l| l["source"] == "addrs")
            .count()
    };
    let (ca_, cb_) = (changed(&a), changed(&b));
    let rounds = punch_rounds(&a).await - rounds0;
    let traffic = wire_traffic(&a.dev);
    let learnt: Vec<String> = lines(&a, None, "learnt", 0)
        .iter()
        .map(|l| format!("v4 {} v6 {} from {}", l["v4"], l["v6"], l["from"]))
        .collect();
    let homes: Vec<String> = lines(&a, None, "home", 0)
        .iter()
        .map(|l| format!("{} {}", l["url"], l["state"]))
        .collect();
    let addrs: Vec<String> = lines(&a, None, "addrs", 0)
        .iter()
        .map(|l| format!("{}", l["addrs"]))
        .collect();
    eprintln!("[netlab] p4 alpha homes {homes:?}; addrs {addrs:?}");
    let v4 = format!("{:?} {:?}", a.dev.ip(), a.dev.ip6());
    let ok = ca_ == 0 && cb_ == 0;
    result(
        s,
        ok,
        format!(
            "over {:?}: interface changes alpha {ca_}, beta {cb_}; alpha's interface {v4}, QAD said {learnt:?}; alpha: {rounds} punch rounds, {} packets {} bytes, {} selections",
            long(),
            traffic.0 - traffic0.0,
            traffic.1 - traffic0.1,
            selections(&a, &b, since).len(),
        ),
    );
    assert_eq!(
        (ca_, cb_),
        (0, 0),
        "no interface change when no interface moved"
    );
}
