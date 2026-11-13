//! Foreground reads on the original silently wedged relay geometry.
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime};

use khor_core::Hop;
use patchbay::Lab;
use serde_json::json;

use super::{flows, wedge};
use crate::lab::{LabNode, mesh, node, relay, result};
use crate::nat::{Net, router};

async fn checked_list(
    from: &LabNode,
    to: &str,
    directory: &str,
    marker: &str,
) -> Result<(), String> {
    let (to, directory, marker) = (to.to_owned(), directory.to_owned(), marker.to_owned());
    from.with(|n| async move {
        let (_, entries, truncated) = n.ls_of(&to, &directory).await?;
        if !truncated && entries.iter().any(|entry| entry.name == marker) {
            Ok(())
        } else {
            Err("remote listing omitted the fresh scratch marker".into())
        }
    })
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "real foreground relay-fault contract; guarded fixed three-case protocol"]
async fn c1_real_reads_continue_when_the_active_relay_tcp_flow_is_silent() {
    let s = "c-demand";
    let lab = Lab::new().await.unwrap();
    let r = relay(&lab, s, "relay", None).await;
    let ra = router(&lab, "left", Net::Strict).await;
    let rb = router(&lab, "right", Net::Strict).await;
    let da = lab.add_device("a").uplink(ra.id()).build().await.unwrap();
    let db = lab.add_device("b").uplink(rb.id()).build().await.unwrap();
    let a = node(s, da, "alpha", &[&r], &[]).await;
    let b = node(s, db, "beta", &[&r], &[]).await;
    tokio::time::sleep(Duration::from_secs(3)).await;
    mesh(&[&a, &b]).await;
    for (from, to) in [(&a, &b), (&b, &a)] {
        from.until(to, Duration::from_secs(60), |l| {
            l.hop == Hop::Relay && l.rtt_ms.is_some()
        })
        .await
        .expect("both residents have the original relay path");
    }
    let left = a.home.0.join("relay-demand");
    let right = b.home.0.join("relay-demand");
    for directory in [&left, &right] {
        std::fs::create_dir_all(directory).unwrap();
        std::fs::write(directory.join("setup-marker"), b"setup").unwrap();
    }
    let left = left.to_string_lossy().into_owned();
    let right = right.to_string_lossy().into_owned();
    for (case, from, to, directory) in [
        ("c-demand-setup-ab", &a, "beta", right.as_str()),
        ("c-demand-setup-ba", &b, "alpha", left.as_str()),
    ] {
        let (_, error) = crate::attempts::measure(
            case,
            &from.id,
            to,
            Duration::from_secs(30),
            checked_list(from, to, directory, "setup-marker"),
        )
        .await;
        assert!(error.is_none(), "real setup request: {error:?}");
    }
    let a_ip = ra.uplink_ip().unwrap();
    let old = flows(&r.dev, a_ip);
    assert!(!old.is_empty(), "alpha holds a real relay TCP flow");
    let epoch = Instant::now();
    let wall_ms = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    wedge(&r.dev, a_ip, &old);
    let pivot = Instant::now();
    let mut command = Command::new("nft");
    command
        .args(["-j", "list", "table", "inet", "netlab_wedge"])
        .stdout(Stdio::piped());
    let rules = r
        .dev
        .spawn_command_sync(command)
        .unwrap()
        .wait_with_output()
        .unwrap();
    assert!(
        rules.status.success(),
        "actual wedge rules can be read back"
    );
    let rules: serde_json::Value = serde_json::from_slice(&rules.stdout).unwrap();
    eprintln!(
        "[netlab-relay-demand] {}",
        json!({
            "schema":1,"revision":"relay_silent_actual_demand_v1","event":"manifest",
            "wall_ms":wall_ms,"wedge_started_us":0,"wedge_finished_us":pivot.duration_since(epoch).as_micros(),
            "alpha_public_ip":a_ip,"old_tcp_flows":old,"nft_readback":rules,
            "read_count":40,"caller_us":2000000,"period_us":250000,"setup_reads":2,
            "direct_or_peer_relay_alternative":false,"product_changes":false,
        })
    );
    let mut samples = Vec::with_capacity(40);
    for index in 0..40 {
        let scheduled = pivot + Duration::from_millis(250) * index;
        tokio::time::sleep_until(scheduled.into()).await;
        let marker = format!("after-wedge-{index:02}");
        std::fs::write(std::path::Path::new(&right).join(&marker), b"fresh").unwrap();
        let started = Instant::now();
        let (elapsed, error) = crate::attempts::measure(
            "c-demand-read",
            &a.id,
            "beta",
            Duration::from_secs(2),
            checked_list(&a, "beta", &right, &marker),
        )
        .await;
        samples.push(json!({
            "schema":1,"revision":"relay_silent_actual_demand_v1","event":"sample",
            "index":index,"marker":marker,
            "scheduled_us":scheduled.duration_since(pivot).as_micros(),
            "started_us":started.duration_since(pivot).as_micros(),
            "ended_us":pivot.elapsed().as_micros(),"elapsed_us":elapsed.as_micros(),"error":error,
        }));
    }
    let first = samples
        .iter()
        .find(|row| row["error"].is_null())
        .map(|row| row["ended_us"].as_u64().unwrap());
    let all_answers = samples.iter().all(|row| row["error"].is_null());
    for row in &samples {
        eprintln!("[netlab-relay-demand] {row}");
    }
    let ok = all_answers && first.is_some_and(|us| us <= 2_000_000);
    eprintln!(
        "[netlab-relay-demand] {}",
        json!({
            "schema":1,"revision":"relay_silent_actual_demand_v1","event":"complete",
            "samples":samples.len(),"all_answers":all_answers,"first_valid_response_us":first,"passed":ok,
        })
    );
    result(
        s,
        ok,
        format!(
            "40 real foreground reads, all_answers={all_answers}; first valid response {first:?} us"
        ),
    );
    assert!(
        ok,
        "foreground responses keep their original two-second caller and fresh marker"
    );
}
