//! The command ledger read off real shells (roadmap 1.2's acceptance):
//! zsh 5.9, bash 3.2 and fish 4.9, each with khor's integration loaded the
//! way a host loads it, ran the same ten commands — a failing one, a `cd`,
//! `vim` on the alternate screen, a pipe, a multi-line loop and one that
//! exits 3 — and their terminal output was recorded byte for byte
//! (`tests/fixtures/shell/*.bin`, host and user names replaced).

use khor_node::commands::Ledger;

const TEN: [(&str, i32); 10] = [
    ("echo one", 0),
    ("false", 1),
    ("cd /tmp", 0),
    ("vim -u NONE -N", 0),
    ("ls /", 0),
    ("true && echo two", 0),
    (r#"printf "x\\n" | wc -l"#, 0),
    ("sleep 1", 0),
    ("", 0),
    ("", 3),
];

fn read(name: &str, multi: &str, last: &str) {
    let bytes = std::fs::read(format!(
        "{}/tests/fixtures/shell/{name}.bin",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    let mut ledger = Ledger::default();
    let mut vim_full_screen = false;
    // Fed in 61-byte pieces: marks are split across reads as a PTY splits
    // them.
    for (i, chunk) in bytes.chunks(61).enumerate() {
        ledger.feed(chunk, i as u64);
        let d = ledger.doing();
        if d.running && d.command.as_deref() == Some("vim -u NONE -N") && d.full_screen {
            vim_full_screen = true;
        }
    }
    let saved = ledger.saved();
    let got: Vec<(String, Option<i32>)> = saved
        .recent
        .iter()
        .map(|c| (c.text.clone().unwrap_or_default(), c.exit))
        .collect();
    let mut want: Vec<(String, Option<i32>)> =
        TEN.iter().map(|(t, e)| (t.to_string(), Some(*e))).collect();
    want[8].0 = multi.to_owned();
    want[9].0 = last.to_owned();
    assert_eq!(
        got, want,
        "{name}: the ten commands, as typed, with their statuses"
    );
    assert_eq!(
        saved.recent[2].cwd.as_deref(),
        Some(saved.recent[0].cwd.as_deref().unwrap()),
        "{name}: cd's own line ran where the shell stood"
    );
    assert_eq!(
        saved.recent[3].cwd.as_deref(),
        Some("/tmp"),
        "{name}: after cd, the next command runs in /tmp"
    );
    assert!(
        vim_full_screen,
        "{name}: vim held the alternate screen while it ran"
    );
    assert!(
        !saved.doing.running && !saved.doing.full_screen,
        "{name}: back at a prompt"
    );
    assert!(
        saved
            .recent
            .windows(2)
            .all(|w| w[0].end_at <= Some(w[1].at)),
        "{name}: in stream order"
    );
}

#[test]
fn zsh_marks_ten_commands() {
    read("zsh", "for i in 1 2; do\necho $i; done", "(exit 3)");
}

#[test]
fn bash_marks_ten_commands() {
    // bash's history joins a multi-line command into one line.
    read("bash", "for i in 1 2; do echo $i; done", "sh -c 'exit 3'");
}

#[test]
fn fish_marks_ten_commands() {
    read("fish", "for i in 1 2\necho $i; end", "sh -c 'exit 3'");
}

/// **A tmux the person typed in a khor shell** (SESS-BRIEF 3, step 1):
/// recorded from a real pty — zsh with khor's integration typed `tmux`
/// on a private server, two panes whose zsh loads the integration ran
/// commands (one still running in `%1` while `%0` ran two more), a third
/// pane ran `zsh -f`, and the server was killed. Each pane's commands
/// are its own, with their pane, directory and status; the shell's own
/// `tmux` ends when tmux does, not at the first pane's prompt.
#[test]
fn a_tmux_typed_in_a_khor_shell_keeps_each_panes_commands_apart() {
    let bytes = std::fs::read(format!(
        "{}/tests/fixtures/shell/zsh-tmux.bin",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    let mut ledger = Ledger::default();
    let mut done = Vec::new();
    let mut tmux_ran = false;
    for (i, chunk) in bytes.chunks(61).enumerate() {
        ledger.feed(chunk, i as u64);
        done.extend(ledger.take_finished());
        let d = ledger.doing();
        if d.command.as_deref().is_some_and(|c| c.starts_with("tmux ")) && d.running {
            tmux_ran = true;
        }
    }
    let got: Vec<(Option<&str>, &str, Option<i32>, &str)> = done
        .iter()
        .map(|c| {
            (
                c.pane.as_deref(),
                c.text.as_deref().unwrap_or(""),
                c.exit,
                c.cwd.as_deref().unwrap_or(""),
            )
        })
        .collect();
    let tmux = "tmux -L k3 -f /dev/null new -s s3 -c /s3/w/one";
    assert_eq!(
        got,
        [
            (Some("%0"), "echo one-a", Some(0), "/s3/w/one"),
            (Some("%0"), "(exit 2)", Some(2), "/s3/w/one"),
            (Some("%1"), "echo two-a", Some(0), "/s3/w/two"),
            (Some("%0"), "echo one-b", Some(0), "/s3/w/one"),
            (Some("%0"), "cd ../three", Some(0), "/s3/w/one"),
            (Some("%1"), "sleep 2; echo two-b", Some(0), "/s3/w/two"),
            (Some("%0"), "echo one-c", Some(0), "/s3/w/three"),
            (None, tmux, Some(1), "/s3/w/start"),
        ]
    );
    assert!(tmux_ran, "the shell's row said tmux ran while it did");
    let d = ledger.doing();
    assert_eq!(
        (d.command.as_deref(), d.running, d.cwd.as_deref()),
        (Some("exit"), true, Some("/s3/w/start")),
        "the shell's own context, back at its prompt where it typed exit"
    );    // Step 2: which panes send marks; `zsh -f` in %2 sent none.
    assert_eq!(
        ledger.saved().marked_panes,
        Some(vec!["%0".to_owned(), "%1".to_owned()])
    );
}
