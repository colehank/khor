//! Fixed fixture inputs; no product settings or environment-selected models.
use std::time::Duration;

use patchbay::{Device, LinkDirection, Router};

use super::observer::{Mode, Owned};
use super::{BYTES, PERIOD, REVISION, calibration, condition, emit};

#[derive(Clone, Copy)]
pub(super) struct Spec {
    pub revision: &'static str,
    pub bytes: usize,
    pub period: Duration,
    pub bulk_limit: Duration,
    pub high: bool,
}

pub(super) const LOW: Spec = Spec {
    revision: REVISION,
    bytes: BYTES,
    period: PERIOD,
    bulk_limit: Duration::from_secs(120),
    high: false,
};

pub(super) const HIGH: Spec = Spec {
    revision: "resident_mixed_workload_high_v3",
    bytes: 192 * 1024 * 1024,
    period: Duration::from_millis(400),
    bulk_limit: Duration::from_secs(240),
    high: true,
};

pub(super) async fn configure(x: &Device, y: &Device, rx: &Router, ry: &Router, spec: Spec) {
    if spec.high {
        calibration::configure_high(x, y, rx, ry, spec.revision).await;
    } else {
        x.default_iface()
            .unwrap()
            .set_condition(condition(), LinkDirection::Both)
            .await
            .unwrap();
    }
}

pub(super) async fn captures(
    x: &Device,
    y: &Device,
    rx: &Router,
    ry: &Router,
    spec: Spec,
    mode: Mode,
    when: &str,
) -> bool {
    let mut a = Owned(
        x.spawn(|_| async { calibration::bounded_snapshot().await })
            .unwrap(),
    );
    let mut b = Owned(
        rx.spawn(|_| async { calibration::bounded_snapshot().await })
            .unwrap(),
    );
    let (a, b) = tokio::join!(&mut a.0, &mut b.0);
    let mut captures = vec![("x", a.unwrap(), false), ("gateway", b.unwrap(), true)];
    if spec.high {
        let mut c = Owned(
            y.spawn(|_| async { calibration::bounded_snapshot().await })
                .unwrap(),
        );
        let mut d = Owned(
            ry.spawn(|_| async { calibration::bounded_snapshot().await })
                .unwrap(),
        );
        let (c, d) = tokio::join!(&mut c.0, &mut d.0);
        captures.extend([
            ("y", c.unwrap(), false),
            ("right_gateway", d.unwrap(), true),
        ]);
    }
    let mut valid = true;
    for (side, capture, router) in captures {
        valid &= ["qdisc_json", "qdisc_text", "interfaces"]
            .iter()
            .all(|key| capture[*key]["ok"] == true);
        if spec.high {
            valid &= calibration::split_snapshot_valid(&capture, router);
        }
        emit(
            serde_json::json!({"schema":1,"revision":spec.revision,"event":"capture",
            "mode":mode.name(),"name":format!("{side}_{when}"),"capture":capture}),
        );
    }
    valid
}
