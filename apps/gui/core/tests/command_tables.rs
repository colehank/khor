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

/// The generated handler plus commands synchronously admitted to attachment queues.
fn tauri_commands() -> BTreeSet<String> {
    let src = repo_file("../src-tauri/src/lib.rs");
    let open = src.find("generate_handler![").expect(
        "the tauri handler list is spelled `generate_handler![` — if it moved, fix this finder",
    );
    let start = open + "generate_handler![".len();
    let end = start
        + src[start..]
            .find(']')
            .expect("the handler list is closed by `]`");
    let mut commands: BTreeSet<String> = src[start..end]
        .split(',')
        .map(str::trim)
        .filter(|w| !w.is_empty())
        .map(str::to_owned)
        .collect();
    if src.contains("native_io::dispatch(invoke)") {
        let native = repo_file("../src-tauri/src/native_io.rs");
        let marker = "const COMMANDS: &[&str] = &[";
        let start = native
            .find(marker)
            .expect("native dispatcher lost its command table")
            + marker.len();
        let end = start
            + native[start..]
                .find("];")
                .expect("native command table is not closed");
        let admitted: Vec<_> = native[start..end]
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| {
                s.strip_prefix('"')
                    .and_then(|s| s.strip_suffix('"'))
                    .expect("native command table must contain string literals")
            })
            .collect();
        assert!(
            !admitted.is_empty(),
            "native command finder found no commands"
        );
        for name in admitted {
            assert!(
                commands.insert(name.to_owned()),
                "duplicate native command {name}"
            );
        }
    }
    commands
}

/// What every HTTP skin answers: the string arms of the shared
/// dispatch. Only literal patterns at the dispatch match's own depth count;
/// formatting may split an or-pattern over several lines.
fn served_commands() -> BTreeSet<String> {
    command_patterns(&repo_file("src/api.rs"))
}

fn quoted_end(bytes: &[u8], start: usize) -> usize {
    let mut end = start + 1;
    while end < bytes.len() {
        match bytes[end] {
            b'\\' => end += 2,
            b'"' => return end + 1,
            _ => end += 1,
        }
    }
    bytes.len()
}

fn command_head(bytes: &[u8], mut at: usize) -> Option<(Vec<String>, usize)> {
    let mut names = Vec::new();
    loop {
        if bytes.get(at) != Some(&b'"') {
            return None;
        }
        let end = quoted_end(bytes, at);
        let name = bytes.get(at + 1..end.checked_sub(1)?)?;
        if name.is_empty()
            || !name
                .iter()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'_')
        {
            return None;
        }
        names.push(std::str::from_utf8(name).ok()?.to_owned());
        at = end;
        while bytes.get(at).is_some_and(u8::is_ascii_whitespace) {
            at += 1;
        }
        match bytes.get(at) {
            Some(b'|') => {
                at += 1;
                while bytes.get(at).is_some_and(u8::is_ascii_whitespace) {
                    at += 1;
                }
            }
            Some(b'=') if bytes.get(at + 1) == Some(&b'>') => return Some((names, at + 2)),
            _ => return None,
        }
    }
}

fn command_patterns(src: &str) -> BTreeSet<String> {
    let marker = "match cmd {";
    let start = src
        .find(marker)
        .expect("shared command dispatch must match cmd");
    let bytes = src.as_bytes();
    let mut at = start + marker.len();
    let mut depth = 1;
    let mut names = BTreeSet::new();
    while at < bytes.len() && depth > 0 {
        match bytes[at] {
            b'/' if bytes.get(at + 1) == Some(&b'/') => {
                at += 2;
                while at < bytes.len() && bytes[at] != b'\n' {
                    at += 1;
                }
            }
            b'"' => {
                if depth == 1 {
                    if let Some((head, end)) = command_head(bytes, at) {
                        names.extend(head);
                        at = end;
                        continue;
                    }
                }
                at = quoted_end(bytes, at);
            }
            b'{' | b'(' | b'[' => {
                depth += 1;
                at += 1;
            }
            b'}' | b')' | b']' => {
                depth -= 1;
                at += 1;
            }
            _ => at += 1,
        }
    }
    names
}

#[test]
fn command_finder_keeps_all_names_in_multiline_patterns() {
    let src = r#"fn answer(cmd: &str) {
        match cmd {
            "move_page" | "hide_page" | "close_page" | "show_page" | "page_step" | "page_deck"
            | "page_ground" => Ok("null"),
        }
    }"#;
    assert_eq!(
        command_patterns(src),
        BTreeSet::from(
            [
                "move_page",
                "hide_page",
                "close_page",
                "show_page",
                "page_step",
                "page_deck",
                "page_ground",
            ]
            .map(str::to_owned)
        )
    );
}

