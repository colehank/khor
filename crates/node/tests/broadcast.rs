//! Real-connection acceptance for the broadcast (STORE-BRIEF phase 3,
//! `khor_node::wide`): the khor's own group conversation stands on every
//! machine once there are two, its lines spread through whichever
//! machine is up, and a file in it is an offer each machine answers for
//! itself — accepted, pulled from the sender, its state kept per
//! receiver, counted on the sender. Every await under a timeout.

mod util;

use std::fs;
use std::path::Path;
use std::time::Duration;

use khor_core::State;
use khor_node::wide::BROADCAST;
use khor_node::{MsgBody, Node};
use khor_sync::chat::Step;
use tokio::time::timeout;

/// A scratch tree that takes itself away however the test leaves (the
/// copy in `transfer.rs` has the reasoning).
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
    let p = std::env::temp_dir().join(format!("khor-bcast-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    util::hermetic(&p);
    Scratch(p)
}

async fn wait_for_endpoint_file(root: &Path) {
    // 90 s for the reason `transfer.rs` gives (a new binary's first
    // network touch on a Mac over ssh).
    let path = root.join(".khor").join("endpoint.json");
    timeout(Duration::from_secs(90), async {
        while !path.exists() {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("serve should write endpoint.json");
}

fn row(n: &Node) -> Option<khor_core::Session> {
    n.sessions()
        .unwrap()
        .into_iter()
        .find(|v| v.session.id.0 == BROADCAST)
        .map(|v| v.session)
}

async fn sync(n: &Node) {
    timeout(Duration::from_secs(45), n.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_broadcast_reaches_every_machine_and_each_answers_its_offer() {
    let ra = root("a");
    let rb = root("b");
    let rc = root("c");
    let server = Node::open_as(ra.to_path_buf(), "alpha").unwrap();
    let serve = tokio::spawn(async move { server.serve().await });
    wait_for_endpoint_file(&ra).await;
    let a = Node::open_as(ra.to_path_buf(), "alpha").unwrap();
    let b = Node::open_as(rb.to_path_buf(), "beta").unwrap();
    let c = Node::open_as(rc.to_path_buf(), "gamma").unwrap();

    // A machine alone has no group: no row, nothing to create.
    assert!(row(&b).is_none(), "one machine, no broadcast");

    for n in [&b, &c] {
        let ticket = a.invite().unwrap();
        timeout(Duration::from_secs(45), n.pair(&ticket))
            .await
            .expect("pairing must not hang")
            .unwrap();
    }
    sync(&b).await;
    sync(&c).await;
    // The row stands on every machine, from its own copy, with nothing
    // created first: the khor's own, wearing the broadcast kind.
    for n in [&a, &b, &c] {
        let r = row(n).expect("the broadcast row stands on every machine");
        assert_eq!(r.kind.0, khor_core::kind::BROADCAST);
        assert_eq!(r.category.as_deref(), Some(khor_core::category::KHOR));
        assert_eq!(n.wide_members().unwrap().len(), 3, "every machine is a member");
    }

    // A line from beta spreads through alpha: gamma reads it although
    // beta serves nothing (the sender gone is no matter).
    b.tell(BROADCAST, "hello everyone").unwrap();
    sync(&b).await;
    sync(&c).await;
    let said = c.log(BROADCAST).unwrap().messages;
    assert!(
        said.iter().any(|m| matches!(&m.body, MsgBody::Text(t) if t == "hello everyone")),
        "gamma reads beta's line through alpha"
    );
    let rc_row = row(&c).unwrap();
    assert_eq!(rc_row.last.as_deref(), Some("hello everyone"), "the row previews the newest line");
    // Every machine is the same person's: a line typed on beta was seen
    // by its writer, and the seen mark travels (`Node::tell`'s rule).
    assert_eq!((rc_row.unread, rc_row.state.state), (0, State::Idle), "own words are unread nowhere");

    // alpha offers a file. Nothing moves by itself: each machine's row
    // waits on that machine's yes (待批), the sender's does not.
    let payload: Vec<u8> = (0..300_000u32).map(|i| (i % 241) as u8).collect();
    let src = ra.join("all.bin");
    fs::write(&src, &payload).unwrap();
    let tid = a.send(BROADCAST, &src, "").unwrap();
    let msg = tid.0.strip_prefix("transfer/").unwrap().to_owned();
    sync(&b).await;
    sync(&c).await;
    for n in [&b, &c] {
        assert_eq!(row(n).unwrap().state.state, State::Blocked, "an unanswered offer is 待批");
    }
    assert_ne!(row(&a).unwrap().state.state, State::Blocked, "the sender waits on nobody");
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(!rb.join("khor-drop").join("all.bin").exists(), "an offer is not pulled before it is accepted");
    let face = |n: &Node| {
        let (log, faces) = n.wide_log(BROADCAST).unwrap();
        let i = log.messages.iter().position(|m| m.id == msg).unwrap();
        faces[i].clone().unwrap()
    };
    let f = face(&a);
    assert_eq!((f.got, f.of, f.here), (0, 2, None), "nobody has it yet, two were asked");

    // beta accepts: pulled from alpha into beta's own receive location.
    timeout(Duration::from_secs(60), b.take(&tid))
        .await
        .expect("accept must not hang")
        .unwrap();
    let landed = rb.join("khor-drop").join("all.bin");
    assert_eq!(fs::read(&landed).unwrap(), payload, "verbatim, in beta's receive location");
    assert_eq!(face(&b).here.unwrap().step, Step::Landed);
    assert_ne!(row(&b).unwrap().state.state, State::Blocked, "answered, beta's row lets go");
    assert_eq!(row(&c).unwrap().state.state, State::Blocked, "gamma's own still waits");

    // gamma's pull breaks (the sender's file is gone): its own record
    // says so, beta's is untouched, and a retry resumes once it is back.
    fs::rename(&src, ra.join("away.bin")).unwrap();
    timeout(Duration::from_secs(60), c.take(&tid))
        .await
        .expect("a failing accept must not hang")
        .unwrap_err();
    let failed = face(&c).here.unwrap();
    assert_eq!(failed.step, Step::Failed);
    assert!(!failed.why.is_empty(), "with its sentence");
    assert_eq!(row(&c).unwrap().state.state, State::Errored, "a broken pull is 中断");
    fs::rename(ra.join("away.bin"), &src).unwrap();
    timeout(Duration::from_secs(60), c.take(&tid))
        .await
        .expect("the retry must not hang")
        .unwrap();
    assert_eq!(fs::read(rc.join("khor-drop").join("all.bin")).unwrap(), payload);

    // The sender counts both, from the records the receivers wrote.
    sync(&b).await;
    sync(&c).await;
    let f = face(&a);
    assert_eq!((f.got, f.of), (2, 2), "the sender reads 2/2");
    let both = a.log(BROADCAST).unwrap().messages.into_iter().find(|m| m.id == msg).unwrap();
    assert_eq!(both.fates.len(), 2, "one record per receiver");

    // The machine that sent it has nothing to receive.
    let own = timeout(Duration::from_secs(60), a.take(&tid))
        .await
        .expect("must not hang")
        .unwrap_err();
    assert_eq!(own, khor_catalog::msg::own_broadcast_file());

    serve.abort();
}
