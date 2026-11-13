//! Real ordinary requests on cached wires, with a scratch peer at loopback.

use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

struct Fixture {
    _root: tempfile::TempDir,
    node: Arc<Node>,
    peer: DeviceInfo,
    ep: iroh::Endpoint,
    server: iroh::Endpoint,
    local: iroh::endpoint::Connection,
    remote: iroh::endpoint::Connection,
}

impl Fixture {
    async fn new() -> Self {
        let server = iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
            .relay_mode(iroh::RelayMode::Disabled)
            .alpns(vec![ALPN.to_vec()])
            .clear_ip_transports()
            .bind_addr("127.0.0.1:0")
            .unwrap()
            .bind()
            .await
            .unwrap();
        let (root, node, mut peer, ep) = fixture_for(server.id().to_string()).await;
        peer.addrs = server
            .addr()
            .addrs
            .iter()
            .filter_map(|addr| match addr {
                iroh::TransportAddr::Ip(addr) => Some(addr.to_string()),
                _ => None,
            })
            .collect();
        assert!(!peer.addrs.is_empty());
        // Finish the handshake and adopt the wire before starting any deadline.
        let (local, remote) = tokio::time::timeout(Duration::from_secs(5), async {
            tokio::join!(ep.connect(server.addr(), ALPN), async {
                server.accept().await.unwrap().accept().unwrap().await
            })
        })
        .await
        .unwrap();
        let (local, remote) = (local.unwrap(), remote.unwrap());
        node.wires
            .adopt(&peer.id, wires::Proto::Main, local.clone());
        Self {
            _root: root,
            node: Arc::new(node),
            peer,
            ep,
            server,
            local,
            remote,
        }
    }

    fn line(&self) -> Line {
        Line {
            id: self.peer.id.clone(),
            name: self.peer.name.clone(),
            proto: wires::Proto::Main,
            conn: self.local.clone(),
        }
    }

    async fn close(&self) {
        self.ep.close().await;
        self.server.close().await;
    }
}

