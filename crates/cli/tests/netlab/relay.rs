//! Scenarios b and c: the relay itself misbehaving.
//!
//! b. A far relay (1.5 s round trip, the shape of n0's aps1 through the
//!    TUN, net-f §f.7) beside a near one (Beijing): the home settles on
//!    the near one and stays there across a full net report (every 5 min,
//!    `FULL_REPORT_INTERVAL`). Both plain, as khor's own relays are;
//!    the far one serving QUIC address discovery, as n0's do, is the
//!    case patches 11 and 14 are about and needs the net-direct branch.
//!
//! c. A relay TCP connection goes silent mid-life: every packet of the
//!    flows that exist at that moment is dropped, in both directions, and
//!    nobody sends a reset. That is MacLab's proxy TUN on 2026-10-09
//!    (net-f §f.8): a socket to euc1 "Established" for 20 minutes with
//!    frozen counters. New flows pass, as through the TUN. The node has
//!    to notice and reconnect; the scenario measures how long it takes.

use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime};

use patchbay::{Device, Lab};

use crate::lab::{mesh, node, relay, result, word};
use crate::nat::{Net, router};

/// How long a silent relay flow may go unnoticed before the node is on a
/// fresh one and its peer hears it again.
const NOTICE: Duration = Duration::from_secs(30);

/// Established TCP flows on `dev` whose far end is `ip`, as
/// (local port, far port).
fn flows(dev: &Device, ip: std::net::Ipv4Addr) -> Vec<(u16, u16)> {
    let mut cmd = Command::new("ss");
    cmd.args(["-Htn", "state", "established"]).stdout(Stdio::piped());
    let out = dev.spawn_command_sync(cmd).unwrap().wait_with_output().unwrap();
    let port = |s: &str| s.rsplit(':').next().and_then(|p| p.parse().ok());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let cols: Vec<&str> = l.split_whitespace().collect();
            let (local, far) = (cols.get(2)?, cols.get(3)?);
            far.starts_with(&format!("{ip}:")).then(|| Some((port(local)?, port(far)?)))?
        })
        .collect()
}

/// Drops every packet of these flows on `dev`, both ways, with nft.
/// (`in` and `out` are nft keywords, not chain names.)
fn wedge(dev: &Device, ip: std::net::Ipv4Addr, flows: &[(u16, u16)]) {
    let mut rules = String::from("table inet netlab_wedge {\n chain wedge_in { type filter hook input priority -10; policy accept;\n");
    for (_, far) in flows {
        rules += &format!("  ip saddr {ip} tcp sport {far} drop\n");
    }
    rules += " }\n chain wedge_out { type filter hook output priority -10; policy accept;\n";
    for (_, far) in flows {
        rules += &format!("  ip daddr {ip} tcp dport {far} drop\n");
    }
    rules += " }\n}\n";
    let mut cmd = Command::new("nft");
    cmd.args(["-f", "-"]).stdin(Stdio::piped());
    let mut child = dev.spawn_command_sync(cmd).unwrap();
    use std::io::Write;
    child.stdin.take().unwrap().write_all(rules.as_bytes()).unwrap();
    assert!(child.wait().unwrap().success(), "nft took the wedge:\n{rules}");
}

/// One full net report and a minute: long enough for the home to have
/// been chosen twice by a full report.
const WATCH: Duration = Duration::from_secs(6 * 60 + 30);

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn b1_the_home_settles_on_the_near_relay_and_stays() {
    let s = "b1";
    let lab = Lab::new().await.unwrap();
    let near = relay(&lab, s, "near", None).await;
    let far = relay(&lab, s, "far", Some(Duration::from_millis(1500))).await;
    let rh = router(&lab, "home", Net::Home).await;
    let dh = lab.add_device("h").uplink(rh.id()).build().await.unwrap();
    // The far relay first in the list, so the list's order cannot be
    // what puts the home on the near one.
    let n = node(s, dh, "alpha", &[&far, &near], &[]).await;
    tokio::time::sleep(WATCH).await;
    let homes: Vec<(f64, String)> = n
        .journal(None)
        .into_iter()
        .filter(|e| e["e"] == "home" && e["state"] == "connected")
        .map(|e| {
            let url = e["url"].as_str().unwrap_or("?");
            let which = if url.contains(near.url.trim_start_matches("http://").trim_end_matches('/')) { "near" } else { "far" };
            (e["at"].as_u64().unwrap() as f64 / 1000.0, which.to_owned())
        })
        .collect();
    let t0 = homes.first().map_or(0.0, |(t, _)| *t);
    let moves: Vec<String> = homes.iter().map(|(t, w)| format!("{w}@{:.0}s", t - t0)).collect();
    let last = homes.last().map(|(_, w)| w.as_str());
    let ok = last == Some("near") && homes.iter().skip_while(|(_, w)| w != "near").all(|(_, w)| w == "near");
    result(s, ok, format!("homes over {WATCH:?}: {moves:?}"));
    assert_eq!(last, Some("near"), "the home is the near relay");
    assert!(ok, "once near, the home stays near: {moves:?}");
}

