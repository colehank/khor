//! Real-connection acceptance for the live kind (docs/SESSION.md): a
//! session backed by a real process on its home machine becomes a row on
//! another device through peer reports — with its source attached — and
//! the cross-device seen loop closes over the CRDT. Controls: the
//! missing-ending rule, remote close refused by name, and Act kept off
//! live sessions at the wire. Every await under a timeout.

use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use khor_core::State;
use khor_node::proto::{Request, Response};
use khor_node::{Node, SessionId};
use tokio::time::timeout;

mod util;

fn root(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("khor-lv-{tag}-{}", std::process::id()));
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
async fn a_live_row_travels_as_a_report_and_the_seen_loop_closes() {
    let ra = root("a");
    let rb = root("b");
    let sa = Node::open_as(ra.clone(), "alpha").unwrap();
    let serve_a = tokio::spawn(async move { sa.serve().await });
    wait_for_endpoint_file(&ra).await;

    let a = Node::open_as(ra.clone(), "alpha").unwrap();
    let b = Node::open_as(rb.clone(), "beta").unwrap();
    let ticket = a.invite().unwrap();
    timeout(Duration::from_secs(15), b.pair(&ticket))
        .await
        .expect("pairing must not hang")
        .unwrap();

    // The agent: a real process alpha watches. Registered blocked — the
    // word whose whole point is being seen from elsewhere.
    let mut agent = std::process::Command::new("sleep").arg("60").spawn().unwrap();
    let id = SessionId("tui/agent1".to_owned());
    let ephemeral = a.open_ephemeral("tui", "the agent").unwrap();
    a.report_state(&ephemeral, State::Busy).unwrap();
    {
        // Register the watched one through the same public surface the
        // wrapper uses: open, then bind the pid.
        use khor_node::live::LiveKind;
        let k = LiveKind::new(ra.clone(), a.device());
        k.register(&id, "tui", "claude in proj", Some(agent.id())).unwrap();
        k.report(&id, State::Blocked).unwrap();
    }

    // beta is not home: its row can only be a report, and says so.
    timeout(Duration::from_secs(20), b.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();
    let views = b.sessions().unwrap();
    let view = views
        .iter()
        .find(|v| v.session.id == id)
        .expect("the live row should reach the other device");
    assert_eq!(view.session.state.state, State::Blocked, "等准许,从别的设备看得到");
    assert_eq!(view.session.title, "claude in proj");
    let (reporter, _) = view.source.as_ref().expect("a reported row must carry its source");
    assert_eq!(reporter, "alpha");
    assert!(
        views.iter().any(|v| v.session.id == ephemeral && v.session.state.state == State::Busy),
        "the ephemeral row travels too"
    );

    // The turn ends; the seen loop closes across devices: beta looks,
    // the watermark replicates, home derives 空闲, beta sees 空闲.
    a.report_state(&id, State::Done).unwrap();
    timeout(Duration::from_secs(20), b.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();
    let views = b.sessions().unwrap();
    let view = views.iter().find(|v| v.session.id == id).unwrap();
    assert_eq!(
        (view.session.state.state, view.session.unread),
        (State::Done, 1),
        "turn done, unread on every device"
    );
    b.seen(&id).unwrap();
    timeout(Duration::from_secs(20), b.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();
    let views = b.sessions().unwrap();
    let view = views.iter().find(|v| v.session.id == id).unwrap();
    assert_eq!(
        (view.session.state.state, view.session.unread),
        (State::Idle, 0),
        "looked at on beta clears the row everywhere"
    );

    // Remote close is not a thing yet — refused by name, pointing home.
    let err = b.close(&id).unwrap_err();
    assert!(err.contains("alpha"), "remote close should name the home: {err}");

    // The missing-ending rule: the agent dies, nobody records an exit,
    // the row turns 失败 — and the far side sees it.
    agent.kill().unwrap();
    agent.wait().unwrap();
    let row = a
        .sessions()
        .unwrap()
        .into_iter()
        .find(|v| v.session.id == id)
        .unwrap()
        .session;
    assert_eq!(row.state.state, State::Failed, "a dead process with no recorded ending");
    timeout(Duration::from_secs(20), b.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();
    let views = b.sessions().unwrap();
    let view = views.iter().find(|v| v.session.id == id).unwrap();
    assert_eq!(view.session.state.state, State::Failed);

    // ── wire control: Act stays off live sessions ───────────
    let alpha_info = b
        .devices()
        .unwrap()
        .into_iter()
        .find(|d| d.name == "alpha")
        .unwrap();
    let beta_key =
        khor_net::identity::load_or_create(&rb.join(".khor").join("identity.key")).unwrap();
    let resp = timeout(
        Duration::from_secs(15),
        util::raw_request(
            beta_key,
            &alpha_info.id,
            &alpha_info.addrs,
            &Request::Act { session: id.0.clone(), action: "accept".into() },
        ),
    )
    .await
    .expect("a refused act must not hang")
    .unwrap();
    match resp {
        Response::Refused { why } => {
            assert!(why.contains("传输"), "accept on a live session must be refused: {why}")
        }
        other => panic!("Act on a live session must be refused, got {other:?}"),
    }

    serve_a.abort();
    for r in [&ra, &rb] {
        let _ = fs::remove_dir_all(r);
    }
}
