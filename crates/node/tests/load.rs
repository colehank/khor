//! Real-connection acceptance for the load op (`proto::Request::Load`):
//! what a machine is busy with is **asked for** while its card is open,
//! so the only way to know it works is to make one machine ask another
//! over real UDP — and to make sure the answer's two blocking halves (a
//! process table and a file) stayed off the reactor they travel on
//! (`tests/vitals.rs` has the scar).

mod util;

use std::path::Path;
use std::time::Duration;

use khor_node::Node;
use khor_node::TopBy;
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

/// **One test of this file at a time** (SHELL.md 第五轮 第 4 条).
///
/// The reactor gauge (`khor_node::reactor`) is one process-wide number,
/// fed by every serve in the process and reset by every `forget()`. The
/// two tests below each start serves and each measure a window with it,
/// and cargo runs them on parallel threads — so one test's setup (a
/// pairing, an endpoint bound on a single-threaded runtime) could land
/// in the other's window, and the other's `forget()` could wipe the
/// start of this one's. On a busy machine that setup is slow enough to
/// cross the 1000 ms line: a reading about the wrong test, not about the
/// load handler.
static ONE_AT_A_TIME: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn root(tag: &str) -> Scratch {
    let p = std::env::temp_dir().join(format!("khor-load-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    util::hermetic(&p);
    Scratch(p)
}

async fn wait_for_endpoint_file(root: &Path) {
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
    let _one = ONE_AT_A_TIME.lock().await;
    let ra = root("a");
    let rb = root("b");

    let alpha = Node::open_as(ra.to_path_buf(), "alpha").unwrap();
    let server = Node::open_as(ra.to_path_buf(), "alpha").unwrap();
    let _serve = tokio::spawn(async move { server.serve().await });
    wait_for_endpoint_file(&ra).await;

    let beta = Node::open_as(rb.to_path_buf(), "beta").unwrap();
    assert!(
        beta.load_on("alpha", 3, TopBy::Busiest, true)
            .await
            .is_err(),
        "a machine beta has never met must be refused, not answered"
    );

    let ticket = alpha.invite().unwrap();
    timeout(Duration::from_secs(60), beta.pair(&ticket))
        .await
        .expect("pairing must not hang")
        .unwrap();
    khor_node::reactor::forget();

    let load = timeout(
        Duration::from_secs(60),
        beta.load_on("alpha", 3, TopBy::Busiest, true),
    )
    .await
    .expect("the ask must finish")
    .expect("alpha answers its paired peer");
    let stalled = khor_node::reactor::worst_stall_ms();
    assert!(
        stalled < 1000,
        "the serve's reactor was unavailable for {stalled} ms — the load handler is blocking it"
    );
    assert!(!load.top.is_empty(), "a live machine runs processes");
    assert!(
        load.top.len() <= 3,
        "three rows asked, {} answered",
        load.top.len()
    );
    for r in &load.top {
        assert!(!r.name.is_empty(), "a process has a name: {r:?}");
        assert!(r.cpu_pct >= 0.0, "a rate is not negative: {r:?}");
    }
    assert_eq!(
        load.cpu_day.buckets.len(),
        288,
        "a day of five-minute buckets"
    );
    // The serve's recorder takes its first reading on its first beat —
    // which comes after the endpoint is bound, and in a test process
    // the first touch of the network stack is the slow part (#73) —
    // so the newest bucket of alpha's day is asked for until it is a
    // reading, not a hole. What is asserted is that the file the
    // recorder wrote is the one the handler reads, across the wire;
    // how many seconds the first beat took is not the claim.
    let mut day = load.cpu_day;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    while day.buckets.last().copied().flatten().is_none() && tokio::time::Instant::now() < deadline
    {
        tokio::time::sleep(Duration::from_secs(2)).await;
        day = beta
            .load_on("alpha", 3, TopBy::Busiest, true)
            .await
            .expect("alpha keeps answering")
            .cpu_day;
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

/// **A second ask rides the pipe the first one opened.**
///
/// The app asks a far machine for its top every five seconds while a
/// card is open. Before this, every ask dialled its own connection —
/// a direct road and a relay road raced, both full handshakes — and
/// dropped it on the way out; the far machine and whatever relay sits
/// between them saw twelve of those a minute to answer a question that
/// had not changed. A QUIC connection carries many streams, so the
/// pipe is what gets held and the stream is what gets opened.
///
/// **What makes this a control rather than a ritual:** the two asks
/// answered before the change too, so "both answered" proves nothing.
/// What separates the two versions is the count of pipes held after
/// them — one, not two, and the same name both times — and the dial
/// count, which no ask after the first may move. `WireWatch` is
/// there for exactly this reading and says so.
///
/// The path is the app's: every face routes through the resident serve
/// (`via_serve`), so beta runs one and the watch is taken on the node
/// that serve consumes — the map is shared, so it still sees it.
#[tokio::test]
async fn a_second_ask_rides_the_pipe_the_first_one_opened() {
    let _one = ONE_AT_A_TIME.lock().await;
    let ra = root("reuse-a");
    let rb = root("reuse-b");

    let alpha = Node::open_as(ra.to_path_buf(), "alpha").unwrap();
    let server_a = Node::open_as(ra.to_path_buf(), "alpha").unwrap();
    let _serve_a = tokio::spawn(async move { server_a.serve().await });
    wait_for_endpoint_file(&ra).await;

    let beta = Node::open_as(rb.to_path_buf(), "beta").unwrap();
    let ticket = alpha.invite().unwrap();
    timeout(Duration::from_secs(60), beta.pair(&ticket))
        .await
        .expect("pairing must not hang")
        .unwrap();

    // Beta's own serve, the way every face reaches the wire.
    let server_b = Node::open_as(rb.to_path_buf(), "beta").unwrap();
    let wires = server_b.wire_watch();
    let _serve_b = tokio::spawn(async move { server_b.serve().await });
    wait_for_endpoint_file(&rb).await;
    khor_node::reactor::forget();

    // The serve's own first round may already hold the wire to alpha
    // (round NET-FIX: the pump's sync keeps what it dials), so the
    // control is the dial count, not an empty map: whatever the first
    // ask finds or dials is the pipe the next two ride, with no dial of
    // their own.
    let held = wires.names().await;
    assert!(
        held.is_empty() || held == ["alpha"],
        "before any ask only the pump's own wire may be held: {held:?}"
    );

    let mut dialed_after_first = None;
    for round in 1..=3 {
        timeout(
            Duration::from_secs(60),
            beta.load_on("alpha", 3, TopBy::Busiest, true),
        )
        .await
        .unwrap_or_else(|_| panic!("ask {round} must finish"))
        .unwrap_or_else(|e| panic!("ask {round} must be answered: {e}"));
        assert_eq!(
            wires.names().await,
            vec!["alpha".to_owned()],
            "after ask {round} the serve holds one pipe, to the machine it asked"
        );
        let dialed = wires.counts().dialed;
        let first = *dialed_after_first.get_or_insert(dialed);
        assert_eq!(
            dialed, first,
            "ask {round} dialled again instead of riding the held pipe"
        );
        assert_eq!(
            wires.is_open("alpha").await,
            Some(true),
            "the pipe held after ask {round} is one that can carry the next stream"
        );
    }
}
