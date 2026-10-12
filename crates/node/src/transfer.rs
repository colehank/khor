//! The transfer kind: bytes ride point-to-point, never the CRDT
//! (docs/NET.md 同步). A file message in a machine's window is the
//! summary — it travels the chat CRDT to everyone, and the receiver
//! pulls the payload as soon as it reads it, nobody asked (STORE-BRIEF
//! phase 2: 直接传); how that move stands rides the message itself
//! (`khor_sync::chat::Fate`). The sender keeps an [`Offer`] per file: the
//! path it serves the bytes from, and what each receiver fetched.
//!
//! Disk layout: a received payload lands in the receiving machine's
//! receive directory under its own name, found again by its digest
//! (`crate::road`'s receipts); its partial and the pid of a live pull sit
//! under `.khor/received/`. A khor before the receive directory landed it
//! in the channel's `files/<digest8>-<name>` ([`payload_path`]), which is
//! still read for a file that has not moved. `close` on a transfer row
//! clears only an unfinished pull; a landed file is the person's
//! (deleted from 存储 or a file tree, 2026-10-07).

use std::fs;
use std::path::{Path, PathBuf};

use khor_catalog::msg;
use khor_core::{SessionId, State};
use khor_sync::chat::{FileRef, Message, MsgBody, Step};

/// Where an offered payload is read from, recorded at send time.
/// Machine-local — the path means nothing on any other device. Keyed by
/// digest so `Fetch` is one lookup.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Offer {
    pub path: PathBuf,
    pub name: String,
    pub size: u64,
    /// The sender's only window into the far side: a fetch has begun /
    /// the final slice was served — by anyone. What an offer recorded by
    /// a khor before [`Offer::to`] says; read only when `to` is empty.
    pub started: bool,
    pub done: bool,
    /// **The same, per receiver**, by device id (STORE-BRIEF foundation
    /// A): one file sent to two machines is two deliveries. It was one
    /// pair of flags per digest, so the second machine's offer read
    /// 已送达 the moment the first had fetched it.
    #[serde(default)]
    pub to: std::collections::BTreeMap<String, Fetched>,
    /// Written by [`hold`]: bytes this machine landed and serves on, not
    /// a file it sent. Only such a record goes when its file is gone
    /// ([`forget_gone_held`]); a sender's own offer outlives its file
    /// being moved away and back.
    #[serde(default)]
    pub held: bool,
}

/// What one receiver did with an offer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Fetched {
    pub started: bool,
    pub done: bool,
}

impl Offer {
    /// A slice served to `receiver`; `last` when it was the final one.
    /// Answers whether anything changed (the record is written only
    /// then: slices are served several at once, and a middle slice
    /// rewriting what it loaded could undo the last slice's `done`).
    pub fn note_fetch(&mut self, receiver: &str, last: bool) -> bool {
        let was = (self.started, self.done, self.to.get(receiver).copied());
        self.started = true;
        self.done |= last;
        let f = self.to.entry(receiver.to_owned()).or_default();
        f.started = true;
        f.done |= last;
        was != (self.started, self.done, Some(*f))
    }

    /// Where `receiver` stands with this offer: its own record, or — for
    /// an offer recorded before receivers were told apart — the one pair
    /// of flags there was.
    pub fn fetched_by(&self, receiver: &str) -> Fetched {
        if self.to.is_empty() {
            return Fetched {
                started: self.started,
                done: self.done,
            };
        }
        self.to.get(receiver).copied().unwrap_or_default()
    }
}

pub fn offers_dir(root: &Path) -> PathBuf {
    root.join(".khor").join("transfers")
}

/// The digest lands in a filename; only lowercase blake3 hex survives.
pub fn offer_path(root: &Path, digest: &str) -> Result<PathBuf, String> {
    if digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(msg::not_a_digest(format_args!("{digest:?}")));
    }
    Ok(offers_dir(root).join(digest))
}

