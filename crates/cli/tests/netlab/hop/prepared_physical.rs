//! Four actual residents: a frozen first middle must not delay the second.
//! This observes actual attempts and carried business, not a transport adapter.
use super::*;
use crate::lab::LabNode;
use khor_core::RoadKind;
use std::collections::BTreeSet;

fn peer_hop(nodes: &[&LabNode], on: bool) {
    for node in nodes {
        let value = if on { "on" } else { "off" };
        khor_node::config::set(&node.home.0, "peer-hop", value).unwrap();
        assert_eq!(khor_node::config::on(&node.home.0, "peer-hop"), on);
    }
}

async fn warm_tunnel(from: &LabNode, to: &LabNode) {
    let name = to.name.clone();
    // The IPC pipe door uses the resident's existing endpoint. Port 9 is
    // unused in these fresh namespaces; its refusal leaves the tunnel up.
    let _ = tokio::time::timeout(
        Duration::from_secs(10),
        from.with(move |node| async move {
            node.pipe_via(&name, "127.0.0.1", 9, tokio::io::empty(), tokio::io::sink())
                .await
        }),
    )
    .await
    .expect("the resident completes its bounded tunnel probe");
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if from
                .journal(Some(to))
                .iter()
                .any(|row| row["e"] == "dial" && row["tunnel"] == true && !row["won"].is_null())
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("an actual successful resident-owned Tunnel dial is recorded");
}

fn lifecycle(node: &LabNode, peer: &LabNode, since: u64) -> Vec<serde_json::Value> {
    std::fs::read_to_string(node.home.0.join("serve.log"))
        .unwrap()
        .lines()
        .filter_map(|line| line.split_once("[khor-carried-lifecycle] "))
        .filter_map(|(_, json)| serde_json::from_str::<serde_json::Value>(json).ok())
        .filter(|row| {
            row["peer"].as_str() == Some(peer.id.as_str())
                && row["at"].as_u64().is_some_and(|at| at >= since)
        })
        .collect()
}

fn business<'a>(rows: &'a [serde_json::Value], phase: &str) -> Vec<&'a serde_json::Value> {
    rows.iter().filter(|row| row["phase"] == phase).collect()
}

fn wires(rows: &[&serde_json::Value]) -> BTreeSet<u64> {
    rows.iter()
        .filter_map(|row| row["details"]["wire"].as_u64())
        .collect()
}

fn streams(rows: &[&serde_json::Value]) -> BTreeSet<String> {
    rows.iter()
        .filter_map(|row| row["details"]["stream"].as_str().map(str::to_owned))
        .collect()
}

fn carrier_held(rows: &[serde_json::Value], before: u64) -> bool {
    let commits = business(rows, "link_attachment_committed");
    commits.len() == 1
        && commits[0]["at"].as_u64().is_some_and(|at| at <= before)
        && commits[0]["details"]["serial"].as_u64().is_some()
        && commits[0]["details"].get("previous_serial") == Some(&serde_json::Value::Null)
        && !rows.iter().any(|row| {
            matches!(
                row["phase"].as_str(),
                Some("link_detached" | "link_io_ended")
            )
        })
}

fn retired(node: &LabNode, peer: &LabNode, wires: &BTreeSet<u64>, since: u64) -> bool {
    node.journal(Some(peer)).iter().any(|row| {
        row["e"] == "retired"
            && row["at"].as_u64().is_some_and(|at| at >= since)
            && row["wire"]
                .as_u64()
                .is_some_and(|wire| wires.contains(&wire))
    })
}

async fn measured_candidates(
    x: &LabNode,
    y: &LabNode,
    m: &LabNode,
    n: &LabNode,
) -> Option<(u32, u32)> {
    let (target, first, second) = (y.id.clone(), m.id.clone(), n.id.clone());
    x.with(move |node| async move {
        let devices = node.devices().ok()?;
        if !devices
            .iter()
            .any(|device| device.id == target && device.hop_staged)
        {
            return None;
        }
        let links = node.links().await.ok()?;
        let cost = |id: &str| {
            let device = devices.iter().find(|device| device.id == id)?;
            let first = links.iter().find(|(peer, _)| peer == id)?;
            if first.1.hop != Hop::Direct {
                return None;
            }
            let second = node.roads_of(device).into_iter().find(|road| {
                road.to == target && road.road == RoadKind::Direct && road.via.is_none()
            })?;
            Some(first.1.rtt_ms? + u32::from(second.ms?))
        };
        Some((cost(&first)?, cost(&second)?))
    })
    .await
}

