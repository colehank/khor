// Compiles the production std-only scheduler and connection directly. The
// Cargo tests in chat/term/native_io cover their protocol and invoke adapters.
#[path = "../../apps/gui/core/src/command_queue.rs"]
mod command_queue;
#[path = "../../apps/gui/core/src/connection.rs"]
mod connection;

use command_queue::CommandQueue;
use connection::Connection;
use std::{
    future::Future,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    pin::Pin,
    sync::{Arc, Mutex, atomic::{AtomicUsize, Ordering}, mpsc},
    task::{Context, Poll, Wake, Waker},
    time::{Duration, Instant},
};

fn socket() -> (Arc<Connection>, TcpStream) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (server, _) = listener.accept().unwrap();
    server.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
    (Arc::new(Connection::new(client).unwrap()), server)
}

#[test]
fn ordered_submission_does_not_wait_or_block_another_attachment() {
    let queue = CommandQueue::default();
    let (release, blocked) = mpsc::channel();
    let first = queue.submit(move || blocked.recv().map_err(|e| e.to_string())).unwrap();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut replies = Vec::new();
    for n in 0..100 {
        let seen = seen.clone();
        replies.push(queue.submit(move || { seen.lock().unwrap().push(n); Ok(n) }).unwrap());
    }
    let other = CommandQueue::default();
    assert_eq!(other.submit(|| Ok(42)).unwrap().wait().unwrap(), 42);
    assert!(seen.lock().unwrap().is_empty());
    release.send(()).unwrap();
    first.wait().unwrap();
    for (n, reply) in replies.into_iter().enumerate().rev() {
        assert_eq!(reply.wait().unwrap(), n);
    }
    assert_eq!(*seen.lock().unwrap(), (0..100).collect::<Vec<_>>());
}

#[test]
fn a_blocked_socket_can_close_without_taking_its_write_lock() {
    let queue = CommandQueue::default();
    let (conn, _peer) = socket();
    let (started, writing) = mpsc::channel();
    let target = conn.clone();
    let at = Instant::now();
    let writing_reply = queue.submit(move || {
        let mut writer = target.lock().unwrap();
        started.send(()).unwrap();
        writer.write_all(&vec![0; 32 * 1024 * 1024]).map_err(|e| e.to_string())
    }).unwrap();
    assert!(at.elapsed() < Duration::from_millis(200));
    writing.recv_timeout(Duration::from_secs(2)).unwrap();
    let (finished, done) = mpsc::channel();
    std::thread::spawn(move || finished.send(writing_reply.wait()).unwrap());
    assert!(done.recv_timeout(Duration::from_millis(200)).is_err(), "peer is not reading");
    let pending_ran = Arc::new(AtomicUsize::new(0));
    let target = pending_ran.clone();
    let pending = queue.submit(move || { target.fetch_add(1, Ordering::SeqCst); Ok(()) }).unwrap();
    // First cancel queued work, then interrupt the in-flight write.
    queue.close();
    let at = Instant::now();
    conn.close();
    assert!(at.elapsed() < Duration::from_millis(200));
    assert!(pending.wait().is_err());
    assert!(done.recv_timeout(Duration::from_secs(2)).unwrap().is_err());
    assert_eq!(pending_ran.load(Ordering::SeqCst), 0);
    assert!(queue.submit(|| Ok(())).is_err());
}

#[test]
fn close_and_reopen_are_different_queue_and_connection_instances() {
    let old = CommandQueue::default();
    let (old_conn, mut old_peer) = socket();
    let (release, blocked) = mpsc::channel();
    let (started, began) = mpsc::channel();
    let first = old.submit(move || { started.send(()).unwrap(); blocked.recv().map_err(|e| e.to_string()) }).unwrap();
    began.recv_timeout(Duration::from_secs(2)).unwrap();
    let target = old_conn.clone();
    let stale = old.submit(move || target.lock().unwrap().write_all(b"old").map_err(|e| e.to_string())).unwrap();
    old.close(); old_conn.close();
    let new = CommandQueue::default();
    let (new_conn, mut new_peer) = socket();
    let target = new_conn.clone();
    new.submit(move || target.lock().unwrap().write_all(b"new").map_err(|e| e.to_string())).unwrap().wait().unwrap();
    assert!(stale.wait().is_err());
    release.send(()).unwrap(); first.wait().unwrap();
    let mut bytes = [0; 3];
    new_peer.read_exact(&mut bytes).unwrap(); assert_eq!(&bytes, b"new");
    assert_eq!(old_peer.read(&mut bytes).unwrap(), 0);
    let (_, another_peer) = socket();
    assert!(old_conn.replace(another_peer).is_err(), "late rejoin cannot revive a closed attachment");
}

