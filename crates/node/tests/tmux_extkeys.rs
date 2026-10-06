//! **Shift+Enter crosses tmux** (item 39). The owner, 2026-10-05: in one
//! tmux session, Shift+Enter is a newline in Claude Code under iTerm2 and
//! sent the line under khor. tmux hands a pane extended keys only when
//! its outer terminal has the `extkeys` feature; iTerm2 gets it from its
//! name (XTVERSION), xterm.js's name is not in tmux's table, so khor's
//! client never had it unless the person's `terminal-features` said so.
//! `Tmux::grouped_attach_argv` now gives khor's client `-T extkeys`.
//!
//! The whole attach, for real, as `tests/tmux_mouse.rs` does it: a server
//! of this test's own with `extended-keys on` and nothing else, a pane
//! program that asks for modifyOtherKeys (as Claude Code does) and writes
//! what it reads to a file, `LiveKind::attach_multiplexed`, a real host
//! and its tmux client. Then: tmux asks khor's terminal for
//! modifyOtherKeys (`CSI > 4 ; 2 m` in the stream), and the face's
//! Shift+Enter (`CSI 27 ; 2 ; 13 ~`, `lib/term-kitty.ts`) reaches the pane
//! program as those same bytes, where a plain Enter stays a CR.
//!
//! **Never the user's tmux**: own socket under its own `TMUX_TMPDIR`,
//! `-f /dev/null`, `TMUX` cleared. No tmux, or one older than 3.2 (no
//! `-T`): skipped, and said so.
#![cfg_attr(not(unix), allow(dead_code, unused_imports))]

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use khor_core::DeviceId;
use khor_node::adaptor::{Discovery, Procs, tmux::Tmux};
use khor_node::host::{ClientOp, write_frame};
use khor_node::live::LiveKind;

const NAME: &str = "shift_enter_reaches_a_pane_program_that_asked_for_extended_keys";

/// How the re-exec'd host learns which root it serves.
const ROOT_ENV: &str = "KHOR_TEST_TMUX_EXTKEYS_ROOT";

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
    let scratch = std::env::temp_dir().join(format!("khxk-{}", std::process::id()));
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
        socket: format!("khor-extkeys-{}", std::process::id()),
    };
    let log = root.join("keys.log");
    // The pane program: asks for modifyOtherKeys level 2, as Claude Code
    // does at its start, then writes every byte it reads.
    let program = format!(
        "printf '\\033[>4;2m'; stty raw -echo; exec cat > '{}'",
        log.display()
    );
    // `extended-keys` on before the pane exists, in the same command: a
    // tmux 3.7b with it off drops the pane's `CSI > 4 ; 2 m` as it parses
    // it, and the program asks once, at its start. Set after the session
    // (as this test first did), the request lost the race and the pane
    // stayed `VT10x`.
    s.tmux(&[
        "start-server",
        ";",
        "set-option",
        "-s",
        "extended-keys",
        "on",
        ";",
        "new-session",
        "-d",
        "-s",
        "work",
        "-x",
        "80",
        "-y",
        "24",
        "sh",
        "-c",
        &program,
    ]);
    let tmux = Tmux::on_socket(&s.socket);
    if !tmux.takes_client_features() {
        println!("test {NAME} ... skipped: this tmux is older than 3.2 (no -T)");
        return;
    }
    until("the pane to ask for extended keys", || {
        s.tmux(&["display-message", "-p", "-t", "work", "#{pane_key_mode}"]) == "Ext 2"
    });

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

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let clients = || s.tmux(&["list-clients", "-F", "#{client_termfeatures}"]);
        until("khor's client to attach", || !clients().is_empty());
        assert!(
            clients().split(',').any(|f| f == "extkeys"),
            "khor's client has the extkeys feature: {}",
            clients()
        );

        // What xterm sees: tmux asks khor's terminal for modifyOtherKeys.
        let mut conn = khor_node::host::connect(&dir, 80, 24).expect("the host takes an attacher");
        conn.set_read_timeout(Some(Duration::from_millis(200)))
            .unwrap();
        let mut modes = khor_node::modes::Modes::default();
        let mut seen = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(10);
        while !String::from_utf8_lossy(&modes.preamble()).contains("\x1b[>4;2m") {
            assert!(
                Instant::now() < deadline,
                "tmux never asked khor's terminal for modifyOtherKeys; stream: {:?}",
                String::from_utf8_lossy(&seen)
            );
            let mut buf = [0u8; 65536];
            if let Ok(n) = std::io::Read::read(&mut conn, &mut buf) {
                modes.feed(&buf[..n]);
                seen.extend_from_slice(&buf[..n]);
            }
        }

        // The face's Shift+Enter at level 2, then a plain Enter.
        write_frame(&mut conn, &ClientOp::Input(b"\x1b[27;2;13~".to_vec())).unwrap();
        write_frame(&mut conn, &ClientOp::Input(b"\r".to_vec())).unwrap();
        let read = || std::fs::read(&log).unwrap_or_default();
        until("the keys to reach the pane program", || {
            read().ends_with(b"\r")
        });
        assert_eq!(
            read(),
            b"\x1b[27;2;13~\r",
            "{:?}",
            String::from_utf8_lossy(&read())
        );
    }));

    // Take the host down by its pid; `Server` takes the rest.
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
