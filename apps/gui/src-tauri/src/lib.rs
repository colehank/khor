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

/// 常去的地方 for the wizard's first question
/// (`khor_gui_core::places`).
#[tauri::command]
async fn places(machine: Option<String>) -> Result<Vec<khor_gui_core::PlaceRow>, String> {
    khor_gui_core::places(&Node::root_from_env(), machine.as_deref().unwrap_or("")).await
}

/// The workspace landing's whole list, off this machine's own table
/// (`khor_gui_core::workspaces::workspaces`) — no machine is asked.
#[tauri::command]
fn workspaces() -> Result<Vec<khor_gui_core::workspaces::WorkspaceRow>, String> {
    khor_gui_core::workspaces::workspaces(&Node::root_from_env())
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
fn set_relays(urls: Vec<String>, only_mine: bool) -> Result<khor_gui_core::RelayChoices, String> {
    khor_gui_core::set_relays(&Node::root_from_env(), &urls, only_mine)
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

#[tauri::command]
fn face_choices() -> Result<FaceChoices, String> {
    khor_gui_core::face_choices(&Node::root_from_env())
}

/// An axis left out stays where it is — the same shape as `khor face`'s
/// flags, because they are the same call underneath.
#[tauri::command]
fn restyle(
    colors: Option<Vec<String>>,
    variant: Option<String>,
    shape: Option<String>,
) -> Result<(), String> {
    khor_gui_core::restyle(&Node::root_from_env(), colors, variant, shape)
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

#[tauri::command]
fn open_link(url: String) -> Result<(), String> {
    khor_gui_core::web::open_link(&url)
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

#[tauri::command]
fn dir_pins() -> Result<Vec<khor_gui_core::files::DirPinRow>, String> {
    khor_gui_core::files::dir_pins(&Node::root_from_env())
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

#[tauri::command]
fn pin_dir(machine: String, path: String, on: bool) -> Result<(), String> {
    khor_gui_core::files::pin_dir(&Node::root_from_env(), &machine, &path, on)
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

#[tauri::command]
fn web_pins() -> Result<Vec<khor_gui_core::web::WebPinRow>, String> {
    khor_gui_core::web::web_pins(&Node::root_from_env())
}

#[tauri::command]
fn pin_web(machine: String, url: String, on: bool) -> Result<(), String> {
    khor_gui_core::web::pin_web(&Node::root_from_env(), &machine, &url, on)
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
    view.hide().map_err(|e| e.to_string())
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
    view.show().map_err(|e| e.to_string())
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
/// 709 ms   #0f0f0e   this ground
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
/// Puts the three window buttons where an older macOS put them: 14 wide
/// at x = 7, 25, 43 — the trio ends at 57, inside the 64-wide rail with
/// room on both sides.
///
/// 工作区第八轮 第 58 条, second look (2026-09-19):「左上角按钮没有变小,
/// 还超过侧边栏了」— the user, pointing at WeChat. **Their size is the
/// SDK's, not khor's, and it cannot be changed**: an app linked against
/// the macOS 26 SDK gets the new 14-wide buttons 23 apart (measured on
/// every title-bar style this SDK offers — plain, transparent, every
/// toolbar style — all 14 and 23); WeChat is linked against 15.2 and
/// gets the old 12-wide ones 18 apart. What can be moved is where each
/// one sits, and 18 apart is what the old spacing was.
///
/// **Not tauri's `trafficLightPosition`.** That keeps the system spacing
/// and re-asserts it on every draw (tao's `draw_rect`), so it would undo
/// this on the next frame. Instead the frames are set here and set
/// again whenever AppKit lays the title bar out afresh — a resize, a
/// focus, a theme change — which is when it would put them back.
#[cfg(target_os = "macos")]
fn squeeze_the_window_buttons(window: &tauri::WebviewWindow) {
    use objc2::msg_send;
    use objc2::runtime::AnyObject;
    use objc2_foundation::{NSPoint, NSRect};
    let Ok(win) = window.ns_window() else { return };
    let w = win as *mut AnyObject;
    // NSWindowButton: closeButton = 0, miniaturizeButton = 1, zoomButton = 2.
    for (i, x) in [7.0f64, 25.0, 43.0].into_iter().enumerate() {
        unsafe {
            let button: *mut AnyObject = msg_send![w, standardWindowButton: i];
            if button.is_null() {
                continue;
            }
            let frame: NSRect = msg_send![button, frame];
            if frame.origin.x == x {
                continue;
            }
            let _: () = msg_send![button, setFrameOrigin: NSPoint::new(x, frame.origin.y)];
        }
    }
}

#[cfg(desktop)]
fn paint_the_window_before_the_page(app: &tauri::AppHandle) {
    use tauri::{Manager, Theme};
    let Some(w) = app.get_webview_window("main") else {
        return;
    };
    let dark = matches!(w.theme(), Ok(Theme::Dark));
    let ground = if dark {
        tauri::webview::Color(0x0f, 0x0f, 0x0e, 0xff)
    } else {
        tauri::webview::Color(0xf4, 0xf4, 0xef, 0xff)
    };
    // Nothing to do if the platform refuses: the failure is the old
    // behaviour, which is a flash rather than a fault.
    let _ = w.set_background_color(Some(ground));
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
    // **Per window, not through tauri's `theme`.** On macOS that call
    // sets NSApp's appearance for every window, and an app with an
    // appearance of its own stops following the desktop — khor's whole
    // face would have frozen in the scheme it had at that moment.
    #[cfg(target_os = "macos")]
    if let Ok(win) = made.ns_window() {
        wear_the_apps_appearance(win);
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
    // draws no switch there anyway (`views/WorkspacePane`).
    Err("no windows here".into())
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
    Err(khor_catalog::msg::PAGE_NEEDS_DESKTOP.into())
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

/// Opens a conversation the agent already has, at **its own** cwd —
/// not a fresh one, and not the wizard's 目录 answer.
#[tauri::command]
fn resume_session(agent: String, session: String) -> Result<String, String> {
    khor_gui_core::agents::resume_session(&Node::root_from_env(), &agent, &session)
}

/// 自检's checklist (`khor_gui_core::doctor`). Async: each check has a
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

pub fn run() {
    // If this process was re-exec'd as a session host, be one — the app
    // spawns hosts through its own binary (`host.rs main_if_host`).
    khor_node::host::main_if_host(Node::root_from_env());

    // The app embeds serve — the desktop is a full node, not a client
    // (one mesh, no client/server split). Running `khor serve` on the
    // same home at the same time is the known conflict; handover is on
    // the ledger.
    std::thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        rt.block_on(async {
            match Node::open(Node::root_from_env()) {
                Ok(n) => {
                    if let Err(e) = n.serve().await {
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
    match khor_web::listen(Node::root_from_env(), khor_web::port()) {
        Ok(face) => eprintln!("khor web face: {}", face.addr),
        Err(e) => eprintln!("{e}"),
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // The window khor's own overlays are drawn in, hung off the
            // face and hidden until something has a list to show
            // (`hang_menu_window`).
            #[cfg(desktop)]
            hang_menu_window(app.handle());
            #[cfg(desktop)]
            paint_the_window_before_the_page(app.handle());
            #[cfg(target_os = "macos")]
            if let Some(main) = tauri::Manager::get_webview_window(app, "main") {
                squeeze_the_window_buttons(&main);
            }
            #[cfg(not(desktop))]
            let _ = app;
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
                    // AppKit lays the title bar out again on a resize
                    // (full screen in and out included) and the buttons
                    // go back to the SDK's spacing; put them back
                    // (`squeeze_the_window_buttons`).
                    tauri::WindowEvent::Resized(_) => {
                        #[cfg(target_os = "macos")]
                        if let Some(main) = window.app_handle().get_webview_window("main") {
                            squeeze_the_window_buttons(&main);
                        }
                    }
                    tauri::WindowEvent::ThemeChanged(theme) => {
                        #[cfg(target_os = "macos")]
                        if let Some(main) = window.app_handle().get_webview_window("main") {
                            squeeze_the_window_buttons(&main);
                        }
                        for (label, w) in window.app_handle().webview_windows() {
                            if label.starts_with("code:") {
                                #[cfg(target_os = "macos")]
                                if let Ok(win) = w.ns_window() {
                                    wear_appearance(win, *theme);
                                }
                                #[cfg(not(target_os = "macos"))]
                                let _ = (w, theme);
                            }
                        }
                    }
                    tauri::WindowEvent::Focused(here) => {
                        if let Some(menu) = menu()
                            && menu_up().load(std::sync::atomic::Ordering::Relaxed)
                        {
                            let _ = if *here { menu.show() } else { menu.hide() };
                        }
                    }
                    _ => {}
                }
            }
            #[cfg(not(desktop))]
            let _ = (window, event);
        })
        .invoke_handler(tauri::generate_handler![
            sessions,
            widen,
            day_strips,
            places,
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
            open_link,
            chat_replay,
            chat_leave,
            history,
            history_follow,
            ls,
            pull,
            dir_pins,
            pin_dir,
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
            web_pins,
            pin_web,
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
            takeover,
            takeover_term,
            doctor,
            doctor_fix,
            about,
            invite,
            pair
        ])
        .run(tauri::generate_context!())
        .expect("tauri run");
}

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
