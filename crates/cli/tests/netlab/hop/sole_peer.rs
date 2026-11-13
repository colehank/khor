//! No caller-to-target warmup: the only neighbor has a real Relay second leg.
//! Background preparation is allowed and reported; a ready background hop is
//! useful behavior, not evidence of a cold foreground transport handshake.

use super::*;

fn firewall(node: &crate::lab::LabNode) -> serde_json::Value {
    let mut command = std::process::Command::new("nft");
    command.args(["-j", "list", "table", "inet", "sole_peer"]);
    command.stdout(std::process::Stdio::piped());
    let output = node
        .dev
        .spawn_command_sync(command)
        .unwrap()
        .wait_with_output()
        .unwrap();
    assert!(output.status.success(), "read the installed isolation");
    serde_json::from_slice(&output.stdout).unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
#[ignore = "isolated sole-peer physical acceptance"]
async fn sp1_first_business_uses_a_sole_neighbor_with_a_real_relay_second_leg() {
    let s = "sp1";
    let lab = Lab::new().await.unwrap();
    let r = relay(&lab, s, "relay", None).await;
    let lan = router(&lab, "lan", Net::Strict).await;
    let remote = router(&lab, "remote", Net::Strict).await;
    let dx = lab.add_device("x").uplink(lan.id()).build().await.unwrap();
    let dm = lab.add_device("m").uplink(lan.id()).build().await.unwrap();
    let dy = lab
        .add_device("y")
        .uplink(remote.id())
        .build()
        .await
        .unwrap();
    for dev in [&dx, &dm, &dy] {
        slow(dev, LEG).await;
    }
    let ip = r.dev.ip().unwrap();
    nft(
        &dx,
        &format!(
            "table inet sole_peer {{\n chain blocked_in {{ type filter hook input priority -10; policy accept; ip saddr {ip} counter drop; }}\n chain blocked_out {{ type filter hook output priority -10; policy accept; ip daddr {ip} counter drop; }}\n}}\n"
        ),
    );
    // No UDP reaches Y, including hole punching. M reaches it over the
    // actual HTTP Relay; X cannot reach that relay in either direction.
    nft(
        &dy,
        "table inet sole_peer {\n chain blocked_in { type filter hook input priority -10; policy accept; meta l4proto udp counter drop; }\n chain blocked_out { type filter hook output priority -10; policy accept; meta l4proto udp counter drop; }\n}\n",
    );
    let x = node(s, dx, "x", &[&r], &[]).await;
    let m = node(s, dm, "m", &[&r], &[]).await;
    let y = node(s, dy, "y", &[&r], &[]).await;
    // Pair through M. Do not call X->Y, wait for a hop, or wait for the
    // long no-relay announcement before the first business request.
    mesh(&[&m, &x, &y]).await;
    let no_relay = x
        .with(|n| async move {
            n.devices()
                .unwrap()
                .into_iter()
                .find(|d| d.id == n.device_str())
                .map(|d| d.no_relay)
        })
        .await;
    let before = x.link(&y).await;
    let directory = y.home.0.join("sole-peer-content");
    std::fs::create_dir(&directory).unwrap();
    let marker = format!("from-{}", y.id);
    std::fs::write(directory.join(&marker), b"sole-peer physical fixture\n").unwrap();
    crate::attempts::phase(s, "first_business_started", now_ms());
    let (name, path) = (y.name.clone(), directory.to_str().unwrap().to_owned());
    let (took, error) = crate::attempts::measure(
        s,
        &x.id,
        &y.name,
        Duration::from_secs(5),
        x.with(move |n| async move {
            let (_, entries, truncated) = n.ls_of(&name, &path).await?;
            if truncated || !entries.iter().any(|entry| entry.name == marker) {
                return Err("the target's actual directory marker is missing".into());
            }
            Ok(())
        }),
    )
    .await;
    let (xy, xm, my) = (x.link(&y).await, x.link(&m).await, m.link(&y).await);
    let carried = xy
        .as_ref()
        .is_some_and(|l| l.hop == Hop::Forwarded && l.via.as_deref() == Some(m.id.as_str()));
    let legs = xm.as_ref().is_some_and(|l| l.hop == Hop::Direct)
        && my.as_ref().is_some_and(|l| l.hop == Hop::Relay);
    let ok = error.is_none() && no_relay == Some(false) && carried && legs;
    eprintln!(
        "[netlab-sole-peer] {}",
        serde_json::json!({
            "revision": "sole_peer_physical_v1", "case": s,
            "before": format!("{before:?}"), "after": format!("{xy:?}"),
            "first_leg": format!("{xm:?}"), "second_leg": format!("{my:?}"),
            "no_relay_at_request": no_relay, "elapsed_us": took.as_micros(),
            "error": error, "caller_relay_firewall": firewall(&x),
            "target_udp_firewall": firewall(&y), "passed": ok,
        })
    );
    hop_diagnostics(&x, &y, &m);
    result(
        s,
        ok,
        format!(
            "first real directory {took:?}; through sole M: {carried}; Direct/Relay legs: {legs}; error {error:?}"
        ),
    );
    assert!(
        ok,
        "automatic sole-peer business without no-relay grace or caller-to-target warmup"
    );
}
