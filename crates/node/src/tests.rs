use std::fs;
use std::path::PathBuf;

use khor_core::State;
use khor_sync::chat::{channel_dir, ChatDoc, Sender};

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

/// Close deletes received payloads, never history: the history is
/// network-replicated and a local delete would be pulled right back.
#[test]
fn closing_a_chat_deletes_its_files_but_not_the_history() {
    let r = root("close");
    let n = Node::open(r.clone()).unwrap();
    n.tell(n.name().to_owned().as_str(), "the line that stays").unwrap();
    let dir = channel_dir(&r, n.name()).unwrap();
    let files = dir.join("files");
    fs::create_dir_all(&files).unwrap();
    fs::write(files.join("payload.bin"), b"received bytes").unwrap();

    let id = n.sessions().unwrap()[0].session.id.clone();
    n.close(&id).unwrap();
    assert!(!files.exists(), "received payloads should be gone");
    let log = n.log(n.name().to_owned().as_str()).unwrap();
    assert_eq!(log.messages.len(), 1, "the history must survive close");
    let row = &n.sessions().unwrap()[0].session;
    assert_eq!((row.unread, row.state.state), (0, State::Idle));
    let _ = fs::remove_dir_all(&r);
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
