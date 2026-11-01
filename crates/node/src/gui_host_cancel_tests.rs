use super::*;
#[path = "gui_host_startup_tests.rs"]
mod startup_tests;
use std::{
    io::Read,
    process::{Child, Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
struct Worker(Child);
impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn a_ghost_waiter_reaps_even_when_the_pid_receiver_leaves() {
    let scratch = Scratch(std::env::temp_dir().join(format!(
        "khor-ghost-wait-{}",
        crate::link::fresh_hex().unwrap()
    )));
    std::fs::create_dir(&scratch.0).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut normal = Command::new("/bin/sh");
    normal
        .args(["-c", "exit 7"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let pid = match spawn_ghost_with_waiter(normal, deadline) {
        Ok(pid) => pid,
        Err(_) => panic!("the actual ghost waiter did not report its child"),
    };
    while crate::link::pid_alive(pid) {
        assert!(
            Instant::now() < deadline,
            "the waiter left its child unreaped"
        );
        std::thread::sleep(Duration::from_millis(5));
    }

    // Abandoning the PID response is the caller-timeout/drop path. The
    // production lifetime body still owns and waits the same actual child.
    let pid_file = scratch.0.join("abandoned-pid");
    let mut abandoned = Command::new("/bin/sh");
    abandoned
        .args([
            "-c",
            "printf '%s' \"$$\" > \"$1\"; exit 7",
            "ghost-wait-test",
        ])
        .arg(&pid_file)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let (spawned, received) = mpsc::sync_channel(1);
    drop(received);
    let (done, finished) = mpsc::sync_channel(1);
    let waiter = std::thread::spawn(move || {
        ghost_child_lifetime(abandoned, spawned);
        let _ = done.send(());
    });
    finished
        .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        .expect("abandoned caller prevented the exact child wait");
    waiter.join().unwrap();
    let pid: u32 = std::fs::read_to_string(pid_file).unwrap().parse().unwrap();
    // pid_alive only probes: using pid_running here would itself waitpid and
    // hide the very missing reaper this regression is intended to catch.
    assert!(
        !crate::link::pid_alive(pid),
        "abandoned response left a zombie"
    );

    match spawn_ghost_with_waiter(Command::new(scratch.0.join("missing-executable")), deadline) {
        Err(GhostSpawnError::BeforeSpawn(why)) => {
            assert_eq!(why.kind(), std::io::ErrorKind::NotFound)
        }
        _ => panic!("a definite command-spawn failure lost its classification"),
    }
}

#[test]
fn an_uncooperative_acp_has_a_bounded_stop_and_can_be_reopened() {
    let scratch = Scratch(std::env::temp_dir().join(format!(
        "khor-acp-stop-{}",
        crate::link::fresh_hex().unwrap()
    )));
    std::fs::create_dir(&scratch.0).unwrap();
    let mut worker = Worker(
        Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "gui_host::cancel_tests::ordinary_acp_stop_worker",
                "--ignored",
            ])
            .env("KHOR_ACP_STOP_SCRATCH", &scratch.0)
            .env("KHOR_ACP_AMBIENT", "synthetic-inherited-environment")
            .current_dir(&scratch.0)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(30);
    let status = loop {
        if let Some(status) = worker.0.try_wait().unwrap() {
            break status;
        }
        assert!(
            Instant::now() < deadline,
            "ordinary ACP Stop worker exceeded its outer deadline"
        );
        std::thread::sleep(Duration::from_millis(10));
    };
    let mut stdout = String::new();
    let mut stderr = String::new();
    worker
        .0
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut stdout)
        .unwrap();
    worker
        .0
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .unwrap();
    if let Some(evidence) = std::env::var_os("KHOR_ACP_TEST_EVIDENCE") {
        let evidence = PathBuf::from(evidence);
        for mode in ["ignore-cancel", "normal"] {
            let destination = evidence.join(mode);
            std::fs::create_dir_all(&destination).unwrap();
            for name in ["pid.json", "prompt.jsonl", "cancel.jsonl", "received.jsonl"] {
                let source = scratch.0.join(mode).join(name);
                if source.exists() {
                    std::fs::copy(source, destination.join(name)).unwrap();
                }
            }
        }
    }
    assert!(status.success(), "{stdout}\n{stderr}");
}

fn await_file(path: &std::path::Path) -> String {
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        if let Ok(text) = std::fs::read_to_string(path) {
            if !text.is_empty() {
                return text;
            }
        }
        assert!(
            Instant::now() < until,
            "fixture did not publish {}",
            path.display()
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn open_host(
    root: &std::path::Path,
    scratch: &std::path::Path,
    mode: &str,
) -> (
    std::thread::JoinHandle<Result<(), String>>,
    TcpStream,
    mpsc::Receiver<GuiNote>,
) {
    open_host_with_timeouts(root, scratch, mode, HostTimeouts::default())
}

fn open_host_with_timeouts(
    root: &std::path::Path,
    scratch: &std::path::Path,
    mode: &str,
    timeouts: HostTimeouts,
) -> (
    std::thread::JoinHandle<Result<(), String>>,
    TcpStream,
    mpsc::Receiver<GuiNote>,
) {
    let host = launch_host(root, scratch, mode, timeouts);
    let (conn, notes) = attach_host(root, scratch, mode);
    (host, conn, notes)
}
fn fixture_command(scratch: &std::path::Path, mode: &str) -> String {
    let python = std::env::var("KHOR_ACP_TEST_PYTHON").unwrap_or_else(|_| "python3".into());
    serde_json::json!({"command":python,"args":[scratch.join("agent.py"),mode,scratch.join(mode)]})
        .to_string()
}

fn launch_host(
    root: &std::path::Path,
    scratch: &std::path::Path,
    mode: &str,
    timeouts: HostTimeouts,
) -> std::thread::JoinHandle<Result<(), String>> {
    let ready = scratch.join(format!("{mode}.ready"));
    let command = fixture_command(scratch, mode);
    if let Ok(native_id) = std::env::var(RESUME_ENV) {
        // The isolated owned worker uses the same production preghost owner,
        // actual context/cwd and claim validation as the normal opener. No
        // synthetic token, fake provider receipt or admission bypass.
        let key = khor_net::identity::load_or_create(&root.join(".khor/identity.key")).unwrap();
        let live = LiveKind::new(
            root.to_owned(),
            khor_core::DeviceId(*key.public().as_bytes()),
        );
        let cmd = vec![command.clone()];
        let native = gui_source_identity(
            &live,
            khor_core::DeviceId(*key.public().as_bytes()),
            &cmd,
            &native_id,
        )
        .unwrap();
        let context = gui_environment(root, Some(&native_id)).unwrap();
        let cwd = crate::cwd::standing().unwrap_or_else(std::env::temp_dir);
        let launch =
            serde_json::to_vec(&(&cwd, "ACP cancellation fixture", &cmd, &context)).unwrap();
        let claim = live.prepare_gui_recovery(native, &launch).unwrap();
        unsafe {
            std::env::set_var(RECOVERY_ENV, claim);
        }
    } else {
        unsafe {
            std::env::remove_var(RECOVERY_ENV);
        }
    }
    let host = {
        let root = root.to_owned();
        let ready = ready.clone();
        std::thread::spawn(move || {
            gui_host_main_with_timeouts(
                root,
                ready,
                "ACP cancellation fixture".into(),
                vec![command],
                timeouts,
            )
        })
    };
    host
}
fn attach_host(
    root: &std::path::Path,
    scratch: &std::path::Path,
    mode: &str,
) -> (TcpStream, mpsc::Receiver<GuiNote>) {
    let ready = scratch.join(format!("{mode}.ready"));
    let until = Instant::now() + Duration::from_secs(5);
    let key = khor_net::identity::load_or_create(&root.join(".khor/identity.key")).unwrap();
    let live = LiveKind::new(
        root.to_owned(),
        khor_core::DeviceId(*key.public().as_bytes()),
    );
    let native = std::env::var(RESUME_ENV).ok();
    let vendor = std::env::var(VENDOR_ENV).ok();
    let agent = if native.is_none() {
        Some(gui_new_agent(
            &[fixture_command(scratch, mode)],
            vendor.as_deref(),
        ))
    } else {
        vendor
    };
    let claim = native
        .as_ref()
        .and_then(|_| std::env::var(RECOVERY_ENV).ok());
    let id = loop {
        if let Ok(id) = std::fs::read_to_string(&ready) {
            let id = SessionId(id.trim().to_owned());
            if !id.0.is_empty()
                && live
                    .confirm_gui_ready(
                        &id,
                        std::process::id(),
                        native.as_deref(),
                        agent.as_deref(),
                        claim.as_deref(),
                    )
                    .is_ok()
            {
                break id;
            }
        }
        assert!(
            Instant::now() < until,
            "fixture did not durably publish Ready within the original5s bound"
        );
        std::thread::sleep(Duration::from_millis(5));
    };
    let file = crate::host::read_host_file(&live.dir_of(&id).unwrap()).unwrap();
    let mut conn = TcpStream::connect(("127.0.0.1", file.port)).unwrap();
    conn.set_write_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    write_frame(
        &mut conn,
        &crate::host::Hello {
            cookie: file.cookie,
            cols: 0,
            rows: 0,
        },
    )
    .unwrap();
    let welcome: crate::host::Welcome = read_frame(&mut conn).unwrap();
    assert!(welcome.ok);
    let mut reader = conn.try_clone().unwrap();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        while let Ok(note) = read_frame::<GuiNote>(&mut reader) {
            let end = matches!(note, GuiNote::Gone);
            if tx.send(note).is_err() || end {
                break;
            }
        }
    });
    (conn, rx)
}

#[test]
#[ignore = "isolated real GUI host/ACP cancellation worker"]
fn ordinary_acp_stop_worker() {
    let scratch = PathBuf::from(std::env::var_os("KHOR_ACP_STOP_SCRATCH").unwrap());
    std::fs::write(
        scratch.join("agent.py"),
        include_str!("fixtures/acp-cancel.py"),
    )
    .unwrap();
    let root = scratch.join("store");
    std::fs::create_dir(&root).unwrap();
    let (host, mut conn, notes) = open_host(&root, &scratch, "ignore-cancel");
    let pid: serde_json::Value =
        serde_json::from_str(&await_file(&scratch.join("ignore-cancel/pid.json"))).unwrap();
    assert_eq!(
        pid["ambient"], "synthetic-inherited-environment",
        "ACP must preserve its inherited environment"
    );
    assert_eq!(
        std::fs::canonicalize(pid["cwd"].as_str().unwrap()).unwrap(),
        std::fs::canonicalize(&scratch).unwrap(),
        "ACP must preserve the standing cwd"
    );
    write_frame(&mut conn, &GuiOp::Say("synthetic first prompt".into())).unwrap();
    await_file(&scratch.join("ignore-cancel/prompt.jsonl"));
    let stopped = Instant::now();
    write_frame(&mut conn, &GuiOp::Stop).unwrap();
    await_file(&scratch.join("ignore-cancel/cancel.jsonl"));
    let until = stopped + Duration::from_secs(15);
    loop {
        let note = notes
            .recv_timeout(until.saturating_duration_since(Instant::now()))
            .expect(
                "ordinary ACP Stop did not retire its uncooperative process within fifteen seconds",
            );
        if matches!(note, GuiNote::Gone) {
            let child = pid["pid"].as_i64().unwrap() as libc::pid_t;
            assert_eq!(
                unsafe { libc::kill(child, 0) },
                -1,
                "Gone was published before the ACP child was reaped"
            );
            assert_eq!(
                std::io::Error::last_os_error().raw_os_error(),
                Some(libc::ESRCH)
            );
            break;
        }
    }
    drop(conn);
    host.join().unwrap().unwrap();
    // A forced close requires explicit reopen, preserving the same ACP context.
    unsafe { std::env::set_var(RESUME_ENV, "synthetic-q037-session") };
    let (host, mut conn, notes) = open_host(&root, &scratch, "normal");
    write_frame(&mut conn, &GuiOp::Say("synthetic next prompt".into())).unwrap();
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        if let GuiNote::Turn(stop) = notes
            .recv_timeout(until.saturating_duration_since(Instant::now()))
            .expect("reopened ACP did not complete the next turn")
        {
            assert_eq!(stop, "EndTurn");
            break;
        }
    }
    write_frame(&mut conn, &GuiOp::Close).unwrap();
    host.join().unwrap().unwrap();
}

fn run_host_case(case: &str) {
    let scratch = Scratch(std::env::temp_dir().join(format!(
        "khor-acp-{case}-{}",
        crate::link::fresh_hex().unwrap()
    )));
    std::fs::create_dir(&scratch.0).unwrap();
    let mut worker = Worker(
        Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "gui_host::cancel_tests::managed_host_case_worker",
                "--ignored",
                "--nocapture",
            ])
            .env("KHOR_ACP_STOP_SCRATCH", &scratch.0)
            .env("KHOR_ACP_HOST_CASE", case)
            .env("KHOR_ACP_AMBIENT", "synthetic-inherited-environment")
            .current_dir(&scratch.0)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(45);
    let status = loop {
        if let Some(status) = worker.0.try_wait().unwrap() {
            break status;
        }
        assert!(
            Instant::now() < deadline,
            "ACP {case} worker exceeded its outer deadline"
        );
        std::thread::sleep(Duration::from_millis(10));
    };
    let mut stdout = String::new();
    let mut stderr = String::new();
    worker
        .0
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut stdout)
        .unwrap();
    worker
        .0
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .unwrap();
    if let Some(evidence) = std::env::var_os("KHOR_ACP_TEST_EVIDENCE") {
        let evidence = PathBuf::from(evidence).join(case);
        std::fs::create_dir_all(&evidence).unwrap();
        for entry in std::fs::read_dir(&scratch.0).unwrap().flatten() {
            if entry.path().is_dir() && entry.file_name() != "store" {
                let destination = evidence.join(entry.file_name());
                std::fs::create_dir_all(&destination).unwrap();
                for file in std::fs::read_dir(entry.path()).unwrap().flatten() {
                    if file.path().is_file() {
                        std::fs::copy(file.path(), destination.join(file.file_name())).unwrap();
                    }
                }
            }
        }
        std::fs::write(evidence.join("worker.stdout"), &stdout).unwrap();
        std::fs::write(evidence.join("worker.stderr"), &stderr).unwrap();
    }
    assert!(status.success(), "{case}: {stdout}\n{stderr}");
    assert_eq!(
        stdout
            .lines()
            .filter(|line| line.starts_with("test result:"))
            .count(),
        1,
        "{case}: owned helper must produce exactly one real test summary: {stdout}"
    );
    assert!(
        stdout.contains("test result: ok. 1 passed; 0 failed; 0 ignored;"),
        "{case}: owned helper did not actually run its case: {stdout}"
    );
}

