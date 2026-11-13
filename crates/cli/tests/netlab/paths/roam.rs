//! Node business outcomes while the existing patchbay replug moves a real interface.
use super::mobile::{complete, diagnostics, fixture, reads, summary};
use super::mobility::addresses;
use super::*;
use std::net::SocketAddr;

#[derive(Clone, Copy)]
enum Move {
    V4Nat,
    DualStack,
    V4ToV6,
}

impl Move {
    fn case(self) -> &'static str {
        match self {
            Self::V4Nat => "roam-v4-nat",
            Self::DualStack => "roam-dual",
            Self::V4ToV6 => "roam-v4-to-v6",
        }
    }

    fn revision(self) -> &'static str {
        match self {
            Self::V4Nat => "node_v4_nat_replug_v1",
            Self::DualStack => "node_dual_stack_replug_v1",
            Self::V4ToV6 => "node_v4_to_v6_replug_v1",
        }
    }
}

fn snapshot(dev: &Device, s: &str, phase: &str) -> serde_json::Value {
    let value = serde_json::json!({
        "case": s, "phase": phase, "at_ms": now_ms(),
        "configured_v4": dev.ip(), "configured_v6": dev.ip6(),
        "v4": addresses(dev, "-4"), "v6": addresses(dev, "-6"),
    });
    eprintln!("[netlab-roam-addresses] {value}");
    value
}

