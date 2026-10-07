//! **The file road** (audit/STORE-BRIEF.md, foundation A): one way for
//! bytes to land on a machine, whatever asked for them — a file dropped on
//! a terminal, a file offered in a conversation, and (later) a file sent
//! straight into a conversation, a broadcast, the khor agent.
//!
//! A move is a [`Parcel`]: where the bytes come from ([`Source`]), where
//! they land ([`Dest`]), and the caller's policy (a size cap, whether a
//! broken pull is carried on from). [`Mode`] says who starts it: `Direct`
//! is pulled at once by the receiver, with nobody asked; `Offer` waits for
//! the receiver's 收下 and is pulled then. Either way the receiver is the
//! one that pulls (`Node::land`), and a [`Job`] carries its progress and
//! its cancel.
//!
//! **Landing.** Files land in the machine's receive directory unless the
//! caller names one: `~/khor-drop` by default (a phone: the app's
//! Documents), changed per machine on its row in the device table
//! (`DeviceDoc::set_receive_dir`) by anyone, resolved only by the machine
//! itself ([`resolve`]). A name already taken is suffixed (`a-2.txt`).
//!
//! **Receipts.** What landed in the receive directory is recorded by its
//! content digest under `.khor/received/<digest>.json` — how a message
//! that names a file by digest finds where it landed, now that the name on
//! disk is the person's own and no longer carries the digest. A receipt
//! whose file is gone, or no longer the size it landed at, finds nothing.
//! Partial bytes of an offer, and the pid of its live pull, sit beside it
//! (`<digest>.partial`, `<digest>.pulling`), keyed by content so a resumed
//! pull finds them whichever conversation asked.
//!
//! **Per receiver.** The sender keeps what each receiver did with an
//! offer (`transfer::Offer::to`), never one state per file: one file sent
//! to two machines is two deliveries.
//!
//! What phase 2 and 3 add (STORE-BRIEF): a conversation's file sent
//! `Direct` into the receive directory or a named one, is a `Parcel` with
//! `Source::Offer` pulled at once instead of on 收下; a broadcast is one
//! offer whose receipts are counted per receiver. Neither needs a second
//! road.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use khor_catalog::msg;
use khor_sync::chat::FileRef;

use crate::drops::DropState;

/// Who starts a move.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// The receiver pulls at once; nobody is asked (a terminal drop).
    Direct,
    /// The receiver pulls once it has accepted (a conversation's file).
    Offer,
}

/// Where the bytes are.
#[derive(Debug, Clone)]
pub enum Source {
    /// A file by its path on a machine, by channel name (`FetchPath`).
    Path { machine: String, path: String },
    /// An offered file by its digest, held by the device `from`
    /// (`Fetch`): verified against the digest before it gets its name.
    Offer { from: String, file: FileRef },
}

/// Where they land.
#[derive(Debug, Clone)]
pub enum Dest {
    /// The machine's receive directory ([`resolve`]).
    Receive,
    /// A directory the asker named, absolute and already there.
    Dir(PathBuf),
}

/// One move, as the receiver runs it.
#[derive(Debug, Clone)]
pub struct Parcel {
    pub mode: Mode,
    pub source: Source,
    pub into: Dest,
    /// The largest file this caller takes, checked against the size the
    /// first slice reports.
    pub cap: Option<u64>,
    /// Whether a pull broken by an unreachable machine keeps its part
    /// file to carry on from. A drop does not: it is asked again by
    /// dropping again.
    pub resume: bool,
}

/// What a move did: bytes moved this run (0 for a file already here),
/// where it landed, and its digest when the road knows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Landed {
    pub moved: u64,
    pub path: PathBuf,
    pub digest: Option<String>,
}

// ── jobs: progress and cancel ───────────────────────────────

/// How long a finished job's answer is kept for an asker that comes
/// back for it.
const KEPT_FOR: Duration = Duration::from_secs(600);

/// One pull, as it goes.
#[derive(Default)]
pub struct Job {
    pub moved: AtomicU64,
    pub total: AtomicU64,
    /// Set by [`stop`]: the pull ends before its next slice. Shared, so
    /// a pull on a blocking thread can read it.
    pub stopped: Arc<AtomicBool>,
    /// The landed path, or why it did not land; `None` while it runs.
    end: Mutex<Option<(Instant, Result<String, String>)>>,
}

