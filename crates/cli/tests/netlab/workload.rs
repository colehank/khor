//! Actual resident directory reads, a remote PTY, and a file pull together.
//! This records a baseline, not a latency SLO or a queue-policy comparison.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use khor_core::Hop;
use khor_node::{Node, host};
use patchbay::{Lab, LinkCondition};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::lab::{mesh, node, relay, result};
use crate::nat::{Net, router};

#[path = "workload/calibration.rs"]
mod calibration;
#[path = "workload/observer.rs"]
mod observer;
use observer::{Log, Mode, Owned};
#[path = "workload/profile.rs"]
mod profile;
use profile::{HIGH, LOW, Spec};

/// Shared by the resident workload and its independent kernel TCP calibration.
fn condition() -> LinkCondition {
    LinkCondition::new()
        .latency_ms(20)
        .rate_mbit(8)
        .buffer_ms(40)
}

const REVISION: &str = "resident_mixed_workload_v4";
const SAMPLES: usize = 120;
const PERIOD: Duration = Duration::from_millis(100);
// Measurement bounds, not user-approved UX targets. Failed/blocked samples
// remain in the population; a small successful subset cannot pass this test.
const CALLER: Duration = Duration::from_secs(2);
const BYTES: usize = 16 * 1024 * 1024;

fn emit(value: Value) {
    eprintln!("[netlab-workload] {value}");
}

fn at(epoch: Instant) -> u64 {
    epoch.elapsed().as_micros().try_into().unwrap()
}

fn framed(op: &host::ClientOp) -> Vec<u8> {
    let mut bytes = Vec::new();
    host::write_frame(&mut bytes, op).unwrap();
    bytes
}

/// The expected prefix is absent from the input. Even if terminal echo is
/// accidentally on, an echoed input cannot impersonate the shell's response.
async fn echo(stream: &mut TcpStream, token: &str) -> Result<(), String> {
    let frame = framed(&host::ClientOp::Input(khor_node::shell_start::input(token)));
    stream.write_all(&frame).await.map_err(|e| e.to_string())?;
    let expected = format!("NETLAB-REPLY-{token}");
    output_until(stream, &expected, token == "warmup-unique").await
}

async fn output_until(stream: &mut TcpStream, expected: &str, trace: bool) -> Result<(), String> {
    let mut received = Vec::new();
    loop {
        let mut bytes = [0; 4096];
        let n = stream.read(&mut bytes).await.map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("terminal stream ended before its response".into());
        }
        received.extend_from_slice(&bytes[..n]);
        if trace && received.len() <= 2048 {
            eprintln!(
                "[netlab-workload-startup] bytes={:?}",
                String::from_utf8_lossy(&bytes[..n])
            );
        }
        if received
            .windows(expected.len())
            .any(|b| b == expected.as_bytes())
        {
            return Ok(());
        }
        if received.len() > 1024 * 1024 {
            return Err("terminal response exceeded the replay bound".into());
        }
    }
}

async fn directory(
    node: &Node,
    path: &str,
    phase_start: Instant,
    log: &Log,
    spec: Spec,
) -> Vec<Value> {
    let mut rows = Vec::with_capacity(SAMPLES);
    for index in 0..SAMPLES {
        let scheduled = phase_start + spec.period * index as u32;
        tokio::time::sleep_until(scheduled.into()).await;
        let attempt = log.start("directory", index, scheduled);
        let (outcome, error) = match tokio::time::timeout(CALLER, node.ls_of("y", path)).await {
            Ok(Ok((_, entries, false)))
                if entries
                    .iter()
                    .any(|e| e.name == "payload.bin" && e.size == spec.bytes as u64) =>
            {
                ("ok", None)
            }
            Ok(Ok(_)) => ("error", Some("directory omitted the known payload".into())),
            Ok(Err(e)) => ("error", Some(e)),
            Err(_) => ("timeout", Some("directory caller deadline".into())),
        };
        rows.push(attempt.finish(outcome, error));
    }
    rows
}

