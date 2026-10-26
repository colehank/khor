//! Durable, bounded cancellation of known children. No resident means no
//! progress guarantee; the next normal entry resumes the private pending set.
use crate::{
    Node,
    agent_process::Control,
    delegation::{self, Binding, BoundAction, protocol},
    mcp_worker::{Operation, Request, Worker},
};
use khor_core::DeviceId;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    collections::{HashMap, VecDeque},
    fs::{self, File, OpenOptions},
    io::Read,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
    time::{Duration, Instant},
};

#[path = "remote_cancel_issues.rs"]
mod issues;

const RUN_LIMIT: usize = 256;
pub(crate) const CHILD_LIMIT: usize = 256;
const PAGE: usize = 32;
const SMALL: u64 = 16 * 1024;
const BUDGET: Duration = Duration::from_secs(5);

#[derive(Clone, Serialize, Deserialize)]
pub(crate) enum Metadata {
    Persist { run: String, reason: String },
    Status { run: String },
    Discover { after: Option<String> },
}
#[derive(Clone, Serialize, Deserialize)]
struct Intent {
    version: u32,
    intent_id: String,
    run: String,
    execution_home: DeviceId,
    requested_at_ms: i64,
    reason: String,
}
#[derive(Default, Serialize, Deserialize)]
struct Progress {
    child_cursor: Option<String>,
    revision: u64,
    scan_cursor: Option<String>,
    scan_complete: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum Phase {
    Queued,
    InFlight,
    OutcomeUnknown,
    AckPending,
    ConfirmedCanceled,
    TerminalOther(protocol::TaskState),
    NeedsAttention(String),
}
impl Phase {
    fn active(&self) -> bool {
        matches!(
            self,
            Self::Queued | Self::InFlight | Self::OutcomeUnknown | Self::AckPending
        )
    }
}
#[derive(Clone, Serialize, Deserialize)]
struct Observation {
    task: String,
    target_home: DeviceId,
    caller: DeviceId,
    agent: String,
    task_state: protocol::TaskState,
    revision: u64,
    observed_at_ms: i64,
}
#[derive(Clone, Serialize, Deserialize)]
struct ChildReceipt {
    version: u32,
    intent_id: String,
    child_key: String,
    binding: Binding,
    task: Option<String>,
    phase: Phase,
    attempt_no: u64,
    attempt_id: String,
    next_retry_at_ms: i64,
    last_ack: Option<Observation>,
    last_observed: Option<Observation>,
    safe_error_code: Option<String>,
    revision: u64,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "state", deny_unknown_fields)]