impl Job {
    pub fn is_stopped(&self) -> bool {
        self.stopped.load(Ordering::Relaxed)
    }

    pub fn finish(&self, outcome: Result<String, String>) {
        if let Ok(mut end) = self.end.lock() {
            *end = Some((Instant::now(), outcome));
        }
    }

    pub fn state(&self) -> Result<DropState, String> {
        let end = self.end.lock().map(|e| e.as_ref().map(|(_, o)| o.clone())).unwrap_or(None);
        let (moved, total) = (self.moved.load(Ordering::Relaxed), self.total.load(Ordering::Relaxed));
        match end {
            None => Ok(DropState {
                moved,
                total,
                ..DropState::default()
            }),
            Some(Ok(dest)) => Ok(DropState {
                moved,
                total,
                dest,
                done: true,
            }),
            Some(Err(why)) => Err(why),
        }
    }

    fn ended_long_ago(&self) -> bool {
        self.end
            .lock()
            .map(|e| e.as_ref().is_some_and(|(at, _)| at.elapsed() > KEPT_FOR))
            .unwrap_or(false)
    }
}

fn jobs() -> &'static Mutex<HashMap<String, Arc<Job>>> {
    static JOBS: std::sync::OnceLock<Mutex<HashMap<String, Arc<Job>>>> = std::sync::OnceLock::new();
    JOBS.get_or_init(Default::default)
}

/// The job this key names, and whether it was made just now (the caller
/// then starts it). Keys are the asker's random words, so one table
/// serves every node in the process.
pub fn job(key: &str) -> (Arc<Job>, bool) {
    let mut all = jobs().lock().unwrap_or_else(|e| e.into_inner());
    all.retain(|_, j| !j.ended_long_ago());
    if let Some(j) = all.get(key) {
        return (j.clone(), false);
    }
    let j = Arc::new(Job::default());
    all.insert(key.to_owned(), j.clone());
    (j, true)
}

/// The job this key names, when there is one; makes none.
pub fn peek(key: &str) -> Option<Arc<Job>> {
    jobs().lock().unwrap_or_else(|e| e.into_inner()).get(key).cloned()
}

/// Drops the job this key names, so the next [`job`] starts a new one
/// (a retry of a move that ended).
pub fn forget_job(key: &str) {
    jobs().lock().unwrap_or_else(|e| e.into_inner()).remove(key);
}

/// The job key of a conversation file's move, by its message **and the
/// machine receiving it**: the table is the process's, and a broadcast
/// file is one message many machines move — two homes in one process
/// (a test's scratch nodes) must not answer each other's moves.
pub fn sent_key(msg_id: &str, receiver: &str) -> String {
    format!("sent-{receiver}-{msg_id}")
}

/// **The job this key names, cancelled**: its pull stops before the next
/// slice and answers [`cancelled`]. A key no job has yet is held as a job
/// already ended so — an ask that arrives after its own cancel then
/// starts nothing — and costs nothing else.
pub fn stop(key: &str) {
    let (j, fresh) = job(key);
    j.stopped.store(true, Ordering::Relaxed);
    if fresh {
        j.finish(Err(cancelled()));
    }
}

/// Why a cancelled move did not land.
pub fn cancelled() -> String {
    msg::drop_cancelled().to_owned()
}

// ── the receive directory ───────────────────────────────────

/// What `~` means for a store: the person's home for the machine's own
/// store (and a host room under it); the store itself for any other (a
/// scratch home, a test's), so no test and no scratch node ever writes
/// into the person's own. On a phone, the app's Documents — the one
/// place in its sandbox the Files app shows.
pub fn home_for(root: &Path) -> PathBuf {
    if let Some(home) = std::env::home_dir() {
        #[cfg(target_os = "ios")]
        if root == crate::Node::sandbox_root(&home) {
            return home.join("Documents");
        }
        if root == home || root.parent() == Some(&home.join(".khor-hosts")) {
            return home;
        }
    }
    root.to_path_buf()
}

