use super::{Control, Pipes, Session};
use std::{
    fs,
    io::{self, BufRead, BufReader, Read, Write},
    path::PathBuf,
    process::{Command, ExitStatus},
    sync::{Arc, mpsc},
    thread::{self, JoinHandle},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const READY: &str = "KHOR_SESSION_READY ";
const DONE: &[u8] = b"{\"type\":\"done\"}\n";
const TAIL: &[u8] = b"KHOR_BUFFERED_TAIL_END\n";

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("khor-session-{}-{nonce}", std::process::id()));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn fixture_command(mode: &str, scratch: &Scratch) -> Command {
    let (_, module) = module_path!().split_once("::").unwrap();
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            &format!("{module}::session_fixture_child"),
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env_clear()
        .env("KHOR_SESSION_FIXTURE_MODE", mode)
        .env("KHOR_SESSION_FIXTURE_ROOT", &scratch.0);
    command
}

struct Fixture {
    session: Option<Session>,
    control: Arc<Control>,
    stdin: Option<Box<dyn Write + Send>>,
    output: mpsc::Receiver<io::Result<Vec<u8>>>,
    error: mpsc::Receiver<io::Result<Vec<u8>>>,
    workers: Vec<JoinHandle<()>>,
    leader: libc::pid_t,
    descendant: Option<libc::pid_t>,
    _scratch: Scratch,
}

impl Fixture {
    fn spawn(mode: &str) -> Self {
        let scratch = Scratch::new();
        let control = Arc::new(Control::default());
        let (
            session,
            Pipes {
                stdin,
                stdout,
                stderr,
            },
        ) = Session::spawn(fixture_command(mode, &scratch), control.clone()).unwrap();
        let (ready_tx, ready_rx) = mpsc::channel();
        let (output_tx, output) = mpsc::channel();
        let stdout_worker = thread::spawn(move || {
            let mut stdout = BufReader::new(stdout);
            let mut prefix = Vec::new();
            let ready = loop {
                let mut line = String::new();
                match stdout.read_line(&mut line) {
                    Ok(0) => {
                        break Err(format!(
                            "fixture ended without ready: {}",
                            String::from_utf8_lossy(&prefix)
                        ));
                    }
                    Err(error) => break Err(error.to_string()),
                    Ok(_) => {
                        prefix.extend_from_slice(line.as_bytes());
                        if let Some(at) = line.find(READY) {
                            let ids: Vec<libc::pid_t> = line[at + READY.len()..]
                                .split_whitespace()
                                .map(|pid| pid.parse().unwrap())
                                .collect();
                            break Ok((ids[0], (ids[1] != 0).then_some(ids[1])));
                        }
                    }
                }
            };
            let _ = ready_tx.send(ready);
            let result = stdout.read_to_end(&mut prefix).map(|_| prefix);
            let _ = output_tx.send(result);
        });
        let (error_tx, error) = mpsc::channel();
        let stderr_worker = thread::spawn(move || {
            let mut stderr = stderr;
            let mut bytes = Vec::new();
            let result = stderr.read_to_end(&mut bytes).map(|_| bytes);
            let _ = error_tx.send(result);
        });
        // Build the cleanup owner before an assertion can fail while waiting for ready.
        let mut fixture = Self {
            session: Some(session),
            control,
            stdin: Some(stdin),
            output,
            error,
            workers: vec![stdout_worker, stderr_worker],
            leader: 0,
            descendant: None,
            _scratch: scratch,
        };
        let (leader, descendant) = ready_rx
            .recv_timeout(Duration::from_secs(5))
            .unwrap()
            .unwrap();
        assert!(leader > 0 && descendant.is_none_or(|pid| pid > 0));
        fixture.leader = leader;
        fixture.descendant = descendant;
        fixture
    }

