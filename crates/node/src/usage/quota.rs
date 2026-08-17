//! The codex CLI's rate-limit snapshot, read off the newest rollout.
//!
//! # What this answers, and what it deliberately does not
//!
//! It answers "what did the backend serving codex last say about its
//! windows, and who was that backend" — the label travels with the
//! numbers (`khor_core::CodexQuota` says why). It does **not** aggregate,
//! average, or reach across sessions: a rate window is the backend's
//! live bookkeeping, so only the newest session that carries a snapshot
//! is worth reading, with its age attached.
//!
//! # Which way this one can be wrong
//!
//! Over-count has no path: every figure is copied from one line of one
//! file, no arithmetic. Under-report is the open side — a rollout whose
//! `token_count` shape moves stops matching and the answer becomes
//! `None`, which the caller must show as "khor 没读到", never as "no
//! quota exists". Both observed shapes on this machine (official with
//! windows, relay with nulls) are pinned by fixtures; shapes beyond
//! those two are upstream knowledge, untested against a live account.

use std::path::{Path, PathBuf};

use khor_core::{CodexQuota, QuotaWindow};

/// The newest snapshot on this machine, or `None` when no rollout
/// carries one (including: no codex at all).
pub fn codex(home: &Path) -> Option<CodexQuota> {
    let mut files = rollouts(&home.join(".codex").join("sessions"));
    // Newest first. The date is in the path (YYYY/MM/DD dirs and an ISO
    // stamp in the name), so the full path sorts by session start —
    // fresher than mtime, which a resumed old session rewrites.
    files.sort();
    files.reverse();
    files.iter().find_map(|f| snapshot_in(f))
}

fn rollouts(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            out.extend(rollouts(&p));
        } else if p.file_name().is_some_and(|n| {
            let n = n.to_string_lossy();
            n.starts_with("rollout-") && n.ends_with(".jsonl")
        }) {
            out.push(p);
        }
    }
    out
}

/// The **last** `token_count` line of one file, with the session's
/// provider. Last, not first: the backend restates its windows on every
/// turn, and the earlier ones are history.
fn snapshot_in(file: &Path) -> Option<CodexQuota> {
    let text = std::fs::read_to_string(file).ok()?;
    let mut provider = None;
    let mut last: Option<CodexQuota> = None;
    for line in text.lines() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let payload = v.get("payload").unwrap_or(&serde_json::Value::Null);
        if provider.is_none() {
            provider = payload
                .get("model_provider")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned);
        }
        // Two shapes on disk, measured 2026-08-17: the 2025-era format
        // nests the key at `payload.info.rate_limits`; the 2026 format
        // writes `payload.rate_limits` beside `info`. Both are real
        // files on this machine, and reading only one of them made the
        // reader answer None on the whole tree once.
        let Some(limits) = payload
            .get("rate_limits")
            .or_else(|| payload.get("info").and_then(|i| i.get("rate_limits")))
        else {
            continue;
        };
        let Some(at_ms) = v
            .get("timestamp")
            .and_then(serde_json::Value::as_str)
            .and_then(|t| t.parse::<jiff::Timestamp>().ok())
            .map(|t| t.as_millisecond() as u64)
        else {
            continue;
        };
        last = Some(CodexQuota {
            provider: provider.clone(),
            at_ms,
            primary: window(limits.get("primary")),
            secondary: window(limits.get("secondary")),
        });
    }
    last
}

/// One window, or `None` for the relay's explicit `null` as much as for
/// an absent key — both are "this backend does not say".
fn window(v: Option<&serde_json::Value>) -> Option<QuotaWindow> {
    let v = v?;
    Some(QuotaWindow {
        used_percent: v.get("used_percent")?.as_f64()?,
        window_minutes: v.get("window_minutes")?.as_u64()?,
        // Seconds on disk, milliseconds on the type — the one unit
        // conversion here, done once at the edge.
        resets_at_ms: v.get("resets_at").and_then(serde_json::Value::as_u64).map(|s| s * 1000),
    })
}
