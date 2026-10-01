//! Attaching to a tmux session turns the mouse on for khor's client, and
//! only for khor's client.
//!
//! The owner, 2026-10-01: 「滚轮翻 tmux 历史这，自动打开吧」. Their tmux
//! has no config, so `mouse` is off, and in the alternate screen a
//! terminal's wheel is ↑ ↓ — a claude or a shell in the pane walks its
//! input history instead of tmux scrolling. `Tmux::grouped_attach_argv`
//! now ends with `set-option -q mouse on`, which lands on the grouped
//! session the attach creates for khor (its module head has why that
//! session and not the user's).
//!
//! The whole attach, for real: `LiveKind::attach_multiplexed` on a row a
//! real sweep found, a real host (this binary, re-exec'd as `_host` —
//! hence `harness = false` and the `main_if_host` call first), a real
//! tmux client in its PTY. Then the session options are read back, and a
//! wheel goes in through the host the way the app sends one.
//!
//! **Never the user's tmux**: a server of this test's own (`-L`, `-f
//! /dev/null`), its socket under a `TMUX_TMPDIR` of this test's own, and
//! `TMUX` cleared so nothing here can fall back to the server the test
//! happens to be run from. No tmux on the machine: skipped, and said so.
#![cfg_attr(not(unix), allow(dead_code, unused_imports))]

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use khor_core::DeviceId;
use khor_node::adaptor::{Discovery, Procs, tmux::Tmux};
use khor_node::live::LiveKind;

const NAME: &str = "attaching_a_tmux_session_turns_the_mouse_on_for_khor_s_client_only";

/// How the re-exec'd host learns which root it serves.
const ROOT_ENV: &str = "KHOR_TEST_TMUX_MOUSE_ROOT";

fn main() {
    if let Ok(root) = std::env::var(ROOT_ENV) {
        khor_node::host::main_if_host(PathBuf::from(root));
    }
    // A plain `main`, so it answers the two things cargo and its filters
    // ask of a test binary: `--list`, and a name to run or skip.
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
    if !have_tmux() {
        println!("test {NAME} ... skipped: no tmux on this machine");
        return;
    }
    run();
    println!("test {NAME} ... ok");
}

