//! **A file put where a session's machine can read it** (item 60): the
//! road under dropping a file onto a terminal.
//!
//! Three cases, decided by the asker (`Node::drop_file`):
//!
//! - **The file is already on the session's machine**: nothing moves,
//!   the path is the answer as it is.
//! - **The session's machine is this one**: this machine pulls the file
//!   off the machine that has it, into [`Node::drop_dir`].
//! - **The session's machine is another**: it is asked to do the same
//!   (`proto::Request::DropFile`), and pulls from the machine that has
//!   the file directly — this one only orders and asks how far it got.
//!
//! **One road for the bytes**, the pull by path (`FetchPath`, its slices
//! and their change contract): a drop is a pull whose landing is chosen
//! for the person, whose size is capped, and which never leaves a part
//! file behind to carry on from — a drop that failed is asked again by
//! dropping again, not resumed.
//!
//! The pull runs where the file lands, as a job named by the asker's
//! token, so the asker can ask again with the same words and read the
//! job's progress ([`DropState`]); asking twice never starts two pulls.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// The largest file a drop takes, for now (the owner, 2026-10-06: 200
/// MB). Checked against the size the first slice reports, before the
/// rest is asked for.
pub const CAP: u64 = 200 * 1024 * 1024;

/// How long a finished job's answer is kept for an asker that comes
/// back for it.
const KEPT_FOR: Duration = Duration::from_secs(600);

/// How far a drop got: bytes landed of how many (0 until the far
/// machine has said), and once `done`, the absolute path over there.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DropState {
    pub moved: u64,
    pub total: u64,
    pub dest: String,
    pub done: bool,
}

impl DropState {
    /// A drop that moved nothing: the file is already where it is read.
    pub fn there(path: &str) -> DropState {
        DropState {
            dest: path.to_owned(),
            done: true,
            ..DropState::default()
        }
    }
}

/// One pull into the drop directory, as it goes.
#[derive(Default)]
pub struct Job {
    pub moved: AtomicU64,
    pub total: AtomicU64,
    /// The landed path, or why it did not land; `None` while it runs.
    end: Mutex<Option<(Instant, Result<String, String>)>>,
}

impl Job {
    pub fn finish(&self, outcome: Result<String, String>) {
        if let Ok(mut end) = self.end.lock() {
            *end = Some((Instant::now(), outcome));
        }
    }

    pub fn state(&self) -> Result<DropState, String> {
        let end = self.end.lock().map(|e| e.as_ref().map(|(_, o)| o.clone())).unwrap_or(None);
        let (moved, total) = (self.moved.load(Ordering::Relaxed), self.total.load(Ordering::Relaxed));
        match end {
            None => Ok(DropState {
                moved,
                total,
                ..DropState::default()
            }),
            Some(Ok(dest)) => Ok(DropState {
                moved,
                total,
                dest,
                done: true,
            }),
            Some(Err(why)) => Err(why),
        }
    }

    fn ended_long_ago(&self) -> bool {
        self.end
            .lock()
            .map(|e| e.as_ref().is_some_and(|(at, _)| at.elapsed() > KEPT_FOR))
            .unwrap_or(false)
    }
}

fn jobs() -> &'static Mutex<HashMap<String, Arc<Job>>> {
    static JOBS: std::sync::OnceLock<Mutex<HashMap<String, Arc<Job>>>> = std::sync::OnceLock::new();
    JOBS.get_or_init(Default::default)
}

/// The job this token names, and whether it was made just now (the
/// caller then starts it). Tokens are the asker's random words, so one
/// table serves every node in the process.
pub fn job(token: &str) -> (Arc<Job>, bool) {
    let mut all = jobs().lock().unwrap_or_else(|e| e.into_inner());
    all.retain(|_, j| !j.ended_long_ago());
    if let Some(j) = all.get(token) {
        return (j.clone(), false);
    }
    let j = Arc::new(Job::default());
    all.insert(token.to_owned(), j.clone());
    (j, true)
}

/// Where a dropped file lands in `dir`, and the part file it grows in.
///
/// **A name already taken is suffixed, never replaced**: `a.txt`, then
/// `a-2.txt`, `a-3.txt`. A hyphen and not a space, because the path is
/// pasted into a shell. The part file is made here (`create_new`), so
/// two drops of one name at once each hold their own.
pub fn landing(dir: &Path, name: &str) -> Result<crate::files::Landing, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let p = Path::new(name);
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or(name);
    let ext = p.extension().and_then(|s| s.to_str());
    for n in 1..10_000u32 {
        let candidate = match (n, ext) {
            (1, _) => name.to_owned(),
            (_, Some(ext)) => format!("{stem}-{n}.{ext}"),
            (_, None) => format!("{stem}-{n}"),
        };
        let dest = dir.join(&candidate);
        if dest.symlink_metadata().is_ok() {
            continue;
        }
        let part = dir.join(format!(".khor-pull-{candidate}"));
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&part) {
            Ok(_) => return Ok(crate::files::Landing { part, dest }),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e.to_string()),
        }
    }
    Err(khor_catalog::msg::dest_exists(dir.join(name).display()))
}

