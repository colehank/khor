use khor_gui_core::chat::{self, ChatFrame, Control};
use khor_node::{
    Node, SessionId,
    gui_host::{GuiOp, gui_host_main},
    host::{self, Hello, Welcome, read_frame, write_frame},
};
use std::{
    io::{self, Write},
    net::{Shutdown, TcpListener, TcpStream},
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

struct Fixture {
    root: PathBuf,
    id: String,
    addr: String,
    cookie: String,
    host: Option<std::thread::JoinHandle<Result<(), String>>>,
    stop: Arc<AtomicBool>,
    accepted: Arc<AtomicUsize>,
    first_part: mpsc::Receiver<()>,
    release: mpsc::Sender<()>,
    relay: Option<std::thread::JoinHandle<()>>,
    streams: Arc<Mutex<Vec<TcpStream>>>,
    control: Option<PathBuf>,
    finished: bool,
}
impl Fixture {
    fn new() -> Self {
        Self::start(None)
    }
    fn start(control: Option<PathBuf>) -> Self {
        let root = std::env::temp_dir().join(format!(
            "khor-upload-{}",
            khor_node::link::fresh_hex().unwrap()
        ));
        let root = control
            .as_ref()
            .map(|dir| dir.join("store"))
            .unwrap_or(root);
        std::fs::create_dir(&root).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let ready = root.join("ready");
        let mut command = vec![
            "python3".into(),
            format!(
                "{}/tests/fixtures/upload-agent.py",
                env!("CARGO_MANIFEST_DIR")
            ),
            root.join("requests.jsonl").to_str().unwrap().into(),
        ];
        if control.is_some() {
            drop(private_file(&root.join("requests.jsonl")));
            command.push("--redacted".into());
        }
        let rr = root.clone();
        let rd = ready.clone();
        let thread =
            std::thread::spawn(move || gui_host_main(rr, rd, "upload fixture".into(), command));
        let until = Instant::now() + Duration::from_secs(20);
        while !ready.exists() {
            assert!(Instant::now() < until, "host did not initialize");
            std::thread::sleep(Duration::from_millis(10));
        }
        let id = std::fs::read_to_string(ready).unwrap().trim().to_owned();
        let node = Node::open(root.clone()).unwrap();
        let dir = node.session_dir(&SessionId(id.clone())).unwrap();
        let file = host::read_host_file(&dir).unwrap();
        let addr = format!("127.0.0.1:{}", file.port);
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let port = listener.local_addr().unwrap().port();
        let stop = Arc::new(AtomicBool::new(false));
        let accepted = Arc::new(AtomicUsize::new(0));
        let exiting = stop.clone();
        let count = accepted.clone();
        let upstream = addr.clone();
        let (at_part, first_part) = mpsc::channel();
        let (release, wait) = mpsc::channel();
        let gate = Arc::new(Mutex::new(Some((at_part, wait))));
        let streams = Arc::new(Mutex::new(Vec::new()));
        let connections = streams.clone();
        let relay_control = control.clone();
        let relay = std::thread::spawn(move || {
            let mut workers = Vec::new();
            while !exiting.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((mut client, _)) => {
                        client.set_nonblocking(false).unwrap();
                        let connection = count.fetch_add(1, Ordering::Relaxed) + 1;
                        {
                            let mut sockets = connections.lock().unwrap();
                            if exiting.load(Ordering::Relaxed) {
                                let _ = client.shutdown(Shutdown::Both);
                                break;
                            }
                            sockets.push(client.try_clone().unwrap());
                        }
                        let upstream = upstream.clone();
                        let gate = gate.clone();
                        let control = relay_control.clone();
                        let sockets = connections.clone();
                        let stopping = exiting.clone();
                        workers.push(std::thread::spawn(move || {
                            let _ = client.set_read_timeout(Some(Duration::from_secs(if control.is_some() { 300 } else { 10 })));
                            let _ = client.set_write_timeout(Some(Duration::from_secs(10)));
                            let Ok(hello) = read_frame::<Hello>(&mut client) else {
                                return;
                            };
                            let mut server = TcpStream::connect(upstream).unwrap();
                            sockets.lock().unwrap().push(server.try_clone().unwrap());
                            let _ = server.set_write_timeout(Some(Duration::from_secs(10)));
                            let _ = server.set_read_timeout(Some(Duration::from_secs(10)));
                            write_frame(&mut server, &hello).unwrap();
                            let welcome: Welcome = read_frame(&mut server).unwrap();
                            write_frame(&mut client, &welcome).unwrap();
                            let _ = server.set_read_timeout(None);
                            app_event(&control, serde_json::json!({"event":"connected", "connection":connection}));
                            let mut notes = server.try_clone().unwrap();
                            let mut back = client.try_clone().unwrap();
                            let notes = std::thread::spawn(move || {
                                let _ = io::copy(&mut notes, &mut back);
                                let _ = back.shutdown(Shutdown::Both);
                            });
                            while let Ok(op) = read_frame::<GuiOp>(&mut client) {
                                let part = matches!(op, GuiOp::Part(_));
                                if write_frame(&mut server, &op).is_err() {
                                    break;
                                }
                                if control.is_some() {
                                    app_event(&control, op_event(connection, &op));
                                }
                                if part {
                                    if let Some((at, wait)) = gate.lock().unwrap().take() {
                                        let _ = at.send(());
                                        if let Some(dir) = &control {
                                            publish_private(&dir.join("first-part.json"), &serde_json::json!({
                                                "event":"first_part_forwarded", "connection":connection,
                                                "gate_seconds":30
                                            }));
                                            let deadline = Instant::now() + Duration::from_secs(30);
                                            while !dir.join("release").is_file()
                                                && !dir.join("stop").is_file()
                                                && !stopping.load(Ordering::Relaxed)
                                                && Instant::now() < deadline
                                            {
                                                std::thread::sleep(Duration::from_millis(10));
                                            }
                                            app_event(&control, serde_json::json!({
                                                "event":"gate_released", "connection":connection,
                                                "explicit":dir.join("release").is_file(),
                                                "stop":dir.join("stop").is_file()
                                            }));
                                        } else {
                                            let _ = wait.recv_timeout(Duration::from_secs(5));
                                        }
                                    }
                                }
                            }
                            let _ = server.shutdown(Shutdown::Both);
                            let _ = client.shutdown(Shutdown::Both);
                            notes.join().unwrap();
                        }));
                    }
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(5))
                    }
                    Err(_) => break,
                }
            }
            for worker in workers {
                worker.join().unwrap();
            }
        });
        let path = host::host_file_path(&dir);
        let mut json: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        json["port"] = serde_json::json!(port);
        std::fs::write(path, serde_json::to_vec(&json).unwrap()).unwrap();
        let fixture = Self {
            root,
            id,
            addr,
            cookie: file.cookie,
            host: Some(thread),
            stop,
            accepted,
            first_part,
            release,
            relay: Some(relay),
            streams,
            control,
            finished: false,
        };
        if fixture.control.is_none() {
            assert!(chat::chat_open(&fixture.root, &fixture.id).unwrap());
        }
        fixture
    }
    fn until(&self, mut predicate: impl FnMut(&ChatFrame) -> bool) -> ChatFrame {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let batch = chat::chat_poll(&self.id, 0).unwrap();
            if let Some(frame) = batch.frames.iter().find(|frame| predicate(frame)) {
                return frame.clone();
            }
            assert!(
                Instant::now() < deadline,
                "missing frame; gone={}, frames={:?}",
                batch.gone,
                batch.frames
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    fn prompt_count(&self) -> usize {
        std::fs::read_to_string(self.root.join("requests.jsonl"))
            .unwrap()
            .lines()
            .filter(|line| line.contains("\"method\": \"session/prompt\""))
            .count()
    }
    fn settings(&self) {
        self.until(|frame| matches!(frame, ChatFrame::Settings { .. }));
        chat::chat_set_mode(&self.id, "bold").unwrap();
        chat::chat_set_option(&self.id, "model", &serde_json::json!("two")).unwrap();
        self.until(|frame|matches!(frame,ChatFrame::Settings{settings} if settings["modes"]["currentModeId"]=="bold" && settings["options"][0]["currentValue"]=="two"));
    }
    fn finish_prompt(&self) {
        let ChatFrame::Ask { ask, options, .. } =
            self.until(|frame| matches!(frame, ChatFrame::Ask { .. }))
        else {
            unreachable!()
        };
        chat::chat_answer(&self.id, ask, Some(options[0].0.clone())).unwrap();
        self.until(|frame| matches!(frame,ChatFrame::Turn{stop} if stop=="EndTurn"));
    }
}
impl Fixture {
    fn finish(&mut self) -> Result<(), String> {
        if self.finished {
            return Ok(());
        }
        self.finished = true;
        self.stop.store(true, Ordering::Relaxed);
        let _ = self.release.send(());
        if self.control.is_none() {
            let _ = chat::chat_leave(&self.id);
        }
        for stream in self.streams.lock().unwrap().iter() {
            let _ = stream.shutdown(Shutdown::Both);
        }
        let close = match khor_node::gui_host::connect_at(&self.addr, &self.cookie) {
            Ok(mut attached) => {
                let _ = attached
                    .stream
                    .set_write_timeout(Some(Duration::from_secs(2)));
                write_frame(&mut attached.stream, &GuiOp::Close)
            }
            Err(error) => Err(error),
        };
        let close_error = close.err();
        let mut error = None;
        let until = Instant::now() + Duration::from_secs(10);
        while (self.relay.as_ref().is_some_and(|h| !h.is_finished())
            || self.host.as_ref().is_some_and(|h| !h.is_finished()))
            && Instant::now() < until
        {
            std::thread::sleep(Duration::from_millis(10));
        }
        if let Some(relay) = self.relay.take() {
            if !relay.is_finished() {
                error.get_or_insert("upload relay cleanup is unconfirmed".into());
            } else if relay.join().is_err() {
                error.get_or_insert("upload relay thread panicked".into());
            }
        }
        if let Some(host) = self.host.take() {
            if !host.is_finished() {
                error.get_or_insert("upload host cleanup is unconfirmed".into());
            } else {
                match host.join() {
                    Ok(Ok(())) => (),
                    Ok(Err(why)) => {
                        error.get_or_insert(why);
                    }
                    Err(_) => {
                        error.get_or_insert("upload host thread panicked".into());
                    }
                }
            }
        }
        if self.control.is_none() && error.is_none() {
            let _ = std::fs::remove_dir_all(&self.root);
        }
        match error {
            Some(why) => Err(match close_error {
                Some(close) => format!("{why}; close request failed: {close}"),
                None => why,
            }),
            None => Ok(()),
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if let Err(why) = self.finish() {
            if std::thread::panicking() {
                eprintln!("upload fixture cleanup failed: {why}");
            } else {
                panic!("upload fixture cleanup failed: {why}");
            }
        }
    }
}

fn private_file(path: &std::path::Path) -> std::fs::File {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path).unwrap()
}

fn publish_private(path: &std::path::Path, value: &serde_json::Value) {
    let pending = path.with_extension("pending");
    let mut file = private_file(&pending);
    serde_json::to_writer(&mut file, value).unwrap();
    file.write_all(b"\n").unwrap();
    file.sync_all().unwrap();
    std::fs::rename(pending, path).unwrap();
}

fn app_event(control: &Option<PathBuf>, mut value: serde_json::Value) {
    if let Some(dir) = control {
        static LOG: Mutex<()> = Mutex::new(());
        let _log = LOG.lock().unwrap();
        value["unix_ms"] = serde_json::json!(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        );
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(dir.join("wire.jsonl"))
            .unwrap();
        serde_json::to_writer(&mut file, &value).unwrap();
        file.write_all(b"\n").unwrap();
        file.sync_all().unwrap();
    }
}

fn op_event(connection: usize, op: &GuiOp) -> serde_json::Value {
    let (kind, bytes) = match op {
        GuiOp::Part(bytes) => ("Part", bytes.len()),
        GuiOp::Parts => ("Parts", 0),
        GuiOp::Stop => ("Stop", 0),
        GuiOp::Say(text) => ("Say", text.len()),
        GuiOp::SayWith {
            text,
            images,
            files,
        } => (
            "SayWith",
            text.len()
                + images.iter().map(|(_, data)| data.len()).sum::<usize>()
                + files.iter().map(|(_, text)| text.len()).sum::<usize>(),
        ),
        GuiOp::Answer { .. } => ("Answer", 0),
        GuiOp::Close => ("Close", 0),
        GuiOp::Replay => ("Replay", 0),
        GuiOp::SetMode(_) => ("SetMode", 0),
        GuiOp::SetOption { .. } => ("SetOption", 0),
        GuiOp::Delegate { .. } => ("Delegate", 0),
        GuiOp::ElicitAnswer { .. } => ("ElicitAnswer", 0),
        GuiOp::Control { .. } => ("Control", 0),
    };
    let frame_bytes = 4 + khor_node::proto::encode(op).unwrap().len();
    serde_json::json!({"event":"forwarded", "connection":connection, "kind":kind,
        "payload_bytes":bytes, "frame_bytes":frame_bytes})
}

/// Test driver helper only: no business assertions and never counted as a pass.
/// KHOR_UPLOAD_APP_DIR must name a new private directory. The driver creates
/// release after clicking Stop, then stop after collecting its actual UI/wire
/// evidence. Timeout is a setup failure; it cannot count as cancellation green.
#[test]
#[ignore = "bounded resident fixture for the final real App upload acceptance"]
fn app_upload_host() {
    let began = Instant::now();
    let control =
        PathBuf::from(std::env::var_os("KHOR_UPLOAD_APP_DIR").expect("KHOR_UPLOAD_APP_DIR"));
    assert!(
        control.is_absolute(),
        "use an absolute private control directory"
    );
    assert!(
        !control.to_str().unwrap().chars().any(char::is_whitespace),
        "the fixture command requires a control path without whitespace"
    );
    let mut builder = std::fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder
        .create(&control)
        .expect("control directory must not already exist");
    drop(private_file(&control.join("wire.jsonl")));
    let mut fixture = Fixture::start(Some(control.clone()));
    let node = Node::open(fixture.root.clone()).unwrap();
    let sid = SessionId(fixture.id.clone());
    assert!(node.far_machine(&sid).unwrap().is_none());
    let session_dir = node.session_dir(&sid).unwrap();
    publish_private(
        &control.join("ready.json"),
        &serde_json::json!({
            "pid":std::process::id(), "root":fixture.root, "session":fixture.id,
            "session_dir":session_dir, "host_file":host::host_file_path(&session_dir),
            "upstream_addr":fixture.addr,
            "relay_addr":format!("127.0.0.1:{}", host::read_host_file(&session_dir).unwrap().port),
            "first_part":control.join("first-part.json"), "release":control.join("release"),
            "stop":control.join("stop"), "wire_log":control.join("wire.jsonl"),
            "acp_log":fixture.root.join("requests.jsonl"), "limit_seconds":300,
            "scope":"real local GUI host and deterministic Python ACP; App must attach itself"
        }),
    );
    while !control.join("stop").is_file() && began.elapsed() < Duration::from_secs(270) {
        std::thread::sleep(Duration::from_millis(20));
    }
    let requested = control.join("stop").is_file();
    fixture.finish().expect("owned host/relay cleanup");
    publish_private(
        &control.join("finished.json"),
        &serde_json::json!({
            "stop_requested":requested, "connections":fixture.accepted.load(Ordering::Relaxed),
            "elapsed_ms":began.elapsed().as_millis()
        }),
    );
    assert!(
        requested,
        "resident fixture reached its deadline without stop"
    );
    assert!(
        began.elapsed() <= Duration::from_secs(300),
        "fixture cleanup exceeded total deadline"
    );
}

fn cancel_upload() {
    let fixture = Fixture::new();
    fixture.settings();
    let turn = chat::submit_say_with_token(
        &fixture.id,
        "cancelled upload".into(),
        vec![("image/png".into(), "x".repeat(16 * 1024 * 1024))],
        vec![],
        Some("draft-1".into()),
    )
    .unwrap();
    fixture
        .first_part
        .recv_timeout(Duration::from_secs(5))
        .unwrap();
    let stop = chat::submit_control(&fixture.id, Control::Stop).unwrap();
    fixture.release.send(()).unwrap();
    stop.wait().unwrap();
    assert!(turn.wait().is_err());
    fixture.until(|frame|matches!(frame,ChatFrame::SendCancelled{token:Some(token),reconnect} if token=="draft-1" && !*reconnect));
    let batch = chat::chat_poll(&fixture.id, 0).unwrap();
    assert!(!batch.gone);
    assert!(
        !batch
            .frames
            .iter()
            .any(|frame| matches!(frame, ChatFrame::SendCommitted { .. })),
        "cancelled upload cannot report a successful write commit"
    );
    assert!(
        !batch
            .frames
            .iter()
            .any(|frame| matches!(frame, ChatFrame::Unsent { .. }))
    );
    assert_eq!(fixture.prompt_count(), 0);
    let next = chat::submit_say_with_token(
        &fixture.id,
        "fresh turn".into(),
        vec![],
        vec![],
        Some("draft-2".into()),
    )
    .unwrap();
    next.wait().unwrap();
    fixture.finish_prompt();
    assert_eq!(fixture.prompt_count(), 1);
    assert_eq!(
        fixture.accepted.load(Ordering::Relaxed),
        1,
        "cancelling an upload must keep the original connection"
    );
}
#[test]
fn cancelling_an_upload_keeps_the_same_connection_for_the_next_turn() {
    cancel_upload()
}

#[test]
fn residual_upload_parts_cannot_revive_a_stopped_turn_on_the_same_socket() {
    let fixture = Fixture::new();
    let mut client = khor_node::gui_host::connect_at(&fixture.addr, &fixture.cookie)
        .unwrap()
        .stream;
    client
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    for split in [true, false] {
        let bytes =
            khor_node::proto::encode(&GuiOp::Say("cancelled residual upload".into())).unwrap();
        let at = if split { bytes.len() / 2 } else { bytes.len() };
        write_frame(&mut client, &GuiOp::Part(bytes[..at].to_vec().into())).unwrap();
        write_frame(&mut client, &GuiOp::Stop).unwrap();
        if split {
            write_frame(&mut client, &GuiOp::Part(bytes[at..].to_vec().into())).unwrap();
        }
        write_frame(&mut client, &GuiOp::Parts).unwrap();
        loop {
            if matches!(read_frame::<khor_node::gui_host::GuiNote>(&mut client).unwrap(), khor_node::gui_host::GuiNote::Turn(stop) if stop=="Cancelled")
            {
                break;
            }
        }
        write_frame(&mut client, &GuiOp::Say("fresh turn".into())).unwrap();
        loop {
            if let khor_node::gui_host::GuiNote::Ask { ask, options, .. } =
                read_frame(&mut client).unwrap()
            {
                write_frame(
                    &mut client,
                    &GuiOp::Answer {
                        ask,
                        option: Some(options[0].0.clone()),
                    },
                )
                .unwrap();
                break;
            }
        }
        loop {
            if matches!(read_frame::<khor_node::gui_host::GuiNote>(&mut client).unwrap(), khor_node::gui_host::GuiNote::Turn(stop) if stop=="EndTurn")
            {
                break;
            }
        }
    }
    assert_eq!(
        fixture.prompt_count(),
        2,
        "cancelled residual uploads must not reach ACP"
    );
}

#[test]
fn a_committed_small_turn_uses_agent_stop_and_keeps_the_attachment() {
    let fixture = Fixture::new();
    chat::submit_say_with_token(
        &fixture.id,
        "already committed".into(),
        vec![],
        vec![],
        Some("committed".into()),
    )
    .unwrap()
    .wait()
    .unwrap();
    fixture.until(|frame| matches!(frame, ChatFrame::Ask { .. }));
    chat::chat_stop(&fixture.id).unwrap();
    fixture.until(|frame| matches!(frame, ChatFrame::Turn { stop } if stop == "Cancelled"));
    let batch = chat::chat_poll(&fixture.id, 0).unwrap();
    assert!(!batch.gone);
    assert!(
        !batch
            .frames
            .iter()
            .any(|frame| matches!(frame, ChatFrame::SendCancelled { .. }))
    );
    assert_eq!(fixture.prompt_count(), 1);
    assert_eq!(fixture.accepted.load(Ordering::Relaxed), 1);
}
