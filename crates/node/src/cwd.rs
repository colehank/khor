//! **Where the process stands** (docs/SHELL.md 第十四轮 第 4 条).
//!
//! A node's reads and writes name absolute paths, so the process's own
//! working directory ought to be nobody's business. It is not: `gix`
//! goes through the current directory on its way to a repository, and a
//! process whose directory has been *deleted* is refused with
//! 「"…" does not appear to be a git repository」 over a checkout that is
//! plainly there. That is what the user read in the workspace on
//! 2026-09-24 — the install script starts the app with `nohup` from
//! inside the export directory and then removes it. `is_checkout` is one
//! `stat` and kept answering, so the row wore a branch while the file
//! view printed git's refusal.
//!
//! So khor stands somewhere that cannot be taken away. **`/` and not the
//! store root**: the store root is a real directory in an install and a
//! temp directory under `cargo test`, and a pin onto something a later
//! line deletes is the same bug with a longer fuse. `/` is the one
//! directory that outlives every process on the machine.
//!
//! **Two strengths, because the current directory is process state and
//! a library does not own it.**
//!
//! - [`pin`] moves, full stop. It is called by the one process that
//!   owns its own directory and was started by a script that deletes it:
//!   the desktop app (`khor_gui::run`).
//! - [`pin_if_adrift`] moves only when the directory khor is standing in
//!   can no longer be read — which is exactly the failure above, and
//!   nothing else. Every `Node::open` calls it, so any face of khor that
//!   finds itself standing nowhere steps onto solid ground before it
//!   reads a repository.
//!
//! The weaker one is what `Node::open` gets because **opening a node is
//! not a reason to move a process**: `khor-gui-core`'s bindings tests
//! write their TypeScript to a path relative to the current directory,
//! and a node opened on another thread of the same test binary under an
//! unconditional pin sent them to `/` — 「Read-only file system」, eleven
//! tests, measured 2026-09-24.
//!
//! What a move costs is「where the person was standing」, which three
//! callers genuinely want — `khor run` puts the session in it, and the
//! GUI ghost learns its place from it. That is not the current
//! directory, it is **the directory this process was in before khor
//! moved it**, and [`standing`] is that.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use khor_catalog::msg;

/// The directory khor was standing in when it pinned itself, or `None`
/// when that could not be read (it was already gone). Only ever written
/// by a pin, and only read after one.
static STOOD_IN: OnceLock<Option<PathBuf>> = OnceLock::new();

/// Set once a pin has been placed, so a pin moves once and says so
/// once. The GUI's data layer opens a node per call, and a line per call
/// would be the log.
static PINNED: OnceLock<()> = OnceLock::new();

/// Where khor stands once it has pinned itself: the filesystem root,
/// which no run of anything deletes.
const STABLE: &str = "/";

/// **Where the person was standing when they started this process** —
/// `khor run`'s session directory, the GUI ghost's place.
///
/// Before any pin this is simply the current directory, live: nothing
/// has moved, so there is nothing to remember. After one it is the
/// directory khor left, read the moment before it left.
pub fn standing() -> Option<PathBuf> {
    if PINNED.get().is_none() {
        return std::env::current_dir().ok();
    }
    STOOD_IN.get().cloned().flatten()
}

/// Stand somewhere that cannot be deleted, once per process.
///
/// For the process that owns its own directory — the desktop app, which
/// an install script starts from a directory it then removes.
pub fn pin() {
    place(false);
}

/// The same, but **only if khor is standing nowhere**: the directory it
/// was started in has been deleted or is otherwise unreadable, which is
/// the one state in which an absolute path stops being enough.
///
/// This is what `Node::open` calls. A library that moved the process
/// every time somebody opened a node would be deciding something that
/// is not a node's to decide.
pub fn pin_if_adrift() {
    place(true);
}

fn place(only_when_adrift: bool) {
    if PINNED.get().is_some() {
        return;
    }
    // Read before the move, never after: this is the last moment the
    // answer is still the person's directory.
    let here = std::env::current_dir().ok();
    if only_when_adrift && here.is_some() {
        return;
    }
    let stable = Path::new(STABLE);
    let _ = STOOD_IN.set(here);
    match std::env::set_current_dir(stable) {
        Ok(()) => {
            let _ = PINNED.set(());
            // **Said only where there was something to say.** Every
            // `khor` command opens a node, and a line on every run would
            // be a greeting nobody reads printed over the answer they
            // asked for. A pin that had to happen — khor was standing
            // nowhere — is the run whose log has to show where it went.
            if only_when_adrift {
                eprintln!("{}", msg::cwd_pinned(stable.display()));
            }
        }
        // Said rather than swallowed: the whole point of the pin is that
        // a later refusal is not a mystery, and a pin that did not
        // happen is exactly the state that produces one.
        Err(e) => eprintln!("{}", msg::cwd_pin_failed(stable.display(), e)),
    }
}