#[test]
fn cooperative_cancel_keeps_its_final_note_order_and_the_same_process() {
    run_host_case("cooperative");
}
#[test]
fn pending_control_keeps_capture_readable_without_reloading_the_process() {
    run_host_case("control-capture");
}
#[test]
fn continuous_notes_and_ops_do_not_starve_stop_or_its_completion() {
    run_host_case("flood");
}
#[test]
fn an_unread_gui_does_not_block_another_gui_stop_or_confirmed_gone() {
    run_host_case("slow-gui");
}
#[test]
fn timed_out_control_is_unknown_and_retires_the_process() {
    run_host_case("control-timeout");
}
#[test]
fn host_handshake_refusal_and_timeout_return_only_after_reap() {
    run_host_case("handshake");
}

#[test]
fn hosted_acp_journal_captures_inputs_without_reappending_captured_history() {
    run_host_case("journal");
}
#[test]
fn no_load_capture_fences_bound_identical_chunks_for_independent_views() {
    run_host_case("capture-no-load");
}
#[test]
fn first_resume_capture_records_its_actual_partial_origin() {
    run_host_case("journal-resume");
}
#[test]
fn failed_or_closed_bootstrap_does_not_publish_ready_or_keep_a_writer() {
    run_host_case("journal-abort");
}

