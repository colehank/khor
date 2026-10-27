//! The network's agent: one executor, durable receipts, and a private Pi pipe.
//! Replicated receipts are display data and never enqueue model or tool work.
use crate::{AgentApproval, AgentConfig, AgentTurn, CommandRecord, Node};
use khor_catalog::msg;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
#[path = "agent_accounts.rs"]
mod accounts;
#[path = "khor_agent_input.rs"]
mod input;
#[path = "khor_agent_io.rs"]
mod io;
#[path = "khor_agent_output.rs"]
mod output;
use crate::agent_process as process;
#[path = "khor_agent_runtime.rs"]
pub(crate) mod runtime;
#[path = "khor_agent_updates.rs"]
mod updates;
use output::TextStream;
use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
};

#[cfg(all(test, unix))]
use std::process::Child;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Action {
    Read,
    Manage {
        request: Value,
    },
    Configure {
        account: String,
        model: String,
    },
    Prompt {
        token: String,
        text: String,
    },
    Answer {
        run: String,
        ask: String,
        allow: bool,
    },
    Stop {
        run: String,
    },
    CancellationStatus {
        run: String,
    },
    RetryCancellation {
        run: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Report {
    pub config: Option<AgentConfig>,
    pub run: Option<String>,
    #[serde(default)]
    pub data: Option<Value>,
}

/// Whether this process's agent producers and cancellation owners have retired.
/// Closing does not assert that any remote task has been canceled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeClose {
    Closed,
    Busy,
}

#[derive(Serialize, Deserialize)]
struct Saved {
    fingerprint: String,
    record: CommandRecord,
}

struct Running {
    id: String,
    input: Mutex<Option<input::Input>>,
    process: Arc<process::Control>,
    worker: Mutex<Option<Arc<process::Control>>>,
    path: OnceLock<String>,
    stopped: AtomicBool,
    approval: Mutex<Option<(String, Option<bool>)>>,
}

impl Running {
    fn new(id: String) -> Self {
        Self {
            id,
            input: Mutex::new(None),
            process: Arc::default(),
            worker: Mutex::new(None),
            path: OnceLock::new(),
            stopped: AtomicBool::new(false),
            approval: Mutex::new(None),
        }
    }
    fn request_stop(&self) {
        {
            // Registration and Stop share this lock, including the pre-spawn
            // window. A pre-stopped Control retires a racing spawn as well.
            let worker = self.worker.lock().unwrap_or_else(|p| p.into_inner());
            self.stopped.store(true, Ordering::SeqCst);
            if let Some(control) = worker.as_ref() {
                control.terminate_now();
            }
        }
        self.process.stop();
        if let Some(input) = self
            .input
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .as_ref()
        {
            input.abort();
        }
    }
    fn register_worker(&self, control: Arc<process::Control>) -> bool {
        let mut worker = self.worker.lock().unwrap_or_else(|p| p.into_inner());
        if self.stopped.load(Ordering::SeqCst) || worker.is_some() {
            return false;
        }
        *worker = Some(control);
        true
    }
    fn release_worker(&self, control: &Arc<process::Control>) {
        let mut worker = self.worker.lock().unwrap_or_else(|p| p.into_inner());
        if worker
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, control))
        {
            *worker = None;
        }
    }
}
struct Execution {
    child: process::Session,
    input: input::Input,
    writer: Option<std::thread::JoinHandle<()>>,
    reader: Option<std::thread::JoinHandle<()>>,
    drain: Option<std::thread::JoinHandle<()>>,
}
impl Execution {
    fn finish(&mut self, force: bool) -> std::io::Result<std::process::ExitStatus> {
        self.input.close();
        let exit = self.child.finish(force);
        for thread in [&mut self.writer, &mut self.reader, &mut self.drain] {
            if let Some(thread) = thread.take() {
                let _ = thread.join();
            }
        }
        exit
    }
}
impl Drop for Execution {
    fn drop(&mut self) {
        if self.writer.is_some() {
            let _ = self.finish(true);
        }
    }
}
fn running() -> &'static Mutex<HashMap<PathBuf, Arc<Running>>> {
    static RUNS: OnceLock<Mutex<HashMap<PathBuf, Arc<Running>>>> = OnceLock::new();
    RUNS.get_or_init(Default::default)
}
fn stop_local_producers(root: &Path) {
    accounts::stop_jobs(root);
    crate::acp_management::stop_jobs(root);
    let active = running()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .get(root)
        .cloned();
    if let Some(active) = active {
        active.request_stop();
    }
}
fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
pub fn providers() -> Value {
    accounts::catalog()
}
pub fn account_provider(account: &crate::Account) -> String {
    accounts::provider_id(account)
}
pub fn account_ready(node: &Node, account: &crate::Account) -> bool {
    accounts::inference_ready(node, account)
}
fn directory(root: &Path) -> PathBuf {
    root.join(".khor/agent")
}
fn file(root: &Path, id: &str) -> PathBuf {
    directory(root).join("runs").join(format!("{id}.json"))
}
fn safe_token(token: &str) -> bool {
    (16..=64).contains(&token.len())
        && token
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
}
fn configuration_lock(root: &Path) -> Result<File, String> {
    fs::create_dir_all(directory(root)).map_err(|e| e.to_string())?;
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(directory(root).join("config.lock"))
        .map_err(|e| e.to_string())?;
    file.lock().map_err(|e| e.to_string())?;
    Ok(file)
}
fn claim(root: &Path) -> Result<File, String> {
    try_claim(root)?.ok_or_else(|| msg::agent_busy().to_owned())
}
fn try_claim(root: &Path) -> Result<Option<File>, String> {
    fs::create_dir_all(directory(root).join("runs")).map_err(|e| e.to_string())?;
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(directory(root).join("executor.lock"))
        .map_err(|e| e.to_string())?;
    match lock.try_lock() {
        Ok(()) => Ok(Some(lock)),
        Err(std::fs::TryLockError::WouldBlock) => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

enum CloseClaim {
    Held(File),
    Busy,
    Missing,
}
fn close_claim(root: &Path, admission: bool) -> Result<CloseClaim, String> {
    // Only the existing agent directory may gain its admission lock. Closing
    // a cold Root must never initialize the directory or an executor claim.
    let path = directory(root).join(if admission {
        "config.lock"
    } else {
        "executor.lock"
    });
    let file = match OpenOptions::new()
        .create(admission)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path)
    {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(CloseClaim::Missing);
        }
        Err(error) => return Err(error.to_string()),
    };
    match file.try_lock() {
        Ok(()) => Ok(CloseClaim::Held(file)),
        Err(std::fs::TryLockError::WouldBlock) => Ok(CloseClaim::Busy),
        Err(error) => Err(error.to_string()),
    }
}
fn prompt_admission(root: &Path) -> Result<File, String> {
    crate::remote_cancel::check_admission(root)?;
    fs::create_dir_all(directory(root)).map_err(|e| e.to_string())?;
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(directory(root).join("config.lock"))
        .map_err(|e| e.to_string())?;
    file.try_lock().map_err(|_| msg::agent_busy().to_owned())?;
    crate::remote_cancel::acquire(root)?;
    Ok(file)
}
fn spawn_executor<F>(root: &Path, execute: F) -> std::io::Result<std::thread::JoinHandle<()>>
where
    F: FnOnce() + Send + 'static,
{
    #[cfg(all(test, unix))]
    if runtime_close_tests::fail_spawn(root) {
        return Err(std::io::Error::other("controlled executor factory failure"));
    }
    let _ = root;
    std::thread::Builder::new()
        .name("khor-agent".into())
        .spawn(execute)
}
impl Node {
    /// Explicitly acquire this process's cancellation runtime for new work.
    /// Ordinary reads and resident ticks never reopen a successfully closed Root.
    pub fn acquire_agent_runtime(root: &Path) -> Result<(), String> {
        let _call = runtime::enter(root, &OnceLock::new())?;
        crate::remote_cancel::acquire(root)
    }
    /// Seal agent API admission for this process, stop/join its executor and
    /// provider/ACP setup producers, then retire its cancellation owners. A Busy result retains the terminal seal
    /// and the Root; the owner may retry. Remote cancellation can remain pending.
    /// Durable GUI hosts and unrelated Node I/O keep their separate owners.
    pub async fn shutdown_agent_runtime(root: &Path) -> Result<RuntimeClose, String> {
        Self::retire_agent_runtime(root, true).await
    }
    pub(crate) async fn retire_agent_runtime(
        root: &Path,
        terminal: bool,
    ) -> Result<RuntimeClose, String> {
        let Some(_drain) = runtime::drain(root, terminal)? else {
            return Ok(RuntimeClose::Busy);
        };
        let by = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            // Repeat while admitted calls retire: a Prompt may have crossed
            // admission just before the seal and not installed Running yet.
            stop_local_producers(root);
            if runtime::idle(root) {
                match Self::close_agent_runtime(root).await? {
                    RuntimeClose::Closed => {
                        runtime::retired(root);
                        return Ok(RuntimeClose::Closed);
                    }
                    RuntimeClose::Busy => {}
                }
            }
            if tokio::time::Instant::now() >= by {
                return Ok(RuntimeClose::Busy);
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    }
    /// Quiesce this Root's local agent runtime before its owner releases it.
    /// Busy callers retain the Root; durable cancellation files remain on disk.
    pub async fn close_agent_runtime(root: &Path) -> Result<RuntimeClose, String> {
        let root = root.to_path_buf();
        tokio::task::spawn_blocking(move || {
            let Some(closing) = crate::remote_cancel::begin_close(&root)? else {
                return Ok(RuntimeClose::Busy);
            };
            #[cfg(all(test, unix))]
            runtime_close_tests::producer_point(&root, "close-reserved");
            // Match Prompt's admission -> executor ordering. Both files remain
            // held through all owner joins, even if the async waiter is dropped.
            let _admission = match close_claim(&root, true)? {
                CloseClaim::Held(file) => Some(file),
                CloseClaim::Missing => None,
                CloseClaim::Busy => return Ok(RuntimeClose::Busy),
            };
            let (missing_claim, _executor) = match close_claim(&root, false)? {
                CloseClaim::Held(file) => (false, Some(file)),
                CloseClaim::Missing => (true, None),
                CloseClaim::Busy => return Ok(RuntimeClose::Busy),
            };
            if running()
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .contains_key(&root)
            {
                return Ok(RuntimeClose::Busy);
            }
            closing.finish(missing_claim)
        })
        .await
        .map_err(|_| "Agent runtime close owner failed".to_owned())?
    }
}

fn save(root: &Path, saved: &Saved) -> Result<(), String> {
    let mut saved = Saved {
        fingerprint: saved.fingerprint.clone(),
        record: saved.record.clone(),
    };
    if let Some(agent) = saved.record.agent.as_mut() {
        let previous = fs::read(file(root, &saved.record.id))
            .ok()
            .and_then(|b| serde_json::from_slice::<Saved>(&b).ok())
            .and_then(|s| s.record.agent)
            .map_or(saved.record.started_at_ms, |a| a.updated_at_ms);
        agent.updated_at_ms = now().max(previous + 1);
    }
    let target = file(root, &saved.record.id);
    let temp = target.with_extension("tmp");
    crate::link::write_private(
        &temp,
        &serde_json::to_vec(&saved).map_err(|e| e.to_string())?,
    )?;
    File::open(&temp)
        .and_then(|f| f.sync_all())
        .map_err(|e| e.to_string())?;
    fs::rename(&temp, &target).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    File::open(target.parent().expect("agent receipt directory"))
        .and_then(|f| f.sync_all())
        .map_err(|e| e.to_string())?;
    Node::open(root.to_path_buf())?.record_command(&saved.record)?;
    Ok(())
}
fn send(active: &Running, value: &Value) -> Result<(), String> {
    let input = active
        .input
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or_else(|| msg::agent_not_running().to_owned())?;
    let mut bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    input.send(bytes)
}

pub(crate) fn configuration_reply(action: &Action, report: &Report) -> bool {
    matches!(action, Action::Configure { .. })
        || matches!(action,Action::Manage{request}if request["operation"]=="poll"&&report.data.as_ref().is_some_and(|data|data["status"]=="done"&&data["result"]["options"].is_object()&&data["result"]["configOptions"].is_array()))
}

impl Node {
    pub fn khor_agent_config(&self) -> Result<Option<AgentConfig>, String> {
        self.chat.agent_config()
    }

