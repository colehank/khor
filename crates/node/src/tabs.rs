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
    /// Put at the top by a person (docs/SHELL.md 第二轮). A pinned tab
    /// sorts above every unpinned one and cannot be closed until it is
    /// unpinned.
    #[serde(default)]
    pub pinned: bool,
    /// When a person last put this tab in front or pointed it somewhere,
    /// in milliseconds — what 标签页上限 reads to find the one touched
    /// longest ago (docs/SHELL.md 第三轮). Zero on a file written before
    /// it existed, which reads as 「longest ago」, the honest answer.
    #[serde(default)]
    pub touched: u64,
    /// **The session it was opened from** (UX-37 §1): a URL a terminal
    /// printed, or a page opened in that session's 附栏 — the id of the
    /// session row. Empty where none is known (a tab opened on the
    /// landing, any tab written before this existed), and such a tab is
    /// in no session's slice. Never written while empty, so a file read
    /// by an older build looks the same as before.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub session: String,
    /// Session content's renderer, independent of its address. Missing on
    /// existing browsing tabs; workspace/file/page on kind `content`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view: Option<String>,
    /// An opening request may be retried after losing its acknowledgement.
    /// Kept on the node so distinct blank tabs never deduplicate by address.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub request: String,
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
    ///
    /// **It also keeps every pinned tab above every unpinned one**, and
    /// that is here rather than in [`pin`] because more than one road can
    /// reorder the list: a drag through [`save_all`] that lifted a tab
    /// over a pin would otherwise be written as asked. A stable sort, so
    /// the order within each half is whatever the caller made it.
    fn settle(&mut self) {
        self.tabs.sort_by_key(|t| !t.pinned);
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

/// Written **beside** the file and renamed over it, so a reader never
/// sees half a list.
///
/// The temp name carries this process, because two of them really do
/// write here (第 2 条 put a terminal and a window over one file) and a
/// shared temp name is two writers clobbering each other's draft before
/// either has renamed.
fn save(root: &Path, all: &BTreeMap<String, Open>) -> Result<(), String> {
    let dot = root.join(".khor");
    std::fs::create_dir_all(&dot).map_err(|_| msg::cant_make_dir(dot.display()))?;
    let text = serde_json::to_vec_pretty(all).map_err(|e| e.to_string())?;
    let p = path(root);
    let tmp = dot.join(format!("tabs.json.{}", std::process::id()));
    std::fs::write(&tmp, text).map_err(|e| msg::cant_write(tmp.display(), e))?;
    std::fs::rename(&tmp, &p).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        msg::cant_write(p.display(), e)
    })
}

/// An exclusive hold on the tab table for the length of one
/// read-modify-write.
///
/// # Why a lock at all
///
/// Every change here is read-the-file, change it, write it back — and
/// since docs/SHELL.md 第 2 条 there are **two processes** doing that:
/// the window and `khor tabs`. Two interleaved read-modify-writes lose
/// one of the two changes outright, and what a person sees is a tab
/// they just opened quietly not being there.
///
/// Atomic rename alone does not fix it. Rename stops a reader seeing a
/// half-written file; it does nothing about two writers who both read
/// the same 「before」.
///
/// # Why `flock` on a file of its own
///
/// **Of its own**, because [`save`] renames the table — a lock taken on
/// the table's inode would be released the moment somebody replaced it,
/// and the next writer would lock a file nobody else is holding. The
/// lock file is only ever created and locked, never rewritten.
///
/// **`flock` rather than a lock somebody has to remove**, because the
/// kernel drops it when the fd closes: a process killed mid-change
/// leaves nothing to clean up and nothing to time out. A lock file this
/// app had to garbage-collect would be one more thing that goes wrong
/// on the machine the user is actually using.
struct Hold(std::fs::File);

