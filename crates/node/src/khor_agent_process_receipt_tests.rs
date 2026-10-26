use super::*;

#[test]
fn reaped_receipt_preserves_unread_pipe_tail_and_is_consumed_once() {
    let mut command = Command::new("/bin/sh");
    command.args([
        "-c",
        "read -r gate; printf buffered-tail; printf diagnostic-tail >&2",
    ]);
    let control = Arc::new(Control::default());
    let (mut session, mut pipes) = Session::spawn(command, control).unwrap();
    assert!(
        session.poll_exit().is_none(),
        "stdin gate keeps the child alive"
    );
    pipes.stdin.write_all(b"go\n").unwrap();
    drop(pipes.stdin);
    let deadline = Instant::now() + Duration::from_secs(2);
    let status = loop {
        if let Some(status) = session.poll_exit() {
            break status.unwrap();
        }
        assert!(Instant::now() < deadline, "exit receipt was not published");
        std::thread::sleep(Duration::from_millis(5));
    };
    assert!(status.success());
    assert!(session.poll_exit().is_none());
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    pipes.stdout.read_to_end(&mut stdout).unwrap();
    pipes.stderr.read_to_end(&mut stderr).unwrap();
    assert_eq!(stdout, b"buffered-tail");
    assert_eq!(stderr, b"diagnostic-tail");
    assert_eq!(session.finish(true).unwrap(), status);
    assert!(session.poll_exit().is_none());
}

#[test]
fn missing_supervisor_receipt_is_one_error_then_none() {
    let (tx, rx) = mpsc::sync_channel(1);
    drop(tx);
    let mut session = Session {
        control: Arc::new(Control::default()),
        supervisor: None,
        exit: Some(rx),
        pid: 0,
    };
    assert!(session.poll_exit().unwrap().is_err());
    assert!(session.poll_exit().is_none());
}
