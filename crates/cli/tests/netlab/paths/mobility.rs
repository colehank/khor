//! Physical address-change calibration; not a Node recovery/SLO test.
use super::*;
use std::net::Ipv6Addr;

const REVISION: &str = "ipv6_renumber_v1";

pub(super) fn command(dev: &Device, args: &[&str]) -> std::process::Output {
    let mut command = Command::new(args[0]);
    command
        .args(&args[1..])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    dev.spawn_command_sync(command)
        .unwrap()
        .wait_with_output()
        .unwrap()
}

pub(super) fn addresses(dev: &Device, family: &str) -> serde_json::Value {
    let output = command(dev, &["ip", "-j", family, "address", "show"]);
    assert!(output.status.success(), "read namespace addresses");
    serde_json::from_slice(&output.stdout).unwrap()
}

pub(super) fn entry(
    snapshot: &serde_json::Value,
    ip: Ipv6Addr,
) -> Option<(String, serde_json::Value)> {
    for iface in snapshot.as_array()? {
        for info in iface["addr_info"].as_array()? {
            if info["local"]
                .as_str()
                .and_then(|s| s.parse::<Ipv6Addr>().ok())
                == Some(ip)
            {
                return Some((iface["ifname"].as_str()?.to_owned(), info.clone()));
            }
        }
    }
    None
}

fn snapshot(dev: &Device, phase: &str) -> (serde_json::Value, serde_json::Value) {
    let v4 = addresses(dev, "-4");
    let v6 = addresses(dev, "-6");
    eprintln!(
        "[netlab-mobility-addresses] {}",
        serde_json::json!({
            "revision": REVISION, "phase": phase, "at_ms": now_ms(), "v4": v4, "v6": v6,
        }),
    );
    (v4, v6)
}

fn ping(dev: &Device, from: &str, family: &str, address: &str, should_reply: bool) {
    let began = Instant::now();
    let output = command(dev, &["ping", family, "-n", "-c", "1", "-W", "2", address]);
    eprintln!(
        "[netlab-mobility-check] {}",
        serde_json::json!({
            "revision": REVISION, "from": from, "family": family, "address": address,
            "expected": if should_reply { "reply" } else { "unreachable" },
            "code": output.status.code(), "elapsed_us": began.elapsed().as_micros(),
            "stdout": String::from_utf8_lossy(&output.stdout),
            "stderr": String::from_utf8_lossy(&output.stderr),
        }),
    );
    assert_eq!(output.status.code(), Some(if should_reply { 0 } else { 1 }));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "address injection calibration, not a Node recovery gate"]
async fn ipv6_replacement_preserves_ipv4_and_changes_reachability() {
    let s = "mobility-v6-calibration";
    let lab = Lab::new().await.unwrap();
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
    let a = lab.add_device("a").uplink(left.id()).build().await.unwrap();
    let b = lab
        .add_device("b")
        .uplink(right.id())
        .build()
        .await
        .unwrap();
    let old = a.ip6().expect("dual-stack calibration has IPv6");
    let peer = b.ip6().expect("dual-stack peer has IPv6");
    let v4_peer = b.ip().expect("dual-stack peer has IPv4");
    let (before_v4, before_v6) = snapshot(&a, "before");
    let (interface, info) = entry(&before_v6, old).expect("old address is on the device");
    let prefix = info["prefixlen"].as_u64().unwrap();
    assert!(
        prefix <= 120,
        "room for a second address on the routed prefix"
    );
    let mut octets = old.octets();
    octets[15] ^= 64;
    let new = Ipv6Addr::from(octets);
    assert_ne!(new, peer);
    assert!(entry(&before_v6, new).is_none(), "new address is unused");
    ping(&b, "b-before", "-6", &old.to_string(), true);
    ping(&a, "a-before", "-6", &peer.to_string(), true);
    ping(&a, "a-before", "-4", &v4_peer.to_string(), true);

    let replacement = format!("{new}/{prefix}");
    let output = command(
        &a,
        &[
            "ip",
            "-6",
            "address",
            "add",
            &replacement,
            "dev",
            &interface,
        ],
    );
    assert!(output.status.success(), "add the owned namespace address");
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        let (_, info) = entry(&addresses(&a, "-6"), new).expect("new address exists");
        assert_ne!(info["dadfailed"], serde_json::Value::Bool(true));
        if info["tentative"] != serde_json::Value::Bool(true) {
            break;
        }
        assert!(Instant::now() < until, "new address completes DAD");
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let (added_v4, added_v6) = snapshot(&a, "added");
    assert_eq!(added_v4, before_v4);
    assert!(entry(&added_v6, old).is_some() && entry(&added_v6, new).is_some());
    ping(&b, "b-added", "-6", &new.to_string(), true);

    let previous = format!("{old}/{prefix}");
    let output = command(
        &a,
        &["ip", "-6", "address", "del", &previous, "dev", &interface],
    );
    assert!(
        output.status.success(),
        "remove only the old namespace address"
    );
    let (after_v4, after_v6) = snapshot(&a, "removed");
    assert_eq!(after_v4, before_v4);
    assert!(entry(&after_v6, old).is_none() && entry(&after_v6, new).is_some());
    ping(&b, "b-removed", "-6", &new.to_string(), true);
    ping(&a, "a-removed", "-6", &peer.to_string(), true);
    ping(&a, "a-removed", "-4", &v4_peer.to_string(), true);
    ping(&b, "b-old-removed", "-6", &old.to_string(), false);
    result(
        s,
        true,
        format!(
            "{REVISION}: IPv6 {old} -> {new}, IPv4 unchanged; 7 positive, 1 negative packet checks"
        ),
    );
}
