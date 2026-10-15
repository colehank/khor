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
//!
//! **A drop can be cancelled** (item 71, the owner 2026-10-07: the
//! pill's × while files go in): the asker names the job by the same
//! token ([`stop`]; from afar `proto::Request::DropStop`), and the pull
//! stops before its next slice, its part file going as on any other
//! failure. A file that already landed stays where it landed.

use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

/// The largest file a drop takes, for now (the owner, 2026-10-06: 200
/// MB). Checked against the size the first slice reports, before the
/// rest is asked for.
pub const CAP: u64 = 200 * 1024 * 1024;

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

pub use crate::road::{cancelled, job, stop, Job};

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
    if src.is_dir() {
        return crate::tree::copy_into(src, dir, job);
    }
    let size = std::fs::metadata(src).map_err(|e| e.to_string())?.len();
    let name = name_of(&src.display().to_string())?;
    if size > CAP {
        return Err(khor_catalog::msg::drop_too_big(CAP >> 20, &name));
    }
    job.total.store(size, Ordering::Relaxed);
    let at = landing(dir, &name)?;
    // One copy, not slices (a clone on APFS): a cancel that came while
    // it ran is honoured before the rename, so nothing lands.
    let copied = std::fs::copy(src, &at.part)
        .map_err(|e| e.to_string())
        .and_then(|_| if job.is_stopped() { Err(cancelled()) } else { Ok(()) })
        .and_then(|()| std::fs::rename(&at.part, &at.dest).map_err(|e| e.to_string()));
    if let Err(why) = copied {
        let _ = std::fs::remove_file(&at.part);
        return Err(why);
    }
    job.moved.store(size, Ordering::Relaxed);
    Ok(at.dest)
}

/// The default receive directory (`crate::road::default_dir`): where a
/// drop lands when nobody set the machine's receive location. A node
/// reads its setting through `Node::receive_dir`.
pub fn dir_for(root: &Path) -> PathBuf {
    crate::road::default_dir(root)
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_directory_drop_preserves_nested_files_empty_directories_and_taken_names() {
        let root=crate::testdir::TempTree::fresh("drop-tree");
        let source=root.join("data"); let dest=root.join("receive");
        std::fs::create_dir_all(source.join("nested/empty")).unwrap();
        std::fs::write(source.join("nested/a.txt"),b"abc").unwrap();
        std::fs::create_dir_all(dest.join("data")).unwrap();
        std::fs::write(dest.join("data/original"),b"keep").unwrap();
        let landed=super::copy_into(&source,&dest,&super::Job::default()).unwrap();
        assert_eq!(landed,dest.join("data-2"));
        assert!(landed.join("nested/empty").is_dir());
        assert_eq!(std::fs::read(landed.join("nested/a.txt")).unwrap(),b"abc");
        assert_eq!(std::fs::read(dest.join("data/original")).unwrap(),b"keep");
    }
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

    /// **A cancel reaches the job its token names, and only that one**
    /// (item 71); a cancel for a token no job has is held as one ended
    /// cancelled, so an ask arriving after it starts no pull.
    #[test]
    fn a_stop_names_one_job_and_a_stop_before_the_ask_starts_nothing() {
        let (running, _) = job("drops-test-token-stop-run");
        let (other, _) = job("drops-test-token-stop-other");
        stop("drops-test-token-stop-run");
        assert!(running.is_stopped());
        assert!(!other.is_stopped(), "another drop goes on");
        stop("drops-test-token-stop-early");
        let (early, fresh) = job("drops-test-token-stop-early");
        assert!(!fresh, "the ask after the cancel finds it and starts nothing");
        assert_eq!(early.state(), Err(cancelled()));
    }

    /// A copy into a directory (放到当前目录) cancelled while it ran
    /// leaves neither the file nor its part file.
    #[test]
    fn a_cancelled_copy_leaves_nothing() {
        let dir = crate::testdir::TempTree::fresh("drops-copy-stop");
        let src = dir.join("a.txt");
        std::fs::write(&src, b"x").unwrap();
        let into = dir.join("into");
        std::fs::create_dir_all(&into).unwrap();
        let j = Job::default();
        j.stopped.store(true, Ordering::Relaxed);
        assert_eq!(copy_into(&src, &into, &j), Err(cancelled()));
        assert_eq!(std::fs::read_dir(&into).unwrap().count(), 0, "no file and no part file");
    }
}