pub(super) async fn run() {
    let s = "f4r";
    let lab = Lab::new().await.unwrap();
    let relay = relay(&lab, s, "relay", Some(Duration::from_millis(800))).await;
    let (rx, ry, public) = (
        router(&lab, "natx", Net::Strict).await,
        router(&lab, "naty", Net::Strict).await,
        router(&lab, "public", Net::Public).await,
    );
    let dx = lab.add_device("x").uplink(rx.id()).build().await.unwrap();
    let dy = lab.add_device("y").uplink(ry.id()).build().await.unwrap();
    let dm = lab
        .add_device("m")
        .uplink(public.id())
        .build()
        .await
        .unwrap();
    let dn = lab
        .add_device("n")
        .uplink(public.id())
        .build()
        .await
        .unwrap();
    for device in [&dx, &dy, &dm] {
        slow(device, LEG).await;
    }
    slow(&dn, Duration::from_millis(80)).await;
    // Completion receipts bind real business streams to their connection.
    // This case includes lifecycle logging even outside a diagnostic suite;
    // the original three physical controls retain their existing modes.
    let diagnostics = [("KHOR_NET_DIAGNOSTICS", "lifecycle")];
    let mut x = node(s, dx, "x", &[&relay], &diagnostics).await;
    let mut y = node(s, dy, "y", &[&relay], &diagnostics).await;
    let m = node(s, dm, "m", &[&relay], &diagnostics).await;
    let n = node(s, dn, "n", &[&relay], &diagnostics).await;
    // The lower identity owns an outgoing hop if both residents ask.
    if y.id < x.id {
        std::mem::swap(&mut x, &mut y);
    }
    peer_hop(&[&x, &y, &m, &n], false);
    tokio::time::sleep(Duration::from_secs(5)).await;
    mesh(&[&x, &y, &m, &n]).await;
    for end in [&x, &y] {
        for middle in [&m, &n] {
            assert!(
                end.until(middle, Duration::from_secs(90), |link| {
                    link.hop == Hop::Direct && link.rtt_ms.is_some()
                })
                .await
                .is_some(),
                "{s}: warm actual direct legs before the fault"
            );
        }
    }
    for middle in [&m, &n] {
        warm_tunnel(&x, middle).await;
        warm_tunnel(middle, &y).await;
    }
    let costs = tokio::time::timeout(Duration::from_secs(60), async {
        loop {
            if let Some(costs) = measured_candidates(&x, &y, &m, &n).await {
                break costs;
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    })
    .await
    .expect("prepared hint and both actual two-leg road facts arrive");
    assert!(
        costs.0 < costs.1,
        "{s}: frozen M is actually ranked ahead of N"
    );
    eprintln!(
        "[netlab-prepared] {s} x={} y={} m={} n={} costs={costs:?}",
        x.id, y.id, m.id, n.id
    );

    cut_as(&x.dev, relay.dev.ip().unwrap(), "prepared_no_relay");
    let name = y.name.clone();
    x.with(move |node| async move { node.let_go_of(&name).await })
        .await
        .unwrap();
    let name = x.name.clone();
    y.with(move |node| async move { node.let_go_of(&name).await })
        .await
        .unwrap();
    // No carried link has ever been allowed in this fresh fixture.
    assert!(
        x.link(&y)
            .await
            .is_none_or(|link| link.hop != Hop::Forwarded)
    );
    m.freeze();
    x.freeze();
    y.freeze();
    peer_hop(&[&x, &y, &m, &n], true);
    let began = now_ms();
    let activated = std::time::Instant::now();
    crate::attempts::phase(s, "prepared_race_started", began);
    y.thaw();
    x.thaw();
    let name = y.name.clone();
    let (first_time, first_error) = crate::attempts::measure(
        s,
        &x.name,
        &y.name,
        Duration::from_millis(1500),
        x.with(move |node| async move { node.ls_of(&name, "/tmp").await.map(|_| ()) }),
    )
    .await;
    let first_response = activated.elapsed();
    let stopped = std::fs::read_to_string(format!("/proc/{}/status", m.pid()))
        .unwrap()
        .lines()
        .any(|line| line.starts_with("State:") && line.contains("T (stopped)"));
    // Restore the resident even when the first business contract failed.
    m.thaw();
    let restored = now_ms();
    crate::attempts::phase(s, "first_middle_resumed", restored);
    let via_n = |link: &khor_core::Link| {
        link.hop == Hop::Forwarded && link.via.as_deref() == Some(n.id.as_str())
    };
    let carried = x.until(&y, Duration::from_secs(2), via_n).await;
    let x_wires = wires(&business(
        &lifecycle(&x, &y, began),
        "list_response_received",
    ));
    let y_wires = wires(&business(&lifecycle(&y, &x, began), "list_response_sent"));
    let two_asks = x.journal(Some(&y)).iter().any(|row| {
        row["e"] == "hop"
            && row["at"]
                .as_u64()
                .is_some_and(|at| at >= began && at <= restored)
            && row["via"].as_str() == Some(n.id.as_str())
            && row["tried"] == 2
    });
    let hold = std::time::Instant::now();
    let mut samples = 0;
    let mut held = true;
    while hold.elapsed() < Duration::from_secs(10) || samples < 30 {
        let name = y.name.clone();
        let (_, error) = crate::attempts::measure(
            s,
            &x.name,
            &y.name,
            Duration::from_secs(2),
            x.with(move |node| async move { node.ls_of(&name, "/tmp").await.map(|_| ()) }),
        )
        .await;
        samples += 1;
        let (x_link, y_link) = (x.link(&y).await, y.link(&x).await);
        let current = error.is_none()
            && x_link.as_ref().is_some_and(via_n)
            && y_link.as_ref().is_some_and(via_n);
        if !current {
            eprintln!(
                "[netlab-prepared-link] {s} sample={samples} error={error:?} x_hop={:?} x_via={:?} y_hop={:?} y_via={:?}",
                x_link.as_ref().map(|link| link.hop),
                x_link.as_ref().and_then(|link| link.via.as_deref()),
                y_link.as_ref().map(|link| link.hop),
                y_link.as_ref().and_then(|link| link.via.as_deref()),
            );
        }
        // Keep all subsequent samples as evidence; one failed observation
        // still fails the original held gate for the entire case.
        held &= current;
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    // Path journal rows are deduplicated across a peer's connections and
    // cannot identify the business owner. Require every completed list on
    // both ends, matching stream ids, one wire per end and one uninterrupted
    // outer attachment. The first response must precede M's restoration.
    let x_rows = lifecycle(&x, &y, began);
    let y_rows = lifecycle(&y, &x, began);
    let received = business(&x_rows, "list_response_received");
    let sent = business(&y_rows, "list_response_sent");
    let expected = samples + 1;
    let selected_every = received.iter().all(|row| {
        row["details"]["measurements"]
            .as_array()
            .is_some_and(|paths| {
                paths
                    .iter()
                    .any(|path| path["selected"] == true && path["carried"] == true)
            })
    });
    let carrier_both = carrier_held(&x_rows, restored) && carrier_held(&y_rows, restored);
    let continuous = x_wires.len() == 1
        && y_wires.len() == 1
        && received.len() == expected
        && sent.len() == expected
        && wires(&received) == x_wires
        && wires(&sent) == y_wires
        && streams(&received).len() == expected
        && streams(&received) == streams(&sent)
        && received[0]["at"].as_u64().is_some_and(|at| at <= restored)
        && sent[0]["at"].as_u64().is_some_and(|at| at <= restored)
        && selected_every
        && carrier_both
        && !retired(&x, &y, &x_wires, restored)
        && !retired(&y, &x, &y_wires, restored);
    let ok = first_error.is_none()
        && first_response < Duration::from_millis(1500)
        && stopped
        && two_asks
        && carried.is_some()
        && held
        && samples >= 30
        && hold.elapsed() >= Duration::from_secs(10)
        && continuous;
    for (owner, peer) in [(&x, &y), (&y, &x), (&x, &m), (&x, &n)] {
        path_diagnostics(owner, peer);
    }
    for owner in [&x, &y, &m, &n] {
        sample_diagnostics(s, owner);
    }
    eprintln!(
        "[netlab-prepared-limits] {s} late_M_OK=unobserved cancellation=unobserved cpu=unknown bytes=unknown; restoring M does not prove a late admission occurred"
    );
    result(
        s,
        ok,
        format!(
            "activation_to_content={first_response:?} request={first_time:?} error={first_error:?} M_stopped={stopped} two_asks={two_asks} via_N={} samples={samples} hold={:?} held={held} continuous={continuous} selected_every={selected_every} carrier_both={carrier_both} x_wires={x_wires:?} y_wires={y_wires:?} received={} sent={} matching_streams={}",
            carried.is_some(),
            hold.elapsed(),
            received.len(),
            sent.len(),
            streams(&received) == streams(&sent)
        ),
    );
    assert!(ok, "{s}: prepared race and actual carried continuity");
}
