use super::*;
use std::{
    net::TcpListener,
    sync::mpsc,
    time::{Duration, Instant},
};

struct Attached {
    id: String,
    chat: Arc<Chat>,
    peer: TcpStream,
}

impl Attached {
    fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let conn = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        conn.set_write_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let (peer, _) = listener.accept().unwrap();
        peer.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
        let chat = Arc::new(Chat {
            conn: crate::connection::Connection::new(conn).unwrap(),
            commands: Default::default(),
            turns: Default::default(),
            frames: Mutex::new(Frames::default()),
            gone: AtomicBool::new(false),
            stall: Mutex::new(None),
            far: None,
            arrived: tokio::sync::Notify::new(),
            sending: (AtomicU64::new(0), AtomicU64::new(0)),
            holds: AtomicUsize::new(1),
            left: AtomicBool::new(false),
        });
        let id = format!("chat-io-{}", khor_node::link::fresh_hex().unwrap());
        assert!(
            chats()
                .lock()
                .unwrap()
                .insert(id.clone(), chat.clone())
                .is_none()
        );
        Self { id, chat, peer }
    }
}

impl Drop for Attached {
    fn drop(&mut self) {
        let _ = chat_leave(&self.id);
        self.chat.commands.close();
        self.chat.turns.close();
        self.chat.conn.close();
    }
}

#[test]
fn controls_keep_fifo_order_and_complete_only_after_their_frames_are_written() {
    let mut attached = Attached::new();
    let mode = submit_control(&attached.id, Control::SetMode("plan".into())).unwrap();
    let stop = submit_control(&attached.id, Control::Stop).unwrap();
    let replay = submit_control(&attached.id, Control::Replay).unwrap();
    assert!(
        matches!(read_frame::<GuiOp>(&mut attached.peer).unwrap(), GuiOp::SetMode(mode) if mode == "plan")
    );
    assert!(matches!(
        read_frame::<GuiOp>(&mut attached.peer).unwrap(),
        GuiOp::Stop
    ));
    assert!(matches!(
        read_frame::<GuiOp>(&mut attached.peer).unwrap(),
        GuiOp::Replay
    ));
    mode.wait().unwrap();
    stop.wait().unwrap();
    replay.wait().unwrap();
}

#[test]
fn a_turn_waits_for_previously_admitted_mode_and_option() {
    let mut attached = Attached::new();
    attached
        .peer
        .set_read_timeout(Some(Duration::from_millis(100)))
        .unwrap();
    let (started, started_rx) = mpsc::channel();
    let (release, release_rx) = mpsc::channel();
    let blocked = attached
        .chat
        .commands
        .submit(move || {
            started.send(()).unwrap();
            release_rx
                .recv_timeout(Duration::from_secs(3))
                .map_err(|e| e.to_string())
        })
        .unwrap();
    started_rx.recv_timeout(Duration::from_secs(3)).unwrap();
    let mode = submit_control(&attached.id, Control::SetMode("plan".into())).unwrap();
    let option = submit_control(
        &attached.id,
        Control::SetOption {
            option: "model".into(),
            value: serde_json::json!("reasoner"),
        },
    )
    .unwrap();
    let turn = submit_say(&attached.id, "use those settings".into(), vec![], vec![]).unwrap();
    let premature = read_frame::<GuiOp>(&mut attached.peer);
    release.send(()).unwrap();
    assert!(
        premature.is_err(),
        "turn passed controls still waiting in their FIFO"
    );
    attached
        .peer
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    assert!(
        matches!(read_frame::<GuiOp>(&mut attached.peer).unwrap(), GuiOp::SetMode(mode) if mode == "plan")
    );
    assert!(
        matches!(read_frame::<GuiOp>(&mut attached.peer).unwrap(), GuiOp::SetOption { id, value: Choice::Value(value) } if id == "model" && value == "reasoner")
    );
    assert!(
        matches!(read_frame::<GuiOp>(&mut attached.peer).unwrap(), GuiOp::SayWith { text, .. } if text == "use those settings")
    );
    blocked.wait().unwrap();
    mode.wait().unwrap();
    option.wait().unwrap();
    turn.wait().unwrap();
}

#[test]
fn stop_can_pass_a_large_turn_before_its_final_parts_frame() {
    let mut attached = Attached::new();
    let turn = submit_say(&attached.id, "x".repeat(16 * 1024 * 1024), vec![], vec![]).unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    while attached.chat.sending.0.load(Ordering::Relaxed) == 0 {
        assert!(Instant::now() < deadline, "turn never started writing");
        std::thread::sleep(Duration::from_millis(5));
    }
    // The unread multi-megabyte turn fills the socket before Stop arrives.
    std::thread::sleep(Duration::from_millis(100));
    let stop = submit_control(&attached.id, Control::Stop).unwrap();
    let mut stopped = false;
    let mut slices = 0;
    loop {
        match read_frame::<GuiOp>(&mut attached.peer).unwrap() {
            GuiOp::Part(_) => slices += 1,
            GuiOp::Stop => stopped = true,
            GuiOp::Parts => break,
            _ => panic!("unexpected operation in the sliced turn"),
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(slices > 1);
    assert!(stopped, "Stop waited behind the entire large turn");
    stop.wait().unwrap();
    turn.wait().unwrap();
}

#[test]
fn leaving_rejects_a_turn_waiting_for_preceding_controls() {
    let attached = Attached::new();
    let (started, started_rx) = mpsc::channel();
    let (release, release_rx) = mpsc::channel();
    let blocked = attached
        .chat
        .commands
        .submit(move || {
            started.send(()).unwrap();
            release_rx
                .recv_timeout(Duration::from_secs(3))
                .map_err(|e| e.to_string())
        })
        .unwrap();
    started_rx.recv_timeout(Duration::from_secs(3)).unwrap();
    let mode = submit_control(&attached.id, Control::SetMode("plan".into())).unwrap();
    let turn = submit_say(&attached.id, "must not send".into(), vec![], vec![]).unwrap();
    chat_leave(&attached.id).unwrap();
    let (finished, result) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = finished.send(turn.wait());
    });
    assert!(
        result
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .is_err()
    );
    assert!(mode.wait().is_err());
    release.send(()).unwrap();
    blocked.wait().unwrap();
}

#[test]
fn leaving_the_last_holder_interrupts_a_turn_blocked_on_the_real_socket() {
    let attached = Attached::new();
    let turn = submit_say(&attached.id, "x".repeat(16 * 1024 * 1024), vec![], vec![]).unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    while attached.chat.sending.0.load(Ordering::Relaxed) == 0 {
        assert!(Instant::now() < deadline, "turn never started writing");
        std::thread::sleep(Duration::from_millis(5));
    }
    let (finished, result) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = finished.send(turn.wait());
    });
    assert!(
        result.recv_timeout(Duration::from_millis(100)).is_err(),
        "fixture peer does not consume the large turn"
    );
    chat_leave(&attached.id).unwrap();
    assert!(
        result
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .is_err()
    );
    assert!(!chats().lock().unwrap().contains_key(&attached.id));
}
