//! Real three-resident paths; only cached roster input is altered by the fixture.
use super::*;

use crate::lab::{LabNode, Relay};
use khor_core::{Road, RoadKind};
use khor_sync::devices::{DeviceDoc, devices_dir};
use khor_sync::store::load_pile;

struct Fixture {
    x: LabNode,
    m: LabNode,
    y: LabNode,
    _relay: Relay,
    _lan: Router,
    _remote: Router,
    _lab: Lab,
}

async fn fixture(s: &str) -> Fixture {
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
    nft(
        &dy,
        "table inet sole_peer {\n chain blocked_in { type filter hook input priority -10; policy accept; meta l4proto udp counter drop; }\n chain blocked_out { type filter hook output priority -10; policy accept; meta l4proto udp counter drop; }\n}\n",
    );
    let x = node(s, dx, "x", &[&r], &[]).await;
    let m = node(s, dm, "m", &[&r], &[]).await;
    let y = node(s, dy, "y", &[&r], &[]).await;
    mesh(&[&m, &x, &y]).await;
    Fixture {
        x,
        m,
        y,
        _relay: r,
        _lan: lan,
        _remote: remote,
        _lab: lab,
    }
}

fn write_hint(node: &LabNode, from: &str, to: &str, value: &str) {
    // The existing store gives this scratch writer its own lease. No resident
    // private state, production test hook, or adapted second-leg transport.
    let loaded = load_pile::<DeviceDoc>(&devices_dir(&node.home.0), 0x6e65746c61626331).unwrap();
    assert!(loaded.broken.is_empty());
    loaded.doc.set_road(from, to, value).unwrap();
    let mut store = loaded.store;
    store.flush(&loaded.doc).unwrap();
}

async fn hint(node: &LabNode, from: &str, to: &str) -> Option<String> {
    let (from, to) = (from.to_owned(), to.to_owned());
    node.with(move |n| async move {
        n.devices()
            .unwrap()
            .into_iter()
            .find(|d| d.id == from)?
            .roads
            .into_iter()
            .find(|(id, _)| *id == to)
            .map(|(_, value)| value)
    })
    .await
}

fn content(y: &LabNode, s: &str) -> (String, String) {
    let directory = y.home.0.join(format!("sole-cache-{s}"));
    std::fs::create_dir(&directory).unwrap();
    let marker = format!("from-{}-{s}", y.id);
    std::fs::write(directory.join(&marker), b"actual target directory\n").unwrap();
    (directory.to_str().unwrap().to_owned(), marker)
}

async fn read(
    x: &LabNode,
    y: &LabNode,
    path: &str,
    marker: &str,
    within: Duration,
    s: &str,
) -> (Duration, Option<String>) {
    let (name, path, marker) = (y.name.clone(), path.to_owned(), marker.to_owned());
    crate::attempts::measure(
        s,
        &x.id,
        &y.name,
        within,
        x.with(move |n| async move {
            let (_, entries, truncated) = n.ls_of(&name, &path).await?;
            if truncated || !entries.iter().any(|entry| entry.name == marker) {
                return Err("actual target marker missing".into());
            }
            Ok(())
        }),
    )
    .await
}

fn cold(link: &Option<khor_core::Link>) -> bool {
    link.as_ref()
        .is_none_or(|l| l.hop == Hop::Idle || l.down_ms.is_some())
}

async fn relayed(f: &Fixture) -> bool {
    let xy = f.x.link(&f.y).await;
    let xm = f.x.link(&f.m).await;
    let my = f.m.link(&f.y).await;
    xy.is_some_and(|l| {
        l.hop == Hop::Forwarded && l.via.as_deref() == Some(f.m.id.as_str()) && l.down_ms.is_none()
    }) && xm.is_some_and(|l| l.hop == Hop::Direct && l.down_ms.is_none())
        && my.is_some_and(|l| l.hop == Hop::Relay && l.down_ms.is_none())
}

fn firewall(n: &LabNode) -> serde_json::Value {
    let mut command = std::process::Command::new("nft");
    command.args(["-j", "list", "table", "inet", "sole_peer"]);
    command.stdout(std::process::Stdio::piped());
    let out = n
        .dev
        .spawn_command_sync(command)
        .unwrap()
        .wait_with_output()
        .unwrap();
    assert!(out.status.success());
    serde_json::from_slice(&out.stdout).unwrap()
}

