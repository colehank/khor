use std::fs;
use std::path::{Path, PathBuf};

use khor_core::State;
use khor_sync::chat::{channel_dir, ChatDoc, Sender};
/// For `SeenDoc::changes_since` — the seen document's export lives on the
/// trait, unlike `ChatDoc`'s, which also has it inherently.
use khor_sync::store::Doc;

use super::*;

fn root(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("khor-node-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn notes_to_self_are_seen_the_moment_they_are_told() {
    let r = root("tell");
    let n = Node::open(r.clone()).unwrap();
    n.tell(n.name().to_owned().as_str(), "a note to self").unwrap();

    let rows = n.sessions().unwrap();
    assert_eq!(rows.len(), 1);
    let row = &rows[0].session;
    assert_eq!(row.title, n.name());
    assert_eq!(row.home, n.device());
    assert_eq!(row.unread, 0, "own words never count as unread");
    assert_eq!(row.state.state, State::Idle);
    assert!(row.state.at.0 > 0, "the stamp should be the last message's");

    let log = n.log(n.name().to_owned().as_str()).unwrap();
    assert_eq!(log.broken, 0);
    assert_eq!(log.messages.len(), 1);
    let _ = fs::remove_dir_all(&r);
}

/// The dumb path in miniature: a block from another device lands as a
/// file, and the row must go Done/unread without any network.
#[test]
fn a_foreign_block_raises_unread_and_seen_clears_it() {
    let r = root("foreign");
    let n = Node::open(r.clone()).unwrap();

    let far = ChatDoc::new(0xF0F0).unwrap();
    far.tell(&Sender { id: "dev-far".into(), name: "far".into() }, "from far away")
        .unwrap();
    let block = far.changes_since(&Default::default()).unwrap();
    let dir = channel_dir(&r, n.name()).unwrap();
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("u-000000000000f0f0-00000000.loro"), &block).unwrap();

    let row = &n.sessions().unwrap()[0].session;
    assert_eq!(row.unread, 1, "the far line should count as unread");
    assert_eq!(row.state.state, State::Done);

    n.seen(&row.id).unwrap();
    let row = &n.sessions().unwrap()[0].session;
    assert_eq!(row.unread, 0);
    assert_eq!(row.state.state, State::Idle);
    let _ = fs::remove_dir_all(&r);
}

#[test]
fn watch_receives_what_tell_emits() {
    let r = root("watch");
    let n = Node::open(r.clone()).unwrap();
    let rx = n.watch();
    n.tell(n.name().to_owned().as_str(), "push one").unwrap();

    let first = rx.try_recv().expect("an event should arrive");
    let second = rx.try_recv().expect("a row update should follow");
    assert!(matches!(first, NodeEvent::Event(_)), "event first");
    match second {
        NodeEvent::Row(row) => assert_eq!(row.unread, 0),
        other => panic!("then a row update, got {other:?}"),
    }
    let _ = fs::remove_dir_all(&r);
}

/// Close is putting the row away, never deletion (chat 收起批, on top of
/// 传输并入对话批's "close deletes nothing"): the row leaves the list,
/// history AND received files stay exactly where they were, and the
/// next message brings the row back owing only itself — the WeChat
/// shape, with the whole ledger intact underneath.
#[test]
fn closing_a_chat_folds_the_row_and_a_message_brings_it_back() {
    let r = root("close");
    let n = Node::open(r.clone()).unwrap();
    n.tell(n.name().to_owned().as_str(), "the line that stays").unwrap();
    let dir = channel_dir(&r, n.name()).unwrap();
    let files = dir.join("files");
    fs::create_dir_all(&files).unwrap();
    fs::write(files.join("payload.bin"), b"received bytes").unwrap();

    let id = n.sessions().unwrap()[0].session.id.clone();
    n.close(&id).unwrap();
    assert!(
        files.join("payload.bin").exists(),
        "a received file landed on this filesystem; close must not take it back"
    );
    let log = n.log(n.name().to_owned().as_str()).unwrap();
    assert_eq!(log.messages.len(), 1, "the history must survive close");
    assert!(
        n.sessions().unwrap().is_empty(),
        "the put-away row must leave the list"
    );

    // The way back is a message past the mark. The pause is real: the
    // put-away watermark is the last message's clock, and a reply landing
    // in the very same millisecond would still be under it.
    std::thread::sleep(std::time::Duration::from_millis(5));
    n.tell(n.name().to_owned().as_str(), "the line that reopens").unwrap();
    let rows = n.sessions().unwrap();
    assert_eq!(rows.len(), 1, "a new message must bring the row back");
    assert_eq!(
        n.log(n.name().to_owned().as_str()).unwrap().messages.len(),
        2,
        "…and it comes back with the whole history, not a fresh page"
    );
    let row = &rows[0].session;
    assert_eq!((row.unread, row.state.state), (0, State::Idle));
    let _ = fs::remove_dir_all(&r);
}

/// Closing marks seen to the same instant: a revived row owes only the
/// messages that revived it, never the debt the person dismissed when
/// they put the row away. Foreign words, because own words are auto-seen
/// and would make this assertion pass with the mark broken.
#[test]
fn a_revived_chat_owes_only_what_revived_it() {
    let r = root("close-unread");
    let n = Node::open(r.clone()).unwrap();
    let dir = channel_dir(&r, n.name()).unwrap();
    fs::create_dir_all(&dir).unwrap();
    let far = ChatDoc::new(0xF0F0).unwrap();
    let sender = Sender { id: "dev-far".into(), name: "far".into() };
    far.tell(&sender, "unread when closed").unwrap();
    fs::write(
        dir.join("u-000000000000f0f0-00000000.loro"),
        far.changes_since(&Default::default()).unwrap(),
    )
    .unwrap();
    let before = &n.sessions().unwrap()[0].session;
    assert_eq!(before.unread, 1, "control: the dismissed debt was real");
    let id = before.id.clone();

    n.close(&id).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(5));
    far.tell(&sender, "the reviver").unwrap();
    fs::write(
        dir.join("u-000000000000f0f0-00000001.loro"),
        far.changes_since(&Default::default()).unwrap(),
    )
    .unwrap();

    let rows = n.sessions().unwrap();
    assert_eq!(rows.len(), 1, "the second message must bring the row back");
    assert_eq!(
        rows[0].session.unread, 1,
        "only the reviver counts: close marked the dismissed debt seen"
    );
    let _ = fs::remove_dir_all(&r);
}

/// forget 批: a machine shown the door takes its rows with it — its chat
/// row stops being derived (the table no longer holds it) and its cached
/// report stops being painted, while the in-table sibling keeps every
/// row untouched (the control). Nothing is deleted: the report cache
/// stays on disk, so a machine invited back resurfaces its last rows at
/// their honest age.
#[test]
fn a_forgotten_machines_rows_leave_the_list() {
    let r = root("forget-rows");
    let n = Node::open(r.clone()).unwrap();
    let keep_id = "aa".repeat(32);
    let gone_id = "bb".repeat(32);
    {
        let loaded = n.devices_loaded().unwrap();
        loaded.doc.upsert(&keep_id, "keeper", &[]).unwrap();
        loaded.doc.upsert(&gone_id, "beta-probe", &[]).unwrap();
        let mut store = loaded.store;
        store.flush(&loaded.doc).unwrap();
    }
    // One reported row each — a clone of a real row rather than a
    // literal, so this test does not have to know every Session field.
    let mut template = n.sessions().unwrap()[0].session.clone();
    template.id = SessionId("shell/on-keeper".into());
    n.cache_peer_rows(&keep_id, "keeper", &[template.clone()]).unwrap();
    template.id = SessionId("shell/on-beta".into());
    n.cache_peer_rows(&gone_id, "beta-probe", &[template]).unwrap();

    let ids = |views: &[SessionView]| -> Vec<String> {
        views.iter().map(|v| v.session.id.0.clone()).collect()
    };
    let before = ids(&n.sessions().unwrap());
    for id in ["chat/keeper", "chat/beta-probe", "shell/on-keeper", "shell/on-beta"] {
        assert!(before.iter().any(|i| i == id), "control: {id} is on the list before the forget");
    }

    n.forget_device("beta-probe").unwrap();

    let after = ids(&n.sessions().unwrap());
    for id in ["chat/beta-probe", "shell/on-beta"] {
        assert!(!after.iter().any(|i| i == id), "{id} must leave with its machine");
    }
    for id in ["chat/keeper", "shell/on-keeper"] {
        assert!(after.iter().any(|i| i == id), "{id} belongs to a machine still in the table");
    }
    assert!(
        r.join(".khor").join("peers").join(&gone_id).exists(),
        "the report cache stays on disk — only the painting stops"
    );
    let _ = fs::remove_dir_all(&r);
}

/// **A send makes a message, not a row** (传输并入对话批). The offer is
/// in the machine's log as a file message; no `transfer/` row joins the
/// list; and the sender's own chat row is not 待批 — the approval being
/// waited on is the far machine's, and a badge this machine cannot
/// clear by answering would break 角标可归零. (The receiving seat's
/// half of that pair is the next test.)
#[test]
fn a_send_makes_a_file_message_and_no_transfer_row() {
    let r = root("send-msg");
    let n = Node::open(r.clone()).unwrap();
    let payload = r.join("hello.txt");
    fs::write(&payload, b"five!").unwrap();

    // To this machine's own window: sender seat, and a channel that
    // exists without pairing anybody.
    let tid = n.send(n.name().to_owned().as_str(), &payload).unwrap();
    assert!(tid.0.starts_with("transfer/"), "the accept handle is still a transfer id");

    let rows = n.sessions().unwrap();
    assert!(
        !rows.iter().any(|v| v.session.id.0.starts_with("transfer/")),
        "no transfer row on the list any more"
    );
    let log = n.log(n.name().to_owned().as_str()).unwrap();
    assert!(
        log.messages.iter().any(|m| matches!(&m.body, MsgBody::Files(f) if f[0].name == "hello.txt")),
        "the offer must be a file message in the machine's log"
    );
    let chat = rows
        .iter()
        .find(|v| v.session.id.0.starts_with("chat/"))
        .expect("the machine's chat row exists");
    assert_ne!(
        chat.session.state.state,
        State::Blocked,
        "the sender's seat has nothing to approve: its row must not say 待批"
    );
    let _ = fs::remove_dir_all(&r);
}

