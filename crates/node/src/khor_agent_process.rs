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
#[derive(Default)]
struct Requests {
    deadline: Option<Instant>,
    finish: bool,
}
#[derive(Default)]
pub(super) struct Control(Mutex<Requests>);
impl Control {
    pub(super) fn stop(&self) {
        let mut requests = self.0.lock().unwrap_or_else(|p| p.into_inner());
        requests
            .deadline
            .get_or_insert_with(|| Instant::now() + EXIT_GRACE);
    }
    fn finish(&self, force: bool) {
        let mut requests = self.0.lock().unwrap_or_else(|p| p.into_inner());
        requests.finish = true;
        let deadline = Instant::now() + if force { Duration::ZERO } else { EXIT_GRACE };
        requests.deadline = Some(requests.deadline.map_or(deadline, |was| was.min(deadline)));
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
                        let requests = requests.0.lock().unwrap_or_else(|p| p.into_inner());
                        (
                            requests.finish,
                            requests
                                .deadline
                                .is_some_and(|deadline| Instant::now() >= deadline),
                        )
                    };
                    if reaped.is_none() {
                        let exited = match process.exited() {
                            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                            result => result?,
                        };
                        if exited || expired {
                            process.terminate()?;
                            reaped = Some(process.wait()?);
                            // An unrelated inherited pipe must not keep an already
                            // exited run alive forever, even without a final event.
                            requests.stop();
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
