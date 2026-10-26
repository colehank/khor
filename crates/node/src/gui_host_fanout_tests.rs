use super::*;
use std::io::Read;
use std::net::TcpListener;
use std::sync::mpsc;

fn pair() -> (TcpStream, TcpStream) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let peer = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (socket, _) = listener.accept().unwrap();
    peer.set_read_timeout(Some(Duration::from_secs(8))).unwrap();
    (socket, peer)
}

fn short_limits() -> Limits {
    Limits {
        frames: 8,
        bytes: 32 * 1024 * 1024,
        write_timeout: Duration::from_millis(20),
        stall_timeout: Duration::from_millis(200),
        close_grace: Duration::from_millis(350),
    }
}

fn client(fanout: &Fanout, who: u64) -> Arc<Client> {
    fanout.registry.lock().unwrap().clients[&who].clone()
}

fn wait_done(client: &Client) {
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut queue = client.shared.queue.lock().unwrap();
    while !queue.done {
        let now = Instant::now();
        assert!(now < deadline, "writer did not retire");
        let (next, _) = client
            .shared
            .changed
            .wait_timeout(queue, deadline - now)
            .unwrap();
        queue = next;
    }
}

fn wait_inflight(client: &Client) {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        {
            let queue = client.shared.queue.lock().unwrap();
            assert!(!queue.done, "writer retired before the budget probe");
            if queue.frames.is_empty() && queue.outstanding_frames == 1 {
                return;
            }
        }
        assert!(
            Instant::now() < deadline,
            "writer never took the queued frame"
        );
        thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn a_stalled_client_does_not_block_admission_or_another_client() {
    let fanout = Fanout::with_limits(short_limits());
    let (slow, _unread_peer) = pair();
    let (fast, mut reader) = pair();
    fanout.add(1, slow).unwrap();
    fanout.add(2, fast).unwrap();
    let slow_client = client(&fanout, 1);
    let large: Arc<[u8]> = vec![0x55; 16 * 1024 * 1024].into();
    let started = Instant::now();
    fanout.send_to(1, large);
    fanout.send(Arc::from(&b"next"[..]));
    assert!(
        started.elapsed() < Duration::from_millis(250),
        "admission blocked on socket writes"
    );
    let mut received = [0; 4];
    reader.read_exact(&mut received).unwrap();
    assert_eq!(&received, b"next");
    wait_done(&slow_client);
    fanout.send_to(2, Arc::from(&b"live"[..]));
    reader.read_exact(&mut received).unwrap();
    assert_eq!(&received, b"live");
    fanout.close();
    assert!(slow_client.writer.lock().unwrap().is_none());
}

#[test]
fn a_large_replay_may_make_progress_for_longer_than_two_seconds() {
    let fanout = Fanout::new();
    let (socket, mut reader) = pair();
    fanout.add(1, socket).unwrap();
    let length = 32 * 1024 * 1024 + 4;
    let mut bytes = vec![0x6d; length];
    bytes[..4].copy_from_slice(&((length - 4) as u32).to_be_bytes());
    let frame: Arc<[u8]> = bytes.into();
    let expected = frame.clone();
    let (tx, rx) = mpsc::channel();
    let reading = thread::spawn(move || {
        let started = Instant::now();
        let mut offset = 0;
        let mut chunk = [0; 64 * 1024];
        while offset < length {
            let take = chunk.len().min(length - offset);
            reader.read_exact(&mut chunk[..take]).unwrap();
            assert_eq!(&chunk[..take], &expected[offset..offset + take]);
            offset += take;
            thread::sleep(Duration::from_millis(8));
        }
        tx.send(started.elapsed()).unwrap();
    });
    fanout.send_to(1, frame);
    let elapsed = rx.recv_timeout(Duration::from_secs(12)).unwrap();
    assert!(elapsed > Duration::from_secs(2));
    reading.join().unwrap();
    fanout.close();
}

#[test]
fn byte_budget_includes_the_frame_currently_being_written() {
    let limits = Limits {
        bytes: 16 * 1024 * 1024,
        ..short_limits()
    };
    let fanout = Fanout::with_limits(limits);
    let (socket, mut peer) = pair();
    fanout.add(1, socket).unwrap();
    let owner = client(&fanout, 1);
    fanout.send_to(1, vec![1; limits.bytes].into());
    // The first frame exceeds the loopback send buffer and remains outstanding.
    wait_inflight(&owner);
    fanout.send_to(1, Arc::from(&b"overflow"[..]));
    wait_done(&owner);
    let mut received = Vec::new();
    peer.read_to_end(&mut received).unwrap();
    assert!(
        received.len() < limits.bytes,
        "over-budget connection was not shut down"
    );
    fanout.close();
    assert!(owner.writer.lock().unwrap().is_none());
}

#[test]
fn frame_budget_includes_inflight_and_retirement_is_connection_local() {
    let limits = Limits {
        frames: 1,
        ..short_limits()
    };
    let fanout = Fanout::with_limits(limits);
    let (socket, _peer) = pair();
    let (other, mut reader) = pair();
    fanout.add(1, socket).unwrap();
    fanout.add(2, other).unwrap();
    let owner = client(&fanout, 1);
    fanout.send_to(1, vec![7; 16 * 1024 * 1024].into());
    wait_inflight(&owner);
    fanout.send_to(1, Arc::from(&b"second"[..]));
    wait_done(&owner);
    fanout.send_to(2, Arc::from(&b"ok"[..]));
    let mut received = [0; 2];
    reader.read_exact(&mut received).unwrap();
    assert_eq!(&received, b"ok");
    fanout.close();
}

#[test]
fn a_single_frame_over_the_byte_budget_retires_before_writing() {
    let fanout = Fanout::with_limits(Limits {
        bytes: 4,
        ..short_limits()
    });
    let (socket, mut peer) = pair();
    fanout.add(1, socket).unwrap();
    let owner = client(&fanout, 1);
    fanout.send_to(1, Arc::from(&b"large"[..]));
    wait_done(&owner);
    assert_eq!(peer.read(&mut [0; 1]).unwrap(), 0);
    fanout.close();
}

#[test]
fn close_flushes_admitted_frames_and_is_idempotent() {
    let fanout = Arc::new(Fanout::new());
    let (socket, mut peer) = pair();
    fanout.add(1, socket).unwrap();
    let owner = client(&fanout, 1);
    fanout.send_to(1, Arc::from(&b"history"[..]));
    fanout.send(Arc::from(&b"Gone"[..]));
    let another = fanout.clone();
    let closing = thread::spawn(move || another.close());
    fanout.close();
    closing.join().unwrap();
    fanout.close();
    let mut received = Vec::new();
    peer.read_to_end(&mut received).unwrap();
    assert_eq!(&received, b"historyGone");
    assert!(owner.writer.lock().unwrap().is_none());
    let (socket, _) = pair();
    assert_eq!(
        fanout.add(2, socket).unwrap_err().kind(),
        io::ErrorKind::BrokenPipe
    );
}

#[test]
fn close_has_one_shared_deadline_and_joins_blocked_writers() {
    let limits = Limits {
        stall_timeout: Duration::from_secs(10),
        ..short_limits()
    };
    let fanout = Fanout::with_limits(limits);
    let mut peers = Vec::new();
    let mut owners = Vec::new();
    for who in 0..3 {
        let (socket, peer) = pair();
        fanout.add(who, socket).unwrap();
        peers.push(peer);
        owners.push(client(&fanout, who));
    }
    fanout.send(vec![9; 16 * 1024 * 1024].into());
    let started = Instant::now();
    fanout.close();
    assert!(
        started.elapsed() < Duration::from_millis(900),
        "close waited per client"
    );
    for owner in owners {
        assert!(owner.shared.queue.lock().unwrap().done);
        assert!(owner.writer.lock().unwrap().is_none());
    }
}

#[test]
fn reader_eof_retires_idle_and_blocked_writers_and_add_reaps_handles() {
    let fanout = Fanout::with_limits(short_limits());
    let mut owners = Vec::new();
    for who in 0..2 {
        let (socket, peer) = pair();
        fanout.add(who, socket).unwrap();
        let owner = client(&fanout, who);
        if who == 1 {
            fanout.send_to(who, vec![1; 16 * 1024 * 1024].into());
        }
        peer.shutdown(Shutdown::Both).unwrap();
        // The host's reader loop observes EOF, then explicitly retires writer.
        fanout.remove(who);
        fanout.remove(who);
        wait_done(&owner);
        owners.push(owner);
    }
    let (socket, _peer) = pair();
    fanout.add(2, socket).unwrap();
    assert_eq!(fanout.registry.lock().unwrap().clients.len(), 1);
    for owner in owners {
        assert!(owner.writer.lock().unwrap().is_none());
    }
    fanout.close();
}

#[derive(Default)]
struct CapacityWake {
    awakened: Mutex<bool>,
    changed: Condvar,
}
impl std::task::Wake for CapacityWake {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        *self.awakened.lock().unwrap() = true;
        self.changed.notify_one();
    }
}
impl CapacityWake {
    fn wait(&self) {
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut awakened = self.awakened.lock().unwrap();
        while !*awakened {
            let now = Instant::now();
            assert!(now < deadline, "capacity waiter was not woken");
            let (next, _) = self.changed.wait_timeout(awakened, deadline - now).unwrap();
            awakened = next;
        }
    }
}

