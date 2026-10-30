use khor_node::{
    gui_host,
    host::{HANDSHAKE, Hello, Welcome, read_frame, write_frame},
};
use std::{
    io::{self, Read},
    net::TcpListener,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

fn exchange(welcome: Welcome) -> (Result<gui_host::Attached, String>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let hello: Hello = read_frame(&mut stream).unwrap();
        assert_eq!(hello.cookie, "owned-gui-greeting-cookie");
        assert_eq!((hello.cols, hello.rows), (0, 0));
        write_frame(&mut stream, &welcome).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_millis(100)))
            .unwrap();
        let mut byte = [0];
        match stream.read(&mut byte) {
            Ok(0) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) => {}
            other => panic!("GUI handshake sent an unexpected protocol frame: {other:?}"),
        }
    });
    (
        gui_host::connect_at(&address, "owned-gui-greeting-cookie"),
        server,
    )
}

#[test]
fn current_gui_greeting_keeps_a_raw_stream_without_terminal_negotiation() {
    let (result, server) = exchange(Welcome {
        ok: true,
        why: "khor-gui/2".into(),
    });
    let attached = result.unwrap();
    assert_eq!(attached.stream.read_timeout().unwrap(), None);
    assert_eq!(attached.stream.write_timeout().unwrap(), None);
    drop(attached);
    server.join().unwrap();
}

#[test]
fn empty_unknown_terminal_and_other_gui_greetings_are_rejected() {
    for greeting in ["", "unknown", "khor-host/5", "khor-gui/1", "khor-gui/3"] {
        let (result, server) = exchange(Welcome {
            ok: true,
            why: greeting.into(),
        });
        let why = result.err();
        server.join().unwrap();
        assert!(
            why.is_some(),
            "unsupported GUI greeting was accepted: {greeting:?}"
        );
        assert_eq!(
            khor_catalog::read(why.as_deref().unwrap()).map(|(key, _)| key),
            Some("msg.host-gui-protocol"),
            "a protocol mismatch must not become an unrelated connection error"
        );
    }
}

#[test]
fn a_refused_gui_handshake_preserves_the_hosts_reason() {
    let (result, server) = exchange(Welcome {
        ok: false,
        why: "owned fixture refusal".into(),
    });
    assert_eq!(result.err().as_deref(), Some("owned fixture refusal"));
    server.join().unwrap();
}

#[test]
fn a_silent_gui_host_obeys_the_shared_handshake_deadline() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let done = Arc::new(AtomicBool::new(false));
    let observed = done.clone();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let _: Hello = read_frame(&mut stream).unwrap();
        let began = Instant::now();
        while !observed.load(Ordering::Acquire)
            && began.elapsed() < HANDSHAKE + Duration::from_secs(3)
        {
            thread::sleep(Duration::from_millis(10));
        }
    });
    let began = Instant::now();
    let result = gui_host::connect_at(&address, "owned-gui-greeting-cookie");
    let elapsed = began.elapsed();
    done.store(true, Ordering::Release);
    server.join().unwrap();
    let why = result.err().expect("a silent host must not connect");
    assert_eq!(
        khor_catalog::read(&why).map(|(key, _)| key),
        Some("msg.host-silent")
    );
    assert!(
        elapsed >= HANDSHAKE / 2 && elapsed < HANDSHAKE + Duration::from_secs(3),
        "{elapsed:?}"
    );
}
