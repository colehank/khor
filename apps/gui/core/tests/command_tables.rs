//! **The app and the browser must answer the same commands.**
//!
//! `khor_gui_core`'s commands are written down in two places: the tauri
//! handler list (`apps/gui/src-tauri`, what the shipped app answers) and
//! `src/api.rs`'s match, which every HTTP skin dispatches through.
//! Nothing makes the two agree, and **each way of disagreeing is silent
//! in the direction nobody is looking**:
//!
//! - a command added only to `api.rs` — the smoke run goes green and
//!   the shipped app has no such command;
//! - a command added only to tauri — the app works and every browser
//!   check of it 404s, which reads as the feature being broken.
//!
//! **Why there are still only two places.** The match used to live in
//! the dev bridge, and the LAN listener a phone opens (批⑦) needed the
//! same 42 arms. A second copy would have been a third table — one
//! nothing on either side of this comparison checks, which is strictly
//! worse than the drift this gate was written for. So the arms moved
//! into the library, and every server that grows from here shares them
//! rather than adding a row to this file.
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

/// What every HTTP skin answers: the string arms of the shared
/// dispatch. Anchored at line start so a `"word"` inside a comment or a
/// nested expression cannot pass for an arm.
fn served_commands() -> BTreeSet<String> {
    let src = repo_file("src/api.rs");
    src.lines()
        .flat_map(|line| {
            let t = line.trim_start();
            if !t.starts_with('"') || !t.contains("=>") {
                return Vec::new();
            }
            // ⚠️ **An arm may name several commands**, and until
            // 2026-09-12 this finder could not see one that did. It read
            // `"word"` then required `=>` immediately after, so
            // `"move_page" | "hide_page" | … =>` matched nothing at all
            // — and five commands that have had a shared arm for months
            // were reported as missing from this side. The gate had been
            // red about them the whole time, which is the good half of
            // how it was found; the bad half is that the sentence it was
            // red with said the wrong thing about the code.
            //
            // Everything before the `=>`, split on `|`, each piece a
            // quoted word. A guard clause (`if x =>`) has no quotes and
            // drops out.
            let head = t.split("=>").next().unwrap_or("");
            head.split('|')
                .filter_map(|piece| {
                    let p = piece.trim();
                    p.strip_prefix('"')?.split_once('"').map(|(w, _)| w.to_owned())
                })
                .filter(|w| !w.is_empty())
                .collect()
        })
        .collect()
}

/// A command that lives on one side on purpose, with the reason.
///
/// It existed empty so that the first entry would be **a decision
/// somebody writes down** rather than an omission that happens
/// (`khor-cli`'s `NOT_IN_USAGE`, same judgment). Delete a line here and
/// that command goes red, which is the point.
const ONE_SIDED: &[(&str, &str)] = &[
    (
        "pick_and_send",
        "app-only by nature: it opens the DESKTOP's native file picker (tauri-plugin-dialog) — \
         a browser face has its own <input type=file> and uploads bytes instead (/api/upload), \
         so an HTTP arm for this would be a dialog on a machine nobody is looking at",
    ),
    // ── the menu's five, written down 2026-09-12 ──────────────────
    //
    // **One reason, five commands, and it is a fact about windows.**
    // The menu is a second native window of the app's own — the same
    // page told to be its other half (`api.ts` `isMenuFace`) — and it
    // exists because a popup drawn inside the document cannot float
    // over a child webview, which is a webview-over-webview problem a
    // browser face does not have. A browser draws its menus in its own
    // document, where they work.
    //
    // So these five are not「missing on the web」: the web face has no
    // second window to show, hide, size, read or pick from, and an
    // HTTP arm answering `null` would be a command that reports
    // success at doing nothing. The account is one line each so that
    // deleting a line turns that command red, which is the point of
    // this table.
    ("show_menu", "the menu is a second native window; a browser face draws menus in its own document"),
    ("hide_menu", "the other half of `show_menu`, and gone for the same reason"),
    ("size_menu", "resizes that window to its content; nothing on the web face has a window to resize"),
    ("menu_rows", "hands the menu window the rows it should draw — there is no such window here"),
    ("menu_pick", "carries a choice from that window back to the face that opened it"),
    // ── the window's own width, written down 2026-09-16 ───────────
    //
    // **The same fact about windows as the five above, from the other
    // end.** 「右上角的icon,都是点击后,在最右侧,拓宽窗口, 而不是挤占现有
    // 窗口」 (the user): opening the sessions column asks the app's
    // window to grow to the right instead of taking the room off the
    // file. A browser face is a page inside a window somebody else
    // placed — it has no window of its own to resize, and an HTTP arm
    // here would be a command that reports success at doing nothing.
    // `api.ts`'s `widenWindow` is a no-op outside the app for that
    // reason, and `check-face` asserts the silence.
    ("widen", "grows the app's own window when a column opens; a page in a browser has no window of its own"),
];

/// **Both tables name the same commands.**
///
/// Enumerated by difference rather than by count: two tables of equal
/// size can still disagree, and a count would call that a match.
#[test]
fn the_app_and_the_browser_answer_the_same_commands() {
    let tauri = tauri_commands();
    let served = served_commands();

    // The finders, before the comparison. Two empty sets agree, so
    // without this the gate would pass loudest at the moment it broke.
    //
    // **20 still means what it meant.** The arms moved file (批⑦) and
    // not one of them was added or dropped, so this floor is the same
    // distance below the real count it has always been — it catches a
    // finder that rotted, not a table that shrank. If a batch ever does
    // remove commands in bulk, this number is the thing to re-read.
    assert!(tauri.len() > 20, "the tauri finder found {} commands — it is broken", tauri.len());
    assert!(served.len() > 20, "the api.rs finder found {} commands — it is broken", served.len());

    let excused: BTreeSet<&str> = ONE_SIDED.iter().map(|(w, _)| *w).collect();
    let only_tauri: Vec<&str> =
        tauri.iter().map(String::as_str).filter(|w| !served.contains(*w) && !excused.contains(w)).collect();
    let only_served: Vec<&str> =
        served.iter().map(String::as_str).filter(|w| !tauri.contains(*w) && !excused.contains(w)).collect();

    assert!(
        only_tauri.is_empty(),
        "the app answers these and browser verification does not — every check of them 404s, \
         which reads as the feature being broken: {only_tauri:?}"
    );
    assert!(
        only_served.is_empty(),
        "every HTTP skin answers these and the shipped app does not — the smoke run goes green \
         over a command nobody can call in the app: {only_served:?}"
    );
}

/// The excuse list may not name a command that is on both sides: an
/// excuse nobody needs is an excuse nobody reads, and it would quietly
/// exempt that command the day it really did drift.
#[test]
fn no_command_is_excused_from_a_rule_it_already_keeps() {
    let (tauri, served) = (tauri_commands(), served_commands());
    for (word, why) in ONE_SIDED {
        assert!(
            !(tauri.contains(*word) && served.contains(*word)),
            "`{word}` is on both sides, so its excuse ({why}) is stale — delete the line"
        );
    }
}
