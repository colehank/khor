//! The tauri skin: every command is one call into khor-gui-core, which
//! is one call into khor-node — CLI and GUI stay equivalent because they
//! are the same functions (docs/KHOR.md 三条地基).

use khor_gui_core::{DeviceRow, FaceChoices, SessionRow};
use khor_node::Node;

/// `by` is the arrangement key (`khor_node::list::Arrange`). The window
/// passes the one the user last chose; it is that screen's own state,
/// not something the network holds an opinion about.
#[tauri::command]
fn sessions(by: String) -> Result<Vec<SessionRow>, String> {
    khor_gui_core::list_sessions(&Node::root_from_env(), &by)
}

/// The day strips, on their own beat rather than riding the row list
/// (`khor_gui_core::day_strips`). `cells` is how many the strip has room
/// for, measured by the screen.
#[tauri::command]
fn day_strips(
    cells: u32,
) -> Result<std::collections::BTreeMap<String, Vec<khor_gui_core::DayRun>>, String> {
    khor_gui_core::day_strips(&Node::root_from_env(), cells)
}

/// The workspace landing's whole list, off this machine's own table
/// (`khor_gui_core::workspaces::workspaces`) — no machine is asked.
#[tauri::command]
fn workspaces() -> Result<Vec<khor_gui_core::workspaces::WorkspaceRow>, String> {
    khor_gui_core::workspaces::workspaces(&Node::root_from_env())
}

// ── the browsing tabs (docs/SHELL.md 第 2 条) ────────────────────────
//
// **Five thin shells over `khor_gui_core::tabs`, which is five thin
// shells over `khor_node::tabs`** — the same functions `khor tabs`
// calls, which is the whole reason the list moved out of the webview's
// own storage. Nothing here decides anything: not which tab takes the
// front after a close, not what an id looks like, not what an
// unreadable file means.
//
// Every write answers with the list as it now stands. A caller that had
// to ask again to learn what its own write did would be two reads out
// of step for the length of a round trip, and the column would flicker
// through the old order on every close.

/// This screen's tabs of one kind (`page` today).
#[tauri::command]
fn tabs_list(kind: String) -> khor_gui_core::tabs::TabsOpen {
    khor_gui_core::tabs::list(&Node::root_from_env(), &kind)
}

/// Opens one and puts it in front; answers its id. Both places absent
/// is the fresh tab the 「+」 mints.
#[tauri::command]
fn tabs_open(kind: String, device: Option<String>, url: Option<String>) -> Result<String, String> {
    khor_gui_core::tabs::open(
        &Node::root_from_env(),
        &kind,
        device.as_deref().unwrap_or(""),
        url.as_deref().unwrap_or(""),
    )
}

/// Points an open tab somewhere and names it.
#[tauri::command]
fn tabs_go(
    kind: String,
    id: String,
    device: Option<String>,
    url: Option<String>,
    title: Option<String>,
) -> Result<khor_gui_core::tabs::TabsOpen, String> {
    khor_gui_core::tabs::go(
        &Node::root_from_env(),
        &kind,
        &id,
        device.as_deref().unwrap_or(""),
        url.as_deref().unwrap_or(""),
        title.as_deref().unwrap_or(""),
    )
}

#[tauri::command]
fn tabs_focus(kind: String, id: String) -> Result<khor_gui_core::tabs::TabsOpen, String> {
    khor_gui_core::tabs::focus(&Node::root_from_env(), &kind, &id)
}

#[tauri::command]
fn tabs_close(kind: String, id: String) -> Result<khor_gui_core::tabs::TabsOpen, String> {
    khor_gui_core::tabs::close(&Node::root_from_env(), &kind, &id)
}

/// Pins or unpins one (docs/SHELL.md 第二轮) — `khor tabs pin|unpin`.
#[tauri::command]
fn tabs_pin(kind: String, id: String, on: bool) -> Result<khor_gui_core::tabs::TabsOpen, String> {
    khor_gui_core::tabs::pin(&Node::root_from_env(), &kind, &id, on)
}

/// The whole column at once — the one gesture that cannot be spelled as
/// a close and an open: a drag that reorders.
#[tauri::command]
fn tabs_save(
    kind: String,
    open: khor_gui_core::tabs::TabsOpen,
) -> Result<khor_gui_core::tabs::TabsOpen, String> {
    khor_gui_core::tabs::save_all(&Node::root_from_env(), &kind, &open)
}

/// Declares a directory a workspace; `machine` absent is this one.
#[tauri::command]
async fn ws_add(machine: Option<String>, path: String) -> Result<(), String> {
    khor_gui_core::workspaces::ws_add(
        &Node::root_from_env(),
        machine.as_deref().unwrap_or(""),
        &path,
    )
    .await
}

/// Takes it off the list. The directory itself is not touched.
/// **Keyed by `device`** — a row whose machine has left the table has
/// no name, and it is the row that most needs taking off.
#[tauri::command]
fn ws_rm(device: String, path: String) -> Result<(), String> {
    khor_gui_core::workspaces::ws_rm(&Node::root_from_env(), &device, &path)
}

#[tauri::command]
fn ws_pin(device: String, path: String, on: bool) -> Result<(), String> {
    khor_gui_core::workspaces::ws_pin(&Node::root_from_env(), &device, &path, on)
}

/// When that directory on that machine last changed
/// (`khor_gui_core::workspaces::touched`).
#[tauri::command]
async fn touched(machine: Option<String>, path: String) -> Result<i64, String> {
    khor_gui_core::workspaces::touched(
        &Node::root_from_env(),
        machine.as_deref().unwrap_or(""),
        &path,
    )
    .await
}

/// One file off a machine, for reading (`khor_gui_core::files::read_text`).
#[tauri::command]
async fn read_text(
    machine: String,
    path: String,
) -> Result<khor_gui_core::files::FileText, String> {
    khor_gui_core::files::read_text(&Node::root_from_env(), &machine, &path).await
}

/// 置顶一个账号 (`khor_gui_core::pin_account`).
///
/// ⚠️ **Missing from this list until 2026-09-12**, so the 額度 landing's
/// pin did nothing at all in the shipped app while working perfectly in
/// a browser. `command_tables` is the gate for exactly this and it was
/// blind to it: its finder could not read an alternation arm, and these
/// two were hidden behind that same bug.
#[tauri::command]
fn pin_account(id: String, on: bool) -> Result<(), String> {
    khor_gui_core::pin_account(&Node::root_from_env(), &id, on)
}

/// 给一个账号改名 (`khor_gui_core::rename_account`) — 额度列表里双击名字。
#[tauri::command]
fn rename_account(id: String, name: String) -> Result<(), String> {
    khor_gui_core::rename_account(&Node::root_from_env(), &id, &name)
}

/// 编辑一个账号 (`khor_gui_core::update_account`). `key` absent means
/// leave the stored one alone — an edit of the name must not make
/// somebody paste a secret again.
#[tauri::command]
fn update_account(
    id: String,
    name: String,
    base_url: Option<String>,
    key: Option<String>,
    budget: Option<f64>,
    unit: Option<String>,
) -> Result<(), String> {
    khor_gui_core::update_account(
        &Node::root_from_env(),
        &id,
        &name,
        base_url.as_deref(),
        key.as_deref(),
        budget,
        unit.as_deref(),
    )
}

/// One file shaped for a screen (`khor_gui_core::files::preview`).
#[tauri::command]
async fn preview(machine: String, path: String) -> Result<khor_gui_core::files::Preview, String> {
    khor_gui_core::files::preview(&Node::root_from_env(), &machine, &path).await
}

/// Whether a machine has VS Code (`khor_gui_core::code::here`).
#[tauri::command]
async fn code_here(machine: String) -> Result<khor_gui_core::code::Editor, String> {
    khor_gui_core::code::here(&Node::root_from_env(), &machine).await
}

/// Where to point a webview at a machine's VS Code
/// (`khor_gui_core::code::open`).
#[tauri::command]
async fn code_open(machine: String, dir: String) -> Result<khor_gui_core::code::CodeAt, String> {
    khor_gui_core::code::open(&Node::root_from_env(), &machine, &dir).await
}

/// Stops it (`khor_gui_core::code::stop`).
#[tauri::command]
async fn code_stop(machine: String) -> Result<(), String> {
    khor_gui_core::code::stop(&Node::root_from_env(), &machine).await
}

/// Saves a whole file back (`khor_gui_core::files::write_text`).
#[tauri::command]
async fn write_text(
    machine: String,
    path: String,
    was: String,
    text: String,
) -> Result<khor_gui_core::files::Wrote, String> {
    khor_gui_core::files::write_text(&Node::root_from_env(), &machine, &path, &was, &text).await
}

/// The three sides of a refused save, merged
/// (`khor_gui_core::files::merge_text`). Writes nothing — the save that
/// follows is `write_text` like any other.
#[tauri::command]
fn merge_text(base: String, mine: String, theirs: String) -> khor_gui_core::files::Merged {
    khor_gui_core::files::merge_text(&base, &mine, &theirs)
}

/// One checkout's marks and branch (`khor_gui_core::git::status`).
#[tauri::command]
async fn git_status(machine: String, dir: String) -> Result<khor_gui_core::git::GitStatus, String> {
    khor_gui_core::git::status(&Node::root_from_env(), &machine, &dir).await
}

/// Whether that directory is a checkout (`khor_gui_core::git::checkout`).
#[tauri::command]
async fn checkout(machine: String, dir: String) -> Result<bool, String> {
    khor_gui_core::git::checkout(&Node::root_from_env(), &machine, &dir).await
}

/// Where one file differs from HEAD (`khor_gui_core::git::diff`).
#[tauri::command]
async fn git_diff(machine: String, path: String) -> Result<khor_gui_core::git::GitDiff, String> {
    khor_gui_core::git::diff(&Node::root_from_env(), &machine, &path).await
}

#[tauri::command]
async fn quota_board() -> Result<Vec<khor_gui_core::AccountRing>, String> {
    khor_gui_core::quota_board(&Node::root_from_env()).await
}

#[tauri::command]
fn register_account(
    name: String,
    provider: String,
    kind: String,
    base_url: Option<String>,
    key: String,
    budget: Option<f64>,
) -> Result<String, String> {
    khor_gui_core::register_account(
        &Node::root_from_env(),
        &name,
        &provider,
        &kind,
        base_url.as_deref(),
        &key,
        budget,
    )
}

#[tauri::command]
fn remove_account(id: String) -> Result<(), String> {
    khor_gui_core::remove_account(&Node::root_from_env(), &id)
}

#[tauri::command]
async fn devices() -> Result<Vec<DeviceRow>, String> {
    khor_gui_core::list_devices(&Node::root_from_env()).await
}

/// The machine's process top and the CPU day its serve recorded.
///
/// **Asked while a card is open, never on the device beat** — the top
/// is a reading somebody is looking at, and `machine` empty means this
/// one (`Node::load_on`). `top` is the row count the card has room for:
/// required, the `cells` rule — a default here would be this side
/// sizing a table it cannot see.
#[tauri::command]
async fn load(
    machine: String,
    top: u32,
    by: Option<String>,
    desc: Option<bool>,
) -> Result<khor_node::Load, String> {
    // The same words the HTTP arm takes, and the same refusal for one
    // it does not know (`api.rs` carries the reasoning): a column read
    // as the default would look exactly like a working sort.
    let by = match by.as_deref() {
        None | Some("") | Some("busiest") => khor_node::TopBy::Busiest,
        Some("pid") => khor_node::TopBy::Pid,
        Some("name") => khor_node::TopBy::Name,
        Some("user") => khor_node::TopBy::User,
        Some("cpu") => khor_node::TopBy::Cpu,
        Some("mem") => khor_node::TopBy::Mem,
        Some("gpu") => khor_node::TopBy::Gpu,
        Some("vram") => khor_node::TopBy::Vram,
        Some("time") => khor_node::TopBy::Time,
        Some(other) => return Err(format!("no such column: {other}")),
    };
    khor_gui_core::load(
        &Node::root_from_env(),
        &machine,
        top,
        by,
        desc.unwrap_or(true),
    )
    .await
}

/// **Blocking, and the first call can take seconds**: reading every
/// transcript on this machine is 18 s cold (`khor_node::usage`). Tauri
/// runs commands on a worker thread rather than on the UI thread, so
/// this does not freeze the window — and every call after the first is
/// a walk of directory entries.
#[tauri::command]
fn usage() -> Result<khor_node::Usage, String> {
    khor_gui_core::usage(&Node::root_from_env())
}

/// The subscription's remaining windows. Answers a shape carrying its own
/// trouble rather than a `Result`, because "khor could not ask" is one of
/// the things this reading has to be able to say out loud — five of them,
/// each with a different next step (`khor_node::quota`), and a `Result`
/// would flatten them into one error string on the way to the face.
#[tauri::command]
async fn quota() -> khor_gui_core::QuotaAnswer {
    khor_gui_core::quota().await
}

#[tauri::command]
fn seen(id: String) -> Result<(), String> {
    khor_gui_core::seen(&Node::root_from_env(), &id)
}

#[tauri::command]
fn close_session(id: String) -> Result<(), String> {
    khor_gui_core::close_session(&Node::root_from_env(), &id)
}

#[tauri::command]
fn rename_session(id: String, title: String) -> Result<(), String> {
    khor_gui_core::rename_session(&Node::root_from_env(), &id, &title)
}

#[tauri::command]
fn rename_machine(id: String, name: String) -> Result<String, String> {
    khor_gui_core::rename_machine(&Node::root_from_env(), &id, &name)
}

#[tauri::command]
fn relays() -> Result<khor_gui_core::RelayChoices, String> {
    khor_gui_core::relays(&Node::root_from_env())
}

#[tauri::command]
fn set_relays(
    relays: Vec<khor_gui_core::RelayWant>,
    only_mine: bool,
) -> Result<khor_gui_core::RelayChoices, String> {
    khor_gui_core::set_relays(&Node::root_from_env(), &relays, only_mine)
}

#[tauri::command]
async fn road_share() -> Result<khor_gui_core::RoadShare, String> {
    khor_gui_core::road_share(&Node::root_from_env()).await
}

#[tauri::command]
fn tell(machine: String, text: String) -> Result<(), String> {
    khor_gui_core::tell(&Node::root_from_env(), &machine, &text)
}

#[tauri::command]
fn pin_session(id: String, on: bool) -> Result<(), String> {
    khor_gui_core::pin_session(&Node::root_from_env(), &id, on)
}

#[tauri::command]
fn pin_device(machine: String, on: bool) -> Result<(), String> {
    khor_gui_core::pin_device(&Node::root_from_env(), &machine, on)
}

#[tauri::command]
fn forget_device(machine: String) -> Result<String, String> {
    khor_gui_core::forget_device(&Node::root_from_env(), &machine)
}

#[tauri::command]
fn set_watch(machine: String, paths: Vec<String>) -> Result<Vec<String>, String> {
    khor_gui_core::set_watch(&Node::root_from_env(), &machine, &paths)
}

/// `machine` is a device id and **empty is this one** — 「头像设置这, 要
/// 支持对每台机器的修改」 (the user, 2026-09-20), so the editor opens on
/// whichever machine was pressed rather than always on this one.
#[tauri::command]
fn face_choices(machine: String) -> Result<FaceChoices, String> {
    khor_gui_core::face_choices(&Node::root_from_env(), &machine)
}