#[test]
fn an_unpublished_opening_cannot_report_or_exit_the_previous_gui_execution() {
    let scratch = Scratch(std::env::temp_dir().join(format!(
        "khor-gui-opening-publication-{}",
        crate::link::fresh_hex().unwrap()
    )));
    let home = khor_core::DeviceId([7; 32]);
    let live = LiveKind::new(scratch.0.clone(), home);
    let identity = JournalIdentity {
        home,
        agent: "fixture".into(),
        native_id: "opening-run".into(),
    };
    let id = crate::adaptor::id_for(&identity.native_id);
    let old = live
        .begin_gui_run(&id, "old", std::process::id(), &identity, None, false)
        .unwrap();
    live.report_run(&old, State::Done, Source::Reported)
        .unwrap();
    assert!(
        live.check_gui_source(&identity).is_err(),
        "a Done word alone does not retire an executing host"
    );
    live.record_exit_run(&old, 0).unwrap();
    live.check_gui_source(&identity).unwrap();
    let dir = live.dir_of(&id).unwrap();
    let pair = || ["meta.json", "state.json"].map(|name| std::fs::read(dir.join(name)).unwrap());
    let before = pair();
    let mut failed = RunPublication::Opening(State::Idle);
    failed
        .report(&live, State::Blocked, Source::Reported)
        .unwrap();
    failed.record_exit(&live, 1).unwrap();
    assert_eq!(pair(), before);

    let mut opening = RunPublication::Opening(State::Idle);
    opening
        .report(&live, State::Blocked, Source::Reported)
        .unwrap();
    let key = live
        .begin_gui_run(&id, "new", std::process::id(), &identity, None, true)
        .unwrap();
    opening.publish(&live, key).unwrap();
    assert!(matches!(&opening, RunPublication::Live(_)));
    assert!(
        live.report_run(&old, State::Busy, Source::Reported)
            .is_err()
    );
    let meta: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.join("meta.json")).unwrap()).unwrap();
    assert_eq!(
        meta["gui_run"]["state"]["word"],
        serde_json::to_value(State::Blocked).unwrap()
    );
    opening.record_exit(&live, 0).unwrap();
}

#[test]
fn an_initial_load_of_many_small_records_is_complete_in_canonical_history() {
    run_host_case("many-load");
}

#[test]
fn stopping_a_flooding_turn_preserves_capture_and_the_next_turn() {
    run_host_case("stop-capture");
}

#[test]
fn closing_a_flooding_turn_reaps_with_an_unread_view() {
    run_host_case("close-capture");
}

#[test]
fn a_progressing_giant_note_reader_does_not_hold_stop_or_the_next_turn() {
    run_host_case("drip-live");
}

struct DripReader {
    socket: TcpStream,
    worker: Option<std::thread::JoinHandle<()>>,
}
impl Drop for DripReader {
    fn drop(&mut self) {
        let _ = self.socket.shutdown(std::net::Shutdown::Both);
        if let Some(worker) = self.worker.take() {
            worker.join().unwrap();
        }
    }
}

// This fixture deliberately tests only the poll scheduler's historical
// Closed-before-tail rule. Production uses the real EventReceiver; no adapter
// can inject this synthetic sequence into the event store.
async fn raw_host_input(
    events: &mut tokio::sync::mpsc::UnboundedReceiver<khor_acp::Event>,
    ops: &mut tokio::sync::mpsc::UnboundedReceiver<(u64, GuiOp, bool)>,
    prefer_ops: bool,
    events_closed: bool,
) -> HostInput {
    std::future::poll_fn(|cx| {
        poll_host_input(
            cx,
            |cx| events.poll_recv(cx),
            ops,
            prefer_ops,
            events_closed,
        )
    })
    .await
}

#[test]
fn closed_does_not_strand_an_event_already_inside_the_completion_fence() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let (source, mut events) = tokio::sync::mpsc::unbounded_channel();
        let (_input, mut ops) = tokio::sync::mpsc::unbounded_channel();
        source.send(khor_acp::Event::Closed(None)).unwrap();
        source
            .send(khor_acp::Event::Ready {
                session: agent_client_protocol::schema::v1::SessionId::new("accepted-tail"),
            })
            .unwrap();
        assert!(matches!(
            raw_host_input(&mut events, &mut ops, false, false).await,
            HostInput::Event(Some(khor_acp::Event::Closed(None)))
        ));
        // Both were accepted into the same finite completion snapshot. Closed
        // defers shutdown, but the already-captured tail still has to drain.
        let tail = tokio::time::timeout(
            Duration::from_millis(100),
            raw_host_input(
                &mut events,
                &mut ops,
                false,
                pause_closed_events(true, Some(1)),
            ),
        )
        .await
        .expect("Closed stranded an already-accepted event inside the completion fence");
        assert!(matches!(
            tail,
            HostInput::Event(Some(khor_acp::Event::Ready { .. }))
        ));
        drop(source);
        assert!(
            tokio::time::timeout(
                Duration::from_millis(20),
                raw_host_input(
                    &mut events,
                    &mut ops,
                    false,
                    pause_closed_events(true, Some(0))
                )
            )
            .await
            .is_err(),
            "a drained closed receiver must not keep the loop spinning"
        );
    });
}

fn process_pid(scratch: &std::path::Path, mode: &str) -> libc::pid_t {
    let value: serde_json::Value =
        serde_json::from_str(&await_file(&scratch.join(mode).join("pid.json"))).unwrap();
    value["pid"].as_i64().unwrap() as libc::pid_t
}
fn assert_alive(pid: libc::pid_t) {
    assert_eq!(
        unsafe { libc::kill(pid, 0) },
        0,
        "ACP process was unexpectedly retired"
    );
}
fn assert_reaped(pid: libc::pid_t) {
    assert_eq!(
        unsafe { libc::kill(pid, 0) },
        -1,
        "Gone/return preceded process reap"
    );
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ESRCH)
    );
}
fn next_note(notes: &mpsc::Receiver<GuiNote>, deadline: Instant) -> GuiNote {
    notes
        .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        .expect("host did not publish its expected completion")
}

fn await_capture(notes: &mpsc::Receiver<GuiNote>, expected: [u8; 16]) -> CaptureFence {
    let deadline = Instant::now() + Duration::from_secs(4);
    loop {
        match next_note(notes, deadline) {
            GuiNote::Captured { token, result } => {
                assert_eq!(token, expected, "a view received another view's capture");
                return result.unwrap();
            }
            GuiNote::Gone => panic!("capture retired the live ACP process"),
            _ => {}
        }
    }
}

fn capture_page(
    node: &crate::Node,
    fence: &CaptureFence,
    newer: bool,
) -> crate::adaptor::transcript_page::Page<crate::session_content::ContentEvent> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let key = node.content_begin(&fence.binding).await.unwrap();
            let request = crate::session_content::remote::ClientRead::claim(
                node.root(),
                fence.binding.clone(),
                &key,
            )
            .unwrap();
            node.content_page(&request, Some(fence.cursor.clone()), newer)
                .await
                .unwrap()
        })
}

