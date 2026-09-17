//! The files landing's serving half: what one paired machine shows
//! another of its disk. Listing only — the payload path is the
//! transfer kind's, and what 待批 guards there is bytes, not looking
//! (docs/NET.md 入网即全信).
//!
//! # The path contract
//!
//! Empty means the user's home directory — the browse has to start
//! somewhere, and home is where the things a person goes looking for
//! live. Anything else must be absolute: a relative path would be
//! relative to wherever the serve process happened to start, an answer
//! about a directory nobody named.
//!
//! # What a row says
//!
//! `metadata()` follows symlinks on purpose: browsing wants "can I
//! step in", not "how is it wired". A link whose target is gone still
//! gets a row (size 0, not a directory) — dropping it would make a
//! visible name unexplainable. Entries the walk cannot stat at all
//! keep the same shape for the same reason.

use khor_catalog::msg;

use crate::proto::DirEntry;

/// Cap per answer. A directory bigger than this answers its first cap
/// in order plus `truncated: true` — the no-silent-caps rule; the cap
/// itself exists because one answer is one frame.
pub const MOST_ENTRIES: usize = 4096;

/// The listing behind [`crate::proto::Request::Ls`]. Ordered here —
/// dot-named entries sink below everything, each half directories
/// first, each quarter by case-folded name — so every face paints the
/// same order and none re-sorts.
///
/// Dotfiles sink because a home directory leads with dozens of them
/// (`.cache/`, `.config/`…) and the things a person came to browse
/// start a screen down. Sunk, not hidden: no "show hidden" toggle —
/// a default that shows everything beats a setting (docs/UX.md 设置),
/// and hiding would make a real file unpinnable.
pub fn list_dir(path: &str) -> Result<(std::path::PathBuf, Vec<DirEntry>, bool), String> {
    let dir = if path.is_empty() {
        std::env::home_dir().ok_or(msg::NO_HOME_DIR)?
    } else if std::path::Path::new(path).is_absolute() {
        std::path::PathBuf::from(path)
    } else {
        return Err(msg::path_not_absolute(path));
    };
    // **The path handed back is the physical one.** A session records
    // where it ran as `current_dir()`, which has every symlink resolved;
    // a workspace declared through a link (`target/` on this machine
    // points at another disk) would never share a prefix with the
    // sessions inside it, and its column stayed empty forever (found by
    // smoke 批⑥, 2026-09-15). Only the machine holding the directory
    // can resolve its links, and this is where every far read of a
    // directory lands, so this is where the spelling is settled.
    let dir = dir.canonicalize().map_err(|e| msg::cant_list_dir(&dir.display(), e))?;
    let rd = std::fs::read_dir(&dir).map_err(|e| msg::cant_list_dir(&dir.display(), e))?;
    let mut entries: Vec<DirEntry> = Vec::new();
    for e in rd.flatten() {
        let Some(name) = e.file_name().to_str().map(String::from) else {
            // A name that is not UTF-8 cannot ride the frame as itself;
            // a lossy copy would name a file that does not exist.
            continue;
        };
        let meta = std::fs::metadata(e.path());
        let (dir, size, at_ms) = match meta {
            Ok(m) => (
                m.is_dir(),
                if m.is_dir() { 0 } else { m.len() },
                m.modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0),
            ),
            Err(_) => (false, 0, 0),
        };
        entries.push(DirEntry { name, dir, size, at_ms });
    }
    entries.sort_by(|a, b| {
        a.name
            .starts_with('.')
            .cmp(&b.name.starts_with('.'))
            .then_with(|| b.dir.cmp(&a.dir))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    let truncated = entries.len() > MOST_ENTRIES;
    entries.truncate(MOST_ENTRIES);
    Ok((dir, entries, truncated))
}

/// One slice of a file by absolute path, plus the change contract
/// (`proto::Response::PathSlice`): total size and mtime ride out so the
/// puller can notice the file moving under it — no digest guards this
/// path, so "did it change" is the whole of the integrity story.
pub fn read_slice(path: &str, offset: u64) -> Result<(u64, u64, Vec<u8>), String> {
    use std::io::{Read, Seek, SeekFrom};
    if !std::path::Path::new(path).is_absolute() {
        return Err(msg::path_not_absolute(path));
    }
    let meta = std::fs::metadata(path).map_err(|e| msg::cant_list_dir(&path, e))?;
    if meta.is_dir() {
        return Err(msg::not_a_file_path(path));
    }
    let total = meta.len();
    if offset > total {
        return Err(msg::offset_out_of_range(offset, total));
    }
    let at_ms = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let mut f = std::fs::File::open(path).map_err(|e| msg::cant_list_dir(&path, e))?;
    f.seek(SeekFrom::Start(offset)).map_err(|e| e.to_string())?;
    let want = (total - offset).min(crate::proto::SLICE) as usize;
    let mut buf = vec![0u8; want];
    f.read_exact(&mut buf).map_err(msg::cant_read_slice)?;
    Ok((total, at_ms, buf))
}

/// What a file's bytes hash to, hex. **blake3 and not the mtime**, for
/// [`write_whole`]'s benefit: a clock has a granularity (a second, on
/// some filesystems still in use) and two edits inside one tick are
/// invisible to it, while a digest is a fact about the bytes and has no
/// tick at all. It is also the answer to a question mtime cannot
/// answer — 「is this still what I read」 — where mtime only ever
/// answers 「has somebody touched it」, which is a different and
/// weaker claim.
///
/// Already in this tree (`blake3`, workspace dependency), so naming it
/// here costs no package.
pub fn digest(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

/// Writes a whole file by absolute path, **only if it still holds what
/// the writer last read**.
///
/// `was` is [`digest`] of the bytes the writer started from; `""` means
/// 「there should be nothing here」, which is how a new file is made.
/// Anything else on disk is a refusal, not an overwrite:
///
/// | on disk | `was` | what happens |
/// |---|---|---|
/// | nothing | `""` | created |
/// | nothing | a digest | refused — it was deleted under the writer |
/// | matches `was` | that digest | replaced |
/// | anything else | anything | refused — it changed under the writer |
///
/// **The landing is the same dot-name-then-rename `landing` uses**, and
/// for the same reason: a crash leaves a dotfile beside the real one,
/// never a half-written file wearing the real name. The temporary lives
/// in the destination's own directory because `rename` is only atomic
/// inside one filesystem, and `/tmp` is very often another one.
///
/// Answers the new [`digest`] and the new mtime, so a writer can keep
/// editing without reading the file back.
pub fn write_whole(path: &str, bytes: &[u8], was: &str) -> Result<(u64, String), String> {
    use std::io::Write;
    let p = std::path::Path::new(path);
    if !p.is_absolute() {
        return Err(msg::path_not_absolute(path));
    }
    // One frame is the ceiling and the caller is expected to know it —
    // this is the wire's limit worn where the bytes land, so a path
    // that reached here another way cannot get past it either.
    if bytes.len() as u64 > crate::proto::SLICE {
        return Err(msg::too_big_to_write(bytes.len() as u64, crate::proto::SLICE));
    }
    match std::fs::read(p) {
        Ok(now) => {
            if was.is_empty() || digest(&now) != was {
                return Err(msg::file_changed_under_you(path));
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            if !was.is_empty() {
                return Err(msg::file_changed_under_you(path));
            }
        }
        Err(e) => return Err(msg::cant_list_dir(&path, e)),
    }
    let dir = p.parent().ok_or_else(|| msg::not_a_file_path(path))?;
    let name = p.file_name().and_then(|n| n.to_str()).ok_or_else(|| msg::not_a_file_path(path))?;
    let part = dir.join(format!(".khor-save-{name}"));
    // Scoped so the handle is closed before the rename: on Windows a
    // rename over an open file fails, and this module is compiled for
    // five platforms.
    {
        let mut f = std::fs::File::create(&part).map_err(|e| msg::cant_save_file(part.display(), e))?;
        f.write_all(bytes).map_err(|e| msg::cant_save_file(part.display(), e))?;
        // **Before the rename, not after.** The rename is what makes the
        // new name point at these bytes; if the bytes are not on the
        // disk yet, a power cut between the two leaves the real name
        // pointing at nothing.
        f.sync_all().map_err(|e| msg::cant_save_file(part.display(), e))?;
    }
    std::fs::rename(&part, p).map_err(|e| {
        let _ = std::fs::remove_file(&part);
        msg::cant_save_file(p.display(), e)
    })?;
    let at_ms = std::fs::metadata(p)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    Ok((at_ms, digest(bytes)))
}

/// Where a pulled file lands and how it gets there: written beside its
/// destination under a dot-name, renamed only when whole — a crash
/// leaves a dotfile, never a truncated file wearing a real name. An
/// existing destination refuses up front: overwriting what a person
/// already has is the one irreversible act in this module.
pub struct Landing {
    pub part: std::path::PathBuf,
    pub dest: std::path::PathBuf,
}

pub fn landing(remote_path: &str, dir: &std::path::Path) -> Result<Landing, String> {
    let name = std::path::Path::new(remote_path)
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| msg::not_a_file_path(remote_path))?;
    let dest = dir.join(name);
    if dest.exists() {
        return Err(msg::dest_exists(dest.display()));
    }
    Ok(Landing { part: dir.join(format!(".khor-pull-{name}")), dest })
}

/// **Whether this machine has a recycle bin to put things in.**
///
/// A compile-time fact, and it is answered here rather than guessed at
/// the screen: a face that branched on `navigator.platform` would be
/// deciding what khor's own binary can do from the outside, and it
/// would be wrong on exactly the builds where it matters (the same
/// binary runs the desktop app and the web face a phone opens).
///
/// The three desktops have one; iOS and Android do not, and khor ships
/// to all five (docs/KHOR.md).
pub const TRASH_HERE: bool =
    cfg!(any(target_os = "macos", target_os = "windows", target_os = "linux"));

/// Takes a path off this machine's disk: into the recycle bin when
/// `trash` is asked for and this machine has one, straight off it
/// otherwise.
///
/// # The recycle bin is a dependency, and here is what it cost
///
/// Three candidates, each measured as a `Cargo.lock` diff rather than
/// estimated (the repo's rule for a new package):
///
/// 1. **`trash` 5.2.9 (MIT) — +2 packages** (`trash`, and `urlencoding`
///    behind it; both MIT). macOS, Windows and Linux, one API.
/// 2. **`objc2-foundation`'s `NSFileManager` — +0 packages**, because
///    tauri already puts it in the tree. **macOS only**: Windows and
///    Linux would fall back to deleting outright.
/// 3. **Hand-rolled** — the freedesktop `.trashinfo` dance on Linux and
///    `IFileOperation` on Windows. Zero packages and a specification
///    implemented by hand, which is what「尽量用成熟库,不自写」(the
///    user, 2026-08-16) exists to refuse.
///
/// (1), because the criterion is covering the three desktops and two
/// small MIT crates is the cheapest way to. Saving them would mean
/// telling somebody on Windows that 删除 is 不可恢复 when it did not
/// have to be — an irreversible act is the last place to economise.
///
/// # A directory goes whole
///
/// Both kinds are taken, and a directory takes what is under it — the
/// same thing VS Code's 删除 does, and the reason the sentence a person
/// confirms says so out loud (`khor_catalog::gui::file_remove_dir`).
///
/// **A symlink is taken as itself, never followed.** `remove_file`
/// unlinks the link; walking through it would delete somebody else's
/// tree from a name that merely pointed at it.
pub fn remove(path: &str, trash: bool) -> Result<(), String> {
    if !std::path::Path::new(path).is_absolute() {
        return Err(msg::path_not_absolute(path));
    }
    let p = std::path::Path::new(path);
    // `symlink_metadata`, not `metadata`: a link whose target is gone
    // is still a name on this disk and still removable, and a link
    // whose target is a directory must not be recursed into.
    let meta = std::fs::symlink_metadata(p).map_err(|e| msg::cant_remove(path, e))?;
    if trash && TRASH_HERE {
        return trash_it(p);
    }
    if meta.is_dir() {
        std::fs::remove_dir_all(p).map_err(|e| msg::cant_remove(path, e))
    } else {
        std::fs::remove_file(p).map_err(|e| msg::cant_remove(path, e))
    }
}

#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
fn trash_it(p: &std::path::Path) -> Result<(), String> {
    trash::delete(p).map_err(|e| msg::cant_remove(&p.display(), e))
}

/// Unreachable where there is no recycle bin — [`TRASH_HERE`] is false
/// there and [`remove`] never asks. Present so the module compiles on
/// the two platforms that have none.
#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn trash_it(p: &std::path::Path) -> Result<(), String> {
    Err(msg::cant_remove(&p.display(), "no trash on this platform"))
}

/// Makes one directory by absolute path.
///
/// **One, not a chain.** `create_dir_all` would turn a mistyped
/// `src/copmonents/x` into two new directories nobody asked for and
/// answer「done」, and the caller here is a person typing a name into a
/// row of a tree whose parent is on the screen in front of them. So the
/// parent must already be there, and if it is not, the operating
/// system's own sentence about that is the one shown.
///
/// **A name already taken is refused**, and refused before anything is
/// written: `create_dir` fails on an existing directory anyway, but it
/// fails with the same words for「it is a file」and for「the parent is
/// read-only」, and 「那个名字已经有了」is the one of the three the
/// person can act on by typing a different name.
pub fn mkdir(path: &str) -> Result<(), String> {
    let p = std::path::Path::new(path);
    if !p.is_absolute() {
        return Err(msg::path_not_absolute(path));
    }
    // `symlink_metadata`: a dangling link is still a name in use, and
    // `exists()` follows links and would call it free.
    if std::fs::symlink_metadata(p).is_ok() {
        return Err(msg::name_taken(path));
    }
    std::fs::create_dir(p).map_err(|e| msg::cant_mkdir(path, e))
}

/// Makes an empty file by absolute path, refusing a name already taken.
///
/// This is [`write_whole`] with no bytes and an empty expectation —
/// 「there should be nothing here」 — and it is spelled out here rather
/// than left to the caller so that `khor touch` and the tree's 新建文件
/// cannot drift into two different answers about an existing name.
pub fn touch(path: &str) -> Result<(), String> {
    write_whole(path, b"", "").map(|_| ())
}

/// Moves a name on this disk: rename inside a directory, across two of
/// them, both.
///
/// **The target is checked first and never overwritten.**
/// `std::fs::rename` replaces an existing file silently on unix, so a
/// mistyped destination would take another file with it and answer
/// 「done」. The check is a window — somebody could create that name in
/// between — and it is the right size of answer for what this is: a
/// person renaming a row in a tree, not a build step.
pub fn rename_path(from: &str, to: &str) -> Result<(), String> {
    let (a, b) = (std::path::Path::new(from), std::path::Path::new(to));
    if !a.is_absolute() {
        return Err(msg::path_not_absolute(from));
    }
    if !b.is_absolute() {
        return Err(msg::path_not_absolute(to));
    }
    // The same name twice is not a move and not an error worth its own
    // sentence — it is the name already being taken, by itself.
    if std::fs::symlink_metadata(b).is_ok() {
        return Err(msg::name_taken(to));
    }
    std::fs::rename(a, b).map_err(|e| msg::cant_move_name(from, e))
}

/// What this machine opens paths and pages with, if anything.
///
/// **An environment variable wins**, and that is not a test hatch bolted
/// on: a desktop is free to have no `xdg-open`, and the person who knows
/// what their machine opens things with is the person sitting at it.
/// That it also lets the smoke run point the opener at `/usr/bin/true`
/// instead of a window appearing on somebody's screen is the same fact
/// used from the other end.
///
/// Empty means「this machine opens nothing」and is honoured: setting it
/// to nothing is how a person says so, and reading it as「fall back to
/// the platform's」would be khor overruling them.
pub fn launcher() -> Option<String> {
    if let Ok(given) = std::env::var(LAUNCHER_ENV) {
        let given = given.trim().to_owned();
        return if given.is_empty() { None } else { Some(given) };
    }
    // Windows is deliberately not in this list. `explorer` answers a
    // non-zero exit code on success, so「did it open」could only be
    // guessed at — and a guess is the one thing [`launch`] must not
    // answer. A Windows user names their opener in the variable above.
    if cfg!(target_os = "macos") {
        Some("open".to_owned())
    } else if cfg!(target_os = "linux") {
        Some("xdg-open".to_owned())
    } else {
        None
    }
}

/// The variable that names this machine's opener.
pub const LAUNCHER_ENV: &str = "KHOR_LAUNCHER";

/// The whitelist [`launch`] applies, on its own so that **every road
/// asks it before the target leaves this machine** (`Link::launch`):
/// a satellite's opener is reached over ssh with the target quoted
/// into a script, and a quoted `-a` is still a flag to `open` at the
/// far end. One judgment, made once, ahead of the branch.
pub fn openable(target: &str) -> Result<(), String> {
    if target.starts_with('-') {
        return Err(msg::not_openable(target));
    }
    let page = target.starts_with("http://") || target.starts_with("https://");
    if !page && !std::path::Path::new(target).is_absolute() {
        return Err(msg::not_openable(target));
    }
    Ok(())
}

/// Hands one path or page to this machine's opener.
///
/// **What may be handed over is a whitelist**, because what reaches here
/// came out of a model's own words in a conversation: an absolute path
/// on this disk, or an `http(s)` address. `file:`, `mailto:`, a scheme
/// some app registered — each of those is an action rather than a page
/// (`khor_gui_core::web::open_link`, the same judgment one machine
/// closer to the screen).
///
/// **A leading `-` is refused whatever else it spells.** The opener gets
/// exactly one argument and no shell, so nothing here can become a
/// second command — but `open -a Calculator` is one argument that is
/// still a flag, and a name khor cannot vouch for must not be able to
/// become one.
///
/// Answers as soon as the opener starts. Waiting for it would be waiting
/// for a person to close a window.
pub fn launch(target: &str) -> Result<(), String> {
    openable(target)?;
    let opener = launcher().ok_or_else(|| msg::no_launcher(LAUNCHER_ENV))?;
    std::process::Command::new(&opener)
        .arg(target)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| msg::cant_launch(target, e))
}

