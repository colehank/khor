//! Bounded, independent socket writers for already encoded GUI frames.
//!
//! The caller owns framing and the reader half. On reader EOF it must call
//! `remove`, so an idle writer is retired even when there is nothing to send.

use std::collections::{HashMap, VecDeque};
use std::io::{self, Write};
use std::net::{Shutdown, TcpStream};
use std::sync::{Arc, Condvar, Mutex};
use std::task::{Context, Poll, Waker};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

// This is the host's per-connection backlog budget, not a wire-format cap:
// host::write_frame has no outgoing size limit. The host caps replay at 32 MiB;
// allow one such encoded frame plus subsequent live notes. The frame currently
// being written still counts against both budgets.
const MAX_FRAMES: usize = 128;
const MAX_BYTES: usize = 64 * 1024 * 1024;
const WRITE_TIMEOUT: Duration = Duration::from_millis(100);
const STALL_TIMEOUT: Duration = Duration::from_secs(2);
const CLOSE_GRACE: Duration = Duration::from_secs(2);

#[derive(Clone, Copy)]
struct Limits {
    frames: usize,
    bytes: usize,
    write_timeout: Duration,
    stall_timeout: Duration,
    close_grace: Duration,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            frames: MAX_FRAMES,
            bytes: MAX_BYTES,
            write_timeout: WRITE_TIMEOUT,
            stall_timeout: STALL_TIMEOUT,
            close_grace: CLOSE_GRACE,
        }
    }
}

#[derive(Default)]
struct Queue {
    frames: VecDeque<Arc<[u8]>>,
    outstanding_frames: usize,
    outstanding_bytes: usize,
    draining: bool,
    aborted: bool,
    done: bool,
    capacity_waiters: Vec<Waker>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SendError {
    Closed,
    FrameTooLarge,
    TooManyWaiters,
}

fn wake_capacity(waiters: Vec<Waker>) {
    for waiter in waiters {
        waiter.wake();
    }
}

struct Shared {
    queue: Mutex<Queue>,
    changed: Condvar,
    // A separate handle lets close/reader EOF wake a blocked OS write.
    socket: TcpStream,
    limits: Limits,
}

impl Shared {
    fn abort(&self) {
        let waiters = {
            let mut queue = self.queue.lock().unwrap();
            queue.aborted = true;
            queue.frames.clear();
            self.changed.notify_all();
            std::mem::take(&mut queue.capacity_waiters)
        };
        let _ = self.socket.shutdown(Shutdown::Both);
        wake_capacity(waiters);
    }

    fn poll_enqueue(&self, frame: &Arc<[u8]>, cx: &mut Context<'_>) -> Poll<Result<(), SendError>> {
        let mut queue = self.queue.lock().unwrap();
        if queue.aborted || queue.draining || queue.done {
            return Poll::Ready(Err(SendError::Closed));
        }
        if frame.len() > self.limits.bytes {
            return Poll::Ready(Err(SendError::FrameTooLarge));
        }
        if queue.outstanding_frames >= self.limits.frames
            || frame.len() > self.limits.bytes.saturating_sub(queue.outstanding_bytes)
        {
            if !queue
                .capacity_waiters
                .iter()
                .any(|waiter| waiter.will_wake(cx.waker()))
            {
                // Waiters are bounded too; a caller cannot build an unbounded
                // side queue of suspended publishers on one connection.
                if queue.capacity_waiters.len() >= self.limits.frames {
                    return Poll::Ready(Err(SendError::TooManyWaiters));
                }
                queue.capacity_waiters.push(cx.waker().clone());
            }
            return Poll::Pending;
        }
        queue.outstanding_frames += 1;
        queue.outstanding_bytes += frame.len();
        queue.frames.push_back(frame.clone());
        self.changed.notify_one();
        Poll::Ready(Ok(()))
    }

