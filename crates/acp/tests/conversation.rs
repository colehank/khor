//! One real conversation against the scripted stub — child process,
//! stdio JSON-RPC, nothing in-process. What these prove is the crate's
//! whole contract: updates stream while a prompt is pending, a
//! permission answer **crosses the wire** (the stub echoes back what it
//! received, so the assertion is on the agent's account, not ours), and
//! nothing hangs when this side walks away.

use std::time::Duration;

use khor_acp::{start, Event};
use tokio::time::timeout;

const STUB: &str = env!("CARGO_BIN_EXE_acp-stub");

async fn next(rx: &mut tokio::sync::mpsc::UnboundedReceiver<Event>) -> Event {
    timeout(Duration::from_secs(10), rx.recv())
        .await
        .expect("an event within ten seconds")
        .expect("the channel is open")
}

fn text_of(event: Event) -> String {
    match event {
        Event::Note(n) => format!("{:?}", n.update),
        other => panic!("expected a session update, got {other:?}"),
    }
}

#[tokio::test]
async fn a_turn_streams_and_the_chosen_option_reaches_the_agent() {
    let (handle, mut rx) = start(STUB, std::env::temp_dir()).await.expect("stub starts");
    let Event::Ready { session } = next(&mut rx).await else {
        panic!("first event must be Ready")
    };
    assert_eq!(format!("{session:?}").contains("stub-session-1"), true);

    let turn = tokio::spawn({
        let handle_prompt = async move {
            let stop = handle.prompt("hello").await.expect("the turn completes");
            (handle, stop)
        };
        handle_prompt
    });

    // The two scripted chunks arrive while the prompt is still pending —
    // which is the "updates stream during a turn" half of the contract.
    assert!(text_of(next(&mut rx).await).contains("thinking it over"));
    assert!(text_of(next(&mut rx).await).contains("about to act"));

    let Event::Ask(ask) = next(&mut rx).await else {
        panic!("the stub asks permission after its chunks")
    };
    assert_eq!(ask.request.options.len(), 2, "the stub offers exactly go/stop");
    let go = ask.request.options[0].option_id.0.to_string();
    ask.choose(&go);

    // The stub names what it received: this line is the agent's own
    // account of the answer, not this side's bookkeeping.
    assert!(text_of(next(&mut rx).await).contains("picked:go"));

    let (_handle, stop) = timeout(Duration::from_secs(10), turn)
        .await
        .expect("the turn resolves")
        .expect("the task lives");
    assert_eq!(format!("{stop:?}"), "EndTurn");
}

#[tokio::test]
async fn a_dismissed_ask_reaches_the_agent_as_a_refusal() {
    let (handle, mut rx) = start(STUB, std::env::temp_dir()).await.expect("stub starts");
    let _ready = next(&mut rx).await;
    let turn = tokio::spawn(async move {
        let stop = handle.prompt("hello").await.expect("the turn completes");
        (handle, stop)
    });
    let _chunk = next(&mut rx).await;
    let _chunk = next(&mut rx).await;
    let Event::Ask(ask) = next(&mut rx).await else { panic!("an ask") };
    ask.dismiss();
    assert!(text_of(next(&mut rx).await).contains("dismissed"));
    let _ = timeout(Duration::from_secs(10), turn).await.expect("the turn resolves");
}

/// Walking away without answering must read as a refusal on the agent's
/// side, never as a hang — the spawned responder answers `Cancelled`
/// when the `Ask` drops (lib.rs, the event-loop section).
#[tokio::test]
async fn dropping_an_ask_unanswered_still_ends_the_turn() {
    let (handle, mut rx) = start(STUB, std::env::temp_dir()).await.expect("stub starts");
    let _ready = next(&mut rx).await;
    let turn = tokio::spawn(async move {
        let stop = handle.prompt("hello").await.expect("the turn completes");
        (handle, stop)
    });
    let _chunk = next(&mut rx).await;
    let _chunk = next(&mut rx).await;
    match next(&mut rx).await {
        Event::Ask(ask) => drop(ask),
        other => panic!("an ask, got {other:?}"),
    }
    assert!(text_of(next(&mut rx).await).contains("dismissed"));
    let _ = timeout(Duration::from_secs(10), turn).await.expect("the turn resolves");
}

