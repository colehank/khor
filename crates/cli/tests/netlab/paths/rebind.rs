//! A router changes only the external UDP port; the Node interface stays unchanged.
use super::mobile::{complete, diagnostics, fixture, reads, summary};
use super::mobility::addresses;
use super::*;
use std::io::Write;
use std::net::SocketAddr;

struct Network {
    _reflector: patchbay::ReflectorGuard,
    a: Device,
    b: Device,
    nat: Router,
    _lab: Lab,
}

impl Network {
    fn reflector(&self) -> SocketAddr {
        SocketAddr::new(self.b.ip().unwrap().into(), 39999)
    }

    fn mapped(&self, port: u16) -> SocketAddr {
        SocketAddr::new(self.nat.uplink_ip().unwrap().into(), port)
    }

    fn mapping(&self, s: &str, port: u16, initial: bool) {
        let header = if initial {
            "table ip rebind_v4 {\n chain mapped { type nat hook postrouting priority 90; policy accept;\n }\n}\n"
        } else {
            "flush chain ip rebind_v4 mapped\n"
        };
        let rules = format!(
            "{header}add rule ip rebind_v4 mapped oifname \"ix\" ip saddr {} ip daddr {} meta l4proto udp snat to {}\n",
            self.a.ip().unwrap(),
            self.b.ip().unwrap(),
            self.mapped(port),
        );
        let mut command = Command::new("nft");
        command
            .args(["-f", "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = self.nat.spawn_command_sync(command).unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(rules.as_bytes())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        eprintln!(
            "[netlab-rebind-rules] {}",
            serde_json::json!({
                "case": s, "port": port, "initial": initial, "rules": rules,
                "code": output.status.code(), "stdout": String::from_utf8_lossy(&output.stdout),
                "stderr": String::from_utf8_lossy(&output.stderr),
            })
        );
        assert!(
            output.status.success(),
            "install rules only in the owned router namespace"
        );
    }

    fn conntrack(&self, s: &str, phase: &str, delete: bool) -> serde_json::Value {
        let mut command = Command::new("conntrack");
        command
            .args([
                if delete { "-D" } else { "-L" },
                "-p",
                "udp",
                "--orig-src",
                &self.a.ip().unwrap().to_string(),
                "--orig-dst",
                &self.b.ip().unwrap().to_string(),
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let output = self
            .nat
            .spawn_command_sync(command)
            .unwrap()
            .wait_with_output()
            .unwrap();
        let value = serde_json::json!({
            "case": s, "phase": phase, "delete": delete, "at_ms": now_ms(),
            "code": output.status.code(), "stdout": String::from_utf8_lossy(&output.stdout),
            "stderr": String::from_utf8_lossy(&output.stderr),
        });
        eprintln!("[netlab-rebind-conntrack] {value}");
        assert!(
            output.status.success(),
            "inspect or delete only this fixture's UDP flows"
        );
        value
    }

    fn snapshot(&self, s: &str, phase: &str) -> (serde_json::Value, serde_json::Value) {
        let v4 = addresses(&self.a, "-4");
        let v6 = addresses(&self.a, "-6");
        eprintln!(
            "[netlab-rebind-addresses] {}",
            serde_json::json!({
                "case": s, "phase": phase, "at_ms": now_ms(), "v4": v4, "v6": v6,
            })
        );
        (v4, v6)
    }
}

async fn network() -> Network {
    let lab = Lab::new().await.unwrap();
    let nat = lab
        .add_router("nat")
        .nat(Nat::Strict)
        .ip_support(IpSupport::V4Only)
        .build()
        .await
        .unwrap();
    let public = lab
        .add_router("public")
        .preset(RouterPreset::PublicV4)
        .build()
        .await
        .unwrap();
    let a = lab.add_device("a").uplink(nat.id()).build().await.unwrap();
    let b = lab
        .add_device("b")
        .uplink(public.id())
        .build()
        .await
        .unwrap();
    let reflector = b
        .spawn_reflector(SocketAddr::new(b.ip().unwrap().into(), 39999))
        .await
        .unwrap();
    Network {
        _reflector: reflector,
        a,
        b,
        nat,
        _lab: lab,
    }
}

async fn direct(s: &str, a: &LabNode, b: &LabNode) {
    let mut held = None;
    loop {
        let both = a.link(b).await.is_some_and(|v| v.hop == Hop::Direct)
            && b.link(a).await.is_some_and(|v| v.hop == Hop::Direct);
        if both {
            let since = held.get_or_insert_with(|| {
                crate::attempts::phase(s, "direct_hold_started", now_ms());
                Instant::now()
            });
            if since.elapsed() >= Duration::from_secs(10) {
                return;
            }
        } else {
            held = None;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

fn observed(b: &LabNode, a: &LabNode, address: SocketAddr) -> bool {
    b.journal(Some(a)).iter().any(|event| {
        event["e"] == "path"
            && event["road"] == "direct"
            && matches!(event["change"].as_str(), Some("started" | "selected"))
            && event["addr"].as_str() == Some(address.to_string().as_str())
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "physical NAT port-only injection calibration; no Node business"]
async fn r0_fixed_local_udp_tuple_gets_a_new_external_port() {
    let s = "rebind-calibration";
    let net = network().await;
    net.mapping(s, 50001, true);
    let before = net.snapshot(s, "before");
    let old = net.a.probe_udp_mapping(net.reflector()).unwrap();
    assert_eq!(old, net.mapped(50001));
    let listed = net.conntrack(s, "before", false);
    net.mapping(s, 50002, false);
    let deleted = net.conntrack(s, "deleted", true);
    let new = net.a.probe_udp_mapping(net.reflector()).unwrap();
    let after = net.snapshot(s, "after");
    let remaining = net.conntrack(s, "after", false);
    eprintln!(
        "[netlab-rebind-calibration] {}",
        serde_json::json!({
            "revision": "nat_port_rebind_calibration_v1", "old": old, "new": new,
            "addresses_unchanged": before == after, "before": listed,
            "deleted": deleted, "after": remaining,
        })
    );
    assert_eq!(new, net.mapped(50002));
    assert_eq!(before, after, "neither family nor interface changed");
    println!(
        "[netlab-result] {s} | green | nat_port_rebind_calibration_v1 old={old} new={new} addresses_unchanged=true"
    );
}

async fn business(
    s: &str,
    net: &Network,
    a: &LabNode,
    b: &LabNode,
) -> Result<serde_json::Value, String> {
    tokio::time::timeout(Duration::from_secs(120), async {
        mesh(&[a, b]).await;
        for (from, to) in [(a, b), (b, a)] {
            let checked = reads(&format!("{s}-setup"), from, to, 1).await;
            if !complete(&checked, 1) {
                return Err(format!("setup failed: {}", summary(&checked)));
            }
        }
        direct(s, a, b).await;
        Ok(())
    })
    .await
    .map_err(|_| "rebind setup exceeded 120 seconds".to_owned())??;
    let before_ab = reads(&format!("{s}-before-ab"), a, b, 30).await;
    let before_ba = reads(&format!("{s}-before-ba"), b, a, 30).await;
    if !complete(&before_ab, 30) || !complete(&before_ba, 30) {
        return Err("before-rebind business incomplete".to_owned());
    }
    tokio::time::timeout(Duration::from_secs(45), direct(s, a, b))
        .await
        .map_err(|_| "old direct road did not hold before rebinding".to_owned())?;
    if !observed(b, a, net.mapped(50001)) {
        return Err("old external QUIC path was not observed".to_owned());
    }
    let before = net.snapshot(s, "before");
    let listed = net.conntrack(s, "before", false);
    crate::attempts::phase(s, "mapping_change_started", now_ms());
    net.mapping(s, 50002, false);
    let deleted = net.conntrack(s, "deleted", true);
    crate::attempts::phase(s, "mapping_changed", now_ms());
    let first_ba = reads(&format!("{s}-first-ba"), b, a, 1).await;
    crate::attempts::phase(s, "first_remote_finished", now_ms());
    let first_ab = reads(&format!("{s}-first-ab"), a, b, 1).await;
    let after_ab = reads(&format!("{s}-after-ab"), a, b, 30).await;
    let after_ba = reads(&format!("{s}-after-ba"), b, a, 30).await;
    crate::attempts::phase(s, "reads_finished", now_ms());
    let after = net.snapshot(s, "after");
    let observed_mapping = net
        .a
        .probe_udp_mapping(net.reflector())
        .map_err(|e| e.to_string())?;
    let remaining = net.conntrack(s, "after", false);
    let report = serde_json::json!({
        "case": s, "revision": "node_nat_port_rebind_v1", "nodes": [a.name, b.name],
        "old": net.mapped(50001), "new": net.mapped(50002), "observed_mapping": observed_mapping,
        "addresses_unchanged": before == after, "before": listed, "deleted": deleted,
        "after": remaining, "new_quic_path_selected": observed(b, a, net.mapped(50002)),
        "before_ab": summary(&before_ab), "before_ba": summary(&before_ba),
        "first_ab": summary(&first_ab), "first_ba": summary(&first_ba),
        "after_ab": summary(&after_ab), "after_ba": summary(&after_ba),
    });
    eprintln!("[netlab-rebind-result] {report}");
    if before != after
        || observed_mapping != net.mapped(50002)
        || !complete(&first_ba, 1)
        || !complete(&first_ab, 1)
        || !complete(&after_ab, 30)
        || !complete(&after_ba, 30)
    {
        return Err(format!("port-only rebind business incomplete: {report}"));
    }
    Ok(report)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
#[ignore = "real Node business after NAT port-only rebinding without changing its interface"]
async fn r1_remote_reads_when_only_the_external_nat_port_changes() {
    let s = "rebind-node";
    let net = network().await;
    let relay = relay(&net._lab, s, "relay", Some(Duration::from_millis(100))).await;
    net.mapping(s, 50001, true);
    let a = node(s, net.a.clone(), "rebind-a", &[&relay], &[]).await;
    let b = node(s, net.b.clone(), "rebind-b", &[&relay], &[]).await;
    fixture(&a);
    fixture(&b);
    crate::attempts::phase(s, "session_started", now_ms());
    let checked = tokio::time::timeout(Duration::from_secs(240), business(s, &net, &a, &b)).await;
    diagnostics(s, &a, &b);
    diagnostics(s, &b, &a);
    let ok = matches!(&checked, Ok(Ok(_)));
    result(s, ok, format!("node_nat_port_rebind_v1 {checked:?}"));
    assert!(
        ok,
        "complete remote-first business through a port-only NAT rebind"
    );
}
