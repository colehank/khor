//! Independent datagram capacity diagnostic, never a TCP/Node acceptance shortcut.
use std::time::Duration;

use patchbay::{Device, Lab, Router};
use serde_json::{Value, json};

use crate::lab::result;
use crate::nat::{Net, router};

use super::{HIGH_QUEUES, Model, datagrams, emit, queues};

const REVISION: &str = "workload_kernel_udp_capacity_v2";

struct Owned<T>(tokio::task::JoinHandle<T>);

impl<T> Drop for Owned<T> {
    fn drop(&mut self) {
        self.0.abort();
    }
}

pub(super) async fn capture() -> Value {
    let mut rows = serde_json::Map::new();
    for (name, program, args) in [
        ("qdisc_json", "tc", vec!["-s", "-j", "qdisc", "show"]),
        ("qdisc_text", "tc", vec!["-s", "-d", "qdisc", "show"]),
        ("interfaces", "ip", vec!["-s", "-j", "link", "show"]),
    ] {
        let mut command = tokio::process::Command::new(program);
        command.kill_on_drop(true).args(args);
        let value = match tokio::time::timeout(Duration::from_secs(2), command.output()).await {
            Ok(Ok(out)) => json!({"ok":out.status.success(),"timed_out":false,
                "stdout":String::from_utf8_lossy(&out.stdout),"stderr":String::from_utf8_lossy(&out.stderr)}),
            Ok(Err(error)) => json!({"ok":false,"timed_out":false,"error":error.to_string()}),
            Err(_) => json!({"ok":false,"timed_out":true,"error":"capacity snapshot deadline"}),
        };
        rows.insert(name.into(), value);
    }
    Value::Object(rows)
}

async fn snapshots(x: &Device, y: &Device, rx: &Router, ry: &Router, when: &str) -> bool {
    let mut a = Owned(x.spawn(|_| async { capture().await }).unwrap());
    let mut b = Owned(y.spawn(|_| async { capture().await }).unwrap());
    let mut c = Owned(rx.spawn(|_| async { capture().await }).unwrap());
    let mut d = Owned(ry.spawn(|_| async { capture().await }).unwrap());
    let (a, b, c, d) = tokio::join!(&mut a.0, &mut b.0, &mut c.0, &mut d.0);
    let mut valid = true;
    for (side, capture, router) in [
        ("x", a.unwrap(), false),
        ("y", b.unwrap(), false),
        ("gateway", c.unwrap(), true),
        ("right_gateway", d.unwrap(), true),
    ] {
        valid &= queues::valid_capture(&capture, router);
        emit(json!({"schema":1,"revision":REVISION,"event":"capture",
            "name":format!("{side}_{when}"),"capture":capture}));
    }
    valid
}

async fn direction(sender: &Device, receiver: &Device, name: &str, tag: u64) -> (bool, bool) {
    let source = sender.ip().unwrap();
    let destination = receiver.ip().unwrap();
    let (ready, port) = tokio::sync::oneshot::channel();
    let mut sink = Owned(
        receiver
            .spawn(
                move |_| async move { datagrams::receive(destination, source, tag, ready).await },
            )
            .unwrap(),
    );
    let peer = tokio::time::timeout(Duration::from_secs(2), port)
        .await
        .expect("UDP receiver ready deadline")
        .expect("UDP receiver bound");
    let mut producer = Owned(
        sender
            .spawn(move |_| async move { datagrams::send(source, peer, tag).await })
            .unwrap(),
    );
    let (sent, received) = tokio::time::timeout(Duration::from_secs(28), async {
        tokio::join!(&mut producer.0, &mut sink.0)
    })
    .await
    .expect("one total direction deadline");
    let sent = sent.expect("UDP sender owner");
    let received = received.expect("UDP receiver owner");
    let valid = datagrams::valid(&sent, &received);
    let sent_count = datagrams::population(&sent.sent);
    let unique = datagrams::population(&received.received);
    let rate = datagrams::rate(&received.by_second);
    let capacity = (40.0..=60.0).contains(&rate);
    emit(
        json!({"schema":1,"revision":REVISION,"event":"udp_direction","direction":name,
        "tag":tag,"sender":sent.record(),"receiver":received.record(),"observations_ok":valid,
        "sent_packets":sent_count,"unique_packets":unique,"missing_packets":sent_count.saturating_sub(unique),
        "unique_payload_bytes":unique*datagrams::SIZE as u64,
        "receiver_steady_mbit":rate,"capacity_bounds_ok":capacity}),
    );
    (valid, capacity)
}

pub(super) async fn run() {
    let model = Model {
        case: "uq0",
        revision: REVISION,
        ..HIGH_QUEUES
    };
    let lab = Lab::new().await.unwrap();
    let rx = router(&lab, "pubx", Net::Public).await;
    let ry = router(&lab, "puby", Net::Public).await;
    let x = lab.add_device("x").uplink(rx.id()).build().await.unwrap();
    let y = lab.add_device("y").uplink(ry.id()).build().await.unwrap();
    queues::configure(&x, &y, &rx, &ry, model).await;
    emit(
        json!({"schema":1,"revision":REVISION,"event":"udp_manifest",
        "layout":"forwarding_split","payload_bytes":datagrams::SIZE,"offered_bit_per_sec":datagrams::RATE,
        "planned_packets":datagrams::PACKETS,"send_seconds":20,"receive_seconds":25,
        "ready_limit_seconds":2,"direction_limit_seconds":28,"steady_seconds":[5,15],
        "rtt_ms":300,"mbit_each_direction":50,"queue_ms":40,"burst_bytes":65536,
        "capacity_mbit_bounds":[40,60],"sender_steady_mbit_bounds":[65,75],
        "receiver_first_limit_us":2000000,"pacer_tick_us":datagrams::TICK_US,"pacer_max_lateness_us":20000,
        "pacer_late_10ms_percent_limit":1,"pacer_skip_percent_limit":1,
        "layer":"kernel_UDP","residents":false,"slo":false,"physical_calibration_accepted":false}),
    );
    let before = snapshots(&x, &y, &rx, &ry, "before").await;
    let upload = direction(&x, &y, "upload", 0x7564705f75705f31).await;
    let download = direction(&y, &x, "download", 0x7564705f646e5f31).await;
    let after = snapshots(&x, &y, &rx, &ry, "after").await;
    let valid = before && after && upload.0 && download.0;
    let capacity = upload.1 && download.1;
    emit(
        json!({"schema":1,"revision":REVISION,"event":"udp_complete",
        "observations_ok":valid,"capacity_bounds_ok":capacity,
        "physical_calibration_accepted":false,"full_N3_accepted":false,"slo":false}),
    );
    result(
        "uq0",
        valid && capacity,
        format!("UDP population/pacer={valid} capacity_bounds={capacity}; diagnostic only"),
    );
    assert!(
        valid && capacity,
        "uq0: independent datagram observations and capacity bounds"
    );
}