/// The receive directory when nobody set one: `~/khor-drop`; on a phone,
/// Documents itself.
pub fn default_dir(root: &Path) -> PathBuf {
    let home = home_for(root);
    #[cfg(target_os = "ios")]
    if home.ends_with("Documents") {
        return home;
    }
    home.join("khor-drop")
}

/// A receive-dir setting tidied for the table, or refused: empty (the
/// default), `~`, `~/…`, or absolute, never with a `..` in it.
pub fn tidy_setting(dir: &str) -> Result<String, String> {
    let d = dir.trim();
    let d = if d.len() > 1 { d.trim_end_matches('/') } else { d };
    if d.is_empty() {
        return Ok(String::new());
    }
    let p = Path::new(d);
    let led = d == "~" || d.starts_with("~/") || p.is_absolute();
    if !led || p.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return Err(msg::receive_dir_not_absolute(d));
    }
    Ok(d.to_owned())
}

/// The setting as a path on this machine: `~` expanded ([`home_for`]),
/// nothing checked.
pub fn expand(root: &Path, setting: &str) -> PathBuf {
    match setting.strip_prefix('~') {
        Some(rest) => home_for(root).join(rest.trim_start_matches('/')),
        None => PathBuf::from(setting),
    }
}

/// **Where this machine's received files land**, for a setting (`None`
/// or empty for the default): the setting expanded, made when missing,
/// and checked writable — or, when it is not usable, the default, with
/// the reason. `make` false only reads (the open-time report), and a
/// directory that is not there yet then counts as usable.
pub fn resolve(root: &Path, setting: Option<&str>, make: bool) -> (PathBuf, Option<String>) {
    let default = default_dir(root);
    let Some(setting) = setting.filter(|s| !s.trim().is_empty()) else {
        if make {
            let _ = std::fs::create_dir_all(&default);
        }
        return (default, None);
    };
    let wanted = match tidy_setting(setting) {
        Ok(s) => expand(root, &s),
        Err(why) => return (default, Some(why)),
    };
    match usable(&wanted, make) {
        Ok(()) => (wanted, None),
        Err(why) => {
            if make {
                let _ = std::fs::create_dir_all(&default);
            }
            (default, Some(msg::receive_dir_unusable(wanted.display(), why)))
        }
    }
}

fn usable(dir: &Path, make: bool) -> Result<(), String> {
    if !make {
        return match std::fs::metadata(dir) {
            Ok(m) if !m.is_dir() => Err(msg::no_such_dir(dir.display())),
            _ => Ok(()),
        };
    }
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let probe = dir.join(format!(".khor-probe-{}", std::process::id()));
    std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&probe)
        .map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&probe);
    Ok(())
}

impl crate::Node {
    /// **Where files sent to this machine land now** (STORE-BRIEF phase
    /// 1): its own row's setting resolved ([`resolve`]), made when
    /// missing, the default when the setting is unusable — and that
    /// answer reported on the row (`receive_at`), so every other screen
    /// lists the directory the files really go to.
    pub fn receive_dir(&self) -> Result<PathBuf, String> {
        let (dir, why) = self.receive_place(true)?;
        if let Some(why) = why {
            eprintln!("{why}");
        }
        Ok(dir)
    }

    /// [`Self::receive_dir`] with the reason a setting did not take, and
    /// `make` false for a read that creates nothing. Reports the answer on
    /// this machine's row when it changed: where (`receive_at`) always,
    /// and — only after a full resolution — which setting that answers
    /// and why it fell back (`receive_for`, `receive_why`). A read of a
    /// setting already resolved in full answers what the row says, so it
    /// cannot undo a fallback that only making the directory found.
    pub fn receive_place(&self, make: bool) -> Result<(PathBuf, Option<String>), String> {
        let loaded = self.devices_loaded()?;
        let me = loaded.doc.get(&self.device_str);
        let setting = me.as_ref().and_then(|d| d.receive_dir.clone());
        let taken = me.as_ref().and_then(|d| d.receive_for.clone());
        if !make && taken == setting {
            if let Some(at) = me.as_ref().and_then(|d| d.receive_at.clone()) {
                return Ok((PathBuf::from(at), me.and_then(|d| d.receive_why)));
            }
        }
        let (dir, why) = resolve(&self.root, setting.as_deref(), make);
        let at = dir.display().to_string();
        let mut wrote = false;
        if me.as_ref().and_then(|d| d.receive_at.as_deref()) != Some(at.as_str()) {
            loaded.doc.set_receive_at(&self.device_str, &at)?;
            wrote = true;
        }
        if make && (taken != setting || me.as_ref().and_then(|d| d.receive_why.clone()) != why) {
            loaded
                .doc
                .set_receive_resolved(&self.device_str, setting.as_deref().unwrap_or(""), why.as_deref().unwrap_or(""))?;
            wrote = true;
        }
        if wrote {
            let mut store = loaded.store;
            store.flush(&loaded.doc)?;
        }
        Ok((dir, why))
    }