fn router_state(router: &Router, s: &str, phase: &str) {
    let mut command = Command::new("nft");
    command
        .args(["-j", "list", "ruleset"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let output = router
        .spawn_command_sync(command)
        .unwrap()
        .wait_with_output()
        .unwrap();
    // Retain stdout even when the older nft cannot encode a rule as JSON.
    // The textual listing below is the independent ruleset record.
    let stdout = String::from_utf8(output.stdout).unwrap();
    let (rules, json_error) = match serde_json::from_str::<serde_json::Value>(&stdout) {
        Ok(rules) => (Some(rules), None),
        Err(error) => (None, Some(error.to_string())),
    };
    let mut command = Command::new("nft");
    command
        .args(["list", "ruleset"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let text = router
        .spawn_command_sync(command)
        .unwrap()
        .wait_with_output()
        .unwrap();
    let text_stdout = String::from_utf8(text.stdout).unwrap();
    eprintln!(
        "[netlab-roam-router] {}",
        serde_json::json!({
            "case": s, "router": router.name(), "phase": phase, "rules": rules,
            "json_error": json_error,
            "stdout": stdout, "stderr": String::from_utf8_lossy(&output.stderr),
            "code": output.status.code(),
            "text": text_stdout, "text_stderr": String::from_utf8_lossy(&text.stderr),
            "text_code": text.status.code(),
        })
    );
    assert!(
        output.status.success(),
        "read the actual JSON ruleset output"
    );
    assert!(text.status.success(), "read the actual textual ruleset");
}

async fn business(
    a: &LabNode,
    b: &LabNode,
    cell: &Router,
    reflector: SocketAddr,
    movement: Move,
) -> Result<serde_json::Value, String> {
    let s = movement.case();
    tokio::time::timeout(Duration::from_secs(120), async {
        mesh(&[a, b]).await;
        for (from, to) in [(a, b), (b, a)] {
            let checked = reads(&format!("{s}-setup"), from, to, 1).await;
            if !complete(&checked, 1) {
                return Err(format!("pre-move business failed: {}", summary(&checked)));
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
        Ok(())
    })
    .await
    .map_err(|_| "roam preparation exceeded 120 seconds".to_owned())??;
    crate::attempts::phase(s, "ready", now_ms());
    let before_mapping = a
        .dev
        .probe_udp_mapping(reflector)
        .map_err(|e| e.to_string())?;
    let before_ab = reads(&format!("{s}-before-ab"), a, b, 30).await;
    let before_ba = reads(&format!("{s}-before-ba"), b, a, 30).await;
    if !complete(&before_ab, 30) || !complete(&before_ba, 30) {
        return Err(format!(
            "before-move reads incomplete: ab {} ba {}",
            summary(&before_ab),
            summary(&before_ba)
        ));
    }
    let before = snapshot(&a.dev, s, "before");
    let (old_v4, old_v6) = (a.dev.ip(), a.dev.ip6());
    crate::attempts::phase(s, "replug_started", now_ms());
    a.dev
        .iface("eth0")
        .ok_or("mover eth0 missing")?
        .replug(cell.id())
        .await
        .map_err(|e| e.to_string())?;
    crate::attempts::phase(s, "replug_finished", now_ms());
    // No moving-side foreground business, probes or diagnostics before this request.
    let first_ba = reads(&format!("{s}-first-ba"), b, a, 1).await;
    crate::attempts::phase(s, "first_remote_finished", now_ms());
    let first_ab = reads(&format!("{s}-first-ab"), a, b, 1).await;
    let after_ab = reads(&format!("{s}-after-ab"), a, b, 30).await;
    let after_ba = reads(&format!("{s}-after-ba"), b, a, 30).await;
    crate::attempts::phase(s, "reads_finished", now_ms());
    let after = snapshot(&a.dev, s, "after");
    let (new_v4, new_v6) = (a.dev.ip(), a.dev.ip6());
    // The reflector uses a separate fixed local UDP port, not Khor's QUIC socket.
    // Probe only after all business measurements so it cannot teach Khor the new path.
    let after_mapping = new_v4.map(|_| {
        a.dev
            .probe_udp_mapping(reflector)
            .map_err(|e| e.to_string())
    });
    let geometry = match movement {
        Move::V4Nat => {
            old_v4.is_some()
                && new_v4.is_some()
                && old_v4 != new_v4
                && old_v6.is_none()
                && new_v6.is_none()
        }
        Move::DualStack => {
            old_v4.is_some()
                && new_v4.is_some()
                && old_v4 != new_v4
                && old_v6.is_some()
                && new_v6.is_some()
                && old_v6 != new_v6
                && old_v6.map(|ip| ip.octets()[..8].to_vec())
                    != new_v6.map(|ip| ip.octets()[..8].to_vec())
        }
        Move::V4ToV6 => {
            old_v4.is_some() && old_v6.is_none() && new_v4.is_none() && new_v6.is_some()
        }
    };
    let mapping_changed = match &after_mapping {
        Some(Ok(after)) => before_mapping.ip() != after.ip(),
        None => matches!(movement, Move::V4ToV6),
        _ => false,
    };
    let report = serde_json::json!({
        "case": s, "revision": movement.revision(), "nodes": [a.name, b.name],
        "before": before, "after": after, "geometry": geometry,
        "before_udp_mapping": before_mapping, "after_udp_mapping": after_mapping,
        "mapping_changed": mapping_changed,
        "before_ab": summary(&before_ab), "before_ba": summary(&before_ba),
        "first_ab": summary(&first_ab), "first_ba": summary(&first_ba),
        "after_ab": summary(&after_ab), "after_ba": summary(&after_ba),
    });
    eprintln!("[netlab-roam-result] {report}");
    if !geometry
        || !mapping_changed
        || !complete(&first_ba, 1)
        || !complete(&first_ab, 1)
        || !complete(&after_ab, 30)
        || !complete(&after_ba, 30)
    {
        return Err(format!("roam business incomplete: {report}"));
    }
    Ok(report)
}

async fn run(movement: Move) {
    use crate::nat::{Net, router};
    let s = movement.case();
    let lab = Lab::new().await.unwrap();
    let relay = relay(&lab, s, "relay", Some(Duration::from_millis(100))).await;
    let wifi = router(
        &lab,
        "wifi",
        match movement {
            Move::DualStack => Net::Campus,
            _ => Net::Home,
        },
    )
    .await;
    let cell = match movement {
        Move::V4Nat => router(&lab, "cell", Net::Strict).await,
        Move::DualStack => router(&lab, "cell", Net::Cellular).await,
        Move::V4ToV6 => lab
            .add_router("cell")
            .preset(RouterPreset::Public)
            .ip_support(IpSupport::V6Only)
            .build()
            .await
            .unwrap(),
    };
    let far = lab
        .add_router("far")
        .preset(RouterPreset::Public)
        .ip_support(IpSupport::DualStack)
        .build()
        .await
        .unwrap();
    let da = lab.add_device("a").uplink(wifi.id()).build().await.unwrap();
    let db = lab.add_device("b").uplink(far.id()).build().await.unwrap();
    let reflector = SocketAddr::new(db.ip().unwrap().into(), 39999);
    let _reflector = db.spawn_reflector(reflector).await.unwrap();
    let a = node(s, da, "roam-a", &[&relay], &[]).await;
    let b = node(s, db, "roam-b", &[&relay], &[]).await;
    fixture(&a);
    fixture(&b);
    router_state(&wifi, s, "before");
    router_state(&cell, s, "before");
    crate::attempts::phase(s, "session_started", now_ms());
    let checked = tokio::time::timeout(
        Duration::from_secs(240),
        business(&a, &b, &cell, reflector, movement),
    )
    .await;
    diagnostics(s, &a, &b);
    diagnostics(s, &b, &a);
    router_state(&wifi, s, "after");
    router_state(&cell, s, "after");
    let ok = matches!(&checked, Ok(Ok(_)));
    result(s, ok, format!("{} {checked:?}", movement.revision()));
    assert!(
        ok,
        "complete remote-first business reads across the actual uplink move"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
#[ignore = "real Node IPv4 and NAT change, complete business outcomes"]
async fn roam_v4_nat_remote_first() {
    run(Move::V4Nat).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
#[ignore = "real Node IPv4/NAT and IPv6 prefix change together"]
async fn roam_dual_stack_remote_first() {
    run(Move::DualStack).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
#[ignore = "real Node loses IPv4 including the HTTP relay and gains IPv6"]
async fn roam_v4_to_v6_only_remote_first() {
    run(Move::V4ToV6).await;
}
