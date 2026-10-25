use crate::agent_process::{Control, Pipes, Session};
use agent_client_protocol::{AcpAgent, ByteStreams};
use std::{
    collections::VecDeque,
    io::Read,
    process::Command,
    str::FromStr,
    sync::{Arc, Mutex},
    thread::JoinHandle,
};
use tokio::sync::watch;

#[path = "acp_process_io.rs"]
mod io;

pub(crate) struct Transport {
    pub streams: ByteStreams<io::Writer, io::Reader>,
    pub lifecycle: khor_acp::TransportLifecycle,
    pub control: Arc<Control>,
    pub stderr: Arc<Mutex<VecDeque<u8>>>,
}

struct Building {
    session: Option<Session>,
    control: Arc<Control>,
    input: Option<Arc<io::Queue<io::WriteOp>>>,
    output: Option<Arc<io::Queue<std::io::Result<Vec<u8>>>>>,
    workers: Vec<JoinHandle<()>>,
}
impl Building {
    fn close_queues(&self) {
        if let Some(input) = &self.input {
            input.close();
        }
        if let Some(output) = &self.output {
            output.close();
        }
    }
    fn join_workers(&mut self) -> Result<(), String> {
        let mut error = None;
        for worker in self.workers.drain(..) {
            if worker.join().is_err() {
                error = Some("ACP I/O worker panicked".to_owned());
            }
        }
        error.map_or(Ok(()), Err)
    }
    fn run(mut self) -> Result<khor_acp::ProcessExit, String> {
        let mut session = self.session.take().expect("building owns session");
        let status = session.wait().map_err(|error| error.to_string());
        self.close_queues();
        let workers = self.join_workers();
        let status = status?;
        workers?;
        Ok(khor_acp::ProcessExit {
            code: status.code(),
        })
    }
}
impl Drop for Building {
    fn drop(&mut self) {
        self.close_queues();
        self.control.shutdown();
        if let Some(mut session) = self.session.take() {
            let _ = session.finish(true);
        }
        let _ = self.join_workers();
    }
}

/// Keep ACP's inherited environment/cwd; Pi's model-specific allowlist is unrelated.
fn command_of(command: &str) -> Result<Command, String> {
    let config = AcpAgent::from_str(command)
        .map_err(|error| error.to_string())?
        .into_config();
    let mut command = Command::new(config.command());
    command.args(config.arguments());
    // The Windows owned-process adapter consumes only explicit Command environment.
    command
        .env_clear()
        .envs(std::env::vars_os())
        .envs(config.environment());
    command.current_dir(std::env::current_dir().map_err(|error| error.to_string())?);
    Ok(command)
}

pub(crate) fn spawn(command: &str) -> Result<Transport, String> {
    let command = command_of(command)?;
    let control = Arc::new(Control::default());
    let (
        session,
        Pipes {
            stdin,
            stdout,
            mut stderr,
        },
    ) = Session::spawn(command, control.clone()).map_err(|error| error.to_string())?;
    let mut building = Building {
        session: Some(session),
        control: control.clone(),
        input: None,
        output: None,
        workers: Vec::new(),
    };
    let (reader, output, worker) = io::read_pipe(stdout).map_err(|error| error.to_string())?;
    building.output = Some(output.clone());
    building.workers.push(worker);
    let (writer, input, worker) = io::write_pipe(stdin).map_err(|error| error.to_string())?;
    building.input = Some(input.clone());
    building.workers.push(worker);
    let tail = Arc::new(Mutex::new(VecDeque::new()));
    let captured = tail.clone();
    building.workers.push(
        std::thread::Builder::new()
            .name("acp-stderr".into())
            .spawn(move || {
                let mut bytes = [0u8; io::CHUNK];
                loop {
                    match stderr.read(&mut bytes) {
                        Ok(0) => break,
                        Ok(n) => {
                            let mut tail = captured.lock().unwrap_or_else(|p| p.into_inner());
                            tail.extend(&bytes[..n]);
                            let excess = tail.len().saturating_sub(64 * 1024);
                            tail.drain(..excess);
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                        Err(_) => break,
                    }
                }
            })
            .map_err(|error| error.to_string())?,
    );
    let (done, completion) = watch::channel(None);
    std::thread::Builder::new()
        .name("acp-process-owner".into())
        .spawn(move || {
            let result = building.run();
            done.send_replace(Some(result));
        })
        .map_err(|error| error.to_string())?;
    let requests = control.clone();
    let lifecycle = khor_acp::TransportLifecycle::new(
        move || {
            input.close();
            output.close();
            requests.shutdown();
        },
        completion,
    );
    Ok(Transport {
        streams: ByteStreams::new(writer, reader),
        lifecycle,
        control,
        stderr: tail,
    })
}

#[cfg(test)]
#[path = "acp_process_tests.rs"]
mod tests;
