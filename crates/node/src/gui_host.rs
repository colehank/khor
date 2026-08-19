//! The GUI session's host: one detached process holding one ACP agent.
//!
//! # The second way to open a session (user ruling, 2026-08-17)
//!
//! `khor open --tui` gives the agent a terminal and khor reads its
//! screen; `khor open --gui` gives the agent a **protocol** and khor is
//! the client. Same store, same registry, same six words — what differs
//! is the channel, so this file is `host.rs`'s sibling, not its mode.
//!
//! # The row is registered under the vendor's own session id
//!
//! Registration waits until `session/new` answers, and the id is
//! [`crate::adaptor::id_for`] of the vendor's uuid — the same spelling
//! the disk sweep and the hook path mint. That single decision is what
//! keeps one conversation to one row with no dedup anywhere: the sweep
//! merges into this row by id, and a hook arriving without
//! `KHOR_SESSION` lands on this row too and fills the category khor
//! deliberately does not guess (the argv ruling: an adapter's command
//! name is not a vendor). The id is spelled `tui/…` because that is the
//! vendor-session agreement; **behaviour follows `Meta::kind`, which
//! says `gui`** — an id is an address, not a description.
//!
//! # Words, first-hand
//!
//! Every word here is [`crate::live::Source::Reported`]: khor is the
//! protocol client, so "a prompt is in flight" and "the agent asked
//! permission" are its own facts, not readings of anything. 空闲 while
//! nothing is asked of the agent; 忙碌 while a prompt is pending; 待批
//! while a permission request waits — and the answer clears it back to
//! 忙碌, because the turn is still running (待批答才清). The turn's end
//! maps `EndTurn` → 完成, `Cancelled` → 空闲 (stopping it was the
//! user's own act), and the refusals and ceilings → 中断: the session
//! is alive and continuing needs no restart, which is 中断's exact
//! boundary with 失败.
//!
//! 完成 from a protocol turn-end widens "`Stop` is the only source of
//! 完成": that sentence was written when hooks were the only party that
//! could *know* a turn ended. Here khor is the party the turn ends
//! *at*. Same fact, second first-hand witness — on the ledger.
//!
//! # What the socket speaks
//!
//! The host file and cookie handshake are `host.rs`'s, reused verbatim.
//! After the welcome the stream carries [`GuiNote`] frames out and
//! [`GuiOp`] frames in — length-prefixed msgpack like everything else
//! on this socket, positional arrays, so the wire discipline holds:
//! new fields go at the tail with defaults, none in the middle.
//! **Live frames are broadcast; history is asked for.** An attacher
//! sees the conversation from now on, and one that wants the past sends
//! [`GuiOp::Replay`]: the host asks the agent itself (`session/load`
//! replays the whole conversation before it answers — probed on the
//! real adapter, and re-loading a live session works) and relays the
//! replayed updates as [`GuiNote::History`] frames **to that connection
//! only** — broadcast history would repaint every other face's past.
//! The host still buffers nothing: a second copy here would be a
//! transcript nobody owns. A replay asked for mid-turn waits for the
//! turn to end — the live updates already on screen are the same
//! content, and a load answered during a turn would interleave the two
//! indistinguishably (the protocol does not tag replayed updates).

use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use khor_catalog::msg;
use khor_core::{SessionId, State};
use serde::{Deserialize, Serialize};

use crate::host::{host_file_path, read_frame, write_frame, Hello, HostFile, Welcome};
use crate::live::{LiveKind, Source};

/// What a connected face may do to the conversation.
#[derive(Serialize, Deserialize)]
pub enum GuiOp {
    /// One user turn. The host runs it; updates come back as notes.
    Say(String),
    /// The answer to [`GuiNote::Ask`] with this number: the chosen
    /// option id, or `None` to refuse.
    Answer { ask: u64, option: Option<String> },
    /// End the conversation: the agent dies with the connection. Held
    /// at its position — frames are positional msgpack, and an old host
    /// reading a new client must fail on the frame, never misread an
    /// earlier variant.
    Close,
    /// Ask for the conversation so far, replayed by the agent itself
    /// and answered as [`GuiNote::History`] frames **to this connection
    /// only**, closed by [`GuiNote::HistoryEnd`]. Sent mid-turn it
    /// waits for the turn to end (module head). Tail-appended.
    Replay,
    /// End the turn that is running — not the session. The agent stays,
    /// its conversation stays, and the ending is `Cancelled`, which is
    /// 空闲: stopping it was the user's own act, so nothing is broken.
    /// Tail-appended.
    Stop,
}

