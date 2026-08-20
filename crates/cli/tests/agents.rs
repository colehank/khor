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
/// The budget here is five seconds: far under that timeout, far over
/// anything this stub needs, so it fails on the behaviour rather than
/// on the machine's mood.
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
        took < Duration::from_secs(5),
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
