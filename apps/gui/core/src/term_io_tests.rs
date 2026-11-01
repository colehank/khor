use super::*;
use std::{
    io::Read,
    net::{TcpListener, TcpStream},
    sync::mpsc,
    time::Duration,
};

struct Attached {
    id: String,
    lease: String,
    term: Arc<Term>,
    peer: TcpStream,
}

impl Attached {
    fn new(id: Option<String>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let conn = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (peer, _) = listener.accept().unwrap();
        peer.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
        let term = Arc::new(Term {
            far: None,
            conn: crate::connection::Connection::new(conn).unwrap(),
            commands: Default::default(),
            parser: Mutex::new(vt100::Parser::new(24, 80, 0)),
            acs: Mutex::new(crate::acs::Acs::default()),
            gone: AtomicBool::new(false),
            stall: Mutex::new(None),
            arrived: tokio::sync::Notify::new(),
            left: AtomicBool::new(false),
            feed: Mutex::new(Feed::default()),
            fed: AtomicU64::new(0),
            history: AtomicU32::new(0),
        });
        let id = id.unwrap_or_else(|| format!("term-io-{}", khor_node::link::fresh_hex().unwrap()));
        let lease = terms().begin(Path::new("."), &id).unwrap();
        let opening = terms().claim(Path::new("."), &id, &lease).unwrap();
        let (term, _) = opening.publish(term).unwrap();
        drop(opening);
        Self {
            id,
            lease,
            term,
            peer,
        }
    }

    fn read(&mut self) -> ClientOp {
        khor_node::host::read_frame(&mut self.peer).unwrap()
    }

    fn paint(&self, bytes: &[u8]) {
        assert_eq!(
            paint(
                &self.term.acs,
                &self.term.parser,
                &self.term.feed,
                &self.term.fed,
                bytes,
            ),
            None
        );
    }

    fn block(&self) -> (mpsc::Sender<()>, crate::command_queue::Pending<()>) {
        let (started, entered) = mpsc::channel();
        let (release, held) = mpsc::channel();
        let pending = self
            .term
            .commands
            .submit(move || {
                started.send(()).unwrap();
                held.recv_timeout(Duration::from_secs(3))
                    .map_err(|e| e.to_string())
            })
            .unwrap();
        entered.recv_timeout(Duration::from_secs(3)).unwrap();
        (release, pending)
    }
}

impl Drop for Attached {
    fn drop(&mut self) {
        let _ = term_leave(Path::new("."), &self.id, &self.lease);
    }
}

#[test]
fn input_resize_and_clear_preserve_the_actual_host_protocol_order() {
    let mut attached = Attached::new(None);
    let key = submit_key(
        Path::new("."),
        &attached.id,
        &attached.lease,
        b"typed".to_vec(),
    )
    .unwrap();
    let size = submit_resize(Path::new("."), &attached.id, &attached.lease, 100, 30).unwrap();
    let clear = submit_forget(Path::new("."), &attached.id, &attached.lease, true).unwrap();
    assert!(matches!(attached.read(), ClientOp::Input(bytes) if bytes == b"typed"));
    assert!(matches!(
        attached.read(),
        ClientOp::Resize {
            cols: 100,
            rows: 30
        }
    ));
    assert!(matches!(attached.read(), ClientOp::Forget));
    assert!(matches!(attached.read(), ClientOp::Input(bytes) if bytes == [0x0c]));
    key.wait().unwrap();
    size.wait().unwrap();
    assert!(clear.wait().unwrap());
    assert_eq!(plock(&attached.term.parser).screen().size(), (30, 100));
}

