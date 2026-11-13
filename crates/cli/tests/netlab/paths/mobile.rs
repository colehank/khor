//! Real Node directory reads across an isolated IPv6 address replacement.
use super::mobility::{addresses, command, entry};
use super::*;
use std::io::Write;
use std::net::Ipv6Addr;

const CONTENT: &[u8] = b"owned network fixture\n";

#[derive(Clone, Copy)]
enum FirstBusiness {
    Both,
    Remote,
}

impl FirstBusiness {
    fn revision(self) -> &'static str {
        match self {
            Self::Both => "ipv6_node_overlap_v1",
            Self::Remote => "ipv6_node_overlap_remote_first_v1",
        }
    }
}

fn ipv6_only(n: &LabNode) {
    let rules = "table inet mobile_ipv4 {\n chain cut_in { type filter hook input priority -10; policy accept;\n  iifname \"eth0\" meta nfproto ipv4 drop\n }\n chain cut_out { type filter hook output priority -10; policy accept;\n  oifname \"eth0\" meta nfproto ipv4 drop\n }\n}\n";
    let mut cmd = Command::new("nft");
    cmd.args(["-f", "-"]).stdin(Stdio::piped());
    let mut child = n.dev.spawn_command_sync(cmd).unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(rules.as_bytes())
        .unwrap();
    assert!(
        child.wait().unwrap().success(),
        "install owned namespace isolation"
    );
    isolation(n);
}

fn isolation(n: &LabNode) {
    let output = command(
        &n.dev,
        &["nft", "-j", "list", "table", "inet", "mobile_ipv4"],
    );
    assert!(output.status.success());
    let table: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    eprintln!("[netlab-mobile-isolation] {} {table}", n.name);
}

fn state(n: &LabNode, phase: &str, revision: &str) -> (serde_json::Value, serde_json::Value) {
    let v4 = addresses(&n.dev, "-4");
    let v6 = addresses(&n.dev, "-6");
    eprintln!(
        "[netlab-mobile-addresses] {}",
        serde_json::json!({
            "revision": revision, "node": n.name, "phase": phase,
            "at_ms": now_ms(), "v4": v4, "v6": v6,
        }),
    );
    (v4, v6)
}

pub(crate) fn fixture(n: &LabNode) {
    let dir = n.home.0.join("mobile-fixture");
    std::fs::create_dir(&dir).unwrap();
    std::fs::write(dir.join(format!("peer-{}", n.id)), CONTENT).unwrap();
}

