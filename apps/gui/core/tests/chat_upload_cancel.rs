use khor_gui_core::chat::{self, ChatFrame, Control};
use khor_node::{
    Node, SessionId,
    gui_host::{GuiOp, gui_host_main},
    host::{self, Hello, Welcome, read_frame, write_frame},
};
use std::{
    io,
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
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "khor-upload-{}",
            khor_node::link::fresh_hex().unwrap()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let ready = root.join("ready");
        let command = vec![
            "python3".into(),
            format!(
                "{}/tests/fixtures/upload-agent.py",
                env!("CARGO_MANIFEST_DIR")
            ),
            root.join("requests.jsonl").to_str().unwrap().into(),
        ];
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
        std::thread::spawn(move || {
            while !exiting.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((mut client, _)) => {
                        client.set_nonblocking(false).unwrap();
                        count.fetch_add(1, Ordering::Relaxed);
                        let upstream = upstream.clone();
                        let gate = gate.clone();
                        std::thread::spawn(move || {
                            let _ = client.set_read_timeout(Some(Duration::from_secs(10)));
                            let Ok(hello) = read_frame::<Hello>(&mut client) else {
                                return;
                            };
                            let mut server = TcpStream::connect(upstream).unwrap();
                            write_frame(&mut server, &hello).unwrap();
                            let welcome: Welcome = read_frame(&mut server).unwrap();
                            write_frame(&mut client, &welcome).unwrap();
                            let mut notes = server.try_clone().unwrap();
                            let mut back = client.try_clone().unwrap();
                            std::thread::spawn(move || {
                                let _ = io::copy(&mut notes, &mut back);
                                let _ = back.shutdown(Shutdown::Both);
                            });
                            while let Ok(op) = read_frame::<GuiOp>(&mut client) {
                                let part = matches!(op, GuiOp::Part(_));
                                if write_frame(&mut server, &op).is_err() {
                                    break;
                                }
                                if part {
                                    if let Some((at, wait)) = gate.lock().unwrap().take() {
                                        at.send(()).unwrap();
                                        let _ = wait.recv_timeout(Duration::from_secs(5));
                                    }
                                }
                            }
                            let _ = server.shutdown(Shutdown::Both);
                            let _ = client.shutdown(Shutdown::Both);
                        });
                    }
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(5))
                    }
                    Err(_) => break,
                }
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
        };
        assert!(chat::chat_open(&fixture.root, &fixture.id).unwrap());
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
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self.release.send(());
        let _ = chat::chat_leave(&self.id);
        if let Some(host) = self.host.take() {
            if let Ok(mut attached) = khor_node::gui_host::connect_at(&self.addr, &self.cookie) {
                let _ = write_frame(&mut attached.stream, &GuiOp::Close);
            }
            let result = host.join();
            if std::thread::panicking() {
                eprintln!("host result: {result:?}");
                eprintln!(
                    "ACP requests: {}",
                    std::fs::read_to_string(self.root.join("requests.jsonl")).unwrap_or_default()
                );
            }
        }
        self.stop.store(true, Ordering::Relaxed);
        let _ = std::fs::remove_dir_all(&self.root);
    }
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