    /// **A setting changed elsewhere is taken on as soon as it is seen**
    /// (the lead, 2026-10-07): when this machine's own `receive_dir` is
    /// not the one it last resolved in full — set from another machine
    /// and arrived on a sync — it resolves it now (makes the directory,
    /// probes it, reports where and why), not at its next receive. The
    /// default needs nothing: unset and never resolved are one state.
    pub fn follow_receive_dir(&self) -> Result<(), String> {
        let me = self.devices_loaded()?.doc.get(&self.device_str);
        let setting = me.as_ref().and_then(|d| d.receive_dir.clone());
        let taken = me.and_then(|d| d.receive_for);
        if setting != taken {
            let (_, why) = self.receive_place(true)?;
            if let Some(why) = why {
                eprintln!("{why}");
            }
        }
        Ok(())
    }

    /// [`Self::follow_receive_dir`] once a sync has moved the device
    /// table: best effort — the sync itself went through, so a failure
    /// here is said, not handed back to it.
    pub(crate) fn follow_after_sync(&self) {
        if let Err(why) = self.follow_receive_dir() {
            eprintln!("{why}");
        }
    }

    /// Sets where files sent to `machine` land, for everyone; empty takes
    /// it back to the default. Refused here, at the one write entry
    /// ([`tidy_setting`]), so the CLI and the face say one sentence.
    /// Answers the setting as kept. This machine's own takes at once;
    /// another machine resolves it as soon as a sync brings it the change
    /// ([`Self::follow_receive_dir`]), or when it next starts.
    pub fn set_receive_dir(&self, machine: &str, dir: &str) -> Result<String, String> {
        let kept = tidy_setting(dir)?;
        let (_, home) = self.resolve(machine)?;
        let loaded = self.devices_loaded()?;
        loaded.doc.set_receive_dir(&home.hex(), &kept)?;
        let mut store = loaded.store;
        store.flush(&loaded.doc)?;
        if home.hex() == self.device_str {
            self.receive_place(true)?;
        }
        Ok(kept)
    }
}

// ── receipts ────────────────────────────────────────────────

/// One file that landed in the receive directory.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Receipt {
    pub digest: String,
    /// The name it was sent under.
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    /// The device id it came from; empty when not known.
    #[serde(default)]
    pub from: String,
    pub at_ms: i64,
    pub mode: Mode,
}

pub fn received_dir(root: &Path) -> PathBuf {
    root.join(".khor").join("received")
}

/// The digest lands in a filename; only lowercase blake3 hex survives.
fn keyed(root: &Path, digest: &str, ext: &str) -> Option<PathBuf> {
    (digest.len() == 64 && digest.bytes().all(|b| b.is_ascii_hexdigit()))
        .then(|| received_dir(root).join(format!("{}.{ext}", digest.to_ascii_lowercase())))
}

/// Where an offer's bytes grow until they are verified.
pub fn partial_of(root: &Path, digest: &str) -> Option<PathBuf> {
    keyed(root, digest, "partial")
}

/// The pid of a live pull of `digest`.
pub fn marker_of(root: &Path, digest: &str) -> Option<PathBuf> {
    keyed(root, digest, "pulling")
}

/// Records a landing, whole or not at all (`transfer::save_offer`'s
/// reason).
pub fn record(root: &Path, r: &Receipt) -> Result<(), String> {
    let path = keyed(root, &r.digest, "json").ok_or_else(|| msg::not_a_digest(&r.digest))?;
    std::fs::create_dir_all(received_dir(root)).map_err(|e| e.to_string())?;
    let tmp = path.with_extension(format!("{}.tmp", std::process::id()));
    std::fs::write(&tmp, serde_json::to_vec(r).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())
}

