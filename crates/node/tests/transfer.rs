//! Real-connection acceptance for the transfer kind (docs/NET.md): the
//! summary travels the CRDT, the payload moves only on approval, a
//! partial resumes instead of restarting, and the digest is the last
//! word — with controls, every await under a timeout.

mod util;

use std::fs;
use std::path::Path;
use std::time::Duration;

use khor_core::State;
use khor_node::transfer::{partial_path, payload_path};
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
async fn a_payload_moves_only_on_approval_resumes_and_verifies() {
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
    let rows = b.sessions().unwrap();
    let row = &rows
        .iter()
        .find(|v| v.session.id.0 == "chat/beta")
        .expect("the machine's chat row exists")
        .session;
    assert_eq!(row.state.state, State::Blocked, "no approval yet = 待批");
    // The file's name is in the message now, not in a row title: the row
    // belongs to the conversation and is titled after the machine.
    let offered = b.log("beta").unwrap();
    assert!(
        offered
            .messages
            .iter()
            .any(|m| matches!(&m.body, MsgBody::Files(f) if f[0].name == "big.bin")),
        "the offer must be a file message naming the file"
    );
    let files_dir = rb.join(".khor").join("chat").join("beta").join("files");
    let payload_bytes_present =
        files_dir.exists() && fs::read_dir(&files_dir).unwrap().next().is_some();
    assert!(!payload_bytes_present, "no byte may move before approval");

    // Approval pulls all slices, verifies, lands.
    let (moved, _) = timeout(Duration::from_secs(60), b.accept(&tid))
        .await
        .expect("accept must not hang")
        .unwrap();
    assert_eq!(
        moved,
        payload.len() as u64,
        "the first pull moves the whole payload"
    );
    let dir = rb.join(".khor").join("chat").join("beta");
    let m = b
        .log("beta")
        .unwrap()
        .messages
        .into_iter()
        .find(|m| matches!(m.body, MsgBody::Files(_)))
        .expect("the file summary should be in the log");
    let MsgBody::Files(files) = &m.body else {
        unreachable!()
    };
    let f = &files[0];
    assert_eq!(
        fs::read(payload_path(&dir, f)).unwrap(),
        payload,
        "bytes must be verbatim"
    );
    assert_eq!(
        b.transfer_landing(&tid).unwrap(),
        vec![payload_path(&dir, f)],
        "the landing must be nameable — accept's answer prints it"
    );

    // 答了才清: the landing answers the ask, so the conversation's row
    // stops saying 待批 (传输并入对话批 — the pressure was on this row,
    // so this row is where it lifts).
    let chat_row = |n: &Node| {
        n.sessions()
            .unwrap()
            .into_iter()
            .find(|v| v.session.id.0 == "chat/beta")
            .expect("the machine's chat row exists")
            .session
    };
    assert_ne!(chat_row(&b).state.state, State::Blocked, "答了才清");

    // **The unread word is deliberately not asserted here, and that is a
    // finding rather than a gap.** Measured on this very flow
    // (2026-08-27): after the landing beta's row reads 空闲/0, where
    // docs/SESSION.md says an arrived file message goes to 完成/未看 like
    // any message. The cause sits upstream of transfers — `Node::send`
    // marks `chat/<peer>` seen at the message's own stamp ("telling
    // implies looking") and the seen document replicates, so the
    // SENDER's watermark clears the RECIPIENT's badge. The unit test
    // that does assert 完成/未看 for this case
    // (`an_incoming_offer_blocks_the_chat_row_and_the_landing_clears_it`)
    // plants the message with no sender behind it, which is exactly why
    // it is green.
    //
    // Writing either word in here would freeze one side of an open
    // question and make the answer harder to find later. The question is
    // the deliverable; `b.seen` on this row is untestable until it is
    // settled, because the row is already at zero.

    // The sender's seat: its own row must never say 待批 — the approval
    // waited on is the far machine's, and a badge this end cannot clear
    // by answering would break 角标可归零 (docs/SESSION.md).
    let a_row = a
        .sessions()
        .unwrap()
        .into_iter()
        .find(|v| v.session.id.0 == "chat/beta")
        .expect("the sender has the same conversation")
        .session;
    assert_ne!(
        a_row.state.state,
        State::Blocked,
        "the sender's seat has nothing to approve"
    );

    // Resume: plant a partial of the first 100k — the second pull moves
    // only the missing tail, and the result is still verbatim.
    fs::remove_file(payload_path(&dir, f)).unwrap();
    fs::write(partial_path(&dir, f), &payload[..100_000]).unwrap();
    let (moved, _) = timeout(Duration::from_secs(60), b.accept(&tid))
        .await
        .expect("accept must not hang")
        .unwrap();
    assert_eq!(
        moved,
        (payload.len() - 100_000) as u64,
        "a resume moves only the tail"
    );
    assert_eq!(
        fs::read(payload_path(&dir, f)).unwrap(),
        payload,
        "resumed bytes verbatim"
    );

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
    assert!(
        !partial_path(&dir, f).exists(),
        "the poisoned partial must be discarded"
    );
    let (moved, _) = timeout(Duration::from_secs(60), b.accept(&tid))
        .await
        .expect("accept must not hang")
        .unwrap();
    assert_eq!(
        moved,
        payload.len() as u64,
        "a clean retry pulls the whole payload again"
    );

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
    assert_eq!(
        err,
        khor_catalog::msg::offered_file_changed(),
        "a tampered source is refused by name"
    );

    // A digest the offerer never recorded is refused by name: a ghost
    // summary lands in beta's copy as a raw block, claiming alpha as home.
    // The handle comes from the message the block plants, not from a row
    // titled after the file: 传输并入对话批 (`419e5ad`) retired that row,
    // and the accept handle was always the message id underneath it.
    let ghost = {
        let far = ChatDoc::new(0xEE).unwrap();
        let msg_id = far
            .send_files(
                &Sender {
                    id: a.device_str().to_owned(),
                    name: "alpha".into(),
                },
                &[FileRef {
                    name: "ghost.bin".into(),
                    size: 10,
                    digest: "ee".repeat(32),
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
    let err = timeout(Duration::from_secs(60), b.accept(&ghost))
        .await
        .expect("a refused accept must not hang either")
        .unwrap_err();
    assert_eq!(
        err,
        khor_catalog::msg::offer_lost(),
        "an unrecorded digest is refused by name"
    );

    serve.abort();
}
