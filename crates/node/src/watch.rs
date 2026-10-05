//! **A directory tree pushed, not asked** (round NET-4): the machine
//! that holds the directories watches the ones a face has open and says
//! when one changed; the face asks `Ls` for those alone.
//!
//! # Why a watch, and what it replaces
//!
//! A tree asked every level it remembers open each time it mounted, and
//! every level touched after khor's own changes, and still missed a
//! change made over there by anyone else (an agent writing files is the
//! common one) until somebody walked away and back. A watcher answers
//! the question a tree actually has — *did anything here change?* — at
//! the moment it changes. The same shape as an editor's file watcher
//! (VS Code's remote server watches the folders open in its explorer and
//! pushes changes to the client) and as LSP's `didChangeWatchedFiles`.
//!
//! # The two halves
//!
//! - **Over there** ([`serve`]): one `Request::Watch` on its own stream
//!   names up to [`MAX_WATCH`] directories. Each is watched alone, not
//!   recursively (`notify`: FSEvents on macOS, inotify on Linux), the
//!   stream says which it took (`Response::Watching`), and then one
//!   `Response::Changed` per burst of changes (gathered for
//!   [`GATHER`]) until the asker lets the stream go.
//! - **Here** ([`Watches`], in the resident): per machine, the set of
//!   directories faces have asked about lately, each on a stream; a
//!   counter that moves with every change; and the long-poll a face holds
//!   (`ipc::Op::WatchOn`): *which of these changed since `since`?* A
//!   directory not wanted for [`KEEP`] is let go. A directory whose
//!   stream started after `since` counts as changed — nobody was watching
//!   it then — and so does every one when the machine cannot watch, which
//!   the answer says (`unavailable`), so the face goes back to asking.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::proto::{self, Request, Response};

/// The most directories one watch takes: the levels a person has open
/// in one tree, not a disk.
pub const MAX_WATCH: usize = 64;

/// How long changes are gathered into one `Changed`: a build writing a
/// hundred files is one answer, not a hundred.
pub const GATHER: Duration = Duration::from_millis(150);

/// How long a directory stays watched after a face last asked about it
/// — long enough that walking back to a tree a minute later finds it
/// still current.
pub const KEEP: Duration = Duration::from_secs(60);

/// A frame on a watch stream: length, then the response.
async fn write_frame(send: &mut iroh::endpoint::SendStream, resp: &Response) -> Result<(), String> {
    let body = proto::encode(resp)?;
    let mut frame = Vec::with_capacity(4 + body.len());
    frame.extend_from_slice(&(body.len() as u32).to_be_bytes());
    frame.extend_from_slice(&body);
    send.write_all(&frame).await.map_err(|e| e.to_string())
}

async fn read_frame(recv: &mut iroh::endpoint::RecvStream) -> Result<Response, String> {
    let mut len = [0u8; 4];
    recv.read_exact(&mut len).await.map_err(|e| e.to_string())?;
    let len = u32::from_be_bytes(len) as usize;
    if len > proto::MAX_FRAME {
        return Err(khor_catalog::msg::frame_too_big(len));
    }
    let mut body = vec![0u8; len];
    recv.read_exact(&mut body).await.map_err(|e| e.to_string())?;
    proto::answer(&body)
}