pub(crate) enum Durability {
    DurablyQueued,
    NotPersisted { code: String },
    PersistenceUnknown { code: String },
    Absent,
    Unavailable { code: String },
}
#[derive(Serialize, Deserialize)]
struct Snapshot {
    run: String,
    execution_home: DeviceId,
    as_of_ms: i64,
    revision: u64,
    durability: Durability,
    counts: HashMap<String, usize>,
    active: usize,
    backlog: usize,
    scan_complete: bool,
    all_known_children_canceled: bool,
    unresolved: usize,
    issue_count: usize,
    late_input_fence: bool,
}
#[derive(Default, Serialize, Deserialize)]
struct Reply {
    snapshot: Option<serde_json::Value>,
    runs: Vec<String>,
    cursor: Option<String>,
    #[serde(default)]
    discovery_issues: usize,
}
fn decode_reply(raw: &str) -> Result<Reply, String> {
    if raw.len() > 128 * 1024 {
        return Err("InvalidReply".into());
    }
    let reply: Reply = serde_json::from_str(raw).map_err(|_| "InvalidReply")?;
    if reply.runs.len() > PAGE
        || reply.discovery_issues > PAGE
        || reply.runs.iter().any(|run| !safe_run(run))
        || reply
            .cursor
            .as_ref()
            .is_some_and(|cursor| cursor.len() > 255)
    {
        return Err("InvalidReply".into());
    }
    Ok(reply)
}
fn validated_snapshot(reply: &Reply, run: &str, persist: bool) -> Option<serde_json::Value> {
    let snapshot = reply.snapshot.as_ref()?;
    if snapshot["run"] != run || !snapshot.is_object() {
        return None;
    }
    let durability: Durability = serde_json::from_value(snapshot["durability"].clone()).ok()?;
    let full = serde_json::from_value::<Snapshot>(snapshot.clone()).ok();
    if let Some(full) = &full {
        if full.counts.len() > 8
            || full
                .counts
                .values()
                .any(|count| *count > CHILD_LIMIT * 2 + 1)
            || full.issue_count > CHILD_LIMIT * 2 + 1
            || (full.all_known_children_canceled && (!full.scan_complete || full.unresolved != 0))
        {
            return None;
        }
    } else if matches!(durability, Durability::DurablyQueued | Durability::Absent)
        || snapshot.as_object()?.len() != 2
    {
        return None;
    }
    let valid = if persist {
        matches!(
            durability,
            Durability::DurablyQueued
                | Durability::NotPersisted { .. }
                | Durability::PersistenceUnknown { .. }
        )
    } else {
        matches!(
            durability,
            Durability::DurablyQueued | Durability::Absent | Durability::Unavailable { .. }
        )
    };
    valid.then(|| snapshot.clone())
}
pub(crate) fn decode_metadata_result(
    run: &str,
    persist: bool,
    result: Result<String, String>,
) -> serde_json::Value {
    let (snapshot, code) = match result {
        Ok(raw) => (
            decode_reply(&raw)
                .ok()
                .and_then(|reply| validated_snapshot(&reply, run, persist)),
            "InvalidReply",
        ),
        Err(_) => (
            None,
            if persist {
                "MetadataOutcomeUnknown"
            } else {
                "MetadataUnavailable"
            },
        ),
    };
    snapshot.unwrap_or_else(|| serde_json::json!({"run":run,"durability":if persist { Durability::PersistenceUnknown { code:code.into() } } else { Durability::Unavailable { code:code.into() } }}))
}
fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
fn base(root: &Path) -> PathBuf {
    root.join(".khor/agent/cancellations")
}
fn key(run: &str) -> String {
    blake3::hash(run.as_bytes()).to_hex().to_string()
}
fn directory(root: &Path, run: &str) -> PathBuf {
    base(root).join(key(run))
}
fn pending(root: &Path, run: &str) -> PathBuf {
    base(root)
        .join("pending")
        .join(format!("{}.json", key(run)))
}
pub(crate) fn read<T: DeserializeOwned>(path: &Path, limit: u64) -> Result<T, String> {
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|_| "ReadFailed")?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "ReadFailed")?;
    if bytes.len() as u64 > limit {
        return Err("ResourceLimit".into());
    }
    serde_json::from_slice(&bytes).map_err(|_| "InvalidRecord".into())
}
/// The boolean means an authoritative rename may have happened.
fn write<T: Serialize>(path: &Path, value: &T) -> Result<(), (String, bool)> {
    let bytes = serde_json::to_vec(value).map_err(|_| ("EncodeFailed".into(), false))?;
    if bytes.len() as u64 > SMALL {
        return Err(("ResourceLimit".into(), false));
    }
    let parent = path.parent().ok_or(("InvalidPath".into(), false))?;
    fs::create_dir_all(parent).map_err(|_| ("WriteFailed".into(), false))?;
    let temporary = parent.join(format!("{}.tmp", protocol::new_task_id()));
    let result = (|| {
        crate::link::write_private(&temporary, &bytes)
            .map_err(|_| ("WriteFailed".into(), false))?;
        File::open(&temporary)
            .and_then(|f| f.sync_all())
            .map_err(|_| ("SyncFailed".into(), false))?;
        fs::rename(&temporary, path).map_err(|_| ("RenameFailed".into(), false))?;
        #[cfg(unix)]
        File::open(parent)
            .and_then(|f| f.sync_all())
            .map_err(|_| ("SyncFailed".into(), true))?;
        Ok(())
    })();
    let _ = fs::remove_file(temporary);
    result
}
fn gate(path: &Path) -> Result<File, String> {
    let mut options = OpenOptions::new();
    options.create(true).truncate(false).read(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options.open(path).map_err(|_| "LockUnavailable")?;
    file.try_lock().map_err(|_| "Busy")?;
    Ok(file)
}
fn safe_run(run: &str) -> bool {
    !run.is_empty()
        && run.len() <= 128
        && run
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}
fn valid_run(node: &Node, run: &str) -> Result<bool, String> {
    if !safe_run(run) {
        return Err("InvalidRun".into());
    }
    let value: serde_json::Value = read(
        &node
            .root()
            .join(".khor/agent/runs")
            .join(format!("{run}.json")),
        8 * 1024 * 1024,
    )?;
    if value["record"]["id"] != run
        || value["record"]["owner"] != node.device_str()
        || !value["record"]["agent"].is_object()
    {
        return Err("RunIdentityMismatch".into());
    }
    Ok(!value["record"]["ended_at_ms"].is_null())
}
/// Both link discovery and the pending namespace have hard entry budgets.
/// Sorting is safe only after enforcing that bound, never on all history.
pub(crate) fn names(path: &Path, limit: usize) -> Result<Vec<String>, String> {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(_) => return Err("ScanFailed".into()),
    };
    let mut names = Vec::new();
    for (index, entry) in entries.enumerate() {
        if index >= limit {
            return Err("ResourceLimit".into());
        }
        let entry = entry.map_err(|_| "ScanFailed")?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "InvalidEntry")?;
        if name.ends_with(".json") {
            if !entry.file_type().map_err(|_| "ScanFailed")?.is_file() {
                return Err("InvalidEntry".into());
            }
            names.push(name);
        }
    }
    names.sort();
    Ok(names)
}
fn load_intent(node: &Node, run: &str) -> Result<Intent, String> {
    let intent: Intent = read(&directory(node.root(), run).join("intent.json"), 4096)?;
    if intent.version != 1 || intent.run != run || intent.execution_home != node.device() {
        return Err("IntentIdentityMismatch".into());
    }
    Ok(intent)
}
fn receipt_path(root: &Path, run: &str, child: &str) -> PathBuf {
    directory(root, run).join("children").join(child)
}
fn load_progress(root: &Path, run: &str) -> Result<Progress, String> {
    let path = directory(root, run).join("progress.json");
    match read(&path, 4096) {
        Ok(value) => Ok(value),
        Err(error) if path.try_exists().map_err(|_| "ReadFailed")? => Err(error),
        Err(_) => Ok(Progress::default()),
    }
}
fn discover_children(node: &Node, intent: &Intent) -> Result<(), String> {
    let mut progress = load_progress(node.root(), &intent.run)?;
    if progress.scan_complete {
        return Ok(());
    }
    let links = names(
        &delegation::children_dir(node.root(), &intent.run),
        CHILD_LIMIT * 2,
    )?;
    if links.len() > CHILD_LIMIT {
        return Err("ResourceLimit".into());
    }
    let page: Vec<_> = links
        .iter()
        .filter(|name| {
            progress
                .scan_cursor
                .as_ref()
                .is_none_or(|cursor| *name > cursor)
        })
        .take(PAGE)
        .cloned()
        .collect();
    for child in &page {
        match discover_child(node, intent, child) {
            Ok(mut receipt) => {
                receipt.revision = receipt.revision.saturating_add(1);
                write(&receipt_path(node.root(), &intent.run, child), &receipt)
                    .map_err(|(code, _)| code)?;
                issues::child(node, intent, child, None)?;
            }
            Err(code) => issues::child(node, intent, child, Some(&code))?,
        }
    }
    progress.scan_cursor = page.last().cloned();
    if page.len() < PAGE {
        progress.scan_complete = valid_run(node, &intent.run)?;
        if !progress.scan_complete {
            progress.scan_cursor = None;
        }
    }
    progress.revision = progress.revision.saturating_add(1);
    write(
        &directory(node.root(), &intent.run).join("progress.json"),
        &progress,
    )
    .map_err(|(e, _)| e)
}
fn discover_child(node: &Node, intent: &Intent, child: &str) -> Result<ChildReceipt, String> {
    let link: delegation::ChildLink = read(
        &delegation::children_dir(node.root(), &intent.run).join(child),
        SMALL,
    )
    .map_err(|code| format!("Link{code}"))?;
    if link.binding.caller != node.device() || link.binding.agent.is_empty() {
        return Err("LinkIdentityMismatch".into());
    }
    let path = receipt_path(node.root(), &intent.run, child);
    match read::<ChildReceipt>(&path, SMALL) {
        Ok(mut receipt) => {
            if receipt.version != 1
                || receipt.intent_id != intent.intent_id
                || receipt.binding != link.binding
                || receipt.child_key != child
                || receipt
                    .task
                    .as_ref()
                    .is_some_and(|task| Some(task) != link.task.as_ref())
            {
                return Err("ReceiptIdentityMismatch".into());
            }
            if receipt.task.is_none() && link.task.is_some() {
                receipt.task = link.task;
                receipt.phase = Phase::Queued;
            }
            Ok(receipt)
        }
        Err(code) if path.try_exists().map_err(|_| "ReadFailed")? => Err(format!("Receipt{code}")),
        Err(_) => Ok(ChildReceipt {
            version: 1,
            intent_id: intent.intent_id.clone(),
            child_key: child.into(),
            binding: link.binding,
            task: link.task.clone(),
            phase: if link.task.is_some() {
                Phase::Queued
            } else {
                Phase::NeedsAttention("UnknownTask".into())
            },
            attempt_no: 0,
            attempt_id: String::new(),
            next_retry_at_ms: 0,
            last_ack: None,
            last_observed: None,
            safe_error_code: None,
            revision: 0,
        }),
    }
}
struct Collection {
    receipts: Vec<(String, ChildReceipt)>,
    scan_complete: bool,
    issues: usize,
}
fn collect(node: &Node, intent: &Intent, persist_errors: bool) -> Result<Collection, String> {
    let children = names(
        &directory(node.root(), &intent.run).join("children"),
        CHILD_LIMIT * 2,
    )?;
    if children.len() > CHILD_LIMIT {
        return Err("ResourceLimit".into());
    }
    let mut unresolved = issues::children(node, intent)?;
    let mut receipts = Vec::new();
    for child in children {
        let receipt = read::<ChildReceipt>(&receipt_path(node.root(), &intent.run, &child), SMALL)
            .and_then(|receipt| {
                if receipt.version == 1
                    && receipt.intent_id == intent.intent_id
                    && receipt.child_key == child
                    && receipt.binding.caller == node.device()
                {
                    Ok(receipt)
                } else {
                    Err("ReceiptIdentityMismatch".into())
                }
            });
        match receipt {
            Ok(receipt) if !unresolved.contains(&child) => receipts.push((child, receipt)),
            Ok(_) => {}
            Err(code) => {
                if persist_errors {
                    issues::child(node, intent, &child, Some(&code))?;
                }
                unresolved.insert(child);
            }
        }
    }
    Ok(Collection {
        receipts,
        scan_complete: load_progress(node.root(), &intent.run)?.scan_complete,
        issues: unresolved.len() + usize::from(issues::pending_unresolved(node, &intent.run)),
    })
}
fn snapshot(node: &Node, run: &str, durability: Durability) -> serde_json::Value {
    let mut snapshot = Snapshot {
        run: run.into(),
        execution_home: node.device(),
        as_of_ms: now(),
        revision: 0,
        durability,
        counts: HashMap::new(),
        active: 0,
        backlog: 0,
        scan_complete: false,
        all_known_children_canceled: false,
        unresolved: 1,
        issue_count: 0,
        late_input_fence: false,
    };
    if let Ok(intent) = load_intent(node, run) {
        if let Ok(Collection {
            receipts,
            scan_complete: complete,
            issues,
        }) = collect(node, &intent, false)
        {
            snapshot.scan_complete = complete;
            snapshot.unresolved = issues;
            snapshot.issue_count = issues;
            if issues > 0 {
                snapshot.counts.insert("NeedsAttention".into(), issues);
            }
            snapshot.revision = load_progress(node.root(), run).map_or(0, |p| p.revision);
            for (_, receipt) in &receipts {
                let name = match receipt.phase {
                    Phase::Queued => "Queued",
                    Phase::InFlight => "InFlight",
                    Phase::OutcomeUnknown => "OutcomeUnknown",
                    Phase::AckPending => "AckPending",
                    Phase::ConfirmedCanceled => "ConfirmedCanceled",
                    Phase::TerminalOther(_) => "TerminalOther",
                    Phase::NeedsAttention(_) => "NeedsAttention",
                };
                *snapshot.counts.entry(name.into()).or_default() += 1;
                snapshot.revision = snapshot.revision.saturating_add(receipt.revision);
                snapshot.active += usize::from(receipt.phase.active());
                snapshot.unresolved +=
                    usize::from(!matches!(receipt.phase, Phase::ConfirmedCanceled));
            }
            snapshot.unresolved += usize::from(!complete);
            snapshot.backlog = snapshot.active + usize::from(!complete);
            snapshot.all_known_children_canceled =
                complete && snapshot.unresolved == 0 && valid_run(node, run) == Ok(true);
        }
    }
    serde_json::to_value(snapshot).expect("small cancellation snapshot")
}
pub(crate) fn metadata(root: &Path, operation: Metadata) -> Result<String, String> {
    let query = operation.clone();
    match metadata_inner(root, operation) {
        Ok(reply) => Ok(reply),
        Err(code) => {
            let (run, durability) = match query {
                Metadata::Persist { run, .. } => (run, Durability::NotPersisted { code }),
                Metadata::Status { run } => (run, Durability::Unavailable { code }),
                Metadata::Discover { .. } => return Err(code),
            };
            serde_json::to_string(&Reply {
                snapshot: Some(serde_json::json!({"run":run,"durability":durability})),
                ..Default::default()
            })
            .map_err(|_| "EncodeFailed".into())
        }
    }
}
fn metadata_inner(root: &Path, operation: Metadata) -> Result<String, String> {
    if matches!(operation, Metadata::Discover { .. })
        && !base(root)
            .join("pending")
            .try_exists()
            .map_err(|_| "ReadFailed")?
    {
        // A retained in-memory home must not recreate a removed store merely
        // to discover that it has no pending cancellation namespace.
        return serde_json::to_string(&Reply::default()).map_err(|_| "EncodeFailed".into());
    }
    let node = Node::open(root.to_path_buf()).map_err(|_| "StoreUnavailable")?;
    let mut reply = Reply::default();
    match operation {
        Metadata::Persist { run, reason } => {
            let retry_discovery = reason == "Retry";
            valid_run(&node, &run)?;
            fs::create_dir_all(base(root).join("pending")).map_err(|_| "WriteFailed")?;
            let _metadata = gate(&base(root).join("metadata.lock"))?;
            let dir = directory(root, &run);
            fs::create_dir_all(&dir).map_err(|_| "WriteFailed")?;
            let _run = gate(&dir.join("run.lock"))?;
            let intent_path = dir.join("intent.json");
            let prior = match read::<Intent>(&intent_path, 4096) {
                Ok(_) => Some(load_intent(&node, &run)?),
                Err(error) if intent_path.try_exists().map_err(|_| "ReadFailed")? => {
                    return Err(error);
                }
                Err(_) => None,
            };
            let entries = names(&base(root).join("pending"), RUN_LIMIT * 2)?;
            if !pending(root, &run).exists() && entries.len() >= RUN_LIMIT {
                reply.snapshot = Some(snapshot(
                    &node,
                    &run,
                    Durability::NotPersisted {
                        code: "ResourceLimit".into(),
                    },
                ));
            } else {
                let intent = prior.unwrap_or(Intent {
                    version: 1,
                    intent_id: protocol::new_task_id(),
                    run: run.clone(),
                    execution_home: node.device(),
                    requested_at_ms: now(),
                    reason,
                });
                issues::before_persist(&node, &run)?;
                // Publish the pending reference first. Recovery never calls it
                // complete if intent publication failed or is not yet visible.
                let durability = match write(&pending(root, &run), &run)
                    .and_then(|_| write(&intent_path, &intent))
                {
                    Ok(()) => Durability::DurablyQueued,
                    Err((code, true)) => Durability::PersistenceUnknown { code },
                    Err((code, false)) => {
                        if intent_path.exists() {
                            Durability::PersistenceUnknown { code }
                        } else {
                            Durability::NotPersisted { code }
                        }
                    }
                };
                if matches!(durability, Durability::DurablyQueued) {
                    let _ = issues::persisted(&node, &run);
                    if retry_discovery {
                        if let Ok(mut progress) = load_progress(root, &run) {
                            progress.scan_cursor = None;
                            progress.scan_complete = false;
                            let _ = write(&dir.join("progress.json"), &progress);
                        }
                    }
                    // Discovery failure keeps the committed intent durable and
                    // the snapshot incomplete. A later normal step retries it.
                    let _ = discover_children(&node, &intent);
                    reply.runs.push(run.clone());
                }
                reply.snapshot = Some(snapshot(&node, &run, durability));
            }
        }
        Metadata::Status { run } => {
            valid_run(&node, &run)?;
            let durability = if directory(root, &run)
                .join("intent.json")
                .try_exists()
                .map_err(|_| "ReadFailed")?
            {
                load_intent(&node, &run)?;
                Durability::DurablyQueued
            } else if pending(root, &run).try_exists().map_err(|_| "ReadFailed")? {
                Durability::Unavailable {
                    code: "PendingIntentUnavailable".into(),
                }
            } else {
                Durability::Absent
            };
            reply.snapshot = Some(snapshot(&node, &run, durability));
        }
        Metadata::Discover { after } => {
            let entries = names(&base(root).join("pending"), RUN_LIMIT * 2)?;
            let page: Vec<_> = entries
                .iter()
                .filter(|name| after.as_ref().is_none_or(|a| *name > a))
                .take(PAGE)
                .collect();
            if !page.is_empty() {
                let _metadata = gate(&base(root).join("metadata.lock"))?;
                for name in &page {
                    let (run, unresolved) = issues::discover_pending(&node, name)?;
                    reply.discovery_issues += usize::from(unresolved);
                    if let Some(run) = run {
                        reply.runs.push(run);
                    }
                }
            }
            reply.cursor = if page.len() == PAGE {
                page.last().map(|s| (*s).clone())
            } else {
                None
            };
        }
    }
    serde_json::to_string(&reply).map_err(|_| "EncodeFailed".into())
}
fn network_allowed(root: &Path) -> Result<bool, String> {
    let path = root.join(".khor/pause.json");
    match read::<serde_json::Value>(&path, 4096) {
        Ok(value) => value["on"]
            .as_bool()
            .map(|on| !on)
            .ok_or("PauseUnknown".into()),
        Err(_) if !path.try_exists().map_err(|_| "PauseUnknown")? => Ok(true),
        Err(_) => Err("PauseUnknown".into()),
    }
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "state")]
pub(crate) enum StepResult {
    Retired,
    ReadyAt { not_before_ms: i64 },
}
fn step_result(result: StepResult) -> Result<String, String> {
    serde_json::to_string(&result).map_err(|_| "EncodeFailed".into())
}
fn finish_step(
    node: &Node,
    intent: &Intent,
    receipts: &[(String, ChildReceipt)],
    complete: bool,
    updated: Option<(&str, &ChildReceipt)>,
) -> Result<String, String> {
    let mut due = if complete { None } else { Some(now()) };
    for (child, receipt) in receipts {
        if updated.is_some_and(|(key, _)| key == child) {
            continue;
        }
        if receipt.phase.active() {
            due = Some(due.map_or(receipt.next_retry_at_ms, |time| {
                time.min(receipt.next_retry_at_ms)
            }));
        }
    }
    if let Some((_, receipt)) = updated {
        if receipt.phase.active() {
            due = Some(due.map_or(receipt.next_retry_at_ms, |time| {
                time.min(receipt.next_retry_at_ms)
            }));
        }
    }
    if let Some(not_before_ms) = due {
        step_result(StepResult::ReadyAt {
            not_before_ms: not_before_ms.max(now()),
        })
    } else {
        issues::retire_pending(node, intent)?;
        step_result(StepResult::Retired)
    }
}
/// Exactly one Cancel or Get, fenced by a private run lease and attempt ID.
pub(crate) fn step(root: &Path, run: &str) -> Result<String, String> {
    let node = Node::open(root.to_path_buf())?;
    if !network_allowed(root)? || !valid_run(&node, run)? {
        return step_result(StepResult::ReadyAt {
            not_before_ms: now() + 1000,
        });
    }
    let dir = directory(root, run);
    let _run = gate(&dir.join("run.lock"))?;
    let intent = load_intent(&node, run)?;
    discover_children(&node, &intent)?;
    let Collection {
        receipts,
        scan_complete: complete,
        ..
    } = collect(&node, &intent, true)?;
    let progress_path = dir.join("progress.json");
    let mut progress = load_progress(root, run)?;
    let eligible = |(_, receipt): &&(String, ChildReceipt)| {
        receipt.phase.active() && receipt.next_retry_at_ms <= now()
    };
    let next = receipts
        .iter()
        .filter(eligible)
        .find(|(name, _)| {
            progress
                .child_cursor
                .as_ref()
                .is_none_or(|cursor| name > cursor)
        })
        .or_else(|| receipts.iter().filter(eligible).next());
    let Some((child, prior)) = next else {
        return finish_step(&node, &intent, &receipts, complete, None);
    };
    let child_path = receipt_path(root, run, child);
    fs::create_dir_all(child_path.parent().expect("child receipt directory"))
        .map_err(|_| "WriteFailed")?;
    let _child = gate(&child_path.with_extension("lock"))?;
    let mut receipt = prior.clone();
    if matches!(receipt.phase, Phase::InFlight) {
        receipt.phase = Phase::OutcomeUnknown;
        receipt.safe_error_code = Some("PreviousOwnerLost".into());
        receipt.revision = receipt.revision.saturating_add(1);
        write(&child_path, &receipt).map_err(|(e, _)| e)?;
    }
    let observe = matches!(receipt.phase, Phase::OutcomeUnknown | Phase::AckPending);
    let task = receipt.task.clone().ok_or("UnknownTask")?;
    receipt.attempt_no = receipt.attempt_no.saturating_add(1);
    receipt.attempt_id = protocol::new_task_id();
    let attempt_id = receipt.attempt_id.clone();
    receipt.phase = Phase::InFlight;
    receipt.revision = receipt.revision.saturating_add(1);
    progress.child_cursor = Some(child.clone());
    progress.revision = progress.revision.saturating_add(1);
    write(&progress_path, &progress).map_err(|(e, _)| e)?;
    write(&child_path, &receipt).map_err(|(e, _)| e)?;
    let binding = receipt.binding.clone();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| "RuntimeUnavailable")?;
    let action = if observe {
        BoundAction::Get { task: task.clone() }
    } else {
        BoundAction::Cancel { task: task.clone() }
    };
    if !network_allowed(root)? {
        return step_result(StepResult::ReadyAt {
            not_before_ms: now() + 1000,
        });
    }
    let outcome = runtime.block_on(node.delegation_on(
        &binding.home.hex(),
        delegation::Action::Bound {
            binding: binding.clone(),
            action,
        },
    ));
    let current: ChildReceipt = read(&child_path, SMALL)?;
    if load_intent(&node, run)?.intent_id != intent.intent_id
        || current.intent_id != intent.intent_id
        || current.attempt_id != attempt_id
    {
        return Err("StaleAttempt".into());
    }
    receipt.phase = Phase::OutcomeUnknown;
    receipt.safe_error_code = Some("DeliveryUnknown".into());
    if outcome.as_ref().is_err_and(|error| {
        matches!(
            error.as_str(),
            "Delegation reply identity mismatch"
                | "Delegation binding identity mismatch"
                | "Delegation binding protocol mismatch"
                | "Delegation endpoint binding changed"
        )
    }) {
        receipt.phase = Phase::NeedsAttention("BindingMismatch".into());
        receipt.safe_error_code = Some("BindingMismatch".into());
    }
    if let Ok(delegation::Answer::Task { record }) = outcome {
        if record.target.home == binding.home
            && record.caller == binding.caller
            && record.target.agent == binding.agent
            && record.task.id == task
        {
            let prior_revision = receipt
                .last_ack
                .as_ref()
                .into_iter()
                .chain(receipt.last_observed.as_ref())
                .map(|o| o.revision)
                .max();
            if prior_revision.is_some_and(|revision| record.revision < revision) {
                receipt.safe_error_code = Some("StaleObservation".into());
                receipt.next_retry_at_ms = now() + 1000;
                receipt.revision = receipt.revision.saturating_add(1);
                write(&child_path, &receipt).map_err(|(e, _)| e)?;
                return finish_step(&node, &intent, &receipts, complete, Some((child, &receipt)));
            }
            let observation = Observation {
                task,
                target_home: record.target.home,
                caller: record.caller,
                agent: record.target.agent.clone(),
                task_state: record.task.status.state.clone(),
                revision: record.revision,
                observed_at_ms: now(),
            };
            receipt.safe_error_code = None;
            receipt.phase = match record.task.status.state {
                protocol::TaskState::Canceled => Phase::ConfirmedCanceled,
                state if state.is_terminal() => Phase::TerminalOther(state),
                _ => {
                    if observe && receipt.last_ack.is_none() {
                        Phase::Queued
                    } else {
                        Phase::AckPending
                    }
                }
            };
            if observe {
                receipt.last_observed = Some(observation);
            } else {
                receipt.last_ack = Some(observation);
            }
        } else {
            receipt.phase = Phase::NeedsAttention("ReplyIdentityMismatch".into());
            receipt.safe_error_code = Some("ReplyIdentityMismatch".into());
        }
    }
    receipt.next_retry_at_ms =
        now() + (1i64 << receipt.attempt_no.saturating_sub(1).min(5)).min(30) * 1000;
    receipt.revision = receipt.revision.saturating_add(1);
    write(&child_path, &receipt).map_err(|(e, _)| e)?;
    finish_step(&node, &intent, &receipts, complete, Some((child, &receipt)))
}

