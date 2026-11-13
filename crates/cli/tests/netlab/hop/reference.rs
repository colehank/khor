//! Same f4 geometry, with one eligible road or the ordinary selector.
use super::*;
use crate::lab::LabNode;

fn diagnostics(s: &str, x: &LabNode, y: &LabNode, m: &LabNode) {
    super::hop_diagnostics(x, y, m);
    for n in [x, y, m] {
        super::sample_diagnostics(s, n);
    }
}

fn relay_isolation(n: &LabNode, ip: std::net::Ipv4Addr) {
    super::cut_as(&n.dev, ip, "reference_relay");
    let mut command = std::process::Command::new("nft");
    command.args(["-j", "list", "table", "inet", "reference_relay"]);
    command.stdout(std::process::Stdio::piped());
    let output = n
        .dev
        .spawn_command_sync(command)
        .unwrap()
        .wait_with_output()
        .unwrap();
    assert!(
        output.status.success(),
        "read the installed isolation table"
    );
    let table: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    eprintln!("[netlab-isolation] {} {table}", n.name);
}

async fn requests(
    s: &str,
    from: &LabNode,
    to: &LabNode,
    count: usize,
) -> (Vec<Duration>, Vec<String>) {
    let mut samples = Vec::new();
    let mut errors = Vec::new();
    for _ in 0..count {
        let name = to.name.clone();
        let (took, error) = crate::attempts::measure(
            s,
            &from.id,
            &to.name,
            Duration::from_secs(30),
            from.with(move |node| async move { node.ls_of(&name, "/tmp").await.map(|_| ()) }),
        )
        .await;
        samples.push(took);
        if let Some(error) = error {
            errors.push(error);
            break;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    (samples, errors)
}

fn distribution(samples: &[Duration]) -> serde_json::Value {
    let mut values: Vec<_> = samples.iter().map(|d| d.as_micros()).collect();
    values.sort();
    if values.is_empty() {
        return serde_json::json!({ "n": 0 });
    }
    let rank = |percent: usize| (values.len() * percent).div_ceil(100) - 1;
    serde_json::json!({
        "n": values.len(), "unit": "us", "p50": values[rank(50)],
        "p95": values[rank(95)], "max": values.last(),
    })
}

async fn reference(s: &str, want: Option<Hop>) {
    let (_lab, relay, _rx, x, y, m) = super::f4_topology(s, want == Some(Hop::Relay)).await;
    crate::attempts::phase(s, "reference_setup_started", super::now_ms());
    let began = tokio::time::Instant::now();
    let prepared = tokio::time::timeout(Duration::from_secs(120), async {
        for (from, to) in [(&y, &x), (&x, &y)] {
            let (_, errors) = requests(&format!("{s}-bootstrap"), from, to, 1).await;
            if !errors.is_empty() {
                return Err(format!(
                    "bootstrap {} -> {}: {errors:?}",
                    from.name, to.name
                ));
            }
        }
        if want != Some(Hop::Relay) {
            // A roster is not an established hop. Wait for its actual
            // opening before removing the road used to bootstrap it.
            loop {
                let offered = [(&x, &y), (&y, &x)].into_iter().any(|(from, to)| {
                    from.journal(Some(to)).iter().any(|event| {
                        event["e"] == "hop" && event["via"].as_str() == Some(m.id.as_str())
                    })
                });
                if offered {
                    crate::attempts::phase(s, "reference_candidate_offered", super::now_ms());
                    break;
                }
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
        }
        if want == Some(Hop::Forwarded) {
            let ip = relay.dev.ip().expect("the sole relay is IPv4");
            relay_isolation(&x, ip);
            relay_isolation(&y, ip);
            crate::attempts::phase(s, "reference_other_road_blocked", super::now_ms());
        }
        tokio::time::sleep_until(began + Duration::from_secs(60)).await;
        for (from, to) in [(&y, &x), (&x, &y)] {
            let (_, errors) = requests(&format!("{s}-setup"), from, to, 1).await;
            if !errors.is_empty() {
                return Err(format!("warmup {} -> {}: {errors:?}", from.name, to.name));
            }
        }
        let mut held: Option<(std::time::Instant, (Hop, Hop))> = None;
        loop {
            let a = x.link(&y).await;
            let b = y.link(&x).await;
            let roads = a
                .zip(b)
                .filter(|(a, b)| {
                    [a.hop, b.hop].into_iter().all(|road| {
                        matches!(road, Hop::Direct | Hop::Relay | Hop::Forwarded)
                            && want.is_none_or(|target| road == target)
                    })
                })
                .map(|(a, b)| (a.hop, b.hop));
            let now = std::time::Instant::now();
            match roads {
                Some(roads) => {
                    if held.as_ref().is_none_or(|(_, previous)| *previous != roads) {
                        held = Some((now, roads));
                        crate::attempts::phase(s, "reference_hold_started", super::now_ms());
                    }
                    if held.as_ref().is_some_and(|(since, _)| {
                        now.duration_since(*since) >= Duration::from_secs(10)
                    }) {
                        break;
                    }
                }
                None => held = None,
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        Ok(())
    })
    .await;
    if !matches!(prepared, Ok(Ok(()))) {
        diagnostics(s, &x, &y, &m);
        result(
            s,
            false,
            format!("reference preparation failed: {prepared:?}"),
        );
        panic!("{s}: reference preparation failed");
    }
    crate::attempts::phase(s, "reference_measurement_started", super::now_ms());
    let before = (x.link(&y).await, y.link(&x).await);
    let (yx, yx_errors) = requests(&format!("{s}-yx"), &y, &x, 30).await;
    let (xy, xy_errors) = requests(&format!("{s}-xy"), &x, &y, 30).await;
    let after = (x.link(&y).await, y.link(&x).await);
    crate::attempts::phase(s, "reference_measurement_finished", super::now_ms());
    let ends_on_target = after
        .0
        .as_ref()
        .zip(after.1.as_ref())
        .is_some_and(|(a, b)| {
            [a.hop, b.hop].into_iter().all(|road| {
                matches!(road, Hop::Direct | Hop::Relay | Hop::Forwarded)
                    && want.is_none_or(|target| road == target)
            })
        });
    let ok = yx.len() == 30
        && xy.len() == 30
        && yx_errors.is_empty()
        && xy_errors.is_empty()
        && ends_on_target;
    diagnostics(s, &x, &y, &m);
    result(
        s,
        ok,
        format!(
            "hop_reference_v2 want {want:?}; yx {} errors {yx_errors:?}; xy {} errors {xy_errors:?}; before {before:?}; after {after:?}",
            distribution(&yx),
            distribution(&xy),
        ),
    );
    assert!(ok, "complete bidirectional reference on the eligible road");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
#[ignore = "isolated steady-state reference, not the f4 fault test"]
async fn fr_automatic_uses_the_same_geometry_and_workload() {
    reference("fr-auto", None).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
#[ignore = "isolated steady-state reference, not the f4 fault test"]
async fn fr_carried_has_no_relay_data_path() {
    reference("fr-carried", Some(Hop::Forwarded)).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
#[ignore = "isolated steady-state reference, not the f4 fault test"]
async fn fr_relay_has_no_peer_hop_candidate() {
    reference("fr-relay", Some(Hop::Relay)).await;
}
