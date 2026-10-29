//! Current terminal handshake and renderer controls through the real core API
//! and a bounded TCP peer. No old binary, user home, tmux server or app is used.
use base64::Engine;
use khor_gui_core::term;
use khor_node::{
    Node, SessionId,
    host::{self, ClientOp, Hello, HostFile, Welcome},
};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

struct Fixture {
    root: PathBuf,
    id: SessionId,
    lease: String,
    peer: Option<thread::JoinHandle<()>>,
}
impl Fixture {
    fn new(greeting: &'static str, exchange: impl FnOnce(&mut TcpStream) + Send + 'static) -> Self {
        static SERIAL: AtomicU64 = AtomicU64::new(0);
        let serial = SERIAL.fetch_add(1, Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("khor-term-current-{}-{serial}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let node = Node::open(root.clone()).unwrap();
        let id = SessionId(format!("shell/current-{}-{serial}", std::process::id()));
        let live = khor_node::live::LiveKind::new(root.clone(), node.device());
        live.register(&id, "shell", "owned TCP fixture", None, None)
            .unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        // This marker identifies the owned in-process TCP peer. It is never
        // passed to Node::close, which would terminate its hosting process.
        let record = HostFile {
            port: listener.local_addr().unwrap().port(),
            cookie: "owned-current-cookie".into(),
            host_pid: std::process::id(),
            child_pid: std::process::id(),
        };
        std::fs::write(
            host::host_file_path(&live.dir_of(&id).unwrap()),
            serde_json::to_vec(&record).unwrap(),
        )
        .unwrap();
        let peer = thread::spawn(move || {
            let until = Instant::now() + Duration::from_secs(5);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(error) => panic!("fixture accept: {error}"),
                }
                assert!(
                    Instant::now() < until,
                    "core never connected to the owned peer"
                );
                thread::sleep(Duration::from_millis(5));
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            stream
                .set_write_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let hello: Hello = host::read_frame(&mut stream).unwrap();
            assert_eq!(hello.cookie, "owned-current-cookie");
            host::write_frame(
                &mut stream,
                &Welcome {
                    ok: true,
                    why: greeting.into(),
                },
            )
            .unwrap();
            exchange(&mut stream);
        });
        let lease =
            khor_gui_core::attachment_begin(&root, &id.0, khor_gui_core::AttachmentKind::Terminal)
                .unwrap();
        Self {
            lease,
            root,
            id,
            peer: Some(peer),
        }
    }
    fn finish(&mut self) {
        let _ = term::term_leave(&self.root, &self.id.0, &self.lease);
        self.peer.take().unwrap().join().unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = term::term_leave(&self.root, &self.id.0, &self.lease);
        if let Some(peer) = self.peer.take() {
            let result = peer.join();
            if !thread::panicking() {
                assert!(result.is_ok(), "owned TCP peer failed");
            }
        }
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
fn until(fixture: &Fixture, needle: &str) -> Vec<u8> {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let chunk = term::term_stream(&fixture.root, &fixture.id.0, &fixture.lease, 0).unwrap();
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(chunk.bytes)
            .unwrap();
        if String::from_utf8_lossy(&bytes).contains(needle) {
            return bytes;
        }
        assert!(
            Instant::now() < deadline,
            "missing {needle:?}; gone={}",
            chunk.gone
        );
        thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn core_rejects_noncurrent_welcomes_without_registering_an_attachment() {
    for greeting in ["", "khor-host/4", "khor-host/6", "khor-gui/1"] {
        let mut fixture = Fixture::new(greeting, |stream| {
            assert_eq!(
                stream.read(&mut [0u8; 1]).unwrap(),
                0,
                "the rejected peer received a client op"
            );
        });
        let error = term::term_open(&fixture.root, &fixture.id.0, &fixture.lease, 80, 24, 5000)
            .unwrap_err();
        assert_eq!(
            khor_catalog::read(&error).map(|(key, _)| key),
            Some("msg.peer-non-answer")
        );
        assert!(term::term_attach(&fixture.root, &fixture.id.0, &fixture.lease).is_err());
        fixture.finish();
    }
}

#[test]
fn core_sends_history_and_only_renderer_ready_then_forgets_and_continues_on_one_socket() {
    let (at_history, waiting) = mpsc::channel();
    let mut fixture = Fixture::new("khor-host/5", move |stream| {
        assert!(matches!(
            host::read_frame(stream).unwrap(),
            ClientOp::Version(5)
        ));
        assert!(matches!(
            host::read_frame(stream).unwrap(),
            ClientOp::Scrollback(5000)
        ));
        stream.write_all(b"before-clear\r\n").unwrap();
        stream
            .set_read_timeout(Some(Duration::from_millis(100)))
            .unwrap();
        let error = stream
            .read(&mut [0u8; 1])
            .expect_err("core sent Ready before the renderer called it");
        assert!(matches!(
            error.kind(),
            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
        ));
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        at_history.send(()).unwrap();
        assert!(matches!(host::read_frame(stream).unwrap(), ClientOp::Ready));
        assert!(matches!(
            host::read_frame(stream).unwrap(),
            ClientOp::Forget
        ));
        assert!(
            matches!(host::read_frame(stream).unwrap(), ClientOp::Input(bytes) if bytes == [0x0c])
        );
        assert!(matches!(
            host::read_frame(stream).unwrap(),
            ClientOp::Resize { cols: 90, rows: 28 }
        ));
        stream.write_all(b"after-clear\r\n").unwrap();
        assert!(
            matches!(host::read_frame(stream).unwrap(), ClientOp::Input(bytes) if bytes == b"same-socket\r")
        );
        stream.write_all(b"same-socket\r\n").unwrap();
    });
    term::term_open(&fixture.root, &fixture.id.0, &fixture.lease, 80, 24, 5000).unwrap();
    waiting.recv_timeout(Duration::from_secs(3)).unwrap();
    until(&fixture, "before-clear");
    term::term_attach(&fixture.root, &fixture.id.0, &fixture.lease).unwrap();
    term::term_ready(&fixture.root, &fixture.id.0, &fixture.lease).unwrap();
    assert!(term::term_forget(&fixture.root, &fixture.id.0, &fixture.lease, true).unwrap());
    term::term_resize(&fixture.root, &fixture.id.0, &fixture.lease, 90, 28).unwrap();
    let after = until(&fixture, "after-clear");
    assert!(!String::from_utf8_lossy(&after).contains("before-clear"));
    term::term_key(
        &fixture.root,
        &fixture.id.0,
        &fixture.lease,
        b"same-socket\r".to_vec(),
    )
    .unwrap();
    until(&fixture, "same-socket");
    fixture.finish();
}