#[test]
fn a_clear_preserves_the_prompt_modes_and_caught_up_cursor() {
    let mut attached = Attached::new(None);
    attached.paint(
        b"\x1b[?2004h\x1b[?1h\x1b=\x1b[?25lsecret one\r\nsecret two\r\n\x1b[33m$ \x1b[0mtyped",
    );
    let end = attached.term.fed.load(Ordering::Relaxed);
    let clear = submit_forget(Path::new("."), &attached.id, &attached.lease, false).unwrap();
    assert!(matches!(attached.read(), ClientOp::Forget));
    assert!(clear.wait().unwrap());
    assert_eq!(attached.term.fed.load(Ordering::Relaxed), end);

    let (bytes, seq, reset) = take_since(
        &attached.term.parser,
        &attached.term.feed,
        &attached.term.fed,
        0,
    );
    assert!(reset && seq == end);
    assert!(!String::from_utf8_lossy(&bytes).contains("secret"));
    let mut face = vt100::Parser::new(24, 80, 0);
    face.process(&bytes);
    {
        let parser = plock(&attached.term.parser);
        for screen in [face.screen(), parser.screen()] {
            assert_eq!(screen.contents().trim_end(), "$ typed");
            assert_eq!(screen.cursor_position(), (0, 7));
            assert_eq!(screen.cell(0, 0).unwrap().fgcolor(), vt100::Color::Idx(3));
            assert!(screen.bracketed_paste() && screen.application_cursor());
            assert!(screen.application_keypad() && screen.hide_cursor());
        }
    }
    let (idle, seq, reset) = take_since(
        &attached.term.parser,
        &attached.term.feed,
        &attached.term.fed,
        end,
    );
    assert!(idle.is_empty() && seq == end && !reset);
    attached.paint(b"-after");
    let (tail, _, reset) = take_since(
        &attached.term.parser,
        &attached.term.feed,
        &attached.term.fed,
        end,
    );
    assert_eq!(tail, b"-after");
    assert!(!reset);

    attached.paint(b"\r\nsecond secret\r\n\x1b[32m$ two\x1b[0m");
    let end = attached.term.fed.load(Ordering::Relaxed);
    let clear = submit_forget(Path::new("."), &attached.id, &attached.lease, false).unwrap();
    assert!(matches!(attached.read(), ClientOp::Forget));
    assert!(clear.wait().unwrap());
    let (bytes, seq, reset) = take_since(
        &attached.term.parser,
        &attached.term.feed,
        &attached.term.fed,
        0,
    );
    assert!(reset && seq == end);
    let mut face = vt100::Parser::new(24, 80, 0);
    face.process(&bytes);
    assert_eq!(face.screen().contents().trim_end(), "$ two");
    assert_eq!(
        plock(&attached.term.parser).screen().contents().trim_end(),
        "$ two"
    );
    assert_eq!(
        face.screen().cell(0, 0).unwrap().fgcolor(),
        vt100::Color::Idx(2)
    );
}

#[test]
fn a_later_feed_trim_cannot_revive_the_screen_a_clear_forgot() {
    let mut attached = Attached::new(None);
    attached.paint(b"secret before clear\r\n\x1b[?2004h\x1b[33m$ \x1b[0mtyped");
    let clear = submit_forget(Path::new("."), &attached.id, &attached.lease, false).unwrap();
    assert!(matches!(attached.read(), ClientOp::Forget));
    assert!(clear.wait().unwrap());
    let since = attached.term.fed.load(Ordering::Relaxed);
    // Complete bounded OSC titles grow the actual feed without scrolling
    // the old screen away: clearing only the cursor boundary cannot pass.
    let mut title = b"\x1b]0;".to_vec();
    title.extend_from_slice(&[b'x'; 1000]);
    title.push(7);
    for _ in 0..(FEED_CAP / title.len() + 2) {
        attached.paint(&title);
    }
    attached.paint(b"-after-trim");
    assert!(attached.term.fed.load(Ordering::Relaxed) > since + FEED_CAP as u64);
    let (bytes, seq, reset) = take_since(
        &attached.term.parser,
        &attached.term.feed,
        &attached.term.fed,
        since,
    );
    assert!(reset, "the post-clear cursor must fall off the real feed");
    assert!(!String::from_utf8_lossy(&bytes).contains("secret"));
    let mut face = vt100::Parser::new(24, 80, 0);
    face.process(&bytes);
    assert_eq!(face.screen().contents().trim_end(), "$ typed-after-trim");
    assert_eq!(
        face.screen().cell(0, 0).unwrap().fgcolor(),
        vt100::Color::Idx(3)
    );
    assert!(face.screen().bracketed_paste());
    assert_eq!(face.screen().cursor_position(), (0, 18));
    let (idle, end, reset) = take_since(
        &attached.term.parser,
        &attached.term.feed,
        &attached.term.fed,
        seq,
    );
    assert!(idle.is_empty() && end == seq && !reset);
}

