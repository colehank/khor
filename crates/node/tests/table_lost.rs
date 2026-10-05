//! Round NET-10: a machine whose device table lost every row but its own
//! is brought back by the next round of any machine its roster names,
//! over a real wire.
//!
//! The owner's hinton, 2026-10-05: its table held its own row and nothing
//! else, so its gate refused every machine (「not in the machine table」)
//! and its pump had nobody to visit, while every other machine still
//! listed it as paired. Neither side could start the round that would
//! have brought the rows back.

mod util;

use std::path::Path;
use std::time::Duration;

use khor_node::{Node, Standing};
use tokio::time::timeout;

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
    let p = std::env::temp_dir().join(format!("khor-lost-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    util::hermetic(&p);
    Scratch(p)
}

/// 90 s: issue #73's first-touch stall (`pairing.rs` has the reasoning).
async fn wait_for_endpoint_file(root: &Path) {
    let path = root.join(".khor").join("endpoint.json");
    timeout(Duration::from_secs(90), async {
        while !path.exists() {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("serve should write endpoint.json");
}

fn names(n: &Node) -> Vec<String> {
    n.devices().unwrap().into_iter().map(|d| d.name).collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_machine_whose_table_lost_its_rows_is_brought_back_by_the_next_round() {
    let (ra, rb) = (root("a"), root("b"));
    let mut serves = Vec::new();
    for (r, name) in [(&ra, "alpha"), (&rb, "beta")] {
        let n = Node::open_as(r.to_path_buf(), name).unwrap();
        serves.push(tokio::spawn(async move { n.serve().await }));
    }
    for r in [&ra, &rb] {
        wait_for_endpoint_file(r).await;
    }
    let a = Node::open_as(ra.to_path_buf(), "alpha").unwrap();
    let b = Node::open_as(rb.to_path_buf(), "beta").unwrap();
    timeout(Duration::from_secs(60), b.pair(&a.invite().unwrap()))
        .await
        .expect("pairing must not hang")
        .unwrap();
    assert_eq!(names(&b), ["alpha", "beta"]);
    assert_eq!(b.standing_of(b.device_str()), Standing::Trusted, "alpha holds the root and signed beta in");

    // Beta's table loses every block — its ledger stays, as hinton's did —
    // and its next open writes its own row back.
    let dir = khor_sync::devices::devices_dir(&rb);
    for e in std::fs::read_dir(&dir).unwrap().flatten() {
        if e.path().extension().is_some_and(|x| x == "loro") {
            std::fs::remove_file(e.path()).unwrap();
        }
    }
    let b = Node::open_as(rb.to_path_buf(), "beta").unwrap();
    assert_eq!(names(&b), ["beta"], "only its own row is left");
    assert_eq!(b.standing_of(b.device_str()), Standing::Pending, "and no roster");

    // Alpha's next round: refused at beta's door, then let in.
    let outcomes = timeout(Duration::from_secs(60), a.sync_now())
        .await
        .expect("a round must not hang")
        .unwrap();
    let healed = names(&b) == ["alpha", "beta"];
    for s in serves {
        s.abort();
    }
    assert!(healed, "beta's table after alpha's round: {:?} (round: {outcomes:?})", names(&b));
    assert_eq!(b.standing_of(b.device_str()), Standing::Trusted, "the roster came back with the rows");
    assert_eq!(b.standing_of(a.device_str()), Standing::Trusted);
    let outcome = outcomes.into_iter().find(|(n, _)| n == "beta").map(|(_, o)| o);
    assert!(matches!(outcome, Some(Ok(_))), "and the round itself went through: {outcome:?}");
}
