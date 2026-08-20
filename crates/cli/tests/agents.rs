//! The generic ACP gate through the real binary (批⑥): a person names
//! an agent, opens a session with it, and the row says whose it is.
//!
//! The agent here is `acp-stub` — khor's scripted ACP agent, which khor
//! knows nothing about beyond "a command that speaks the protocol".
//! That ignorance is the point: everything these assert would work the
//! same for gemini or for something written this afternoon.
//!
//! Every wait has a deadline, and every session these open is closed —
//! `open --gui` spawns a **detached** host, so a test that walks away
//! leaves one behind (the orphan discipline).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

fn khor(home: &Path) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_khor"));
    c.env("KHOR_HOME", home).env("KHOR_NAME", "box").env_remove("KHOR_SESSION");
    c
}

/// Runs a verb and hands back `(ok, stdout, stderr)` — refusals are as
/// much a subject here as successes, so failure is not an assertion.
fn run(home: &Path, args: &[&str]) -> (bool, String, String) {
    let out = khor(home).args(args).output().expect("khor runs");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// The scripted agent, built by the same cargo that is running this and
/// found beside this test's own binary (`gui_session.rs`'s recipe —
/// `CARGO_BIN_EXE_` cannot name another crate's binary).
fn stub() -> PathBuf {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let status = Command::new(cargo)
        .args(["build", "-q", "-p", "khor-acp", "--bin", "acp-stub"])
        .status()
        .expect("cargo runs");
    assert!(status.success(), "the stub builds");
    let me = std::env::current_exe().expect("a test binary");
    me.parent().and_then(|d| d.parent()).expect("target/debug").join("acp-stub")
}

fn home(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("khor-agents-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

/// Closes a session when it goes out of scope, however it goes.
///
/// **An assertion panics past its cleanup; a `Drop` does not.**
/// `open.rs` learned this the same way this file did: three detached
/// ghosts were found alive after a round of deliberate red checks, each
/// from a test that opened a session and then failed at an assertion
/// written above the close. The leak is invisible while the tests pass,
/// which is exactly when nobody looks.
struct Closer {
    home: PathBuf,
    id: String,
}

impl Drop for Closer {
    fn drop(&mut self) {
        let _ = khor(&self.home).args(["close", &self.id]).output();
        // The row leaving the list is the host having gone; without the
        // wait a test can finish while its ghost is still winding down.
        let deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < deadline {
            let out = khor(&self.home).arg("sessions").output();
            match out {
                Ok(o) if !String::from_utf8_lossy(&o.stdout).contains(&self.id) => return,
                _ => std::thread::sleep(Duration::from_millis(200)),
            }
        }
    }
}

/// **A person names an agent khor has never heard of, and opens a
/// session with it.** The whole batch in one path: register, open, and
/// the row lands under the name its owner gave it.
///
/// The category assertion is the load-bearing one. khor refuses to
/// *guess* a vendor from a command line, and this row's command is a
/// path to a binary called `acp-stub` — so the only way `zed` reaches
/// the row is the registration, which is the user saying it.
#[test]
fn an_agent_nobody_shipped_opens_a_session_that_says_whose_it_is() {
    let home = home("open");
    let (ok, _, err) =
        run(&home, &["agents", "add", "zed", "--", &stub().to_string_lossy()]);
    assert!(ok, "registering: {err}");

    let (ok, listed, err) = run(&home, &["agents"]);
    assert!(ok, "{err}");
    assert!(listed.contains("zed"), "the registration lists under its name: {listed}");
    assert!(listed.contains("acp-stub"), "and spells the command back: {listed}");

    let (ok, id, err) = run(&home, &["open", "--gui", "--agent", "zed"]);
    assert!(ok, "opening: {err}");
    let id = id.trim().to_owned();
    assert!(!id.is_empty(), "the id is the deliverable");
    // Handed over **before** the first assertion that could panic.
    let _closer = Closer { home: home.clone(), id: id.clone() };

    let (ok, rows, err) = run(&home, &["sessions", "--by", "category"]);
    assert!(ok, "{err}");
    // **The group heading, not the word anywhere on the screen.** The
    // first spelling of this assertion looked for "zed" in the output
    // and passed with the category torn out — because the title
    // defaults to the agent's name too, so the row said "zed" either
    // way. A category group is the one place only a category can put
    // it (`list::GROUP_CATEGORY`, printed through `cli::group_header`).
    assert!(
        rows.contains("── zed"),
        "the row is filed under the name its owner gave the agent: {rows}"
    );

    // The close itself is asserted here rather than left to the guard:
    // that it *works* is a fact worth a red, and the guard is only
    // there for the paths that never reach this line.
    let (ok, _, err) = run(&home, &["close", &id]);
    assert!(ok, "closing: {err}");
}

/// **A name nobody registered is refused by name, and never falls back
/// to a shell.** The fallback is what makes this worth a test: `open`
/// with no command opens the user's shell, so a typo in `--agent` that
/// fell through would open a *working* session on the wrong thing —
/// success is the failure mode.
#[test]
fn a_typo_in_the_agent_name_opens_nothing() {
    let home = home("typo");
    let (ok, _, _) = run(&home, &["agents", "add", "zed", "--", &stub().to_string_lossy()]);
    assert!(ok);
    let (ok, out, err) = run(&home, &["open", "--gui", "--agent", "zedd"]);
    assert!(!ok, "a name nobody registered opens nothing, got: {out}");
    assert!(err.contains("zedd"), "the refusal names what was typed: {err}");
    assert!(err.contains("zed"), "and what is registered: {err}");
    assert!(
        run(&home, &["sessions"]).1.lines().all(|l| !l.contains("tui/")),
        "no session was opened"
    );
    let _ = std::fs::remove_dir_all(&home);
}

/// **`--gui --on <machine>` is refused out loud.**
///
/// It used to be accepted and ignored — the gui branch returned before
/// `--on` was ever read — so the session opened *here* and the screen
/// said nothing about it. A flag that is silently dropped is worse than
/// one that is rejected, because everything visible agrees with the
/// person: they asked for the far machine, they got an id, and only the
/// row's home says otherwise.
#[test]
fn asking_for_a_conversation_on_another_machine_is_refused_not_ignored() {
    let home = home("on");
    let (ok, out, err) =
        run(&home, &["open", "--gui", "--on", "turing", "--", &stub().to_string_lossy()]);
    assert!(!ok, "it must not quietly open one here, got: {out}");
    assert!(err.contains("--on"), "the refusal names the flag it is about: {err}");
    assert!(
        run(&home, &["sessions"]).1.lines().all(|l| !l.contains("tui/")),
        "and nothing was opened locally"
    );
    let _ = std::fs::remove_dir_all(&home);
}

/// **An agent that wants a login says so, in its own words, at once.**
///
/// Timed on purpose. The opener waits on a file the ghost writes, and
/// the ghost's stderr goes nowhere — so before the reason was left
/// beside that file, *every* refusal ended at the opener's own
/// thirty-second timeout wearing the sentence "the host never came up".
///
/// **The sentence is the load-bearing assertion, not the clock.** The
/// login words can only come from the ghost leaving its reason where
/// the opener looks; the old path had exactly one ending, and it said
/// something else. The budget is corroboration — it guards a future
/// where the reason is written but only read after the loop.
///
/// Fifteen seconds, not five. Five is what this was, and this file had
/// one unexplained failure during a full sweep whose message was not
/// captured and which did not recur in seventeen runs. This is the only
/// assertion here that can go red for a reason other than the
/// behaviour, so it is the only one worth loosening on suspicion — and
/// half the timeout it is distinguishing itself from is still an
/// unambiguous answer. The message prints the elapsed time, so a
/// recurrence names its own number instead of needing a guess.
#[test]
fn an_agent_that_wants_a_login_says_so_before_any_timeout_could() {
    let home = home("login");
    let launch = serde_json::json!({
        "command": stub().to_string_lossy(),
        "env": { "KHOR_STUB_LOGIN": "1" },
    })
    .to_string();
    let began = Instant::now();
    let (ok, out, err) = run(&home, &["open", "--gui", "--", &launch]);
    let took = began.elapsed();
    assert!(!ok, "an agent demanding a login opens no session, got: {out}");
    assert!(err.contains("登录"), "it says what is wrong: {err}");
    assert!(
        took < Duration::from_secs(15),
        "the refusal must arrive at the refusal, not at the opener's timeout: {took:?}"
    );
    let _ = std::fs::remove_dir_all(&home);
}

/// The control for the one above: a command that is not an agent at all
/// must not borrow the login sentence. Without it, mapping every
/// refusal onto "log in first" would pass that test and be wrong about
/// every case in the world.
#[test]
fn a_command_that_is_not_an_agent_gets_its_own_sentence() {
    let home = home("notagent");
    let (ok, _, err) = run(&home, &["open", "--gui", "--", "/nonexistent/khor-no-such-agent"]);
    assert!(!ok);
    assert!(!err.contains("登录"), "a missing binary is not a login problem: {err}");
    assert!(err.contains("ACP"), "it says what khor was expecting instead: {err}");
    let _ = std::fs::remove_dir_all(&home);
}

/// **An ad-hoc command gets no category at all, and does not borrow
/// one.** The other half of the assertion above: a name reaches a row
/// because somebody said it, so a session opened without a
/// registration must sit in the "could not tell" group rather than in
/// a neighbour's (docs/SESSION.md 认不出就不落词).
///
/// It is also the control that keeps the test above honest about
/// *where* the name comes from: if khor read the vendor off the command
/// line, this row would be categorised too.
#[test]
fn a_command_opened_without_a_registration_is_filed_as_unplaceable() {
    let home = home("adhoc");
    let (ok, id, err) = run(&home, &["open", "--gui", "--", &stub().to_string_lossy()]);
    assert!(ok, "opening: {err}");
    let id = id.trim().to_owned();
    let _closer = Closer { home: home.clone(), id: id.clone() };

    let (ok, rows, err) = run(&home, &["sessions", "--by", "category"]);
    assert!(ok, "{err}");
    assert!(
        rows.contains("认不出的"),
        "a row khor cannot attribute says so: {rows}"
    );
    assert!(
        !rows.contains("── zed") && !rows.contains("── claude") && !rows.contains("── codex"),
        "and it borrows nobody's group: {rows}"
    );

    let (ok, _, err) = run(&home, &["close", &id]);
    assert!(ok, "closing: {err}");
}

/// **The whole gate, against a real third-party agent** — run by hand,
/// because it costs a real model turn.
///
/// The recipe, spelled out rather than pointed at (2026-08-21,
/// `@zed-industries/claude-code-acp` 0.16.2, no new credentials — it
/// rides the claude login the machine already has):
///
/// ```text
/// npm install --prefix <scratch> @zed-industries/claude-code-acp
/// env -u CLAUDECODE -u CLAUDE_CODE_ENTRYPOINT \
///     -u all_proxy -u http_proxy -u https_proxy \
///     ACP_AGENT_CMD=<scratch>/node_modules/.bin/claude-code-acp \
///     cargo test -p khor-cli --test agents a_real_third_party \
///         -- --ignored --nocapture
/// ```
///
/// Each `-u` closes a hole that was hit rather than guessed:
/// `CLAUDECODE` and `CLAUDE_CODE_ENTRYPOINT` trip claude's
/// nested-session guard (khor in production is not inside a claude
/// session; this test usually is), and the proxy variables trip an
/// `UND_ERR_INVALID_ARG` in the bundled SDK cli.
///
/// What it printed on the run that this was written against:
/// `name=Some("@zed-industries/claude-code-acp") replays=true`, one
/// permission ask offering **three** options
/// (`allow_always` / `allow` / `reject` — the shims offer two, which is
/// why nothing here indexes into that menu), seven streamed chunks,
/// `EndTurn`, four replayed frames.
///
/// Everything here goes through the product's own path: the person
/// registers a command by name, opens a session with it, and talks to
/// the host over the same socket a face uses. khor knows nothing about
/// this agent — no adaptor, no vendor constant, no file format — which
/// is exactly the claim the batch is making.
///
/// Prints what the agent said about itself rather than asserting it: a
/// third party's capabilities are its business, and a test that
/// demanded `replays: true` would be asserting somebody else's roadmap.
#[test]
#[ignore = "needs a real ACP agent in ACP_AGENT_CMD; costs a real turn"]
fn a_real_third_party_agent_goes_through_the_whole_gate() {
    use khor_core::DeviceId;
    use khor_node::gui_host::{GuiNote, GuiOp};
    use khor_node::host::{read_frame, read_host_file, write_frame, Hello, Welcome};
    use khor_node::live::LiveKind;
    use std::net::TcpStream;

    let cmd = std::env::var("ACP_AGENT_CMD").expect("ACP_AGENT_CMD");
    let home = home("real");
    let (ok, _, err) = run(&home, &["agents", "add", "zed", "--", &cmd]);
    assert!(ok, "registering: {err}");

    let (ok, id, err) = run(&home, &["open", "--gui", "--agent", "zed"]);
    assert!(ok, "opening: {err}");
    let id = id.trim().to_owned();
    let _closer = Closer { home: home.clone(), id: id.clone() };
    println!("session: {id}");

    let (ok, rows, _) = run(&home, &["sessions", "--by", "category"]);
    assert!(ok);
    assert!(rows.contains("── zed"), "a real agent's row says whose it is too: {rows}");

    let k = LiveKind::new(home.clone(), DeviceId([1; 32]));
    let hf = read_host_file(&k.dir_of(&khor_core::SessionId(id.clone())).expect("a session dir"))
        .expect("a host file");
    let mut conn = TcpStream::connect(("127.0.0.1", hf.port)).expect("the host listens");
    conn.set_read_timeout(Some(Duration::from_secs(120))).unwrap();
    write_frame(&mut conn, &Hello { cookie: hf.cookie, cols: 0, rows: 0 }).unwrap();
    let w: Welcome = read_frame(&mut conn).unwrap();
    assert!(w.ok, "{}", w.why);

    let GuiNote::Agent { name, replays } = read_frame::<GuiNote>(&mut conn).unwrap() else {
        panic!("the facts come first")
    };
    println!("agent says: name={name:?} replays={replays}");

    // A turn that is likely to want permission — the ask is answered if
    // it comes and not asserted if it does not: whether an agent gates
    // a given act is its own policy, and a test that required one would
    // be asserting that policy rather than khor's handling of it.
    // The path is absolute and inside this test's own home. It was
    // relative once, and a real agent duly wrote into the repository —
    // the session's cwd is wherever the opener stood, which under
    // `cargo test` is the package directory. A test that litters a
    // shared working tree is a test that somebody else commits.
    let target = home.join("khor-gate-ok.txt");
    write_frame(
        &mut conn,
        &GuiOp::Say(format!(
            "Create a file at {} containing the word ok. Do not touch anything else.",
            target.display()
        )),
    )
    .unwrap();
    let mut chunks = 0;
    let mut asked = 0;
    let stop = loop {
        match read_frame::<GuiNote>(&mut conn).expect("a frame within two minutes") {
            GuiNote::Note(_) => chunks += 1,
            GuiNote::Ask { ask, title, options } => {
                asked += 1;
                println!("asked: {title} {options:?}");
                let go = options.first().map(|(id, _)| id.clone());
                write_frame(&mut conn, &GuiOp::Answer { ask, option: go }).unwrap();
            }
            GuiNote::Answered { .. } | GuiNote::Turning => {}
            GuiNote::Turn(stop) => break stop,
            other => panic!("unexpected frame before the turn ended: {}", kind_of(&other)),
        }
    };
    println!("stop={stop} chunks={chunks} asks={asked}");
    assert!(chunks > 0, "a real turn streams at least one update");

    // The past, asked for through the same door a face uses.
    write_frame(&mut conn, &GuiOp::Replay).unwrap();
    let mut played = 0;
    loop {
        match read_frame::<GuiNote>(&mut conn).expect("a frame") {
            GuiNote::History(_) => played += 1,
            GuiNote::HistoryEnd => break,
            _ => {}
        }
    }
    println!("replayed={played} (replays={replays})");
    if replays {
        assert!(played > 0, "an agent that advertises replay must replay something");
    }

    // The approval was answered, so the act it gated must have
    // happened — otherwise "answering an ask" is this side's own
    // bookkeeping rather than something that crossed the wire. Only
    // asserted when there *was* an ask (the agent's own policy).
    if asked > 0 {
        assert!(target.exists(), "the allowed write really landed: {}", target.display());
    }

    // **The control, on the same connection: a refusal must really
    // refuse.** Without it, an answer channel that quietly approves
    // everything passes every assertion above — "it worked" proves
    // nothing until the thing that should not work does not.
    let denied = home.join("khor-gate-denied.txt");
    write_frame(
        &mut conn,
        &GuiOp::Say(format!(
            "Create a file at {} containing the word no. Do not touch anything else.",
            denied.display()
        )),
    )
    .unwrap();
    let mut refused = 0;
    loop {
        match read_frame::<GuiNote>(&mut conn).expect("a frame within two minutes") {
            GuiNote::Ask { ask, options, .. } => {
                refused += 1;
                // The option the agent itself labels as a rejection,
                // by kind rather than by position: the real agent
                // offers three (allow_always / allow / reject) and the
                // shims offer two, so an index would be a guess about
                // somebody else's menu.
                let no = options.iter().find(|(id, _)| id.contains("reject") || id.contains("deny"));
                write_frame(&mut conn, &GuiOp::Answer { ask, option: no.map(|(id, _)| id.clone()) })
                    .unwrap();
            }
            GuiNote::Turn(stop) => {
                println!("denied turn: stop={stop} asks={refused}");
                break;
            }
            _ => {}
        }
    }
    if refused > 0 {
        assert!(
            !denied.exists(),
            "a refused write must not land: {} exists",
            denied.display()
        );
    }

    write_frame(&mut conn, &GuiOp::Close).unwrap();
}

fn kind_of(n: &khor_node::gui_host::GuiNote) -> &'static str {
    use khor_node::gui_host::GuiNote;
    match n {
        GuiNote::Note(_) => "Note",
        GuiNote::Ask { .. } => "Ask",
        GuiNote::Turn(_) => "Turn",
        GuiNote::Gone => "Gone",
        GuiNote::History(_) => "History",
        GuiNote::HistoryEnd => "HistoryEnd",
        GuiNote::Turning => "Turning",
        GuiNote::Answered { .. } => "Answered",
        GuiNote::Agent { .. } => "Agent",
    }
}