    fn blocked_writer(&mut self) -> mpsc::Receiver<io::Result<()>> {
        let mut stdin = self.stdin.take().unwrap();
        let (sent, received) = mpsc::channel();
        self.workers.push(thread::spawn(move || {
            // The abort is behind a real pipe-filling start, as with a stalled stdin writer.
            let result = stdin
                .write_all(&vec![b'x'; 8 * 1024 * 1024])
                .and_then(|_| stdin.write_all(b"\n{\"type\":\"abort\"}\n"));
            let _ = sent.send(result);
        }));
        assert!(
            matches!(
                received.recv_timeout(Duration::from_millis(150)),
                Err(mpsc::RecvTimeoutError::Timeout)
            ),
            "fixture did not block the real stdin write"
        );
        received
    }

    fn finish(&mut self, force: bool) -> io::Result<ExitStatus> {
        self.session.take().unwrap().finish(force)
    }

    fn output(&self, timeout: Duration) -> io::Result<Vec<u8>> {
        self.output
            .recv_timeout(timeout)
            .expect("stdout worker did not finish")
    }

    fn stderr(&self) -> io::Result<Vec<u8>> {
        self.error
            .recv_timeout(Duration::from_secs(2))
            .expect("stderr worker did not finish")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        drop(self.stdin.take());
        drop(self.session.take());
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

fn assert_reaped(pid: libc::pid_t) {
    assert_eq!(
        unsafe { libc::kill(pid, 0) },
        -1,
        "leader still exists after Session cleanup"
    );
    assert_eq!(io::Error::last_os_error().raw_os_error(), Some(libc::ESRCH));
}

fn assert_no_executing_descendant(pid: libc::pid_t) {
    let until = Instant::now() + Duration::from_secs(2);
    loop {
        if unsafe { libc::kill(pid, 0) } < 0 {
            assert_eq!(io::Error::last_os_error().raw_os_error(), Some(libc::ESRCH));
            return;
        }
        let status = Command::new("/bin/ps")
            .args(["-o", "stat=", "-p", &pid.to_string()])
            .output()
            .unwrap();
        let state = String::from_utf8(status.stdout).unwrap();
        if state.trim_start().starts_with('Z') {
            // This is an adopted child, not one this test can waitpid. Do not call it reaped.
            eprintln!(
                "descendant {pid} is terminated but still a zombie pending its adopter: {}",
                state.trim()
            );
            return;
        }
        assert!(
            Instant::now() < until,
            "descendant {pid} still executes: {state:?}"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

fn assert_buffered_done(output: &[u8]) {
    assert!(
        output.windows(DONE.len()).any(|part| part == DONE),
        "done was lost from stdout"
    );
    assert!(
        output.windows(TAIL.len()).any(|part| part == TAIL),
        "tail was lost from stdout"
    );
}

#[test]
fn natural_exit_preserves_done_and_buffered_tail_before_cleanup() {
    let mut fixture = Fixture::spawn("natural");
    drop(fixture.stdin.take());
    let output = fixture.output(Duration::from_secs(3)).unwrap();
    assert_buffered_done(&output);
    assert!(fixture.stderr().unwrap().is_empty());
    assert!(fixture.finish(false).unwrap().success());
    assert_reaped(fixture.leader);
}

#[test]
fn natural_exit_retires_the_pipe_owning_descendant_without_waiting_for_stop() {
    let started = Instant::now();
    let mut fixture = Fixture::spawn("natural-tree");
    drop(fixture.stdin.take());
    let output = fixture.output(Duration::from_secs(3)).unwrap();
    assert_buffered_done(&output);
    assert!(fixture.stderr().unwrap().is_empty());
    assert!(fixture.finish(false).unwrap().success());
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "natural exit waited for the Stop grace"
    );
    assert_reaped(fixture.leader);
    assert_no_executing_descendant(fixture.descendant.unwrap());
}

#[test]
fn blocked_stdin_does_not_block_stop_and_repeated_stops_keep_the_first_deadline() {
    let mut fixture = Fixture::spawn("blocked");
    let writer = fixture.blocked_writer();
    let first_stop = Instant::now();
    fixture.control.stop();
    assert!(
        first_stop.elapsed() < Duration::from_millis(250),
        "Stop waited for the blocked stdin"
    );
    for seconds in [3, 6, 9] {
        thread::sleep(
            (first_stop + Duration::from_secs(seconds)).saturating_duration_since(Instant::now()),
        );
        let before = Instant::now();
        fixture.control.stop();
        assert!(before.elapsed() < Duration::from_millis(250));
        assert!(
            matches!(writer.try_recv(), Err(mpsc::TryRecvError::Empty)),
            "writer ended before the ten-second grace"
        );
    }
    let remaining =
        (first_stop + Duration::from_millis(12_500)).saturating_duration_since(Instant::now());
    assert!(
        writer
            .recv_timeout(remaining)
            .expect("Stop did not cancel the blocked writer at its first deadline")
            .is_err()
    );
    let elapsed = first_stop.elapsed();
    assert!(
        elapsed >= Duration::from_millis(9_500) && elapsed < Duration::from_millis(12_500),
        "unexpected Stop grace: {elapsed:?}"
    );
    // No finish request preceded the I/O result: the independent supervisor enforced Stop.
    let _ = fixture.output(Duration::from_secs(1));
    let _ = fixture.stderr();
    let joining = Instant::now();
    assert!(!fixture.finish(false).unwrap().success());
    assert!(joining.elapsed() < Duration::from_secs(1));
    assert_reaped(fixture.leader);
}

#[test]
fn unwinding_the_session_retires_its_tree_and_joins_blocked_io() {
    let mut fixture = Fixture::spawn("drop-tree");
    let writer = fixture.blocked_writer();
    let session = fixture.session.take().unwrap();
    let started = Instant::now();
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        let _owned_session = session;
        panic!("fixture caller failed after spawn");
    }));
    assert!(panic.is_err());
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "Drop waited for the ten-second grace"
    );
    assert!(
        writer
            .recv_timeout(Duration::from_secs(1))
            .unwrap()
            .is_err()
    );
    let _ = fixture.output(Duration::from_secs(1));
    let _ = fixture.stderr();
    assert_reaped(fixture.leader);
    assert_no_executing_descendant(fixture.descendant.unwrap());
}

