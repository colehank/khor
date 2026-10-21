//! Codex's native thread-id OSC title identifies the currently selected thread.
//! It is selection metadata, never transcript/screen text or a process-start guess.
use crate::adaptor::{Findings, Procs};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// A title contract already verified for this exact running TUI client.
/// Disk configuration governs future starts; the TUI renders titles from its
/// own local settings. Never reuse this evidence for a recycled PID.
#[derive(Default, serde::Serialize, serde::Deserialize)]
struct TitleClients {
    home: PathBuf,
    clients: BTreeMap<u32, i64>,
}

fn client_contracts(root: &Path, home: &Path, procs: &Procs) -> TitleClients {
    let home = std::fs::canonicalize(home).unwrap_or_else(|_| home.to_path_buf());
    let mut known: TitleClients = std::fs::read(root.join(".khor/codex-title-clients.json"))
        .ok().and_then(|s| serde_json::from_slice(&s).ok())
        .filter(|k: &TitleClients| k.home == home).unwrap_or_default();
    known.home = home;
    known.clients.retain(|pid, since| *since > 0 && scoped(procs, *pid)
        && procs.get(*pid).is_some_and(|p| p.started_ms == *since));
    known
}

fn remember_clients(root: &Path, known: &TitleClients, procs: &Procs) {
    let dir = root.join(".khor");
    if std::fs::create_dir_all(&dir).is_err() { return; }
    let Ok(lock) = std::fs::OpenOptions::new().create(true).truncate(false)
        .read(true).write(true).open(dir.join("codex-title-clients.lock")) else { return };
    if lock.try_lock().is_err() { return; }
    // Merge concurrently verified clients, but prune all dead/recycled ones.
    let mut merged = client_contracts(root, &known.home, procs);
    merged.clients.extend(known.clients.iter().map(|(p, s)| (*p, *s)));
    let Ok(bytes) = serde_json::to_vec(&merged) else { return };
    let path = dir.join("codex-title-clients.json");
    if std::fs::read(&path).ok().as_deref() == Some(bytes.as_slice()) { return; }
    let tmp = path.with_extension(format!("{}.tmp", std::process::id()));
    if crate::link::write_private(&tmp, &bytes).is_ok() {
        let _ = std::fs::rename(&tmp, &path);
    }
    let _ = std::fs::remove_file(tmp);
}

fn uuid_part(value: &str) -> bool {
    value.bytes().enumerate().all(|(i, c)| {
        if matches!(i, 8 | 13 | 18 | 23) {
            c == b'-'
        } else {
            c.is_ascii_hexdigit()
        }
    })
}

pub(crate) fn metadata_title(title: &str) -> bool {
    (title.len() == 36 && uuid_part(title))
        || title
            .strip_suffix("...")
            .is_some_and(|p| p.len() == 29 && uuid_part(p))
}

/// Codex 0.162 renders UUIDs as 29 canonical UUID characters followed by `...`.
/// Match only a unique writer-owned root thread; never fill the missing suffix.
fn selected<'a>(title: &str, threads: &'a [String]) -> Option<&'a str> {
    let prefix = if title.len() == 36 && uuid_part(title) {
        title
    } else if let Some(prefix) = title
        .strip_suffix("...")
        .filter(|p| p.len() == 29 && uuid_part(p))
    {
        prefix
    } else {
        return None;
    };
    let mut matches = threads.iter().filter(|id| {
        id.len() == 36
            && uuid_part(id)
            && id
                .get(..prefix.len())
                .is_some_and(|p| p.eq_ignore_ascii_case(prefix))
    });
    let id = matches.next()?;
    matches.next().is_none().then_some(id.as_str())
}