/// An axis left out stays where it is — the same shape as `khor avatar`'s
/// flags, because they are the same call underneath.
#[tauri::command]
fn restyle(
    machine: String,
    colors: Option<Vec<String>>,
    variant: Option<String>,
    shape: Option<String>,
) -> Result<(), String> {
    khor_gui_core::restyle(&Node::root_from_env(), &machine, colors, variant, shape)
}

/// The three hook commands are about **this machine's** claude: the
/// settings file they read and write is rooted at this node's vendor
/// home, so there is no argument naming a machine and no way to point
/// one of these at somebody else's.
#[tauri::command]
fn hooks_state() -> Result<khor_gui_core::HooksState, String> {
    khor_gui_core::hooks_state(&Node::root_from_env())
}

#[tauri::command]
fn install_hooks() -> Result<khor_gui_core::HooksState, String> {
    khor_gui_core::install_hooks(&Node::root_from_env())
}

#[tauri::command]
fn uninstall_hooks() -> Result<khor_gui_core::HooksState, String> {
    khor_gui_core::uninstall_hooks(&Node::root_from_env())
}

/// The six chat commands are one attachment registry in gui-core: the
/// window polls for frames because that is the shape both skins can
/// serve (`khor_gui_core::chat` module head).
#[tauri::command]
fn chat_open(id: String) -> Result<bool, String> {
    khor_gui_core::chat::chat_open(&Node::root_from_env(), &id)
}

#[tauri::command]
fn chat_poll(id: String, since: u64) -> Result<khor_gui_core::chat::ChatBatch, String> {
    khor_gui_core::chat::chat_poll(&id, since)
}

#[tauri::command]
fn chat_say(id: String, text: String) -> Result<(), String> {
    khor_gui_core::chat::chat_say(&id, &text)
}

#[tauri::command]
fn chat_answer(id: String, ask: u64, option: Option<String>) -> Result<(), String> {
    khor_gui_core::chat::chat_answer(&id, ask, option)
}

/// A page's site icon, through the page's own exit (docs/SHELL.md 第四轮).
#[tauri::command]
async fn page_icon(url: String, proxy: Option<String>) -> Result<String, String> {
    khor_gui_core::web::page_icon(&Node::root_from_env(), &url, proxy.as_deref().unwrap_or(""))
        .await
}

/// Every site icon kept on this disk (`khor_gui_core::web::page_icons`).
#[tauri::command]
fn page_icons() -> std::collections::BTreeMap<String, String> {
    khor_gui_core::web::page_icons(&Node::root_from_env())
}

#[cfg(not(target_os = "ios"))]
#[tauri::command]
fn open_link(url: String) -> Result<(), String> {
    khor_gui_core::web::open_link(&url)
}

/// The phone's half (docs/IOS.md 第二步): **the system opens the page**,
/// in whatever browser the person uses — an app's sandbox has no program
/// to hand a URL to, and the desktop road's `xdg-open` came back as a raw
/// OS error. The same whitelist as the desktop (`web::page_only`), asked
/// before anything leaves; UIKit wants the main thread, so the hand-over
/// runs there and this answers once it is queued.
#[cfg(target_os = "ios")]
#[tauri::command]
fn open_link(app: tauri::AppHandle, url: String) -> Result<(), String> {
    khor_gui_core::web::page_only(&url)?;
    app.run_on_main_thread(move || {
        use objc2::MainThreadMarker;
        use objc2_foundation::{NSDictionary, NSString, NSURL};
        use objc2_ui_kit::UIApplication;
        let Some(mtm) = MainThreadMarker::new() else {
            return;
        };
        let Some(page) = NSURL::URLWithString(&NSString::from_str(&url)) else {
            return;
        };
        // SAFETY: an empty options dictionary is the documented default,
        // and no completion handler is asked for.
        unsafe {
            UIApplication::sharedApplication(mtm).openURL_options_completionHandler(
                &page,
                &NSDictionary::new(),
                None,
            );
        }
    })
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn chat_stop(id: String) -> Result<(), String> {
    khor_gui_core::chat::chat_stop(&id)
}
#[tauri::command]
fn chat_set_mode(id: String, mode: String) -> Result<(), String> {
    khor_gui_core::chat::chat_set_mode(&id, &mode)
}

#[tauri::command]
fn chat_set_option(id: String, option: String, value: serde_json::Value) -> Result<(), String> {
    khor_gui_core::chat::chat_set_option(&id, &option, &value)
}

#[tauri::command]
fn chat_replay(id: String) -> Result<(), String> {
    khor_gui_core::chat::chat_replay(&id)
}

#[tauri::command]
fn chat_leave(id: String) -> Result<(), String> {
    khor_gui_core::chat::chat_leave(&id)
}

/// A discovered session's recorded past — the vendor's transcript in
/// replay-shaped frames (`khor_gui_core::chat::history`).
#[tauri::command]
fn history(id: String) -> Result<Vec<khor_gui_core::chat::ChatFrame>, String> {
    khor_gui_core::chat::history(&Node::root_from_env(), &id)
}

/// A discovered conversation, **followed**: whatever its transcript has
/// said since `from`, and where to continue. The read-only half of
/// watching a session that is running in its own terminal.
#[tauri::command]
fn history_follow(id: String, from: u64) -> Result<khor_gui_core::chat::Followed, String> {
    khor_gui_core::chat::history_follow(&Node::root_from_env(), &id, from)
}

/// One machine's directory, for the files landing. Async because the
/// machine may be far away (the same reason `pair` is).
#[tauri::command]
async fn ls(machine: String, path: String) -> Result<khor_gui_core::files::DirListing, String> {
    khor_gui_core::files::ls(&Node::root_from_env(), &machine, &path).await
}

/// Takes a file into this machine's downloads; answers where it landed.
#[tauri::command]
async fn pull(machine: String, path: String) -> Result<String, String> {
    khor_gui_core::files::pull(&Node::root_from_env(), &machine, &path).await
}

/// Takes a path off a machine's disk. This machine's goes to the
/// recycle bin; every other machine's is deleted (`files::remove`).
#[tauri::command]
async fn remove(machine: Option<String>, path: String) -> Result<(), String> {
    khor_gui_core::files::remove(
        &Node::root_from_env(),
        machine.as_deref().unwrap_or(""),
        &path,
    )
    .await
}

/// One new directory on a machine (`files::mkdir`). One level, and a
/// name already taken is refused.
#[tauri::command]
async fn mkdir(machine: Option<String>, path: String) -> Result<(), String> {
    khor_gui_core::files::mkdir(
        &Node::root_from_env(),
        machine.as_deref().unwrap_or(""),
        &path,
    )
    .await
}

/// One new empty file on a machine (`files::touch`). A name already
/// taken is refused and the file wearing it is not emptied.
#[tauri::command]
async fn touch(machine: Option<String>, path: String) -> Result<(), String> {
    khor_gui_core::files::touch(
        &Node::root_from_env(),
        machine.as_deref().unwrap_or(""),
        &path,
    )
    .await
}

/// Moves a name on a machine (`files::rename_path`). Crossing
/// directories is the same verb; the target is never overwritten.
#[tauri::command]
async fn rename_path(machine: Option<String>, from: String, to: String) -> Result<(), String> {
    khor_gui_core::files::rename_path(
        &Node::root_from_env(),
        machine.as_deref().unwrap_or(""),
        &from,
        &to,
    )
    .await
}

/// Opens a path or a page **on the machine that holds it**
/// (`files::launch`) — the other end of `open_link`, which opens on the
/// machine this window is running on.
#[tauri::command]
async fn launch(machine: Option<String>, target: String) -> Result<(), String> {
    khor_gui_core::files::launch(
        &Node::root_from_env(),
        machine.as_deref().unwrap_or(""),
        &target,
    )
    .await
}

/// Everything under one directory on one machine that holds the words
/// (`files::search`) — what the box at the head of the tree asks
/// (工作区第六轮 第 47 条).
#[tauri::command]
async fn search(
    machine: Option<String>,
    dir: String,
    query: String,
) -> Result<khor_gui_core::files::Found, String> {
    khor_gui_core::files::search(
        &Node::root_from_env(),
        machine.as_deref().unwrap_or(""),
        &dir,
        &query,
    )
    .await
}

/// Whether this machine has a recycle bin at all.
#[tauri::command]
fn trash_here() -> bool {
    khor_gui_core::files::trash_here()
}

/// Whether this browser draws PDFs itself (`files::pdf_here`).
#[tauri::command]
fn pdf_here() -> bool {
    khor_gui_core::files::pdf_here()
}

/// The seven terminal commands are one attachment registry in gui-core,
/// mirroring chat: the window replays the feed and tails it, because
/// a byte stream is the shape both skins can serve
/// (`khor_gui_core::term` module head — xterm.js is the renderer).
#[tauri::command]
fn term_open(id: String, cols: u16, rows: u16) -> Result<(), String> {
    khor_gui_core::term::term_open(&Node::root_from_env(), &id, cols, rows)
}

#[tauri::command]
fn term_attach(id: String) -> Result<khor_gui_core::term::TermStart, String> {
    khor_gui_core::term::term_attach(&id)
}

#[tauri::command]
fn term_stream(id: String, since: u64) -> Result<khor_gui_core::term::TermChunk, String> {
    khor_gui_core::term::term_stream(&id, since)
}

#[tauri::command]
fn term_key(id: String, keys: String) -> Result<(), String> {
    khor_gui_core::term::term_key(&id, keys.into_bytes())
}

/// Files dropped on a terminal: their paths, shell-quoted, pasted in
/// (`term::term_drop`). The quoting is the registry's because it is
/// about the shell's syntax, and one wrong escape is a command.
///
/// The root rides along because the terminal may not be on this
/// machine: a drop onto a far session sends the files there first and
/// pastes where they landed, and finding that out is a node's job.
/// The command's own arguments are unchanged, so nothing above this
/// line — the frontend, the bridge's table — learns there is a network
/// under it.
#[tauri::command]
fn term_drop(id: String, paths: Vec<String>) -> Result<(), String> {
    khor_gui_core::term::term_drop(&Node::root_from_env(), &id, &paths)
}

#[tauri::command]
fn term_resize(id: String, cols: u16, rows: u16) -> Result<(), String> {
    khor_gui_core::term::term_resize(&id, cols, rows)
}

#[tauri::command]
fn term_leave(id: String) -> Result<(), String> {
    khor_gui_core::term::term_leave(&id)
}

/// Where this person has been, of one kind, most recent first
/// (`khor_gui_core::recents`). The cap is the caller's because it is a
/// question about the surface asking — a strip of tabs and a landing's
/// list want different lengths of the same list.
#[tauri::command]
fn recents(kind: String, limit: u32) -> Result<Vec<khor_gui_core::recents::RecentRow>, String> {
    khor_gui_core::recents::recents(&Node::root_from_env(), &kind, limit as usize)
}

#[tauri::command]
fn recent_visit(kind: String, machine: String, place: String) -> Result<(), String> {
    khor_gui_core::recents::recent_visit(&Node::root_from_env(), &kind, &machine, &place)
}

#[tauri::command]
fn forget_recent(kind: String, machine: String, place: String) -> Result<(), String> {
    khor_gui_core::recents::forget_recent(&Node::root_from_env(), &kind, &machine, &place)
}

/// Gives a page's borrow back.
///
/// **墓碑: this used to close a window too** (2026-09-09). A borrowed page
/// opened in a window of its own until the tab body could hold one, and
/// `open_web` built that window with the borrow's proxy on it. The whole
/// road is gone — the machine card's 借网 opens a browsing tab, and the
/// tab's page is a child webview (`open_page`) — so what is left is the
/// half that was never about windows: handing the network back.
#[tauri::command]
async fn close_web(session: String) -> Result<(), String> {
    khor_gui_core::web::drop_web_borrow(&Node::root_from_env(), &session).await
}

// ── the page inside a browsing tab (docs/NET.md 借网, 单元 2) ──
//
// **A page cannot be drawn by the face that draws everything else.** khor's
// whole interface is one webview rendering a React app; an arbitrary site
// cannot be put inside that page (`iframe` is refused by most sites, reads
// back neither title nor address, and the proxy can only be set when a
// webview is built). So a browsing tab's page is a **second webview**, a
// child of the same window, positioned over the rectangle the tab body
// occupies. The frontend owns the rectangle because only it knows the
// layout; this side owns the webview because only it can make one.
//
// **One child per tab, hidden rather than closed on a tab switch.** That is
// what makes it a tab rather than a bookmark: a page keeps its scroll, its
// half-filled form and its live connection while you look at another one.
//
// The upstream bug that used to rule this out — a window showing only its
// last child — was fixed in tauri 2024-11 (#11616, `build_as_child`), and
// the fix is in the version this builds against. The issue is still open
// upstream; it was never triaged.

/// Where a tab's page sits inside the window, in the frontend's own CSS
/// pixels — which are tauri's logical pixels, so nothing here does DPI
/// arithmetic and nothing can disagree about the scale factor.
#[derive(Debug, Clone, Copy, serde::Deserialize)]
struct PageBox {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    /// The face's own viewport height, sent with every rectangle.
    ///
    /// **It is here to close a coordinate mismatch, measured rather than
    /// assumed** (2026-09-09, macOS). A child webview's y is flipped by
    /// wry against the height tauri reports for the window — and on macOS
    /// tauri reports the **frame**, titlebar included: `inner_size` and
    /// `outer_size` both answered 1080x720 for a window whose content
    /// area is 688 tall. The flip therefore lands the page one titlebar
    /// too high: the face asked for y=93 and the page painted at 61,
    /// covering the address bar it was supposed to sit under.
    ///
    /// The face is the one side that knows the true content height, so it
    /// says so and this side takes the difference. **Not a 28 or a 32
    /// written down here**: that number is the titlebar of one window
    /// style on one platform, and it is zero on the platforms whose two
    /// sizes agree — a constant would be right today and wrong on the
    /// first window that is not this one.
    view: f64,
}

/// How much lower the page must sit than the face asked, because the two
/// sides measure the window differently. Zero wherever they agree.
#[cfg(desktop)]
fn frame_lift(window: &tauri::Window, at: PageBox) -> f64 {
    let Ok(inner) = window.inner_size() else {
        return 0.0;
    };
    let scale = window.scale_factor().unwrap_or(1.0);
    if scale <= 0.0 || at.view <= 0.0 {
        return 0.0;
    }
    let told = inner.height as f64 / scale;
    // Never negative: a face that measured a viewport *taller* than the
    // window is a face mid-resize, and pulling the page up on that would
    // be a jump the next rectangle undoes.
    (told - at.view).max(0.0)
}

/// One thing a page told us about itself, addressed to the tab it belongs
/// to. The tab id rides along because the face has several and the event
/// arrives with no other way to tell them apart.
#[derive(Debug, Clone, serde::Serialize)]
struct PageWord {
    tab: String,
    text: String,
}

/// What each live page webview is wired to, by label: the exit machine
/// it was built against, and where it last actually was.
///
/// **The exit half makes one specific failure impossible.** A webview's
/// proxy is fixed when it is built and there is no runtime setter (checked
/// against wry: `with_proxy_config` on the builder is the only door). So a
/// page webview reused for a different exit machine would keep leaving
/// through the old one — **not failing, succeeding through the wrong
/// network**, which is the shape docs/NET.md 可达性 rules out. Knowing what
/// each one was built against lets `open_page` rebuild instead of reuse.
///
/// **The address half is here because asking the webview kills the app.**
/// `webview.url()` reaches `url_from_webview` in wry
/// (`wkwebview/mod.rs:1349`), which does `webview.URL().unwrap()`. On
/// 2026-09-10 the first run of a built app **panicked the whole process**
/// there (exit 101), and this line was the only route to it. The tree had
/// been green since `e251db9` because no gate in this repo reaches a
/// native webview.
///
/// ⚠️ **What is measured, and what is not.** A `WKWebView` answers `nil`
/// for its URL only before `load` is called on it — measured with a bare
/// one: `nil` when fresh, and the URL is already there in the statement
/// after `load` returns. A navigation that **fails** is not `nil` either;
/// pointed at a dead port it ends in `didFinish` on `about:blank`. So the
/// `nil` window is the gap between `add_child` handing the request to the
/// event loop and wry loading in it, which is narrow — **and it happened
/// anyway**, on a machine doing nothing unusual.
///
/// **The fix does not depend on knowing how wide that window is.** A
/// panic below `url()` cannot be caught (it answers a `Result` and
/// unwraps under it), so the address is kept on this side instead and
/// that route stops existing. It is written when a navigation is asked
/// for and again when one **finishes**, which is what makes it the page's
/// real address rather than the last one requested — a page that
/// redirects, or follows a link, ends up somewhere nobody asked for.
fn page_exits() -> &'static std::sync::Mutex<std::collections::HashMap<String, (String, String)>> {
    static EXITS: std::sync::OnceLock<
        std::sync::Mutex<std::collections::HashMap<String, (String, String)>>,
    > = std::sync::OnceLock::new();
    EXITS.get_or_init(Default::default)
}

