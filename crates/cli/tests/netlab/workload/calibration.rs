//! Independent kernel TCP evidence for the exact impairment used by w1.
//! No resident/Noq traffic or personal network configuration is touched.
use std::sync::Arc;
use std::time::{Duration, Instant};

use patchbay::{Device, Lab, LinkDirection, Router};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use crate::lab::result;
use crate::nat::{Net, router};

#[path = "calibration_burst.rs"]
mod burst;
#[path = "calibration_capacity.rs"]
mod capacity;
#[path = "calibration_datagrams.rs"]
mod datagrams;
#[path = "calibration_queues.rs"]
mod queues;
#[path = "calibration_sockets.rs"]
mod sockets;

const REVISION: &str = "workload_kernel_tcp_calibration_v1";
const SIZE: usize = 8 * 1024 * 1024;
const WARM: usize = 4;
const ECHOES: usize = 64;
const CALLER: Duration = Duration::from_secs(2);

pub(super) async fn configure_high(
    x: &Device,
    y: &Device,
    rx: &Router,
    ry: &Router,
    revision: &'static str,
) {
    queues::configure(
        x,
        y,
        rx,
        ry,
        Model {
            revision,
            ..HIGH_QUEUES
        },
    )
    .await;
}

pub(super) async fn bounded_snapshot() -> Value {
    capacity::capture().await
}

pub(super) fn split_snapshot_valid(capture: &Value, router: bool) -> bool {
    queues::valid_capture(capture, router)
}

#[derive(Clone, Copy)]
struct Model {
    case: &'static str,
    revision: &'static str,
    size: usize,
    rtt_ms: u64,
    mbit: u64,
    min_rtt_us: [u64; 2],
    goodput_mbit: [f64; 2],
    bulk_limit: Duration,
    condition: fn() -> patchbay::LinkCondition,
    socket_diagnostic: bool,
    socket_plan: &'static [u64],
    burst_bytes: Option<u32>,
    separate_queues: bool,
}

const LOW: Model = Model {
    case: "w0",
    revision: REVISION,
    size: SIZE,
    rtt_ms: 40,
    mbit: 8,
    min_rtt_us: [35000, 80000],
    goodput_mbit: [6.4, 9.6],
    bulk_limit: Duration::from_secs(120),
    condition: super::condition,
    socket_diagnostic: false,
    socket_plan: &[3, 8, 15, 25],
    burst_bytes: None,
    separate_queues: false,
};

const HIGH: Model = Model {
    case: "h0",
    revision: "workload_kernel_tcp_calibration_high_v1",
    size: 96 * 1024 * 1024,
    rtt_ms: 300,
    mbit: 50,
    min_rtt_us: [270000, 420000],
    goodput_mbit: [40.0, 60.0],
    bulk_limit: Duration::from_secs(240),
    condition: high_condition,
    socket_diagnostic: false,
    socket_plan: &[3, 8, 15, 25],
    burst_bytes: None,
    separate_queues: false,
};

const HIGH_SOCKETS: Model = Model {
    case: "hd0",
    revision: "workload_kernel_tcp_sockets_v1",
    socket_diagnostic: true,
    ..HIGH
};

const HIGH_LONG: Model = Model {
    case: "hl0",
    revision: "workload_kernel_tcp_long_v1",
    size: 384 * 1024 * 1024,
    socket_plan: &[3, 8, 15, 25, 50, 80],
    ..HIGH_SOCKETS
};

const HIGH_BURST_A: Model = Model {
    case: "ha0",
    revision: "workload_kernel_tcp_burst_v1",
    burst_bytes: Some(4096),
    ..HIGH
};

const HIGH_BURST_B: Model = Model {
    case: "hb0",
    burst_bytes: Some(65536),
    ..HIGH_BURST_A
};

const HIGH_QUEUES: Model = Model {
    case: "hq0",
    revision: "workload_kernel_tcp_queues_v1",
    separate_queues: true,
    ..HIGH_BURST_B
};

fn high_condition() -> patchbay::LinkCondition {
    patchbay::LinkCondition::new()
        .latency_ms(150)
        .rate_mbit(50)
        .buffer_ms(40)
}

