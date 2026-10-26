//! One finite local command. The caller owns the input buffer; three workers
//! share its absolute deadline and are joined after the owned process is reaped.
use crate::agent_process::{Control, Session};
use std::{
    collections::VecDeque,
    io::{self, Read, Write},
    process::{Command, ExitStatus},
    sync::{Arc, mpsc},
    thread::{self, ScopedJoinHandle},
    time::{Duration, Instant},
};

pub(super) const STDOUT_LIMIT: usize = 1024 * 1024;
pub(super) const STDERR_LIMIT: usize = 256 * 1024;

pub(super) struct Output {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub stderr_truncated: bool,
    pub timed_out: bool,
}

struct Tail {
    bytes: Vec<u8>,
    truncated: bool,
    error: Option<String>,
}
enum Event {
    Input(io::Result<()>),
    Stdout(Result<Vec<u8>, String>),
    Stderr(Tail),
}
#[derive(Default)]
struct Results {
    input: Option<io::Result<()>>,
    stdout: Option<Vec<u8>>,
    stderr: Option<Tail>,
    error: Option<String>,
}
impl Results {
    fn receive(&mut self, event: Event) {
        match event {
            Event::Input(result) => self.input = Some(result),
            Event::Stdout(Ok(bytes)) => self.stdout = Some(bytes),
            Event::Stdout(Err(error)) => {
                self.error.get_or_insert(error);
            }
            Event::Stderr(tail) => {
                if let Some(error) = &tail.error {
                    self.error.get_or_insert(error.clone());
                }
                self.stderr = Some(tail);
            }
        }
    }
    fn complete(&self) -> bool {
        self.input.is_some() && self.stdout.is_some() && self.stderr.is_some()
    }
}

struct Owner<'scope> {
    session: Option<Session>,
    control: Arc<Control>,
    workers: Vec<ScopedJoinHandle<'scope, ()>>,
}
impl Owner<'_> {
    fn finish(&mut self) -> Result<ExitStatus, String> {
        self.control.shutdown();
        let status = self
            .session
            .take()
            .expect("owner joins session once")
            .finish(true)
            .map_err(|error| error.to_string());
        let mut panicked = false;
        for worker in self.workers.drain(..) {
            panicked |= worker.join().is_err();
        }
        let status = status?;
        if panicked {
            return Err("SSH I/O worker panicked".into());
        }
        Ok(status)
    }
}
impl Drop for Owner<'_> {
    fn drop(&mut self) {
        self.control.shutdown();
        if let Some(mut session) = self.session.take() {
            let _ = session.finish(true);
        }
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

fn stdout(mut reader: Box<dyn Read + Send>) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    let mut chunk = [0; 8192];
    loop {
        let count = match reader.read(&mut chunk) {
            Ok(0) => return Ok(bytes),
            Ok(count) => count,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(format!("SSH stdout read failed: {error}")),
        };
        if count > STDOUT_LIMIT - bytes.len() {
            return Err(format!(
                "SSH stdout exceeded {STDOUT_LIMIT} byte capture budget"
            ));
        }
        bytes.extend_from_slice(&chunk[..count]);
    }
}

fn stderr(mut reader: Box<dyn Read + Send>) -> Tail {
    let mut bytes = VecDeque::new();
    let mut chunk = [0; 8192];
    let mut truncated = false;
    let error = loop {
        let count = match reader.read(&mut chunk) {
            Ok(0) => break None,
            Ok(count) => count,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => break Some(format!("SSH stderr read failed: {error}")),
        };
        let excess = (bytes.len() + count).saturating_sub(STDERR_LIMIT);
        if excess != 0 {
            bytes.drain(..excess);
            truncated = true;
        }
        bytes.extend(&chunk[..count]);
    };
    Tail {
        bytes: bytes.into_iter().collect(),
        truncated,
        error,
    }
}