/// **The far half**: watches `dirs` and writes what changes to `send`
/// until the asker lets the stream go. The asker is already admitted
/// (`link::Node::handle`).
pub(crate) async fn serve(dirs: Vec<String>, mut send: iroh::endpoint::SendStream) {
    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    {
        use notify::Watcher;
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<std::path::PathBuf>();
        let watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            if let Ok(event) = res {
                for path in event.paths {
                    let _ = tx.send(path);
                }
            }
        });
        let Ok(mut watcher) = watcher else {
            let _ = write_frame(
                &mut send,
                &Response::Refused {
                    why: khor_catalog::msg::watch_unavailable().into(),
                },
            )
            .await;
            let _ = send.finish();
            return;
        };
        let mut taken: Vec<String> = Vec::new();
        for dir in dirs.into_iter().take(MAX_WATCH) {
            let path = std::path::Path::new(&dir);
            if path.is_absolute()
                && path.is_dir()
                && watcher.watch(path, notify::RecursiveMode::NonRecursive).is_ok()
            {
                taken.push(dir);
            }
        }
        if write_frame(
            &mut send,
            &Response::Watching {
                dirs: taken.clone(),
            },
        )
        .await
        .is_err()
        {
            return;
        }
        // Each watched directory by the path the watcher reports it under
        // as well as the one it was asked as: FSEvents speaks canonical
        // paths (`/private/var/…` for a `/var/…` that was asked), and the
        // answer must name the directory the way the asker does.
        let mut known: HashMap<std::path::PathBuf, String> = HashMap::new();
        for d in &taken {
            known.insert(std::path::PathBuf::from(d), d.clone());
            if let Ok(canon) = std::fs::canonicalize(d) {
                known.insert(canon, d.clone());
            }
        }
        // A path names a directory that changed: itself when it is one of
        // the watched (renamed, removed), else its parent (a name inside
        // it came, went or was written).
        let dir_of = |p: &std::path::Path| -> Option<String> {
            known
                .get(p)
                .or_else(|| known.get(p.parent()?))
                .cloned()
        };
        loop {
            tokio::select! {
                first = rx.recv() => {
                    let Some(first) = first else { break };
                    let mut changed: Vec<String> = Vec::new();
                    let mut take = |p: std::path::PathBuf| {
                        if let Some(d) = dir_of(&p)
                            && !changed.contains(&d)
                        {
                            changed.push(d);
                        }
                    };
                    take(first);
                    let until = tokio::time::Instant::now() + GATHER;
                    while let Ok(Some(p)) = tokio::time::timeout_at(until, rx.recv()).await {
                        take(p);
                    }
                    if changed.is_empty() {
                        continue;
                    }
                    if write_frame(&mut send, &Response::Changed { dirs: changed }).await.is_err() {
                        break;
                    }
                }
                _ = send.stopped() => break,
            }
        }
        drop(watcher);
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = dirs;
        let _ = write_frame(
            &mut send,
            &Response::Refused {
                why: khor_catalog::msg::watch_unavailable().into(),
            },
        )
        .await;
        let _ = send.finish();
    }
}

/// What a face is told (`Watches::wait`): the counter now, the
/// directories that changed since the one it gave, and whether that
/// machine cannot be watched (the face then asks, as before).
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DirChanges {
    pub seq: u64,
    pub dirs: Vec<String>,
    pub unavailable: bool,
}

/// One directory a face asked about.
struct Dir {
    /// The counter when its stream said it took it; `None` while the
    /// stream is starting, or after it ended.
    from: Option<u64>,
    wanted: Instant,
    /// Which stream carries it.
    stream: u64,
}

#[derive(Default)]
struct Machine {
    seq: u64,
    /// Recent changes, oldest first, as (counter, directory).
    log: VecDeque<(u64, String)>,
    /// The counter of the newest change the log has let go of: a face
    /// further behind than this cannot be told what it missed.
    log_floor: u64,
    dirs: HashMap<String, Dir>,
    /// Streams, by number, and the task reading each.
    streams: HashMap<u64, tokio::task::JoinHandle<()>>,
    next_stream: u64,
    unavailable: bool,
}

/// How many changes a machine's log keeps: a face further behind than
/// this is told every directory changed, and asks them all once.
const LOG_MAX: usize = 1024;

/// **The near half**, in the resident: per machine, what faces asked to
/// have watched and what changed (module head).
#[derive(Default)]
pub struct Watches {
    machines: Mutex<HashMap<String, Machine>>,
    arrived: tokio::sync::Notify,
}