pub(super) async fn observed_by<T>(task: &mut Aborts<T>, by: tokio::time::Instant) -> Option<T> {
    match tokio::time::timeout_at(by, task.0.as_mut().unwrap()).await {
        Ok(Ok(answer)) => Some(answer),
        Ok(Err(_)) => None,
        Err(_) => {
            task.0.as_ref().unwrap().abort();
            let _ = task.0.as_mut().unwrap().await;
            None
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ordinary_expiry_stops_its_stream_and_preserves_a_concurrent_request() {
    observe_ordinary_expiry(Duration::from_secs(1)).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_long_wait_on_an_acknowledging_wire_keeps_its_concurrent_request() {
    observe_ordinary_expiry(Duration::from_millis(4100)).await;
}

async fn observe_ordinary_expiry(within: Duration) {
    let f = Fixture::new().await;
    let heard_before = wires::heard_count(&f.local);
    let patience = wires::Patience::of(&f.local).ack;
    assert_eq!(patience, Duration::from_secs(4));
    let (entered, ready) = tokio::sync::oneshot::channel();
    let (control_entered, control_ready) = tokio::sync::oneshot::channel();
    let (stopped, stop_seen) = tokio::sync::oneshot::channel();
    let requests = Arc::new(AtomicUsize::new(0));
    let mut serving = Aborts(Some(tokio::spawn({
        let conn = f.remote.clone();
        let requests = requests.clone();
        async move {
            let (delayed, mut recv) = conn.accept_bi().await.unwrap();
            let bytes = recv.read_to_end(MAX_FRAME).await.unwrap();
            assert!(matches!(
                proto::question(&bytes).unwrap(),
                Request::Sessions
            ));
            requests.fetch_add(1, Ordering::SeqCst);
            entered.send(()).unwrap();
            let release = tokio::sync::Notify::new();
            tokio::join!(
                async {
                    let _ = stopped.send(matches!(delayed.stopped().await, Ok(Some(_))));
                    release.notify_one();
                },
                async {
                    let (mut send, mut recv) = conn.accept_bi().await.unwrap();
                    let bytes = recv.read_to_end(MAX_FRAME).await.unwrap();
                    assert!(matches!(
                        proto::question(&bytes).unwrap(),
                        Request::Sessions
                    ));
                    requests.fetch_add(1, Ordering::SeqCst);
                    control_entered.send(()).unwrap();
                    // This second request is in flight at expiry, and answers
                    // only after the first stream has been stopped.
                    release.notified().await;
                    let answer = proto::encode(&Response::SessionRows { rows: vec![] }).unwrap();
                    send.write_all(&answer).await.unwrap();
                    send.finish().unwrap();
                }
            );
            conn.closed().await;
        }
    })));
    let by = tokio::time::Instant::now() + within;
    let mut asking = Aborts(Some(tokio::spawn({
        let node = f.node.clone();
        let line = f.line();
        crate::deadline::scope(by, async move {
            node.exchange_on(&line, &Request::Sessions).await
        })
    })));
    tokio::time::timeout_at(by, ready).await.unwrap().unwrap();
    let mut control = Aborts(Some(tokio::spawn({
        let node = f.node.clone();
        let ep = f.ep.clone();
        let peer = f.peer.clone();
        async move { node.ask_on(&ep, &peer, &Request::Sessions).await }
    })));
    tokio::time::timeout_at(by, control_ready)
        .await
        .unwrap()
        .unwrap();
    assert!(tokio::time::Instant::now() < by);
    let answer = observed_by(&mut asking, by + Duration::from_millis(500)).await;
    let stream_stopped = matches!(
        tokio::time::timeout(Duration::from_millis(500), stop_seen).await,
        Ok(Ok(true))
    );
    let control = observed_by(
        &mut control,
        tokio::time::Instant::now() + Duration::from_secs(3),
    )
    .await;
    let live = f.node.wires.live(&f.peer.id, wires::Proto::Main);
    let counts = f.node.wires.counts.snapshot();
    let heard_after = wires::heard_count(&f.local);
    let down = f.node.wires.link(&f.peer.id).down;
    let backoff = f.node.wires.backing_off(&f.peer.id);
    f.close().await;
    serving.0.as_ref().unwrap().abort();
    let _ = serving.0.as_mut().unwrap().await;

    assert!(matches!(answer, Some(Err(Miss::CallerExpired))));
    assert!(
        stream_stopped,
        "expiry must stop only the abandoned response stream"
    );
    assert!(matches!(control, Some(Ok(Response::SessionRows { .. }))));
    assert_eq!(live.map(|conn| conn.stable_id()), Some(f.local.stable_id()));
    assert_eq!(requests.load(Ordering::SeqCst), 2);
    assert_eq!(counts.reads_retried, 0);
    assert_eq!(counts.dial_failed, 0);
    assert_eq!(counts.answers_late, 0);
    assert_eq!(counts.retired_unanswered, 0);
    assert!(
        heard_after > heard_before,
        "the peer's transport actually acknowledged"
    );
    if within > patience {
        assert!(
            tokio::time::Instant::now() >= by,
            "this control crosses the real ACK floor"
        );
    }
    assert_eq!(down, None);
    assert_eq!(backoff, None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_dial_uses_the_earlier_operations_deadline() {
    observe_update_dial_limit(Duration::from_millis(200), Duration::from_millis(200)).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_dial_keeps_ten_second_cap_inside_a_long_operation() {
    observe_update_dial_limit(Duration::from_secs(30), Duration::from_secs(10)).await;
}

async fn observe_update_dial_limit(operation: Duration, expected: Duration) {
    let socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let (_root, node, mut peer, ep) = fixture().await;
    peer.addrs = vec![socket.local_addr().unwrap().to_string()];
    let packets = Arc::new(AtomicUsize::new(0));
    let (first, first_seen) = tokio::sync::oneshot::channel();
    let mut sink = Aborts(Some(tokio::spawn({
        let packets = packets.clone();
        async move {
            let mut first = Some(first);
            let mut buffer = [0u8; 2048];
            loop {
                socket.recv_from(&mut buffer).await.unwrap();
                packets.fetch_add(1, Ordering::SeqCst);
                if let Some(first) = first.take() {
                    let _ = first.send(tokio::time::Instant::now());
                }
            }
        }
    })));
    let start = tokio::time::Instant::now();
    let observed = tokio::time::timeout_at(
        start + expected + Duration::from_millis(500),
        crate::deadline::scope(start + operation, node.dial_update(&ep, &peer)),
    )
    .await;
    let elapsed = start.elapsed();
    let first = tokio::time::timeout(Duration::from_millis(100), first_seen).await;
    let counts = node.wires.counts.snapshot();
    let down = node.wires.link(&peer.id).down;
    let backoff = node.wires.backing_off(&peer.id);
    ep.close().await;
    sink.0.as_ref().unwrap().abort();
    let _ = sink.0.as_mut().unwrap().await;

    assert!(matches!(first, Ok(Ok(at)) if at < start + expected));
    assert!(
        packets.load(Ordering::SeqCst) > 0,
        "the real update dial sent UDP"
    );
    assert!(
        matches!(observed, Ok(Err(_))),
        "dial_update itself must stop, before the fixture's outer timeout"
    );
    assert!(elapsed >= expected - Duration::from_millis(20));
    assert!(elapsed <= expected + Duration::from_millis(500));
    assert_eq!(counts.dial_failed, 0);
    assert_eq!(counts.retired_unanswered, 0);
    assert_eq!(counts.answers_late, 0);
    assert_eq!(down, None);
    assert_eq!(backoff, None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_pending_wire_confirmation_does_not_keep_its_owner_alive() {
    let f = Fixture::new().await;
    let owner = Arc::downgrade(&f.node.wires);
    assert!(f.node.wires.probe_unheard(
        &f.peer.id,
        wires::Proto::Main,
        &f.local,
        &tokio::runtime::Handle::current(),
    ));
    let ep = f.ep.clone();
    let server = f.server.clone();
    drop(f);
    let released = owner.upgrade().is_none();
    ep.close().await;
    server.close().await;
    assert!(
        released,
        "a pending confirmation must not retain its node/wires"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_old_confirmation_cannot_retire_a_replacement_wire() {
    let f = Fixture::new().await;
    let (next, remote) = tokio::time::timeout(Duration::from_secs(5), async {
        tokio::join!(f.ep.connect(f.server.addr(), ALPN), async {
            f.server.accept().await.unwrap().accept().unwrap().await
        })
    })
    .await
    .unwrap();
    let (next, remote) = (next.unwrap(), remote.unwrap());
    assert_ne!(next.stable_id(), f.local.stable_id());
    assert!(f.node.wires.probe_unheard(
        &f.peer.id,
        wires::Proto::Main,
        &f.local,
        &tokio::runtime::Handle::current(),
    ));
    f.node
        .wires
        .adopt(&f.peer.id, wires::Proto::Main, next.clone());
    assert!(f.local.close_reason().is_some());
    tokio::time::sleep(Duration::from_millis(1500)).await;
    let unreachable = f.node.wires.out_of_reach(&f.peer.id);
    let mut serving = Aborts(Some(tokio::spawn(async move {
        let (mut send, mut recv) = remote.accept_bi().await.unwrap();
        let bytes = recv.read_to_end(MAX_FRAME).await.unwrap();
        assert!(matches!(
            proto::question(&bytes).unwrap(),
            Request::Sessions
        ));
        let answer = proto::encode(&Response::SessionRows { rows: vec![] }).unwrap();
        send.write_all(&answer).await.unwrap();
        send.finish().unwrap();
        remote.closed().await;
    })));
    let line = Line {
        conn: next.clone(),
        ..f.line()
    };
    let answer = tokio::time::timeout(
        Duration::from_secs(3),
        f.node.exchange_on(&line, &Request::Sessions),
    )
    .await;
    let live = f.node.wires.live(&f.peer.id, wires::Proto::Main);
    let counts = f.node.wires.counts.snapshot();
    f.close().await;
    serving.0.as_ref().unwrap().abort();
    let _ = serving.0.as_mut().unwrap().await;
    assert!(matches!(answer, Ok(Ok(Response::SessionRows { .. }))));
    assert_eq!(live.map(|conn| conn.stable_id()), Some(next.stable_id()));
    assert!(
        !unreachable,
        "the old proof must not mark the replacement unreachable"
    );
    assert_eq!(counts.retired_unanswered, 0);
    assert_eq!(counts.retired_closed, 0);
}

#[cfg(target_os = "linux")]
fn isolated_network_namespace(test: &str) -> bool {
    use std::process::Command;

    const PARENT_NETNS: &str = "KHOR_WIRE_TEST_PARENT_NETNS";
    let namespace = std::fs::read_link("/proc/self/ns/net").unwrap();
    let Ok(parent_namespace) = std::env::var(PARENT_NETNS) else {
        let out = Command::new("unshare")
            .arg("-Urn")
            .arg(std::env::current_exe().unwrap())
            .arg("--exact")
            .arg(format!("link::dial_budget::wire::{test}"))
            .args(["--nocapture", "--test-threads=1"])
            .env(PARENT_NETNS, &namespace)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "isolated namespace child failed: stdout={} stderr={}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        return false;
    };
    assert_ne!(namespace.to_string_lossy(), parent_namespace);
    let mapping = std::fs::read_to_string("/proc/self/uid_map").unwrap();
    let mapping: Vec<_> = mapping.split_whitespace().collect();
    assert_eq!(mapping.len(), 3);
    assert_eq!(mapping[0], "0");
    assert_eq!(mapping[2], "1", "a fresh unprivileged user namespace");
    assert!(
        Command::new("ip")
            .args(["link", "set", "lo", "up"])
            .status()
            .unwrap()
            .success()
    );
    true
}

#[cfg(target_os = "linux")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn abandoning_a_long_silent_exchange_still_retires_the_dead_wire() {
    observe_abandoned_wire(false).await;
}

#[cfg(target_os = "linux")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_network_change_tick_respects_a_pending_wire_confirmation() {
    observe_abandoned_wire(true).await;
}

#[cfg(target_os = "linux")]
async fn observe_abandoned_wire(changed: bool) {
    use std::future::Future;
    use std::io::Write;
    use std::process::{Command, Stdio};

    let test = if changed {
        "a_network_change_tick_respects_a_pending_wire_confirmation"
    } else {
        "abandoning_a_long_silent_exchange_still_retires_the_dead_wire"
    };
    if !isolated_network_namespace(test) {
        return;
    }

    let f = Fixture::new().await;
    assert_eq!(f.peer.addrs.len(), 1);
    let server: std::net::SocketAddr = f.peer.addrs[0].parse().unwrap();
    assert!(server.ip().is_loopback());
    tokio::time::sleep(Duration::from_secs(1)).await;
    let rules = format!(
        "table inet khor_f7 {{\n    chain output {{\n        type filter hook output priority 0; policy accept;\n        udp sport {} counter drop;\n        udp dport {} counter drop;\n    }}\n}}\n",
        server.port(),
        server.port()
    );
    let mut firewall = Command::new("nft")
        .args(["-f", "-"])
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    firewall
        .stdin
        .take()
        .unwrap()
        .write_all(rules.as_bytes())
        .unwrap();
    assert!(firewall.wait().unwrap().success());
    // Only this new network namespace is filtered, on every native server path.
    tokio::time::sleep(Duration::from_millis(200)).await;
    let heard_before = wires::heard_count(&f.local);
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(
        wires::heard_count(&f.local),
        heard_before,
        "firewall fixture must be quiet"
    );
    let patience = wires::Patience::of(&f.local).ack;
    assert_eq!(patience, Duration::from_secs(4));
    let grace = wires::ask_every_path(&f.local);
    let line = f.line();
    let by = tokio::time::Instant::now() + patience + grace + Duration::from_millis(100);
    let mut asking = Box::pin(crate::deadline::scope(
        by,
        f.node.exchange_on(&line, &Request::Sessions),
    ));
    let pending = std::future::poll_fn(|cx| {
        std::task::Poll::Ready(matches!(asking.as_mut().poll(cx), std::task::Poll::Pending))
    })
    .await;
    assert!(pending, "the real exchange must enter its response wait");
    if changed {
        f.node.wires.network_changed();
    }
    // Keep the application future unpolled while transport I/O continues.
    // This models an operation abandoned after a scheduler delay and makes
    // Drop, rather than the inner liveness beat, own the retirement. Wait out
    // both ACK patience and all-path grace; do not pin an early-retirement policy.
    tokio::time::sleep_until(by + Duration::from_millis(100)).await;
    assert_eq!(
        f.node
            .wires
            .live(&f.peer.id, wires::Proto::Main)
            .map(|c| c.stable_id()),
        Some(f.local.stable_id())
    );
    // Cancel from outside any entered runtime. The request captured its
    // runtime while polling; confirmation still belongs to the wire.
    std::thread::scope(|scope| {
        scope.spawn(move || drop(asking)).join().unwrap();
    });
    assert!(
        f.node.wires.live(&f.peer.id, wires::Proto::Main).is_some(),
        "abandoning starts confirmation, it is not proof of death"
    );
    let tick_retired = if changed { f.node.wires.tick() } else { 0 };
    let live_after_tick = f.node.wires.live(&f.peer.id, wires::Proto::Main);
    let confirmed = tokio::time::timeout(grace + Duration::from_millis(500), async {
        while f.node.wires.live(&f.peer.id, wires::Proto::Main).is_some() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .is_ok();
    let unheard = wires::heard_count(&f.local) == heard_before;
    let rules = Command::new("nft")
        .args(["list", "table", "inet", "khor_f7"])
        .output()
        .unwrap();
    assert!(rules.status.success());
    let rules = String::from_utf8(rules.stdout).unwrap();
    let words: Vec<_> = rules.split_whitespace().collect();
    let dropped: u64 = words
        .windows(2)
        .filter(|pair| pair[0] == "packets")
        .map(|pair| pair[1].parse::<u64>().unwrap())
        .sum();
    let remote_open = f.remote.close_reason().is_none();
    let live = f.node.wires.live(&f.peer.id, wires::Proto::Main);
    let counts = f.node.wires.counts.snapshot();
    f.close().await;

    assert!(unheard, "fixture leaked UDP despite namespace rules");
    assert!(
        dropped > 0,
        "the real request/probes must hit namespace drop rules"
    );
    assert!(
        remote_open,
        "fixture must not substitute a connection close for silence"
    );
    assert!(
        confirmed,
        "the wire owner must finish its all-path confirmation"
    );
    assert!(
        live.is_none(),
        "long real silence must not keep handing out a dead wire"
    );
    assert_eq!(
        tick_retired, 0,
        "tick must not preempt a pending confirmation"
    );
    assert_eq!(
        live_after_tick.map(|c| c.stable_id()),
        Some(f.local.stable_id())
    );
    assert_eq!(counts.retired_unanswered, 1);
    assert_eq!(counts.retired_network, 0);
    assert_eq!(counts.retired_silent, 0);
    assert_eq!(counts.retired_closed, 0);
    assert_eq!(counts.reads_retried, 0);
    assert_eq!(counts.dial_failed, 0);
    assert_eq!(counts.answers_late, 0);
}

#[cfg(target_os = "linux")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn expiry_before_path_probe_grace_preserves_a_slow_live_wire() {
    use std::process::Command;

    if !isolated_network_namespace("expiry_before_path_probe_grace_preserves_a_slow_live_wire") {
        return;
    }
    let f = Fixture::new().await;
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert_eq!(wires::Patience::of(&f.local).ack, Duration::from_secs(4));
    assert!(
        Command::new("tc")
            .args([
                "qdisc", "add", "dev", "lo", "root", "netem", "delay", "2400ms",
            ])
            .status()
            .unwrap()
            .success()
    );
    let heard_before = wires::heard_count(&f.local);
    let requests = Arc::new(AtomicUsize::new(0));
    let (received, reception) = tokio::sync::oneshot::channel();
    let mut serving = Aborts(Some(tokio::spawn({
        let conn = f.remote.clone();
        let requests = requests.clone();
        async move {
            for _ in 0..2 {
                let (mut send, mut recv) = conn.accept_bi().await.unwrap();
                let bytes = recv.read_to_end(MAX_FRAME).await.unwrap();
                assert!(matches!(
                    proto::question(&bytes).unwrap(),
                    Request::Sessions
                ));
                requests.fetch_add(1, Ordering::SeqCst);
                let answer = proto::encode(&Response::SessionRows { rows: vec![] }).unwrap();
                send.write_all(&answer).await.unwrap();
                send.finish().unwrap();
            }
            received.send(tokio::time::Instant::now()).unwrap();
            conn.closed().await;
        }
    })));
    let start = tokio::time::Instant::now();
    let by = start + Duration::from_millis(4100);
    let mut asking = Aborts(Some(tokio::spawn({
        let node = f.node.clone();
        let line = f.line();
        crate::deadline::scope(by, async move {
            node.exchange_on(&line, &Request::Sessions).await
        })
    })));
    let mut control = Aborts(Some(tokio::spawn({
        let node = f.node.clone();
        let line = f.line();
        crate::deadline::scope(start + Duration::from_secs(12), async move {
            node.exchange_on(&line, &Request::Sessions).await
        })
    })));
    let answer = observed_by(&mut asking, by + Duration::from_millis(400)).await;
    let expired_at = tokio::time::Instant::now();
    let heard_at_expiry = wires::heard_count(&f.local);
    let live_at_expiry = f.node.wires.live(&f.peer.id, wires::Proto::Main);
    let received_at = tokio::time::timeout_at(by, reception)
        .await
        .unwrap()
        .unwrap();
    let control = observed_by(&mut control, start + Duration::from_secs(12)).await;
    let heard_after = wires::heard_count(&f.local);
    let live_after = f.node.wires.live(&f.peer.id, wires::Proto::Main);
    let counts = f.node.wires.counts.snapshot();
    let down = f.node.wires.link(&f.peer.id).down;
    let backoff = f.node.wires.backing_off(&f.peer.id);
    assert!(
        Command::new("tc")
            .args(["qdisc", "del", "dev", "lo", "root"])
            .status()
            .unwrap()
            .success()
    );
    f.close().await;
    serving.0.as_ref().unwrap().abort();
    let _ = serving.0.as_mut().unwrap().await;

    eprintln!(
        "[wire-grace-fixture] received_after_ms={} expired_after_ms={} heard_before={} heard_at_expiry={} heard_after={} requests={} live_at_expiry={} control_ok={} retired_unanswered={}",
        received_at.duration_since(start).as_millis(),
        expired_at.duration_since(start).as_millis(),
        heard_before,
        heard_at_expiry,
        heard_after,
        requests.load(Ordering::SeqCst),
        live_at_expiry.is_some(),
        matches!(control, Some(Ok(Response::SessionRows { .. }))),
        counts.retired_unanswered
    );
    assert!(matches!(answer, Some(Err(Miss::CallerExpired))));
    assert!(expired_at <= by + Duration::from_millis(400));
    assert_eq!(
        requests.load(Ordering::SeqCst),
        2,
        "both real requests reached the peer"
    );
    assert!(received_at >= start + Duration::from_millis(2200));
    assert!(received_at + Duration::from_millis(2400) > expired_at);
    assert_eq!(
        heard_at_expiry, heard_before,
        "no packet arrived before the caller expired"
    );
    assert_eq!(
        live_at_expiry.map(|conn| conn.stable_id()),
        Some(f.local.stable_id()),
        "caller expiry before path-probe grace cannot condemn a shared wire"
    );
    assert!(matches!(control, Some(Ok(Response::SessionRows { .. }))));
    assert!(
        heard_after > heard_before,
        "the delayed native UDP road actually answered"
    );
    assert_eq!(
        live_after.map(|conn| conn.stable_id()),
        Some(f.local.stable_id())
    );
    assert_eq!(counts.retired_unanswered, 0);
    assert_eq!(counts.retired_closed, 0);
    assert_eq!(counts.reads_retried, 0);
    assert_eq!(counts.dial_failed, 0);
    assert_eq!(counts.answers_late, 0);
    assert_eq!(down, None);
    assert_eq!(backoff, None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_read_retry_uses_the_original_operations_remaining_time() {
    let f = Fixture::new().await;
    let by = tokio::time::Instant::now() + Duration::from_secs(2);
    let (entered, ready) = tokio::sync::oneshot::channel();
    let (stopped, stop_seen) = tokio::sync::oneshot::channel();
    let requests = Arc::new(AtomicUsize::new(0));
    let mut serving = Aborts(Some(tokio::spawn({
        let first = f.remote.clone();
        let server = f.server.clone();
        let requests = requests.clone();
        async move {
            let (_send, mut recv) = first.accept_bi().await.unwrap();
            let bytes = recv.read_to_end(MAX_FRAME).await.unwrap();
            assert!(matches!(
                proto::question(&bytes).unwrap(),
                Request::Sessions
            ));
            requests.fetch_add(1, Ordering::SeqCst);
            // Spend most of the one operation budget on the first attempt.
            tokio::time::sleep_until(by - Duration::from_millis(700)).await;
            first.close(0u32.into(), b"lost first answer");
            let second = server
                .accept()
                .await
                .unwrap()
                .accept()
                .unwrap()
                .await
                .unwrap();
            let (delayed, mut recv) = second.accept_bi().await.unwrap();
            let bytes = recv.read_to_end(MAX_FRAME).await.unwrap();
            assert!(matches!(
                proto::question(&bytes).unwrap(),
                Request::Sessions
            ));
            requests.fetch_add(1, Ordering::SeqCst);
            entered.send(tokio::time::Instant::now()).unwrap();
            let _ = stopped.send(matches!(delayed.stopped().await, Ok(Some(_))));
            second.closed().await;
        }
    })));
    let mut asking = Aborts(Some(tokio::spawn({
        let node = f.node.clone();
        let ep = f.ep.clone();
        let peer = f.peer.clone();
        crate::deadline::scope(by, async move {
            node.ask_on(&ep, &peer, &Request::Sessions).await
        })
    })));
    let retry_at = tokio::time::timeout_at(by, ready).await.unwrap().unwrap();
    assert!(
        retry_at < by,
        "fixture must reach the second exchange before expiry"
    );
    let answer = observed_by(&mut asking, by + Duration::from_millis(500)).await;
    let ended_at = tokio::time::Instant::now();
    let stream_stopped = matches!(
        tokio::time::timeout(Duration::from_millis(500), stop_seen).await,
        Ok(Ok(true))
    );
    let live = f.node.wires.live(&f.peer.id, wires::Proto::Main);
    let counts = f.node.wires.counts.snapshot();
    f.close().await;
    serving.0.as_ref().unwrap().abort();
    let _ = serving.0.as_mut().unwrap().await;

    assert!(
        matches!(answer, Some(Err(_))),
        "retry ignored the operation deadline"
    );
    assert!(ended_at >= by && ended_at < by + Duration::from_millis(500));
    assert!(stream_stopped);
    assert!(live.is_some_and(|conn| conn.stable_id() != f.local.stable_id()));
    assert_eq!(requests.load(Ordering::SeqCst), 2);
    assert_eq!(counts.reads_retried, 1);
    assert_eq!(counts.dial_failed, 0);
    assert_eq!(counts.answers_late, 0);
    assert_eq!(counts.retired_closed, 1);
    assert_eq!(counts.retired_unanswered, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_unanswered_write_is_not_replayed() {
    lost_answers(false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_unanswered_read_is_retried_at_most_once() {
    lost_answers(true).await;
}

async fn lost_answers(reading: bool) {
    let f = Fixture::new().await;
    let path = f._root.path().join("written-once");
    let req = if reading {
        Request::Sessions
    } else {
        Request::Mkdir {
            path: path.to_string_lossy().into_owned(),
        }
    };
    assert_eq!(req.is_reading(), reading);
    let requests = Arc::new(AtomicUsize::new(0));
    let mut serving = Aborts(Some(tokio::spawn({
        let mut next = Some(f.remote.clone());
        let server = f.server.clone();
        let requests = requests.clone();
        let path = path.clone();
        async move {
            // Continue serving possible replays after dropping every answer;
            // an unserved second connection would hide a repeated write.
            for _ in 0..4 {
                let conn = match next.take() {
                    Some(conn) => conn,
                    None => match tokio::time::timeout(Duration::from_millis(800), server.accept())
                        .await
                    {
                        Ok(Some(incoming)) => incoming.accept().unwrap().await.unwrap(),
                        _ => break,
                    },
                };
                let (_send, mut recv) = conn.accept_bi().await.unwrap();
                let bytes = recv.read_to_end(MAX_FRAME).await.unwrap();
                match proto::question(&bytes).unwrap() {
                    Request::Sessions if reading => {}
                    Request::Mkdir { path: requested } if !reading => {
                        assert_eq!(std::path::Path::new(&requested), path);
                        std::fs::create_dir_all(&path).unwrap();
                    }
                    _ => panic!("unexpected request in replay fixture"),
                }
                requests.fetch_add(1, Ordering::SeqCst);
                conn.close(0u32.into(), b"answer deliberately lost after execution");
            }
        }
    })));
    let mut asking = Aborts(Some(tokio::spawn({
        let node = f.node.clone();
        let ep = f.ep.clone();
        let peer = f.peer.clone();
        async move { node.ask_on(&ep, &peer, &req).await }
    })));
    let answer = observed_by(
        &mut asking,
        tokio::time::Instant::now() + Duration::from_secs(5),
    )
    .await;
    let server_done = observed_by(
        &mut serving,
        tokio::time::Instant::now() + Duration::from_secs(3),
    )
    .await;
    let counts = f.node.wires.counts.snapshot();
    f.close().await;

    assert!(matches!(answer, Some(Err(_))));
    assert!(server_done.is_some());
    assert_eq!(requests.load(Ordering::SeqCst), if reading { 2 } else { 1 });
    assert_eq!(counts.reads_retried, u64::from(reading));
    assert_eq!(counts.dial_failed, 0);
    assert_eq!(path.is_dir(), !reading);
}

#[cfg(target_os = "linux")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancelling_a_silent_accepted_answer_confirms_the_wire() {
    if !isolated_network_namespace("cancelling_a_silent_accepted_answer_confirms_the_wire") {
        return;
    }
    super::observe_accepted_answer_stop(None, true, false).await;
}

#[cfg(target_os = "linux")]
pub(super) async fn observe_accepted_blackhole(
    node: &Node,
    peer: &DeviceInfo,
    conn: &iroh::endpoint::Connection,
    remote: &iroh::endpoint::Connection,
    asking: &mut Aborts<Early>,
    requests: &AtomicUsize,
) {
    use std::io::Write;
    use std::process::{Command, Stdio};

    assert_eq!(peer.addrs.len(), 1);
    let server: std::net::SocketAddr = peer.addrs[0].parse().unwrap();
    assert!(server.ip().is_loopback());
    let rules = format!(
        "table inet khor_f10 {{\n    chain output {{\n        type filter hook output priority 0; policy accept;\n        udp sport {} counter drop;\n        udp dport {} counter drop;\n    }}\n}}\n",
        server.port(),
        server.port()
    );
    let mut firewall = Command::new("nft")
        .args(["-f", "-"])
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    firewall
        .stdin
        .take()
        .unwrap()
        .write_all(rules.as_bytes())
        .unwrap();
    assert!(firewall.wait().unwrap().success());
    tokio::time::sleep(Duration::from_millis(300)).await;
    let count = wires::heard_count(conn);
    let silence = wires::Patience::of(conn).silent;
    assert!(silence >= Duration::from_secs(15) && silence < Duration::from_secs(16));
    let grace = wires::ask_every_path(conn);
    tokio::time::sleep(silence + Duration::from_millis(100)).await;
    assert_eq!(
        wires::heard_count(conn),
        count,
        "native UDP fixture must stay unheard"
    );
    assert!(
        remote.close_reason().is_none(),
        "remote must not substitute a close for silence"
    );
    assert!(
        !asking.0.as_ref().unwrap().is_finished(),
        "Accepted must still await its answer"
    );
    assert_eq!(
        requests.load(Ordering::SeqCst),
        2,
        "warmup and early request only"
    );
    let task = asking.0.take().unwrap();
    task.abort();
    assert!(matches!(task.await, Err(error) if error.is_cancelled()));
    let held_at_cancel = node.wires.live(&peer.id, wires::Proto::Main);
    let confirmed = tokio::time::timeout(grace + Duration::from_millis(500), async {
        while node.wires.live(&peer.id, wires::Proto::Main).is_some() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .is_ok();
    let out = Command::new("nft")
        .args(["list", "table", "inet", "khor_f10"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    let words: Vec<_> = text.split_whitespace().collect();
    let dropped: u64 = words
        .windows(2)
        .filter(|pair| pair[0] == "packets")
        .map(|pair| pair[1].parse::<u64>().unwrap())
        .sum();
    let counts = node.wires.counts.snapshot();
    assert!(dropped > 0, "real native UDP must hit drop rules");
    assert_eq!(wires::heard_count(conn), count);
    assert!(remote.close_reason().is_none());
    assert_eq!(
        held_at_cancel.map(|c| c.stable_id()),
        Some(conn.stable_id())
    );
    assert!(
        confirmed,
        "cancelled silent Accepted answer must leave a bounded wire confirmation"
    );
    assert_eq!(counts.retired_unanswered, 1);
    assert_eq!(counts.retired_network, 0);
    assert_eq!(counts.retired_silent, 0);
    assert_eq!(counts.retired_closed, 0);
    assert_eq!(counts.reads_retried, 0);
    assert_eq!(counts.dial_failed, 0);
    assert_eq!(counts.answers_late, 0);
}
