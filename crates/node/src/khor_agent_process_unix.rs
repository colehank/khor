use super::Pipes;
use std::{
    io::{self, Read, Write},
    os::{fd::AsRawFd, unix::process::CommandExt},
    process::{Child, Command, ExitStatus, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

pub(super) struct Process {
    child: Child,
    owns_group: bool,
    cancelled: Arc<AtomicBool>,
}

struct Pipe<T> {
    inner: T,
    cancelled: Arc<AtomicBool>,
}
impl<T: AsRawFd> Pipe<T> {
    fn new(inner: T, cancelled: &Arc<AtomicBool>) -> io::Result<Self> {
        let fd = inner.as_raw_fd();
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self {
            inner,
            cancelled: cancelled.clone(),
        })
    }
    fn retry(&self) -> io::Result<()> {
        if self.cancelled.load(Ordering::Acquire) {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "agent pipe closed",
            ));
        }
        std::thread::sleep(Duration::from_millis(10));
        Ok(())
    }
}
impl<T: Read + AsRawFd> Read for Pipe<T> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        loop {
            if self.cancelled.load(Ordering::Acquire) {
                return Ok(0);
            }
            match self.inner.read(bytes) {
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => self.retry()?,
                result => return result,
            }
        }
    }
}
impl<T: Write + AsRawFd> Write for Pipe<T> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        loop {
            if self.cancelled.load(Ordering::Acquire) {
                return Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "agent pipe closed",
                ));
            }
            match self.inner.write(bytes) {
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => self.retry()?,
                result => return result,
            }
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

pub(super) fn spawn(mut command: Command) -> io::Result<(Process, Pipes)> {
    command
        .process_group(0)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let child = command.spawn()?;
    let cancelled = Arc::new(AtomicBool::new(false));
    let mut process = Process {
        child,
        owns_group: true,
        cancelled,
    };
    let pipes = Pipes {
        stdin: Box::new(Pipe::new(
            process.child.stdin.take().unwrap(),
            &process.cancelled,
        )?),
        stdout: Box::new(Pipe::new(
            process.child.stdout.take().unwrap(),
            &process.cancelled,
        )?),
        stderr: Box::new(Pipe::new(
            process.child.stderr.take().unwrap(),
            &process.cancelled,
        )?),
    };
    Ok((process, pipes))
}

pub(super) fn spawn_with_profile(
    command: Command,
    _profile: super::SpawnProfile,
) -> io::Result<(Process, Pipes)> {
    spawn(command)
}
impl Process {
    #[cfg(test)]
    pub(super) fn id(&self) -> u32 {
        self.child.id()
    }
    pub(super) fn exited(&mut self) -> io::Result<bool> {
        // WNOWAIT preserves the leader's PID as the process-group ownership
        // anchor. Reaping before killpg could target a reused process group.
        let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
        let status = unsafe {
            libc::waitid(
                libc::P_PID,
                self.child.id(),
                &mut info,
                libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
            )
        };
        if status < 0 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() == Some(libc::ECHILD) {
                self.owns_group = false;
            }
            return Err(error);
        }
        Ok(info.si_signo == libc::SIGCHLD)
    }
    pub(super) fn terminate(&mut self) -> io::Result<()> {
        if !self.owns_group {
            return Err(io::Error::other("agent process ownership lost"));
        }
        let status = unsafe { libc::kill(-(self.child.id() as libc::pid_t), libc::SIGKILL) };
        if status < 0 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() == Some(libc::ESRCH) {
                return Ok(());
            }
            #[cfg(target_os = "macos")]
            if error.raw_os_error() == Some(libc::EPERM) {
                // Darwin can make an exiting group unsignalable before waitid
                // exposes its exit. Keep our unreaped PID anchor and briefly
                // wait for that evidence; EPERM itself is never confirmation.
                let deadline = std::time::Instant::now() + Duration::from_millis(100);
                loop {
                    match self.exited() {
                        Ok(true) => {
                            // A full/truncated list or any other member retains
                            // the original permission error; do not guess that
                            // descendants have exited merely because we did.
                            let mut members = [0 as libc::pid_t; 2];
                            let count = unsafe {
                                libc::proc_listpgrppids(
                                    self.child.id() as libc::pid_t,
                                    members.as_mut_ptr().cast(),
                                    std::mem::size_of_val(&members) as libc::c_int,
                                )
                            };
                            if count == 1 && members[0] == self.child.id() as libc::pid_t {
                                return Ok(());
                            }
                            break;
                        }
                        Ok(false) => {}
                        Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                        Err(error) => return Err(error),
                    }
                    if std::time::Instant::now() >= deadline {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(1));
                }
            }
            return Err(error);
        }
        Ok(())
    }
    pub(super) fn wait(&mut self) -> io::Result<ExitStatus> {
        self.owns_group = false;
        self.child.wait()
    }
    pub(super) fn cancel_io(&mut self) {
        self.cancelled.store(true, Ordering::Release);
    }
}
impl Drop for Process {
    fn drop(&mut self) {
        self.cancel_io();
        if self.owns_group {
            let _ = self.terminate();
            self.owns_group = false;
            let _ = self.child.wait();
        }
    }
}

#[cfg(all(test, target_os = "macos"))]
#[path = "khor_agent_process_unix_tests.rs"]
mod tests;
