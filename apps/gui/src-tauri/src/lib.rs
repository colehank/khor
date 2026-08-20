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

#[tauri::command]
fn devices() -> Result<Vec<DeviceRow>, String> {
    khor_gui_core::list_devices(&Node::root_from_env())
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

#[tauri::command]
fn seen(id: String) -> Result<(), String> {
    khor_gui_core::seen(&Node::root_from_env(), &id)
}

#[tauri::command]
fn close_session(id: String) -> Result<(), String> {
    khor_gui_core::close_session(&Node::root_from_env(), &id)
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

/// The six terminal commands are one attachment registry in gui-core,
/// mirroring chat: the window polls for a screen because that is the
/// shape both skins can serve (`khor_gui_core::term` module head).
#[tauri::command]
fn term_open(id: String, cols: u16, rows: u16) -> Result<(), String> {
    khor_gui_core::term::term_open(&Node::root_from_env(), &id, cols, rows)
}

#[tauri::command]
fn term_poll(id: String, since: u64) -> Result<khor_gui_core::term::TermBatch, String> {
    khor_gui_core::term::term_poll(&id, since)
}

#[tauri::command]
fn term_key(id: String, keys: String) -> Result<(), String> {
    khor_gui_core::term::term_key(&id, keys.into_bytes())
}

/// Pasted text, wrapped by the terminal registry according to what the
/// program running in there asked for (`term::term_paste`).
#[tauri::command]
fn term_paste(id: String, text: String) -> Result<(), String> {
    khor_gui_core::term::term_paste(&id, &text)
}

/// Files dropped on a terminal: their paths, shell-quoted, pasted in
/// (`term::term_drop`). The quoting is the registry's because it is
/// about the shell's syntax, and one wrong escape is a command.
#[tauri::command]
fn term_drop(id: String, paths: Vec<String>) -> Result<(), String> {
    khor_gui_core::term::term_drop(&id, &paths)
}

#[tauri::command]
fn term_resize(id: String, cols: u16, rows: u16) -> Result<(), String> {
    khor_gui_core::term::term_resize(&id, cols, rows)
}

#[tauri::command]
fn term_leave(id: String) -> Result<(), String> {
    khor_gui_core::term::term_leave(&id)
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
fn open_session(
    dir: String,
    title: Option<String>,
    form: String,
    agent: Option<String>,
) -> Result<String, String> {
    khor_gui_core::open_session(
        &Node::root_from_env(),
        &dir,
        title.as_deref().unwrap_or(""),
        &form,
        agent.as_deref().unwrap_or(""),
    )
}

/// The wizard's 智能体 list (批⑥): every ACP agent this person named.
/// khor's own two are not in it — they are not registrations, and the
/// face puts them beside this list rather than inside it.
#[tauri::command]
fn agents() -> Result<Vec<khor_gui_core::AgentRow>, String> {
    khor_gui_core::agents(&Node::root_from_env())
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

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            sessions,
            devices,
            usage,
            seen,
            close_session,
            tell,
            pin_session,
            pin_device,
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
            open_link,
            chat_replay,
            chat_leave,
            history,
            ls,
            pull,
            dir_pins,
            pin_dir,
            term_open,
            term_poll,
            term_key,
            term_paste,
            term_drop,
            term_resize,
            term_leave,
            web_pins,
            pin_web,
            open_web,
            open_session,
            agents,
            takeover,
            takeover_term,
            invite,
            pair
        ])
        .run(tauri::generate_context!())
        .expect("tauri run");
}
