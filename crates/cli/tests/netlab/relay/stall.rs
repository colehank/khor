//! A relay TCP connection can keep delivering packets at several seconds
//! of latency. Slow just that connection while keeping fresh connections
//! fast, then exercise the same already-open remote terminal.
use std::collections::BTreeSet;
use std::net::{Ipv4Addr, SocketAddr};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use khor_core::Hop;
use khor_node::host;
use patchbay::{Device, Lab};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::oneshot;

use super::flows;
use crate::lab::{mesh, node, relay, result, slow};
use crate::nat::{Net, PUNCH, router};

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
        "filter_text":command(dev,"tc",&["-s","filter","show","dev",interface,"parent","1:"]),
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
    let tcp = command(
        dev,
        "ss",
        &[
            "-HtinO",
            "state",
            "established",
            &format!("( sport = :{port} and dst {ip} )"),
        ],
    );
    let held = held_flows(dev, ip, port);
    json!({"at_us":started,"ended_us":pivot.elapsed().as_micros(),"same":held==*expected,"flows":held,"tcp":tcp})
}

fn framed(op: host::ClientOp) -> Vec<u8> {
    let mut bytes = Vec::new();
    host::write_frame(&mut bytes, &op).unwrap();
    bytes
}

async fn output_until(stream: &mut TcpStream, expected: &str) -> Result<(), String> {
    let mut received = Vec::new();
    loop {
        let mut buf = [0; 4096];
        let len = stream.read(&mut buf).await.map_err(|e| e.to_string())?;
        if len == 0 {
            return Err("terminal stream closed".into());
        }
        received.extend_from_slice(&buf[..len]);
        if received
            .windows(expected.len())
            .any(|w| w == expected.as_bytes())
        {
            return Ok(());
        }
        if received.len() > 1024 * 1024 {
            return Err("excessive terminal output".into());
        }
    }
}

async fn terminal_echo(stream: &mut TcpStream, token: &str) -> Result<(), String> {
    let frame = framed(host::ClientOp::Input(khor_node::shell_start::input(token)));
    stream.write_all(&frame).await.map_err(|e| e.to_string())?;
    output_until(stream, &format!("NETLAB-REPLY-{token}")).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "real PTY and per-flow netem require the guarded Linux netlab"]
