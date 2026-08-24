//! The transfer kind: bytes ride point-to-point, never the CRDT
//! (docs/NET.md 同步). A file message in a machine's window is the
//! summary — it travels the chat CRDT to everyone; this kind turns it
//! into a session row and moves the payload only after the user's
//! approval (docs/SESSION.md 传输: 摘要到了,等你许可收全量).
//!
//! Disk layout, all under the channel that carries the summary:
//! `files/<digest8>-<name>` (verified payload), `files/.partial-…`
//! (resumable), `files/.pulling-<digest8>` (pid of a live pull). `close`
//! on the chat row deletes `files/` wholesale; `close` on a transfer row
//! deletes just that transfer's payload.

use std::fs;
use std::path::{Path, PathBuf};

use khor_catalog::msg;
use khor_core::{SessionId, State};
use khor_sync::chat::{FileRef, Message, MsgBody};

/// Where an offered payload is read from, recorded at send time.
/// Machine-local — the path means nothing on any other device. Keyed by
/// digest so `Fetch` is one lookup.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Offer {
    pub path: PathBuf,
    pub name: String,
    pub size: u64,
    /// The sender's only window into the far side: a fetch has begun /
    /// the final slice was served.
    pub started: bool,
    pub done: bool,
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

pub fn save_offer(root: &Path, digest: &str, offer: &Offer) -> Result<(), String> {
    let dir = offers_dir(root);
    fs::create_dir_all(&dir).map_err(msg::cant_make_transfers_dir)?;
    fs::write(
        offer_path(root, digest)?,
        serde_json::to_vec(offer).map_err(|e| e.to_string())?,
    )
    .map_err(msg::cant_record_offer)
}

pub fn load_offer(root: &Path, digest: &str) -> Result<Option<Offer>, String> {
    let Ok(text) = fs::read_to_string(offer_path(root, digest)?) else {
        return Ok(None);
    };
    serde_json::from_str(&text)
        .map(Some)
        .map_err(msg::offer_garbled)
}

/// Streaming blake3 over the file. Returns (hex digest, size).
pub fn digest_file(path: &Path) -> Result<(String, u64), String> {
    use std::io::Read;
    let mut f = fs::File::open(path).map_err(|e| msg::cant_read(path.display(), e))?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = vec![0u8; 1 << 20];
    let mut size = 0u64;
    loop {
        let n = f.read(&mut buf).map_err(|e| msg::cant_finish_reading(path.display(), e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        size += n as u64;
    }
    Ok((hasher.finalize().to_hex().to_string(), size))
}

// ── payload locations inside a channel dir ──────────────────

pub fn payload_path(channel_dir: &Path, f: &FileRef) -> PathBuf {
    channel_dir.join("files").join(format!("{}-{}", digest8(f), safe_name(&f.name)))
}

pub fn partial_path(channel_dir: &Path, f: &FileRef) -> PathBuf {
    channel_dir.join("files").join(format!(".partial-{}-{}", digest8(f), safe_name(&f.name)))
}

pub fn pulling_marker(channel_dir: &Path, f: &FileRef) -> PathBuf {
    channel_dir.join("files").join(format!(".pulling-{}", digest8(f)))
}

fn digest8(f: &FileRef) -> String {
    f.digest.chars().take(8).collect()
}

/// The sender's file name lands in a receiver-side path: same whitelist
/// idea as channel names, one implementation would be nicer but the
/// alphabet differs (spaces and CJK are fine in a leaf name; separators
/// and dot-prefixes are not).
fn safe_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .filter(|c| !matches!(c, '/' | '\\' | '\0'))
        .collect();
    let cleaned = cleaned.trim_start_matches('.').to_string();
    if cleaned.is_empty() { "file".into() } else { cleaned }
}

// ── the row ─────────────────────────────────────────────────

/// The sender's neutral account of one offer (发端中性词批): what has
/// happened to it, never what anyone must do. Keys cross to the faces
/// (`key`), words are looked up at the last moment (zh.toml's rule).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Delivery {
    /// Nobody has taken the offer up yet.
    Waiting,
    /// The far side is pulling bytes right now.
    Fetching,
    /// The whole payload was served.
    Served,
}

impl Delivery {
    pub fn key(&self) -> &'static str {
        match self {
            Delivery::Waiting => "waiting",
            Delivery::Fetching => "fetching",
            Delivery::Served => "served",
        }
    }
}

pub struct TransferKind {
    root: PathBuf,
    device_str: String,
    name: String,
}

