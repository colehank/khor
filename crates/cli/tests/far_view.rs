//! **A phone opens a Claude Code session that lives in the Mac's tmux**
//! (2026-10-02, the owner on a real iPhone: the session said it had no
//! host, while the Mac's own window opened it fine).
//!
//! Such a row has no host of its own: khor reaches its terminal through
//! a **viewer** — a lens bridged onto the person's tmux — and a viewer
//! leaves its address in `view.json`, not `host.json`
//! (`khor_node::host::view_file_path`). The machine that hosts the row
//! answered a far `Reach` by standing the viewer up and then reading
//! `host.json` regardless, so every far face asking for such a session
//! got `no-host`. The Mac's window never asked that question: its own
//! terminal door reads the seat.
//!
//! Two in-process nodes: alpha hosts the row (a viewer host standing in
//! for the tmux bridge, so the test needs no tmux), beta is a machine
//! that hosts nothing — a phone (`Node::open_hosting(.., false)`). Both
//! use only a `khor relay --dev` this test starts on loopback
//! (`relays.json`, only-mine): nothing here touches the Khor tier's
//! relays or n0's. Temporary stores and identities only.
//!
//! Its own binary because a viewer host learns it is one from the
//! process environment (`KHOR_HOST_VIEWER`), and that is set here.

use std::io::{BufRead, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use khor_node::Node;
use tokio::time::timeout;

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn scratch(tag: &str) -> Scratch {
    let p = std::env::temp_dir().join(format!("khor-farview-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    Scratch(p)
}

/// A child process, stopped by pid however the test leaves.
struct Reaped(Child);

impl Drop for Reaped {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// `khor relay --dev` on a free loopback port; its url.
fn local_relay(dir: &Path) -> (Reaped, String) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_khor"))
        .env("KHOR_HOME", dir)
        .env_remove("KHOR_RELAY")
        .args(["relay", "--dev", "--bind", "127.0.0.1:0", "--state"])
        .arg(dir.join("state"))
        .stdout(Stdio::piped())
        .stderr(std::fs::File::create(dir.join("relay.log")).unwrap())
        .spawn()
        .unwrap();
    let first = std::io::BufReader::new(child.stdout.take().unwrap())
        .lines()
        .next()
        .expect("a banner line")
        .unwrap();
    let at = first.find("http://").expect("a url on the first line");
    let url = first[at..].split_whitespace().next().unwrap().to_string();
    (Reaped(child), url)
}

/// A home that binds with that relay and nothing else.
fn home_on(dir: &Path, relay: &str) {
    std::fs::create_dir_all(dir.join(".khor")).unwrap();
    std::fs::write(
        dir.join(".khor").join("relays.json"),
        serde_json::json!({ "relays": [{ "url": relay }], "only_mine": true }).to_string(),
    )
    .unwrap();
}

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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_phone_reaches_a_session_whose_terminal_is_a_viewer_on_the_machine_that_hosts_it() {
    // SAFETY: the one test in this binary; nothing else reads the
    // environment while these are set. `KHOR_HOST_VIEWER` makes the
    // in-process host below a viewer — what the tmux bridge spawns for
    // an agent sitting in somebody's tmux (`host::spawn_viewer_host`).
    unsafe {
        std::env::remove_var("KHOR_RELAY");
        std::env::set_var("KHOR_HOST_VIEWER", "keep");
    }
    let (ra, rb, rr) = (scratch("alpha"), scratch("beta"), scratch("relay"));
    let (_relay, url) = local_relay(&rr.0);
    home_on(&ra.0, &url);
    home_on(&rb.0, &url);

    let sa = Node::open_as(ra.0.clone(), "alpha").unwrap();
    let serve_a = tokio::spawn(async move { sa.serve().await });
    let sb = Node::open_hosting(rb.0.clone(), "beta", false).unwrap();
    let serve_b = tokio::spawn(async move { sb.serve_resumable().await });
    wait_for_endpoint_file(&ra.0).await;
    wait_for_endpoint_file(&rb.0).await;
    let a = Node::open_as(ra.0.clone(), "alpha").unwrap();
    let b = Node::open_hosting(rb.0.clone(), "beta", false).unwrap();
    let ticket = a.invite().unwrap();
    timeout(Duration::from_secs(90), b.pair(&ticket))
        .await
        .expect("pairing must not hang")
        .expect("pairing succeeds");

    // On alpha: a Claude Code row whose terminal is a viewer — the
    // `khor-gui _host tui/<uuid> … -- tmux new-session -t … ; select-window`
    // the owner's Mac was running, with a plain program in tmux's place.
    let id = khor_core::SessionId("tui/far-view".into());
    let kind = khor_node::live::LiveKind::new(ra.0.clone(), a.device());
    kind.register(&id, "tui", "claude", None, None).unwrap();
    let dir = kind.dir_of(&id).expect("registered");
    let host = {
        let (root, id) = (ra.0.clone(), id.clone());
        let cmd = vec![
            "/bin/sh".to_owned(),
            "-c".to_owned(),
            "printf 'through-the-viewer\\n'; exec sleep 60".to_owned(),
        ];
        std::thread::spawn(move || khor_node::host::host_main(root, id, (80, 24), cmd))
    };
    let deadline = Instant::now() + Duration::from_secs(30);
    while !khor_node::host::view_file_path(&dir).exists() {
        assert!(Instant::now() < deadline, "the viewer never came up");
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(
        !khor_node::host::host_file_path(&dir).exists(),
        "control: the row has a viewer and no host of its own"
    );
    // Control: alpha's own window opens it — its door reads the seat.
    assert_eq!(
        a.attach_tmux(&id),
        Ok(khor_node::live::Seat::View),
        "control: on alpha the seat is the viewer"
    );

    // The phone reaches it, the way the app's terminal door does.
    let reached = timeout(Duration::from_secs(60), b.reach("alpha", &id))
        .await
        .expect("the reach must come back");
    let (addr, cookie) = match reached {
        Ok(at) => at,
        Err(why) => panic!("the phone must reach the viewer on alpha, it was told: {why}"),
    };
    let mut conn =
        khor_node::host::connect_at(&addr, &cookie, 80, 24).expect("the viewer welcomes the phone");
    let seen = read_until(&mut conn, "through-the-viewer", Duration::from_secs(20));
    assert!(
        seen.contains("through-the-viewer"),
        "what arrives is the viewer's terminal: {seen:?}"
    );

    // A machine that hosts nothing never looks on its own disk for a
    // host: the local doors answer without reading a file.
    assert!(
        b.session_dir(&id).is_none(),
        "the phone has no session dir to attach to"
    );
    assert!(!b.is_hosted(&id), "the phone hosts no terminal");
    assert!(
        b.attach_tmux(&id).is_err(),
        "the phone has nothing to bridge"
    );

    drop(conn);
    // The viewer's program is `sleep 60`; ending the test does not wait
    // for it — the host thread is left to the process's exit.
    drop(host);
    serve_a.abort();
    serve_b.abort();
}
