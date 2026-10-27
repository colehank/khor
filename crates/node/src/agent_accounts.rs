//! Provider-owned setup runs only on the chosen credential home.
use super::{directory, input, runtime, sidecar};
use crate::agent_process::{Control, Session};
use crate::{Account, AccountKind, Node, Provider};
use khor_catalog::msg;
use khor_sync::accounts::Inference;
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::ExitStatus,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

pub(super) fn catalog() -> Value {
    serde_json::from_str(include_str!("../../../apps/agent/providers.json"))
        .expect("generated Pi providers")
}
pub(super) fn credential_path(root: &Path, id: &str) -> PathBuf {
    directory(root)
        .join("credentials")
        .join(id)
        .join("auth.json")
}
fn account_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 256
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
}
pub(super) fn inference_ready(node: &Node, account: &Account) -> bool {
    if !account_id(&account.id) {
        return false;
    }
    if account.inference.is_some() {
        return account.home != node.device_str()
            || credential_path(node.root(), &account.id).is_file();
    }
    account.kind == AccountKind::Bill
        && !matches!(
            account.provider,
            Provider::Claude | Provider::Openai | Provider::Codex | Provider::Other(_)
        )
        && (account.home != node.device_str()
            || fs::read_to_string(node.root().join(".khor/keys").join(&account.id))
                .is_ok_and(|key| !key.trim().is_empty()))
}
pub(super) fn provider_id(account: &Account) -> String {
    if let Some(i) = &account.inference {
        return i.provider.clone();
    }
    match account.provider.word() {
        "claude" => "anthropic",
        "codex" => "openai-codex",
        "moonshot" => "moonshotai",
        "moonshot-cn" => "moonshotai-cn",
        p => p,
    }
    .into()
}
fn account_info(node: &Node, account: &Account) -> Value {
    json!({"id":account.id,"name":account.name,"home":account.home,"home_name":node.shown_name(),
        "provider":provider_id(account),"default_model":account.inference.as_ref().map(|i|i.default_model.as_str()).unwrap_or(""),
        "base_url":account.inference.as_ref().and_then(|i|i.base_url.clone()).or_else(||account.base_url.clone()),
        "auth_type":account.inference.as_ref().map(|i|i.auth_type.clone()),"configured":inference_ready(node,account)})
}
struct Job {
    control: Arc<Control>,
    input: Mutex<Option<input::Input>>,
    finished: AtomicBool,
    state: Mutex<Value>,
    since: Instant,
}
impl Job {
    fn stop(&self) {
        self.control.terminate_now();
        if let Ok(input) = self.input.try_lock() {
            if let Some(input) = input.as_ref() {
                input.close();
            }
        }
        if let Ok(mut state) = self.state.try_lock() {
            if !matches!(
                state["status"].as_str(),
                Some("done" | "failed" | "cancelled")
            ) {
                *state = json!({"status":"cancelled"});
            }
        }
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        self.control.terminate_now();
    }
}
struct Registration {
    root: PathBuf,
    id: String,
    job: Arc<Job>,
    committed: bool,
}
impl Drop for Registration {
    fn drop(&mut self) {
        if !self.committed {
            self.job.stop();
            let mut jobs = jobs().lock().unwrap_or_else(|p| p.into_inner());
            let key = (self.root.clone(), self.id.clone());
            if jobs
                .get(&key)
                .is_some_and(|job| Arc::ptr_eq(job, &self.job))
            {
                jobs.remove(&key);
            }
        }
    }
}
struct Setup {
    root: PathBuf,
    child: Option<Session>,
    input: input::Input,
    writer: Option<JoinHandle<()>>,
    drain: Option<JoinHandle<()>>,
}
impl Setup {
    fn finish(&mut self, force: bool) -> Result<ExitStatus, String> {
        self.input.close();
        let status = self
            .child
            .take()
            .expect("setup process owner")
            .finish(force)
            .map_err(|e| e.to_string());
        let mut error = None;
        for thread in [&mut self.writer, &mut self.drain] {
            if let Some(thread) = thread.take() {
                if thread.join().is_err() {
                    error = Some("Setup I/O owner panicked".to_owned());
                }
            }
        }
        match (status, error) {
            (Ok(status), None) => Ok(status),
            (Err(error), _) | (_, Some(error)) => {
                runtime::unconfirmed(&self.root);
                Err(error)
            }
        }
    }
}
impl Drop for Setup {
    fn drop(&mut self) {
        if self.child.is_some() {
            let _ = self.finish(true);
        }
    }
}
pub(super) fn stop_jobs(root: &Path) {
    let Some(jobs) = job_store().get() else {
        return;
    };
    let Ok(jobs) = jobs.try_lock() else { return };
    let owned: Vec<_> = jobs
        .iter()
        .filter(|((path, _), _)| path == root)
        .map(|(_, job)| job.clone())
        .collect();
    drop(jobs);
    for job in owned {
        job.stop();
    }
}
type Jobs = HashMap<(PathBuf, String), Arc<Job>>;
fn job_store() -> &'static OnceLock<Mutex<Jobs>> {
    static JOBS: OnceLock<Mutex<Jobs>> = OnceLock::new();
    &JOBS
}
fn jobs() -> &'static Mutex<Jobs> {
    job_store().get_or_init(|| {
        std::thread::spawn(|| {
            loop {
                std::thread::sleep(Duration::from_secs(15));
                if let Some(map) = job_store().get() {
                    if let Ok(mut map) = map.lock() {
                        map.retain(|_, job| {
                            if job.since.elapsed() < Duration::from_secs(600) {
                                return true;
                            }
                            job.stop();
                            false
                        });
                    }
                }
            }
        });
        Mutex::new(HashMap::new())
    })
}
fn text<'a>(v: &'a Value, key: &str) -> Result<&'a str, String> {
    v[key].as_str().ok_or_else(|| msg::agent_arguments().into())
}
fn own(node: &Node, id: &str) -> Result<Account, String> {
    if !account_id(id) {
        return Err(msg::agent_arguments().into());
    }
    node.accounts()?
        .into_iter()
        .find(|a| a.id == id && a.home == node.device_str())
        .ok_or_else(|| msg::agent_key_missing().into())
}
fn clean_model(value: &str) -> bool {
    value.len() <= 256 && !value.chars().any(char::is_control)
}
fn address(value: &str) -> bool {
    value.is_empty()
        || ((value.starts_with("https://") || value.starts_with("http://"))
            && value.len() < 4096
            && !value.chars().any(char::is_whitespace)
            && !value.contains('@')
            && !value.contains('?')
            && !value.contains('#'))
}
impl Node {
    pub(super) fn manage_agent(&self, request: Value) -> Result<Value, String> {
        let _call = runtime::enter(self.root(), &self.agent_generation)?;
        self.refuse_unless_hosting()?;
        let operation = text(&request, "operation")?;
        if operation.starts_with("acp_") {
            return crate::acp_management::manage(self, &request);
        }
        let root = self.root().to_path_buf();
        if matches!(operation, "poll" | "reply" | "cancel") {
            let id = text(&request, "job")?;
            let job = jobs()
                .lock()
                .map_err(|e| e.to_string())?
                .get(&(root, id.into()))
                .cloned()
                .ok_or_else(|| msg::agent_not_running().to_owned())?;
            if operation == "cancel" {
                let mut state = job.state.lock().map_err(|e| e.to_string())?;
                if !matches!(
                    state["status"].as_str(),
                    Some("done" | "failed" | "cancelled")
                ) {
                    *state = json!({"status":"cancelled"});
                    drop(state);
                    job.stop();
                }
            } else if operation == "reply" {
                let mut state = job.state.lock().map_err(|e| e.to_string())?;
                if state["status"] != "waiting" || state["ask"] != request["ask"] {
                    return Err(msg::agent_arguments().into());
                }
                let value = text(&request, "value")?;
                if value.len() > 64 * 1024 {
                    return Err(msg::agent_arguments().into());
                }
                let mut input = job.input.lock().map_err(|e| e.to_string())?;
                let input = input
                    .as_mut()
                    .ok_or_else(|| msg::agent_not_running().to_owned())?;
                let mut bytes = serde_json::to_vec(
                    &json!({"type":"auth_reply","ask":request["ask"],"value":value}),
                )
                .map_err(|e| e.to_string())?;
                bytes.push(b'\n');
                input.send(bytes)?;
                state["status"] = json!("running");
                state.as_object_mut().unwrap().remove("prompt");
            }
            let result = job.state.lock().map_err(|e| e.to_string())?.clone();
            if job.finished.load(Ordering::SeqCst)
                && matches!(
                    result["status"].as_str(),
                    Some("done" | "failed" | "cancelled")
                )
            {
                jobs()
                    .lock()
                    .map_err(|e| e.to_string())?
                    .remove(&(self.root().to_path_buf(), id.into()));
            }
            return Ok(result);
        }
        if operation == "default" {
            let mut account = own(self, text(&request, "account")?)?;
            let model = text(&request, "model")?.trim();
            if model.is_empty() || !clean_model(model) {
                return Err(msg::agent_model_required().into());
            }
            if account.inference.is_none() {
                if !inference_ready(self, &account) {
                    return Err(msg::agent_key_missing().into());
                }
                let key = fs::read_to_string(root.join(".khor/keys").join(&account.id))
                    .map_err(|_| msg::agent_key_missing().to_owned())?;
                if key.trim().is_empty() {
                    return Err(msg::agent_key_missing().into());
                }
                let path = credential_path(&root, &account.id);
                fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    fs::set_permissions(path.parent().unwrap(), fs::Permissions::from_mode(0o700))
                        .map_err(|e| e.to_string())?;
                }
                let provider = provider_id(&account);
                crate::link::write_private(
                    &path,
                    &serde_json::to_vec(
                        &json!({provider.clone():{"type":"api_key","key":key.trim()}}),
                    )
                    .map_err(|e| e.to_string())?,
                )?;
                account.inference = Some(Inference {
                    provider,
                    base_url: account.base_url.clone(),
                    default_model: String::new(),
                    auth_type: "api_key".into(),
                });
            }
            let inference = account.inference.as_mut().expect("configured inference");
            inference.default_model = model.into();
            self.commit_account(account.clone())?;
            return Ok(
                json!({"status":"done","account":account.id,"accountInfo":account_info(self,&account)}),
            );
        }
        if !matches!(
            operation,
            "models" | "login" | "session_settings" | "session_set_option"
        ) {
            return Err(msg::agent_arguments().into());
        }
        let existing = request["account"]
            .as_str()
            .filter(|id| !id.is_empty())
            .map(|id| own(self, id))
            .transpose()?;
        let session_config = if matches!(operation, "session_settings" | "session_set_option") {
            let config = self
                .khor_agent_config()?
                .filter(|c| {
                    c.home == self.device_str()
                        && existing.as_ref().is_some_and(|a| a.id == c.account)
                })
                .ok_or_else(|| msg::agent_not_configured().to_owned())?;
            if operation == "session_set_option"
                && (!request["configId"].is_string()
                    || !(request["value"].is_string() || request["value"].is_boolean())
                    || request["value"].to_string().len() > 512)
            {
                return Err(msg::agent_arguments().into());
            }
            Some(config)
        } else {
            None
        };
        let id = existing
            .as_ref()
            .map(|a| a.id.clone())
            .unwrap_or(crate::link::fresh_hex()?);
        let provider = request["provider"]
            .as_str()
            .map(str::to_owned)
            .or_else(|| existing.as_ref().map(provider_id))
            .ok_or_else(|| msg::agent_arguments().to_owned())?;
        let providers = catalog();
        let definition = providers
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["id"] == provider && p["chat"] == true)
            .ok_or_else(|| msg::agent_arguments().to_owned())?;
        if existing
            .as_ref()
            .is_some_and(|a| provider_id(a) != provider)
        {
            return Err(msg::agent_arguments().into());
        }
        let auth_type = request["authType"].as_str().unwrap_or("api_key");
        if operation == "login"
            && !definition["methods"]
                .as_array()
                .unwrap()
                .iter()
                .any(|m| m["id"] == auth_type)
        {
            return Err(msg::agent_arguments().into());
        }
        let base = request["baseUrl"]
            .as_str()
            .map(str::to_owned)
            .or_else(|| {
                existing.as_ref().and_then(|a| {
                    a.inference
                        .as_ref()
                        .and_then(|i| i.base_url.clone())
                        .or_else(|| a.base_url.clone())
                })
            })
            .unwrap_or_default();
        let name = request["name"]
            .as_str()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .or_else(|| existing.as_ref().map(|a| a.name.clone()))
            .unwrap_or_else(|| definition["name"].as_str().unwrap_or(&provider).into());
        if !address(&base) || (provider == "newapi" && base.is_empty()) || name.len() > 256 {
            return Err(msg::agent_arguments().into());
        }
        let metadata = Inference {
            provider: provider.clone(),
            base_url: (!base.is_empty()).then_some(base.clone()),
            default_model: existing
                .as_ref()
                .and_then(|a| a.inference.as_ref().map(|i| i.default_model.clone()))
                .unwrap_or_default(),
            auth_type: auth_type.into(),
        };
        let account = existing.clone().unwrap_or(Account {
            id: id.clone(),
            name: name.clone(),
            provider: Provider::parse(&provider).unwrap_or(Provider::Other(provider.clone())),
            base_url: None,
            budget: None,
            home: self.device_str().into(),
            pinned: false,
            unit: None,
            kind: AccountKind::Bill,
            inference: None,
        });
        let path = credential_path(&root, &id);
        fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path.parent().unwrap(), fs::Permissions::from_mode(0o700))
                .map_err(|e| e.to_string())?;
        }
        let mut seat = json!({"provider":provider,"baseUrl":(!base.is_empty()).then_some(base)});
        if operation != "login"
            && existing
                .as_ref()
                .is_some_and(|a| a.inference.is_none() && inference_ready(self, a))
        {
            if let Ok(key) = fs::read_to_string(root.join(".khor/keys").join(&id)) {
                seat["key"] = json!(key.trim());
            }
        }
        let mut command = sidecar(&root)?;
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
        command.env("PATH", Node::user_path());
        let control = Arc::new(Control::default());
        let job_id = crate::link::fresh_hex()?;
        let job = Arc::new(Job {
            control: control.clone(),
            finished: AtomicBool::new(false),
            input: Mutex::new(None),
            state: Mutex::new(json!({"status":"running"})),
            since: Instant::now(),
        });
        {
            let mut jobs = jobs().lock().map_err(|e| e.to_string())?;
            jobs.retain(|_, j| {
                if j.since.elapsed() < Duration::from_secs(600) {
                    true
                } else {
                    j.stop();
                    false
                }
            });
            if jobs.len() >= 16 {
                return Err(msg::agent_busy().into());
            }
            jobs.insert((root.clone(), job_id.clone()), job.clone());
        }
        let mut registration = Registration {
            root: root.clone(),
            id: job_id.clone(),
            job: job.clone(),
            committed: false,
        };
        let (mut child, pipes) =
            Session::spawn(command, control.clone()).map_err(|e| msg::agent_runtime_failed(e))?;
        let (input, writer) = match input::Input::spawn(pipes.stdin) {
            Ok(input) => input,
            Err(error) => {
                if child.finish(true).is_err() {
                    runtime::unconfirmed(&root);
                }
                return Err(error.to_string());
            }
        };
        if control.is_cancelled() {
            input.close();
        }
        *job.input.lock().unwrap() = Some(input.clone());
        let stdout = pipes.stdout;
        let mut setup = Setup {
            root: root.clone(),
            child: Some(child),
            input: input.clone(),
            writer: Some(writer),
            drain: None,
        };
        setup.drain = Some(
            std::thread::Builder::new()
                .name("setup-stderr".into())
                .spawn(move || super::io::drain_stderr(pipes.stderr))
                .map_err(|e| e.to_string())?,
        );
        let mut start = json!({"type":"manage","operation":operation,"account":seat,"credentialPath":path,"authType":auth_type,"deviceId":self.device_str()});
        if let Some(config) = &session_config {
            start["model"] = json!(config.model);
            start["options"] = json!(config.options);
            start["configId"] = request["configId"].clone();
            start["value"] = request["value"].clone();
        }
        let mut bytes = serde_json::to_vec(&start).map_err(|e| e.to_string())?;
        bytes.push(b'\n');
        let is_new = existing.is_none();
        let operation = operation.to_owned();
        let work = job.clone();
        let owner_root = root.clone();
        let owner = std::thread::Builder::new()
            .name("agent-setup".into())
            .spawn(move || {
                let result = (|| -> Result<(), String> {
                    setup.input.send(bytes)?;
                    for line in BufReader::new(stdout).lines() {
                        let line = line.map_err(|e| e.to_string())?;
                        if line.len() > 2 * 1024 * 1024 {
                            return Err(msg::agent_arguments().into());
                        }
                        let event: Value = serde_json::from_str(&line)
                            .map_err(|_| msg::agent_arguments().to_owned())?;
                        let mut state = work.state.lock().map_err(|e| e.to_string())?;
                        if state["status"] == "cancelled" {
                            return Ok(());
                        }
                        match event["type"].as_str() {
                            Some("auth_prompt") => {
                                state["status"] = json!("waiting");
                                state["ask"] = event["ask"].clone();
                                state["prompt"] = event["prompt"].clone();
                            }
                            Some("auth_event") => {
                                let mut events =
                                    state["events"].as_array().cloned().unwrap_or_default();
                                events.push(event["event"].clone());
                                if events.len() > 4 {
                                    events.remove(0);
                                }
                                state["events"] = events.into();
                            }
                            Some("result") => {
                                if operation == "session_set_option" {
                                    let _configuration = super::configuration_lock(&root)?;
                                    let node = Node::open(root.clone())?;
                                    if node.khor_agent_config()? != session_config {
                                        return Err(msg::agent_arguments().into());
                                    }
                                    let mut config = session_config
                                        .clone()
                                        .ok_or_else(|| msg::agent_not_configured().to_owned())?;
                                    config.model = event["value"]["model"]
                                        .as_str()
                                        .filter(|m| !m.is_empty() && clean_model(m))
                                        .ok_or_else(|| msg::agent_arguments().to_owned())?
                                        .to_owned();
                                    config.options =
                                        serde_json::from_value(event["value"]["options"].clone())
                                            .map_err(|_| msg::agent_arguments().to_owned())?;
                                    if config.options.len() > 32
                                        || config.options.iter().any(|(id, value)| {
                                            id.len() > 256
                                                || !(value.is_string() || value.is_boolean())
                                                || value.to_string().len() > 512
                                        })
                                    {
                                        return Err(msg::agent_arguments().into());
                                    }
                                    node.chat.set_agent_config(&config)?;
                                    let _ = node.emit_row_of(&crate::SessionId(
                                        crate::wide::BROADCAST.into(),
                                    ));
                                }
                                if operation == "login" {
                                    let node = Node::open(root.clone())?;
                                    let mut account = node
                                        .accounts()?
                                        .into_iter()
                                        .find(|a| a.id == account.id)
                                        .unwrap_or(account.clone());
                                    let stored: Value = serde_json::from_slice(
                                        &fs::read(credential_path(&root, &account.id))
                                            .map_err(|e| e.to_string())?,
                                    )
                                    .map_err(|_| msg::agent_key_missing().to_owned())?;
                                    if stored[&metadata.provider]["type"].as_str()
                                        != event["value"]["credentialType"].as_str()
                                    {
                                        return Err(msg::agent_key_missing().into());
                                    }
                                    let mut inference = metadata.clone();
                                    // Another device may have changed the default
                                    // while this provider's login was pending.
                                    if let Some(latest) = &account.inference {
                                        inference.default_model = latest.default_model.clone();
                                    }
                                    inference.auth_type = event["value"]["credentialType"]
                                        .as_str()
                                        .unwrap_or(&inference.auth_type)
                                        .into();
                                    account.name = name.clone();
                                    account.inference = Some(inference);
                                    node.commit_account(account.clone())?;
                                    state["account"] = json!(account.id);
                                    state["accountInfo"] = account_info(&node, &account);
                                }
                                state["status"] = json!("done");
                                state["result"] = event["value"].clone();
                                state.as_object_mut().unwrap().remove("prompt");
                            }
                            Some("failed") => {
                                return Err(event["error"]
                                    .as_str()
                                    .unwrap_or(msg::agent_arguments())
                                    .into());
                            }
                            _ => {}
                        }
                    }
                    Ok(())
                })();
                let reaped = setup
                    .finish(result.is_err() || work.state.lock().unwrap()["status"] == "cancelled");
                if reaped.is_err() {
                    runtime::unconfirmed(&root);
                }
                let result = result.and_then(|()| {
                    let status = reaped?;
                    if (!status.success() && !work.control.is_cancelled())
                        || work.state.lock().unwrap()["status"] != "done"
                    {
                        return Err(msg::agent_runtime_missing().into());
                    }
                    Ok(())
                });
                if let Err(error) = result {
                    let mut state = work.state.lock().unwrap();
                    if state["status"] != "cancelled" {
                        *state = json!({"status":"failed","error":error});
                    }
                }
                work.input.lock().unwrap().take();
                #[cfg(all(test, unix))]
                super::runtime_close_tests::producer_point(&root, "setup-cleanup");
                if is_new && work.state.lock().unwrap()["status"] != "done" {
                    let registered = Node::open(root.clone())
                        .and_then(|node| node.accounts())
                        .is_ok_and(|accounts| accounts.iter().any(|a| a.id == account.id));
                    if !registered {
                        let _ = fs::remove_dir_all(
                            credential_path(&root, &account.id).parent().unwrap(),
                        );
                    }
                }
                work.finished.store(true, Ordering::SeqCst);
            })
            .map_err(|e| e.to_string())?;
        runtime::register_producer(&owner_root, owner);
        registration.committed = true;
        Ok(json!({"job":job_id,"status":"running"}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn poll(node: &Node, id: &str, expected: &str) -> Value {
        let until = Instant::now() + Duration::from_secs(20);
        loop {
            let state = node
                .manage_agent(json!({"operation":"poll","job":id}))
                .unwrap();
            if state["status"] == expected {
                return state;
            }
            assert_ne!(state["status"], "failed", "{state}");
            assert!(
                Instant::now() < until,
                "setup did not reach {expected}: {state}"
            );
            std::thread::sleep(Duration::from_millis(25));
        }
    }
    #[tokio::test]
    async fn pi_login_keeps_credentials_local_reuses_accounts_and_switches_only_future_turns() {
        let root = std::env::temp_dir().join(format!(
            "khor-pi-accounts-{}",
            crate::link::fresh_hex().unwrap()
        ));
        let node = Node::open_as(root.clone(), "fixture-home").unwrap();
        let started=node.manage_agent(json!({"operation":"login","provider":"anthropic","authType":"api_key","name":"fixture inference"})).unwrap();
        let id = started["job"].as_str().unwrap();
        let waiting = poll(&node, id, "waiting");
        assert_eq!(waiting["prompt"]["type"], "secret");
        assert!(
            node.manage_agent(
                json!({"operation":"reply","job":id,"ask":999,"value":"must-not-be-used"})
            )
            .is_err()
        );
        node.manage_agent(json!({"operation":"reply","job":id,"ask":waiting["ask"],"value":"fixture-private-key"})).unwrap();
        let done = poll(&node, id, "done");
        let account = own(&node, done["account"].as_str().unwrap()).unwrap();
        assert!(credential_path(&root, &account.id).is_file());
        assert!(
            !serde_json::to_string(&account)
                .unwrap()
                .contains("fixture-private-key")
        );
        assert!(
            !root.join(".khor/keys").join(&account.id).exists(),
            "inference key must not become a usage/admin credential"
        );
        node.manage_agent(
            json!({"operation":"default","account":account.id,"model":"default-model"}),
        )
        .unwrap();
        assert_eq!(
            own(&node, &account.id)
                .unwrap()
                .inference
                .unwrap()
                .default_model,
            "default-model"
        );
        let _running = super::super::claim(&root).unwrap();
        node.khor_agent_here(super::super::Action::Configure {
            account: account.id.clone(),
            model: "conversation-model".into(),
        })
        .await
        .unwrap();
        assert_eq!(
            node.khor_agent_config().unwrap().unwrap().model,
            "conversation-model"
        );
        assert_eq!(
            own(&node, &account.id)
                .unwrap()
                .inference
                .unwrap()
                .default_model,
            "default-model",
            "conversation selection must not overwrite the account default"
        );
        let admin = node
            .register_account(
                "usage only",
                Provider::Openai,
                AccountKind::Bill,
                None,
                "fixture-admin",
                None,
            )
            .unwrap();
        assert!(
            !inference_ready(&node, &admin),
            "usage credentials must not silently become model credentials"
        );
        let reconnect = node.manage_agent(json!({"operation":"login","account":account.id,"provider":"anthropic","authType":"api_key"})).unwrap();
        let reconnect_id = reconnect["job"].as_str().unwrap();
        let reconnect_prompt = poll(&node, reconnect_id, "waiting");
        node.manage_agent(
            json!({"operation":"default","account":account.id,"model":"changed-during-login"}),
        )
        .unwrap();
        node.manage_agent(json!({"operation":"reply","job":reconnect_id,"ask":reconnect_prompt["ask"],"value":"fixture-reconnected"})).unwrap();
        assert_eq!(poll(&node, reconnect_id, "done")["account"], account.id);
        assert_eq!(
            own(&node, &account.id)
                .unwrap()
                .inference
                .unwrap()
                .default_model,
            "changed-during-login"
        );
        assert!(
            node.manage_agent(
                json!({"operation":"default","account":"../../invalid","model":"bad"})
            )
            .is_err()
        );
        // A legacy inference key can acquire a default without losing its
        // independent usage registration or being silently re-registered.
        let legacy = node
            .register_account(
                "legacy gateway",
                Provider::Newapi,
                AccountKind::Bill,
                Some("http://127.0.0.1:9"),
                "fixture-legacy",
                None,
            )
            .unwrap();
        node.manage_agent(
            json!({"operation":"default","account":legacy.id,"model":"legacy-model"}),
        )
        .unwrap();
        assert_eq!(
            own(&node, &legacy.id)
                .unwrap()
                .inference
                .unwrap()
                .default_model,
            "legacy-model"
        );
        assert_eq!(
            fs::read_to_string(root.join(".khor/keys").join(&legacy.id)).unwrap(),
            "fixture-legacy"
        );
        let stored: Value =
            serde_json::from_slice(&fs::read(credential_path(&root, &legacy.id)).unwrap()).unwrap();
        assert_eq!(stored["newapi"]["key"], "fixture-legacy");
        // Closing during a provider prompt must not register an empty account.
        let pending = node.manage_agent(json!({"operation":"login","provider":"anthropic","authType":"api_key","name":"cancelled draft"})).unwrap();
        let pending_id = pending["job"].as_str().unwrap();
        poll(&node, pending_id, "waiting");
        let cancelled = node
            .manage_agent(json!({"operation":"cancel","job":pending_id}))
            .unwrap();
        assert_eq!(cancelled["status"], "cancelled");
        assert!(
            !node
                .accounts()
                .unwrap()
                .iter()
                .any(|a| a.name == "cancelled draft")
        );
        drop(_running);
        assert_eq!(
            Node::shutdown_agent_runtime(&root).await.unwrap(),
            super::super::RuntimeClose::Closed
        );
        drop(node);
        let _ = fs::remove_dir_all(root);
    }
}
