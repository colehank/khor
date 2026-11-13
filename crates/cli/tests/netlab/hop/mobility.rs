//! A peer relay moves to another public network while its clients keep reading.
use super::*;
use crate::lab::LabNode;
use crate::paths::mobile::{complete, diagnostics, fixture, reads, summary};
use std::process::{Command, Stdio};

const REVISION: &str = "peer_relay_replug_v1";

fn addresses(m: &LabNode, phase: &str) -> serde_json::Value {
    let mut command = Command::new("ip");
    command
        .args(["-j", "address", "show"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let output = m
        .dev
        .spawn_command_sync(command)
        .unwrap()
        .wait_with_output()
        .unwrap();
    assert!(output.status.success());
    let kernel: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let state = serde_json::json!({
        "revision": REVISION, "phase": phase, "at_ms": now_ms(),
        "configured_v4": m.dev.ip(), "configured_v6": m.dev.ip6(), "kernel": kernel,
    });
    eprintln!("[netlab-peer-mobility-addresses] {state}");
    state
}

async fn held(s: &str, x: &LabNode, y: &LabNode, m: &LabNode, phase: &str) {
    let mut held = None;
    loop {
        let both = match (x.link(y).await, y.link(x).await) {
            (Some(a), Some(b)) => {
                a.hop == Hop::Forwarded
                    && b.hop == Hop::Forwarded
                    && a.via.as_deref() == Some(m.id.as_str())
                    && b.via.as_deref() == Some(m.id.as_str())
            }
            _ => false,
        };
        if both {
            let since = held.get_or_insert_with(|| {
                crate::attempts::phase(s, &format!("{phase}_hold_started"), now_ms());
                std::time::Instant::now()
            });
            if since.elapsed() >= Duration::from_secs(10) {
                crate::attempts::phase(s, &format!("{phase}_held"), now_ms());
                return;
            }
        } else if held.take().is_some() {
            crate::attempts::phase(s, &format!("{phase}_hold_reset"), now_ms());
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

async fn business(
    s: &str,
    x: &LabNode,
    y: &LabNode,
    m: &LabNode,
    next: &patchbay::Router,
) -> Result<serde_json::Value, String> {
    tokio::time::timeout(Duration::from_secs(150), async {
        for (from, to) in [(x, y), (y, x)] {
            let checked = reads(&format!("{s}-setup"), from, to, 1).await;
            if !complete(&checked, 1) {
                return Err(format!("initial business failed: {}", summary(&checked)));
            }
        }
        held(s, x, y, m, "prepare").await;
        Ok(())
    })
    .await
    .map_err(|_| "peer relay preparation exceeded 150 seconds".to_owned())??;
    let before_ab = reads(&format!("{s}-before-ab"), x, y, 30).await;
    let before_ba = reads(&format!("{s}-before-ba"), y, x, 30).await;
    if !complete(&before_ab, 30) || !complete(&before_ba, 30) {
        return Err(format!(
            "before-move reads incomplete: ab {} ba {}",
            summary(&before_ab),
            summary(&before_ba)
        ));
    }
    // Recheck immediately before injection: the baseline workload may itself
    // have changed the selector, so an earlier carried summary is insufficient.
    tokio::time::timeout(Duration::from_secs(45), held(s, x, y, m, "before_move"))
        .await
        .map_err(|_| "both carried roads did not hold before moving the middle".to_owned())?;
    let old = m.dev.ip().ok_or("middle needs IPv4")?;
    let before = addresses(m, "before");
    crate::attempts::phase(s, "middle_replug_started", now_ms());
    m.dev
        .iface("eth0")
        .ok_or("middle eth0 missing")?
        .replug(next.id())
        .await
        .map_err(|e| e.to_string())?;
    // Replug recreates the interface; retain the original 20 ms device RTT.
    slow(&m.dev, LEG).await;
    crate::attempts::phase(s, "middle_replug_finished", now_ms());
    let first_ab_case = format!("{s}-first-ab");
    let first_ba_case = format!("{s}-first-ba");
    let (first_ab, first_ba) = tokio::join!(
        reads(&first_ab_case, x, y, 1),
        reads(&first_ba_case, y, x, 1),
    );
    let after_ab = reads(&format!("{s}-after-ab"), x, y, 30).await;
    let after_ba = reads(&format!("{s}-after-ba"), y, x, 30).await;
    crate::attempts::phase(s, "reads_finished", now_ms());
    let after = addresses(m, "after");
    // This is a bounded diagnostic observation, not a new optimal-route SLO.
    let recovered = tokio::time::timeout(Duration::from_secs(60), held(s, x, y, m, "after_move"))
        .await
        .is_ok();
    let report = serde_json::json!({
        "case": s, "revision": REVISION, "nodes": [x.name, y.name, m.name],
        "middle": m.id, "before": before, "after": after,
        "old": old, "new": m.dev.ip(), "carried_observed_again": recovered,
        "before_ab": summary(&before_ab), "before_ba": summary(&before_ba),
        "first_ab": summary(&first_ab), "first_ba": summary(&first_ba),
        "after_ab": summary(&after_ab), "after_ba": summary(&after_ba),
    });
    eprintln!("[netlab-peer-mobility-result] {report}");
    if m.dev.ip().is_none()
        || m.dev.ip() == Some(old)
        || !complete(&first_ab, 1)
        || !complete(&first_ba, 1)
        || !complete(&after_ab, 30)
        || !complete(&after_ba, 30)
    {
        return Err(format!("peer relay mobility business incomplete: {report}"));
    }
    Ok(report)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
#[ignore = "real peer relay IPv4 network move; availability calibration, not an optimal-route SLO"]
async fn pm_middle_changes_network_while_both_clients_read() {
    let s = "peer-middle-v4";
    let (lab, _relay, _rx, x, y, m) = super::f4_topology(s, false).await;
    let next = router(&lab, "middle-next", Net::Public).await;
    fixture(&x);
    fixture(&y);
    crate::attempts::phase(s, "session_started", now_ms());
    let checked =
        tokio::time::timeout(Duration::from_secs(360), business(s, &x, &y, &m, &next)).await;
    for (from, to) in [(&x, &y), (&y, &x), (&x, &m), (&m, &x), (&y, &m), (&m, &y)] {
        diagnostics(s, from, to);
    }
    let ok = matches!(&checked, Ok(Ok(_)));
    result(s, ok, format!("{REVISION} {checked:?}"));
    assert!(
        ok,
        "complete business outcomes while the peer relay changes network"
    );
}
