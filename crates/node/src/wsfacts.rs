//! **A workspace row's facts, kept by the machine that holds it** (round
//! NET-8, for UX-15's workspace row: 「3 个改动」, the branch, and when
//! it last changed).
//!
//! The face used to ask each row's machine for its git status and its
//! newest mtime every time the list came up — a status walk and a tree
//! walk per row per look, over the network. Here the machine holding a
//! workspace keeps the answer and redoes it only when something in the
//! tree changed: a file watcher marks the workspace stale, and the next
//! recompute is at most one per [`EVERY`] per workspace, gathered over
//! [`GATHER`]. Nothing here polls the disk. Other machines learn the
//! facts as one reading of a round (`Request::WorkspaceFacts`), sent only
//! when they changed.
//!
//! **What is watched.** macOS and Windows watch a tree natively and
//! cheaply, so one recursive watch per workspace, with [`SKIP`]'s
//! directories filtered out of what it says. Linux's inotify is one
//! watch per directory, so the tree is walked once, [`SKIP`]'s
//! directories left out, at most [`MOST_DIRS`] watched; a directory made
//! later is watched when its parent says so. A tree past the cap is
//! still counted right — the status walk sees everything — only its
//! deepest changes may not wake the count until something watched does.
//!
//! [`SKIP`]: crate::workspace::SKIP

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use khor_core::WorkspaceFact;

/// How long changes are gathered before a recompute: a save that writes
/// three files, or a checkout, is one recompute.
const GATHER: Duration = Duration::from_millis(800);

/// At most one recompute per workspace this often: a build writing for a
/// minute is a dozen status walks, not thousands. The pump's own pace
/// with a face on screen (`link::SYNC_EVERY`).
const EVERY: Duration = Duration::from_secs(5);

/// The most directories one Linux workspace watches.
#[cfg_attr(not(any(target_os = "macos", target_os = "windows", target_os = "linux")), allow(dead_code))]
const MOST_DIRS: usize = 4096;

/// What a watcher says about one workspace. (A phone has no watcher and
/// hosts no workspace: only `Again` is ever said there.)
#[cfg_attr(not(any(target_os = "macos", target_os = "windows", target_os = "linux")), allow(dead_code))]
enum Said {
    /// Something in the tree changed at `at_ms`; `dated` when it is a
    /// change to the work itself, not git's own bookkeeping under `.git`.
    Changed { ws: String, at_ms: i64, dated: bool, new_dir: Option<PathBuf> },
    /// Look again: a recompute that was due too soon.
    Again(String),
}

struct Entry {
    fact: WorkspaceFact,
    last: Option<Instant>,
    /// Kept for as long as the workspace is: dropping it stops the watch.
    /// Only Linux reads it again (a new directory is one more watch).
    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    watcher: Option<notify::RecommendedWatcher>,
    #[cfg(target_os = "linux")]
    dirs: usize,
}

/// The resident's keeper of this machine's workspace facts.
pub(crate) struct WsFacts {
    entries: Mutex<HashMap<String, Entry>>,
    said: tokio::sync::mpsc::UnboundedSender<Said>,
    /// Where this machine's facts are written for its own faces
    /// ([`file_of`]): the home, and this machine's id.
    root: PathBuf,
    me: String,
}

/// The facts file for `device` under `root`: this machine's written by
/// its resident, every other one's by the round that brought them.
pub(crate) fn file_of(root: &Path, device: &str) -> PathBuf {
    root.join(".khor").join("wsfacts").join(format!("{device}.json"))
}

/// What a facts file holds.
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct Kept {
    pub(crate) at_ms: i64,
    pub(crate) facts: Vec<WorkspaceFact>,
}

pub(crate) fn read(root: &Path, device: &str) -> Option<Kept> {
    serde_json::from_slice(&std::fs::read(file_of(root, device)).ok()?).ok()
}

