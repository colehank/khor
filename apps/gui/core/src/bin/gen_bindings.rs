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
    // Not reachable through any row above: the spending answer is asked
    // for on its own, so nothing embeds it and `export_all_to` would
    // never walk into it.
    khor_node::Usage::export_all_to(&dir).expect("export Usage");
    khor_gui_core::chat::ChatBatch::export_all_to(&dir).expect("export ChatBatch");
    println!("bindings -> {}", dir.display());
}