/// Remembers where a page webview is, keeping whatever exit it is on.
fn page_went(label: &str, to: &str) {
    if let Ok(mut m) = page_exits().lock()
        && let Some(slot) = m.get_mut(label)
    {
        slot.1 = to.to_owned();
    }
}

/// How a browsing tab presents itself to the sites it opens.
///
/// **A browser has to look like one, and this is not vanity.** A webview
/// inside an app identifies itself as bare WebKit: the same string Safari
/// sends, minus the `Version/… Safari/…` tail — and sites read that tail.
///
/// Measured 2026-09-09 against baidu through a borrow. With the tail it
/// answers with its real homepage, 713 KB. Without it, 227 bytes whose
/// whole body is
/// `location.replace(location.href.replace("https://","http://"))` — a
/// deliberate downgrade of clients it does not recognise. HSTS puts that
/// straight back on https, which serves the same 227 bytes, and the tab
/// spins: **2179 navigations in one sitting, white the whole time**,
/// because that page has nothing in it. From the outside it reads as「打
/// 不开」, and khor is not doing any of it — `open_page` was called 15
/// times while the page navigated 1254.
///
/// **The build number is taken out of the string, never written down.**
/// What Safari appends is `Safari/<the same build as AppleWebKit/…>`, and
/// that build is already in what the face reports — so this cannot go
/// stale the way a constant would, and it stays right on a machine whose
/// WebKit is not this one's. A face whose agent carries no `AppleWebKit/`
/// gets no override at all: inventing an identity for a client we cannot
/// read is worse than presenting the honest one, and on Linux and Windows
/// the honest one already names a browser.
fn as_a_browser(agent: &str) -> Option<String> {
    if agent.is_empty() || agent.contains("Safari/") {
        return None;
    }
    let build = agent
        .split("AppleWebKit/")
        .nth(1)?
        .split_whitespace()
        .next()?;
    Some(format!("{agent} Safari/{build}"))
}

/// A tab id as a webview label.
///
/// **Total, because the tab id stopped being tame.** Tab ids were minted
/// from a timestamp and a counter, so the old guard only had to keep a
/// separator out of a key. Then a workspace's editor column started
/// naming its own page after a machine (工作区批5) and tauri refused the
/// label outright — it takes only `[A-Za-z0-9-/:_]`, and a space is not
/// in it, so every press of the VS Code switch drew `runtime error:
/// Window labels must only include...` where the editor should have
/// been. That shipped and stayed invisible for a day: every gate runs in
/// a browser, and a browser builds no child webview, so the one way to
/// see it was to open the desktop app and press the button
/// (2026-09-13).
///
/// **Escaped rather than replaced, because a label is a key.** Folding
/// the disallowed characters onto `-` makes `my mac` and `my-mac` the
/// same label, and machine names are the user's to type — an editor
/// column would have opened onto the neighbour's page. `_` marks an
/// escape and is escaped itself, so the map is injective over any
/// input, non-ASCII included, and an ordinary ASCII name still reads as
/// itself.
fn page_label(tab: &str) -> String {
    let mut out = String::from("page-");
    for b in tab.bytes() {
        match b {
            b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' | b'-' => out.push(b as char),
            _ => out.push_str(&format!("_{b:02x}")),
        }
    }
    out
}

/// Opens (or re-points) the page a browsing tab is showing.
///
/// Answers with the borrow, because the face is what closes with
/// it. Async because the borrow dials.
#[cfg(desktop)]
#[tauri::command]
async fn open_page(
    window: tauri::Window,
    tab: String,
    machine: String,
    url: String,
    at: PageBox,
    agent: String,
) -> Result<khor_gui_core::web::WebBorrow, String> {
    use tauri::{Emitter, LogicalPosition, LogicalSize, Manager, WebviewUrl};
    // **An empty machine means「go direct」** (工作区批5), and the one
    // thing that asks for it is a workspace's VS Code: the URL is
    // already a loopback address of khor's own tunnel, so there is
    // nothing to borrow and nothing a proxy could add.
    //
    // **It also means no shim, and that is a decision, not an
    // omission.** `page_shim.js` exists because a *borrowed* page can
    // reach the network through WebTransport and WebAuthn without
    // passing the proxy, which reports the device's real address to
    // somebody else's site (`scripts/check-shim.mjs`). A direct page
    // has no proxy to escape and no exit to contradict — the far side
    // is the person's own machine, reached through a pipe khor built.
    // Taking APIs away there would protect nothing and could break an
    // editor.
    let direct = machine.is_empty();
    let borrow = if direct {
        khor_gui_core::web::WebBorrow {
            session: String::new(),
            addr: String::new(),
        }
    } else {
        khor_gui_core::web::borrow_web(&Node::root_from_env(), &machine).await?
    };
    let target: tauri::Url = url.parse().map_err(|_| format!("not a url: {url}"))?;
    let proxy: Option<tauri::Url> = if direct {
        None
    } else {
        Some(
            format!("http://{}", borrow.addr)
                .parse()
                .map_err(|e| format!("{e}"))?,
        )
    };
    let label = page_label(&tab);
    let going_to = target.to_string();
    let top = at.y + frame_lift(&window, at);

    // Reuse only when it is the same exit. A different one means the
    // webview in front of us is wired to a proxy it can no longer be
    // talked out of, so it goes.
    let wired = page_exits()
        .lock()
        .ok()
        .and_then(|m| m.get(&label).cloned());
    let same_exit = wired
        .as_ref()
        .map(|(exit, _)| exit == &machine)
        .unwrap_or(false);
    if let Some(live) = window.app_handle().get_webview(&label) {
        if same_exit {
            // **Only navigate when it is somewhere else.** Coming back to
            // a tab mounts its body afresh and asks again for the address
            // it is already on; navigating to where you already are is a
            // reload, and a reload on every tab switch is exactly the
            // thing keeping a webview per tab exists to avoid.
            //
            // Read from this side rather than from the webview: see
            // `page_exits`. Asking a webview that has not started its
            // navigation for its URL takes the process down with it.
            let now = wired.as_ref().map(|(_, at)| at.as_str());
            if now != Some(target.as_str()) {
                page_went(&label, target.as_str());
                live.navigate(target).map_err(|e| e.to_string())?;
            }
            live.set_position(LogicalPosition::new(at.x, top))
                .map_err(|e| e.to_string())?;
            live.set_size(LogicalSize::new(at.w, at.h))
                .map_err(|e| e.to_string())?;
            live.show().map_err(|e| e.to_string())?;
            return Ok(borrow);
        }
        let _ = live.close();
    }

    // Every event goes to **this** window and no further. A page webview
    // is somebody else's site; broadcasting khor's own events into it
    // would be handing a stranger the app's internal traffic.
    let face = window.label().to_string();
    let (t_tab, l_tab) = (tab.clone(), tab.clone());
    let (t_face, l_face) = (face.clone(), face.clone());
    // **The shim rides on the builder, beside the proxy, on purpose.**
    // Both are fixed when the webview is built and neither has a runtime
    // setter, so they can only ever be wrong together — and a page
    // pointed at a borrow *without* the shim is the worst of the three
    // states: it looks borrowed and reports where the device really is.
    // The reuse path above keeps both for the same reason it keeps the
    // proxy: the user scripts belong to the webview, not to a load.
    //
    // Read `page_shim.js` before touching this. It takes two WebKit APIs
    // away from the page and **cannot take the third**, and the third is
    // still leaking every hostname the page prefetches.
    let mut builder = tauri::webview::WebviewBuilder::new(&label, WebviewUrl::External(target));
    if let Some(proxy) = proxy {
        // The two ride together or not at all — see `direct` above.
        builder = builder
            .proxy_url(proxy)
            .initialization_script_for_all_frames(khor_gui_core::web::PAGE_SHIM);
        // A user agent that says「app」is what makes a site serve its
        // mobile face to a desktop window; an editor khor started needs
        // no such dressing.
        if let Some(seem) = as_a_browser(&agent) {
            builder = builder.user_agent(&seem);
        }
    }
    let builder = builder
        .on_document_title_changed(move |view, title| {
            let _ = view.app_handle().emit_to(
                t_face.clone(),
                "page:title",
                PageWord {
                    tab: t_tab.clone(),
                    text: title,
                },
            );
        })
        .on_page_load(move |view, payload| {
            let word = match payload.event() {
                tauri::webview::PageLoadEvent::Started => "started",
                tauri::webview::PageLoadEvent::Finished => "finished",
            };
            let app = view.app_handle();
            let _ = app.emit_to(
                l_face.clone(),
                "page:load",
                PageWord {
                    tab: l_tab.clone(),
                    text: word.to_owned(),
                },
            );
            // **On arrival, not on departure.** This used to fire for
            // both halves of a load, so every navigation reported the
            // tab's place twice — and the first of the two is a place the
            // tab has not reached yet. A navigation that starts and never
            // finishes would have left the address bar naming somewhere
            // the tab is not, which is this repo's oldest failure shape:
            // a step that cannot be taken does not become "no answer", it
            // becomes the answer next to it.
            //
            // It is also not free. A page in a redirect loop reported
            // 2484 times in one sitting (baidu, 2026-09-09, before
            // `as_a_browser`), and each report is a render of khor's own
            // face.
            //
            // **Where the tab is, taken from what actually loaded here.**
            // This used to be reported from `on_navigation`, which fires
            // for a `target="_blank"` link too — and that link's page
            // never loads in *this* webview, it becomes a tab of its own.
            // So one click moved both: the new tab opened, and the tab
            // that was left behind followed it. Measured 2026-09-09.
            //
            // ⚠️ **A page that moves itself with `pushState` does not
            // load anything**, so it does not reach here and the address
            // stays where it was. Real browsers follow that too; the hook
            // for it is a script in the page, not this callback.
            if matches!(payload.event(), tauri::webview::PageLoadEvent::Finished) {
                let at = payload.url().to_string();
                // Kept on this side too, because the reuse path reads it
                // from here — a page that redirected, or followed a link,
                // is somewhere nobody asked it to go.
                page_went(&page_label(&l_tab), &at);
                let _ = app.emit_to(
                    l_face.clone(),
                    "page:went",
                    PageWord {
                        tab: l_tab.clone(),
                        text: at,
                    },
                );
            }
        });
    // **Kept, and deliberately silent.** `on_navigation` answers whether
    // a navigation may happen at all, which is where a scheme allowlist
    // would live — `file:` and whatever else a borrowed page must not be
    // able to reach through this window. It used to also report the
    // address, and that was wrong: it fires for a `target="_blank"` link
    // whose page never loads here (see `on_page_load`).
    let builder = builder.on_navigation(|_to| true);
    // **A link that asks for a new window gets a new tab**, which is the
    // only answer that leaves the decision where it belongs. Whether a
    // link opens where you are or beside it is the page author's call —
    // `target="_blank"` on one link and not another is a distinction they
    // made on purpose — and khor's job is to carry it, not to overrule it
    // in either direction.
    //
    // Without this the default answer is to deny, and **denying is
    // silent**: measured on 2026-09-09, baidu's results are such links
    // and pressing one looked like a page that had stopped responding.
    // The first fix here navigated in place, which worked and was still
    // khor deciding — corrected the same evening.
    //
    // `Deny` and an event, rather than `Create`: the new tab is the
    // *face's* to mint, because the tab strip, the store and the borrow
    // ledger are all on that side and a webview created behind their back
    // would belong to no tab at all.
    let popped = window.app_handle().clone();
    let (p_tab, p_face) = (tab.clone(), face.clone());
    let builder = builder.on_new_window(move |to, _features| {
        let _ = popped.emit_to(
            p_face.clone(),
            "page:popup",
            PageWord {
                tab: p_tab.clone(),
                text: to.to_string(),
            },
        );
        tauri::webview::NewWindowResponse::Deny
    });

    window
        .add_child(
            builder,
            LogicalPosition::new(at.x, top),
            LogicalSize::new(at.w, at.h),
        )
        .map_err(|e| e.to_string())?;
    if let Ok(mut m) = page_exits().lock() {
        m.insert(label, (machine, going_to));
    }
    Ok(borrow)
}

