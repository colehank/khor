//! Every `khor serve` this user is running on this machine, whichever
//! store it serves (SHELL.md 第五轮 第 0 条).
//!
//! Everything else in khor asks about *one* store — `endpoint.json`,
//! `serve.pid`, the socket — and that is exactly how turing ran two
//! residents for five days without one row of `khor doctor` going
//! amber: a keeper from 9 月 17 日 serving `.khor-hosts/turing` (a
//! second identity for the same machine, left over from the `.profile`
//! line of 9 月 11 日, restarted by a stale `khor-serve-up` on every
//! login), and the resident the installer had just restarted serving
//! `$HOME`. The old one held port 5467; the new one's web face never
//! came up. Each store's own serve was healthy. Only the process table
//! knows about both.
//!
//! **One resident is a keeper and its child**, not two: `ps` shows both
//! as `khor serve` by design (the keeper module has why), and a doctor
//! that counted processes would cry wolf on every healthy machine. A
//! kept serve whose keeper is gone counts on its own — that is a
//! squatter, and exactly the kind of thing this is here to name.

use std::path::{Path, PathBuf};

/// One `khor serve` process, as read out of the process table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seen {
    pub pid: u32,
    pub parent: Option<u32>,
    /// Carries [`crate::keeper::INNER_ENV`]: the kept serve, not the
    /// keeper.
    pub inner: bool,
    /// Its `KHOR_HOME`, when it was started with one.
    pub home: Option<PathBuf>,
}

/// One resident: the pid a person would stop (the keeper's, or the lone
/// serve's), and the store it is serving.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resident {
    pub pid: u32,
    pub root: PathBuf,
}

/// Folds processes into residents. `default_root` is the store a serve
/// without `KHOR_HOME` works out for itself — the same rule for every
/// process of one user on one machine, so the reader's own answer is
/// theirs too.
///
/// Sorted by pid, so two runs print the same line.
pub fn residents(seen: &[Seen], default_root: &Path) -> Vec<Resident> {
    let keepers: std::collections::HashSet<u32> =
        seen.iter().filter(|s| !s.inner).map(|s| s.pid).collect();
    let mut out: Vec<Resident> = seen
        .iter()
        .filter(|s| !(s.inner && s.parent.is_some_and(|p| keepers.contains(&p))))
        .map(|s| Resident {
            pid: s.pid,
            root: s.home.clone().unwrap_or_else(|| default_root.to_path_buf()),
        })
        .collect();
    out.sort_by_key(|r| r.pid);
    out
}

/// Whether a command line is `khor serve`: the program's file name
/// starts with `khor` (the keeper runs from `khor-<fingerprint>` copies
/// behind the shield) and is not the app, and the verb is `serve`.
fn is_serve(cmd: &[std::ffi::OsString]) -> bool {
    let Some(prog) = cmd.first() else {
        return false;
    };
    let name = Path::new(prog)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    name.starts_with("khor")
        && !name.starts_with("khor-gui")
        && cmd.get(1).is_some_and(|v| v == "serve")
}

/// Every `khor serve` of this user on this machine, right now.
///
/// **Only this user's**: another account's serve on a shared box is
/// theirs, and its environment is not ours to read anyway.
#[cfg(unix)]
pub fn here() -> Vec<Seen> {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
    let mut sys = System::new();
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing()
            .with_cmd(UpdateKind::Always)
            .with_environ(UpdateKind::Always)
            .with_user(UpdateKind::Always),
    );
    let me = unsafe { libc::getuid() };
    let inner_key = format!("{}=", crate::keeper::INNER_ENV);
    let mut out = Vec::new();
    for (pid, p) in sys.processes() {
        // A thread shows up as a process on Linux; its parent's row is
        // the one that counts.
        if p.thread_kind()
            .is_some_and(|k| k == sysinfo::ThreadKind::Userland)
        {
            continue;
        }
        if p.user_id().map(|u| **u) != Some(me) || !is_serve(p.cmd()) {
            continue;
        }
        let mut inner = false;
        let mut home = None;
        for kv in p.environ() {
            let kv = kv.to_string_lossy();
            if kv.starts_with(&inner_key) {
                inner = true;
            } else if let Some(v) = kv.strip_prefix("KHOR_HOME=") {
                home = Some(PathBuf::from(v));
            }
        }
        out.push(Seen {
            pid: pid.as_u32(),
            parent: p.parent().map(|p| p.as_u32()),
            inner,
            home,
        });
    }
    out
}

#[cfg(not(unix))]
pub fn here() -> Vec<Seen> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seen(pid: u32, parent: u32, inner: bool, home: Option<&str>) -> Seen {
        Seen {
            pid,
            parent: Some(parent),
            inner,
            home: home.map(PathBuf::from),
        }
    }

    /// turing, 2026-09-22, as `ps` and `/proc/<pid>/environ` read it:
    /// four processes, two residents.
    #[test]
    fn turing_reads_as_two_residents_not_four_processes() {
        let side = "/nfs/home/.khor-hosts/turing";
        let front = "/nfs/home";
        let ps = [
            seen(259652, 1, false, Some(side)),
            seen(2387480, 259652, true, Some(side)),
            seen(2387159, 1, false, Some(front)),
            seen(2387167, 2387159, true, Some(front)),
        ];
        assert_eq!(
            residents(&ps, Path::new("/unused")),
            vec![
                Resident {
                    pid: 259652,
                    root: side.into()
                },
                Resident {
                    pid: 2387159,
                    root: front.into()
                },
            ]
        );
    }

    /// hinton the same morning: a keeper and its child, one resident —
    /// the shape every healthy machine has, and the one a count of
    /// processes would have called two.
    #[test]
    fn a_keeper_and_its_child_are_one_resident() {
        let ps = [
            seen(833241, 1, false, None),
            seen(1228765, 833241, true, None),
        ];
        assert_eq!(
            residents(&ps, Path::new("/home/x")),
            vec![Resident {
                pid: 833241,
                root: "/home/x".into()
            }]
        );
    }

    /// A kept serve whose keeper died is a resident of its own: nobody
    /// stands behind it, and it is holding a store.
    #[test]
    fn an_orphaned_inner_counts() {
        let ps = [seen(10, 1, true, Some("/s"))];
        assert_eq!(residents(&ps, Path::new("/d")).len(), 1);
    }

    #[test]
    fn only_khor_serve_is_a_serve() {
        let c = |v: &[&str]| v.iter().map(std::ffi::OsString::from).collect::<Vec<_>>();
        assert!(is_serve(&c(&["/home/x/.local/bin/khor", "serve"])));
        assert!(is_serve(&c(&[
            "/tmp/khor-shield-1110-28ed/khor-21aaba0-6ab16313-3b7cbf8",
            "serve"
        ])));
        assert!(!is_serve(&c(&["/usr/bin/khor", "devices"])));
        assert!(!is_serve(&c(&["/Applications/khor.app/khor-gui", "serve"])));
        assert!(!is_serve(&c(&["/usr/bin/vim", "serve"])));
    }
}