/// **Offering is not looking.** `khor send` used to raise the offered-to
/// channel's seen watermark to the offer's own stamp — "telling implies
/// looking", borrowed from `tell` — and the seen document is replicated,
/// so the machine that had something to hand over cleared the badge on
/// the machine the file was actually for. The recipient's row read
/// 空闲/0 where docs/SESSION.md says an arrived file goes to 完成/未看.
///
/// This watches the cause from the only seat that can be watched with
/// one node: the offering machine's own `send` must leave the watermark
/// where it found it. (The effect at the other end, over a real
/// connection, is `tests/seen_watermark.rs`.)
///
/// **The positive control is the same probe on the same document one
/// line later**: `tell` does raise it. Without that, "the watermark is
/// 0" and "this probe reads 0 for every id there is" are the same
/// sentence in a test report.
#[test]
fn an_offer_must_not_raise_the_seen_watermark() {
    let r = root("offer-watermark");
    let n = Node::open(r.clone()).unwrap();
    let me = n.name().to_owned();
    let sid = format!("chat/{me}");
    let watermark = |n: &Node| n.seen_loaded().unwrap().doc.watermark(&sid);

    let payload = r.join("handover.bin");
    fs::write(&payload, b"bytes").unwrap();
    n.send(&me, &payload).unwrap();
    assert_eq!(watermark(&n), 0, "offering is not looking: no mark may be written");

    // And the offering seat still does not badge itself for what it just
    // handed over. That answer is the seat's, derived locally
    // (`ChatKind::i_offered`) — never a mark in a document that travels
    // to every machine and speaks for all of them.
    let row = n
        .sessions()
        .unwrap()
        .into_iter()
        .find(|v| v.session.id.0 == sid)
        .expect("the machine's chat row exists")
        .session;
    assert_eq!(row.unread, 0, "the machine that offered it has nothing to look at");

    // Same probe, same document, one line later.
    std::thread::sleep(std::time::Duration::from_millis(5));
    n.tell(&me, "and a line about it").unwrap();
    assert!(
        watermark(&n) > 0,
        "control: telling *is* looking, and this probe can read a mark when there is one"
    );
    let _ = fs::remove_dir_all(&r);
}

/// The receiving seat's half of the pair: an offer sent TO this machine
/// presses 待批 onto its chat row — the approval asked for is this
/// machine's — and the payload landing clears it (accept 即清), to
/// 完成/未看 and not to 空闲.
///
/// **What this fixture can and cannot reach.** The far machine here is
/// planted bytes, so whatever it "did" is whatever this function writes:
/// the test can model one world, never compare two. It used to plant a
/// message with **nobody behind it** — no sender's watermark at all —
/// which is a world no `khor send` could produce, and so it stayed green
/// through the entire life of the bug that `Node::send` marked
/// `chat/<recipient>` seen as it offered and cleared this very badge.
/// Planting *that* mark instead only swaps which single world is frozen:
/// it would be red forever, including after the fix.
///
/// So the fixture below plants the mark a far machine really does leave
/// — the one a **`tell`** raises, which still replicates and by
/// construction stands *below* a later offer — and the property under
/// test is that the offer counts above a legitimately standing
/// watermark. **The send-side leak itself is not testable from one
/// node**: it is a fact about what the other machine wrote, and it is
/// owned by `tests/seen_watermark.rs` (two real nodes, a real send) and
/// by `an_offer_must_not_raise_the_seen_watermark` below, which watches
/// this machine's own `send` from the offering seat.
#[test]
fn an_incoming_offer_blocks_the_chat_row_and_the_landing_clears_it() {
    let r = root("recv-offer");
    let n = Node::open(r.clone()).unwrap();

    // A file offer from another device, landed as a chat block — the
    // same shape `a_foreign_block_raises_unread_and_seen_clears_it`
    // plants a text message with. A line comes first, so that the
    // watermark planted below is one a real machine leaves behind.
    let far = ChatDoc::new(0xFAFA).unwrap();
    let who = Sender { id: "dev-far".into(), name: "far".into() };
    far.tell(&who, "sending you something").unwrap();
    let said_at = far.messages().last().expect("the line is in the doc").at;
    // Distinct stamps: same-millisecond messages would put the offer at
    // the watermark rather than above it, and the reading below would be
    // about the clock instead of about the rule.
    std::thread::sleep(std::time::Duration::from_millis(5));
    let f = khor_node_test_fileref();
    far.send_files(&who, std::slice::from_ref(&f)).unwrap();
    let block = far.changes_since(&Default::default()).unwrap();
    let dir = channel_dir(&r, n.name()).unwrap();
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("u-000000000000fafa-00000000.loro"), &block).unwrap();

    // The far machine's watermark, arriving the way it really does: a
    // foreign block in this node's own seen store, raised by its `tell`
    // (telling implies looking) and therefore standing below the offer.
    let far_seen = SeenDoc::new(0xFAFA).unwrap();
    far_seen.mark(&format!("chat/{}", n.name()), said_at).unwrap();
    let sdir = seen_dir(&r);
    fs::create_dir_all(&sdir).unwrap();
    fs::write(
        sdir.join("u-000000000000fafa-00000000.loro"),
        far_seen.changes_since(&Default::default()).unwrap(),
    )
    .unwrap();

    let chat_row = |n: &Node| {
        n.sessions()
            .unwrap()
            .into_iter()
            .find(|v| v.session.id.0.starts_with("chat/"))
            .expect("the machine's chat row exists")
            .session
    };
    assert_eq!(
        chat_row(&n).state.state,
        State::Blocked,
        "an offer waiting on THIS machine's approval must press 待批 onto its row"
    );

    // The payload lands (what accept's pull produces): the ask is
    // answered, so 待批 clears — to the row's own message-based word.
    let files = dir.join("files");
    fs::create_dir_all(&files).unwrap();
    fs::write(crate::transfer::payload_path(&dir, &f), b"bytes!").unwrap();
    let row = chat_row(&n);
    assert_ne!(row.state.state, State::Blocked, "答了才清: the landing clears 待批");
    assert_eq!(row.state.state, State::Done, "…to the unseen file message's own 完成");
    assert_eq!(
        row.unread, 1,
        "and 完成 here means one message nobody at this machine has looked at — \
         the far machine's watermark is not this machine's answer"
    );
    let _ = fs::remove_dir_all(&r);
}

/// The file ref the incoming-offer test plants.
fn khor_node_test_fileref() -> khor_sync::chat::FileRef {
    khor_sync::chat::FileRef { name: "drop.bin".into(), size: 6, digest: "ef".repeat(32) }
}

/// Pinning lifts a row to the top of the list the library hands out, and
/// unpinning puts it back where it was — the order is the node's, so no
/// face has to own a comparison.
#[test]
fn a_pinned_row_leads_the_list_and_goes_back_when_unpinned() {
    let r = root("pin");
    let n = Node::open(r.clone()).unwrap();
    n.tell(n.name().to_owned().as_str(), "a note").unwrap();
    let live = n.open_ephemeral(khor_core::kind::TUI, "an agent").unwrap();

    let ids = |n: &Node| -> Vec<String> {
        n.sessions().unwrap().into_iter().map(|v| v.session.id.0).collect()
    };
    let chat_first = ids(&n);
    assert_eq!(chat_first.len(), 2);
    assert!(chat_first[0].starts_with("chat/"), "control: {chat_first:?}");

    n.pin_session(&live, true).unwrap();
    assert_eq!(ids(&n)[0], live.0, "the pinned row must lead");
    let pinned: Vec<bool> = n.sessions().unwrap().into_iter().map(|v| v.pinned).collect();
    assert_eq!(pinned, vec![true, false], "and only it wears the flag");

    n.pin_session(&live, false).unwrap();
    assert_eq!(ids(&n), chat_first, "unpinning restores the original order exactly");
    let _ = fs::remove_dir_all(&r);
}

/// A pin is a key, not a copy of the row. An id nothing answers to is
/// pinnable and paints nothing: refusing it would mean a session that
/// ends turns its own pin into an error, and an offline row could not be
/// pinned from the device looking at it.
#[test]
fn an_id_with_no_row_can_be_pinned_and_shows_nothing() {
    let r = root("pin-ghost");
    let n = Node::open(r.clone()).unwrap();
    n.tell(n.name().to_owned().as_str(), "a note").unwrap();

    n.pin_session(&SessionId("tui/long-gone".into()), true).unwrap();
    let rows = n.sessions().unwrap();
    assert_eq!(rows.len(), 1, "a pin must not conjure a row: {:?}", rows.len());
    assert!(!rows[0].pinned, "and it must not land on somebody else's row");
    let _ = fs::remove_dir_all(&r);
}

/// What khor says a row belongs to when khor itself started it.
///
/// A wrapped command is 命令 (`category::SHELL`) — the user started it,
/// and that is a real answer. **A tui khor cannot attribute carries no
/// category at all**, and the point of enumerating both here is that
/// filing the tui under `shell` would be the neighbour trap: it reads as
/// a confident answer where an honest gap belongs. A chat row is khor's
/// own doing, so it says so.
#[test]
fn khor_names_what_it_can_place_and_leaves_the_rest_blank() {
    let r = root("category");
    let n = Node::open(r.clone()).unwrap();
    n.tell(n.name().to_owned().as_str(), "a note").unwrap();
    let shell = n.open_ephemeral(khor_core::kind::SHELL, "build").unwrap();
    let tui = n.open_ephemeral(khor_core::kind::TUI, "some agent").unwrap();

    let of = |id: &SessionId| -> Option<String> {
        n.sessions()
            .unwrap()
            .into_iter()
            .find(|v| v.session.id == *id)
            .and_then(|v| v.session.category)
    };
    assert_eq!(of(&shell).as_deref(), Some(khor_core::category::SHELL));
    assert_eq!(of(&tui), None, "khor must not guess a vendor from a command line");

    let chat = SessionId(format!("chat/{}", n.name()));
    assert_eq!(of(&chat).as_deref(), Some(khor_core::category::KHOR));
    let _ = fs::remove_dir_all(&r);
}

