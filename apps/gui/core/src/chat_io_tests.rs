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
            sends: Default::default(),
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
        let current = chats()
            .lock()
            .unwrap()
            .get(&self.id)
            .is_some_and(|chat| Arc::ptr_eq(chat, &self.chat));
        if current {
            let _ = chat_leave(&self.id);
        }
        self.chat.commands.close();
        self.chat.turns.close();
        self.chat.conn.close();
    }
}

#[test]
fn closing_a_queued_stop_unblocks_the_upload_and_cannot_stop_a_reopened_attachment() {
    let attached = Attached::new();
    let turn = submit_say_with_token(
        &attached.id,
        "x".repeat(16 * 1024 * 1024),
        vec![],
        vec![],
        Some("retired-upload".into()),
    )
    .unwrap();
    let until = Instant::now() + Duration::from_secs(3);
    while attached.chat.sending.0.load(Ordering::Relaxed) == 0 {
        assert!(Instant::now() < until);
        std::thread::yield_now();
    }
    let (release, blocked) = mpsc::channel();
    let (started, at_blocker) = mpsc::channel();
    let blocker = attached
        .chat
        .commands
        .submit(move || {
            started.send(()).unwrap();
            blocked.recv().map_err(|e| e.to_string())
        })
        .unwrap();
    at_blocker.recv_timeout(Duration::from_secs(2)).unwrap();
    let stop = submit_control(&attached.id, Control::Stop).unwrap();
    chat_leave(&attached.id).unwrap();
    assert!(stop.wait().is_err());
    let (finished, result) = mpsc::channel();
    std::thread::spawn(move || finished.send(turn.wait()).unwrap());
    assert!(
        result
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .is_err()
    );
    assert!(attached.chat.frames.lock().unwrap().held.iter().any(|frame| matches!(frame, ChatFrame::SendCancelled { token: Some(token), .. } if token == "retired-upload")));
    let mut fresh = Attached::new();
    {
        let mut registry = chats().lock().unwrap();
        registry.remove(&fresh.id);
        fresh.id = attached.id.clone();
        registry.insert(fresh.id.clone(), fresh.chat.clone());
    }
    release.send(()).unwrap();
    blocker.wait().unwrap();
    let say = submit_say(&fresh.id, "fresh attachment".into(), vec![], vec![]).unwrap();
    assert!(
        matches!(read_frame::<GuiOp>(&mut fresh.peer).unwrap(), GuiOp::SayWith { text, .. } if text == "fresh attachment")
    );
    say.wait().unwrap();
}

#[test]
fn a_write_error_is_correlated_before_the_first_poll() {
    let attached = Attached::new();
    attached.chat.conn.close();
    assert!(
        submit_say_with_token(
            &attached.id,
            "same words".into(),
            vec![],
            vec![],
            Some("failed-draft".into())
        )
        .unwrap()
        .wait()
        .is_err()
    );
    let batch = chat_poll(&attached.id, 0).unwrap();
    assert!(
        matches!(batch.frames.as_slice(), [ChatFrame::Unsent { token: Some(token), .. }] if token == "failed-draft")
    );
}

#[test]
fn queued_close_failure_keeps_its_original_token_across_same_text_resubmission() {
    let attached = Attached::new();
    let (release, blocked) = mpsc::channel();
    let (started, at_blocker) = mpsc::channel();
    let blocker = attached
        .chat
        .commands
        .submit(move || {
            started.send(()).unwrap();
            blocked.recv().map_err(|e| e.to_string())
        })
        .unwrap();
    at_blocker.recv_timeout(Duration::from_secs(2)).unwrap();
    let old = submit_say_with_token(
        &attached.id,
        "same words".into(),
        vec![],
        vec![],
        Some("old-draft".into()),
    )
    .unwrap();
    chat_leave(&attached.id).unwrap();
    assert!(old.wait().is_err());
    let mut fresh = Attached::new();
    {
        let mut registry = chats().lock().unwrap();
        registry.remove(&fresh.id);
        fresh.id = attached.id.clone();
        registry.insert(fresh.id.clone(), fresh.chat.clone());
    }
    let new = submit_say_with_token(
        &fresh.id,
        "same words".into(),
        vec![],
        vec![],
        Some("new-draft".into()),
    )
    .unwrap();
    assert!(
        matches!(read_frame::<GuiOp>(&mut fresh.peer).unwrap(), GuiOp::SayWith { text, .. } if text == "same words")
    );
    new.wait().unwrap();
    assert!(
        matches!(attached.chat.frames.lock().unwrap().held.as_slice(), [ChatFrame::Unsent { token: Some(token), .. }] if token == "old-draft")
    );
    assert!(
        matches!(fresh.chat.frames.lock().unwrap().held.as_slice(), [ChatFrame::SendCommitted { token }] if token == "new-draft")
    );
    release.send(()).unwrap();
    blocker.wait().unwrap();
}

