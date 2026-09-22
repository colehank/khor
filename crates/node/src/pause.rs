//! `khor pause` / `khor resume`: this machine stops talking to the
//! network without leaving it (docs/MORE.md 暂停).
//!
//! # Not a preference, so not on the config table
//!
//! `khor config` is 「the preferences this machine keeps」 and every row
//! on it is a choice about how khor should behave from now on. This is
//! not one: it is an act with a duration, the shape 「锁上 / 解锁」 has,
//! and `lock.rs` is the module it resembles rather than `config.rs`.
//! The tell is what each answers when asked twice — a preference set to
//! the value it already holds is a no-op worth no sentence, while
//! pausing an already paused machine is a person being told something
//! they did not know.
//!
//! # It survives this process, because the thing it is about does
//!
//! `.khor/pause.json`, beside `lock.json`: a serve that came back from
//! a restart already talking would undo a pause nobody lifted, and the
//! keeper restarts a serve for reasons of its own (`link::serve`'s
//! orphan and deaf arms both exit on purpose).
//!
//! # Read where it matters, never cached
//!
//! Every reader hits the file: the accept arm of [`crate::link`]'s serve
//! loop, its sync tick, `invite` and `pair`. That is a small read on a
//! path that is already doing IO, and it buys the rule the ruling asks
//! for — **a running serve obeys immediately** — with no IPC op, no
//! shared flag between two processes, and no way for the CLI's answer
//! and the resident's behaviour to drift apart.

use std::path::{Path, PathBuf};

use khor_catalog::msg;

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct Stored {
    #[serde(default)]
    on: bool,
}

fn path(root: &Path) -> PathBuf {
    root.join(".khor").join("pause.json")
}

/// **Unreadable and unparseable both read as「not paused」**, the
/// direction `lock::stored` chooses and for the mirrored reason: a
/// machine that fell silent because a file went bad would look exactly
/// like a broken network, and the cure would be deleting a file nobody
/// knows to look for.
pub fn is_on(root: &Path) -> bool {
    std::fs::read_to_string(path(root))
        .ok()
        .and_then(|t| serde_json::from_str::<Stored>(&t).ok())
        .map(|s| s.on)
        .unwrap_or(false)
}

/// Pauses or resumes, and **refuses the state it is already in by
/// name**.
///
/// Answering「好了」to `khor pause` on a paused machine would be a
/// sentence about an act that did not happen — `lock::set_on`'s rule,
/// and the one `Node::leave` follows when there is no network to leave.
pub fn set_on(root: &Path, on: bool) -> Result<(), String> {
    if is_on(root) == on {
        return Err(if on {
            msg::already_paused().to_owned()
        } else {
            msg::not_paused().to_owned()
        });
    }
    let dot = root.join(".khor");
    std::fs::create_dir_all(&dot).map_err(|_| msg::cant_make_dir(dot.display()))?;
    let text = serde_json::to_vec(&Stored { on }).map_err(|e| e.to_string())?;
    crate::link::write_private(&path(root), &text)
}

// ── 按机器暂停 (SHELL.md 第五轮 第 3 条) ─────────────────────────────
//
// **This machine's side of one other machine**: stop dialling it, stop
// answering it, stop syncing with it. The other machine keeps running
// and is told nothing — a remote "pause yourself" would leave it unable
// to hear the "resume" (SHELL.md says why that shape was not taken).
//
// **Kept here, beside `pause.json`, and never in a synced document**:
// it is a decision this machine made about its own talking, and the
// paused machine must not be the one to read it back. A file of ids
// rather than names: a rename must not un-pause anybody.

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct Peers {
    #[serde(default)]
    ids: std::collections::BTreeSet<String>,
}

fn peers_path(root: &Path) -> PathBuf {
    root.join(".khor").join("paused-peers.json")
}

/// The machines this one has paused, by device id. Unreadable reads as
/// none, for [`is_on`]'s reason.
pub fn peers(root: &Path) -> std::collections::BTreeSet<String> {
    std::fs::read_to_string(peers_path(root))
        .ok()
        .and_then(|t| serde_json::from_str::<Peers>(&t).ok())
        .map(|p| p.ids)
        .unwrap_or_default()
}

/// Whether this machine has paused the one with this device id.
pub fn peer_on(root: &Path, id: &str) -> bool {
    peers(root).contains(id)
}

/// Pauses or resumes one other machine, refusing the state it is
/// already in by name (`set_on`'s rule). `name` is only for the words.
pub fn set_peer(root: &Path, id: &str, name: &str, on: bool) -> Result<(), String> {
    let mut all = peers(root);
    if all.contains(id) == on {
        return Err(if on {
            msg::peer_already_paused(name)
        } else {
            msg::peer_not_paused(name)
        });
    }
    if on {
        all.insert(id.to_owned());
    } else {
        all.remove(id);
    }
    let dot = root.join(".khor");
    std::fs::create_dir_all(&dot).map_err(|_| msg::cant_make_dir(dot.display()))?;
    let text = serde_json::to_vec(&Peers { ids: all }).map_err(|e| e.to_string())?;
    crate::link::write_private(&peers_path(root), &text)
}
