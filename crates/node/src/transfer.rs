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
use khor_core::{kind, DeviceId, Kind, Millis, Session, SessionId, State, StateStamp};
use khor_sync::chat::{FileRef, Message, MsgBody};

use crate::device_id_from_hex;

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

    /// Rows for one channel's file messages. Only the sender and the
    /// recipient machine derive a row (a third device sees the summary in
    /// the chat; its transfer row waits on remote state — 挂账).
    ///
    /// `watermark` is the seen watermark for each transfer session id.
    pub fn rows(
        &self,
        channel: &str,
        channel_dir: &Path,
        msgs: &[Message],
        watermark: impl Fn(&str) -> i64,
    ) -> Vec<Session> {
        let mut out = Vec::new();
        for m in msgs {
            let MsgBody::Files(files) = &m.body else { continue };
            if m.retracted || files.is_empty() {
                continue;
            }
            let i_sent = m.from.id == self.device_str;
            let i_receive = channel == self.name && !i_sent;
            if !i_sent && !i_receive {
                continue;
            }
            let id = Self::session_id(&m.id);
            let f = &files[0];
            let (state, at, unread) = if i_sent {
                self.sender_face(f, m.at)
            } else {
                self.receiver_face(channel_dir, f, m.at, watermark(&id.0))
            };
            out.push(Session {
                id,
                kind: Kind(kind::TRANSFER.to_owned()),
                title: f.name.clone(),
                home: device_id_from_hex(&m.from.id).unwrap_or(DeviceId([0; 32])),
                state: StateStamp { state, at: Millis(at.max(0) as u64) },
                unread,
                // Same as a device chat: khor's own doing, no vendor.
                category: Some(khor_core::category::KHOR.to_owned()),
                last: None,
                via: None,
            term: false,
            });
        }
        out
    }

    /// The sender's view rides the offer record: no fetch yet = the far
    /// user's approval is the only cure = 待批, from either seat.
    fn sender_face(&self, f: &FileRef, msg_at: i64) -> (State, i64, u64) {
        match load_offer(&self.root, &f.digest).ok().flatten() {
            Some(o) if o.done => (State::Done, msg_at, 0),
            Some(o) if o.started => (State::Busy, msg_at, 0),
            _ => (State::Blocked, msg_at, 0),
        }
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

    /// The six-word mapping for a receiver, walked over disk states:
    /// nothing = 待批, partial without a live pull = 中断, payload = 完成
    /// while unseen, 空闲 once the watermark covers it.
    #[test]
    fn a_receivers_row_walks_blocked_errored_done_idle() {
        let root = tmp("faces");
        let ch = root.join("ch");
        fs::create_dir_all(ch.join("files")).unwrap();
        let k = kind(&root);
        let f = file_ref("report.pdf");
        let m = [msg("m1", "far", f.clone())];

        let row = &k.rows("mybox", &ch, &m, |_| 0)[0];
        assert_eq!(row.state.state, State::Blocked, "no payload = waiting for approval");
        assert_eq!(row.unread, 0);

        fs::write(partial_path(&ch, &f), b"he").unwrap();
        fs::write(pulling_marker(&ch, &f), format!("{}", std::process::id())).unwrap();
        let row = &k.rows("mybox", &ch, &m, |_| 0)[0];
        assert_eq!(row.state.state, State::Busy, "a live pull (this pid) = busy");

        fs::remove_file(pulling_marker(&ch, &f)).unwrap();
        let row = &k.rows("mybox", &ch, &m, |_| 0)[0];
        assert_eq!(row.state.state, State::Errored, "a partial with no live pull = resumable");

        fs::remove_file(partial_path(&ch, &f)).unwrap();
        fs::write(payload_path(&ch, &f), b"hello").unwrap();
        let row = &k.rows("mybox", &ch, &m, |_| 0)[0];
        assert_eq!(row.state.state, State::Done, "payload landed, not looked at yet");
        assert_eq!(row.unread, 1);

        let row = &k.rows("mybox", &ch, &m, |_| i64::MAX)[0];
        assert_eq!((row.state.state, row.unread), (State::Idle, 0), "seen clears it");
        let _ = fs::remove_dir_all(&root);
    }

    /// The sender's face rides the offer record; a third device derives
    /// no row at all (its view waits on remote state — 挂账).
    #[test]
    fn the_senders_row_rides_the_offer_and_third_parties_get_none() {
        let root = tmp("sender");
        let ch = root.join("ch");
        fs::create_dir_all(&ch).unwrap();
        let k = kind(&root);
        let f = file_ref("data.bin");
        let mine = [msg("m1", &"me".repeat(32), f.clone())];

        let row = &k.rows("otherbox", &ch, &mine, |_| 0)[0];
        assert_eq!(row.state.state, State::Blocked, "no fetch yet = the far approval is the cure");

        save_offer(&root, &f.digest, &Offer {
            path: "/tmp/x".into(), name: f.name.clone(), size: 5,
            started: true, done: false,
        }).unwrap();
        let row = &k.rows("otherbox", &ch, &mine, |_| 0)[0];
        assert_eq!(row.state.state, State::Busy);

        let theirs = [msg("m2", "someone-else", f.clone())];
        assert!(
            k.rows("otherbox", &ch, &theirs, |_| 0).is_empty(),
            "neither sender nor recipient = no row yet"
        );
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
