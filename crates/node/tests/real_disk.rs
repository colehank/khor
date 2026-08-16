//! Manual checks against this machine's actual agent directories.
//!
//! `#[ignore]` because they only mean anything where a real claude or
//! codex has run, and because they are read-only observations of files
//! another program owns — never part of a green build, always available
//! with `cargo test -p khor-node --test real_disk -- --ignored`.
//!
//! Their job is the one thing fixtures structurally cannot do: catch the
//! day the fixtures become fiction. A recorded sample stays green forever
//! by construction, so the vendor changing its file layout is invisible
//! from inside the test suite. These read the real thing and fail when it
//! stops matching.

use std::collections::BTreeSet;
use std::path::PathBuf;

use khor_node::adaptor::{claude::Claude, Adaptor, Procs};

fn real_home() -> PathBuf {
    std::env::home_dir().expect("a home directory")
}

fn fixture(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vendors").join(rel)
}

fn keys_of(path: &PathBuf) -> BTreeSet<String> {
    let text = std::fs::read_to_string(path).expect("readable");
    let v: serde_json::Value = serde_json::from_str(&text).expect("json");
    v.as_object().expect("an object").keys().cloned().collect()
}

/// The fields khor reads out of a claude status file. If claude renames
/// one of these, discovery goes quiet — so this is the list worth
/// pinning against reality rather than against our own recording.
const DEPENDED_ON: [&str; 5] = ["pid", "sessionId", "startedAt", "status", "statusUpdatedAt"];

/// Every field khor depends on is present in every real status file on
/// this machine, and the committed fixture spells them the same way.
///
/// The fixture half matters as much as the real half: a fixture that
/// drifted into a private dialect would keep the unit tests green while
/// describing a file format that no longer exists anywhere.
#[test]
#[ignore]
fn the_fixture_and_this_machines_claude_agree_on_the_schema() {
    let dir = real_home().join(".claude/sessions");
    let real: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("no claude sessions dir at {}: {e}", dir.display()))
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("json"))
        .collect();
    assert!(
        !real.is_empty(),
        "no status files on this machine — the check would pass vacuously"
    );

    for path in &real {
        let keys = keys_of(path);
        for field in DEPENDED_ON {
            assert!(
                keys.contains(field),
                "{} has no {field:?}; khor reads it. Keys present: {keys:?}",
                path.display()
            );
        }
    }

    let fixture_keys = keys_of(&fixture("claude/.claude/sessions/4002.json"));
    for field in DEPENDED_ON {
        assert!(
            fixture_keys.contains(field),
            "the fixture dropped {field:?}; it no longer stands in for a real file"
        );
    }
    // Everything the fixture claims must actually occur out there, or the
    // fixture is describing a format of its own invention.
    let real_keys: BTreeSet<String> =
        real.iter().flat_map(|p| keys_of(p)).collect();
    let invented: Vec<&String> = fixture_keys.difference(&real_keys).collect();
    assert!(invented.is_empty(), "fields the fixture invented: {invented:?}");
}

/// Nothing running on this machine is a session khor cannot read.
///
/// This is the schema-drift alarm proper: a claude that renames a status
/// word, or reshapes the file, shows up here as a non-zero count while
/// every unit test stays green.
#[test]
#[ignore]
fn khor_can_read_every_live_claude_on_this_machine() {
    let claude = Claude::at(real_home().join(".claude"));
    let procs = Procs::snapshot();
    let sweep = claude.sweep(&procs);
    println!(
        "live claude sessions read: {}, unreadable: {}",
        sweep.rows.len(),
        sweep.unmapped
    );
    for row in &sweep.rows {
        // Titles only — the row's own contents are the user's business.
        println!("  {} {:?}", row.id().0, row.word);
    }
    assert!(
        !sweep.rows.is_empty(),
        "no live claude found; run one before trusting the count below"
    );
    assert_eq!(
        sweep.unmapped, 0,
        "a live claude session khor could not read — the vendor's format moved"
    );
}