#[test]
fn a_redraw_is_requested_only_after_the_local_screen_and_feed_clear() {
    let mut attached = Attached::new(None);
    attached.paint(b"secret before clear\r\n$ typed");
    let term = attached.term.clone();
    let parser = plock(&term.parser);
    let clear = submit_forget(Path::new("."), &attached.id, &attached.lease, true).unwrap();
    assert!(matches!(attached.read(), ClientOp::Forget));
    attached
        .peer
        .set_read_timeout(Some(Duration::from_millis(100)))
        .unwrap();
    let error = attached
        .peer
        .read(&mut [0])
        .expect_err("Ctrl-L preceded the local clear");
    assert!(matches!(
        error.kind(),
        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
    ));
    drop(parser);
    attached
        .peer
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    assert!(matches!(attached.read(), ClientOp::Input(bytes) if bytes == [0x0c]));
    // This redraw may arrive as soon as the peer reads Ctrl-L.
    attached.paint(b"\x1b[H\x1b[2Jredrawn");
    assert!(clear.wait().unwrap());
    let (bytes, _) = take_all(&term.feed, &term.fed);
    let mut face = vt100::Parser::new(24, 80, 0);
    face.process(&bytes);
    assert_eq!(face.screen().contents().trim_end(), "redrawn");
    assert_eq!(
        plock(&term.parser).screen().contents().trim_end(),
        "redrawn"
    );
}

#[test]
fn an_alternate_screen_is_kept_and_a_failed_forget_does_not_clear() {
    let mut attached = Attached::new(None);
    attached.paint(b"normal\r\n\x1b[?1049hprogram\r\nactive");
    let before = take_all(&attached.term.feed, &attached.term.fed);
    let clear = submit_forget(Path::new("."), &attached.id, &attached.lease, false).unwrap();
    assert!(matches!(attached.read(), ClientOp::Forget));
    assert!(clear.wait().unwrap());
    assert_eq!(take_all(&attached.term.feed, &attached.term.fed), before);
    assert!(plock(&attached.term.parser).screen().alternate_screen());
    assert!(
        plock(&attached.term.parser)
            .screen()
            .contents()
            .contains("program")
    );

    attached.paint(b"\x1b[?1049lsecret\r\n$ typed");
    let before = take_all(&attached.term.feed, &attached.term.fed);
    let screen = plock(&attached.term.parser).screen().contents();
    attached.term.conn.close();
    assert!(
        submit_forget(Path::new("."), &attached.id, &attached.lease, true)
            .unwrap()
            .wait()
            .is_err()
    );
    assert_eq!(take_all(&attached.term.feed, &attached.term.fed), before);
    assert_eq!(plock(&attached.term.parser).screen().contents(), screen);
}