    /// Only called on the selected home, through the authenticated node route.
    pub(crate) async fn khor_agent_here(&self, action: Action) -> Result<Report, String> {
        let _call = runtime::enter(self.root(), &self.agent_generation)?;
        if let Action::Stop { run } = &action {
            let active = running()
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .get(self.root())
                .filter(|r| &r.id == run)
                .cloned()
                .ok_or_else(|| msg::agent_not_running().to_owned())?;
            active.request_stop();
            let data = crate::remote_cancel::request(
                self.root(),
                run,
                true,
                !self.paused() && !self.resting.load(Ordering::Relaxed),
                false,
            )
            .await;
            return Ok(Report {
                config: self.khor_agent_config().ok().flatten(),
                run: Some(run.clone()),
                data: Some(data),
            });
        }
        if let Action::CancellationStatus { run } | Action::RetryCancellation { run } = &action {
            let data = crate::remote_cancel::request(
                self.root(),
                run,
                matches!(action, Action::RetryCancellation { .. }),
                !self.paused() && !self.resting.load(Ordering::Relaxed),
                true,
            )
            .await;
            return Ok(Report {
                config: self.khor_agent_config().ok().flatten(),
                run: Some(run.clone()),
                data: Some(data),
            });
        }
        self.reconcile_agent()?;
        let root = self.root().to_path_buf();
        let mut report = Report {
            config: self.khor_agent_config()?,
            run: None,
            data: None,
        };
        match action {
            Action::Manage { request } => {
                report.data = Some(self.manage_agent(request)?);
                report.config = self.khor_agent_config()?;
            }
            Action::Read => {
                self.reconcile_agent()?;
            }
            Action::Configure { account, model } => {
                self.refuse_unless_hosting()?;
                let account = self.agent_account(&account)?;
                if model.trim().is_empty()
                    || model.len() > 256
                    || model.chars().any(char::is_control)
                {
                    return Err(msg::agent_model_required().to_owned());
                }
                let _configuration = configuration_lock(&root)?;
                let current = self.khor_agent_config()?;
                let config = AgentConfig {
                    home: self.device_str().into(),
                    account: account.id.clone(),
                    model: model.trim().into(),
                    options: current
                        .as_ref()
                        .filter(|c| c.account == account.id && c.model == model.trim())
                        .map(|c| c.options.clone())
                        .unwrap_or_default(),
                };
                self.chat.set_agent_config(&config)?;
                let _ = self.emit_row_of(&crate::SessionId(crate::wide::BROADCAST.into()));
                report.config = Some(config);
            }
            Action::Prompt { token, text } => {
                self.refuse_unless_hosting()?;
                let _admission = prompt_admission(&root)?;
                crate::remote_cancel::pump(
                    &root,
                    !self.paused() && !self.resting.load(Ordering::Relaxed),
                );
                if !safe_token(&token) || text.trim().is_empty() || text.len() > 128 * 1024 {
                    return Err(msg::agent_arguments().to_owned());
                }
                let config = report
                    .config
                    .clone()
                    .filter(|c| c.home == self.device_str())
                    .ok_or_else(|| msg::agent_not_configured().to_owned())?;
                let account = self.agent_account(&config.account)?;
                let key = if account.inference.is_some() {
                    String::new()
                } else {
                    fs::read_to_string(self.root().join(".khor/keys").join(&account.id))
                        .map_err(|_| msg::agent_key_missing().to_owned())?
                };
                let id = format!("agent-{}-{token}", &self.device_str()[..16]);
                let fingerprint = blake3::hash(
                    &serde_json::to_vec(&(&text, &config)).map_err(|e| e.to_string())?,
                )
                .to_hex()
                .to_string();
                // The receipt may be read without the executor lock: final files
                // are atomically renamed, and a retry must work while it runs.
                if file(&root, &id).try_exists().map_err(|e| e.to_string())? {
                    let prior: Saved = serde_json::from_slice(
                        &fs::read(file(&root, &id)).map_err(|e| e.to_string())?,
                    )
                    .map_err(|e| e.to_string())?;
                    if prior.fingerprint != fingerprint {
                        return Err(msg::console_token_conflict().to_owned());
                    }
                    #[cfg(all(test, unix))]
                    runtime_close_tests::producer_point(&root, "retry-record");
                    self.record_command(&prior.record)?;
                    report.run = Some(id);
                    return Ok(report);
                }
                let claim = Arc::new(claim(&root)?);
                // Re-check under the cross-process executor claim.
                if file(&root, &id).exists() {
                    return Err(msg::agent_busy().to_owned());
                }
                let saved = Saved {
                    fingerprint,
                    record: CommandRecord {
                        id: id.clone(),
                        line: text.trim().into(),
                        owner: self.device_str().into(),
                        owner_name: self.shown_name().into(),
                        status: "running".into(),
                        started_at_ms: now(),
                        agent: Some(AgentTurn {
                            model: config.model.clone(),
                            approval: None,
                            updated_at_ms: 0,
                            updates: Vec::new(),
                            updates_truncated: false,
                        }),
                        ..Default::default()
                    },
                };
                save(&root, &saved)?;
                let active = Arc::new(Running::new(id.clone()));
                running()
                    .lock()
                    .map_err(|e| e.to_string())?
                    .insert(root.clone(), active.clone());
                let mut start = json!({"type":"start","protocol":1,"run":id,"text":text,"stateDir":directory(&root).join("sessions"),"model":config.model,"options":config.options,
                    "account":{"provider":account.provider.word(),"baseUrl":account.base_url,"key":key.trim()}});
                if let Some(inference) = &account.inference {
                    start["account"] =
                        json!({"provider":inference.provider,"baseUrl":inference.base_url});
                    start["credentialPath"] = json!(accounts::credential_path(&root, &account.id));
                }
                let recovery_root = root.clone();
                let recovery_id = id.clone();
                let executor_claim = claim.clone();
                let spawned = spawn_executor(&recovery_root, move || {
                    let _claim = executor_claim;
                    #[cfg(all(test, unix))]
                    runtime_close_tests::producer_point(&root, "executor-start");
                    let id = saved.record.id.clone();
                    if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        execute(root.clone(), saved, active, start, key.trim());
                    }))
                    .is_err()
                    {
                        if let Ok(bytes) = fs::read(file(&root, &id)) {
                            if let Ok(mut saved) = serde_json::from_slice::<Saved>(&bytes) {
                                saved.record.status = "interrupted".into();
                                saved.record.error = Some(msg::console_interrupted().into());
                                saved.record.ended_at_ms =
                                    Some(now().max(saved.record.started_at_ms + 1));
                                if let Some(agent) = saved.record.agent.as_mut() {
                                    agent.approval = None;
                                }
                                let _ = save(&root, &saved);
                            }
                        }
                    }
                    running()
                        .lock()
                        .unwrap_or_else(|p| p.into_inner())
                        .remove(&root);
                });
                match spawned {
                    Ok(executor) => runtime::register_producer(&recovery_root, executor),
                    Err(error) => {
                        running()
                            .lock()
                            .unwrap_or_else(|p| p.into_inner())
                            .remove(&recovery_root);
                        let mut saved: Saved = serde_json::from_slice(
                            &fs::read(file(&recovery_root, &recovery_id))
                                .map_err(|e| e.to_string())?,
                        )
                        .map_err(|e| e.to_string())?;
                        saved.record.status = "failed".into();
                        saved.record.error = Some(error.to_string());
                        saved.record.ended_at_ms = Some(now());
                        #[cfg(all(test, unix))]
                        runtime_close_tests::producer_point(&recovery_root, "spawn-failure-save");
                        save(&recovery_root, &saved)?;
                    }
                }
                report.run = Some(id);
            }
            Action::Answer { run, ask, allow } => {
                let active = running()
                    .lock()
                    .map_err(|e| e.to_string())?
                    .get(&root)
                    .filter(|r| r.id == run)
                    .cloned()
                    .ok_or_else(|| msg::agent_not_running().to_owned())?;
                let mut pending = active.approval.lock().map_err(|e| e.to_string())?;
                let Some((id, answer)) = pending
                    .as_mut()
                    .filter(|(id, answer)| id == &ask && answer.is_none())
                else {
                    return Err(msg::agent_approval_expired().to_owned());
                };
                let _ = id;
                *answer = Some(allow);
                report.run = Some(run);
            }
            Action::Stop { .. }
            | Action::CancellationStatus { .. }
            | Action::RetryCancellation { .. } => unreachable!("handled before reconciliation"),
        }
        Ok(report)
    }

    fn agent_account(&self, id: &str) -> Result<crate::Account, String> {
        self.accounts()?
            .into_iter()
            .find(|a| a.id == id && a.home == self.device_str())
            .filter(|a| accounts::inference_ready(self, a))
            .ok_or_else(|| msg::agent_key_missing().to_owned())
    }

    pub fn reconcile_agent(&self) -> Result<(), String> {
        let Ok(_call) = runtime::enter(self.root(), &self.agent_generation) else {
            return Ok(());
        };
        if crate::remote_cancel::is_closed(self.root()) {
            return Ok(());
        }
        // A stale passive reader must not recreate directories after close.
        // The same existing-directory admission guard also covers its display
        // receipt writes; unlike a Prompt this never explicitly reopens a Root.
        let _admission = match close_claim(self.root(), true)? {
            CloseClaim::Held(file) => file,
            CloseClaim::Busy | CloseClaim::Missing => return Ok(()),
        };
        if crate::remote_cancel::is_closed(self.root()) {
            return Ok(());
        }
        crate::remote_cancel::pump(
            self.root(),
            !self.paused() && !self.resting.load(Ordering::Relaxed),
        );
        if !self.hosts() {
            return Ok(());
        }
        if !directory(self.root()).join("runs").is_dir() {
            return Ok(());
        }
        let Ok(_claim) = claim(self.root()) else {
            return Ok(());
        };
        // A passive read may have crossed the close before obtaining the
        // claim. Recheck while holding it before touching any display receipt.
        if crate::remote_cancel::is_closed(self.root()) {
            return Ok(());
        }
        let recorded: HashMap<_, _> = self
            .chat
            .log(crate::wide::BROADCAST)?
            .messages
            .into_iter()
            .filter_map(|m| m.command.map(|c| (c.id.clone(), c)))
            .collect();
        for entry in fs::read_dir(directory(self.root()).join("runs"))
            .map_err(|e| e.to_string())?
            .flatten()
        {
            if entry.path().extension().is_none_or(|e| e != "json") {
                continue;
            }
            let Ok(bytes) = fs::read(entry.path()) else {
                continue;
            };
            let Ok(mut saved) = serde_json::from_slice::<Saved>(&bytes) else {
                continue;
            };
            if saved.record.ended_at_ms.is_none() {
                saved.record.status = "interrupted".into();
                saved.record.error = Some(msg::console_interrupted().into());
                saved.record.ended_at_ms = Some(now().max(saved.record.started_at_ms + 1));
                if let Some(agent) = saved.record.agent.as_mut() {
                    agent.approval = None;
                }
                save(self.root(), &saved)?;
            } else if recorded.get(&saved.record.id) != Some(&saved.record) {
                self.record_command(&saved.record)?;
            }
        }
        Ok(())
    }
}

