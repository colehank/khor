//! What this screen has open in its browsing tabs (docs/SHELL.md 第 2 条).
//!
//! Shape: `.khor/tabs.json`, `{ "<kind>": { "tabs": [...], "active": "<id>" } }`.
//!
//! # It is local, and [`crate::…`] is not where that was decided
//!
//! `khor_sync::recents`'s own head settled it before this file existed:
//! 「A tab strip is what one screen currently has open, and it belongs
//! to that screen — the desk having four tabs open says nothing about
//! what the phone should be showing. Memory travels, the screen's state
//! stays with the screen.」 So this is a file beside `config.json`,
//! `lock.json` and `avatar.json` — the three things this machine
//! decides and never sends — and not a document in the replicated
//! store.
//!
//! # Then why is it in the node at all, rather than in the browser's
//! own storage
//!
//! It *was* `localStorage`, and that was enough for everything except
//! the one thing the ruling also asks for: 「CLI 等价：标签页列表是本机
//! 状态，CLI 至少要能列、能开、能关」. A list only the webview can read
//! is a list `khor tabs` cannot, and **CLI 与 GUI 等价** is one of this
//! repo's three standing rules (docs/handoff/001.md GUI 每批要守的三条)
//! — the tauri command is a thin shell over the same function the
//! terminal calls, never a second implementation.
//!
//! Moving it here buys the restart promise outright as well: a file on
//! this disk outlives the webview's storage being cleared, a web face
//! served to a phone, and the app being reinstalled over its store.
//!
//! # Keyed by kind, for the reason the browser's own store was
//!
//! `page` today, and a second browsing kind (the editor tab, on the
//! account) would be `code`. Two kinds under one list would parse
//! cleanly out of each other — a URL and a workspace path are both
//! strings — which is exactly the danger: the browser landing would
//! come up holding paths and point its address bar at them, and every
//! refusal after that would be about the wrong subject. One key per
//! kind makes that unreachable rather than unlikely.
//!
//! # Ids are minted here, and that is the whole point of the move
//!
//! The face used to mint them. With two callers — a window and a
//! terminal — two minters is two id shapes, and the first collision is
//! a tab the GUI opened that `khor tabs close` cannot name. So there is
//! one minter, and it is the side both callers already go through.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use khor_catalog::msg;
use serde::{Deserialize, Serialize};

/// One open tab: a machine, a place on it, and what to call it.
///
/// **`title` is remembered rather than derived**, and that is what lets
/// a restart draw the list before anything is loaded (第 2 条:「页面
/// 内容可以在点到那枚时才加载」). A tab that has never been anywhere
/// carries an empty one and the face says 新标签页 in its own words —
/// the node has no business holding a display word.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tab {
    /// This screen's handle for the tab. Minted here; never shown.
    pub id: String,
    /// The exit machine's device id, or empty on a tab pointed nowhere
    /// yet — the fresh-tab state, which is a real state and not a
    /// missing one.
    #[serde(default)]
    pub device: String,
    /// Where it is, in that kind's own spelling. Empty asks for that
    /// kind's own starting point.
    #[serde(default)]
    pub url: String,
    /// What it was last called. Empty until it has been somewhere.
    #[serde(default)]
    pub title: String,
}

/// One kind's tabs, and which of them is in front.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Open {
    #[serde(default)]
    pub tabs: Vec<Tab>,
    /// The tab in front. **Never dangling on the way out**: every road
    /// out of this module runs [`Open::settle`], so a reader is not
    /// handed an id that names no row.
    #[serde(default)]
    pub active: String,
}

impl Open {
    /// Makes `active` name a tab that is actually here.
    ///
    /// **An id naming no row and no id at all are one failure**, and
    /// both land on the first tab rather than on nothing — the same
    /// choice the face's own store made when it was the one reading
    /// this file's job. An empty list keeps an empty `active`; minting
    /// a tab is a decision, and reading is not the place to make one.
    fn settle(&mut self) {
        if self.tabs.iter().any(|t| t.id == self.active) {
            return;
        }
        self.active = self.tabs.first().map(|t| t.id.clone()).unwrap_or_default();
    }
}

fn path(root: &Path) -> PathBuf {
    root.join(".khor").join("tabs.json")
}

/// Where the tabs are written, for the tests that assert a read writes
/// nothing. Not public otherwise: a caller that knew the path would be
/// a second way to write it (`config::path_for_tests`'s rule).
#[cfg(test)]
pub fn path_for_tests(root: &Path) -> PathBuf {
    path(root)
}