pub(crate) fn write(root: &Path, device: &str, facts: &[WorkspaceFact]) {
    let path = file_of(root, device);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let kept = Kept {
        at_ms: crate::live::now_ms(),
        facts: facts.to_vec(),
    };
    if let Ok(bytes) = serde_json::to_vec(&kept) {
        let _ = crate::live::write_whole(&path, &bytes);
    }
}

fn now_ms() -> i64 {
    crate::live::now_ms()
}

/// One workspace's facts, read now. **Blocking**: a status walk.
///
/// `was` is what was known of it before — this resident's last reading,
/// or the facts file a resident before it wrote: an unchanged git
/// segment keeps its date; a changed one is dated now; one never seen is
/// dated by git's own files ([`crate::git::last_moved`]).
fn compute(path: &str, changed_ms: i64, was: Option<&WorkspaceFact>) -> WorkspaceFact {
    let dir = Path::new(path);
    let checkout = crate::git::is_checkout(dir);
    let read = checkout.then(|| crate::git::segment(dir).ok()).flatten();
    let changed = read.as_ref().map(|s| s.changed);
    let changed_capped = read.as_ref().is_some_and(|s| s.more);
    let git = read.map(|s| {
        let fresh = khor_core::GitSegment {
            branch: s.branch,
            head: s.head,
            subject: s.subject,
            ahead: s.ahead,
            behind: s.behind,
            stashes: s.stashes,
            conflicted: s.conflicted,
            staged: s.staged,
            unstaged: s.unstaged,
            untracked: s.untracked,
            capped: s.more,
            since_ms: 0,
        };
        let since_ms = match was.and_then(|w| w.git.as_ref()) {
            Some(w) if w.same(&fresh) => w.since_ms,
            Some(_) => now_ms(),
            None => crate::git::last_moved(dir),
        };
        khor_core::GitSegment { since_ms, ..fresh }
    });
    WorkspaceFact {
        path: path.to_owned(),
        branch: git.as_ref().and_then(|g| g.branch.clone()),
        git,
        changed,
        changed_capped,
        changed_ms,
        // A plain folder's row says its newest file (UX-48); a checkout's
        // says HEAD's subject instead.
        newest: (!checkout)
            .then(|| crate::workspace::newest_file(dir))
            .flatten()
            .map(|(path, at_ms)| khor_core::NewestFile { path, at_ms }),
    }
}

/// Whether `path` is inside one of the skipped directories under `ws`.
#[cfg_attr(not(any(target_os = "macos", target_os = "windows", target_os = "linux")), allow(dead_code))]
fn skipped(ws: &Path, path: &Path) -> Option<&'static str> {
    let rel = path.strip_prefix(ws).ok()?;
    rel.components().find_map(|c| {
        let name = c.as_os_str();
        crate::workspace::SKIP.iter().find(|s| std::ffi::OsStr::new(s) == name).copied()
    })
}

impl WsFacts {
    /// Starts the keeper: the task that gathers what the watchers say and
    /// recomputes. Its facts are written to this machine's facts file.
    pub(crate) fn start(root: &Path, me: &str) -> Arc<WsFacts> {
        let (said, mut heard) = tokio::sync::mpsc::unbounded_channel::<Said>();
        let keeper = Arc::new(WsFacts {
            entries: Mutex::new(HashMap::new()),
            said,
            root: root.to_path_buf(),
            me: me.to_owned(),
        });
        let k = Arc::downgrade(&keeper);
        tokio::spawn(async move {
            while let Some(first) = heard.recv().await {
                tokio::time::sleep(GATHER).await;
                let mut batch = vec![first];
                while let Ok(more) = heard.try_recv() {
                    batch.push(more);
                }
                let Some(keeper) = k.upgrade() else { return };
                let mut due: HashSet<String> = HashSet::new();
                for said in batch {
                    match said {
                        Said::Changed { ws, at_ms, dated, new_dir } => {
                            keeper.heard(&ws, at_ms, dated, new_dir);
                            due.insert(ws);
                        }
                        Said::Again(ws) => {
                            due.insert(ws);
                        }
                    }
                }
                for ws in due {
                    keeper.clone().recompute(ws);
                }
            }
        });
        keeper
    }

