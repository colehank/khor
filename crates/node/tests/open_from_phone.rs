//! **A shell opened from a phone on a computer** (2026-10-14, the
//! owner's iPhone: 新建 on MacLab spun on and on while MacLab had opened
//! the shell). Two scratch homes over a real wire: a desk that hosts, and
//! a phone that does not (`Node::open_hosting(.., false)`) with the
//! resumable resident the app runs. The phone's face asks the desk for a
//! shell the way the wizard does (`open_on`, a create token, through the
//! resident), and then waits for that id on its own list — which is what
//! the app's pane waits for (`App.tsx` `waitingFor`) before it draws the
//! terminal.
//!
//! Its own `main`: the desk's open re-execs this binary as the session's
//! host (`host::main_if_host`), which the default harness cannot answer.

mod util;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use khor_node::Node;
use tokio::time::timeout;

const NAME: &str = "a_shell_opened_from_a_phone_is_on_the_phones_list_when_the_open_answers";

/// How the re-exec'd host learns which root it serves.
const ROOT_ENV: &str = "KHOR_TEST_OPEN_FROM_PHONE_ROOT";

fn main() {
    if let Ok(root) = std::env::var(ROOT_ENV) {
        khor_node::host::main_if_host(PathBuf::from(root));
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--list") {
        println!("{NAME}: test");
        return;
    }
    let filters: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
    if !filters.is_empty() && !filters.iter().any(|f| NAME.contains(f.as_str())) {
        println!("running 0 tests");
        return;
    }
    println!("running 1 test");
    if cfg!(not(unix)) {
        println!("test {NAME} ... skipped: the host is unix-only here");
        return;
    }
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap()
        .block_on(run());
    println!("test {NAME} ... ok");
}

struct Scratch(PathBuf);

struct Hosted<'a>(&'a Node, khor_core::SessionId);

