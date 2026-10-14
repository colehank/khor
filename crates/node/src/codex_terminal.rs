//! Codex's native thread-id OSC title identifies the currently selected thread.
//! It is selection metadata, never transcript/screen text or a process-start guess.
use crate::adaptor::{Findings, Procs};
use std::collections::{BTreeMap, BTreeSet};

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
        let mut here = clients.iter().filter(|pid| held.contains(pid));
        let pid = *here.next()?;
        if here.next().is_some() { return None; }
        let args: Vec<_> = found.procs.command(pid)?.iter().skip(1).map(std::ffi::OsString::from).collect();
        let configured = found.codex_title_home.as_deref().zip(found.codex_title_root.as_deref()).is_some_and(|(home, root)| {
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