/// What the host tells every connected face.
///
/// `Clone` because a pending ask is **re-sent to a face that arrives
/// after it was raised**: the row says 待批, and a face that cannot see
/// what it is being asked has no way to unblock it (found live — a
/// second window on a session mid-ask showed a conversation with no
/// buttons and no way forward).
#[derive(Clone, Serialize, Deserialize)]
pub enum GuiNote {
    /// One `session/update`, as the protocol's own JSON. Passed through
    /// whole on purpose: this host relays a conversation, it does not
    /// curate one — what to paint is the face's judgment.
    Note(String),
    /// The agent asked permission. `options` are `(id, label)` pairs in
    /// the agent's own order; answer by number with [`GuiOp::Answer`].
    Ask { ask: u64, title: String, options: Vec<(String, String)> },
    /// The turn ended, with the protocol's stop reason as a string.
    Turn(String),
    /// The conversation is over; no more frames follow.
    Gone,
    /// One replayed update (`session/update` JSON, [`GuiNote::Note`]'s
    /// shape), answering this connection's [`GuiOp::Replay`] — never
    /// broadcast. Tail-appended.
    History(String),
    /// The replay is complete. Arrives with no [`GuiNote::History`]
    /// before it when there is nothing to replay **or the load failed**
    /// — to a face the two are one fact: no past to paint, the live
    /// stream unaffected. Tail-appended.
    HistoryEnd,
    /// A turn is already running — told to a face the moment it
    /// attaches, and to nobody else. Without it, a second face opens
    /// on a conversation mid-turn with an inviting empty input, and
    /// what it says is dropped on the floor (one turn at a time is the
    /// host's rule, and silence is how it used to say so).
    /// Tail-appended.
    Turning,
    /// An [`GuiNote::Ask`] has its answer: the option id that was
    /// chosen, or `None` for a refusal. Broadcast, and tail-appended.
    ///
    /// Without it, "answered" lives only in the answering face's own
    /// memory — so the next face to read this conversation paints live
    /// buttons on a permission that was settled minutes ago, and
    /// pressing one answers a request that already has an answer.
    Answered { ask: u64, option: Option<String> },
}

/// Where the freshly-minted session id is written for the opener —
/// which cannot know it in advance: the id exists only once the agent
/// has answered `session/new`.
pub fn spawn_gui_host(
    cwd: Option<&std::path::Path>,
    title: &str,
    cmd: &[String],
    vendor: Option<&str>,
) -> Result<SessionId, String> {
    spawn_ghost(cwd, title, cmd, None, vendor)
}

/// [`spawn_gui_host`], but resuming an existing vendor session — the
/// takeover (批C): the ghost loads the recorded conversation instead of
/// opening a fresh one, and re-seats the existing row (`LiveKind::reopen`)
/// instead of registering a new one.
pub fn spawn_gui_host_resume(
    cwd: Option<&std::path::Path>,
    title: &str,
    cmd: &[String],
    session: &str,
) -> Result<SessionId, String> {
    // No vendor: a resume re-seats a row that already has one, and
    // `LiveKind::reopen` does not touch a category anyway.
    spawn_ghost(cwd, title, cmd, Some(session), None)
}

/// How `_ghost` learns it resumes: env, not argv — the argv convention
/// stays what every existing binary parses (`VIEWER_ENV`'s precedent).
const RESUME_ENV: &str = "KHOR_GHOST_RESUME";

/// Whose session this is, when the **opener** knows — the wizard's own
/// 智能体 field, which is the user saying it, not khor guessing from an
/// adapter's command name (the argv ruling in the module head covers
/// the guess; a declaration is not a guess). Absent for anything opened
/// without that answer, and a hook may still fill it in later:
/// `LiveKind::learn_category` never overwrites what is already there.
const VENDOR_ENV: &str = "KHOR_GHOST_VENDOR";