fn capture_chunks(
    page: &crate::adaptor::transcript_page::Page<crate::session_content::ContentEvent>,
) -> usize {
    page.events
        .iter()
        .filter(|event| {
            matches!(&event.payload,
                crate::session_content::ContentEvent::Update { update: note, .. }
                    if note["update"]["content"]["text"] == "capture identical chunk"
            )
        })
        .count()
}
fn await_turn(notes: &mpsc::Receiver<GuiNote>, expected: &str, final_text: &str) {
    let deadline = Instant::now() + Duration::from_secs(4);
    let mut final_note = false;
    loop {
        match next_note(notes, deadline) {
            GuiNote::Note(json) if json.contains(final_text) => final_note = true,
            GuiNote::Turn(stop) => {
                assert_eq!(stop, expected);
                assert!(final_note, "Turn overtook its final Note");
                return;
            }
            GuiNote::Gone => panic!("host retired before its cooperative completion"),
            _ => {}
        }
    }
}
fn close_host(
    host: std::thread::JoinHandle<Result<(), String>>,
    conn: &mut TcpStream,
    notes: &mpsc::Receiver<GuiNote>,
    pid: libc::pid_t,
) {
    write_frame(conn, &GuiOp::Close).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let note = match notes.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
            Ok(note) => note,
            Err(error) => {
                let result = if host.is_finished() {
                    Some(host.join().map_err(|_| "host thread panicked".to_owned()))
                } else {
                    None
                };
                let diagnostics = PathBuf::from(std::env::var_os("KHOR_ACP_STOP_SCRATCH").unwrap())
                    .join("close-diagnostics");
                std::fs::create_dir_all(&diagnostics).unwrap();
                std::fs::write(
                    diagnostics.join("result.txt"),
                    format!("receive={error}; host={result:?}\n"),
                )
                .unwrap();
                panic!("host Close did not publish Gone: {error}; host={result:?}");
            }
        };
        if matches!(note, GuiNote::Gone) {
            assert_reaped(pid);
            break;
        }
    }
    host.join().unwrap().unwrap();
}
fn extra_face(root: &std::path::Path, scratch: &std::path::Path, mode: &str) -> TcpStream {
    let id = khor_core::SessionId(
        await_file(&scratch.join(format!("{mode}.ready")))
            .trim()
            .to_owned(),
    );
    let live = LiveKind::new(root.to_owned(), khor_core::DeviceId([1; 32]));
    let file = crate::host::read_host_file(&live.dir_of(&id).unwrap()).unwrap();
    let mut conn = TcpStream::connect(("127.0.0.1", file.port)).unwrap();
    conn.set_write_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    write_frame(
        &mut conn,
        &Hello {
            cookie: file.cookie,
            cols: 0,
            rows: 0,
        },
    )
    .unwrap();
    assert!(read_frame::<Welcome>(&mut conn).unwrap().ok);
    conn
}