/// b2: the case that flapped. Two far stand-ins for n0's tier answer QUIC
/// address discovery, the near plain relay does not (MacLab: aps1 and
/// euc1 against Beijing). Upstream's full report stops at two QAD answers
/// before the near relay's HTTPS probe is in; patch 14 makes it wait,
/// patch 11 keeps the home through the incremental reports between.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn b2_the_home_stays_near_when_far_relays_answer_qad() {
    let s = "b2";
    let lab = Lab::new().await.unwrap();
    let near = relay(&lab, s, "near", None).await;
    let far = crate::lab::qad_relays(&lab, s, &["far1", "far2"], Some(Duration::from_millis(1500))).await;
    let rh = router(&lab, "home", Net::Home).await;
    let dh = lab.add_device("h").uplink(rh.id()).build().await.unwrap();
    let ca = far.ca.to_string_lossy().into_owned();
    let n = node(
        s,
        dh,
        "alpha",
        &[&near],
        &[(khor_net::endpoint::PUBLIC_RELAYS_ENV, far.env.as_str()), (khor_net::endpoint::RELAY_CA_ENV, ca.as_str())],
    )
    .await;
    tokio::time::sleep(WATCH).await;
    let near_host = near.url.trim_start_matches("http://").trim_end_matches('/').to_owned();
    let homes: Vec<(f64, String)> = n
        .journal(None)
        .into_iter()
        .filter(|e| e["e"] == "home" && e["state"] == "connected")
        .map(|e| {
            let url = e["url"].as_str().unwrap_or("?");
            let which = if url.contains(&near_host) {
                "near".to_owned()
            } else {
                far.urls.iter().position(|u| url.contains(u.trim_start_matches("https://"))).map_or(url.to_owned(), |i| format!("far{}", i + 1))
            };
            (e["at"].as_u64().unwrap() as f64 / 1000.0, which)
        })
        .collect();
    let t0 = homes.first().map_or(0.0, |(t, _)| *t);
    let moves: Vec<String> = homes.iter().map(|(t, w)| format!("{w}@{:.0}s", t - t0)).collect();
    let last = homes.last().map(|(_, w)| w.as_str());
    let ok = last == Some("near") && homes.iter().skip_while(|(_, w)| w != "near").all(|(_, w)| w == "near");
    let published = n.published_relays();
    result(s, ok, format!("homes over {WATCH:?}: {moves:?}; publishes {:?}", published.first()));
    assert_eq!(last, Some("near"), "the home is the near relay");
    assert!(ok, "once near, the home stays near: {moves:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn c_a_relay_flow_gone_silent_is_noticed_and_replaced() {
    let s = "c";
    let lab = Lab::new().await.unwrap();
    let r = relay(&lab, s, "relay", None).await;
    let (ra, rb) = (router(&lab, "left", Net::Strict).await, router(&lab, "right", Net::Strict).await);
    let da = lab.add_device("a").uplink(ra.id()).build().await.unwrap();
    let db = lab.add_device("b").uplink(rb.id()).build().await.unwrap();
    let a = node(s, da, "alpha", &[&r], &[]).await;
    let b = node(s, db, "beta", &[&r], &[]).await;
    tokio::time::sleep(Duration::from_secs(3)).await;
    mesh(&[&a, &b]).await;
    let (_, before) = b
        .until(&a, Duration::from_secs(60), |l| l.rtt_ms.is_some())
        .await
        .expect("beta reaches alpha by the relay");

    let a_ip = ra.uplink_ip().expect("alpha's NAT has a public address");
    let old = flows(&r.dev, a_ip);
    assert!(!old.is_empty(), "alpha holds a relay connection");
    let at_ms = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis() as u64;
    wedge(&r.dev, a_ip, &old);
    let wedged = Instant::now();
    eprintln!("[netlab] wedged alpha's relay flows {old:?}");

    let mut fresh = None;
    let mut heard = None;
    let mut down_seen = false;
    while wedged.elapsed() < Duration::from_secs(120) && (fresh.is_none() || heard.is_none()) {
        if fresh.is_none() && flows(&r.dev, a_ip).iter().any(|f| !old.contains(f)) {
            fresh = Some(wedged.elapsed());
        }
        if let Some(l) = b.link(&a).await {
            down_seen |= l.down_ms.is_some();
            let since = wedged.elapsed().as_millis() as u64;
            if heard.is_none() && l.heard_ms.is_some_and(|h| h + 500 < since) {
                heard = Some(Duration::from_millis(since - l.heard_ms.unwrap()));
            }
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    let after = |e: &serde_json::Value| e["at"].as_u64().is_some_and(|t| t >= at_ms);
    let homes: Vec<String> = a
        .journal(None)
        .into_iter()
        .filter(|e| e["e"] == "home" && after(e))
        .map(|e| format!("{}@{:.1}s", e["state"].as_str().unwrap_or("?"), (e["at"].as_u64().unwrap() - at_ms) as f64 / 1000.0))
        .collect();
    let last = b.link(&a).await;
    let ok = fresh.is_some_and(|t| t <= NOTICE) && heard.is_some_and(|t| t <= NOTICE);
    result(
        s,
        ok,
        format!(
            "before {}; new relay flow {}; beta hears alpha again {}; wire down seen {down_seen}; alpha's home lines {homes:?}; now {}",
            word(&before),
            fresh.map_or("never in 120 s".into(), |t| format!("after {t:.1?}")),
            heard.map_or("never in 120 s".into(), |t| format!("after {t:.1?}")),
            last.as_ref().map_or("nothing".into(), word),
        ),
    );
    assert!(fresh.is_some_and(|t| t <= NOTICE), "alpha replaces its silent relay flow within {NOTICE:?}");
    assert!(heard.is_some_and(|t| t <= NOTICE), "beta hears alpha again within {NOTICE:?}");
}