impl Hold {
    fn take(root: &Path) -> Result<Hold, String> {
        let dot = root.join(".khor");
        std::fs::create_dir_all(&dot).map_err(|_| msg::cant_make_dir(dot.display()))?;
        let p = dot.join("tabs.lock");
        let f = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&p)
            .map_err(|e| msg::cant_write(p.display(), e))?;
        // Blocks until the other writer is done. There is no timeout on
        // purpose: what is held is one read-modify-write of a few
        // hundred bytes, so the wait is bounded by that and not by
        // anything a person or a network does.
        let got = unsafe { libc::flock(std::os::unix::io::AsRawFd::as_raw_fd(&f), libc::LOCK_EX) };
        if got != 0 {
            return Err(msg::cant_write(
                p.display(),
                std::io::Error::last_os_error(),
            ));
        }
        Ok(Hold(f))
    }
}

impl Drop for Hold {
    fn drop(&mut self) {
        // Closing the fd releases it; this is the explicit spelling so
        // the release is not a thing a reader has to know about `File`.
        let _ = unsafe {
            libc::flock(
                std::os::unix::io::AsRawFd::as_raw_fd(&self.0),
                libc::LOCK_UN,
            )
        };
    }
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
    // **Held across the read as well as the write.** A lock taken only
    // for the write would let both writers read the same 「before」 and
    // then take turns overwriting it, which is the whole failure.
    let _hold = Hold::take(root)?;
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
fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

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
    open_from(root, kind, device, url, "")
}

/// [`open`], remembering the session it was opened from (`Tab::session`;
/// empty = none known).
pub fn open_from(root: &Path, kind: &str, device: &str, url: &str, session: &str) -> Result<String, String> {
    open_beside(root, kind, device, url, session, "")
}

/// [`open_from`], the new row right after the tab `after` rather than at
/// the end — **a link that asked for a new window** (UX-42 §15): Safari
/// puts that tab beside the one it came from. An `after` naming no row
/// (or none) is the end, as [`open`].
pub fn open_beside(root: &Path, kind: &str, device: &str, url: &str, session: &str, after: &str) -> Result<String, String> {
    open_with_view(root, kind, device, url, session, after, None)
}

pub fn open_with_view(root: &Path, kind: &str, device: &str, url: &str, session: &str, after: &str, view: Option<&str>) -> Result<String, String> {
    open_requested(root, kind, device, url, session, after, view, "")
}

pub fn open_requested(root: &Path, kind: &str, device: &str, url: &str, session: &str, after: &str, view: Option<&str>, request: &str) -> Result<String, String> {
    let view = checked_view(kind, view)?;
    let limit = crate::config::tabs_limit(root);
    change(root, kind, |open| {
        if !request.is_empty() {
            if let Some(tab) = open.tabs.iter().find(|t| t.request == request && t.session == session) {
                let id = tab.id.clone();
                open.active = id.clone();
                return Ok(id);
            }
        }
        let id = mint();
        let at = open.tabs.iter().position(|t| !after.is_empty() && t.id == after).map_or(open.tabs.len(), |i| i + 1);
        open.tabs.insert(at, Tab {
            id: id.clone(),
            device: device.to_string(),
            url: url.to_string(),
            title: String::new(),
            pinned: false,
            touched: now_ms(),
            session: session.to_string(),
            view: view.clone(),
            request: request.to_owned(),
        });
        open.active = id.clone();
        if let Some(limit) = limit
            && open.tabs.len() > limit
            && let Some(at) = stalest(open)
        {
            open.tabs.remove(at);
        }
        Ok(id)
    })
}

/// Which tab 标签页上限 closes (docs/SHELL.md 第三轮 第 4 条): the one
/// **touched longest ago** that is neither pinned nor in front — the tab
/// just opened is in front, so it is never its own victim. Ties go to
/// the one further left. `None` when every tab is kept, and then nothing
/// closes: a strip that grows past the limit is better than one that
/// drops something a person said to keep.
fn stalest(open: &Open) -> Option<usize> {
    open.tabs
        .iter()
        .enumerate()
        .filter(|(_, t)| !t.pinned && t.id != open.active)
        .min_by_key(|(i, t)| (t.touched, *i))
        .map(|(i, _)| i)
}