fn sidecar(root: &Path) -> Result<Command, String> {
    if let Some(path) = std::env::var_os("KHOR_AGENT_SIDECAR") {
        return Ok(Command::new(path));
    }
    #[cfg(khor_embedded_agent)]
    {
        const BUNDLE: &[u8] = include_bytes!(env!("KHOR_AGENT_BUNDLE"));
        let digest = blake3::hash(BUNDLE).to_hex().to_string();
        let dir = directory(root).join("runtime").join(&digest);
        let path = dir.join(if cfg!(windows) {
            "khor-agent.exe"
        } else {
            "khor-agent"
        });
        if !fs::read(&path).is_ok_and(|bytes| blake3::hash(&bytes).to_hex().as_str() == digest) {
            fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let temp = path.with_extension("tmp");
            crate::link::write_private(&temp, BUNDLE)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&temp, fs::Permissions::from_mode(0o700))
                    .map_err(|e| e.to_string())?;
            }
            fs::rename(temp, &path).map_err(|e| e.to_string())?;
        }
        Ok(Command::new(path))
    }
    #[cfg(not(khor_embedded_agent))]
    {
        let _ = root;
        let exe = crate::self_exe()?;
        let bin = exe
            .parent()
            .unwrap_or(Path::new("."))
            .join(if cfg!(windows) {
                "khor-agent.exe"
            } else {
                "khor-agent"
            });
        if bin.is_file() {
            return Ok(Command::new(bin));
        }
        if cfg!(debug_assertions) {
            let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/agent/src/host.ts");
            if script.is_file() {
                let mut c = Command::new("node");
                c.arg(script);
                return Ok(c);
            }
        }
        Err(msg::agent_runtime_missing().into())
    }
}

