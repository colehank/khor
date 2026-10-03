//! Real-connection acceptance for spending: usage is **asked for**, not
//! replicated (`khor_core::Usage`), so the only way to know it works is
//! to make one machine ask another over real UDP.
//!
//! Two nodes on one machine, real pairing, real sync — and **two
//! different sets of transcripts**, which is the whole design of this
//! check. Two nodes on one machine share a CPU and share memory, so the
//! vitals acceptance next door cannot tell "it came over the wire" from
//! "we both read the same thing"; spending can, because a node reads the
//! transcripts under its own root. Alpha is given one number and beta
//! another, and the assertion is that beta reports **alpha's**.

mod util;

use std::path::Path;
use std::time::Duration;

use khor_node::Node;
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
    let p = std::env::temp_dir().join(format!("khor-usage-net-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    util::hermetic(&p);
    Scratch(p)
}

/// Gives a node's own home one claude transcript, billing `out` tokens.
///
/// Under the node's root rather than through `KHOR_VENDOR_HOME`, because
/// that variable is one setting for the whole process and both nodes here
/// live in it — the isolation this test rests on is the per-root one that
/// `khor_node::adaptor::vendor_home` gives by default.
fn spend(root: &Path, id: &str, out: u64) {
    let dir = root.join(".claude/projects/p");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("s.jsonl"),
        format!(
            r#"{{"type":"assistant","timestamp":"2026-08-17T06:00:00Z","message":{{"id":"{id}","usage":{{"input_tokens":1,"cache_read_input_tokens":0,"cache_creation_input_tokens":0,"output_tokens":{out}}}}}}}
"#
        ),
    )
    .unwrap();
}

fn output_of(u: &khor_core::Usage) -> u64 {
    u.days.iter().map(|d| d.tokens.output).sum()
}