#[test]
fn a_cancelled_queued_paste_writes_no_terminal_input() {
    let mut attached = Attached::new(None);
    let (release, first) = attached.block();
    let stopped = Arc::new(AtomicBool::new(false));
    let paste = term_paste(attached.term.clone(), "never paste".into(), stopped.clone()).unwrap();
    stopped.store(true, Ordering::Relaxed);
    release.send(()).unwrap();
    first.wait().unwrap();
    assert_eq!(paste.wait().unwrap_err(), khor_node::drops::cancelled());
    attached.peer.set_nonblocking(true).unwrap();
    let mut byte = [0];
    assert_eq!(
        attached.peer.read(&mut byte).unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

#[test]
fn queued_input_cannot_cross_a_closed_and_reopened_terminal_id() {
    let old = Attached::new(None);
    let (release, first) = old.block();
    let stale = submit_key(Path::new("."), &old.id, &old.lease, b"stale".to_vec()).unwrap();
    term_leave(Path::new("."), &old.id, &old.lease).unwrap();
    let mut reopened = Attached::new(Some(old.id.clone()));
    let fresh = submit_key(
        Path::new("."),
        &reopened.id,
        &reopened.lease,
        b"fresh".to_vec(),
    )
    .unwrap();
    assert!(stale.wait().is_err());
    release.send(()).unwrap();
    first.wait().unwrap();
    fresh.wait().unwrap();
    assert!(matches!(reopened.read(), ClientOp::Input(bytes) if bytes == b"fresh"));
    reopened.peer.set_nonblocking(true).unwrap();
    let mut byte = [0];
    assert_eq!(
        reopened.peer.read(&mut byte).unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

#[test]
fn active_reopen_retains_its_own_connection_when_an_old_open_lands_last() {
    use std::io::Write as _;
    fn pair() -> (TcpStream, TcpStream) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (peer, _) = listener.accept().unwrap();
        (client, peer)
    }
    fn wait_for_marker(id: &str, lease: &str, marker: &str) -> Result<(), String> {
        let until = std::time::Instant::now() + Duration::from_secs(2);
        loop {
            let batch = term_stream(Path::new("."), id, lease, 0)?;
            let bytes = B64.decode(batch.bytes).map_err(|e| e.to_string())?;
            if bytes
                .windows(marker.len())
                .any(|bytes| bytes == marker.as_bytes())
            {
                return Ok(());
            }
            if std::time::Instant::now() >= until {
                return Err("active terminal connection did not receive its own marker".into());
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    struct Cleanup(String, String);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = term_leave(Path::new("."), &self.0, &self.1);
        }
    }
    let id = format!("term-reopen-{}", khor_node::link::fresh_hex().unwrap());
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
            open_terminal_with(Path::new("."), &old_id, &old_view, 80, 24, 0, || {
                entered.send(()).map_err(|e| e.to_string())?;
                held.recv_timeout(Duration::from_secs(3))
                    .map_err(|e| e.to_string())?;
                Ok((old_stream, None))
            })
        });
        at_connector.recv_timeout(Duration::from_secs(2)).unwrap();
        term_leave(Path::new("."), &id, &old_lease).unwrap();
        let new_open = open_terminal_with(Path::new("."), &id, &new_lease, 80, 24, 0, || {
            Ok((new_stream, None))
        });
        let before = new_open.and_then(|_| {
            new_peer
                .write_all(b"active-B-before-A\r\n")
                .map_err(|e| e.to_string())?;
            wait_for_marker(&id, &new_lease, "active-B-before-A")
        });
        release.send(()).unwrap();
        let old_result = old.join().unwrap();
        assert!(
            old_result.is_err(),
            "retired A must not acquire a live view"
        );
        let after = (|| {
            new_peer
                .write_all(b"active-B-after-A\r\n")
                .map_err(|e| e.to_string())?;
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
fn two_views_read_the_same_bytes_and_one_leave_preserves_the_other() {
    use std::io::Write as _;
    let root = Path::new(".");
    let id = format!("term-two-views-{}", khor_node::link::fresh_hex().unwrap());
    let first = begin_view(root, &id).unwrap();
    let second = begin_view(root, &id).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (mut peer, _) = listener.accept().unwrap();
    peer.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    open_terminal_with(root, &id, &first, 80, 24, 0, || Ok((client, None))).unwrap();
    open_terminal_with(root, &id, &second, 80, 24, 0, || {
        panic!("a second live view must share the connection")
    })
    .unwrap();
    assert!(matches!(
        khor_node::host::read_frame::<ClientOp>(&mut peer).unwrap(),
        ClientOp::Resize { cols: 80, rows: 24 }
    ));
    peer.write_all(b"both-views\r\n").unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    let (a, b) = loop {
        let a = term_stream(root, &id, &first, 0).unwrap();
        let b = term_stream(root, &id, &second, 0).unwrap();
        if a.seq > 0 && b.seq > 0 {
            break (a, b);
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the shared reader did not deliver its marker"
        );
        std::thread::sleep(Duration::from_millis(2));
    };
    assert_eq!(B64.decode(&a.bytes).unwrap(), b"both-views\r\n");
    assert_eq!(a.bytes, b.bytes);
    assert_eq!(a.seq, b.seq);
    term_leave(root, &id, &first).unwrap();
    assert!(
        term_key(root, &id, &first, b"stale".to_vec()).is_err(),
        "released view has no input authority"
    );
    term_key(root, &id, &second, b"still-here".to_vec()).unwrap();
    assert!(
        matches!(khor_node::host::read_frame::<ClientOp>(&mut peer).unwrap(), ClientOp::Input(bytes) if bytes == b"still-here")
    );
    assert_eq!(
        term_stream(root, &id, &second, 0).unwrap().seq,
        b.seq,
        "one view does not drain the other cursor"
    );
    term_leave(root, &id, &second).unwrap();
}