fn execute(root: PathBuf, mut saved: Saved, active: Arc<Running>, start: Value, key: &str) {
    let mut text_output = TextStream::new(key);
    let mut snapshots = updates::Recorder::new(key);
    let mut process_stopped = true;
    let outcome = (|| -> Result<(), String> {
        if active.stopped.load(Ordering::SeqCst) {
            return Ok(());
        }
        // Extraction and hashing of the bundled runtime stay off the reactor.
        let mut command = sidecar(&root)?;
        // Pass only operating-system settings. Provider keys, tokens, Pi config
        // and MCP credentials from the parent environment cannot select auth.
        command.env_clear();
        for name in [
            "HOME",
            "USERPROFILE",
            "SYSTEMROOT",
            "WINDIR",
            "TMPDIR",
            "TEMP",
            "LANG",
            "LC_ALL",
            "SSL_CERT_FILE",
            "SSL_CERT_DIR",
            "HTTP_PROXY",
            "HTTPS_PROXY",
            "ALL_PROXY",
            "NO_PROXY",
        ] {
            if let Some(value) = std::env::var_os(name) {
                command.env(name, value);
            }
        }
        let path = Node::user_path().to_owned();
        let _ = active.path.set(path.clone());
        command
            .env("PATH", path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if active.stopped.load(Ordering::SeqCst) {
            return Ok(());
        }
        let (child, pipes) = process::Session::spawn(command, active.process.clone())
            .map_err(|e| msg::agent_runtime_failed(e))?;
        process_stopped = false;
        let (input, writer) = input::Input::spawn(pipes.stdin).map_err(|e| e.to_string())?;
        {
            let mut current = active.input.lock().unwrap_or_else(|p| p.into_inner());
            if active.stopped.load(Ordering::SeqCst) {
                input.abort();
            }
            *current = Some(input.clone());
        }
        let mut execution = Execution {
            child,
            input,
            writer: Some(writer),
            reader: None,
            drain: None,
        };
        execution.drain = Some(std::thread::spawn(move || io::drain_stderr(pipes.stderr)));
        let stdout = pipes.stdout;
        // Keep consuming cancellation/EOF while a person is considering an
        // approval. Blocking the reader here could execute an expired request.
        let (out, events) = mpsc::sync_channel(64);
        execution.reader = Some(std::thread::spawn(move || {
            for line in io::event_lines(stdout) {
                let line = line.map_err(|error| match error {
                    io::EventError::Read(error) => error.to_string(),
                    io::EventError::TooLarge => msg::agent_arguments().into(),
                });
                if out.send(line).is_err() {
                    break;
                }
            }
        }));
        let mut completed = false;
        let mut pending = McpSession::default();
        let mut last_flush = std::time::Instant::now();
        let read_result = (|| -> Result<(), String> {
            send(&active, &start)?;
            loop {
                let line = match events.recv_timeout(std::time::Duration::from_millis(100)) {
                    Ok(line) => line?,
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        // Process all queued cancellation messages before
                        // consuming a user's answer and starting a write.
                        if let Some(response) = pending.poll(&root, &mut saved, &active)? {
                            send_mcp(&active, response)?;
                        }
                        continue;
                    }
                };
                let event: Value =
                    serde_json::from_str(&line).map_err(|_| msg::agent_arguments().to_owned())?;
                match event["type"].as_str() {
                    Some("mcp") => {
                        if let Some(response) =
                            pending.message(&root, &mut saved, &active, &event["message"])?
                        {
                            send_mcp(&active, response)?;
                        }
                    }
                    Some("update") => {
                        // Validate the Pi display bridge with the same ACP v1
                        // schema used for Claude and Codex. Receipts never run it.
                        let update: agent_client_protocol::schema::v1::SessionUpdate =
                            serde_json::from_value(event["update"].clone())
                                .map_err(|_| msg::agent_arguments().to_owned())?;
                        let value = serde_json::to_value(update).map_err(|e| e.to_string())?;
                        if value["sessionUpdate"] == "agent_message_chunk"
                            && value["content"]["type"] == "text"
                        {
                            text_output.append_to(
                                &mut saved.record.stdout,
                                value["content"]["text"].as_str().unwrap_or(""),
                            );
                        }
                        snapshots.push(saved.record.agent.as_mut().expect("agent turn"), value);
                        if last_flush.elapsed() >= std::time::Duration::from_millis(200) {
                            save(&root, &saved)?;
                            last_flush = std::time::Instant::now();
                        }
                    }
                    Some("settings") => {
                        let value = json!({"sessionUpdate":"config_option_update","configOptions":event["settings"]["configOptions"]});
                        let update: agent_client_protocol::schema::v1::SessionUpdate =
                            serde_json::from_value(value)
                                .map_err(|_| msg::agent_arguments().to_owned())?;
                        snapshots.push(
                            saved.record.agent.as_mut().expect("agent turn"),
                            serde_json::to_value(update).map_err(|e| e.to_string())?,
                        );
                    }
                    Some("text") => {
                        text_output.append_to(
                            &mut saved.record.stdout,
                            event["delta"].as_str().unwrap_or(""),
                        );
                        if last_flush.elapsed() >= std::time::Duration::from_millis(200) {
                            save(&root, &saved)?;
                            last_flush = std::time::Instant::now();
                        }
                    }
                    Some("tool") => {}
                    Some("done") => {
                        completed = true;
                        break;
                    }
                    Some("failed") => {
                        return Err(event["error"]
                            .as_str()
                            .unwrap_or(msg::agent_failed())
                            .to_owned());
                    }
                    _ => return Err(msg::agent_arguments().into()),
                }
                // Running work can complete while events remain queued. Only
                // the idle branch may consume an approval: queued cancellation
                // notifications must win over a previously submitted answer.
                if let Some(response) = pending.poll_worker(&active)? {
                    send_mcp(&active, response)?;
                }
            }
            if !completed {
                return Err(msg::agent_failed().into());
            }
            Ok(())
        })();
        pending.retire(&active)?;
        active
            .input
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .take();
        // The supervisor progresses independently of blocked pipe operations.
        // Only its successful wait confirms that the owned process was reaped.
        drop(events);
        let exit = execution.finish(read_result.is_err());
        process_stopped = exit.is_ok();
        let exit = exit.map_err(|e| e.to_string())?;
        read_result?;
        if !exit.success() {
            return Err(msg::agent_failed().into());
        }
        Ok(())
    })();
    text_output.finish_into(&mut saved.record.stdout);
    if let Some(agent) = saved.record.agent.as_mut() {
        snapshots.finish(agent);
    }
    let stopped = active.stopped.load(Ordering::SeqCst) && process_stopped;
    saved.record.status = if stopped {
        "cancelled"
    } else if outcome.is_ok() {
        "done"
    } else {
        "failed"
    }
    .into();
    saved.record.error = if stopped {
        None
    } else {
        // Raw failures reach this display boundary exactly once.
        outcome.err().map(|e| output::redact(&e, key).into_owned())
    };
    if stopped {
        saved.record.stderr = msg::agent_stopped().into();
    }
    saved.record.ended_at_ms = Some(now().max(saved.record.started_at_ms + 1));
    *active.approval.lock().unwrap_or_else(|p| p.into_inner()) = None;
    if let Some(agent) = saved.record.agent.as_mut() {
        agent.approval = None;
    }
    let _ = save(&root, &saved);
    if stopped || saved.record.status == "failed" {
        crate::remote_cancel::local_retired(root, active.id.clone());
    }
}

