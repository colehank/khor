//! Khor-wide sessions (STORE-BRIEF foundation B): a session that belongs
//! to the whole khor, not to one machine.
//!
//! **The broadcast** (`wide/all`, phase 3) is the first: the group
//! conversation of every machine. Its document is a conversation
//! (`khor_sync::chat::ChatDoc`) every machine keeps and syncs under the
//! wire name `wide/all`, so a message spreads machine to machine at each
//! round and outlives its sender going offline. Every machine derives
//! the row from its own copy (`WideKind`), the way a machine's window is
//! derived — nothing about it travels as a report.
//!
//! **A file in it is an offer.** Each machine answers it on its own: it
//! accepts (the transfer's `take`, here), then pulls the bytes from the
//! sender into its own receive location, writing how that stands under
//! its own key on the message (`ChatDoc::set_fate_of`). So every device
//! reads every machine's state, and the sender counts the machines that
//! have it. The pull asks the sender by digest; the road is
//! content-addressed, so a later step can ask any machine that already
//! holds the digest instead (`road::Source::Offer`'s `from`).
//!
//! **The row wears its latest message's word** (STORE-BRIEF phase 5,
//! the row's word; the owner, 2026-10-12: 「所有 session 都要保持最新消息」).
//! Only the newest message still standing speaks, from this machine's
//! seat: a file to this machine still moving is 忙碌, one whose move
//! broke is 中断 (a retry resumes); otherwise unseen messages make 未看
//! and the rest 空闲. An older message's fate stays on its own bubble and
//! presses nothing on the row — a terminal or an agent wears its live
//! process's word the same way. An offer this machine has not answered
//! is a file it *may* take, unread like any message, never 待批.
//! ⚠️ 墓碑: the row folded every file in the history and kept the most
//! pressing word, so a pull that broke weeks ago said 中断 for ever and
//! an offer nobody took said 待批 for ever.
//!
//! **A fate is kept by (machine, bytes), not by message** ([`Kept`]): the
//! road is content-addressed, so the same bytes landed later under
//! another message retire an earlier bubble's 中断 too.
//!
//! The row stands on every machine as soon as the khor has two machines
//! that run khor (a satellite has no khor to answer an offer with), with
//! nothing to create first.
//!
//! **How the khor agent fits** (phase 4, not built): another board,
//! `wide/<name>`, the same document with its head naming the machine
//! that runs it (`ChatDoc::home`). Its row is derived the same way on
//! every machine; what differs is that the agent's answers come from its
//! home, and a move of home is a write to the head.

use std::collections::HashMap;

use khor_catalog::msg;
use khor_core::{Millis, Session, SessionId, State, StateStamp};
use khor_sync::chat::{Fate, Message, MsgBody, Step};
use khor_sync::devices::DeviceInfo;

use crate::chat::ChannelLog;
use crate::{KindSurface, Node};

/// The broadcast's board, which is also its session id and the name its
/// document syncs under.
pub const BROADCAST: &str = "wide/all";

/// The prefix every khor-wide board and session id carries.
pub use khor_sync::chat::WIDE;

/// The board a caller named — refused unless this build keeps one by
/// that name (only the broadcast, so far). A typo must not open a new
/// khor-wide conversation that every machine then syncs.
pub(crate) fn board(name: &str) -> Result<&'static str, String> {
    if name == BROADCAST {
        Ok(BROADCAST)
    } else {
        Err(msg::bad_channel_name(format_args!("{name:?}")))
    }
}

/// One broadcast file, as this machine reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WideFace {
    /// This machine's own answer to the offer; `None` while it has not
    /// answered, and on the machine that sent it.
    pub here: Option<Fate>,
    /// The machines that have it (their fate is landed) …
    pub got: u32,
    /// … out of the machines it was offered to: every member but its
    /// sender.
    pub of: u32,
}

/// A khor-wide conversation's readout, each message with its face —
/// `None` for anything that is not a file.
pub type WideLog = (ChannelLog, Vec<Option<WideFace>>);

