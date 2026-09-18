//! A machine's own relays (docs/NET.md 中继): the list is the user's,
//! it is local to this machine, and it survives the node that wrote it.
//!
//! No network. Every assertion here is about a file on disk and the list
//! a freshly opened node binds with — the relays are never dialled, so
//! nothing in this file depends on the aliyun box being up (which is
//! what `relay.rs` is for, and why that one is `#[ignore]`).

use std::path::PathBuf;

use khor_node::Node;

/// An address in TEST-NET-3 (RFC 5737), which exists so that a document
/// can name an address nobody routes. It has to parse as a relay url and
/// it must never be dialled — both true here.
const MINE: &str = "http://203.0.113.9:3340";

fn root(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("khor-relays-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

/// The whole round trip, with the arm that makes it mean something.
///
/// **The control is the first assertion, not an afterthought.** "the
/// list contains my relay" is true of a list that contains everything,
/// and a merged list is exactly the kind that might: it already carries
/// the built-in tier and whatever `KHOR_RELAY` says. Reading it *before*
/// anything is stored is the only thing here that can tell "the setting
/// worked" from "that address was going to be in there anyway".
#[test]
fn a_typed_relay_joins_the_list_survives_a_reopen_and_a_bad_one_is_refused() {
    let root = root("roundtrip");

    let node = Node::open(root.clone()).expect("a fresh home should open");
    assert!(
        !node.relays().iter().any(|r| r == MINE),
        "control: with nothing typed, this address must not already be in the list — \
         otherwise the assertion below proves nothing"
    );
    assert!(
        node.relay_choices().is_empty(),
        "a home nobody has configured has no relays of its own"
    );
    // The built-in tier is still there; this setting adds, it does not
    // take over. A list that came back empty would pass the assertion
    // above while meaning the opposite of what it says.
    let before = node.relays().len();
    assert!(before > 0, "the built-in relay tier should still be present");

    let merged = node.set_relays(&[MINE.to_string()]).expect("a real relay url should be accepted");
    assert!(
        merged.iter().any(|r| r == MINE),
        "the call should report the list the next node will use"
    );
    assert_eq!(
        merged.len(),
        before + 1,
        "one address typed, one address added — the built-in tier is not replaced"
    );

    // The node that wrote it is not the node that has to use it: the
    // list is read when a node opens, so this is the assertion that the
    // setting reaches anything at all.
    drop(node);
    let reopened = Node::open(root.clone()).expect("the home should reopen");
    assert!(
        reopened.relays().iter().any(|r| r == MINE),
        "a relay typed on this machine must be in the list the next node binds with"
    );
    assert_eq!(
        reopened.relay_choices(),
        vec![MINE.to_string()],
        "and the editable half should be exactly what was typed"
    );

    // Refused, and refused **without touching what was already stored**.
    // A validation that clears the list on its way to saying no would be
    // worse than one that never checked.
    let refused = reopened.set_relays(&["这不是一个地址".to_string()]);
    assert!(refused.is_err(), "a string that is not a relay url must not be stored");
    assert_eq!(
        reopened.relay_choices(),
        vec![MINE.to_string()],
        "a refused write must leave the previous list alone"
    );

    // And the list can be emptied — the delete key has to reach the
    // thing the add button wrote.
    reopened.set_relays(&[]).expect("clearing should be allowed");
    assert!(
        reopened.relay_choices().is_empty(),
        "clearing should remove the user's own entries"
    );
    let bare = Node::open(root.clone()).expect("the home should reopen");
    assert!(
        !bare.relays().iter().any(|r| r == MINE),
        "and the cleared address should be gone from the list a node binds with"
    );

    let _ = std::fs::remove_dir_all(&root);
}