fn emit(row: Value) {
    eprintln!("[netlab-workload-calibration] {row}");
}

/// The parent owns the only server task, including every failure/early return.
struct Server(tokio::task::JoinHandle<Vec<Result<(), String>>>);

impl Drop for Server {
    fn drop(&mut self) {
        self.0.abort();
    }
}

pub(super) async fn snapshot() -> Value {
    let mut captures = serde_json::Map::new();
    for (name, args) in [
        ("qdisc_json", vec!["-s", "-j", "qdisc", "show"]),
        ("qdisc_text", vec!["-s", "-d", "qdisc", "show"]),
        ("interfaces", vec!["-s", "-j", "link", "show"]),
    ] {
        let command = if name == "interfaces" { "ip" } else { "tc" };
        let row = match tokio::process::Command::new(command)
            .args(args)
            .output()
            .await
        {
            Ok(out) => {
                json!({"ok":out.status.success(),"stdout":String::from_utf8_lossy(&out.stdout),
                "stderr":String::from_utf8_lossy(&out.stderr)})
            }
            Err(error) => json!({"ok":false,"error":error.to_string()}),
        };
        captures.insert(name.into(), row);
    }
    Value::Object(captures)
}

fn qdiscs_present(capture: &Value) -> bool {
    let Some(raw) = capture["qdisc_json"]["stdout"].as_str() else {
        return false;
    };
    let Ok(Value::Array(rows)) = serde_json::from_str::<Value>(raw) else {
        return false;
    };
    ["netem", "tbf"]
        .iter()
        .all(|kind| rows.iter().any(|r| r["kind"] == *kind))
        && ["qdisc_json", "qdisc_text", "interfaces"]
            .iter()
            .all(|key| capture[*key]["ok"] == true)
}

async fn serve(mut stream: TcpStream, payload: &[u8], model: Model) -> Result<(), String> {
    stream.set_nodelay(true).map_err(|e| e.to_string())?;
    let mut mode = [0];
    stream
        .read_exact(&mut mode)
        .await
        .map_err(|e| e.to_string())?;
    stream.write_all(&[1]).await.map_err(|e| e.to_string())?;
    let operation = match mode[0] {
        b'u' => "upload",
        b'd' => "download",
        _ => "echo",
    };
    let local = stream.local_addr().map_err(|e| e.to_string())?;
    let peer = stream.peer_addr().map_err(|e| e.to_string())?;
    let transfer = async {
        match mode[0] {
            b'e' => {
                for _ in 0..WARM + ECHOES {
                    let mut bytes = [0; 64];
                    stream
                        .read_exact(&mut bytes)
                        .await
                        .map_err(|e| e.to_string())?;
                    stream.write_all(&bytes).await.map_err(|e| e.to_string())?;
                }
            }
            b'u' => {
                let mut received = vec![0; model.size];
                stream
                    .read_exact(&mut received)
                    .await
                    .map_err(|e| e.to_string())?;
                // A completion ack means all bytes arrived and were compared.
                stream
                    .write_all(&[u8::from(received == payload)])
                    .await
                    .map_err(|e| e.to_string())?;
                if received != payload {
                    return Err("upload byte mismatch".into());
                }
            }
            b'd' => stream.write_all(payload).await.map_err(|e| e.to_string())?,
            _ => return Err("unexpected calibration mode".into()),
        }
        stream.shutdown().await.map_err(|e| e.to_string())
    };
    sockets::observe(transfer, model, "server", operation, local, peer).await
}

async fn connect(addr: std::net::SocketAddr, mode: u8) -> Result<TcpStream, String> {
    let mut stream = TcpStream::connect(addr).await.map_err(|e| e.to_string())?;
    stream.set_nodelay(true).map_err(|e| e.to_string())?;
    stream.write_all(&[mode]).await.map_err(|e| e.to_string())?;
    let mut ready = [0];
    stream
        .read_exact(&mut ready)
        .await
        .map_err(|e| e.to_string())?;
    if ready != [1] {
        return Err("calibration peer did not confirm its mode".into());
    }
    Ok(stream)
}

