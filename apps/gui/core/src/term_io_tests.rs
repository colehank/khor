use super::*;
use std::{
    io::Read,
    net::{TcpListener, TcpStream},
    sync::mpsc,
    time::Duration,
};

struct Attached {
    id: String,
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
            conn: crate::connection::Connection::new(conn).unwrap(),
            commands: Default::default(),
            parser: Mutex::new(vt100::Parser::new(24, 80, 0)),
            acs: Mutex::new(crate::acs::Acs::default()),
            gone: AtomicBool::new(false),
            stall: Mutex::new(None),
            arrived: tokio::sync::Notify::new(),
            holds: AtomicUsize::new(1),
            left: AtomicBool::new(false),
            feed: Mutex::new(Feed::default()),
            fed: AtomicU64::new(0),
            history: AtomicU32::new(0),
        });
        let id = id.unwrap_or_else(|| format!("term-io-{}", khor_node::link::fresh_hex().unwrap()));
        assert!(plock(terms()).insert(id.clone(), term.clone()).is_none());
        Self { id, term, peer }
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
        let ours = plock(terms())
            .get(&self.id)
            .is_some_and(|t| Arc::ptr_eq(t, &self.term));
        if ours {
            let _ = term_leave(&self.id);
        }
        self.term.commands.close();
        self.term.conn.close();
    }
}

#[test]
fn input_resize_and_clear_preserve_the_actual_host_protocol_order() {
    let mut attached = Attached::new(None);
    let key = submit_key(&attached.id, b"typed".to_vec()).unwrap();
    let size = submit_resize(&attached.id, 100, 30).unwrap();
    let clear = submit_forget(&attached.id, true).unwrap();
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
    let clear = submit_forget(&attached.id, false).unwrap();
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
    let clear = submit_forget(&attached.id, false).unwrap();
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
    let clear = submit_forget(&attached.id, false).unwrap();
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
    let clear = submit_forget(&attached.id, true).unwrap();
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
    let clear = submit_forget(&attached.id, false).unwrap();
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
    assert!(submit_forget(&attached.id, true).unwrap().wait().is_err());
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
    let stale = submit_key(&old.id, b"stale".to_vec()).unwrap();
    term_leave(&old.id).unwrap();
    let mut reopened = Attached::new(Some(old.id.clone()));
    let fresh = submit_key(&reopened.id, b"fresh".to_vec()).unwrap();
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
