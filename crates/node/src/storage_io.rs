//! Admission and file work must never borrow a network runtime worker.
//! A job retains its seat until the synchronous call actually returns.

use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, mpsc};
use std::time::Duration;

use khor_catalog::msg;
use tokio::sync::{Semaphore, oneshot};

const CONTROL_WITHIN: Duration = Duration::from_secs(2);
const NETWORK_FILE_WITHIN: Duration = Duration::from_secs(3);
const RECEIPTS_WITHIN: Duration = Duration::from_millis(150);

type Job = Box<dyn FnOnce() + Send>;

pub(crate) struct StorageIo {
    pub(crate) control: Lane,
    pub(crate) observations: Lane,
    pub(crate) routing: Lane,
    pub(crate) receipts: Lane,
    local_files: Lane,
    network_files: Lane,
    mounts: Vec<Mount>,
}

impl Default for StorageIo {
    fn default() -> Self {
        Self {
            control: Lane::new("khor-storage-control", 8, Some(CONTROL_WITHIN)),
            observations: Lane::new("khor-storage-observations", 4, Some(CONTROL_WITHIN)),
            routing: Lane::new("khor-storage-routing", 4, Some(CONTROL_WITHIN)),
            receipts: Lane::new("khor-storage-receipts", 2, Some(RECEIPTS_WITHIN)),
            local_files: Lane::new("khor-storage-local", 16, None),
            network_files: Lane::new("khor-storage-network", 4, Some(NETWORK_FILE_WITHIN)),
            mounts: mounts(),
        }
    }
}

impl StorageIo {
    pub(crate) fn file(&self, path: &str) -> &Lane {
        let home;
        let path = if path.is_empty() {
            home = std::env::home_dir();
            home.as_deref().unwrap_or_else(|| Path::new(""))
        } else {
            Path::new(path)
        };
        if self.network_path(path) {
            &self.network_files
        } else {
            &self.local_files
        }
    }

    pub(crate) fn move_file(&self, from: &str, to: &str) -> &Lane {
        if self.network_path(Path::new(from)) || self.network_path(Path::new(to)) {
            &self.network_files
        } else {
            &self.local_files
        }
    }

    #[cfg(test)]
    pub(crate) fn test_network_mount(&mut self, target: PathBuf, within: Duration) {
        self.mounts.insert(
            0,
            Mount {
                target,
                network: true,
            },
        );
        self.network_files = Lane::new("khor-test-network-files", 1, Some(within));
    }

    fn network_path(&self, path: &Path) -> bool {
        let path = normalize(path);
        self.mounts
            .iter()
            .find(|mount| path.starts_with(&mount.target))
            .is_some_and(|mount| mount.network)
            // Listing a mount's containing directory stats that entry too.
            || self.mounts.iter().any(|mount| {
                mount.network && mount.target.parent().is_some_and(|parent| parent == path)
            })
    }
}

#[derive(Debug)]
struct Mount {
    target: PathBuf,
    network: bool,
}

fn normalize(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for part in path.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other),
        }
    }
    normalized
}

#[cfg(target_os = "linux")]
fn mounts() -> Vec<Mount> {
    std::fs::read_to_string("/proc/self/mountinfo")
        .ok()
        .map_or_else(Vec::new, |text| parse_mounts(&text))
}

#[cfg(not(target_os = "linux"))]
fn mounts() -> Vec<Mount> {
    Vec::new()
}

#[cfg(target_os = "linux")]
fn parse_mounts(text: &str) -> Vec<Mount> {
    use std::os::unix::ffi::OsStringExt;

    let mut mounts: Vec<_> = text
        .lines()
        .filter_map(|line| {
            let (mount, fs) = line.split_once(" - ")?;
            let target = mount.split_whitespace().nth(4)?;
            let kind = fs.split_whitespace().next()?;
            let mut bytes = Vec::with_capacity(target.len());
            let mut src = target.as_bytes().iter().copied();
            while let Some(byte) = src.next() {
                if byte != b'\\' {
                    bytes.push(byte);
                    continue;
                }
                let mut decoded = 0u16;
                for _ in 0..3 {
                    let digit = src.next()?.checked_sub(b'0').filter(|d| *d < 8)?;
                    decoded = decoded * 8 + u16::from(digit);
                }
                bytes.push(u8::try_from(decoded).ok()?);
            }
            Some(Mount {
                target: PathBuf::from(std::ffi::OsString::from_vec(bytes)),
                network: matches!(kind, "nfs" | "nfs4" | "cifs" | "smb3" | "fuse.sshfs" | "9p"),
            })
        })
        .collect();
    // A local bind at a deeper component can sit inside an NFS mount.
    mounts.sort_by_key(|mount| std::cmp::Reverse(mount.target.components().count()));
    mounts
}

