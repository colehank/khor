//! Execution-node ACP setup jobs. Agent launch commands resolve through the registry.
use crate::Node;
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
struct Job {
    state: Mutex<Value>,
    answers: tokio::sync::mpsc::UnboundedSender<(u64, Value)>,
    cancel: AtomicBool,
    since: Instant,
}
type Jobs = HashMap<(PathBuf, String), Arc<Job>>;
fn job_store() -> &'static OnceLock<Mutex<Jobs>> {
    static JOBS: OnceLock<Mutex<Jobs>> = OnceLock::new();
    &JOBS
}
fn jobs() -> &'static Mutex<Jobs> {
    job_store().get_or_init(Default::default)
}
pub(crate) fn stop_jobs(root: &Path) {
    let Some(jobs) = job_store().get() else {
        return;
    };
    let Ok(jobs) = jobs.try_lock() else { return };
    for ((path, _), job) in jobs.iter() {
        if path == root {
            job.cancel.store(true, Ordering::SeqCst);
        }
    }
}
pub(super) fn manage(node: &Node, request: &Value) -> Result<Value, String> {
    let _call = crate::khor_agent::runtime::enter(node.root(), &node.agent_generation)?;
    let operation = request["operation"]
        .as_str()
        .ok_or("Missing ACP operation")?;
    let root = node.root().to_path_buf();
    if matches!(operation, "acp_poll" | "acp_reply" | "acp_cancel") {
        let id = request["job"].as_str().ok_or("Missing setup job")?;
        let job = jobs()
            .lock()
            .map_err(|e| e.to_string())?
            .get(&(root.clone(), id.into()))
            .cloned()
            .ok_or("ACP setup is no longer pending")?;
        if operation == "acp_cancel" {
            let mut state = job.state.lock().map_err(|e| e.to_string())?;
            if !matches!(
                state["status"].as_str(),
                Some("done" | "failed" | "cancelled")
            ) {
                job.cancel.store(true, Ordering::SeqCst);
                *state = json!({"status":"cancelled"});
            }
        }
        if operation == "acp_reply" {
            let ask = request["ask"].as_u64().ok_or("Missing interaction ID")?;
            let mut state = job.state.lock().map_err(|e| e.to_string())?;
            if state["status"] != "waiting" || state["ask"] != ask {
                return Err("Interaction is no longer pending".into());
            }
            if request["response"].to_string().len() > 128 * 1024 {
                return Err("Interaction response is too large".into());
            }
            job.answers
                .send((ask, request["response"].clone()))
                .map_err(|e| e.to_string())?;
            state["status"] = json!("running");
        }
        let state = job.state.lock().map_err(|e| e.to_string())?.clone();
        return Ok(state);
    }
    let name = request["agent"]
        .as_str()
        .ok_or("Missing registered ACP agent")?;
    let spec = node.agent_or_refuse(name)?;
    let command = Node::agent_launch(&spec)?;
    if operation == "acp_open" {
        let environment: khor_acp::Environment =
            serde_json::from_value(request["environment"].clone())
                .map_err(|e| format!("Invalid session context: {e}"))?;
        if request["environment"].to_string().len() > 256 * 1024
            || environment.mcp_servers.len() > 32
            || environment.additional_directories.len() > 32
        {
            return Err("Session context is too large".into());
        }
        for path in &environment.additional_directories {
            if !path.is_absolute() || !path.is_dir() {
                return Err(format!(
                    "Workspace root is not an absolute directory on this machine: {}",
                    path.display()
                ));
            }
        }
        let cwd = PathBuf::from(request["cwd"].as_str().ok_or("Missing session directory")?);
        if !cwd.is_absolute() || !cwd.is_dir() {
            return Err("Choose an absolute directory on the execution node".into());
        }
        let title = request["title"]
            .as_str()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or(name);
        let token = request["token"]
            .as_str()
            .filter(|s| !s.is_empty() && s.len() <= 128)
            .ok_or("Missing session request token")?;
        type Opens = HashMap<(PathBuf, String), (Instant, String, Result<String, String>)>;
        static OPENS: OnceLock<Mutex<Opens>> = OnceLock::new();
        let mut opens = OPENS
            .get_or_init(Default::default)
            .lock()
            .map_err(|e| e.to_string())?;
        opens.retain(|_, (at, _, _)| at.elapsed() < Duration::from_secs(600));
        let key = (root.clone(), token.to_owned());
        let fingerprint = blake3::hash(request.to_string().as_bytes())
            .to_hex()
            .to_string();
        if let Some((_, prior, result)) = opens.get(&key) {
            if prior != &fingerprint {
                return Err("Session request token was already used for different options".into());
            }
            return result
                .clone()
                .map(|id| json!({"status":"done","session":id}));
        }
        if opens.len() >= 64 {
            return Err("Too many session opening requests".into());
        }
        let result = crate::gui_host::spawn_gui_host_context(
            &root,
            &cwd,
            title,
            &[command],
            name,
            &environment,
        )
        .map(|id| id.0);
        opens.insert(key, (Instant::now(), fingerprint, result.clone()));
        return result.map(|id| json!({"status":"done","session":id}));
    }
    if !matches!(operation, "acp_initialize" | "acp_authenticate") {
        return Err("Unknown ACP setup operation".into());
    }
    let method = if operation == "acp_authenticate" {
        Some(
            request["methodId"]
                .as_str()
                .filter(|s| !s.is_empty() && s.len() <= 256)
                .ok_or("Missing authentication method")?
                .to_owned(),
        )
    } else {
        None
    };
    let id = format!("acp-{}", crate::link::fresh_hex()?);
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let job = Arc::new(Job {
        state: Mutex::new(json!({"status":"running"})),
        answers: tx,
        cancel: AtomicBool::new(false),
        since: Instant::now(),
    });
    {
        let mut map = jobs().lock().map_err(|e| e.to_string())?;
        map.retain(|_, j| {
            if j.since.elapsed() > Duration::from_secs(600) {
                j.cancel.store(true, Ordering::SeqCst);
                false
            } else {
                true
            }
        });
        if map.len() >= 128
            || map
                .iter()
                .filter(|((r, _), j)| {
                    *r == root
                        && matches!(
                            j.state.lock().unwrap()["status"].as_str(),
                            Some("waiting" | "running")
                        )
                })
                .count()
                >= 16
        {
            return Err("Too many ACP setup operations".into());
        }
        map.insert((root.clone(), id.clone()), job.clone());
    }
    let owner_root = root.clone();
    let owner = std::thread::Builder::new().name("acp-setup".into()).spawn(move || {
        let result = tokio::runtime::Builder::new_current_thread().enable_all().build()
            .map_err(|e|e.to_string()).and_then(|rt| rt.block_on(async {
                if job.cancel.load(Ordering::SeqCst) { return Err("Cancelled".into()); }
                let transport = crate::acp_process::spawn(&command)?;
                let lifecycle = transport.lifecycle;
                let mut setup = khor_acp::manage_on(transport.streams, method);
                let mut next = 0u64;
                let mut pending: Option<(u64, khor_acp::Elicitation)> = None;
                let mut tick = tokio::time::interval(Duration::from_millis(100));
                let mut events_open = true;
                let result = loop {
                    tokio::select! {
                        event = setup.events.recv(), if events_open => match event {
                            Some(khor_acp::Event::Elicit(question)) => {
                                if pending.is_some() { drop(question); continue; }
                                next += 1;
                                *job.state.lock().unwrap() = json!({"status":"waiting","ask":next,"request":question.request});
                                pending = Some((next,question));
                            }
                            Some(khor_acp::Event::ElicitDone(id)) => {
                                if pending.as_ref().is_some_and(|(_,q)|serde_json::to_value(&q.request).is_ok_and(|v|v["mode"]=="url"&&v["elicitationId"]==id)) {
                                    let (_,q) = pending.take().unwrap();
                                    if let Ok(response) = q.validate(&json!({"action":"accept"})) { let _ = q.answer(response); }
                                    *job.state.lock().unwrap() = json!({"status":"running"});
                                }
                            }
                            None => events_open = false,
                            _ => {}
                        },
                        answer = rx.recv() => if let Some((ask,value)) = answer {
                            if let Some((id,question)) = &pending {
                                if *id == ask {
                                    match question.validate(&value) {
                                        Ok(response) => {
                                            let (_,question) = pending.take().unwrap();
                                            let _ = question.answer(response);
                                            *job.state.lock().unwrap() = json!({"status":"running"});
                                        }
                                        Err(error) => {
                                            let mut state = job.state.lock().unwrap();
                                            state["status"] = json!("waiting"); state["error"] = json!(error);
                                        }
                                    }
                                }
                            }
                        },
                        result = &mut setup.result => break result.map_err(|e|e.to_string()).and_then(|result|result),
                        _ = tick.tick() => if job.cancel.load(Ordering::SeqCst) || job.since.elapsed()>Duration::from_secs(600) {
                            break Err("Cancelled".into());
                        }
                    }
                };
                drop(pending);
                drop(setup);
                lifecycle.request_shutdown();
                match tokio::time::timeout(Duration::from_secs(5), lifecycle.confirmed_exit()).await {
                    Ok(Ok(_)) => result,
                    _ => {
                        crate::khor_agent::runtime::unconfirmed(&root);
                        Err("ACP setup process cleanup is unconfirmed".into())
                    }
                }
            }));
        *job.state.lock().unwrap() = if job.cancel.load(Ordering::SeqCst) {
            json!({"status":"cancelled"})
        } else {
            match result {
                Ok(value) => json!({"status":"done","result":value}),
                Err(error) => json!({"status":"failed","error":error}),
            }
        };
    });
    let owner = match owner {
        Ok(owner) => owner,
        Err(error) => {
            jobs()
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .remove(&(owner_root.clone(), id.clone()));
            return Err(error.to_string());
        }
    };
    crate::khor_agent::runtime::register_producer(&owner_root, owner);
    Ok(json!({"job":id,"status":"running"}))
}