#[cfg(test)]
mod remove_tests {
    use super::*;

    /// A scratch tree that takes itself away, however the test leaves
    /// (工作区批①'s `Scratch` — an assertion panics past a cleanup
    /// written at the end, a `Drop` does not).
    struct Scratch(std::path::PathBuf);

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    impl std::ops::Deref for Scratch {
        type Target = std::path::Path;

        fn deref(&self) -> &std::path::Path {
            &self.0
        }
    }

    fn tmp(tag: &str) -> Scratch {
        let p = std::env::temp_dir().join(format!("khor-rm-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        Scratch(p)
    }

    /// A file goes, and a path that is not there is refused rather than
    /// reported as a success — 「it is gone either way」 is true and is
    /// not what somebody pressing 删除 asked.
    #[test]
    fn a_file_goes_and_a_path_that_is_not_there_is_refused() {
        let dir = tmp("file");
        let f = dir.join("a.rs");
        std::fs::write(&f, "x").unwrap();

        remove(f.to_str().unwrap(), false).unwrap();
        assert!(!f.exists());

        let err = remove(f.to_str().unwrap(), false).expect_err("nothing there any more");
        assert!(err.contains("a.rs"), "the refusal names the path: {err}");
        // …and a relative path is refused before anything is touched,
        // for the reason `Ls` demands an absolute one.
        assert!(remove("a.rs", false).is_err(), "a relative path names nowhere");
    }

    /// **A directory goes whole**, which is the claim the confirmation
    /// sentence makes out loud. The control is the file beside it: a
    /// `remove_dir` that only took empty directories would fail here
    /// rather than take too much, and both are wrong in ways worth
    /// telling apart.
    #[test]
    fn a_directory_takes_what_is_under_it() {
        let dir = tmp("dir");
        let sub = dir.join("src/deep");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("one.rs"), "x").unwrap();
        let keep = dir.join("kept.rs");
        std::fs::write(&keep, "x").unwrap();

        remove(dir.join("src").to_str().unwrap(), false).unwrap();
        assert!(!dir.join("src").exists(), "the directory and everything under it");
        assert!(keep.exists(), "and nothing beside it");
    }

    /// **A symlink is taken as itself.** Following it would delete
    /// somebody else's tree from a name that merely pointed at it —
    /// the one mistake here that cannot be undone from inside khor.
    ///
    /// **What holds each half is worth writing down, because they are
    /// not the same thing.** The first arm is std's: `remove_dir_all`
    /// refuses to walk through a symlink, so swapping this module's
    /// `symlink_metadata` for `metadata` does *not* break it — tried,
    /// and it stayed green. The second arm is this module's own: a
    /// dangling link is a name that really is on the disk and really
    /// can be removed, and `metadata` cannot even see it, so a
    /// following `stat` refuses to delete something that is plainly
    /// there. That is the arm that goes red if the call is changed.
    #[cfg(unix)]
    #[test]
    fn a_symlink_goes_and_what_it_pointed_at_stays() {
        let dir = tmp("link");
        let real = dir.join("real");
        std::fs::create_dir_all(&real).unwrap();
        std::fs::write(real.join("keep.rs"), "x").unwrap();
        let link = dir.join("link");
        std::os::unix::fs::symlink(&real, &link).unwrap();

        remove(link.to_str().unwrap(), false).unwrap();
        assert!(!link.exists(), "the link is gone");
        assert!(real.join("keep.rs").exists(), "and what it pointed at is not");

        // A link pointing at nothing: still a name, still removable.
        let dangling = dir.join("dangling");
        std::os::unix::fs::symlink(dir.join("never-existed"), &dangling).unwrap();
        remove(dangling.to_str().unwrap(), false).unwrap();
        assert!(
            std::fs::symlink_metadata(&dangling).is_err(),
            "a link with nothing behind it is a name on this disk and comes off it",
        );
    }

    /// **The recycle bin really receives it**, and the landing is what
    /// is asserted.
    ///
    /// 「It left the workspace」 alone would be true of a plain delete
    /// as well, which is the one thing this road must not turn out to
    /// be — the whole promise of 删到废纸篓 is that it can be undone.
    /// So on macOS the file is looked for in `~/.Trash` and taken back
    /// out again; this is also why the name carries the pid, so a real
    /// person's bin is never the thing being read or cleaned.
    ///
    /// Windows and Linux compile the same road and are not checked
    /// here: the shell's recycle bin has no path to look in, and the
    /// freedesktop one is under a data directory this test would have
    /// to re-derive — a second implementation of the thing being
    /// tested. They keep the weaker half (it left), said plainly rather
    /// than dressed up.
    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    #[test]
    fn the_recycle_bin_is_where_a_trashed_file_goes() {
        let dir = tmp("trash");
        let name = format!("khor-trash-probe-{}.txt", std::process::id());
        let f = dir.join(&name);
        std::fs::write(&f, "x").unwrap();

        remove(f.to_str().unwrap(), true).unwrap();
        assert!(!f.exists(), "it left the workspace");
        assert!(TRASH_HERE, "…and this machine was asked for the bin, not the fallback");

        #[cfg(target_os = "macos")]
        {
            let landed = std::env::home_dir().expect("a home directory").join(".Trash").join(&name);
            assert!(landed.is_file(), "it is in the bin at {}", landed.display());
            let _ = std::fs::remove_file(&landed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("khor-files-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    /// Dotfiles sink, then directories lead, then names fold case —
    /// asymmetric fixture on purpose: a file sorting before a
    /// directory, `B` before `a`, or a dot-*directory* floating above a
    /// plain *file* (what a plain byte sort does, and what dirs-first
    /// alone would also do) would each fail here.
    #[test]
    fn dotfiles_sink_directories_lead_and_names_fold_case() {
        let root = tmp("order");
        std::fs::write(root.join("apple"), b"x").unwrap();
        std::fs::write(root.join("Banana"), b"xy").unwrap();
        std::fs::create_dir(root.join("zoo")).unwrap();
        std::fs::create_dir(root.join(".config")).unwrap();
        std::fs::write(root.join(".env"), b"z").unwrap();
        let (_, rows, truncated) = list_dir(root.to_str().unwrap()).unwrap();
        let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["zoo", "apple", "Banana", ".config", ".env"]);
        assert!(rows[0].dir && !rows[1].dir);
        assert!(rows[3].dir && !rows[4].dir);
        assert_eq!(rows[2].size, 2);
        assert!(!truncated);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Relative paths are refused, not resolved: an answer about
    /// "wherever serve started" is an answer about nothing.
    #[test]
    fn a_relative_path_is_refused_by_name() {
        assert!(list_dir("some/where").is_err());
    }

    /// Bigger than the cap says so — the difference between "that is
    /// everything" and "that is the first page" must be on the wire.
    #[test]
    fn a_directory_over_the_cap_admits_truncation() {
        let root = tmp("cap");
        for i in 0..(MOST_ENTRIES + 3) {
            std::fs::write(root.join(format!("f{i:05}")), b"").unwrap();
        }
        let (_, rows, truncated) = list_dir(root.to_str().unwrap()).unwrap();
        assert_eq!(rows.len(), MOST_ENTRIES);
        assert!(truncated);
        let _ = std::fs::remove_dir_all(&root);
    }
}

#[cfg(test)]
mod write_tests {
    use super::*;

    fn tmp(tag: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("khor-write-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn an_empty_expectation_makes_a_file_and_refuses_to_clobber_one() {
        let dir = tmp("new");
        let path = dir.join("fresh.txt");
        let p = path.to_str().unwrap();
        let (_, d) = write_whole(p, b"one\n", "").expect("a new file");
        assert_eq!(std::fs::read(&path).unwrap(), b"one\n");
        assert_eq!(d, digest(b"one\n"));
        // The same call again is now a writer who thinks the path is
        // free while somebody has put a file there.
        let err = write_whole(p, b"two\n", "").expect_err("not free any more");
        assert!(err.contains("fresh.txt"), "the refusal names the path: {err}");
        assert_eq!(std::fs::read(&path).unwrap(), b"one\n", "and nothing was written");
    }

    #[test]
    fn writing_back_what_you_read_replaces_it() {
        let dir = tmp("round");
        let path = dir.join("a.rs");
        std::fs::write(&path, b"fn main() {}\n").unwrap();
        let was = digest(b"fn main() {}\n");
        let (_, now) = write_whole(path.to_str().unwrap(), b"fn main() { go(); }\n", &was).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"fn main() { go(); }\n");
        assert_eq!(now, digest(b"fn main() { go(); }\n"), "the answer is the new digest");
    }

    #[test]
    fn a_file_that_changed_under_the_writer_is_refused_whole() {
        let dir = tmp("race");
        let path = dir.join("a.rs");
        std::fs::write(&path, b"first\n").unwrap();
        let was = digest(b"first\n");
        // Somebody else saves between the read and the write.
        std::fs::write(&path, b"theirs\n").unwrap();
        let err = write_whole(path.to_str().unwrap(), b"mine\n", &was).expect_err("refused");
        assert!(err.contains("a.rs"), "{err}");
        assert_eq!(
            std::fs::read(&path).unwrap(),
            b"theirs\n",
            "the other person's work is still there — a refusal that wrote anyway is the bug",
        );
    }

    #[test]
    fn a_file_deleted_under_the_writer_is_not_quietly_recreated() {
        // The other direction of the same fact, and it is deliberately
        // not「create it then」: the writer was editing something that
        // existed, and somebody removing it is a thing to be told about
        // rather than to undo by accident.
        let dir = tmp("gone");
        let path = dir.join("a.rs");
        let err = write_whole(path.to_str().unwrap(), b"mine\n", &digest(b"first\n"))
            .expect_err("refused");
        assert!(err.contains("a.rs"), "{err}");
        assert!(!path.exists(), "and it stayed gone");
    }

    #[test]
    fn nothing_is_left_beside_the_file_when_it_worked() {
        // The dot-name is a crash artefact, not a resting state.
        let dir = tmp("clean");
        let path = dir.join("a.rs");
        write_whole(path.to_str().unwrap(), b"x\n", "").unwrap();
        let left: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(left, vec!["a.rs".to_owned()], "one file, no dotfile");
    }

    #[test]
    fn more_than_one_frame_is_refused_before_anything_is_opened() {
        let dir = tmp("big");
        let path = dir.join("a.bin");
        let big = vec![b'x'; crate::proto::SLICE as usize + 1];
        let err = write_whole(path.to_str().unwrap(), &big, "").expect_err("over the ceiling");
        assert!(err.contains(&crate::proto::SLICE.to_string()), "{err}");
        assert!(!path.exists(), "and no file was started");
    }
}

/// 新建 / 改名 / 打开 (工作区第五轮). `remove_tests`' `Scratch` for the
/// same reason it exists there: an assertion panics past a cleanup
/// written at the end, a `Drop` does not.
#[cfg(test)]
mod make_tests {
    use super::*;

    struct Scratch(std::path::PathBuf);

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    impl std::ops::Deref for Scratch {
        type Target = std::path::Path;

        fn deref(&self) -> &std::path::Path {
            &self.0
        }
    }

    fn tmp(tag: &str) -> Scratch {
        let p = std::env::temp_dir().join(format!("khor-make-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        Scratch(p)
    }

    /// **One directory, and only one.** The control is the second
    /// assertion: a `create_dir_all` would pass the first line and then
    /// quietly build a chain nobody typed, which is the mistake this
    /// verb refuses rather than the one it makes.
    #[test]
    fn one_directory_is_made_and_never_a_chain() {
        let dir = tmp("mkdir");
        let one = dir.join("src");
        mkdir(one.to_str().unwrap()).unwrap();
        assert!(one.is_dir(), "the directory a person typed");

        let deep = dir.join("a/b/c");
        let err = mkdir(deep.to_str().unwrap()).expect_err("the parent is not there");
        assert!(err.contains("a/b/c"), "the refusal names the path: {err}");
        assert!(!dir.join("a").exists(), "and nothing on the way to it was made either");
    }

    /// A name already taken is refused **by the sentence a person can
    /// act on** — 「换一个名字」 — rather than by the operating system's
    /// three different words for it. Both kinds of name count: a file
    /// occupies the name just as a directory does.
    #[test]
    fn a_name_already_taken_is_refused_whatever_wears_it() {
        let dir = tmp("taken");
        let d = dir.join("src");
        std::fs::create_dir(&d).unwrap();
        let f = dir.join("a.rs");
        std::fs::write(&f, "x").unwrap();

        for taken in [&d, &f] {
            let err = mkdir(taken.to_str().unwrap()).expect_err("the name is taken");
            assert!(
                err.contains(&msg::name_taken(taken.display().to_string())),
                "one sentence for both kinds: {err}",
            );
        }
        assert_eq!(std::fs::read(&f).unwrap(), b"x", "and the file that wore the name is untouched");
    }

    /// `touch` is a write of no bytes, so the promise it inherits is
    /// `write_whole`'s: it makes the name and it refuses one already
    /// there **without emptying it**. That last clause is the whole
    /// reason this is a test — `: > f` in a shell would have passed
    /// every other line here.
    #[test]
    fn touch_makes_an_empty_file_and_never_empties_one() {
        let dir = tmp("touch");
        let f = dir.join("x.md");
        touch(f.to_str().unwrap()).unwrap();
        assert_eq!(std::fs::read(&f).unwrap(), b"", "zero bytes, and the name exists");

        std::fs::write(&f, "words\n").unwrap();
        let err = touch(f.to_str().unwrap()).expect_err("the name is taken");
        assert!(err.contains("x.md"), "the refusal names the path: {err}");
        assert_eq!(
            std::fs::read(&f).unwrap(),
            b"words\n",
            "a refusal that emptied the file anyway is the bug",
        );
    }

    /// **Across directories is the same verb**, which is why it is
    /// spelled `mv`: a rename inside one directory and a move into
    /// another are one thing to the person doing it.
    #[test]
    fn a_name_moves_inside_a_directory_and_across_two() {
        let dir = tmp("mv");
        let a = dir.join("one.rs");
        std::fs::write(&a, "x").unwrap();
        let b = dir.join("two.rs");
        rename_path(a.to_str().unwrap(), b.to_str().unwrap()).unwrap();
        assert!(!a.exists() && b.exists(), "the name moved, not copied");

        let sub = dir.join("src");
        std::fs::create_dir(&sub).unwrap();
        let c = sub.join("two.rs");
        rename_path(b.to_str().unwrap(), c.to_str().unwrap()).unwrap();
        assert!(!b.exists() && c.exists(), "crossing a directory is the same verb");
    }

    /// **The target is never overwritten**, and this is the assertion
    /// the whole check exists for: `std::fs::rename` replaces a file on
    /// unix without a word, so without the test in front of it one
    /// mistyped name eats another file and answers 好了.
    #[test]
    fn a_taken_target_is_refused_and_what_wore_the_name_survives() {
        let dir = tmp("clobber");
        let a = dir.join("mine.rs");
        std::fs::write(&a, "mine\n").unwrap();
        let b = dir.join("theirs.rs");
        std::fs::write(&b, "theirs\n").unwrap();

        let err = rename_path(a.to_str().unwrap(), b.to_str().unwrap())
            .expect_err("that name is taken");
        assert!(err.contains("theirs.rs"), "the refusal names the taken name: {err}");
        assert_eq!(
            std::fs::read(&b).unwrap(),
            b"theirs\n",
            "the other file is still there — a refusal that moved anyway is the bug",
        );
        assert_eq!(std::fs::read(&a).unwrap(), b"mine\n", "and so is the one being moved");
    }

    /// Relative paths name nowhere, for the reason `Ls` demands an
    /// absolute one. Both arguments are asked, because a relative
    /// *target* is the half that would otherwise resolve against
    /// whatever directory the serve process happened to start in.
    #[test]
    fn a_relative_path_names_nowhere_on_either_side() {
        let dir = tmp("rel");
        let a = dir.join("one.rs");
        std::fs::write(&a, "x").unwrap();
        assert!(mkdir("src").is_err(), "a relative path names nowhere");
        assert!(rename_path("one.rs", a.to_str().unwrap()).is_err(), "nor as a source");
        assert!(rename_path(a.to_str().unwrap(), "two.rs").is_err(), "nor as a target");
        assert!(a.exists(), "and nothing moved while it was being refused");
    }

    /// **The whitelist is the point**, and it is the same judgment
    /// `khor_gui_core::web::open_link` makes one machine closer to the
    /// screen: what reaches `launch` came out of a model's own words.
    /// Only the refusals are asserted — the allowed side would open
    /// something on whoever ran the tests.
    #[test]
    fn only_an_absolute_path_or_a_page_is_openable() {
        for target in [
            "src/main.rs",
            "./notes.md",
            "",
            "file:///etc/passwd",
            "mailto:someone@example.test",
            "javascript:alert(1)",
            "vscode://file/etc/passwd",
            "-a",
            "-a Calculator",
        ] {
            let err = launch(target).expect_err("must refuse");
            assert!(
                err.contains(&msg::not_openable(target)),
                "refused for being unopenable, not for something else: {target} -> {err}",
            );
        }
    }

    /// **A machine that opens nothing says so by name**, and the name
    /// it says is the variable — the one thing a person can do about it.
    /// Set to nothing is honoured as 「this machine opens nothing」
    /// rather than read as 「fall back to the platform's」, which would
    /// be khor overruling somebody who told it the answer.
    #[test]
    fn a_machine_with_no_opener_refuses_by_naming_the_variable() {
        // Serialised against the test below by running in one test:
        // `set_var` is process-wide, and two tests reading it at once
        // would each see the other's value.
        let before = std::env::var(LAUNCHER_ENV).ok();
        unsafe { std::env::set_var(LAUNCHER_ENV, "") };
        assert_eq!(launcher(), None, "empty means this machine opens nothing");
        let err = launch("/etc/hosts").expect_err("no opener");
        assert!(err.contains(LAUNCHER_ENV), "the refusal names the variable: {err}");

        unsafe { std::env::set_var(LAUNCHER_ENV, "/usr/bin/true") };
        assert_eq!(launcher().as_deref(), Some("/usr/bin/true"), "and a named one wins");
        launch("/etc/hosts").expect("the opener khor was told to use");

        match before {
            Some(v) => unsafe { std::env::set_var(LAUNCHER_ENV, v) },
            None => unsafe { std::env::remove_var(LAUNCHER_ENV) },
        }
    }
}