impl Watches {
    /// Which of `dirs` on machine `id` changed since `since`, waiting up
    /// to `longest` for one to. `open` starts a stream for directories
    /// not yet watched: it is handed their list and must run the stream,
    /// calling back through [`Watches::took`], [`Watches::changed`] and
    /// [`Watches::ended`].
    pub(crate) async fn wait(
        self: &Arc<Self>,
        id: &str,
        dirs: &[String],
        since: u64,
        longest: Duration,
        open: impl FnOnce(u64, Vec<String>) -> tokio::task::JoinHandle<()>,
    ) -> DirChanges {
        {
            let mut all = self.machines.lock().unwrap_or_else(|p| p.into_inner());
            let m = all.entry(id.to_owned()).or_default();
            if m.seq == 0 {
                m.seq = 1;
            }
            let now = Instant::now();
            // Let go of what nobody asked about for a while: a stream
            // whose every directory is stale ends.
            m.dirs.retain(|d, dir| dirs.contains(d) || now.duration_since(dir.wanted) < KEEP);
            let live: HashSet<u64> = m.dirs.values().map(|d| d.stream).collect();
            m.streams.retain(|n, task| {
                let keep = live.contains(n);
                if !keep {
                    task.abort();
                }
                keep
            });
            let mut fresh = Vec::new();
            for d in dirs.iter().take(MAX_WATCH) {
                match m.dirs.get_mut(d) {
                    Some(dir) => dir.wanted = now,
                    None => fresh.push(d.clone()),
                }
            }
            if !fresh.is_empty() && !m.unavailable {
                let n = m.next_stream;
                m.next_stream += 1;
                for d in &fresh {
                    m.dirs.insert(
                        d.clone(),
                        Dir {
                            from: None,
                            wanted: now,
                            stream: n,
                        },
                    );
                }
                m.streams.insert(n, open(n, fresh));
            }
        }
        let deadline = tokio::time::Instant::now() + longest;
        loop {
            let woke = self.arrived.notified();
            tokio::pin!(woke);
            woke.as_mut().enable();
            {
                let all = self.machines.lock().unwrap_or_else(|p| p.into_inner());
                let m = &all[id];
                if m.unavailable {
                    return DirChanges {
                        seq: m.seq,
                        dirs: dirs.to_vec(),
                        unavailable: true,
                    };
                }
                let lost = since < m.log_floor;
                // Not watched yet (its stream is starting): unknown, and
                // unknown is changed — the face asks. Said only once the
                // wait is over, so a stream that takes in time answers in
                // the ordinary way.
                let mut starting: Vec<String> = Vec::new();
                let mut changed: Vec<String> = Vec::new();
                for d in dirs {
                    match m.dirs.get(d).and_then(|dir| dir.from) {
                        None => starting.push(d.clone()),
                        Some(from) if from > since || lost => changed.push(d.clone()),
                        Some(_) => {
                            if m.log.iter().any(|(s, c)| *s > since && c == d) {
                                changed.push(d.clone());
                            }
                        }
                    }
                }
                let over = tokio::time::Instant::now() >= deadline;
                if (!changed.is_empty() && starting.is_empty()) || over {
                    changed.extend(starting);
                    return DirChanges {
                        seq: m.seq,
                        dirs: changed,
                        unavailable: false,
                    };
                }
            }
            let _ = tokio::time::timeout_at(deadline, woke).await;
        }
    }

    /// Stream `n` of machine `id` took these directories (and not the
    /// others it was asked for, which are dropped: a directory that is
    /// gone, or not a directory).
    fn took(&self, id: &str, n: u64, taken: &[String]) {
        let mut all = self.machines.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(m) = all.get_mut(id) {
            m.seq += 1;
            let at = m.seq;
            m.dirs.retain(|d, dir| dir.stream != n || taken.contains(d));
            for dir in m.dirs.values_mut().filter(|dir| dir.stream == n) {
                dir.from = Some(at);
            }
        }
        self.arrived.notify_waiters();
    }

    fn changed(&self, id: &str, dirs: Vec<String>) {
        let mut all = self.machines.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(m) = all.get_mut(id) {
            m.seq += 1;
            let at = m.seq;
            for d in dirs {
                m.log.push_back((at, d));
            }
            while m.log.len() > LOG_MAX {
                if let Some((s, _)) = m.log.pop_front() {
                    m.log_floor = s;
                }
            }
        }
        self.arrived.notify_waiters();
    }

    /// Stream `n` ended: its directories are not watched any more, and
    /// are started again by the next face that asks. `refused` when that
    /// machine said it cannot watch at all.
    fn ended(&self, id: &str, n: u64, refused: bool) {
        let mut all = self.machines.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(m) = all.get_mut(id) {
            m.dirs.retain(|_, dir| dir.stream != n);
            m.streams.remove(&n);
            if refused {
                m.unavailable = true;
            }
        }
        self.arrived.notify_waiters();
    }
}