/// **Every name khor already answers to is refused, one by one.**
///
/// Enumerated rather than stated as a property, because the property
/// version ("registering a reserved name fails") is also satisfied by a
/// registry that refuses *everything* — and by one that refuses only
/// the first name somebody thought of. The control at the bottom is the
/// other half: an ordinary name must still go through, or this gate
/// would be passing for the wrong reason.
///
/// What it guards is not tidiness. A registration's name becomes its
/// rows' `category`, which is the same word `UsageDay::category` uses:
/// an agent called `claude` would file its sessions into claude's
/// group, have its row preview hunted for in claude's transcripts, and
/// look covered by the `claude` line in the spending panel — which
/// counts only what the claude adaptor read off disk.
#[test]
fn the_names_khor_answers_to_are_not_up_for_registration() {
    let r = root("agents-reserved");
    let n = Node::open(r.clone()).unwrap();
    let argv = vec!["some-acp-agent".to_owned()];
    for name in ["claude", "codex", "khor", "shell"] {
        let refused = n.register_agent(name, &argv);
        assert!(refused.is_err(), "{name} is khor's own word, it must not be registrable");
        assert!(
            refused.unwrap_err().contains(name),
            "the refusal must name the name — a person cannot guess this list"
        );
        assert_eq!(n.agent(name).unwrap(), None, "and nothing was written under it");
    }
    // The control: without this, refusing every name on earth passes
    // every assertion above.
    n.register_agent("gemini", &argv).unwrap();
    // Read back through a **second** Node on the same store, because
    // "registered" is a claim about disk: this control is what caught
    // the registry never being flushed at all, with all four refusals
    // above passing while nothing was ever written.
    let again = Node::open(r.clone()).unwrap();
    assert_eq!(
        again.agent("gemini").unwrap().map(|s| s.typed()),
        Some("some-acp-agent".to_owned()),
        "an ordinary name registers, and survives being read by another process"
    );
    // And the list itself is the four, not "some reserved names": a
    // vendor dropped from it is a hole nobody would notice until a row
    // landed in the wrong group.
    assert_eq!(
        Node::RESERVED_AGENT_NAMES,
        &["claude", "codex", "khor", "shell"],
        "the reserved list is exactly khor's own four words"
    );
    let _ = fs::remove_dir_all(&r);
}

/// A name that is only whitespace, and a registration with no command,
/// are refused for two different reasons — and the blank name is the
/// one with teeth: an empty category collides with the group key that
/// means "nobody could tell whose this is" (`list::GROUP_CATEGORY`), so
/// the one group that is an admission of ignorance would start
/// collecting rows that are not.
#[test]
fn a_registration_needs_both_a_name_and_a_command() {
    let r = root("agents-blank");
    let n = Node::open(r.clone()).unwrap();
    assert!(n.register_agent("   ", &["x".to_owned()]).is_err(), "a blank name names nothing");
    assert!(n.register_agent("ok", &[]).is_err(), "a registration with no command starts nothing");
    assert!(n.agents().unwrap().is_empty(), "neither wrote anything");
    let _ = fs::remove_dir_all(&r);
}

/// The gap above closes the moment the vendor speaks for itself: a
/// claude hook naming a session is claude saying "this one is mine".
#[test]
fn a_vendors_hook_places_the_row_that_khor_could_not() {
    let r = root("category-hook");
    let n = Node::open(r.clone()).unwrap();
    let payload = r#"{"session_id":"feed01","cwd":"/tmp/proj","hook_event_name":"SessionStart"}"#;
    n.claude_hook(payload).unwrap();

    let row = n
        .sessions()
        .unwrap()
        .into_iter()
        .find(|v| v.session.id.0.starts_with("tui/"))
        .expect("the hook should have registered a row");
    assert_eq!(
        row.session.category.as_deref(),
        Some(crate::adaptor::claude::VENDOR),
        "a claude hook is claude naming its own session"
    );
    let _ = fs::remove_dir_all(&r);
}

/// A claude status file the sweep will pick up on every list: the pid
/// cannot exist (above every configured pid_max, the `pid_alive` test's
/// number), so the row surfaces on the crash rule with the vendor's own
/// `name` as its title — recomputed from this file each sweep, which is
/// exactly the force a user's rename has to outlive.
fn plant_discovered_claude(r: &Path, vendor_session: &str, vendor_name: &str) {
    let dir = crate::adaptor::vendor_home(r).join(".claude").join("sessions");
    fs::create_dir_all(&dir).unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    fs::write(
        dir.join("4242.json"),
        format!(
            r#"{{"pid":{},"sessionId":"{vendor_session}","name":"{vendor_name}","status":"busy","statusUpdatedAt":{now}}}"#,
            u32::MAX - 1
        ),
    )
    .unwrap();
}

/// A claude status file whose pid is **this** process — a sighting with
/// a live pid, which is the shape the rule under test is about.
fn plant_live_claude(r: &Path, vendor_session: &str, status: &str, pid: u32) {
    let dir = crate::adaptor::vendor_home(r).join(".claude").join("sessions");
    fs::create_dir_all(&dir).unwrap();
    let now = crate::live::now_ms();
    // The sweep vouches for a pid only when the file's `startedAt`
    // matches the process table's start (`Procs::alive_since` — a
    // recycled number is not the process that wrote the file), so the
    // fixture carries the real start of the process it names.
    let started = crate::adaptor::Procs::snapshot()
        .get(pid)
        .map(|p| p.started_ms)
        .expect("the process the fixture names must be in the table");
    fs::write(
        dir.join(format!("{pid}.json")),
        format!(
            r#"{{"pid":{pid},"sessionId":"{vendor_session}","name":"live-one","startedAt":{started},"status":"{status}","statusUpdatedAt":{now}}}"#
        ),
    )
    .unwrap();
}

/// **A word reported by a process that is not the one running the
/// session loses to the vendor's own live file.** Two arms, because
/// one alone is satisfied by a rule that always lets the disk win —
/// and that rule would demote every hook.
///
/// Found live 2026-09-01: a ghost resumed a session the person was
/// typing into in their own terminal; the reopen wrote 空闲 as
/// `Reported`, and `Reported` outranked the disk for as long as the
/// ghost lived, while claude's own file said busy the whole time.
#[test]
fn a_word_reported_by_another_process_loses_to_the_vendors_live_file() {
    let r = root("other-reporter");
    let n = Node::open(r.clone()).unwrap();
    let live = crate::live::LiveKind::new(r.clone(), n.device());
    let me = std::process::id();
    // The parent (the test harness) is another live process — a
    // reporter that is not the claude the file describes.
    let other = unsafe { libc::getppid() } as u32;
    assert!(crate::link::pid_alive(other), "probe dead: the other reporter must be alive");

    let vendor_session = "aaaaaaa2-0000-4000-8000-00000000000a";
    plant_live_claude(&r, vendor_session, "busy", me);
    let id = crate::adaptor::id_for(vendor_session);
    let word_of = |n: &Node| {
        n.sessions()
            .unwrap()
            .into_iter()
            .find(|v| v.session.id == id)
            .map(|v| v.session.state.state)
            .expect("the planted file surfaces as a row")
    };
    assert_eq!(word_of(&n), State::Busy, "control: the file alone reads busy");

    // A reopen by another process writes 空闲 first-hand — about itself.
    live.reopen(&id, khor_core::kind::GUI, "live-one", other).unwrap();
    assert_eq!(
        word_of(&n),
        State::Busy,
        "the reporter is not the process the file describes, so the file wins"
    );

    // The other arm: the same report from the very process the file
    // describes keeps its rank — this is a hook's shape, and a hook
    // must not be demoted.
    live.reopen(&id, khor_core::kind::GUI, "live-one", me).unwrap();
    assert_eq!(
        word_of(&n),
        State::Idle,
        "a first-hand word from the process itself outranks its own file"
    );
    let _ = fs::remove_dir_all(&r);
}

/// **A conversation that is running somewhere is not resumed.** The
/// refusal names the process, and no ghost is spawned — asserted on
/// the sentence, and on the row still wearing the live file's word.
#[test]
fn resuming_a_session_that_is_live_elsewhere_is_refused_by_name() {
    let r = root("resume-live");
    let n = Node::open(r.clone()).unwrap();
    let me = std::process::id();
    let vendor_session = "aaaaaaa2-0000-4000-8000-00000000000b";
    plant_live_claude(&r, vendor_session, "busy", me);
    let cwd = r.join("cwd");
    fs::create_dir_all(&cwd).unwrap();
    let refused = n.reopen_gui_as(&cwd, "t", &["true".into()], "claude", vendor_session);
    assert_eq!(
        refused,
        Err(msg::resume_running_elsewhere(me)),
        "a live session is refused, naming the process"
    );
    // The control, on the predicate and **not on the door**: a session
    // nobody is running has no live sighting, so the refusal cannot
    // fire for it. Not driven through `reopen_gui_as` — past the
    // refusal that door spawns `self_exe() _ghost …`, and in a test
    // `self_exe()` is this harness, which reads `_ghost … t -- true`
    // as a test filter and runs the suite again, ghosts and all
    // (found the hard way: four of them, ten minutes each).
    let dead = "aaaaaaa2-0000-4000-8000-00000000000c";
    let live = crate::live::LiveKind::new(r.clone(), n.device());
    assert!(
        live.sighting_of(&crate::adaptor::id_for(dead)).is_none(),
        "a session nobody runs has no sighting to refuse on"
    );
    let _ = fs::remove_dir_all(&r);
}

/// **The user's name outlives the vendor's sweep.** The sweep rebuilds a
/// discovered row's title from the vendor's status file on every list,
/// so a rename stored where the sweep writes would last one round. Two
/// lists after the rename stand for two sweeps; the second is the one a
/// meta-writing implementation fails.
#[test]
fn a_users_rename_outlives_the_vendors_sweep() {
    let r = root("rename-sweep");
    let n = Node::open(r.clone()).unwrap();
    plant_discovered_claude(&r, "renamed0001", "vendor-name");

    let row_of = |n: &Node| {
        n.sessions()
            .unwrap()
            .into_iter()
            .find(|v| v.session.id.0.starts_with("tui/"))
            .expect("the planted status file must surface as a row")
    };
    let before = row_of(&n);
    assert_eq!(before.session.title, "vendor-name", "control: the vendor names it first");

    n.rename_session(&before.session.id, "the rent job").unwrap();
    assert_eq!(row_of(&n).session.title, "the rent job", "the user's name must win the sweep");
    assert_eq!(
        row_of(&n).session.title,
        "the rent job",
        "…and the sweep after that: the vendor keeps deriving, the overlay keeps winning"
    );
    let _ = fs::remove_dir_all(&r);
}

/// The other half of the pair above: with no rename on record the
/// vendor's own title shows. An implementation that ignored the overlay
/// entirely passes this one alone — together they pin the overlay to
/// "applied exactly when set".
#[test]
fn an_unnamed_discovered_row_keeps_the_vendors_title() {
    let r = root("rename-none");
    let n = Node::open(r.clone()).unwrap();
    plant_discovered_claude(&r, "unnamed0001", "vendor-name");

    for _ in 0..2 {
        let row = n
            .sessions()
            .unwrap()
            .into_iter()
            .find(|v| v.session.id.0.starts_with("tui/"))
            .expect("the planted status file must surface as a row");
        assert_eq!(row.session.title, "vendor-name", "no override on record: the vendor's title stands");
    }
    let _ = fs::remove_dir_all(&r);
}

