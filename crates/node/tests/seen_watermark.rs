//! Whose badge does a watermark clear? Real-connection acceptance for
//! the read watermark on file messages (docs/NET.md 验收), with
//! controls.
//!
//! **Why this is an integration test with two real nodes.** The unit
//! test that covers the same ground
//! (`an_incoming_offer_blocks_the_chat_row_and_the_landing_clears_it`)
//! plants the incoming offer by writing a foreign block straight into
//! the channel directory — a message with **no sender behind it**. That
//! fixture cannot perform the step that breaks this: the sending
//! machine raising `chat/<peer>`'s watermark as it offers. So that test
//! was green through the whole life of the bug. **A test can be green
//! purely because its fixture cannot reach the fatal step** — the guard
//! is to take the measurement on the seat that actually loses, over a
//! real `send`.
//!
//! **A negative assertion needs a positive one beside it, taken at the
//! same moment off the same probe.** "unread is 0" and "the probe found
//! nothing at all — wrong row, wrong id, nothing rendered" read exactly
//! alike in a test report, and the second one is true of *any* id. So
//! every 0 asserted below is preceded by a reading off the same call
//! that is non-zero, or by a positive assertion that the thing being
//! counted really did arrive. Where a control is itself a 0, it carries
//! its own control above it.
//!
//! What the rule is, and what it is not. The watermark is **about the
//! person**, replicated on purpose (docs/NET.md 已读: 手机看了，桌面角
//! 标同步清), and "telling implies looking" belongs in it: a line this
//! person typed is content this person has seen, on every device they
//! own — `tests/pairing.rs` asserts exactly that and it stays true. A
//! **file offer is not that**. What it announces is not the words in it
//! but a payload that has not arrived anywhere yet, and "arrived" is a
//! fact about a machine, not about a person. Sending one is therefore
//! not looking at one, on any seat.

mod util;

use std::fs;
use std::path::Path;
use std::time::Duration;

use khor_core::State;
use khor_node::{MsgBody, Node, SessionId};
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
    let p = std::env::temp_dir().join(format!("khor-seen-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    util::hermetic(&p);
    Scratch(p)
}

async fn wait_for_endpoint_file(root: &Path) {
    // 90s, not 10: on a Mac operated over ssh a freshly compiled test
    // binary is a new face to macOS, and its first network-stack touch
    // hangs 20-40s waiting on an authorization prompt nobody can ever
    // click. `tests/transfer.rs` has the full reasoning.
    let path = root.join(".khor").join("endpoint.json");
    timeout(Duration::from_secs(90), async {
        while !path.exists() {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("serve should write endpoint.json");
}

fn row(n: &Node, id: &str) -> khor_core::Session {
    n.sessions()
        .unwrap()
        .into_iter()
        .find(|v| v.session.id.0 == id)
        .unwrap_or_else(|| panic!("{id} must be on the list"))
        .session
}

/// The offering machine's watermark is not the receiving machine's
/// answer.
///
/// alpha offers beta a file. On beta the offer is 待批 — that word comes
/// from the seat and never from unread, and it already worked — and the
/// offer is also **content beta has not looked at**. Once the payload
/// lands, 待批 lifts to the row's own message-based reading, which
/// docs/SESSION.md names: 传完的文件不再施压，文件消息落地和普通消息
/// 一样走「新内容未看」那一档. What happened instead: `Node::send`
/// marked `chat/beta` seen at the offer's own stamp, the seen document
/// replicates, and the badge was cleared by the machine that had
/// something to hand over.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_offer_is_unseen_on_the_machine_it_was_offered_to() {
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

    let src = ra.join("drop.bin");
    fs::write(&src, b"the payload").unwrap();
    let tid = a.send("beta", &src).unwrap();
    timeout(Duration::from_secs(45), b.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();

    // ── the measurement ────────────────────────────────────
    assert!(
        b.transfer_is_here(&tid),
        "control: the summary must have reached beta"
    );
    let r = row(&b, "chat/beta");
    assert_eq!(
        r.unread, 1,
        "the offer is content this machine has not looked at: unread must count it"
    );
    assert_eq!(
        r.state.state,
        State::Blocked,
        "control: the seat's own word is 待批 and this fix must not disturb it"
    );

    // The landing answers the ask, so 待批 lifts — to 完成/未看, not to
    // 空闲 (docs/SESSION.md 传输并入对话批).
    timeout(Duration::from_secs(60), b.accept(&tid))
        .await
        .expect("accept must not hang")
        .unwrap();
    let r = row(&b, "chat/beta");
    assert_eq!(
        r.state.state,
        State::Done,
        "传完的文件走「新内容未看」那一档"
    );
    assert_eq!(r.unread, 1, "…and the debt is exactly the one message");

    // ── control: the offering seat does not badge itself ───
    // alpha chose the file and handed it over; nothing on alpha is
    // waiting to be looked at. A fix that merely deleted the mark would
    // put a badge here and fail.
    assert_eq!(
        row(&a, "chat/beta").unread,
        0,
        "control: the machine that offered it has nothing new to look at"
    );

    // ── control: 说话即看过 is untouched, and still person-wide ──
    // A line alpha types into its own window is seen on beta too, once
    // the watermark syncs (the rule `tests/pairing.rs` pins). A fix that
    // made unread device-local — the obvious wrong turn here — would
    // pass everything above and fail this.
    a.tell("alpha", "a note to self").unwrap();
    timeout(Duration::from_secs(45), b.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();
    assert!(
        b.log("alpha")
            .unwrap()
            .messages
            .iter()
            .any(|m| matches!(&m.body, MsgBody::Text(t) if t.contains("note to self"))),
        "control for the control: the line really did travel, so the 0 below means something"
    );
    assert_eq!(
        row(&b, "chat/alpha").unread,
        0,
        "own words never count as unread, on any device"
    );

    // ── control: the badge still reaches zero (docs/UX.md 铁律) ──
    b.seen(&SessionId("chat/beta".into())).unwrap();
    let r = row(&b, "chat/beta");
    assert_eq!((r.unread, r.state.state), (0, State::Idle), "看了就清");

    serve.abort();
    let _ = fs::remove_dir_all(&*ra);
}