pub(crate) fn scoped(procs: &Procs, pid: u32) -> bool {
    if procs.get(pid).is_none_or(|p| !crate::adaptor::process_name_matches(&p.name, "codex")) { return false; }
    let Some(argv) = procs.command(pid) else {
        return false;
    };
    let args: Vec<std::ffi::OsString> = argv.iter().skip(1).map(std::ffi::OsString::from).collect();
    crate::codex_process::interactive(&args)
}

/// Assign live scoped TUI clients before LiveKind seats vendor rows in hosts.
/// tmux pane metadata outranks the enclosing host's possibly stale OSC title.
pub(crate) fn select(found: &mut Findings, hosts: &[(u32, Option<String>)]) {
    let location = found.codex_title_root.clone().zip(found.codex_title_home.clone());
    let mut known = location.as_ref().map(|(root, home)| client_contracts(root, home, &found.procs));
    let clients: BTreeSet<_> = found
        .procs
        .named("codex")
        .into_iter()
        .filter(|(pid, _)| scoped(&found.procs, *pid))
        .map(|(pid, _)| pid)
        .collect();
    // A missing, malformed, changed or ambiguous native title must clear any
    // former birth/cwd guess for this scoped client before it can wear a row.
    for row in &mut found.rows {
        if row.category == "codex" && (!found.codex_threads.is_empty() || !found.codex_pending.is_empty()) {
            row.sighting.pids.retain(|p| !clients.contains(p));
        }
    }
    let candidates: Vec<_> = found
        .codex_threads
        .iter()
        .chain(&found.codex_pending)
        .cloned()
        .collect();
    let mut cwd_by_pid = BTreeMap::<u32, Option<String>>::new();
    let mut pane_clients = BTreeSet::new();
    let mut choices = BTreeMap::<u32, BTreeSet<Option<String>>>::new();
    let client_in = |root| {
        let held = found.procs.held_by(&[root]);
        // A Ctrl-Z job is alive and still belongs to this shell, but its
        // last title must not compete with the current runnable client. Keep
        // it in `clients` above so any former binding is cleared as well.
        let mut here = clients
            .iter()
            .filter(|pid| held.contains(pid) && !found.procs.suspended(**pid));
        let pid = *here.next()?;
        if here.next().is_some() { return None; }
        let args: Vec<_> = found.procs.command(pid)?.iter().skip(1).map(std::ffi::OsString::from).collect();
        let configured = known.as_ref().is_some_and(|k| k.clients.contains_key(&pid)) || found.codex_title_home.as_deref().zip(found.codex_title_root.as_deref()).is_some_and(|(home, root)| {
            found.procs.get(pid).is_some_and(|p| crate::recognition::native_title_for(root, home, p.cwd.as_deref(), p.started_ms, &args))
        });
        crate::codex_process::selection_title_or(&args, configured).then_some(pid)
    };
    for pane in found.multiplexed.iter().flat_map(|m| &m.panes) {
        if let Some(pid) = client_in(pane.pid) {
            pane_clients.insert(pid);
            cwd_by_pid.insert(pid, pane.cwd.clone());
            choices.entry(pid).or_default().insert(
                pane.title
                    .as_deref()
                    .and_then(|title| selected(title, &candidates))
                    .map(str::to_owned),
            );
        }
    }
    for (root, title) in hosts {
        if let Some(pid) = client_in(*root)
            && !pane_clients.contains(&pid)
        {
            choices.entry(pid).or_default().insert(
                title
                    .as_deref()
                    .and_then(|title| selected(title, &candidates))
                    .map(str::to_owned),
            );
        }
    }
    for (pid, ids) in choices {
        if ids.len() != 1 {
            continue;
        }
        let Some(Some(id)) = ids.into_iter().next() else {
            continue;
        };
        if let Some(row) = found
            .rows
            .iter_mut()
            .find(|r| r.category == "codex" && r.sighting.vendor_session_id == id)
        {
            row.sighting.pids.push(pid);
        } else if found.codex_pending.contains(&id) {
            let process = found.procs.get(pid).expect("selected live client");
            found.rows.push(crate::adaptor::Found {
                category: "codex",
                kind: khor_core::kind::TUI,
                sighting: crate::adaptor::Sighting {
                    vendor_session_id: id,
                    title: "codex".into(),
                    word: khor_core::State::Idle,
                    at_ms: process.started_ms,
                    pids: vec![pid],
                    cwd: cwd_by_pid.get(&pid).cloned().flatten().or_else(|| {
                        process
                            .cwd
                            .as_ref()
                            .map(|p| p.to_string_lossy().into_owned())
                    }),
                },
            });
        } else {
            continue;
        }
        if let Some(known) = &mut known {
            let started_ms = found.procs.get(pid).unwrap().started_ms;
            if started_ms > 0 { known.clients.insert(pid, started_ms); }
        }
    }
    if let (Some((root, _)), Some(known)) = (location, known) {
        if !known.clients.is_empty() || root.join(".khor/codex-title-clients.json").exists() {
            remember_clients(&root, &known, &found.procs);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adaptor::{Found, Multiplexed, PaneReading, Proc, Sighting};
    const A: &str = "019fd755-e464-7aaa-8aaa-aaaaaaaaaaaa";
    const B: &str = "019fd755-e465-7aaa-8aaa-bbbbbbbbbbbb";
    fn title(id: &str) -> String {
        format!("{}...", &id[..29])
    }
    fn row(id: &str) -> Found {
        Found {
            category: "codex",
            kind: "tui",
            sighting: Sighting {
                vendor_session_id: id.into(),
                title: "project".into(),
                word: khor_core::State::Done,
                at_ms: 100,
                pids: vec![],
                cwd: Some("/same".into()),
            },
        }
    }
    fn fixture() -> Findings {
        let proc = |name: &str, ppid| Proc {
            name: name.into(),
            started_ms: 100,
            cwd: Some("/same".into()),
            ppid,
        };
        let args = vec![
            "codex",
            "--no-daemon",
            "-c",
            "tui.terminal_title=[\"thread-id\"]",
            "resume",
            A,
        ];
        Findings {
            codex_threads: vec![A.into(), B.into()],
            rows: vec![row(A), row(B)],
            procs: Procs::of([(10, proc("zsh", None)), (11, proc("codex", Some(10)))])
                .with_commands([(11, args.into_iter().map(str::to_owned).collect())]),
            ..Default::default()
        }
    }
    #[test]
    fn verified_title_contract_survives_readers_but_not_pid_reuse_or_another_home() {
        let root = std::env::temp_dir().join(format!("khor-title-clients-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let reading = || {
            let mut found = fixture();
            found.codex_title_root = Some(root.clone());
            found.codex_title_home = Some(root.clone());
            found
        };
        let mut first = reading();
        select(&mut first, &[(10, Some(title(A)))]);
        assert_eq!(first.rows[0].sighting.pids, vec![11]);
        let mut next = reading();
        // Remove the configuration evidence in the next reader; the verified
        // exact client still needs the CURRENT title, not remembered thread A.
        next.procs = next.procs.with_commands([(11, vec!["codex".into()])]);
        select(&mut next, &[(10, Some(title(B)))]);
        assert!(next.rows[0].sighting.pids.is_empty());
        assert_eq!(next.rows[1].sighting.pids, vec![11]);
        let mut replacement = reading();
        replacement.procs = Procs::of([
            (10, Proc { name: "zsh".into(), started_ms: 100, cwd: None, ppid: None }),
            (11, Proc { name: "codex".into(), started_ms: 200, cwd: None, ppid: Some(10) }),
        ]).with_commands([(11, vec!["codex".into()])]);
        select(&mut replacement, &[(10, Some(title(A)))]);
        assert!(replacement.rows.iter().all(|r| r.sighting.pids.is_empty()));
        let mut wrong_home = reading();
        wrong_home.procs = wrong_home.procs.with_commands([(11, vec!["codex".into()])]);
        wrong_home.codex_title_home = Some(root.join("other"));
        select(&mut wrong_home, &[(10, Some(title(A)))]);
        assert!(wrong_home.rows.iter().all(|r| r.sighting.pids.is_empty()));
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn strict_unique_uuid_metadata_rejects_malformed_unicode_and_collisions() {
        let ids = vec![A.into(), B.into()];
        assert_eq!(selected(&title(A), &ids), Some(A));
        assert_eq!(selected(A, &ids), Some(A));
        for invalid in [
            "project",
            "019fd755",
            "019fd755-e464-7aaa-8aaa-aaaaaaa…",
            "019fd755-e464-7aaa-8aaa-aaaaaé...",
        ] {
            assert_eq!(selected(invalid, &ids), None);
        }
        let collision = format!("{}bbbbbbb", &A[..29]);
        assert_eq!(selected(&title(A), &[A.into(), collision]), None);
        assert_eq!(
            selected(&title(A), &[B.into()]),
            None,
            "no writer-owned candidate"
        );
    }
    #[test]
    fn current_title_switches_even_done_rows_and_clears_old_and_missing_guesses() {
        let mut found = fixture();
        found.rows[0].sighting.pids.push(11);
        select(&mut found, &[(10, Some(title(B)))]);
        assert!(found.rows[0].sighting.pids.is_empty());
        assert_eq!(found.rows[1].sighting.pids, vec![11]);
        select(&mut found, &[(10, Some(title(A)))]);
        assert_eq!(found.rows[0].sighting.pids, vec![11]);
        assert!(found.rows[1].sighting.pids.is_empty());
        select(&mut found, &[(10, None)]);
        assert!(found.rows.iter().all(|r| r.sighting.pids.is_empty()));
    }
    #[test]
    fn same_directory_clients_keep_distinct_native_selection_and_daemons_are_excluded() {
        let mut found = fixture();
        let proc = |name: &str, ppid| Proc {
            name: name.into(),
            started_ms: 100,
            cwd: Some("/same".into()),
            ppid,
        };
        let argv = || {
            ["codex", "-c", "tui.terminal_title=[\"thread-id\"]"]
                .into_iter()
                .map(str::to_owned)
                .collect()
        };
        found.procs = Procs::of([
            (10, proc("zsh", None)),
            (11, proc("codex", Some(10))),
            (20, proc("zsh", None)),
            (21, proc("codex", Some(20))),
            (31, proc("codex", Some(20))),
        ])
        .with_commands([
            (11, argv()),
            (21, argv()),
            (
                31,
                [
                    "codex",
                    "app-server",
                    "-c",
                    "tui.terminal_title=[\"thread-id\"]",
                ]
                .into_iter()
                .map(str::to_owned)
                .collect(),
            ),
        ]);
        select(&mut found, &[(10, Some(title(A))), (20, Some(title(B)))]);
        assert_eq!(found.rows[0].sighting.pids, vec![11]);
        assert_eq!(found.rows[1].sighting.pids, vec![21]);
        assert!(
            !found.rows.iter().any(|r| r.sighting.pids.contains(&31)),
            "the shared server is never a terminal client"
        );
    }

    #[cfg(unix)]
    #[test]
    fn native_snapshot_distinguishes_stopped_and_resumed_jobs_without_losing_liveness() {
        struct Child(std::process::Child);
        impl Drop for Child {
            fn drop(&mut self) { let _ = self.0.kill(); let _ = self.0.wait(); }
        }
        let child = Child(std::process::Command::new("/bin/sleep").arg("30").spawn().unwrap());
        let pid = child.0.id();
        // SAFETY: signal only the child owned and reaped by this test.
        assert_eq!(unsafe { libc::kill(pid as i32, libc::SIGSTOP) }, 0);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        loop {
            let snapshot = Procs::snapshot();
            assert!(snapshot.get(pid).is_some(), "stopped jobs remain alive");
            if snapshot.suspended(pid) { break; }
            assert!(std::time::Instant::now() < deadline, "OS snapshot did not see SIGSTOP");
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        // SAFETY: same owned child; no user process is signalled.
        assert_eq!(unsafe { libc::kill(pid as i32, libc::SIGCONT) }, 0);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        loop {
            let snapshot = Procs::snapshot();
            assert!(snapshot.get(pid).is_some());
            if !snapshot.suspended(pid) { break; }
            assert!(std::time::Instant::now() < deadline, "OS snapshot did not see SIGCONT");
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }

    #[test]
    fn suspended_job_does_not_block_the_current_client_or_keep_its_old_binding() {
        let mut found = fixture();
        let current = found.procs.get(11).unwrap().clone();
        let argv = found.procs.command(11).unwrap().to_vec();
        found.procs = Procs::of([
            (10, found.procs.get(10).unwrap().clone()),
            (11, current.clone()),
            (12, current),
        ]).with_commands([(11, argv.clone()), (12, argv)]).with_suspended([11]);
        found.rows[0].sighting.pids.push(11);
        select(&mut found, &[(10, Some(title(B)))]);
        assert!(found.rows[0].sighting.pids.is_empty(), "the stopped client's stale seat clears");
        assert_eq!(found.rows[1].sighting.pids, vec![12]);

        // The same rule applies inside a tmux pane. A stopped job does not
        // override the active pane title or make the pane look ambiguous.
        found.multiplexed.push(Multiplexed {
            found: row("mux"), holds: vec![10, 11, 12], running: None,
            panes: vec![PaneReading { pid: 10, title: Some(title(A)), ..Default::default() }],
        });
        select(&mut found, &[(10, Some(title(B)))]);
        assert_eq!(found.rows[0].sighting.pids, vec![12]);
        assert!(found.rows[1].sighting.pids.is_empty());

        // fg restores eligibility on the next snapshot; this is not a
        // persistent exclusion and does not kill or resume either process.
        found.procs = found.procs.clone().with_suspended([12]);
        select(&mut found, &[]);
        assert_eq!(found.rows[0].sighting.pids, vec![11]);

        found.procs = found.procs.clone().with_suspended([]);
        select(&mut found, &[]);
        assert!(found.rows.iter().all(|r| r.sighting.pids.is_empty()), "two runnable clients stay ambiguous");
        found.procs = found.procs.clone().with_suspended([11, 12]);
        select(&mut found, &[]);
        assert!(found.rows.iter().all(|r| r.sighting.pids.is_empty()), "a stopped-only terminal cannot reuse a stale title");
    }

    #[test]
    fn pane_metadata_outranks_outer_host_and_subagents_or_custom_titles_never_bind() {
        let mut found = fixture();
        found.multiplexed.push(Multiplexed {
            found: row("mux"),
            holds: vec![10, 11],
            running: None,
            panes: vec![PaneReading {
                pid: 10,
                title: Some(title(B)),
                ..Default::default()
            }],
        });
        select(&mut found, &[(10, Some(title(A)))]);
        assert!(found.rows[0].sighting.pids.is_empty());
        assert_eq!(found.rows[1].sighting.pids, vec![11]);
        found.codex_threads.retain(|id| id != B); // A subagent can remain a row but is ineligible.
        select(&mut found, &[(10, Some(title(A)))]);
        assert!(found.rows.iter().all(|r| r.sighting.pids.is_empty()));
        found.procs = found.procs.clone().with_commands([(
            11,
            vec![
                "codex".into(),
                "--no-daemon".into(),
                "-c".into(),
                "tui.terminal_title=[\"project\"]".into(),
            ],
        )]);
        select(&mut found, &[(10, Some(title(A)))]);
        assert!(found.rows.iter().all(|r| r.sighting.pids.is_empty()));
    }
}