/// **Written whole or not at all**: to a temporary name, then renamed
/// over the record. Slices are served several at once (`link::PULL_AHEAD`),
/// each reading this record, and a plain write let one read it half
/// written — an empty file that would not parse, refusing a slice of a
/// healthy pull (seen once in five runs of `pull_road.rs`).
pub fn save_offer(root: &Path, digest: &str, offer: &Offer) -> Result<(), String> {
    let dir = offers_dir(root);
    fs::create_dir_all(&dir).map_err(msg::cant_make_transfers_dir)?;
    let path = offer_path(root, digest)?;
    // A name of its own per write: two pulls of one offer may record at
    // once, and a shared temporary name would let one rename the other's
    // half-written file into place.
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = path.with_extension(format!("{}-{n}.tmp", std::process::id()));
    fs::write(&tmp, serde_json::to_vec(offer).map_err(|e| e.to_string())?).map_err(msg::cant_record_offer)?;
    fs::rename(&tmp, &path).map_err(msg::cant_record_offer)
}

pub fn load_offer(root: &Path, digest: &str) -> Result<Option<Offer>, String> {
    let Ok(text) = fs::read_to_string(offer_path(root, digest)?) else {
        return Ok(None);
    };
    serde_json::from_str(&text)
        .map(Some)
        .map_err(msg::offer_garbled)
}

/// **A machine that landed a broadcast file holds it** (BCAST-BRIEF): the
/// landed bytes are offered on under the same digest, so a machine that
/// cannot reach the sender pulls them from here (`serve_slice`). A record
/// already serving its file whole is kept — this machine's own send of the
/// same bytes keeps what each receiver did with it; a stale one (its file
/// gone or no longer that size) is replaced. A holder whose landed file is
/// edited later answers `offered_file_changed`, and the puller moves on.
pub fn hold(root: &Path, f: &FileRef, landed: &Path) -> Result<(), String> {
    if let Ok(Some(o)) = load_offer(root, &f.digest)
        && fs::metadata(&o.path).is_ok_and(|m| m.len() == o.size)
    {
        return Ok(());
    }
    save_offer(
        root,
        &f.digest,
        &Offer {
            path: landed.to_path_buf(),
            name: f.name.clone(),
            size: f.size.max(0) as u64,
            started: false,
            done: false,
            to: Default::default(),
            held: true,
        },
    )
}

/// **A held record whose file is gone is forgotten** (BCAST-BRIEF): the
/// landed file was deleted or moved, so this machine holds nothing under
/// that digest any more. Called for one digest on a slice that missed it
/// (`serve_slice`), and for every record when the serve starts, so
/// records do not pile up for files nobody asks about. Answers how many
/// went.
pub fn forget_gone_held(root: &Path, digest: Option<&str>) -> usize {
    let digests: Vec<String> = match digest {
        Some(d) => vec![d.to_owned()],
        None => fs::read_dir(offers_dir(root))
            .map(|rd| rd.flatten().filter_map(|e| e.file_name().into_string().ok()).collect())
            .unwrap_or_default(),
    };
    let mut gone = 0;
    for d in digests {
        let Ok(Some(o)) = load_offer(root, &d) else { continue };
        if o.held && !o.path.exists() && offer_path(root, &d).is_ok_and(|p| fs::remove_file(p).is_ok()) {
            gone += 1;
        }
    }
    gone
}