#[test]
fn command_finder_ignores_strings_and_patterns_outside_dispatch_arms() {
    let src = r#"const NOTE: &str = "not_a_command =>";
    fn answer(cmd: &str) {
        match cmd {
            // "comment" =>
            "real" => {
                let text = "body =>";
                match text { "nested" => (), _ => () }
            }
            "second" => "plain_body",
            _ => "fallback",
        }
    }
    fn other() { match "outside" { "unrelated" => () } }"#;
    assert_eq!(
        command_patterns(src),
        BTreeSet::from(["real".to_owned(), "second".to_owned()])
    );
}

/// A command that lives on one side on purpose, with the reason.
///
/// It existed empty so that the first entry would be **a decision
/// somebody writes down** rather than an omission that happens
/// (`khor-cli`'s `NOT_IN_USAGE`, same judgment). Delete a line here and
/// that command goes red, which is the point.
const ONE_SIDED: &[(&str, &str)] = &[
    (
        "file_preview_supported",
        "reports native Quick Look availability; HTTP has no native Office preview",
    ),
    (
        "file_preview_open",
        "presents native Quick Look for a completed app-owned file job",
    ),
    (
        "file_handoff_supported",
        "native device capability; HTTP faces use Web Share",
    ),
    (
        "file_handoff_open",
        "presents the native original-file activity sheet; HTTP faces have no app-owned native presenter",
    ),
    (
        "file_handoff_save",
        "presents the native Save panel or original-file activity sheet for a completed app-owned handoff; HTTP faces use their browser file picker",
    ),
    (
        "page_steps",
        "reads the app-owned native WebView's actual back/forward history; HTTP faces have no native page handle and pageSteps returns null there",
    ),
    (
        "page_sheet",
        "coordinates the native iOS webview scroll recognizer with the content sheet; HTTP faces have no separate native webview",
    ),
    (
        "window_appearance_support",
        "reports native desktop background capabilities; browser windows belong to the browser",
    ),
    (
        "set_window_appearance",
        "sets the main native window background and OS material, which an HTTP face cannot control",
    ),
    (
        "save_terminal_palette",
        "opens the native save-file panel for a local palette; browser faces use a download",
    ),
    // ── a picked file's bytes, and the phone's edit menu, 2026-10-07 ──
    //
    // **Served, just not as an arm.** A picked file arrives as raw bytes
    // (item 60 part 3), so the bridge answers `/term_drop_stage?name=…`
    // on its own route before the JSON arms (`lib.rs`), where this finder
    // does not look. And the system edit menu's items over a terminal
    // selection (item 54) belong to the phone's native menu, which a
    // browser tab does not have.
    (
        "term_drop_stage",
        "a picked file's raw bytes: the bridge serves it on its own route ahead of the JSON arms",
    ),
    (
        "term_drop_stage_into",
        "folder child bytes use a raw upload route ahead of the JSON arms, like term_drop_stage",
    ),
    (
        "term_pick_folders",
        "opens the native directory picker; browser faces use their own webkitdirectory file input",
    ),
    (
        "edit_menu",
        "the phone's own edit menu over a selection, which a browser tab does not have",
    ),
    // Item 73's keyboard-up selection shows and hides the same native
    // menu at a point the page names: the same menu, so the same reason.
    (
        "edit_menu_at",
        "the phone's own edit menu shown over khor's selection, which a browser tab does not have",
    ),
    (
        "edit_menu_hide",
        "hides the phone's own edit menu, which a browser tab does not have",
    ),
    // ── the root's words, written down 2026-09-30 (docs/CONNECT.md 身份)
    //
    // **A secret, not a window** — the second entry here that is about
    // the machine. The 24 words restore the network's root, and a root
    // outlives every web key: a link that leaked once could otherwise
    // hand out a key that signs machines in for good. The app shows them
    // on the holder's own screen when somebody presses 「显示…」 on
    // 「…」 → 安全 (on demand since 2026-10-01; at first start before
    // that); anywhere else the way to them is `khor root words <本机名>`
    // in a terminal on the holder.
    (
        "root_words",
        "the root's recovery words, shown on the holder's own screen; a browser face is reached \
         with a key in a link, and the words outlive every web key — `khor root words` on the \
         holder is the way to them from anywhere else",
    ),
    // ── the menu bar's own word back, 2026-09-30 ──────────────────────
    //
    // **The page telling the app's menu which items apply right now**
    // (⌘W with something to close, two tabs to step between). It is the
    // twin of the menu's five below: only macOS draws a menu bar, and a
    // browser tab has none whose items could be enabled or greyed. The
    // page already sends it only in the app (`api.ts` checks `inApp`).
    (
        "menu_state",
        "the app's own menu bar, which a browser tab does not have; the page only sends it in the app",
    ),
    // ── the window's own three, 2026-09-23 (docs/SHELL.md 第十轮) ────
    //
    // **A fact about windows, like the menu's five below.** Windows is
    // the one platform that draws khor's three window buttons itself,
    // and what they press is this window — a browser tab has no window
    // of its own to minimise, maximise or close, and an HTTP arm here
    // would be a command reporting success at doing nothing to a window
    // nobody is looking at.
    (
        "window_minimize",
        "the app's own window; a browser tab has no window of its own to minimise",
    ),
    (
        "window_maximize_toggle",
        "the app's own window; a browser tab has none to maximise or restore",
    ),
    (
        "window_close",
        "the app's own window; a browser tab is closed by the browser",
    ),
    // ── 2026-09-23 (docs/SHELL.md 第十一轮) ────────────────────────
    //
    // **A measurement of one window's own three buttons.** It asks
    // AppKit how wide macOS draws its trio on *this* window, so the
    // title row can keep that much clear (`--traffic`). A browser tab
    // has no such buttons over khor's row at all, and an HTTP arm would
    // answer a number about a window nobody is looking at — which the
    // face would then hold empty for nothing.
    (
        "traffic_room",
        "the app's own window: what macOS's three buttons take of its title row, measured from that window; \
         a browser face has no window buttons over the row and keeps nothing clear",
    ),
    (
        "pick_paths",
        "app-only by nature: it opens the native file picker (tauri-plugin-dialog) for the \
         broadcast's ＋ — a browser face has its own <input type=file> and stages bytes instead \
         (/api/upload), so an HTTP arm for this would be a dialog on a machine nobody is looking at. \
         ⚠️ 墓碑 (STORE-BRIEF phase 5): `pick_and_send`, which also sent what it picked",
    ),
    // ── round CHAT, 2026-10-03 ────────────────────────────────────
    //
    // **Not missing on the web: served there by a route of its own.**
    // A conversation's file arrives as raw bytes, which `api::answer`'s
    // JSON arguments cannot carry; the bridge and the web face answer it
    // beside `upload`, from the request body (`bin/bridge.rs`,
    // `khor_web`), and both reach the same `chat::chat_put`.
    (
        "chat_put",
        "raw bytes, not JSON: the bridge and the web face serve it as a body route beside /upload \
         (`chat::chat_put`), so this table's api.rs finder cannot see it",
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
    (
        "show_menu",
        "the menu is a second native window; a browser face draws menus in its own document",
    ),
    (
        "hide_menu",
        "the other half of `show_menu`, and gone for the same reason",
    ),
    (
        "size_menu",
        "resizes that window to its content; nothing on the web face has a window to resize",
    ),
    (
        "menu_rows",
        "hands the menu window the rows it should draw — there is no such window here",
    ),
    (
        "menu_pick",
        "carries a choice from that window back to the face that opened it",
    ),
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
    (
        "widen",
        "grows the app's own window when a column opens; a page in a browser has no window of its own",
    ),
    // Its other half since UX-46 (2026-10-07): the window keeps the
    // account of what it grew by and gives that back, by column.
    (
        "unwiden",
        "gives back what `widen` took for a column; a page in a browser grew no window to give back",
    ),
    // ── the editor's own window, written down 2026-09-17 ──────────
    //
    // **The third fact about windows on this list, and the plainest.**
    // 「VSCODE做成弹出式的, 弹出一个窗口显示vscode」 (the user, 第六轮
    // 第 46 条): the app builds a second native window and points it at
    // that machine's editor. A browser face opens a **named tab**
    // instead — `window.open(url, "khor-code:<machine>")`, which is the
    // same promise (one editor, one place, pressing again brings it
    // forward) spelled in the only vocabulary a page has. So this is
    // not a command the web face is missing: it is a command the web
    // face has no use for, because the browser already answers it.
    // ── the lock screen's other key, written down 2026-09-19 ──────
    //
    // **A fact about the machine, not about windows** — the first entry
    // on this list that is not. 「macOS app：系统验证（Touch ID / 登录
    // 密码，`LocalAuthentication`）」 (docs/MORE.md 安全): the app puts
    // the system's own sheet up and is told yes or no. A browser has no
    // such thing to ask, and an HTTP arm here could only ever answer
    // `false` — a command that reports failure at a thing it never
    // attempted.
    //
    // It is deliberately an *extra*: `khor_node::lock` refuses to turn
    // the lock on until a PIN is set, because the PIN is the key every
    // face has and a phone opening the web face has nothing else. So
    // this being app-only costs nobody a way in.
    (
        "system_unlock",
        "puts up the system's own Touch ID / login-password sheet (LocalAuthentication); \
         a browser has nothing to ask, and the PIN is the key every face has",
    ),
    // ── the window's own appearance, written down 2026-09-20 ─────
    //
    // **The fourth fact about windows on this list.** 外观 (批 D2) keys
    // every stylesheet off `:root[data-theme]`, which covers the
    // document and nothing else: the ground behind the webview is
    // painted from Rust before any HTML exists, the title bar is
    // AppKit's, and each editor window is khor's own. A browser face
    // has none of the three — the page it is in was placed by somebody
    // else — so an HTTP arm here could only report success at doing
    // nothing, and `api.ts` makes it a no-op outside the app for
    // `widen`'s reason.
    (
        "set_window_theme",
        "gives the app's own window and its title bar the resolved theme; a page in a browser \
         has no window, no title bar and no editor windows of its own",
    ),
    (
        "open_code_window",
        "opens the editor in a second native window; a browser face opens a named tab with \
         `window.open` and never asks khor for a window",
    ),
    // ── a bell, felt (roadmap 4.2) ─────
    (
        "bell_haptic",
        "taps the phone's haptic engine; a browser has no haptics to reach, and the pane \
         flashes its edge there as everywhere",
    ),
    // ── the settings window (UX-10) ─────
    (
        "open_settings",
        "opens settings in a native window of their own; a browser face shows the same pages in \
         its one page",
    ),
    (
        "name_settings",
        "titles that native window after its pane; a browser face has no window to title",
    ),
    // ── the cursor joystick, felt (item 23) ─────
    (
        "joy_haptic",
        "ticks the phone's haptic engine as the cursor stick sends arrows; a browser has no \
         haptic engine to reach",
    ),
    // ── haptics with a meaning (UX-10) ─────
    (
        "haptic",
        "plays the phone's selection, success or warning pattern; a browser has no haptic \
         engine to reach",
    ),
    // ── a line into the app's log (item 18 A) ─────
    (
        "face_note",
        "writes a line to the app's own stderr and the notes `khor doctor` reads (item 33), for \
         a device nobody watches; a browser face has its own console and no app log",
    ),
    // ── a finished command, said by the system (roadmap 2.4) ─────
    (
        "notify_finished",
        "posts a notification through the operating system's notification centre; a page in \
         a browser is not an app the system notifies for, and `lib/finished` catches the miss",
    ), // ── 全网谁够得着 (round ROAD-1) ─────
    (
        "reach_probe",
        "has every machine in the network open a connection to an address of the caller's \
         choosing; the lead kept it to the app (2026-10-08), so a browser holding the web \
         face's key cannot turn the fleet into a prober",
    ),
    // ── 添加机器 over SSH (round join-d) ─────
    (
        "add_start",
        "carries a password or a key's passphrase into the OS secret store of the machine that \
         runs ssh, and installs khor on another computer; the app only, like the root's words",
    ),
    (
        "add_read",
        "reads the progress of an add only the app can start (`add_start`)",
    ),
    (
        "promote_start",
        "installs khor on an SSH host over this machine's ssh; the app only, like `add_start`",
    ),
    (
        "ssh_config_hosts",
        "lists the Host names of this machine's ssh config for the app's add form; a browser \
         holding the web face's key has no business reading the owner's ssh config",
    ),
    (
        "open_copy_id",
        "opens a terminal on this machine running ssh-copy-id, which asks for a password in it; \
         the add form that offers it is the app's",
    ),
    (
        "pick_key_file",
        "opens the system's file picker on this machine; a page in a browser has no such picker",
    ),
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
    assert!(
        tauri.len() > 20,
        "the tauri finder found {} commands — it is broken",
        tauri.len()
    );
    assert!(
        served.len() > 20,
        "the api.rs finder found {} commands — it is broken",
        served.len()
    );

    let excused: BTreeSet<&str> = ONE_SIDED.iter().map(|(w, _)| *w).collect();
    let only_tauri: Vec<&str> = tauri
        .iter()
        .map(String::as_str)
        .filter(|w| !served.contains(*w) && !excused.contains(w))
        .collect();
    let only_served: Vec<&str> = served
        .iter()
        .map(String::as_str)
        .filter(|w| !tauri.contains(*w) && !excused.contains(w))
        .collect();

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