/// Keeps a page over its tab body as the layout moves under it. Called by
/// the side that can see the layout; there is no way for this side to know
/// a rectangle changed.
#[cfg(desktop)]
#[tauri::command]
fn move_page(window: tauri::Window, tab: String, at: PageBox) -> Result<(), String> {
    use tauri::{LogicalPosition, LogicalSize, Manager};
    let Some(view) = window.app_handle().get_webview(&page_label(&tab)) else {
        return Ok(());
    };
    let top = at.y + frame_lift(&window, at);
    view.set_position(LogicalPosition::new(at.x, top))
        .map_err(|e| e.to_string())?;
    view.set_size(LogicalSize::new(at.w, at.h))
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Takes a page off the screen without ending it.
///
/// **Two jobs, one call.** A tab switch hides the page you are leaving, and
/// so does opening anything that has to float over it: a native child
/// webview always paints above the DOM, so khor's own menus and the address
/// bar's completions would otherwise be drawn underneath the site.
#[cfg(desktop)]
#[tauri::command]
fn hide_page(app: tauri::AppHandle, tab: String) -> Result<(), String> {
    use tauri::Manager;
    let Some(view) = app.get_webview(&page_label(&tab)) else {
        return Ok(());
    };
    view.hide().map_err(|e| e.to_string())?;
    Ok(())
}

/// Puts a page back after `hide_page` took it away.
///
/// **Separate from `open_page` on purpose.** Coming back from a menu is
/// not opening anything: it must not dial a borrow, must not navigate,
/// and must not cost the page a frame of reload. A page that is not there
/// is not an error here — the tab may have been closed while a menu was
/// up, and nothing about that is worth a refusal.
#[cfg(desktop)]
#[tauri::command]
fn show_page(app: tauri::AppHandle, tab: String) -> Result<(), String> {
    use tauri::Manager;
    let Some(view) = app.get_webview(&page_label(&tab)) else {
        return Ok(());
    };
    view.show().map_err(|e| e.to_string())?;
    Ok(())
}

// ⚠️ 墓碑 (2026-09-23, docs/SHELL.md 第十轮): `squeeze_the_window_buttons`
// and `keep_the_buttons_in_the_rail` stood here — they moved macOS's
// three buttons to an older spacing and put them back whenever AppKit
// laid the title bar out again (`window_buttons.rs`, and a frame-change
// observer on top of it). **Nothing of the system's is moved now**: the
// left block's first row is where they stand (第十三轮 — the window's
// own row of 第十轮 lasted three rounds), and `trafficLightPosition`
// places the buttons inside it once, at the SDK's own size and spacing,
// with the number `traffic` measures (第十一轮 — it lived in
// `tauri.conf.json` for one round, written down and wrong). The row's
// height, where the trio starts and the gap after it are `traffic`'s
// three constants, held to `tokens.css` by `check-boot.mjs`.

/// The window itself, **built here rather than in `tauri.conf.json`**.
///
/// One thing needs it: macOS's three buttons are placed by a number that
/// has to be *measured* first (`traffic`'s module head — the SDK's button
/// height and AppKit's own inset both go into it), and tauri takes that
/// number only when the window is built. A config file cannot hold a
/// measurement.
///
/// Everything else here is what that file used to say, unchanged, plus
/// the ground: setting it on the builder paints the window from its
/// first frame rather than a moment later (`paint_the_window_before_the_page`
/// has the measurements).
#[cfg(desktop)]
fn build_the_window(app: &tauri::AppHandle) -> tauri::Result<()> {
    use tauri::{WebviewUrl, WebviewWindowBuilder};
    let mut win = WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
        .title("Khor")
        .inner_size(1080.0, 720.0)
        .min_inner_size(400.0, 500.0);
    #[cfg(target_os = "macos")]
    {
        win = win
            .title_bar_style(tauri::utils::TitleBarStyle::Overlay)
            .hidden_title(true)
            // **The left block on the system's sidebar material** (docs/
            // SHELL.md 第十五轮 第 7 条, D4): AppKit's own
            // `NSVisualEffectView` in its sidebar material behind the
            // webview, which only shows where the page paints nothing —
            // the left block, whose ground goes transparent on this
            // platform alone (`app.css` `:root[data-material]`); the
            // right side, the board and R0 keep painting their own.
            // `FollowsWindowActiveState`: it goes flat with the window
            // behind another, as every Mac sidebar does.
            //
            // The window has to be transparent for the material to be
            // seen, and on macOS that is tauri's `macos-private-api`
            // (`Cargo.toml`, `tauri.conf.json` — the cost is written
            // there: no Mac App Store).
            .transparent(true)
            .effects(
                tauri::window::EffectsBuilder::new()
                    .effect(tauri::window::Effect::Sidebar)
                    .state(tauri::window::EffectState::FollowsWindowActiveState)
                    .build(),
            );
        if let Some(lights) = traffic::measure() {
            traffic::remember(lights);
            win = win.traffic_light_position(tauri::LogicalPosition::new(
                traffic::X,
                traffic::inset_y(traffic::PANE_HEAD, &lights),
            ));
        }
    }
    // **A store named by `KHOR_HOME` gets a page store of its own**
    // (2026-09-27). The webview's localStorage is keyed by the app, not
    // by khor's store, so a second khor on another store — the native
    // gate's, or anyone's second profile — read and wrote the owner's
    // open tabs, list width and theme mirror (found when the gate's app
    // came up showing the owner's session tabs). Unset, nothing changes:
    // the default store keeps the page store it has always had.
    if let Some(explicit) = std::env::var_os("KHOR_HOME") {
        // FNV-1a, spelled out: std's hasher may change between Rust
        // releases, and a changed id is a profile whose page store is
        // silently gone after a toolchain update.
        let fnv = |seed: u64, bytes: &[u8]| {
            bytes.iter().fold(seed, |h, b| (h ^ u64::from(*b)).wrapping_mul(0x100_0000_01b3))
        };
        let bytes = explicit.as_encoded_bytes();
        let mut id = [0u8; 16];
        id[..8].copy_from_slice(&fnv(0xcbf2_9ce4_8422_2325, bytes).to_le_bytes());
        id[8..].copy_from_slice(&fnv(0x6c62_272e_07bb_0142, bytes).to_le_bytes());
        #[cfg(target_os = "macos")]
        {
            win = win.data_store_identifier(id);
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = id;
            win = win.data_directory(std::path::PathBuf::from(&explicit).join("webview"));
        }
    }
    win.build()?;
    Ok(())
}

/// How much of the left block's first row macOS's three buttons stand
/// in, in CSS pixels — what the face sets `--traffic` from, and where
/// the search box starts. **Zero where the system draws no buttons over
/// the row** (Windows draws its own at the tab strip's end, Linux keeps
/// its decorations, a browser has no window).
#[cfg(desktop)]
#[tauri::command]
fn traffic_room() -> f64 {
    #[cfg(target_os = "macos")]
    {
        traffic::measure()
            .map(|m| traffic::room(traffic::X, traffic::GAP, &m))
            .unwrap_or(0.0)
    }
    #[cfg(not(target_os = "macos"))]
    0.0
}

/// A phone draws no buttons over the row, so this is the true answer, not
/// a refusal — `show_menu`'s kind of mobile half.
#[cfg(not(desktop))]
#[tauri::command]
fn traffic_room() -> f64 {
    0.0
}

/// Give the window the app's own ground **before the page has one**.
///
/// ⚠️ **Measured 2026-09-11 by screenshotting a cold launch**: the
/// window is on screen at ~500 ms and the first HTML paints at ~760 ms.
/// For that quarter second the webview shows its platform default —
/// near-white on macOS — and on a dark desktop that is a flash of the
/// wrong colour, then a settle. `index.html` paints a first frame of its
/// own, and it cannot help here: it has to arrive first, and this is
/// about the time before anything arrives.
///
/// **Which colour is not a constant, which is why this is code and not
/// a line in `tauri.conf.json`.** That file takes one
/// `backgroundColor`, and this app has two grounds; a single value
/// would trade the dark desktop's flash for a light one's. `theme()`
/// asks the system what it is, right now, and the window is painted to
/// match.
///
/// ⚠️ **These two values are `--background` from
/// `apps/gui/src/styles/tokens.css`, light and dark.** They are
/// checked against it by `apps/gui/scripts/check-boot.mjs`, which reads
/// this file — the same gate that checks the ones in `index.html`, for
/// the same reason: a palette that moves and leaves these behind is a
/// quarter-second flash of last month's colour with nothing to say so.
///
/// ⚠️ **`setup` is not early enough to cover the whole gap, measured
/// after the fact.** Reading the pixel at the centre of the window out
/// of cold-launch screenshots:
///
/// ```text
/// 528 ms   #1e1e1e   the platform default, before and after this change
/// 709 ms   #0f0f0e   this ground (the frame is #242422 since 2026-09-30)
/// 760 ms   the first HTML frame
/// ```
///
/// So it moves the settle from ~760 ms to ~700 ms and leaves the first
/// ~200 ms as it was. **Covering those needs the colour on the window
/// *builder*, which means building the window here instead of in
/// `tauri.conf.json`** — and that means this file owning the title, the
/// size and the two minimums as well. Worth doing when something else
/// already needs the window built in Rust; not worth it for 200 ms of
/// colour on its own.
///
/// **It does not cover a desktop that changes theme while the app
/// runs.** The page follows that on its own (`prefers-color-scheme`),
/// and this ground is only ever seen at launch, so the cost of being
/// stale is a colour nobody is looking at.
#[cfg(desktop)]
fn paint_the_window_before_the_page(app: &tauri::AppHandle) {
    use tauri::{Manager, Theme};
    let Some(w) = app.get_webview_window("main") else {
        return;
    };
    // **The store's choice, not the desktop's answer** (批 D2). This is
    // the one reader of `theme` on the Rust side, and it has to be: the
    // window is painted before any HTML exists, so the page cannot do
    // it. `auto` — and a store this process cannot read — is the
    // desktop, which is what khor did before the key existed.
    let dark = match khor_node::config::get(&Node::root_from_env(), "theme") {
        Ok("dark") => true,
        Ok("light") => false,
        _ => matches!(w.theme(), Ok(Theme::Dark)),
    };
    let ground = if dark {
        tauri::webview::Color(0x24, 0x24, 0x22, 0xff)
    } else {
        tauri::webview::Color(0xeb, 0xeb, 0xe5, 0xff)
    };
    // Nothing to do if the platform refuses: the failure is the old
    // behaviour, which is a flash rather than a fault.
    //
    // **Not on macOS since 第十五轮** (D4): the window there is
    // transparent over the sidebar material (`build_the_window`), and an
    // opaque ground on the webview would paint over the material for good
    // — the material itself is what stands in the window before the page.
    #[cfg(not(target_os = "macos"))]
    let _ = w.set_background_color(Some(ground));
    #[cfg(target_os = "macos")]
    let _ = ground;
}

/// Back, forward and reload for one tab's page.
///
/// **Two of the three are script, and that is a gap rather than a
/// choice.** tauri has `reload` and nothing else: there is no `go_back`,
/// no `go_forward` and — the one that shows — no `can_go_back`, so the
/// arrows cannot grey out when there is nowhere to go. Pressing one at
/// the start of a history does nothing and says nothing, which is the
/// honest state of it until the platform grows the question.
#[cfg(desktop)]
#[tauri::command]
fn page_step(app: tauri::AppHandle, tab: String, way: String) -> Result<(), String> {
    use tauri::Manager;
    let Some(view) = app.get_webview(&page_label(&tab)) else {
        return Ok(());
    };
    match way.as_str() {
        "back" => view.eval("history.back()").map_err(|e| e.to_string()),
        "forward" => view.eval("history.forward()").map_err(|e| e.to_string()),
        "reload" => view.reload().map_err(|e| e.to_string()),
        // Refused by name, the `load` rule: a word this build does not
        // know, read as one of the three, would look like a working
        // button doing somebody else's job.
        other => Err(format!("no such step: {other}")),
    }
}

/// Ends a page: the webview goes and the tab stops having one.
///
/// **Not the borrow.** One exit machine is one borrow however many tabs
/// point at it, so the last tab to stop naming a machine is what gives its
/// network back (`views/borrows`), and that is a count only the face keeps.
#[cfg(desktop)]
#[tauri::command]
fn close_page(app: tauri::AppHandle, tab: String) -> Result<(), String> {
    use tauri::Manager;
    let label = page_label(&tab);
    if let Ok(mut m) = page_exits().lock() {
        m.remove(&label);
    }
    let Some(view) = app.get_webview(&label) else {
        return Ok(());
    };
    view.close().map_err(|e| e.to_string())
}

// ── khor's own overlays, in a window of their own (单元 2, 浮层) ──
//
// **A native child webview always paints above the DOM.** That is not a
// z-index anybody can raise; it is another layer of the window. So the
// address bar's completions — drawn in khor's own page, one row under the
// address they complete — came out *underneath* the site the tab is
// showing. Two answers were tried in front of somebody and both were
// judged wrong: taking the page away while the menu is up blanked the
// whole site, and pushing the page's top down shoved the site out from
// under the eyes of the person reading it. 「不对,应该是浮在页面上的」.
//
// **What browsers do is a second window**, and it is the only answer that
// makes the menu genuinely float: an operating-system window sits above
// another window's child views by construction. It carries the same React
// list, rendered by the same component (`components/OmniMenu`), so there
// is one menu and not two that drift apart.
//
// Three properties are what make it read as a menu rather than a window,
// and each is doing a job:
//
// - **`parent`** — the platform then keeps it above the face, moves it
//   with the face and destroys it with the face. Without a parent the app
//   would stop exiting when its window is closed (a hidden window is
//   still a window), and the alternative — `always_on_top` — would have
//   floated khor's completions over other applications.
// - **`focusable(false)`** — the caret has to stay in the address box.
//   This menu is driven from that box's keyboard (↑↓/Tab/Enter all belong
//   to the field), and a window that took focus would blur the field
//   mid-completion. On macOS this is `canBecomeKeyWindow` answering no
//   (tao 0.35, `platform_impl/macos/window.rs`).
// - **`accept_first_mouse(true)`** — the other half of that one. Every
//   click into a window that cannot become key is a "first mouse" click,
//   and a view that does not accept one swallows it. Without this the
//   rows would be visible and unpressable.

/// Where an overlay has to appear, in the face's own CSS pixels, with the
/// two viewport measurements that turn them into screen ones.
#[derive(Debug, Clone, Copy, serde::Deserialize)]
struct Spot {
    x: f64,
    y: f64,
    w: f64,
    /// The face's viewport height and width. **Both are here to be
    /// subtracted from the window frame, and neither is decoration** —
    /// see `on_screen`, which is where the arithmetic and the reason for
    /// it live. `PageBox.view` carries the same measurement for the same
    /// kind of reason.
    view: f64,
    across: f64,
}

/// The window khor's own overlays are drawn in. One, reused: building a
/// webview costs a page load, and a menu that had to load a page every
/// time somebody pressed the address bar would arrive after they had
/// finished typing.
#[cfg(desktop)]
const MENU: &str = "menu";

/// Where a `Spot` lands on the screen, in physical pixels, and how wide.
///
/// **Both offsets are measured rather than written down.** The face
/// reports the size of its own viewport; this side asks the platform for
/// the size of the whole window frame; the difference is the chrome,
/// whatever the platform decided that is — a titlebar on macOS, a
/// titlebar and three borders on Windows, nothing at all where the window
/// is undecorated. A constant here would be right on the machine it was
/// measured on and wrong on the first window that is not that one, which
/// is the mistake `PageBox.view` exists to have already made once.
///
/// The side borders are what let the top be worked out: chrome is
/// left+right+bottom+top, the sides are half of what the width is short
/// by, and the bottom border is the same as a side one on every window
/// style this runs in. What is left over is the titlebar.
#[cfg(desktop)]
fn on_screen(window: &tauri::Window, at: Spot) -> Result<(f64, f64, f64), String> {
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    let frame = window.outer_size().map_err(|e| e.to_string())?;
    let origin = window.outer_position().map_err(|e| e.to_string())?;
    if scale <= 0.0 || at.view <= 0.0 || at.across <= 0.0 {
        return Err("the face has not measured itself yet".into());
    }
    // Never negative: a face that measured a viewport bigger than the
    // window is a face caught mid-resize, and a menu placed on that would
    // jump to somewhere the next rectangle undoes.
    let side = ((frame.width as f64 - at.across * scale) / 2.0).max(0.0);
    let down = ((frame.height as f64 - at.view * scale) - side).max(0.0);
    Ok((
        origin.x as f64 + side + at.x * scale,
        origin.y as f64 + down + at.y * scale,
        at.w * scale,
    ))
}

/// The rows the menu is showing and the face that asked for them.
///
/// **The face's label rides along because a pick has to go home.** The
/// menu window is one window serving whichever face opened it, and an
/// event broadcast to every webview would hand khor's traffic to the site
/// in the browsing tab (`open_page` says why that matters).
#[cfg(desktop)]
fn menu_held() -> &'static std::sync::Mutex<Option<(String, serde_json::Value)>> {
    static HELD: std::sync::OnceLock<std::sync::Mutex<Option<(String, serde_json::Value)>>> =
        std::sync::OnceLock::new();
    HELD.get_or_init(Default::default)
}