fn have_tmux() -> bool {
    Command::new("tmux")
        .arg("-V")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

struct Server {
    socket: String,
}

impl Server {
    fn tmux(&self, args: &[&str]) -> String {
        let out = Command::new("tmux")
            .arg("-L")
            .arg(&self.socket)
            .arg("-f")
            .arg("/dev/null")
            .args(args)
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .expect("tmux runs");
        String::from_utf8_lossy(&out.stdout).trim().to_owned()
    }

    /// The `mouse` a session has set on itself (empty: inherits).
    fn own_mouse(&self, session: &str) -> String {
        self.tmux(&["show-options", "-v", "-t", session, "mouse"])
    }

    /// The `mouse` a session actually runs with.
    fn mouse(&self, session: &str) -> String {
        self.tmux(&["show-options", "-Av", "-t", session, "mouse"])
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.tmux(&["kill-server"]);
    }
}

fn until(what: &str, ready: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while !ready() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[cfg(not(unix))]
fn run() {}

#[cfg(unix)]
fn run() {
    let scratch = std::env::temp_dir().join(format!("khtm-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&scratch);
    let root = scratch.join("root");
    let vendors = scratch.join("vendors");
    let sockets = scratch.join("s");
    for d in [&root, &vendors, &sockets] {
        std::fs::create_dir_all(d).unwrap();
    }
    // Single-threaded so far, and the only test in this binary: the one
    // moment the environment may be changed. Everything spawned below —
    // the sweep's tmux, the host, the host's tmux client — inherits it.
    unsafe {
        std::env::set_var("TMUX_TMPDIR", &sockets);
        std::env::remove_var("TMUX");
        std::env::remove_var("TMUX_PANE");
        std::env::remove_var("KHOR_TMUX");
        std::env::set_var(ROOT_ENV, &root);
    }
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        attach_and_check(&root, &vendors)
    }));
    let _ = std::fs::remove_dir_all(&scratch);
    if let Err(e) = result {
        std::panic::resume_unwind(e);
    }
}

#[cfg(unix)]
fn attach_and_check(root: &Path, vendors: &Path) {
    let s = Server {
        socket: format!("khor-mouse-{}", std::process::id()),
    };
    s.tmux(&["new-session", "-d", "-s", "work", "-x", "80", "-y", "24"]);
    s.tmux(&[
        "new-session",
        "-d",
        "-s",
        "bystander",
        "-x",
        "80",
        "-y",
        "24",
    ]);
    // Something to scroll back to.
    s.tmux(&["send-keys", "-t", "work", "seq 1 300", "Enter"]);
    until("the history to fill", || {
        s.tmux(&["display-message", "-p", "-t", "work", "#{history_size}"])
            .parse::<u32>()
            .is_ok_and(|n| n > 100)
    });
    // The control: nothing has the mouse on before khor attaches.
    assert_eq!(s.tmux(&["show-options", "-gv", "mouse"]), "off");
    for name in ["work", "bystander"] {
        assert_eq!(s.own_mouse(name), "", "{name} sets nothing of its own");
        assert_eq!(s.mouse(name), "off");
    }

    let tmux = Tmux::on_socket(&s.socket);
    assert!(tmux.knows_mouse());
    let id = tmux
        .sweep(&Procs::snapshot())
        .rows
        .iter()
        .find(|m| m.found.sighting.title == "work")
        .expect("the sweep finds the session")
        .found
        .id();
    let k = LiveKind::new(root.to_path_buf(), DeviceId([7; 32]))
        .discovering(Arc::new(Discovery::at(vendors).with_tmux(tmux)));
    k.attach_multiplexed(&id)
        .expect("the attach stands a host up");
    let dir = k.dir_of(&id).expect("registered by the attach");
    let host_pid = khor_node::host::read_host_file(&dir).unwrap().host_pid;

    let grouped = || {
        s.tmux(&[
            "list-sessions",
            "-F",
            "#{session_name}|#{session_group}|#{session_attached}",
        ])
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split('|').collect();
            (f.len() == 3 && f[0] != "work" && f[1] == "work" && f[2] != "0")
                .then(|| f[0].to_owned())
        })
        .next()
    };
    until("khor's client to attach", || grouped().is_some());
    let mine = grouped().unwrap();

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        assert_eq!(s.own_mouse(&mine), "on", "khor's own session has it on");
        assert_eq!(
            s.own_mouse("work"),
            "",
            "the session khor attached to is not written"
        );
        assert_eq!(s.mouse("work"), "off");
        assert_eq!(s.own_mouse("bystander"), "", "nor is any other session");
        assert_eq!(s.mouse("bystander"), "off");
        assert_eq!(
            s.tmux(&["show-options", "-gv", "mouse"]),
            "off",
            "and the global option is untouched"
        );

        // What xterm sees: tmux asks the terminal for SGR mouse reports,
        // and the wheel goes in as one — the bytes xterm sends for it.
        let mut conn = khor_node::host::connect(&dir, 80, 24).expect("the host takes an attacher");
        conn.set_read_timeout(Some(Duration::from_millis(200)))
            .unwrap();
        let mut modes = khor_node::modes::Modes::default();
        let mut seen = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(10);
        let asked_for_reports = |m: &khor_node::modes::Modes| {
            String::from_utf8_lossy(&m.preamble()).contains("\x1b[?1006h")
        };
        while !asked_for_reports(&modes) {
            assert!(
                Instant::now() < deadline,
                "tmux never asked khor's terminal for SGR mouse reports; stream: {:?}",
                String::from_utf8_lossy(&seen)
            );
            let mut buf = [0u8; 65536];
            if let Ok(n) = std::io::Read::read(&mut conn, &mut buf) {
                modes.feed(&buf[..n]);
                seen.extend_from_slice(&buf[..n]);
            }
        }
        assert_eq!(
            s.tmux(&["display-message", "-p", "-t", "work", "#{pane_in_mode}"]),
            "0"
        );
        for _ in 0..3 {
            khor_node::host::write_frame(
                &mut conn,
                &khor_node::host::ClientOp::Input(b"\x1b[<64;10;5M".to_vec()),
            )
            .unwrap();
        }
        until("the wheel to scroll tmux's history", || {
            let at = s.tmux(&[
                "display-message",
                "-p",
                "-t",
                "work",
                "#{pane_in_mode} #{scroll_position}",
            ]);
            at.split_once(' ')
                .is_some_and(|(mode, pos)| mode == "1" && pos.parse::<u32>().is_ok_and(|p| p > 0))
        });
    }));

    // Take the host down by its pid; its client's detach destroys the
    // grouped session (`destroy-unattached`), and `Server` takes the rest.
    unsafe {
        libc::kill(host_pid as i32, libc::SIGTERM);
    }
    let gone = Instant::now() + Duration::from_secs(5);
    while unsafe { libc::kill(host_pid as i32, 0) } == 0 && Instant::now() < gone {
        std::thread::sleep(Duration::from_millis(50));
    }
    if unsafe { libc::kill(host_pid as i32, 0) } == 0 {
        unsafe {
            libc::kill(host_pid as i32, libc::SIGKILL);
        }
    }
    if let Err(e) = outcome {
        std::panic::resume_unwind(e);
    }
}
