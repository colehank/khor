//! Real-connection acceptance for the transfer kind (docs/NET.md): the
//! summary travels the broadcast's CRDT, the receiver pulls the payload
//! into its receive location; a partial resumes instead of restarting, a
//! cancel keeps nothing, and the digest is the last word — with
//! controls, every await under a timeout. Pushes to a named machine
//! (STORE-BRIEF phase 5) are `one_channel.rs`'s.

mod util;

use std::fs;
use std::path::Path;
use std::time::Duration;

use khor_node::wide::BROADCAST;
use khor_node::{MsgBody, Node, SessionId};
use khor_sync::chat::{ChatDoc, FileRef, Sender};
use tokio::time::timeout;

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
    let p = std::env::temp_dir().join(format!("khor-xfer-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    util::hermetic(&p);
    Scratch(p)
}

async fn wait_for_endpoint_file(root: &Path) {
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
async fn a_payload_lands_with_nobody_asked_resumes_and_verifies() {
    let ra = root("a");
    let rb = root("b");
    let server = Node::open_as(ra.to_path_buf(), "alpha").unwrap();
    let serve = tokio::spawn(async move { server.serve().await });
    wait_for_endpoint_file(&ra).await;
    let a = Node::open_as(ra.to_path_buf(), "alpha").unwrap();
    let ticket = a.invite().unwrap();
    let b = Node::open_as(rb.to_path_buf(), "beta").unwrap();
    timeout(Duration::from_secs(45), b.pair(&ticket))
        .await
        .expect("pairing must not hang")
        .unwrap();

    // alpha offers a multi-slice file in the broadcast.
    let payload: Vec<u8> = (0..1_600_000u32).map(|i| (i % 251) as u8).collect();
    let src = ra.join("big.bin");
    fs::write(&src, &payload).unwrap();
    let tid = offer(&a, &src).await;

    // The summary reaches beta on a sync.
    timeout(Duration::from_secs(45), b.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();
    // 传输并入对话批 (2026-08-23, `419e5ad`): a transfer is a file
    // message in the machine's conversation and presses on THAT row —
    // `TransferKind::rows` yields nothing at all any more. That commit
    // updated the unit tests beside it and left the three integration
    // tests still looking for a `transfer/` row (here, `act.rs`,
    // `ipc.rs`), so all three had been red since.
    assert!(
        b.transfer_is_here(&tid),
        "the summary must have reached beta"
    );
    // The file's name is in the message.
    let offered = b.log(BROADCAST).unwrap();
    assert!(
        offered
            .messages
            .iter()
            .any(|m| matches!(&m.body, MsgBody::Files(f) if f[0].name == "big.bin")),
        "the offer must be a file message naming the file"
    );
    // beta has no resident here to start the pull by itself, so the
    // move is asked for: all slices pulled, verified, landed.
    let (moved, _) = timeout(Duration::from_secs(60), b.take(&tid))
        .await
        .expect("take must not hang")
        .unwrap();
    assert_eq!(
        moved,
        payload.len() as u64,
        "the first pull moves the whole payload"
    );
    let dir = khor_sync::chat::wide_dir(&rb, BROADCAST).unwrap();
    let m = b
        .log(BROADCAST)
        .unwrap()
        .messages
        .into_iter()
        .find(|m| matches!(m.body, MsgBody::Files(_)))
        .expect("the file summary should be in the log");
    let MsgBody::Files(files) = &m.body else {
        unreachable!()
    };
    let f = &files[0];
    // Where it landed: the receive directory, under its own name, found
    // again by its digest (`khor_node::road`).
    let landed =
        |f: &khor_sync::chat::FileRef| khor_node::road::landed(&rb, &dir, f).expect("landed");
    assert_eq!(landed(f), rb.join("khor-drop").join("big.bin"));
    assert_eq!(
        fs::read(landed(f)).unwrap(),
        payload,
        "bytes must be verbatim"
    );
    assert_eq!(
        b.transfer_landing(&tid).unwrap(),
        vec![landed(f)],
        "the landing must be nameable — retry's answer prints it"
    );
    // The message carries the landing, for every device to read.
    let fate = m
        .fates
        .get(b.device_str())
        .cloned()
        .expect("the receiver wrote how the move stands");
    assert_eq!(fate.step, khor_sync::chat::Step::Landed);
    assert_eq!(fate.landed, vec![landed(f).display().to_string()]);
    assert_eq!(fate.moved, payload.len() as i64);

    // Resume: plant a partial of the first 100k — the second pull moves
    // only the missing tail, and the result is still verbatim.
    fs::remove_file(landed(f)).unwrap();
    fs::write(
        khor_node::road::partial_of(&rb, &f.digest).unwrap(),
        &payload[..100_000],
    )
    .unwrap();
    let (moved, _) = timeout(Duration::from_secs(60), b.take(&tid))
        .await
        .expect("take must not hang")
        .unwrap();
    assert_eq!(
        moved,
        (payload.len() - 100_000) as u64,
        "a resume moves only the tail"
    );
    assert_eq!(
        fs::read(landed(f)).unwrap(),
        payload,
        "resumed bytes verbatim"
    );

    // ── control groups ─────────────────────────────────────
    // A poisoned partial cannot sneak through: right length, wrong bytes
    // — the digest fails, the partial is discarded, a clean retry works.
    fs::remove_file(landed(f)).unwrap();
    let mut bad = payload[..100_000].to_vec();
    bad[0] ^= 0xff;
    fs::write(khor_node::road::partial_of(&rb, &f.digest).unwrap(), &bad).unwrap();
    let err = timeout(Duration::from_secs(60), b.take(&tid))
        .await
        .expect("a failing take must not hang either")
        .unwrap_err();
    let probe = khor_catalog::msg::digest_mismatch('\u{0}', '\u{0}');
    assert!(
        err.contains(probe.split('\u{0}').next().unwrap()),
        "wrong bytes must fail the digest: {err}"
    );
    assert!(
        !khor_node::road::partial_of(&rb, &f.digest)
            .unwrap()
            .exists(),
        "the poisoned partial must be discarded"
    );
    let failed = b
        .log(BROADCAST)
        .unwrap()
        .messages
        .into_iter()
        .find(|x| x.id == m.id)
        .unwrap()
        .fates[b.device_str()]
    .clone();
    assert_eq!(
        failed.step,
        khor_sync::chat::Step::Failed,
        "a failure is on the message"
    );
    assert!(!failed.why.is_empty(), "with its sentence: {failed:?}");
    let (moved, _) = timeout(Duration::from_secs(60), b.take(&tid))
        .await
        .expect("take must not hang")
        .unwrap();
    assert_eq!(
        moved,
        payload.len() as u64,
        "a clean retry pulls the whole payload again"
    );

    // A source tampered after the offer is refused before bytes move.
    let src2 = ra.join("small.bin");
    fs::write(&src2, b"0123456789").unwrap();
    let tid2 = offer(&a, &src2).await;
    fs::write(&src2, b"0123456789ABC").unwrap();
    timeout(Duration::from_secs(20), b.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();
    let err = timeout(Duration::from_secs(60), b.take(&tid2))
        .await
        .expect("a refused take must not hang either")
        .unwrap_err();
    assert!(
        err.contains(khor_catalog::msg::offered_file_changed()),
        "a tampered source is refused by name, with the source it was asked of: {err}"
    );

    // A digest the offerer never recorded is refused by name: a ghost
    // summary lands in beta's copy as a raw block, claiming alpha as home.
    // The handle comes from the message the block plants, not from a row
    // titled after the file: 传输并入对话批 (`419e5ad`) retired that row,
    // and the accept handle was always the message id underneath it.
    let ghost = {
        let far = ChatDoc::new(0xEE).unwrap();
        let msg_id = far
            .post(
                &Sender {
                    id: a.device_str().to_owned(),
                    name: "alpha".into(),
                },
                "",
                &[],
                &[FileRef {
                    name: "ghost.bin".into(),
                    size: 10,
                    digest: "ee".repeat(32),
                    holder: String::new(),
                }],
            )
            .unwrap();
        let block = far.changes_since(&Default::default()).unwrap();
        fs::write(dir.join("u-00000000000000ee-00000000.loro"), &block).unwrap();
        SessionId(format!("transfer/{msg_id}"))
    };
    // The plant really is visible from here — otherwise the refusal
    // below would be about a message beta never got, which is a
    // different refusal wearing the same words.
    assert!(
        b.transfer_is_here(&ghost),
        "the ghost summary must be readable on beta"
    );
    let err = timeout(Duration::from_secs(60), b.take(&ghost))
        .await
        .expect("a refused take must not hang either")
        .unwrap_err();
    assert!(
        err.contains(khor_catalog::msg::offer_lost()),
        "an unrecorded digest is refused by name: {err}"
    );

    serve.abort();
}

/// The message with this transfer id, as `n` reads it.
fn message(n: &Node, tid: &SessionId) -> khor_sync::chat::Message {
    let id = tid.0.strip_prefix("transfer/").unwrap();
    n.log(BROADCAST)
        .unwrap()
        .messages
        .into_iter()
        .find(|m| m.id == id)
        .expect("the message is here")
}

/// A file offered in the broadcast to nobody in particular (STORE-BRIEF
/// phase 5): its transfer id. Each machine takes it when it is asked to.
async fn offer(n: &Node, src: &Path) -> SessionId {
    let id = timeout(Duration::from_secs(45), n.post("", &[src.to_path_buf()]))
        .await
        .expect("a post must not hang")
        .unwrap();
    khor_node::transfer::TransferKind::session_id(&id)
}

/// **A cancel keeps nothing** (STORE-BRIEF phase 2): a pull stopped
/// while it moves ends before its next slices, its partial goes, the
/// message says 取消 for every device, and a retry starts it again.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_cancelled_pull_leaves_nothing_and_a_retry_lands_it() {
    let ra = root("stop-a");
    let rb = root("stop-b");
    let server = Node::open_as(ra.to_path_buf(), "alpha").unwrap();
    let serve = tokio::spawn(async move { server.serve().await });
    wait_for_endpoint_file(&ra).await;
    let a = Node::open_as(ra.to_path_buf(), "alpha").unwrap();
    let ticket = a.invite().unwrap();
    let b = std::sync::Arc::new(Node::open_as(rb.to_path_buf(), "beta").unwrap());
    timeout(Duration::from_secs(45), b.pair(&ticket))
        .await
        .expect("pairing must not hang")
        .unwrap();

    let payload: Vec<u8> = (0..64_000_000u32).map(|i| (i % 253) as u8).collect();
    let src = ra.join("large.bin");
    fs::write(&src, &payload).unwrap();
    let tid = offer(&a, &src).await;
    timeout(Duration::from_secs(45), b.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();
    let digest = blake3::hash(&payload).to_hex().to_string();
    let key = khor_node::road::sent_key(tid.0.strip_prefix("transfer/").unwrap(), b.device_str());

    let pulling = {
        let (b, tid) = (b.clone(), tid.clone());
        tokio::spawn(async move { b.take(&tid).await })
    };
    // The cancel lands once bytes are moving, as a person's × would.
    timeout(Duration::from_secs(60), async {
        loop {
            if khor_node::road::peek(&key)
                .is_some_and(|j| j.moved.load(std::sync::atomic::Ordering::Relaxed) > 0)
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .expect("the pull must start");
    khor_node::road::stop(&key);
    let err = timeout(Duration::from_secs(60), pulling)
        .await
        .expect("a cancelled pull must end")
        .unwrap()
        .unwrap_err();
    assert_eq!(err, khor_node::road::cancelled());
    assert!(
        !khor_node::road::partial_of(&rb, &digest).unwrap().exists(),
        "no partial kept"
    );
    assert!(
        !rb.join("khor-drop").join("large.bin").exists(),
        "nothing landed"
    );
    assert_eq!(
        message(&b, &tid).fates[b.device_str()].step,
        khor_sync::chat::Step::Stopped
    );

    let (moved, landed) = timeout(Duration::from_secs(120), b.take(&tid))
        .await
        .expect("a retry must not hang")
        .unwrap();
    assert_eq!(
        moved,
        payload.len() as u64,
        "a retry after a cancel starts from nothing"
    );
    assert_eq!(fs::read(&landed[0]).unwrap(), payload);

    serve.abort();
}

/// **A receive location set from another machine is taken on when the
/// sync brings it** (the lead, 2026-10-07): beta sets alpha's, beta's
/// sync pushes it, and alpha's resident resolves it then — the directory
/// made and `receive_at` reported — without alpha receiving anything. A
/// second sync carries the answer back to beta. (It watches the effect:
/// a round's quota reading also opens a node on alpha, and every open
/// follows the setting too, so this stays green with the sync hooks
/// alone taken out.)
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_receive_dir_set_elsewhere_is_resolved_on_sync() {
    let ra = root("follow-a");
    let rb = root("follow-b");
    let server = Node::open_as(ra.to_path_buf(), "alpha").unwrap();
    let serve = tokio::spawn(async move { server.serve().await });
    wait_for_endpoint_file(&ra).await;
    let a = Node::open_as(ra.to_path_buf(), "alpha").unwrap();
    let alpha = a.device_str().to_owned();
    let ticket = a.invite().unwrap();
    let b = Node::open_as(rb.to_path_buf(), "beta").unwrap();
    timeout(Duration::from_secs(45), b.pair(&ticket))
        .await
        .expect("pairing must not hang")
        .unwrap();

    b.set_receive_dir("alpha", "~/inbox").unwrap();
    timeout(Duration::from_secs(45), b.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();
    let want = ra.join("inbox").display().to_string();
    let mine = |n: &Node| {
        n.devices()
            .unwrap()
            .into_iter()
            .find(|d| d.id == alpha)
            .unwrap()
    };
    assert_eq!(
        mine(&a).receive_at.as_deref(),
        Some(want.as_str()),
        "alpha resolved it as the sync arrived"
    );
    assert_eq!(mine(&a).receive_for.as_deref(), Some("~/inbox"));
    assert!(ra.join("inbox").is_dir(), "and made it");

    timeout(Duration::from_secs(45), b.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();
    assert_eq!(
        mine(&b).receive_at.as_deref(),
        Some(want.as_str()),
        "beta reads where alpha's files land now"
    );

    serve.abort();
}

/// A line pushed to a machine is on that machine when the call returns
/// — the push itself is the round, not the beat (30 s on a machine
/// without a face), and not a sync the test makes.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_line_told_now_reaches_the_other_machine_without_waiting_for_the_beat() {
    let ra = root("tell-a");
    let rb = root("tell-b");
    let server = Node::open_as(ra.to_path_buf(), "alpha").unwrap();
    let serve = tokio::spawn(async move { server.serve().await });
    wait_for_endpoint_file(&ra).await;
    let a = Node::open_as(ra.to_path_buf(), "alpha").unwrap();
    let ticket = a.invite().unwrap();
    let b = Node::open_as(rb.to_path_buf(), "beta").unwrap();
    timeout(Duration::from_secs(45), b.pair(&ticket))
        .await
        .expect("pairing must not hang")
        .unwrap();

    timeout(Duration::from_secs(45), b.post("@alpha are you there", &[]))
        .await
        .expect("a post must not hang")
        .unwrap();
    // No sync here: the push brought it across.
    let said = a.log(BROADCAST).unwrap().messages;
    assert!(
        said.iter()
            .any(|m| matches!(&m.body, MsgBody::Text(t) if t == "are you there")),
        "alpha reads beta's line at once"
    );

    serve.abort();
}
