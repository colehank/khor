//! Durable, bounded cancellation of known children. One process-owned driver
//! advances admitted work while this process lives. After exit, the next normal
//! entry resumes the private pending set; progress across process exit is not promised.
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
    sync::{Arc, Condvar, Mutex, OnceLock},
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum Phase {
    Queued,
    InFlight,
    OutcomeUnknown,
    AckPending,
    ConfirmedCanceled,
    InputFenced,
    InputSuperseded,
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
#[derive(Clone, PartialEq, Serialize, Deserialize)]
struct Observation {
    task: String,
    target_home: DeviceId,
    caller: DeviceId,
    agent: String,
    task_state: protocol::TaskState,
    revision: u64,
    observed_at_ms: i64,
}
#[derive(Clone, PartialEq, Serialize, Deserialize)]
struct InputIdentity {
    message: String,
    parent: delegation::TaskRef,
}
#[derive(Clone, PartialEq, Serialize, Deserialize)]
struct ChildReceipt {
    version: u32,
    intent_id: String,
    child_key: String,
    binding: Binding,
    task: Option<String>,
    #[serde(default)]
    input: Option<InputIdentity>,
    phase: Phase,
    attempt_no: u64,
    attempt_id: String,
    next_retry_at_ms: i64,
    last_ack: Option<Observation>,
    last_observed: Option<Observation>,
    safe_error_code: Option<String>,
    revision: u64,
}
impl ChildReceipt {
    fn requires_input_scope(&self) -> bool {
        self.input.is_some() || self.task.is_none()
    }
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
        if full.counts.len() > 10
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
pub(crate) fn gate(path: &Path) -> Result<File, String> {
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
    let mut deferred = false;
    for child in &page {
        match discover_child(node, intent, child) {
            Ok(mut receipt) => {
                receipt.revision = receipt.revision.saturating_add(1);
                write(&receipt_path(node.root(), &intent.run, child), &receipt)
                    .map_err(|(code, _)| code)?;
                issues::child(node, intent, child, None)?;
            }
            Err(code) if code == "Busy" => {
                deferred = true;
                break;
            }
            Err(code) => issues::child(node, intent, child, Some(&code))?,
        }
        progress.scan_cursor = Some(child.clone());
    }
    if page.len() < PAGE && !deferred {
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
fn source_child(
    node: &Node,
    intent: &Intent,
    child: &str,
) -> Result<(delegation::ChildLink, Option<InputIdentity>), String> {
    let link: delegation::ChildLink = read(
        &delegation::children_dir(node.root(), &intent.run).join(child),
        SMALL,
    )
    .map_err(|code| format!("Link{code}"))?;
    if link.binding.caller != node.device()
        || link.binding.agent.is_empty()
        || delegation::child_key(&link.binding, &link.message)? != child
        || link
            .input_parent
            .as_ref()
            .is_some_and(|parent| parent.home != node.device())
    {
        return Err("LinkIdentityMismatch".into());
    }
    let input = if matches!(link.binding.protocol, delegation::BindingProtocol::Native) {
        let parent = match &link.input_parent {
            Some(parent) => Some(parent.clone()),
            None if link.task.is_none() => {
                let path = node.root().join(".khor/agent/messages").join(child);
                if path.try_exists().map_err(|_| "ReadFailed")? {
                    Some(read::<delegation::TaskRef>(&path, SMALL)?)
                } else {
                    None
                }
            }
            None => None,
        };
        if parent.as_ref().is_some_and(|parent| {
            parent.home != node.device()
                || parent.id.trim().is_empty()
                || parent.id.len() > 256
                || parent.id.chars().any(char::is_control)
        }) {
            return Err("LinkIdentityMismatch".into());
        }
        parent.map(|parent| InputIdentity {
            message: link.message.clone(),
            parent,
        })
    } else {
        None
    };
    if link.input_scoped && input.is_none() {
        return Err("LinkIdentityMismatch".into());
    }
    Ok((link, input))
}
fn validate_child_receipt(
    intent: &Intent,
    child: &str,
    receipt: &ChildReceipt,
    link: &delegation::ChildLink,
    input: &Option<InputIdentity>,
) -> Result<(), String> {
    if receipt.version != 1
        || receipt.intent_id != intent.intent_id
        || receipt.binding != link.binding
        || receipt.child_key != child
        || (link.input_scoped && receipt.input.is_none())
        || receipt
            .input
            .as_ref()
            .is_some_and(|identity| Some(identity) != input.as_ref())
        || receipt
            .task
            .as_ref()
            .is_some_and(|task| Some(task) != link.task.as_ref())
    {
        return Err("ReceiptIdentityMismatch".into());
    }
    Ok(())
}
fn scope_child(
    node: &Node,
    intent: &Intent,
    child: &str,
    link: delegation::ChildLink,
    input: &InputIdentity,
) -> Result<(delegation::ChildLink, Option<InputIdentity>), String> {
    delegation::scope_input_child(
        node.root(),
        &delegation::TaskRef {
            home: node.device(),
            id: intent.run.clone(),
        },
        &link.machine,
        &link.binding,
        &link.message,
        &input.parent,
    )?;
    source_child(node, intent, child)
}
fn discover_child(node: &Node, intent: &Intent, child: &str) -> Result<ChildReceipt, String> {
    let (mut link, mut input) = source_child(node, intent, child)?;
    let path = receipt_path(node.root(), &intent.run, child);
    match read::<ChildReceipt>(&path, SMALL) {
        Ok(mut receipt) => {
            // Existing committed scope with a missing/mismatched input is
            // corruption, not permission to downgrade to task-wide Cancel.
            validate_child_receipt(intent, child, &receipt, &link, &input)?;
            let input_mode = receipt.requires_input_scope();
            if input_mode && !link.input_scoped {
                if let Some(identity) = input.clone() {
                    (link, input) = scope_child(node, intent, child, link, &identity)?;
                }
            }
            if input_mode && input.is_some() && receipt.input.is_none() {
                receipt.input = input.clone();
                receipt.phase = Phase::Queued;
            }
            if receipt.task.is_none() && link.task.is_some() {
                receipt.task = link.task.clone();
                receipt.phase = Phase::Queued;
            }
            validate_child_receipt(intent, child, &receipt, &link, &input)?;
            Ok(receipt)
        }
        Err(code) if path.try_exists().map_err(|_| "ReadFailed")? => Err(format!("Receipt{code}")),
        Err(_) => {
            if link.task.is_none() && !link.input_scoped {
                if let Some(identity) = input.clone() {
                    (link, input) = scope_child(node, intent, child, link, &identity)?;
                }
            }
            Ok(ChildReceipt {
                version: 1,
                intent_id: intent.intent_id.clone(),
                child_key: child.into(),
                binding: link.binding,
                task: link.task.clone(),
                input: if link.input_scoped {
                    input.clone()
                } else {
                    None
                },
                phase: if link.task.is_some() || input.is_some() {
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
            })
        }
    }
}
struct Collection {
    receipts: Vec<(String, ChildReceipt)>,
    scan_complete: bool,
    issues: usize,
}
fn collect(node: &Node, intent: &Intent, persist_errors: bool) -> Result<Collection, String> {
    let receipt_names = names(
        &directory(node.root(), &intent.run).join("children"),
        CHILD_LIMIT * 2,
    )?;
    let link_names = names(
        &delegation::children_dir(node.root(), &intent.run),
        CHILD_LIMIT * 2,
    )?;
    if receipt_names.len() > CHILD_LIMIT || link_names.len() > CHILD_LIMIT {
        return Err("ResourceLimit".into());
    }
    let mut complete = load_progress(node.root(), &intent.run)?.scan_complete;
    let mut children = receipt_names.clone();
    children.extend(link_names.iter().cloned());
    children.sort();
    children.dedup();
    let mut unresolved = issues::children(node, intent)?;
    let mut receipts = Vec::new();
    for child in children {
        // Discovery creates receipts a bounded page at a time. Before its
        // completion, unseen links cannot count as canceled or as corruption.
        if !complete && !receipt_names.contains(&child) {
            continue;
        }
        let receipt: Result<ChildReceipt, String> = (|| {
            if !link_names.contains(&child) {
                return Err("LinkMissing".into());
            }
            let (link, input) = source_child(node, intent, &child)?;
            let receipt =
                read::<ChildReceipt>(&receipt_path(node.root(), &intent.run, &child), SMALL)?;
            validate_child_receipt(intent, &child, &receipt, &link, &input)?;
            let needs_scope =
                !link.input_scoped && input.is_some() && receipt.requires_input_scope();
            if needs_scope && !persist_errors {
                return Err("InputScopeUncommitted".into());
            }
            if persist_errors {
                let updated = discover_child(node, intent, &child)?;
                if updated != receipt {
                    write(&receipt_path(node.root(), &intent.run, &child), &updated)
                        .map_err(|(code, _)| code)?;
                }
                Ok(updated)
            } else {
                Ok(receipt)
            }
        })();
        match receipt {
            Ok(receipt) if !unresolved.contains(&child) => receipts.push((child, receipt)),
            Ok(_) => {}
            Err(code) => {
                if code == "Busy" {
                    // A legitimate short publication race is Deferred. Keep
                    // this run scheduled; never persist a permanent bad child.
                    complete = false;
                } else if persist_errors {
                    issues::child(node, intent, &child, Some(&code))?;
                }
                unresolved.insert(child);
            }
        }
    }
    Ok(Collection {
        receipts,
        scan_complete: complete,
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
                    Phase::InputFenced => "InputFenced",
                    Phase::InputSuperseded => "InputSuperseded",
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
            // This aggregate promises only explicit absence fences for every
            // linked input. Accepted/superseded inputs keep their distinct truth.
            snapshot.late_input_fence = complete
                && issues == 0
                && !receipts.is_empty()
                && receipts
                    .iter()
                    .all(|(_, receipt)| matches!(receipt.phase, Phase::InputFenced));
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
/// Exactly one task Cancel/Get or exact-input CancelInput, fenced by a private run lease and attempt ID.
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
    let task = receipt.task.clone();
    if task.is_none() && receipt.input.is_none() {
        return Err("UnknownTask".into());
    }
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
    let action = if let Some(input) = &receipt.input {
        BoundAction::CancelInput {
            message: input.message.clone(),
            parent: Some(input.parent.clone()),
        }
    } else if observe {
        BoundAction::Get {
            task: task.clone().expect("known task"),
        }
    } else {
        BoundAction::Cancel {
            task: task.clone().expect("known task"),
        }
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
    if let Some(input) = receipt.input.clone() {
        let mut observed = None;
        match outcome {
            Ok(delegation::Answer::Input {
                binding: received,
                message,
                parent,
                resolution,
            }) if received == binding
                && input.message == message
                && Some(&input.parent) == parent.as_ref() =>
            {
                use delegation::InputResolution;
                match resolution {
                    InputResolution::Fenced if task.is_none() => {
                        receipt.phase = Phase::InputFenced;
                        receipt.safe_error_code = None;
                    }
                    InputResolution::Known { task: found, .. }
                    | InputResolution::Superseded { task: found }
                        if found.home != binding.home
                            || task.as_ref().is_some_and(|task| task != &found.id) =>
                    {
                        receipt.phase = Phase::NeedsAttention("ReplyIdentityMismatch".into());
                        receipt.safe_error_code = Some("ReplyIdentityMismatch".into());
                    }
                    InputResolution::Fenced => {
                        receipt.phase = Phase::NeedsAttention("ReplyIdentityMismatch".into());
                        receipt.safe_error_code = Some("ReplyIdentityMismatch".into());
                    }
                    resolution => {
                        let found = match &resolution {
                            InputResolution::Known { task, .. }
                            | InputResolution::Superseded { task } => task,
                            InputResolution::Fenced => unreachable!(),
                        };
                        // Link first: a crash leaves the existing input receipt
                        // InFlight and the exact recovered task discoverable.
                        let link: delegation::ChildLink =
                            read(&delegation::children_dir(root, run).join(child), SMALL)?;
                        delegation::recover_input_child(
                            root,
                            &delegation::TaskRef {
                                home: node.device(),
                                id: run.into(),
                            },
                            &link.machine,
                            &binding,
                            &input.message,
                            &input.parent,
                            &found.id,
                        )?;
                        receipt.task = Some(found.id.clone());
                        match resolution {
                            InputResolution::Known {
                                task,
                                state,
                                revision,
                                ..
                            } => observed = Some((task.id, state, revision)),
                            InputResolution::Superseded { .. } => {
                                receipt.phase = Phase::InputSuperseded;
                                receipt.safe_error_code = None;
                            }
                            InputResolution::Fenced => unreachable!(),
                        }
                    }
                }
            }
            Ok(_) => {
                receipt.phase = Phase::NeedsAttention("ReplyIdentityMismatch".into());
                receipt.safe_error_code = Some("ReplyIdentityMismatch".into());
            }
            Err(_) => {}
        }
        if let Some((task, state, revision)) = observed {
            let prior_revision = receipt
                .last_ack
                .as_ref()
                .into_iter()
                .chain(receipt.last_observed.as_ref())
                .map(|observation| observation.revision)
                .max();
            if prior_revision.is_some_and(|prior| revision < prior) {
                receipt.safe_error_code = Some("StaleObservation".into());
            } else {
                let observation = Observation {
                    task,
                    target_home: binding.home,
                    caller: binding.caller,
                    agent: binding.agent,
                    task_state: state.clone(),
                    revision,
                    observed_at_ms: now(),
                };
                receipt.safe_error_code = None;
                receipt.phase = match state {
                    protocol::TaskState::Canceled => Phase::ConfirmedCanceled,
                    state if state.is_terminal() => Phase::TerminalOther(state),
                    _ => Phase::AckPending,
                };
                if observe {
                    receipt.last_observed = Some(observation);
                } else {
                    receipt.last_ack = Some(observation);
                }
            }
        }
        receipt.next_retry_at_ms =
            now() + (1i64 << receipt.attempt_no.saturating_sub(1).min(5)).min(30) * 1000;
        receipt.revision = receipt.revision.saturating_add(1);
        write(&child_path, &receipt).map_err(|(error, _)| error)?;
        return finish_step(&node, &intent, &receipts, complete, Some((child, &receipt)));
    }
    let task = task.ok_or("UnknownTask")?;
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
    fn join(&mut self) -> Result<String, String> {
        self.0
            .take()
            .ok_or_else(|| "WorkerOwnerMissing".to_owned())?
            .join()
            .unwrap_or_else(|_| Err("WorkerOwnerFailed".into()))
    }
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
impl Drop for OwnedWorker {
    fn drop(&mut self) {
        if self.0.is_some() {
            let _ = self.join();
        }
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
    closing: std::collections::HashSet<PathBuf>,
    closed: std::collections::HashSet<PathBuf>,
    closing_steps: usize,
    closing_metadata: bool,
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

#[derive(Default)]
struct DriverSignal {
    pending: Mutex<bool>,
    changed: Condvar,
}
fn driver_signal() -> &'static DriverSignal {
    static SIGNAL: OnceLock<DriverSignal> = OnceLock::new();
    SIGNAL.get_or_init(Default::default)
}
fn wake_driver() {
    let signal = driver_signal();
    *signal.pending.lock().unwrap_or_else(|p| p.into_inner()) = true;
    signal.changed.notify_one();
}
struct Driver {
    // Process lifetime, not a Root job. Root close joins only its existing owners.
    _owner: std::thread::JoinHandle<()>,
}
fn ensure_driver() -> Result<(), String> {
    static DRIVER: OnceLock<Mutex<Option<Driver>>> = OnceLock::new();
    // Callers already hold Scheduler. The loop never holds this mutex while
    // taking Scheduler, and only a finished handle may be joined here.
    let mut driver = DRIVER
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    if driver.as_ref().is_some_and(|d| d._owner.is_finished()) {
        let finished = driver.take().expect("finished driver owner");
        let _ = finished._owner.join();
    }
    if driver.is_none() {
        let owner = std::thread::Builder::new()
            .name("remote-cancel-driver".into())
            .spawn(drive_admitted)
            .map_err(|_| "DriverUnavailable".to_owned())?;
        *driver = Some(Driver { _owner: owner });
    }
    Ok(())
}
fn drive_admitted() {
    let signal = driver_signal();
    let mut next = None::<Instant>;
    loop {
        let mut pending = signal.pending.lock().unwrap_or_else(|p| p.into_inner());
        while !*pending {
            if let Some(at) = next {
                let delay = at.saturating_duration_since(Instant::now());
                if delay.is_zero() {
                    break;
                }
                let (guard, result) = signal
                    .changed
                    .wait_timeout(pending, delay)
                    .unwrap_or_else(|p| p.into_inner());
                pending = guard;
                if result.timed_out() {
                    break;
                }
            } else {
                pending = signal
                    .changed
                    .wait(pending)
                    .unwrap_or_else(|p| p.into_inner());
            }
        }
        *pending = false;
        drop(pending); // Never acquire Scheduler while holding the control lock.
        let mut state = scheduler().lock().unwrap_or_else(|p| p.into_inner());
        state.pump(false); // Already admitted work only; never Discover/Root scans.
        next = state.driver_deadline(Instant::now());
        // Mutations notify under Scheduler. This pump has consumed all preceding
        // mutations; discard its own start/enqueue notifications before unlocking.
        // In particular, a failed retired Persist must wait rather than self-spin.
        *signal.pending.lock().unwrap_or_else(|p| p.into_inner()) = false;
    }
}
#[cfg(all(test, unix))]
pub(crate) fn test_close_state(root: &Path) -> (bool, bool, usize, bool, usize) {
    let state = scheduler().lock().unwrap_or_else(|p| p.into_inner());
    (
        state.closing.contains(root),
        state.closed.contains(root),
        state.steps.len() + state.closing_steps,
        state.metadata.is_some() || state.closing_metadata,
        state
            .retired
            .iter()
            .filter(|(path, _)| path == root)
            .count(),
    )
}
#[cfg(all(test, unix))]
pub(crate) fn test_defer_scan(root: &Path) {
    let mut state = scheduler().lock().unwrap_or_else(|p| p.into_inner());
    state
        .homes
        .get_mut(root)
        .expect("admitted fixture Root")
        .next_scan = Instant::now() + Duration::from_secs(60);
}
#[cfg(all(test, unix))]
pub(crate) fn test_root_count() -> usize {
    scheduler()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .root_count()
}
#[cfg(all(test, unix))]
pub(crate) fn test_resume_step(root: &Path, run: &str) {
    let mut state = scheduler().lock().unwrap_or_else(|p| p.into_inner());
    assert!(state.steps.len() + state.closing_steps < 4);
    // Fixture setup uses an already committed real intent, then exercises the
    // same finite Step owner as normal pending discovery. No worker reply is injected.
    let worker = start(root.into(), Operation::CancelStep { run: run.into() }).unwrap();
    state.steps.push(StepOwner {
        root: root.into(),
        run: run.into(),
        worker,
        failures: 0,
    });
}
#[cfg(all(test, unix))]
pub(crate) fn test_ready_durable_step(root: &Path, run: &str) {
    let mut state = scheduler().lock().unwrap_or_else(|p| p.into_inner());
    assert!(state.can_admit_root(root));
    state.homes.entry(root.into()).or_insert(Home {
        cursor: None,
        next_scan: Instant::now() + Duration::from_secs(30),
        allowed: true,
    });
    state.enqueue(root.into(), run.into(), Instant::now(), 0);
    state.pump(false);
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
    ensure_driver()?;
    wake_driver();
    #[cfg(test)]
    let traced = match &operation {
        Operation::CancelStep { run } => Some((root.clone(), run.clone())),
        _ => None,
    };
    #[cfg(all(test, unix))]
    let worker_root = root.clone();
    let mut worker = Worker::spawn(
        Request { root, operation },
        Arc::new(Control::default()),
        BUDGET,
        None,
    )?;
    #[cfg(all(test, unix))]
    if let Some(pid) = worker.pid() {
        crate::khor_agent::runtime_close_tests::owner_started(&worker_root, pid);
    }
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
    /// A Root can occupy several lifecycle states during transitions, but its
    /// admission reservation is counted once. Cold closes reserve capacity too.
    fn root_count(&self) -> usize {
        self.homes
            .keys()
            .chain(&self.closed)
            .chain(&self.closing)
            .collect::<std::collections::HashSet<_>>()
            .len()
    }
    fn can_admit_root(&self, root: &Path) -> bool {
        self.homes.contains_key(root)
            || self.closed.contains(root)
            || self.closing.contains(root)
            || self.root_count() < RUN_LIMIT
    }
    fn enqueue(&mut self, root: PathBuf, run: String, at: Instant, failures: u8) {
        // Discovery coalesces without bringing an existing not-before forward.
        if self.closing.contains(&root)
            || self.closed.contains(&root)
            || self.ready.len() >= RUN_LIMIT
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
        wake_driver();
    }
    fn start_metadata(&mut self, root: PathBuf, operation: Metadata) -> Result<Completion, String> {
        if self.closing.contains(&root) {
            return Err("RuntimeClosing".into());
        }
        if self.closed.contains(&root) {
            return Err("RuntimeClosed".into());
        }
        if self.closing_metadata {
            return Err("Busy".into());
        }
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
    fn pump(&mut self, discover: bool) {
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
        if self.metadata.is_none() && !self.closing_metadata {
            if let Some(index) = self
                .retired
                .iter()
                .position(|(root, _)| !self.closing.contains(root) && !self.closed.contains(root))
            {
                let (root, run) = self.retired.remove(index).expect("retired entry");
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
            } else if discover && let Some(root) = self.next_home() {
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
            if self.steps.len() + self.closing_steps >= 4 {
                break;
            }
            let Some(item) = self.ready.pop_front() else {
                break;
            };
            if item.at > Instant::now()
                || self.closing.contains(&item.root)
                || self.closed.contains(&item.root)
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
    fn driver_deadline(&self, now: Instant) -> Option<Instant> {
        if self.metadata.is_some() || !self.steps.is_empty() {
            return Some(now + Duration::from_millis(5));
        }
        let mut next = None;
        // Retired entries are already admitted. A failed native start retries
        // with a finite delay; an idle process never starts a discovery scan.
        if !self.closing_metadata
            && self
                .retired
                .iter()
                .any(|(root, _)| !self.closing.contains(root) && !self.closed.contains(root))
        {
            next = Some(now + Duration::from_secs(1));
        }
        if self.steps.len() + self.closing_steps < 4 {
            for item in &self.ready {
                if !self.closing.contains(&item.root)
                    && !self.closed.contains(&item.root)
                    && self.homes.get(&item.root).is_some_and(|home| home.allowed)
                {
                    let at = item.at.max(now);
                    next = Some(next.map_or(at, |prior: Instant| prior.min(at)));
                }
            }
        }
        next
    }
    fn next_home(&mut self) -> Option<PathBuf> {
        for _ in 0..self.home_order.len() {
            let root = self.home_order.pop_front()?;
            let due = self
                .homes
                .get(&root)
                .is_some_and(|h| h.next_scan <= Instant::now())
                && !self.closing.contains(&root)
                && !self.closed.contains(&root);
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
    wake_driver();
    if state.closing.contains(root) || state.closed.contains(root) {
        state.pump(true);
        return;
    }
    if let Some(home) = state.homes.get_mut(root) {
        home.allowed = allowed;
    } else if state.can_admit_root(root) {
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

/// An explicit new-work entry may reopen a closed runtime. Normal Node::open
/// and passive reads intentionally do not call this.
pub(crate) fn acquire(root: &Path) -> Result<(), String> {
    let mut state = scheduler().lock().unwrap_or_else(|p| p.into_inner());
    if state.closing.contains(root) {
        return Err("RuntimeClosing".into());
    }
    if !state.can_admit_root(root) {
        return Err("ResourceLimit".into());
    }
    state.closed.remove(root);
    if !state.homes.contains_key(root) {
        state.home_order.push_back(root.into());
        state.homes.insert(
            root.into(),
            Home {
                cursor: None,
                next_scan: Instant::now(),
                // The normal entry supplies its real cached pause/rest state.
                allowed: false,
            },
        );
    }
    wake_driver();
    Ok(())
}
pub(crate) fn check_admission(root: &Path) -> Result<(), String> {
    if scheduler()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .closing
        .contains(root)
    {
        Err("RuntimeClosing".into())
    } else {
        Ok(())
    }
}
pub(crate) fn is_closed(root: &Path) -> bool {
    let state = scheduler().lock().unwrap_or_else(|p| p.into_inner());
    state.closing.contains(root) || state.closed.contains(root)
}
pub(crate) struct CloseGuard {
    root: PathBuf,
    steps: usize,
    metadata: bool,
}
impl Drop for CloseGuard {
    fn drop(&mut self) {
        let mut state = scheduler().lock().unwrap_or_else(|p| p.into_inner());
        state.closing.remove(&self.root);
        state.closing_steps -= self.steps;
        if self.metadata {
            state.closing_metadata = false;
        }
        wake_driver();
    }
}
/// Fence before probing a cold Root's existing locks, so even a Missing lock
/// cannot let a simultaneous explicit acquire cross the close operation.
pub(crate) fn begin_close(root: &Path) -> Result<Option<CloseGuard>, String> {
    let mut state = scheduler().lock().unwrap_or_else(|p| p.into_inner());
    if state.closing.contains(root) {
        return Ok(None);
    }
    if !state.can_admit_root(root) {
        return Ok(None);
    }
    state.closing.insert(root.into());
    wake_driver();
    Ok(Some(CloseGuard {
        root: root.into(),
        steps: 0,
        metadata: false,
    }))
}
impl CloseGuard {
    /// The caller holds admission and any existing executor claim. Existing owners
    /// retain their original deadline; joins never hold the scheduler mutex.
    pub(crate) fn finish(
        mut self,
        missing_claim: bool,
    ) -> Result<crate::khor_agent::RuntimeClose, String> {
        use crate::khor_agent::RuntimeClose;
        let root_path = self.root.clone();
        let root = root_path.as_path();
        let (mut metadata, mut steps, reserved) = {
            let mut state = scheduler().lock().unwrap_or_else(|p| p.into_inner());
            if state.retired.iter().any(|(path, _)| path == root) {
                return Ok(RuntimeClose::Busy);
            }
            if missing_claim
                && (state.metadata.as_ref().is_some_and(|m| m.root == root)
                    || state.steps.iter().any(|owner| owner.root == root))
            {
                return Ok(RuntimeClose::Busy);
            }
            let reserved = state.metadata.as_ref().and_then(|owner| {
                if owner.root == root {
                    if let Metadata::Persist { run, .. } = &owner.operation {
                        return Some(run.clone());
                    }
                }
                None
            });
            // Reserve a bounded retirement slot before taking a Persist owner.
            // Its real result may prove that the intent never committed.
            if reserved.is_some() && state.retired.len() >= RUN_LIMIT {
                return Ok(RuntimeClose::Busy);
            }
            if let Some(run) = &reserved {
                state.retired.push_back((root.into(), run.clone()));
            }
            let metadata = if state.metadata.as_ref().is_some_and(|m| m.root == root) {
                state.metadata.take()
            } else {
                None
            };
            if metadata.is_some() {
                state.closing_metadata = true;
            }
            let mut owned = Vec::new();
            for step in std::mem::take(&mut state.steps) {
                if step.root == root {
                    owned.push(step);
                } else {
                    state.steps.push(step);
                }
            }
            state.ready.retain(|item| item.root != root);
            state.durable.retain(|(path, _, _)| path != root);
            state.closing_steps += owned.len();
            (metadata, owned, reserved)
        };
        self.steps = steps.len();
        self.metadata = metadata.is_some();
        let mut error = None;
        let mut committed = reserved.is_none();
        if let Some(owner) = metadata.as_mut() {
            let result = owner.worker.join();
            if let Err(code) = &result {
                error = Some(code.clone());
            }
            if let Some(run) = &reserved {
                committed = result
                    .as_ref()
                    .ok()
                    .and_then(|raw| decode_reply(raw).ok())
                    .and_then(|reply| validated_snapshot(&reply, run, true))
                    .is_some_and(|snapshot| snapshot["durability"]["state"] == "DurablyQueued");
            }
            // Return exactly the worker's real reply to an existing coalesced
            // caller, without scheduling any further work for this closing Root.
            *owner.completion.lock().unwrap_or_else(|p| p.into_inner()) = Some(result);
        }
        for step in &mut steps {
            if let Err(code) = step.worker.join() {
                error.get_or_insert(code);
            }
        }
        let mut state = scheduler().lock().unwrap_or_else(|p| p.into_inner());
        if committed {
            if let Some(run) = reserved {
                state
                    .retired
                    .retain(|entry| entry != &(root.into(), run.clone()));
            }
        } else {
            return Ok(RuntimeClose::Busy);
        }
        if state.retired.iter().any(|(path, _)| path == root) {
            return Ok(RuntimeClose::Busy);
        }
        if let Some(code) = error {
            return Err(code);
        }
        state.homes.remove(root);
        state.home_order.retain(|path| path != root);
        state.closed.insert(root.into());
        Ok(RuntimeClose::Closed)
    }
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
    wake_driver();
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
    wake_driver();
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
        if state.closing.contains(root) {
            return serde_json::json!({"run":run,"durability":if persist {
                Durability::NotPersisted { code: "RuntimeClosing".into() }
            } else {
                Durability::Unavailable { code: "RuntimeClosing".into() }
            }});
        }
        if state.closed.contains(root) {
            if persist && retry {
                state.closed.remove(root);
            } else {
                return serde_json::json!({"run":run,"durability":if persist {
                    Durability::NotPersisted { code: "RuntimeClosed".into() }
                } else {
                    Durability::Unavailable { code: "RuntimeClosed".into() }
                }});
            }
        }
        if let Some(home) = state.homes.get_mut(root) {
            home.allowed = allowed;
        }
        if !state.homes.contains_key(root) && state.can_admit_root(root) {
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
        wake_driver();
        state.pump(false);
        // Reap before admission, without starting a periodic metadata scan.
        if !state.homes.contains_key(root) {
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
                // Stop has admitted a finite owned producer, but cannot claim
                // persistence before that producer reports its actual result.
                if persist && !retry {
                    break Err("MetadataOutcomeUnknown".into());
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

#[cfg(test)]
mod driver_tests {
    use super::*;

    fn ready_state(at: Instant) -> (Scheduler, PathBuf) {
        let root = PathBuf::from("/driver-policy-only");
        let mut state = Scheduler::default();
        state.homes.insert(
            root.clone(),
            Home {
                cursor: None,
                next_scan: Instant::now(),
                allowed: true,
            },
        );
        state.ready.push_back(Ready {
            root: root.clone(),
            run: "policy-only".into(),
            at,
            failures: 0,
        });
        (state, root)
    }

    #[test]
    fn future_ready_waits_for_its_deadline_and_paused_or_fenced_work_idles() {
        let now = Instant::now();
        let due = now + Duration::from_secs(30);
        let (mut state, root) = ready_state(due);
        assert_eq!(state.driver_deadline(now), Some(due));
        state.homes.get_mut(&root).unwrap().allowed = false;
        assert_eq!(state.driver_deadline(now), None);
        state.homes.get_mut(&root).unwrap().allowed = true;
        state.closing.insert(root.clone());
        assert_eq!(state.driver_deadline(now), None);
        state.closing.clear();
        state.closed.insert(root);
        assert_eq!(state.driver_deadline(now), None);
    }

    #[test]
    fn joining_quotas_do_not_busy_poll_and_retired_persist_does_not_discover() {
        let now = Instant::now();
        let (mut state, root) = ready_state(now);
        state.closing_steps = 4;
        assert_eq!(state.driver_deadline(now), None);
        state.retired.push_back((root, "policy-only".into()));
        state.closing_metadata = true;
        assert_eq!(state.driver_deadline(now), None);
        state.closing_metadata = false;
        assert_eq!(
            state.driver_deadline(now),
            Some(now + Duration::from_secs(1))
        );
        state.retired.clear();
        state.ready.clear();
        // A due Home scan does not create a driver deadline or periodic IO.
        assert_eq!(state.driver_deadline(now), None);
    }

    #[test]
    fn an_owned_worker_gets_finite_polling_until_its_real_thread_is_joined() {
        let now = Instant::now();
        let mut state = Scheduler::default();
        state.metadata = Some(MetadataOwner {
            root: PathBuf::from("/driver-policy-only"),
            operation: Metadata::Status {
                run: "policy-only".into(),
            },
            worker: OwnedWorker(Some(std::thread::spawn(|| Ok(String::new())))),
            completion: Arc::new(Mutex::new(None)),
        });
        assert_eq!(
            state.driver_deadline(now),
            Some(now + Duration::from_millis(5))
        );
        let mut owner = state.metadata.take().unwrap();
        assert!(owner.worker.join().is_ok());
        assert_eq!(state.driver_deadline(now), None);
    }
}
