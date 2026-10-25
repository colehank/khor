use std::{
    io::{self, Read, Write},
    process::{Command, ExitStatus},
    sync::{Arc, Mutex},
    thread::JoinHandle,
    time::{Duration, Instant},
};
#[cfg(unix)]
#[path = "khor_agent_process_unix.rs"]
mod platform;
#[cfg(windows)]
#[path = "khor_agent_process_windows.rs"]
mod platform;

const EXIT_GRACE: Duration = Duration::from_secs(10);
pub(super) struct Pipes {
    pub stdin: Box<dyn Write + Send>,
    pub stdout: Box<dyn Read + Send>,
    pub stderr: Box<dyn Read + Send>,
}
#[derive(Clone, Debug)]
pub(super) struct Operation {
    owner: Arc<()>,
    generation: u64,
}
impl PartialEq for Operation {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner) && self.generation == other.generation
    }
}
impl Eq for Operation {}

struct ActiveOperation {
    id: Operation,
    deadline: Option<Instant>,
}

#[derive(Default)]
struct Requests {
    owner: Arc<()>,
    terminal_deadline: Option<Instant>,
    operation: Option<ActiveOperation>,
    generation: u64,
    finish: bool,
    terminating: bool,
}
impl Requests {
    fn expired(&mut self, now: Instant) -> bool {
        let expired = self
            .terminal_deadline
            .is_some_and(|deadline| now >= deadline)
            || self
                .operation
                .as_ref()
                .and_then(|operation| operation.deadline)
                .is_some_and(|deadline| now >= deadline);
        if expired {
            self.terminating = true;
        }
        expired
    }
}

#[derive(Default)]
pub(super) struct Control(Mutex<Requests>);
impl Control {
    pub(super) fn stop(&self) {
        self.request_terminal(EXIT_GRACE, false);
    }
    fn request_terminal(&self, grace: Duration, finish: bool) {
        let mut requests = self.0.lock().unwrap_or_else(|p| p.into_inner());
        requests.finish |= finish;
        let deadline = Instant::now() + grace;
        requests.terminal_deadline = Some(
            requests
                .terminal_deadline
                .map_or(deadline, |was| was.min(deadline)),
        );
    }
    pub(super) fn begin_operation(&self) -> Option<Operation> {
        let mut requests = self.0.lock().unwrap_or_else(|p| p.into_inner());
        if requests.expired(Instant::now())
            || requests.terminating
            || requests.terminal_deadline.is_some()
            || requests.operation.is_some()
        {
            return None;
        }
        requests.generation = requests.generation.checked_add(1)?;
        let id = Operation {
            owner: requests.owner.clone(),
            generation: requests.generation,
        };
        requests.operation = Some(ActiveOperation {
            id: id.clone(),
            deadline: None,
        });
        Some(id)
    }
    pub(super) fn cancel_operation(&self, id: &Operation, grace: Duration) -> bool {
        let mut requests = self.0.lock().unwrap_or_else(|p| p.into_inner());
        if requests.expired(Instant::now()) || requests.terminating {
            return false;
        }
        let Some(operation) = requests
            .operation
            .as_mut()
            .filter(|operation| operation.id == *id)
        else {
            return false;
        };
        operation
            .deadline
            .get_or_insert_with(|| Instant::now() + grace);
        true
    }
    pub(super) fn complete_operation(&self, id: &Operation) -> bool {
        let mut requests = self.0.lock().unwrap_or_else(|p| p.into_inner());
        if requests.expired(Instant::now()) || requests.terminating {
            return false;
        }
        if !requests
            .operation
            .as_ref()
            .is_some_and(|operation| operation.id == *id)
        {
            return false;
        }
        requests.operation = None;
        true
    }
    pub(super) fn shutdown(&self) {
        self.finish(true);
    }
    fn finish(&self, force: bool) {
        self.request_terminal(if force { Duration::ZERO } else { EXIT_GRACE }, true);
    }
    fn natural_exit(&self) {
        let mut requests = self.0.lock().unwrap_or_else(|p| p.into_inner());
        requests.terminating = true;
        let deadline = Instant::now() + EXIT_GRACE;
        requests.terminal_deadline = Some(
            requests
                .terminal_deadline
                .map_or(deadline, |was| was.min(deadline)),
        );
    }
}
pub(super) struct Session {
    control: Arc<Control>,
    supervisor: Option<JoinHandle<io::Result<ExitStatus>>>,
}
impl Session {
    pub(super) fn spawn(command: Command, control: Arc<Control>) -> io::Result<(Self, Pipes)> {
        let (mut process, pipes) = platform::spawn(command)?;
        let requests = control.clone();
        let supervisor = std::thread::Builder::new()
            .name("agent-supervisor".into())
            .spawn(move || {
                let mut reaped = None;
                loop {
                    let (finish, expired) = {
                        let mut requests = requests.0.lock().unwrap_or_else(|p| p.into_inner());
                        let expired = requests.expired(Instant::now());
                        (requests.finish, expired)
                    };
                    if reaped.is_none() {
                        let exited = match process.exited() {
                            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                            result => result?,
                        };
                        if exited {
                            requests.natural_exit();
                        }
                        if exited || expired {
                            process.terminate()?;
                            reaped = Some(process.wait()?);
                        }
                    }
                    if (finish || expired) && reaped.is_some() {
                        process.cancel_io();
                        return Ok(reaped.unwrap());
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
            })?;
        Ok((
            Self {
                control,
                supervisor: Some(supervisor),
            },
            pipes,
        ))
    }
    pub(super) fn finish(&mut self, force: bool) -> io::Result<ExitStatus> {
        self.control.finish(force);
        self.wait()
    }
    pub(super) fn wait(&mut self) -> io::Result<ExitStatus> {
        self.supervisor
            .take()
            .expect("agent supervisor joined once")
            .join()
            .map_err(|_| io::Error::other("agent supervisor panicked"))?
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        if let Some(supervisor) = self.supervisor.take() {
            self.control.finish(true);
            let _ = supervisor.join();
        }
    }
}

#[cfg(all(test, unix))]
#[path = "khor_agent_process_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "khor_agent_process_control_tests.rs"]
mod control_tests;
