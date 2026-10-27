//! **What nobody looking still costs** (round NET-6, from the term lane's
//! load pass, audit/term-load.md): a desk and a phone, scratch, both
//! relay-only through a road adding `DIAG_RTT_MS` (default 200).
//!
//! - **S, a busy session nobody watches:** five idle sessions on the desk
//!   (`DIAG_IDLE_SESSIONS` to change it)
//!   and one whose shell marks a command about once a second (the shell
//!   integration's own `OSC 133` / `633` marks, so the command ledger
//!   and the row's `doing` move as they do under a real shell). The
//!   phone's face is on screen and opens none of them. Bytes a minute on
//!   the phone's wires, then the same with the busy session stopped.
//! - **G, the phone gone:** the phone's app closed (wires let go, ears
//!   closed). What the desk still sends: its dials, its wire bytes, and
//!   every byte through the road towards the relay — with its face on
//!   screen and without.
//!
//! A probe, not a gate:
//! `cargo test -p khor-cli --test idle_cost -- --ignored --nocapture`.

mod scratch;

use std::sync::atomic::Ordering::Relaxed;
use std::time::{Duration, Instant};

use khor_node::Node;
use scratch::*;

fn env_u64(k: &str, d: u64) -> u64 {
    std::env::var(k)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(d)
}

/// A shell loop that marks one command a second, as a shell with khor's
/// integration does: prompt, input, the command line, its output, its end.
const BUSY: &str = r#"i=0; while :; do i=$((i+1)); printf '\033]133;A\007$ \033]133;B\007\033]633;E;make test --shard %s\007\033]133;C\007' "$i"; printf 'running shard %s: ok\n' "$i"; printf '\033]133;D;0\007'; sleep 1; done"#;

fn open_on(home: &std::path::Path, title: &str, cmd: &[&str]) -> String {
    let out = khor(home)
        .args(["open", "-d", "--title", title, "--"])
        .args(cmd)
        .env("KHOR_NAME", "desk")
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
        "a session id: {sid:?} {}",
        String::from_utf8_lossy(&out.stderr)
    );
    sid
}

