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
