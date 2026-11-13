//! One diagnostic knob, changed only inside this fresh lab's namespaces.
//! Keeping latency fixed also changes TBF's derived FIFO limit; record that
//! coupling rather than claiming to isolate token timing from queue capacity.
use std::time::Duration;

use serde_json::{Value, json};

use super::{Model, emit};

struct Owner(tokio::task::AbortHandle);

impl Drop for Owner {
    fn drop(&mut self) {
        self.0.abort();
    }
}

pub(super) async fn command(args: &[String]) -> Value {
    let mut command = tokio::process::Command::new("tc");
    command.kill_on_drop(true).args(args);
    match tokio::time::timeout(Duration::from_secs(2), command.output()).await {
        Ok(Ok(out)) => json!({"ok":out.status.success(),"timed_out":false,
            "stdout":String::from_utf8_lossy(&out.stdout),"stderr":String::from_utf8_lossy(&out.stderr)}),
        Ok(Err(error)) => json!({"ok":false,"timed_out":false,"error":error.to_string()}),
        Err(_) => json!({"ok":false,"timed_out":true,"error":"tc diagnostic command deadline"}),
    }
}

pub(super) async fn apply(model: Model, side: &'static str) {
    let before = command(&["-j".into(), "qdisc".into(), "show".into()]).await;
    assert_eq!(before["ok"], true, "initial tc readback: {before}");
    let rows: Vec<Value> = serde_json::from_str(before["stdout"].as_str().unwrap()).unwrap();
    let rates: Vec<_> = rows.iter().filter(|r| r["kind"] == "tbf").collect();
    let nets: Vec<_> = rows.iter().filter(|r| r["kind"] == "netem").collect();
    assert_eq!(rates.len(), 1);
    assert_eq!(nets.len(), 1);
    let (rate, net) = (rates[0], nets[0]);
    assert_eq!(rate["parent"], "1:1");
    assert_eq!(rate["handle"], "10:");
    assert_eq!(rate["options"]["rate"], 6250000);
    assert_eq!(rate["options"]["lat"], 40000);
    assert!((4080..=4096).contains(&rate["options"]["burst"].as_u64().unwrap()));
    assert_eq!(net["root"], true);
    assert_eq!(net["handle"], "1:");
    assert_eq!(net["dev"], rate["dev"]);
    assert_eq!(net["options"]["limit"], 1000);
    assert_eq!(
        net["options"]["delay"],
        json!({"delay":0.15,"jitter":0,"correlation":0})
    );
    let dev = rate["dev"].as_str().unwrap();
    let requested = model.burst_bytes.unwrap();
    let args: Vec<String> = [
        "qdisc",
        "change",
        "dev",
        dev,
        "parent",
        "1:1",
        "handle",
        "10:",
        "tbf",
        "rate",
        "50mbit",
        "burst",
        &requested.to_string(),
        "latency",
        "40ms",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    let changed = command(&args).await;
    emit(
        json!({"schema":1,"revision":model.revision,"event":"shaper_change",
        "side":side,"device":dev,"requested_burst_bytes":requested,
        "latency_us":40000,"derived_fifo_limit_changes":true,
        "before":before,"args":args,"command":changed}),
    );
    assert_eq!(changed["ok"], true, "tc diagnostic change: {changed}");
}

pub(super) async fn owned(task: tokio::task::JoinHandle<()>) {
    let _owner = Owner(task.abort_handle());
    task.await.unwrap();
}
