//! Prints the frontend's TS types from the Rust structs. Run by the
//! npm `gen` script; the output directory is git-ignored — generated
//! files are never a second source of truth.

use ts_rs::TS;

fn main() {
    // **Read at run time, never baked in at compile time.** This used
    // to be `env!("CARGO_MANIFEST_DIR")`, which is the path of whichever
    // checkout *compiled* the binary — and `scripts/build-on.sh` runs
    // `cargo run` from a throwaway export of HEAD against the shared
    // target dir, where cargo happily reuses a binary compiled from an
    // earlier export. So on 2026-09-16 `npm run gen` in a fresh export
    // wrote its 75 files into `$TMPDIR/khor-build-on.52819/…`, a tree
    // left by an interrupted run days before, and the export it was
    // run in got nothing; tsc then failed on the missing modules and
    // two of the three machines refused the install. `cargo run` sets
    // `CARGO_MANIFEST_DIR` for the process it starts, so the run-time
    // value names the crate the person is actually standing in. An
    // explicit first argument wins over both, for anybody driving this
    // without cargo.
    let dir = match std::env::args().nth(1) {
        Some(given) => std::path::PathBuf::from(given),
        None => std::path::PathBuf::from(
            std::env::var("CARGO_MANIFEST_DIR")
                .unwrap_or_else(|_| env!("CARGO_MANIFEST_DIR").to_string()),
        )
        .join("../src/gen/bindings"),
    };
    khor_gui_core::SessionRow::export_all_to(&dir).expect("export SessionRow");
    khor_gui_core::DeviceRow::export_all_to(&dir).expect("export DeviceRow");
    // Asked for on its own beat and keyed by session id, so no row
    // embeds it and `export_all_to` would never walk into it. Walks into
    // `DayMark` from here.
    khor_gui_core::DayRun::export_all_to(&dir).expect("export DayRun");
    khor_gui_core::FaceChoices::export_all_to(&dir).expect("export FaceChoices");
    khor_gui_core::RelayChoices::export_all_to(&dir).expect("export RelayChoices");
    khor_gui_core::HooksState::export_all_to(&dir).expect("export HooksState");
    khor_gui_core::Ticket::export_all_to(&dir).expect("export Ticket");
    // The identity card's foot: asked for on its own, embedded in no row.
    khor_gui_core::About::export_all_to(&dir).expect("export About");
    // 自检's rows: asked on their own beat, embedded in nothing.
    khor_gui_core::DoctorRow::export_all_to(&dir).expect("export DoctorRow");
    // Not reachable through any row above: the wizard asks for the
    // agent list on its own, so nothing embeds it.
    khor_gui_core::AgentRow::export_all_to(&dir).expect("export AgentRow");
    // The wizard's first question, asked on its own beat; no row embeds
    // it.
    khor_gui_core::PlaceRow::export_all_to(&dir).expect("export PlaceRow");
    // The workspace landing asks per machine on its own beat; no row
    // embeds it either.
    khor_gui_core::workspaces::WorkspaceRow::export_all_to(&dir).expect("export WorkspaceRow");
    // One file's text, asked for when a file is opened; no row embeds it.
    khor_gui_core::files::FileText::export_all_to(&dir).expect("export FileText");
    khor_gui_core::git::GitStatus::export_all_to(&dir).expect("export GitStatus");
    khor_gui_core::git::GitDiff::export_all_to(&dir).expect("export GitDiff");
    khor_gui_core::files::Wrote::export_all_to(&dir).expect("export Wrote");
    // The answer to a refused save's third exit; nothing embeds it.
    khor_gui_core::files::Merged::export_all_to(&dir).expect("export Merged");
    khor_gui_core::code::CodeAt::export_all_to(&dir).expect("export CodeAt");
    khor_gui_core::code::Editor::export_all_to(&dir).expect("export Editor");
    khor_gui_core::files::Preview::export_all_to(&dir).expect("export Preview");
    // What the box at the head of the tree gets back (工作区第六轮 第 47
    // 条); nothing embeds it, and it walks into `SearchHit` from here.
    khor_gui_core::files::Found::export_all_to(&dir).expect("export Found");
    // The 额度 page asks for its rings on its own beat; no row embeds
    // them. Walks into `AccountReading` and `Strain` from here.
    khor_gui_core::AccountRing::export_all_to(&dir).expect("export AccountRing");
    // Same: the registry snapshot and one agent's recorded
    // conversations are each asked for on their own beat, so no row
    // embeds either. `AgentSaid` walks into `SeenSession` from here.
    khor_gui_core::agents::KnownAgent::export_all_to(&dir).expect("export KnownAgent");
    khor_gui_core::agents::AgentSaid::export_all_to(&dir).expect("export AgentSaid");
    // Not reachable through any row above: the spending answer is asked
    // for on its own, so nothing embeds it and `export_all_to` would
    // never walk into it.
    khor_node::Usage::export_all_to(&dir).expect("export Usage");
    // The machine card's process top and CPU day: the `load` op's
    // answer, a `khor_core` type no row embeds.
    khor_core::Load::export_all_to(&dir).expect("export Load");
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
    khor_gui_core::term::TermStart::export_all_to(&dir).expect("export TermStart");
    khor_gui_core::term::TermChunk::export_all_to(&dir).expect("export TermChunk");
    println!("bindings -> {}", dir.display());
}
