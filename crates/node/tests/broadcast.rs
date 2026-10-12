//! Real-connection acceptance for the broadcast (STORE-BRIEF phase 3,
//! `khor_node::wide`): the khor's own group conversation stands on every
//! machine once there are two, its lines spread through whichever
//! machine is up, and a file in it is an offer each machine answers for
//! itself — accepted, pulled from the sender, its state kept per
//! receiver, counted on the sender — and pulled from any machine that
//! landed it before when the sender cannot be reached (BCAST-BRIEF's
//! holders). Every await under a timeout.

mod util;

use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use khor_catalog::msg;
use khor_core::State;
use khor_node::link;
use khor_node::wide::BROADCAST;
use khor_node::{MsgBody, Node};
use khor_sync::chat::Step;
use tokio::time::timeout;

/// A scratch tree that takes itself away however the test leaves (the
/// copy in `transfer.rs` has the reasoning).
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
    let p = std::env::temp_dir().join(format!("khor-bcast-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    util::hermetic(&p);
    Scratch(p)
}

async fn wait_for_endpoint_file(root: &Path) {
    // 90 s for the reason `transfer.rs` gives (a new binary's first
    // network touch on a Mac over ssh).
    let path = root.join(".khor").join("endpoint.json");
    timeout(Duration::from_secs(90), async {
        while !path.exists() {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("serve should write endpoint.json");
}

fn row(n: &Node) -> Option<khor_core::Session> {
    n.sessions()
        .unwrap()
        .into_iter()
        .find(|v| v.session.id.0 == BROADCAST)
        .map(|v| v.session)
}

async fn sync(n: &Node) {
    timeout(Duration::from_secs(45), n.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_broadcast_reaches_every_machine_and_each_answers_its_offer() {
    let ra = root("a");
    let rb = root("b");
    let rc = root("c");
    let server = Node::open_as(ra.to_path_buf(), "alpha").unwrap();
    let serve = tokio::spawn(async move { server.serve().await });
    wait_for_endpoint_file(&ra).await;
    let a = Node::open_as(ra.to_path_buf(), "alpha").unwrap();
    let b = Node::open_as(rb.to_path_buf(), "beta").unwrap();
    let c = Node::open_as(rc.to_path_buf(), "gamma").unwrap();

    // A machine alone has no group: no row, nothing to create.
    assert!(row(&b).is_none(), "one machine, no broadcast");

    for n in [&b, &c] {
        let ticket = a.invite().unwrap();
        timeout(Duration::from_secs(45), n.pair(&ticket))
            .await
            .expect("pairing must not hang")
            .unwrap();
    }
    sync(&b).await;
    sync(&c).await;
    // The row stands on every machine, from its own copy, with nothing
    // created first: the khor's own, wearing the broadcast kind.
    for n in [&a, &b, &c] {
        let r = row(n).expect("the broadcast row stands on every machine");
        assert_eq!(r.kind.0, khor_core::kind::BROADCAST);
        assert_eq!(r.category.as_deref(), Some(khor_core::category::KHOR));
        assert_eq!(n.wide_members().unwrap().len(), 3, "every machine is a member");
    }

    // A line from beta spreads through alpha: gamma reads it although
    // beta serves nothing (the sender gone is no matter).
    timeout(Duration::from_secs(45), b.post("hello everyone", &[]))
        .await
        .expect("a post must not hang")
        .unwrap();
    sync(&b).await;
    sync(&c).await;
    let said = c.log(BROADCAST).unwrap().messages;
    assert!(
        said.iter().any(|m| matches!(&m.body, MsgBody::Text(t) if t == "hello everyone")),
        "gamma reads beta's line through alpha"
    );
    let rc_row = row(&c).unwrap();
    assert_eq!(rc_row.last.as_deref(), Some("hello everyone"), "the row previews the newest line");
    // Every machine is the same person's: a line typed on beta was seen
    // by its writer, and the seen mark travels (`Node::tell`'s rule).
    assert_eq!((rc_row.unread, rc_row.state.state), (0, State::Idle), "own words are unread nowhere");

    // alpha offers a file. Nothing moves by itself, and nothing waits on
    // anybody: on each other machine it is a file one may take, unread
    // like any message (STORE-BRIEF phase 5: an offer never says 待批).
    let payload: Vec<u8> = (0..300_000u32).map(|i| (i % 241) as u8).collect();
    let src = ra.join("all.bin");
    fs::write(&src, &payload).unwrap();
    let tid = khor_node::transfer::TransferKind::session_id(
        &timeout(Duration::from_secs(45), a.post("", std::slice::from_ref(&src)))
            .await
            .expect("a post must not hang")
            .unwrap(),
    );
    let msg = tid.0.strip_prefix("transfer/").unwrap().to_owned();
    sync(&b).await;
    sync(&c).await;
    for n in [&b, &c] {
        assert_eq!(row(n).unwrap().state.state, State::Done, "an unanswered offer is 未看, never 待批");
    }
    assert_ne!(row(&a).unwrap().state.state, State::Blocked, "the sender waits on nobody");
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(!rb.join("khor-drop").join("all.bin").exists(), "an offer is not pulled before it is accepted");
    let face = |n: &Node| {
        let (log, faces) = n.wide_log(BROADCAST).unwrap();
        let i = log.messages.iter().position(|m| m.id == msg).unwrap();
        faces[i].clone().unwrap()
    };
    let f = face(&a);
    assert_eq!((f.got, f.of, f.here), (0, 2, None), "nobody has it yet, two were asked");

    // beta accepts: pulled from alpha into beta's own receive location.
    timeout(Duration::from_secs(60), b.take(&tid))
        .await
        .expect("accept must not hang")
        .unwrap();
    let landed = rb.join("khor-drop").join("all.bin");
    assert_eq!(fs::read(&landed).unwrap(), payload, "verbatim, in beta's receive location");
    assert_eq!(face(&b).here.unwrap().step, Step::Landed);
    assert_ne!(row(&b).unwrap().state.state, State::Blocked, "answered, beta's row lets go");
    assert_eq!(row(&c).unwrap().state.state, State::Done, "gamma's own is still unread, not waiting");

    // gamma's pull breaks (the sender's file is gone): its own record
    // says so, beta's is untouched, and a retry resumes once it is back.
    fs::rename(&src, ra.join("away.bin")).unwrap();
    timeout(Duration::from_secs(60), c.take(&tid))
        .await
        .expect("a failing accept must not hang")
        .unwrap_err();
    let failed = face(&c).here.unwrap();
    assert_eq!(failed.step, Step::Failed);
    assert!(!failed.why.is_empty(), "with its sentence");
    assert_eq!(row(&c).unwrap().state.state, State::Errored, "a broken pull is 中断");
    fs::rename(ra.join("away.bin"), &src).unwrap();
    timeout(Duration::from_secs(60), c.take(&tid))
        .await
        .expect("the retry must not hang")
        .unwrap();
    assert_eq!(fs::read(rc.join("khor-drop").join("all.bin")).unwrap(), payload);

    // The sender counts both, from the records the receivers wrote.
    sync(&b).await;
    sync(&c).await;
    let f = face(&a);
    assert_eq!((f.got, f.of), (2, 2), "the sender reads 2/2");
    let both = a.log(BROADCAST).unwrap().messages.into_iter().find(|m| m.id == msg).unwrap();
    assert_eq!(both.fates.len(), 2, "one record per receiver");

    // The machine that sent it has nothing to receive.
    let own = timeout(Duration::from_secs(60), a.take(&tid))
        .await
        .expect("must not hang")
        .unwrap_err();
    assert_eq!(own, khor_catalog::msg::own_broadcast_file());

    serve.abort();
}

/// Whether a round reached `name`.
fn heard(out: &[(String, Result<String, String>)], name: &str) -> bool {
    out.iter().any(|(n, r)| n == name && r.is_ok())
}

/// **The sender gone, a holder serves** (BCAST-BRIEF): alpha offers a
/// file and goes deaf the way a phone in the background does
/// (`link::deafen`). gamma's 收下 with nobody else holding it fails and
/// says so; once beta has landed it (alpha back for that), gamma pulls
/// it from beta while alpha is deaf again — a holder whose landed file
/// was edited since is skipped, named in the failure, not fatal — and
/// the record on gamma names beta as where the bytes came from.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_broadcast_file_is_pulled_from_a_holder_while_its_sender_is_away() {
    let began = Instant::now();
    let ra = root("ha");
    let rb = root("hb");
    let rc = root("hc");
    let sa = Node::open_as(ra.to_path_buf(), "alpha").unwrap();
    let serve_a = tokio::spawn(async move { sa.serve_resumable().await });
    let sb = Node::open_as(rb.to_path_buf(), "beta").unwrap();
    let serve_b = tokio::spawn(async move { sb.serve().await });
    wait_for_endpoint_file(&ra).await;
    wait_for_endpoint_file(&rb).await;
    let a = Node::open_as(ra.to_path_buf(), "alpha").unwrap();
    let b = Node::open_as(rb.to_path_buf(), "beta").unwrap();
    let c = Node::open_as(rc.to_path_buf(), "gamma").unwrap();
    for n in [&b, &c] {
        let ticket = a.invite().unwrap();
        timeout(Duration::from_secs(45), n.pair(&ticket))
            .await
            .expect("pairing must not hang")
            .unwrap();
    }
    // beta learns gamma (or it would refuse gamma's asks), gamma learns
    // where beta listens: a round each until gamma's reaches beta.
    sync(&b).await;
    let reached = Instant::now();
    loop {
        let out = timeout(Duration::from_secs(45), c.sync_now()).await.expect("sync must not hang").unwrap();
        if heard(&out, "beta") {
            break;
        }
        assert!(reached.elapsed() < Duration::from_secs(60), "gamma reaches beta: {out:?}");
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    let payload: Vec<u8> = (0..300_000u32).map(|i| (i % 239) as u8).collect();
    let src = ra.join("held.bin");
    fs::write(&src, &payload).unwrap();
    let tid = khor_node::transfer::TransferKind::session_id(
        &timeout(Duration::from_secs(45), a.post("", std::slice::from_ref(&src)))
            .await
            .expect("a post must not hang")
            .unwrap(),
    );
    let msg_id = tid.0.strip_prefix("transfer/").unwrap().to_owned();
    sync(&b).await;
    sync(&c).await;
    let face = |n: &Node| {
        let (log, faces) = n.wide_log(BROADCAST).unwrap();
        let i = log.messages.iter().position(|m| m.id == msg_id).expect("the offer is here");
        faces[i].clone().unwrap()
    };

    // alpha goes away; nobody else has it: gamma's 收下 fails, naming the
    // sender and saying no other machine has it yet.
    assert!(link::deafen(&ra).await, "alpha's serve is resumable");
    let t = Instant::now();
    let why = timeout(Duration::from_secs(90), c.take(&tid))
        .await
        .expect("a take with nobody to pull from must not hang")
        .unwrap_err();
    let nobody_took = t.elapsed();
    let sep = msg::take_tried_separator();
    assert_eq!(why, format!("{}{sep}{}", msg::take_sender_down("alpha"), msg::take_no_holder()));
    let failed = face(&c).here.unwrap();
    assert_eq!((failed.step, failed.why.as_str()), (Step::Failed, why.as_str()), "the record says the same");

    // alpha is back; beta lands it from alpha and so holds it.
    let mut resuming = link::resume(&ra).expect("alpha's serve takes a resume");
    timeout(Duration::from_secs(30), resuming.ears()).await.expect("new ears must not hang").expect("alpha hears again");
    let _ = timeout(Duration::from_secs(60), resuming.synced()).await.expect("the round after resume must come back");
    timeout(Duration::from_secs(60), b.take(&tid))
        .await
        .expect("beta's accept must not hang")
        .unwrap();
    let on_b = rb.join("khor-drop").join("held.bin");
    assert_eq!(fs::read(&on_b).unwrap(), payload, "beta landed it");
    let at_b = face(&b).here.unwrap();
    assert_eq!((at_b.step, at_b.via.as_str()), (Step::Landed, a.device_str()), "from alpha");
    // beta holds it: the landed file's record, written by the take. A
    // 收下 that finds it landed writes it again when it went missing.
    let records = || -> Vec<String> {
        fs::read_dir(rb.join(".khor").join("transfers"))
            .map(|rd| rd.flatten().filter_map(|e| fs::read_to_string(e.path()).ok()).collect())
            .unwrap_or_default()
    };
    assert!(records().iter().any(|r| r.contains("\"held\":true")), "beta holds it: {:?}", records());
    fs::remove_dir_all(rb.join(".khor").join("transfers")).unwrap();
    timeout(Duration::from_secs(60), b.take(&tid))
        .await
        .expect("a take of a landed file must not hang")
        .unwrap();
    assert!(records().iter().any(|r| r.contains("\"held\":true")), "held again: {:?}", records());

    // alpha away again; gamma reads beta's landing from beta itself.
    assert!(link::deafen(&ra).await);
    let seen = Instant::now();
    while face(&c).got < 1 {
        assert!(seen.elapsed() < Duration::from_secs(90), "gamma reads beta's landing");
        sync(&c).await;
    }

    // beta's file edited since: its record no longer matches, so beta is
    // skipped with its reason, and the failure names both machines —
    // beta first: gamma's last dial to alpha failed, and a sender whose
    // dials are failing goes after the holders (`Node::wire_rank`).
    fs::write(&on_b, b"edited on beta").unwrap();
    let why = timeout(Duration::from_secs(90), c.take(&tid))
        .await
        .expect("must not hang")
        .unwrap_err();
    assert_eq!(
        why,
        format!(
            "{}{sep}{}",
            msg::take_holder_failed("beta", msg::offered_file_changed()),
            msg::take_sender_down("alpha")
        )
    );

    // beta's file as it landed again: gamma 收下 again and lands it from
    // beta, with alpha still away.
    fs::write(&on_b, &payload).unwrap();
    let t = Instant::now();
    timeout(Duration::from_secs(90), c.take(&tid))
        .await
        .expect("a take from a holder must not hang")
        .unwrap();
    let from_holder_took = t.elapsed();
    // Not behind a dial to alpha: that alone is `DIAL_TIMEOUT` (10 s).
    assert!(from_holder_took < Duration::from_secs(8), "beta is asked first: {from_holder_took:?}");
    assert_eq!(fs::read(rc.join("khor-drop").join("held.bin")).unwrap(), payload, "gamma has alpha's bytes");
    let at_c = face(&c).here.unwrap();
    assert_eq!((at_c.step, at_c.via.as_str()), (Step::Landed, b.device_str()), "the record says it came from beta");
    assert!(at_c.why.is_empty());

    // Counted the same as a landing from the sender: two of two, on
    // beta's copy (gamma's move nudged it there) and on gamma's.
    sync(&c).await;
    assert_eq!((face(&c).got, face(&c).of), (2, 2));
    assert_eq!((face(&b).got, face(&b).of), (2, 2), "beta counts gamma's landing from beta");
    eprintln!(
        "timings: nobody-holds take {nobody_took:?}, take from the holder {from_holder_took:?}, whole test {:?}",
        began.elapsed()
    );

    serve_a.abort();
    serve_b.abort();
}
