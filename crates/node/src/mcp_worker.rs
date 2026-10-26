//! A finite, owned process for synchronous MCP work, including blocking locks.
//! Killing it confirms local retirement; durable remote child links remain
//! cancellation intents until their own executor supplies a terminal receipt.
use crate::agent_process::{Control, Session, SpawnProfile};
use khor_catalog::msg;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::VecDeque,
    io::{self, Read, Write},
    path::PathBuf,
    process::{Command, ExitStatus},
    sync::{Arc, Mutex, OnceLock, mpsc},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

const INPUT_LIMIT: usize = 2 * 1024 * 1024;
const TEXT_LIMIT: usize = 64 * 1024 * 1024;
const OUTPUT_LIMIT: usize = 128 * 1024 * 1024 + 64 * 1024;
const STDERR_LIMIT: usize = 256 * 1024;
pub(crate) const TOOL_BUDGET: Duration = Duration::from_secs(300);
const SWEEP_BUDGET: Duration = Duration::from_secs(5);
#[cfg(test)]
pub(crate) static SWEEPS_STARTED: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Request {
    pub root: PathBuf,
    pub operation: Operation,
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum Operation {
    Tool {
        run: String,
        authorized: bool,
        name: String,
        args: Value,
    },
    CancelChildren {
        run: String,
    },
}

#[cfg(windows)]
static WORKER_PROCESS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Only the two durable Khor host constructors call this. Temporary children
/// inherit containment; default Pi and ACP jobs never enable breakaway.
pub(crate) fn prepare_durable_child(command: &mut Command) {
    #[cfg(windows)]
    if WORKER_PROCESS.load(std::sync::atomic::Ordering::Relaxed) {
        use std::os::windows::process::CommandExt;
        use windows_sys::Win32::System::Threading::CREATE_BREAKAWAY_FROM_JOB;
        command.creation_flags(CREATE_BREAKAWAY_FROM_JOB);
    }
    #[cfg(not(windows))]
    let _ = command;
}

/// Shared CLI/GUI hook, before normal command/configuration/GUI dispatch.
pub fn main_if_requested() -> Option<i32> {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(std::ffi::OsStr::new("_mcp_worker")) {
        return None;
    }
    if args.next().is_some() {
        return Some(1);
    }
    #[cfg(windows)]
    WORKER_PROCESS.store(true, std::sync::atomic::Ordering::Relaxed);
    let result = read_request(std::io::stdin().lock()).and_then(|request| {
        let budget = match &request.operation {
            Operation::Tool { .. } => TOOL_BUDGET,
            Operation::CancelChildren { .. } => SWEEP_BUDGET,
        };
        // The parent's owner can disappear at process exit. Keep an independent
        // fuse inside the child so Unix workers cannot become 300s orphan
        // sweeps after the five-second coordinator itself has exited.
        thread::Builder::new()
            .name("mcp-worker-fuse".into())
            .spawn(move || {
                thread::sleep(budget);
                #[cfg(unix)]
                unsafe {
                    // Only a Session-created, still-live group leader owns this
                    // whole group. A direct shell invocation must not kill its
                    // caller's process group.
                    if libc::getpgrp() == libc::getpid() {
                        libc::kill(0, libc::SIGKILL);
                    }
                }
                std::process::exit(124);
            })
            .map_err(|_| "MCP worker fuse could not start".to_owned())?;
        dispatch(request)
    });
    Some(match write_result(std::io::stdout().lock(), result) {
        Ok(()) => 0,
        Err(_) => 1,
    })
}

fn read_request(reader: impl Read) -> Result<Request, String> {
    let mut bytes = Vec::new();
    reader
        .take((INPUT_LIMIT + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| "MCP worker input could not be read".to_owned())?;
    if bytes.len() > INPUT_LIMIT {
        return Err("MCP worker input exceeded its byte budget".into());
    }
    let request: Request = serde_json::from_slice(&bytes)
        .map_err(|_| "MCP worker requires one complete request".to_owned())?;
    if !request.root.is_absolute() {
        return Err("MCP worker requires an absolute store path".into());
    }
    Ok(request)
}

fn dispatch(request: Request) -> Result<String, String> {
    // Only this explicitly requested store participates in configuration.
    crate::config::apply_lang(&request.root);
    match request.operation {
        Operation::Tool {
            run,
            authorized,
            name,
            args,
        } => {
            crate::mcp::validate(&name, &args)?;
            if crate::mcp::reads(&name) == Some(false) && !authorized {
                return Err(msg::agent_declined().into());
            }
            let node = crate::Node::open(request.root.clone())?;
            if node
                .khor_agent_config()?
                .is_none_or(|c| c.home != node.device_str())
            {
                return Err(msg::agent_not_configured().into());
            }
            crate::mcp::call_with_parent(
                &request.root,
                &name,
                &args,
                Some(crate::delegation::TaskRef {
                    home: node.device(),
                    id: run,
                }),
            )
        }
        Operation::CancelChildren { run } => {
            crate::delegation::cancel_children(&request.root, &run)?;
            Ok(String::new())
        }
    }
}

struct Encoded(Vec<u8>);
impl Write for Encoded {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > OUTPUT_LIMIT - self.0.len() {
            return Err(io::Error::other(
                "MCP worker encoded result exceeded its byte budget",
            ));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
pub(crate) fn encode_message(value: &Value) -> Result<Vec<u8>, String> {
    let mut encoded = Encoded(Vec::new());
    serde_json::to_writer(&mut encoded, value)
        .map_err(|_| "MCP bridge response exceeded its encoded byte budget".to_owned())?;
    encoded
        .write_all(b"\n")
        .map_err(|_| "MCP bridge response exceeded its encoded byte budget".to_owned())?;
    Ok(encoded.0)
}
fn write_result(mut writer: impl Write, result: Result<String, String>) -> io::Result<()> {
    let result = match result {
        Ok(text) if text.len() > TEXT_LIMIT => {
            Err("MCP tool result exceeded its byte budget".into())
        }
        result => result,
    };
    let mut encoded = Encoded(Vec::new());
    if serde_json::to_writer(&mut encoded, &result).is_err() {
        // Nothing has been emitted: oversize is an error, never truncated success.
        encoded.0.clear();
        serde_json::to_writer(
            &mut encoded,
            &Result::<String, _>::Err("MCP worker encoded result exceeded its byte budget"),
        )
        .map_err(io::Error::other)?;
    }
    writer.write_all(&encoded.0)?;
    writer.flush()
}

enum Event {
    Input(io::Result<()>),
    Output(Result<Vec<u8>, String>),
    Errors(io::Result<()>),
}
#[derive(Default)]
struct Results {
    input: bool,
    output: Option<Vec<u8>>,
    errors: bool,
    failure: Option<String>,
}
impl Results {
    fn receive(&mut self, event: Event) {
        match event {
            Event::Input(Ok(())) => self.input = true,
            Event::Output(Ok(bytes)) => self.output = Some(bytes),
            Event::Errors(Ok(())) => self.errors = true,
            _ => {
                self.failure
                    .get_or_insert("MCP worker I/O failed or exceeded its byte budget".into());
            }
        }
    }
    fn complete(&self) -> bool {
        self.input && self.output.is_some() && self.errors
    }
}

/// One owner, one process, three joined I/O workers, and one absolute budget.
pub(crate) struct Worker {
    pub control: Arc<Control>,
    session: Option<Session>,
    threads: Vec<JoinHandle<()>>,
    events: mpsc::Receiver<Event>,
    results: Results,
    status: Option<ExitStatus>,
    deadline: Instant,
}
impl Worker {
    pub(crate) fn spawn(
        request: Request,
        control: Arc<Control>,
        budget: Duration,
        path: Option<String>,
    ) -> Result<Self, String> {
        let deadline = Instant::now()
            .checked_add(budget)
            .ok_or("MCP worker deadline overflow")?;
        let bytes =
            serde_json::to_vec(&request).map_err(|_| "MCP worker request could not be encoded")?;
        if bytes.len() > INPUT_LIMIT {
            return Err("MCP worker input exceeded its byte budget".into());
        }
        let mut command = command()?;
        command.arg("_mcp_worker").env_clear();
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
            "http_proxy",
            "https_proxy",
            "all_proxy",
            "no_proxy",
            "SHELL",
            "COMSPEC",
            // This is an OS authentication broker address, not a provider
            // credential. Satellite SSH used it in the original Rust process.
            "SSH_AUTH_SOCK",
            // Explicit SSH/secret-store test locations must not fall through
            // to the person's real config or Keychain in a re-exec.
            "KHOR_SSH_DIR",
            "KHOR_SECRET_DIR",
            // Preserve the actual Khor transport policy and lab isolation.
            // These remain private environment values, never stderr/receipts.
            "KHOR_RELAY",
            khor_net::endpoint::RELAY_ONLY_ENV,
            khor_net::endpoint::RELAY_SUCCESSOR_ENV,
            khor_net::endpoint::RELAY_CA_ENV,
            khor_net::endpoint::PUBLIC_RELAYS_ENV,
            khor_net::endpoint::BIND_IP_ENV,
            khor_net::endpoint::NO_MDNS_ENV,
            khor_net::endpoint::LAB_ENV,
            khor_net::carry::SPARE_ENV,
        ] {
            if let Some(value) = std::env::var_os(name) {
                command.env(name, value);
            }
        }
        // Owners never run a login shell. The active executor supplies the
        // resolved path; pre-start cancellation only needs the inherited path.
        let path = path
            .map(std::ffi::OsString::from)
            .or_else(|| std::env::var_os(crate::PATH_ENV))
            .or_else(|| std::env::var_os("PATH"))
            .unwrap_or_default();
        command.env("PATH", &path).env(crate::PATH_ENV, path);
        let (session, pipes) =
            Session::spawn_with_profile(command, control.clone(), SpawnProfile::McpWorker)
                .map_err(|_| "MCP worker could not start")?;
        let (tx, events) = mpsc::sync_channel(3);
        let mut owner = Self {
            control,
            session: Some(session),
            threads: Vec::new(),
            events,
            results: Results::default(),
            status: None,
            deadline,
        };
        let crate::agent_process::Pipes {
            mut stdin,
            stdout,
            stderr,
        } = pipes;
        let input_tx = tx.clone();
        owner.threads.push(
            thread::Builder::new()
                .name("mcp-worker-input".into())
                .spawn(move || {
                    let result = stdin.write_all(&bytes).and_then(|_| stdin.flush());
                    drop(stdin);
                    let _ = input_tx.send(Event::Input(result));
                })
                .map_err(|_| "MCP worker input owner could not start")?,
        );
        let output_tx = tx.clone();
        owner.threads.push(
            thread::Builder::new()
                .name("mcp-worker-output".into())
                .spawn(move || {
                    let _ = output_tx.send(Event::Output(read_output(stdout)));
                })
                .map_err(|_| "MCP worker output owner could not start")?,
        );
        owner.threads.push(
            thread::Builder::new()
                .name("mcp-worker-errors".into())
                .spawn(move || {
                    let _ = tx.send(Event::Errors(drain_errors(stderr)));
                })
                .map_err(|_| "MCP worker error owner could not start")?,
        );
        Ok(owner)
    }

    pub(crate) fn poll(&mut self) -> Option<Result<String, String>> {
        for event in self.events.try_iter() {
            self.results.receive(event);
        }
        if let Some(status) = self.session.as_mut()?.poll_exit() {
            match status {
                Ok(status) => self.status = Some(status),
                Err(_) => {
                    self.results.failure =
                        Some("MCP worker could not confirm process retirement".into());
                }
            }
        }
        if Instant::now() >= self.deadline {
            self.results.failure = Some("MCP worker exceeded its operation budget".into());
        }
        if self.status.is_some_and(|s| !s.success()) {
            self.results.failure = Some("MCP worker exited without a successful result".into());
        }
        if let Some(error) = self.results.failure.take() {
            let retired = self.cancel_and_join();
            return Some(Err(retired.err().unwrap_or(error)));
        }
        if self.status.is_none() || !self.results.complete() {
            return None;
        }
        if self.control.is_cancelled() {
            let retired = self.cancel_and_join();
            return Some(Err(retired
                .err()
                .unwrap_or_else(|| "MCP worker was cancelled".into())));
        }
        if let Err(error) = self.retire() {
            return Some(Err(error));
        }
        if self.control.is_cancelled() {
            self.results.output = None;
            return Some(Err("MCP worker was cancelled".into()));
        }
        let bytes = self.results.output.take().expect("complete output");
        Some(
            serde_json::from_slice(&bytes)
                .unwrap_or_else(|_| Err("MCP worker returned an invalid result".into())),
        )
    }
    pub(crate) fn cancel_and_join(&mut self) -> Result<(), String> {
        self.control.terminate_now();
        // Do not decode or emit a cancelled result, however large its buffer.
        self.results.output = None;
        self.retire()
    }
    fn retire(&mut self) -> Result<(), String> {
        let reaped = self.session.take().map(|mut s| s.finish(true)).transpose();
        let mut panicked = false;
        for handle in self.threads.drain(..) {
            panicked |= handle.join().is_err();
        }
        reaped.map_err(|_| "MCP worker process could not be reaped")?;
        if panicked {
            return Err("MCP worker I/O owner panicked".into());
        }
        Ok(())
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.cancel_and_join();
    }
}

fn command() -> Result<Command, String> {
    // Only unit fixtures can substitute a newly built native CLI. Production
    // always executes the same native binary, including GUI installations.
    #[cfg(test)]
    if let Some(path) = std::env::var_os("KHOR_TEST_MCP_EXECUTABLE") {
        return Ok(Command::new(path));
    }
    Ok(Command::new(crate::self_exe()?))
}

fn read_output(mut reader: impl Read) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    let mut chunk = [0; 8192];
    loop {
        match reader.read(&mut chunk) {
            Ok(0) => return Ok(bytes),
            Ok(count) => {
                if count > OUTPUT_LIMIT - bytes.len() {
                    return Err("MCP worker output exceeded its byte budget".into());
                }
                bytes.extend_from_slice(&chunk[..count]);
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(_) => return Err("MCP worker output could not be read".into()),
        }
    }
}
fn drain_errors(mut reader: impl Read) -> io::Result<()> {
    let mut tail: VecDeque<u8> = VecDeque::new();
    let mut chunk = [0; 8192];
    loop {
        match reader.read(&mut chunk) {
            Ok(0) => return Ok(()),
            Ok(count) => {
                let excess = (tail.len() + count).saturating_sub(STDERR_LIMIT);
                tail.drain(..excess);
                tail.extend(&chunk[..count]);
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
}

/// Finite run-owned sweeps are independent of the local executor claim. Handles
/// are retained and joined, admission is bounded, and durable links are never
/// removed on timeout. Fair per-child scheduling/remote receipts are separate.
pub(crate) fn schedule_cancel(
    root: PathBuf,
    run: String,
    path: Option<String>,
) -> Result<(), String> {
    static SWEEPS: OnceLock<Mutex<Vec<JoinHandle<()>>>> = OnceLock::new();
    let mut sweeps = SWEEPS
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    let mut active = Vec::new();
    for handle in sweeps.drain(..) {
        if handle.is_finished() {
            let _ = handle.join();
        } else {
            active.push(handle);
        }
    }
    *sweeps = active;
    if sweeps.len() >= 4 {
        return Err("Remote cancellation remains pending: sweep capacity is full".into());
    }
    let handle = thread::Builder::new()
        .name("mcp-cancel-sweep".into())
        .spawn(move || {
            let control = Arc::new(Control::default());
            let Ok(mut worker) = Worker::spawn(
                Request {
                    root,
                    operation: Operation::CancelChildren { run },
                },
                control,
                SWEEP_BUDGET,
                path,
            ) else {
                return;
            };
            while worker.poll().is_none() {
                thread::sleep(Duration::from_millis(10));
            }
        })
        .map_err(|_| "Remote cancellation remains pending: sweep owner could not start")?;
    sweeps.push(handle);
    #[cfg(test)]
    SWEEPS_STARTED.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    Ok(())
}

#[cfg(all(test, unix))]
#[path = "mcp_worker_tests.rs"]
mod tests;
