//! **A machine the relay saw leave is said gone at once** (round NET-5,
//! vendor/iroh/KHOR.md patch 4): two residents in this process that can
//! reach each other only through the tests' loopback relay
//! (`KHOR_RELAY_ONLY`); alpha's ears close, and the relay tells beta.
//! Beta's word for alpha must leave 中继 within a couple of seconds, and
//! a listing on alpha must fail in a short dial — before this, the word
//! held until the silence limit and the first ask waited out an ack and
//! a full dial (14 s over a 200 ms road). The control: alpha's ears back,
//! and a listing answers again.

mod util;

use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use khor_node::{Hop, Node, link};
use tokio::time::timeout;

struct Scratch(std::path::PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn root(tag: &str) -> Scratch {
    let p = std::env::temp_dir().join(format!("khor-departed-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    util::hermetic(&p);
    Scratch(p)
}

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

async fn hop(n: &Node, id: &str) -> Option<Hop> {
    n.links()
        .await
        .ok()?
        .into_iter()
        .find(|(i, _)| i == id)
        .map(|(_, l)| l.hop)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_machine_its_relay_saw_leave_is_said_gone_at_once() {
    // SAFETY: before any endpoint in this test binary binds.
    unsafe { std::env::set_var("KHOR_RELAY_ONLY", "1") };
    let ra = root("a");
    let rb = root("b");
    let sa = Node::open_as(ra.0.clone(), "alpha").unwrap();
    let _serve_a = tokio::spawn(async move { sa.serve_resumable().await });
    let sb = Node::open_as(rb.0.clone(), "beta").unwrap();
    let _serve_b = tokio::spawn(async move { sb.serve().await });
    wait_for_endpoint_file(&ra.0).await;
    wait_for_endpoint_file(&rb.0).await;
    let a = Node::open_as(ra.0.clone(), "alpha").unwrap();
    let b = Node::open_as(rb.0.clone(), "beta").unwrap();
    // Both on the relay before either can be found there.
    tokio::time::sleep(Duration::from_secs(2)).await;
    timeout(Duration::from_secs(60), b.pair(&a.invite().unwrap()))
        .await
        .expect("pairing must not hang")
        .unwrap();
    let alpha_id = a.device_str().to_owned();
    let dir = ra.0.display().to_string();
    let began = Instant::now();
    while b.ls_of("alpha", &dir).await.is_err() {
        assert!(
            began.elapsed() < Duration::from_secs(60),
            "beta reaches alpha"
        );
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    assert_eq!(
        hop(&b, &alpha_id).await,
        Some(Hop::Relay),
        "through the relay"
    );
    // **And which relay** (round NET-8 addendum, UX-16's map): the
    // tests' own, by host and port.
    let named = b
        .links()
        .await
        .unwrap()
        .into_iter()
        .find(|(id, _)| *id == alpha_id)
        .and_then(|(_, l)| l.relay);
    let want = util::loop_relay()
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .to_owned();
    assert_eq!(
        named,
        Some(want),
        "the link names the relay it goes through"
    );

    // Alpha's ears close: the relay sees it leave and says so.
    assert!(link::deafen(&ra.0).await, "alpha's resident deafens");
    let left = Instant::now();
    while hop(&b, &alpha_id).await == Some(Hop::Relay) {
        assert!(
            left.elapsed() < Duration::from_secs(3),
            "beta still says 中继 for alpha {:?} after its relay saw it leave",
            left.elapsed()
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let word_took = left.elapsed();
    let asked = Instant::now();
    assert!(b.ls_of("alpha", &dir).await.is_err(), "alpha is not there");
    let ask_took = asked.elapsed();
    assert!(
        ask_took < Duration::from_secs(6),
        "the ask took {ask_took:?}"
    );

    // **And the pump leaves it alone** (round NET-6): it will dial when
    // it is back. Beta's face is on, so its pump beats every 5 s; before,
    // each beat its backoff allowed was a failed dial, about 9 KB through
    // the relay.
    // **The face arriving is a person's act and dials past every wait**
    // (NET-STABLE 第二步): one dial, given a departed machine's short
    // budget. What is measured is the pump after it.
    let before_face = b.wire_counts().await.unwrap().dial_failed;
    b.face_is_here(true).await.unwrap();
    let faced = Instant::now();
    while b.wire_counts().await.unwrap().dial_failed == before_face
        && faced.elapsed() < Duration::from_secs(8)
    {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let failed = b.wire_counts().await.unwrap().dial_failed;
    assert!(
        failed - before_face <= 1,
        "the face's round dialled alpha once at most"
    );
    tokio::time::sleep(link::SYNC_EVERY * 3 + Duration::from_secs(1)).await;
    let pump_failed = b.wire_counts().await.unwrap().dial_failed - failed;
    assert_eq!(
        pump_failed, 0,
        "beta's pump dialled the machine its relay said left"
    );

    // The control: alpha's ears back, and it answers again.
    link::resume(&ra.0)
        .expect("resumable")
        .synced()
        .await
        .unwrap();
    let back = Instant::now();
    while b.ls_of("alpha", &dir).await.is_err() {
        assert!(
            back.elapsed() < Duration::from_secs(30),
            "beta reaches alpha again"
        );
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    eprintln!(
        "[departed] word left 中继 in {word_took:?}; a listing on the gone machine failed in {ask_took:?}; pump dials while gone {pump_failed}; \
         back, a listing answered in {:?}",
        back.elapsed()
    );
}
