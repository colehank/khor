//! 在哪台: a conversation born on **another machine** from this one's
//! wizard, and spoken to from here.
//!
//! The far machine is a real `khor serve` (the binary, under a home of
//! its own) because the road under test ends in a ghost that machine
//! spawns from its own executable — an in-process node would spawn the
//! test harness as the ghost. This side is an in-process node with a
//! resident of its own, because reaching a far host needs one (the
//! pipe has to outlive the call that asked for it).
//!
//! What is asserted is the agent's account and the far machine's
//! account, never this side's bookkeeping: alpha's own session list
//! carries the row, alpha's own places carry the directory, and the
//! echo that comes back through the pipe is the fake claude's.

use std::fs;
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

use khor_node::Node;
use khor_node::gui_host::{GuiNote, GuiOp};
use khor_node::host::{connect_at, read_frame, write_frame};
use tokio::time::timeout;

fn root(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("khor-far-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

fn wait_for(what: &str, secs: u64, mut f: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(secs);
    while Instant::now() < deadline {
        if f() {
            return;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    panic!("timed out waiting for {what}");
}

/// Kills the far serve (its keeper) when the test ends, however it ends.
struct Reaper(u32);
impl Drop for Reaper {
    fn drop(&mut self) {
        let _ = Command::new("kill")
            .args(["-TERM", &self.0.to_string()])
            .status();
    }
}

/// A claude that echoes — the shim's twin in `cagent.rs`, cut to the
/// one turn this test needs.
const FAKE_CLAUDE: &str = r#"#!/usr/bin/env python3
import json, sys
args = sys.argv[1:]
sid = args[args.index("--session-id") + 1]
def emit(o):
    sys.stdout.write(json.dumps(o) + "\n"); sys.stdout.flush()
first = True
for line in sys.stdin:
    m = json.loads(line)
    if m.get("type") != "user":
        continue
    text = m["message"]["content"][0]["text"]
    if first:
        first = False
        emit({"type": "system", "subtype": "init", "session_id": sid, "slash_commands": []})
    emit({"type": "assistant", "message": {"content": [{"type": "text", "text": "echo: " + text}]}})
    emit({"type": "result", "subtype": "success", "session_id": sid})
"#;

fn next_note(conn: &mut TcpStream) -> GuiNote {
    read_frame::<GuiNote>(conn).expect("a frame from the far host")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_conversation_is_born_on_the_machine_the_wizard_named_and_spoken_to_from_here() {
    let ra = root("alpha");
    let rb = root("beta");
    let fake = ra.join("claude.py");
    fs::write(&fake, FAKE_CLAUDE).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let far_dir = ra.join("work");
    fs::create_dir_all(&far_dir).unwrap();
    // As the ghost will see it: it learns its place from its own
    // `current_dir`, which is canonical — on macOS the temp dir sits
    // behind a `/var` → `/private/var` link.
    let far_dir = far_dir.canonicalize().unwrap();

    // alpha: the real binary, resident, with the fake as its claude.
    // Never `wait()`ed on purpose: `Reaper` below owns its end and signals
    // it by pid, so the test can leave however it leaves.
    #[allow(clippy::zombie_processes)]
    let keeper = Command::new(env!("CARGO_BIN_EXE_khor"))
        .arg("serve")
        .env("KHOR_HOME", &ra)
        .env("KHOR_NAME", "alpha")
        .env("KHOR_CLAUDE", &fake)
        .env_remove("KHOR_SESSION")
        .stdout(std::process::Stdio::null())
        .stderr(fs::File::create(ra.join("serve.log")).unwrap())
        .spawn()
        .unwrap();
    let _reap = Reaper(keeper.id());
    wait_for("alpha's serve to stand up", 30, || {
        ra.join(".khor").join("endpoint.json").exists()
    });

    // beta: in-process, with a resident of its own — the pairing and
    // the reach both go through it, the way the app's do.
    let serve_b = Node::open_as(rb.clone(), "beta").unwrap();
    let serve = tokio::spawn(async move { serve_b.serve().await });
    // Awaited, not slept: the serve is a task on this runtime, and a
    // thread asleep is a worker it cannot have. Ninety seconds because
    // binding an endpoint here waits on a relay handshake first
    // (`pairing.rs` gives it the same).
    let endpoint_b = rb.join(".khor").join("endpoint.json");
    timeout(Duration::from_secs(90), async {
        while !endpoint_b.exists() {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("beta's serve to stand up");
    let a = Node::open_as(ra.clone(), "alpha").unwrap();
    let ticket = a
        .invite()
        .expect("alpha mints a ticket through its resident");
    let b = Node::open_as(rb.clone(), "beta").unwrap();
    let peer = timeout(Duration::from_secs(90), b.pair(&ticket))
        .await
        .expect("pairing must not hang")
        .expect("pairing succeeds");
    assert_eq!(peer, "alpha");

    // The wizard's four answers, carried as a name and resolved on alpha.
    let id = timeout(
        Duration::from_secs(60),
        b.open_agent_on(
            "alpha",
            &far_dir.display().to_string(),
            "far-one",
            "chat",
            "claude",
        ),
    )
    .await
    .expect("the far open must not hang")
    .expect("alpha opens it");

    // alpha's own account: the row is its, born as a conversation.
    let rows = a.sessions().expect("alpha lists its sessions");
    let mine = rows
        .iter()
        .find(|v| v.session.id == id)
        .unwrap_or_else(|| panic!("alpha's list has no {}: {rows:?}", id.0));
    assert_eq!(
        mine.session.kind.0,
        khor_core::kind::GUI,
        "born as a conversation"
    );
    assert_eq!(mine.session.title, "far-one");
    assert_eq!(
        mine.session.place.as_deref(),
        Some(far_dir.display().to_string().as_str()),
        "born standing in the directory the wizard named"
    );

    // And alpha's places, read from here, carry that directory — the
    // wizard's first question answered for the machine it asks about.
    let places = timeout(Duration::from_secs(30), b.places_on("alpha"))
        .await
        .expect("places must not hang")
        .expect("alpha answers its places");
    assert!(
        places
            .iter()
            .any(|(path, ..)| *path == far_dir.display().to_string()),
        "alpha's places must carry the far directory: {places:?}"
    );

    // The control: a name alpha never registered is refused *by alpha*,
    // in its registry's own words — alpha has registered nothing, and
    // that is the sentence — not opened on some default.
    let refused = timeout(
        Duration::from_secs(30),
        b.open_agent_on(
            "alpha",
            &far_dir.display().to_string(),
            "x",
            "chat",
            "nobody-here",
        ),
    )
    .await
    .expect("the refusal must not hang");
    assert_eq!(
        refused.as_ref().err().map(String::as_str),
        Some(khor_catalog::msg::NO_AGENTS_REGISTERED),
        "an unregistered name must come back as alpha's own refusal: {refused:?}"
    );

    // Spoken to from here: the resident pipes onto alpha's ghost, and
    // what arrives is the host protocol — the same frames a local face
    // reads, with the fake claude's echo as the proof the line went
    // the whole way round.
    let (addr, cookie) = timeout(Duration::from_secs(30), b.reach("alpha", &id))
        .await
        .expect("the reach must not hang")
        .expect("beta's resident pipes onto alpha's host");
    let mut conn = connect_at(&addr, &cookie, 0, 0).expect("the far host welcomes the pipe");
    conn.set_read_timeout(Some(Duration::from_secs(30)))
        .unwrap();
    let GuiNote::Agent { .. } = next_note(&mut conn) else {
        panic!("the facts come first")
    };
    let GuiNote::Settings(_) = next_note(&mut conn) else {
        panic!("then the settings — alpha's shim lists claude's modes")
    };
    write_frame(&mut conn, &GuiOp::Say("hello far".into())).unwrap();
    let mut echoed = false;
    loop {
        match next_note(&mut conn) {
            GuiNote::Note(json) => echoed |= json.contains("echo: hello far"),
            GuiNote::Turn(stop) => {
                assert_eq!(stop, "EndTurn");
                break;
            }
            _ => {}
        }
    }
    assert!(
        echoed,
        "the fake claude's echo must come back through the pipe"
    );

    write_frame(&mut conn, &GuiOp::Close).unwrap();
    let GuiNote::Gone = next_note(&mut conn) else {
        panic!("a goodbye")
    };
    serve.abort();
    let _ = fs::remove_dir_all(&rb);
}