pub(crate) async fn reads(
    case: &str,
    from: &LabNode,
    to: &LabNode,
    count: usize,
) -> (Vec<Duration>, Vec<String>) {
    let mut durations = Vec::new();
    let mut errors = Vec::new();
    for _ in 0..count {
        let name = to.name.clone();
        let path = to
            .home
            .0
            .join("mobile-fixture")
            .to_string_lossy()
            .into_owned();
        let marker = format!("peer-{}", to.id);
        let (took, error) = crate::attempts::measure(
            case,
            &from.id,
            &to.name,
            Duration::from_secs(30),
            from.with(move |node| async move {
                let (_, entries, truncated) = node.ls_of(&name, &path).await?;
                if !truncated
                    && entries
                        .iter()
                        .any(|e| e.name == marker && !e.dir && e.size == CONTENT.len() as u64)
                {
                    Ok(())
                } else {
                    Err("the requested peer's fixture marker is missing".to_owned())
                }
            }),
        )
        .await;
        durations.push(took);
        if let Some(error) = error {
            errors.push(error);
            break;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    (durations, errors)
}

pub(crate) fn complete(result: &(Vec<Duration>, Vec<String>), count: usize) -> bool {
    result.0.len() == count && result.1.is_empty()
}

pub(crate) fn summary(result: &(Vec<Duration>, Vec<String>)) -> serde_json::Value {
    let mut values: Vec<_> = result.0.iter().map(|v| v.as_micros()).collect();
    values.sort();
    if values.is_empty() {
        return serde_json::json!({ "n": 0, "errors": result.1 });
    }
    let rank = |p: usize| (values.len() * p).div_ceil(100) - 1;
    serde_json::json!({
        "n": values.len(), "unit": "us", "p50": values[rank(50)],
        "p95": values[rank(95)], "max": values.last(), "errors": result.1,
    })
}

pub(crate) fn diagnostics(s: &str, n: &LabNode, peer: &LabNode) {
    for (scope, events) in [("peer", n.journal(Some(peer))), ("here", n.journal(None))] {
        for event in events
            .into_iter()
            .rev()
            .take(100)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
        {
            eprintln!("[netlab-mobile-journal] {} {scope} {event}", n.name);
        }
    }
    sample_diagnostics(s, n);
}

async fn session(
    s: &str,
    a: &LabNode,
    b: &LabNode,
    first: FirstBusiness,
) -> Result<serde_json::Value, String> {
    let revision = first.revision();
    tokio::time::timeout(Duration::from_secs(120), async {
        mesh(&[a, b]).await;
        ipv6_only(a);
        ipv6_only(b);
        for (from, to) in [(a, b), (b, a)] {
            let checked = reads(&format!("{s}-setup"), from, to, 1).await;
            if !complete(&checked, 1) {
                return Err(format!("IPv6-only setup failed: {}", summary(&checked)));
            }
        }
        let mut held = None;
        loop {
            let links = (a.link(b).await, b.link(a).await);
            let direct = links.0.is_some_and(|v| v.hop == Hop::Direct)
                && links.1.is_some_and(|v| v.hop == Hop::Direct);
            if direct {
                let since = held.get_or_insert_with(Instant::now);
                if since.elapsed() >= Duration::from_secs(10) {
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
    .map_err(|_| "IPv6-only preparation exceeded 120 seconds".to_owned())??;
    crate::attempts::phase(s, "ipv6_ready", now_ms());
    let before_ab = reads(&format!("{s}-before-ab"), a, b, 30).await;
    let before_ba = reads(&format!("{s}-before-ba"), b, a, 30).await;
    if !complete(&before_ab, 30) || !complete(&before_ba, 30) {
        return Err(format!(
            "baseline failed: ab {} ba {}",
            summary(&before_ab),
            summary(&before_ba)
        ));
    }
    let old = a.dev.ip6().ok_or("a needs IPv6")?;
    let (before_v4, before_v6) = state(a, "before", revision);
    let (interface, info) = entry(&before_v6, old).ok_or("old IPv6 is absent")?;
    let prefix = info["prefixlen"].as_u64().ok_or("prefix missing")?;
    if prefix > 120 {
        return Err("unsupported calibration prefix".to_owned());
    }
    let mut octets = old.octets();
    octets[15] ^= 64;
    let new = Ipv6Addr::from(octets);
    if entry(&before_v6, new).is_some() {
        return Err("replacement address already exists".to_owned());
    }
    let added = format!("{new}/{prefix}");
    crate::attempts::phase(s, "address_add_started", now_ms());
    let output = command(
        &a.dev,
        &["ip", "-6", "address", "add", &added, "dev", &interface],
    );
    if !output.status.success() {
        return Err("could not add owned IPv6".to_owned());
    }
    let due = Instant::now() + Duration::from_secs(10);
    loop {
        let (_, info) = entry(&addresses(&a.dev, "-6"), new).ok_or("new IPv6 is absent")?;
        if info["dadfailed"] == serde_json::Value::Bool(true) {
            return Err("new IPv6 DAD failed".to_owned());
        }
        if info["tentative"] != serde_json::Value::Bool(true) {
            break;
        }
        if Instant::now() >= due {
            return Err("new IPv6 did not become ready".to_owned());
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let (added_v4, added_v6) = state(a, "added", revision);
    if added_v4 != before_v4 || entry(&added_v6, old).is_none() || entry(&added_v6, new).is_none() {
        return Err("address overlap precondition failed".to_owned());
    }
    let removed = format!("{old}/{prefix}");
    crate::attempts::phase(s, "address_remove_started", now_ms());
    let output = command(
        &a.dev,
        &["ip", "-6", "address", "del", &removed, "dev", &interface],
    );
    if !output.status.success() {
        return Err("could not remove old owned IPv6".to_owned());
    }
    crate::attempts::phase(s, "address_removed", now_ms());
    let first_ab_case = format!("{s}-first-ab");
    let first_ba_case = format!("{s}-first-ba");
    let (first_ab, first_ba) = match first {
        FirstBusiness::Both => tokio::join!(
            reads(&first_ab_case, a, b, 1),
            reads(&first_ba_case, b, a, 1),
        ),
        FirstBusiness::Remote => {
            let ba = reads(&first_ba_case, b, a, 1).await;
            crate::attempts::phase(s, "first_remote_finished", now_ms());
            let ab = reads(&first_ab_case, a, b, 1).await;
            (ab, ba)
        }
    };
    let after_ab = reads(&format!("{s}-after-ab"), a, b, 30).await;
    let after_ba = reads(&format!("{s}-after-ba"), b, a, 30).await;
    crate::attempts::phase(s, "reads_finished", now_ms());
    let (after_v4, after_v6) = state(a, "removed", revision);
    isolation(a);
    isolation(b);
    let new_check = command(
        &b.dev,
        &["ping", "-6", "-n", "-c", "1", "-W", "2", &new.to_string()],
    );
    let old_check = command(
        &b.dev,
        &["ping", "-6", "-n", "-c", "1", "-W", "2", &old.to_string()],
    );
    let report = serde_json::json!({
        "revision": revision, "case": s, "nodes": [a.name, b.name],
        "old": old.to_string(), "new": new.to_string(),
        "before_ab": summary(&before_ab), "before_ba": summary(&before_ba),
        "first_ab": summary(&first_ab), "first_ba": summary(&first_ba),
        "after_ab": summary(&after_ab), "after_ba": summary(&after_ba),
        "new_ping_code": new_check.status.code(), "old_ping_code": old_check.status.code(),
        "new_ping_stdout": String::from_utf8_lossy(&new_check.stdout),
        "old_ping_stdout": String::from_utf8_lossy(&old_check.stdout),
        "new_ping_stderr": String::from_utf8_lossy(&new_check.stderr),
        "old_ping_stderr": String::from_utf8_lossy(&old_check.stderr),
    });
    eprintln!("[netlab-mobile-result] {report}");
    if !complete(&first_ab, 1)
        || !complete(&first_ba, 1)
        || !complete(&after_ab, 30)
        || !complete(&after_ba, 30)
        || before_v4 != after_v4
        || entry(&after_v6, old).is_some()
        || entry(&after_v6, new).is_none()
        || !new_check.status.success()
        || old_check.status.code() != Some(1)
    {
        return Err(format!("IPv6-only business migration incomplete: {report}"));
    }
    Ok(report)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
#[ignore = "real Node mobility baseline, not the packet-only calibration"]
async fn mv6_overlap_keeps_bidirectional_reads_without_an_ipv4_backup() {
    run("mobile-v6", FirstBusiness::Both).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
#[ignore = "remote initiates the first business request after the moving peer changes IPv6"]
async fn mv6_overlap_remote_initiates_without_a_moving_side_business_request() {
    run("mobile-v6-remote", FirstBusiness::Remote).await;
}

async fn run(s: &str, first: FirstBusiness) {
    let lab = Lab::new().await.unwrap();
    let relay = relay(&lab, s, "relay", Some(Duration::from_millis(100))).await;
    let left = lab
        .add_router("left")
        .preset(RouterPreset::Public)
        .ip_support(IpSupport::DualStack)
        .build()
        .await
        .unwrap();
    let right = lab
        .add_router("right")
        .preset(RouterPreset::Public)
        .ip_support(IpSupport::DualStack)
        .build()
        .await
        .unwrap();
    let da = lab.add_device("a").uplink(left.id()).build().await.unwrap();
    let db = lab
        .add_device("b")
        .uplink(right.id())
        .build()
        .await
        .unwrap();
    let (a_name, b_name) = match first {
        FirstBusiness::Both => ("mobile-a", "mobile-b"),
        FirstBusiness::Remote => ("mobile-remote-a", "mobile-remote-b"),
    };
    let a = node(s, da, a_name, &[&relay], &[]).await;
    let b = node(s, db, b_name, &[&relay], &[]).await;
    fixture(&a);
    fixture(&b);
    crate::attempts::phase(s, "session_started", now_ms());
    let checked = tokio::time::timeout(Duration::from_secs(240), session(s, &a, &b, first)).await;
    diagnostics(s, &a, &b);
    diagnostics(s, &b, &a);
    let ok = matches!(&checked, Ok(Ok(_)));
    result(s, ok, format!("{} {checked:?}", first.revision()));
    assert!(
        ok,
        "complete bidirectional business reads across IPv6 replacement"
    );
}