/// A khor-opened row renames the same way, and clearing the name falls
/// back to the title it was opened with — an empty title is the eraser,
/// not a name (`khor_sync::titles`). The rename also survives reopening
/// the node: the overlay is a document on disk, not process state.
#[test]
fn clearing_a_rename_falls_back_to_the_opened_title() {
    let r = root("rename-clear");
    let n = Node::open(r.clone()).unwrap();
    let id = n.open_ephemeral(khor_core::kind::TUI, "opened-title").unwrap();

    let title_of = |n: &Node| {
        n.sessions()
            .unwrap()
            .into_iter()
            .find(|v| v.session.id == id)
            .expect("the opened row must be on the list")
            .session
            .title
    };
    assert_eq!(title_of(&n), "opened-title", "control: the opening title shows first");

    n.rename_session(&id, "my words").unwrap();
    assert_eq!(title_of(&n), "my words");

    // A second open of the same store reads the same overlay — the
    // document is what a serve restart comes back to.
    drop(n);
    let n = Node::open(r.clone()).unwrap();
    assert_eq!(title_of(&n), "my words", "the rename must survive reopening the node");

    n.rename_session(&id, "").unwrap();
    assert_eq!(title_of(&n), "opened-title", "clearing must fall back to the opened title");
    let _ = fs::remove_dir_all(&r);
}

/// A khor-opened agent row's conversation is found through the vendor
/// leaf the hook recorded — khor's minted leaf names no file in
/// projects/, so without the bridge the transcript must NOT resolve
/// (that is the control: a lookup that found it anyway would mean the
/// bridge is decoration and this test tests nothing).
#[test]
fn a_hosted_rows_conversation_is_found_through_the_vendor_leaf() {
    let r = root("vendor-leaf");
    let n = Node::open(r.clone()).unwrap();
    let tui = n.open_ephemeral(khor_core::kind::TUI, "an agent").unwrap();

    // The vendor's transcript, named by the vendor's own session id.
    let uuid = "0a9f61c2-77aa-4bde-9c01-3e5f8a7b1234";
    let proj = crate::adaptor::vendor_home(&r).join(".claude").join("projects").join("p");
    fs::create_dir_all(&proj).unwrap();
    fs::write(
        proj.join(format!("{uuid}.jsonl")),
        concat!(
            r#"{"type":"user","message":{"role":"user","content":"the real words"}}"#,
            "\n"
        ),
    )
    .unwrap();

    // Control first: khor's own leaf must find nothing.
    assert!(
        n.transcript_of(&tui).is_err(),
        "without the bridge the khor mint must resolve no transcript"
    );

    // The hook's half, minus the env: record the vendor leaf, then the
    // same lookup lands on the vendor's file.
    let live = crate::live::LiveKind::new(r.clone(), n.device());
    live.learn_vendor_leaf(&tui, &crate::live::clean_leaf(uuid)).unwrap();
    let said = n.transcript_of(&tui).unwrap();
    assert_eq!(
        said,
        vec![crate::adaptor::claude::Utterance::User("the real words".into())],
        "the vendor leaf must lead to the vendor's own file"
    );
    let _ = fs::remove_dir_all(&r);
}

/// Machine pins are keyed by device id but spoken in machine names, and
/// an unknown name is refused the way every other verb refuses one.
#[test]
fn pinning_a_machine_that_is_not_in_the_table_is_refused_by_name() {
    let r = root("pin-machine");
    let n = Node::open(r.clone()).unwrap();
    n.pin_device(n.name().to_owned().as_str(), true).unwrap();
    assert!(n.devices().unwrap()[0].pinned, "this machine should be pinned now");

    let e = n.pin_device("no-such-box", true).unwrap_err();
    let probe = khor_catalog::msg::no_such_machine('\u{0}', '\u{0}');
    assert!(e.contains(probe.split('\u{0}').next().unwrap()), "{e}");
    let _ = fs::remove_dir_all(&r);
}

#[test]
fn telling_an_unknown_machine_is_refused_by_name() {
    let r = root("unknown");
    let n = Node::open(r.clone()).unwrap();
    let e = n.tell("no-such-box", "hi").unwrap_err();
    let probe = khor_catalog::msg::no_such_machine('\u{0}', '\u{0}');
    assert!(e.contains(probe.split('\u{0}').next().unwrap()), "{e}");
    assert!(e.contains(n.name()), "the error should name existing machines: {e}");
    let _ = fs::remove_dir_all(&r);
}

#[test]
fn a_wrong_session_id_names_what_exists() {
    let r = root("badid");
    let n = Node::open(r.clone()).unwrap();
    let e = n.seen(&SessionId("chat/elsewhere".into())).unwrap_err();
    assert!(e.contains("chat/"), "the error should carry the existing ids: {e}");
    let _ = fs::remove_dir_all(&r);
}

/// **A session khor mints for itself carries the full 64 bits.**
///
/// The width is not decoration: these ids key two network-wide tables
/// that hold no machine in the key (`khor_sync::pins` grades all four
/// kinds), so shortening the leaf silently raises the collision rate on
/// a machine nobody is looking at. The arithmetic is on
/// `link::LEAF_HEX`; this is the guard that makes changing it a decision
/// rather than an edit.
#[test]
fn a_minted_session_id_carries_its_full_width() {
    let r = root("mint");
    let n = Node::open(r.clone()).unwrap();

    let id = n.open_ephemeral(khor_core::kind::SHELL, "a title").unwrap();
    let leaf = id.0.split_once('/').expect("an id is <kind>/<leaf>").1;
    // **The literal, not `link::LEAF_HEX`.** Comparing the mint against
    // the constant that drives it passes no matter what the constant
    // says — both sides move together, and narrowing the id back to 32
    // bits was measured to leave this test green. The width is the fact
    // being guarded, so the number has to be written down twice.
    assert_eq!(
        leaf.len(),
        16,
        "a minted leaf must be 16 hex digits = 64 bits: {}",
        id.0
    );
    assert!(
        leaf.chars().all(|c| c.is_ascii_hexdigit()),
        "the leaf must be hex, or the bit count above means nothing: {leaf}"
    );

    // Two mints in a row must differ — a constant leaf would satisfy
    // every assertion above.
    let other = n.open_ephemeral(khor_core::kind::SHELL, "another").unwrap();
    assert_ne!(id.0, other.0, "two sessions must not share an id");

    let _ = fs::remove_dir_all(&r);
}

/// **A reported row's strip is its home machine's own record** (远端
/// 日条批). Before this, every reported row painted 没在看 for its
/// whole day — a peer's row is one report with an age, not a history —
/// and the history lives on the home machine, fetched with its rows
/// (`Request::Days`) and cached beside them.
///
/// Two peers, and the control is the one that never answered: it keeps
/// painting 没在看, because that is what this machine knows about it.
#[test]
fn a_reported_rows_strip_comes_from_its_home_machines_day() {
    let r = root("peer-day");
    let n = Node::open(r.clone()).unwrap();
    let told_id = "a".repeat(64);
    let mute_id = "b".repeat(64);
    {
        let loaded = n.devices_loaded().unwrap();
        loaded.doc.upsert(&told_id, "told", &[]).unwrap();
        loaded.doc.upsert(&mute_id, "mute", &[]).unwrap();
        let mut store = loaded.store;
        store.flush(&loaded.doc).unwrap();
    }
    let mut template = n.sessions().unwrap()[0].session.clone();
    template.id = SessionId("shell/on-told".into());
    template.home = crate::device_id_from_hex(&told_id).unwrap();
    n.cache_peer_rows(&told_id, "told", &[template.clone()]).unwrap();
    template.id = SessionId("shell/on-mute".into());
    template.home = crate::device_id_from_hex(&mute_id).unwrap();
    n.cache_peer_rows(&mute_id, "mute", &[template]).unwrap();

    // What `told` would have answered: its own record, with one 忙碌
    // minute on the row it reported.
    let now = crate::live::now_ms();
    let mut theirs = crate::day::Day::empty(now);
    theirs.observe(now, [("shell/on-told".to_owned(), State::Busy)]);
    n.cache_peer_day(&told_id, &theirs.to_wire()).unwrap();

    let strips = n.day_strips(crate::day::MINUTES).unwrap();
    let told = &strips["shell/on-told"];
    assert!(
        told.iter().any(|run| run.cell == khor_core::DayCell::Word(State::Busy)),
        "the home machine's word reaches the strip: {told:?}"
    );
    let mute = &strips["shell/on-mute"];
    assert!(
        mute.iter().all(|run| run.cell == khor_core::DayCell::Unwatched),
        "a machine that never answered is still 没在看, not borrowed: {mute:?}"
    );
}

/// The four classes of the fold ruling (退场批), enumerated — a "fold
/// everything" implementation passes the first alone, which is why the
/// other three exist.
#[test]
fn only_dead_seen_and_old_rows_are_stale() {
    let hours = |h: u64| h * 60 * 60 * 1000;
    let now = hours(1000);
    let row = |dead: bool, unread: u64, died_at: u64| Session {
        id: SessionId("shell/x".into()),
        kind: khor_core::Kind(khor_core::kind::SHELL.into()),
        title: "probe".into(),
        home: DeviceId([1; 32]),
        state: khor_core::StateStamp { state: State::Failed, at: khor_core::Millis(died_at) },
        unread,
        place: None,
        mux: false,
        dead,
        unbacked: false,
        category: None,
        last: None,
        via: None,
        term: false,
    };
    assert!(
        is_stale(&row(true, 0, now - hours(49)), false, now),
        "dead + looked at + 49h ago must fold"
    );
    assert!(
        !is_stale(&row(true, 1, now - hours(49)), false, now),
        "unread is a debt: an unread dead row must stay"
    );
    assert!(
        !is_stale(&row(true, 0, now - hours(47)), false, now),
        "dead but not yet 48h: too fresh to fold"
    );
    assert!(
        !is_stale(&row(false, 0, now - hours(400)), false, now),
        "alive rows never fold, however old their stamp"
    );
    assert!(
        !is_stale(&row(true, 0, now - hours(49)), true, now),
        "a pinned row never folds: the hand outranks the clock"
    );
}