impl TransferKind {
    pub fn new(root: PathBuf, device_str: String, name: String) -> TransferKind {
        TransferKind { root, device_str, name }
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
    /// states come through too (完成/空闲 once the payload landed):
    /// on a chip "no word" must mean "not my seat", never "done" —
    /// those are different answers to a person deciding whether to act.
    /// `None`: a text message, a retracted one, or **any sender-seat
    /// message** — the sender's chip speaks [`Delivery`], a neutral
    /// fact, not a state word (发端中性词批; it used to say 忙碌 while
    /// the fetch ran, which `Delivery::Fetching` replaces).
    pub fn message_face(&self, channel: &str, channel_dir: &Path, m: &Message) -> Option<State> {
        let MsgBody::Files(files) = &m.body else { return None };
        if m.retracted || files.is_empty() {
            return None;
        }
        let i_sent = m.from.id == self.device_str;
        let i_receive = channel == self.name && !i_sent;
        if !i_receive {
            return None;
        }
        let (state, _, _) = self.receiver_face(channel_dir, &files[0], m.at, 0);
        Some(state)
    }

    /// Where the handover stands, said from the sender's seat as a
    /// neutral fact (发端中性词批): the offer has not been taken up,
    /// is being fetched, or was served whole. **Not a state word** — it
    /// never joins the six, never ranks, never presses the row: 待批
    /// stays the receiver's alone (`urgency`'s iron rule, untouched;
    /// the row-level 忙碌 while bytes flow also stays `urgency`'s).
    /// `None` for every seat but the sender's.
    pub fn message_delivery(&self, m: &Message) -> Option<Delivery> {
        let MsgBody::Files(files) = &m.body else { return None };
        if m.retracted || files.is_empty() || m.from.id != self.device_str {
            return None;
        }
        Some(match load_offer(&self.root, &files[0].digest).ok().flatten() {
            Some(o) if o.done => Delivery::Served,
            Some(o) if o.started => Delivery::Fetching,
            _ => Delivery::Waiting,
        })
    }

    /// The most pressing live word among one channel's file messages,
    /// folded onto that machine's chat row (传输并入对话批, 2026-08-23:
    /// a transfer is a file message in the conversation, not a row of
    /// its own — the pressure lands on the row that carries the
    /// message).
    ///
    /// What counts as pressure, by seat:
    /// - **receiver** (this machine was sent the file): 待批 while the
    ///   payload waits on this machine's approval, 忙碌 while a pull
    ///   runs, 中断 on a resumable break. 待批 comes from this seat and
    ///   no other — the approval asked for is *this* machine's, which
    ///   is the word's whole definition (docs/SESSION.md 六词).
    /// - **sender**: 忙碌 while the far side is fetching. The wait for
    ///   the far side's approval is *their* 待批, never ours — a badge
    ///   this machine cannot clear by answering would break 角标可归零.
    ///   (The retired transfer row said 待批 from either seat; on its
    ///   own row that was a fact about the transfer, on the machine's
    ///   row it would be a debt the wrong person is billed for.)
    /// - settled transfers (payload landed, or the offer served whole)
    ///   press nothing: 完成/未看 is already the chat row's own
    ///   message-based reading — a file message lands like any message.
    ///
    /// `None` when nothing is in flight; the caller keeps the row's own
    /// word. Ties go to the most pressing rank, stamped with the moment
    /// that made it true.
    pub fn urgency(
        &self,
        channel: &str,
        channel_dir: &Path,
        msgs: &[Message],
    ) -> Option<(State, i64)> {
        let mut best: Option<(State, i64)> = None;
        for m in msgs {
            let MsgBody::Files(files) = &m.body else { continue };
            if m.retracted || files.is_empty() {
                continue;
            }
            let i_sent = m.from.id == self.device_str;
            let i_receive = channel == self.name && !i_sent;
            let f = &files[0];
            let pressed = if i_receive {
                let (state, at, _) = self.receiver_face(channel_dir, f, m.at, 0);
                match state {
                    State::Blocked | State::Busy | State::Errored => Some((state, at)),
                    _ => None,
                }
            } else if i_sent {
                match load_offer(&self.root, &f.digest).ok().flatten() {
                    Some(o) if o.started && !o.done => Some((State::Busy, m.at)),
                    _ => None,
                }
            } else {
                None
            };
            if let Some((state, at)) = pressed {
                if best.is_none_or(|(b, _)| state.rank() < b.rank()) {
                    best = Some((state, at));
                }
            }
        }
        best
    }

    /// The receiver's view rides the payload on disk.
    fn receiver_face(
        &self,
        channel_dir: &Path,
        f: &FileRef,
        msg_at: i64,
        watermark: i64,
    ) -> (State, i64, u64) {
        let done = payload_path(channel_dir, f);
        if done.exists() {
            let at = mtime_ms(&done).unwrap_or(msg_at);
            let unread = u64::from(at > watermark);
            let state = if unread > 0 { State::Done } else { State::Idle };
            return (state, at, unread);
        }
        let partial = partial_path(channel_dir, f);
        if partial.exists() {
            let at = mtime_ms(&partial).unwrap_or(msg_at);
            if marker_pid_alive(&pulling_marker(channel_dir, f)) {
                return (State::Busy, at, 0);
            }
            // 断了可续 — the partial is the resume point, not garbage.
            return (State::Errored, at, 0);
        }
        (State::Blocked, msg_at, 0)
    }

    /// Deletes this transfer's received payload (verified and partial
    /// both). The summary stays in the CRDT — the row falls back to 待批
    /// and the payload can be pulled again.
    pub fn close_payload(&self, channel_dir: &Path, f: &FileRef) -> Result<(), String> {
        for p in [
            payload_path(channel_dir, f),
            partial_path(channel_dir, f),
            pulling_marker(channel_dir, f),
        ] {
            if p.exists() {
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

fn marker_pid_alive(marker: &Path) -> bool {
    let Ok(text) = fs::read_to_string(marker) else {
        return false;
    };
    let Ok(pid) = text.trim().parse::<u32>() else {
        return false;
    };
    crate::link::pid_alive(pid)
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
            from: Sender { id: from.into(), name: "x".into() },
            body: MsgBody::Files(vec![f]),
            retracted: false,
            edited: false,
        }
    }

    fn kind(root: &Path) -> TransferKind {
        TransferKind::new(root.to_path_buf(), "me".repeat(32), "mybox".into())
    }

    fn tmp(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("khor-transfer-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    /// The receiver's pressure, walked over disk states (传输并入对话批
    /// reshaped the retired row-walk test — same disk states, the word
    /// now lands on the chat row): nothing = 待批, live pull = 忙碌,
    /// partial without a live pull = 中断, and a landed payload presses
    /// **nothing** — 完成/未看 is the chat row's own message-based
    /// reading, so a settled transfer must not keep shouting.
    #[test]
    fn a_receivers_pressure_walks_blocked_busy_errored_then_settles() {
        let root = tmp("faces");
        let ch = root.join("ch");
        fs::create_dir_all(ch.join("files")).unwrap();
        let k = kind(&root);
        let f = file_ref("report.pdf");
        let m = [msg("m1", "far", f.clone())];

        let word = |k: &TransferKind| k.urgency("mybox", &ch, &m).map(|(w, _)| w);
        assert_eq!(word(&k), Some(State::Blocked), "no payload = waiting for THIS machine");

        fs::write(partial_path(&ch, &f), b"he").unwrap();
        fs::write(pulling_marker(&ch, &f), format!("{}", std::process::id())).unwrap();
        assert_eq!(word(&k), Some(State::Busy), "a live pull (this pid) = busy");

        fs::remove_file(pulling_marker(&ch, &f)).unwrap();
        assert_eq!(word(&k), Some(State::Errored), "a partial with no live pull = resumable");

        fs::remove_file(partial_path(&ch, &f)).unwrap();
        fs::write(payload_path(&ch, &f), b"hello").unwrap();
        assert_eq!(word(&k), None, "a landed payload presses nothing; the message itself is the news");
        let _ = fs::remove_dir_all(&root);
    }

    /// **待批 comes from the seat whose approval is being waited on,
    /// and no other.** The sender's offer with no fetch yet presses
    /// nothing — the wait is the far side's 待批 (the retired row said
    /// 待批 from either seat; folded onto the machine's row that would
    /// bill the debt to the wrong person). Fetching presses 忙碌; a
    /// served offer presses nothing; a third party presses nothing.
    #[test]
    fn the_senders_seat_never_presses_blocked() {
        let root = tmp("sender");
        let ch = root.join("ch");
        fs::create_dir_all(&ch).unwrap();
        let k = kind(&root);
        let f = file_ref("data.bin");
        let mine = [msg("m1", &"me".repeat(32), f.clone())];
        let word = |k: &TransferKind, m: &[Message]| k.urgency("otherbox", &ch, m).map(|(w, _)| w);

        assert_eq!(
            word(&k, &mine),
            None,
            "an unanswered offer is the far side's 待批, never the sender's"
        );

        save_offer(&root, &f.digest, &Offer {
            path: "/tmp/x".into(), name: f.name.clone(), size: 5,
            started: true, done: false,
        }).unwrap();
        assert_eq!(word(&k, &mine), Some(State::Busy), "bytes flowing = busy on the sender too");

        save_offer(&root, &f.digest, &Offer {
            path: "/tmp/x".into(), name: f.name.clone(), size: 5,
            started: true, done: true,
        }).unwrap();
        assert_eq!(word(&k, &mine), None, "a served offer presses nothing");

        let theirs = [msg("m2", "someone-else", f.clone())];
        assert_eq!(word(&k, &theirs), None, "neither sender nor recipient = no pressure");
        let _ = fs::remove_dir_all(&root);
    }

    /// A malicious file name must not escape the files/ directory.
    #[test]
    fn a_hostile_file_name_cannot_escape_the_payload_dir() {
        let ch = PathBuf::from("/home/x/.khor/chat/box");
        for bad in ["../../etc/passwd", "/etc/passwd", ".hidden", "a/b"] {
            let f = FileRef { name: bad.into(), size: 1, digest: "cd".repeat(32) };
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
