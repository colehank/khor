//! Real-connection acceptance for the transfer kind (docs/NET.md): the
//! summary travels the CRDT, the receiver pulls the payload with nobody
//! asked (STORE-BRIEF phase 2: 直接传), into its receive location or a
//! directory the sender chose; a partial resumes instead of restarting,
//! a cancel keeps nothing, and the digest is the last word — with
//! controls, every await under a timeout.

mod util;

use std::fs;
use std::path::Path;
use std::time::Duration;

use khor_core::State;
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

    // alpha offers a multi-slice file addressed to beta's window.
    let payload: Vec<u8> = (0..1_600_000u32).map(|i| (i % 251) as u8).collect();
    let src = ra.join("big.bin");
    fs::write(&src, &payload).unwrap();
    let tid = a.send("beta", &src, "").unwrap();

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
    let rows = b.sessions().unwrap();
    let row = &rows
        .iter()
        .find(|v| v.session.id.0 == "chat/beta")
        .expect("the machine's chat row exists")
        .session;
    assert_ne!(row.state.state, State::Blocked, "a file on its way waits on nobody: never 待批");
    assert_eq!(
        b.log_faced("beta").unwrap().1.last().copied().flatten(),
        Some(State::Busy),
        "its chip says 忙碌 until it lands"
    );
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
    // Where it landed: the receive directory, under its own name, found
    // again by its digest (`khor_node::road`).
    let landed = |f: &khor_sync::chat::FileRef| khor_node::road::landed(&rb, &dir, f).expect("landed");
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
    let fate = m.fate.clone().expect("the receiver wrote how the move stands");
    assert_eq!(fate.step, khor_sync::chat::Step::Landed);
    assert_eq!(fate.landed, vec![landed(f).display().to_string()]);
    assert_eq!(fate.moved, payload.len() as i64);

    // The landing lifts the pressure off the conversation's row
    // (传输并入对话批 — the pressure was on this row, so this row is
    // where it lifts).
    let chat_row = |n: &Node| {
        n.sessions()
            .unwrap()
            .into_iter()
            .find(|v| v.session.id.0 == "chat/beta")
            .expect("the machine's chat row exists")
            .session
    };
    assert_ne!(chat_row(&b).state.state, State::Busy, "landed presses nothing");

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

    // The sender's seat: its own row never says 待批 — nobody waits on
    // anybody's approval for a conversation's file any more.
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
        "nothing to approve on either seat"
    );

    // Resume: plant a partial of the first 100k — the second pull moves
    // only the missing tail, and the result is still verbatim.
    fs::remove_file(landed(f)).unwrap();
    fs::write(khor_node::road::partial_of(&rb, &f.digest).unwrap(), &payload[..100_000]).unwrap();
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
        !khor_node::road::partial_of(&rb, &f.digest).unwrap().exists(),
        "the poisoned partial must be discarded"
    );
    let failed = b.log("beta").unwrap().messages.into_iter().find(|x| x.id == m.id).unwrap().fate.unwrap();
    assert_eq!(failed.step, khor_sync::chat::Step::Failed, "a failure is on the message");
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
    let tid2 = a.send("beta", &src2, "").unwrap();
    fs::write(&src2, b"0123456789ABC").unwrap();
    timeout(Duration::from_secs(20), b.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();
    let err = timeout(Duration::from_secs(60), b.take(&tid2))
        .await
        .expect("a refused take must not hang either")
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
                "",
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
    assert_eq!(
        err,
        khor_catalog::msg::offer_lost(),
        "an unrecorded digest is refused by name"
    );

    serve.abort();
}

/// The message with this transfer id, as `n` reads it.
fn message(n: &Node, machine: &str, tid: &SessionId) -> khor_sync::chat::Message {
    let id = tid.0.strip_prefix("transfer/").unwrap();
    n.log(machine).unwrap().messages.into_iter().find(|m| m.id == id).expect("the message is here")
}

