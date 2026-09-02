//! Real-connection acceptance for the load op (`proto::Request::Load`):
//! what a machine is busy with is **asked for** while its card is open,
//! so the only way to know it works is to make one machine ask another
//! over real UDP — and to make sure the answer's two blocking halves (a
//! process table and a file) stayed off the reactor they travel on
//! (`tests/vitals.rs` has the scar).

use std::path::PathBuf;
use std::time::Duration;

use khor_node::Node;
use tokio::time::timeout;

fn root(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("khor-load-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

async fn wait_for_endpoint_file(root: &PathBuf) {
    let path = root.join(".khor").join("endpoint.json");
    timeout(Duration::from_secs(90), async {
        while !path.exists() {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("serve should write endpoint.json");
}

/// One machine's top and CPU day reach another over the wire, sized as
/// asked, with the serve's own recorder already on the day — and the
/// trip does not stall the link.
///
/// The control is the ask before pairing: a machine beta has never met
/// is refused by name, so the rows found afterwards cannot be beta's
/// own under another name. The second control is the size: three rows
/// asked, at most three answered — a table sized by the answerer would
/// have paid for a whole process list nobody has room for.
#[tokio::test]
async fn a_machine_answers_what_it_is_busy_with_over_the_wire() {
    let ra = root("a");
    let rb = root("b");

    let alpha = Node::open_as(ra.clone(), "alpha").unwrap();
    let server = Node::open_as(ra.clone(), "alpha").unwrap();
    let _serve = tokio::spawn(async move { server.serve().await });
    wait_for_endpoint_file(&ra).await;

    let beta = Node::open_as(rb.clone(), "beta").unwrap();
    assert!(
        beta.load_on("alpha", 3).await.is_err(),
        "a machine beta has never met must be refused, not answered"
    );

    let ticket = alpha.invite().unwrap();
    timeout(Duration::from_secs(60), beta.pair(&ticket))
        .await
        .expect("pairing must not hang")
        .unwrap();
    khor_node::reactor::forget();

    let load = timeout(Duration::from_secs(60), beta.load_on("alpha", 3))
        .await
        .expect("the ask must finish")
        .expect("alpha answers its paired peer");
    let stalled = khor_node::reactor::worst_stall_ms();
    assert!(
        stalled < 1000,
        "the serve's reactor was unavailable for {stalled} ms — the load handler is blocking it"
    );
    assert!(!load.top.is_empty(), "a live machine runs processes");
    assert!(load.top.len() <= 3, "three rows asked, {} answered", load.top.len());
    for r in &load.top {
        assert!(!r.name.is_empty(), "a process has a name: {r:?}");
        assert!(r.cpu_pct >= 0.0, "a rate is not negative: {r:?}");
    }
    assert_eq!(load.cpu_day.buckets.len(), 288, "a day of five-minute buckets");
    // The serve's recorder takes its first reading on its first beat —
    // which comes after the endpoint is bound, and in a test process
    // the first touch of the network stack is the slow part (#73) —
    // so the newest bucket of alpha's day is asked for until it is a
    // reading, not a hole. What is asserted is that the file the
    // recorder wrote is the one the handler reads, across the wire;
    // how many seconds the first beat took is not the claim.
    let mut day = load.cpu_day;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    while day.buckets.last().copied().flatten().is_none() && tokio::time::Instant::now() < deadline {
        tokio::time::sleep(Duration::from_secs(2)).await;
        day = beta.load_on("alpha", 3).await.expect("alpha keeps answering").cpu_day;
    }
    let file = std::fs::read_to_string(ra.join(".khor").join("history").join("cpu"));
    assert!(
        day.buckets.last().copied().flatten().is_some(),
        "alpha's own serve has been recording, so its newest bucket is measured (file: {file:?})"
    );
    // …and everything older is a hole, because nobody measured it:
    // a band invented from one reading would be the lie the type
    // exists to prevent.
    assert!(
        day.buckets[..286].iter().all(Option::is_none),
        "a serve a minute old has no history to show"
    );
}