#[test]
#[ignore = "isolated real GUI host lifecycle/fairness worker"]
fn managed_host_case_worker() {
    let scratch = PathBuf::from(std::env::var_os("KHOR_ACP_STOP_SCRATCH").unwrap());
    let case = std::env::var("KHOR_ACP_HOST_CASE").unwrap();
    std::fs::write(
        scratch.join("agent.py"),
        include_str!("fixtures/acp-cancel.py"),
    )
    .unwrap();
    let root = scratch.join("store");
    std::fs::create_dir(&root).unwrap();
    match case.as_str() {
        "capture-no-load" => {
            unsafe {
                std::env::set_var(VENDOR_ENV, "acp:capture-fixture");
                std::env::remove_var(RESUME_ENV);
            }
            let (host, mut a, notes_a) = open_host(&root, &scratch, "journal-no-load");
            let deadline = Instant::now() + Duration::from_secs(4);
            loop {
                if let GuiNote::Agent { replays, .. } = next_note(&notes_a, deadline) {
                    assert!(!replays, "fixture must actually advertise NoLoad");
                    break;
                }
            }
            write_frame(
                &mut a,
                &GuiOp::SayWith {
                    text: "first accepted prompt".into(),
                    images: vec![("image/png".into(), "aW1hZ2U=".into())],
                    files: vec![("original.txt".into(), "原文".into())],
                    token: None,
                },
            )
            .unwrap();
            await_turn(&notes_a, "EndTurn", "capture identical chunk");
            let token_a = [1; 16];
            write_frame(&mut a, &GuiOp::Capture { token: token_a }).unwrap();
            let first = await_capture(&notes_a, token_a);

            let mut b = extra_face(&root, &scratch, "journal-no-load");
            let mut reader = b.try_clone().unwrap();
            let (tx, notes_b) = mpsc::channel();
            std::thread::spawn(move || {
                while let Ok(note) = read_frame::<GuiNote>(&mut reader) {
                    let end = matches!(note, GuiNote::Gone);
                    if tx.send(note).is_err() || end {
                        break;
                    }
                }
            });
            write_frame(&mut b, &GuiOp::Say("second accepted prompt".into())).unwrap();
            await_turn(&notes_b, "EndTurn", "capture identical chunk");
            let token_b = [2; 16];
            write_frame(&mut b, &GuiOp::Capture { token: token_b }).unwrap();
            let second = await_capture(&notes_b, token_b);
            assert_eq!(first.binding, second.binding);
            assert_ne!(first.cursor, second.cursor);
            // A's new reply is an in-stream barrier after B's response.
            // await_capture rejects any leaked B reply before that barrier.
            write_frame(&mut a, &GuiOp::Capture { token: token_a }).unwrap();
            let latest_a = await_capture(&notes_a, token_a);
            assert_eq!(latest_a.cursor, second.cursor);
            for (token, fence) in [(token_a, &first), (token_b, &second)] {
                assert!(
                    crate::proto::encode(&captured_note(token, Ok(fence.clone())))
                        .unwrap()
                        .len()
                        <= CAPTURE_WIRE_BYTES
                );
            }
            let node = crate::Node::open(root.clone()).unwrap();
            let before = capture_page(&node, &first, false);
            let after = capture_page(&node, &first, true);
            let all = capture_page(&node, &second, false);
            assert!(!before.reset && !after.reset && !all.reset);
            assert_eq!(capture_chunks(&before), 1);
            assert_eq!(capture_chunks(&after), 1);
            assert_eq!(capture_chunks(&all), 2);
            assert!(before.events.len() <= crate::adaptor::transcript_page::EVENT_LIMIT);
            assert!(after.events.len() <= crate::adaptor::transcript_page::EVENT_LIMIT);
            let records = first
                .binding
                .resolve_exact(&root, node.device())
                .unwrap()
                .unwrap();
            let (mut file, _) = records.open().unwrap();
            let mut text = String::new();
            file.read_to_string(&mut text).unwrap();
            let rows: Vec<serde_json::Value> = text
                .lines()
                .map(|line| serde_json::from_str(line).unwrap())
                .collect();
            assert_eq!(rows.iter().filter(|row| row["kind"] == "prompt").count(), 2);
            let prompt = rows
                .iter()
                .find(|row| row["text"] == "first accepted prompt")
                .unwrap();
            assert_eq!(prompt["images"][0][1], "aW1hZ2U=");
            assert_eq!(prompt["files"][0][1], "原文");
            assert_eq!(
                rows.iter()
                    .filter(|row| row["notification"]["update"]["content"]["text"]
                        == "capture identical chunk")
                    .count(),
                2
            );
            a.shutdown(std::net::Shutdown::Both).unwrap();
            drop(notes_a);
            assert_alive(process_pid(&scratch, "journal-no-load"));
            close_host(
                host,
                &mut b,
                &notes_b,
                process_pid(&scratch, "journal-no-load"),
            );
            let received =
                std::fs::read_to_string(scratch.join("journal-no-load/received.jsonl")).unwrap();
            let calls: Vec<serde_json::Value> = received
                .lines()
                .map(|line| serde_json::from_str(line).unwrap())
                .collect();
            let row = LiveKind::new(root.clone(), node.device())
                .dir_of(first.binding.view())
                .unwrap();
            let attempt: serde_json::Value =
                serde_json::from_slice(&std::fs::read(row.join("recovery-attempt.json")).unwrap())
                    .unwrap();
            assert_eq!(
                attempt["latest"]["support"],
                serde_json::json!({"load":false,"resume":false})
            );
            assert_eq!(
                calls
                    .iter()
                    .filter(|call| call["method"] == "session/new")
                    .count(),
                1
            );
            assert!(
                !calls
                    .iter()
                    .any(|call| call["method"] == "session/load"
                        || call["method"] == "session/resume")
            );
        }
        "journal" => {
            unsafe {
                std::env::set_var(VENDOR_ENV, "acp:journal-fixture");
                std::env::set_var(RESUME_ENV, "synthetic-q037-session");
            }
            let (host, mut conn, notes) = open_host(&root, &scratch, "journal-load");
            let id = SessionId(
                await_file(&scratch.join("journal-load.ready"))
                    .trim()
                    .to_owned(),
            );
            let node = crate::Node::open(root.clone()).unwrap();
            let live = LiveKind::new(root.clone(), node.device());
            let directory = live.dir_of(&id).unwrap();
            let identity: JournalIdentity =
                serde_json::from_slice(&std::fs::read(directory.join("journal.json")).unwrap())
                    .unwrap();
            let current = identity.directory(&root).join("current");
            let records = current.join("records.jsonl");
            let image = "i".repeat(150_000);
            write_frame(
                &mut conn,
                &GuiOp::SayWith {
                    text: "original user words".into(),
                    images: vec![("image/png".into(), image.clone())],
                    files: vec![("文档.txt".into(), "原文".into())],
                    token: None,
                },
            )
            .unwrap();
            await_turn(&notes, "MaxTokens", "journal repeated delta");
            let accepted: serde_json::Value = serde_json::from_str(
                await_file(&scratch.join("journal-load/accepted.jsonl"))
                    .lines()
                    .next()
                    .unwrap(),
            )
            .unwrap();
            assert_eq!(accepted["before_dispatch"], true);
            let text = std::fs::read_to_string(&records).unwrap();
            let rows: Vec<serde_json::Value> = text
                .lines()
                .map(|line| serde_json::from_str(line).unwrap())
                .collect();
            let prompt = rows.iter().find(|row| row["kind"] == "prompt").unwrap();
            assert_eq!(prompt["text"], "original user words");
            assert_eq!(prompt["images"][0][1], image);
            assert_eq!(prompt["files"][0][1], "原文");
            assert_eq!(
                rows.iter()
                    .filter(|row| row["notification"]["update"]["content"]["text"]
                        == "journal repeated delta")
                    .count(),
                2
            );
            let turn = rows.iter().find(|row| row["kind"] == "turn").unwrap();
            assert_eq!(turn["outcome"], "ended");
            assert_eq!(turn["detail"], "MaxTokens");
            let cancelled = std::sync::atomic::AtomicBool::new(false);
            let page = node.transcript_page(&id, None, false, &cancelled).unwrap();
            let own = live
                .rows(|_| 0)
                .into_iter()
                .find(|row| row.id == id)
                .expect("the real host publishes its session row");
            let alias = node.import_native_observation(&own).unwrap();
            assert!(matches!(
                node.session_content(&alias, None).unwrap(),
                crate::session_content::Availability::Available {
                    capture: Some(_),
                    ..
                }
            ));
            let observed = node
                .transcript_page(&alias, None, false, &cancelled)
                .unwrap();
            assert!(
                observed.events.iter().any(|event| match &event.payload {
                    crate::session_content::ContentEvent::Update {
                        update: notification,
                        ..
                    } => notification["update"]["content"][0]["newText"] == "new",
                    _ => false,
                }),
                "rich ACP diff survives the common read-only source pipeline"
            );
            assert!(
                !page.references.is_empty(),
                "large prompt remains an explicit raw reference"
            );
            let reference = page
                .references
                .iter()
                .find(|reference| reference.range.end - reference.range.start > 64 * 1024)
                .unwrap();
            let cursor = page.cursor_before(crate::adaptor::transcript_page::Position {
                start: reference.range.start,
                end: reference.range.end,
                item: 0,
            });
            let chunk = node
                .transcript_record(&id, &cursor, reference.range.start, &cancelled)
                .unwrap();
            assert!(chunk.bytes.len() <= 64 * 1024);
            assert!(chunk.next.is_some());
            assert!(String::from_utf8_lossy(&chunk.bytes).contains("original user words"));
            let token = [71; 16];
            write_frame(&mut conn, &GuiOp::Capture { token }).unwrap();
            let first = await_capture(&notes, token);
            let _ = capture_page(&node, &first, false);
            let (mut reattached, reattached_notes) = attach_host(&root, &scratch, "journal-load");
            let token = [72; 16];
            write_frame(&mut reattached, &GuiOp::Capture { token }).unwrap();
            let second = await_capture(&reattached_notes, token);
            assert_eq!(
                first.binding, second.binding,
                "reattach must preserve canonical source identity"
            );
            let _ = capture_page(&node, &second, false);
            reattached.shutdown(std::net::Shutdown::Both).unwrap();
            drop(reattached);
            drop(reattached_notes);
            assert_eq!(
                std::fs::read(&records).unwrap(),
                text.as_bytes(),
                "repeated Capture and reattach do not mutate the canonical capture"
            );
            close_host(
                host,
                &mut conn,
                &notes,
                process_pid(&scratch, "journal-load"),
            );
            std::fs::rename(&records, current.join("previous.jsonl")).unwrap();
            std::fs::copy(current.join("previous.jsonl"), &records).unwrap();
            assert!(
                node.transcript_record(&id, &cursor, reference.range.start, &cancelled)
                    .err()
                    .unwrap()
                    .contains("changed")
            );
            std::fs::remove_file(scratch.join("journal-load.ready")).unwrap();
            let (host, mut conn, notes) = open_host(&root, &scratch, "journal-load");
            assert_eq!(
                std::fs::read(&records).unwrap(),
                text.as_bytes(),
                "Load preserves the old published base byte for byte"
            );
            let token = [73; 16];
            write_frame(&mut conn, &GuiOp::Capture { token }).unwrap();
            let loaded = await_capture(&notes, token);
            assert_ne!(
                loaded.binding, first.binding,
                "a complete Load publishes a separate display generation"
            );
            let loaded_source = loaded
                .binding
                .resolve_exact(&root, node.device())
                .unwrap()
                .unwrap();
            let read_loaded = || {
                let (mut file, _) = loaded_source.open().unwrap();
                let mut raw = String::new();
                file.read_to_string(&mut raw).unwrap();
                raw
            };
            let after = read_loaded();
            assert_eq!(
                after.matches("journal snapshot base").count(),
                1,
                "startup load is a full new base, not another append of the committed prefix"
            );
            assert!(after.contains("\"kind\":\"gap\""));
            assert!(
                node.transcript_page(&id, Some(&page.newer), true, &cancelled)
                    .unwrap()
                    .reset
            );
            close_host(
                host,
                &mut conn,
                &notes,
                process_pid(&scratch, "journal-load"),
            );
            let (host, mut conn, notes) = open_host(&root, &scratch, "journal-resume");
            let resumed = read_loaded();
            assert!(resumed.starts_with(&after));
            assert!(resumed.contains("\"opening\":\"resume\""));
            assert_eq!(resumed.matches("新运行早期").count(), 1);
            assert_eq!(std::fs::read(&records).unwrap(), text.as_bytes());
            close_host(
                host,
                &mut conn,
                &notes,
                process_pid(&scratch, "journal-resume"),
            );
        }
        "journal-resume" => {
            unsafe {
                std::env::set_var(VENDOR_ENV, "acp:journal-fixture");
                std::env::set_var(RESUME_ENV, "synthetic-q037-session");
            }
            let (host, mut conn, notes) = open_host(&root, &scratch, "journal-resume");
            let id = SessionId(
                await_file(&scratch.join("journal-resume.ready"))
                    .trim()
                    .to_owned(),
            );
            let node = crate::Node::open(root.clone()).unwrap();
            let row = LiveKind::new(root.clone(), node.device())
                .dir_of(&id)
                .unwrap();
            let attempt: serde_json::Value =
                serde_json::from_slice(&std::fs::read(row.join("recovery-attempt.json")).unwrap())
                    .unwrap();
            // This fixture advertises both capabilities. Persist the actual
            // Initialize facts; the resume1/load0/new0 checks below independently
            // prove which opening was selected, even when Load is available.
            assert_eq!(
                attempt["latest"]["support"],
                serde_json::json!({"load":true,"resume":true})
            );
            assert_eq!(
                node.session_content(&id, None).unwrap(),
                crate::session_content::Availability::Available {
                    following: true,
                    capture: Some(crate::session_content::Capture {
                        opening: "resume",
                        gaps: 0
                    })
                }
            );
            let received =
                std::fs::read_to_string(scratch.join("journal-resume/received.jsonl")).unwrap();
            let calls: Vec<serde_json::Value> = received
                .lines()
                .map(|line| serde_json::from_str(line).unwrap())
                .collect();
            assert_eq!(
                calls
                    .iter()
                    .filter(|call| call["method"] == "session/resume")
                    .count(),
                1
            );
            assert!(!calls.iter().any(|call|call["method"]=="session/load"||call["method"]=="session/new"));
            let page = node
                .transcript_page(&id, None, false, &std::sync::atomic::AtomicBool::new(false))
                .unwrap();
            assert_eq!(
                page.events
                    .iter()
                    .filter(|event| matches!(&event.payload,
                crate::session_content::ContentEvent::Update{update:note,..}
                if note["update"]["content"]["text"]=="新运行早期"))
                    .count(),
                1
            );
            let source = crate::session_content::journal::Source::registered(
                &root,
                node.device(),
                &id,
                LiveKind::new(root.clone(), node.device())
                    .dir_of(&id)
                    .as_deref(),
            )
            .unwrap()
            .unwrap();
            let mut raw = String::new();
            source.open().unwrap().read_to_string(&mut raw).unwrap();
            let rows: Vec<serde_json::Value> = raw
                .lines()
                .map(|line| serde_json::from_str(line).unwrap())
                .collect();
            assert_eq!(
                rows.iter()
                    .filter(|row| row["notification"]["update"]["content"]["text"] == "新运行早期")
                    .count(),
                1
            );
            close_host(
                host,
                &mut conn,
                &notes,
                process_pid(&scratch, "journal-resume"),
            );
        }
        "journal-abort" => {
            unsafe {
                std::env::set_var(VENDOR_ENV, "acp:journal-fixture");
                std::env::remove_var(RESUME_ENV);
            }
            for mode in ["journal-invalid", "journal-opening"] {
                let root = root.join(mode);
                std::fs::create_dir_all(&root).unwrap();
                let ready = scratch.join(format!("{mode}.ready"));
                let marker = scratch.join(mode);
                let command=serde_json::json!({"command":"python3","args":[scratch.join("agent.py"),mode,marker]}).to_string();
                let id = crate::adaptor::id_for("synthetic-q037-session");
                let node = crate::Node::open(root.clone()).unwrap();
                let live = LiveKind::new(root.clone(), node.device());
                let expected_identity = gui_new_source_identity(
                    node.device(),
                    std::slice::from_ref(&command),
                    "synthetic-q037-session",
                );
                let handle = {
                    let root = root.clone();
                    let ready = ready.clone();
                    std::thread::spawn(move || {
                        gui_host_main_with_timeouts(
                            root,
                            ready,
                            "bootstrap fixture".into(),
                            vec![command],
                            HostTimeouts::default(),
                        )
                    })
                };
                if mode == "journal-opening" {
                    let until = Instant::now() + Duration::from_secs(5);
                    let host = loop {
                        if handle.is_finished() {
                            panic!(
                                "bootstrap host exited before publishing its endpoint: {:?}",
                                handle.join().unwrap()
                            );
                        }
                        if let Some(directory) = live.dir_of(&id) {
                            if let Ok(host) = crate::host::read_host_file(&directory) {
                                break host;
                            }
                        }
                        assert!(Instant::now() < until);
                        std::thread::sleep(Duration::from_millis(1));
                    };
                    let mut conn = TcpStream::connect(("127.0.0.1", host.port)).unwrap();
                    write_frame(
                        &mut conn,
                        &Hello {
                            cookie: host.cookie,
                            cols: 0,
                            rows: 0,
                        },
                    )
                    .unwrap();
                    assert!(read_frame::<Welcome>(&mut conn).unwrap().ok);
                    write_frame(&mut conn, &GuiOp::Close).unwrap();
                }
                let until = Instant::now() + Duration::from_secs(10);
                while !handle.is_finished() {
                    assert!(Instant::now() < until, "bootstrap failed to retire");
                    std::thread::sleep(Duration::from_millis(5));
                }
                handle.join().unwrap().unwrap();
                assert!(!ready.exists());
                assert!(refusal_path(&ready).exists());
                assert_reaped(process_pid(&scratch, mode));
                assert!(
                    !live.dir_of(&id).unwrap().join("journal.json").exists(),
                    "an aborted unpublished New cannot publish a semantic binding"
                );
                let identity = expected_identity;
                assert!(!identity.directory(&root).join("current").exists());
                assert!(!identity.directory(&root).join("pending").exists());
                let journal = Journal::open(&root, identity, JournalOpening::New).unwrap();
                assert!(journal.is_pending());
                drop(journal);
            }
        }
        "cooperative" | "flood" => {
            let mode = if case == "flood" {
                "cooperative-flood"
            } else {
                "cooperative"
            };
            let (host, mut conn, notes) = open_host(&root, &scratch, mode);
            let pid = process_pid(&scratch, mode);
            write_frame(&mut conn, &GuiOp::Say("first".into())).unwrap();
            await_file(&scratch.join(mode).join("prompt.jsonl"));
            if case == "flood" {
                await_file(&scratch.join(mode).join("flood-progress.jsonl"));
            }
            let cancelled = Instant::now();
            write_frame(&mut conn, &GuiOp::Stop).unwrap();
            let producer = if case == "flood" {
                let mut stream = conn.try_clone().unwrap();
                Some(std::thread::spawn(move || {
                    use std::io::Write;
                    let bytes = crate::proto::encode(&GuiOp::Answer {
                        ask: u64::MAX,
                        option: None,
                    })
                    .unwrap();
                    let mut frame = Vec::new();
                    frame.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
                    frame.extend_from_slice(&bytes);
                    let batch = frame.repeat(256);
                    let deadline = Instant::now() + Duration::from_secs(3);
                    for _ in 0..2048 {
                        if Instant::now() >= deadline || stream.write_all(&batch).is_err() {
                            break;
                        }
                    }
                }))
            } else {
                None
            };
            await_file(&scratch.join(mode).join("cancel.jsonl"));
            assert!(
                cancelled.elapsed() < Duration::from_secs(2),
                "Stop was starved by traffic"
            );
            await_turn(&notes, "Cancelled", "synthetic final note after cancel");
            if let Some(producer) = producer {
                producer.join().unwrap();
            }
            // Cross the real production ten-second operation grace, not a short test override.
            std::thread::sleep(
                (cancelled + Duration::from_millis(10_500))
                    .saturating_duration_since(Instant::now()),
            );
            assert_alive(pid);
            write_frame(&mut conn, &GuiOp::Say("next".into())).unwrap();
            await_turn(
                &notes,
                "EndTurn",
                "synthetic final note before normal completion",
            );
            assert_alive(pid);
            close_host(host, &mut conn, &notes, pid);
        }
        "drip-live" => {
            use std::io::Read;
            use std::os::fd::AsRawFd;
            use std::sync::atomic::{AtomicUsize, Ordering};
            let mode = "drip-live";
            let (host, conn, notes) = open_host(&root, &scratch, mode);
            let pid = process_pid(&scratch, mode);
            // Only the raw view is present when the giant live Note is sent.
            // Ordinary GUI frame readers reject >1MiB and must not be used as
            // an unnoticed giant-frame sink. Attach the control view afterward.
            conn.shutdown(std::net::Shutdown::Both).unwrap();
            drop(conn);
            drop(notes);
            let mut reader = extra_face(&root, &scratch, mode);
            let receive_bytes: libc::c_int = 64 * 1024;
            assert_eq!(
                unsafe {
                    libc::setsockopt(
                        reader.as_raw_fd(),
                        libc::SOL_SOCKET,
                        libc::SO_RCVBUF,
                        (&receive_bytes as *const libc::c_int).cast(),
                        std::mem::size_of_val(&receive_bytes) as libc::socklen_t,
                    )
                },
                0
            );
            reader
                .set_read_timeout(Some(Duration::from_secs(15)))
                .unwrap();
            write_frame(&mut reader, &GuiOp::Say("first".into())).unwrap();
            // Initial capability frames are small. A raw reader deliberately
            // accepts the legal >1MiB outgoing live Note, then keeps progressing.
            let body_length = loop {
                let mut prefix = [0; 4];
                reader.read_exact(&mut prefix).unwrap();
                let length = u32::from_be_bytes(prefix) as usize;
                if length > 30 * 1024 * 1024 {
                    break length;
                }
                let mut body = vec![0; length];
                reader.read_exact(&mut body).unwrap();
            };
            assert!(body_length < 32 * 1024 * 1024);
            let (mut conn, notes) = attach_host(&root, &scratch, mode);
            let progressed = Arc::new(AtomicUsize::new(0));
            let progress = progressed.clone();
            let shutdown = reader.try_clone().unwrap();
            let reading_started = Instant::now();
            let worker = std::thread::spawn(move || {
                let mut chunk = [0; 64 * 1024];
                loop {
                    if reader.read_exact(&mut chunk).is_err() {
                        break;
                    }
                    progress.fetch_add(chunk.len(), Ordering::Release);
                    std::thread::sleep(Duration::from_millis(80));
                }
            });
            let drip = DripReader {
                socket: shutdown,
                worker: Some(worker),
            };
            let until = Instant::now() + Duration::from_secs(15);
            while progressed.load(Ordering::Acquire) < 5 * 1024 * 1024 {
                assert!(
                    Instant::now() < until,
                    "drip reader stopped before the cancellation probe"
                );
                std::thread::sleep(Duration::from_millis(5));
            }
            // A progressing large live frame retains its normal socket policy.
            // Stop/next-turn processing on a different view cannot wait for this
            // view to finish reading it. No retired publisher grace is involved.
            assert_alive(pid);
            assert!(reading_started.elapsed() > Duration::from_secs(2));
            let stopped = Instant::now();
            let progress_at_stop = progressed.load(Ordering::Acquire);
            std::fs::write(scratch.join(mode).join("drip-before-stop.json"), serde_json::to_vec(&serde_json::json!({"body_bytes":body_length,"read_bytes":progress_at_stop,"seconds":reading_started.elapsed().as_secs_f64()})).unwrap()).unwrap();
            write_frame(&mut conn, &GuiOp::Stop).unwrap();
            write_frame(&mut conn, &GuiOp::Say("after cancellation".into())).unwrap();
            let deadline = stopped + Duration::from_secs(4);
            let mut final_note = false;
            let mut cancelled = false;
            let mut final_cancel_note = false;
            loop {
                assert!(
                    Instant::now() < deadline,
                    "a progressing view held cooperative Stop or the next turn"
                );
                match next_note(&notes, deadline) {
                    GuiNote::Refused { why } if why == refused::TURN_IN_FLIGHT => {
                        // Retry only an explicitly refused prompt; keep the
                        // probe bounded without creating an unrelated input flood.
                        std::thread::sleep(Duration::from_millis(10));
                        write_frame(&mut conn, &GuiOp::Say("after cancellation".into())).unwrap();
                    }
                    GuiNote::Note(json)
                        if json.contains("synthetic final note before normal completion") =>
                    {
                        final_note = true
                    }
                    GuiNote::Note(json) if json.contains("synthetic final note after cancel") => {
                        final_cancel_note = true
                    }
                    GuiNote::Turn(stop) if stop == "Cancelled" => {
                        assert!(
                            final_cancel_note,
                            "Cancelled overtook its final accepted Note"
                        );
                        cancelled = true;
                    }
                    GuiNote::Turn(stop) => {
                        assert_eq!(stop, "EndTurn");
                        assert!(cancelled && final_note);
                        break;
                    }
                    GuiNote::Gone => panic!("a slow view retired the cooperative ACP process"),
                    _ => {}
                }
            }
            assert!(
                scratch.join(mode).join("cancel.jsonl").is_file(),
                "the cooperative Stop must reach the actual SDK peer"
            );
            while progressed.load(Ordering::Acquire) <= progress_at_stop {
                assert!(
                    stopped.elapsed() < Duration::from_secs(3),
                    "the ongoing reader stopped making progress"
                );
                std::thread::sleep(Duration::from_millis(5));
            }
            assert!(
                stopped.elapsed() < Duration::from_secs(3),
                "the slow view blocked cooperative Stop or the next turn"
            );
            assert!(
                progressed.load(Ordering::Acquire) > progress_at_stop,
                "the capacity probe was a stalled socket, not an ongoing drip"
            );
            assert_alive(pid);
            std::fs::write(scratch.join(mode).join("drip-after-stop.json"), serde_json::to_vec(&serde_json::json!({"read_bytes":progressed.load(Ordering::Acquire),"seconds":stopped.elapsed().as_secs_f64(),"next_prompt":"EndTurn"})).unwrap()).unwrap();
            drop(drip);
            close_host(host, &mut conn, &notes, pid);
        }
        "stop-capture" | "close-capture" => {
            let mode = if case == "close-capture" {
                "capture-close"
            } else {
                "capture-flood"
            };
            let (host, mut conn, notes) = open_host(&root, &scratch, mode);
            let pid = process_pid(&scratch, mode);
            let unread = extra_face(&root, &scratch, mode);
            write_frame(&mut conn, &GuiOp::Say("first".into())).unwrap();
            await_file(&scratch.join(mode).join("flood-progress.jsonl"));
            if case == "close-capture" {
                close_host(host, &mut conn, &notes, pid);
                drop(unread);
                return;
            }
            write_frame(&mut conn, &GuiOp::Stop).unwrap();
            write_frame(
                &mut conn,
                &GuiOp::ElicitAnswer {
                    ask: u64::MAX,
                    response: "{}".into(),
                },
            )
            .unwrap();
            let deadline = Instant::now() + Duration::from_secs(3);
            let (mut final_note, mut cancelled, mut refused) = (false, false, false);
            while !cancelled || !refused {
                match next_note(&notes, deadline) {
                    GuiNote::Note(json) if json.contains("synthetic final note after cancel") => {
                        final_note = true
                    }
                    GuiNote::Turn(stop) => {
                        assert_eq!(stop, "Cancelled");
                        assert!(final_note, "Cancelled overtook its accepted final Note");
                        cancelled = true;
                    }
                    GuiNote::Refused { .. } => refused = true,
                    GuiNote::Gone => panic!("Stop retired the cooperative process"),
                    _ => {}
                }
            }
            assert!(
                scratch.join(mode).join("cancel.jsonl").is_file(),
                "current-turn Stop must actually cross the SDK cancel path"
            );
            assert_alive(pid);
            let token = [73; 16];
            write_frame(&mut conn, &GuiOp::Capture { token }).unwrap();
            let capture = await_capture(&notes, token);
            let node = crate::Node::open(root.clone()).unwrap();
            let page = capture_page(&node, &capture, false);
            assert!(
                page.events.iter().any(|event| matches!(&event.payload,
                crate::session_content::ContentEvent::Update { update: note, .. }
                    if note.to_string().contains("synthetic final note after cancel"))),
                "the confirmed cancelled tail must remain in canonical history"
            );
            write_frame(&mut conn, &GuiOp::Say("next".into())).unwrap();
            await_turn(
                &notes,
                "EndTurn",
                "synthetic final note before normal completion",
            );
            assert_alive(pid);
            assert!(
                !scratch.join(mode).join("load.jsonl").exists(),
                "Capture must not reload the live agent"
            );
            close_host(host, &mut conn, &notes, pid);
            drop(unread);
        }
        "many-load" => {
            let mode = "many-small-load";
            unsafe {
                std::env::set_var(RESUME_ENV, "synthetic-q037-session");
            }
            let deadline = Instant::now() + Duration::from_secs(12);
            let host = launch_host(&root, &scratch, mode, HostTimeouts::default());
            // Preserve the original5s basic-handshake bound and12s historical
            // data probe, now through supported opening rather than live Replay.
            await_file(&scratch.join(mode).join("load.jsonl"));
            let ready = scratch.join(format!("{mode}.ready"));
            while !ready.is_file() {
                assert!(!host.is_finished());
                assert!(
                    Instant::now() < deadline,
                    "initial history did not commit within the data probe"
                );
                std::thread::sleep(Duration::from_millis(5));
            }
            let (mut conn, notes) = attach_host(&root, &scratch, mode);
            let pid = process_pid(&scratch, mode);
            let token = [74; 16];
            write_frame(&mut conn, &GuiOp::Capture { token }).unwrap();
            let capture = await_capture(&notes, token);
            let node = crate::Node::open(root.clone()).unwrap();
            let (count, total) = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    tokio::time::timeout_at(tokio::time::Instant::from_std(deadline), async {
                        let mut cursor = Some(capture.cursor.clone());
                        let mut end = 16384usize;
                        let mut count = 0;
                        let mut total = 0;
                        while let Some(before) = cursor {
                            assert!(
                                Instant::now() < deadline,
                                "canonical pagination exceeded the original12s history probe"
                            );
                            let key = node.content_begin(&capture.binding).await.unwrap();
                            let request = crate::session_content::remote::ClientRead::claim(
                                &root,
                                capture.binding.clone(),
                                &key,
                            )
                            .unwrap();
                            let page = node
                                .content_page(&request, Some(before), false)
                                .await
                                .unwrap();
                            assert!(!page.reset && page.references.is_empty());
                            let base = end
                                .checked_sub(page.events.len())
                                .expect("history duplicated rows");
                            for (index, event) in page.events.iter().enumerate() {
                                let crate::session_content::ContentEvent::Update {
                                    update: note,
                                    ..
                                } = &event.payload
                                else {
                                    panic!("unexpected non-history record")
                                };
                                let text = note["update"]["content"]["text"].as_str().unwrap();
                                assert!(
                                    text.starts_with(&format!("load-item-{}:", base + index)),
                                    "history rows were skipped or reordered"
                                );
                                total += note.to_string().len();
                            }
                            count += page.events.len();
                            end = base;
                            cursor = page.older;
                        }
                        assert_eq!(end, 0);
                        (count, total)
                    })
                    .await
                    .expect("canonical data exceeded the original12s history probe")
                });
            assert_eq!(count, 16384);
            assert!(total < 32 * 1024 * 1024);
            let received =
                std::fs::read_to_string(scratch.join(mode).join("received.jsonl")).unwrap();
            assert_eq!(
                received
                    .lines()
                    .filter(
                        |line| serde_json::from_str::<serde_json::Value>(line).unwrap()["method"]
                            == "session/load"
                    )
                    .count(),
                1
            );
            assert!(!received.lines().any(|line| {
                serde_json::from_str::<serde_json::Value>(line).unwrap()["method"] == "session/new"
            }));
            assert_alive(pid);
            close_host(host, &mut conn, &notes, pid);
        }
        "control-capture" => {
            let mode = "control-pending";
            let (host, mut conn, notes) = open_host(&root, &scratch, mode);
            let pid = process_pid(&scratch, mode);
            write_frame(
                &mut conn,
                &GuiOp::Control {
                    token: "hold".into(),
                    operation: "authenticate".into(),
                    payload: "{\"methodId\":\"synthetic\"}".into(),
                },
            )
            .unwrap();
            await_file(&scratch.join(mode).join("control.jsonl"));
            let token = [75; 16];
            write_frame(&mut conn, &GuiOp::Capture { token }).unwrap();
            write_frame(
                &mut conn,
                &GuiOp::ElicitAnswer {
                    ask: u64::MAX,
                    response: "{}".into(),
                },
            )
            .unwrap();
            let deadline = Instant::now() + Duration::from_secs(3);
            let (mut captured, mut refused) = (None, false);
            while captured.is_none() || !refused {
                match next_note(&notes, deadline) {
                    GuiNote::Captured {
                        token: received,
                        result,
                    } => {
                        assert_eq!(received, token);
                        captured = Some(result.unwrap());
                    }
                    GuiNote::Refused { .. } => refused = true,
                    GuiNote::ControlResult { .. } => {
                        panic!("Control completed before the fixture release")
                    }
                    GuiNote::Gone => panic!("Capture killed a pending Control"),
                    _ => {}
                }
            }
            let node = crate::Node::open(root.clone()).unwrap();
            let _ = capture_page(&node, &captured.unwrap(), false);
            assert_alive(pid);
            assert!(
                !scratch.join(mode).join("load.jsonl").exists(),
                "Capture must read canonical history without an agent reload"
            );
            std::fs::write(scratch.join(mode).join("release-control"), b"release").unwrap();
            let deadline = Instant::now() + Duration::from_secs(4);
            loop {
                match next_note(&notes, deadline) {
                    GuiNote::ControlResult { token, error, .. } if token == "hold" => {
                        assert!(error.is_none());
                        break;
                    }
                    GuiNote::Gone => panic!("canonical capture retired the Control's process"),
                    _ => {}
                }
            }
            assert_alive(pid);
            assert!(!scratch.join(mode).join("load.jsonl").exists());
            close_host(host, &mut conn, &notes, pid);
        }
        "slow-gui" => {
            let mode = "large-flood";
            let (host, mut conn, notes) = open_host(&root, &scratch, mode);
            let pid = process_pid(&scratch, mode);
            let _unread = extra_face(&root, &scratch, mode);
            write_frame(&mut conn, &GuiOp::Say("first".into())).unwrap();
            await_file(&scratch.join(mode).join("flood-progress.jsonl"));
            let stopped = Instant::now();
            write_frame(&mut conn, &GuiOp::Stop).unwrap();
            await_file(&scratch.join(mode).join("cancel.jsonl"));
            assert!(
                stopped.elapsed() < Duration::from_secs(2),
                "an unread GUI blocked another GUI's Stop"
            );
            let deadline = stopped + Duration::from_secs(15);
            loop {
                if matches!(next_note(&notes, deadline), GuiNote::Gone) {
                    assert_reaped(pid);
                    break;
                }
            }
            host.join().unwrap().unwrap();
        }
        "control-timeout" => {
            let mode = "control-timeout";
            let timeouts = HostTimeouts {
                control: Duration::from_millis(200),
                ..HostTimeouts::default()
            };
            let (host, mut conn, notes) = open_host_with_timeouts(&root, &scratch, mode, timeouts);
            let pid = process_pid(&scratch, mode);
            write_frame(
                &mut conn,
                &GuiOp::Control {
                    token: "timeout".into(),
                    operation: "authenticate".into(),
                    payload: r#"{"methodId":"synthetic"}"#.into(),
                },
            )
            .unwrap();
            await_file(&scratch.join(mode).join("control.jsonl"));
            let deadline = Instant::now() + Duration::from_secs(5);
            let mut unknown = false;
            loop {
                match next_note(&notes, deadline) {
                    GuiNote::ControlResult {
                        token, data, error, ..
                    } if token == "timeout" => {
                        assert!(data.is_none());
                        unknown = error.is_some_and(|why| why.contains("unknown"));
                    }
                    GuiNote::Gone => {
                        assert!(unknown, "timeout was not reported as an unknown completion");
                        assert_reaped(pid);
                        break;
                    }
                    _ => {}
                }
            }
            host.join().unwrap().unwrap();
        }
        "handshake" => {
            for mode in ["handshake-refuse", "handshake-stall"] {
                let ready = scratch.join(format!("{mode}.ready"));
                let marker = scratch.join(mode);
                let command = serde_json::json!({"command":"python3","args":[scratch.join("agent.py"),mode,marker]}).to_string();
                let timeouts = HostTimeouts {
                    handshake: Duration::from_millis(250),
                    ..HostTimeouts::default()
                };
                let result = gui_host_main_with_timeouts(
                    root.clone(),
                    ready.clone(),
                    "refusal".into(),
                    vec![command],
                    timeouts,
                );
                assert!(result.is_err());
                assert!(!ready.exists());
                assert!(refusal_path(&ready).exists());
                assert_reaped(process_pid(&scratch, mode));
            }
        }
        _ => panic!("unknown host case {case}"),
    }
}
