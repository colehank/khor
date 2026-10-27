//! **A tmux the person started in a khor shell is not handed out black**
//! (item 57; the owner: a shell with `tmux` typed in it opened black, and
//! a drag brought the picture back).
//!
//! The host's screen model (item 46) is vt100, and vt100's resize cuts a
//! row without clearing a wide character it splits at the new last
//! column; the next erase or write on that cell panics (`row.rs`
//! `clear_wide`). A tmux client redraws every row after a resize, so a
//! pane with CJK reaching the right edge and a face one column narrower
//! is enough. The model was replaced by a blank one on the normal screen
//! and nothing asked tmux to draw again, so every later face joining at
//! that size was handed a blank alternate screen — until a drag folded
//! the deck, which is a resize, which is tmux's redraw.
//!
//! A real host, a real pty, a private tmux server inside the shell: the
//! face narrows the pty by one column, leaves, and a face joins again at
//! that size and must see what tmux shows.
#![cfg(unix)]
use std::io::Read;
use std::os::{
    fd::AsRawFd,
    unix::fs::{FileTypeExt, MetadataExt},
};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use khor_core::{DeviceId, SessionId};
use khor_node::adaptor::{Discovery, Procs, tmux::Tmux};
use khor_node::host::{ClientOp, write_frame};
use khor_node::live::LiveKind;

