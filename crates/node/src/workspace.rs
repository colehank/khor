//! **工作区: a directory on a machine that a person said they work in.**
//!
//! # Declared, not derived
//!
//! The list khor used to show was inferred — the vendors' session
//! records folded up to the nearest checkout. The user threw that out
//! on 2026-09-14 with two facts nothing on a disk can answer:
//! 「用户有agent记录的目录不一定是工作区.可能只是随手打开的」and
//! 「git目录也不一定是工作区.很多工作区开始的时候不是git目录,或用户
//! 根本就没想着用git管理」. So the list lives in a replicated document
//! (`khor_sync::workspaces`) and this module is what the rest of khor
//! reads it as.
//!
//! What survived the deletion is [`crate::place`]: 常去的地方, the
//! wizard's completion list, is still derived from the same records.
//! The two were one fold with one filter between them; they are now two
//! different kinds of thing, which is why only one of them has a table.
//!
//! # A workspace is `(machine, directory)` and its name is the basename
//!
//! One repository checked out on three machines is three workspaces —
//! three disks, three sets of uncommitted files, three answers to
//! 「什么时候改的」. The name is the directory's last component and is
//! never stored: a name that can be set is a name that can drift from
//! the directory it points at, and the directory is what identifies the
//! row.

use std::path::Path;

/// One workspace, as every face reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceInfo {
    /// The machine's id, hex — the table's half of the key.
    pub device: String,
    /// That machine's current display name, or empty when the device
    /// table has no row for it. **The row survives either way**: the
    /// directory is still real on that disk, and hiding the row would
    /// make it unremovable from everywhere.
    pub machine: String,
    /// Absolute, on that machine.
    pub path: String,
    /// [`basename`] of the path. Derived here so every face spells it
    /// the same way.
    pub name: String,
    pub pinned: bool,
}

/// `/a/b/khor` → `khor`. The last real component, so a trailing slash
/// does not answer with an empty name, and a path with no component at
/// all answers with itself rather than with nothing.
pub fn basename(path: &str) -> String {
    path.rsplit('/')
        .find(|s| !s.is_empty())
        .unwrap_or(path)
        .to_owned()
}

/// Directories [`touched`] does not walk into.
///
/// Every one of these is written by a tool rather than by a person, and
/// all of them are touched constantly — a `cargo build` would make the
/// whole list re-sort while nobody edited anything. The same names are
/// skipped by the satellite's script, which is why they are one list
/// and not two (`crate::satellite::TOUCHED_SH`).
pub const SKIP: [&str; 7] = [
    ".git",
    "node_modules",
    "target",
    ".venv",
    "__pycache__",
    "dist",
    "build",
];

/// How many entries one answer is allowed to look at.
///
/// **The answer is a time, not a promise.** Past this many entries the
/// walk stops and answers the newest mtime it has already seen, which
/// can be older than the real newest. That is the honest trade for a
/// row on a list: a workspace holding a tree nobody meant to put there
/// must not spend a screen's worth of time being stat'ed, and being off
/// by a few minutes on such a row costs the reader nothing. Nothing
/// decides anything on this number — it orders a list.
pub const MOST_ENTRIES: usize = 20_000;

/// The newest file mtime under `dir`, ms since the epoch; `0` when
/// there is no file to date (an empty directory, or one this process
/// cannot read).
///
/// **A file's mtime and not a session's timestamp**, which is the whole
/// point of the column: a workspace edited in an editor, or by somebody
/// over ssh, or by a tool — none of those open a khor session, and all
/// of them are work. A directory nobody has ever run an agent in is
/// still a directory that changed this morning.
///
/// Symlinks are dated but never followed. A link's own mtime is a fact
/// about this tree; walking through it would leave the workspace, and
/// two links pointing at each other would not terminate.
pub fn touched(dir: &Path) -> i64 {
    let mut newest = 0i64;
    let mut seen = 0usize;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd.flatten() {
            if seen >= MOST_ENTRIES {
                return newest;
            }
            seen += 1;
            let Ok(kind) = e.file_type() else { continue };
            if kind.is_dir() {
                let name = e.file_name();
                if SKIP.iter().any(|s| std::ffi::OsStr::new(s) == name) {
                    continue;
                }
                stack.push(e.path());
                continue;
            }
            let at = e
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0);
            newest = newest.max(at);
        }
    }
    newest
}

/// How close a file's birth time must be to its modification time for
/// [`was_added`] to call it made rather than edited.
pub const ADDED_WITHIN_MS: i64 = 60_000;