#[derive(Default)]
struct McpSession {
    pending: Option<Value>,
    progress: Option<std::time::Instant>,
    progress_count: u64,
    operation: Option<(Value, crate::mcp_worker::Worker)>,
}
impl McpSession {
    fn retire(&mut self, active: &Running) -> Result<(), String> {
        if let Some((_, mut worker)) = self.operation.take() {
            let retired = worker.cancel_and_join();
            active.release_worker(&worker.control);
            retired?;
        }
        Ok(())
    }

    fn start(
        &mut self,
        root: &Path,
        active: &Running,
        request: &Value,
        authorized: bool,
    ) -> Result<Option<Value>, String> {
        let id = &request["id"];
        if self.operation.is_some() || self.pending.is_some() {
            return Ok(Some(tool_error(id, msg::agent_busy())));
        }
        let control = Arc::new(process::Control::default());
        if !active.register_worker(control.clone()) {
            return Ok(Some(tool_error(id, msg::agent_stopped())));
        }
        let request = crate::mcp_worker::Request {
            root: root.to_path_buf(),
            operation: crate::mcp_worker::Operation::Tool {
                run: active.id.clone(),
                authorized,
                name: request["params"]["name"].as_str().unwrap_or("").into(),
                args: request["params"]["arguments"].clone(),
            },
        };
        match crate::mcp_worker::Worker::spawn(
            request,
            control.clone(),
            crate::mcp_worker::TOOL_BUDGET,
            active.path.get().cloned(),
        ) {
            Ok(worker) => {
                self.operation = Some((id.clone(), worker));
                Ok(None)
            }
            Err(error) => {
                active.release_worker(&control);
                Ok(Some(tool_error(id, &error)))
            }
        }
    }
    fn clear(&mut self, root: &Path, saved: &mut Saved, active: &Running) -> Result<(), String> {
        self.pending = None;
        *active.approval.lock().map_err(|e| e.to_string())? = None;
        saved.record.status = "running".into();
        saved.record.agent.as_mut().expect("agent turn").approval = None;
        save(root, saved)
    }