    fn enqueue(&self, frame: Arc<[u8]>) {
        let overflow = {
            let mut queue = self.queue.lock().unwrap();
            if queue.aborted || queue.draining || queue.done {
                return;
            }
            if queue.outstanding_frames >= self.limits.frames
                || frame.len() > self.limits.bytes.saturating_sub(queue.outstanding_bytes)
            {
                // Commit retirement under the same lock as admission: no new
                // frame can slip in while shutdown happens outside the lock.
                queue.aborted = true;
                queue.frames.clear();
                self.changed.notify_all();
                true
            } else {
                queue.outstanding_frames += 1;
                queue.outstanding_bytes += frame.len();
                queue.frames.push_back(frame);
                self.changed.notify_one();
                false
            }
        };
        if overflow {
            self.abort();
        }
    }
}

struct Client {
    shared: Arc<Shared>,
    writer: Mutex<Option<JoinHandle<()>>>,
}

impl Client {
    fn join(&self) {
        if let Some(writer) = self.writer.lock().unwrap().take() {
            let _ = writer.join();
        }
    }
}

#[derive(Default)]
struct Registry {
    clients: HashMap<u64, Arc<Client>>,
    closed: bool,
}

pub(super) struct Fanout {
    registry: Mutex<Registry>,
    // Concurrent close callers all wait for the same completed cleanup.
    closing: Mutex<()>,
    limits: Limits,
}

impl Fanout {
    pub(super) fn new() -> Self {
        Self::with_limits(Limits::default())
    }

    fn with_limits(limits: Limits) -> Self {
        Self {
            registry: Mutex::new(Registry::default()),
            closing: Mutex::new(()),
            limits,
        }
    }

    pub(super) fn add(&self, who: u64, socket: TcpStream) -> io::Result<()> {
        socket.set_write_timeout(Some(self.limits.write_timeout))?;
        let shared = Arc::new(Shared {
            queue: Mutex::new(Queue::default()),
            changed: Condvar::new(),
            socket: socket.try_clone()?,
            limits: self.limits,
        });
        // Reap completed writers on admission, never on the send path. A
        // closed reader's slot cannot accumulate across successive attaches.
        let retired = {
            let mut registry = self.registry.lock().unwrap();
            if registry.closed {
                return Err(io::Error::new(io::ErrorKind::BrokenPipe, "fanout closed"));
            }
            if registry.clients.contains_key(&who) {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "client already attached",
                ));
            }
            // Spawn before removing completed handles, so a failed thread
            // creation cannot drop their join ownership on the error path.
            let worker_shared = shared.clone();
            let writer = thread::Builder::new()
                .name("gui-fanout".into())
                .spawn(move || run_writer(socket, worker_shared))?;
            let finished: Vec<_> = registry
                .clients
                .iter()
                .filter_map(|(&id, client)| client.shared.queue.lock().unwrap().done.then_some(id))
                .collect();
            let retired: Vec<_> = finished
                .into_iter()
                .filter_map(|id| registry.clients.remove(&id))
                .collect();
            registry.clients.insert(
                who,
                Arc::new(Client {
                    shared,
                    writer: Mutex::new(Some(writer)),
                }),
            );
            retired
        };
        for client in retired {
            client.join();
        }
        Ok(())
    }

    /// Broadcast one complete frame; admission performs no socket writes.
    pub(super) fn send(&self, frame: Arc<[u8]>) {
        let clients: Vec<_> = {
            let registry = self.registry.lock().unwrap();
            if registry.closed {
                return;
            }
            registry.clients.values().cloned().collect()
        };
        for client in clients {
            client.shared.enqueue(frame.clone());
        }
    }

    pub(super) fn send_to(&self, who: u64, frame: Arc<[u8]>) {
        let client = {
            let registry = self.registry.lock().unwrap();
            if registry.closed {
                return;
            }
            registry.clients.get(&who).cloned()
        };
        if let Some(client) = client {
            client.shared.enqueue(frame);
        }
    }

    /// History producers await actual queue space without retiring a reader
    /// merely because a finite replay is produced faster than socket writes.
    /// Live `send`/`send_to` retain their overflow-retirement policy.
    pub(super) fn poll_send_to(
        &self,
        who: u64,
        frame: &Arc<[u8]>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), SendError>> {
        let client = {
            let registry = self.registry.lock().unwrap();
            if registry.closed {
                return Poll::Ready(Err(SendError::Closed));
            }
            registry.clients.get(&who).cloned()
        };
        match client {
            Some(client) => client.shared.poll_enqueue(frame, cx),
            None => Poll::Ready(Err(SendError::Closed)),
        }
    }

    /// Reader EOF/error retires this socket, including a blocked writer.
    pub(super) fn remove(&self, who: u64) {
        let client = self.registry.lock().unwrap().clients.get(&who).cloned();
        if let Some(client) = client {
            client.shared.abort();
        }
    }

    /// Drain admitted frames (including Gone), then shut down and join every
    /// writer. One shared grace bounds close even with many stalled clients.
    pub(super) fn close(&self) {
        let _closing = self.closing.lock().unwrap();
        let clients: Vec<_> = {
            let mut registry = self.registry.lock().unwrap();
            registry.closed = true;
            registry.clients.drain().map(|(_, client)| client).collect()
        };
        let deadline = Instant::now() + self.limits.close_grace;
        for client in &clients {
            let mut queue = client.shared.queue.lock().unwrap();
            queue.draining = true;
            client.shared.changed.notify_all();
        }
        for client in &clients {
            let mut queue = client.shared.queue.lock().unwrap();
            while !queue.done {
                let now = Instant::now();
                if now >= deadline {
                    break;
                }
                let (next, _) = client
                    .shared
                    .changed
                    .wait_timeout(queue, deadline - now)
                    .unwrap();
                queue = next;
            }
        }
        for client in &clients {
            client.shared.abort();
        }
        for client in clients {
            client.join();
        }
    }
}