/// **Whether a file was made rather than edited** (UX-51): its birth
/// time (macOS `st_birthtime`, Linux `statx`'s btime) within
/// [`ADDED_WITHIN_MS`] of its modification time. False where the file
/// system keeps no birth time — an unknown is read as an edit.
pub fn was_added(path: &Path) -> bool {
    let ms = |t: std::time::SystemTime| {
        t.duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .ok()
    };
    let Ok(m) = std::fs::metadata(path) else {
        return false;
    };
    match (
        m.created().ok().and_then(ms),
        m.modified().ok().and_then(ms),
    ) {
        (Some(born), Some(changed)) => (changed - born).abs() <= ADDED_WITHIN_MS,
        _ => false,
    }
}

/// How many entries [`newest_file`] looks at before it answers what it
/// has: a folder holding a dump of files still answers at once.
pub const MOST_NEWEST: usize = 4_000;

/// **A plain folder's most recently modified file** (UX-48), as a path
/// relative to `dir` and its mtime in ms since the epoch: the top level
/// and one directory down. Dot-named entries (`.DS_Store`, `.cache/`)
/// and [`SKIP`]'s directories are left out — neither is the work. Only
/// regular files are named; a link is not followed. `None` when there
/// is no file to name. Ties go to the path that sorts first, so the
/// answer does not flicker between equal times.
pub fn newest_file(dir: &Path) -> Option<(String, i64)> {
    look_folder(dir).0
}

/// [`newest_file`], with the relative path of every file it looked at —
/// what a later reading compares against to tell a file made since from
/// one rewritten in place (UX-51, `wsfacts`).
pub fn look_folder(dir: &Path) -> (Option<(String, i64)>, std::collections::HashSet<String>) {
    let mut names = std::collections::HashSet::new();
    let best = newest_into(dir, &mut names);
    (best, names)
}

