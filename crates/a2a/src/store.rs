use crate::{
    AgentTarget, DeviceId, Error, InputResolution, Record, Result, SessionRef, Submission, TaskRef,
};
use a2a::{Message, Role, Task, TaskState, TaskStatus};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub const MAX_MESSAGE_BYTES: usize = 128 * 1024;
pub const MAX_TASK_BYTES: usize = 4 * 1024 * 1024;
const MAX_STORE_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone)]
pub struct Store {
    directory: PathBuf,
    home: DeviceId,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Ledger {
    version: u32,
    home: DeviceId,
    tasks: BTreeMap<String, Entry>,
    contexts: BTreeMap<String, Context>,
    messages: BTreeMap<String, Dedup>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Entry {
    pub task: Task,
    pub target: AgentTarget,
    pub caller: DeviceId,
    pub parent: Option<TaskRef>,
    pub revision: u64,
    pub cancellation_requested: bool,
    pub turn: u64,
    pub pending_message: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Context {
    target: AgentTarget,
    caller: DeviceId,
    session: Option<SessionRef>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Dedup {
    fingerprint: Option<String>,
    task: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    fence: Option<InputFence>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct InputFence {
    caller: DeviceId,
    target: AgentTarget,
    message: String,
    parent: Option<TaskRef>,
}

impl Dedup {
    fn validate(&self, caller: DeviceId, to: &AgentTarget, message: &str) -> Result<()> {
        match (&self.fingerprint, &self.task, &self.fence) {
            (Some(fingerprint), Some(task), None)
                if fingerprint.len() == 64
                    && fingerprint.bytes().all(|byte| byte.is_ascii_hexdigit()) =>
            {
                opaque(task)
            }
            (None, None, Some(fence))
                if fence.caller == caller
                    && fence.target == *to
                    && fence.message == message
                    && fence.parent.as_ref().is_none_or(|parent| {
                        parent.home == caller && opaque(&parent.id).is_ok()
                    }) =>
            {
                Ok(())
            }
            _ => Err(Error::Corrupt(
                "Input index has no valid acceptance or exact fence".into(),
            )),
        }
    }
}

fn input_resolution(
    home: DeviceId,
    state: &Ledger,
    caller: DeviceId,
    to: &AgentTarget,
    message: &str,
) -> Result<Option<InputResolution>> {
    let key = fingerprint(&(caller, &to.agent, message))?;
    let Some(prior) = state.messages.get(&key) else {
        return Ok(None);
    };
    prior.validate(caller, to, message)?;
    let Some(task) = &prior.task else {
        return Ok(Some(InputResolution::Fenced));
    };
    let entry = caller_entry(state, caller, to, task)?;
    let task = TaskRef {
        home,
        id: task.clone(),
    };
    if entry.pending_message != message {
        return Ok(Some(InputResolution::Superseded { task }));
    }
    Ok(Some(InputResolution::Known {
        task,
        state: entry.task.status.state.clone(),
        revision: entry.revision,
        cancellation_requested: entry.cancellation_requested,
    }))
}

fn cancel_entry(state: &mut Ledger, caller: DeviceId, to: &AgentTarget, id: &str) -> Result<()> {
    let entry = caller_entry(state, caller, to, id)?;
    if entry.task.status.state == TaskState::Canceled {
        return Ok(());
    }
    if entry.task.status.state.is_terminal() {
        return Err(Error::NotCancelable(id.into()));
    }
    let idle = entry.task.status.state != TaskState::Working;
    let entry = state.tasks.get_mut(id).expect("validated task");
    if !entry.cancellation_requested {
        entry.cancellation_requested = true;
        entry.revision += 1;
    }
    if idle {
        status(entry, TaskState::Canceled, None);
    }
    Ok(())
}

fn json_error(error: serde_json::Error) -> Error {
    Error::Corrupt(error.to_string())
}
fn opaque(value: &str) -> Result<()> {
    if value.trim().is_empty() || value.len() > 256 || value.chars().any(char::is_control) {
        return Err(Error::Invalid(
            "An identifier is empty, too long or contains control characters",
        ));
    }
    Ok(())
}
fn target(home: DeviceId, target: &AgentTarget) -> Result<()> {
    if target.home != home {
        return Err(Error::Invalid(
            "Delegation reached the wrong execution home",
        ));
    }
    opaque(&target.agent)
}
fn caller_entry<'a>(
    state: &'a Ledger,
    caller: DeviceId,
    target: &AgentTarget,
    id: &str,
) -> Result<&'a Entry> {
    state
        .tasks
        .get(id)
        .filter(|e| e.caller == caller && &e.target == target)
        .ok_or_else(|| Error::NotFound(id.into()))
}
fn record(state: &Ledger, id: &str) -> Result<Record> {
    let e = state
        .tasks
        .get(id)
        .ok_or_else(|| Error::NotFound(id.into()))?;
    let context = state
        .contexts
        .get(&e.task.context_id)
        .filter(|c| c.target == e.target && c.caller == e.caller)
        .ok_or_else(|| Error::Corrupt("Task context is missing or inconsistent".into()))?;
    Ok(Record {
        task: e.task.clone(),
        target: e.target.clone(),
        caller: e.caller,
        parent: e.parent.clone(),
        context_session: context.session.clone(),
        revision: e.revision,
        cancellation_requested: e.cancellation_requested,
    })
}
fn status(entry: &mut Entry, state: TaskState, mut message: Option<Message>) {
    if let Some(message) = &mut message {
        message.task_id = Some(entry.task.id.clone());
        message.context_id = Some(entry.task.context_id.clone());
    }
    if (state.is_terminal() || matches!(state, TaskState::InputRequired | TaskState::AuthRequired))
        && let Some(message) = &message
    {
        entry
            .task
            .history
            .get_or_insert_default()
            .push(message.clone());
    }
    entry.task.status = TaskStatus {
        state,
        message,
        timestamp: Some(std::time::SystemTime::now().into()),
    };
    entry.revision += 1;
}

// This hash is a retry fingerprint, not an authentication signature. Object
// key order must not turn equivalent metadata into a conflicting request.
fn fingerprint(value: &impl Serialize) -> Result<String> {
    fn ordered(value: serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::Object(map) => {
                let sorted: BTreeMap<_, _> =
                    map.into_iter().map(|(k, v)| (k, ordered(v))).collect();
                serde_json::Value::Object(sorted.into_iter().collect())
            }
            serde_json::Value::Array(values) => {
                serde_json::Value::Array(values.into_iter().map(ordered).collect())
            }
            other => other,
        }
    }
    let bytes = serde_json::to_vec(&ordered(serde_json::to_value(value).map_err(json_error)?))
        .map_err(json_error)?;
    Ok(blake3::hash(&bytes).to_hex().to_string())
}

fn private_file(path: &Path) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.create(true).truncate(false).read(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}
fn try_lease(path: &Path) -> Result<File> {
    let file = private_file(path)?;
    match file.try_lock() {
        Ok(()) => Ok(file),
        Err(std::fs::TryLockError::WouldBlock) => Err(Error::Busy),
        Err(std::fs::TryLockError::Error(error)) => Err(Error::Io(error)),
    }
}

impl Store {
    /// Open local execution records. The directory is deliberately outside the
    /// replicated document stores; importing display data must never run work.
    pub fn open(directory: impl Into<PathBuf>, home: DeviceId) -> Result<Self> {
        let directory = directory.into();
        let mut builder = fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(directory.join("leases"))?;
        Ok(Self { directory, home })
    }

    pub fn home(&self) -> DeviceId {
        self.home
    }

    fn load(&self) -> Result<Ledger> {
        let mut file = match File::open(self.directory.join("tasks.json")) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Ledger {
                    version: 1,
                    home: self.home,
                    tasks: BTreeMap::new(),
                    contexts: BTreeMap::new(),
                    messages: BTreeMap::new(),
                });
            }
            Err(e) => return Err(e.into()),
        };
        let mut bytes = Vec::new();
        Read::by_ref(&mut file)
            .take((MAX_STORE_BYTES + 1) as u64)
            .read_to_end(&mut bytes)?;
        if bytes.len() > MAX_STORE_BYTES {
            return Err(Error::Corrupt("Store exceeds its size limit".into()));
        }
        let state: Ledger = serde_json::from_slice(&bytes).map_err(json_error)?;
        if state.version != 1 || state.home != self.home {
            return Err(Error::Corrupt(
                "Unsupported store version or execution home".into(),
            ));
        }
        Ok(state)
    }

    fn save(&self, state: &Ledger) -> Result<()> {
        let bytes = serde_json::to_vec(state).map_err(json_error)?;
        if bytes.len() > MAX_STORE_BYTES {
            return Err(Error::Invalid("Delegation store is full"));
        }
        let temporary = self.directory.join(format!("{}.tmp", a2a::new_task_id()));
        let outcome = (|| -> Result<()> {
            let mut options = OpenOptions::new();
            options.create_new(true).write(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options.open(&temporary)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            fs::rename(&temporary, self.directory.join("tasks.json"))?;
            #[cfg(unix)]
            File::open(&self.directory)?.sync_all()?;
            Ok(())
        })();
        if outcome.is_err() {
            let _ = fs::remove_file(temporary);
        }
        outcome
    }

    fn transaction<T>(
        &self,
        write: bool,
        action: impl FnOnce(&mut Ledger) -> Result<T>,
    ) -> Result<T> {
        let gate = private_file(&self.directory.join("store.lock"))?;
        gate.lock()?;
        let mut state = self.load()?;
        let result = action(&mut state)?;
        if write {
            self.save(&state)?;
        }
        Ok(result)
    }

    fn lease(&self, context: &str) -> Result<File> {
        let name = blake3::hash(context.as_bytes()).to_hex().to_string();
        try_lease(&self.directory.join("leases").join(name))
    }

    /// Accept a client message after host authentication and policy checks.
    /// This only writes a receipt; dispatch is a separate, explicit action.
    pub fn accept(
        &self,
        caller: DeviceId,
        to: &AgentTarget,
        message: &Message,
        parent: Option<TaskRef>,
    ) -> Result<Submission> {
        target(self.home, to)?;
        opaque(&message.message_id)?;
        for id in [&message.task_id, &message.context_id]
            .into_iter()
            .flatten()
        {
            opaque(id)?;
        }
        if message.role != Role::User || message.parts.is_empty() {
            return Err(Error::Invalid(
                "A delegation requires a client message with content",
            ));
        }
        if let Some(parent) = &parent {
            opaque(&parent.id)?;
        }
        if serde_json::to_vec(message).map_err(json_error)?.len() > MAX_MESSAGE_BYTES {
            return Err(Error::Invalid("Delegation message exceeds its size limit"));
        }
        let key = fingerprint(&(caller, &to.agent, &message.message_id))?;
        let digest = fingerprint(&(to, message, &parent))?;
        self.transaction(true, |state| {
            if let Some(prior) = state.messages.get(&key) {
                prior.validate(caller, to, &message.message_id)?;
                let Some(task) = &prior.task else {
                    return Err(Error::InputFenced);
                };
                if prior.fingerprint.as_ref() != Some(&digest) {
                    return Err(Error::Conflict);
                }
                return Ok(Submission {
                    record: record(state, task)?,
                    accepted: false,
                });
            }
            let task_id;
            if let Some(id) = &message.task_id {
                let entry = caller_entry(state, caller, to, id)?;
                if entry.task.status.state.is_terminal() {
                    return Err(Error::Terminal);
                }
                if message
                    .context_id
                    .as_ref()
                    .is_some_and(|c| c != &entry.task.context_id)
                    || entry.parent != parent
                {
                    return Err(Error::Conflict);
                }
                if entry.cancellation_requested
                    || !matches!(
                        entry.task.status.state,
                        TaskState::InputRequired | TaskState::AuthRequired
                    )
                {
                    return Err(Error::Busy);
                }
                let _lease = self.lease(&entry.task.context_id)?;
                task_id = id.clone();
                let entry = state.tasks.get_mut(id).expect("validated task");
                let mut delivered = message.clone();
                delivered.context_id = Some(entry.task.context_id.clone());
                entry.task.history.get_or_insert_default().push(delivered);
                entry.pending_message = message.message_id.clone();
                status(entry, TaskState::Submitted, None);
            } else {
                let context = if let Some(id) = &message.context_id {
                    state
                        .contexts
                        .get(id)
                        .filter(|c| c.caller == caller && c.target == *to)
                        .ok_or_else(|| Error::NotFound(id.clone()))?;
                    id.clone()
                } else {
                    let id = a2a::new_context_id();
                    state.contexts.insert(
                        id.clone(),
                        Context {
                            target: to.clone(),
                            caller,
                            session: None,
                        },
                    );
                    id
                };
                task_id = a2a::new_task_id();
                let mut delivered = message.clone();
                delivered.task_id = Some(task_id.clone());
                delivered.context_id = Some(context.clone());
                state.tasks.insert(
                    task_id.clone(),
                    Entry {
                        task: Task {
                            id: task_id.clone(),
                            context_id: context,
                            status: TaskStatus {
                                state: TaskState::Submitted,
                                message: None,
                                timestamp: Some(std::time::SystemTime::now().into()),
                            },
                            artifacts: None,
                            history: Some(vec![delivered]),
                            metadata: None,
                        },
                        target: to.clone(),
                        caller,
                        parent,
                        revision: 1,
                        cancellation_requested: false,
                        turn: 0,
                        pending_message: message.message_id.clone(),
                    },
                );
            }
            let entry = state.tasks.get(&task_id).expect("accepted task");
            Self::bounded(entry)?;
            state.messages.insert(
                key,
                Dedup {
                    fingerprint: Some(digest),
                    task: Some(task_id.clone()),
                    fence: None,
                },
            );
            Ok(Submission {
                record: record(state, &task_id)?,
                accepted: true,
            })
        })
    }

    fn bounded(entry: &Entry) -> Result<()> {
        // A full working record must still have room for interruption/failure
        // metadata. Terminal receipts can use the reserved space.
        let limit = if entry.task.status.state.is_terminal() {
            MAX_TASK_BYTES
        } else {
            MAX_TASK_BYTES - 4096
        };
        if serde_json::to_vec(entry).map_err(json_error)?.len() > limit {
            return Err(Error::Invalid("Delegation task exceeds its size limit"));
        }
        Ok(())
    }

    pub fn get(&self, caller: DeviceId, to: &AgentTarget, id: &str) -> Result<Record> {
        target(self.home, to)?;
        self.transaction(false, |state| {
            caller_entry(state, caller, to, id)?;
            record(state, id)
        })
    }

    /// Deterministic task-ID cursor; no task histories are silently truncated.
    pub fn list(
        &self,
        caller: DeviceId,
        to: &AgentTarget,
        context: Option<&str>,
        after: Option<&str>,
        limit: usize,
    ) -> Result<Vec<Record>> {
        target(self.home, to)?;
        if !(1..=100).contains(&limit) {
            return Err(Error::Invalid("Page size must be between 1 and 100"));
        }
        self.transaction(false, |state| {
            state
                .tasks
                .values()
                .filter(|e| e.caller == caller && e.target == *to)
                .filter(|e| context.is_none_or(|c| e.task.context_id == c))
                .filter(|e| after.is_none_or(|id| e.task.id.as_str() > id))
                .take(limit)
                .map(|e| record(state, &e.task.id))
                .collect()
        })
    }

    /// Canceling a running task records intent. Only the executor can confirm
    /// that work actually stopped; a cancel request is never a rollback claim.
    pub fn request_cancel(&self, caller: DeviceId, to: &AgentTarget, id: &str) -> Result<Record> {
        target(self.home, to)?;
        self.transaction(true, |state| {
            // Submitted work has not started. Input/auth-required states are
            // quiescent by the executor contract; another task holding this
            // context's lease must not strand their cancellation.
            cancel_entry(state, caller, to, id)?;
            record(state, id)
        })
    }

    /// A read-only index observation. None is not proof that a late input is fenced.
    pub fn find_owned_message(
        &self,
        caller: DeviceId,
        to: &AgentTarget,
        message: &str,
    ) -> Result<Option<InputResolution>> {
        target(self.home, to)?;
        opaque(message)?;
        self.transaction(false, |state| {
            input_resolution(self.home, state, caller, to, message)
        })
    }

    /// Lookup, fence, and current-input cancellation share accept's write lock.
    /// Fences retain the exact caller/agent/message until explicit store removal;
    /// they use the existing bounded ledger, with no TTL or automatic pruning.
    pub fn cancel_input(
        &self,
        caller: DeviceId,
        to: &AgentTarget,
        message: &str,
        parent: Option<TaskRef>,
    ) -> Result<InputResolution> {
        target(self.home, to)?;
        opaque(message)?;
        if let Some(parent) = &parent {
            opaque(&parent.id)?;
            if parent.home != caller {
                return Err(Error::Invalid("Input parent belongs to another caller"));
            }
        }
        let key = fingerprint(&(caller, &to.agent, message))?;
        self.transaction(true, |state| {
            if let Some(prior) = state.messages.get(&key).cloned() {
                prior.validate(caller, to, message)?;
                if let Some(fence) = prior.fence {
                    if fence.parent != parent {
                        return Err(Error::Conflict);
                    }
                    return Ok(InputResolution::Fenced);
                }
                let id = prior.task.expect("validated input task");
                let entry = caller_entry(state, caller, to, &id)?;
                if entry.parent != parent {
                    return Err(Error::Conflict);
                }
                // A task can accept a later message. An old Stop must not
                // cancel that new turn between lookup and a separate Cancel.
                if entry.pending_message == message && !entry.task.status.state.is_terminal() {
                    cancel_entry(state, caller, to, &id)?;
                }
            } else {
                state.messages.insert(
                    key,
                    Dedup {
                        fingerprint: None,
                        task: None,
                        fence: Some(InputFence {
                            caller,
                            target: to.clone(),
                            message: message.into(),
                            parent,
                        }),
                    },
                );
            }
            input_resolution(self.home, state, caller, to, message)?
                .ok_or_else(|| Error::Corrupt("Input resolution disappeared".into()))
        })
    }

    pub(crate) fn claim(&self, id: &str, message: &str) -> Result<(File, u64, Record, Message)> {
        self.transaction(true, |state| {
            let entry = state
                .tasks
                .get(id)
                .ok_or_else(|| Error::NotFound(id.into()))?;
            if entry.task.status.state.is_terminal() {
                return Err(Error::Terminal);
            }
            if entry.task.status.state != TaskState::Submitted
                || entry.pending_message != message
                || entry.cancellation_requested
            {
                return Err(Error::Busy);
            }
            let lease = self.lease(&entry.task.context_id)?;
            if state.tasks.values().any(|other| {
                other.task.id != id
                    && other.task.context_id == entry.task.context_id
                    && other.task.status.state == TaskState::Working
            }) {
                // Reconcile an orphaned prior task before starting another.
                return Err(Error::Busy);
            }
            let input = entry
                .task
                .history
                .as_ref()
                .and_then(|h| h.iter().find(|m| m.message_id == message))
                .cloned()
                .ok_or_else(|| Error::Corrupt("Accepted input is missing".into()))?;
            let entry = state.tasks.get_mut(id).expect("validated task");
            entry.turn += 1;
            let turn = entry.turn;
            status(entry, TaskState::Working, None);
            Ok((lease, turn, record(state, id)?, input))
        })
    }

    pub(crate) fn executor_update(
        &self,
        id: &str,
        turn: u64,
        state: TaskState,
        message: Option<Message>,
        artifact: Option<a2a::Artifact>,
    ) -> Result<Record> {
        if matches!(state, TaskState::Unspecified | TaskState::Submitted) {
            return Err(Error::Invalid(
                "An executor cannot reset a task to submitted or unspecified",
            ));
        }
        if message
            .as_ref()
            .is_some_and(|m| m.role != Role::Agent || m.parts.is_empty())
        {
            return Err(Error::Invalid(
                "An executor status message must be an agent message",
            ));
        }
        if let Some(message) = &message {
            opaque(&message.message_id)?;
        }
        if let Some(artifact) = &artifact {
            opaque(&artifact.artifact_id)?;
            if artifact.parts.is_empty() {
                return Err(Error::Invalid("An artifact requires content"));
            }
        }
        self.transaction(true, |ledger| {
            let entry = ledger
                .tasks
                .get_mut(id)
                .ok_or_else(|| Error::NotFound(id.into()))?;
            if entry.task.status.state.is_terminal() {
                return Err(Error::Terminal);
            }
            if entry.turn != turn || entry.task.status.state == TaskState::Submitted {
                return Err(Error::Conflict);
            }
            if let Some(artifact) = artifact {
                let artifacts = entry.task.artifacts.get_or_insert_default();
                if let Some(prior) = artifacts
                    .iter_mut()
                    .find(|a| a.artifact_id == artifact.artifact_id)
                {
                    *prior = artifact;
                } else {
                    artifacts.push(artifact);
                }
            }
            status(entry, state, message);
            Self::bounded(entry)?;
            record(ledger, id)
        })
    }

    pub(crate) fn bind_session(&self, id: &str, turn: u64, session: SessionRef) -> Result<Record> {
        opaque(&session.id.0)?;
        self.transaction(true, |state| {
            let entry = state
                .tasks
                .get(id)
                .ok_or_else(|| Error::NotFound(id.into()))?;
            if entry.task.status.state.is_terminal() {
                return Err(Error::Terminal);
            }
            if entry.turn != turn || entry.task.status.state == TaskState::Submitted {
                return Err(Error::Conflict);
            }
            if session.home != self.home {
                return Err(Error::Invalid(
                    "A session binding must belong to the execution home",
                ));
            }
            let context_id = entry.task.context_id.clone();
            if state
                .contexts
                .iter()
                .any(|(id, c)| id != &context_id && c.session.as_ref() == Some(&session))
            {
                return Err(Error::Conflict);
            }
            let context = state
                .contexts
                .get_mut(&context_id)
                .ok_or_else(|| Error::Corrupt("Context missing".into()))?;
            if context.session.as_ref().is_some_and(|s| s != &session) {
                return Err(Error::Conflict);
            }
            if context.session.is_none() {
                context.session = Some(session);
                for entry in state
                    .tasks
                    .values_mut()
                    .filter(|e| e.task.context_id == context_id)
                {
                    entry.revision += 1;
                }
            }
            record(state, id)
        })
    }

    /// Startup reconciliation only. A released execution lease proves the
    /// previous worker is gone, not that its remote effects did not happen.
    /// Submitted tasks remain unexecuted; interrupted work is never requeued.
    pub fn recover(&self) -> Result<Vec<Record>> {
        self.transaction(true, |state| {
            let ids: Vec<_> = state
                .tasks
                .values()
                .filter(|e| e.task.status.state == TaskState::Working)
                .map(|e| (e.task.id.clone(), e.task.context_id.clone()))
                .collect();
            let mut recovered = Vec::new();
            for (id, context) in ids {
                let _lease = match self.lease(&context) {
                    Ok(lease) => lease,
                    Err(Error::Busy) => continue,
                    Err(error) => return Err(error),
                };
                let entry = state.tasks.get_mut(&id).expect("listed task");
                status(
                    entry,
                    TaskState::Failed,
                    Some(Message::new(
                        Role::Agent,
                        vec![a2a::Part::text(
                            "Execution was interrupted; the task was not restarted.",
                        )],
                    )),
                );
                recovered.push(record(state, &id)?);
            }
            Ok(recovered)
        })
    }
}
