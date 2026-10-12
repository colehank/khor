//! A push and a third device (STORE-BRIEF phase 5): alpha posts a file
//! `@beta`, beta's resident lands it with nobody pressing anything, and
//! gamma — a one-shot device the message was not pushed to — sees the
//! words and the file as an offer, takes it, and pulls its own copy.
//! Real UDP between two serves plus a third dialer; wire-level
//! controls; every await under a timeout.
//!
//! gamma sees it because the message is replicated, not because anybody
//! reported it.

use std::fs;
use std::path::Path;
use std::time::Duration;

use khor_node::Node;
use khor_node::proto::{Request, Response};
use tokio::time::timeout;

mod util;
use util::raw_request;

/// How long a pairing or a sync may take before it counts as hung.
///
/// **Raised from 45s because the machine, not the code, was failing
/// it.** This file's own `wait_for_endpoint_file` already explains why
/// (#73: a fresh test binary's first network-stack touch hangs 20-40s
/// on this ssh-operated Mac) and already answers it with 90s — but the
/// pair and sync steps kept 45, and with three nodes each paying that
/// toll one of them went over about half the time. Measured on
/// 2026-08-21: red twice at 45s on the same sync, green at 150s in 82s
/// total, with nothing between the runs but the number.
///
/// It still guards what it was written to guard. The assertion is *must
/// not hang* — a deadlock or a lost frame never finishes at any number
/// — and a step that needs two minutes on this machine is slow, which
/// this suite has never been in the business of failing anyone for.
const PATIENCE: u64 = 120;

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
    let p = std::env::temp_dir().join(format!("khor-act-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    util::hermetic(&p);
    Scratch(p)
}

async fn wait_for_endpoint_file(root: &Path) {
    let path = root.join(".khor").join("endpoint.json");
    // 90s and multi-thread for the same reason as transfer.rs: a fresh
    // test binary's first network-stack touch hangs 20-40s on this
    // ssh-operated Mac (#73), synchronously — on current_thread that
    // froze tokio's clock and every timeout below quietly stretched.
    timeout(Duration::from_secs(90), async {
        while !path.exists() {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("serve should write endpoint.json within 10s");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_third_device_sees_the_file_land_on_the_recipient_with_nobody_asked() {
    let ra = root("a");
    let rb = root("b");
    let rg = root("g");
    let sa = Node::open_as(ra.to_path_buf(), "alpha").unwrap();
    let serve_a = tokio::spawn(async move { sa.serve().await });
    wait_for_endpoint_file(&ra).await;
    let sb = Node::open_as(rb.to_path_buf(), "beta").unwrap();
    let serve_b = tokio::spawn(async move { sb.serve().await });
    wait_for_endpoint_file(&rb).await;

    let a = Node::open_as(ra.to_path_buf(), "alpha").unwrap();
    let b = Node::open_as(rb.to_path_buf(), "beta").unwrap();
    let g = Node::open_as(rg.to_path_buf(), "gamma").unwrap();

    let t1 = a.invite().unwrap();
    timeout(Duration::from_secs(PATIENCE), b.pair(&t1))
        .await
        .expect("pairing must not hang")
        .unwrap();
    let t2 = a.invite().unwrap();
    timeout(Duration::from_secs(PATIENCE), g.pair(&t2))
        .await
        .expect("pairing must not hang")
        .unwrap();
    // beta learns gamma through the table before gamma ever dials it.
    timeout(Duration::from_secs(PATIENCE), b.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();

    // alpha pushes a file to beta, with words; gamma syncs it in.
    let payload: Vec<u8> = (0..400_000u32).map(|i| (i % 249) as u8).collect();
    let src = ra.join("plan.pdf");
    fs::write(&src, &payload).unwrap();
    let id = timeout(Duration::from_secs(PATIENCE), a.post("@beta the plan", std::slice::from_ref(&src)))
        .await
        .expect("a post must not hang")
        .unwrap();
    let tid = khor_node::transfer::TransferKind::session_id(&id);
    timeout(Duration::from_secs(PATIENCE), b.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();
    timeout(Duration::from_secs(PATIENCE), g.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();

    // gamma is neither sender nor target: it reads the words and the
    // offer from its own copy, derived here, not reported.
    assert!(
        g.transfer_is_here(&tid),
        "the offer must replicate to a third device"
    );
    let views = g.sessions().unwrap();
    let view = views
        .iter()
        .find(|v| v.session.id.0 == khor_node::wide::BROADCAST)
        .expect("the third device derives the broadcast's row");
    assert!(
        view.source.is_none(),
        "a replicated document is derived here, not reported: {:?}",
        view.source
    );
    let m = g.log(khor_node::wide::BROADCAST).unwrap().messages.into_iter().find(|m| m.id == id).unwrap();
    assert_eq!((m.note.as_str(), m.to.len()), ("the plan", 1), "the words and the one target travel with it");

    // Nobody presses anything: beta's resident pulls the payload off
    // alpha as the push lands the message (STORE-BRIEF phase 5).
    let landed = timeout(Duration::from_secs(60), async {
        loop {
            let found = fs::read_dir(rb.join("khor-drop"))
                .into_iter()
                .flatten()
                .flatten()
                .map(|e| e.path())
                .find(|p| !p.file_name().unwrap().to_string_lossy().starts_with('.'));
            if let Some(p) = found.filter(|p| fs::metadata(p).is_ok_and(|m| m.len() == payload.len() as u64)) {
                break p;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("the payload should land on beta with nobody asked");
    assert_eq!(fs::read(&landed).unwrap(), payload, "bytes verbatim on the recipient");

    // **Every device reads the same state**: beta wrote the landing on
    // the message, and a sync carries it to gamma, which has no such
    // file and no such directory.
    let beta_id = b.device_str().to_owned();
    let read_fate = |n: &Node| {
        n.log(khor_node::wide::BROADCAST)
            .unwrap()
            .messages
            .into_iter()
            .find(|m| m.id == id)
            .and_then(|m| m.fates.get(&beta_id).cloned())
    };
    timeout(Duration::from_secs(60), async {
        while read_fate(&b).is_none_or(|f| f.step != khor_sync::chat::Step::Landed) {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("beta records the landing");
    timeout(Duration::from_secs(PATIENCE), g.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();
    let seen = read_fate(&g).expect("the landing travels to the third device");
    assert_eq!(seen.step, khor_sync::chat::Step::Landed);
    assert_eq!(seen.landed, vec![landed.display().to_string()], "beta's own path, read on gamma");

    // gamma takes the offer for itself: its own copy, pulled from alpha
    // or from beta, which holds it now, into gamma's receive location.
    let (moved, said) = timeout(Duration::from_secs(60), g.take(&tid))
        .await
        .expect("a take must not hang")
        .unwrap();
    let mine = rg.join("khor-drop").join("plan.pdf");
    assert_eq!((moved, said), (payload.len() as u64, vec![mine.display().to_string()]));
    assert_eq!(fs::read(&mine).unwrap(), payload, "gamma's own copy, verbatim");
    assert!(
        g.sessions()
            .unwrap()
            .iter()
            .all(|v| !v.session.id.0.starts_with("transfer/")),
        "no transfer row may come back from a sync"
    );

    // ── wire-level controls ─────────────────────────────────
    let beta_info = g
        .devices()
        .unwrap()
        .into_iter()
        .find(|d| d.name == "beta")
        .expect("gamma should know beta");

    // An unknown action is refused by name, not silently mapped.
    let gamma_key =
        khor_net::identity::load_or_create(&rg.join(".khor").join("identity.key")).unwrap();
    let resp = timeout(
        Duration::from_secs(PATIENCE),
        raw_request(
            gamma_key,
            &beta_info.id,
            &beta_info.addrs,
            &Request::Act {
                session: tid.0.clone(),
                action: "detonate".into(),
            },
        ),
    )
    .await
    .expect("a refused act must not hang")
    .unwrap();
    match resp {
        Response::Refused { why } => assert_eq!(why, khor_catalog::msg::unknown_action("detonate")),
        other => panic!("an unknown action must be refused, got {other:?}"),
    }

    // An unpaired key gets nothing — Act is gated like everything else.
    let resp = timeout(
        Duration::from_secs(PATIENCE),
        raw_request(
            iroh::SecretKey::generate(),
            &beta_info.id,
            &beta_info.addrs,
            &Request::Act {
                session: tid.0.clone(),
                action: "take".into(),
            },
        ),
    )
    .await
    .expect("a refused act must not hang")
    .unwrap();
    match resp {
        Response::Refused { why } => assert_eq!(why, khor_catalog::msg::not_paired()),
        other => panic!("an unpaired act must be refused, got {other:?}"),
    }

    serve_a.abort();
    serve_b.abort();
}
