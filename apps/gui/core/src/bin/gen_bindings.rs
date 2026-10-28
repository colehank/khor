//! Generate the complete frontend bindings from one Rust root inventory.

#[path = "gen_bindings/managed.rs"]
mod managed;

use std::{
    any::TypeId,
    collections::{BTreeSet, HashSet},
    path::Path,
};
use ts_rs::{TS, TypeVisitor};

// Both preflight and export visit this one list, including standalone answers
// that no other root embeds. Dependencies are walked by ts-rs in both passes.
macro_rules! roots {
    ($visit:ident) => {
        $visit!(khor_gui_core::console::ConsoleCommand);
        $visit!(khor_gui_core::console_complete::Completion);
        $visit!(khor_gui_core::SeenMessage);
        $visit!(khor_gui_core::launch::LaunchPreset);
        $visit!(khor_gui_core::SessionRow);
        $visit!(khor_gui_core::term::EndedPage);
        $visit!(khor_gui_core::DeviceRow);
        $visit!(khor_gui_core::import::CandidateRow);
        $visit!(khor_gui_core::FindHit);
        $visit!(khor_gui_core::DayRun);
        $visit!(khor_gui_core::FaceChoices);
        $visit!(khor_gui_core::KhorAgentView);
        $visit!(khor_gui_core::RelayChoices);
        $visit!(khor_gui_core::RelayWant);
        $visit!(khor_gui_core::RoadShare);
        $visit!(khor_gui_core::Connections);
        $visit!(khor_gui_core::ReachReport);
        $visit!(khor_gui_core::ProxyLine);
        $visit!(khor_gui_core::JournalLine);
        $visit!(khor_gui_core::add::AddAsk);
        $visit!(khor_gui_core::add::AddRun);
        $visit!(khor_gui_core::add::LoginAsk);
        $visit!(khor_gui_core::RecognitionEntry);
        $visit!(khor_gui_core::HooksState);
        $visit!(khor_gui_core::Ticket);
        $visit!(khor_gui_core::About);
        $visit!(khor_gui_core::LeaveOutcome);
        $visit!(khor_gui_core::Waiting);
        $visit!(khor_gui_core::SyncOutcome);
        $visit!(khor_gui_core::DoctorRow);
        $visit!(khor_gui_core::LockFacts);
        $visit!(khor_gui_core::ConfigRow);
        $visit!(khor_gui_core::AgentRow);
        $visit!(khor_gui_core::workspaces::WorkspaceRow);
        $visit!(khor_gui_core::files::FileText);
        $visit!(khor_gui_core::file_handoff::FileHandoff);
        $visit!(khor_gui_core::files::FileStat);
        $visit!(khor_gui_core::files::DirWatch);
        $visit!(khor_gui_core::git::GitStatus);
        $visit!(khor_gui_core::git::GitDiff);
        $visit!(khor_gui_core::git::GitLog);
        $visit!(khor_gui_core::git::GitBlame);
        $visit!(khor_gui_core::git::GitShow);
        $visit!(khor_gui_core::git::GitCommitShow);
        $visit!(khor_gui_core::files::Wrote);
        $visit!(khor_gui_core::files::Merged);
        $visit!(khor_gui_core::code::CodeAt);
        $visit!(khor_gui_core::code::Editor);
        $visit!(khor_gui_core::files::Preview);
        $visit!(khor_gui_core::files::Found);
        $visit!(khor_gui_core::AccountRing);
        $visit!(khor_gui_core::subscriptions::SubscriptionCandidate);
        $visit!(khor_gui_core::agents::KnownAgent);
        $visit!(khor_gui_core::agents::AgentSaid);
        $visit!(khor_node::Usage);
        $visit!(khor_core::Load);
        $visit!(khor_gui_core::QuotaAnswer);
        $visit!(khor_gui_core::chat::ChatBatch);
        $visit!(khor_gui_core::chat::HistoryPage);
        $visit!(khor_gui_core::MachineLog);
        $visit!(khor_gui_core::files::DirListing);
        $visit!(khor_gui_core::recents::RecentRow);
        $visit!(khor_gui_core::web::WebBorrow);
        $visit!(khor_gui_core::tabs::TabsOpen);
        $visit!(khor_gui_core::tabs::TabRow);
        $visit!(khor_gui_core::term::TermStart);
        $visit!(khor_gui_core::term::TermChunk);
        $visit!(khor_gui_core::term::DropEnd);
        $visit!(khor_gui_core::term::DropItem);
        $visit!(khor_gui_core::miss::Miss);
        $visit!(khor_gui_core::term::PaneHistory);
        $visit!(khor_gui_core::term::HistoryRow);
    };
}

#[derive(Default)]
struct Paths {
    seen: HashSet<TypeId>,
    files: BTreeSet<String>,
    error: Option<String>,
}
impl TypeVisitor for Paths {
    fn visit<T: TS + 'static + ?Sized>(&mut self) {
        if self.error.is_some() || !self.seen.insert(TypeId::of::<T>()) {
            return;
        }
        let Some(path) = T::output_path() else { return };
        match managed::type_name(path) {
            Ok(name) => {
                self.files.insert(name);
            }
            Err(error) => {
                self.error = Some(error);
                return;
            }
        }
        T::visit_dependencies(self);
    }
}
fn planned_files() -> Result<BTreeSet<String>, String> {
    let mut paths = Paths::default();
    macro_rules! inspect {
        ($ty:ty) => {
            paths.visit::<$ty>();
        };
    }
    roots!(inspect);
    match paths.error {
        Some(error) => Err(error),
        None => Ok(paths.files),
    }
}
fn export(dir: &Path) -> Result<(), String> {
    macro_rules! emit {
        ($ty:ty) => {
            <$ty>::export_all_to(dir)
                .map_err(|error| format!("export {}: {error}", stringify!($ty)))?;
        };
    }
    roots!(emit);
    Ok(())
}
fn run() -> Result<(), String> {
    let mut args = std::env::args_os().skip(1);
    let given = args.next();
    if args.next().is_some() {
        return Err("expected at most one bindings output directory".into());
    }
    let default_output = given.is_none();
    // Cargo sets this at execution time. A shared-target binary must never
    // write into the checkout that happened to compile it (2026-09-16).
    let dir = match given {
        Some(path) => path.into(),
        None => std::path::PathBuf::from(
            std::env::var_os("CARGO_MANIFEST_DIR")
                .ok_or("set CARGO_MANIFEST_DIR or pass an explicit output directory")?,
        )
        .join("../src/gen/bindings"),
    };
    let files = planned_files()?;
    managed::generate(&dir, default_output, &files, export)?;
    println!("bindings -> {}", dir.display());
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("bindings: {error}");
        std::process::exit(1);
    }
}