#[test]
fn a_stop_admitted_before_the_first_frame_cancels_the_already_admitted_turn() {
    let attached = Attached::new();
    let (release, blocked) = mpsc::channel();
    let blocker = attached
        .chat
        .turns
        .submit(move || blocked.recv().map_err(|e| e.to_string()))
        .unwrap();
    let turn = submit_say_with_token(
        &attached.id,
        "must-not-send".into(),
        vec![],
        vec![],
        Some("draft-1".into()),
    )
    .unwrap();
    submit_control(&attached.id, Control::Stop)
        .unwrap()
        .wait()
        .unwrap();
    release.send(()).unwrap();
    blocker.wait().unwrap();
    assert!(
        turn.wait().is_err(),
        "Stop did not cancel the turn synchronously admitted before it"
    );
    let frames = &attached.chat.frames.lock().unwrap().held;
    assert!(
        matches!(frames.as_slice(), [ChatFrame::SendCancelled { token: Some(token), reconnect: false }] if token == "draft-1")
    );
    assert!(!attached.chat.gone.load(Ordering::Relaxed));
}

#[test]
fn send_tokens_are_optional_bounded_and_never_encoded_on_the_host_wire() {
    let mut attached = Attached::new();
    for token in ["", "x y", "秘密", &"x".repeat(129)] {
        assert!(
            submit_say_with_token(
                &attached.id,
                "hi".into(),
                vec![],
                vec![],
                Some(token.into())
            )
            .is_err()
        );
    }
    for token in [None, Some("draft-2_A.b:c".into()), Some("x".repeat(128))] {
        let expected = token.clone();
        let pending =
            submit_say_with_token(&attached.id, "hi".into(), vec![], vec![], token).unwrap();
        assert!(
            matches!(read_frame::<GuiOp>(&mut attached.peer).unwrap(), GuiOp::SayWith { text, images, files } if text == "hi" && images.is_empty() && files.is_empty())
        );
        pending.wait().unwrap();
        let frames = &attached.chat.frames.lock().unwrap().held;
        if let Some(expected) = expected {
            assert!(
                matches!(frames.last(), Some(ChatFrame::SendCommitted { token }) if token == &expected)
            );
        } else {
            assert!(
                frames.is_empty(),
                "legacy untagged success needs no uncorrelated receipt"
            );
        }
    }
}

#[test]
fn http_admits_the_token_before_returning_and_rejects_invalid_tokens() {
    let attached = Attached::new();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let root = std::path::Path::new(".");
    for token in [
        serde_json::json!(3),
        serde_json::json!(""),
        serde_json::json!("x".repeat(129)),
    ] {
        assert!(
            crate::api::answer(
                &runtime,
                root,
                "chat_say",
                &serde_json::json!({"id": attached.id, "text": "hi", "token": token})
            )
            .is_err()
        );
    }
    let (release, blocked) = mpsc::channel();
    let blocker = attached
        .chat
        .turns
        .submit(move || blocked.recv().map_err(|e| e.to_string()))
        .unwrap();
    assert_eq!(
        crate::api::answer(
            &runtime,
            root,
            "chat_say",
            &serde_json::json!({"id":attached.id,"text":"hi","token":"http-draft"})
        )
        .unwrap(),
        "null"
    );
    submit_control(&attached.id, Control::Stop)
        .unwrap()
        .wait()
        .unwrap();
    release.send(()).unwrap();
    blocker.wait().unwrap();
    assert!(
        matches!(attached.chat.frames.lock().unwrap().held.as_slice(), [ChatFrame::SendCancelled{token:Some(token),reconnect:false}] if token=="http-draft")
    );
}

#[test]
fn http_returns_at_admission_and_commits_only_after_the_complete_input_is_written() {
    let mut attached = Attached::new();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let (release, blocked) = mpsc::channel();
    let blocker = attached
        .chat
        .turns
        .submit(move || blocked.recv().map_err(|e| e.to_string()))
        .unwrap();
    assert_eq!(
        crate::api::answer(
            &runtime,
            std::path::Path::new("."),
            "chat_say",
            &serde_json::json!({"id":attached.id,"text":"commit me","token":"http-commit"})
        )
        .unwrap(),
        "null"
    );
    assert!(
        chat_poll(&attached.id, 0).unwrap().frames.is_empty(),
        "HTTP admission cannot report a write commit"
    );
    release.send(()).unwrap();
    blocker.wait().unwrap();
    assert!(
        matches!(read_frame::<GuiOp>(&mut attached.peer).unwrap(), GuiOp::SayWith { text, .. } if text == "commit me")
    );
    let until = Instant::now() + Duration::from_secs(2);
    loop {
        let batch = chat_poll(&attached.id, 0).unwrap();
        if !batch.frames.is_empty() {
            assert!(
                matches!(batch.frames.as_slice(), [ChatFrame::SendCommitted { token }] if token == "http-commit")
            );
            assert_eq!(
                serde_json::to_value(&batch.frames).unwrap(),
                serde_json::json!([{"kind":"send_committed","token":"http-commit"}])
            );
            break;
        }
        assert!(
            Instant::now() < until,
            "successful write had no commit receipt"
        );
        std::thread::yield_now();
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
    assert!(
        turn.wait().is_err(),
        "cancelled upload reported successful submission"
    );
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