    fn message(
        &mut self,
        root: &Path,
        saved: &mut Saved,
        active: &Running,
        request: &Value,
    ) -> Result<Option<Value>, String> {
        if request["method"] == "notifications/cancelled" {
            if self
                .operation
                .as_ref()
                .is_some_and(|(id, _)| *id == request["params"]["requestId"])
            {
                self.retire(active)?;
            }
            if self
                .pending
                .as_ref()
                .is_some_and(|p| p["id"] == request["params"]["requestId"])
            {
                self.clear(root, saved, active)?;
            }
            return Ok(None);
        }
        if request.get("id").is_none() {
            return Ok(None);
        }
        let name = request["params"]["name"].as_str().unwrap_or("");
        let args = &request["params"]["arguments"];
        if request["method"] == "tools/call"
            && crate::mcp::reads(name) == Some(false)
            && crate::mcp::validate(name, args).is_ok()
            && !active.stopped.load(Ordering::SeqCst)
        {
            if self.pending.is_some() || self.operation.is_some() {
                return Ok(Some(tool_error(&request["id"], msg::agent_busy())));
            }
            let ask = crate::link::fresh_hex()?;
            *active.approval.lock().map_err(|e| e.to_string())? = Some((ask.clone(), None));
            saved.record.status = "blocked".into();
            saved.record.agent.as_mut().expect("agent turn").approval = Some(AgentApproval {
                id: ask,
                tool: name.into(),
                arguments: args.to_string(),
            });
            save(root, saved)?;
            self.pending = Some(request.clone());
            self.progress = Some(std::time::Instant::now());
            self.progress_count = 0;
            return Ok(None);
        }
        if request["method"] == "tools/call" {
            self.start(root, active, request, false)
        } else {
            mcp_request(request)
        }
    }

    fn poll_worker(&mut self, active: &Running) -> Result<Option<Value>, String> {
        if self.operation.is_some() {
            if active.stopped.load(Ordering::SeqCst) {
                self.retire(active)?;
                return Ok(None);
            }
            let answer = self
                .operation
                .as_mut()
                .and_then(|(_, worker)| worker.poll());
            if let Some(answer) = answer {
                let (id, worker) = self.operation.take().expect("polled operation");
                active.release_worker(&worker.control);
                if active.stopped.load(Ordering::SeqCst) {
                    return Ok(None);
                }
                return Ok(Some(tool_response(&id, answer)));
            }
        }
        Ok(None)
    }

    fn poll(
        &mut self,
        root: &Path,
        saved: &mut Saved,
        active: &Running,
    ) -> Result<Option<Value>, String> {
        if let Some(response) = self.poll_worker(active)? {
            return Ok(Some(response));
        }
        let Some(request) = self.pending.as_ref() else {
            return Ok(None);
        };
        let decision = active
            .approval
            .lock()
            .map_err(|e| e.to_string())?
            .as_ref()
            .and_then(|(_, answer)| *answer);
        if decision.is_none() && !active.stopped.load(Ordering::SeqCst) {
            // Pi's MCP client renews its 30s deadline on progress. A person's
            // decision has no deadline; cancellation is still handled above.
            if self
                .progress
                .is_some_and(|at| at.elapsed() >= std::time::Duration::from_secs(5))
            {
                self.progress = Some(std::time::Instant::now());
                if let Some(token) = request["params"]["_meta"].get("progressToken") {
                    self.progress_count += 1;
                    return Ok(Some(
                        json!({"jsonrpc":"2.0","method":"notifications/progress","params":{
                            "progressToken":token,"progress":self.progress_count,"message":khor_catalog::state::blocked()
                        }}),
                    ));
                }
            }
            return Ok(None);
        }
        let request = request.clone();
        self.clear(root, saved, active)?;
        if decision != Some(true) || active.stopped.load(Ordering::SeqCst) {
            return Ok(Some(tool_error(&request["id"], msg::agent_declined())));
        }
        self.start(root, active, &request, true)
    }
}

fn tool_error(id: &Value, message: &str) -> Value {
    json!({"jsonrpc":"2.0","id":id,"result":{"content":[{"type":"text","text":message}],"isError":true}})
}

fn tool_response(id: &Value, answer: Result<String, String>) -> Value {
    let (text, failed) = match answer {
        Ok(text) => (text, false),
        Err(error) => (error, true),
    };
    let mut content = json!({"type":"text"});
    content["text"] = Value::String(text);
    let mut result = json!({"isError":failed});
    result["content"] = Value::Array(vec![content]);
    let mut frame = json!({"jsonrpc":"2.0","id":id});
    frame["result"] = result;
    frame
}

fn send_mcp(active: &Running, response: Value) -> Result<(), String> {
    if active.stopped.load(Ordering::SeqCst) {
        return Ok(());
    }
    let mut frame = json!({"type":"mcp"});
    frame["message"] = response;
    let bytes = match crate::mcp_worker::encode_message(&frame) {
        Ok(bytes) => bytes,
        Err(error) => {
            // Include the request id, bridge envelope and newline in the same
            // total budget. Replace the entire content before emitting bytes.
            frame["message"]["result"] =
                json!({"content":[{"type":"text","text":error}],"isError":true});
            crate::mcp_worker::encode_message(&frame)?
        }
    };
    if active.stopped.load(Ordering::SeqCst) {
        return Ok(());
    }
    let input = active
        .input
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or_else(|| msg::agent_not_running().to_owned())?;
    input.send(bytes)
}