fn tmux(sock: &str, args: &[&str]) -> String {
    let out = Command::new("tmux")
        .arg("-f")
        .arg("/dev/null")
        .arg("-L")
        .arg(sock)
        .args(args)
        .env_remove("TMUX")
        .env_remove("TMUX_PANE")
        .stdin(Stdio::null())
        .output()
        .expect("tmux runs");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SocketIdentity {
    dev: u64,
    ino: u64,
    uid: u32,
    nlink: u64,
}

/// Only this test's named server and the socket it actually reported.
/// Explicit finish reports cleanup errors; Drop preserves unknown state.
struct Server {
    name: String,
    path: PathBuf,
    identity: Option<SocketIdentity>,
    finished: bool,
}

impl Server {
    fn new(name: String) -> Result<Self, String> {
        let pid = std::process::id();
        if ![format!("khor-test-57-{pid}"), format!("khor-test-58-{pid}")].contains(&name) {
            return Err("not this fixture's server name".into());
        }
        // tmux -L uses TMUX_TMPDIR (otherwise /tmp), then tmux-UID.
        let base = std::env::var_os("TMUX_TMPDIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| "/tmp".into());
        let base = base.canonicalize().map_err(|e| e.to_string())?;
        // SAFETY: geteuid only reads this process's effective user id.
        let uid = unsafe { libc::geteuid() };
        let path = base.join(format!("tmux-{uid}")).join(&name);
        match std::fs::symlink_metadata(&path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            _ => return Err("private tmux socket path was not absent at setup".into()),
        }
        Ok(Self {
            name,
            path,
            identity: None,
            finished: false,
        })
    }

    fn socket_identity(&self) -> Result<Option<SocketIdentity>, String> {
        let m = match std::fs::symlink_metadata(&self.path) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.to_string()),
        };
        // SAFETY: geteuid has no arguments or memory side effects.
        let uid = unsafe { libc::geteuid() };
        let parent = std::fs::symlink_metadata(self.path.parent().ok_or("socket has no parent")?)
            .map_err(|e| e.to_string())?;
        if !m.file_type().is_socket()
            || m.uid() != uid
            || m.nlink() != 1
            || !parent.is_dir()
            || parent.uid() != uid
            || parent.mode() & 0o077 != 0
        {
            return Err("private socket or directory has unknown type/owner/links/mode".into());
        }
        Ok(Some(SocketIdentity {
            dev: m.dev(),
            ino: m.ino(),
            uid: m.uid(),
            nlink: m.nlink(),
        }))
    }

    /// -N forbids starting another server; -S addresses only the known path.
    fn command(&self, args: &[&str]) -> Result<(std::process::ExitStatus, String), String> {
        let mut child = Command::new("tmux")
            .args(["-f", "/dev/null", "-N", "-S"])
            .arg(&self.path)
            .args(args)
            .env_remove("TMUX")
            .env_remove("TMUX_PANE")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| e.to_string())?;
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
                let stdout = child.stdout.take().ok_or("tmux stdout missing")?;
                // A control client must not leave an inherited pipe holding
                // cleanup open after it exits. Unknown output state is an Err.
                // SAFETY: stdout owns this live descriptor; fcntl only changes
                // its status flags and no other thread accesses it.
                let flags = unsafe { libc::fcntl(stdout.as_raw_fd(), libc::F_GETFL) };
                if flags < 0
                    || unsafe {
                        libc::fcntl(stdout.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK)
                    } < 0
                {
                    return Err(std::io::Error::last_os_error().to_string());
                }
                let mut out = String::new();
                stdout
                    .take(4096)
                    .read_to_string(&mut out)
                    .map_err(|e| e.to_string())?;
                return Ok((status, out));
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let until = Instant::now() + Duration::from_secs(1);
                while Instant::now() < until {
                    if child.try_wait().map_err(|e| e.to_string())?.is_some() {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                return Err("private tmux control timed out; preserve socket".into());
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn started(&mut self) -> Result<(), String> {
        let identity = self
            .socket_identity()?
            .ok_or("private server socket missing")?;
        if self.identity.is_some() && self.identity != Some(identity) {
            return Err("private socket identity was already recorded differently".into());
        }
        let (status, path) = self.command(&["display-message", "-p", "#{socket_path}"])?;
        if !status.success()
            || PathBuf::from(path.trim())
                .canonicalize()
                .map_err(|e| e.to_string())?
                != self.path
            || self.socket_identity()? != Some(identity)
        {
            return Err("server did not report the unchanged private socket".into());
        }
        self.identity = Some(identity);
        Ok(())
    }

    fn finish(&mut self) -> Result<(), String> {
        if self.finished {
            return Ok(());
        }
        if self.identity.is_none() {
            if self.socket_identity()?.is_none() {
                self.finished = true;
                return Ok(());
            }
            self.started()?;
        }
        let identity = self.identity.ok_or("private socket identity missing")?;
        if self.socket_identity()? != Some(identity) {
            return Err("private socket changed before stop; preserve it".into());
        }
        // A server may have exited already. Only its recorded socket's state
        // decides whether removal is safe, never the kill command's status.
        let _ = self.command(&["kill-server"])?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| e.to_string())?;
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            match self.socket_identity()? {
                None => {
                    self.finished = true;
                    return Ok(());
                }
                Some(now) if now != identity => {
                    return Err("private socket changed after stop; preserve it".into());
                }
                Some(_) => {}
            }
            let probe = runtime.block_on(async {
                tokio::time::timeout(
                    Duration::from_millis(100),
                    tokio::net::UnixStream::connect(&self.path),
                )
                .await
            });
            match probe {
                Ok(Err(e)) if e.raw_os_error() == Some(libc::ECONNREFUSED) => {
                    if self.socket_identity()? != Some(identity) {
                        return Err("private socket changed before unlink; preserve it".into());
                    }
                    std::fs::remove_file(&self.path).map_err(|e| e.to_string())?;
                    self.finished = true;
                    return Ok(());
                }
                Ok(Err(e)) if e.kind() == std::io::ErrorKind::NotFound => {}
                Ok(Err(e)) => {
                    return Err(format!("unknown private socket state; preserve it: {e}"));
                }
                Ok(Ok(stream)) => drop(stream),
                Err(_) => {}
            }
            if Instant::now() >= deadline {
                return Err("private server is still live; preserve socket".into());
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        if let Err(error) = self.finish() {
            eprintln!("private tmux fixture {} cleanup failed: {error}", self.name);
        }
    }
}

fn read_for(conn: &mut std::net::TcpStream, d: Duration) -> Vec<u8> {
    conn.set_read_timeout(Some(Duration::from_millis(100)))
        .unwrap();
    let mut got = Vec::new();
    let mut buf = [0u8; 65536];
    let until = Instant::now() + d;
    while Instant::now() < until {
        match conn.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => got.extend_from_slice(&buf[..n]),
            Err(_) => {}
        }
    }
    got
}

#[test]
fn a_tmux_typed_in_a_shell_is_not_handed_out_black_after_a_narrowing() {
    if Command::new("tmux")
        .arg("-V")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_or(true, |s| !s.success())
    {
        eprintln!("skipped: no tmux on this machine");
        return;
    }
    let root = std::env::temp_dir().join(format!("khor-shelltmux-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let sock = format!("khor-test-57-{}", std::process::id());
    let mut server = Server::new(sock.clone()).expect("private socket starts absent");
    let id = SessionId("shell/57-0".into());
    let k = LiveKind::new(root.clone(), DeviceId([1; 32]));
    k.register(&id, "shell", "sh", None, None).unwrap();
    let host = {
        let root = root.clone();
        let id = id.clone();
        std::thread::spawn(move || {
            khor_node::host::host_main(root, id, (80, 24), vec!["/bin/sh".into()])
        })
    };
    let dir = k.dir_of(&id).expect("registered");
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut face = loop {
        if let Ok(a) = khor_node::host::connect(&dir, 80, 24) {
            break a;
        }
        assert!(Instant::now() < deadline, "the host never took an attacher");
        std::thread::sleep(Duration::from_millis(50));
    };
    let _ = read_for(&mut face, Duration::from_millis(300));
    // The person types tmux at the shell's prompt.
    let typed = format!("tmux -L {sock} -f /dev/null new -s one /bin/sh\r");
    write_frame(&mut face, &ClientOp::Input(typed.into_bytes())).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while tmux(&sock, &["list-clients"]).is_empty() {
        assert!(Instant::now() < deadline, "tmux never started in the shell");
        let _ = read_for(&mut face, Duration::from_millis(100));
    }
    server.started().expect("the server reports its own socket");
    // CJK to the pane's right edge: 40 wide characters fill 80 columns.
    let wide = "\u{4e2d}".repeat(40);
    let line =
        format!("clear; for i in 1 2 3 4 5 6; do printf '%s\\n' '{wide}'; done; echo MARK-57");
    tmux(&sock, &["send-keys", "-t", "one", "-l", &line]);
    tmux(&sock, &["send-keys", "-t", "one", "Enter"]);
    let deadline = Instant::now() + Duration::from_secs(10);
    while !tmux(&sock, &["capture-pane", "-p", "-t", "one"])
        .lines()
        .any(|l| l == "MARK-57")
    {
        assert!(Instant::now() < deadline, "the pane never showed the mark");
        let _ = read_for(&mut face, Duration::from_millis(100));
    }
    let _ = read_for(&mut face, Duration::from_millis(500));
    // The face is one column narrower (another device, a fold): tmux
    // redraws every row at 79 over cells the resize split.
    write_frame(&mut face, &ClientOp::Resize { cols: 79, rows: 24 }).unwrap();
    let _ = read_for(&mut face, Duration::from_millis(2000));
    drop(face);
    std::thread::sleep(Duration::from_millis(300));

    // A face joins again at the size the pty has: no SIGWINCH, so what it
    // is handed is the host's model and nothing else.
    let mut again = khor_node::host::connect(&dir, 79, 24).expect("the face comes back");
    let got = read_for(&mut again, Duration::from_millis(1500));
    let shown = tmux(&sock, &["capture-pane", "-p", "-t", "one"]);
    server.finish().expect("private server and socket are gone");
    // The shell ends, and the host with it.
    let _ = read_for(&mut again, Duration::from_millis(300));
    write_frame(&mut again, &ClientOp::Input(b"exit\r".to_vec())).unwrap();
    let _ = read_for(&mut again, Duration::from_millis(300));
    drop(again);
    let _ = host.join();
    let _ = std::fs::remove_dir_all(&root);

    let mut face = vt100::Parser::new(24, 79, 0);
    face.process(&got);
    let screen = face.screen();
    let rows: Vec<String> = screen
        .rows(0, 79)
        .map(|r| r.trim_end().to_owned())
        .collect();
    eprintln!(
        "the face was handed {} bytes:\n{}",
        got.len(),
        rows.join("\n")
    );
    assert!(
        screen.alternate_screen(),
        "the face is on tmux's alternate screen"
    );
    let want: Vec<&str> = shown.lines().map(str::trim_end).collect();
    assert!(
        want.iter().any(|l| l.contains('\u{4e2d}')),
        "control: tmux shows the wide rows: {shown}"
    );
    for (i, w) in want.iter().enumerate() {
        assert_eq!(
            rows[i], *w,
            "row {i}: the face shows what tmux shows, not a blank"
        );
    }
}

/// **A shell's own tmux client is found by its terminal** (item 58): the
/// person typed `tmux` in a khor shell, and the row reads that session's
/// history, takes the wheel into its copy mode and back out, and writes
/// nothing of the person's. Once the client leaves, the row is a plain
/// shell again.
#[test]
fn a_shells_own_tmux_client_is_found_by_its_terminal() {
    if Command::new("tmux")
        .arg("-V")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_or(true, |s| !s.success())
    {
        eprintln!("skipped: no tmux on this machine");
        return;
    }
    let root = std::env::temp_dir().join(format!("khor-shelltmux58-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let sock = format!("khor-test-58-{}", std::process::id());
    let mut server = Server::new(sock.clone()).expect("private socket starts absent");
    let id = SessionId("shell/58-0".into());
    let k = LiveKind::new(root.clone(), DeviceId([1; 32]));
    k.register(&id, "shell", "sh", None, None).unwrap();
    let host = {
        let root = root.clone();
        let id = id.clone();
        std::thread::spawn(move || {
            khor_node::host::host_main(root, id, (80, 24), vec!["/bin/sh".into()])
        })
    };
    let dir = k.dir_of(&id).expect("registered");
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut face = loop {
        if let Ok(a) = khor_node::host::connect(&dir, 80, 24) {
            break a;
        }
        assert!(Instant::now() < deadline, "the host never took an attacher");
        std::thread::sleep(Duration::from_millis(50));
    };
    let _ = read_for(&mut face, Duration::from_millis(300));
    assert_eq!(
        k.tmux_in_shell(&id),
        None,
        "a shell at its prompt has no tmux client in front"
    );

    let typed = format!("tmux -L {sock} -f /dev/null new -s mine /bin/sh\r");
    write_frame(&mut face, &ClientOp::Input(typed.into_bytes())).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while tmux(&sock, &["list-clients"]).is_empty() {
        assert!(Instant::now() < deadline, "tmux never started in the shell");
        let _ = read_for(&mut face, Duration::from_millis(100));
    }
    server.started().expect("the server reports its own socket");
    let front = k.tmux_in_shell(&id).expect("the client in front is found");
    assert_eq!(
        front.socket,
        vec!["-L".to_owned(), sock.clone()],
        "its server, from its argv"
    );
    assert_eq!(front.session_name, "mine");
    assert!(
        front.tty.starts_with("/dev/"),
        "the terminal as tmux names it: {}",
        front.tty
    );
    assert_eq!(
        tmux(&sock, &["list-clients", "-F", "#{client_tty}"]).trim(),
        front.tty,
        "the client on the shell's terminal"
    );
    assert_eq!(
        tmux(
            &sock,
            &["display-message", "-p", "-t", "mine", "#{pane_id}"]
        )
        .trim(),
        front.pane_id,
        "and the pane in front of it (SESS-BRIEF 2c)"
    );

    // The row side (SESS-BRIEF 1b): the discovery watching that server
    // names the session's row as its own sweep does; one watching
    // another server names none.
    let shell = khor_node::host::read_host_file(&dir).unwrap().child_pid;
    let watched = Tmux::on_socket(&sock);
    let swept = watched
        .sweep(&Procs::snapshot())
        .rows
        .iter()
        .find(|m| m.found.sighting.title == "mine")
        .expect("the sweep lists the session")
        .found
        .id();
    let vendors = root.join("vendors");
    std::fs::create_dir_all(&vendors).unwrap();
    assert_eq!(
        Discovery::at(&vendors).with_tmux(watched).attached(shell),
        Some(swept),
        "the session's row, as the sweep names it"
    );
    assert_eq!(
        Discovery::at(&vendors)
            .with_tmux(Tmux::on_socket(&format!("{sock}-other")))
            .attached(shell),
        None,
        "a session id means nothing on another server"
    );
    assert_eq!(
        Discovery::at(&vendors).attached(shell),
        None,
        "no server watched, nothing asked"
    );

    tmux(
        &sock,
        &[
            "send-keys",
            "-t",
            "mine",
            "-l",
            "i=1; while [ $i -le 200 ]; do echo row-$i; i=$((i+1)); done",
        ],
    );
    tmux(&sock, &["send-keys", "-t", "mine", "Enter"]);
    let deadline = Instant::now() + Duration::from_secs(10);
    while !tmux(&sock, &["capture-pane", "-p", "-t", "mine"]).contains("row-200") {
        assert!(Instant::now() < deadline, "the pane never printed its rows");
        let _ = read_for(&mut face, Duration::from_millis(100));
    }
    let h = k
        .tmux_history(&id, None)
        .expect("the session's history is read through the shell's row");
    assert!(
        h.lines.iter().any(|l| l == "row-1"),
        "the first row is in the history: {:?}",
        &h.lines[..h.lines.len().min(5)]
    );
    assert!(
        !h.alternate && !h.mouse,
        "a shell in the pane: the drag scrolls the history"
    );

    let place = || {
        tmux(
            &sock,
            &[
                "display-message",
                "-p",
                "-t",
                "mine",
                "#{pane_in_mode} #{scroll_position}",
            ],
        )
        .trim()
        .to_owned()
    };
    assert_eq!(k.tmux_wheel(&id, 0), Ok(true), "asked: tmux is in front");
    assert_eq!(place(), "0", "only asked: nothing moved");
    assert_eq!(k.tmux_wheel(&id, 5), Ok(true));
    assert_eq!(place(), "1 5", "the wheel up: copy mode, five rows up");
    assert_eq!(k.tmux_wheel(&id, -5), Ok(true));
    assert_eq!(
        place(),
        "0",
        "the wheel back down to the bottom leaves copy mode"
    );
    assert_eq!(
        tmux(&sock, &["show-options", "-g", "mouse"]).trim(),
        "mouse off",
        "nothing global written"
    );
    assert_eq!(
        tmux(&sock, &["show-options", "-t", "mine"]).trim(),
        "",
        "no option on the person's session"
    );

    // The person detaches: a plain shell again.
    tmux(&sock, &["detach-client", "-s", "mine"]);
    let deadline = Instant::now() + Duration::from_secs(10);
    while k.tmux_in_shell(&id).is_some() {
        assert!(Instant::now() < deadline, "the client never left");
        let _ = read_for(&mut face, Duration::from_millis(100));
    }
    assert!(
        k.tmux_history(&id, None).is_err(),
        "no history once the client left"
    );
    assert_eq!(
        Discovery::at(&vendors)
            .with_tmux(Tmux::on_socket(&sock))
            .attached(shell),
        None,
        "and no session once it left"
    );
    assert_eq!(
        k.tmux_wheel(&id, 3),
        Ok(false),
        "the wheel is the face's again"
    );

    server.finish().expect("private server and socket are gone");
    write_frame(&mut face, &ClientOp::Input(b"exit\r".to_vec())).unwrap();
    let _ = read_for(&mut face, Duration::from_millis(300));
    drop(face);
    let _ = host.join();
    let _ = std::fs::remove_dir_all(&root);
}
