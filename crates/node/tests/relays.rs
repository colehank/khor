//! A machine's own relays (docs/NET.md 中继): the list is the user's,
//! it is local to this machine, and it survives the node that wrote it.
//!
//! No network. Every assertion here is about a file on disk and the list
//! a freshly opened node binds with — the relays are never dialled, so
//! nothing in this file depends on the aliyun box being up (which is
//! what `relay.rs` is for, and why that one is `#[ignore]`).

use khor_node::{Node, Relay};

/// An address in TEST-NET-3 (RFC 5737), which exists so that a document
/// can name an address nobody routes. It has to parse as a relay url and
/// it must never be dialled — both true here.
const MINE: &str = "http://203.0.113.9:3340";

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

fn root(tag: &str) -> Scratch {
    let p = std::env::temp_dir().join(format!("khor-relays-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    Scratch(p)
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

    let node = Node::open(root.to_path_buf()).expect("a fresh home should open");
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
    assert!(
        before > 0,
        "the built-in relay tier should still be present"
    );

    let merged = node
        .set_relays(&[Relay::bare(MINE)], false)
        .expect("a real relay url should be accepted");
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
    let reopened = Node::open(root.to_path_buf()).expect("the home should reopen");
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
    let refused = reopened.set_relays(&[Relay::bare("这不是一个地址")], false);
    assert!(
        refused.is_err(),
        "a string that is not a relay url must not be stored"
    );
    assert_eq!(
        reopened.relay_choices(),
        vec![MINE.to_string()],
        "a refused write must leave the previous list alone"
    );

    // And the list can be emptied — the delete key has to reach the
    // thing the add button wrote.
    reopened
        .set_relays(&[], false)
        .expect("clearing should be allowed");
    assert!(
        reopened.relay_choices().is_empty(),
        "clearing should remove the user's own entries"
    );
    let bare = Node::open(root.to_path_buf()).expect("the home should reopen");
    assert!(
        !bare.relays().iter().any(|r| r == MINE),
        "and the cleared address should be gone from the list a node binds with"
    );
}

/// 「只用我自己那台」replaces the Khor tier instead of joining it — and it
/// cannot be turned on with nothing to replace it with.
///
/// **The first assertion is again the arm that gives the rest meaning.**
/// "the list is exactly my relay" is also true of a machine whose
/// built-in tier happened to be empty, so the default case is measured
/// first: more than one entry, and mine is not the only one.
#[test]
fn only_mine_replaces_the_list_and_cannot_be_set_with_an_empty_one() {
    let root = root("onlymine");
    let node = Node::open(root.to_path_buf()).expect("a fresh home should open");

    // Refused before anything is stored: there is nothing to be the
    // only one. The switch must not come on by itself either.
    assert!(
        node.set_relays(&[], true).is_err(),
        "「only mine」with no relay of my own must be refused, not silently downgraded"
    );
    assert!(
        !node.relays_only_mine(),
        "and the refusal must not have stored it"
    );

    // Added: mine plus the built-in tier. This is the control for the
    // exclusive case below — without it, "the list is just mine" could
    // mean the tier was never there.
    let joined = node
        .set_relays(&[Relay::bare(MINE)], false)
        .expect("a real relay url should be accepted");
    assert!(
        joined.len() > 1,
        "added means the built-in tier is still in the list"
    );

    // Exclusive: the same one address, and now it is the whole map.
    let alone = node
        .set_relays(&[Relay::bare(MINE)], true)
        .expect("one relay of my own is enough to be the only one");
    assert_eq!(
        alone,
        vec![MINE.to_string()],
        "「only mine」must replace the built-in tier, not sit on top of it"
    );

    let reopened = Node::open(root.to_path_buf()).expect("the home should reopen");
    assert!(
        reopened.relays_only_mine(),
        "the switch should survive a reopen"
    );
    assert_eq!(
        reopened.relays(),
        [MINE.to_string()],
        "and so should the list it makes"
    );

    // Clearing the list while exclusive is on is the same forbidden
    // state arriving by a different door.
    assert!(
        reopened.set_relays(&[], true).is_err(),
        "emptying the list must not be a way into「only mine, and none」"
    );
}

/// A relay's key is stored with it, reaches the bind, and reaches
/// nothing that is published.
///
/// **The control is the url list, not the key.** "the bind carries the
/// key" is satisfied by a node that also put the key into the list it
/// hands to the device table and the ticket — which would copy a shared
/// secret into every machine's history. So the published list is read
/// for the key too, and must not have it.
#[test]
fn a_relay_key_reaches_the_bind_and_nothing_that_is_published() {
    const KEY: &str = "sesame-4f2a";
    let root = root("keyed");
    let node = Node::open(root.to_path_buf()).expect("a fresh home should open");
    node.set_relays(
        &[Relay {
            url: MINE.to_string(),
            token: Some(KEY.to_string()),
        }],
        false,
    )
    .expect("a keyed relay should be accepted");

    let reopened = Node::open(root.to_path_buf()).expect("the home should reopen");
    let bound = reopened
        .relay_map()
        .iter()
        .find(|r| r.url == MINE)
        .expect("the typed relay is in the map the bind uses");
    assert_eq!(
        bound.token.as_deref(),
        Some(KEY),
        "the bind must carry the key the relay was entered with"
    );
    assert!(
        reopened.relays().iter().all(|r| !r.contains(KEY)),
        "the published list must not carry the key: {:?}",
        reopened.relays()
    );
    assert_eq!(
        reopened.relays_keyed(),
        vec![MINE.to_string()],
        "the screen learns that the relay is keyed"
    );

    // A write that names the relay without a key keeps the stored one —
    // the screen never holds keys, so removing *another* relay must not
    // strip this one's.
    const OTHER: &str = "http://203.0.113.10:3340";
    reopened
        .set_relays(&[Relay::bare(MINE), Relay::bare(OTHER)], false)
        .expect("adding a second relay should be accepted");
    reopened
        .set_relays(&[Relay::bare(MINE)], false)
        .expect("removing the second relay should be accepted");
    let again = Node::open(root.to_path_buf()).expect("the home should reopen");
    assert_eq!(
        again
            .relay_map()
            .iter()
            .find(|r| r.url == MINE)
            .and_then(|r| r.token.as_deref()),
        Some(KEY),
        "a relay named without a key keeps the key it already had"
    );

    // Removing the relay takes its key with it: adding it back bare
    // gives a bare relay, not the old key resurrected.
    again.set_relays(&[], false).expect("clearing should be allowed");
    again
        .set_relays(&[Relay::bare(MINE)], false)
        .expect("re-adding should be allowed");
    assert!(
        again.relays_keyed().is_empty(),
        "a relay removed and added back without a key has none"
    );

    // The file holding the key is private to this user.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(root.join(".khor").join("relays.json"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o077, 0, "relays.json must be 0600, is {mode:o}");
    }
}

/// A relay file from before relays took keys — a bare array of url
/// strings — still reads as those relays. Losing it would leave a
/// machine that upgraded on the Khor tier alone with nothing saying the
/// user's list had gone.
#[test]
fn a_relay_file_of_bare_urls_still_reads() {
    let root = root("bare");
    std::fs::create_dir_all(root.join(".khor")).unwrap();
    std::fs::write(
        root.join(".khor").join("relays.json"),
        format!(r#"{{"relays": ["{MINE}", "not a url"], "only_mine": true}}"#),
    )
    .unwrap();
    let node = Node::open(root.to_path_buf()).expect("the home should open");
    assert_eq!(node.relay_choices(), vec![MINE.to_string()]);
    assert!(node.relays_only_mine(), "the switch is read too");
    assert!(
        node.relays().iter().any(|r| r == MINE),
        "and the relay is bound with"
    );
}