async fn established_terminal_old_uplink_delay() {
    terminal_with_relay_delay(false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "real PTY and per-flow netem require the guarded Linux netlab"]
async fn established_terminal_slow_relay_uses_live_middle() {
    terminal_with_relay_delay(true).await;
}

async fn terminal_with_relay_delay(peer_hop: bool) {
    let s = if peer_hop {
        "relay-stall-terminal-via-middle"
    } else {
        "relay-stall-terminal-only"
    };
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
            "priomap", "1", "1", "1", "1", "1", "1", "1", "1", "1", "1", "1", "1", "1", "1", "1",
            "1",
        ],
    );
    let initial_delay_ms = if peer_hop { 0 } else { 124 };
    let initial_delay = format!("{initial_delay_ms}ms");
    command(
        &r.dev,
        "tc",
        &[
            "qdisc",
            "add",
            "dev",
            &interface,
            "parent",
            "1:3",
            "handle",
            "30:",
            "netem",
            "delay",
            &initial_delay,
            "limit",
            "1000",
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
    let middle = if peer_hop {
        let public = router(&lab, "public", Net::Public).await;
        let device = lab
            .add_device("middle")
            .uplink(public.id())
            .build()
            .await
            .unwrap();
        // Before the impairment the steady-state selector should prefer
        // the public relay. The middle still has ample interactive capacity.
        slow(&device, Duration::from_millis(70)).await;
        let middle = node(s, device, "middle", &[&r], &[]).await;
        for end in [&a, &b] {
            khor_node::config::set(&end.home.0, "peer-hop", "off").unwrap();
        }
        Some(middle)
    } else {
        None
    };
    tokio::time::sleep(Duration::from_secs(3)).await;
    let mut peers = vec![&a, &b];
    peers.extend(middle.iter());
    mesh(&peers).await;
    for (from, to) in [(&a, &b), (&b, &a)] {
        from.until(to, Duration::from_secs(60), |l| {
            l.hop == Hop::Relay && l.rtt_ms.is_some()
        })
        .await
        .expect("both residents use the actual single relay");
    }
    if let Some(middle) = &middle {
        for end in [&a, &b] {
            // Allow iroh to retry hole punching before injecting the stall.
            end.until(middle, PUNCH, |link| {
                link.hop == Hop::Direct && link.rtt_ms.is_some()
            })
            .await
            .expect("the middle has a healthy first leg from both ends");
        }
        let (via, target) = (middle.id.clone(), b.id.clone());
        tokio::time::timeout(Duration::from_secs(35), async {
            loop {
                let (via, target) = (via.clone(), target.clone());
                if a.with(move |node| async move {
                    node.devices().unwrap().iter().any(|d| {
                        d.id == via
                            && node.roads_of(d).iter().any(|road| {
                                road.to == target
                                    && road.road == khor_core::RoadKind::Direct
                                    && road.ms.is_some()
                            })
                    })
                })
                .await
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
        })
        .await
        .expect("alpha knows that the middle has a healthy onward leg");
    }
    let cwd = b.home.0.join("terminal-stall");
    std::fs::create_dir_all(&cwd).unwrap();
    let target = cwd.to_string_lossy().into_owned();
    let (ready_tx, ready_rx) = oneshot::channel();
    let (fault_tx, fault_rx) = oneshot::channel();
    let an = a.with(|n| async move { n }).await;
    let terminal = a.dev.spawn(move |_| async move {
        let program = vec!["/bin/sh".into(), "-c".into(), r#"stty -echo; printf 'NETLAB-ECHO-READY\n'; while IFS= read -r line; do printf '%s\n' "$line" >> commands.log; printf 'NETLAB-REPLY-%s\n' "$line"; done"#.into()];
        let id = an.open_on("beta", "shell", "netlab terminal relay", &target, &program, (100,30), Some("netlab-terminal-relay-stall")).await.unwrap();
        let (addr, cookie) = an.reach("beta", &id).await.unwrap();
        let socket = tokio::task::spawn_blocking(move || host::connect_terminal(&addr, &cookie, 100, 30))
            .await.unwrap().unwrap();
        socket.set_nonblocking(true).unwrap();
        let mut stream = TcpStream::from_std(socket).unwrap();
        stream.write_all(&framed(host::ClientOp::Ready)).await.unwrap();
        output_until(&mut stream, "NETLAB-ECHO-READY").await.unwrap();
        terminal_echo(&mut stream, "before-fault").await.unwrap();
        ready_tx.send(()).unwrap();
        fault_rx.await.unwrap();
        let mut outcomes = Vec::new();
        for token in ["first-after-fault", "second-after-fault"] {
            let began = Instant::now();
            let error = match tokio::time::timeout(Duration::from_secs(9), terminal_echo(&mut stream, token)).await {
                Ok(result) => result.err(),
                Err(_) => Some("terminal caller deadline".into()),
            };
            outcomes.push(json!({"token":token,"us":began.elapsed().as_micros(),"error":error}));
        }
        outcomes
    }).unwrap();
    tokio::time::timeout(Duration::from_secs(45), ready_rx)
        .await
        .unwrap()
        .unwrap();
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
    assert_eq!(expected.len(), 1, "this diagnostic isolates one connection");
    let (_, old_port) = *expected.iter().next().unwrap();
    let relay_ip = addr.ip().to_string();
    let in_router = |program: &str, args: &[&str]| {
        let mut cmd = Command::new(program);
        cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());
        let out = ra
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
    };
    let wan_route: Value =
        serde_json::from_str(&in_router("ip", &["-j", "route", "get", &relay_ip])).unwrap();
    let wan = wan_route[0]["dev"].as_str().unwrap();
    in_router(
        "tc",
        &[
            "qdisc", "replace", "dev", wan, "root", "handle", "1:", "prio", "bands", "3",
            "priomap", "1", "1", "1", "1", "1", "1", "1", "1", "1", "1", "1", "1", "1", "1", "1",
            "1",
        ],
    );
    in_router(
        "tc",
        &[
            "qdisc", "add", "dev", wan, "parent", "1:3", "handle", "30:", "netem", "delay",
            "3200ms", "limit", "1000",
        ],
    );
    let source_match = old_port.to_string();
    let relay_port = port.to_string();
    let source = format!("{a_ip}/32");
    let destination = format!("{relay_ip}/32");
    let mut filter = vec![
        "filter",
        "add",
        "dev",
        wan,
        "protocol",
        "ip",
        "parent",
        "1:",
        "prio",
        "2",
        "u32",
        "match",
        "ip",
        "protocol",
        "6",
        "0xff",
        "match",
        "ip",
        "src",
        &source,
        "match",
        "ip",
        "dst",
        &destination,
        "match",
        "ip",
        "dport",
        &relay_port,
        "0xffff",
    ];
    if !peer_hop {
        filter.extend(["match", "ip", "sport", &source_match, "0xffff"]);
    }
    filter.extend(["flowid", "1:3"]);
    in_router("tc", &filter);
    command(
        &r.dev,
        "tc",
        &[
            "filter", "del", "dev", &interface, "protocol", "ip", "parent", "1:", "prio", "1",
        ],
    );
    let pivot = Instant::now();
    let after_shape = in_router("tc", &["-s", "-j", "qdisc", "show", "dev", wan]);
    let wan_filter = in_router("tc", &["-s", "filter", "show", "dev", wan, "parent", "1:"]);
    eprintln!(
        "[netlab-relay-old-flow-uplink-delay] {}",
        json!({
            "schema":1,"revision":"old_flow_uplink_delay_v2","event":"manifest",
            "alpha_public_ip":a_ip,"relay_port":port,"interface":interface,"route":route,
            "before_shape":before_shape,"after_shape":after_shape,"wan_filter":wan_filter,"old_tcp_flows":expected,
            "initial_delay_us":initial_delay_ms*1000,"post_delay_us":3200000,"post_fault_terminal_commands":2,
            "terminal_caller_us":9000000,"flow_period_us":200000,
            "all_relay_tcp_delayed":peer_hop,"no_drop_reset_or_address_change":true,
        })
    );
    let probe_url = format!("{}/ping", r.url.trim_end_matches('/'));
    let mut probe = Command::new("curl");
    probe
        .args([
            "--noproxy",
            "*",
            "-sS",
            "-4",
            "--max-time",
            "2",
            "-o",
            "/dev/null",
            "-w",
            "%{http_code} %{time_connect} %{time_starttransfer} %{time_total}",
            &probe_url,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let fresh_probe = a.dev.spawn_command_sync(probe).unwrap();
    if peer_hop {
        for end in [&a, &b] {
            khor_node::config::set(&end.home.0, "peer-hop", "on").unwrap();
        }
    }
    fault_tx.send(()).unwrap();
    tokio::pin!(terminal);
    let mut clock = tokio::time::interval(Duration::from_millis(200));
    clock.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut flow_samples = Vec::new();
    let outcomes = loop {
        tokio::select! {
            outcome = &mut terminal => break outcome.unwrap(),
            _ = clock.tick() => flow_samples.push(sample_flows(&r.dev, a_ip, port, &expected, pivot)),
        }
    };
    flow_samples.push(sample_flows(&r.dev, a_ip, port, &expected, pivot));
    let fresh = fresh_probe.wait_with_output().unwrap();
    let fresh_http =
        json!({"ok":fresh.status.success(),"body":String::from_utf8_lossy(&fresh.stdout)});
    let observed = std::fs::read_to_string(cwd.join("commands.log")).unwrap_or_default();
    eprintln!(
        "[netlab-terminal-relay-stall] {}",
        json!({
            "outcomes":outcomes,"flow_samples":flow_samples,"fresh_http":fresh_http,
            "commands":observed.lines().collect::<Vec<_>>(), "journal":a.journal(Some(&b)),
            "reverse_journal":b.journal(Some(&a)),
        })
    );
    assert_eq!(
        fresh.status.success(),
        !peer_hop,
        "the fresh relay is slow only in the middle case"
    );
    let recovered = outcomes
        .iter()
        .all(|x| x["error"].is_null() && x["us"].as_u64().unwrap() < 2_000_000);
    let once = observed == "before-fault\nfirst-after-fault\nsecond-after-fault\n";
    let last = &flow_samples.last().unwrap()["flows"];
    let replaced = last
        .as_array()
        .unwrap()
        .iter()
        .any(|flow| flow != &json!([port, old_port]));
    crate::lab::sample_diagnostics(s, &a);
    crate::lab::sample_diagnostics(s, &b);
    let forwarded = a
        .journal(Some(&b))
        .iter()
        .any(|row| row["e"] == "hop" && !row["via"].is_null())
        || b.journal(Some(&a))
            .iter()
            .any(|row| row["e"] == "hop" && !row["via"].is_null());
    let recovery = if peer_hop { forwarded } else { replaced };
    result(
        s,
        recovered && once && recovery,
        format!(
            "each answer under two seconds={recovered}, exact input={once}, old TCP replaced={replaced}, peer hop opened={forwarded}"
        ),
    );
    assert!(
        recovered && once && recovery,
        "open terminal should recover in place without replaying input"
    );
}
