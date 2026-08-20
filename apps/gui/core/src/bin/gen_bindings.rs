//! Prints the frontend's TS types from the Rust structs. Run by the
//! npm `gen` script; the output directory is git-ignored — generated
//! files are never a second source of truth.

use ts_rs::TS;

fn main() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/gen/bindings");
    khor_gui_core::SessionRow::export_all_to(&dir).expect("export SessionRow");
    khor_gui_core::DeviceRow::export_all_to(&dir).expect("export DeviceRow");
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
    khor_gui_core::chat::ChatBatch::export_all_to(&dir).expect("export ChatBatch");
    khor_gui_core::files::DirListing::export_all_to(&dir).expect("export DirListing");
    khor_gui_core::files::DirPinRow::export_all_to(&dir).expect("export DirPinRow");
    khor_gui_core::web::WebPinRow::export_all_to(&dir).expect("export WebPinRow");
    khor_gui_core::web::WebBorrow::export_all_to(&dir).expect("export WebBorrow");
    khor_gui_core::term::TermBatch::export_all_to(&dir).expect("export TermBatch");
    println!("bindings -> {}", dir.display());
}