/// **Runs one watch stream** for the near half: asks `dirs` of the far
/// machine on a stream of the held wire and reports into `watches`
/// until the stream ends.
pub(crate) async fn run(
    watches: Arc<Watches>,
    id: String,
    n: u64,
    conn: iroh::endpoint::Connection,
    dirs: Vec<String>,
) {
    let refused = async {
        let (mut send, mut recv) = conn.open_bi().await.map_err(|_| false)?;
        let ask = proto::encode(&Request::Watch { dirs }).map_err(|_| false)?;
        send.write_all(&ask).await.map_err(|_| false)?;
        send.finish().map_err(|_| false)?;
        match read_frame(&mut recv).await.map_err(|_| false)? {
            Response::Watching { dirs } => watches.took(&id, n, &dirs),
            // An older khor refuses the frame (it does not know the
            // question), and one that cannot watch says so: either way
            // this machine's tree is asked, as before.
            Response::Refused { .. } => return Err(true),
            _ => return Err(false),
        }
        loop {
            match read_frame(&mut recv).await.map_err(|_| false)? {
                Response::Changed { dirs } => watches.changed(&id, dirs),
                _ => return Err(false),
            }
        }
        #[allow(unreachable_code)]
        Ok::<(), bool>(())
    }
    .await
    .err()
    .unwrap_or(false);
    watches.ended(&id, n, refused);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **The near half's arithmetic**, without a wire: a directory
    /// watched since before `since` and quiet is not changed; one that
    /// changed is; one whose stream started after `since` is; and a
    /// machine that refused says so, with every directory.
    #[tokio::test]
    async fn changed_since_means_what_a_face_holds_is_stale() {
        let w = Arc::new(Watches::default());
        let dirs = vec!["/a".to_owned(), "/b".to_owned()];
        let mut started = 0;
        let open = |_: u64, _: Vec<String>| {
            started += 1;
            tokio::spawn(async {})
        };
        // The stream is starting: at the deadline, not knowing is
        // changed — the face asks.
        let first = w.wait("m", &dirs, 0, Duration::from_millis(50), open).await;
        assert!(first.dirs.len() == 2 && !first.unavailable);
        assert_eq!(started, 1, "one stream for both");
        w.took("m", 0, &dirs);
        // Everything is news to a face that holds nothing.
        let all = w.wait("m", &dirs, 0, Duration::from_millis(50), |_, _| unreachable!()).await;
        assert_eq!(all.dirs.len(), 2);
        let since = all.seq;
        // Quiet: nothing changed since.
        let quiet = w.wait("m", &dirs, since, Duration::from_millis(50), |_, _| unreachable!()).await;
        assert!(quiet.dirs.is_empty());
        // One changed, and only it is named.
        let waiter = {
            let w = w.clone();
            let dirs = dirs.clone();
            tokio::spawn(async move { w.wait("m", &dirs, since, Duration::from_secs(5), |_, _| unreachable!()).await })
        };
        tokio::time::sleep(Duration::from_millis(20)).await;
        w.changed("m", vec!["/b".to_owned()]);
        let one = waiter.await.unwrap();
        assert_eq!(one.dirs, vec!["/b".to_owned()]);
        // The stream ended: the next ask starts another, and until it
        // takes, nothing is claimed quiet.
        w.ended("m", 0, false);
        let again = w
            .wait("m", &dirs, one.seq, Duration::from_millis(50), |n, d| {
                assert_eq!((n, d.len()), (1, 2));
                tokio::spawn(async {})
            })
            .await;
        assert_eq!(again.dirs.len(), 2, "watched again: not known, so changed");
        w.took("m", 1, &dirs);
        let after = w.wait("m", &dirs, one.seq, Duration::from_millis(50), |_, _| unreachable!()).await;
        assert_eq!(after.dirs.len(), 2, "watched again only since after `since`: both may have changed");
        // A machine that cannot watch.
        w.ended("m", 1, true);
        let no = w.wait("m", &dirs, after.seq, Duration::from_millis(50), |_, _| unreachable!()).await;
        assert!(no.unavailable && no.dirs.len() == 2);
    }
}