pub(crate) struct Lane {
    name: &'static str,
    workers: usize,
    within: Option<Duration>,
    seats: Arc<Semaphore>,
    worker_queue: OnceLock<Result<mpsc::Sender<Job>, String>>,
}

impl Lane {
    fn new(name: &'static str, workers: usize, within: Option<Duration>) -> Self {
        Self {
            name,
            workers,
            within,
            seats: Arc::new(Semaphore::new(workers)),
            worker_queue: OnceLock::new(),
        }
    }

    fn workers(&self) -> Result<&mpsc::Sender<Job>, String> {
        self.worker_queue
            .get_or_init(|| {
                let (send, jobs) = mpsc::channel::<Job>();
                let jobs = Arc::new(Mutex::new(jobs));
                for _ in 0..self.workers {
                    let jobs = jobs.clone();
                    std::thread::Builder::new()
                        .name(self.name.into())
                        .spawn(move || {
                            loop {
                                let next = jobs.lock().unwrap_or_else(|p| p.into_inner()).recv();
                                let Ok(job) = next else { break };
                                // A panic drops only this job and its seat.
                                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(job));
                            }
                        })
                        .map_err(|error| error.to_string())?;
                }
                Ok(send)
            })
            .as_ref()
            .map_err(Clone::clone)
    }

    pub(crate) async fn read<T: Send + 'static>(
        &self,
        work: impl FnOnce() -> Result<T, String> + Send + 'static,
    ) -> Result<T, String> {
        self.run(work, msg::storage_io_busy(), false).await
    }

    /// Allow brief bursts of authenticated requests to share these workers.
    /// Even once the authority stalls, an admission waits only to its deadline.
    pub(crate) async fn admit<T: Send + 'static>(
        &self,
        work: impl FnOnce() -> Result<T, String> + Send + 'static,
    ) -> Result<T, String> {
        self.run(work, msg::storage_io_busy(), true).await
    }

    pub(crate) async fn write<T: Send + 'static>(
        &self,
        work: impl FnOnce() -> Result<T, String> + Send + 'static,
    ) -> Result<T, String> {
        self.run(work, msg::storage_io_write_pending(), false).await
    }

    async fn run<T: Send + 'static>(
        &self,
        work: impl FnOnce() -> Result<T, String> + Send + 'static,
        after_deadline: &str,
        wait_for_seat: bool,
    ) -> Result<T, String> {
        let deadline = crate::deadline::earlier(
            self.within
                .map(|within| tokio::time::Instant::now() + within),
        );
        let busy = || msg::storage_io_busy().to_owned();
        if crate::deadline::expired(deadline) {
            return Err(busy());
        }
        let seats = self.seats.clone();
        let seat = if wait_for_seat {
            crate::deadline::wait(deadline, seats.acquire_owned())
                .await
                .map_err(|_| busy())?
                .map_err(|_| busy())?
        } else {
            seats.try_acquire_owned().map_err(|_| busy())?
        };
        let (reply, receive) = oneshot::channel();
        let runtime = tokio::runtime::Handle::try_current().ok();
        self.workers()?
            .send(Box::new(move || {
                let _seat = seat;
                let _runtime = runtime.as_ref().map(|runtime| runtime.enter());
                if !reply.is_closed() && !crate::deadline::expired(deadline) {
                    let outcome = work();
                    let _ = reply.send(outcome);
                }
            }))
            .map_err(|_| busy())?;
        crate::deadline::wait(deadline, receive)
            .await
            .map_err(|_| after_deadline.to_owned())?
            .map_err(|_| after_deadline.to_owned())?
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn longest_mount_prefix_and_mount_entry_are_classified() {
        let mounts = parse_mounts(
            r"22 1 8:1 / / rw - ext4 /dev/sda1 rw
24 22 0:34 / /nfs/team\040data rw - nfs4 server:/team-data rw
25 24 8:2 / /nfs/team\040data/local rw - ext4 /dev/sda2 rw",
        );
        let io = StorageIo {
            mounts,
            ..StorageIo::default()
        };
        assert!(io.network_path(Path::new("/nfs/team data/a/../b")));
        assert!(io.network_path(Path::new("/nfs")));
        assert!(!io.network_path(Path::new("/nfs/team data/local/hello")));
        assert!(!io.network_path(Path::new("/nfs/team database")));
        assert!(!io.network_path(Path::new("/tmp/file")));
    }
}