fn spawn_ghost(
    cwd: Option<&std::path::Path>,
    title: &str,
    cmd: &[String],
    resume: Option<&str>,
    vendor: Option<&str>,
) -> Result<SessionId, String> {
    let exe = crate::self_exe()?;
    let ready = std::env::temp_dir().join(format!(
        "khor-gui-ready-{}-{}",
        std::process::id(),
        crate::link::fresh_hex()?
    ));
    let mut c = std::process::Command::new(exe);
    if let Some(cwd) = cwd {
        c.current_dir(cwd);
    }
    match resume {
        Some(sid) => {
            c.env(RESUME_ENV, sid);
        }
        None => {
            c.env_remove(RESUME_ENV);
        }
    }
    match vendor {
        Some(v) => {
            c.env(VENDOR_ENV, v);
        }
        None => {
            c.env_remove(VENDOR_ENV);
        }
    }
    c.arg("_ghost")
        .arg(&ready)
        .arg(title)
        .arg("--")
        .args(cmd)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(unix)]
    unsafe {
        use std::os::unix::process::CommandExt;
        c.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
    c.spawn().map_err(msg::host_wont_start)?;
    // Longer than host.rs's wait on purpose: before the marker can
    // exist an agent process has to boot and answer two requests.
    for _ in 0..600 {
        if let Ok(id) = std::fs::read_to_string(&ready) {
            let _ = std::fs::remove_file(&ready);
            return Ok(SessionId(id.trim().to_owned()));
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    Err(msg::HOST_NEVER_READY.into())
}

struct Fanout {
    clients: Mutex<Vec<(u64, std::net::TcpStream)>>,
}

impl Fanout {
    fn send(&self, note: &GuiNote) {
        let mut clients = self.clients.lock().unwrap();
        clients.retain_mut(|(_, c)| write_frame(c, note).is_ok());
    }

    /// To one connection — history frames only ever travel this way.
    fn send_to(&self, who: u64, note: &GuiNote) {
        let mut clients = self.clients.lock().unwrap();
        clients.retain_mut(|(id, c)| *id != who || write_frame(c, note).is_ok());
    }
}

/// The detached process `_ghost` runs. Blocks until the conversation is
/// over, then reports the ending like any host: a recorded exit, even
/// if nobody is watching.
pub fn gui_host_main(root: PathBuf, ready: PathBuf, title: String, cmd: Vec<String>) -> Result<(), String> {
    let key = khor_net::identity::load_or_create(&root.join(".khor").join("identity.key"))
        .map_err(|e| e.to_string())?;
    let live = LiveKind::new(root, khor_core::DeviceId(*key.public().as_bytes()));

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;

    let command = cmd.join(" ");
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::env::temp_dir());
    let resume = std::env::var(RESUME_ENV).ok();
    let (handle, mut events) = rt
        .block_on(async {
            match &resume {
                Some(sid) => khor_acp::start_resume(&command, cwd, sid).await,
                None => khor_acp::start(&command, cwd).await,
            }
        })
        .map_err(|e| e.to_string())?;

    // The vendor's uuid is the id — the whole merge story hangs on this
    // line (module head).
    let id = crate::adaptor::id_for(&handle.session().0);
    if resume.is_some() {
        // The takeover re-seats a row that already exists (and clears
        // the ending the takeover itself just caused).
        live.reopen(&id, khor_core::kind::GUI, &title, std::process::id())?;
    } else {
        // The vendor the opener declared, if it did: the wizard's own
        // 智能体 field is the user saying whose session this is, which
        // is not the guess the argv ruling forbids.
        let vendor = std::env::var(VENDOR_ENV).ok();
        live.register(
            &id,
            khor_core::kind::GUI,
            &title,
            Some(std::process::id()),
            vendor.as_deref(),
        )?;
        // Register wrote the spawn's 忙碌; a fresh GUI session is a
        // prompt nobody has typed into yet.
        live.report(&id, State::Idle, Source::Reported)?;
    }
    let dir = live.dir_of(&id).ok_or_else(|| msg::not_a_session_id(&id.0))?;

    let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(msg::cant_listen_loopback)?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let cookie = crate::link::fresh_hex()?;
    crate::link::write_private(
        &host_file_path(&dir),
        &serde_json::to_vec(&HostFile {
            port,
            cookie: cookie.clone(),
            host_pid: std::process::id(),
            // `close` kills `-child_pid` then the host: naming the host
            // twice is deliberate. The agent is not in the host's
            // process group (the library gives it its own), so the kill
            // lands here, and dropping the connection takes the agent's
            // whole group with it — the child dies *because* the host
            // does, which is the ordering close wants anyway.
            child_pid: std::process::id(),
        })
        .map_err(|e| e.to_string())?,
    )?;
    std::fs::write(&ready, &id.0).map_err(|e| e.to_string())?;

    let fanout = Arc::new(Fanout { clients: Mutex::new(Vec::new()) });
    let (ops_tx, mut ops_rx) = tokio::sync::mpsc::unbounded_channel::<(u64, GuiOp)>();
    // A face that just arrived, so the loop can hand it what is
    // outstanding. Its own channel rather than a `GuiOp`: joining is
    // not something a face *does*, it is something that happened to
    // this host, and a face must not be able to spell it.
    let (joined_tx, mut joined_rx) = tokio::sync::mpsc::unbounded_channel::<u64>();

    // Attachers: cookie in, frames both ways after the welcome. Each
    // gets a number so a replay can be answered to its asker alone;
    // every other op is the same from anyone.
    {
        let fanout = fanout.clone();
        let cookie = cookie.clone();
        let ops_tx = ops_tx.clone();
        let joined_tx = joined_tx.clone();
        std::thread::spawn(move || {
            let mut next_client: u64 = 0;
            for conn in listener.incoming().flatten() {
                let mut conn = conn;
                let Ok(hello) = read_frame::<Hello>(&mut conn) else { continue };
                if hello.cookie != cookie {
                    let _ = write_frame(&mut conn, &Welcome { ok: false, why: msg::WRONG_COOKIE.into() });
                    continue;
                }
                if write_frame(&mut conn, &Welcome { ok: true, why: String::new() }).is_err() {
                    continue;
                }
                let Ok(mut readable) = conn.try_clone() else { continue };
                let who = next_client;
                next_client += 1;
                fanout.clients.lock().unwrap().push((who, conn));
                let _ = joined_tx.send(who);
                let ops_tx = ops_tx.clone();
                std::thread::spawn(move || {
                    while let Ok(op) = read_frame::<GuiOp>(&mut readable) {
                        if ops_tx.send((who, op)).is_err() {
                            break;
                        }
                    }
                });
            }
        });
    }

    // The conversation loop: one task, owning every state change, so
    // the words cannot race each other.
    let loop_live = live.clone();
    let loop_id = id.clone();
    let loop_fanout = fanout.clone();
    let ending = rt.block_on(async move {
        let (live, id, fanout) = (loop_live, loop_id, loop_fanout);
        // The pending asks, each with the very note it was announced
        // by: a face that arrives later must be told the same sentence
        // the first face got, not a second rendering of it.
        let mut asks: Vec<(u64, khor_acp::Ask, GuiNote)> = Vec::new();
        let mut next_ask: u64 = 0;
        // Who asked for history while a turn was running: answered when
        // the turn ends (module head — a mid-turn load would interleave
        // replayed and live updates indistinguishably).
        let mut replay_wanted: Vec<u64> = Vec::new();
        let mut in_turn: Option<tokio::task::JoinHandle<()>> = None;
        let (turn_tx, mut turn_rx) = tokio::sync::mpsc::unbounded_channel::<Result<String, String>>();
        let handle = Arc::new(handle);
        loop {
            tokio::select! {
                // Biased, events first: a turn's last update and its end
                // become ready together, and a face must read the words
                // before the goodbye — an unbiased pick would reorder
                // them on some runs and every screen would inherit the
                // dice.
                biased;
                event = events.recv() => match event {
                    Some(khor_acp::Event::Note(n)) => {
                        let json = serde_json::to_string(&n).unwrap_or_default();
                        fanout.send(&GuiNote::Note(json));
                    }
                    Some(khor_acp::Event::Ask(ask)) => {
                        let n = next_ask;
                        next_ask += 1;
                        let title = ask.request.tool_call.fields.title.clone().unwrap_or_default();
                        let options = ask
                            .request
                            .options
                            .iter()
                            .map(|o| (o.option_id.0.to_string(), o.name.clone()))
                            .collect();
                        let note = GuiNote::Ask { ask: n, title, options };
                        fanout.send(&note);
                        asks.push((n, ask, note));
                        let _ = live.report(&id, State::Blocked, Source::Reported);
                    }
                    Some(khor_acp::Event::Ready { .. }) => {}
                    // A channel that just stops is an ending too, and
                    // painting it any other way would need a word khor
                    // does not have.
                    Some(khor_acp::Event::Closed(err)) => break err,
                    None => break None,
                },
                // A face that arrived mid-ask gets the ask. Nothing
                // else is re-sent: the conversation's past is what
                // `Replay` is for, and the row already carries the word.
                who = joined_rx.recv() => {
                    if let Some(who) = who {
                        if in_turn.is_some() {
                            fanout.send_to(who, &GuiNote::Turning);
                        }
                        for (_, _, note) in &asks {
                            fanout.send_to(who, note);
                        }
                    }
                }
                op = ops_rx.recv() => match op {
                    Some((_, GuiOp::Say(text))) => {
                        if in_turn.is_some() {
                            // One turn at a time; a queue would answer
                            // "who said what when" wrongly on every face.
                            continue;
                        }
                        let _ = live.report(&id, State::Busy, Source::Reported);
                        let handle = handle.clone();
                        let turn_tx = turn_tx.clone();
                        in_turn = Some(tokio::spawn(async move {
                            let out = handle
                                .prompt(&text)
                                .await
                                .map(|stop| format!("{stop:?}"));
                            let _ = turn_tx.send(out);
                        }));
                    }
                    Some((_, GuiOp::Answer { ask, option })) => {
                        let Some(seat) = asks.iter().position(|(n, _, _)| *n == ask) else {
                            continue;
                        };
                        let (_, pending, _) = asks.swap_remove(seat);
                        match &option {
                            Some(oid) => pending.choose(oid),
                            None => pending.dismiss(),
                        }
                        // Every face learns the answer, including the
                        // ones that arrive afterwards: the record of a
                        // settled permission outlives the answering.
                        fanout.send(&GuiNote::Answered { ask, option });
                        if asks.is_empty() {
                            // 待批答才清 — and it clears to 忙碌: the
                            // turn the ask interrupted is still running.
                            let _ = live.report(&id, State::Busy, Source::Reported);
                        }
                    }
                    Some((_, GuiOp::Stop)) => {
                        // The cancel goes to the agent, not to the task:
                        // aborting the join handle here would leave the
                        // agent still working with nobody reading it.
                        // The turn ends the way every turn ends, through
                        // its own `Turn` note.
                        if in_turn.is_some() {
                            let _ = handle.cancel();
                        }
                    }
                    Some((_, GuiOp::Close)) => break None,
                    Some((who, GuiOp::Replay)) => {
                        if in_turn.is_some() {
                            replay_wanted.push(who);
                        } else if let Err(e) = replay_to(&handle, &mut events, &fanout, who).await {
                            break e;
                        }
                    }
                    None => break None,
                },
                turn = turn_rx.recv() => {
                    in_turn = None;
                    match turn {
                        Some(Ok(stop)) => {
                            let word = match stop.as_str() {
                                "EndTurn" => State::Done,
                                "Cancelled" => State::Idle,
                                _ => State::Errored,
                            };
                            let _ = live.report(&id, word, Source::Reported);
                            fanout.send(&GuiNote::Turn(stop));
                        }
                        Some(Err(e)) => {
                            let _ = live.report(&id, State::Errored, Source::Reported);
                            fanout.send(&GuiNote::Turn(e));
                        }
                        None => {}
                    }
                    // The turn is over: the replays that waited on it,
                    // in the order they were asked for.
                    let mut fell = false;
                    let mut fall = None;
                    for who in replay_wanted.drain(..) {
                        if let Err(e) = replay_to(&handle, &mut events, &fanout, who).await {
                            (fell, fall) = (true, e);
                            break;
                        }
                    }
                    if fell {
                        break fall;
                    }
                }
            }
        }
    });

    fanout.send(&GuiNote::Gone);
    for (_, c) in fanout.clients.lock().unwrap().drain(..) {
        let _ = c.shutdown(std::net::Shutdown::Both);
    }
    // 0 for an ending nobody minded, 1 for a torn connection: the exit
    // code is how `record_exit` tells 完成-shaped endings from 失败.
    let _ = live.record_exit(&id, if ending.is_none() { 0 } else { 1 });
    Ok(())
}

/// One replay, answered to one connection: ask the agent
/// ([`khor_acp::Handle::replay`] — every replayed update is on the
/// events channel when it resolves), drain, relay as [`GuiNote::History`]
/// to the asker, close with [`GuiNote::HistoryEnd`]. Runs only between
/// turns, so what the drain can meet besides replayed notes is an
/// ending — returned as `Err` for the caller to break the loop with.
/// A failed load relays nothing and still closes the bracket
/// ([`GuiNote::HistoryEnd`]'s doc): a partial replay would paint a past
/// that silently stops short, which is worse than no past at all.
async fn replay_to(
    handle: &khor_acp::Handle,
    events: &mut tokio::sync::mpsc::UnboundedReceiver<khor_acp::Event>,
    fanout: &Fanout,
    who: u64,
) -> Result<(), Option<String>> {
    let loaded = handle.replay().await;
    let mut fell = false;
    let mut fall = None;
    while let Ok(event) = events.try_recv() {
        match event {
            khor_acp::Event::Note(n) => {
                if loaded.is_ok() {
                    let json = serde_json::to_string(&n).unwrap_or_default();
                    fanout.send_to(who, &GuiNote::History(json));
                }
            }
            khor_acp::Event::Closed(err) => (fell, fall) = (true, err),
            _ => {}
        }
    }
    fanout.send_to(who, &GuiNote::HistoryEnd);
    if fell {
        Err(fall)
    } else {
        Ok(())
    }
}