/// The receipt for `digest` whose file is still there at the size it
/// landed — `None` for a file deleted, moved or changed since.
pub fn receipt(root: &Path, digest: &str) -> Option<Receipt> {
    let text = std::fs::read(keyed(root, digest, "json")?).ok()?;
    let r: Receipt = serde_json::from_slice(&text).ok()?;
    let meta = std::fs::metadata(&r.path).ok()?;
    (meta.is_file() && meta.len() == r.size).then_some(r)
}

/// **Where each received file in `dir` came from, and when it landed**
/// (UX-48: a storage row reads the machine a file came from and how long
/// ago it was received), by file name: the receipts whose file is in
/// `dir` still at the size it landed. `dir` is canonical (a listing's).
/// Reads every receipt, so it is asked only beside a listing.
pub fn arrivals(root: &Path, dir: &Path) -> std::collections::HashMap<String, (String, i64)> {
    let mut out = std::collections::HashMap::new();
    let Ok(rd) = std::fs::read_dir(received_dir(root)) else {
        return out;
    };
    let mut canon: std::collections::HashMap<PathBuf, Option<PathBuf>> = Default::default();
    for e in rd.flatten() {
        let p = e.path();
        if p.extension().is_none_or(|x| x != "json") {
            continue;
        }
        let Some(r) = std::fs::read(&p).ok().and_then(|t| serde_json::from_slice::<Receipt>(&t).ok()) else {
            continue;
        };
        let (Some(parent), Some(name)) = (r.path.parent(), r.path.file_name().and_then(|n| n.to_str())) else {
            continue;
        };
        let here = canon
            .entry(parent.to_path_buf())
            .or_insert_with(|| parent.canonicalize().ok())
            .as_deref()
            == Some(dir);
        if !here || std::fs::metadata(&r.path).map_or(true, |m| !m.is_file() || m.len() != r.size) {
            continue;
        }
        // Two receipts for one name (one landed, deleted, another took
        // its name): the later landing is the file there now.
        let newer = out.get(name).is_none_or(|(_, at): &(String, i64)| r.at_ms > *at);
        if newer {
            out.insert(name.to_owned(), (r.from, r.at_ms));
        }
    }
    out
}

/// A listing of `dir` with each received file's sender and landing time
/// filled in from its receipt ([`arrivals`]); nothing read when the
/// listing holds no file.
pub fn mark_arrivals(root: &Path, dir: &Path, entries: &mut [crate::proto::DirEntry]) {
    if !entries.iter().any(|e| !e.dir) {
        return;
    }
    let got = arrivals(root, dir);
    if got.is_empty() {
        return;
    }
    for e in entries.iter_mut().filter(|e| !e.dir) {
        if let Some((from, at)) = got.get(&e.name) {
            e.from = (!from.is_empty()).then(|| from.clone());
            e.received_ms = u64::try_from(*at).ok();
        }
    }
}

/// Forgets a receipt (the file it names is left alone).
pub fn forget(root: &Path, digest: &str) {
    if let Some(p) = keyed(root, digest, "json") {
        let _ = std::fs::remove_file(p);
    }
}

/// Where a conversation's file is on this machine: its receipt, else the
/// conversation folder a khor before the receive directory landed it in
/// (`transfer::payload_path`) — a file the move to the receive directory
/// could not take stays there and is still found.
pub fn landed(root: &Path, channel_dir: &Path, f: &FileRef) -> Option<PathBuf> {
    if let Some(r) = receipt(root, &f.digest) {
        return Some(r.path);
    }
    let old = crate::transfer::payload_path(channel_dir, f);
    old.symlink_metadata().is_ok_and(|m| m.is_file()).then_some(old)
}

