//! Both endpoints move before the next pair of real foreground requests.
use super::mobile::diagnostics;
use super::mobility::addresses;
use super::*;
use crate::nat::{Net, router};
use std::net::SocketAddr;

fn snapshot(n: &LabNode) -> serde_json::Value {
    serde_json::json!({
        "node":n.name,"configured_v4":n.dev.ip(),"configured_v6":n.dev.ip6(),
        "v4":addresses(&n.dev,"-4"),"v6":addresses(&n.dev,"-6"),
    })
}

fn present(state: &serde_json::Value, family: &str, address: &str) -> bool {
    state[family]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|interface| interface["addr_info"].as_array())
        .flatten()
        .any(|entry| entry["local"].as_str() == Some(address))
}

fn changed(
    before: &serde_json::Value,
    after: &serde_json::Value,
    family: &str,
    old: Option<String>,
    new: Option<String>,
) -> bool {
    match (old, new) {
        (Some(old), Some(new)) => {
            old != new
                && present(before, family, &old)
                && !present(after, family, &old)
                && present(after, family, &new)
        }
        _ => false,
    }
}

fn rules(r: &Router) -> serde_json::Value {
    let mut command = Command::new("nft");
    command
        .args(["list", "ruleset"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let output = r
        .spawn_command_sync(command)
        .unwrap()
        .wait_with_output()
        .unwrap();
    assert!(output.status.success(), "read actual owned router rules");
    serde_json::json!({"router":r.name(),"code":output.status.code(),
        "text":String::from_utf8_lossy(&output.stdout),
        "stderr":String::from_utf8_lossy(&output.stderr)})
}

async fn read(
    from: &LabNode,
    to: &LabNode,
    marker: &str,
    case: &str,
    within: Duration,
) -> (Duration, Option<String>) {
    let directory = to.home.0.join("both-replug");
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(directory.join(marker), b"fresh network read").unwrap();
    let (name, marker) = (to.name.clone(), marker.to_owned());
    let directory = directory.to_string_lossy().into_owned();
    crate::attempts::measure(
        case,
        &from.id,
        &to.name,
        within,
        from.with(move |node| async move {
            let (_, entries, truncated) = node.ls_of(&name, &directory).await?;
            if !truncated
                && entries
                    .iter()
                    .any(|entry| entry.name == marker && !entry.dir && entry.size == 18)
            {
                Ok(())
            } else {
                Err("fresh peer marker missing from actual directory response".to_owned())
            }
        }),
    )
    .await
}

async fn sample(
    from: &LabNode,
    to: &LabNode,
    direction: &str,
    s: &str,
    index: u32,
    pivot: Instant,
) -> serde_json::Value {
    let marker = format!("{direction}-{index:02}");
    let started = pivot.elapsed();
    let (elapsed, error) = read(
        from,
        to,
        &marker,
        &format!("{s}-read-{direction}"),
        Duration::from_secs(2),
    )
    .await;
    serde_json::json!({"event":"sample","direction":direction,"index":index,"marker":marker,
        "scheduled_us":u64::from(index)*500_000,"started_us":started.as_micros(),
        "ended_us":pivot.elapsed().as_micros(),"elapsed_us":elapsed.as_micros(),"error":error})
}

async fn session(
    a: &LabNode,
    b: &LabNode,
    cells: [&Router; 2],
    reflector: SocketAddr,
    s: &str,
    dual: bool,
) -> Result<serde_json::Value, String> {
    tokio::time::timeout(Duration::from_secs(120), async {
        mesh(&[a, b]).await;
        for (direction, from, to) in [("ab", a, b), ("ba", b, a)] {
            let (_, error) = read(
                from,
                to,
                "setup",
                &format!("{s}-setup-{direction}"),
                Duration::from_secs(30),
            )
            .await;
            if let Some(error) = error {
                return Err(error);
            }
        }
        let mut held = None;
        loop {
            let available = match (a.link(b).await, b.link(a).await) {
                (Some(a), Some(b)) => {
                    a.down_ms.is_none()
                        && b.down_ms.is_none()
                        && a.rtt_ms.is_some()
                        && b.rtt_ms.is_some()
                }
                _ => false,
            };
            if available {
                if held.get_or_insert_with(Instant::now).elapsed() >= Duration::from_secs(10) {
                    break;
                }
            } else {
                held = None;
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        Ok::<_, String>(())
    })
    .await
    .map_err(|_| "both-end preparation exceeded120s".to_owned())??;
    let before = [snapshot(a), snapshot(b)];
    let old = [(a.dev.ip(), a.dev.ip6()), (b.dev.ip(), b.dev.ip6())];
    let before_mapping = [
        a.dev
            .probe_udp_mapping(reflector)
            .map_err(|e| e.to_string())?,
        b.dev
            .probe_udp_mapping(reflector)
            .map_err(|e| e.to_string())?,
    ];
    let epoch = Instant::now();
    let move_a = async {
        let start = epoch.elapsed();
        a.dev
            .iface("eth0")
            .ok_or("a eth0 missing")?
            .replug(cells[0].id())
            .await
            .map_err(|e| e.to_string())?;
        Ok::<_, String>((start.as_micros(), epoch.elapsed().as_micros()))
    };
    let move_b = async {
        let start = epoch.elapsed();
        b.dev
            .iface("eth0")
            .ok_or("b eth0 missing")?
            .replug(cells[1].id())
            .await
            .map_err(|e| e.to_string())?;
        Ok::<_, String>((start.as_micros(), epoch.elapsed().as_micros()))
    };
    let (ma, mb) = tokio::join!(move_a, move_b);
    let intervals = [ma?, mb?];
    let pivot = Instant::now();
    // The first work after both moves is business, in both directions.
    // No diagnostic or mapping probe can teach Khor either new endpoint.
    let mut samples = Vec::with_capacity(40);
    for index in 0..20 {
        tokio::time::sleep_until((pivot + Duration::from_millis(500) * index).into()).await;
        let (ab, ba) = tokio::join!(
            sample(a, b, "ab", s, index, pivot),
            sample(b, a, "ba", s, index, pivot)
        );
        samples.extend([ab, ba]);
    }
    let after = [snapshot(a), snapshot(b)];
    let new = [(a.dev.ip(), a.dev.ip6()), (b.dev.ip(), b.dev.ip6())];
    let after_mapping = [
        a.dev
            .probe_udp_mapping(reflector)
            .map_err(|e| e.to_string())?,
        b.dev
            .probe_udp_mapping(reflector)
            .map_err(|e| e.to_string())?,
    ];
    let geometry = old.iter().zip(new.iter()).all(|((ov4, ov6), (nv4, nv6))| {
        ov4.is_some()
            && nv4.is_some()
            && ov4 != nv4
            && if dual {
                ov6.is_some()
                    && nv6.is_some()
                    && ov6 != nv6
                    && ov6.map(|v| v.octets()[..8].to_vec())
                        != nv6.map(|v| v.octets()[..8].to_vec())
            } else {
                ov6.is_none() && nv6.is_none()
            }
    });
    let kernel_geometry = (0..2).all(|i| {
        changed(
            &before[i],
            &after[i],
            "v4",
            old[i].0.map(|v| v.to_string()),
            new[i].0.map(|v| v.to_string()),
        ) && (!dual
            || changed(
                &before[i],
                &after[i],
                "v6",
                old[i].1.map(|v| v.to_string()),
                new[i].1.map(|v| v.to_string()),
            ))
    });
    let mapping_changed = before_mapping
        .iter()
        .zip(after_mapping.iter())
        .all(|(old, new)| old.ip() != new.ip());
    let first: Vec<_> = ["ab", "ba"]
        .into_iter()
        .map(|direction| {
            samples
                .iter()
                .find(|row| row["direction"] == direction && row["error"].is_null())
                .and_then(|row| row["ended_us"].as_u64())
        })
        .collect();
    let answers = samples.iter().all(|row| row["error"].is_null());
    let passed = geometry
        && kernel_geometry
        && mapping_changed
        && answers
        && first.iter().all(|v| v.is_some_and(|us| us <= 2_000_000));
    let report = serde_json::json!({"schema":1,"revision":"both_endpoint_replug_v1","event":"complete",
        "case":s,"dual_stack":dual,"before":before,"after":after,
        "before_mapping":before_mapping,"after_mapping":after_mapping,
        "movement_intervals_us":intervals,"pivot_us":pivot.duration_since(epoch).as_micros(),
        "read_count_per_direction":20,"caller_us":2_000_000,"period_us":500_000,
        "geometry":geometry,"kernel_geometry":kernel_geometry,"mapping_changed":mapping_changed,"first_reply_us":first,
        "all_answers":answers,"samples":samples,"passed":passed});
    eprintln!("[netlab-both-replug] {report}");
    if passed {
        Ok(report)
    } else {
        Err(format!("both-end business or geometry failed: {report}"))
    }
}

async fn run(dual: bool) {
    let s = if dual { "both-dual" } else { "both-v4" };
    let lab = Lab::new().await.unwrap();
    let relay = relay(&lab, s, "relay", Some(Duration::from_millis(100))).await;
    let network = |old| {
        if dual {
            if old { Net::Campus } else { Net::Cellular }
        } else if old {
            Net::Home
        } else {
            Net::Strict
        }
    };
    let wa = router(&lab, "wifi-a", network(true)).await;
    let wb = router(&lab, "wifi-b", network(true)).await;
    let ca = router(&lab, "cell-a", network(false)).await;
    let cb = router(&lab, "cell-b", network(false)).await;
    let public = lab
        .add_router("reflector-public")
        .preset(RouterPreset::Public)
        .ip_support(IpSupport::DualStack)
        .build()
        .await
        .unwrap();
    let rd = lab
        .add_device("reflector")
        .uplink(public.id())
        .build()
        .await
        .unwrap();
    let address = SocketAddr::new(rd.ip().unwrap().into(), 39999);
    let _reflector = rd.spawn_reflector(address).await.unwrap();
    let da = lab.add_device("a").uplink(wa.id()).build().await.unwrap();
    let db = lab.add_device("b").uplink(wb.id()).build().await.unwrap();
    let a = node(s, da, "both-a", &[&relay], &[]).await;
    let b = node(s, db, "both-b", &[&relay], &[]).await;
    for r in [&wa, &wb, &ca, &cb] {
        eprintln!(
            "[netlab-both-router] {}",
            serde_json::json!({"phase":"before","rules":rules(r)})
        );
    }
    let checked = tokio::time::timeout(
        Duration::from_secs(240),
        session(&a, &b, [&ca, &cb], address, s, dual),
    )
    .await;
    diagnostics(s, &a, &b);
    diagnostics(s, &b, &a);
    for r in [&wa, &wb, &ca, &cb] {
        eprintln!(
            "[netlab-both-router] {}",
            serde_json::json!({"phase":"after","rules":rules(r)})
        );
    }
    let ok = matches!(&checked, Ok(Ok(_)));
    result(
        s,
        ok,
        format!("both endpoints changed before real requests: {checked:?}"),
    );
    assert!(
        ok,
        "both endpoints recover fresh-marker business within original frozen callers"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
#[ignore = "both IPv4/NAT endpoints move before new business; guarded fixed triples"]
async fn both_v4_nat_endpoints_move_before_bidirectional_business() {
    run(false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
#[ignore = "both dual-stack endpoints change IPv4/NAT and IPv6 prefixes; guarded fixed triples"]
async fn both_dual_stack_endpoints_move_before_bidirectional_business() {
    run(true).await;
}