type Completion = Arc<Mutex<Option<Result<String, String>>>>;
struct OwnedWorker(Option<std::thread::JoinHandle<Result<String, String>>>);
impl OwnedWorker {
    fn poll(&mut self) -> Option<Result<String, String>> {
        if !self.0.as_ref()?.is_finished() {
            return None;
        }
        Some(
            self.0
                .take()
                .expect("finished owner")
                .join()
                .unwrap_or_else(|_| Err("WorkerOwnerFailed".into())),
        )
    }
}
struct MetadataOwner {
    root: PathBuf,
    operation: Metadata,
    worker: OwnedWorker,
    completion: Completion,
}
struct StepOwner {
    root: PathBuf,
    run: String,
    worker: OwnedWorker,
    failures: u8,
}
struct Ready {
    root: PathBuf,
    run: String,
    at: Instant,
    failures: u8,
}
struct Home {
    cursor: Option<String>,
    next_scan: Instant,
    allowed: bool,
}
#[derive(Default)]
struct Scheduler {
    metadata: Option<MetadataOwner>,
    steps: Vec<StepOwner>,
    homes: HashMap<PathBuf, Home>,
    home_order: VecDeque<PathBuf>,
    ready: VecDeque<Ready>,
    retired: VecDeque<(PathBuf, String)>,
    durable: VecDeque<(PathBuf, String, Completion)>,
}
fn scheduler() -> &'static Mutex<Scheduler> {
    static STATE: OnceLock<Mutex<Scheduler>> = OnceLock::new();
    STATE.get_or_init(Default::default)
}
#[cfg(test)]
fn test_starts() -> &'static Mutex<VecDeque<(PathBuf, String, u32, i64)>> {
    static STARTS: OnceLock<Mutex<VecDeque<(PathBuf, String, u32, i64)>>> = OnceLock::new();
    STARTS.get_or_init(Default::default)
}
#[cfg(test)]
pub(crate) fn test_step_starts(root: &Path) -> Vec<(String, u32, i64)> {
    test_starts()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .iter()
        .filter(|(path, _, _, _)| path == root)
        .map(|(_, run, pid, at)| (run.clone(), *pid, *at))
        .collect()
}
#[cfg(test)]
pub(crate) fn test_reap_metadata() -> bool {
    let mut state = scheduler().lock().unwrap_or_else(|p| p.into_inner());
    state.pump(false);
    state.metadata.is_none()
}
fn start(root: PathBuf, operation: Operation) -> Result<OwnedWorker, String> {
    // A finite owner keeps polling/reaping while the caller awaits another
    // event. The process and its I/O retain Worker's original absolute budget.
    #[cfg(test)]
    let traced = match &operation {
        Operation::CancelStep { run } => Some((root.clone(), run.clone())),
        _ => None,
    };
    let mut worker = Worker::spawn(
        Request { root, operation },
        Arc::new(Control::default()),
        BUDGET,
        None,
    )?;
    #[cfg(test)]
    if let Some((root, run)) = traced
        && let Some(pid) = worker.pid()
    {
        let mut starts = test_starts().lock().unwrap_or_else(|p| p.into_inner());
        if starts.len() >= RUN_LIMIT * 4 {
            starts.pop_front();
        }
        starts.push_back((root, run, pid, now()));
    }
    let thread = std::thread::Builder::new()
        .name("remote-cancel-owner".into())
        .spawn(move || {
            loop {
                if let Some(result) = worker.poll() {
                    return result;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
        })
        .map_err(|_| "OwnerUnavailable")?;
    Ok(OwnedWorker(Some(thread)))
}
impl Scheduler {
    fn enqueue(&mut self, root: PathBuf, run: String, at: Instant, failures: u8) {
        // Discovery coalesces without bringing an existing not-before forward.
        if self.ready.len() >= RUN_LIMIT
            || self
                .ready
                .iter()
                .any(|item| item.root == root && item.run == run)
            || self
                .steps
                .iter()
                .any(|step| step.root == root && step.run == run)
        {
            return;
        }
        self.ready.push_back(Ready {
            root,
            run,
            at,
            failures,
        });
    }
    fn start_metadata(&mut self, root: PathBuf, operation: Metadata) -> Result<Completion, String> {
        let completion = Arc::new(Mutex::new(None));
        let worker = start(
            root.clone(),
            Operation::CancellationMetadata {
                operation: operation.clone(),
            },
        )?;
        #[cfg(test)]
        if matches!(operation, Metadata::Persist { .. }) {
            crate::mcp_worker::SWEEPS_STARTED.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
        self.metadata = Some(MetadataOwner {
            root,
            operation,
            worker,
            completion: completion.clone(),
        });
        Ok(completion)
    }
    fn pump(&mut self, background: bool) {
        if let Some(result) = self.metadata.as_mut().and_then(|m| m.worker.poll()) {
            let owner = self.metadata.take().expect("metadata owner");
            if let Ok(raw) = &result {
                if let Ok(reply) = decode_reply(raw) {
                    if let Metadata::Persist { run, .. } = &owner.operation {
                        if validated_snapshot(&reply, run, true)
                            .as_ref()
                            .is_some_and(|s| s["durability"]["state"] == "DurablyQueued")
                        {
                            self.durable
                                .retain(|(root, id, _)| root != &owner.root || id != run);
                            if self.durable.len() >= RUN_LIMIT {
                                self.durable.pop_front();
                            }
                            self.durable.push_back((
                                owner.root.clone(),
                                run.clone(),
                                owner.completion.clone(),
                            ));
                        }
                    }
                    let valid = match &owner.operation {
                        Metadata::Persist { run, .. } => {
                            validated_snapshot(&reply, run, true).is_some()
                                && reply.runs.iter().all(|id| id == run)
                        }
                        Metadata::Status { run } => {
                            validated_snapshot(&reply, run, false).is_some()
                                && reply.runs.is_empty()
                        }
                        Metadata::Discover { .. } => reply.snapshot.is_none(),
                    };
                    if valid {
                        for run in reply.runs {
                            self.enqueue(owner.root.clone(), run, Instant::now(), 0);
                        }
                    }
                    if matches!(owner.operation, Metadata::Discover { .. }) {
                        if let Some(home) = self.homes.get_mut(&owner.root) {
                            home.cursor = reply.cursor;
                        }
                    }
                }
            }
            *owner.completion.lock().unwrap_or_else(|p| p.into_inner()) = Some(result);
        }
        let mut index = 0;
        while index < self.steps.len() {
            if let Some(result) = self.steps[index].worker.poll() {
                let owner = self.steps.remove(index);
                match result
                    .ok()
                    .filter(|raw| raw.len() <= 4096)
                    .and_then(|raw| serde_json::from_str::<StepResult>(&raw).ok())
                {
                    Some(StepResult::Retired) => {}
                    Some(StepResult::ReadyAt { not_before_ms }) => {
                        let delay = not_before_ms.saturating_sub(now()).clamp(0, 30_000) as u64;
                        self.enqueue(
                            owner.root,
                            owner.run,
                            Instant::now() + Duration::from_millis(delay),
                            0,
                        );
                    }
                    None => {
                        let failures = owner.failures.saturating_add(1).min(6);
                        let delay = (1u64 << failures.saturating_sub(1)).min(30);
                        self.enqueue(
                            owner.root,
                            owner.run,
                            Instant::now() + Duration::from_secs(delay),
                            failures,
                        );
                    }
                }
            } else {
                index += 1;
            }
        }
        if background && self.metadata.is_none() {
            if let Some((root, run)) = self.retired.pop_front() {
                if self
                    .start_metadata(
                        root.clone(),
                        Metadata::Persist {
                            run: run.clone(),
                            reason: "RunFailed".into(),
                        },
                    )
                    .is_err()
                {
                    self.retired.push_back((root, run));
                }
            } else if let Some(root) = self.next_home() {
                let home = self.homes.get_mut(&root).expect("known home");
                let operation = Metadata::Discover {
                    after: home.cursor.clone(),
                };
                home.next_scan = Instant::now() + Duration::from_secs(1);
                let _ = self.start_metadata(root, operation);
            }
        }
        let count = self.ready.len();
        for _ in 0..count {
            if self.steps.len() >= 4 {
                break;
            }
            let Some(item) = self.ready.pop_front() else {
                break;
            };
            if item.at > Instant::now()
                || !self.homes.get(&item.root).is_some_and(|home| home.allowed)
            {
                self.ready.push_back(item);
                continue;
            }
            match start(
                item.root.clone(),
                Operation::CancelStep {
                    run: item.run.clone(),
                },
            ) {
                Ok(worker) => self.steps.push(StepOwner {
                    root: item.root,
                    run: item.run,
                    worker,
                    failures: item.failures,
                }),
                Err(_) => {
                    let failures = item.failures.saturating_add(1).min(6);
                    self.enqueue(
                        item.root,
                        item.run,
                        Instant::now()
                            + Duration::from_secs((1u64 << failures.saturating_sub(1)).min(30)),
                        failures,
                    );
                }
            }
        }
    }
    fn next_home(&mut self) -> Option<PathBuf> {
        for _ in 0..self.home_order.len() {
            let root = self.home_order.pop_front()?;
            let due = self
                .homes
                .get(&root)
                .is_some_and(|h| h.next_scan <= Instant::now());
            self.home_order.push_back(root.clone());
            if due {
                return Some(root);
            }
        }
        None
    }
}
/// The reactor supplies cached pause/rest state. This function scans no files.
pub(crate) fn pump(root: &Path, allowed: bool) {
    let mut state = scheduler().lock().unwrap_or_else(|p| p.into_inner());
    if let Some(home) = state.homes.get_mut(root) {
        home.allowed = allowed;
    } else if state.homes.len() < RUN_LIMIT {
        state.home_order.push_back(root.into());
        state.homes.insert(
            root.into(),
            Home {
                cursor: None,
                next_scan: Instant::now(),
                allowed,
            },
        );
    }
    state.pump(true);
}
/// Called by the synchronous executor only after local retirement; memory only.
pub(crate) fn local_retired(root: PathBuf, run: String) {
    let mut state = scheduler().lock().unwrap_or_else(|p| p.into_inner());
    let known = state
        .ready
        .iter()
        .any(|item| item.root == root && item.run == run)
        || state.steps.iter().any(|s| s.root == root && s.run == run)
        || state
            .durable
            .iter()
            .any(|(r, id, _)| r == &root && id == &run)
        || state.metadata.as_ref().is_some_and(|m| {
            m.root == root
                && matches!(&m.operation, Metadata::Persist { run: id, .. } if id == &run)
        });
    if !known
        && state.retired.len() < RUN_LIMIT
        && !state
            .retired
            .iter()
            .any(|r| r == &(root.clone(), run.clone()))
    {
        state.retired.push_back((root, run));
    }
    state.pump(!known);
}
#[cfg(test)]
pub(crate) fn release_test_root(root: &Path) -> Result<(), String> {
    let mut state = scheduler().lock().unwrap_or_else(|p| p.into_inner());
    if let Some(home) = state.homes.get_mut(root) {
        home.allowed = false;
    }
    state.pump(false);
    if state.steps.iter().any(|s| s.root == root)
        || state.metadata.as_ref().is_some_and(|m| m.root == root)
    {
        return Err("Owned cancellation workers are still retiring".into());
    }
    state.ready.retain(|item| item.root != root);
    state.retired.retain(|(r, _)| r != root);
    state.durable.retain(|(r, _, _)| r != root);
    state.homes.remove(root);
    state.home_order.retain(|r| r != root);
    Ok(())
}
pub(crate) async fn request(
    root: &Path,
    run: &str,
    persist: bool,
    allowed: bool,
    retry: bool,
) -> serde_json::Value {
    let operation = if persist {
        Metadata::Persist {
            run: run.into(),
            reason: if retry { "Retry" } else { "Stop" }.into(),
        }
    } else {
        Metadata::Status { run: run.into() }
    };
    let completion = {
        let mut state = scheduler().lock().unwrap_or_else(|p| p.into_inner());
        if let Some(home) = state.homes.get_mut(root) {
            home.allowed = allowed;
        }
        if !state.homes.contains_key(root) && state.homes.len() < RUN_LIMIT {
            state.home_order.push_back(root.into());
            state.homes.insert(
                root.into(),
                Home {
                    cursor: None,
                    next_scan: Instant::now() + Duration::from_secs(1),
                    allowed,
                },
            );
        }
        state.pump(false);
        // Reap before admission, without starting a periodic metadata scan.
        if persist && !state.homes.contains_key(root) {
            Err("ResourceLimit".to_owned())
        } else if persist
            && !retry
            && let Some((_, _, completion)) = state
                .durable
                .iter()
                .find(|(r, id, _)| r == root && id == run)
        {
            Ok(completion.clone())
        } else if let Some(owner) = &state.metadata {
            let same = owner.root == root
                && match (&owner.operation, &operation) {
                    (Metadata::Persist { run: a, .. }, Metadata::Persist { run: b, .. })
                    | (Metadata::Status { run: a }, Metadata::Status { run: b }) => a == b,
                    _ => false,
                };
            if same {
                Ok(owner.completion.clone())
            } else {
                Err("Busy".to_owned())
            }
        } else {
            state.start_metadata(root.into(), operation)
        }
    };
    let result = match completion {
        Err(code) => {
            return serde_json::json!({"run":run,"durability":if persist { Durability::NotPersisted { code } } else { Durability::Unavailable { code } }});
        }
        Ok(completion) => {
            let deadline = Instant::now() + BUDGET + Duration::from_millis(250);
            loop {
                {
                    let mut state = scheduler().lock().unwrap_or_else(|p| p.into_inner());
                    state.pump(false);
                }
                if let Some(result) = completion.lock().unwrap_or_else(|p| p.into_inner()).clone() {
                    break result;
                }
                if Instant::now() >= deadline {
                    break Err("MetadataOutcomeUnknown".into());
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        }
    };
    decode_metadata_result(run, persist, result)
}