/// Every kind's tabs, as the file has them.
///
/// **Unreadable and unparseable both read as「nothing open」**, the
/// direction `config::stored` and `lock::stored` already take: what a
/// person loses to a file nobody can read is their tabs, and what they
/// must never lose to it is the app coming up at all.
fn stored(root: &Path) -> BTreeMap<String, Open> {
    std::fs::read_to_string(path(root))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn save(root: &Path, all: &BTreeMap<String, Open>) -> Result<(), String> {
    let dot = root.join(".khor");
    std::fs::create_dir_all(&dot).map_err(|_| msg::cant_make_dir(dot.display()))?;
    let text = serde_json::to_vec_pretty(all).map_err(|e| e.to_string())?;
    let p = path(root);
    std::fs::write(&p, text).map_err(|e| msg::cant_write(p.display(), e))
}

/// One kind's tabs.
pub fn list(root: &Path, kind: &str) -> Open {
    let mut open = stored(root).remove(kind).unwrap_or_default();
    open.settle();
    open
}

/// Reads, changes, settles, writes — the one road every write takes.
///
/// **Every mutation goes through here**, so `active` cannot be left
/// naming a closed tab by a caller that forgot: the invariant is held
/// by the road rather than by each traveller.
fn change<T>(
    root: &Path,
    kind: &str,
    with: impl FnOnce(&mut Open) -> Result<T, String>,
) -> Result<T, String> {
    let mut all = stored(root);
    let open = all.entry(kind.to_string()).or_default();
    let out = with(open)?;
    open.settle();
    save(root, &all)?;
    Ok(out)
}

/// A fresh handle.
///
/// Time, **this process**, and a counter — no random source. An id is
/// written to a file and read back, so it has to survive a restart, and
/// randomness buys nothing for that.
///
/// ⚠️ **The process id is not decoration, and leaving it out was a bug
/// this file shipped for about ten minutes.** The first version read
/// 「the counter is per process, which is all it has to be — the
/// millisecond is what separates two processes」, and that is exactly
/// backwards: two processes starting in the same millisecond both mint
/// `t<ms>-0`. Which is not a hypothetical here — the window and
/// `khor tabs open` are two processes over one file, and 第 2 条 is the
/// ruling that put them there. Caught by reading a real `khor tabs ls`:
/// every id ended in `-0`.
fn mint() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    format!("t{ms:x}-{:x}-{n}", std::process::id())
}

/// Opens a tab and puts it in front. Answers its id.
///
/// `device` and `url` may both be empty: that is the fresh tab the
/// 「+」 mints, and it is a real state rather than a half-made row.
pub fn open(root: &Path, kind: &str, device: &str, url: &str) -> Result<String, String> {
    change(root, kind, |open| {
        let id = mint();
        open.tabs.push(Tab {
            id: id.clone(),
            device: device.to_string(),
            url: url.to_string(),
            title: String::new(),
        });
        open.active = id.clone();
        Ok(id)
    })
}

/// Points an open tab somewhere, and names it.
///
/// **Refused by name for a tab that is not there**, rather than
/// silently adding one: a caller pointing at a closed tab has a stale
/// idea of what is open, and a row appearing under it would hide that.
pub fn go(
    root: &Path,
    kind: &str,
    id: &str,
    device: &str,
    url: &str,
    title: &str,
) -> Result<(), String> {
    change(root, kind, |open| {
        let tab = open
            .tabs
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| msg::no_such_tab(id))?;
        tab.device = device.to_string();
        tab.url = url.to_string();
        tab.title = title.to_string();
        Ok(())
    })
}

/// Brings a tab to the front.
pub fn focus(root: &Path, kind: &str, id: &str) -> Result<(), String> {
    change(root, kind, |open| {
        if !open.tabs.iter().any(|t| t.id == id) {
            return Err(msg::no_such_tab(id));
        }
        open.active = id.to_string();
        Ok(())
    })
}

/// Closes a tab, and says which one is in front now.
///
/// **The neighbour on the right takes the place, falling back to the
/// left at the end of the list** — the browser convention, and the one
/// that keeps the eye where the hand already is. Closing the last tab
/// leaves the list empty: this is a store, and 「what a landing with no
/// tabs draws」 is the face's decision, not a row invented down here.
pub fn close(root: &Path, kind: &str, id: &str) -> Result<(), String> {
    change(root, kind, |open| {
        let at = open
            .tabs
            .iter()
            .position(|t| t.id == id)
            .ok_or_else(|| msg::no_such_tab(id))?;
        let was_front = open.active == id;
        open.tabs.remove(at);
        if was_front {
            open.active = open
                .tabs
                .get(at.min(open.tabs.len().saturating_sub(1)))
                .map(|t| t.id.clone())
                .unwrap_or_default();
        }
        Ok(())
    })
}

