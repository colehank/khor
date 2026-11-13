//! A healthy relay can become slower without needing a new TCP connection.
use std::collections::BTreeSet;
use std::net::{Ipv4Addr, SocketAddr};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use khor_core::Hop;
use patchbay::{Device, Lab};
use serde_json::{Value, json};

use super::flows;
use crate::lab::{LabNode, mesh, node, relay, result};
use crate::nat::{Net, router};

fn command(dev: &Device, program: &str, args: &[&str]) -> String {
    let mut cmd = Command::new(program);
    cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());
    let out = dev
        .spawn_command_sync(cmd)
        .unwrap()
        .wait_with_output()
        .unwrap();
    assert!(
        out.status.success(),
        "{program} {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

fn readback(dev: &Device, interface: &str) -> Value {
    json!({
        "qdisc":serde_json::from_str::<Value>(&command(dev,"tc",&["-s","-j","qdisc","show","dev",interface])).unwrap(),
        "filter":serde_json::from_str::<Value>(&command(dev,"tc",&["-s","-j","filter","show","dev",interface,"parent","1:"])).unwrap(),
    })
}

fn held_flows(dev: &Device, ip: Ipv4Addr, port: u16) -> BTreeSet<(u16, u16)> {
    flows(dev, ip)
        .into_iter()
        .filter(|(local, _)| *local == port)
        .collect()
}

fn sample_flows(
    dev: &Device,
    ip: Ipv4Addr,
    port: u16,
    expected: &BTreeSet<(u16, u16)>,
    pivot: Instant,
) -> Value {
    let started = pivot.elapsed().as_micros();
    let held = held_flows(dev, ip, port);
    json!({"at_us":started,"ended_us":pivot.elapsed().as_micros(),"same":held==*expected,"flows":held})
}

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
            Err("healthy relay omitted the fresh scratch marker".into())
        }
    })
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "healthy relay jitter contract; guarded fixed three-case protocol"]
async fn healthy_delay_spikes_keep_the_same_relay_tcp_flow() {
    let s = "relay-jitter";
    let lab = Lab::new().await.unwrap();
    let r = relay(&lab, s, "relay", None).await;
    let ra = router(&lab, "left", Net::Strict).await;
    let rb = router(&lab, "right", Net::Strict).await;
    let a_ip = ra.uplink_ip().unwrap();
    let addr: SocketAddr = r
        .url
        .strip_prefix("http://")
        .unwrap()
        .trim_end_matches('/')
        .parse()
        .unwrap();
    let port = addr.port();
    let route: Value = serde_json::from_str(&command(
        &r.dev,
        "ip",
        &["-j", "route", "get", &a_ip.to_string()],
    ))
    .unwrap();
    let interface = route[0]["dev"].as_str().unwrap().to_owned();
    assert_ne!(interface, "lo");
    command(
        &r.dev,
        "tc",
        &[
            "qdisc", "replace", "dev", &interface, "root", "handle", "1:", "prio", "bands", "3",
        ],
    );
    command(
        &r.dev,
        "tc",
        &[
            "qdisc", "add", "dev", &interface, "parent", "1:3", "handle", "30:", "netem", "delay",
            "124ms", "limit", "1000",
        ],
    );
    command(
        &r.dev,
        "tc",
        &[
            "filter",
            "add",
            "dev",
            &interface,
            "protocol",
            "ip",
            "parent",
            "1:",
            "prio",
            "1",
            "u32",
            "match",
            "ip",
            "protocol",
            "6",
            "0xff",
            "match",
            "ip",
            "dst",
            &format!("{a_ip}/32"),
            "match",
            "ip",
            "sport",
            &port.to_string(),
            "0xffff",
            "flowid",
            "1:3",
        ],
    );
    let before_shape = readback(&r.dev, &interface);
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
        .expect("both residents use the actual single relay");
    }
    let left = a.home.0.join("healthy-jitter");
    let right = b.home.0.join("healthy-jitter");
    for directory in [&left, &right] {
        std::fs::create_dir_all(directory).unwrap();
        std::fs::write(directory.join("setup-marker"), b"setup").unwrap();
    }
    let left = left.to_string_lossy().into_owned();
    let right = right.to_string_lossy().into_owned();
    let mut baseline = Vec::new();
    for (case, from, to, directory) in [
        ("relay-jitter-setup-ab", &a, "beta", right.as_str()),
        ("relay-jitter-setup-ba", &b, "alpha", left.as_str()),
    ] {
        let (elapsed, error) = crate::attempts::measure(
            case,
            &from.id,
            to,
            Duration::from_secs(30),
            checked_list(from, to, directory, "setup-marker"),
        )
        .await;
        assert!(error.is_none(), "healthy baseline: {error:?}");
        baseline.push(elapsed.as_micros());
    }
    assert!(
        baseline[0] >= 80_000,
        "actual alpha reply traverses the initial delay"
    );
    let expected = held_flows(&r.dev, a_ip, port);
    assert!(
        !expected.is_empty(),
        "real established relay TCP flow exists"
    );
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert_eq!(
        held_flows(&r.dev, a_ip, port),
        expected,
        "baseline connection is stable before the pivot"
    );
    command(
        &r.dev,
        "tc",
        &[
            "qdisc", "change", "dev", &interface, "parent", "1:3", "handle", "30:", "netem",
            "delay", "976ms", "limit", "1000",
        ],
    );
    let pivot = Instant::now();
    let after_shape = readback(&r.dev, &interface);
    eprintln!(
        "[netlab-relay-jitter] {}",
        json!({
            "schema":1,"revision":"healthy_relay_jitter_v1","event":"manifest",
            "alpha_public_ip":a_ip,"relay_port":port,"interface":interface,"route":route,
            "before_shape":before_shape,"after_shape":after_shape,"baseline_us":baseline,"old_tcp_flows":expected,
            "initial_delay_us":124000,"post_delay_us":976000,"setup_reads":2,"read_count":12,
            "caller_us":5000000,"period_us":500000,"flow_period_us":100000,"tail_observation_us":2000000,
            "no_drop_reset_or_address_change":true,
        })
    );
    let mut clock = tokio::time::interval(Duration::from_millis(100));
    clock.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut flow_samples = vec![sample_flows(&r.dev, a_ip, port, &expected, pivot)];
    let mut samples = Vec::with_capacity(12);
    for index in 0..12 {
        let scheduled = pivot + Duration::from_millis(500) * index;
        tokio::time::sleep_until(scheduled.into()).await;
        let marker = format!("healthy-after-{index:02}");
        std::fs::write(std::path::Path::new(&right).join(&marker), b"fresh").unwrap();
        let started = Instant::now();
        let reading = crate::attempts::measure(
            "relay-jitter-read",
            &a.id,
            "beta",
            Duration::from_secs(5),
            checked_list(&a, "beta", &right, &marker),
        );
        tokio::pin!(reading);
        let (elapsed, error) = loop {
            tokio::select! {
                outcome = &mut reading => break outcome,
                _ = clock.tick() => flow_samples.push(sample_flows(&r.dev,a_ip,port,&expected,pivot)),
            }
        };
        samples.push(json!({"schema":1,"revision":"healthy_relay_jitter_v1","event":"sample",
            "index":index,"marker":marker,"scheduled_us":scheduled.duration_since(pivot).as_micros(),
            "started_us":started.duration_since(pivot).as_micros(),"ended_us":pivot.elapsed().as_micros(),
            "elapsed_us":elapsed.as_micros(),"error":error}));
    }
    let tail_until = tokio::time::Instant::now() + Duration::from_secs(2);
    loop {
        tokio::select! {
            _ = tokio::time::sleep_until(tail_until) => break,
            _ = clock.tick() => flow_samples.push(sample_flows(&r.dev,a_ip,port,&expected,pivot)),
        }
    }
    flow_samples.push(sample_flows(&r.dev, a_ip, port, &expected, pivot));
    let final_shape = readback(&r.dev, &interface);
    let unchanged = flow_samples.iter().all(|sample| sample["same"] == true);
    let answers = samples.iter().all(|sample| sample["error"].is_null());
    let delayed = samples
        .iter()
        .any(|sample| sample["elapsed_us"].as_u64().unwrap() >= 800_000);
    let no_drops = final_shape["qdisc"]
        .as_array()
        .unwrap()
        .iter()
        .find(|qdisc| qdisc["kind"] == "netem")
        .is_some_and(|qdisc| qdisc["drops"].as_u64() == Some(0));
    for sample in &samples {
        eprintln!("[netlab-relay-jitter] {sample}");
    }
    let ok = unchanged && answers && delayed && no_drops;
    eprintln!(
        "[netlab-relay-jitter] {}",
        json!({
            "schema":1,"revision":"healthy_relay_jitter_v1","event":"complete","samples":samples.len(),
            "tcp_unchanged":unchanged,"all_answers":answers,"delay_observed":delayed,"no_queue_drops":no_drops,
            "flow_samples":flow_samples,"final_shape":final_shape,"passed":ok,
        })
    );
    result(
        s,
        ok,
        format!(
            "healthy124→976ms relay, answers={answers}, same TCP={unchanged}, no drops={no_drops}"
        ),
    );
    assert!(
        ok,
        "a healthy slower relay keeps its TCP flow and real replies"
    );
}
