//! The resident serve holds this key's only endpoint; one-shot verbs
//! hand their job over a loopback socket instead of being refused — the
//! exact scenario the transfer batch could only reject by name. Real
//! UDP between two live serves, real hand-offs, controls included.

//!
//! # The clock here is issue #73's, not this file's subject
//!
//! Every deadline below is sized to swallow one full first-touch stall
//! (#73: 20-40s per process on this machine — the comment in
//! `wait_for_endpoint_file` has the mechanism). **Uniformly**, because
//! which call pays it is not deterministic: measured 2026-08-21, a bind
//! cost 21ms and a dial 1.1s while the first real sync cost 37s. The
//! runtime is multi-thread for the other half of #73: that stall is
//! *synchronous*, and on a current-thread runtime it freezes tokio's
//! clock, so every timeout in the file silently stretches with it and a
//! real hang stops looking like one.

use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use khor_core::State;
use khor_node::transfer::payload_path;
use khor_node::{ipc, MsgBody, Node};
use tokio::time::timeout;

fn root(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("khor-ipc-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

async fn wait_for_endpoint_file(root: &PathBuf) {
    // 90s, not 10 (issue #73, sampled 2026-08-20; every real-connection
    // test in this directory timed out for it on 2026-08-21). On a Mac
    // operated over ssh, a freshly compiled test binary is a new face to
    // macOS: its first network-stack touch (interface enumeration,
    // SystemConfiguration) hangs **20-40s per process**, waiting on an
    // authorization prompt nobody can ever click.
    //
    // What the budgets in this file are distinguishing themselves from:
    // a real hang, which is unbounded. 90s against a measured 40s
    // ceiling is more than double, deliberately — a timeout that never
    // fires costs nothing when the test passes, and a flaky red costs a
    // person a trip to find out it was the machine.
    let path = root.join(".khor").join("endpoint.json");
    timeout(Duration::from_secs(90), async {
        while !path.exists() {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("serve should write endpoint.json");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn one_shot_verbs_ride_the_resident_serve() {
    let ra = root("a");
    let rb = root("b");
    let sa = Node::open_as(ra.clone(), "alpha").unwrap();
    let serve_a = tokio::spawn(async move { sa.serve().await });
    wait_for_endpoint_file(&ra).await;
    let sb = Node::open_as(rb.clone(), "beta").unwrap();
    let serve_b = tokio::spawn(async move { sb.serve().await });
    wait_for_endpoint_file(&rb).await;

    let a = Node::open_as(ra.clone(), "alpha").unwrap();
    let b = Node::open_as(rb.clone(), "beta").unwrap();

    // endpoint.json now carries a capability — it must be owner-only.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(rb.join(".khor").join("endpoint.json"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600, "endpoint.json must not be readable by others");
    }

    // Pairing while beta's own serve holds the key: routed, works, and
    // both tables know both machines.
    let ticket = a.invite().unwrap();
    let name = timeout(Duration::from_secs(60), b.pair(&ticket))
        .await
        .expect("routed pairing must not hang")
        .unwrap();
    assert_eq!(name, "alpha");
    let names: Vec<String> = b.devices().unwrap().into_iter().map(|d| d.name).collect();
    assert_eq!(names, vec!["alpha", "beta"], "beta's table should hold both");

    // A transfer end to end with BOTH serves alive: sync and accept ride
    // beta's serve; the payload comes off alpha's serve.
    let payload: Vec<u8> = (0..600_000u32).map(|i| (i % 253) as u8).collect();
    let src = ra.join("bundle.bin");
    fs::write(&src, &payload).unwrap();
    let tid = a.send("beta", &src).unwrap();
    timeout(Duration::from_secs(60), b.sync_now())
        .await
        .expect("routed sync must not hang")
        .unwrap();
    let rows = b.sessions().unwrap();
    let row = &rows.iter().find(|v| v.session.id == tid).expect("the summary should make a row").session;
    assert_eq!(row.state.state, State::Blocked);
    let (moved, _) = timeout(Duration::from_secs(60), b.accept(&tid))
        .await
        .expect("routed accept must not hang")
        .unwrap();
    assert_eq!(moved, payload.len() as u64);
    let dir = rb.join(".khor").join("chat").join("beta");
    let m = b
        .log("beta")
        .unwrap()
        .messages
        .into_iter()
        .find(|m| matches!(m.body, MsgBody::Files(_)))
        .unwrap();
    let MsgBody::Files(files) = &m.body else { unreachable!() };
    assert_eq!(fs::read(payload_path(&dir, &files[0])).unwrap(), payload, "bytes verbatim");

    // ── control groups ─────────────────────────────────────
    // A wrong cookie is refused by name: the port alone opens nothing.
    let text = fs::read_to_string(rb.join(".khor").join("endpoint.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    let port = v["ipc_port"].as_u64().expect("the file should carry the hand-off port") as u16;
    let reply = timeout(
        Duration::from_secs(10),
        ipc::call(port, "not-the-cookie", ipc::Op::SyncNow),
    )
    .await
    .expect("a refused hand-off must not hang")
    .unwrap();
    match reply {
        ipc::Reply::Refused { why } => assert_eq!(why, khor_catalog::msg::HANDOFF_WRONG_COOKIE),
        other => panic!("a wrong cookie must be refused, got {other:?}"),
    }

    // Serve gone → the direct path takes over. The in-process serve task
    // shares our pid, so fake its death: a pid that just exited.
    serve_b.abort();
    tokio::time::sleep(Duration::from_millis(200)).await;
    let dead = {
        let mut c = std::process::Command::new("true").spawn().unwrap();
        let pid = c.id();
        c.wait().unwrap();
        pid
    };
    let mut v: serde_json::Value = serde_json::from_str(&text).unwrap();
    v["pid"] = serde_json::json!(dead);
    fs::write(rb.join(".khor").join("endpoint.json"), v.to_string()).unwrap();
    let outcomes = timeout(Duration::from_secs(60), b.sync_now())
        .await
        .expect("the direct path must not hang")
        .unwrap();
    let (_, verdict) = outcomes
        .iter()
        .find(|(n, _)| n == "alpha")
        .expect("there should be an alpha entry");
    verdict.as_ref().expect("with the key free again, the direct path should work");

    serve_a.abort();
    for r in [&ra, &rb] {
        let _ = fs::remove_dir_all(r);
    }
}