/// Whether the menu is meant to be on screen right now.
///
/// **It exists because showing is two steps and hiding is one.** The
/// window is shown by `size_menu`, which arrives after the menu window has
/// laid its list out — and a `hide_menu` can land in that gap. Without
/// this, a menu dismissed while its last render was still in flight would
/// reappear by itself.
/// How wide the box that owns the menu is, in physical pixels.
///
/// **Kept here because the window cannot be asked.** `set_size` goes
/// through the event loop, so a `size_menu` arriving hard on the heels of
/// a `show_menu` reads back the width the window had *before* the show —
/// and writing that back with the new height pinned the menu at one pixel
/// wide. Measured 2026-09-09: the window server listed it 1x151, in front
/// and correctly placed, and invisible for all practical purposes.
#[cfg(desktop)]
fn menu_wide() -> &'static std::sync::atomic::AtomicU32 {
    static WIDE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);
    &WIDE
}

#[cfg(desktop)]
fn menu_up() -> &'static std::sync::atomic::AtomicBool {
    static UP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    &UP
}

/// Builds the overlay window, once, while the app is starting.
///
/// **At startup rather than on the first press, and the reason is that a
/// failure here has to be visible.** Built lazily, a window that would not
/// build turned into a refused command, and a refused command turned into
/// a face quietly drawing its menu the old way — the same silence that hid
/// a missing capabilities file for a day (`onPageWord`). Here it says so
/// out loud, and everything downstream reads "no window" as "draw it in
/// the page", which is a state rather than an error.
///
/// It also buys the thing lazy building cost: the first press of the
/// address bar no longer waits for a page load.
#[cfg(desktop)]
fn hang_menu_window(app: &tauri::AppHandle) {
    // **Not a child window, and that was measured rather than chosen.**
    // The first shape of this hung the overlay off the face with
    // `parent`, which reads better — the platform then keeps it above the
    // face, moves it along and destroys it with the face. Two things
    // broke:
    //
    //   - `orderOut:` **takes a child off its parent** on macOS, so the
    //     first hide detached it and every show after that put it behind
    //     the face. From the outside that is 「只有第一次点会弹出」.
    //   - `addChildWindow:` binds the child's window level to the
    //     parent's, so `always_on_top` could not fix it: the window
    //     server listed both at layer 0 with the menu second.
    //
    // A level of its own is the shape that survives being hidden. What it
    // costs is the two things the parent was giving: the app has to close
    // this window itself (see `run`, or a hidden window keeps it alive
    // after its face is gone), and the menu no longer follows a window
    // being dragged — it is repositioned on every show, so what shows is
    // dragging the window *while a completion list is up*.
    let built = tauri::WebviewWindowBuilder::new(
        app,
        MENU,
        // **The same page as the face, told which half of itself to be.**
        // A second HTML entry point would be a second place for the
        // menu's look to be defined, and the two would drift; the query
        // is read once at boot (`main.tsx`) and picks a root.
        tauri::WebviewUrl::App("index.html?face=menu".into()),
    )
    .title("")
    .decorations(false)
    .resizable(false)
    .skip_taskbar(true)
    .focusable(false)
    .accept_first_mouse(true)
    .shadow(true)
    .always_on_top(true)
    // Built invisible and shown by `size_menu`: a window sized before its
    // list has been laid out would flash at whatever size it was built.
    .visible(false)
    .inner_size(1.0, 1.0)
    .build();
    if let Err(e) = built {
        eprintln!("khor: the overlay window did not build, menus stay in the page: {e}");
    }
}

/// Puts khor's completions on screen under the box that owns them.
///
/// Answers whether the float took it. **Not an error when it did not**:
/// a platform with no overlay window draws its menus in the page, which
/// is not a failure but the other half of this control, and the face has
/// one to fall back to (`useFloatedMenu`).
#[cfg(desktop)]
#[tauri::command]
fn show_menu(window: tauri::Window, at: Spot, rows: serde_json::Value) -> Result<bool, String> {
    use tauri::{Emitter, Manager, PhysicalPosition, PhysicalSize};
    let app = window.app_handle().clone();
    let Some(menu) = app.get_webview_window(MENU) else {
        return Ok(false);
    };
    let (x, y, w) = on_screen(&window, at)?;
    let face = window.label().to_string();
    if let Ok(mut held) = menu_held().lock() {
        *held = Some((face, rows.clone()));
    }
    menu_up().store(true, std::sync::atomic::Ordering::Relaxed);
    // The width is this side's to set — it is the box's width, which only
    // the face can see. The height is the list's, and the list is the one
    // that knows (`size_menu`).
    let wide = (w.max(1.0) as u32).max(1);
    menu_wide().store(wide, std::sync::atomic::Ordering::Relaxed);
    let tall = menu.inner_size().map_err(|e| e.to_string())?.height.max(1);
    menu.set_position(PhysicalPosition::new(x, y))
        .map_err(|e| e.to_string())?;
    menu.set_size(PhysicalSize::new(wide, tall))
        .map_err(|e| e.to_string())?;
    app.emit_to(MENU, "menu:rows", rows)
        .map_err(|e| e.to_string())?;
    Ok(true)
}

/// The menu window says how tall its list came out, and is shown at that
/// height.
///
/// **It has to be the one to say it.** The height of a list of rows is a
/// layout fact, and working one out on this side would mean writing down
/// a row height — a number that goes wrong the first time somebody changes
/// the type scale, and goes wrong silently, as a menu with its last row
/// cut off.
#[cfg(desktop)]
#[tauri::command]
fn size_menu(app: tauri::AppHandle, h: f64) -> Result<(), String> {
    use tauri::{Manager, PhysicalSize};
    let Some(menu) = app.get_webview_window(MENU) else {
        return Ok(());
    };
    let scale = menu.scale_factor().map_err(|e| e.to_string())?;
    // **Not read back off the window** — see `menu_wide`, which is the
    // account of what reading it back cost.
    let wide = menu_wide().load(std::sync::atomic::Ordering::Relaxed);
    menu.set_size(PhysicalSize::new(wide, (h * scale).max(1.0) as u32))
        .map_err(|e| e.to_string())?;
    if menu_up().load(std::sync::atomic::Ordering::Relaxed) {
        menu.show().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// What the menu has to show right now, pulled by the menu window when it
/// mounts.
///
/// **A pull as well as an event, because the two cannot be ordered.** The
/// window is built and the rows are chosen in the same breath, and a
/// webview still loading its page cannot be listening yet — an event sent
/// then reaches nobody, and the first press of the address bar would open
/// an empty menu.
#[cfg(desktop)]
#[tauri::command]
fn menu_rows() -> Result<serde_json::Value, String> {
    Ok(menu_held()
        .lock()
        .ok()
        .and_then(|h| h.as_ref().map(|(_, rows)| rows.clone()))
        .unwrap_or(serde_json::Value::Null))
}

/// Somebody pressed a row. It goes back to the face that opened the menu,
/// and to no other webview.
#[cfg(desktop)]
#[tauri::command]
fn menu_pick(app: tauri::AppHandle, key: String) -> Result<(), String> {
    use tauri::Emitter;
    let face = menu_held()
        .lock()
        .ok()
        .and_then(|h| h.as_ref().map(|(f, _)| f.clone()));
    let Some(face) = face else { return Ok(()) };
    app.emit_to(face, "menu:pick", key)
        .map_err(|e| e.to_string())
}

/// Takes the menu off the screen. **Hidden rather than closed**, for
/// `open_page`'s reason turned around: the window is reused, and closing
/// it would cost the next press of the address bar a page load.
#[cfg(desktop)]
#[tauri::command]
fn hide_menu(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    menu_up().store(false, std::sync::atomic::Ordering::Relaxed);
    let Some(menu) = app.get_webview_window(MENU) else {
        return Ok(());
    };
    menu.hide().map_err(|e| e.to_string())
}

// **The mobile halves are not refusals, and that is the difference from
// the page ones below.** A phone has no second window to hang a menu in,
// and it does not need one: it has no page webview either, so nothing is
// painting over khor's own overlays and the completions drawn in the page
// are the right ones. `false` is that answer — "the float did not take
// it" — and the face renders the list where it always did.
#[cfg(not(desktop))]
#[tauri::command]
fn show_menu(at: Spot, rows: serde_json::Value) -> Result<bool, String> {
    let _ = (at, rows);
    Ok(false)
}

#[cfg(not(desktop))]
#[tauri::command]
fn size_menu(h: f64) -> Result<(), String> {
    let _ = h;
    Ok(())
}

#[cfg(not(desktop))]
#[tauri::command]
fn menu_rows() -> Result<serde_json::Value, String> {
    Ok(serde_json::Value::Null)
}

#[cfg(not(desktop))]
#[tauri::command]
fn menu_pick(key: String) -> Result<(), String> {
    let _ = key;
    Ok(())
}

#[cfg(not(desktop))]
#[tauri::command]
fn hide_menu() -> Result<(), String> {
    Ok(())
}

/// Makes the window wider by `delta` logical pixels (negative narrows
/// it), instead of taking the room out of what is already on screen.
///
/// 「右上角的icon,都是点击后,在最右侧,拓宽窗口, 而不是挤占现有窗口」
/// — the user, 2026-09-16. One column of the workspace pane really adds
/// a column (the sessions), and adding a column inside a fixed window
/// is taking one away from the file.
///
/// **Not a node call, and that is the rule rather than an omission**:
/// `khor-gui-core` holds what the CLI can also do, and a window is not
/// something `khor` has. It never leaves this file.
///
/// **It grows to the right and stops at the screen.** The far edge is
/// the current monitor's work area — the part not spent on a dock or a
/// menu bar — so a window already against it simply keeps its size and
/// the column takes its room from the page, which is what every face
/// did before this. Growing past the edge would put the thing the
/// person just opened where they cannot see it.
///
/// The origin is left alone: this window grows rightwards, and moving
/// it as well would be khor rearranging a window somebody placed.
#[cfg(desktop)]
#[tauri::command]
fn widen(window: tauri::Window, delta: i32) -> Result<(), String> {
    use tauri::PhysicalSize;
    if delta == 0 {
        return Ok(());
    }
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    if scale <= 0.0 {
        return Err("the window has no scale factor yet".into());
    }
    // **Physical throughout.** `outer_size`, `outer_position` and the
    // monitor's work area are all physical pixels; `delta` is the
    // face's own CSS pixels, which are logical, so it is the one number
    // that gets scaled. Mixing the two reads as a window that grows by
    // the right amount on a 1× screen and twice that on a retina one.
    let size = window.outer_size().map_err(|e| e.to_string())?;
    let want = (size.width as f64 + delta as f64 * scale).max(1.0);
    // The right edge this window would end at, against the edge the
    // screen has. `current_monitor` answers `None` on a window the
    // platform has not placed yet, and then there is no ceiling to
    // apply — asking for the room is still the right move, the window
    // manager is the one that says no.
    let ceiling = match window.current_monitor().map_err(|e| e.to_string())? {
        Some(mon) => {
            let area = mon.work_area();
            let left = window.outer_position().map_err(|e| e.to_string())?.x as f64;
            (area.position.x as f64 + area.size.width as f64 - left).max(1.0)
        }
        None => want,
    };
    let wide = want.min(ceiling);
    // Nothing to do rather than a set_size that changes nothing: a
    // resize is a layout in the webview, and a window already at the
    // screen's edge would take one on every press.
    if (wide - size.width as f64).abs() < 1.0 {
        return Ok(());
    }
    window
        .set_size(PhysicalSize::new(wide.round() as u32, size.height))
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(not(desktop))]
#[tauri::command]
fn widen(delta: i32) -> Result<(), String> {
    let _ = delta;
    Ok(())
}

/// What one machine's editor window is called
/// (工作区第六轮 第 46 条).
///
/// **Keyed by the machine, never by the name on the glass.** A label is
/// this app's handle on a window and a handle has to hold still; the
/// shown name is one a person can retype (`EditableName`), and renaming
/// a machine with its editor open would mint a second window and leave
/// the first one unreachable for the life of the app.
///
/// Tauri labels take letters, digits, `-`, `/`, `:` and `_`, so
/// anything else in a channel name becomes `_`. Two machines whose
/// names differ only in a punctuation mark would then share a window —
/// which is a worse answer than refusing, and it is still the right one
/// here: channel names are khor's own and hex.
#[cfg(desktop)]
fn code_label(machine: &str) -> String {
    let kept: String = machine
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!("code:{kept}")
}

/// 「系统验证」on the lock screen: Touch ID, the watch, or the login
/// password — whatever this Mac is set up to accept (docs/MORE.md 安全).
///
/// **The one door that is the app's alone**, and it is an *extra* rather
/// than the key: a PIN must be set before the lock can come on at all
/// (`khor_node::lock`), because a phone opening the web face has no
/// LocalAuthentication to fall back on. So a browser does not answer
/// this command and does not draw its button (`command_tables`'s
/// `ONE_SIDED`).
///
/// # Raw `objc2`, and what that buys
///
/// `LAContext`'s check is asynchronous — it puts a sheet on the screen
/// and calls back — so the reply block signals a semaphore this side
/// waits on, inside `spawn_blocking`. **Never on the event loop**: that
/// is the thread AppKit draws the sheet from, and waiting there is the
/// deadlock `open_code_window` already carries a note about.
///
/// `objc2` and `block2` are both already in the tree (tao/wry bring
/// them), so this adds no package — `objc2-local-authentication` would
/// have, for one class and two selectors.
///
/// **A refusal is `false`, never an error.** Cancelling the sheet, a Mac
/// with no biometry, a policy the machine will not evaluate — all of
/// them mean「not opened」, and a lock screen that told them apart would
/// be telling somebody about the security hardware of a machine they
/// have not unlocked.
#[cfg(target_os = "macos")]
#[tauri::command]
async fn system_unlock(reason: String) -> bool {
    tokio::task::spawn_blocking(move || unsafe {
        use objc2::rc::Retained;
        use objc2::runtime::AnyObject;
        use objc2::{class, msg_send};
        use objc2_foundation::NSString;

        let ctx: *mut AnyObject = msg_send![class!(LAContext), new];
        if ctx.is_null() {
            return false;
        }
        let ctx: Retained<AnyObject> = Retained::from_raw(ctx).unwrap();
        // `deviceOwnerAuthentication` (2), not `…WithBiometrics` (1):
        // the first falls back to the login password, which is what
        // 「系统验证」 says and what a desktop Mac with no sensor has.
        const OWNER_AUTHENTICATION: isize = 2;
        let why = NSString::from_str(&reason);

        // The block signals; this thread waits. A channel would need the
        // block to own a Rust value across the ObjC boundary, and a
        // semaphore is the shape Apple's own samples use.
        let sem: *mut AnyObject = msg_send![class!(NSObject), alloc];
        let _ = sem;
        let done = std::sync::Arc::new((
            std::sync::Mutex::new(None::<bool>),
            std::sync::Condvar::new(),
        ));
        let theirs = done.clone();
        let reply = block2::RcBlock::new(move |ok: objc2::runtime::Bool, _err: *mut AnyObject| {
            let (lock, cv) = &*theirs;
            *lock.lock().unwrap() = Some(ok.as_bool());
            cv.notify_all();
        });
        let _: () = msg_send![
            &*ctx,
            evaluatePolicy: OWNER_AUTHENTICATION,
            localizedReason: &*why,
            reply: &*reply,
        ];

        // **Bounded.** A sheet nobody answers must not hold a blocking
        // thread for the life of the app; the timeout reads as「not
        // opened」, which is the same thing every other refusal reads as.
        let (mutex, cv) = &*done;
        let mut got = mutex.lock().unwrap();
        while got.is_none() {
            let (next, timed_out) = cv
                .wait_timeout(got, std::time::Duration::from_secs(60))
                .unwrap();
            got = next;
            if timed_out.timed_out() {
                break;
            }
        }
        got.unwrap_or(false)
    })
    .await
    .unwrap_or(false)
}

/// Every platform that has no LocalAuthentication answers 「not opened」.
///
/// **A stub rather than a cfg'd handler list**: `generate_handler!` is
/// one macro call and a list that differed per platform would be a
/// second table of commands, which is the thing `command_tables` exists
/// to prevent. The face does not draw the button here anyway — it asks
/// whether this is the app, and the app is macOS-only for this one door.
#[cfg(not(target_os = "macos"))]
#[tauri::command]
async fn system_unlock(reason: String) -> bool {
    let _ = reason;
    false
}

/// Gives one NSWindow the appearance NSApp currently resolves to — the
/// desktop's, since khor never sets NSApp's own (`open_code_window`).
#[cfg(target_os = "macos")]
fn wear_the_apps_appearance(win: *mut std::ffi::c_void) {
    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send};
    unsafe {
        let nsapp: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
        let effective: *mut AnyObject = msg_send![nsapp, effectiveAppearance];
        if effective.is_null() {
            return;
        }
        let w = win as *mut AnyObject;
        let _: () = msg_send![w, setAppearance: effective];
    }
}

/// Gives one NSWindow the named appearance for a theme — what the main
/// window's `ThemeChanged` hands on, because a window with an appearance
/// of its own no longer follows the desktop by itself.
#[cfg(target_os = "macos")]
fn wear_appearance(win: *mut std::ffi::c_void, theme: tauri::Theme) {
    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send};
    let name = match theme {
        tauri::Theme::Dark => c"NSAppearanceNameDarkAqua",
        _ => c"NSAppearanceNameAqua",
    };
    unsafe {
        let ns: *mut AnyObject = msg_send![class!(NSString), stringWithUTF8String: name.as_ptr()];
        let appearance: *mut AnyObject = msg_send![class!(NSAppearance), appearanceNamed: ns];
        if appearance.is_null() {
            return;
        }
        let w = win as *mut AnyObject;
        let _: () = msg_send![w, setAppearance: appearance];
    }
}