#[test]
#[ignore = "isolated real process fixture for Session supervision tests"]
fn session_fixture_child() {
    let mode = std::env::var("KHOR_SESSION_FIXTURE_MODE").unwrap();
    let scratch = Scratch(PathBuf::from(
        std::env::var_os("KHOR_SESSION_FIXTURE_ROOT").unwrap(),
    ));
    // Every fixture owns its own eventual exit. A failed or killed test parent cannot
    // leave the intentionally non-reading process or its descendant running forever.
    thread::spawn(|| {
        thread::sleep(Duration::from_secs(22));
        std::process::exit(124);
    });
    if mode == "descendant" {
        fs::write(
            scratch.0.join("descendant-ready"),
            std::process::id().to_string(),
        )
        .unwrap();
        loop {
            thread::sleep(Duration::from_millis(100));
        }
    }
    let descendant = if mode.ends_with("tree") {
        let child = fixture_command("descendant", &scratch).spawn().unwrap();
        let ready = scratch.0.join("descendant-ready");
        let until = Instant::now() + Duration::from_secs(3);
        while !ready.exists() {
            assert!(Instant::now() < until, "descendant did not start");
            thread::sleep(Duration::from_millis(10));
        }
        child.id()
    } else {
        0
    };
    println!("{READY}{} {descendant}", std::process::id());
    io::stdout().flush().unwrap();
    if mode.starts_with("natural") {
        let mut stdout = io::stdout().lock();
        stdout.write_all(DONE).unwrap();
        stdout.write_all(&vec![b't'; 256 * 1024]).unwrap();
        stdout.write_all(TAIL).unwrap();
        stdout.flush().unwrap();
        std::process::exit(0);
    }
    assert!(matches!(mode.as_str(), "blocked" | "drop-tree"));
    loop {
        thread::sleep(Duration::from_millis(100));
    }
}