async fn client(addr: std::net::SocketAddr, payload: Arc<Vec<u8>>, model: Model) -> Vec<Value> {
    let epoch = Instant::now();
    let mut rows = Vec::new();
    let connected = tokio::time::timeout(CALLER, connect(addr, b'e')).await;
    let (mut stream, connect_error) = match connected {
        Ok(Ok(s)) => (Some(s), None),
        Ok(Err(error)) => (None, Some(error)),
        Err(_) => (None, Some("echo connection deadline".into())),
    };
    emit(
        json!({"schema":1,"revision":model.revision,"event":"connect","operation":"echo",
        "ok":stream.is_some(),"error":connect_error}),
    );
    let mut broken = stream.is_none();
    for index in 0..WARM + ECHOES {
        let started_us = epoch.elapsed().as_micros();
        let mut bytes = [0; 64];
        bytes.fill(index as u8);
        let attempted = async {
            if broken {
                return Err("echo blocked after earlier failure".into());
            }
            let s = stream.as_mut().unwrap();
            s.write_all(&bytes).await.map_err(|e| e.to_string())?;
            let mut answer = [0; 64];
            s.read_exact(&mut answer).await.map_err(|e| e.to_string())?;
            if answer != bytes {
                return Err("echo bytes differ".into());
            }
            Ok::<_, String>(())
        };
        let was_broken = broken;
        let (outcome, error) = match tokio::time::timeout(CALLER, attempted).await {
            Ok(Ok(())) => ("ok", None),
            Ok(Err(e)) => (if was_broken { "blocked" } else { "error" }, Some(e)),
            Err(_) => ("timeout", Some("echo caller deadline".into())),
        };
        broken |= outcome != "ok";
        let ended_us = epoch.elapsed().as_micros();
        let row = json!({"schema":1,"revision":model.revision,"event":"echo","index":index,
            "warmup":index<WARM,"started_us":started_us,"ended_us":ended_us,
            "elapsed_us":ended_us-started_us,"outcome":outcome,"error":error,"bytes":64});
        emit(row.clone());
        rows.push(row);
    }
    drop(stream);
    for (direction, mode) in [("upload", b'u'), ("download", b'd')] {
        let started_us = epoch.elapsed().as_micros();
        let transfer = async {
            let mut s = tokio::time::timeout(CALLER, connect(addr, mode))
                .await
                .map_err(|_| "bulk connect deadline".to_owned())??;
            let local = s.local_addr().map_err(|e| e.to_string())?;
            let peer = s.peer_addr().map_err(|e| e.to_string())?;
            let payload_transfer = async {
                if mode == b'u' {
                    s.write_all(&payload).await.map_err(|e| e.to_string())?;
                    let mut ack = [0];
                    s.read_exact(&mut ack).await.map_err(|e| e.to_string())?;
                    if ack != [1] {
                        return Err("peer rejected upload bytes".into());
                    }
                } else {
                    let mut received = vec![0; model.size];
                    s.read_exact(&mut received)
                        .await
                        .map_err(|e| e.to_string())?;
                    if received != *payload {
                        return Err("download byte mismatch".into());
                    }
                }
                Ok::<_, String>(())
            };
            sockets::observe(payload_transfer, model, "client", direction, local, peer).await
        };
        let (outcome, error) = match tokio::time::timeout(model.bulk_limit, transfer).await {
            Ok(Ok(())) => ("ok", None),
            Ok(Err(e)) => ("error", Some(e)),
            Err(_) => ("timeout", Some("bulk caller deadline".into())),
        };
        let ended_us = epoch.elapsed().as_micros();
        let row = json!({"schema":1,"revision":model.revision,"event":"bulk","direction":direction,
            "started_us":started_us,"ended_us":ended_us,"elapsed_us":ended_us-started_us,
            "bytes":model.size,"exact_bytes":outcome=="ok","outcome":outcome,"error":error});
        emit(row.clone());
        rows.push(row);
    }
    rows
}

