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
            version: AtomicU32::new(5),
            history: AtomicU32::new(0),
        });
        let id = id.unwrap_or_else(|| format!("term-io-{}", khor_node::link::fresh_hex().unwrap()));
        assert!(plock(terms()).insert(id.clone(), term.clone()).is_none());
        Self { id, term, peer }
    }

    fn read(&mut self) -> ClientOp {
        khor_node::host::read_frame(&mut self.peer).unwrap()
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
