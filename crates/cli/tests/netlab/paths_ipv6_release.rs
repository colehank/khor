//! Reuse the unchanged original IPv6 calibration and Node scenarios only.
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime};

use khor_core::Hop;
use patchbay::{Device, IpSupport, Lab, RouterPreset};

use crate::lab::{LabNode, mesh, node, relay, result, sample_diagnostics};
#[path = "paths/mobile.rs"]
mod mobile;
#[path = "paths/mobility.rs"]
mod mobility;

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
