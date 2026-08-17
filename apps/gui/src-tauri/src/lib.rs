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
fn chat_replay(id: String) -> Result<(), String> {
    khor_gui_core::chat::chat_replay(&id)
}

#[tauri::command]
fn chat_leave(id: String) -> Result<(), String> {
    khor_gui_core::chat::chat_leave(&id)
}

/// Answers with the ticket **and** the window it is good for: the window
/// belongs to the library that enforces it, not to the dialog that
/// prints it (`khor_gui_core::Ticket`).
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
            chat_replay,
            chat_leave,
            invite,
            pair
        ])
        .run(tauri::generate_context!())
        .expect("tauri run");
}