    /// Makes the kept set this machine's workspaces now: new ones watched
    /// and read, gone ones let go. Cheap when nothing changed.
    pub(crate) fn reconcile(self: &Arc<Self>, paths: &[String]) {
        let (new, gone): (Vec<String>, bool) = {
            let mut entries = self.entries.lock().unwrap_or_else(|p| p.into_inner());
            let before = entries.len();
            entries.retain(|p, _| paths.contains(p));
            let gone = entries.len() != before;
            (paths.iter().filter(|p| !entries.contains_key(*p)).cloned().collect(), gone)
        };
        if gone {
            self.write_out();
        }
        for path in new {
            let this = self.clone();
            tokio::task::spawn_blocking(move || this.add(path));
        }
    }

    /// One new workspace: dated, watched, and read. **Blocking.**
    fn add(self: Arc<Self>, path: String) {
        let dated = crate::workspace::touched(Path::new(&path));
        #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
        let (watcher, _dirs) = self.watch(&path);
        let was = read(&self.root, &self.me).and_then(|k| k.facts.into_iter().find(|f| f.path == path));
        let fact = compute(&path, dated, was.as_ref());
        {
            let mut entries = self.entries.lock().unwrap_or_else(|p| p.into_inner());
            entries.insert(
                path,
                Entry {
                    fact,
                    last: Some(Instant::now()),
                    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
                    watcher,
                    #[cfg(target_os = "linux")]
                    dirs: _dirs,
                },
            );
        }
        self.write_out();
    }

    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    fn watch(&self, path: &str) -> (Option<notify::RecommendedWatcher>, usize) {
        use notify::Watcher;
        let ws = PathBuf::from(path);
        // FSEvents speaks canonical paths (`/private/var/…`).
        let canon = std::fs::canonicalize(&ws).unwrap_or_else(|_| ws.clone());
        let (said, name, root) = (self.said.clone(), path.to_owned(), canon.clone());
        let watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            let Ok(event) = res else { return };
            if matches!(event.kind, notify::EventKind::Access(_)) {
                return;
            }
            for p in &event.paths {
                let skip = skipped(&root, p);
                // Inside a build or dependency directory: not the work.
                if skip.is_some_and(|s| s != ".git") {
                    continue;
                }
                let new_dir = (cfg!(target_os = "linux")
                    && matches!(event.kind, notify::EventKind::Create(_))
                    && skip.is_none()
                    && p.is_dir())
                .then(|| p.clone());
                let _ = said.send(Said::Changed {
                    ws: name.clone(),
                    at_ms: now_ms(),
                    dated: skip.is_none(),
                    new_dir,
                });
            }
        });
        let Ok(mut watcher) = watcher else { return (None, 0) };
        let mut dirs = 0usize;
        if cfg!(target_os = "linux") {
            // One watch per directory, the tree walked once.
            let mut stack = vec![canon.clone()];
            while let Some(d) = stack.pop() {
                if dirs >= MOST_DIRS {
                    break;
                }
                if watcher.watch(&d, notify::RecursiveMode::NonRecursive).is_ok() {
                    dirs += 1;
                }
                let Ok(rd) = std::fs::read_dir(&d) else { continue };
                for e in rd.flatten() {
                    if !e.file_type().is_ok_and(|t| t.is_dir()) {
                        continue;
                    }
                    let name = e.file_name();
                    // `.git` itself, not inside it: HEAD and the index
                    // move with every commit and checkout.
                    if name == ".git" {
                        if watcher.watch(&e.path(), notify::RecursiveMode::NonRecursive).is_ok() {
                            dirs += 1;
                        }
                        continue;
                    }
                    if crate::workspace::SKIP.iter().any(|s| std::ffi::OsStr::new(s) == name) {
                        continue;
                    }
                    stack.push(e.path());
                }
            }
        } else if watcher.watch(&canon, notify::RecursiveMode::Recursive).is_ok() {
            dirs = 1;
        }
        (Some(watcher), dirs)
    }

    /// A watcher's word: the date moves (for the work, not for `.git`),
    /// and on Linux a new directory is watched too.
    fn heard(&self, ws: &str, at_ms: i64, dated: bool, _new_dir: Option<PathBuf>) {
        let mut entries = self.entries.lock().unwrap_or_else(|p| p.into_inner());
        let Some(entry) = entries.get_mut(ws) else { return };
        if dated {
            entry.fact.changed_ms = entry.fact.changed_ms.max(at_ms);
        }
        #[cfg(target_os = "linux")]
        if let (Some(dir), Some(w)) = (_new_dir, entry.watcher.as_mut())
            && entry.dirs < MOST_DIRS
        {
            use notify::Watcher;
            if w.watch(&dir, notify::RecursiveMode::NonRecursive).is_ok() {
                entry.dirs += 1;
            }
        }
    }

    /// Reads one workspace again, unless it was read less than [`EVERY`]
    /// ago — then once that much has passed.
    fn recompute(self: Arc<Self>, ws: String) {
        let (since, dated, was) = {
            let entries = self.entries.lock().unwrap_or_else(|p| p.into_inner());
            let Some(e) = entries.get(&ws) else { return };
            (e.last.map(|l| l.elapsed()), e.fact.changed_ms, e.fact.clone())
        };
        if let Some(since) = since
            && since < EVERY
        {
            let said = self.said.clone();
            tokio::spawn(async move {
                tokio::time::sleep(EVERY - since).await;
                let _ = said.send(Said::Again(ws));
            });
            return;
        }
        if let Some(e) = self.entries.lock().unwrap_or_else(|p| p.into_inner()).get_mut(&ws) {
            e.last = Some(Instant::now());
        }
        tokio::task::spawn_blocking(move || {
            let fresh = compute(&ws, dated, Some(&was));
            let moved = {
                let mut entries = self.entries.lock().unwrap_or_else(|p| p.into_inner());
                match entries.get_mut(&ws) {
                    Some(e) => {
                        // The date may have moved while the walk ran.
                        let fresh = WorkspaceFact {
                            changed_ms: fresh.changed_ms.max(e.fact.changed_ms),
                            ..fresh
                        };
                        let moved = e.fact != fresh;
                        e.fact = fresh;
                        moved
                    }
                    None => false,
                }
            };
            if moved {
                self.write_out();
            }
        });
    }

    /// Every kept fact, by path: what a round answers.
    pub(crate) fn facts(&self) -> Vec<WorkspaceFact> {
        let entries = self.entries.lock().unwrap_or_else(|p| p.into_inner());
        let mut out: Vec<WorkspaceFact> = entries.values().map(|e| e.fact.clone()).collect();
        out.sort_by(|a, b| a.path.cmp(&b.path));
        out
    }

    fn write_out(&self) {
        write(&self.root, &self.me, &self.facts());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A plain folder's facts name its newest file; a checkout's do not.
    #[test]
    fn a_plain_folders_facts_name_its_newest_file() {
        let root = crate::testdir::TempTree::fresh("wsfacts-newest");
        let plain = root.join("plain");
        std::fs::create_dir_all(plain.join("sub")).unwrap();
        std::fs::write(plain.join("sub").join("n.txt"), "x").unwrap();
        let f = compute(&plain.display().to_string(), 0, None);
        assert_eq!(f.git, None);
        assert_eq!(f.newest.map(|n| n.path), Some("sub/n.txt".to_owned()));
        let repo = root.join("repo");
        std::fs::create_dir_all(repo.join(".git")).unwrap();
        std::fs::write(repo.join("x.txt"), "x").unwrap();
        assert_eq!(compute(&repo.display().to_string(), 0, None).newest, None, "a checkout says HEAD's subject instead");
    }
}
