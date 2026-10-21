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
//! # Read where it matters; in a resident, read by its beat
//!
//! The files are the one truth, and a process with no resident reads
//! them where it asks (`invite`, `pair`, a one-shot verb's dial). **A
//! resident reads them on a beat of its own** ([`poll`], every half
//! second, off the runtime) and every question asked in that process
//! is answered from that reading: they used to be read by every dial —
//! three times, plus once per machine a hop considered — and by every
//! stream answered, and on a home on NFS (turing, net-dial B1) each of
//! those is a call the server may keep, on a thread that carries the
//! link. Half a second is the promise the ruling asks for — **a running
//! serve obeys immediately**, as far as a person can tell — and this
//! process's own [`set_on`] / [`set_peer`] update the reading at once.

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
    if let Some(p) = polled(root) {
        return p.on.load(std::sync::atomic::Ordering::Relaxed);
    }
    read_on(root)
}

fn read_on(root: &Path) -> bool {
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
    crate::link::write_private(&path(root), &text)?;
    if let Some(p) = polled(root) {
        p.on.store(on, std::sync::atomic::Ordering::Relaxed);
    }
    Ok(())
}

/// A resident's reading of both files ([`poll`]).
#[derive(Default)]
struct Polled {
    on: std::sync::atomic::AtomicBool,
    ids: std::sync::RwLock<std::collections::BTreeSet<String>>,
}

/// The homes a resident in this process polls, and their readings.
static POLLED: std::sync::Mutex<Vec<(PathBuf, std::sync::Arc<Polled>)>> = std::sync::Mutex::new(Vec::new());

fn polled(root: &Path) -> Option<std::sync::Arc<Polled>> {
    POLLED
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .iter()
        .find(|(r, _)| r == root)
        .map(|(_, p)| p.clone())
}

/// **The resident's read of both files**, on its beat and never on the
/// runtime that carries the link (`crate::link`'s serve): from the first
/// call on, [`is_on`] and [`peer_on`] in this process answer from it.
/// Answers whether this machine is paused.
pub(crate) fn poll(root: &Path) -> bool {
    let on = read_on(root);
    let ids = read_peers(root);
    let p = {
        let mut all = POLLED.lock().unwrap_or_else(|p| p.into_inner());
        match all.iter().find(|(r, _)| r == root) {
            Some((_, p)) => p.clone(),
            None => {
                let p = std::sync::Arc::new(Polled::default());
                all.push((root.to_path_buf(), p.clone()));
                p
            }
        }
    };
    p.on.store(on, std::sync::atomic::Ordering::Relaxed);
    *p.ids.write().unwrap_or_else(|p| p.into_inner()) = ids;
    on
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
    if let Some(p) = polled(root) {
        return p.ids.read().unwrap_or_else(|p| p.into_inner()).clone();
    }
    read_peers(root)
}

fn read_peers(root: &Path) -> std::collections::BTreeSet<String> {
    std::fs::read_to_string(peers_path(root))
        .ok()
        .and_then(|t| serde_json::from_str::<Peers>(&t).ok())
        .map(|p| p.ids)
        .unwrap_or_default()
}

/// Whether this machine has paused the one with this device id.
pub fn peer_on(root: &Path, id: &str) -> bool {
    match polled(root) {
        Some(p) => p.ids.read().unwrap_or_else(|p| p.into_inner()).contains(id),
        None => read_peers(root).contains(id),
    }
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
    let text = serde_json::to_vec(&Peers { ids: all.clone() }).map_err(|e| e.to_string())?;
    crate::link::write_private(&peers_path(root), &text)?;
    if let Some(p) = polled(root) {
        *p.ids.write().unwrap_or_else(|p| p.into_inner()) = all;
    }
    Ok(())
}