/// Streaming blake3 over the file. Returns (hex digest, size).
pub fn digest_file(path: &Path) -> Result<(String, u64), String> {
    use std::io::Read;
    let mut f = fs::File::open(path).map_err(|e| msg::cant_read(path.display(), e))?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = vec![0u8; 1 << 20];
    let mut size = 0u64;
    loop {
        let n = f
            .read(&mut buf)
            .map_err(|e| msg::cant_finish_reading(path.display(), e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        size += n as u64;
    }
    Ok((hasher.finalize().to_hex().to_string(), size))
}

// ── payload locations inside a channel dir ──────────────────

/// Where a khor before the receive directory landed an accepted file:
/// the channel's `files/<digest8>-<name>`. Read only to find a file that
/// has not moved (`crate::road::landed`, `crate::road::move_chat_files`).
pub fn payload_path(channel_dir: &Path, f: &FileRef) -> PathBuf {
    channel_dir.join("files").join(landed_name(&f.digest, &f.name))
}

/// A leaf in the old conversation folder: the first 8 of its digest,
/// then its name made safe.
pub fn landed_name(digest: &str, name: &str) -> String {
    format!("{}-{}", digest.chars().take(8).collect::<String>(), safe_name(name))
}

pub fn partial_path(channel_dir: &Path, f: &FileRef) -> PathBuf {
    channel_dir
        .join("files")
        .join(format!(".partial-{}-{}", digest8(f), safe_name(&f.name)))
}

pub fn pulling_marker(channel_dir: &Path, f: &FileRef) -> PathBuf {
    channel_dir
        .join("files")
        .join(format!(".pulling-{}", digest8(f)))
}

fn digest8(f: &FileRef) -> String {
    f.digest.chars().take(8).collect()
}

/// The sender's file name lands in a receiver-side path: same whitelist
/// idea as channel names, one implementation would be nicer but the
/// alphabet differs (spaces and CJK are fine in a leaf name; separators
/// and dot-prefixes are not).
pub fn safe_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .filter(|c| !matches!(c, '/' | '\\' | '\0'))
        .collect();
    let cleaned = cleaned.trim_start_matches('.').to_string();
    if cleaned.is_empty() {
        "file".into()
    } else {
        cleaned
    }
}

// ── the row ─────────────────────────────────────────────────

pub struct TransferKind {
    root: PathBuf,
    device_str: String,
    name: String,
}

impl TransferKind {
    pub fn new(root: PathBuf, device_str: String, name: String) -> TransferKind {
        TransferKind {
            root,
            device_str,
            name,
        }
    }

    pub fn session_id(msg_id: &str) -> SessionId {
        SessionId(format!("transfer/{msg_id}"))
    }

    /// One file message's face on this machine — the same seat and the
    /// same words as [`Self::urgency`], read per message instead of
    /// folded onto the row, so a face painting the conversation can
    /// stamp each file chip without re-deriving the state machine
    /// (docs/UX.md 状态呈现: the word comes from here or not at all).
    ///
    /// Beyond the pressure `urgency` reports, the receiver's settled
    /// states come through too (完成/空闲 once the file landed): on a
    /// chip "no word" must mean "not my seat", never "done".
    /// `None`: a text message, a retracted one, or any message this
    /// machine did not receive — every device shows the move itself
    /// from the message's own record (`khor_sync::chat::Fate`); the
    /// state word is the receiver's alone.
    ///
    /// `watermark` is the conversation's own seen mark
    /// (`khor_sync::seen`): 完成 exactly when the row counts that message
    /// unread, 空闲 exactly when it does not — the message's own stamp
    /// against the watermark, both on the senders' clocks.
    pub fn message_face(&self, channel: &str, m: &Message, watermark: i64) -> Option<State> {
        let MsgBody::Files(files) = &m.body else {
            return None;
        };
        if m.retracted || files.is_empty() || channel != self.name || m.from.id == self.device_str {
            return None;
        }
        let settled = if m.at > watermark { State::Done } else { State::Idle };
        Some(match m.fate.as_ref().map(|f| f.step) {
            // Not started yet: the receiver starts it by itself, nobody
            // asked (STORE-BRIEF phase 2) — so it is on its way.
            None | Some(Step::Moving) => State::Busy,
            // 断了可续: a retry carries on from the bytes kept.
            Some(Step::Failed) => State::Errored,
            Some(Step::Landed) => settled,
            // Cancelled on purpose: nothing is owed and nothing is new.
            Some(Step::Stopped) => State::Idle,
        })
    }

    /// The live word of one channel's **latest message**, folded onto
    /// that machine's chat row (传输并入对话批, 2026-08-23: a transfer is
    /// a file message in the conversation, not a row of its own — the
    /// pressure lands on the row that carries the message).
    ///
    /// **Only the newest message still standing speaks** (STORE-BRIEF
    /// phase 5, the row's word): an older file's fate stays on its own
    /// chip and presses nothing on the row, the way a terminal wears its
    /// live process's word. ⚠️ 墓碑: this folded every file message in
    /// the history and kept the most pressing word, so a pull that broke
    /// weeks ago said 中断 on the row for ever.
    ///
    /// What counts as pressure, read off that message's record
    /// (`khor_sync::chat::Fate`), by seat:
    /// - **receiver** (this machine was sent the file): 忙碌 while it
    ///   moves (or has not started), 中断 when it broke and a retry
    ///   carries on.
    /// - **sender**: 忙碌 while it moves.
    /// - nothing is 待批 any more (STORE-BRIEF phase 2: 直接传，不用等
    ///   接收). ⚠️ 墓碑: the receiver's 待批 while a file waited on its
    ///   收下.
    /// - landed or cancelled files press nothing: 完成/未看 is already
    ///   the chat row's own message-based reading.
    ///
    /// `None` when the latest message presses nothing; the caller keeps
    /// the row's own word. Stamped with the message.
    pub fn urgency(&self, channel: &str, msgs: &[Message]) -> Option<(State, i64)> {
        // Still standing, as the row's preview reads it: a retracted
        // slot has no word of its own.
        let m = msgs.iter().rev().find(|m| !m.retracted)?;
        let MsgBody::Files(files) = &m.body else {
            return None;
        };
        if files.is_empty() {
            return None;
        }
        let i_sent = m.from.id == self.device_str;
        let i_receive = channel == self.name && !i_sent;
        let pressed = match m.fate.as_ref().map(|f| f.step) {
            None | Some(Step::Moving) if i_receive => Some(State::Busy),
            Some(Step::Moving) if i_sent => Some(State::Busy),
            Some(Step::Failed) if i_receive => Some(State::Errored),
            _ => None,
        };
        pressed.map(|state| (state, m.at))
    }

    /// Clears what an unfinished pull of this transfer left (the partial
    /// and the pid marker). **The landed file stays, with its receipt**
    /// (the lead, 2026-10-07): it sits in the visible receive directory
    /// and belongs to the person, who deletes it from 存储 or a file tree
    /// — closing a row is not deleting a file. ⚠️ 墓碑 (STORE-BRIEF phase
    /// 1, store-a): close deleted the landed file too, the habit of the
    /// days it lay hidden under the conversation's folder.
    pub fn close_payload(&self, channel_dir: &Path, f: &FileRef) -> Result<(), String> {
        for p in [
            Some(partial_path(channel_dir, f)),
            Some(pulling_marker(channel_dir, f)),
            crate::road::partial_of(&self.root, &f.digest),
            crate::road::marker_of(&self.root, &f.digest),
        ]
        .into_iter()
        .flatten()
        {
            if p.symlink_metadata().is_ok() {
                fs::remove_file(&p).map_err(msg::cant_delete)?;
            }
        }
        Ok(())
    }
}

pub fn mtime_ms(p: &Path) -> Option<i64> {
    let t = fs::metadata(p).ok()?.modified().ok()?;
    let d = t.duration_since(std::time::UNIX_EPOCH).ok()?;
    Some(d.as_millis() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use khor_sync::chat::Sender;

    fn file_ref(name: &str) -> FileRef {
        FileRef {
            name: name.into(),
            size: 5,
            digest: "ab".repeat(32),
        }
    }

    fn msg(id: &str, from: &str, f: FileRef) -> Message {
        Message {
            id: id.into(),
            at: 1000,
            from: Sender {
                id: from.into(),
                name: "x".into(),
            },
            body: MsgBody::Files(vec![f]),
            retracted: false,
            edited: false,
            into: String::new(),
            fate: None,
            fates: Default::default(),
        }
    }

    fn fated(mut m: Message, step: Step) -> Message {
        m.fate = Some(khor_sync::chat::Fate {
            step,
            moved: 0,
            landed: Vec::new(),
            why: String::new(),
            via: String::new(),
        });
        m
    }

    fn kind(root: &Path) -> TransferKind {
        TransferKind::new(root.to_path_buf(), "me".repeat(32), "mybox".into())
    }

    /// A scratch tree that **takes itself away however the test leaves**
    /// (`crates/node/src/tests.rs::Scratch` has the whole reasoning: a
    /// cleanup written at the end of a test is skipped by the assertion
    /// that fails, so a red run leaks a directory every time it is red).
    /// A copy per file, which is this repo's convention for test helpers —
    /// it costs no dev-dependency.
    struct Scratch(std::path::PathBuf);

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    impl std::ops::Deref for Scratch {
        type Target = std::path::Path;

        fn deref(&self) -> &std::path::Path {
            &self.0
        }
    }

    fn tmp(tag: &str) -> Scratch {
        let p = std::env::temp_dir().join(format!("khor-transfer-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        Scratch(p)
    }

    /// **The receiver's pressure, walked over the message's record**
    /// (STORE-BRIEF phase 2): not started and moving = 忙碌, broken =
    /// 中断, landed or cancelled press nothing — and nothing is ever
    /// 待批: a conversation's file waits on nobody.
    #[test]
    fn a_receivers_pressure_walks_busy_errored_then_settles_and_never_blocks() {
        let root = tmp("faces");
        let k = kind(&root);
        let m = msg("m1", "far", file_ref("report.pdf"));
        let word = |m: Message| k.urgency("mybox", &[m]).map(|(w, _)| w);
        assert_eq!(word(m.clone()), Some(State::Busy), "not started = on its way, not waiting on anyone");
        assert_eq!(word(fated(m.clone(), Step::Moving)), Some(State::Busy));
        assert_eq!(word(fated(m.clone(), Step::Failed)), Some(State::Errored), "a retry carries on");
        assert_eq!(word(fated(m.clone(), Step::Landed)), None, "the message itself is the news");
        assert_eq!(word(fated(m.clone(), Step::Stopped)), None);
        let face = |m: &Message, wm| k.message_face("mybox", m, wm);
        assert_eq!(face(&m, 0), Some(State::Busy));
        assert_eq!(face(&fated(m.clone(), Step::Landed), 0), Some(State::Done));
        assert_eq!(face(&fated(m.clone(), Step::Landed), 2000), Some(State::Idle), "looked at");
        assert_eq!(k.message_face("otherbox", &m, 0), None, "not this machine's file");
    }

    /// **Only the latest message speaks** (STORE-BRIEF phase 5, the
    /// row's word): a broken file followed by a line leaves the row to
    /// the line; a broken file after a moving one says 中断, not 忙碌; a
    /// retracted newest slot hands the word back to the one before it.
    #[test]
    fn only_the_latest_message_presses_on_the_row() {
        let root = tmp("latest");
        let k = kind(&root);
        let mut text = msg("m2", "far", file_ref("x"));
        text.body = MsgBody::Text("later".into());
        text.at = 2000;
        let broken = fated(msg("m1", "far", file_ref("old.pdf")), Step::Failed);
        assert_eq!(k.urgency("mybox", &[broken.clone()]).map(|(w, _)| w), Some(State::Errored), "control");
        assert_eq!(k.urgency("mybox", &[broken.clone(), text.clone()]), None, "a later line: the old break stays on its chip");
        let mut newer = fated(msg("m3", "far", file_ref("new.pdf")), Step::Failed);
        newer.at = 3000;
        let moving = fated(msg("m1", "far", file_ref("old.pdf")), Step::Moving);
        assert_eq!(k.urgency("mybox", &[moving, newer.clone()]), Some((State::Errored, 3000)), "the newest one's word, not the most pressing");
        text.retracted = true;
        assert_eq!(k.urgency("mybox", &[broken, text]).map(|(w, _)| w), Some(State::Errored), "a retracted slot has no word");
    }

    /// The sender's seat presses only while the bytes move; a third
    /// party presses nothing.
    #[test]
    fn the_senders_seat_presses_only_while_it_moves() {
        let root = tmp("sender");
        let k = kind(&root);
        let mine = msg("m1", &"me".repeat(32), file_ref("data.bin"));
        let word = |m: Message| k.urgency("otherbox", &[m]).map(|(w, _)| w);
        assert_eq!(word(mine.clone()), None, "not started is the receiver's to start");
        assert_eq!(word(fated(mine.clone(), Step::Moving)), Some(State::Busy));
        assert_eq!(word(fated(mine.clone(), Step::Failed)), None);
        assert_eq!(word(fated(mine.clone(), Step::Landed)), None);
        assert_eq!(word(fated(msg("m2", "someone-else", file_ref("x")), Step::Moving)), None, "neither end");
    }

    /// **One file sent to two machines is two deliveries** (STORE-BRIEF
    /// foundation A): the sender's record of what it served is kept per
    /// receiver — what phase 3's broadcast counts.
    #[test]
    fn one_file_sent_to_two_receivers_is_two_deliveries() {
        let mut offer = Offer {
            path: "/tmp/x".into(),
            name: "data.bin".into(),
            size: 5,
            started: false,
            done: false,
            to: Default::default(),
            held: false,
        };
        assert!(offer.note_fetch("alpha", false));
        assert!(!offer.note_fetch("alpha", false), "nothing new, nothing written");
        assert!(offer.note_fetch("alpha", true));
        assert_eq!(offer.fetched_by("alpha"), Fetched { started: true, done: true });
        assert_eq!(offer.fetched_by("beta"), Fetched::default(), "beta has fetched nothing");
        // An offer recorded before receivers were told apart reads as one.
        let old: Offer = serde_json::from_str(r#"{"path":"/x","name":"a","size":5,"started":true,"done":true}"#).unwrap();
        assert_eq!(old.fetched_by("anyone"), Fetched { started: true, done: true });
    }

    /// **A landed broadcast file is held** (BCAST-BRIEF): offered on from
    /// where it landed; a record already serving its file is kept, a
    /// stale one is replaced.
    #[test]
    fn a_landed_file_is_held_and_a_live_record_is_kept() {
        let root = tmp("hold");
        let f = file_ref("data.bin");
        let landed = root.join("data.bin");
        fs::write(&landed, b"hello").unwrap();
        hold(&root, &f, &landed).unwrap();
        let held = load_offer(&root, &f.digest).unwrap().unwrap();
        assert_eq!((held.path.as_path(), held.size, held.to.len()), (landed.as_path(), 5, 0));

        // This machine's own send of the same bytes stays as it was.
        let own = root.join("own.bin");
        fs::write(&own, b"hello").unwrap();
        let mut sent = held.clone();
        sent.path = own.clone();
        sent.note_fetch("beta", true);
        save_offer(&root, &f.digest, &sent).unwrap();
        hold(&root, &f, &landed).unwrap();
        let kept = load_offer(&root, &f.digest).unwrap().unwrap();
        assert_eq!((kept.path, kept.to.len()), (own.clone(), 1), "a live record is kept");

        // Its file edited: the record no longer serves, the landing does.
        fs::write(&own, b"hello, edited").unwrap();
        hold(&root, &f, &landed).unwrap();
        let held = load_offer(&root, &f.digest).unwrap().unwrap();
        assert_eq!((held.path.as_path(), held.held), (landed.as_path(), true));
    }

    /// A held record goes once its file is gone; a sender's own offer
    /// stays, its file only moved away for now.
    #[test]
    fn a_held_record_goes_with_its_file_and_an_own_offer_stays() {
        let root = tmp("forget");
        let held = file_ref("held.bin");
        let landed = root.join("held.bin");
        fs::write(&landed, b"hello").unwrap();
        hold(&root, &held, &landed).unwrap();
        let own = FileRef { digest: "cd".repeat(32), ..file_ref("own.bin") };
        let offer = Offer { path: root.join("away.bin"), name: own.name.clone(), size: 5, started: false, done: false, to: Default::default(), held: false };
        save_offer(&root, &own.digest, &offer).unwrap();
        assert_eq!(forget_gone_held(&root, None), 0, "the held file is there; the own offer is not a held one");
        fs::remove_file(&landed).unwrap();
        assert_eq!(forget_gone_held(&root, Some(&held.digest)), 1);
        assert!(load_offer(&root, &held.digest).unwrap().is_none());
        assert!(load_offer(&root, &own.digest).unwrap().is_some(), "the sender's own offer stays");
    }

    /// A malicious file name must not escape the files/ directory.
    #[test]
    fn a_hostile_file_name_cannot_escape_the_payload_dir() {
        let ch = PathBuf::from("/home/x/.khor/chat/box");
        for bad in ["../../etc/passwd", "/etc/passwd", ".hidden", "a/b"] {
            let f = FileRef {
                name: bad.into(),
                size: 1,
                digest: "cd".repeat(32),
            };
            let p = payload_path(&ch, &f);
            assert!(
                p.parent().unwrap().ends_with("files"),
                "{bad:?} escaped: {}",
                p.display()
            );
            assert!(
                !p.file_name().unwrap().to_string_lossy().starts_with('.'),
                "{bad:?} produced a dotfile: {}",
                p.display()
            );
        }
    }
}
