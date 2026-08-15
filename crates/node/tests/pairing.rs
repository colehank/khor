//! Real-connection acceptance (docs/NET.md): two nodes on one
//! machine, real UDP, plus the control groups — what must not connect
//! really doesn't. Every await is under a timeout: a control that hangs
//! proves nothing.

use std::path::PathBuf;
use std::time::Duration;

use khor_node::Node;
use khor_sync::chat::{channel_dir, ChatDoc, Sender};
use tokio::time::timeout;

fn root(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("khor-pair-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

async fn wait_for_endpoint_file(root: &PathBuf) {
    let path = root.join(".khor").join("endpoint.json");
    timeout(Duration::from_secs(10), async {
        while !path.exists() {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("serve should write endpoint.json within 10s");
}

#[tokio::test]
async fn pairing_joins_both_tables_and_chat_flows_both_ways() {
    let ra = root("a");
    let rb = root("b");

    // alpha serves; the serve task owns its node instance.
    let server = Node::open_as(ra.clone(), "alpha").unwrap();
    let serve = tokio::spawn(async move { server.serve().await });
    wait_for_endpoint_file(&ra).await;

    // A second alpha instance plays the one-shot CLI role.
    let a = Node::open_as(ra.clone(), "alpha").unwrap();
    let ticket = a.invite().unwrap();

    let b = Node::open_as(rb.clone(), "beta").unwrap();
    let peer_name = timeout(Duration::from_secs(15), b.pair(&ticket))
        .await
        .expect("pairing must not hang")
        .unwrap();
    assert_eq!(peer_name, "alpha");

    // Pairing has no direction: both tables hold both machines; one
    // side alone means pairing is incomplete.
    let b_names: Vec<String> = b.devices().unwrap().into_iter().map(|d| d.name).collect();
    assert_eq!(b_names, vec!["alpha", "beta"], "beta's table should hold both");
    let a_names: Vec<String> = a.devices().unwrap().into_iter().map(|d| d.name).collect();
    assert_eq!(a_names, vec!["alpha", "beta"], "alpha's table should hold both");

    // beta writes into alpha's window and pushes; alpha's serve merges.
    b.tell("alpha", "from beta").unwrap();
    let outcomes = timeout(Duration::from_secs(20), b.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();
    let (_, verdict) = outcomes.iter().find(|(n, _)| n == "alpha").expect("there should be an alpha entry");
    verdict.as_ref().expect("the sync to alpha should succeed");

    let a_log = a.log("alpha").unwrap();
    let texts: Vec<String> = a_log
        .messages
        .iter()
        .map(|m| format!("{:?}", m.body))
        .collect();
    assert!(
        texts.iter().any(|t| t.contains("from beta")),
        "alpha should receive beta's line: {texts:?}"
    );

    // The other direction: alpha notes to self, beta pulls it.
    a.tell("alpha", "back from alpha").unwrap();
    timeout(Duration::from_secs(20), b.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();
    let b_log = b.log("alpha").unwrap();
    let texts: Vec<String> = b_log
        .messages
        .iter()
        .map(|m| format!("{:?}", m.body))
        .collect();
    assert!(
        texts.iter().any(|t| t.contains("back from alpha")),
        "beta should pull alpha's line: {texts:?}"
    );

    // Everything in the window so far was typed by this one person, and
    // telling implies looking — so once the seen watermark syncs, nothing
    // counts as unread on any device.
    let rows = b.sessions().unwrap();
    let alpha_row = rows.iter().find(|v| v.session.title == "alpha").expect("there should be an alpha row");
    assert_eq!(alpha_row.session.unread, 0, "own words never count as unread, on any device");

    // A line nobody here typed (an agent's, say): lands as a raw block in
    // beta's copy of alpha's window, with no seen state riding along.
    {
        let far = ChatDoc::new(0xFA).unwrap();
        far.tell(&Sender { id: "dev-far".into(), name: "far".into() }, "an outside line")
            .unwrap();
        let block = far.changes_since(&Default::default()).unwrap();
        let dir = channel_dir(&rb, "alpha").unwrap();
        std::fs::write(dir.join("u-00000000000000fa-00000000.loro"), &block).unwrap();
    }
    let rows = b.sessions().unwrap();
    let row_b = rows.iter().find(|v| v.session.title == "alpha").unwrap();
    assert!(row_b.session.unread > 0, "the outside line should count as unread on beta");

    // It syncs over and is unread on alpha too (control for the clear).
    timeout(Duration::from_secs(20), b.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();
    let rows = a.sessions().unwrap();
    let row_a = rows.iter().find(|v| v.session.title == "alpha").unwrap();
    assert!(row_a.session.unread > 0, "the outside line should be unread on alpha too");

    // Seen is a decision that travels: beta looks, and alpha's badge
    // clears without anyone touching alpha.
    b.seen(&row_b.session.id).unwrap();
    timeout(Duration::from_secs(20), b.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();
    let rows = a.sessions().unwrap();
    let row_a = rows.iter().find(|v| v.session.title == "alpha").unwrap();
    assert_eq!(row_a.session.unread, 0, "seen on beta must clear on alpha");

    // ── control groups ──────────────────────────────────────────────
    // A burned token pairs nobody: gamma replays beta's ticket.
    let rc = root("c");
    let c = Node::open_as(rc.clone(), "gamma").unwrap();
    let err = timeout(Duration::from_secs(15), c.pair(&ticket))
        .await
        .expect("refusal must not hang either")
        .unwrap_err();
    assert_eq!(err, khor_catalog::msg::BAD_INVITE, "a replay is refused by name");
    assert_eq!(c.devices().unwrap().len(), 1, "gamma's table should hold only itself");

    // An unpaired device syncs nothing — even knowing the address.
    // gamma copies alpha's entry into its own table by hand…
    {
        use khor_sync::devices::devices_dir;
        use khor_sync::store::load;
        let alpha = a.devices().unwrap().into_iter().find(|d| d.name == "alpha").unwrap();
        let loaded = load::<khor_sync::devices::DeviceDoc>(&devices_dir(&rc), 0x77).unwrap();
        loaded.doc.upsert(&alpha.id, &alpha.name, &alpha.addrs).unwrap();
        let mut store = loaded.store;
        store.flush(&loaded.doc).unwrap();
    }
    // …and the far side still refuses by name.
    let outcomes = timeout(Duration::from_secs(20), c.sync_now())
        .await
        .expect("refusal must not hang either")
        .unwrap();
    let (_, verdict) = outcomes.iter().find(|(n, _)| n == "alpha").expect("there should be an alpha entry");
    let err = verdict.as_ref().unwrap_err();
    assert_eq!(err, khor_catalog::msg::NOT_PAIRED, "an unpaired sync is refused by name");

    serve.abort();
    for r in [&ra, &rb, &rc] {
        let _ = std::fs::remove_dir_all(r);
    }
}