/// Adds a tab **already pinned**, without putting it in front — what a
/// migrated 「置顶的网页」 row becomes (`Node::migrate_place_pins`). Not
/// [`open`]: a migration is not somebody opening a page, so it takes
/// nobody's front tab and it is not counted against 标签页上限 (a pin is
/// never what the limit closes anyway). Answers the new id.
pub fn adopt_pinned(root: &Path, kind: &str, device: &str, url: &str) -> Result<String, String> {
    change(root, kind, |open| {
        let id = mint();
        open.tabs.push(Tab {
            id: id.clone(),
            device: device.to_string(),
            url: url.to_string(),
            title: String::new(),
            pinned: true,
            touched: 0,
            session: String::new(),
            view: None,
            request: String::new(),
        });
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
    go_with_view(root, kind, id, device, url, title, None)
}

fn checked_view(kind: &str, view: Option<&str>) -> Result<Option<String>, String> {
    match view {
        Some("workspace" | "file" | "page") if kind == "content" => Ok(view.map(str::to_owned)),
        None => Ok(None),
        _ => Err("invalid content view".to_string()),
    }
}

pub fn go_with_view(root: &Path, kind: &str, id: &str, device: &str, url: &str, title: &str, view: Option<&str>) -> Result<(), String> {
    let view = checked_view(kind, view)?;
    change(root, kind, |open| {
        let tab = open
            .tabs
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| msg::no_such_tab(id))?;
        tab.device = device.to_string();
        tab.url = url.to_string();
        tab.title = title.to_string();
        if view.is_some() { tab.view = view; }
        tab.touched = now_ms();
        Ok(())
    })
}

/// Pins or unpins a tab (docs/SHELL.md 第二轮).
///
/// **Either way it lands on the seam between the two halves**: pinned,
/// under the tabs already pinned — the order they were pinned in is
/// the order they read; unpinned, at the head of the rest, nearest to
/// where it just was.
pub fn pin(root: &Path, kind: &str, id: &str, on: bool) -> Result<(), String> {
    change(root, kind, |open| {
        let at = open
            .tabs
            .iter()
            .position(|t| t.id == id)
            .ok_or_else(|| msg::no_such_tab(id))?;
        let mut tab = open.tabs.remove(at);
        tab.pinned = on;
        let seam = open.tabs.iter().take_while(|t| t.pinned).count();
        open.tabs.insert(seam, tab);
        Ok(())
    })
}

/// Brings a tab to the front.
pub fn focus(root: &Path, kind: &str, id: &str) -> Result<(), String> {
    change(root, kind, |open| {
        let tab = open
            .tabs
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| msg::no_such_tab(id))?;
        tab.touched = now_ms();
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
///
/// **A pinned tab is refused**: a pin is a person saying 「keep this
/// one」, so it has to be taken off before the tab can go — the same
/// sentence for the window and for `khor tabs close`.
pub fn close(root: &Path, kind: &str, id: &str) -> Result<(), String> {
    change(root, kind, |open| {
        let at = open
            .tabs
            .iter()
            .position(|t| t.id == id)
            .ok_or_else(|| msg::no_such_tab(id))?;
        if open.tabs[at].pinned {
            return Err(msg::tab_pinned().into());
        }
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
///
/// **What is pinned is kept from the file, not taken from the caller**:
/// a drag is not a pin, and [`pin`] is the one road that says either.
/// The same for where a tab came from: a face that never heard of
/// `session` sends rows without it, and a drag must not forget it.
pub fn save_all(root: &Path, kind: &str, want: &Open) -> Result<(), String> {
    change(root, kind, |open| {
        let mut rows = Vec::with_capacity(want.tabs.len());
        for t in &want.tabs {
            let Some(was) = open.tabs.iter().find(|k| k.id == t.id) else {
                return Err(msg::no_such_tab(&t.id));
            };
            rows.push(Tab {
                pinned: was.pinned,
                touched: was.touched,
                session: was.session.clone(),
                view: was.view.clone(),
                request: was.request.clone(),
                ..t.clone()
            });
        }
        open.tabs = rows;
        open.active = want.active.clone();
        Ok(())
    })
}

/// Reorder only the named rows of one session, under the same file lock as
/// open/close. Concurrent additions, metadata and other sessions stay intact.
/// Rows closed since the gesture began are ignored; foreign rows are refused.
pub fn reorder(root: &Path, kind: &str, session: &str, ids: &[String]) -> Result<(), String> {
    change(root, kind, |open| {
        let mut rank = std::collections::HashMap::new();
        for id in ids {
            if let Some(row) = open.tabs.iter().find(|row| &row.id == id) {
                if row.session != session {
                    return Err(msg::no_such_tab(id));
                }
                let next = rank.len();
                rank.entry(id.clone()).or_insert(next);
            }
        }
        let mut rows: Vec<_> = open.tabs.iter().filter(|row| rank.contains_key(&row.id)).cloned().collect();
        rows.sort_by_key(|row| rank[&row.id]);
        let mut ordered = rows.into_iter();
        for row in &mut open.tabs {
            if rank.contains_key(&row.id) {
                *row = ordered.next().expect("one replacement per named row");
            }
        }
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
            pinned: false,
            touched: 0,
            session: String::new(),
            view: None,
            request: String::new(),
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

    /// **Two writers do not lose each other's tabs.**
    ///
    /// Every `open` is a read-modify-write, and since 第 2 条 there are
    /// two processes doing them over one file. Without [`Hold`] the two
    /// read the same 「before」 and take turns overwriting it, and what
    /// a person sees is a tab they just opened quietly not being there.
    ///
    /// ⚠️ **Threads, not processes, and that is honest rather than
    /// convenient.** `flock` is held per *open file description*, so
    /// two threads that each open the lock file contend exactly the way
    /// two processes do — which is what makes this a real test of the
    /// lock and not of a mutex. What it does **not** cover is the lock
    /// surviving a process being killed mid-change; that is the
    /// kernel's promise about `flock`, and it is why this is `flock` and
    /// not a lock file khor would have to garbage-collect.
    #[test]
    fn two_writers_do_not_lose_each_others_tabs() {
        let r = root("race");
        // Warm the file first: the interesting interleaving is over an
        // existing table, not over its creation.
        open(&r.0, "page", "m", "first").unwrap();
        let here = r.0.clone();
        let hands: Vec<_> = (0..4)
            .map(|n| {
                let at = here.clone();
                std::thread::spawn(move || {
                    for i in 0..10 {
                        open(&at, "page", "m", &format!("u{n}-{i}")).unwrap();
                    }
                })
            })
            .collect();
        for h in hands {
            h.join().unwrap();
        }
        let after = list(&r.0, "page");
        assert_eq!(
            after.tabs.len(),
            41,
            "every write landed: 1 + 4 writers × 10 (got {:?})",
            after.tabs.iter().map(|t| &t.url).collect::<Vec<_>>(),
        );
        // …and the file is still one readable list, not a torn one.
        let again = list(&r.0, "page");
        assert_eq!(again.tabs.len(), 41);
        assert!(after.tabs.iter().any(|t| t.id == after.active));
    }

    /// The table is renamed into place, so a reader never sees a
    /// half-written list — and the draft carries this process's name,
    /// because two writers sharing one draft would clobber each other
    /// before either renamed.
    #[test]
    fn a_write_leaves_no_draft_behind() {
        let r = root("atomic");
        open(&r.0, "page", "m", "a").unwrap();
        let strays: Vec<String> = fs::read_dir(r.0.join(".khor"))
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.starts_with("tabs.json."))
            .collect();
        assert!(strays.is_empty(), "a draft was left behind: {strays:?}");
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

    fn order(r: &Scratch) -> Vec<String> {
        list(&r.0, "page")
            .tabs
            .iter()
            .map(|t| t.url.clone())
            .collect()
    }

    /// **A pinned tab sorts above every unpinned one** (docs/SHELL.md
    /// 第二轮:「置顶的标签排在最上」), in the order they were pinned —
    /// and unpinning puts it back at the head of the unpinned rows, the
    /// place nearest where the eye already is.
    #[test]
    fn pinning_floats_a_tab_and_unpinning_sets_it_down_below_the_pins() {
        let r = root("pin");
        let _a = open(&r.0, "page", "m", "a").unwrap();
        let b = open(&r.0, "page", "m", "b").unwrap();
        let c = open(&r.0, "page", "m", "c").unwrap();
        pin(&r.0, "page", &c, true).unwrap();
        assert_eq!(order(&r), ["c", "a", "b"], "the pinned tab leads");
        pin(&r.0, "page", &b, true).unwrap();
        assert_eq!(
            order(&r),
            ["c", "b", "a"],
            "a second pin lands under the first"
        );
        pin(&r.0, "page", &c, false).unwrap();
        assert_eq!(order(&r), ["b", "c", "a"], "unpinned, it heads the rest");
        // Read afresh, as the next launch would: the mark is in the file.
        let again = list(&r.0, "page");
        let flags: Vec<bool> = again.tabs.iter().map(|t| t.pinned).collect();
        assert_eq!(flags, [true, false, false]);
    }

    /// A tab opened after a pin still lands under the pins: a new row
    /// is not a decision anybody made about the top.
    #[test]
    fn a_new_tab_opens_below_the_pinned_ones() {
        let r = root("pin-new");
        let a = open(&r.0, "page", "m", "a").unwrap();
        pin(&r.0, "page", &a, true).unwrap();
        open(&r.0, "page", "m", "b").unwrap();
        assert_eq!(order(&r), ["a", "b"]);
    }

    /// **A pinned tab cannot be closed until it is unpinned** — the
    /// ruling's 「关不掉之前要先取消置顶」, held here so the terminal and
    /// the window are refused alike.
    #[test]
    fn a_pinned_tab_refuses_to_close_until_unpinned() {
        let r = root("pin-close");
        let a = open(&r.0, "page", "m", "a").unwrap();
        pin(&r.0, "page", &a, true).unwrap();
        let said = close(&r.0, "page", &a).unwrap_err();
        assert_eq!(said, msg::tab_pinned());
        assert_eq!(list(&r.0, "page").tabs.len(), 1, "and nothing was closed");
        pin(&r.0, "page", &a, false).unwrap();
        close(&r.0, "page", &a).unwrap();
        assert_eq!(list(&r.0, "page").tabs.len(), 0);
    }

    /// Pinning a tab that is not there is refused by name, like every
    /// other verb here.
    #[test]
    fn pinning_a_missing_tab_is_refused_by_name() {
        let r = root("pin-missing");
        open(&r.0, "page", "m", "a").unwrap();
        let said = pin(&r.0, "page", "nope", true).unwrap_err();
        assert!(said.contains("nope"), "the refusal names the tab: {said}");
    }

    /// The bulk save may reorder, **but not lift a tab over a pin, and
    /// not change what is pinned**: that is [`pin`]'s to say, and a
    /// drag is not a pin.
    #[test]
    fn the_bulk_save_cannot_unpin_or_climb_over_a_pin() {
        let r = root("pin-bulk");
        let a = open(&r.0, "page", "m", "a").unwrap();
        open(&r.0, "page", "m", "b").unwrap();
        pin(&r.0, "page", &a, true).unwrap();
        let mut want = list(&r.0, "page");
        want.tabs.reverse();
        for t in &mut want.tabs {
            t.pinned = false;
        }
        save_all(&r.0, "page", &want).unwrap();
        let after = list(&r.0, "page");
        assert_eq!(order(&r), ["a", "b"], "the pinned tab still leads");
        assert!(after.tabs[0].pinned, "and it is still pinned");
    }

    #[test]
    fn reorder_preserves_concurrent_rows_metadata_and_other_sessions() {
        let r = root("reorder");
        let a = open_with_view(&r.0, "content", "m", "a", "A", "", Some("file")).unwrap();
        let foreign = open_from(&r.0, "content", "m", "foreign", "B").unwrap();
        let b = open_from(&r.0, "content", "m", "b", "A").unwrap();
        let closed = open_from(&r.0, "content", "m", "closed", "A").unwrap();
        close(&r.0, "content", &closed).unwrap();
        let new = open_from(&r.0, "content", "m", "new", "A").unwrap();
        let before = list(&r.0, "content");
        reorder(&r.0, "content", "A", &[b.clone(), closed, a.clone(), b.clone()]).unwrap();
        let after = list(&r.0, "content");
        assert_eq!(after.tabs.iter().map(|t| &t.id).collect::<Vec<_>>(), [&b, &foreign, &a, &new]);
        assert_eq!(after.active, before.active);
        for row in &after.tabs { assert_eq!(Some(row), before.tabs.iter().find(|old| old.id == row.id)); }
        assert!(reorder(&r.0, "content", "A", &[foreign]).is_err());
        assert_eq!(list(&r.0, "content"), after);
        pin(&r.0, "content", &a, true).unwrap();
        reorder(&r.0, "content", "A", &[b, a.clone()]).unwrap();
        assert_eq!(list(&r.0, "content").tabs[0].id, a);
    }

    /// 标签页上限 (docs/SHELL.md 第三轮 第 4 条): past the limit, opening
    /// one more closes the tab **touched longest ago** — not the oldest
    /// opened — and never a pinned one or the one in front.
    #[test]
    fn past_the_limit_the_tab_touched_longest_ago_goes() {
        let r = root("limit");
        crate::config::set(&r.0, "tabs-limit", "4").unwrap();
        let a = open(&r.0, "page", "m", "a").unwrap();
        let b = open(&r.0, "page", "m", "b").unwrap();
        open(&r.0, "page", "m", "c").unwrap();
        open(&r.0, "page", "m", "d").unwrap();
        // `a` was opened first but touched since; `b` is now the stalest.
        std::thread::sleep(std::time::Duration::from_millis(3));
        focus(&r.0, "page", &a).unwrap();
        open(&r.0, "page", "m", "e").unwrap();
        let left: Vec<String> = list(&r.0, "page")
            .tabs
            .iter()
            .map(|t| t.url.clone())
            .collect();
        assert_eq!(left, ["a", "c", "d", "e"], "b was touched longest ago");
        assert!(!list(&r.0, "page").tabs.iter().any(|t| t.id == b));
    }

    /// A pinned tab is kept however stale, and when every candidate is
    /// pinned nothing is closed — the strip grows past the limit rather
    /// than losing something a person said to keep.
    #[test]
    fn the_limit_never_takes_a_pinned_tab() {
        let r = root("limit-pin");
        crate::config::set(&r.0, "tabs-limit", "4").unwrap();
        let ids: Vec<String> = ["a", "b", "c", "d"]
            .iter()
            .map(|u| open(&r.0, "page", "m", u).unwrap())
            .collect();
        pin(&r.0, "page", &ids[0], true).unwrap();
        open(&r.0, "page", "m", "e").unwrap();
        let left: Vec<String> = list(&r.0, "page")
            .tabs
            .iter()
            .map(|t| t.url.clone())
            .collect();
        assert!(
            left.contains(&"a".to_string()),
            "the pinned tab stays: {left:?}"
        );
        assert_eq!(left.len(), 4);
        for t in list(&r.0, "page").tabs {
            pin(&r.0, "page", &t.id, true).unwrap();
        }
        open(&r.0, "page", "m", "f").unwrap();
        assert_eq!(
            list(&r.0, "page").tabs.len(),
            5,
            "all pinned: nothing closes"
        );
    }

    /// `off` is what the strips always did: nothing is ever closed.
    #[test]
    fn with_no_limit_nothing_is_closed() {
        let r = root("limit-off");
        for u in 0..12 {
            open(&r.0, "page", "m", &u.to_string()).unwrap();
        }
        assert_eq!(list(&r.0, "page").tabs.len(), 12);
    }

    /// **A tab a page opened sits beside the page** (UX-42 §15), and an
    /// opener nobody knows is the end, as a plain open.
    #[test]
    fn a_tab_a_page_opened_sits_beside_it() {
        let r = root("beside");
        let a = open(&r.0, "page", "m", "a").unwrap();
        open(&r.0, "page", "m", "b").unwrap();
        let c = open_beside(&r.0, "page", "m", "c", "tui/one", &a).unwrap();
        assert_eq!(order(&r), ["a", "c", "b"]);
        let now = list(&r.0, "page");
        assert_eq!(now.active, c, "the new tab is in front");
        assert_eq!(now.tabs[1].session, "tui/one", "it keeps the opener's session");
        open_beside(&r.0, "page", "m", "d", "", "gone").unwrap();
        assert_eq!(order(&r), ["a", "c", "b", "d"], "an unknown opener is the end");
    }

    /// **Where a tab came from survives a drag, and costs nothing where
    /// it is unknown** (UX-37 §1). A face that predates `session` sends
    /// rows without it through `save_all`, and the session must be kept
    /// from the file; a tab with none writes no key, so the file an older
    /// build reads is the file it always read.
    #[test]
    fn the_session_a_tab_came_from_is_kept() {
        let r = root("session");
        let here = open_from(&r.0, "page", "m", "https://a.example", "tui/one").unwrap();
        let bare = open(&r.0, "page", "m", "https://b.example").unwrap();
        let mut dragged = list(&r.0, "page");
        for t in &mut dragged.tabs {
            t.session = String::new();
        }
        dragged.tabs.reverse();
        save_all(&r.0, "page", &dragged).unwrap();
        let now = list(&r.0, "page");
        let of = |id: &str| now.tabs.iter().find(|t| t.id == id).unwrap().session.clone();
        assert_eq!(of(&here), "tui/one", "the drag kept where it came from");
        assert_eq!(of(&bare), "", "control: a tab with no session has none");
        let text = fs::read_to_string(path_for_tests(&r.0)).unwrap();
        assert_eq!(text.matches("\"session\"").count(), 1, "only the tab with one writes the key: {text}");
    }
    #[test]
    fn session_content_keeps_its_handle_and_renderer_across_restart_and_bulk_save() {
        let r = root("content");
        let id = open_with_view(&r.0, "content", "device-a", "/repo", "session-a", "", Some("workspace")).unwrap();
        go_with_view(&r.0, "content", &id, "device-a", "/repo/readme.md", "readme.md", Some("file")).unwrap();
        let mut from_old_face = list(&r.0, "content");
        assert_eq!(from_old_face.tabs[0].view.as_deref(), Some("file"));
        assert_eq!(from_old_face.tabs[0].session, "session-a");
        from_old_face.tabs[0].view = None;
        save_all(&r.0, "content", &from_old_face).unwrap();
        let restored = list(&r.0, "content");
        assert_eq!(restored.tabs[0].id, id);
        assert_eq!(restored.tabs[0].view.as_deref(), Some("file"));
        assert!(list(&r.0, "page").tabs.is_empty());
    }

    #[test]
    fn blank_content_requests_are_distinct_and_retry_without_duplicates() {
        let r = root("blank-content-retry");
        let a = open_requested(&r.0, "content", "", "", "session-a", "", Some("workspace"), "open-a").unwrap();
        let b = open_requested(&r.0, "content", "", "", "session-a", "", Some("workspace"), "open-b").unwrap();
        assert_ne!(a, b);
        go_with_view(&r.0, "content", &a, "mac", "/repo/notes.md", "", Some("file")).unwrap();
        assert_eq!(open_requested(&r.0, "content", "", "", "session-a", "", Some("workspace"), "open-a").unwrap(), a);
        let opened = list(&r.0, "content");
        assert_eq!(opened.tabs.len(), 2);
        assert_eq!(opened.tabs.iter().find(|t| t.id == a).unwrap().url, "/repo/notes.md");
        let c = open_requested(&r.0, "content", "", "", "session-b", "", Some("workspace"), "open-a").unwrap();
        assert_ne!(a, c);
    }

    #[test]
    fn invalid_content_view_does_not_change_the_table() {
        let r = root("bad-content");
        let id = open_with_view(&r.0, "content", "device-a", "/repo", "s", "", Some("workspace")).unwrap();
        let before = list(&r.0, "content");
        assert!(go_with_view(&r.0, "content", &id, "device-a", "https://example.com", "", Some("unknown")).is_err());
        assert_eq!(before, list(&r.0, "content"));
        assert!(open_with_view(&r.0, "page", "device-a", "/repo", "s", "", Some("file")).is_err());
    }

}
