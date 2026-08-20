//! **The app and the browser must answer the same commands.**
//!
//! `khor_gui_core` has two callers and they are written down in two
//! places: the tauri handler list (`apps/gui/src-tauri`, what the
//! shipped app answers) and the dev bridge's match (`src/bin/bridge.rs`,
//! what browser verification answers). Nothing makes them agree, and
//! **each way of disagreeing is silent in the direction nobody is
//! looking**:
//!
//! - a command added only to the bridge — the smoke run goes green and
//!   the shipped app has no such command;
//! - a command added only to tauri — the app works and every browser
//!   check of it 404s, which reads as the feature being broken.
//!
//! This is the same shape as a document named by a string in two places
//! (`khor_sync::agents` over the wire, 批⑥): every in-process test stays
//! green while the thing never happens. There the fix was a real-wire
//! test; here the two tables are source, so the test reads the source.
//!
//! # Reading source is brittle, so the finders must fail loudly
//!
//! A regex that stops matching returns an empty set, and two empty sets
//! agree perfectly — the gate would go green precisely when it had
//! stopped working (`querySelectorAll` returning `[]` is the same trap).
//! So both finders assert they found a plausible number before anything
//! is compared, and that assertion is the one that fires when this file
//! rots rather than a confident "the tables match".

use std::collections::BTreeSet;
use std::path::PathBuf;

fn repo_file(rel: &str) -> String {
    let p: PathBuf = [env!("CARGO_MANIFEST_DIR"), rel].iter().collect();
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("reading {}: {e}", p.display()))
}

/// What the shipped app answers: the names inside `generate_handler![…]`.
fn tauri_commands() -> BTreeSet<String> {
    let src = repo_file("../src-tauri/src/lib.rs");
    let open = src
        .find("generate_handler![")
        .expect("the tauri handler list is spelled `generate_handler![` — if it moved, fix this finder");
    let start = open + "generate_handler![".len();
    let end = start + src[start..].find(']').expect("the handler list is closed by `]`");
    src[start..end]
        .split(',')
        .map(str::trim)
        .filter(|w| !w.is_empty())
        .map(str::to_owned)
        .collect()
}

/// What browser verification answers: the string arms of the bridge's
/// dispatch. Anchored at line start so a `"word"` inside a comment or a
/// nested expression cannot pass for an arm.
fn bridge_commands() -> BTreeSet<String> {
    let src = repo_file("src/bin/bridge.rs");
    src.lines()
        .filter_map(|line| {
            let t = line.trim_start();
            let rest = t.strip_prefix('"')?;
            let (word, after) = rest.split_once('"')?;
            after.trim_start().starts_with("=>").then(|| word.to_owned())
        })
        .filter(|w| !w.is_empty())
        .collect()
}

/// A command that lives on one side on purpose, with the reason.
///
/// Empty, because none has earned it yet — and it exists empty so that
/// the first one is **a decision somebody writes down** rather than an
/// omission that happens (`khor-cli`'s `NOT_IN_USAGE`, same judgment).
/// Delete a line here and that command goes red, which is the point.
const ONE_SIDED: &[(&str, &str)] = &[];

/// **Both tables name the same commands.**
///
/// Enumerated by difference rather than by count: two tables of equal
/// size can still disagree, and a count would call that a match.
#[test]
fn the_app_and_the_browser_answer_the_same_commands() {
    let tauri = tauri_commands();
    let bridge = bridge_commands();

    // The finders, before the comparison. Two empty sets agree, so
    // without this the gate would pass loudest at the moment it broke.
    assert!(tauri.len() > 20, "the tauri finder found {} commands — it is broken", tauri.len());
    assert!(bridge.len() > 20, "the bridge finder found {} commands — it is broken", bridge.len());

    let excused: BTreeSet<&str> = ONE_SIDED.iter().map(|(w, _)| *w).collect();
    let only_tauri: Vec<&str> =
        tauri.iter().map(String::as_str).filter(|w| !bridge.contains(*w) && !excused.contains(w)).collect();
    let only_bridge: Vec<&str> =
        bridge.iter().map(String::as_str).filter(|w| !tauri.contains(*w) && !excused.contains(w)).collect();

    assert!(
        only_tauri.is_empty(),
        "the app answers these and browser verification does not — every check of them 404s, \
         which reads as the feature being broken: {only_tauri:?}"
    );
    assert!(
        only_bridge.is_empty(),
        "browser verification answers these and the shipped app does not — the smoke run goes \
         green over a command nobody can call in the app: {only_bridge:?}"
    );
}

/// The excuse list may not name a command that is on both sides: an
/// excuse nobody needs is an excuse nobody reads, and it would quietly
/// exempt that command the day it really did drift.
#[test]
fn no_command_is_excused_from_a_rule_it_already_keeps() {
    let (tauri, bridge) = (tauri_commands(), bridge_commands());
    for (word, why) in ONE_SIDED {
        assert!(
            !(tauri.contains(*word) && bridge.contains(*word)),
            "`{word}` is on both sides, so its excuse ({why}) is stale — delete the line"
        );
    }
}