fn newest_into(dir: &Path, names: &mut std::collections::HashSet<String>) -> Option<(String, i64)> {
    fn mtime(m: &std::fs::Metadata) -> i64 {
        m.modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0)
    }
    let mut best: Option<(String, i64)> = None;
    let mut seen = 0usize;
    let mut take = |path: String, at: i64| {
        names.insert(path.clone());
        let better = match &best {
            None => true,
            Some((p, t)) => at > *t || (at == *t && path < *p),
        };
        if better {
            best = Some((path, at));
        }
    };
    let mut below: Vec<(String, std::path::PathBuf)> = Vec::new();
    for e in std::fs::read_dir(dir).ok()?.flatten() {
        if seen >= MOST_NEWEST {
            return best;
        }
        seen += 1;
        let Some(name) = e.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        let Ok(kind) = e.file_type() else { continue };
        if kind.is_dir() {
            if !SKIP.contains(&name.as_str()) {
                below.push((name, e.path()));
            }
        } else if kind.is_file()
            && let Ok(m) = e.metadata()
        {
            take(name, mtime(&m));
        }
    }
    for (top, d) in below {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd.flatten() {
            if seen >= MOST_NEWEST {
                return best;
            }
            seen += 1;
            let Some(name) = e.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            if name.starts_with('.') || !e.file_type().is_ok_and(|k| k.is_file()) {
                continue;
            }
            if let Ok(m) = e.metadata() {
                take(format!("{top}/{name}"), mtime(&m));
            }
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// A scratch tree that takes itself away.
    ///
    /// **Cleared on the way out and not only on the way in.** Clearing
    /// first makes a rerun correct; clearing after is what keeps the
    /// machine's temp directory from growing one tree per run forever —
    /// the same leak that cost this checkout 8.6 GB of shield
    /// directories before anybody looked (handoff 001). `Drop` rather
    /// than a line at the end of each test, because an assertion panics
    /// past its cleanup and a `Drop` does not (`crates/cli/tests`'s
    /// `Closer` learned it the same way).
    struct Scratch(PathBuf);

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    impl std::ops::Deref for Scratch {
        type Target = Path;

        fn deref(&self) -> &Path {
            &self.0
        }
    }

    /// The repo's own temp-dir convention (`crates/node/src/tests.rs`),
    /// which is also why this file needs no dev-dependency.
    fn root(tag: &str) -> Scratch {
        let p = std::env::temp_dir().join(format!("khor-ws-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        Scratch(p)
    }

    fn stamp(path: &Path, secs: u64) {
        let t = std::time::UNIX_EPOCH + std::time::Duration::from_secs(secs);
        std::fs::File::open(path).unwrap().set_modified(t).unwrap();
    }

    #[test]
    fn the_name_is_the_last_component_however_the_path_is_spelled() {
        assert_eq!(basename("/Users/x/working/test"), "test");
        assert_eq!(basename("/Users/x/working/test/"), "test");
        assert_eq!(basename("/"), "/");
    }

    /// The newest file anywhere under the tree, not the newest in the
    /// top directory — a workspace is edited from the inside.
    #[test]
    fn touched_answers_the_newest_file_in_the_whole_tree() {
        let tmp = root("newest");
        std::fs::create_dir_all(tmp.join("src/deep")).unwrap();
        std::fs::write(tmp.join("README.md"), "x").unwrap();
        std::fs::write(tmp.join("src/deep/one.rs"), "x").unwrap();
        stamp(&tmp.join("README.md"), 1_000);
        stamp(&tmp.join("src/deep/one.rs"), 2_000);

        assert_eq!(touched(&tmp), 2_000_000, "the deep file is the newest");

        // The control: a directory with nothing to date answers 0
        // rather than inventing a time.
        let empty = tmp.join("src/empty");
        std::fs::create_dir_all(&empty).unwrap();
        assert_eq!(touched(&empty), 0);
    }

    /// **The skip list is the difference between a time and a build
    /// clock.** Without it a `cargo build` re-sorts the whole landing.
    #[test]
    fn touched_does_not_look_inside_the_skipped_directories() {
        let tmp = root("skip");
        std::fs::create_dir_all(tmp.join("target/debug")).unwrap();
        std::fs::create_dir_all(tmp.join("node_modules/pkg")).unwrap();
        std::fs::write(tmp.join("src.txt"), "x").unwrap();
        std::fs::write(tmp.join("target/debug/fresh"), "x").unwrap();
        std::fs::write(tmp.join("node_modules/pkg/index.js"), "x").unwrap();
        stamp(&tmp.join("src.txt"), 1_000);
        stamp(&tmp.join("target/debug/fresh"), 9_000);
        stamp(&tmp.join("node_modules/pkg/index.js"), 8_000);

        assert_eq!(
            touched(&tmp),
            1_000_000,
            "the build output must not date the workspace"
        );
    }

    /// **A plain folder's newest file** (UX-48): the top level and one
    /// directory down, the newest by mtime; hidden entries, the skipped
    /// directories and anything deeper are not looked at.
    #[test]
    fn newest_file_looks_two_levels_down_past_hidden_and_build_directories() {
        let tmp = root("newest-file");
        assert_eq!(newest_file(&tmp), None, "nothing to name");
        std::fs::create_dir_all(tmp.join("notes/deep")).unwrap();
        std::fs::create_dir_all(tmp.join(".cache")).unwrap();
        std::fs::create_dir_all(tmp.join("node_modules")).unwrap();
        for (p, t) in [
            ("a.txt", 1_000),
            ("notes/b.md", 2_000),
            ("notes/deep/c.md", 9_000),
            (".DS_Store", 9_000),
            (".cache/d", 9_000),
            ("node_modules/e.js", 9_000),
        ] {
            std::fs::write(tmp.join(p), "x").unwrap();
            stamp(&tmp.join(p), t);
        }
        assert_eq!(
            newest_file(&tmp),
            Some(("notes/b.md".to_owned(), 2_000_000))
        );
        stamp(&tmp.join("a.txt"), 3_000);
        assert_eq!(
            newest_file(&tmp),
            Some(("a.txt".to_owned(), 3_000_000)),
            "the top level counts the same"
        );
    }

    /// **Made or edited** (UX-51): a file just written is made; one whose
    /// modification time is an hour past its birth was edited since. Where
    /// the file system keeps no birth time both read as edited.
    #[test]
    fn a_file_is_added_while_its_birth_and_its_change_are_a_minute_apart() {
        let tmp = root("was-added");
        let f = tmp.join("new.txt");
        std::fs::write(&f, "x").unwrap();
        let born = std::fs::metadata(&f).unwrap().created().ok();
        assert_eq!(
            was_added(&f),
            born.is_some(),
            "just written: made, where a birth time is kept"
        );
        if let Some(b) = born {
            std::fs::File::open(&f)
                .unwrap()
                .set_modified(b + std::time::Duration::from_secs(3600))
                .unwrap();
            assert!(!was_added(&f), "an hour later: edited");
        }
        assert!(!was_added(&tmp.join("gone.txt")), "no file: not added");
    }
}