async fn wait_for_endpoint_file(root: &Path) {
    // 90s, not 10 (issue #73, sampled 2026-08-20; every real-connection
    // test in this directory timed out for it on 2026-08-21). On a Mac
    // operated over ssh, a freshly compiled test binary is a new face to
    // macOS: its first network-stack touch hangs **20-40s per process**,
    // waiting on an authorization prompt nobody can ever click. What
    // these budgets distinguish themselves from is a real hang, which is
    // unbounded.
    //
    // **The runtime above is deliberately left single-threaded**, unlike
    // the four plain wire tests that took this same treatment. The note
    // on the test below spells out why the cheapest reactor to stall is
    // the one with a single thread; that judgment is older than #73 and
    // #73 does not touch it. Only the clock moved here.
    let path = root.join(".khor").join("endpoint.json");
    timeout(Duration::from_secs(90), async {
        while !path.exists() {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("serve should write endpoint.json");
}

/// One machine's spending reaches another, is that machine's own, and
/// keeps its numbers while ageing once nobody can be reached.
///
/// The default `#[tokio::test]` runtime is single-threaded, the same
/// judgment the vitals acceptance spells out: the cheapest reactor to
/// stall is the one with a single thread.
///
/// **But the timeout below is hygiene here, not an armed guard, and
/// saying so is the point.** The vitals check really does catch a sample
/// taken on the reactor, because sampling a real machine blocks for
/// 1.9 s whatever the test does. Reading *this* test's transcripts takes
/// microseconds — one file, one line — so an implementation that folded
/// on the reactor would sail through. Arming it would need a fixture big
/// enough to block for twenty seconds, which is not a fixture. What keeps
/// the handler off the reactor is the note on it and the eighteen-second
/// figure behind that note; this test does not check it, and a reader
/// should not think it does.
#[tokio::test]
async fn a_machine_reports_its_own_spending_and_it_keeps_ageing_honestly() {
    let ra = root("a");
    let rb = root("b");
    // Two different trees. 4242 is alpha's and nothing beta can read says
    // it, which is what makes the assertion below about the wire.
    spend(&ra, "msg_alpha", 4242);
    spend(&rb, "msg_beta", 7);

    let alpha = Node::open_as(ra.to_path_buf(), "alpha").unwrap();
    let server = Node::open_as(ra.to_path_buf(), "alpha").unwrap();
    let serve = tokio::spawn(async move { server.serve().await });
    wait_for_endpoint_file(&ra).await;

    let beta = Node::open_as(rb.to_path_buf(), "beta").unwrap();
    let alpha_id = alpha.device_str().to_owned();

    // The control: before pairing, beta has heard from nobody but itself.
    assert_eq!(
        beta.usage_of(&alpha_id),
        None,
        "beta must not have an answer for a machine it has never met"
    );
    // …and the second control, the one the vitals check cannot have:
    // beta's own answer is 7, so 4242 arriving later cannot be beta
    // reading its own files under another name.
    let (mine, mine_age) = beta
        .usage_of(beta.device_str())
        .expect("a node can read its own");
    assert_eq!(output_of(&mine), 7, "beta's own transcript bills 7");
    assert_eq!(
        mine_age, 0,
        "this machine reads on demand, so its answer has no age"
    );

    let ticket = alpha.invite().unwrap();
    timeout(Duration::from_secs(60), beta.pair(&ticket))
        .await
        .expect("pairing must not hang")
        .unwrap();
    timeout(Duration::from_secs(60), beta.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();

    let (theirs, age) = beta
        .usage_of(&alpha_id)
        .expect("alpha's answer should have arrived on the sync visit");
    assert_eq!(
        output_of(&theirs),
        4242,
        "beta must report alpha's spending, not its own: {theirs:?}"
    );
    assert_eq!(theirs.unreadable, 0);
    assert!(
        age < 60_000,
        "the answer should be from this visit, not {age} ms ago"
    );

    // Alpha goes away. What beta shows now is the last thing it heard,
    // and it must say how old that is rather than wearing the present
    // (docs/SESSION.md 离线不是第七个词).
    serve.abort();
    tokio::time::sleep(Duration::from_millis(1200)).await;
    let (still, older) = beta
        .usage_of(&alpha_id)
        .expect("the last answer is still the answer");
    assert_eq!(
        still, theirs,
        "not one number moved while nobody could be reached"
    );
    assert!(
        older > age,
        "…and it is visibly older than it was: {age} ms then, {older} ms now"
    );

    let _ = std::fs::remove_dir_all(&*ra);
}

/// **The second place an old spending answer has to survive, and it is
/// not the wire.**
///
/// A peer's answer is kept on disk as JSON
/// (`Node::cache_peer_usage` writes `.khor/usage/<device>`), and that
/// file outlives the khor that wrote it by however long the machine
/// stands. So the round trip that matters here is **old file, new
/// code** — a different codec from the frame's msgpack, with a
/// different way of spelling "the field is not there", and a wire test
/// alone would have covered neither.
///
/// The old shape is written out by hand on purpose: serialising the
/// current struct and deleting keys would be asking the new code to
/// describe the old one.
#[test]
fn a_cached_answer_written_before_the_model_breakdown_still_reads_back() {
    let r = root("olddisk");
    let node = Node::open_as(r.to_path_buf(), "solo").unwrap();
    let peer = "ab".repeat(32);
    let dir = r.join(".khor").join("usage");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join(&peer),
        r#"{"at_ms":1786928400000,"usage":{"days":[{"day":"2026-08-17","category":"claude","tokens":{"input":1,"cached_input":2,"cache_write":3,"output":4}}],"unreadable":5}}"#,
    )
    .unwrap();

    let (usage, _age) = node
        .usage_of(&peer)
        .expect("the file on disk is the answer");
    assert_eq!(
        output_of(&usage),
        4,
        "what the old khor did record survives whole"
    );
    assert_eq!(usage.unreadable, 5);
    assert!(
        usage.models.is_empty(),
        "an answer from before the breakdown says so by being empty, not by being wrong"
    );
    assert_eq!(usage.today, None, "and states no day rather than today's");
}

/// **The network-wide answer states a day, and it is this machine's.**
///
/// `khor_core::Usage::today` is what a face cuts 「the last seven days」
/// with, and it is the one thing in that payload a face cannot work out
/// for itself: the rows carry civil dates in the *answering* machine's
/// zone and nothing says which zone that is. A `None` here does not
/// leave a face with no window — it leaves it reaching for its own
/// today, which is right on the machine it runs on and a day out about
/// a khor it reached over the web.
///
/// **Asserted against this node's other answer rather than against a
/// constant.** `Node::usage` and `Node::usage_everywhere` are two paths
/// to the same question, and what has to hold is that they agree —
/// a date written out by hand here would be a constant only the author
/// could check, which is the shape this tree keeps refusing.
///
/// **What it does not catch**: a second implementation that cuts the
/// day in a *different* zone. The two answers would then differ only
/// during the hours the zones disagree, so this would be red some of
/// the day and green the rest — the reason both paths go through one
/// `khor_node::usage::today`, which is structural rather than
/// asserted. What is asserted here is the failure that is always
/// available: saying nothing.
#[test]
fn the_network_wide_answer_states_this_machines_day_and_not_nobodys() {
    let r = root("today");
    let node = Node::open_as(r.to_path_buf(), "solo").unwrap();
    let everywhere = node
        .usage_everywhere()
        .expect("a node can sum its own network");
    assert!(
        everywhere.today.is_some(),
        "the machine that answered knows what day it is, spent or not"
    );
    assert_eq!(
        everywhere.today,
        node.usage().today,
        "the summary and this machine's own answer cut the same day"
    );
    assert!(
        everywhere.days.is_empty(),
        "…and it says so beside no spending at all, which is where a day is easiest to drop"
    );
}
