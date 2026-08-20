//! Real-connection acceptance for the transfer kind (docs/NET.md): the
//! summary travels the CRDT, the payload moves only on approval, a
//! partial resumes instead of restarting, and the digest is the last
//! word — with controls, every await under a timeout.

use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use khor_core::State;
use khor_node::transfer::{partial_path, payload_path};
use khor_node::{MsgBody, Node};
use khor_sync::chat::{ChatDoc, FileRef, Sender};
use tokio::time::timeout;

fn root(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("khor-xfer-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

async fn wait_for_endpoint_file(root: &PathBuf) {
    // 90s, not 10: on a Mac operated over ssh, a freshly compiled test
    // binary is a new face to macOS — its first network-stack touch
    // (interface enumeration, SystemConfiguration) hangs 20-40s per
    // process, waiting on an authorization prompt nobody can ever click
    // (#73, sampled 2026-08-20).
    // That hang is synchronous, which is also why these tests run on a
    // multi-thread runtime: on current_thread it froze tokio's clock
    // and every timeout in here silently stretched with it.
    let path = root.join(".khor").join("endpoint.json");
    timeout(Duration::from_secs(90), async {
        while !path.exists() {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("serve should write endpoint.json within 10s");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_payload_moves_only_on_approval_resumes_and_verifies() {
    let ra = root("a");
    let rb = root("b");
    let server = Node::open_as(ra.clone(), "alpha").unwrap();
    let serve = tokio::spawn(async move { server.serve().await });
    wait_for_endpoint_file(&ra).await;
    let a = Node::open_as(ra.clone(), "alpha").unwrap();
    let ticket = a.invite().unwrap();
    let b = Node::open_as(rb.clone(), "beta").unwrap();
    timeout(Duration::from_secs(45), b.pair(&ticket))
        .await
        .expect("pairing must not hang")
        .unwrap();

    // alpha offers a multi-slice file addressed to beta's window.
    let payload: Vec<u8> = (0..1_600_000u32).map(|i| (i % 251) as u8).collect();
    let src = ra.join("big.bin");
    fs::write(&src, &payload).unwrap();
    let tid = a.send("beta", &src).unwrap();

    // The summary reaches beta on a sync; not one payload byte does.
    timeout(Duration::from_secs(45), b.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();
    let rows = b.sessions().unwrap();
    let row = &rows.iter().find(|v| v.session.id == tid).expect("the summary should make a row").session;
    assert_eq!(row.state.state, State::Blocked, "no approval yet = 待批");
    assert_eq!(row.title, "big.bin");
    let files_dir = rb.join(".khor").join("chat").join("beta").join("files");
    let payload_bytes_present = files_dir.exists()
        && fs::read_dir(&files_dir).unwrap().next().is_some();
    assert!(!payload_bytes_present, "no byte may move before approval");

    // Approval pulls all slices, verifies, lands.
    let moved = timeout(Duration::from_secs(60), b.accept(&tid))
        .await
        .expect("accept must not hang")
        .unwrap();
    assert_eq!(moved, payload.len() as u64, "the first pull moves the whole payload");
    let dir = rb.join(".khor").join("chat").join("beta");
    let m = b
        .log("beta")
        .unwrap()
        .messages
        .into_iter()
        .find(|m| matches!(m.body, MsgBody::Files(_)))
        .expect("the file summary should be in the log");
    let MsgBody::Files(files) = &m.body else { unreachable!() };
    let f = &files[0];
    assert_eq!(fs::read(payload_path(&dir, f)).unwrap(), payload, "bytes must be verbatim");
    assert_eq!(
        b.transfer_landing(&tid).unwrap(),
        vec![payload_path(&dir, f)],
        "the landing must be nameable — accept's answer prints it"
    );

    let rows = b.sessions().unwrap();
    let row = &rows.iter().find(|v| v.session.id == tid).unwrap().session;
    assert_eq!((row.state.state, row.unread), (State::Done, 1), "landed = 完成/未读");
    b.seen(&tid).unwrap();
    let rows = b.sessions().unwrap();
    let row = &rows.iter().find(|v| v.session.id == tid).unwrap().session;
    assert_eq!((row.state.state, row.unread), (State::Idle, 0), "looked at = 空闲");

    // The sender's row rides its served slices to the same word.
    let a_rows = a.sessions().unwrap();
    let a_row = &a_rows.iter().find(|v| v.session.id == tid).expect("the sender should have a row").session;
    assert_eq!(a_row.state.state, State::Done, "the final slice served = 完成 on the sender");

    // Resume: plant a partial of the first 100k — the second pull moves
    // only the missing tail, and the result is still verbatim.
    fs::remove_file(payload_path(&dir, f)).unwrap();
    fs::write(partial_path(&dir, f), &payload[..100_000]).unwrap();
    let moved = timeout(Duration::from_secs(60), b.accept(&tid))
        .await
        .expect("accept must not hang")
        .unwrap();
    assert_eq!(moved, (payload.len() - 100_000) as u64, "a resume moves only the tail");
    assert_eq!(fs::read(payload_path(&dir, f)).unwrap(), payload, "resumed bytes verbatim");

    // ── control groups ─────────────────────────────────────
    // A poisoned partial cannot sneak through: right length, wrong bytes
    // — the digest fails, the partial is discarded, a clean retry works.
    fs::remove_file(payload_path(&dir, f)).unwrap();
    let mut bad = payload[..100_000].to_vec();
    bad[0] ^= 0xff;
    fs::write(partial_path(&dir, f), &bad).unwrap();
    let err = timeout(Duration::from_secs(60), b.accept(&tid))
        .await
        .expect("a failing accept must not hang either")
        .unwrap_err();
    let probe = khor_catalog::msg::digest_mismatch('\u{0}', '\u{0}');
    assert!(
        err.contains(probe.split('\u{0}').next().unwrap()),
        "wrong bytes must fail the digest: {err}"
    );
    assert!(!partial_path(&dir, f).exists(), "the poisoned partial must be discarded");
    let moved = timeout(Duration::from_secs(60), b.accept(&tid))
        .await
        .expect("accept must not hang")
        .unwrap();
    assert_eq!(moved, payload.len() as u64, "a clean retry pulls the whole payload again");

    // A source tampered after the offer is refused before bytes move.
    let src2 = ra.join("small.bin");
    fs::write(&src2, b"0123456789").unwrap();
    let tid2 = a.send("beta", &src2).unwrap();
    fs::write(&src2, b"0123456789ABC").unwrap();
    timeout(Duration::from_secs(20), b.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();
    let err = timeout(Duration::from_secs(60), b.accept(&tid2))
        .await
        .expect("a refused accept must not hang either")
        .unwrap_err();
    assert_eq!(err, khor_catalog::msg::OFFERED_FILE_CHANGED, "a tampered source is refused by name");

    // A digest the offerer never recorded is refused by name: a ghost
    // summary lands in beta's copy as a raw block, claiming alpha as home.
    {
        let far = ChatDoc::new(0xEE).unwrap();
        far.send_files(
            &Sender { id: a.device_str().to_owned(), name: "alpha".into() },
            &[FileRef { name: "ghost.bin".into(), size: 10, digest: "ee".repeat(32) }],
        )
        .unwrap();
        let block = far.changes_since(&Default::default()).unwrap();
        fs::write(dir.join("u-00000000000000ee-00000000.loro"), &block).unwrap();
    }
    let ghost = b
        .sessions()
        .unwrap()
        .into_iter()
        .find(|v| v.session.title == "ghost.bin")
        .expect("the ghost summary should make a row");
    let err = timeout(Duration::from_secs(60), b.accept(&ghost.session.id))
        .await
        .expect("a refused accept must not hang either")
        .unwrap_err();
    assert_eq!(err, khor_catalog::msg::OFFER_LOST, "an unrecorded digest is refused by name");

    serve.abort();
    for r in [&ra, &rb] {
        let _ = fs::remove_dir_all(r);
    }
}