/// The same judgment through the real pipeline: a khor-opened row whose
/// process exited long ago folds; one that exited just now does not.
/// The age is written into the row's own state file — the stamp the
/// judgment reads is the recorded death, no invented field.
#[test]
fn a_long_dead_row_is_stale_on_the_list_and_a_fresh_one_is_not() {
    let r = root("stale-rows");
    let n = Node::open(r.clone()).unwrap();
    let live = crate::live::LiveKind::new(r.clone(), n.device());
    let old = n.open_ephemeral(khor_core::kind::SHELL, "old-probe").unwrap();
    let fresh = n.open_ephemeral(khor_core::kind::SHELL, "fresh-probe").unwrap();
    live.record_exit(&old, 1).unwrap();
    live.record_exit(&fresh, 1).unwrap();

    // Push the old row's death 49 hours into the past, in its own
    // state file — the place the stamp actually comes from.
    let dir = n.session_dir(&old).unwrap();
    let text = fs::read_to_string(dir.join("state.json")).unwrap();
    let mut state: serde_json::Value = serde_json::from_str(&text).unwrap();
    let died = crate::live::now_ms() - 49 * 60 * 60 * 1000;
    state["at_ms"] = serde_json::json!(died);
    fs::write(dir.join("state.json"), serde_json::to_vec(&state).unwrap()).unwrap();

    let stale_of = |id: &SessionId| {
        n.sessions()
            .unwrap()
            .into_iter()
            .find(|v| v.session.id == *id)
            .expect("the row must still be on the list — folding is not deleting")
            .stale
    };
    assert!(stale_of(&old), "failed 49h ago and looked at: folds");
    assert!(!stale_of(&fresh), "failed just now: stays in plain sight");
    let _ = fs::remove_dir_all(&r);
}

/// **A live word with no voucher goes quiet after a day; a vouched one
/// never does** (词的保鲜期批). Three seats, one clock:
/// - the pid-less hook-observed row whose signal stopped — greys, and
///   its word stands exactly as it was (never a seventh word);
/// - the same-aged row with a live pid — never greys, however old the
///   stamp (the control against an age-only sweep);
/// - the same-aged pid-less row at 空闲 — never greys: a finished word
///   is honest at any age, only live words can be falsified by silence.
#[test]
fn an_unvouched_live_word_goes_quiet_and_a_vouched_one_never_does() {
    let r = root("quiet");
    let n = Node::open(r.clone()).unwrap();
    let live = crate::live::LiveKind::new(r.clone(), n.device());
    let orphan = n.open_ephemeral(khor_core::kind::TUI, "orphan-busy").unwrap();
    let backed = n.open_ephemeral(khor_core::kind::TUI, "backed-busy").unwrap();
    let idle = n.open_ephemeral(khor_core::kind::TUI, "orphan-idle").unwrap();
    // The hook shape: a word on record, no pid to ask (open_ephemeral
    // records the opener's pid; clear it the way a hook registration
    // never sets one — by rewriting meta with no pid).
    for id in [&orphan, &idle] {
        let dir = n.session_dir(id).unwrap();
        let meta = fs::read_to_string(dir.join("meta.json")).unwrap();
        let mut meta: serde_json::Value = serde_json::from_str(&meta).unwrap();
        meta["pid"] = serde_json::Value::Null;
        fs::write(dir.join("meta.json"), serde_json::to_vec(&meta).unwrap()).unwrap();
    }
    live.set_pid(&backed, std::process::id()).unwrap();
    live.report(&orphan, State::Busy, crate::live::Source::Reported).unwrap();
    live.report(&backed, State::Busy, crate::live::Source::Reported).unwrap();
    live.report(&idle, State::Idle, crate::live::Source::Reported).unwrap();
    // Push all three words 25 hours into the past.
    let then = crate::live::now_ms() - 25 * 60 * 60 * 1000;
    for id in [&orphan, &backed, &idle] {
        let dir = n.session_dir(id).unwrap();
        let text = fs::read_to_string(dir.join("state.json")).unwrap();
        let mut state: serde_json::Value = serde_json::from_str(&text).unwrap();
        state["at_ms"] = serde_json::json!(then);
        fs::write(dir.join("state.json"), serde_json::to_vec(&state).unwrap()).unwrap();
    }

    let view = |id: &SessionId| {
        n.sessions()
            .unwrap()
            .into_iter()
            .find(|v| v.session.id == *id)
            .expect("the row must be on the list")
    };
    let o = view(&orphan);
    assert!(o.quiet, "a day-silent 忙碌 nobody can vouch for must grey");
    assert_eq!(o.session.state.state, State::Busy, "…and the word itself never changes");
    assert!(
        !view(&backed).quiet,
        "a live pid is a voucher: the row never greys however old the stamp"
    );
    assert!(
        !view(&idle).quiet,
        "空闲 states a finished fact; silence cannot falsify it"
    );
    let _ = fs::remove_dir_all(&r);
}

/// `pid_alive` answers about this process, and about one that cannot
/// exist. The second half is the control: a function that simply
/// returned true would pass the first half alone.
#[test]
fn a_live_pid_reads_alive_and_an_impossible_one_does_not() {
    assert!(
        crate::link::pid_alive(std::process::id()),
        "this very process must read as alive"
    );
    // Above every configured pid_max on Linux and macOS, so nothing can
    // be wearing it.
    assert!(
        !crate::link::pid_alive(u32::MAX - 1),
        "a pid that cannot exist must not read as alive"
    );
}