/// **A directory the sender chose on the receiver** is where the file
/// lands, and the message says so; a chosen directory that is not there
/// fails in a plain sentence on the message, and nothing lands elsewhere.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_chosen_directory_is_where_it_lands() {
    let ra = root("into-a");
    let rb = root("into-b");
    let server = Node::open_as(ra.to_path_buf(), "alpha").unwrap();
    let serve = tokio::spawn(async move { server.serve().await });
    wait_for_endpoint_file(&ra).await;
    let a = Node::open_as(ra.to_path_buf(), "alpha").unwrap();
    let ticket = a.invite().unwrap();
    let b = Node::open_as(rb.to_path_buf(), "beta").unwrap();
    timeout(Duration::from_secs(45), b.pair(&ticket)).await.expect("pairing must not hang").unwrap();

    let chosen = rb.join("projects").join("in");
    fs::create_dir_all(&chosen).unwrap();
    let src = ra.join("notes.md");
    fs::write(&src, b"# notes\n").unwrap();
    let tid = a.send("beta", &src, &chosen.display().to_string()).unwrap();
    timeout(Duration::from_secs(45), b.sync_now()).await.expect("sync must not hang").unwrap();
    assert_eq!(message(&b, "beta", &tid).into, chosen.display().to_string(), "the target travels on the message");
    let (_, landed) = timeout(Duration::from_secs(60), b.take(&tid)).await.expect("take must not hang").unwrap();
    let want = chosen.join("notes.md");
    assert_eq!(landed, vec![want.display().to_string()]);
    assert_eq!(fs::read(&want).unwrap(), b"# notes\n");
    assert!(!rb.join("khor-drop").join("notes.md").exists(), "not in the default as well");
    assert_eq!(message(&b, "beta", &tid).fate.unwrap().landed, landed);

    // The same bytes sent again to the default: copied from where they
    // are, nothing pulled.
    let tid = a.send("beta", &src, "").unwrap();
    timeout(Duration::from_secs(45), b.sync_now()).await.expect("sync must not hang").unwrap();
    let (moved, landed) = timeout(Duration::from_secs(60), b.take(&tid)).await.expect("take must not hang").unwrap();
    assert_eq!((moved, landed), (0, vec![rb.join("khor-drop").join("notes.md").display().to_string()]));

    // A directory that is not there: a plain failure on the message.
    let gone = rb.join("nowhere");
    let src2 = ra.join("b.txt");
    fs::write(&src2, b"bee").unwrap();
    let tid = a.send("beta", &src2, &gone.display().to_string()).unwrap();
    timeout(Duration::from_secs(45), b.sync_now()).await.expect("sync must not hang").unwrap();
    let err = timeout(Duration::from_secs(60), b.take(&tid)).await.expect("take must not hang").unwrap_err();
    assert_eq!(err, khor_catalog::msg::no_such_dir(gone.display()));
    let fate = message(&b, "beta", &tid).fate.unwrap();
    assert_eq!((fate.step, fate.why), (khor_sync::chat::Step::Failed, err));
    assert!(!gone.exists(), "nothing made where nobody asked");

    serve.abort();
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
    timeout(Duration::from_secs(45), b.pair(&ticket)).await.expect("pairing must not hang").unwrap();

    let payload: Vec<u8> = (0..64_000_000u32).map(|i| (i % 253) as u8).collect();
    let src = ra.join("large.bin");
    fs::write(&src, &payload).unwrap();
    let tid = a.send("beta", &src, "").unwrap();
    timeout(Duration::from_secs(45), b.sync_now()).await.expect("sync must not hang").unwrap();
    let digest = blake3::hash(&payload).to_hex().to_string();
    let key = khor_node::road::sent_key(tid.0.strip_prefix("transfer/").unwrap(), b.device_str());

    let pulling = {
        let (b, tid) = (b.clone(), tid.clone());
        tokio::spawn(async move { b.take(&tid).await })
    };
    // The cancel lands once bytes are moving, as a person's × would.
    timeout(Duration::from_secs(60), async {
        loop {
            if khor_node::road::peek(&key).is_some_and(|j| j.moved.load(std::sync::atomic::Ordering::Relaxed) > 0) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .expect("the pull must start");
    khor_node::road::stop(&key);
    let err = timeout(Duration::from_secs(60), pulling).await.expect("a cancelled pull must end").unwrap().unwrap_err();
    assert_eq!(err, khor_node::road::cancelled());
    assert!(!khor_node::road::partial_of(&rb, &digest).unwrap().exists(), "no partial kept");
    assert!(!rb.join("khor-drop").join("large.bin").exists(), "nothing landed");
    assert_eq!(message(&b, "beta", &tid).fate.unwrap().step, khor_sync::chat::Step::Stopped);

    let (moved, landed) = timeout(Duration::from_secs(120), b.take(&tid)).await.expect("a retry must not hang").unwrap();
    assert_eq!(moved, payload.len() as u64, "a retry after a cancel starts from nothing");
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
    let mine = |n: &Node| n.devices().unwrap().into_iter().find(|d| d.id == alpha).unwrap();
    assert_eq!(mine(&a).receive_at.as_deref(), Some(want.as_str()), "alpha resolved it as the sync arrived");
    assert_eq!(mine(&a).receive_for.as_deref(), Some("~/inbox"));
    assert!(ra.join("inbox").is_dir(), "and made it");

    timeout(Duration::from_secs(45), b.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();
    assert_eq!(mine(&b).receive_at.as_deref(), Some(want.as_str()), "beta reads where alpha's files land now");

    serve.abort();
}

/// A line told with `tell_now` is on the other machine when the call
/// returns — a round started by the line itself, not by the beat (30 s
/// on a machine without a face), and not by a sync the test makes.
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

    timeout(Duration::from_secs(45), b.tell_now("alpha", "are you there"))
        .await
        .expect("tell_now must not hang")
        .unwrap();
    // No sync here: the round the line started brought it across.
    let said = a.log("alpha").unwrap().messages;
    assert!(
        said.iter().any(|m| matches!(&m.body, MsgBody::Text(t) if t == "are you there")),
        "alpha reads beta's line at once"
    );

    serve.abort();
}
