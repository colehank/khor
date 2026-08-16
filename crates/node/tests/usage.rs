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

use std::path::PathBuf;
use std::time::Duration;

use khor_node::Node;
use tokio::time::timeout;

fn root(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("khor-usage-net-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

/// Gives a node's own home one claude transcript, billing `out` tokens.
///
/// Under the node's root rather than through `KHOR_VENDOR_HOME`, because
/// that variable is one setting for the whole process and both nodes here
/// live in it — the isolation this test rests on is the per-root one that
/// `khor_node::adaptor::vendor_home` gives by default.
fn spend(root: &PathBuf, id: &str, out: u64) {
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

    let alpha = Node::open_as(ra.clone(), "alpha").unwrap();
    let server = Node::open_as(ra.clone(), "alpha").unwrap();
    let serve = tokio::spawn(async move { server.serve().await });
    wait_for_endpoint_file(&ra).await;

    let beta = Node::open_as(rb.clone(), "beta").unwrap();
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
    let (mine, mine_age) = beta.usage_of(beta.device_str()).expect("a node can read its own");
    assert_eq!(output_of(&mine), 7, "beta's own transcript bills 7");
    assert_eq!(mine_age, 0, "this machine reads on demand, so its answer has no age");

    let ticket = alpha.invite().unwrap();
    timeout(Duration::from_secs(15), beta.pair(&ticket))
        .await
        .expect("pairing must not hang")
        .unwrap();
    timeout(Duration::from_secs(20), beta.sync_now())
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
    assert!(age < 60_000, "the answer should be from this visit, not {age} ms ago");

    // Alpha goes away. What beta shows now is the last thing it heard,
    // and it must say how old that is rather than wearing the present
    // (docs/SESSION.md 离线不是第七个词).
    serve.abort();
    tokio::time::sleep(Duration::from_millis(1200)).await;
    let (still, older) = beta.usage_of(&alpha_id).expect("the last answer is still the answer");
    assert_eq!(still, theirs, "not one number moved while nobody could be reached");
    assert!(
        older > age,
        "…and it is visibly older than it was: {age} ms then, {older} ms now"
    );

    let _ = std::fs::remove_dir_all(&ra);
    let _ = std::fs::remove_dir_all(&rb);
}
