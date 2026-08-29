//! Prints the frontend's TS types from the Rust structs. Run by the
//! npm `gen` script; the output directory is git-ignored — generated
//! files are never a second source of truth.

use ts_rs::TS;

fn main() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/gen/bindings");
    khor_gui_core::SessionRow::export_all_to(&dir).expect("export SessionRow");
    khor_gui_core::DeviceRow::export_all_to(&dir).expect("export DeviceRow");
    // Asked for on its own beat and keyed by session id, so no row
    // embeds it and `export_all_to` would never walk into it. Walks into
    // `DayMark` from here.
    khor_gui_core::DayRun::export_all_to(&dir).expect("export DayRun");
    khor_gui_core::FaceChoices::export_all_to(&dir).expect("export FaceChoices");
    khor_gui_core::HooksState::export_all_to(&dir).expect("export HooksState");
    khor_gui_core::Ticket::export_all_to(&dir).expect("export Ticket");
    // Not reachable through any row above: the wizard asks for the
    // agent list on its own, so nothing embeds it.
    khor_gui_core::AgentRow::export_all_to(&dir).expect("export AgentRow");
    // Not reachable through any row above: the spending answer is asked
    // for on its own, so nothing embeds it and `export_all_to` would
    // never walk into it.
    khor_node::Usage::export_all_to(&dir).expect("export Usage");
    // Same reason as the spending answer above: the subscription is
    // asked for on its own and no row embeds it, so nothing would walk
    // into it.
    khor_gui_core::QuotaAnswer::export_all_to(&dir).expect("export QuotaAnswer");
    khor_gui_core::chat::ChatBatch::export_all_to(&dir).expect("export ChatBatch");
    // Asked for on its own by the machine-conversation pane; no row
    // embeds it.
    khor_gui_core::MachineLog::export_all_to(&dir).expect("export MachineLog");
    khor_gui_core::files::DirListing::export_all_to(&dir).expect("export DirListing");
    khor_gui_core::files::DirPinRow::export_all_to(&dir).expect("export DirPinRow");
    // Asked for on its own by each landing's recent list; no row embeds
    // it, so `export_all_to` would never walk into it.
    khor_gui_core::recents::RecentRow::export_all_to(&dir).expect("export RecentRow");
    khor_gui_core::web::WebPinRow::export_all_to(&dir).expect("export WebPinRow");
    khor_gui_core::web::WebBorrow::export_all_to(&dir).expect("export WebBorrow");
    khor_gui_core::term::TermBatch::export_all_to(&dir).expect("export TermBatch");
    println!("bindings -> {}", dir.display());
}