/// A minute (`span`) with the phone's face on and its beat kept: what the
/// phone's wires and the road carried.
async fn minute(phone: &Node, span: Duration) -> String {
    let c0 = phone.wire_counts().await.unwrap();
    let (u0, d0) = (ROAD_TO_RELAY.load(Relaxed), ROAD_FROM_RELAY.load(Relaxed));
    let began = Instant::now();
    while began.elapsed() < span {
        let _ = phone.face_is_here(true).await;
        tokio::time::sleep(Duration::from_secs(10).min(span.saturating_sub(began.elapsed()))).await;
    }
    let c1 = phone.wire_counts().await.unwrap();
    let per = |b: u64| b as f64 * 60.0 / span.as_secs_f64() / 1024.0;
    format!(
        "phone wires {:.1} KB/min in, {:.1} KB/min out, rounds {}; road {:.1} KB/min to the relay, {:.1} from it",
        per(c1.received_bytes.saturating_sub(c0.received_bytes)),
        per(c1.sent_bytes.saturating_sub(c0.sent_bytes)),
        c1.rounds.saturating_sub(c0.rounds),
        per(ROAD_TO_RELAY.load(Relaxed) - u0),
        per(ROAD_FROM_RELAY.load(Relaxed) - d0),
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
#[ignore = "a probe over a simulated road: run by hand"]
async fn what_a_busy_session_and_an_absent_phone_cost_with_nobody_looking() {
    let rtt = Duration::from_millis(env_u64("DIAG_RTT_MS", 200));
    let span = Duration::from_secs(env_u64("DIAG_SPAN_SECS", 60));
    // SAFETY: before any endpoint in this process binds.
    unsafe { std::env::set_var("KHOR_RELAY_ONLY", "1") };
    own_tmux();
    let (rd, rp, rr) = (
        scratch("cost-desk"),
        scratch("cost-phone"),
        scratch("cost-relay"),
    );
    let (_relay, direct) = local_relay(&rr.0);
    let to: std::net::SocketAddr = direct
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .parse()
        .unwrap();
    let (at, _road) = slow_road(
        to,
        Impair {
            rtt,
            loss_pct: 0,
            stall: rtt,
        },
    )
    .await;
    let url = format!("http://{at}");
    for h in [&rd.0, &rp.0] {
        home_on(h, &url);
    }
    let _desk_serve = serve_with(&rd.0, "desk", &[("KHOR_RELAY_ONLY", "1")]);
    stood_up(&[&rd.0]).await;
    let phone_server = Node::open_hosting(rp.0.clone(), "phone", false).unwrap();
    let _phone_serve = tokio::spawn(async move { phone_server.serve_resumable().await });
    stood_up(&[&rp.0]).await;
    let desk = Node::open_as(rd.0.clone(), "desk").unwrap();
    let phone = Node::open_hosting(rp.0.clone(), "phone", false).unwrap();
    tokio::time::timeout(Duration::from_secs(90), phone.pair(&desk.invite().unwrap()))
        .await
        .expect("pairing must not hang")
        .unwrap();

    // `DIAG_IDLE_SESSIONS` idle rows beside the busy one (default 5): the
    // term lane's desk had 109, and what a change carries can grow with them.
    let idle: Vec<String> = (0..env_u64("DIAG_IDLE_SESSIONS", 5))
        .map(|k| open_on(&rd.0, &format!("idle {k}"), &["/bin/sleep", "3600"]))
        .collect();
    let busy = open_on(&rd.0, "busy", &["/bin/sh", "-c", BUSY]);
    phone.face_is_here(true).await.unwrap();
    // Settle: the rows across, the ledger running.
    tokio::time::sleep(Duration::from_secs(20)).await;
    let doing = |n: &Node| {
        n.sessions()
            .ok()
            .and_then(|rows| rows.into_iter().find(|r| r.session.id.0 == busy))
            .and_then(|r| r.session.doing)
            .and_then(|d| d.command)
    };
    eprintln!("[cost] the busy row on the phone says {:?}", doing(&phone));

    let busy_min = minute(&phone, span).await;
    let after = doing(&phone);
    eprintln!(
        "[cost] S busy session, nobody watching, {}s: {busy_min}; its row now says {after:?}",
        span.as_secs()
    );
    let _ = khor(&rd.0)
        .args(["close", &busy])
        .env("KHOR_NAME", "desk")
        .output();
    tokio::time::sleep(Duration::from_secs(10)).await;
    let idle_min = minute(&phone, span).await;
    eprintln!("[cost] S all idle, {}s: {idle_min}", span.as_secs());

    // G: the phone gone.
    phone.face_is_here(false).await.unwrap();
    assert!(khor_node::link::suspending(&rp.0).await);
    assert!(khor_node::link::deafen(&rp.0).await);
    tokio::time::sleep(Duration::from_secs(10)).await;
    for face in [true, false] {
        let pid0 = serve_pid(&rd.0);
        let c0 = desk.wire_counts().await.unwrap();
        let u0 = ROAD_TO_RELAY.load(Relaxed);
        let began = Instant::now();
        while began.elapsed() < span {
            let _ = desk.face_is_here(face).await;
            tokio::time::sleep(Duration::from_secs(10).min(span.saturating_sub(began.elapsed())))
                .await;
        }
        let c1 = desk.wire_counts().await.unwrap();
        let per = |b: u64| b as f64 * 60.0 / span.as_secs_f64() / 1024.0;
        eprintln!(
            "[cost] G the phone gone, the desk's face {}, {}s: road {:.1} KB/min to the relay; desk dialled {}, \
             failed {}, wire bytes out {:.1} KB/min, rounds {} (counts {} -> {}); resident pid {pid0:?} -> {:?}",
            if face { "on" } else { "off" },
            span.as_secs(),
            per(ROAD_TO_RELAY.load(Relaxed) - u0),
            c1.dialed.saturating_sub(c0.dialed),
            c1.dial_failed.saturating_sub(c0.dial_failed),
            per(c1.sent_bytes.saturating_sub(c0.sent_bytes)),
            c1.rounds.saturating_sub(c0.rounds),
            c0.rounds,
            c1.rounds,
            serve_pid(&rd.0),
        );
    }
    let log = std::fs::read_to_string(rd.0.join("serve.log")).unwrap_or_default();
    for line in log
        .lines()
        .rev()
        .take(12)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
    {
        eprintln!("[cost] desk log: {line}");
    }
    for sid in idle {
        let _ = khor(&rd.0)
            .args(["close", &sid])
            .env("KHOR_NAME", "desk")
            .output();
    }
}