/// The replay contract: when `replay()` resolves, every replayed update
/// is **already on the receiver** — drained here with `try_recv`, no
/// waiting, because the protocol puts the updates before the
/// `session/load` response and the connection delivers in stream order.
/// An agent that answered first and replayed after would turn every
/// history view into a race; this is the assertion that notices.
#[tokio::test]
async fn replay_is_all_there_when_the_call_answers() {
    let (handle, mut rx) = start(STUB, std::env::temp_dir()).await.expect("stub starts");
    let _ready = next(&mut rx).await;
    handle.replay().await.expect("the load succeeds");
    let mut played = Vec::new();
    while let Ok(event) = rx.try_recv() {
        played.push(text_of(event));
    }
    assert!(
        played.iter().any(|t| t.contains("played back: one"))
            && played.iter().any(|t| t.contains("played back: two")),
        "both replayed chunks are on the channel before anything is awaited: {played:?}"
    );
}

/// Dropping the handle ends everything: the connection task returns,
/// `Closed` arrives, and the child is the library's to kill (process
/// group, npx wrappers included — lib.rs module head).
#[tokio::test]
async fn dropping_the_handle_closes_the_conversation() {
    let (handle, mut rx) = start(STUB, std::env::temp_dir()).await.expect("stub starts");
    let _ready = next(&mut rx).await;
    drop(handle);
    loop {
        match next(&mut rx).await {
            Event::Closed(err) => {
                assert!(err.is_none(), "a close we asked for is not an error: {err:?}");
                break;
            }
            _ => continue,
        }
    }
}

/// The real half: one turn against whatever real ACP agent
/// `ACP_AGENT_CMD` names (run by hand — costs a real model turn).
/// Asserts only what any real agent owes the protocol: a session, at
/// least one update, a stop reason.
///
/// # The recipe, paid for once (2026-08-17, claude-code-acp 0.16.2)
///
/// ```text
/// npm install --prefix <scratch> @zed-industries/claude-code-acp
/// env -u CLAUDECODE -u all_proxy -u http_proxy -u https_proxy \
///   ACP_AGENT_CMD=<scratch>/node_modules/.bin/claude-code-acp \
///   cargo test -p khor-acp --test conversation a_real_agent -- --ignored --nocapture
/// ```
///
/// Each `-u` closes a hole that was hit, not guessed: `CLAUDECODE` trips
/// claude's nested-session guard (khor in production is not inside a
/// claude session; this test usually is). The proxy variables trip a
/// `UND_ERR_INVALID_ARG` in the **bundled** SDK cli — a since-fixed bug
/// (installed claude 2.1.233 handles the same variables) — and on the
/// dev machine they are not needed at all: the system proxy is a TUN
/// that routes transparently. Diagnosed by minimal pairs: bundled cli
/// with proxy env fails, same cli without it answers `ok`, installed
/// cli passes with both.
#[tokio::test]
#[ignore = "needs a real agent command in ACP_AGENT_CMD; costs a real turn"]
async fn a_real_agent_answers_one_tiny_turn() {
    let cmd = std::env::var("ACP_AGENT_CMD").expect("ACP_AGENT_CMD");
    let (handle, mut rx) = start(&cmd, std::env::temp_dir()).await.expect("agent starts");
    let Event::Ready { session } = next(&mut rx).await else { panic!("ready first") };
    println!("session: {session:?}");
    let turn = tokio::spawn(async move {
        let stop = handle.prompt("Reply with exactly: ok").await;
        (handle, stop)
    });
    let mut notes = 0;
    let mut turn = turn;
    // Drain until the turn resolves; a real agent may interleave many
    // update kinds, and which ones is its business.
    let (_handle, stop) = loop {
        tokio::select! {
            event = rx.recv() => {
                match event { Some(Event::Note(n)) => { notes += 1; println!("note: {:?}", n.update); }, Some(e) => println!("{e:?}"), None => panic!("channel closed mid-turn") }
            }
            done = &mut turn => { break done.expect("task lives"); }
        }
    };
    let stop = stop.expect("the turn completes");
    println!("stop: {stop:?}, notes: {notes}");
    assert!(notes > 0, "a real turn streams at least one update");
}