pub(super) fn run(command: Command, input: &[u8], wait: Duration) -> Result<Output, String> {
    run_with_control(command, input, wait, Arc::new(Control::default()))
}

pub(super) fn run_with_control(
    command: Command,
    input: &[u8],
    wait: Duration,
    control: Arc<Control>,
) -> Result<Output, String> {
    // Spawn, feeding stdin, process execution and output draining share this
    // instant. An OS spawn call itself cannot be interrupted by this owner.
    let deadline = Instant::now()
        .checked_add(wait)
        .ok_or("SSH deadline overflow")?;
    let (session, pipes) = Session::spawn(command, control.clone())
        .map_err(|error| format!("SSH process could not start: {error}"))?;
    thread::scope(|scope| {
        let mut owner = Owner {
            session: Some(session),
            control,
            workers: Vec::new(),
        };
        // Each worker sends exactly one bounded result; no byte queue grows
        // while the controlling thread waits for process cleanup.
        let (tx, rx) = mpsc::sync_channel(3);
        let crate::agent_process::Pipes {
            mut stdin,
            stdout: output,
            stderr: errors,
        } = pipes;
        let input_tx = tx.clone();
        owner.workers.push(
            thread::Builder::new()
                .name("ssh-stdin".into())
                .spawn_scoped(scope, move || {
                    let result = stdin.write_all(input).and_then(|_| stdin.flush());
                    drop(stdin); // EOF is part of the remote command's input contract.
                    let _ = input_tx.send(Event::Input(result));
                })
                .map_err(|error| error.to_string())?,
        );
        let output_tx = tx.clone();
        owner.workers.push(
            thread::Builder::new()
                .name("ssh-stdout".into())
                .spawn_scoped(scope, move || {
                    let _ = output_tx.send(Event::Stdout(stdout(output)));
                })
                .map_err(|error| error.to_string())?,
        );
        owner.workers.push(
            thread::Builder::new()
                .name("ssh-stderr".into())
                .spawn_scoped(scope, move || {
                    let _ = tx.send(Event::Stderr(stderr(errors)));
                })
                .map_err(|error| error.to_string())?,
        );
        let mut results = Results::default();
        let mut exited = false;
        let mut timed_out = false;
        loop {
            if let Some(result) = owner.session.as_mut().unwrap().poll_exit() {
                match result {
                    Ok(_) => exited = true,
                    Err(error) => {
                        results.error.get_or_insert(error.to_string());
                        break;
                    }
                }
            }
            if results.error.is_some() {
                break;
            }
            let now = Instant::now();
            if now >= deadline {
                timed_out = true;
                break;
            }
            if exited && results.complete() {
                break;
            }
            match rx.recv_timeout((deadline - now).min(Duration::from_millis(10))) {
                Ok(event) => results.receive(event),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    if !results.complete() {
                        results
                            .error
                            .get_or_insert("SSH I/O worker closed without result".into());
                        break;
                    }
                    // All pipes can close before the process exits. Keep the
                    // original deadline; EOF alone is not successful execution.
                    thread::sleep((deadline - now).min(Duration::from_millis(10)));
                }
            }
        }
        let status = owner.finish();
        for event in rx.try_iter() {
            results.receive(event);
        }
        if let Some(error) = results.error.filter(|_| !timed_out) {
            return Err(match status {
                Ok(_) => error,
                Err(cleanup) => format!("{error}; SSH cleanup failed: {cleanup}"),
            });
        }
        let status = status?;
        if status.success() && !timed_out {
            results
                .input
                .ok_or("SSH stdin worker did not finish")?
                .map_err(|error| format!("SSH input was not fully written: {error}"))?;
        }
        let tail = results.stderr.ok_or("SSH stderr worker did not finish")?;
        Ok(Output {
            status,
            stdout: results.stdout.unwrap_or_default(),
            stderr: tail.bytes,
            stderr_truncated: tail.truncated,
            timed_out,
        })
    })
}
