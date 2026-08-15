//! Third-device acceptance (docs/SESSION.md 动作: 从哪台设备发都行):
//! gamma — a phone-shaped one-shot device — sees a transfer it is not
//! part of through peer reports, approves it, and the recipient machine
//! executes the pull. Real UDP between two serves plus a third dialer;
//! wire-level controls; every await under a timeout.

use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use khor_core::State;
use khor_node::proto::{Request, Response};
use khor_node::Node;
use tokio::time::timeout;

mod util;
use util::raw_request;

fn root(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("khor-act-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
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
async fn a_third_device_sees_the_row_and_its_approval_runs_on_the_recipient() {
    let ra = root("a");
    let rb = root("b");
    let rg = root("g");
    let sa = Node::open_as(ra.clone(), "alpha").unwrap();
    let serve_a = tokio::spawn(async move { sa.serve().await });
    wait_for_endpoint_file(&ra).await;
    let sb = Node::open_as(rb.clone(), "beta").unwrap();
    let serve_b = tokio::spawn(async move { sb.serve().await });
    wait_for_endpoint_file(&rb).await;

    let a = Node::open_as(ra.clone(), "alpha").unwrap();
    let b = Node::open_as(rb.clone(), "beta").unwrap();
    let g = Node::open_as(rg.clone(), "gamma").unwrap();

    let t1 = a.invite().unwrap();
    timeout(Duration::from_secs(15), b.pair(&t1))
        .await
        .expect("pairing must not hang")
        .unwrap();
    let t2 = a.invite().unwrap();
    timeout(Duration::from_secs(15), g.pair(&t2))
        .await
        .expect("pairing must not hang")
        .unwrap();
    // beta learns gamma through the table before gamma ever dials it.
    timeout(Duration::from_secs(20), b.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();

    // alpha offers a file to beta's window; both sync the summary in.
    let payload: Vec<u8> = (0..400_000u32).map(|i| (i % 249) as u8).collect();
    let src = ra.join("plan.pdf");
    fs::write(&src, &payload).unwrap();
    let tid = a.send("beta", &src).unwrap();
    timeout(Duration::from_secs(20), b.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();
    timeout(Duration::from_secs(20), g.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();

    // gamma is neither sender nor recipient — the row it shows can only
    // come from a peer's report, and it says 待批.
    let views = g.sessions().unwrap();
    let view = views
        .iter()
        .find(|v| v.session.id == tid)
        .expect("the reported row should be visible on the third device");
    assert_eq!(view.session.state.state, State::Blocked, "摘要到了,等许可");
    let (reporter, _) = view.source.as_ref().expect("a third device's row must carry its source");
    assert!(
        reporter == "alpha" || reporter == "beta",
        "the row should come from a participant, got {reporter}"
    );

    // gamma approves; the pull runs on beta, the payload comes off alpha.
    let moved = timeout(Duration::from_secs(60), g.accept(&tid))
        .await
        .expect("a routed accept must not hang")
        .unwrap();
    assert_eq!(moved, payload.len() as u64, "the pull should move the whole payload");
    let landed = fs::read_dir(rb.join(".khor").join("chat").join("beta").join("files"))
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .find(|p| !p.file_name().unwrap().to_string_lossy().starts_with('.'))
        .expect("the payload should land on beta");
    assert_eq!(fs::read(&landed).unwrap(), payload, "bytes verbatim on the recipient");

    // Approving twice is idempotent — the second run moves nothing.
    let again = timeout(Duration::from_secs(60), g.accept(&tid))
        .await
        .expect("a repeated accept must not hang")
        .unwrap();
    assert_eq!(again, 0, "a payload already landed moves nothing");

    // After a fresh sync the third device sees the outcome word.
    timeout(Duration::from_secs(20), g.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();
    let views = g.sessions().unwrap();
    let view = views.iter().find(|v| v.session.id == tid).unwrap();
    assert_eq!(view.session.state.state, State::Done, "传完了,第三台也看得到");

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
        Duration::from_secs(15),
        raw_request(
            gamma_key,
            &beta_info.id,
            &beta_info.addrs,
            &Request::Act { session: tid.0.clone(), action: "detonate".into() },
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
        Duration::from_secs(15),
        raw_request(
            iroh::SecretKey::generate(),
            &beta_info.id,
            &beta_info.addrs,
            &Request::Act { session: tid.0.clone(), action: "accept".into() },
        ),
    )
    .await
    .expect("a refused act must not hang")
    .unwrap();
    match resp {
        Response::Refused { why } => assert_eq!(why, khor_catalog::msg::NOT_PAIRED),
        other => panic!("an unpaired act must be refused, got {other:?}"),
    }

    serve_a.abort();
    serve_b.abort();
    for r in [&ra, &rb, &rg] {
        let _ = fs::remove_dir_all(r);
    }
}