/// Moves a finished file into a reserved landing: a rename, or a copy
/// when the two are on different disks. The landing's part file goes
/// either way.
pub fn settle(from: &Path, at: &crate::files::Landing) -> Result<(), String> {
    let moved = std::fs::rename(from, &at.dest).or_else(|_| {
        std::fs::copy(from, &at.part)
            .and_then(|_| std::fs::rename(&at.part, &at.dest))
            .and_then(|()| std::fs::remove_file(from))
    });
    let _ = std::fs::remove_file(&at.part);
    moved.map_err(|e| e.to_string())
}

/// Now, in ms since the epoch.
pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// ── the move out of the conversation folders ────────────────

/// Marks a store whose conversation folders were emptied into the
/// receive directory: the scan is not made again.
fn moved_mark(root: &Path) -> PathBuf {
    received_dir(root).join("chat-files-moved")
}

/// **Files a conversation landed before the receive directory**
/// (`<store>/.khor/chat/<channel>/files/<digest8>-<name>`), moved into
/// `dir` once: under their own name (a taken one suffixed, as a drop's),
/// with a receipt, and on Unix a link left at the old path so a line that
/// still names it (an agent's transcript) opens it. A file that cannot be
/// moved stays where it is and is still found ([`landed`]); the scan is
/// made again at the next open until every file has moved. Answers how
/// many moved.
pub fn move_chat_files(root: &Path, dir: &Path) -> usize {
    if moved_mark(root).exists() {
        return 0;
    }
    let chats = root.join(".khor").join("chat");
    let mut moved = 0;
    let mut stuck = false;
    for ch in std::fs::read_dir(&chats).into_iter().flatten().flatten() {
        for e in std::fs::read_dir(ch.path().join("files")).into_iter().flatten().flatten() {
            let old = e.path();
            let Some(leaf) = old.file_name().and_then(|n| n.to_str()).map(str::to_owned) else {
                continue;
            };
            // A partial, a marker, a link already left behind: not files.
            if leaf.starts_with('.') || !e.file_type().is_ok_and(|t| t.is_file()) {
                continue;
            }
            match move_one(root, dir, &old, &leaf) {
                Ok(()) => moved += 1,
                Err(_) => stuck = true,
            }
        }
    }
    if !stuck {
        let _ = std::fs::create_dir_all(received_dir(root));
        let _ = std::fs::write(moved_mark(root), b"");
    }
    moved
}