/// The bytes a file message carries, digest by digest in its order —
/// what a fate is kept under. `None` for anything that is not files.
fn bytes(m: &Message) -> Option<Vec<&str>> {
    match &m.body {
        MsgBody::Files(fs) if !fs.is_empty() => Some(fs.iter().map(|f| f.digest.as_str()).collect()),
        _ => None,
    }
}

/// **A fate is kept by (machine, bytes), not by message** (STORE-BRIEF
/// phase 5, the row's word, rule 2). The road is content-addressed: the
/// same bytes landed on a machine under a later message have landed for
/// an earlier one too, so that earlier bubble reads landed — its 中断
/// retired — and the sender's count includes the machine.
///
/// A map built at read time over the messages' own records; the wire
/// keeps one record per (message, machine) as before. Only a landing
/// crosses messages: it is a fact about the bytes. A move still running
/// or one that broke belongs to the message that started it — its stop
/// and its retry act on that message's own job.
pub(crate) struct Kept<'a> {
    landed: HashMap<(&'a str, Vec<&'a str>), &'a Fate>,
}

impl<'a> Kept<'a> {
    /// Every landing in the conversation, by (machine, bytes); the later
    /// one in the document's order where the same bytes landed twice.
    pub(crate) fn read(msgs: &'a [Message]) -> Kept<'a> {
        let mut landed = HashMap::new();
        for m in msgs {
            let Some(key) = bytes(m) else { continue };
            for (machine, f) in &m.fates {
                if f.step == Step::Landed {
                    landed.insert((machine.as_str(), key.clone()), f);
                }
            }
        }
        Kept { landed }
    }

    /// How `m`'s bytes stand on `machine`: its own record, unless the
    /// same bytes landed there under another message.
    pub(crate) fn fate(&self, m: &'a Message, machine: &str) -> Option<&'a Fate> {
        let own = m.fates.get(machine);
        if own.is_some_and(|f| f.step == Step::Landed) {
            return own;
        }
        bytes(m)
            .and_then(|key| self.landed.get(&(machine, key)).copied())
            .or(own)
    }
}

/// The face of one file message, read against the members.
fn face_of(m: &Message, me: &str, members: &[DeviceInfo], kept: &Kept<'_>) -> Option<WideFace> {
    if !matches!(m.body, MsgBody::Files(_)) || m.retracted {
        return None;
    }
    let to: Vec<&DeviceInfo> = members.iter().filter(|d| d.id != m.from.id).collect();
    Some(WideFace {
        here: (m.from.id != me).then(|| kept.fate(m, me).cloned()).flatten(),
        got: to
            .iter()
            .filter(|d| kept.fate(m, &d.id).is_some_and(|f| f.step == Step::Landed))
            .count() as u32,
        of: to.len() as u32,
    })
}

impl Node {
    pub fn record_command(&self, record: &khor_sync::chat::CommandRecord) -> Result<String, String> {
        let id = self.chat.command(BROADCAST, record)?;
        let _ = self.emit_row_of(&khor_core::SessionId(BROADCAST.to_owned()));
        Ok(id)
    }

    /// The machines a broadcast reaches: every machine that runs khor.
    /// A satellite is left out — it has no khor of its own to answer an
    /// offer with, and counting it would leave every count one short
    /// forever.
    pub fn wide_members(&self) -> Result<Vec<DeviceInfo>, String> {
        Ok(self
            .devices()?
            .into_iter()
            .filter(|d| d.sponsor.is_none())
            .collect())
    }

    /// The readout with each file's face, from one read of the document.
    pub fn wide_log(&self, name: &str) -> Result<WideLog, String> {
        let board = board(name)?;
        let log = self.chat.log(board)?;
        let members = self.wide_members()?;
        let kept = Kept::read(&log.messages);
        let faces = log
            .messages
            .iter()
            .map(|m| face_of(m, self.device_str(), &members, &kept))
            .collect();
        Ok((log, faces))
    }
}

/// The khor-wide rows (module head).
pub(crate) struct WideKind;

impl KindSurface for WideKind {
    fn claims(&self, _node: &Node, id: &SessionId) -> bool {
        id.0 == BROADCAST
    }

    fn rows(&self, node: &Node) -> Result<Vec<Session>, String> {
        let seen = node.seen_loaded()?;
        let watermark = seen.doc.watermark(BROADCAST);
        let msgs = node.chat.log(BROADCAST)?.messages;
        let mut row: Session = node.chat.row_from(BROADCAST, node.device(), &msgs, watermark, Some(&seen.doc));
        row.title = msg::broadcast_title().to_owned();
        // The latest message still standing speaks, from this machine's
        // seat (the module head): a file to this machine still moving or
        // broken presses its word; anything else leaves the row its own
        // 未看/空闲. Still standing, as the row's preview reads it
        // (`ChatKind::row_from`'s `last`): a retracted slot has no word.
        let me = node.device_str();
        let kept = Kept::read(&msgs);
        let active_agent = msgs.iter().rev().find(|m| {
            !m.retracted
                && m.command.as_ref().is_some_and(|c| {
                    c.agent.is_some()
                        && c.ended_at_ms.is_none()
                        && matches!(c.status.as_str(), "running" | "blocked")
                })
        });
        if let Some(c) = active_agent.and_then(|m| m.command.as_ref()) {
            row.last = Some(khor_core::preview(if c.stdout.is_empty() {
                &c.line
            } else {
                &c.stdout
            }));
        }
        let pressed = active_agent
            .or_else(|| msgs.iter().rev().find(|m| !m.retracted))
            .and_then(|m| {
                if let Some(c) = &m.command {
                    let word = match c.status.as_str() {
                        "running" => State::Busy,
                        "blocked" if c.agent.is_some() => State::Blocked,
                        "failed" => State::Failed,
                        "interrupted" => State::Errored,
                        _ => return None,
                    };
                    return Some((word, m.at));
                }
                if m.from.id == me || bytes(m).is_none() {
                    return None;
                }
                let word = match kept.fate(m, me)?.step {
                    Step::Moving => State::Busy,
                    Step::Failed => State::Errored,
                    // Not answered: an offer one may take, not a
                    // question that blocks the row (rule 3).
                    Step::Landed | Step::Stopped => return None,
                };
                Some((word, m.at))
            });
        // The message's own word, not the more pressing of two: a file
        // still moving here is 忙碌 even while it is unseen — the order
        // the ruling reads (忙碌, 中断, then 未看, then 空闲), and the
        // row goes 忙碌 → 未看 as it lands.
        if let Some((word, at)) = pressed {
            row.state = StateStamp {
                state: word,
                at: Millis(at.max(0) as u64),
            };
        }
        // Put away like a machine's window (`ChatKind`'s rows) — but a
        // newest file still moving or broken here keeps it out.
        if pressed.is_none()
            && let Some(closed) = seen.doc.closed_watermark(BROADCAST)
            && msgs.iter().map(|m| m.at).max().unwrap_or(0) <= closed
        {
            return Ok(Vec::new());
        }
        Ok(vec![row])
    }

    /// Every machine derives its own, from its own copy.
    fn reportable(&self) -> bool {
        false
    }

    fn seen_at(&self, node: &Node, _id: &SessionId) -> Result<i64, String> {
        Ok(node
            .chat
            .log(BROADCAST)?
            .messages
            .iter()
            .map(|m| m.at)
            .max()
            .unwrap_or(0))
    }

    /// Put away, never deleted — a machine's window's close
    /// (docs/SESSION.md 动作): the next message brings it back.
    fn close(&self, node: &Node, id: &SessionId) -> Result<(), String> {
        let newest = self.seen_at(node, id)?;
        let loaded = node.seen_loaded()?;
        loaded.doc.mark(&id.0, newest)?;
        loaded.doc.put_away(&id.0, newest)?;
        let mut store = loaded.store;
        store.flush(&loaded.doc)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    //! **The row's word** (STORE-BRIEF phase 5): the broadcast row wears
    //! its latest message's word, a fate is kept by (machine, bytes), and
    //! an offer never says 待批. Each test plants the far machine's
    //! messages as a foreign block, the way they really arrive.

    use std::fs;

    use khor_sync::chat::{ChatDoc, FileRef, Sender, wide_dir};

    use super::*;

    /// A scratch tree that takes itself away however the test leaves
    /// (`crate::tests::Scratch` has the reasoning; a copy per file is
    /// this repo's convention for test helpers).
    struct Scratch(std::path::PathBuf);

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    impl std::ops::Deref for Scratch {
        type Target = std::path::Path;

        fn deref(&self) -> &std::path::Path {
            &self.0
        }
    }

    /// A node with one more machine in its khor, so the broadcast stands.
    /// No real relay, as `crate::tests::root`.
    fn node(tag: &str) -> (Scratch, Node, Sender) {
        let p = std::env::temp_dir().join(format!("khor-wide-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(p.join(".khor")).unwrap();
        fs::write(
            p.join(".khor").join("relays.json"),
            r#"{"relays":[{"url":"http://127.0.0.1:9"}],"only_mine":true}"#,
        )
        .unwrap();
        let r = Scratch(p);
        let n = Node::open(r.to_path_buf()).unwrap();
        let far = Sender { id: "dd".repeat(32), name: "far".into() };
        let loaded = n.devices_loaded().unwrap();
        loaded.doc.upsert(&far.id, &far.name, &[]).unwrap();
        let mut store = loaded.store;
        store.flush(&loaded.doc).unwrap();
        (r, n, far)
    }

    /// The far machine's document, landed here as its block.
    fn plant(r: &std::path::Path, far: &ChatDoc) {
        let dir = wide_dir(r, BROADCAST).unwrap();
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("u-000000000000fafa-00000000.loro"), far.changes_since(&Default::default()).unwrap()).unwrap();
    }

    fn bytes_ref(name: &str) -> FileRef {
        FileRef { name: name.into(), size: 6, digest: "ef".repeat(32), holder: String::new() }
    }

    fn fate(step: Step, landed: &str) -> Fate {
        Fate {
            step,
            moved: 0,
            landed: if landed.is_empty() { Vec::new() } else { vec![landed.to_owned()] },
            why: if step == Step::Failed { "broke".into() } else { String::new() },
            via: String::new(),
        }
    }

    /// Distinct stamps: same-millisecond messages would make the order a
    /// matter of the clock rather than of the rule.
    fn tick() {
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    #[test]
    fn reading_a_snapshot_leaves_later_messages_unread() {
        let (r, n, far) = node("snapshot-seen");
        let doc = ChatDoc::new(0xfafa).unwrap();
        doc.tell(&far, "first visible message").unwrap();
        let visible = crate::SeenMessage { id: doc.messages()[0].id.clone(), at_ms: doc.messages()[0].at };
        tick();
        doc.tell(&far, "not rendered yet").unwrap();
        plant(&r, &doc);
        n.seen_snapshot(&SessionId(BROADCAST.into()), &[visible.clone()]).unwrap();
        let rows = WideKind.rows(&n).unwrap();
        assert_eq!(rows[0].unread, 1);
        n.seen_snapshot(&SessionId(BROADCAST.into()), &[visible]).unwrap();
        assert_eq!(WideKind.rows(&n).unwrap()[0].unread, 1, "old receipts never move backwards");
        n.seen(&SessionId(BROADCAST.into())).unwrap();
        assert_eq!(WideKind.rows(&n).unwrap()[0].unread, 0);
    }

    #[test]
    fn console_prose_does_not_resolve_mentions_or_paths() {
        let (_r, n, _far) = node("literal-note");
        n.note_here("@unknown:/missing ordinary text", &[]).unwrap();
        let log = n.log(BROADCAST).unwrap();
        assert!(log.messages[0].to.is_empty());
        assert_eq!(log.messages[0].body, MsgBody::Text("@unknown:/missing ordinary text".into()));
    }

    fn word(n: &Node) -> State {
        WideKind.rows(n).unwrap().first().expect("the broadcast row stands").state.state
    }

    /// Bubble `i`'s fate on this machine, as the face reads it.
    fn here(n: &Node, i: usize) -> Option<Step> {
        n.wide_log(BROADCAST).unwrap().1[i].as_ref().and_then(|f| f.here.as_ref().map(|f| f.step))
    }

    /// **The same bytes landed later retire the old 中断** (rule 2), and
    /// the row reads the newest message (rule 1): an old pull that broke
    /// and a newer message with the same bytes that landed → the row is
    /// 未看 (unseen) and then 空闲, and the old bubble reads landed.
    #[test]
    fn a_later_landing_of_the_same_bytes_retires_the_old_break() {
        let (r, n, who) = node("retire");
        let far = ChatDoc::new(0xFAFA).unwrap();
        far.post(&who, "", &[], &[bytes_ref("a.bin")]).unwrap();
        plant(&r, &far);
        let old = far.messages()[0].id.clone();
        n.chat.set_fate(BROADCAST, &old, &fate(Step::Failed, "")).unwrap();
        assert_eq!(word(&n), State::Errored, "control: the newest file broke here");
        assert_eq!(here(&n, 0), Some(Step::Failed));

        tick();
        far.post(&who, "", &[], &[bytes_ref("a-again.bin")]).unwrap();
        plant(&r, &far);
        let new = far.messages()[1].id.clone();
        n.chat.set_fate(BROADCAST, &new, &fate(Step::Landed, "/drop/a-again.bin")).unwrap();
        assert_eq!(word(&n), State::Done, "the newest landed: the row is 未看, the old break presses nothing");
        assert_eq!(here(&n, 0), Some(Step::Landed), "the old bubble reads landed: the same bytes are here");
        assert_eq!(n.wide_log(BROADCAST).unwrap().1[0].as_ref().unwrap().got, 1, "and counts this machine");
        n.seen(&SessionId(BROADCAST.into())).unwrap();
        assert_eq!(word(&n), State::Idle, "looked at: 空闲");
    }

    /// **The row is the latest message's** (rule 1): a line after a file
    /// still moving here → the row says 未看, the bubble 忙碌.
    #[test]
    fn a_newer_line_after_a_moving_file_takes_the_row() {
        let (r, n, who) = node("latest");
        let far = ChatDoc::new(0xFAFA).unwrap();
        far.post(&who, "", &[], &[bytes_ref("big.bin")]).unwrap();
        plant(&r, &far);
        let id = far.messages()[0].id.clone();
        n.chat.set_fate(BROADCAST, &id, &fate(Step::Moving, "")).unwrap();
        assert_eq!(word(&n), State::Busy, "the newest file is moving here: 忙碌, although nobody here has seen it");

        tick();
        far.tell(&who, "did it arrive?").unwrap();
        plant(&r, &far);
        assert_eq!(word(&n), State::Done, "the newest message is a line nobody here has seen: 未看");
        assert_eq!(here(&n, 0), Some(Step::Moving), "the file's own bubble still says 忙碌");
    }

    /// **An offer never says 待批** (rule 3): a broadcast file this
    /// machine never accepted is unread like any message, then 空闲.
    #[test]
    fn an_offer_nobody_took_is_not_blocked() {
        let (r, n, who) = node("offer");
        let far = ChatDoc::new(0xFAFA).unwrap();
        far.post(&who, "", &[], &[bytes_ref("maybe.bin")]).unwrap();
        plant(&r, &far);
        assert_eq!(here(&n, 0), None, "never answered");
        assert_eq!(word(&n), State::Done, "a file one may take: 未看, never 待批");
        n.seen(&SessionId(BROADCAST.into())).unwrap();
        assert_eq!(word(&n), State::Idle, "looked at and still not taken: 空闲, nothing waits");
    }
}
