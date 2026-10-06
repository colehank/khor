//! **The app's terminal loop against a host from before the version
//! handshake** (term-handoff (b); HOST_VERSION 2 came with 51004627).
//! Hosts outlive app upgrades, so the app attaches to `_host` processes an
//! older khor started. This is the app's own path: `term_open`, then the
//! pane's pushed beat, `term_wait` asked again at once each time it
//! answers (the face's 0 ms beat in the app). It counts the beats, keeps
//! the CPU this process spent, and checks a typed line comes back.
//!
//! By hand, against a scratch home whose row is hosted by an old binary:
//! `PROBE_HOME=<scratch home> PROBE_ROW=shell/<id> cargo test -p khor-gui-core
//!  --test old_host_wait -- --ignored --nocapture`

use std::time::{Duration, Instant};

use base64::Engine;
use khor_gui_core::term;

fn cpu_seconds() -> f64 {
    let out = std::process::Command::new("ps")
        .args(["-o", "time=", "-p", &std::process::id().to_string()])
        .output()
        .expect("ps runs");
    String::from_utf8_lossy(&out.stdout)
        .trim()
        .split(':')
        .fold(0.0, |acc, part| {
            acc * 60.0 + part.parse::<f64>().unwrap_or(0.0)
        })
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "probe: needs a scratch home with a hosted row, run by hand"]
async fn the_pushed_beat_against_a_host() {
    let home = std::path::PathBuf::from(std::env::var("PROBE_HOME").expect("PROBE_HOME"));
    let row = std::env::var("PROBE_ROW").expect("PROBE_ROW");
    let secs: u64 = std::env::var("PROBE_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30);
    let opened = Instant::now();
    // 0: no history setting said (item 56), as an older face.
    term::term_open(&home, &row, 100, 30, 0).expect("the row opens");
    let open_ms = opened.elapsed().as_millis();

    let mut since = 0u64;
    let mut seen = Vec::new();
    let mut beats = 0u64;
    let mut empty = 0u64;
    let mut gone = false;
    let cpu0 = cpu_seconds();
    let began = Instant::now();
    let mut typed = false;
    let mut kept = None;
    while began.elapsed() < Duration::from_secs(secs) {
        if !typed && began.elapsed() > Duration::from_secs(secs / 3) {
            term::term_key(&row, b"echo probe-$((6*7))\r".to_vec()).expect("a key goes in");
            typed = true;
        }
        // ⌘K, the one op that waits for version 2, then a resize and a
        // second line: the attachment must still be whole after them.
        if kept.is_none() && began.elapsed() > Duration::from_secs(secs * 2 / 3) {
            kept = Some(term::term_forget(&row, true).expect("forget"));
            term::term_resize(&row, 90, 28).expect("resize");
            term::term_key(&row, b"echo after-$((7*8))\r".to_vec()).expect("a key goes in");
        }
        let chunk = term::term_wait(&row, since, Duration::from_millis(2000))
            .await
            .expect("a beat");
        beats += 1;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(chunk.bytes.as_bytes())
            .unwrap();
        if bytes.is_empty() {
            empty += 1;
        }
        seen.extend_from_slice(&bytes);
        since = chunk.seq;
        if chunk.gone {
            gone = true;
            break;
        }
    }
    let spent = cpu_seconds() - cpu0;
    let took = began.elapsed().as_secs_f64();
    let echoed = String::from_utf8_lossy(&seen).contains("probe-42");
    let after = String::from_utf8_lossy(&seen).contains("after-56");
    term::term_leave(&row).ok();
    println!(
        "old_host_wait: row {row}, open {open_ms} ms, beats {beats} ({:.1}/s, {empty} empty), gone {gone}, echoed {echoed}, forget kept by host {kept:?}, line after it {after}, cpu {:.1} %",
        beats as f64 / took,
        spent * 100.0 / took
    );
}
