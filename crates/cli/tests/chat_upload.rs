//! **A turn with a big picture, to an agent on a far machine** (round
//! NET-3, from the ux lane: two phone photos, 8.5 MB, held the face for
//! minutes). Beta attaches to a conversation running on alpha — the
//! scripted agent, `acp-stub` — through the relay behind a road adding
//! `UPLOAD_RTT_MS` (default 200) of round trip, and sends one turn
//! carrying `UPLOAD_MB` (default 10) of picture the way the app does now
//! (`khor_node::gui_host::write_op`: in slices), timed to the agent's
//! first words back. The control is the old way, one frame: it never
//! arrives (the host's frame cap refuses it, and the writer blocks).

mod scratch;

use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use khor_node::gui_host::{GuiNote, GuiOp, write_op};
use khor_node::host::{connect_at, read_frame, write_frame};
use scratch::*;

fn stub() -> PathBuf {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let status = std::process::Command::new(cargo)
        .args(["build", "-q", "-p", "khor-acp", "--bin", "acp-stub"])
        .status()
        .expect("cargo runs");
    assert!(status.success(), "the stub builds");
    std::env::current_exe()
        .unwrap()
        .parent()
        .and_then(|d| d.parent())
        .expect("target/debug")
        .join("acp-stub")
}

/// Reads notes until the agent says something, or `within` passes.
fn first_words(conn: &mut TcpStream, within: Duration) -> Option<Duration> {
    let began = Instant::now();
    conn.set_read_timeout(Some(Duration::from_millis(500)))
        .unwrap();
    while began.elapsed() < within {
        match read_frame::<GuiNote>(conn) {
            Ok(GuiNote::Note(_)) => return Some(began.elapsed()),
            Ok(_) => {}
            Err(_) => {}
        }
    }
    None
}

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
async fn a_big_turn_reaches_a_far_agent_in_slices() {
    let env = |k: &str, d: u64| {
        std::env::var(k)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(d)
    };
    let rtt = Duration::from_millis(env("UPLOAD_RTT_MS", 200));
    let mb = env("UPLOAD_MB", 10) as usize;
    let (p, _road) = pair_on(
        "upload",
        Some(Impair {
            rtt,
            loss_pct: 0,
            stall: Duration::ZERO,
        }),
    )
    .await;
    let stub = stub();
    let out = khor(&p.ra.0)
        .args(["agents", "add", "stub", "--"])
        .arg(&stub)
        .env("KHOR_NAME", "alpha")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out = khor(&p.ra.0)
        .args(["open", "--gui", "--agent", "stub"])
        .env("KHOR_NAME", "alpha")
        .output()
        .unwrap();
    let sid = String::from_utf8_lossy(&out.stdout)
        .trim()
        .lines()
        .last()
        .unwrap_or_default()
        .to_owned();
    assert!(
        sid.contains('/'),
        "a session: {sid:?} {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let picture = "x".repeat(mb << 20);
    let turn = || GuiOp::SayWith {
        text: "look".into(),
        images: vec![("image/png".into(), picture.clone())],
        files: vec![],
    };

    // The control, the old way: one frame. On its own face, so what it
    // wedges is only itself.
    let (addr, cookie) = p
        .beta
        .reach("alpha", &khor_node::SessionId(sid.clone()))
        .await
        .expect("beta reaches the conversation");
    let (control, waited) = tokio::task::spawn_blocking({
        let (addr, cookie, op) = (addr.clone(), cookie.clone(), turn());
        move || {
            let mut reading = connect_at(&addr, &cookie, 0, 0).expect("a face attaches");
            let mut writing = reading.try_clone().unwrap();
            let shut = reading.try_clone().unwrap();
            let writer = std::thread::spawn(move || write_frame(&mut writing, &op));
            let words = first_words(&mut reading, Duration::from_secs(30));
            let _ = shut.shutdown(std::net::Shutdown::Both);
            let _ = writer.join();
            (words, Duration::from_secs(30))
        }
    })
    .await
    .unwrap();

    // The new way: in slices, with progress.
    let (addr, cookie) = p
        .beta
        .reach("alpha", &khor_node::SessionId(sid.clone()))
        .await
        .expect("beta reaches the conversation");
    let (sent, answered, slices) = tokio::task::spawn_blocking({
        let op = turn();
        move || {
            let reading = connect_at(&addr, &cookie, 0, 0).expect("a face attaches");
            let mut listen = reading.try_clone().unwrap();
            let conn = Mutex::new(reading);
            // Drain what attaching says first (agent facts, settings).
            listen
                .set_read_timeout(Some(Duration::from_millis(300)))
                .unwrap();
            while read_frame::<GuiNote>(&mut listen).is_ok() {}
            let began = Instant::now();
            let mut slices = 0;
            write_op(&conn, &op, |_, _| slices += 1).expect("the turn goes");
            let sent = began.elapsed();
            let words = first_words(&mut listen, Duration::from_secs(120)).map(|w| sent + w);
            (sent, words, slices)
        }
    })
    .await
    .unwrap();

    eprintln!(
        "[upload] {mb} MB of picture, added RTT {} ms: in {slices} slices, all sent in {sent:?}, \
         the agent's first words at {answered:?}; one frame (before): {} within {waited:?}",
        rtt.as_millis(),
        match control {
            Some(w) => format!("answered at {w:?}"),
            None => "never answered".to_owned(),
        },
    );
    assert!(
        control.is_none(),
        "one frame over the host's cap should not have gone"
    );
    assert!(answered.is_some(), "the sliced turn reached the agent");
    let _ = khor(&p.ra.0)
        .args(["close", &sid])
        .env("KHOR_NAME", "alpha")
        .output();
}