/// How far the editor's window is offset from khor's own, in logical
/// pixels. **Offset rather than stacked**: a window that opened exactly
/// over the one that opened it reads as khor having gone away.
#[cfg(desktop)]
const CODE_STEP: f64 = 32.0;

/// Opens that machine's VS Code in **a window of its own**, or brings
/// the one already open to the front (工作区第六轮 第 46 条:「VSCODE做成
/// 弹出式的, 弹出一个窗口显示vscode」— the user).
///
/// **It was a column until this ruling** (第四轮 第 45 条) and before
/// that a pane under the tab strip. What both shapes cost is the thing
/// the user measured by opening them together:「同时打开session和vscode,
/// 排版就乱了」— an editor is a whole application, and it was being
/// asked to live inside a third of a window beside a file tree.
///
/// **Not a node call, for `widen`'s reason**: `khor-gui-core` holds
/// what the CLI can also do, and a window is not something `khor` has.
///
/// **An `async` command on purpose.** Building a window blocks until
/// the event loop has made it, so it must not be called *from* the
/// event loop — and tauri runs synchronous commands on the main thread.
/// This one runs on the pool, where the wait is safe.
///
/// The size is khor's own window's, offset by [`CODE_STEP`]: an editor
/// is the same kind of thing as the window it was opened from, so it
/// gets the same room rather than a number chosen here.
#[cfg(desktop)]
#[tauri::command]
async fn open_code_window(
    app: tauri::AppHandle,
    machine: String,
    url: String,
    title: String,
) -> Result<(), String> {
    use tauri::{Emitter, Manager};
    let label = code_label(&machine);
    // **Already open is not opened again** (「已开就不再开第二个」). A
    // second window onto one editor would be two views of one document
    // with no way to tell which is which.
    if let Some(open) = app.get_webview_window(&label) {
        let _ = open.unminimize();
        open.show().map_err(|e| e.to_string())?;
        open.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }
    let at: tauri::Url = url.parse().map_err(|e| format!("{url}: {e}"))?;
    let mut build = tauri::WebviewWindowBuilder::new(&app, &label, tauri::WebviewUrl::External(at))
        .title(&title);
    if let Some(main) = app.get_webview_window("main") {
        let scale = main.scale_factor().unwrap_or(1.0).max(0.1);
        if let (Ok(size), Ok(pos)) = (main.outer_size(), main.outer_position()) {
            build = build
                .inner_size(size.width as f64 / scale, size.height as f64 / scale)
                .position(
                    pos.x as f64 / scale + CODE_STEP,
                    pos.y as f64 / scale + CODE_STEP,
                );
        }
    }
    let made = build.build().map_err(|e| e.to_string())?;
    // **The title bar wears khor's own light or dark** (2026-09-18:
    // 「弹出的vscode, 页面的标题和实际颜色模式不一, 比如这里是深色, 但弹出
    // 是浅色」— the user). The page inside follows the system through
    // the server's machine settings (`khor_node::code`); the window's
    // chrome did not. Measured, not reasoned: a window built here with
    // no appearance of its own came up with a light title bar over a
    // dark page while `effectiveAppearance` on it answered DarkAqua —
    // AppKit resolves a window's appearance lazily, and this one was
    // drawn before it had been. Setting the appearance outright is the
    // deterministic form of what merely reading it happened to do.
    //
    // **NSApp's appearance, on the main thread** (2026-09-27). Since
    // `set_window_theme` wears the choice on NSApp, that is what this
    // window copies; `auto` leaves NSApp clear, so it copies the desktop.
    // This is an async command — off the main thread — and AppKit only
    // honours `setAppearance:` there (the same fault that left the main
    // window's sidebar light under a dark page, ebacbe2).
    #[cfg(target_os = "macos")]
    {
        let for_main = made.clone();
        let _ = app.run_on_main_thread(move || {
            if let Ok(win) = for_main.ns_window() {
                wear_the_apps_appearance(win);
            }
        });
    }
    // **The switch goes dark when the window does** (「窗口关掉 = 图标
    // 灭」). Nothing else can tell the face: the window is not the
    // face's to watch, and a face that polled for it would be asking a
    // question the platform already answers.
    let home = app.clone();
    made.on_window_event(move |event| {
        if matches!(event, tauri::WindowEvent::Destroyed) {
            let _ = home.emit_to("main", "code:closed", machine.clone());
        }
    });
    Ok(())
}

#[cfg(not(desktop))]
#[tauri::command]
async fn open_code_window(machine: String, url: String, title: String) -> Result<(), String> {
    let _ = (machine, url, title);
    // A phone has no second window to put an editor in, and the face
    // draws no switch there anyway (`components/CodeWindow`).
    Err(khor_catalog::msg::window_needs_desktop().into())
}

// **The mobile halves are refusals, and unlike the last one that stood
// here they are true.** `add_child` is gated `all(desktop, feature =
// "unstable")` and there is no mobile equivalent: a page on a phone needs a
// native plugin adding a WKWebView or a WebView by hand (定形 单元 3/4).
// The one this file used to carry refused Linux and Windows for a thing
// they could already do; this one refuses a thing that genuinely is not
// there yet.
#[cfg(not(desktop))]
#[tauri::command]
async fn open_page(
    tab: String,
    machine: String,
    url: String,
    at: PageBox,
    agent: String,
) -> Result<khor_gui_core::web::WebBorrow, String> {
    let _ = (tab, machine, url, at, agent);
    Err(khor_catalog::msg::page_needs_desktop().into())
}

#[cfg(not(desktop))]
#[tauri::command]
fn move_page(tab: String, at: PageBox) -> Result<(), String> {
    let _ = (tab, at);
    Ok(())
}

#[cfg(not(desktop))]
#[tauri::command]
fn hide_page(tab: String) -> Result<(), String> {
    let _ = tab;
    Ok(())
}

#[cfg(not(desktop))]
#[tauri::command]
fn close_page(tab: String) -> Result<(), String> {
    let _ = tab;
    Ok(())
}

#[cfg(not(desktop))]
#[tauri::command]
fn show_page(tab: String) -> Result<(), String> {
    let _ = tab;
    Ok(())
}

#[cfg(not(desktop))]
#[tauri::command]
fn page_step(tab: String, way: String) -> Result<(), String> {
    let _ = (tab, way);
    Ok(())
}

/// Answers with the ticket **and** the window it is good for: the window
/// belongs to the library that enforces it, not to the dialog that
/// prints it (`khor_gui_core::Ticket`).
/// 接管 (批C): ends the session's terminal side; the conversation
/// continues here (`khor_gui_core::takeover`).
#[tauri::command]
fn takeover(id: String) -> Result<(), String> {
    khor_gui_core::takeover(&Node::root_from_env(), &id)
}

#[tauri::command]
fn takeover_term(id: String) -> Result<(), String> {
    khor_gui_core::takeover_term(&Node::root_from_env(), &id)
}

/// The wizard's door (会话身份批B): a fresh agent session, born in the
/// chosen directory, as a conversation (`chat`) or a terminal (`term`);
/// `agent` is the wizard's 智能体 answer (claude when absent).
#[tauri::command]
async fn open_session(
    dir: String,
    title: Option<String>,
    form: String,
    agent: Option<String>,
    machine: Option<String>,
) -> Result<String, String> {
    khor_gui_core::open_session(
        &Node::root_from_env(),
        &dir,
        title.as_deref().unwrap_or(""),
        &form,
        agent.as_deref().unwrap_or(""),
        machine.as_deref().unwrap_or(""),
    )
    .await
}

/// The machine card's 在这台开终端 — `open --on`, through the same
/// core call the bridge answers with (docs/KHOR.md equivalence).
#[tauri::command]
async fn open_on(
    machine: String,
    title: Option<String>,
    cwd: Option<String>,
) -> Result<String, String> {
    khor_gui_core::open_term_on(
        &Node::root_from_env(),
        &machine,
        title.as_deref().unwrap_or(""),
        cwd.as_deref().unwrap_or(""),
    )
    .await
}

/// A machine conversation's readout — `khor log`, shaped for the pane.
#[tauri::command]
fn machine_log(id: String) -> Result<khor_gui_core::MachineLog, String> {
    khor_gui_core::machine_log(&Node::root_from_env(), &id)
}

/// The waiting file message's 收下 — `khor accept`, same core call.
#[tauri::command]
async fn accept_transfer(msg: String) -> Result<Vec<String>, String> {
    khor_gui_core::accept_transfer(&Node::root_from_env(), &msg).await
}

