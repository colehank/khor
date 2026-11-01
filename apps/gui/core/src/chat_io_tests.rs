use super::*;
use std::{
    net::TcpListener,
    sync::mpsc,
    time::{Duration, Instant},
};

struct Attached {
    id: String,
    lease: String,
    chat: Arc<Chat>,
    peer: TcpStream,
}

impl Attached {
    fn new() -> Self {
        Self::with_id(None)
    }
    fn with_id(id: Option<String>) -> Self {
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
            left: AtomicBool::new(false),
        });
        let id = id.unwrap_or_else(|| format!("chat-io-{}", khor_node::link::fresh_hex().unwrap()));
        let lease = chats().begin(Path::new("."), &id).unwrap();
        let opening = chats().claim(Path::new("."), &id, &lease).unwrap();
        let (chat, _) = opening.publish(chat).unwrap();
        drop(opening);
        Self {
            id,
            lease,
            chat,
            peer,
        }
    }
}

impl Drop for Attached {
    fn drop(&mut self) {
        let _ = chat_leave(Path::new("."), &self.id, &self.lease);
    }
}

#[test]
fn closing_a_queued_stop_unblocks_the_upload_and_cannot_stop_a_reopened_attachment() {
    let attached = Attached::new();
    let turn = submit_say_with_token(
        Path::new("."),
        &attached.id,
        &attached.lease,
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
    let stop =
        submit_control(Path::new("."), &attached.id, &attached.lease, Control::Stop).unwrap();
    chat_leave(Path::new("."), &attached.id, &attached.lease).unwrap();
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
    let mut fresh = Attached::with_id(Some(attached.id.clone()));
    release.send(()).unwrap();
    blocker.wait().unwrap();
    let say = submit_say(
        Path::new("."),
        &fresh.id,
        &fresh.lease,
        "fresh attachment".into(),
        vec![],
        vec![],
    )
    .unwrap();
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
            Path::new("."),
            &attached.id,
            &attached.lease,
            "same words".into(),
            vec![],
            vec![],
            Some("failed-draft".into())
        )
        .unwrap()
        .wait()
        .is_err()
    );
    let batch = chat_poll(Path::new("."), &attached.id, &attached.lease, 0).unwrap();
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
        Path::new("."),
        &attached.id,
        &attached.lease,
        "same words".into(),
        vec![],
        vec![],
        Some("old-draft".into()),
    )
    .unwrap();
    chat_leave(Path::new("."), &attached.id, &attached.lease).unwrap();
    assert!(old.wait().is_err());
    let mut fresh = Attached::with_id(Some(attached.id.clone()));
    let new = submit_say_with_token(
        Path::new("."),
        &fresh.id,
        &fresh.lease,
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
        Path::new("."),
        &attached.id,
        &attached.lease,
        "must-not-send".into(),
        vec![],
        vec![],
        Some("draft-1".into()),
    )
    .unwrap();
    submit_control(Path::new("."), &attached.id, &attached.lease, Control::Stop)
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
                Path::new("."),
                &attached.id,
                &attached.lease,
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
        let pending = submit_say_with_token(
            Path::new("."),
            &attached.id,
            &attached.lease,
            "hi".into(),
            vec![],
            vec![],
            token,
        )
        .unwrap();
        assert!(
            matches!(read_frame::<GuiOp>(&mut attached.peer).unwrap(), GuiOp::SayWith { text, images, files, token } if text == "hi" && images.is_empty() && files.is_empty() && token == expected)
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
fn capture_tokens_are_view_specific_and_an_old_lease_cannot_capture_the_replacement() {
    let mut attached = Attached::new();
    let b = begin_view(Path::new("."), &attached.id).unwrap();
    open_chat_with(Path::new("."), &attached.id, &b, || {
        panic!("second view must share the current socket")
    })
    .unwrap();
    let a_token = "00112233445566778899aabbccddeeff";
    let b_token = "ffeeddccbbaa99887766554433221100";
    for (lease, token) in [(&attached.lease, a_token), (&b, b_token)] {
        chat_capture(Path::new("."), &attached.id, lease, token).unwrap();
        assert!(
            matches!(read_frame::<GuiOp>(&mut attached.peer).unwrap(),GuiOp::Capture { token:actual } if actual==capture_token(token).unwrap())
        );
    }
    chat_leave(Path::new("."), &attached.id, &attached.lease).unwrap();
    assert!(chat_capture(Path::new("."), &attached.id, &attached.lease, a_token).is_err());
    chat_capture(Path::new("."), &attached.id, &b, b_token).unwrap();
    assert!(
        matches!(read_frame::<GuiOp>(&mut attached.peer).unwrap(),GuiOp::Capture { token } if token==capture_token(b_token).unwrap())
    );
    chat_leave(Path::new("."), &attached.id, &b).unwrap();
}

#[test]
fn capture_failures_are_typed_and_live_body_updates_are_only_bounded_wake_signals() {
    for bad in [
        "",
        "short",
        "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
        "秘密秘密秘密秘密秘密",
    ] {
        assert!(capture_token(bad).is_err());
    }
    for (failure, expected) in [
        (khor_node::gui_host::CaptureFailure::NotReady, "not_ready"),
        (
            khor_node::gui_host::CaptureFailure::Unavailable,
            "unavailable",
        ),
    ] {
        let frame = frame_of(GuiNote::Captured {
            token: [3; 16],
            result: Err(failure),
        });
        let value = serde_json::to_value(frame).unwrap();
        assert_eq!(value["token"], "03".repeat(16));
        assert_eq!(value["result"]["reason"], expected);
    }
    let frame=frame_of(GuiNote::Note(serde_json::json!({"update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"x".repeat(1024*1024)}}}).to_string()));
    assert!(matches!(frame, ChatFrame::ContentChanged));
    assert!(serde_json::to_vec(&frame).unwrap().len() < 64);
    assert!(
        matches!(frame_of(GuiNote::Note(serde_json::json!({"update":{"sessionUpdate":"current_mode_update","currentModeId":"plan"}}).to_string())),ChatFrame::Note {update} if update["update"]["currentModeId"]=="plan")
    );
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
                &serde_json::json!({"id": attached.id, "lease": attached.lease, "text": "hi", "token": token})
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
            &serde_json::json!({"id":attached.id,"lease":attached.lease,"text":"hi","token":"http-draft"})
        )
        .unwrap(),
        "null"
    );
    submit_control(Path::new("."), &attached.id, &attached.lease, Control::Stop)
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
            &serde_json::json!({"id":attached.id,"lease":attached.lease,"text":"commit me","token":"http-commit"})
        )
        .unwrap(),
        "null"
    );
    assert!(
        chat_poll(Path::new("."), &attached.id, &attached.lease, 0)
            .unwrap()
            .frames
            .is_empty(),
        "HTTP admission cannot report a write commit"
    );
    release.send(()).unwrap();
    blocker.wait().unwrap();
    assert!(
        matches!(read_frame::<GuiOp>(&mut attached.peer).unwrap(), GuiOp::SayWith { text, .. } if text == "commit me")
    );
    let until = Instant::now() + Duration::from_secs(2);
    loop {
        let batch = chat_poll(Path::new("."), &attached.id, &attached.lease, 0).unwrap();
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
    let mode = submit_control(
        Path::new("."),
        &attached.id,
        &attached.lease,
        Control::SetMode("plan".into()),
    )
    .unwrap();
    let stop =
        submit_control(Path::new("."), &attached.id, &attached.lease, Control::Stop).unwrap();
    let capture = submit_control(
        Path::new("."),
        &attached.id,
        &attached.lease,
        Control::Capture([7; 16]),
    )
    .unwrap();
    assert!(
        matches!(read_frame::<GuiOp>(&mut attached.peer).unwrap(), GuiOp::SetMode(mode) if mode == "plan")
    );
    assert!(matches!(
        read_frame::<GuiOp>(&mut attached.peer).unwrap(),
        GuiOp::Stop
    ));
    assert!(matches!(
        read_frame::<GuiOp>(&mut attached.peer).unwrap(),
        GuiOp::Capture { token } if token == [7; 16]
    ));
    mode.wait().unwrap();
    stop.wait().unwrap();
    capture.wait().unwrap();
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
    let mode = submit_control(
        Path::new("."),
        &attached.id,
        &attached.lease,
        Control::SetMode("plan".into()),
    )
    .unwrap();
    let option = submit_control(
        Path::new("."),
        &attached.id,
        &attached.lease,
        Control::SetOption {
            option: "model".into(),
            value: serde_json::json!("reasoner"),
        },
    )
    .unwrap();
    let turn = submit_say(
        Path::new("."),
        &attached.id,
        &attached.lease,
        "use those settings".into(),
        vec![],
        vec![],
    )
    .unwrap();
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
    let turn = submit_say(
        Path::new("."),
        &attached.id,
        &attached.lease,
        "x".repeat(16 * 1024 * 1024),
        vec![],
        vec![],
    )
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    while attached.chat.sending.0.load(Ordering::Relaxed) == 0 {
        assert!(Instant::now() < deadline, "turn never started writing");
        std::thread::sleep(Duration::from_millis(5));
    }
    // The unread multi-megabyte turn fills the socket before Stop arrives.
    std::thread::sleep(Duration::from_millis(100));
    let stop =
        submit_control(Path::new("."), &attached.id, &attached.lease, Control::Stop).unwrap();
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
    let mode = submit_control(
        Path::new("."),
        &attached.id,
        &attached.lease,
        Control::SetMode("plan".into()),
    )
    .unwrap();
    let turn = submit_say(
        Path::new("."),
        &attached.id,
        &attached.lease,
        "must not send".into(),
        vec![],
        vec![],
    )
    .unwrap();
    chat_leave(Path::new("."), &attached.id, &attached.lease).unwrap();
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
    let turn = submit_say(
        Path::new("."),
        &attached.id,
        &attached.lease,
        "x".repeat(16 * 1024 * 1024),
        vec![],
        vec![],
    )
    .unwrap();
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
    chat_leave(Path::new("."), &attached.id, &attached.lease).unwrap();
    assert!(
        result
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .is_err()
    );
    assert!(
        chats()
            .read(Path::new("."), &attached.id, &attached.lease)
            .is_err()
    );
}

#[test]
fn active_reopen_retains_its_own_connection_when_an_old_open_lands_last() {
    fn pair() -> (TcpStream, TcpStream) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (peer, _) = listener.accept().unwrap();
        (client, peer)
    }
    fn wait_for_marker(id: &str, lease: &str, marker: &str) -> Result<(), String> {
        let until = Instant::now() + Duration::from_secs(2);
        loop {
            let batch = chat_poll(Path::new("."), id, lease, 0)?;
            if batch
                .frames
                .iter()
                .any(|frame| matches!(frame, ChatFrame::Turn {stop} if stop==marker))
            {
                return Ok(());
            }
            if Instant::now() >= until {
                return Err("active GUI connection did not receive its own marker".into());
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    struct Cleanup(String, String);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = chat_leave(Path::new("."), &self.0, &self.1);
        }
    }
    let id = format!("chat-reopen-{}", khor_node::link::fresh_hex().unwrap());
    let old_lease = begin_view(Path::new("."), &id).unwrap();
    let new_lease = begin_view(Path::new("."), &id).unwrap();
    let _cleanup = Cleanup(id.clone(), new_lease.clone());
    let (old_stream, _old_peer) = pair();
    let (new_stream, mut new_peer) = pair();
    let (entered, at_connector) = mpsc::channel();
    let (release, held) = mpsc::channel();
    let (before_old_landed, after_old_landed) = std::thread::scope(|scope| {
        let old_id = id.clone();
        let old_view = old_lease.clone();
        let old = scope.spawn(move || {
            open_chat_with(Path::new("."), &old_id, &old_view, || {
                entered.send(()).map_err(|e| e.to_string())?;
                held.recv_timeout(Duration::from_secs(3))
                    .map_err(|e| e.to_string())?;
                Ok((khor_node::gui_host::Attached { stream: old_stream }, None))
            })
        });
        at_connector.recv_timeout(Duration::from_secs(2)).unwrap();
        chat_leave(Path::new("."), &id, &old_lease).unwrap();
        let new_open = open_chat_with(Path::new("."), &id, &new_lease, || {
            Ok((khor_node::gui_host::Attached { stream: new_stream }, None))
        });
        let before = new_open.and_then(|_| {
            write_frame(&mut new_peer, &GuiNote::Turn("active-B-before-A".into()))?;
            wait_for_marker(&id, &new_lease, "active-B-before-A")
        });
        release.send(()).unwrap();
        let old_result = old.join().unwrap();
        assert!(
            old_result.is_err(),
            "retired A must not acquire a live view"
        );
        let after = (|| {
            write_frame(&mut new_peer, &GuiNote::Turn("active-B-after-A".into()))?;
            wait_for_marker(&id, &new_lease, "active-B-after-A")
        })();
        (before, after)
    });
    assert!(
        before_old_landed.is_ok(),
        "active B was discarded by A's early leave: {before_old_landed:?}"
    );
    assert!(
        after_old_landed.is_ok(),
        "late A replaced or closed active B: {after_old_landed:?}"
    );
}

#[test]
fn two_views_read_the_same_frames_and_one_leave_preserves_the_other() {
    let root = Path::new(".");
    let id = format!("chat-two-views-{}", khor_node::link::fresh_hex().unwrap());
    let first = begin_view(root, &id).unwrap();
    let second = begin_view(root, &id).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (mut peer, _) = listener.accept().unwrap();
    peer.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    assert!(
        open_chat_with(root, &id, &first, || Ok((
            khor_node::gui_host::Attached { stream: client },
            None
        )))
        .unwrap()
    );
    assert!(
        !open_chat_with(root, &id, &second, || panic!(
            "a second live view must share the connection"
        ))
        .unwrap()
    );
    write_frame(&mut peer, &GuiNote::Turn("both-views".into())).unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    let (a, b) = loop {
        let a = chat_poll(root, &id, &first, 0).unwrap();
        let b = chat_poll(root, &id, &second, 0).unwrap();
        if a.next > 0 && b.next > 0 {
            break (a, b);
        }
        assert!(
            Instant::now() < deadline,
            "the shared reader did not deliver its marker"
        );
        std::thread::sleep(Duration::from_millis(2));
    };
    assert_eq!(a.next, b.next);
    for batch in [&a, &b] {
        assert!(matches!(&batch.frames[0], ChatFrame::Turn { stop } if stop == "both-views"));
    }
    chat_leave(root, &id, &first).unwrap();
    assert!(
        chat_stop(root, &id, &first).is_err(),
        "released view has no input authority"
    );
    chat_set_mode(root, &id, &second, "still-here").unwrap();
    assert!(
        matches!(read_frame::<GuiOp>(&mut peer).unwrap(), GuiOp::SetMode(mode) if mode == "still-here")
    );
    assert_eq!(
        chat_poll(root, &id, &second, 0).unwrap().next,
        b.next,
        "one view does not drain the other cursor"
    );
    chat_leave(root, &id, &second).unwrap();
}