async fn on(device: &Device) -> Value {
    device
        .spawn(|_| async { snapshot().await })
        .unwrap()
        .await
        .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
#[ignore = "kernel TCP calibration, alone under the guarded workload calibration task"]
async fn w0_kernel_tcp_confirms_the_workload_bottleneck_in_both_directions() {
    run(LOW).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
#[ignore = "high-BDP kernel calibration, run alone under its fixed guarded task"]
async fn h0_kernel_tcp_confirms_the_high_bdp_bottleneck_in_both_directions() {
    run(HIGH).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
#[ignore = "read-only socket diagnostics under the fixed high-BDP model"]
async fn hd0_high_bdp_tcp_socket_windows_during_both_transfers() {
    run(HIGH_SOCKETS).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
#[ignore = "one extended TCP diagnostic; not ordinary high-profile acceptance"]
async fn hl0_high_bdp_extended_transfer_retains_late_socket_observations() {
    run(HIGH_LONG).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
#[ignore = "fixed burst diagnostic control, run in the guarded ABBA task"]
async fn ha0_high_bdp_tcp_with_four_kib_token_bucket() {
    run(HIGH_BURST_A).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
#[ignore = "fixed burst diagnostic candidate, run in the guarded ABBA task"]
async fn hb0_high_bdp_tcp_with_sixty_four_kib_token_bucket() {
    run(HIGH_BURST_B).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
#[ignore = "forwarding queue topology diagnostic under its fixed guarded task"]
async fn hq0_high_bdp_tcp_with_separate_forwarding_rate_and_delay_queues() {
    run(HIGH_QUEUES).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
#[ignore = "independent paced UDP capacity diagnostic; fixed guarded three fresh cases"]
async fn uq0_independent_datagram_capacity_under_the_high_bdp_model() {
    capacity::run().await;
}

async fn run(model: Model) {
    let lab = Lab::new().await.unwrap();
    let rx = router(&lab, "pubx", Net::Public).await;
    let ry = router(&lab, "puby", Net::Public).await;
    let x = lab.add_device("x").uplink(rx.id()).build().await.unwrap();
    let y = lab.add_device("y").uplink(ry.id()).build().await.unwrap();
    if model.separate_queues {
        queues::configure(&x, &y, &rx, &ry, model).await;
    } else {
        x.default_iface()
            .unwrap()
            .set_condition((model.condition)(), LinkDirection::Both)
            .await
            .unwrap();
    }
    if model.burst_bytes.is_some() && !model.separate_queues {
        let client = x
            .spawn(move |_| async move { burst::apply(model, "x").await })
            .unwrap();
        burst::owned(client).await;
        let gateway = rx
            .spawn(move |_| async move { burst::apply(model, "gateway").await })
            .unwrap();
        burst::owned(gateway).await;
    }
    let mut seed = 0x6d697865646c6f61u64;
    let payload = Arc::new(
        (0..model.size)
            .map(|_| {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                (seed >> 32) as u8
            })
            .collect::<Vec<_>>(),
    );
    emit(
        json!({"schema":1,"revision":model.revision,"event":"manifest","warmup":WARM,"echoes":ECHOES,
        "echo_bytes":64,"bulk_bytes":model.size,"rtt_ms":model.rtt_ms,"mbit_each_direction":model.mbit,"queue_ms":40,
        "min_rtt_us_bounds":model.min_rtt_us,"goodput_mbit_bounds":model.goodput_mbit,
        "bulk_caller_us":model.bulk_limit.as_micros(),
        "socket_diagnostic":model.socket_diagnostic,
        "socket_plan_seconds":model.socket_plan,
        "requested_burst_bytes":model.burst_bytes,
        "separate_queues":model.separate_queues,
        "layer":"kernel_TCP","residents":false,"slo":false,"qdisc_units_review_required":true}),
    );
    let xb = on(&x).await;
    let rb = rx
        .spawn(|_| async { snapshot().await })
        .unwrap()
        .await
        .unwrap();
    for (name, capture) in [("x_before", &xb), ("gateway_before", &rb)] {
        emit(
            json!({"schema":1,"revision":model.revision,"event":"capture","name":name,"capture":capture}),
        );
    }
    let extra_before = queues::extra_captures(&y, &ry, model, "before").await;
    let (tx, ready_rx) = tokio::sync::oneshot::channel();
    let server_payload = payload.clone();
    let mut server = Server(
        y.spawn(move |_| async move {
            let listener = TcpListener::bind(("0.0.0.0", 0)).await.unwrap();
            tx.send(listener.local_addr().unwrap().port()).unwrap();
            let mut outcomes = Vec::new();
            for _ in 0..3 {
                let connection = tokio::time::timeout(model.bulk_limit, listener.accept()).await;
                let outcome = match connection {
                    Ok(Ok((stream, _))) => tokio::time::timeout(
                        model.bulk_limit,
                        serve(stream, &server_payload, model),
                    )
                    .await
                    .map_err(|_| "server transfer deadline".to_owned())
                    .and_then(|r| r),
                    Ok(Err(e)) => Err(e.to_string()),
                    Err(_) => Err("server accept deadline".into()),
                };
                outcomes.push(outcome);
            }
            outcomes
        })
        .unwrap(),
    );
    let port = tokio::time::timeout(CALLER, ready_rx)
        .await
        .unwrap()
        .unwrap();
    let addr = std::net::SocketAddr::from((y.ip().unwrap(), port));
    let rows = x
        .spawn(move |_| async move { client(addr, payload, model).await })
        .unwrap()
        .await
        .unwrap();
    let server_ok = tokio::time::timeout(CALLER, &mut server.0)
        .await
        .is_ok_and(|r| r.is_ok_and(|rows| rows.len() == 3 && rows.iter().all(Result::is_ok)));
    let xa = on(&x).await;
    // Query the gateway from its own namespace as ingress is its veth egress.
    let ra = rx
        .spawn(|_| async { snapshot().await })
        .unwrap()
        .await
        .unwrap();
    for (name, capture) in [("x_after", &xa), ("gateway_after", &ra)] {
        emit(
            json!({"schema":1,"revision":model.revision,"event":"capture","name":name,"capture":capture}),
        );
    }
    let extra_after = queues::extra_captures(&y, &ry, model, "after").await;
    let min_rtt = rows
        .iter()
        .filter(|r| r["event"] == "echo" && r["warmup"] == false && r["outcome"] == "ok")
        .filter_map(|r| r["elapsed_us"].as_u64())
        .min();
    let rates: Vec<f64> = rows
        .iter()
        .filter(|r| r["event"] == "bulk")
        .map(|r| model.size as f64 * 8.0 / r["elapsed_us"].as_u64().unwrap().max(1) as f64)
        .collect();
    let min_rtt_range = model.min_rtt_us[0]..=model.min_rtt_us[1];
    let goodput_range = model.goodput_mbit[0]..=model.goodput_mbit[1];
    let bounds_ok = min_rtt.is_some_and(|v| min_rtt_range.contains(&v))
        && rates.len() == 2
        && rates.iter().all(|v| goodput_range.contains(v));
    let commands_ok = if model.separate_queues {
        queues::valid_capture(&xb, false)
            && queues::valid_capture(&xa, false)
            && queues::valid_capture(&rb, true)
            && queues::valid_capture(&ra, true)
            && extra_before
            && extra_after
    } else {
        [&xb, &rb, &xa, &ra].iter().all(|v| qdiscs_present(v))
    };
    let ok = rows.len() == 70
        && rows.iter().all(|r| r["outcome"] == "ok")
        && server_ok
        && commands_ok
        && bounds_ok;
    emit(
        json!({"schema":1,"revision":model.revision,"event":"complete","functional_and_bounds_ok":ok,
        "samples":rows.len(),"server_ok":server_ok,"qdiscs_present":commands_ok,"bounds_ok":bounds_ok,
        "min_rtt_us":min_rtt,"goodput_mbit":rates,"calibrated":false,"qdisc_units_review_required":true,"slo":false}),
    );
    result(
        model.case,
        ok,
        format!(
            "kernel TCP min_rtt_us={min_rtt:?} rates={rates:?} exact_bytes/server={server_ok} qdiscs_present={commands_ok}; readback unit review still required"
        ),
    );
    assert!(
        ok,
        "{}: complete independent kernel data and frozen physical bounds",
        model.case
    );
}
