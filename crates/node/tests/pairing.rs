//! Real-connection acceptance (docs/NET.md 验收): two nodes on one
//! machine, real UDP, plus the control groups — what must not connect
//! really doesn't. Every await is under a timeout: a control that hangs
//! proves nothing.

use std::path::PathBuf;
use std::time::Duration;

use khor_node::Node;
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
    .expect("serve 该在 10 秒内写出 endpoint.json");
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
        .expect("配对不该挂住")
        .unwrap();
    assert_eq!(peer_name, "alpha");

    // 配对没有方向:双方设备表里都有对方,只有一边记了就是没配完。
    let b_names: Vec<String> = b.devices().unwrap().into_iter().map(|d| d.name).collect();
    assert_eq!(b_names, vec!["alpha", "beta"], "beta 的表该有双方");
    let a_names: Vec<String> = a.devices().unwrap().into_iter().map(|d| d.name).collect();
    assert_eq!(a_names, vec!["alpha", "beta"], "alpha 的表该有双方");

    // beta writes into alpha's window and pushes; alpha's serve merges.
    b.tell("alpha", "从 beta 来的").unwrap();
    let outcomes = timeout(Duration::from_secs(20), b.sync_now())
        .await
        .expect("同步不该挂住")
        .unwrap();
    let (_, verdict) = outcomes.iter().find(|(n, _)| n == "alpha").expect("该有 alpha 一项");
    verdict.as_ref().expect("对 alpha 的同步该成功");

    let a_log = a.log("alpha").unwrap();
    let texts: Vec<String> = a_log
        .messages
        .iter()
        .map(|m| format!("{:?}", m.body))
        .collect();
    assert!(
        texts.iter().any(|t| t.contains("从 beta 来的")),
        "alpha 侧该收到 beta 那句:{texts:?}"
    );

    // The other direction: alpha notes to self, beta pulls it.
    a.tell("alpha", "从 alpha 回的").unwrap();
    timeout(Duration::from_secs(20), b.sync_now())
        .await
        .expect("同步不该挂住")
        .unwrap();
    let b_log = b.log("alpha").unwrap();
    let texts: Vec<String> = b_log
        .messages
        .iter()
        .map(|m| format!("{:?}", m.body))
        .collect();
    assert!(
        texts.iter().any(|t| t.contains("从 alpha 回的")),
        "beta 侧该拉到 alpha 那句:{texts:?}"
    );

    // beta's row for alpha's window: unseen foreign words = Done/unread.
    let rows = b.sessions().unwrap();
    let alpha_row = rows.iter().find(|s| s.title == "alpha").expect("该有 alpha 行");
    assert!(alpha_row.unread > 0, "alpha 那句在 beta 侧该算未读");

    // ── 对照组 ──────────────────────────────────────────────
    // A burned token pairs nobody: gamma replays beta's ticket.
    let rc = root("c");
    let c = Node::open_as(rc.clone(), "gamma").unwrap();
    let err = timeout(Duration::from_secs(15), c.pair(&ticket))
        .await
        .expect("被拒也不该挂住")
        .unwrap_err();
    assert!(err.contains("配对码"), "重放该被指名拒绝:{err}");
    assert_eq!(c.devices().unwrap().len(), 1, "gamma 的表里只该有它自己");

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
        .expect("被拒也不该挂住")
        .unwrap();
    let (_, verdict) = outcomes.iter().find(|(n, _)| n == "alpha").expect("该有 alpha 一项");
    let err = verdict.as_ref().unwrap_err();
    assert!(err.contains("先配对"), "没配对的同步该被指名拒绝:{err}");

    serve.abort();
    for r in [&ra, &rb, &rc] {
        let _ = std::fs::remove_dir_all(r);
    }
}
