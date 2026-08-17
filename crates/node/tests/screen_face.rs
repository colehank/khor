//! A real host, a real PTY, a real screen: the word an agent TUI wears
//! while it is running, and the provenance it wears it with.
//!
//! Everything else about the PTY fallback is unit-tested against screens
//! upstream recorded. This is the one test that runs the whole chain —
//! host process, pty, vt100, the pattern table, the registry write — and
//! it exists for the two claims that only the whole chain can support.
//!
//! **One: the word tracks.** Before this batch, a `khor open --tui --
//! <an agent with no hook installed>` kept the 忙碌 that `register`
//! wrote and wore it from open to exit. A test that only ever saw 空闲
//! would be satisfied by a detector that had stopped working, so this
//! walks a session through all three words a screen can reach.
//!
//! **Two: it is written down as a guess.** The host must report
//! `Source::Screen`, because that is the whole of what keeps this
//! family underneath the vendor's own files at the merge
//! (`live::rows`). Nothing else in the tree would notice that line being
//! changed to `Reported` — the words would all still be right, and a
//! screen reading would quietly start overruling claude's own status
//! file. That is the regression this test is here for.
//!
//! The agent is a stand-in drawing screens transcribed from upstream's
//! test records, not a real one: a real agent would make this test need
//! an API key, a network, and somebody's tokens, and it would draw
//! whatever this month's build draws. What is real is everything khor
//! owns.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use khor_core::{DeviceId, SessionId, State};
use khor_node::live::{LiveKind, Source};

/// Screens transcribed from `kbwo/ccmanager`'s own detector tests, drawn
/// in sequence with a clear between them. Named `claude` because the
/// basename is what selects the table.
const AGENT: &str = r#"#!/bin/sh
draw() { printf '\033[2J\033[H'; printf '%s\n' "$@"; }
draw 'Processing...' 'Press ESC to interrupt' '──────────────────────────────' '❯' '──────────────────────────────'
sleep 4
draw 'Do you want to continue?' '❯ 1. Yes' '  2. No'
sleep 4
draw 'Command completed successfully' '──────────────────────────────' '❯' '──────────────────────────────'
sleep 6
"#;

fn read_state(root: &PathBuf, id: &SessionId) -> Option<(State, Option<Source>)> {
    let k = LiveKind::new(root.clone(), DeviceId([1; 32]));
    let dir = k.dir_of(id)?;
    let raw = std::fs::read(dir.join("state.json")).ok()?;
    let v: serde_json::Value = serde_json::from_slice(&raw).ok()?;
    let word: State = serde_json::from_value(v.get("word")?.clone()).ok()?;
    let source = v.get("source").and_then(|s| serde_json::from_value(s.clone()).ok());
    Some((word, source))
}

/// Waits for a word, returning what it saw. Polls rather than sleeping a
/// fixed time: the host's own cadence and the table's debounce are both
/// wall-clock, so a fixed sleep here would be measuring how busy the
/// machine is (账本 "固定 sleep 的测试，测的是机器闲不闲").
fn wait_for(root: &PathBuf, id: &SessionId, want: State) -> Option<(State, Option<Source>)> {
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut last = None;
    while Instant::now() < deadline {
        last = read_state(root, id);
        if let Some((word, _)) = &last {
            if *word == want {
                return last;
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    last
}

#[test]
fn a_hosted_agent_wears_what_its_screen_says_and_says_where_it_came_from() {
    let root = std::env::temp_dir().join(format!("khor-screenface-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();

    let bin = root.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let agent = bin.join("claude");
    std::fs::write(&agent, AGENT).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&agent, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    let id = SessionId("tui/screenface".into());
    let k = LiveKind::new(root.clone(), DeviceId([1; 32]));
    k.register(&id, "tui", "stand-in", None, None).unwrap();
    assert_eq!(
        read_state(&root, &id).unwrap(),
        (State::Busy, Some(Source::Reported)),
        "precondition: a fresh row opens on the 忙碌 register wrote — the word this batch is about",
    );

    let host = {
        let root = root.clone();
        let id = id.clone();
        let cmd = vec![agent.to_string_lossy().to_string()];
        std::thread::spawn(move || khor_node::host::host_main(root, id, (80, 24), cmd))
    };

    // 忙碌 is where the row already was, so seeing it proves nothing.
    // The two that follow are only reachable by reading the screen.
    let (_, source) = wait_for(&root, &id, State::Blocked)
        .expect("the permission prompt should have been read off the screen");
    assert_eq!(
        source,
        Some(Source::Screen),
        "the fallback must write itself down as a guess, or it outranks the vendor at the merge",
    );

    let (word, source) = wait_for(&root, &id, State::Idle).expect("and then the finished screen");
    assert_eq!((word, source), (State::Idle, Some(Source::Screen)));

    let _ = host.join();
    let _ = std::fs::remove_dir_all(&root);
}

/// The same host, the same kind, a command whose basename is not a
/// vendor: no table, no detector, and the row keeps the word it opened
/// with.
///
/// This is the control for the test above — it is what makes "the word
/// changed" mean "the screen was read" rather than "something moved".
/// It is also the promise made to anyone running a wrapper: khor does
/// not guess a table, so nothing about their session changes.
#[test]
fn a_command_that_names_no_vendor_is_left_exactly_as_it_was() {
    let root = std::env::temp_dir().join(format!("khor-notable-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();

    let bin = root.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    // Byte-for-byte the agent above, under a wrapper's name.
    let agent = bin.join("my-claude-wrapper");
    std::fs::write(&agent, AGENT).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&agent, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    let id = SessionId("tui/notable".into());
    let k = LiveKind::new(root.clone(), DeviceId([1; 32]));
    k.register(&id, "tui", "wrapped", None, None).unwrap();

    let host = {
        let root = root.clone();
        let id = id.clone();
        let cmd = vec![agent.to_string_lossy().to_string()];
        std::thread::spawn(move || khor_node::host::host_main(root, id, (80, 24), cmd))
    };

    // Long enough that the other test had reached 待批 and then 空闲.
    std::thread::sleep(Duration::from_secs(12));
    let (word, source) = read_state(&root, &id).unwrap();
    assert_eq!(
        (word, source),
        (State::Busy, Some(Source::Reported)),
        "no table matched, so nothing should have read this screen",
    );

    let _ = host.join();
    let _ = std::fs::remove_dir_all(&root);
}