/// A path this machine already has, straight to `khor send`'s call —
/// what a tauri file drop hands over, and what the bridge answers for
/// browser verification. The picker flow is `pick_and_send` below.
#[tauri::command]
async fn send_path(machine: String, path: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        khor_gui_core::send_path(
            &Node::root_from_env(),
            &machine,
            std::path::Path::new(&path),
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

/// The composer's 发文件, desktop form: a native picker answers with a
/// real local path and the path goes straight to `khor send`'s own call
/// — no upload exists here, which is the whole advantage of this form
/// (the browser faces stage bytes first; `khor_gui_core::send_upload`).
/// `None` = the person cancelled the picker, which is not an error and
/// must not wear one.
///
/// Rust-side dialog on purpose (`DialogExt`, the official v2 plugin):
/// the webview's own <input type=file> deliberately never reveals a
/// path, and driving the plugin from here keeps the frontend free of
/// the plugin's JS package and its capability grant.
#[tauri::command]
async fn pick_and_send(app: tauri::AppHandle, machine: String) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let Some(picked) = app.dialog().file().blocking_pick_file() else {
        return Ok(None);
    };
    let path = picked.into_path().map_err(|e| e.to_string())?;
    // Digesting a large file is real work; off the async runtime.
    tokio::task::spawn_blocking(move || {
        khor_gui_core::send_path(&Node::root_from_env(), &machine, &path)
    })
    .await
    .map_err(|e| e.to_string())?
    .map(Some)
}

/// The wizard's 智能体 list (批⑥): every ACP agent this person named.
/// khor's own two are not in it — they are not registrations, and the
/// face puts them beside this list rather than inside it.
#[tauri::command]
fn agents() -> Result<Vec<khor_gui_core::AgentRow>, String> {
    khor_gui_core::agents(&Node::root_from_env())
}

/// The ACP registry snapshot khor ships with, and when it was taken —
/// the same data `khor agents known` prints. The wizard is the only
/// way somebody who never opens a terminal finds out these exist.
#[tauri::command]
fn agents_known() -> Vec<khor_gui_core::agents::KnownAgent> {
    khor_gui_core::agents::agents_known()
}

#[tauri::command]
fn agents_known_taken() -> (String, usize, usize) {
    khor_gui_core::agents::agents_known_taken()
}

/// Attaches a snapshot entry, **by id**: the command is looked up on
/// the far side, never sent from the face.
#[tauri::command]
fn register_known_agent(id: String) -> Result<(), String> {
    khor_gui_core::agents::register_known(&Node::root_from_env(), &id)
}

/// What this agent was recorded as saying — no process (0.017s).
#[tauri::command]
fn agent_seen(name: String) -> Result<khor_gui_core::agents::AgentSaid, String> {
    khor_gui_core::agents::agent_seen(&Node::root_from_env(), &name)
}

/// Puts the question to the agent itself. **Starts it — measured
/// 1.5s with the package cached, 5.64s without** — so it hangs off a
/// person's press and nothing else.
#[tauri::command]
fn agent_ask(name: String) -> Result<khor_gui_core::agents::AgentSaid, String> {
    khor_gui_core::agents::agent_ask(&Node::root_from_env(), &name)
}

/// **Windows draws its own three, and these are what they press**
/// (docs/SHELL.md 第十轮). macOS has the system's buttons inside the row
/// and Linux keeps its decorations, so nothing there calls these.
#[cfg(desktop)]
#[tauri::command]
fn window_minimize(window: tauri::Window) -> Result<(), String> {
    window.minimize().map_err(|e| e.to_string())
}

#[cfg(desktop)]
#[tauri::command]
fn window_maximize_toggle(window: tauri::Window) -> Result<(), String> {
    let full = window.is_maximized().map_err(|e| e.to_string())?;
    if full {
        window.unmaximize().map_err(|e| e.to_string())
    } else {
        window.maximize().map_err(|e| e.to_string())
    }
}

/// **`close`, not `destroy`**: closing is a request, and the draft guard
/// (`onQuitRequested`) is hung on exactly that — a destroy would take a
/// half-written message with it.
#[cfg(desktop)]
#[tauri::command]
fn window_close(window: tauri::Window) -> Result<(), String> {
    window.close().map_err(|e| e.to_string())
}

// **The mobile halves refuse, in words.** A phone's one window is the
// system's to minimize and close; answering `Ok` would tell the face a
// thing happened that did not. The face draws none of the three there
// (`windowsApp` is the app on Windows alone), so this is what a stray
// call hears, not something a person meets.
#[cfg(not(desktop))]
#[tauri::command]
fn window_minimize() -> Result<(), String> {
    Err(khor_catalog::msg::window_needs_desktop().into())
}

#[cfg(not(desktop))]
#[tauri::command]
fn window_maximize_toggle() -> Result<(), String> {
    Err(khor_catalog::msg::window_needs_desktop().into())
}

#[cfg(not(desktop))]
#[tauri::command]
fn window_close() -> Result<(), String> {
    Err(khor_catalog::msg::window_needs_desktop().into())
}

/// 全局搜索 (docs/SHELL.md 第十轮): one word over everything khor holds.
#[tauri::command]
fn find(word: String) -> Result<Vec<khor_gui_core::FindHit>, String> {
    khor_gui_core::find(&Node::root_from_env(), &word)
}

/// 导入 (docs/SHELL.md 第八轮): what could be brought onto the list.
#[tauri::command]
fn import_candidates() -> Result<Vec<khor_gui_core::import::CandidateRow>, String> {
    khor_gui_core::import::import_candidates(&Node::root_from_env())
}

/// …and bringing it: the session ids, in order.
#[tauri::command]
fn import_sessions(ids: Vec<String>) -> Result<Vec<String>, String> {
    khor_gui_core::import::import_sessions(&Node::root_from_env(), &ids)
}

/// Opens a conversation the agent already has, at **its own** cwd —
/// not a fresh one, and not the wizard's 目录 answer.
#[tauri::command]
fn resume_session(agent: String, session: String) -> Result<String, String> {
    khor_gui_core::agents::resume_session(&Node::root_from_env(), &agent, &session)
}

// ── 安全 (docs/MORE.md) ─────────────────────────────────────────────

/// How this machine's faces are set to lock (`khor_gui_core::lock_state`).
#[tauri::command]
fn lock_state() -> khor_gui_core::LockFacts {
    khor_gui_core::lock_state(&Node::root_from_env())
}

#[tauri::command]
fn set_lock(on: bool) -> Result<khor_gui_core::LockFacts, String> {
    khor_gui_core::set_lock(&Node::root_from_env(), on)
}

#[tauri::command]
fn set_lock_pin(pin: String) -> Result<khor_gui_core::LockFacts, String> {
    khor_gui_core::set_lock_pin(&Node::root_from_env(), &pin)
}

#[tauri::command]
fn set_lock_timeout(minutes: u32) -> Result<khor_gui_core::LockFacts, String> {
    khor_gui_core::set_lock_timeout(&Node::root_from_env(), minutes)
}

/// Whether this is the PIN. `false` is a wrong PIN, not a failure.
#[tauri::command]
fn unlock(pin: String) -> Result<bool, String> {
    khor_gui_core::unlock(&Node::root_from_env(), &pin)
}

#[tauri::command]
fn web_key() -> Result<Option<String>, String> {
    khor_gui_core::web_key(&Node::root_from_env())
}

#[tauri::command]
fn rotate_web_key() -> Result<String, String> {
    khor_gui_core::rotate_web_key(&Node::root_from_env())
}

#[tauri::command]
fn reset_identity(name: String) -> Result<(), String> {
    khor_gui_core::reset_identity(&Node::root_from_env(), &name)
}

/// 诊断's checklist (`khor_gui_core::doctor`). Async: each check has a
/// three-second budget of its own, and a blocking command would hold the
/// window for however long the slowest one takes.
#[tauri::command]
async fn doctor() -> Result<Vec<khor_gui_core::DoctorRow>, String> {
    khor_gui_core::doctor(&Node::root_from_env()).await
}

/// One check's remedy, by the check's own key.
#[tauri::command]
fn doctor_fix(key: String) -> Result<String, String> {
    khor_gui_core::doctor_fix(&Node::root_from_env(), &key)
}

/// Which khor this is, and which store it is talking to
/// (`khor_gui_core::about`) — the identity card's foot.
#[tauri::command]
fn about() -> khor_gui_core::About {
    khor_gui_core::about(&Node::root_from_env())
}

// ── 设置 (docs/MORE.md) ─────────────────────────────────────────────

/// Every setting and its value (`khor_gui_core::config_rows`). No node
/// is opened, so this answers before anything has been drawn.
/// The theme the face has resolved, worn by **the window** — which is
/// not the page (批 D2, `src/theme.ts`).
///
/// Three things on screen are AppKit's rather than the document's: the
/// ground behind the webview, the title bar, and every editor window
/// khor has opened. None of them follows `data-theme`, and a window
/// that kept the desktop's appearance while the page went light is the
/// half-and-half screen this whole key exists to avoid.
///
/// **Takes the choice, and wears it on the app, not on one window**
/// (2026-09-27). `dark` / `light` set NSApp's appearance; `auto` clears
/// it, so the whole app — every window, and the sidebar material under
/// the left block — follows the desktop by itself, and the page's
/// `prefers-color-scheme` follows with it.
///
/// It used to set the resolved theme on each window. The owner twice
/// found the left block light under a dark page: a window's own
/// appearance was being reset to nil behind khor's back (measured — it
/// read `nil` when the window next came to the front), and a nil window
/// follows NSApp, which nothing had set. Set on NSApp there is nothing to
/// fall back to but the choice. The windows' own appearance is cleared
/// so none of them can hold on to an older one. And `auto` resolved into
/// a fixed window theme had a second fault: the desktop turning dark at
/// sunset no longer reached a window that had been told 「light」.
#[tauri::command]
fn set_window_theme(app: tauri::AppHandle, theme: String) {
    #[cfg(target_os = "macos")]
    {
        use tauri::Manager;
        let wanted = match theme.as_str() {
            "dark" => Some(tauri::Theme::Dark),
            "light" => Some(tauri::Theme::Light),
            _ => None,
        };
        // AppKit only on the main thread (ebacbe2).
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || {
            wear_on_the_app(wanted);
            for (label, w) in handle.webview_windows() {
                let Ok(win) = w.ns_window() else { continue };
                if label == "main" {
                    follow_the_app(win);
                } else if label.starts_with("code:") {
                    // An editor window is given NSApp's appearance
                    // outright: one left to resolve it lazily came up
                    // with a light title bar over a dark page (2026-09-18).
                    wear_the_apps_appearance(win);
                }
            }
        });
        // **And put the three buttons back.** An appearance change makes
        // AppKit lay the title bar out again, which returns the buttons
        // to its default spot — 8 above the row's centre, off the rail's
        // glyph box (2026-09-27, the owner: 「系统的三个按钮和每个侧边导航
        // 没有对齐」; and since startup wears a theme, they were off from
        // the first frame). tao insets them only from its view's
        // `drawRect:`, which a theme change does not reach — so
        // `traffic::place` does tao's moves itself, queued
        // behind the theme change on the main thread.
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || {
            if let Some(w) = handle.get_webview_window("main")
                && let Ok(win) = w.ns_window()
            {
                traffic::place(win);
            }
        });
    }
    #[cfg(not(target_os = "macos"))]
    let _ = (app, theme);
}

/// Sets NSApp's appearance **now** — `None` clears it, so the app follows
/// the desktop. Not tauri's `AppHandle::set_theme`: that posts to the
/// event loop, and the editor windows read NSApp's appearance right
/// after this and would copy the old one. Main thread only.
#[cfg(target_os = "macos")]
fn wear_on_the_app(theme: Option<tauri::Theme>) {
    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send};
    unsafe {
        let nsapp: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
        let appearance: *mut AnyObject = match theme {
            Some(t) => {
                let name = match t {
                    tauri::Theme::Dark => c"NSAppearanceNameDarkAqua",
                    _ => c"NSAppearanceNameAqua",
                };
                let ns: *mut AnyObject = msg_send![class!(NSString), stringWithUTF8String: name.as_ptr()];
                msg_send![class!(NSAppearance), appearanceNamed: ns]
            }
            None => std::ptr::null_mut(),
        };
        let _: () = msg_send![nsapp, setAppearance: appearance];
    }
}

/// Clears one NSWindow's own appearance, so it wears NSApp's.
#[cfg(target_os = "macos")]
fn follow_the_app(win: *mut std::ffi::c_void) {
    use objc2::msg_send;
    use objc2::runtime::AnyObject;
    unsafe {
        let w = win as *mut AnyObject;
        let none: *mut AnyObject = std::ptr::null_mut();
        let _: () = msg_send![w, setAppearance: none];
    }
}

#[tauri::command]
fn config() -> Vec<khor_gui_core::ConfigRow> {
    khor_gui_core::config_rows(&Node::root_from_env())
}

/// Sets one, and answers the whole list again. Refused by name for an
/// unknown key or a value outside that key's ladder.
///
/// **语言 rebuilds the menu bar** (`menu::follow_lang`): the menu's words
/// are read when it is built, and `set_config` is where this process
/// learns the language changed (`khor_gui_core::set_config` re-applies it).
#[tauri::command]
fn set_config(
    app: tauri::AppHandle,
    name: String,
    value: String,
) -> Result<Vec<khor_gui_core::ConfigRow>, String> {
    let rows = khor_gui_core::set_config(&Node::root_from_env(), &name, &value)?;
    #[cfg(desktop)]
    if name == "lang" {
        menu::follow_lang(&app);
    }
    #[cfg(not(desktop))]
    let _ = app;
    Ok(rows)
}

/// The page's word on which menu-bar items apply right now — ⌘W with
/// something to close, two tabs to step between, the landing's 「+」 by
/// name, a lock with a key (`src/lib/shortcuts.ts`, sent on change).
/// A no-op everywhere but macOS, which is the one face with a menu bar.
#[cfg(desktop)]
#[tauri::command]
fn menu_state(app: tauri::AppHandle, state: menu::MenuState) {
    menu::set_state(&app, state);
}

#[cfg(not(desktop))]
#[tauri::command]
fn menu_state() {}

#[tauri::command]
fn invite() -> Result<khor_gui_core::Ticket, String> {
    khor_gui_core::invite(&Node::root_from_env())
}

/// Async on purpose: pairing dials, and a dial can sit on the timeout —
/// a blocking command would freeze the window for those seconds.
#[tauri::command]
async fn pair(ticket: String) -> Result<String, String> {
    khor_gui_core::pair(&Node::root_from_env(), &ticket).await
}

/// Who waits for the root holder (docs/CONNECT.md 根不在场).
#[tauri::command]
fn pending_list() -> Result<khor_gui_core::Waiting, String> {
    khor_gui_core::waiting(&Node::root_from_env())
}

/// The holder signs one waiting machine in. Async for `pair`'s reason:
/// the roster is pushed out, which dials.
#[tauri::command]
async fn approve(machine: String) -> Result<khor_gui_core::Waiting, String> {
    khor_gui_core::approve(&Node::root_from_env(), &machine).await
}

/// The root's words, for 「…」 → 安全's 「显示…」 — app-only
/// (`khor_gui_core`'s `tests/command_tables.rs`). Since 2026-10-01 nothing
/// asks for them on its own: the first-start prompt and its two questions
/// (`root_unshown`, `root_words_seen`) are gone.
#[tauri::command]
fn root_words() -> Result<String, String> {
    khor_gui_core::root_words(&Node::root_from_env())
}

/// This machine leaves the network (docs/MORE.md 离开). Async for
/// `pair`'s reason and one more: it pushes the news to every machine in
/// the table before it empties it, so the slow case is every dial's
/// timeout in a row.
#[tauri::command]
async fn leave(machine: String) -> Result<khor_gui_core::LeaveOutcome, String> {
    khor_gui_core::leave(&Node::root_from_env(), &machine).await
}

/// This machine pauses or resumes (docs/MORE.md 暂停). Not async: it
/// writes one small file, and the resident serve is what reads it.
///
/// It answers the fresh `About` because pausing changes a fact the card
/// is already showing, and re-reading it is how the two stay one answer.
#[tauri::command]
fn set_paused(on: bool) -> Result<khor_gui_core::About, String> {
    let root = Node::root_from_env();
    khor_gui_core::set_paused(&root, on)?;
    Ok(khor_gui_core::about(&root))
}

/// Pauses or resumes talking to one machine, by name (SHELL.md 第五轮 第
/// 3 条) — the row menu's 暂停 / 恢复. Not async, `set_paused`'s reason.
/// Answers the sentence; the row's grey comes from the next device list.
#[tauri::command]
fn set_machine_paused(machine: String, on: bool) -> Result<String, String> {
    khor_gui_core::set_machine_paused(&Node::root_from_env(), &machine, on)
}

/// Whether this face can read a pairing ticket's QR code with a camera
/// (加入网络's 扫码, docs/IOS.md 第三步): the phone builds carry the
/// scanner, a desktop does not. **A fact of the build, not a guess from
/// the browser** — the same footing as `khor_core::hosts_here` — and a
/// face that hears no draws no entry (absent, not refused). Every HTTP
/// skin answers no (`khor_gui_core::api`).
#[tauri::command]
fn scanner_here() -> bool {
    cfg!(mobile)
}

/// The phone is back in front (`WindowEvent::Resumed`): new ears for the
/// node, far attachments asked for again on them, and — once the round
/// that follows has come back — a word to the page that what it lists is
/// worth reading again (`node:woken`, `src/hooks/use-woken.ts`). Off the
/// main thread: it waits on the network, and UIKit is waiting on this
/// handler.
#[cfg(not(desktop))]
fn resume_the_node(app: tauri::AppHandle) {
    use tauri::Emitter;
    std::thread::spawn(move || {
        let Ok(rt) = tokio::runtime::Builder::new_current_thread().enable_all().build() else {
            return;
        };
        let begun = std::time::Instant::now();
        let root = Node::root_from_env();
        let outcome = rt.block_on(khor_gui_core::rejoin::resume(&root));
        let ms = begun.elapsed().as_millis() as u64;
        match &outcome {
            Some(Ok(out)) => eprintln!("khor resume: round back in {ms} ms: {out:?}"),
            Some(Err(why)) => eprintln!("khor resume: {why} ({ms} ms)"),
            None => eprintln!("khor resume: no resumable serve on this home"),
        }
        let _ = app.emit_to("main", "node:woken", ms);
    });
}