impl Drop for Hosted<'_> {
    fn drop(&mut self) {
        let _ = self.0.close(&self.1);
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn root(tag: &str) -> Scratch {
    let p = std::env::temp_dir().join(format!("khor-openphone-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    util::hermetic(&p);
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

async fn run() {
    let desk_root = root("desk");
    let phone_root = root("phone");
    // SAFETY: before any thread of this test reads either; the host the
    // desk re-execs inherits both.
    unsafe {
        std::env::set_var(ROOT_ENV, &desk_root.0);
        std::env::set_var("SHELL", "/bin/sh");
    }
    let sd = Node::open_hosting(desk_root.0.clone(), "desk", true).unwrap();
    let serve_desk = tokio::spawn(async move { sd.serve().await });
    let sp = Node::open_hosting(phone_root.0.clone(), "phone", false).unwrap();
    let serve_phone = tokio::spawn(async move { sp.serve_resumable().await });
    wait_for_endpoint_file(&desk_root.0).await;
    wait_for_endpoint_file(&phone_root.0).await;

    let desk = Node::open_hosting(desk_root.0.clone(), "desk", true).unwrap();
    // The face's node: a fresh open per command, as the app's core does.
    let phone = || Node::open_hosting(phone_root.0.clone(), "phone", false).unwrap();
    timeout(
        Duration::from_secs(60),
        phone().pair(&desk.invite().unwrap()),
    )
    .await
    .expect("pairing must not hang")
    .unwrap();
    // A face on screen: the pump keeps its quick pace.
    phone()
        .face_is_here(true)
        .await
        .expect("the phone's resident hears its face");
    let out = timeout(Duration::from_secs(60), phone().sync_now())
        .await
        .expect("a round must come back")
        .unwrap();
    assert!(
        out.iter().any(|(name, r)| name == "desk" && r.is_ok()),
        "control: the phone's round reaches the desk: {out:?}"
    );

    let asked = Instant::now();
    let id = timeout(
        Duration::from_secs(35),
        phone().open_on(
            "desk",
            "shell",
            "t",
            "",
            &[],
            (69, 33),
            Some("3f0c8a52-6a51-4c55-9b0e-1d2a3b4c5d6e"),
        ),
    )
    .await
    .expect("the open must come back")
    .expect("the desk opens a shell for the phone");
    let _shell_cleanup = Hosted(&desk, id.clone());
    let answered = asked.elapsed();
    let mut listed = None;
    while asked.elapsed() < Duration::from_secs(60) {
        if phone()
            .sessions()
            .unwrap()
            .iter()
            .any(|v| v.session.id == id)
        {
            listed = Some(asked.elapsed());
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    println!("open answered in {answered:?}; on the phone's list at {listed:?}");
    assert!(
        desk.sessions().unwrap().iter().any(|v| v.session.id == id),
        "control: the shell is on the desk's own list"
    );
    let listed = listed.expect("the shell never reached the phone's list");
    assert!(
        listed < answered + Duration::from_secs(1),
        "the phone's list waited {:?} past the open's answer for the shell it opened",
        listed - answered
    );

    // A literal startup belongs to creation and is released by readiness.
    // Retrying keeps one host, one command execution and one shell state.
    let counter = desk_root.0.join("startup-count");
    let value = desk_root.0.join("startup-env");
    let start = khor_node::shell_start::ShellStart {
        token: "0c42a3e7-cb90-44a1-9ffd-a17e2e1b4a5c".into(),
        title: "startup".into(),
        cwd: desk_root.0.display().to_string(),
        command: format!(
            "printf x >> '{}'\nexport KHOR_TEST_LAUNCH=kept",
            counter.display()
        ),
        cols: 69,
        rows: 33,
    };
    let made = timeout(
        Duration::from_secs(35),
        phone().open_shell_on("desk", start.clone()),
    )
    .await
    .unwrap()
    .unwrap();
    let _startup_cleanup = Hosted(&desk, made.clone());
    let again = timeout(
        Duration::from_secs(35),
        phone().open_shell_on("desk", start),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(made, again, "retry must answer the same session");
    // A new session may take much longer to reach the phone than Codex's
    // 250 ms color probe allows. Creation must not run the command while
    // no terminal can answer its queries.
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(
        !counter.exists(),
        "startup ran before a terminal was ready to answer queries"
    );
    let live = khor_node::live::LiveKind::new(desk_root.0.clone(), khor_core::DeviceId([1; 32]));
    let dir = live
        .dir_of(&made)
        .expect("startup session has a registry row");
    let mut first = khor_node::host::connect(&dir, 69, 33).unwrap();
    use khor_node::host::{ClientOp, write_frame};
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(
        !counter.exists(),
        "a connected socket is not yet a ready terminal"
    );
    write_frame(
        &mut first,
        &ClientOp::Input(
            format!(
                "printf '%s' \"$KHOR_TEST_LAUNCH\" > '{}'\r",
                value.display()
            )
            .into_bytes(),
        ),
    )
    .unwrap();
    write_frame(&mut first, &ClientOp::Ready).unwrap();
    write_frame(&mut first, &ClientOp::Ready).unwrap();
    drop(first);
    let mut second = khor_node::host::connect(&dir, 69, 33).unwrap();
    write_frame(&mut second, &ClientOp::Ready).unwrap();
    drop(second);
    let until = Instant::now() + Duration::from_secs(8);
    while std::fs::read_to_string(&value).unwrap_or_default() != "kept" && Instant::now() < until {
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let count = std::fs::read_to_string(&counter).unwrap_or_default();
    let retained = std::fs::read_to_string(&value).unwrap_or_default();
    let _ = desk.close(&made);
    assert_eq!(
        count, "x",
        "creation retry and two attaches must not repeat startup"
    );
    assert_eq!(
        retained, "kept",
        "startup runs in the interactive shell that remains attached"
    );

    // A client from before Ready is refused, and cannot release startup.
    let rejected_count = desk_root.0.join("rejected-client-startup");
    let strict = khor_node::shell_start::ShellStart {
        token: "5a7f8a4c-7426-4899-9714-4a1cb71fd11a".into(),
        title: "current startup".into(),
        cwd: desk_root.0.display().to_string(),
        command: format!("printf x >> '{}'", rejected_count.display()),
        cols: 69,
        rows: 33,
    };
    let strict_id = phone().open_shell_on("desk", strict).await.unwrap();
    let _strict_cleanup = Hosted(&desk, strict_id.clone());
    let strict_dir = live.dir_of(&strict_id).unwrap();
    let file = khor_node::host::read_host_file(&strict_dir).unwrap();
    let mut rejected = std::net::TcpStream::connect(("127.0.0.1", file.port)).unwrap();
    rejected
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    write_frame(
        &mut rejected,
        &khor_node::host::Hello {
            cookie: file.cookie,
            cols: 69,
            rows: 33,
        },
    )
    .unwrap();
    let welcome: khor_node::host::Welcome = khor_node::host::read_frame(&mut rejected).unwrap();
    assert!(welcome.ok);
    assert_eq!(welcome.why, "khor-host/5");
    write_frame(&mut rejected, &ClientOp::Version(4)).unwrap();
    use std::io::Read;
    match rejected.read(&mut [0u8; 1]) {
        Ok(0) => {}
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionAborted
            ) => {}
        other => panic!("a refused version received output or stayed open: {other:?}"),
    }
    assert!(
        !rejected_count.exists(),
        "a refused client released startup"
    );
    let mut current = khor_node::host::connect(&strict_dir, 69, 33).unwrap();
    tokio::time::sleep(Duration::from_millis(350)).await;
    assert!(
        !rejected_count.exists(),
        "a current socket still needs renderer Ready"
    );
    write_frame(&mut current, &ClientOp::Ready).unwrap();
    write_frame(&mut current, &ClientOp::Ready).unwrap();
    let until = Instant::now() + Duration::from_secs(8);
    while !rejected_count.exists() && Instant::now() < until {
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert_eq!(std::fs::read_to_string(rejected_count).unwrap(), "x");

    let _ = desk.close(&id);
    serve_desk.abort();
    serve_phone.abort();
}
