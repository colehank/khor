//! Forwarded propagation and rate queues on distinct router interfaces.
use patchbay::{Device, LinkCondition, LinkDirection, Router};
use serde_json::{Value, json};

use super::{Model, burst, emit, on, snapshot};

pub(super) async fn configure(x: &Device, y: &Device, rx: &Router, ry: &Router, model: Model) {
    for endpoint in [x, y] {
        endpoint
            .default_iface()
            .unwrap()
            .set_condition(LinkCondition::new().latency_ms(150), LinkDirection::Ingress)
            .await
            .unwrap();
    }
    for (router, side) in [(rx, "gateway"), (ry, "right_gateway")] {
        let ip = router.uplink_ip().unwrap();
        let wan: Vec<_> = router
            .interfaces()
            .into_iter()
            .filter(|iface| iface.ip() == Some(ip))
            .collect();
        assert_eq!(wan.len(), 1);
        let name = wan[0].name().to_owned();
        let task = router
            .spawn(move |_| async move {
                let before = burst::command(&["-j".into(), "qdisc".into(), "show".into()]).await;
                assert_eq!(before["ok"], true, "initial router qdiscs: {before}");
                let rows: Vec<Value> =
                    serde_json::from_str(before["stdout"].as_str().unwrap()).unwrap();
                assert!(!rows.iter().any(|r| r["kind"] == "tbf"));
                assert_eq!(rows.iter().filter(|r| r["kind"] == "netem").count(), 1);
                assert!(
                    rows.iter().any(|r| {
                        r["dev"] == name && r["kind"] == "noqueue" && r["root"] == true
                    })
                );
                let args: Vec<String> = [
                    "qdisc", "replace", "dev", &name, "root", "handle", "10:", "tbf", "rate",
                    "50mbit", "burst", "65536", "latency", "40ms",
                ]
                .into_iter()
                .map(str::to_owned)
                .collect();
                let installed = burst::command(&args).await;
                emit(
                    json!({"schema":1,"revision":model.revision,"event":"rate_install",
                    "side":side,"device":name,"uplink_ip":ip.to_string(),"before":before,
                    "args":args,"command":installed}),
                );
                assert_eq!(installed["ok"], true, "root rate queue: {installed}");
            })
            .unwrap();
        burst::owned(task).await;
    }
}

pub(super) fn valid_capture(capture: &Value, router: bool) -> bool {
    if !["qdisc_json", "qdisc_text", "interfaces"]
        .iter()
        .all(|key| capture[*key]["ok"] == true)
    {
        return false;
    }
    let Some(raw) = capture["qdisc_json"]["stdout"].as_str() else {
        return false;
    };
    let Ok(rows) = serde_json::from_str::<Vec<Value>>(raw) else {
        return false;
    };
    let rates: Vec<_> = rows.iter().filter(|r| r["kind"] == "tbf").collect();
    let delays: Vec<_> = rows.iter().filter(|r| r["kind"] == "netem").collect();
    if !router {
        return rates.is_empty() && delays.is_empty();
    }
    rates.len() == 1
        && delays.len() == 1
        && rates[0]["root"] == true
        && delays[0]["root"] == true
        && rates[0]["dev"] != delays[0]["dev"]
}

pub(super) async fn extra_captures(y: &Device, ry: &Router, model: Model, when: &str) -> bool {
    if !model.separate_queues {
        return true;
    }
    let endpoint = on(y).await;
    let router = ry
        .spawn(|_| async { snapshot().await })
        .unwrap()
        .await
        .unwrap();
    for (side, capture) in [("y", &endpoint), ("right_gateway", &router)] {
        emit(
            json!({"schema":1,"revision":model.revision,"event":"capture",
            "name":format!("{side}_{when}"),"capture":capture}),
        );
    }
    valid_capture(&endpoint, false) && valid_capture(&router, true)
}