impl Drop for Fanout {
    fn drop(&mut self) {
        self.close();
    }
}

fn run_writer(mut socket: TcpStream, shared: Arc<Shared>) {
    loop {
        let frame = {
            let mut queue = shared.queue.lock().unwrap();
            while queue.frames.is_empty() && !queue.aborted && !queue.draining {
                queue = shared.changed.wait(queue).unwrap();
            }
            if queue.aborted {
                break;
            }
            match queue.frames.pop_front() {
                Some(frame) => frame,
                None => break,
            }
        };
        if write_frame(&mut socket, &shared, &frame).is_err() {
            break;
        }
        let waiters = {
            let mut queue = shared.queue.lock().unwrap();
            queue.outstanding_frames -= 1;
            queue.outstanding_bytes -= frame.len();
            std::mem::take(&mut queue.capacity_waiters)
        };
        wake_capacity(waiters);
    }
    let _ = socket.shutdown(Shutdown::Both);
    let waiters = {
        let mut queue = shared.queue.lock().unwrap();
        queue.aborted = true;
        queue.frames.clear();
        queue.outstanding_frames = 0;
        queue.outstanding_bytes = 0;
        queue.done = true;
        shared.changed.notify_all();
        std::mem::take(&mut queue.capacity_waiters)
    };
    wake_capacity(waiters);
}

fn write_frame(socket: &mut TcpStream, shared: &Shared, frame: &[u8]) -> io::Result<()> {
    let mut remaining = frame;
    let mut progressed = Instant::now();
    while !remaining.is_empty() {
        if shared.queue.lock().unwrap().aborted {
            return Err(io::ErrorKind::Interrupted.into());
        }
        if progressed.elapsed() >= shared.limits.stall_timeout {
            return Err(io::ErrorKind::TimedOut.into());
        }
        // Bound each syscall as well as its duration. Successful partial writes
        // reset the stall clock, so a large replay can keep making progress.
        let chunk = &remaining[..remaining.len().min(64 * 1024)];
        match socket.write(chunk) {
            Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
            Ok(written) => {
                remaining = &remaining[written..];
                progressed = Instant::now();
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                if progressed.elapsed() >= shared.limits.stall_timeout {
                    return Err(io::ErrorKind::TimedOut.into());
                }
            }
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "gui_host_fanout_tests.rs"]
mod tests;