/// The app. On a phone the system starts it through this same function:
/// `mobile_entry_point` exports the `start_app` symbol the Xcode project
/// in `gen/apple` calls — without it the iOS library links and nothing
/// ever runs it.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // **Stand somewhere that cannot be deleted, before anything else**
    // (docs/SHELL.md 第十四轮 第 4 条). The install script starts this
    // binary with `nohup` from inside the export directory and then
    // removes that directory, so the app spends its whole life standing
    // nowhere — and `gix` goes through the current directory on its way
    // to a repository, which is why the workspace printed 「… does not
    // appear to be a git repository」 over a plain checkout. The app is
    // the process that owns its own directory, so it is the one that
    // moves outright; `Node::open` only rescues a process that is
    // already adrift.
    khor_node::cwd::pin();
    // 语言, before anything is drawn or printed (docs/MORE.md 语言).
    // The face resolves its own (`src/lang.ts`, from the same key); this
    // is the Rust half — every sentence that crosses back as text, a
    // refusal or a 诊断 row, is written in it.
    khor_node::config::apply_lang(&Node::root_from_env());
    // If this process was re-exec'd as a session host, be one — the app
    // spawns hosts through its own binary (`host.rs main_if_host`).
    khor_node::host::main_if_host(Node::root_from_env());
    // AppKit's own menu items in the language just chosen — before the
    // builder brings AppKit up, which settles it (`menu::speak_at_launch`).
    // **After `main_if_host`**: a session host draws no menu.
    #[cfg(desktop)]
    menu::speak_at_launch();
    // 本机开着哪几张脸 (docs/MORE.md 头像页): the desktop app is the one
    // face nothing else on this machine can see, so it says so itself.
    // **After `main_if_host`**, or a re-exec'd session host would claim
    // to be the window. Failing to write it costs a row on one card and
    // nothing else, so it does not stop the app coming up.
    //
    // **Not on a phone** (docs/IOS.md 第二步): its app is not the desktop
    // app, and a pid file saying so made the phone's own card claim one.
    #[cfg(desktop)]
    let _ = khor_node::face::app_here(&Node::root_from_env());
    // **The phone's machine fingerprint, asked here, on the main thread**
    // (`khor_node::hardware`): UIKit hands `identifierForVendor` out on
    // the main thread only, and this is the one moment the app is
    // certainly on it — every later ask, from the serve's thread or a
    // command's, reads the kept answer instead of queueing onto main.
    #[cfg(target_os = "ios")]
    let _ = khor_node::hardware::fingerprint();

    // The app embeds serve — the desktop is a full node, not a client
    // (one mesh, no client/server split). Running `khor serve` on the
    // same home at the same time is the known conflict; handover is on
    // the ledger.
    //
    // **On a phone it is resumed, not restarted** (docs/IOS.md 第三步):
    // iOS suspends the app seconds after it leaves the screen, and
    // nothing stands a phone app back up — so its serve takes new ears in
    // place (`serve_resumable`, `on_window_event`'s `Resumed`), and a far
    // terminal or conversation whose pipe ends attaches again rather than
    // ending (`khor_gui_core::rejoin`). A desktop's serve and faces are
    // exactly what they were.
    #[cfg(mobile)]
    khor_gui_core::rejoin::set_on(true);
    std::thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        rt.block_on(async {
            match Node::open(Node::root_from_env()) {
                Ok(n) => {
                    #[cfg(desktop)]
                    let served = n.serve().await;
                    #[cfg(not(desktop))]
                    let served = n.serve_resumable().await;
                    if let Err(e) = served {
                        eprintln!("serve ended: {e}");
                    }
                }
                Err(e) => eprintln!("serve did not start: {e}"),
            }
        });
    });

    // The browser face rides with the resident, and the app is the
    // resident on a desktop somebody is sitting at. Not fatal if it
    // cannot bind: a busy port must not cost this machine its place in
    // the mesh, and `khor web` checks a live socket before printing a
    // link rather than trusting that this succeeded.
    //
    // **Not on a phone**: it is nobody's resident. The system suspends it
    // in the background (docs/CONNECT.md 手机后台), so a link to its face
    // would be a link that works only while the phone is lit in somebody's
    // hand — the phone is the thing that opens a face, not one that
    // serves it.
    #[cfg(desktop)]
    match khor_web::listen(Node::root_from_env(), khor_web::port()) {
        Ok(face) => eprintln!("khor web face: {}", face.addr),
        Err(e) => eprintln!("{e}"),
    }
    // This machine's readings, from before the first screen asks for
    // them (docs/SHELL.md 第十六轮): the machine page reads the
    // sampler's last round and never waits for a disk.
    khor_node::vitals::local::start(&Node::root_from_env());

    let builder = tauri::Builder::default().plugin(tauri_plugin_dialog::init());
    // 加入网络's 扫码 (docs/IOS.md 第三步): the camera reads a pairing
    // ticket's QR code on a phone. Its grant is `capabilities/scan.json`;
    // whether the face draws the entry at all is `scanner_here`.
    #[cfg(mobile)]
    let builder = builder.plugin(tauri_plugin_barcode_scanner::init());
    // **khor's own menu bar** (`menu.rs`), so tauri's default is not
    // built first: its ⌘W was 「Close Window」, and on khor's one window
    // that stops the node. A picked item goes back to the page as the act
    // it names, into the same function the page's keydown runs.
    #[cfg(desktop)]
    let builder = builder
        .enable_macos_default_menu(false)
        .on_menu_event(|app, event| menu::on_event(app, event.id().as_ref()));
    builder
        .setup(|app| {
            // **The window first**: everything below reaches for it by
            // name (`build_the_window` says why it is not in the config).
            #[cfg(desktop)]
            build_the_window(app.handle())?;
            // The menu bar (macOS; a no-op elsewhere). Its items and the
            // page's last word on them live in `menu::Held`.
            #[cfg(desktop)]
            {
                use tauri::Manager;
                app.manage(menu::Held::default());
                menu::install(app.handle())?;
            }
            // The window khor's own overlays are drawn in, hung off the
            // face and hidden until something has a list to show
            // (`hang_menu_window`).
            #[cfg(desktop)]
            hang_menu_window(app.handle());
            #[cfg(desktop)]
            paint_the_window_before_the_page(app.handle());
            // **A phone's one window, plain.** The config declares no
            // window (`build_the_window` says why), so nothing would be on
            // screen without this; everything that function sets is a
            // desktop window's — size, title bar, material, the three
            // buttons — and a phone's window is the screen.
            #[cfg(not(desktop))]
            tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::default()).build()?;
            Ok(())
        })
        // **The overlay window has to be closed by hand.** It has a level
        // of its own rather than a parent (`hang_menu_window` says why),
        // and tauri leaves an app running while any window is open — so
        // without this, closing khor's window would leave the process
        // alive with nothing on screen.
        .on_window_event(|window, event| {
            #[cfg(desktop)]
            if window.label() == "main" {
                use tauri::Manager;
                let menu = || window.app_handle().get_webview_window(MENU);
                match event {
                    // **The overlay window has to be closed by hand.** It
                    // has a level of its own rather than a parent
                    // (`hang_menu_window` says why), and tauri leaves an
                    // app running while any window is open — so without
                    // this, closing khor's window would leave the process
                    // alive with nothing on screen.
                    tauri::WindowEvent::CloseRequested { .. } => {
                        if let Some(menu) = menu() {
                            let _ = menu.close();
                        }
                        // **And so does every editor window** (工作区第
                        // 六轮 第 46 条). They are windows of khor's own
                        // with no parent, exactly as the overlay is, so
                        // closing khor with one open would leave the
                        // process alive behind somebody else's editor.
                        for (label, w) in window.app_handle().webview_windows() {
                            if label.starts_with("code:") {
                                let _ = w.close();
                            }
                        }
                    }
                    // **A menu belongs to the face it hangs under, and
                    // follows it out of the room.** The overlay floats at
                    // its own window level, which is what keeps it above
                    // the page — and would also have kept it above
                    // whatever somebody switched to mid-completion. It
                    // goes away with the face and comes back with it.
                    //
                    // `menu_up` is deliberately not touched: it records
                    // whether anybody still *wants* a menu, and switching
                    // apps is not an answer to that question. The face
                    // asks and unasks; this only draws or stops drawing.
                    // **The editor windows change with the desktop.**
                    // Each was built wearing the theme khor had at the
                    // time (`open_code_window`), and a theme set on a
                    // window stops following the system on its own —
                    // so the main window, which does follow, passes
                    // every change on.
                    tauri::WindowEvent::ThemeChanged(theme) => {
                        // **The desktop changed; what khor wears is
                        // still the choice** (批 D2). Passing `*theme`
                        // on unconditionally would make a machine set
                        // to 浅色 grow a dark title bar the moment the
                        // desktop turned, which is the half-and-half
                        // screen the key exists to prevent. `auto` — and
                        // a store this process cannot read — is the
                        // desktop, unchanged from before the key.
                        let wanted = match khor_node::config::get(&Node::root_from_env(), "theme") {
                            Ok("dark") => tauri::Theme::Dark,
                            Ok("light") => tauri::Theme::Light,
                            _ => *theme,
                        };
                        for (label, w) in window.app_handle().webview_windows() {
                            if label.starts_with("code:") {
                                #[cfg(target_os = "macos")]
                                if let Ok(win) = w.ns_window() {
                                    wear_appearance(win, wanted);
                                }
                                #[cfg(not(target_os = "macos"))]
                                let _ = (w, wanted);
                            }
                        }
                    }
                    tauri::WindowEvent::Focused(here) => {
                        if let Some(menu) = menu()
                            && menu_up().load(std::sync::atomic::Ordering::Relaxed)
                        {
                            let _ = if *here { menu.show() } else { menu.hide() };
                        }
                        // The page re-reads the store when the window is
                        // back in front (`use-settings`): WKWebView gives
                        // it no `focus` of its own for that.
                        if *here && window.label() == "main" {
                            use tauri::Emitter;
                            let _ = window.emit_to("main", "window:focused", ());
                            // …and so does the menu bar's language: a
                            // `khor config set lang` in a terminal reaches
                            // the page this way, and the menu follows it.
                            khor_node::config::apply_lang(&Node::root_from_env());
                            menu::follow_lang(window.app_handle());
                        }
                    }
                    _ => {}
                }
            }
            // **Back to the foreground** (docs/IOS.md 第三步). The signal is
            // UIKit's `applicationWillEnterForeground`, which tauri hands
            // every window as `WindowEvent::Resumed` — **not**
            // `RunEvent::Resumed`, which despite its name is the event
            // loop waking from a poll and fires all the time. Chosen over
            // the page's `visibilitychange` because it is native and
            // early: the node is given new ears before the page has run a
            // line, and nothing here depends on a page being alive to ask.
            // It fires only after a real trip to the background (not when
            // a notification is pulled down over the app). ⚠️ It comes
            // from the app delegate: an Info.plist that opts into the
            // UIScene lifecycle (`UIApplicationSceneManifest`) moves the
            // call to a scene delegate tao leaves empty, and this goes
            // silent — `gen/apple` does not opt in.
            #[cfg(not(desktop))]
            if window.label() == "main" && matches!(event, tauri::WindowEvent::Resumed) {
                use tauri::Manager;
                resume_the_node(window.app_handle().clone());
            }
        })
        .invoke_handler(tauri::generate_handler![
            tabs_list,
            tabs_open,
            tabs_go,
            tabs_focus,
            tabs_close,
            tabs_pin,
            tabs_save,
            sessions,
            widen,
            day_strips,
            workspaces,
            ws_add,
            ws_rm,
            ws_pin,
            touched,
            remove,
            mkdir,
            touch,
            rename_path,
            launch,
            search,
            open_code_window,
            trash_here,
            pdf_here,
            read_text,
            git_status,
            checkout,
            git_diff,
            write_text,
            merge_text,
            preview,
            code_here,
            code_open,
            code_stop,
            devices,
            load,
            quota_board,
            register_account,
            pin_account,
            rename_account,
            update_account,
            remove_account,
            usage,
            quota,
            seen,
            close_session,
            rename_session,
            rename_machine,
            relays,
            set_relays,
            road_share,
            tell,
            pin_session,
            pin_device,
            forget_device,
            set_watch,
            face_choices,
            restyle,
            hooks_state,
            install_hooks,
            uninstall_hooks,
            chat_open,
            chat_poll,
            chat_say,
            chat_answer,
            chat_stop,
            chat_set_mode,
            chat_set_option,
            page_icon,
            page_icons,
            open_link,
            chat_replay,
            chat_leave,
            history,
            history_follow,
            ls,
            pull,
            term_open,
            term_attach,
            term_stream,
            term_key,
            term_drop,
            term_resize,
            term_leave,
            recents,
            recent_visit,
            forget_recent,
            close_web,
            open_page,
            move_page,
            hide_page,
            close_page,
            show_page,
            page_step,
            show_menu,
            hide_menu,
            size_menu,
            menu_rows,
            menu_pick,
            open_session,
            open_on,
            machine_log,
            accept_transfer,
            send_path,
            pick_and_send,
            agents,
            agents_known,
            agents_known_taken,
            register_known_agent,
            agent_seen,
            agent_ask,
            resume_session,
            find,
            traffic_room,
            window_minimize,
            window_maximize_toggle,
            window_close,
            import_candidates,
            import_sessions,
            takeover,
            takeover_term,
            system_unlock,
            lock_state,
            set_lock,
            set_lock_pin,
            set_lock_timeout,
            unlock,
            web_key,
            rotate_web_key,
            reset_identity,
            doctor,
            doctor_fix,
            about,
            set_window_theme,
            config,
            set_config,
            menu_state,
            invite,
            scanner_here,
            pending_list,
            approve,
            root_words,
            pair,
            leave,
            set_paused,
            set_machine_paused
        ])
        .run(tauri::generate_context!())
        .expect("tauri run");
}

#[cfg(desktop)]
mod menu;
#[cfg(desktop)]
mod traffic;

#[cfg(test)]
mod tests {
    use super::page_label;

    /// What tauri takes in a webview label is alphanumeric, `-`, `/`,
    /// `:` and `_`. What this side mints keeps to a narrower set still,
    /// because `/` and `:` read as separators in a key.
    fn tame(label: &str) -> bool {
        label
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    }

    #[test]
    fn a_label_stays_inside_what_tauri_takes() {
        for tab in [
            "1757000000-3", // what a browsing tab's id has always looked like
            "code:mac-mini",
            "code Mac-mini", // the shape that shipped and drew a runtime error
            "café mac",      // a machine name is the user's to type
            "a/b:c",
            "",
        ] {
            let label = page_label(tab);
            assert!(
                tame(&label),
                "label {label:?} from tab {tab:?} is not one tauri takes"
            );
        }
    }

    #[test]
    fn two_tabs_never_share_a_label() {
        let tabs = ["my mac", "my-mac", "my_mac", "my/mac", "my:mac", "mymac"];
        for (i, a) in tabs.iter().enumerate() {
            for b in &tabs[i + 1..] {
                assert_ne!(
                    page_label(a),
                    page_label(b),
                    "{a:?} and {b:?} share one label"
                );
            }
        }
    }
}