async fn observe(s: &str, variant: u8) {
    let f = fixture(s).await;
    let (path, marker) = content(&f.y, s);
    let from = if variant == 4 { &f.x.id } else { &f.m.id };
    let value = if variant == 2 {
        "null".to_owned()
    } else {
        serde_json::to_string(&Road::own(&f.y.id, RoadKind::Direct, None, Some(1))).unwrap()
    };
    write_hint(&f.x, from, &f.y.id, &value);
    let hint_before = hint(&f.x, from, &f.y.id).await;
    let before = f.x.link(&f.y).await;
    let no_relay =
        f.x.with(|n| async move {
            n.devices()
                .unwrap()
                .into_iter()
                .find(|d| d.id == n.device_str())
                .map(|d| d.no_relay)
        })
        .await;
    let (took, error) = read(&f.x, &f.y, &path, &marker, Duration::from_secs(5), s).await;
    let hint_after = hint(&f.x, from, &f.y.id).await;
    let hint_held = hint_before.as_deref() == Some(value.as_str()) && hint_after == hint_before;
    let actual_two_legs = relayed(&f).await;
    let ok =
        cold(&before) && no_relay == Some(false) && hint_held && error.is_none() && actual_two_legs;
    eprintln!(
        "[netlab-sole-cache] {}",
        serde_json::json!({
            "revision":"sole_peer_cache_physical_v1", "case":s, "variant":variant,
            "before":format!("{before:?}"), "cold":cold(&before), "no_relay_at_request":no_relay,
            "hint_before":hint_before, "hint_after":hint_after, "hint_held":hint_held,
            "elapsed_us":took.as_micros(), "error":error, "actual_two_legs":actual_two_legs,
            "caller_relay_firewall":firewall(&f.x), "target_udp_firewall":firewall(&f.y), "passed":ok,
        })
    );
    hop_diagnostics(&f.x, &f.y, &f.m);
    result(
        s,
        ok,
        format!(
            "cached variant {variant}, first real content {took:?}, held={hint_held}, legs={actual_two_legs}, error={error:?}"
        ),
    );
    assert!(
        ok,
        "unchanged unusable/stale cache must not hide the only real middle"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
#[ignore = "isolated real sole-middle unusable hint"]
async fn sp2_unusable_second_leg_hint_is_verified_by_the_real_middle() {
    observe("sp2", 2).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
#[ignore = "isolated real sole-middle stale Direct hint"]
async fn sp3_stale_direct_hint_uses_the_real_relay_second_leg() {
    observe("sp3", 3).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
#[ignore = "isolated real sole-middle historical target Direct"]
async fn sp4_historical_direct_target_does_not_hide_the_only_middle() {
    observe("sp4", 4).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
#[ignore = "isolated actual middle pause, failed attempt, and explicit recovery"]
async fn sp5_recent_failed_middle_recovers_after_its_target_is_resumed() {
    let s = "sp5";
    let f = fixture(s).await;
    let (path, marker) = content(&f.y, s);
    assert!(
        read(
            &f.m,
            &f.y,
            &path,
            &marker,
            Duration::from_secs(5),
            "sp5-middle-control"
        )
        .await
        .1
        .is_none()
    );
    f.m.with(|n| async move { n.set_machine_paused("y", true).unwrap() })
        .await;
    let middle_down =
        f.m.until(&f.y, Duration::from_secs(10), |l| l.down_ms.is_some())
            .await
            .is_some();
    let caller_down =
        f.x.until(&f.y, Duration::from_secs(10), |l| {
            l.hop == Hop::Idle || l.down_ms.is_some()
        })
        .await
        .is_some();
    let first_leg_up =
        f.x.link(&f.m)
            .await
            .is_some_and(|l| l.hop == Hop::Direct && l.down_ms.is_none());
    let value =
        serde_json::to_string(&Road::own(&f.y.id, RoadKind::Relay, None, Some(40))).unwrap();
    write_hint(&f.x, &f.m.id, &f.y.id, &value);
    let hint_before = hint(&f.x, &f.m.id, &f.y.id).await;
    let x_pid = f.x.pid();
    let (failed_took, failed_error) = read(
        &f.x,
        &f.y,
        &path,
        &marker,
        Duration::from_secs(5),
        "sp5-paused-target",
    )
    .await;
    let still_down = cold(&f.x.link(&f.y).await);
    let hint_failed = hint(&f.x, &f.m.id, &f.y.id).await;
    let restore = std::time::Instant::now();
    let remaining = || Duration::from_secs(5).saturating_sub(restore.elapsed());
    let mut middle_resumed = false;
    let restored = tokio::time::timeout(Duration::from_secs(5), async {
        f.m.with(|n| async move { n.set_machine_paused("y", false).unwrap() })
            .await;
        middle_resumed =
            f.m.until(&f.y, remaining(), |l| {
                l.hop == Hop::Relay && l.down_ms.is_none()
            })
            .await
            .is_some();
        read(&f.x, &f.y, &path, &marker, remaining(), "sp5-after-resume").await
    })
    .await;
    let (took, error) = restored.unwrap_or_else(|_| {
        (
            restore.elapsed(),
            Some("one total five-second resume/read deadline".to_owned()),
        )
    });
    let restored_us = restore.elapsed().as_micros();
    let hint_after = hint(&f.x, &f.m.id, &f.y.id).await;
    let hint_held = hint_before.as_deref() == Some(value.as_str())
        && hint_failed == hint_before
        && hint_after == hint_before;
    let actual_two_legs = relayed(&f).await;
    let unchanged_pid = f.x.pid() == x_pid;
    let ok = middle_down
        && caller_down
        && first_leg_up
        && failed_error.is_some()
        && still_down
        && middle_resumed
        && error.is_none()
        && restored_us < 5_000_000
        && hint_held
        && actual_two_legs
        && unchanged_pid;
    eprintln!(
        "[netlab-sole-cache] {}",
        serde_json::json!({
            "revision":"sole_peer_cache_physical_v1", "case":s, "middle_down":middle_down,
            "caller_down":caller_down, "first_leg_up":first_leg_up, "failed_elapsed_us":failed_took.as_micros(),
            "failed_error":failed_error, "target_stayed_unavailable":still_down, "middle_resumed":middle_resumed,
            "restored_total_us":restored_us, "restored_read_us":took.as_micros(), "error":error,
            "hint_before":hint_before, "hint_failed":hint_failed, "hint_after":hint_after,
            "hint_held":hint_held, "actual_two_legs":actual_two_legs, "unchanged_caller_pid":unchanged_pid,
            "caller_relay_firewall":firewall(&f.x), "target_udp_firewall":firewall(&f.y), "passed":ok,
        })
    );
    hop_diagnostics(&f.x, &f.y, &f.m);
    result(
        s,
        ok,
        format!(
            "real paused failure {failed_error:?}; after resume {restored_us}us error={error:?}; hint held={hint_held}, same pid={unchanged_pid}"
        ),
    );
    assert!(
        ok,
        "a genuine recent failed attempt must not require a caller restart after the only middle recovers"
    );
}

/// Unlike sp5, policy rejection is observed by a real read; an Idle row
/// need not carry down_ms. Keep X-to-M live while the target is refused.
#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
#[ignore = "isolated actual pause refusal and live-middle recovery"]
async fn sp6_actual_pause_refusal_recovers_through_the_same_live_middle() {
    let s = "sp6";
    let f = fixture(s).await;
    let (path, marker) = content(&f.y, s);
    let (middle_path, middle_marker) = content(&f.m, "sp6-first-leg");
    let middle_control = read(
        &f.m,
        &f.y,
        &path,
        &marker,
        Duration::from_secs(5),
        "sp6-middle-control",
    )
    .await;
    let first_control = read(
        &f.x,
        &f.m,
        &middle_path,
        &middle_marker,
        Duration::from_secs(5),
        "sp6-first-control",
    )
    .await;
    assert!(
        middle_control.1.is_none() && first_control.1.is_none(),
        "both real legs before policy interruption"
    );
    let first_leg_before = f.x.link(&f.m).await;
    assert!(
        first_leg_before
            .as_ref()
            .is_some_and(|l| l.hop == Hop::Direct && l.down_ms.is_none())
    );
    let yid = f.y.id.clone();
    let paused =
        f.m.with(move |n| async move {
            n.set_machine_paused("y", true).unwrap();
            n.machine_paused(&yid)
        })
        .await;
    let refused_middle = read(
        &f.m,
        &f.y,
        &path,
        &marker,
        Duration::from_secs(5),
        "sp6-paused-middle-control",
    )
    .await;
    assert!(
        paused && refused_middle.1.is_some(),
        "the actual M-to-Y policy refuses target content"
    );
    let value =
        serde_json::to_string(&Road::own(&f.y.id, RoadKind::Relay, None, Some(40))).unwrap();
    write_hint(&f.x, &f.m.id, &f.y.id, &value);
    let hint_before = hint(&f.x, &f.m.id, &f.y.id).await;
    let caller_before = f.x.link(&f.y).await;
    let no_relay =
        f.x.with(|n| async move {
            n.devices()
                .unwrap()
                .into_iter()
                .find(|d| d.id == n.device_str())
                .map(|d| d.no_relay)
        })
        .await;
    let x_pid = f.x.pid();
    let keepalive = async {
        let began = std::time::Instant::now();
        let mut okay = true;
        for i in 0..5 {
            tokio::time::sleep_until((began + Duration::from_secs(i)).into()).await;
            let control = read(
                &f.x,
                &f.m,
                &middle_path,
                &middle_marker,
                Duration::from_secs(1),
                "sp6-first-leg-heartbeat",
            )
            .await;
            okay &= control.1.is_none();
        }
        okay
    };
    let (failed, first_leg_reads_ok) = tokio::join!(
        read(
            &f.x,
            &f.y,
            &path,
            &marker,
            Duration::from_secs(5),
            "sp6-paused-target"
        ),
        keepalive,
    );
    let stayed_unavailable = cold(&f.x.link(&f.y).await);
    let first_leg_after_failure = f.x.link(&f.m).await;
    let first_leg_up = first_leg_after_failure
        .as_ref()
        .is_some_and(|l| l.hop == Hop::Direct && l.down_ms.is_none());
    let hint_failed = hint(&f.x, &f.m.id, &f.y.id).await;
    let restore = std::time::Instant::now();
    let remaining = || Duration::from_secs(5).saturating_sub(restore.elapsed());
    let mut resumed_policy = false;
    let mut resumed_middle_content = false;
    let restored = tokio::time::timeout(Duration::from_secs(5), async {
        let yid = f.y.id.clone();
        resumed_policy =
            f.m.with(move |n| async move {
                n.set_machine_paused("y", false).unwrap();
                !n.machine_paused(&yid)
            })
            .await;
        resumed_middle_content = read(
            &f.m,
            &f.y,
            &path,
            &marker,
            remaining(),
            "sp6-resumed-middle-control",
        )
        .await
        .1
        .is_none();
        read(&f.x, &f.y, &path, &marker, remaining(), "sp6-after-resume").await
    })
    .await;
    let (took, error) = restored.unwrap_or_else(|_| {
        (
            restore.elapsed(),
            Some("one total five-second resume/read deadline".to_owned()),
        )
    });
    let restored_total_us = restore.elapsed().as_micros();
    let hint_after = hint(&f.x, &f.m.id, &f.y.id).await;
    let hint_held = hint_before.as_deref() == Some(value.as_str())
        && hint_failed == hint_before
        && hint_after == hint_before;
    let actual_two_legs = relayed(&f).await;
    let unchanged_pid = f.x.pid() == x_pid;
    let preconditions = cold(&caller_before)
        && no_relay == Some(false)
        && paused
        && refused_middle.1.is_some()
        && failed.1.is_some()
        && stayed_unavailable
        && first_leg_reads_ok
        && first_leg_up
        && hint_held;
    let okay = preconditions
        && resumed_policy
        && resumed_middle_content
        && error.is_none()
        && restored_total_us < 5_000_000
        && actual_two_legs
        && unchanged_pid;
    eprintln!(
        "[netlab-sole-cache] {}",
        serde_json::json!({
            "revision":"sole_peer_cache_resume_v2", "case":s, "preconditions":preconditions,
            "caller_before":format!("{caller_before:?}"), "no_relay_at_request":no_relay,
            "middle_policy_paused":paused, "middle_refusal":refused_middle.1,
            "failed_elapsed_us":failed.0.as_micros(), "failed_error":failed.1,
            "target_stayed_unavailable":stayed_unavailable, "first_leg_reads_ok":first_leg_reads_ok,
            "first_leg_up":first_leg_up, "first_leg_after_failure":format!("{first_leg_after_failure:?}"),
            "resumed_policy":resumed_policy, "resumed_middle_content":resumed_middle_content,
            "restored_total_us":restored_total_us, "restored_read_us":took.as_micros(), "error":error,
            "hint_before":hint_before, "hint_failed":hint_failed, "hint_after":hint_after, "hint_held":hint_held,
            "actual_two_legs":actual_two_legs, "unchanged_caller_pid":unchanged_pid,
            "caller_relay_firewall":firewall(&f.x), "target_udp_firewall":firewall(&f.y), "passed":okay,
        })
    );
    hop_diagnostics(&f.x, &f.y, &f.m);
    assert!(
        okay,
        "actual policy refusal and same live middle must recover without caller restart"
    );
}

/// Unlike sp5, policy rejection is observed by a real read; an Idle row
/// need not carry down_ms. Keep X-to-M live while the target is refused.
#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
#[ignore = "isolated actual pause refusal and live-middle recovery"]
async fn sp7_applied_pause_refusal_recovers_through_the_same_live_middle() {
    let s = "sp7";
    let f = fixture(s).await;
    let (path, marker) = content(&f.y, s);
    let (middle_path, middle_marker) = content(&f.m, "sp7-first-leg");
    let middle_control = read(
        &f.m,
        &f.y,
        &path,
        &marker,
        Duration::from_secs(5),
        "sp7-middle-control",
    )
    .await;
    let first_control = read(
        &f.x,
        &f.m,
        &middle_path,
        &middle_marker,
        Duration::from_secs(5),
        "sp7-first-control",
    )
    .await;
    assert!(
        middle_control.1.is_none() && first_control.1.is_none(),
        "both real legs before policy interruption"
    );
    let first_leg_before = f.x.link(&f.m).await;
    assert!(
        first_leg_before
            .as_ref()
            .is_some_and(|l| l.hop == Hop::Direct && l.down_ms.is_none())
    );
    let yid = f.y.id.clone();
    let paused =
        f.m.with(move |n| async move {
            n.set_machine_paused("y", true).unwrap();
            n.machine_paused(&yid)
        })
        .await;
    let expected_refusal = khor_catalog::msg::peer_paused_refusal().to_owned();
    assert!(!expected_refusal.is_empty());
    let policy_began = std::time::Instant::now();
    let mut policy_checks = Vec::new();
    // Observe the resident's actual policy after its normal external-file
    // poll. Unrelated timeouts do not establish that pause was applied.
    let refused_middle = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let reply = read(&f.m, &f.y, &path, &marker, Duration::from_secs(1), "sp7-pause-application-check").await;
            let matches_policy = reply.1.as_deref().is_some_and(|error| error.contains(expected_refusal.as_str()));
            let check = serde_json::json!({"phase":"pause_applied", "elapsed_us":policy_began.elapsed().as_micros(),
                "read_us":reply.0.as_micros(), "error":reply.1, "matches_policy":matches_policy});
            eprintln!("[netlab-pause-policy] {check}");
            policy_checks.push(check);
            if matches_policy { return reply; }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }).await.expect("actual resident pause refusal within the setup bound");
    assert!(
        paused && refused_middle.1.is_some(),
        "the actual M-to-Y policy refuses target content"
    );
    let value =
        serde_json::to_string(&Road::own(&f.y.id, RoadKind::Relay, None, Some(40))).unwrap();
    write_hint(&f.x, &f.m.id, &f.y.id, &value);
    let hint_before = hint(&f.x, &f.m.id, &f.y.id).await;
    let caller_before = f.x.link(&f.y).await;
    let no_relay =
        f.x.with(|n| async move {
            n.devices()
                .unwrap()
                .into_iter()
                .find(|d| d.id == n.device_str())
                .map(|d| d.no_relay)
        })
        .await;
    let x_pid = f.x.pid();
    let keepalive = async {
        let began = std::time::Instant::now();
        let mut okay = true;
        for i in 0..5 {
            tokio::time::sleep_until((began + Duration::from_secs(i)).into()).await;
            let control = read(
                &f.x,
                &f.m,
                &middle_path,
                &middle_marker,
                Duration::from_secs(1),
                "sp7-first-leg-heartbeat",
            )
            .await;
            okay &= control.1.is_none();
        }
        okay
    };
    let (failed, first_leg_reads_ok) = tokio::join!(
        read(
            &f.x,
            &f.y,
            &path,
            &marker,
            Duration::from_secs(5),
            "sp7-paused-target"
        ),
        keepalive,
    );
    let stayed_unavailable = cold(&f.x.link(&f.y).await);
    let first_leg_after_failure = f.x.link(&f.m).await;
    let first_leg_up = first_leg_after_failure
        .as_ref()
        .is_some_and(|l| l.hop == Hop::Direct && l.down_ms.is_none());
    let hint_failed = hint(&f.x, &f.m.id, &f.y.id).await;
    let restore = std::time::Instant::now();
    let remaining = || Duration::from_secs(5).saturating_sub(restore.elapsed());
    let mut resume_checks = Vec::new();
    let mut resumed_policy = false;
    let mut resumed_middle_content = false;
    let restored = tokio::time::timeout(Duration::from_secs(5), async {
        let yid = f.y.id.clone();
        resumed_policy =
            f.m.with(move |n| async move {
                n.set_machine_paused("y", false).unwrap();
                !n.machine_paused(&yid)
            })
            .await;
        loop {
            let reply = read(&f.m, &f.y, &path, &marker, remaining(), "sp7-resume-application-check").await;
            let okay = reply.1.is_none();
            let check = serde_json::json!({"phase":"resume_applied", "elapsed_us":restore.elapsed().as_micros(),
                "read_us":reply.0.as_micros(), "error":reply.1, "content_returned":okay});
            eprintln!("[netlab-pause-policy] {check}");
            resume_checks.push(check);
            if okay { resumed_middle_content = true; break; }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        read(&f.x, &f.y, &path, &marker, remaining(), "sp7-after-resume").await
    })
    .await;
    let (took, error) = restored.unwrap_or_else(|_| {
        (
            restore.elapsed(),
            Some("one total five-second resume/read deadline".to_owned()),
        )
    });
    let restored_total_us = restore.elapsed().as_micros();
    let hint_after = hint(&f.x, &f.m.id, &f.y.id).await;
    let hint_held = hint_before.as_deref() == Some(value.as_str())
        && hint_failed == hint_before
        && hint_after == hint_before;
    let actual_two_legs = relayed(&f).await;
    let unchanged_pid = f.x.pid() == x_pid;
    let preconditions = cold(&caller_before)
        && no_relay == Some(false)
        && paused
        && refused_middle.1.is_some()
        && failed.1.is_some()
        && stayed_unavailable
        && first_leg_reads_ok
        && first_leg_up
        && hint_held;
    let okay = preconditions
        && resumed_policy
        && resumed_middle_content
        && error.is_none()
        && restored_total_us < 5_000_000
        && actual_two_legs
        && unchanged_pid;
    eprintln!(
        "[netlab-sole-cache] {}",
        serde_json::json!({
            "revision":"sole_peer_cache_resume_v3", "case":s, "preconditions":preconditions,
            "caller_before":format!("{caller_before:?}"), "no_relay_at_request":no_relay,
            "middle_policy_paused":paused, "middle_refusal":refused_middle.1,
            "failed_elapsed_us":failed.0.as_micros(), "failed_error":failed.1,
            "target_stayed_unavailable":stayed_unavailable, "first_leg_reads_ok":first_leg_reads_ok,
            "first_leg_up":first_leg_up, "first_leg_after_failure":format!("{first_leg_after_failure:?}"),
            "resumed_policy":resumed_policy, "resumed_middle_content":resumed_middle_content,
            "expected_pause_refusal":expected_refusal, "pause_checks":policy_checks, "resume_checks":resume_checks,
            "restored_total_us":restored_total_us, "restored_read_us":took.as_micros(), "error":error,
            "hint_before":hint_before, "hint_failed":hint_failed, "hint_after":hint_after, "hint_held":hint_held,
            "actual_two_legs":actual_two_legs, "unchanged_caller_pid":unchanged_pid,
            "caller_relay_firewall":firewall(&f.x), "target_udp_firewall":firewall(&f.y), "passed":okay,
        })
    );
    hop_diagnostics(&f.x, &f.y, &f.m);
    assert!(
        okay,
        "actual policy refusal and same live middle must recover without caller restart"
    );
}
