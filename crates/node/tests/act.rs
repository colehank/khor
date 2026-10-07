//! Third-device acceptance (docs/SESSION.md 动作: 从哪台设备发都行):
//! gamma — a phone-shaped one-shot device — sees a transfer it is not
//! part of, approves it, and the recipient machine executes the pull.
//! Real UDP between two serves plus a third dialer; wire-level
//! controls; every await under a timeout.
//!
//! It sees it because the offer is a replicated file message, not
//! because anybody reported it — that changed on 2026-08-23 along with
//! which seat may say 待批, and the block below is where both are
//! pinned.

use std::fs;
use std::path::Path;
use std::time::Duration;

use khor_core::State;
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
async fn a_third_device_sees_the_row_and_its_approval_runs_on_the_recipient() {
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

    // alpha offers a file to beta's window; both sync the summary in.
    let payload: Vec<u8> = (0..400_000u32).map(|i| (i % 249) as u8).collect();
    let src = ra.join("plan.pdf");
    fs::write(&src, &payload).unwrap();
    let tid = a.send("beta", &src).unwrap();
    timeout(Duration::from_secs(PATIENCE), b.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();
    timeout(Duration::from_secs(PATIENCE), g.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();

    // gamma is neither sender nor recipient. Two things changed here on
    // 2026-08-23 (`419e5ad`, 传输并入对话批) and this block had asserted
    // the pre-08-23 shape ever since, red the whole time:
    //
    // - **The offer reaches gamma by replication, not by report.** It is
    //   a file message in a CRDT everybody carries, so gamma derives its
    //   own row (`ChatKind::reportable` is false) and `source` is None.
    // - **And gamma must NOT say 待批.** 待批 comes from the seat whose
    //   approval is being waited on and no other (docs/SESSION.md 六词;
    //   `TransferKind::urgency`'s `i_receive`). gamma cannot clear that
    //   badge by answering — it is beta's approval that is wanted — and
    //   a badge its holder cannot clear breaks 角标可归零.
    //
    // So the assertion is the rule itself: gamma sees the offer, and
    // wears the ordinary unseen-content word for it.
    assert!(
        g.transfer_is_here(&tid),
        "the offer must replicate to a third device"
    );
    let views = g.sessions().unwrap();
    let view = views
        .iter()
        .find(|v| v.session.id.0 == "chat/beta")
        .expect("the third device derives beta's chat row like any other");
    assert_ne!(
        view.session.state.state,
        State::Blocked,
        "待批 belongs to the machine whose approval is wanted, and that is not gamma"
    );
    // Which word it does wear is left unasserted on purpose — measured
    // here as 空闲 where docs/SESSION.md says 完成/未看, for a reason that
    // has nothing to do with transfers or with third devices. The note in
    // `transfer.rs` beside the same measurement has it.
    assert!(
        view.source.is_none(),
        "a replicated document is derived here, not reported: {:?}",
        view.source
    );

    // gamma approves; the pull runs on beta, the payload comes off alpha.
    let (moved, said) = timeout(Duration::from_secs(60), g.accept(&tid))
        .await
        .expect("a routed accept must not hang")
        .unwrap();
    assert_eq!(
        moved,
        payload.len() as u64,
        "the pull should move the whole payload"
    );
    let landed = fs::read_dir(rb.join("khor-drop"))
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .find(|p| !p.file_name().unwrap().to_string_lossy().starts_with('.'))
        .expect("the payload should land on beta");
    assert_eq!(
        fs::read(&landed).unwrap(),
        payload,
        "bytes verbatim on the recipient"
    );
    // The path came back across two hops to a machine that has no such
    // file and no such directory — so this compares what the wire said
    // against what is on beta's disk, which is the only way to tell a
    // travelled answer from one gamma computed under its own root.
    assert_eq!(
        said,
        vec![landed.display().to_string()],
        "the recipient's own path must travel back to the device that asked"
    );

    // Approving twice is idempotent — the second run moves nothing.
    let (again, said_again) = timeout(Duration::from_secs(60), g.accept(&tid))
        .await
        .expect("a repeated accept must not hang")
        .unwrap();
    assert_eq!(again, 0, "a payload already landed moves nothing");
    // …and still says where it is. A file that arrived on an earlier
    // run is no less present, and a caller pointing something at it
    // would otherwise be told nothing on every run but the first.
    assert_eq!(
        said_again, said,
        "an already-landed payload still names its path"
    );

    // **The outcome does not travel to the third device any more, and
    // that is the design rather than a loss.** This block used to sync
    // gamma again and read the word off the transfer's own row — a row
    // that WAS reportable. Since 传输并入对话批 (`419e5ad`) the word
    // lives on a chat row, and chat rows are never reported: the
    // document replicates, so every device derives its own
    // (`ChatKind::reportable` returns false, with that reason on it).
    // What landed on beta's disk is beta's own business.
    //
    // So gamma learns the outcome exactly one way, and it already did,
    // above: `accept` answered with the bytes moved and the recipient's
    // own paths. Re-asserting it after a sync would be asserting that a
    // sync changed nothing.
    timeout(Duration::from_secs(PATIENCE), g.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();
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
                action: "accept".into(),
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