fn mcp_request(request: &Value) -> Result<Option<Value>, String> {
    let Some(id) = request.get("id") else {
        return Ok(None);
    };
    let method = request["method"].as_str().unwrap_or("");
    let result = match method {
        "initialize" => {
            json!({"protocolVersion":"2024-11-05","capabilities":{"tools":{}},"serverInfo":{"name":"khor","version":env!("CARGO_PKG_VERSION")}})
        }
        "ping" => json!({}),
        "tools/list" => crate::mcp::tools(),
        _ => {
            return Ok(Some(
                json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":msg::unknown_action(method)}}),
            ));
        }
    };
    Ok(Some(json!({"jsonrpc":"2.0","id":id,"result":result})))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_queued_cancel_wins_over_ui_approval_after_an_intervening_update() {
        let scratch = crate::testdir::TempTree::fresh("mcp-queued-cancel");
        let node = Node::open_as(scratch.join("store"), "queued-cancel-fixture").unwrap();
        let _claim = claim(node.root()).unwrap();
        let active = Arc::new(Running::new("queued-cancel-fixture-run".into()));
        running()
            .lock()
            .unwrap()
            .insert(node.root().into(), active.clone());
        struct RegisteredRun(PathBuf);
        impl Drop for RegisteredRun {
            fn drop(&mut self) {
                running()
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .remove(&self.0);
            }
        }
        let _registered = RegisteredRun(node.root().into());
        let mut saved = Saved {
            fingerprint: "fixture".into(),
            record: CommandRecord {
                id: active.id.clone(),
                status: "running".into(),
                started_at_ms: now(),
                agent: Some(AgentTurn {
                    model: "fixture".into(),
                    approval: None,
                    updated_at_ms: 0,
                    updates: vec![],
                    updates_truncated: false,
                }),
                ..Default::default()
            },
        };
        let mut session = McpSession::default();
        session.message(node.root(), &mut saved, &active,
            &json!({"jsonrpc":"2.0","id":17,"method":"tools/call","params":{"name":"tell","arguments":{
                "machine":node.name(),"text":"queued-cancel-must-not-be-posted"
            }}})).unwrap();
        let ask = active.approval.lock().unwrap().as_ref().unwrap().0.clone();
        let (queued, events) = mpsc::sync_channel(2);
        queued.send(json!({"type":"update","update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"queued update"}}})).unwrap();
        queued.send(json!({"type":"mcp","message":{"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":17}}})).unwrap();
        node.khor_agent_here(Action::Answer {
            run: active.id.clone(),
            ask,
            allow: true,
        })
        .await
        .unwrap();
        assert_eq!(
            active.approval.lock().unwrap().as_ref().unwrap().1,
            Some(true)
        );
        let update = events.recv().unwrap();
        let _: agent_client_protocol::schema::v1::SessionUpdate =
            serde_json::from_value(update["update"].clone()).unwrap();
        // This is the actual execute after-event entry, with the second event
        // already queued. It must not turn that UI answer into an operation.
        assert!(session.poll_worker(&active).unwrap().is_none());
        assert!(session.operation.is_none());
        assert!(active.worker.lock().unwrap().is_none());
        let cancel = events.recv().unwrap();
        session
            .message(node.root(), &mut saved, &active, &cancel["message"])
            .unwrap();
        assert!(session.poll_worker(&active).unwrap().is_none());
        assert!(
            session
                .poll(node.root(), &mut saved, &active)
                .unwrap()
                .is_none()
        );
        assert!(session.operation.is_none());
        assert!(active.worker.lock().unwrap().is_none());
        assert!(active.approval.lock().unwrap().is_none());
        assert!(saved.record.agent.as_ref().unwrap().approval.is_none());
        assert!(node.wide_log(crate::wide::BROADCAST).unwrap().0.messages.iter()
            .all(|m|!matches!(&m.body,crate::MsgBody::Text(text) if text=="queued-cancel-must-not-be-posted")));
    }

    #[test]
    fn ordinary_notifications_cannot_register_a_worker_or_request_write_approval() {
        let scratch = crate::testdir::TempTree::fresh("mcp-notification-no-id");
        let node = Node::open_as(scratch.join("store"), "notification-fixture").unwrap();
        let active = Running::new("notification-fixture-run".into());
        let mut saved = Saved {
            fingerprint: "fixture".into(),
            record: CommandRecord {
                id: active.id.clone(),
                status: "running".into(),
                agent: Some(AgentTurn {
                    model: "fixture".into(),
                    approval: None,
                    updated_at_ms: 0,
                    updates: vec![],
                    updates_truncated: false,
                }),
                ..Default::default()
            },
        };
        let receipt_before = serde_json::to_vec(&saved).unwrap();
        let messages_before = node
            .wide_log(crate::wide::BROADCAST)
            .unwrap()
            .0
            .messages
            .len();
        let mut session = McpSession::default();
        for request in [
            json!({"jsonrpc":"2.0","method":"tools/call","params":{"name":"devices","arguments":{}}}),
            json!({"jsonrpc":"2.0","method":"tools/call","params":{"name":"tell","arguments":{
                "machine":node.name(),"text":"notification-must-not-be-posted"
            }}}),
            json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        ] {
            assert!(
                session
                    .message(node.root(), &mut saved, &active, &request)
                    .unwrap()
                    .is_none()
            );
            assert!(
                session.operation.is_none(),
                "a notification must not own a process"
            );
            assert!(
                active.worker.lock().unwrap().is_none(),
                "a notification must not register a Control"
            );
            assert!(session.pending.is_none());
            assert!(active.approval.lock().unwrap().is_none());
            assert!(saved.record.agent.as_ref().unwrap().approval.is_none());
            assert_eq!(serde_json::to_vec(&saved).unwrap(), receipt_before);
        }
        let messages = node.wide_log(crate::wide::BROADCAST).unwrap().0.messages;
        assert_eq!(messages.len(), messages_before);
        assert!(messages.iter().all(|m| !matches!(&m.body, crate::MsgBody::Text(text) if text=="notification-must-not-be-posted")));
        // An explicitly present null id is different from an absent id.
        let pong = session
            .message(
                node.root(),
                &mut saved,
                &active,
                &json!({"jsonrpc":"2.0","id":null,"method":"ping"}),
            )
            .unwrap()
            .unwrap();
        assert!(pong.get("id").unwrap().is_null());
        assert_eq!(pong["result"], json!({}));
    }

    #[tokio::test]
    #[ignore = "requires the exact newly built CLI via KHOR_TEST_MCP_EXECUTABLE"]
    async fn cancelling_one_worker_cannot_cancel_or_answer_a_later_request() {
        let scratch = crate::testdir::TempTree::fresh("mcp-worker-request-generation");
        let node = Node::open_as(scratch.join("store"), "worker-generation").unwrap();
        let account = node
            .register_account(
                "fixture",
                crate::Provider::Newapi,
                crate::AccountKind::Bill,
                Some("http://127.0.0.1:1"),
                "synthetic-unused-fixture-key",
                None,
            )
            .unwrap();
        node.khor_agent_here(Action::Configure {
            account: account.id,
            model: "fixture".into(),
        })
        .await
        .unwrap();
        let active = Running::new("worker-generation-fixture".into());
        let mut saved = Saved {
            fingerprint: "fixture".into(),
            record: CommandRecord {
                id: active.id.clone(),
                agent: Some(AgentTurn {
                    model: "fixture".into(),
                    approval: None,
                    updated_at_ms: 0,
                    updates: vec![],
                    updates_truncated: false,
                }),
                ..Default::default()
            },
        };
        let request = |id| json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":"devices","arguments":{}}});
        let cancel = |id| json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":id}});
        let mut session = McpSession::default();
        assert!(
            session
                .message(node.root(), &mut saved, &active, &request(7))
                .unwrap()
                .is_none()
        );
        session
            .message(node.root(), &mut saved, &active, &cancel(7))
            .unwrap();
        assert!(session.operation.is_none());
        assert!(active.worker.lock().unwrap().is_none());
        assert!(
            session
                .message(node.root(), &mut saved, &active, &request(8))
                .unwrap()
                .is_none()
        );
        session
            .message(node.root(), &mut saved, &active, &cancel(7))
            .unwrap();
        let by = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let answer = loop {
            if let Some(answer) = session.poll(node.root(), &mut saved, &active).unwrap() {
                break answer;
            }
            assert!(
                std::time::Instant::now() < by,
                "new worker did not complete"
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        };
        assert_eq!(answer["id"], 8);
        assert_eq!(answer["result"]["isError"], false);
        assert!(session.operation.is_none());
        active.request_stop();
        assert_eq!(
            session
                .message(node.root(), &mut saved, &active, &request(9))
                .unwrap()
                .unwrap()["result"]["isError"],
            true
        );
        assert!(active.worker.lock().unwrap().is_none());
    }

    #[tokio::test]
    async fn cancelled_or_stopped_mcp_requests_cannot_consume_a_later_approval() {
        let root = std::env::temp_dir().join(format!(
            "khor-mcp-approval-{}",
            crate::link::fresh_hex().unwrap()
        ));
        let node = Node::open_as(root.clone(), "approval-fixture").unwrap();
        let _claim = claim(&root).unwrap();
        let active = Running {
            id: "agent-approval-fixture".into(),
            input: Mutex::new(None),
            process: Arc::default(),
            worker: Mutex::new(None),
            path: OnceLock::new(),
            stopped: AtomicBool::new(false),
            approval: Mutex::new(None),
        };
        let mut saved = Saved {
            fingerprint: "fixture".into(),
            record: CommandRecord {
                id: active.id.clone(),
                started_at_ms: now(),
                agent: Some(AgentTurn {
                    model: "fixture".into(),
                    approval: None,
                    updated_at_ms: 0,
                    updates: Vec::new(),
                    updates_truncated: false,
                }),
                ..Default::default()
            },
        };
        let request = json!({"jsonrpc":"2.0","id":7,"method":"tools/call","params":{
            "name":"tell","arguments":{"machine":node.name(),"text":"must-not-be-posted"},"_meta":{"progressToken":7}
        }});
        let mut session = McpSession::default();
        assert!(
            session
                .message(&root, &mut saved, &active, &request)
                .unwrap()
                .is_none()
        );
        assert_eq!(saved.record.status, "blocked");
        let ask = active.approval.lock().unwrap().as_ref().unwrap().0.clone();
        session.progress = Some(std::time::Instant::now() - std::time::Duration::from_secs(6));
        let heartbeat = session.poll(&root, &mut saved, &active).unwrap().unwrap();
        assert_eq!(heartbeat["method"], "notifications/progress");
        assert_eq!(heartbeat["params"]["progressToken"], 7);
        // Cancellation wins over an answer already queued by another device.
        active.approval.lock().unwrap().as_mut().unwrap().1 = Some(true);
        session
            .message(
                &root,
                &mut saved,
                &active,
                &json!({
                    "jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":7}
                }),
            )
            .unwrap();
        assert!(session.poll(&root, &mut saved, &active).unwrap().is_none());
        assert!(active.approval.lock().unwrap().is_none());
        assert!(saved.record.agent.as_ref().unwrap().approval.is_none());
        // The network route rejects that same approval after revocation.
        running()
            .lock()
            .unwrap()
            .insert(root.clone(), Arc::new(active));
        assert!(
            node.khor_agent_here(Action::Answer {
                run: saved.record.id.clone(),
                ask,
                allow: true
            })
            .await
            .is_err()
        );
        let active = running().lock().unwrap().remove(&root).unwrap();
        session
            .message(&root, &mut saved, &active, &request)
            .unwrap();
        active.approval.lock().unwrap().as_mut().unwrap().1 = Some(true);
        active.request_stop();
        assert_eq!(
            session.poll(&root, &mut saved, &active).unwrap().unwrap()["result"]["isError"],
            true
        );
        assert!(active.approval.lock().unwrap().is_none());
        assert!(node.wide_log(crate::wide::BROADCAST).unwrap().0.messages.iter()
            .all(|m| !matches!(&m.body, crate::MsgBody::Text(text) if text.contains("must-not-be-posted"))));
        drop(_claim);
        fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(all(test, unix))]
#[path = "khor_agent_output_tests.rs"]
mod output_tests;

#[cfg(all(test, unix))]
#[path = "khor_agent_cancel_tests.rs"]
mod cancel_tests;

#[cfg(all(test, unix))]
#[path = "khor_agent_mcp_cancel_tests.rs"]
mod mcp_cancel_tests;

#[cfg(all(test, unix))]
#[path = "khor_agent_lifecycle_tests.rs"]
mod lifecycle_tests;

#[cfg(all(test, unix))]
#[path = "khor_agent_remote_cancel_tests.rs"]
mod remote_cancel_tests;

#[cfg(all(test, unix))]
#[path = "remote_cancel_tests.rs"]
mod cancellation_receipt_tests;

#[cfg(all(test, unix))]
#[path = "khor_agent_runtime_close_tests.rs"]
pub(crate) mod runtime_close_tests;
