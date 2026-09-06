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

#[tauri::command]
async fn quota_board() -> Result<Vec<khor_gui_core::AccountRing>, String> {
    khor_gui_core::quota_board(&Node::root_from_env()).await
}

#[tauri::command]
fn register_account(
    name: String,
    provider: String,
    base_url: Option<String>,
    key: String,
    budget: Option<f64>,
) -> Result<String, String> {
    khor_gui_core::register_account(
        &Node::root_from_env(),
        &name,
        &provider,
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
    khor_gui_core::load(&Node::root_from_env(), &machine, top, by, desc.unwrap_or(true)).await
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

/// Opens a browsing window whose traffic leaves through `machine`'s
/// network (docs/NET.md 借网). The borrow gives a local proxy address;
/// the window is built pointing its `proxy_url` at it, so every request
/// the page makes goes out the far machine. Async because the borrow
/// dials. The window is labelled by the borrow session so it is unique
/// and closing it is traceable.
///
/// **macOS only**, because per-window `proxy_url` is (the Cargo feature
/// is gated to macOS). Opening one elsewhere without the proxy would leak
/// the request out this machine's own network — the opposite of a borrow
/// — so the other platforms refuse in words until the platform batch
/// wires their proxy paths.
#[cfg(target_os = "macos")]
#[tauri::command]
async fn open_web(
    app: tauri::AppHandle,
    machine: String,
    url: String,
) -> Result<(), String> {
    use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
    let borrow = khor_gui_core::web::borrow_web(&Node::root_from_env(), &machine).await?;
    let target: tauri::Url = url.parse().map_err(|_| format!("not a url: {url}"))?;
    let proxy: tauri::Url =
        format!("http://{}", borrow.addr).parse().map_err(|e| format!("{e}"))?;
    let label = format!("web-{}", borrow.session.replace(['/', ':'], "-"));
    // Reuse a window already open under this label rather than stacking a
    // second — one borrow, one window.
    if let Some(existing) = app.get_webview_window(&label) {
        let _ = existing.set_focus();
        return Ok(());
    }
    WebviewWindowBuilder::new(&app, &label, WebviewUrl::External(target))
        .title(machine)
        .proxy_url(proxy)
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
async fn open_web(machine: String, url: String) -> Result<(), String> {
    let _ = (machine, url);
    Err(khor_catalog::msg::BORROW_WEBVIEW_MACOS_ONLY.into())
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
async fn open_on(machine: String) -> Result<String, String> {
    khor_gui_core::open_term_on(&Node::root_from_env(), &machine).await
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
        khor_gui_core::send_path(&Node::root_from_env(), &machine, std::path::Path::new(&path))
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
        .invoke_handler(tauri::generate_handler![
            sessions,
            day_strips,
            places,
            devices,
            load,
            quota_board,
            register_account,
            remove_account,
            usage,
            quota,
            seen,
            close_session,
            rename_session,
            tell,
            pin_session,
            pin_device,
            forget_device,
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
            open_web,
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
            invite,
            pair
        ])
        .run(tauri::generate_context!())
        .expect("tauri run");
}
