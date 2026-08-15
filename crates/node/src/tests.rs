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
    n.tell(n.name().to_owned().as_str(), "记给自己的一句").unwrap();

    let rows = n.sessions().unwrap();
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row.title, n.name());
    assert_eq!(row.home, n.device());
    assert_eq!(row.unread, 0, "自己说的话不算未读");
    assert_eq!(row.state.state, State::Idle);
    assert!(row.state.at.0 > 0, "时间戳该是最后一条消息的");

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
    far.tell(&Sender { id: "dev-far".into(), name: "far".into() }, "从远端来的")
        .unwrap();
    let block = far.changes_since(&Default::default()).unwrap();
    let dir = channel_dir(&r, n.name()).unwrap();
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("u-000000000000f0f0-00000000.loro"), &block).unwrap();

    let row = &n.sessions().unwrap()[0];
    assert_eq!(row.unread, 1, "远端那句该算未读");
    assert_eq!(row.state.state, State::Done);

    n.seen(&row.id).unwrap();
    let row = &n.sessions().unwrap()[0];
    assert_eq!(row.unread, 0);
    assert_eq!(row.state.state, State::Idle);
    let _ = fs::remove_dir_all(&r);
}

#[test]
fn watch_receives_what_tell_emits() {
    let r = root("watch");
    let n = Node::open(r.clone()).unwrap();
    let rx = n.watch();
    n.tell(n.name().to_owned().as_str(), "推一条").unwrap();

    let first = rx.try_recv().expect("该有事件");
    let second = rx.try_recv().expect("该有行更新");
    assert!(matches!(first, NodeEvent::Event(_)), "先事件");
    match second {
        NodeEvent::Row(row) => assert_eq!(row.unread, 0),
        other => panic!("后该是行更新,实际 {other:?}"),
    }
    let _ = fs::remove_dir_all(&r);
}

#[test]
fn closing_a_chat_deletes_its_files() {
    let r = root("close");
    let n = Node::open(r.clone()).unwrap();
    n.tell(n.name().to_owned().as_str(), "要被删的").unwrap();
    let dir = channel_dir(&r, n.name()).unwrap();
    assert!(dir.exists());

    let id = n.sessions().unwrap()[0].id.clone();
    n.close(&id).unwrap();
    assert!(!dir.exists(), "历史和收下的文件该一起没了");
    // The channel itself persists as an empty conversation.
    let row = &n.sessions().unwrap()[0];
    assert_eq!((row.unread, row.state.state), (0, State::Idle));
    let _ = fs::remove_dir_all(&r);
}

#[test]
fn telling_an_unknown_machine_is_refused_by_name() {
    let r = root("unknown");
    let n = Node::open(r.clone()).unwrap();
    let e = n.tell("no-such-box", "hi").unwrap_err();
    assert!(e.contains("机器不存在"), "{e}");
    assert!(e.contains(n.name()), "错话该报出有的机器:{e}");
    let _ = fs::remove_dir_all(&r);
}

#[test]
fn a_wrong_session_id_names_what_exists() {
    let r = root("badid");
    let n = Node::open(r.clone()).unwrap();
    let e = n.seen(&SessionId("chat/elsewhere".into())).unwrap_err();
    assert!(e.contains("chat/"), "错话该带上现有的 id:{e}");
    let _ = fs::remove_dir_all(&r);
}
