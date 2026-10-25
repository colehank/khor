use super::*;
use futures::{AsyncReadExt, AsyncWriteExt};
use std::{sync::mpsc, time::Duration};

#[test]
fn closing_an_empty_input_queue_wakes_its_worker() {
    let queue = Queue::<usize>::new();
    let other = queue.clone();
    let worker = std::thread::spawn(move || other.recv());
    queue.close();
    assert_eq!(worker.join().unwrap(), None);
}

#[test]
fn closing_a_full_output_queue_releases_its_blocked_reader() {
    let queue = Queue::new();
    for n in 0..SLOTS {
        assert!(queue.push(n));
    }
    let other = queue.clone();
    let (tx, rx) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let result = other.push(99);
        tx.send(result).unwrap();
    });
    assert!(rx.recv_timeout(Duration::from_millis(20)).is_err());
    queue.close();
    assert!(!rx.recv_timeout(Duration::from_secs(1)).unwrap());
    worker.join().unwrap();
}

#[test]
fn bounded_stdin_backpressure_is_cancelled_without_accepting_more_bytes() {
    let queue = Queue::new();
    let mut writer = Writer {
        queue: queue.clone(),
        writes: VecDeque::new(),
        flushing: None,
        accepted: 0,
        flushed: 0,
        closed: false,
    };
    let waker = futures::task::noop_waker();
    let mut cx = Context::from_waker(&waker);
    for _ in 0..SLOTS {
        assert!(matches!(
            Pin::new(&mut writer).poll_write(&mut cx, &[1; CHUNK]),
            Poll::Ready(Ok(CHUNK))
        ));
    }
    assert!(
        Pin::new(&mut writer)
            .poll_write(&mut cx, &[2; CHUNK])
            .is_pending()
    );
    assert_eq!(queue.state.lock().unwrap().items.len(), SLOTS);
    queue.close();
    assert!(matches!(
        Pin::new(&mut writer).poll_write(&mut cx, b"later"),
        Poll::Ready(Err(_))
    ));
}

struct FlushGate {
    bytes: Arc<Mutex<Vec<u8>>>,
    entered: mpsc::Sender<()>,
    release: mpsc::Receiver<()>,
}
impl Write for FlushGate {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.bytes.lock().unwrap().extend(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        self.entered.send(()).unwrap();
        self.release.recv().unwrap();
        Ok(())
    }
}
#[test]
fn flush_waits_for_the_actual_os_writer_acknowledgment() {
    let (entered_tx, entered) = mpsc::channel();
    let (release, release_rx) = mpsc::channel();
    let (done_tx, done) = mpsc::channel();
    let bytes = Arc::new(Mutex::new(Vec::new()));
    let (mut writer, queue, worker) = write_pipe(Box::new(FlushGate {
        bytes: bytes.clone(),
        entered: entered_tx,
        release: release_rx,
    }))
    .unwrap();
    let caller = std::thread::spawn(move || {
        let result = futures::executor::block_on(async {
            writer.write_all(b"complete frame").await?;
            writer.flush().await
        });
        done_tx.send(result).unwrap();
    });
    entered.recv_timeout(Duration::from_secs(1)).unwrap();
    assert!(
        done.try_recv().is_err(),
        "queue admission was reported as an OS flush"
    );
    assert_eq!(&*bytes.lock().unwrap(), b"complete frame");
    release.send(()).unwrap();
    done.recv_timeout(Duration::from_secs(1)).unwrap().unwrap();
    caller.join().unwrap();
    queue.close();
    worker.join().unwrap();
}

#[test]
fn read_eof_retains_all_preceding_buffered_bytes() {
    let bytes = vec![b'x'; CHUNK * 3 + 5];
    let expected = bytes.clone();
    let (mut reader, queue, worker) = read_pipe(Box::new(std::io::Cursor::new(bytes))).unwrap();
    let mut actual = Vec::new();
    futures::executor::block_on(reader.read_to_end(&mut actual)).unwrap();
    assert_eq!(actual, expected);
    drop(reader);
    queue.close();
    worker.join().unwrap();
}

struct FailedFlush;
impl Write for FailedFlush {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::BrokenPipe,
            "synthetic physical flush failure",
        ))
    }
}
#[test]
fn an_actual_flush_error_is_not_an_admission_success() {
    let (mut writer, queue, worker) = write_pipe(Box::new(FailedFlush)).unwrap();
    let result = futures::executor::block_on(async {
        writer.write_all(b"frame").await?;
        writer.flush().await
    });
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::BrokenPipe);
    drop(writer);
    queue.close();
    worker.join().unwrap();
}