fn move_one(root: &Path, dir: &Path, old: &Path, leaf: &str) -> Result<(), String> {
    let (digest, size) = crate::transfer::digest_file(old)?;
    // The leaf's first 8 are the digest's; anything else was not put
    // there by khor and keeps its name whole.
    let name = match leaf.split_once('-') {
        Some((d8, rest)) if !rest.is_empty() && digest.starts_with(d8) => rest,
        _ => leaf,
    };
    let at_ms = crate::transfer::mtime_ms(old).unwrap_or_else(now_ms);
    let dest = match receipt(root, &digest) {
        // The same bytes already landed: one file, and the old one goes.
        Some(r) => {
            std::fs::remove_file(old).map_err(|e| e.to_string())?;
            r.path
        }
        None => {
            let at = crate::drops::landing(dir, name)?;
            let dest = at.dest.clone();
            settle(old, &at)?;
            record(
                root,
                &Receipt {
                    digest,
                    name: name.to_owned(),
                    path: dest.clone(),
                    size,
                    from: String::new(),
                    at_ms,
                    mode: Mode::Offer,
                },
            )?;
            dest
        }
    };
    #[cfg(unix)]
    let _ = std::os::unix::fs::symlink(&dest, old);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file_ref(name: &str, bytes: &[u8]) -> FileRef {
        FileRef {
            name: name.into(),
            size: bytes.len() as i64,
            digest: blake3::hash(bytes).to_hex().to_string(),
        }
    }

    #[test]
    fn a_setting_is_absolute_or_home_led_and_never_climbs() {
        assert_eq!(tidy_setting("  ").unwrap(), "");
        assert_eq!(tidy_setting("~/inbox/").unwrap(), "~/inbox");
        assert_eq!(tidy_setting("~").unwrap(), "~");
        assert_eq!(tidy_setting("/data/in").unwrap(), "/data/in");
        for bad in ["inbox", "./x", "~x", "/a/../b"] {
            assert!(tidy_setting(bad).is_err(), "{bad:?} taken");
        }
    }

    /// A scratch store resolves under itself, never the person's home;
    /// an unusable setting falls back to the default and says why.
    #[test]
    fn the_receive_dir_resolves_under_a_scratch_store_and_falls_back() {
        let root = crate::testdir::TempTree::fresh("road-resolve");
        assert_eq!(resolve(&root, None, false), (root.join("khor-drop"), None));
        let (at, why) = resolve(&root, Some("~/in"), true);
        assert_eq!((at.clone(), why), (root.join("in"), None));
        assert!(at.is_dir(), "made when missing");
        std::fs::write(root.join("plain"), b"x").unwrap();
        let (at, why) = resolve(&root, Some(&root.join("plain").display().to_string()), true);
        assert_eq!(at, root.join("khor-drop"));
        assert!(why.is_some(), "a file is not a directory, and it is said");
        assert!(root.join("khor-drop").is_dir());
    }

    /// **A setting written elsewhere is taken on when it is seen** (the
    /// lead, 2026-10-07): a change to this machine's own register that no
    /// local call made — as a sync brings it — is resolved in full by
    /// `follow_receive_dir`: made, reported with the setting it answers,
    /// and a fallback reported with its reason; a read after that does not
    /// undo the fallback.
    #[test]
    fn a_setting_from_elsewhere_is_resolved_when_followed() {
        let root = crate::testdir::TempTree::fresh("road-follow");
        let n = crate::Node::open(root.to_path_buf()).unwrap();
        let me = n.device_str().to_owned();
        let row = |n: &crate::Node| n.devices_loaded().unwrap().doc.get(&me).unwrap();
        let write = |dir: &str| {
            let loaded = n.devices_loaded().unwrap();
            loaded.doc.set_receive_dir(&me, dir).unwrap();
            let mut store = loaded.store;
            store.flush(&loaded.doc).unwrap();
        };
        n.follow_receive_dir().unwrap();
        assert_eq!(row(&n).receive_for, None, "the default needs nothing");
        assert!(!root.join("khor-drop").exists(), "and makes nothing");

        write("~/inbox");
        n.follow_receive_dir().unwrap();
        let got = row(&n);
        assert_eq!(got.receive_at.as_deref(), Some(root.join("inbox").display().to_string().as_str()));
        assert_eq!((got.receive_for.as_deref(), got.receive_why), (Some("~/inbox"), None));
        assert!(root.join("inbox").is_dir(), "made at once, not at the next receive");

        std::fs::write(root.join("plain"), b"x").unwrap();
        let plain = root.join("plain").display().to_string();
        write(&plain);
        n.follow_receive_dir().unwrap();
        let got = row(&n);
        assert_eq!(got.receive_at.as_deref(), Some(root.join("khor-drop").display().to_string().as_str()));
        assert_eq!(got.receive_for.as_deref(), Some(plain.as_str()));
        assert!(got.receive_why.is_some(), "the fallback says why");
        let (at, why) = n.receive_place(false).unwrap();
        assert_eq!(at, root.join("khor-drop"), "a read does not undo the fallback");
        assert!(why.is_some());

        write("");
        n.follow_receive_dir().unwrap();
        let got = row(&n);
        assert_eq!((got.receive_for, got.receive_why), (None, None), "back to the default");
    }

    #[test]
    fn a_receipt_finds_its_file_only_while_it_is_there_whole() {
        let root = crate::testdir::TempTree::fresh("road-receipt");
        let f = file_ref("a.txt", b"hello");
        let path = root.join("a.txt");
        std::fs::write(&path, b"hello").unwrap();
        let r = Receipt {
            digest: f.digest.clone(),
            name: "a.txt".into(),
            path: path.clone(),
            size: 5,
            from: String::new(),
            at_ms: 1,
            mode: Mode::Offer,
        };
        record(&root, &r).unwrap();
        assert_eq!(landed(&root, &root.join("ch"), &f), Some(path.clone()));
        std::fs::write(&path, b"changed!").unwrap();
        assert_eq!(landed(&root, &root.join("ch"), &f), None, "another size is another file");
        std::fs::remove_file(&path).unwrap();
        assert_eq!(receipt(&root, &f.digest), None);
    }

    /// **A listing of the receive directory says where each received file
    /// came from and when it landed** (UX-48, the storage page's row); a
    /// file with no receipt, or changed since it landed, says neither.
    #[test]
    fn a_listing_names_each_received_files_sender_and_landing() {
        let root = crate::testdir::TempTree::fresh("road-arrivals");
        let dir = root.join("khor-drop");
        std::fs::create_dir_all(&dir).unwrap();
        for (name, body, from, at_ms) in [("a.txt", &b"hello"[..], "far", 7), ("b.txt", b"bye", "", 8)] {
            let path = dir.join(name);
            std::fs::write(&path, body).unwrap();
            let f = file_ref(name, body);
            record(&root, &Receipt { digest: f.digest, name: name.into(), path, size: body.len() as u64, from: from.into(), at_ms, mode: Mode::Offer }).unwrap();
        }
        std::fs::write(dir.join("mine.txt"), b"put here by hand").unwrap();
        let (at, mut rows, _) = crate::files::list_dir(dir.to_str().unwrap()).unwrap();
        mark_arrivals(&root, &at, &mut rows);
        let said = |n: &str| rows.iter().find(|e| e.name == n).map(|e| (e.from.clone(), e.received_ms)).unwrap();
        assert_eq!(said("a.txt"), (Some("far".into()), Some(7)));
        assert_eq!(said("b.txt"), (None, Some(8)), "a receipt that does not know the sender");
        assert_eq!(said("mine.txt"), (None, None), "no receipt");
        std::fs::write(dir.join("a.txt"), b"changed!").unwrap();
        let (at, mut rows, _) = crate::files::list_dir(dir.to_str().unwrap()).unwrap();
        mark_arrivals(&root, &at, &mut rows);
        assert_eq!(rows.iter().find(|e| e.name == "a.txt").map(|e| e.from.clone()), Some(None), "another size is another file");
    }

    /// **The move out of the conversation folders**: each file lands under
    /// its own name (a clash suffixed), the old path still opens it, a
    /// message still finds it by digest, and a second pass moves nothing.
    #[test]
    fn conversation_files_move_once_and_are_still_found() {
        let root = crate::testdir::TempTree::fresh("road-move");
        let dir = root.join("khor-drop");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("notes.md"), b"already here").unwrap();
        let ch = root.join(".khor").join("chat").join("box");
        std::fs::create_dir_all(ch.join("files")).unwrap();
        let f = file_ref("notes.md", b"# notes");
        let old = crate::transfer::payload_path(&ch, &f);
        std::fs::write(&old, b"# notes").unwrap();
        std::fs::write(ch.join("files").join(".partial-x"), b"p").unwrap();

        assert_eq!(move_chat_files(&root, &dir), 1);
        let new = dir.join("notes-2.md");
        assert_eq!(std::fs::read(&new).unwrap(), b"# notes");
        assert_eq!(landed(&root, &ch, &f), Some(new.clone()));
        #[cfg(unix)]
        assert_eq!(std::fs::read(&old).unwrap(), b"# notes", "the old path still opens it");
        assert!(ch.join("files").join(".partial-x").exists(), "a partial is not a file to move");
        assert_eq!(move_chat_files(&root, &dir), 0, "once");
    }

    /// A file that cannot move stays where it was and is still found,
    /// and the pass is made again later.
    #[cfg(unix)]
    #[test]
    fn a_file_that_cannot_move_stays_and_is_still_found() {
        use std::os::unix::fs::PermissionsExt;
        let root = crate::testdir::TempTree::fresh("road-stuck");
        let dir = root.join("khor-drop");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555)).unwrap();
        let ch = root.join(".khor").join("chat").join("box");
        std::fs::create_dir_all(ch.join("files")).unwrap();
        let f = file_ref("a.bin", b"abc");
        let old = crate::transfer::payload_path(&ch, &f);
        std::fs::write(&old, b"abc").unwrap();
        let moved = move_chat_files(&root, &dir);
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(moved, 0);
        assert_eq!(landed(&root, &ch, &f), Some(old.clone()), "still found where it was");
        assert!(!moved_mark(&root).exists(), "tried again next time");
        assert_eq!(move_chat_files(&root, &dir), 1);
    }
}