#[cfg(test)]
mod store_owner_tests {
    /// A store carries the machine it belongs to, and says so when the
    /// wrong one opens it. Measured need: two boxes of this fleet share
    /// one NFS home, so `~/.khor` is literally the same directory on
    /// both — without this they would share one identity and quietly
    /// fight over one registry.
    #[test]
    fn a_store_belongs_to_the_machine_that_made_it() {
        let root = std::env::temp_dir().join(format!("khor-owner-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        // A fresh store gets stamped, and opening it again is fine.
        crate::Node::open(root.clone()).expect("a fresh store opens");
        let stamp = root.join(".khor").join("owner");
        let owner = std::fs::read_to_string(&stamp).unwrap();
        assert!(!owner.trim().is_empty(), "the store records its machine");
        crate::Node::open(root.clone()).expect("the same machine opens it again");

        // Somebody else's store is refused, and the refusal names both.
        std::fs::write(&stamp, "some-other-box").unwrap();
        let err = match crate::Node::open(root.clone()) {
            Err(e) => e,
            Ok(_) => panic!("another machine's store must be refused"),
        };
        assert!(err.contains("some-other-box"), "it names the owner: {err}");
        assert!(err.contains("KHOR_HOME"), "and what to do about it: {err}");

        // The name is a label, not the claim: renaming must not lock a
        // machine out of its own store.
        std::fs::write(&stamp, gethostname::gethostname().to_string_lossy().to_string()).unwrap();
        crate::Node::open_as(root.clone(), "renamed-machine").expect("a rename is not a move");

        let _ = std::fs::remove_dir_all(&root);
    }

    /// Which store a bare `khor` picks under a given home. The rule used
    /// to live in a line the installer appended to `~/.profile`, and on
    /// a shared home that file is one file for the whole fleet — the
    /// line said `.khor-hosts/$(hostname)` for everybody, including the
    /// machine that owns `$HOME`.
    ///
    /// Three arms, because only the middle one needs the rule: an empty
    /// home and a home this machine owns both answer `$HOME`, and an
    /// implementation that always answers `$HOME` passes those two.
    #[test]
    fn a_shared_home_sends_every_other_machine_to_its_own_store() {
        let home = std::env::temp_dir().join(format!("khor-under-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(home.join(".khor")).unwrap();
        let here = gethostname::gethostname().to_string_lossy().to_string();

        // Nobody has claimed it: a plain machine with a plain home.
        assert_eq!(crate::Node::store_under(&home), home, "an unclaimed home is the store");

        // This machine claimed it, so this machine keeps it.
        std::fs::write(home.join(".khor").join("owner"), &here).unwrap();
        assert_eq!(crate::Node::store_under(&home), home, "the owner keeps $HOME");

        // Somebody else claimed it: this machine gets its own, and the
        // path is the one `khor_root` in scripts/onset-lib.sh computes.
        std::fs::write(home.join(".khor").join("owner"), "some-other-box").unwrap();
        assert_eq!(
            crate::Node::store_under(&home),
            home.join(".khor-hosts").join(&here),
            "a shared home sends this machine to its own store"
        );

        // The `KHOR_HOME` override is deliberately not exercised here:
        // setting it would be a process-wide change in a suite that runs
        // in parallel, and `root_from_env` returns it before reading
        // anything on disk.

        let _ = std::fs::remove_dir_all(&home);
    }

    /// A rename must not cost a machine its store — the thing the old
    /// hostname stamp did, measured 2026-09-11 as exit 1 on turing.
    ///
    /// **Three arms, and only the middle one needs the fingerprint.** A
    /// store this machine just made, and one stamped with a name it no
    /// longer uses, must both open; a store belonging to somebody else
    /// must not. An implementation that opened everything passes the
    /// first two.
    #[test]
    fn renaming_a_machine_does_not_cost_it_its_store() {
        let root = std::env::temp_dir().join(format!("khor-rename-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        crate::Node::open(root.clone()).expect("a fresh store opens");
        let stamp = root.join(".khor").join("owner");

        let text = std::fs::read_to_string(&stamp).unwrap();
        let Some(fp) = crate::hardware::fingerprint() else {
            // No fingerprint here, so there is no world in which this
            // gate can fire — and a gate with no such world must not
            // sit in a green list pretending (CLAUDE.md 假绿 14).
            eprintln!("skipped: this platform will not fingerprint itself");
            let _ = std::fs::remove_dir_all(&root);
            return;
        };
        assert!(text.contains(&fp), "the stamp records the box, not just the name: {text:?}");

        // Renamed: same box, a name it has never gone by.
        std::fs::write(&stamp, format!("machine {fp}\nname the-old-name\n")).unwrap();
        crate::Node::open(root.clone()).expect("a renamed machine still opens its own store");
        let after = std::fs::read_to_string(&stamp).unwrap();
        assert!(
            !after.contains("the-old-name"),
            "and the stamp catches up to the new name: {after:?}"
        );

        // Somebody else's box, whatever it is called.
        std::fs::write(&stamp, "machine 00000000000000000000000000000000\nname elsewhere\n").unwrap();
        let err = match crate::Node::open(root.clone()) {
            Err(e) => e,
            Ok(_) => panic!("another box's store is still refused"),
        };
        assert!(err.contains("elsewhere"), "and the refusal names it: {err}");

        // The format every store on disk before today is in.
        std::fs::write(&stamp, gethostname::gethostname().to_string_lossy().to_string()).unwrap();
        crate::Node::open(root.clone()).expect("a bare-hostname stamp still opens");
        let upgraded = std::fs::read_to_string(&stamp).unwrap();
        assert!(upgraded.contains(&fp), "and gets its fingerprint: {upgraded:?}");

        let _ = std::fs::remove_dir_all(&root);
    }

    /// The side room is found by what is stamped inside it, not by what
    /// is written on the door — a renamed machine that recomputed the
    /// path would land in an empty directory and mint a second identity,
    /// which is the failure this whole batch is about.
    #[test]
    fn a_renamed_machine_finds_the_side_room_it_already_had() {
        let Some(fp) = crate::hardware::fingerprint() else {
            eprintln!("skipped: this platform will not fingerprint itself");
            return;
        };
        let home = std::env::temp_dir().join(format!("khor-room-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        let here = gethostname::gethostname().to_string_lossy().to_string();

        // A home somebody else owns, so the side rooms are in play.
        std::fs::create_dir_all(home.join(".khor")).unwrap();
        std::fs::write(
            home.join(".khor").join("owner"),
            "machine ffffffffffffffffffffffffffffffff\nname the-owner\n",
        )
        .unwrap();

        // No room yet: the fallback is the door with this machine's
        // current name on it.
        assert_eq!(
            crate::Node::store_under(&home),
            home.join(".khor-hosts").join(&here),
            "with no room of its own, a machine takes the one named after it"
        );

        // A room made before the rename, plus a decoy that is nobody's.
        let old = home.join(".khor-hosts").join("what-it-used-to-be-called");
        std::fs::create_dir_all(old.join(".khor")).unwrap();
        std::fs::write(old.join(".khor").join("owner"), format!("machine {fp}\nname what-it-used-to-be-called\n")).unwrap();
        let decoy = home.join(".khor-hosts").join("aaa-somebody-else");
        std::fs::create_dir_all(decoy.join(".khor")).unwrap();
        std::fs::write(decoy.join(".khor").join("owner"), "machine 11111111111111111111111111111111\nname other\n").unwrap();
        // Sorted first of the three, and blank — the walk must not read
        // "nothing" as "yours".
        let blank = home.join(".khor-hosts").join("aaa-blank");
        std::fs::create_dir_all(blank.join(".khor")).unwrap();
        std::fs::write(blank.join(".khor").join("owner"), "").unwrap();

        assert_eq!(
            crate::Node::store_under(&home),
            old,
            "the room is the one stamped with this box, whatever the door says"
        );

        let _ = std::fs::remove_dir_all(&home);
    }

    /// The channel is settled once, so renaming the host does not open
    /// a second chat window and strand the first one's watermark.
    #[test]
    fn the_channel_a_store_joined_under_outlives_a_rename() {
        let root = std::env::temp_dir().join(format!("khor-chan-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        let first = crate::Node::open_as(root.clone(), "before").expect("opens");
        assert_eq!(first.name(), "before");
        drop(first);

        // Same store, a machine now calling itself something else.
        let after = crate::Node::open_as(root.clone(), "after").expect("opens again");
        assert_eq!(
            after.name(),
            "before",
            "the channel is what the store joined under, not what the host says today"
        );

        let _ = std::fs::remove_dir_all(&root);
    }
}

#[cfg(test)]
mod self_exe_tests {
    /// The ordinary case answers the ordinary thing, and the upgraded
    /// case is what the doc comment describes. Only the first half can
    /// be exercised in-process (a test cannot delete its own binary
    /// without breaking the harness); the parsing half is asserted on
    /// the string shape Linux actually produces, which is the part that
    /// silently returned a path nothing could spawn.
    #[test]
    fn a_live_binary_answers_its_own_path() {
        let me = crate::self_exe().expect("a running test binary has a path");
        assert!(me.exists(), "and it is a file that exists: {}", me.display());
        assert!(
            !me.to_string_lossy().ends_with(" (deleted)"),
            "never the suffix Linux appends to a replaced binary"
        );
    }
}

/// "Where was I?" at the node's surface (`khor_sync::recents`). The
/// document's own laws are pinned in that module; what is pinned here is
/// the surface's share of them — that the two kinds are two lists, that
/// the cap is honoured, and that the machine and the path are checked
/// before anything is written.
#[cfg(test)]
mod recents_tests {
    use std::fs;

    use crate::{Node, PlaceKind};

    fn root(tag: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("khor-node-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    /// **The list grows by places, not by trips.** Opening the same
    /// directory all morning must leave one row — a list that counted
    /// visits would be a log, and the newest thing in it would be
    /// wherever the person has been sitting longest.
    #[test]
    fn going_back_to_a_place_does_not_lengthen_the_list() {
        let r = root("recents-revisit");
        let n = Node::open(r.clone()).unwrap();
        let me = n.name().to_owned();

        for _ in 0..3 {
            n.recent_visit(PlaceKind::Dir, &me, "/Users/x/proj").unwrap();
            std::thread::sleep(std::time::Duration::from_millis(3));
        }
        assert_eq!(n.recents(PlaceKind::Dir, 10).unwrap().len(), 1, "three trips, one place");

        n.recent_visit(PlaceKind::Dir, &me, "/Users/x/other").unwrap();
        assert_eq!(n.recents(PlaceKind::Dir, 10).unwrap().len(), 2, "a second place is what makes it two");
        let _ = fs::remove_dir_all(&r);
    }

    /// Written in one order, read in the other, and cut to the cap.
    /// The distinctness check above the ordering one is what keeps this
    /// from passing on a clock that never moved — with three equal
    /// stamps the order would be the tiebreak's, and this test would be
    /// asserting something it is not about.
    #[test]
    fn the_list_reads_most_recent_first_and_stops_at_the_cap() {
        let r = root("recents-order");
        let n = Node::open(r.clone()).unwrap();
        let me = n.name().to_owned();

        for p in ["/first", "/second", "/third"] {
            n.recent_visit(PlaceKind::Dir, &me, p).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(3));
        }

        let all = n.recents(PlaceKind::Dir, 10).unwrap();
        let stamps: Vec<i64> = all.iter().map(|p| p.at_ms).collect();
        assert!(
            stamps.windows(2).all(|w| w[0] > w[1]),
            "the clock must have moved between visits or this test is about the tiebreak: {stamps:?}"
        );
        let places: Vec<&str> = all.iter().map(|p| p.place.as_str()).collect();
        assert_eq!(places, vec!["/third", "/second", "/first"], "most recent first");
        assert!(all.iter().all(|p| p.machine == n.device_str()), "each place names whose disk it is on");

        let two = n.recents(PlaceKind::Dir, 2).unwrap();
        assert_eq!(
            two.iter().map(|p| p.place.as_str()).collect::<Vec<_>>(),
            vec!["/third", "/second"],
            "the cap keeps the recent end, not the old one"
        );
        assert!(n.recents(PlaceKind::Dir, 0).unwrap().is_empty(), "a cap of none is none");
        let _ = fs::remove_dir_all(&r);
    }

    /// Two landings, two lists. A page must never surface where
    /// directories are offered — they are one table told apart by a
    /// word in the key, and a reader that ignored the word would pass
    /// every other test in this module.
    #[test]
    fn directories_and_pages_never_show_up_in_each_others_lists() {
        let r = root("recents-kinds");
        let n = Node::open(r.clone()).unwrap();
        let me = n.name().to_owned();

        n.recent_visit(PlaceKind::Dir, &me, "/Users/x/proj").unwrap();
        n.recent_visit(PlaceKind::Page, &me, "https://example.com/read").unwrap();

        let dirs = n.recents(PlaceKind::Dir, 10).unwrap();
        let pages = n.recents(PlaceKind::Page, 10).unwrap();
        assert_eq!(dirs.len(), 1, "one directory: {dirs:?}");
        assert_eq!(dirs[0].place, "/Users/x/proj");
        assert_eq!(pages.len(), 1, "one page: {pages:?}");
        assert_eq!(pages[0].place, "https://example.com/read");
        let _ = fs::remove_dir_all(&r);
    }

    /// Dismissing takes a place off the list; going back puts it back.
    /// The second half is the one that says the tombstone means "stop
    /// suggesting this" rather than "this place is gone".
    #[test]
    fn a_dismissed_place_leaves_the_list_and_a_new_visit_brings_it_back() {
        let r = root("recents-forget");
        let n = Node::open(r.clone()).unwrap();
        let me = n.name().to_owned();

        n.recent_visit(PlaceKind::Page, &me, "https://news.example").unwrap();
        n.recent_visit(PlaceKind::Page, &me, "https://kept.example").unwrap();
        assert_eq!(n.recents(PlaceKind::Page, 10).unwrap().len(), 2, "control: both are on the list");

        n.forget_recent(PlaceKind::Page, &me, "https://news.example").unwrap();
        let left = n.recents(PlaceKind::Page, 10).unwrap();
        assert_eq!(
            left.iter().map(|p| p.place.as_str()).collect::<Vec<_>>(),
            vec!["https://kept.example"],
            "the dismissed one is off the list and the other is untouched"
        );

        std::thread::sleep(std::time::Duration::from_millis(3));
        n.recent_visit(PlaceKind::Page, &me, "https://news.example").unwrap();
        assert_eq!(
            n.recents(PlaceKind::Page, 10).unwrap()[0].place,
            "https://news.example",
            "going back there must put it back, at the top"
        );
        let _ = fs::remove_dir_all(&r);
    }

    /// The record is a document on disk, not process state: a serve
    /// restart, or the CLI running beside the app, comes back to the
    /// same list.
    #[test]
    fn the_list_survives_reopening_the_node() {
        let r = root("recents-reopen");
        let n = Node::open(r.clone()).unwrap();
        let me = n.name().to_owned();
        n.recent_visit(PlaceKind::Dir, &me, "/Users/x/proj").unwrap();
        drop(n);

        let n = Node::open(r.clone()).unwrap();
        assert_eq!(
            n.recents(PlaceKind::Dir, 10).unwrap()[0].place,
            "/Users/x/proj",
            "the record must outlive the process that made it"
        );
        let _ = fs::remove_dir_all(&r);
    }

    /// A relative path names nowhere, and a machine nobody has heard of
    /// names nobody: both are refused before anything is written, or the
    /// list would offer somewhere that cannot be opened. The control
    /// above each refusal is what says the refusal is about the argument
    /// and not about recording visits at all.
    #[test]
    fn a_place_nobody_can_reach_is_refused_rather_than_recorded() {
        let r = root("recents-refuse");
        let n = Node::open(r.clone()).unwrap();
        let me = n.name().to_owned();

        n.recent_visit(PlaceKind::Dir, &me, "/Users/x/proj").expect("control: an absolute path is fine");
        assert!(
            n.recent_visit(PlaceKind::Dir, &me, "proj/sub").is_err(),
            "a relative directory names nowhere"
        );
        assert!(
            n.recent_visit(PlaceKind::Dir, "no-such-machine", "/Users/x/proj").is_err(),
            "a machine not in the table names nobody"
        );
        assert_eq!(
            n.recents(PlaceKind::Dir, 10).unwrap().len(),
            1,
            "…and neither refusal may have written a row"
        );

        // A URL is not a path, so the absolute-path rule must not reach
        // pages — it would refuse every page there is.
        n.recent_visit(PlaceKind::Page, &me, "https://example.com/read")
            .expect("a page is not judged by the path rule");
        let _ = fs::remove_dir_all(&r);
    }
}

/// **The file chip's 空闲, measured.** A chip is the per-message twin of
/// the row's word (`TransferKind::message_face`), and its vocabulary
/// includes 看过了 — `receiver_face` computes 完成 vs 空闲 for a landed
/// payload. This walks the same disk states the row is walked over, and
/// then does the one thing the row-level walk never does: **looks at
/// the row**. Every assertion before the last is a control establishing
/// that the probe reads real, changing values off this seat.
#[test]
fn a_landed_file_chip_stops_saying_done_once_the_row_is_looked_at() {
    let r = root("chip-seen");
    let n = Node::open(r.clone()).unwrap();
    let dir = channel_dir(&r, n.name()).unwrap();
    fs::create_dir_all(&dir).unwrap();

    let far = ChatDoc::new(0xFAFA).unwrap();
    let who = Sender { id: "dev-far".into(), name: "far".into() };
    let f = khor_node_test_fileref();
    far.send_files(&who, std::slice::from_ref(&f)).unwrap();
    fs::write(
        dir.join("u-000000000000fafa-00000000.loro"),
        far.changes_since(&Default::default()).unwrap(),
    )
    .unwrap();

    let chip = |n: &Node| n.log_faced(n.name()).unwrap().1[0].0;
    assert_eq!(
        chip(&n),
        Some(State::Blocked),
        "control: nothing has landed, and this seat is the one being asked"
    );

    fs::create_dir_all(dir.join("files")).unwrap();
    fs::write(crate::transfer::payload_path(&dir, &f), b"bytes!").unwrap();
    assert_eq!(
        chip(&n),
        Some(State::Done),
        "control: it landed and nobody has looked — the probe moves with the disk"
    );

    let id = SessionId(format!("chat/{}", n.name()));
    n.seen(&id).unwrap();
    assert_eq!(
        n.sessions().unwrap()[0].session.unread,
        0,
        "control: the row itself counts as looked at, so the chip is the only thing left to ask"
    );
    assert_eq!(chip(&n), Some(State::Idle), "看过了 — a chip may not keep shouting 完成 forever");
    let _ = fs::remove_dir_all(&r);
}

/// **A line in the conversation covers the file that came before it,
/// and that is a decision, not a leak.**
///
/// `tell` raises the watermark to its own stamp and the mark replicates,
/// so a line the far machine sends after a file sweeps that file's
/// unread with it. A watermark is a point in time; it structurally
/// cannot say "everything before T except that one file". The question
/// this test freezes is whether it should have to.
///
/// **It should not.** 角标数的是「有几件事想要你」(docs/UX.md). A file
/// that has landed, in a conversation the person has since spoken in,
/// **wants nothing**: it sits on that machine, they can fetch it
/// whenever, nothing is stuck and nobody is waiting on an answer. That
/// is a *fact*, and this codebase already sorted facts from demands —
/// 看了也不清的事实（失败沉底）占列表，不占角标.
///
/// And the alternative breaks something harder. To make the file stay
/// owed, its "looked at" would have to be a per-machine fact that does
/// **not** replicate — and then the badge on the desk could only be
/// cleared by walking to the desk. 角标可归零 does not mean "some way
/// exists to clear it"; it means **答了清、看了清**. A badge whose only
/// answer is "be somewhere else" is a badge that stays non-zero for as
/// long as the person is away, and a badge that is never zero is worth
/// nothing. It would also mean rewriting what 已读 *is*: docs/NET.md
/// says 看过是决定：手机看了，桌面角标同步清 — network-wide, one copy.
///
/// **Where the earlier reasoning stops.** A file message's *arrival* is
/// a fact about a machine — that is why `Node::send` marks nothing and
/// why the offering seat does not badge itself. But that reasoning
/// governs **before the landing**, and before the landing there is
/// already a signal that owes nothing to the watermark: 待批, from the
/// seat whose approval is wanted. The assertions below hold that line:
/// the person speaking clears the *unread*, and does not touch 待批.
///
/// **The counter-argument, kept on the record**: the file is on the
/// desk, the person only ever looked on the phone, and so they may
/// never notice it arrived. **Trigger**: someone actually reports "I
/// didn't notice the file had landed on that machine." That day the
/// thing to change is not this test — it is docs/NET.md's 已读 line,
/// and that is a 改口 and wants a 墓碑.
#[test]
fn a_later_line_covers_the_file_before_it_and_that_is_deliberate() {
    let r = root("line-covers-file");
    let n = Node::open(r.clone()).unwrap();
    let dir = channel_dir(&r, n.name()).unwrap();
    fs::create_dir_all(&dir).unwrap();

    let far = ChatDoc::new(0xFAFA).unwrap();
    let who = Sender { id: "dev-far".into(), name: "far".into() };
    let f = khor_node_test_fileref();
    far.send_files(&who, std::slice::from_ref(&f)).unwrap();
    fs::write(
        dir.join("u-000000000000fafa-00000000.loro"),
        far.changes_since(&Default::default()).unwrap(),
    )
    .unwrap();
    assert_eq!(
        n.sessions().unwrap()[0].session.unread,
        1,
        "control: the offer is owed before anyone speaks — so the 0 below is a change, not a blind probe"
    );

    // The far machine says something afterwards. Its `tell` marks seen
    // at its own stamp and that mark replicates: the half of 说话即看过
    // that is right and stays (the person typed the line).
    std::thread::sleep(std::time::Duration::from_millis(5));
    far.tell(&who, "did you get it?").unwrap();
    let said_at = far.messages().last().unwrap().at;
    fs::write(
        dir.join("u-000000000000fafa-00000001.loro"),
        far.changes_since(&Default::default()).unwrap(),
    )
    .unwrap();
    let far_seen = SeenDoc::new(0xFAFA).unwrap();
    far_seen.mark(&format!("chat/{}", n.name()), said_at).unwrap();
    let sdir = seen_dir(&r);
    fs::create_dir_all(&sdir).unwrap();
    fs::write(
        sdir.join("u-000000000000fafa-00000000.loro"),
        far_seen.changes_since(&Default::default()).unwrap(),
    )
    .unwrap();
    assert_eq!(
        n.log(n.name()).unwrap().messages.len(),
        2,
        "control: both the offer and the line really are here"
    );

    let row = n.sessions().unwrap()[0].session.clone();
    assert_eq!(
        row.unread, 0,
        "the person has spoken in this conversation: nothing here is waiting to be read"
    );

    // **And the boundary**: speaking is not answering. The approval this
    // machine owes is untouched by anything said in the window — 答了才
    // 清, and only the landing answers it (docs/SESSION.md 六词).
    assert_eq!(
        row.state.state,
        State::Blocked,
        "待批 is not covered by a line: the file still wants THIS machine's approval"
    );

    // Once it does land, the chip agrees with the row rather than
    // arguing with it — one rule for 看过没看过, not two.
    fs::create_dir_all(dir.join("files")).unwrap();
    fs::write(crate::transfer::payload_path(&dir, &f), b"bytes!").unwrap();
    let chip = n.log_faced(n.name()).unwrap().1[0].0;
    assert_eq!(chip, Some(State::Idle), "the chip reads the same watermark the row does");
    assert_eq!(
        n.sessions().unwrap()[0].session.state.state,
        State::Idle,
        "…and the row, with nothing pressing and nothing owed, is 空闲"
    );
    let _ = fs::remove_dir_all(&r);
}

/// **The `PATH` a GUI process gets is not the one a person has**, and
/// khor now says so by name instead of letting the child die with
/// nothing to report.
///
/// The control is the one that found the bug on 2026-08-31: the same
/// binary, the same agent, the same machine, only the `PATH` differing
/// — from a terminal it listed 85 conversations, and from `khor serve`
/// (a launchd agent, `PATH=/usr/bin:/bin:/usr/sbin:/sbin`) it said
/// **"the agent ended before it answered"**, which is a sentence about
/// the agent and the agent was fine.
mod finding_the_program {
    use crate::{AgentSpec, Node};

    fn spec(command: &str) -> AgentSpec {
        AgentSpec { command: command.into(), args: vec!["-y".into(), "some-acp@1.0.0".into()] }
    }

    /// The two halves in one test, because the claim is a *difference*:
    /// a `PATH` that has the program and one that does not, asked the
    /// same question. Either half alone is satisfied by a function
    /// that always answers the same way.
    #[test]
    fn a_program_off_the_path_is_refused_by_name_and_one_on_it_is_not() {
        // `sh` stands in for `npx`: it is on every unix, in a directory
        // that is on the minimal PATH, so this half cannot fail for
        // being about this machine.
        let ok = Node::launch_on(&spec("sh"), "/usr/bin:/bin")
            .expect("sh is on /bin");
        assert!(ok.contains("/bin/sh"), "the command is resolved, not left as a bare name: {ok}");

        let why = Node::launch_on(&spec("npx"), "/usr/bin:/bin:/usr/sbin:/sbin")
            .expect_err("launchd's PATH has no npx");
        // **The name of the program has to be in the sentence.** That
        // is the whole fix: without it the message is about the agent,
        // and a person goes and checks an agent that is not broken.
        assert!(why.contains("npx"), "the refusal must name what was not found: {why}");
        // …and where khor looked, which is the other half of the next
        // step: install it, or point khor somewhere else.
        assert!(why.contains("/usr/sbin"), "the refusal must say where it looked: {why}");
    }

    /// The launch carries the `PATH` onward, or the child finds the
    /// program and then cannot find what *it* needs — `npx` without
    /// `node` beside it is the same failure one level down.
    #[test]
    fn the_launch_hands_the_path_to_the_child() {
        let out = Node::launch_on(&spec("sh"), "/usr/bin:/bin").unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["env"]["PATH"], "/usr/bin:/bin");
        assert_eq!(v["args"][1], "some-acp@1.0.0", "the arguments survive: {out}");
    }

    /// **Executable, not merely present.** A directory named like the
    /// program, or a data file, would otherwise resolve and push the
    /// failure back to a child that dies with nothing to say.
    #[test]
    fn something_that_is_not_a_program_does_not_count_as_finding_one() {
        let dir = std::env::temp_dir().join(format!("khor-path-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("npx")).unwrap();
        std::fs::write(dir.join("data"), b"not a program").unwrap();
        let here = dir.display().to_string();

        assert!(Node::launch_on(&spec("npx"), &here).is_err(), "a directory is not a program");
        assert!(Node::launch_on(&spec("data"), &here).is_err(), "a data file is not a program");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// **The shell's answer is added to what this process had, never
    /// swapped for it.** A login shell that answers with something
    /// short must not be able to take away directories that were
    /// already working — that is this bug pointing the other way.
    #[test]
    fn the_probed_path_keeps_what_the_process_already_had() {
        // **The merge itself is called, not re-written here.** The
        // first draft of this test built the union in the test body
        // and compared it to itself — true, green, and asserting
        // nothing about khor. `user_path` is a `OnceLock` around a
        // login shell, so the part worth testing is split out.
        assert_eq!(
            crate::merge_paths("/opt/khor-test-bin", "/usr/bin:/bin"),
            "/opt/khor-test-bin:/usr/bin:/bin",
            "the shell's answer leads and what was here follows"
        );
        assert_eq!(
            crate::merge_paths("", "/usr/bin:/bin"),
            "/usr/bin:/bin",
            "a shell that answered nothing takes nothing away"
        );
        assert_eq!(
            crate::merge_paths("/bin:/usr/bin", "/usr/bin:/bin"),
            "/bin:/usr/bin",
            "and a directory named twice appears once"
        );
    }

    /// **The line handed to the login shell is actually run here**,
    /// against a shell with a known environment, and its answer is
    /// parsed by the same function the real probe uses.
    ///
    /// This is the test the first draft did not have, and the thing it
    /// caught is worth stating: the separator had been spelled `"\0"`
    /// in Rust — a NUL *character* in the argument vector, where
    /// `printf` needs the two characters `\` and `0`. `Command::spawn`
    /// refuses an argument containing NUL, `asked_shell`'s error arm is
    /// an empty answer, and an empty answer is indistinguishable from
    /// "your shell had nothing to add". **The probe became a no-op and
    /// nothing anywhere went red** — `merge_paths` still returned a
    /// perfectly good `PATH`, the one that was already there.
    ///
    /// So this test does the one thing the unit tests around it cannot:
    /// it runs the string.
    #[test]
    fn the_line_the_login_shell_is_given_answers_with_all_four_fields() {
        let out = std::process::Command::new("/bin/sh")
            .args(["-c", &crate::probe_script()])
            .env("PATH", "/khor-test/bin")
            .env("LC_ALL", "")
            .env("LC_CTYPE", "")
            .env("LANG", "zh_CN.UTF-8")
            .output()
            .expect("the probe line must be runnable at all");
        assert!(out.status.success(), "the probe line must not error");
        let said = String::from_utf8_lossy(&out.stdout);
        assert_eq!(
            said.matches('\0').count(),
            4,
            "one NUL per field, so a value holding anything still parses: {said:?}"
        );
        let asked = crate::read_shell_answer(&said);
        assert_eq!(asked.path, "/khor-test/bin", "the PATH must survive the round trip");
        assert_eq!(
            asked.locale,
            vec![("LANG", "zh_CN.UTF-8".to_owned())],
            "and only the locale variables that actually name UTF-8"
        );
    }

    /// A shell that answered with a locale that is **not** UTF-8 must
    /// add nothing: khor's only business here is the encoding, and
    /// carrying a non-UTF-8 locale onto a child could only take away a
    /// UTF-8 one that was already in place.
    #[test]
    fn a_locale_that_does_not_name_utf8_is_not_carried() {
        let asked = crate::read_shell_answer("/bin\0\0\0en_US.ISO8859-1\0");
        assert_eq!(asked.path, "/bin");
        assert!(asked.locale.is_empty(), "got {:?}", asked.locale);

        // …and the three spellings that are all really UTF-8.
        for spelling in ["zh_CN.UTF-8", "en_US.utf8", "UTF-8"] {
            assert!(crate::says_utf8(spelling), "{spelling} names UTF-8");
        }
        assert!(!crate::says_utf8("C"), "and a locale that does not, does not");
        assert!(!crate::says_utf8(""), "nor does an empty one");
    }

    /// **Asking the login shell is a repair, not a policy.** It costs
    /// 0.78s and it runs the person's rc files, so a khor whose own
    /// `PATH` can answer must never pay it — and every khor started
    /// from a terminal is one of those. Only the one launchd started,
    /// whose four directories hold neither `tmux` nor `claude`, goes
    /// past this line.
    #[test]
    fn the_login_shell_is_only_asked_when_this_process_cannot_answer() {
        assert!(
            !crate::repair_needed("sh", "/usr/bin:/bin"),
            "a PATH that can find the program is the answer, and no shell is run"
        );
        assert!(
            crate::repair_needed("khor-no-such-program", "/usr/bin:/bin"),
            "and one that cannot is the only case that goes further"
        );
    }
}

/// **A codex conversation is followed, not read once and frozen.**
///
/// The claim is about the *dispatch*, which is a different claim from
/// "codex's reader can seek" — that one has its own tests beside the
/// reader (`adaptor::codex::following`). This is the one those cannot
/// make: that `transcript_follow` actually asks codex.
///
/// The hole it guards is the one this repo keeps finding: a reader that
/// works perfectly and is never called. Taking the `or_else` out of
/// `transcript_follow` leaves every test beside the reader green and
/// reds only this one — the pane goes back to `following: false`, which
/// on screen is a live conversation painted once and then still, with
/// nothing saying it stopped being read.
#[test]
fn a_codex_conversation_can_be_followed_and_not_merely_read_once() {
    let r = root("codex-follow");
    let n = Node::open(r.clone()).unwrap();
    let tui = n.open_ephemeral(khor_core::kind::TUI, "a codex").unwrap();

    let uuid = "019fd755-4444-7aaa-8aaa-aaaaaaaaaaaa";
    let day = crate::adaptor::vendor_home(&r)
        .join(".codex")
        .join("sessions")
        .join("2026")
        .join("09")
        .join("01");
    fs::create_dir_all(&day).unwrap();
    let roll = day.join(format!("rollout-2026-09-01T00-00-00-{uuid}.jsonl"));
    let line = |m: &str| {
        format!(r#"{{"type":"event_msg","payload":{{"type":"agent_message","message":"{m}"}}}}"#)
    };
    fs::write(&roll, line("first") + "\n").unwrap();

    let live = crate::live::LiveKind::new(r.clone(), n.device());
    live.learn_vendor_leaf(&tui, &crate::live::clean_leaf(uuid)).unwrap();

    let one = n.transcript_follow(&tui, 0).unwrap();
    assert!(
        one.following,
        "a codex rollout is seekable, so the pane must be told to keep asking"
    );
    assert!(one.at > 0, "and told where to continue from");
    assert_eq!(
        one.said,
        vec![crate::adaptor::claude::Utterance::Agent("first".into())],
    );

    // What "followed" means, and the half a `following: true` alone
    // would not prove: the second ask brings back what was said after
    // the first, and **only** that.
    let mut f = fs::OpenOptions::new().append(true).open(&roll).unwrap();
    use std::io::Write;
    writeln!(f, "{}", line("second")).unwrap();
    drop(f);
    let two = n.transcript_follow(&tui, one.at).unwrap();
    assert_eq!(
        two.said,
        vec![crate::adaptor::claude::Utterance::Agent("second".into())],
        "the tail, not the whole record again"
    );
    assert!(!two.rewound, "a growing file is appended to, not repainted");

    let _ = fs::remove_dir_all(&r);
}

/// **A workspace whose machine has left the device table is still a
/// row, and the row still comes off** (工作区批①). This is the shape
/// `khor forget` leaves behind: the table keeps the key, the device
/// table no longer has the machine, and the list has to answer with
/// something a person can act on.
///
/// The second half is the bug this was written for. Such a row has **no
/// name**, an empty machine name means 「this one」 to every verb here,
/// and a write routed through a name therefore lands on a key about
/// *this* machine's disk — the same directory on two Macs is two rows,
/// and one of them would be quietly rewritten. So the write is keyed by
/// the id, and this proves the name road does not reach it.
#[test]
fn a_workspace_whose_machine_left_the_table_keeps_its_row_and_comes_off_by_id() {
    let r = root("ws-ghost");
    let n = Node::open(r.clone()).unwrap();
    let ghost = "c".repeat(64);
    let path = "/srv/gone/project";
    {
        let loaded = n.workspaces_loaded().unwrap();
        loaded
            .doc
            .set(&khor_sync::workspaces::key(&ghost, path), khor_sync::workspaces::State::Yes)
            .unwrap();
        let mut store = loaded.store;
        store.flush(&loaded.doc).unwrap();
    }

    let rows = n.workspaces().unwrap();
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0].device, ghost);
    assert_eq!(rows[0].machine, "", "a machine that left the table has no name to print");
    assert_eq!(rows[0].name, "project", "…and the row is still named after its directory");

    // The name road cannot reach it, and must not pretend to: an empty
    // name is this machine, so this writes an entirely different key.
    n.ws_rm(&n.machine_id("").unwrap(), path).unwrap();
    assert_eq!(n.workspaces().unwrap().len(), 1, "the ghost row is not this machine's to remove");

    // The id road does, which is why it is the one the face and the CLI
    // both take.
    assert_eq!(n.machine_id(&ghost).unwrap(), ghost, "an id typed for a name is taken as itself");
    n.ws_rm(&ghost, path).unwrap();
    assert!(n.workspaces().unwrap().is_empty(), "the row it removed was the one shown");

    let _ = fs::remove_dir_all(&r);
}