#[test]
fn a_control_can_run_between_turn_slices() {
    let turns = CommandQueue::default();
    let controls = CommandQueue::default();
    let (conn, mut peer) = socket();
    let (sliced, slice) = mpsc::channel();
    let (resume, continue_turn) = mpsc::channel();
    let target = conn.clone();
    let turn = turns.submit(move || {
        target.lock().unwrap().write_all(b"a").unwrap();
        sliced.send(()).unwrap(); continue_turn.recv().unwrap();
        target.lock().unwrap().write_all(b"b").map_err(|e| e.to_string())
    }).unwrap();
    slice.recv_timeout(Duration::from_secs(2)).unwrap();
    let target = conn.clone();
    controls.submit(move || target.lock().unwrap().write_all(b"S").map_err(|e| e.to_string())).unwrap().wait().unwrap();
    resume.send(()).unwrap(); turn.wait().unwrap();
    let mut bytes = [0; 3]; peer.read_exact(&mut bytes).unwrap(); assert_eq!(&bytes, b"aSb");
}

#[test]
fn completion_wakes_a_future_and_dropped_replies_do_not_lose_input() {
    struct Wakes(AtomicUsize);
    impl Wake for Wakes { fn wake(self: Arc<Self>) { self.0.fetch_add(1, Ordering::SeqCst); } }
    let queue = CommandQueue::default();
    let (release, blocked) = mpsc::channel();
    let mut pending = queue.submit(move || { blocked.recv().unwrap(); Ok(7) }).unwrap();
    let wakes = Arc::new(Wakes(AtomicUsize::new(0)));
    let waker = Waker::from(wakes.clone()); let mut cx = Context::from_waker(&waker);
    assert!(Pin::new(&mut pending).poll(&mut cx).is_pending());
    release.send(()).unwrap();
    queue.submit(|| Ok(())).unwrap().wait().unwrap();
    assert!(wakes.0.load(Ordering::SeqCst) > 0);
    assert!(matches!(Pin::new(&mut pending).poll(&mut cx), Poll::Ready(Ok(7))));
    let (sent, got) = mpsc::channel();
    drop(queue.submit(move || { sent.send(()).unwrap(); Ok(()) }).unwrap());
    got.recv_timeout(Duration::from_secs(2)).unwrap();
}

#[test]
fn errors_and_panics_complete_their_reply_and_leave_the_lane_usable() {
    let queue = CommandQueue::default();
    assert_eq!(queue.submit(|| Err::<(), _>("wire broke".into())).unwrap().wait().unwrap_err(), "wire broke");
    assert!(queue.submit(|| -> Result<(), String> { panic!("fixture") }).unwrap().wait().is_err());
    assert_eq!(queue.submit(|| Ok(3)).unwrap().wait().unwrap(), 3);
}

#[test]
fn queue_backpressure_is_bounded_and_close_releases_captured_owners() {
    let queue = CommandQueue::default();
    let (started, began) = mpsc::channel(); let (release, blocked) = mpsc::channel();
    let first = queue.submit(move || { started.send(()).unwrap(); blocked.recv().unwrap(); Ok(()) }).unwrap();
    began.recv_timeout(Duration::from_secs(2)).unwrap();
    let owner = Arc::new(()); let weak = Arc::downgrade(&owner);
    for _ in 0..256 { let owner = owner.clone(); drop(queue.submit(move || { drop(owner); Ok(()) }).unwrap()); }
    assert!(queue.submit(|| Ok(())).is_err()); drop(owner);
    queue.close(); assert!(weak.upgrade().is_none());
    release.send(()).unwrap(); first.wait().unwrap();
}
