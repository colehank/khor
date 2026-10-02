//! Fresh ears without a restart (docs/IOS.md 第三步).
//!
//! A phone's app is suspended seconds after it leaves the screen, and
//! what its serve holds goes deaf. A desktop serve that goes deaf exits
//! and its keeper starts another; nothing restarts a phone app. So a
//! resumable serve (`Node::serve_resumable`) is given new ears in place
//! by `link::resume` — and these tests cut a real serve's ears
//! (`link::deafen` closes its endpoint under it, which is where a
//! suspension leaves a phone) and prove, over a real wire, that:
//!
//! 1. the cut is real — the deafened serve hears nobody and nobody hears
//!    it (the control, without which a green below proves nothing);
//! 2. after `resume`, **the same process, the same `serve` future**
//!    hears its peer again, the round that follows `resume` is the one
//!    that brings what the peer wrote meanwhile, and the peer can ask it
//!    things again;
//! 3. a far terminal comes back the way a face attaches to one anywhere
//!    (`Node::rejoin`): through a fresh pipe, with the host's ring poured
//!    to it — so what the program wrote while the pipe was dead is in
//!    the replay — and a rejoin that cannot reach says *unreached*, while
//!    one that reaches a session that is not there says *gone*.
//!
//! Temporary stores and temporary identities only.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use khor_node::Node;
use khor_node::link::{self, Rejoin};
use tokio::time::timeout;

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn root(tag: &str) -> Scratch {
    let p = std::env::temp_dir().join(format!("khor-resume-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    Scratch(p)
}

/// 90 s for issue #73's first-touch stall; `files_wire.rs` has why.
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

fn endpoint_json(root: &Path) -> serde_json::Value {
    serde_json::from_slice(&fs::read(root.join(".khor").join("endpoint.json")).unwrap()).unwrap()
}

fn beta_heard(outcomes: &[(String, Result<String, String>)]) -> bool {
    outcomes.iter().any(|(name, r)| name == "beta" && r.is_ok())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_deafened_serve_hears_its_peer_again_after_resume_without_being_reopened() {
    let ra = root("a");
    let rb = root("b");
    let sa = Node::open_as(ra.0.clone(), "alpha").unwrap();
    let serve_a = tokio::spawn(async move { sa.serve_resumable().await });
    let sb = Node::open_as(rb.0.clone(), "beta").unwrap();
    let serve_b = tokio::spawn(async move { sb.serve().await });
    wait_for_endpoint_file(&ra.0).await;
    wait_for_endpoint_file(&rb.0).await;

    let a = Node::open_as(ra.0.clone(), "alpha").unwrap();
    let b = Node::open_as(rb.0.clone(), "beta").unwrap();
    let ticket = a.invite().unwrap();
    timeout(Duration::from_secs(60), b.pair(&ticket))
        .await
        .expect("pairing must not hang")
        .unwrap();
    let dir = ra.0.join("browse");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("apple"), b"x").unwrap();
    let path = dir.to_str().unwrap().to_owned();

    // Control: alpha hears beta, and beta can ask alpha.
    let out = timeout(Duration::from_secs(60), a.sync_now())
        .await
        .expect("a round must come back")
        .unwrap();
    assert!(
        beta_heard(&out),
        "control: alpha's round reaches beta: {out:?}"
    );
    assert!(
        timeout(Duration::from_secs(60), b.ls_of("alpha", &path))
            .await
            .expect("an ask must come back")
            .is_ok(),
        "control: beta can ask alpha"
    );
    // Only a resumable serve is resumed — `serve` keeps restarting.
    assert!(
        link::resume(&rb.0).is_none(),
        "beta's serve is not resumable"
    );
    let before = endpoint_json(&ra.0);

    // **The cut.** Alpha's endpoint is closed under its serve.
    assert!(
        link::deafen(&ra.0).await,
        "alpha's serve is listed for resume"
    );
    let renamed = "beta-while-alpha-was-away";
    b.rename_machine(b.device_str(), renamed).unwrap();
    let out = timeout(Duration::from_secs(60), a.sync_now())
        .await
        .expect("a deaf round still comes back")
        .unwrap();
    assert!(
        !beta_heard(&out),
        "the cut is real: a deaf alpha does not reach beta: {out:?}"
    );
    assert!(
        a.devices().unwrap().iter().all(|d| d.shown() != renamed),
        "the cut is real: beta's new name has not reached alpha"
    );
    assert!(
        timeout(Duration::from_secs(60), b.ls_of("alpha", &path))
            .await
            .expect("an ask must come back")
            .is_err(),
        "the cut is real: beta cannot ask a deaf alpha"
    );
    assert!(
        !serve_a.is_finished(),
        "a resumable serve outlives its endpoint"
    );

    // **The cure.** No reopening: the same serve, new ears, a round.
    let begun = Instant::now();
    let mut resuming = link::resume(&ra.0).expect("alpha's serve takes a resume");
    timeout(Duration::from_secs(30), resuming.ears())
        .await
        .expect("new ears must not hang")
        .expect("alpha gets new ears");
    let ears = begun.elapsed();
    let out = timeout(Duration::from_secs(60), resuming.synced())
        .await
        .expect("the round after resume must come back")
        .unwrap();
    let synced = begun.elapsed();
    println!("resume: ears in {ears:?}, round done in {synced:?}: {out:?}");
    assert!(
        beta_heard(&out),
        "the round that follows resume reaches beta: {out:?}"
    );
    assert!(
        a.devices().unwrap().iter().any(|d| d.shown() == renamed),
        "and it brought what beta wrote while alpha was deaf"
    );
    let after = endpoint_json(&ra.0);
    assert_eq!(
        after["pid"], before["pid"],
        "the same process, not a restart"
    );
    assert_ne!(
        after["addrs"], before["addrs"],
        "a new endpoint, on new ports, written where the verbs look"
    );
    assert!(!serve_a.is_finished(), "and the same serve");

    // And beta can ask alpha again — its ears are open, not just its mouth.
    let asked = timeout(Duration::from_secs(60), b.ls_of("alpha", &path))
        .await
        .expect("an ask must come back");
    assert!(
        asked.is_ok(),
        "beta asks alpha again after resume: {asked:?}"
    );

    serve_a.abort();
    serve_b.abort();
}

/// A program that writes whatever lands in `<self>.say` and waits for
/// `<self>.stop`.
const SAYER: &str = r#"#!/bin/sh
printf 'sayer-up\n'
while [ ! -f "$0.stop" ]; do
  if [ -f "$0.say" ]; then cat "$0.say"; rm -f "$0.say"; fi
  sleep 0.1
done
"#;

/// Reads from a host connection until `want` has been seen, or the
/// budget runs out; answers everything read.
fn read_until(conn: &mut std::net::TcpStream, want: &str, budget: Duration) -> String {
    conn.set_read_timeout(Some(Duration::from_millis(200)))
        .unwrap();
    let end = Instant::now() + budget;
    let mut got = Vec::new();
    let mut buf = [0u8; 65536];
    while Instant::now() < end && !String::from_utf8_lossy(&got).contains(want) {
        match conn.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => got.extend_from_slice(&buf[..n]),
            Err(_) => {}
        }
    }
    String::from_utf8_lossy(&got).into_owned()
}

/// Whether the connection has ended: a read that answers 0, or an error
/// that is not a timeout, within the budget.
fn ends_within(conn: &mut std::net::TcpStream, budget: Duration) -> bool {
    conn.set_read_timeout(Some(Duration::from_millis(200)))
        .unwrap();
    let end = Instant::now() + budget;
    let mut buf = [0u8; 4096];
    while Instant::now() < end {
        match conn.read(&mut buf) {
            Ok(0) => return true,
            Ok(_) => {}
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) => {}
            Err(_) => return true,
        }
    }
    false
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_far_terminal_rejoins_after_resume_and_its_replay_has_what_happened_meanwhile() {
    let ra = root("ta");
    let rb = root("tb");
    // Alpha hosts the session; beta (the phone) attaches across the wire.
    let sa = Node::open_as(ra.0.clone(), "alpha").unwrap();
    let serve_a = tokio::spawn(async move { sa.serve().await });
    let sb = Node::open_as(rb.0.clone(), "beta").unwrap();
    let serve_b = tokio::spawn(async move { sb.serve_resumable().await });
    wait_for_endpoint_file(&ra.0).await;
    wait_for_endpoint_file(&rb.0).await;
    let a = Node::open_as(ra.0.clone(), "alpha").unwrap();
    let b = Node::open_as(rb.0.clone(), "beta").unwrap();
    let ticket = a.invite().unwrap();
    timeout(Duration::from_secs(60), b.pair(&ticket))
        .await
        .expect("pairing must not hang")
        .unwrap();

    // A real host on alpha, in this process (`host_replay.rs`'s way).
    let program = ra.0.join("sayer");
    fs::write(&program, SAYER).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let id = khor_core::SessionId("tui/resume-sayer".into());
    let kind = khor_node::live::LiveKind::new(ra.0.clone(), a.device());
    kind.register(&id, "tui", "sayer", None, None).unwrap();
    let host = {
        let (root, id) = (ra.0.clone(), id.clone());
        let cmd = vec![program.to_string_lossy().to_string()];
        std::thread::spawn(move || khor_node::host::host_main(root, id, (80, 24), cmd))
    };
    let dir = kind.dir_of(&id).expect("registered");
    let deadline = Instant::now() + Duration::from_secs(30);
    while khor_node::host::read_host_file(&dir).is_err() {
        assert!(Instant::now() < deadline, "the host never came up");
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Attached from beta, the way the app attaches a far terminal.
    let (addr, cookie) = timeout(Duration::from_secs(60), b.rejoin("alpha", &id))
        .await
        .expect("a reach must come back")
        .expect("beta reaches alpha's session");
    let mut conn = khor_node::host::connect_at(&addr, &cookie, 80, 24).unwrap();
    let seen = read_until(&mut conn, "sayer-up", Duration::from_secs(20));
    assert!(
        seen.contains("sayer-up"),
        "control: the attach replays the host's ring: {seen:?}"
    );

    // Beta's ears are cut: the pipe ends, and a rejoin cannot reach —
    // and says so as *unreached*, not as a session that ended.
    assert!(link::deafen(&rb.0).await);
    assert!(
        ends_within(&mut conn, Duration::from_secs(30)),
        "the pipe over the closed endpoint ends"
    );
    // The program writes while beta is deaf.
    fs::write(program.with_extension("say"), "said-while-away\n").unwrap();
    let cut = timeout(Duration::from_secs(60), b.rejoin("alpha", &id))
        .await
        .expect("a rejoin must come back");
    assert!(
        matches!(cut, Err(Rejoin::Unreached(_))),
        "a deaf beta's rejoin is unreached, worth asking again: {cut:?}"
    );

    // Resume, then rejoin: a fresh pipe, and the ring has the meanwhile.
    let begun = Instant::now();
    let mut resuming = link::resume(&rb.0).expect("beta's serve takes a resume");
    resuming.ears().await.expect("beta gets new ears");
    let (addr2, cookie2) = timeout(Duration::from_secs(60), b.rejoin("alpha", &id))
        .await
        .expect("a rejoin must come back")
        .expect("after resume, beta reaches alpha's session again");
    let mut again = khor_node::host::connect_at(&addr2, &cookie2, 80, 24).unwrap();
    let replay = read_until(&mut again, "said-while-away", Duration::from_secs(20));
    println!("rejoin: caught up in {:?}", begun.elapsed());
    assert!(
        replay.contains("said-while-away"),
        "the replay after rejoin carries what the program wrote meanwhile: {replay:?}"
    );
    let _ = timeout(Duration::from_secs(60), resuming.synced()).await;

    // A rejoin that reaches alpha for a session that is not there is
    // *gone* — the face's 「结束了」 is then the truth.
    let none = khor_core::SessionId("tui/never-was".into());
    let gone = timeout(Duration::from_secs(60), b.rejoin("alpha", &none))
        .await
        .expect("a rejoin must come back");
    assert!(
        matches!(gone, Err(Rejoin::Gone(_))),
        "no such session is gone, not unreached: {gone:?}"
    );

    fs::write(program.with_extension("stop"), "").unwrap();
    let _ = host.join();
    serve_a.abort();
    serve_b.abort();
}
