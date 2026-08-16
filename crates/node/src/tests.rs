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