async fn terminal(
    stream: &mut TcpStream,
    phase_start: Instant,
    phase: &str,
    log: &Log,
    spec: Spec,
) -> Vec<Value> {
    let mut rows = Vec::with_capacity(SAMPLES);
    let mut broken = false;
    for index in 0..SAMPLES {
        let scheduled = phase_start + spec.period * index as u32;
        tokio::time::sleep_until(scheduled.into()).await;
        let attempt = log.start("terminal", index, scheduled);
        let (outcome, error) = if broken {
            (
                "blocked",
                Some("terminal failed earlier; no implicit reattach".into()),
            )
        } else {
            match tokio::time::timeout(CALLER, echo(stream, &format!("{phase}-{index:04}"))).await {
                Ok(Ok(())) => ("ok", None),
                Ok(Err(e)) => {
                    broken = true;
                    ("error", Some(e))
                }
                Err(_) => {
                    broken = true;
                    ("timeout", Some("terminal caller deadline".into()))
                }
            }
        };
        rows.push(attempt.finish(outcome, error));
    }
    rows
}

async fn phase(
    node: &Arc<Node>,
    stream: &mut TcpStream,
    path: &str,
    landed: &std::path::Path,
    epoch: Instant,
    name: &str,
    bulk: bool,
    mode: Mode,
    spec: Spec,
) -> (Vec<Value>, Value, Option<PathBuf>) {
    // Snapshots are outside the sample window, on this resident's actual IPC.
    let before = node.wire_counts().await.map_err(|e| format!("{e:?}"));
    let phase_start = Instant::now();
    let log = Log::with_revision(epoch, name, mode, spec.revision);
    emit(
        json!({"schema":1,"revision":spec.revision,"event":"phase_started","phase":name,"mode":mode.name(),"at_us":at(epoch),"wire_counts":before}),
    );
    let pull = async {
        if !bulk {
            return (Value::Null, None);
        }
        let started_us = at(epoch);
        let file = format!("{path}/payload.bin");
        let reply = tokio::time::timeout(spec.bulk_limit, node.pull_path("y", &file, landed)).await;
        let ended_us = at(epoch);
        let (outcome, error, moved, dest) = match reply {
            Ok(Ok((moved, dest))) => ("ok", None, Some(moved), Some(dest)),
            Ok(Err(e)) => ("error", Some(e), None, None),
            Err(_) => ("timeout", Some("bulk caller deadline".into()), None, None),
        };
        let row = json!({"schema":1,"revision":spec.revision,"event":"bulk","phase":name,"mode":mode.name(),
            "started_us":started_us,"ended_us":ended_us,"moved":moved,"expected":spec.bytes,
            "outcome":outcome,"error":error,"caller_ms":spec.bulk_limit.as_millis()});
        log.push(row.clone());
        (row, dest)
    };
    // All three futures are borrowed by this parent: no detached measurement
    // tasks or extra endpoints sharing the resident's identity.
    let measured = async {
        tokio::join!(
            directory(node, path, phase_start, &log, spec),
            terminal(stream, phase_start, name, &log, spec),
            pull,
        )
    };
    let ((mut directories, terminals, (bulk_row, dest)), timing) =
        observer::probe_with_revision(epoch, phase_start, name, mode, spec.revision, measured)
            .await;
    let flush_started = Instant::now();
    let (_, overflow) = log.finish();
    let record_flush_us = flush_started.elapsed().as_micros();
    let probe_capped = timing["capped"] == true;
    emit(timing);
    assert!(
        !overflow && !probe_capped,
        "bounded observer retained the entire phase"
    );
    directories.extend(terminals);
    emit(
        json!({"schema":1,"revision":spec.revision,"event":"phase_finished","phase":name,"mode":mode.name(),"logger_overflow":overflow,"probe_capped":probe_capped,"record_flush_us":record_flush_us,
        "at_us":at(epoch),"wire_counts":node.wire_counts().await.map_err(|e|format!("{e:?}"))}),
    );
    (directories, bulk_row, dest)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
#[ignore = "real resident workload baseline; run alone through the guarded netlab task"]
async fn w1_real_terminal_and_directory_during_a_resident_file_pull() {
    run(Mode::Streaming, LOW).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
#[ignore = "paired buffered observer mode; guarded workload observer task"]
async fn w2_buffered_terminal_and_directory_during_a_resident_file_pull() {
    run(Mode::Buffered, LOW).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
#[ignore = "high-BDP actual resident workload; fixed preflight before policy comparison"]
async fn wh2_high_bdp_buffered_directory_terminal_and_complete_file() {
    run(Mode::Buffered, HIGH).await;
}

async fn run(mode: Mode, spec: Spec) {
    let s = if spec.high {
        "wh2-buffer"
    } else if mode == Mode::Streaming {
        "w1-stream"
    } else {
        "w2-buffer"
    };
    let lab = Lab::new().await.unwrap();
    let r = relay(&lab, s, "relay", None).await;
    let rx = router(&lab, "pubx", Net::Public).await;
    let ry = router(&lab, "puby", Net::Public).await;
    let dx = lab.add_device("x").uplink(rx.id()).build().await.unwrap();
    let dy = lab.add_device("y").uplink(ry.id()).build().await.unwrap();
    // The high profile is a cold fixed model, not a change from an unshaped
    // resident. Keep the original low-profile ordering for its regressions.
    if spec.high {
        profile::configure(&dx, &dy, &rx, &ry, spec).await;
    }
    let x = node(s, dx, "x", &[&r], &[("SHELL", "/bin/sh")]).await;
    let y = node(s, dy, "y", &[&r], &[("SHELL", "/bin/sh")]).await;
    mesh(&[&x, &y]).await;
    assert!(
        x.until(&y, Duration::from_secs(60), |l| l.hop == Hop::Direct)
            .await
            .is_some()
    );
    let source = y.home.0.join("workload");
    let landed = x.home.0.join("landed");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::create_dir_all(&landed).unwrap();
    let mut seed = 0x6d697865646c6f61u64;
    let payload: Vec<u8> = (0..spec.bytes)
        .map(|_| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed >> 32) as u8
        })
        .collect();
    std::fs::write(source.join("payload.bin"), &payload).unwrap();
    if !spec.high {
        profile::configure(&x.dev, &y.dev, &rx, &ry, spec).await;
    }
    let path = source.display().to_string();
    let cwd = path.clone();
    let capture_before = profile::captures(&x.dev, &y.dev, &rx, &ry, spec, mode, "before").await;
    let node = x.with(|node| async move { node }).await;
    let mut benchmark = Owned(x.dev.spawn(move |_| async move {
        // Match the existing real-terminal integration entry (term_echo.rs).
        // CLI _host currently omits KHOR_HOST_STARTUP; its separate startup
        // API must not be a prerequisite for measuring a real PTY's network.
        let program = vec!["/bin/sh".into(), "-c".into(),
            "stty -echo; printf 'NETLAB-ECHO-%s\\n' READY; while IFS= read -r line; do printf 'NETLAB-REPLY-%s\\n' \"$line\"; done".into()];
        let id = node.open_on("y", "shell", "netlab echo", &cwd, &program, (100, 30),
            Some("netlab-resident-mixed-workload-v3")).await.expect("a scratch remote PTY");
        let (addr, cookie) = node.reach("y", &id).await.expect("the resident's terminal pipe");
        let socket = tokio::task::spawn_blocking(move || host::connect_terminal(&addr, &cookie, 100, 30))
            .await.unwrap().expect("current terminal protocol handshake");
        socket.set_nonblocking(true).unwrap();
        let mut stream = TcpStream::from_std(socket).unwrap();
        let ready_by = tokio::time::Instant::now() + Duration::from_secs(10);
        tokio::time::timeout_at(ready_by, async {
            stream.write_all(&framed(&host::ClientOp::Ready)).await.map_err(|e| e.to_string())?;
            // Wait for the actual program, including initial model replay.
            // Still one total 10s deadline, and no input echo can match it.
            output_until(&mut stream, "NETLAB-ECHO-READY", true).await?;
            echo(&mut stream, "warmup-unique").await
        }).await.expect("bounded PTY startup").expect("a real shell response, not input echo");
        let epoch = Instant::now();
        emit(json!({"schema":1,"revision":spec.revision,"event":"manifest","mode":mode.name(),"probe_period_us":20000,"logger_capacity":512,"samples_per_operation_per_phase":SAMPLES,
            "phases":["idle_before","bulk","idle_after"],"operations":["directory","terminal"],
            "period_us":spec.period.as_micros(),"caller_us":CALLER.as_micros(),"bulk_bytes":spec.bytes,
            "nominal_rtt_ms":if spec.high {300} else {40},"mbit_per_direction":if spec.high {50} else {8},"queue_ms":40,"random_loss_pct":0,
            "connection_layout":"directory_and_fetch_Main__terminal_Tunnel","slo":false,
            "network_profile":if spec.high {"forwarding_split_300ms_50mbit"} else {"combined_40ms_8mbit"},
            "impairment_installed_before_residents":spec.high,
            "calibrated":false,"seed":"6d697865646c6f61","diagnostic_recording":true}));
        let (mut rows, _, _) = phase(&node, &mut stream, &path, &landed, epoch, "idle_before", false, mode, spec).await;
        let (during, bulk, destination) = phase(&node, &mut stream, &path, &landed, epoch, "bulk", true, mode, spec).await;
        rows.extend(during);
        let (after, _, _) = phase(&node, &mut stream, &path, &landed, epoch, "idle_after", false, mode, spec).await;
        rows.extend(after);
        drop(stream);
        let closed = tokio::time::timeout(Duration::from_secs(10), node.close_on("y", &id)).await.is_ok_and(|r|r.is_ok());
        (rows, bulk, destination, closed)
    }).unwrap());
    let (rows, bulk, destination, closed) =
        (&mut benchmark.0).await.expect("owned workload completes");
    let capture_after = profile::captures(&x.dev, &y.dev, &rx, &ry, spec, mode, "after").await;
    let exact_bytes = destination
        .as_ref()
        .and_then(|p| std::fs::read(p).ok())
        .is_some_and(|bytes| bytes == payload);
    let overlap = |operation: &str| {
        rows.iter()
            .filter(|row| {
                row["phase"] == "bulk"
                    && row["operation"] == operation
                    && row["started_us"].as_u64().unwrap() >= bulk["started_us"].as_u64().unwrap()
                    && row["ended_us"].as_u64().unwrap() <= bulk["ended_us"].as_u64().unwrap()
            })
            .count()
    };
    let all_answers = rows.len() == SAMPLES * 6 && rows.iter().all(|row| row["outcome"] == "ok");
    let direct_after = x
        .link(&y)
        .await
        .is_some_and(|l| l.hop == Hop::Direct && l.down_ms.is_none());
    let ok = capture_before
        && capture_after
        && all_answers
        && exact_bytes
        && bulk["moved"] == spec.bytes as u64
        && closed
        && direct_after
        && overlap("directory") >= 30
        && overlap("terminal") >= 30;
    emit(
        json!({"schema":1,"revision":spec.revision,"event":"complete","mode":mode.name(),"functional_ok":ok,
        "profile_readback_commands_ok":capture_before && capture_after,
        "samples":rows.len(),"all_answers":all_answers,"exact_bytes":exact_bytes,"closed":closed,
        "direct_after":direct_after,"directory_overlap":overlap("directory"),"terminal_overlap":overlap("terminal"),
        "bulk":bulk,"calibrated":false,"slo":false,"p99_claimed":false,
        "x_journal":x.journal(Some(&y)),"y_journal":y.journal(Some(&x))}),
    );
    // Lifecycle runs are a separate diagnostic cohort. Preserve both actual
    // residents' stream facts after measurement and before scratch cleanup.
    crate::lab::sample_diagnostics(s, &x);
    crate::lab::sample_diagnostics(s, &y);
    result(
        s,
        ok,
        format!(
            "real PTY+directory samples={} exact_bytes={exact_bytes} overlap={}/{} closed={closed}; uncalibrated diagnostic baseline, not a latency SLO",
            rows.len(),
            overlap("directory"),
            overlap("terminal")
        ),
    );
    assert!(
        ok,
        "w1: all logical outcomes, actual bulk overlap, intact payload and scratch PTY cleanup"
    );
}