/// The file name a path ends in, refused when it ends in none.
pub fn name_of(path: &str) -> Result<String, String> {
    Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .map(str::to_owned)
        .ok_or_else(|| khor_catalog::msg::not_a_file_path(path))
}

/// The directory a drop was asked into (item 60 part 5): absolute, and
/// a directory that is there — never made for the asker.
pub fn named_dir(dir: &str) -> Result<PathBuf, String> {
    let p = Path::new(dir);
    if !p.is_absolute() {
        return Err(khor_catalog::msg::path_not_absolute(dir));
    }
    if !p.is_dir() {
        return Err(khor_catalog::msg::no_such_dir(dir));
    }
    Ok(p.to_path_buf())
}

/// **A file on this machine copied into `dir`** (item 60 part 5): under
/// the drop's naming and cap, through a part file that a failed copy
/// does not leave behind. A file already in `dir` stays as it is and is
/// the answer.
pub fn copy_into(src: &Path, dir: &Path, job: &Job) -> Result<PathBuf, String> {
    let same = |a: &Path, b: &Path| a.canonicalize().ok().zip(b.canonicalize().ok()).is_some_and(|(a, b)| a == b);
    if src.parent().is_some_and(|p| same(p, dir)) {
        return Ok(src.to_path_buf());
    }
    let size = std::fs::metadata(src).map_err(|e| e.to_string())?.len();
    let name = name_of(&src.display().to_string())?;
    if size > CAP {
        return Err(khor_catalog::msg::drop_too_big(CAP >> 20, &name));
    }
    job.total.store(size, Ordering::Relaxed);
    let at = landing(dir, &name)?;
    let copied = std::fs::copy(src, &at.part)
        .map_err(|e| e.to_string())
        .and_then(|_| std::fs::rename(&at.part, &at.dest).map_err(|e| e.to_string()));
    if let Err(why) = copied {
        let _ = std::fs::remove_file(&at.part);
        return Err(why);
    }
    job.moved.store(size, Ordering::Relaxed);
    Ok(at.dest)
}

/// `~/khor-drop` for the machine's own store; `<root>/khor-drop` for any
/// other (a scratch home, a test's), so no test and no scratch node ever
/// writes into the person's own.
pub fn dir_for(root: &Path) -> PathBuf {
    if let Some(home) = std::env::home_dir()
        && (root == home || root.parent() == Some(&home.join(".khor-hosts")))
    {
        return home.join("khor-drop");
    }
    root.join("khor-drop")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_taken_name_is_suffixed_and_the_part_file_is_held() {
        let dir = crate::testdir::TempTree::fresh("drops-landing");
        std::fs::write(dir.join("a.txt"), b"x").unwrap();
        let first = landing(&dir, "a.txt").unwrap();
        assert_eq!(first.dest, dir.join("a-2.txt"));
        assert!(first.part.exists(), "the part file is made at once");
        // Not landed yet, but its part file holds the name.
        let second = landing(&dir, "a.txt").unwrap();
        assert_eq!(second.dest, dir.join("a-3.txt"));
        let plain = landing(&dir, "Makefile").unwrap();
        assert_eq!(plain.dest, dir.join("Makefile"));
        std::fs::write(dir.join("Makefile"), b"x").unwrap();
        assert_eq!(landing(&dir, "Makefile").unwrap().dest, dir.join("Makefile-2"));
    }

    #[test]
    fn the_drop_directory_is_the_homes_only_for_the_homes_own_store() {
        let home = std::env::home_dir().unwrap();
        assert_eq!(dir_for(&home), home.join("khor-drop"));
        assert_eq!(dir_for(&home.join(".khor-hosts").join("box")), home.join("khor-drop"));
        let scratch = std::env::temp_dir().join("khor-scratch-home");
        assert_eq!(dir_for(&scratch), scratch.join("khor-drop"));
    }

    #[test]
    fn a_token_names_one_job() {
        let (a, fresh) = job("drops-test-token-0001");
        assert!(fresh);
        let (b, again) = job("drops-test-token-0001");
        assert!(!again);
        a.moved.store(7, Ordering::Relaxed);
        assert_eq!(b.state().unwrap().moved, 7);
        b.finish(Ok("/x".into()));
        assert_eq!(a.state().unwrap(), DropState { moved: 7, total: 0, dest: "/x".into(), done: true });
    }
}
