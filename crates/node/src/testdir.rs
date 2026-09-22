//! Temp trees for this crate's unit tests that **take themselves away
//! however the test leaves** (SHELL.md 第五轮 第 4 条).
//!
//! `tests.rs::Scratch` already did this for the tests in that file; the
//! helpers elsewhere in the crate each built their path by hand and
//! cleaned up at the end, if at all — so every `cargo test --workspace`
//! left 39 of them in `$TMPDIR` (measured 2026-09-22, all from this
//! crate's lib binary: `khor-write-*`, `khor-usage-*`, `khor-faketmux-*`,
//! `khor-quota-*.json` …). One type here, so the next helper has
//! something to reach for.

use std::path::{Path, PathBuf};

/// A path under the temp dir that is removed — directory or file — when
/// this is dropped, including by a panicking assertion.
pub(crate) struct TempTree(PathBuf);

impl TempTree {
    /// `$TMPDIR/khor-<name>-<pid>`, emptied and made fresh.
    pub(crate) fn fresh(name: &str) -> TempTree {
        let t = TempTree::at(name);
        std::fs::create_dir_all(&t.0).unwrap();
        t
    }

    /// The same path, removed if it is there and **not** created — for
    /// a test that builds the tree itself or wants a file there.
    pub(crate) fn at(name: &str) -> TempTree {
        let p = std::env::temp_dir().join(format!("khor-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        let _ = std::fs::remove_file(&p);
        TempTree(p)
    }

    pub(crate) fn path(&self) -> PathBuf {
        self.0.clone()
    }
}

impl Drop for TempTree {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
        let _ = std::fs::remove_file(&self.0);
    }
}

impl std::ops::Deref for TempTree {
    type Target = Path;

    fn deref(&self) -> &Path {
        &self.0
    }
}

impl AsRef<Path> for TempTree {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

/// A program path whose directory goes with it (the fake `tmux` the
/// adaptor tests run). Reads as the `&str` the adaptor takes.
pub(crate) struct TempBin {
    _tree: TempTree,
    path: String,
}

impl TempBin {
    pub(crate) fn new(tree: TempTree, path: String) -> TempBin {
        TempBin { _tree: tree, path }
    }
}

impl std::ops::Deref for TempBin {
    type Target = str;

    fn deref(&self) -> &str {
        &self.path
    }
}
