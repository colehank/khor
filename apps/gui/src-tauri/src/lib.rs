//! The tauri skin: every command is one call into khor-gui-core, which
//! is one call into khor-node — CLI and GUI stay equivalent because they
//! are the same functions (docs/KHOR.md 三条地基).

use khor_gui_core::{DeviceRow, SessionRow};
use khor_node::Node;

#[tauri::command]
fn sessions() -> Result<Vec<SessionRow>, String> {
    khor_gui_core::list_sessions(&Node::root_from_env())
}

#[tauri::command]
fn devices() -> Result<Vec<DeviceRow>, String> {
    khor_gui_core::list_devices(&Node::root_from_env())
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
fn invite() -> Result<String, String> {
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
            seen,
            close_session,
            tell,
            pin_session,
            pin_device,
            invite,
            pair
        ])
        .run(tauri::generate_context!())
        .expect("tauri run");
}