/// The whole list at once, for the face that holds one.
///
/// **It exists because the window is the one caller that can reorder**,
/// and a drag that had to be spelled as a close and an open would lose
/// the tab's identity halfway through. Every id is minted here all the
/// same: a caller may only rearrange, rename and re-point the rows it
/// was given, and a row carrying an id this machine never minted is
/// refused rather than trusted — a face that invented ids would be the
/// second minter this module exists to prevent.
pub fn save_all(root: &Path, kind: &str, want: &Open) -> Result<(), String> {
    change(root, kind, |open| {
        let known: Vec<String> = open.tabs.iter().map(|t| t.id.clone()).collect();
        for t in &want.tabs {
            if !known.contains(&t.id) {
                return Err(msg::no_such_tab(&t.id));
            }
        }
        open.tabs = want.tabs.clone();
        open.active = want.active.clone();
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::*;

    /// A scratch tree that takes itself away **however the test leaves**
    /// — `tests.rs`'s `Scratch`, kept here for its reason: a cleanup
    /// written at the end of a test is skipped by the assertion that
    /// fails, and a red run then leaves a directory in `$TMPDIR` every
    /// time it is red.
    struct Scratch(PathBuf);

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn root(tag: &str) -> Scratch {
        let p = std::env::temp_dir().join(format!("khor-tabs-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        Scratch(p)
    }

    /// Nothing open reads as nothing open, and **reading writes
    /// nothing**: a list asked for on a machine that has never had a
    /// tab must not leave a file behind, or 「has this person ever
    /// browsed」 stops being answerable.
    #[test]
    fn nothing_open_and_reading_leaves_no_file() {
        let r = root("empty");
        let open = list(&r.0, "page");
        assert_eq!(open.tabs.len(), 0);
        assert_eq!(open.active, "");
        assert!(!path_for_tests(&r.0).exists());
    }

    /// The list survives being written and read back — this is the
    /// whole of 「重启之后也还在」 at this layer, and the layer above
    /// it (the window) does no remembering of its own.
    #[test]
    fn a_tab_outlives_the_process_that_opened_it() {
        let r = root("restart");
        let id = open(&r.0, "page", "abc123", "https://khor.example/a").unwrap();
        go(
            &r.0,
            "page",
            &id,
            "abc123",
            "https://khor.example/a",
            "khor.example",
        )
        .unwrap();
        // Read afresh, exactly as the next launch would.
        let again = list(&r.0, "page");
        assert_eq!(again.tabs.len(), 1);
        assert_eq!(again.active, id);
        assert_eq!(again.tabs[0].url, "https://khor.example/a");
        assert_eq!(again.tabs[0].title, "khor.example");
    }

    /// Two kinds cannot read each other's tabs.
    ///
    /// **The danger is not a crash, it is a clean parse**: a URL and a
    /// path are both strings, so one landing would come up holding the
    /// other's rows and every refusal after that would be about the
    /// wrong subject.
    #[test]
    fn one_kind_cannot_see_anothers() {
        let r = root("kinds");
        open(&r.0, "page", "abc123", "https://khor.example/a").unwrap();
        assert_eq!(list(&r.0, "page").tabs.len(), 1);
        assert_eq!(list(&r.0, "code").tabs.len(), 0);
        open(&r.0, "code", "abc123", "/home/x/work").unwrap();
        assert_eq!(list(&r.0, "page").tabs.len(), 1);
        assert_eq!(list(&r.0, "code").tabs.len(), 1);
        assert_eq!(list(&r.0, "page").tabs[0].url, "https://khor.example/a");
    }

    /// Closing the tab in front hands the front to its **right-hand**
    /// neighbour, and falls back to the left at the end of the list.
    #[test]
    fn closing_the_front_tab_moves_right_then_left() {
        let r = root("close");
        let a = open(&r.0, "page", "m", "a").unwrap();
        let b = open(&r.0, "page", "m", "b").unwrap();
        let c = open(&r.0, "page", "m", "c").unwrap();
        focus(&r.0, "page", &b).unwrap();
        close(&r.0, "page", &b).unwrap();
        assert_eq!(
            list(&r.0, "page").active,
            c,
            "the right-hand neighbour takes the front"
        );
        close(&r.0, "page", &c).unwrap();
        assert_eq!(
            list(&r.0, "page").active,
            a,
            "at the end of the list, the left one does"
        );
        close(&r.0, "page", &a).unwrap();
        let empty = list(&r.0, "page");
        assert_eq!(empty.tabs.len(), 0);
        assert_eq!(
            empty.active, "",
            "an empty list has no front; a row is not invented here"
        );
    }

    /// Closing a tab that is **not** in front leaves the front alone.
    #[test]
    fn closing_a_background_tab_does_not_move_the_front() {
        let r = root("bg");
        let a = open(&r.0, "page", "m", "a").unwrap();
        let b = open(&r.0, "page", "m", "b").unwrap();
        focus(&r.0, "page", &b).unwrap();
        close(&r.0, "page", &a).unwrap();
        assert_eq!(list(&r.0, "page").active, b);
    }

    /// Every verb that names a tab refuses one that is not there, **by
    /// name**, rather than doing nothing or quietly adding a row.
    #[test]
    fn a_tab_that_is_not_there_is_refused_by_name() {
        let r = root("missing");
        let id = open(&r.0, "page", "m", "a").unwrap();
        for said in [
            close(&r.0, "page", "nope").unwrap_err(),
            focus(&r.0, "page", "nope").unwrap_err(),
            go(&r.0, "page", "nope", "m", "b", "b").unwrap_err(),
        ] {
            assert!(said.contains("nope"), "the refusal names the tab: {said}");
        }
        // …and nothing was disturbed on the way through.
        let open = list(&r.0, "page");
        assert_eq!(open.tabs.len(), 1);
        assert_eq!(open.active, id);
    }

    /// A stored `active` naming no row is the same failure as none at
    /// all, and both land on the first tab.
    ///
    /// **Written as a file rather than through the verbs**, because a
    /// file edited by hand or by an older build is exactly the case
    /// this is about — reaching it through [`focus`] would be reaching
    /// it through the guard that prevents it.
    #[test]
    fn a_dangling_front_reads_as_the_first_tab() {
        let r = root("dangling");
        let id = open(&r.0, "page", "m", "a").unwrap();
        let mut all: BTreeMap<String, Open> =
            serde_json::from_str(&fs::read_to_string(path_for_tests(&r.0)).unwrap()).unwrap();
        all.get_mut("page").unwrap().active = "gone".into();
        fs::write(
            path_for_tests(&r.0),
            serde_json::to_vec_pretty(&all).unwrap(),
        )
        .unwrap();
        assert_eq!(list(&r.0, "page").active, id);
    }

    /// A file nobody can parse reads as nothing open rather than
    /// refusing: what a person loses to it is their tabs, and what they
    /// must never lose to it is the app coming up.
    #[test]
    fn an_unreadable_file_reads_as_nothing_open() {
        let r = root("garbage");
        fs::create_dir_all(r.0.join(".khor")).unwrap();
        fs::write(path_for_tests(&r.0), b"{ this is not json").unwrap();
        assert_eq!(list(&r.0, "page").tabs.len(), 0);
        // …and the next write repairs it rather than refusing forever.
        let id = open(&r.0, "page", "m", "a").unwrap();
        assert_eq!(list(&r.0, "page").active, id);
    }

    /// The bulk save may rearrange, rename and re-point the rows it was
    /// given — and **may not invent an id**.
    #[test]
    fn the_bulk_save_reorders_but_cannot_mint() {
        let r = root("bulk");
        let a = open(&r.0, "page", "m", "a").unwrap();
        let b = open(&r.0, "page", "m", "b").unwrap();
        let mut want = list(&r.0, "page");
        want.tabs.reverse();
        want.active = a.clone();
        save_all(&r.0, "page", &want).unwrap();
        let after = list(&r.0, "page");
        assert_eq!(
            after.tabs.iter().map(|t| t.id.clone()).collect::<Vec<_>>(),
            vec![b.clone(), a.clone()],
        );
        assert_eq!(after.active, a);

        let mut forged = after.clone();
        forged.tabs.push(Tab {
            id: "mine".into(),
            device: "m".into(),
            url: "c".into(),
            title: String::new(),
        });
        let said = save_all(&r.0, "page", &forged).unwrap_err();
        assert!(said.contains("mine"), "the refusal names the id: {said}");
        assert_eq!(list(&r.0, "page").tabs.len(), 2, "and nothing was written");
    }

    /// Two tabs minted in the same millisecond are still two tabs.
    #[test]
    fn ids_do_not_collide_within_a_millisecond() {
        let r = root("mint");
        let ids: Vec<String> = (0..50)
            .map(|_| open(&r.0, "page", "m", "a").unwrap())
            .collect();
        let mut sorted = ids.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), ids.len(), "every id is its own");
    }

    /// …**and two processes in the same millisecond are still two
    /// processes**, which is the half the first version of [`mint`] got
    /// wrong: a per-process counter separates tabs within one process
    /// and nothing at all between two. The window and `khor tabs open`
    /// are exactly two processes over one file.
    ///
    /// Asserted on the id's shape rather than by forking, because what
    /// makes two processes differ is the only thing a second process
    /// would contribute — so a fork would be an expensive way of
    /// reading `std::process::id()`.
    #[test]
    fn an_id_carries_the_process_that_minted_it() {
        let r = root("pid");
        let id = open(&r.0, "page", "m", "a").unwrap();
        let mine = format!("{:x}", std::process::id());
        assert!(
            id.contains(&format!("-{mine}-")),
            "the id says which process minted it ({id} should carry {mine})",
        );
    }
}