#[test]
fn a_history_producer_waits_for_real_capacity_without_retiring_the_reader() {
    let limits = Limits {
        frames: 1,
        ..short_limits()
    };
    let fanout = Fanout::with_limits(limits);
    let (socket, mut reader) = pair();
    fanout.add(1, socket).unwrap();
    let owner = client(&fanout, 1);
    fanout.send_to(1, vec![4; 16 * 1024 * 1024].into());
    wait_inflight(&owner);
    let notification = Arc::new(CapacityWake::default());
    let waker = std::task::Waker::from(notification.clone());
    let mut cx = Context::from_waker(&waker);
    let tail: Arc<[u8]> = Arc::from(&b"history-end"[..]);
    assert!(fanout.poll_send_to(1, &tail, &mut cx).is_pending());
    assert!(!owner.shared.queue.lock().unwrap().aborted);
    let reading = thread::spawn(move || {
        let mut bytes = vec![0; 16 * 1024 * 1024];
        reader.read_exact(&mut bytes).unwrap();
        assert!(bytes.iter().all(|&byte| byte == 4));
        let mut end = [0; 11];
        reader.read_exact(&mut end).unwrap();
        assert_eq!(&end, b"history-end");
    });
    notification.wait();
    assert_eq!(fanout.poll_send_to(1, &tail, &mut cx), Poll::Ready(Ok(())));
    reading.join().unwrap();
    fanout.close();
}

#[test]
fn close_wakes_a_capacity_waiter_with_closed_and_an_oversized_frame_never_waits() {
    let fanout = Fanout::with_limits(Limits {
        frames: 1,
        stall_timeout: Duration::from_secs(10),
        close_grace: Duration::from_millis(50),
        ..short_limits()
    });
    let (socket, _reader) = pair();
    fanout.add(1, socket).unwrap();
    let notification = Arc::new(CapacityWake::default());
    let waker = std::task::Waker::from(notification.clone());
    let mut cx = Context::from_waker(&waker);
    assert_eq!(
        fanout.poll_send_to(1, &vec![0; fanout.limits.bytes + 1].into(), &mut cx),
        Poll::Ready(Err(SendError::FrameTooLarge))
    );
    fanout.send_to(1, vec![4; 16 * 1024 * 1024].into());
    let tail: Arc<[u8]> = Arc::from(&b"end"[..]);
    assert!(fanout.poll_send_to(1, &tail, &mut cx).is_pending());
    fanout.close();
    notification.wait();
    assert_eq!(
        fanout.poll_send_to(1, &tail, &mut cx),
        Poll::Ready(Err(SendError::Closed))
    );
}
