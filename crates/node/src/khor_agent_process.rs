use std::{
    io::{self, Read, Write},
    process::{Command, ExitStatus},
    sync::{Arc, Mutex, mpsc},
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
#[derive(Clone, Copy)]
pub(super) enum SpawnProfile {
    Contained,
    /// Only explicitly designated durable Khor hosts may leave this job.
    McpWorker,
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
    cancelled: bool,
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
    pub(super) fn terminate_now(&self) {
        let mut requests = self.0.lock().unwrap_or_else(|p| p.into_inner());
        requests.cancelled = true;
        requests.finish = true;
        let deadline = Instant::now();
        requests.terminal_deadline = Some(
            requests
                .terminal_deadline
                .map_or(deadline, |was| was.min(deadline)),
        );
    }
    pub(super) fn is_cancelled(&self) -> bool {
        self.0.lock().unwrap_or_else(|p| p.into_inner()).cancelled
    }
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
    exit: Option<mpsc::Receiver<io::Result<ExitStatus>>>,
    #[cfg(unix)]
    pid: u32,
}
impl Session {
    pub(super) fn spawn(command: Command, control: Arc<Control>) -> io::Result<(Self, Pipes)> {
        Self::spawn_with_profile(command, control, SpawnProfile::Contained)
    }
    pub(super) fn spawn_with_profile(
        command: Command,
        control: Arc<Control>,
        profile: SpawnProfile,
    ) -> io::Result<(Self, Pipes)> {
        let (session, pipes, ()) =
            Self::spawn_with_profile_observed(command, control, profile, |_| ())?;
        Ok((session, pipes))
    }
    /// Observe only this owned child, once, before any supervisor can reap it.
    /// The observer is synchronous and must not spawn, await or reenter process
    /// ownership. Failure can be returned as T without losing the real Session.
    pub(super) fn spawn_with_profile_observed<T>(
        command: Command,
        control: Arc<Control>,
        profile: SpawnProfile,
        observe: impl FnOnce(Option<u32>) -> T,
    ) -> io::Result<(Self, Pipes, T)> {
        let (mut process, pipes) = platform::spawn_with_profile(command, profile)?;
        #[cfg(unix)]
        let pid = process.id();
        #[cfg(unix)]
        let observed = observe(Some(pid));
        #[cfg(not(unix))]
        let observed = observe(None);
        let requests = control.clone();
        let (exit_tx, exit_rx) = mpsc::sync_channel(1);
        let supervisor = std::thread::Builder::new()
            .name("agent-supervisor".into())
            .spawn(move || {
                let result: io::Result<ExitStatus> = (|| {
                    let mut reaped = None;
                    loop {
                        let (finish, expired) = {
                            let mut requests = requests.0.lock().unwrap_or_else(|p| p.into_inner());
                            let expired = requests.expired(Instant::now());
                            (requests.finish, expired)
                        };
                        if reaped.is_none() {
                            let exited = match process.exited() {
                                Err(error) if error.kind() == io::ErrorKind::Interrupted => {
                                    continue;
                                }
                                result => result?,
                            };
                            if exited {
                                requests.natural_exit();
                            }
                            if exited || expired {
                                process.terminate()?;
                                let status = process.wait()?;
                                reaped = Some(status);
                                // This receipt confirms process cleanup, not pipe EOF.
                                // Keep buffered tail readable until finish or expiry.
                                let _ = exit_tx.try_send(Ok(status));
                            }
                        }
                        if (finish || expired) && reaped.is_some() {
                            process.cancel_io();
                            return Ok(reaped.unwrap());
                        }
                        std::thread::sleep(Duration::from_millis(20));
                    }
                })();
                if let Err(error) = &result {
                    let _ = exit_tx.try_send(Err(io::Error::new(error.kind(), error.to_string())));
                }
                result
            })?;
        Ok((
            Self {
                control,
                supervisor: Some(supervisor),
                exit: Some(exit_rx),
                #[cfg(unix)]
                pid,
            },
            pipes,
            observed,
        ))
    }
    /// One process-reap receipt. Pending and polls after the first receipt are
    /// None; an unexpectedly closed sender yields one Err. Still call finish /
    /// wait and join I/O owners: the receipt deliberately does not cancel pipes.
    pub(super) fn poll_exit(&mut self) -> Option<io::Result<ExitStatus>> {
        match self.exit.as_ref()?.try_recv() {
            Ok(result) => {
                self.exit = None;
                Some(result)
            }
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.exit = None;
                Some(Err(io::Error::other(
                    "agent supervisor closed without exit receipt",
                )))
            }
        }
    }
    #[cfg(unix)]
    pub(super) fn pid(&self) -> u32 {
        self.pid
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

#[cfg(all(test, unix))]
#[path = "khor_agent_process_receipt_tests.rs"]
mod receipt_tests;
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
