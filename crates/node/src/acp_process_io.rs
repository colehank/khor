use futures::io::{AsyncRead, AsyncWrite};
use std::{
    collections::VecDeque,
    future::Future,
    io::{self, Read, Write},
    pin::Pin,
    sync::{Arc, Condvar, Mutex},
    task::{Context, Poll, Waker},
    thread::JoinHandle,
};
use tokio::sync::oneshot;

pub(super) const CHUNK: usize = 8 * 1024;
const SLOTS: usize = 8;
struct State<T> {
    items: VecDeque<T>,
    closed: bool,
    ended: bool,
    readable: Option<Waker>,
    writable: Option<Waker>,
}
pub(super) struct Queue<T> {
    state: Mutex<State<T>>,
    changed: Condvar,
}
impl<T> Queue<T> {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(State {
                items: VecDeque::new(),
                closed: false,
                ended: false,
                readable: None,
                writable: None,
            }),
            changed: Condvar::new(),
        })
    }
    pub(super) fn close(&self) {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        state.closed = true;
        state.items.clear();
        let readable = state.readable.take();
        let writable = state.writable.take();
        drop(state);
        self.changed.notify_all();
        if let Some(waker) = readable {
            waker.wake();
        }
        if let Some(waker) = writable {
            waker.wake();
        }
    }
    fn end(&self) {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        state.ended = true;
        let wake = state.readable.take();
        drop(state);
        self.changed.notify_all();
        if let Some(wake) = wake {
            wake.wake();
        }
    }
    fn push(&self, item: T) -> bool {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        while state.items.len() == SLOTS && !state.closed && !state.ended {
            state = self.changed.wait(state).unwrap_or_else(|p| p.into_inner());
        }
        if state.closed || state.ended {
            return false;
        }
        state.items.push_back(item);
        let wake = state.readable.take();
        drop(state);
        self.changed.notify_all();
        if let Some(wake) = wake {
            wake.wake();
        }
        true
    }
    fn try_push(&self, item: T, cx: &Context<'_>) -> Poll<io::Result<()>> {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        if state.closed || state.ended {
            return Poll::Ready(Err(closed()));
        }
        if state.items.len() == SLOTS {
            state.writable = Some(cx.waker().clone());
            return Poll::Pending;
        }
        state.items.push_back(item);
        let wake = state.readable.take();
        drop(state);
        self.changed.notify_all();
        if let Some(wake) = wake {
            wake.wake();
        }
        Poll::Ready(Ok(()))
    }
    fn recv(&self) -> Option<T> {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        while state.items.is_empty() && !state.closed && !state.ended {
            state = self.changed.wait(state).unwrap_or_else(|p| p.into_inner());
        }
        let item = state.items.pop_front();
        let wake = state.writable.take();
        drop(state);
        self.changed.notify_all();
        if let Some(wake) = wake {
            wake.wake();
        }
        item
    }
    fn poll_recv(&self, cx: &Context<'_>) -> Poll<Option<T>> {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        let item = state.items.pop_front();
        if item.is_none() && !state.closed && !state.ended {
            state.readable = Some(cx.waker().clone());
            return Poll::Pending;
        }
        let wake = state.writable.take();
        drop(state);
        self.changed.notify_all();
        if let Some(wake) = wake {
            wake.wake();
        }
        Poll::Ready(item)
    }
}
fn closed() -> io::Error {
    io::Error::new(io::ErrorKind::BrokenPipe, "ACP pipe is closed")
}

pub(crate) struct Reader {
    queue: Arc<Queue<io::Result<Vec<u8>>>>,
    bytes: Vec<u8>,
    at: usize,
}
impl AsyncRead for Reader {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        into: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        if into.is_empty() {
            return Poll::Ready(Ok(0));
        }
        if self.at == self.bytes.len() {
            match self.queue.poll_recv(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(None) => return Poll::Ready(Ok(0)),
                Poll::Ready(Some(Err(error))) => return Poll::Ready(Err(error)),
                Poll::Ready(Some(Ok(bytes))) => {
                    self.bytes = bytes;
                    self.at = 0;
                }
            }
        }
        let n = into.len().min(self.bytes.len() - self.at);
        into[..n].copy_from_slice(&self.bytes[self.at..self.at + n]);
        self.at += n;
        Poll::Ready(Ok(n))
    }
}
impl Drop for Reader {
    fn drop(&mut self) {
        self.queue.close();
    }
}

pub(super) enum WriteOp {
    Bytes(Vec<u8>, oneshot::Sender<io::Result<()>>),
    Flush(oneshot::Sender<io::Result<()>>),
}
type Ack = oneshot::Receiver<io::Result<()>>;
pub(crate) struct Writer {
    queue: Arc<Queue<WriteOp>>,
    writes: VecDeque<Ack>,
    flushing: Option<(u64, Ack)>,
    accepted: u64,
    flushed: u64,
    closed: bool,
}
impl Writer {
    fn poll_writes(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        while let Some(ack) = self.writes.front_mut() {
            match Pin::new(ack).poll(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Ok(result)) => {
                    self.writes.pop_front();
                    result?;
                }
                Poll::Ready(Err(_)) => return Poll::Ready(Err(closed())),
            }
        }
        Poll::Ready(Ok(()))
    }
}
impl AsyncWrite for Writer {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        if self.closed {
            return Poll::Ready(Err(closed()));
        }
        if let Poll::Ready(Err(error)) = self.poll_writes(cx) {
            return Poll::Ready(Err(error));
        }
        if bytes.is_empty() {
            return Poll::Ready(Ok(0));
        }
        if self.writes.len() > SLOTS {
            return Poll::Pending;
        }
        let next = self
            .accepted
            .checked_add(1)
            .ok_or_else(|| io::Error::other("ACP write generation exhausted"))?;
        let n = bytes.len().min(CHUNK);
        let (tx, rx) = oneshot::channel();
        match self
            .queue
            .try_push(WriteOp::Bytes(bytes[..n].to_vec(), tx), cx)
        {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
            Poll::Ready(Ok(())) => {
                self.writes.push_back(rx);
                self.accepted = next;
                Poll::Ready(Ok(n))
            }
        }
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.poll_writes(cx) {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(result) => {
                result?;
            }
        }
        if let Some((generation, ack)) = self.flushing.as_mut() {
            let generation = *generation;
            match Pin::new(ack).poll(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Ok(result)) => {
                    result?;
                    self.flushed = generation;
                    self.flushing = None;
                }
                Poll::Ready(Err(_)) => return Poll::Ready(Err(closed())),
            }
        }
        if self.flushed == self.accepted {
            return Poll::Ready(Ok(()));
        }
        let (tx, rx) = oneshot::channel();
        match self.queue.try_push(WriteOp::Flush(tx), cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
            Poll::Ready(Ok(())) => {
                self.flushing = Some((self.accepted, rx));
                self.poll_flush(cx)
            }
        }
    }
    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.as_mut().poll_flush(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(result) => {
                self.closed = true;
                self.queue.close();
                Poll::Ready(result)
            }
        }
    }
}
impl Drop for Writer {
    fn drop(&mut self) {
        self.queue.close();
    }
}

pub(super) fn read_pipe(
    mut pipe: Box<dyn Read + Send>,
) -> io::Result<(Reader, Arc<Queue<io::Result<Vec<u8>>>>, JoinHandle<()>)> {
    let queue = Queue::new();
    let output = queue.clone();
    let worker = std::thread::Builder::new()
        .name("acp-stdout".into())
        .spawn(move || {
            let mut bytes = [0u8; CHUNK];
            loop {
                match pipe.read(&mut bytes) {
                    Ok(0) => break,
                    Ok(n) => {
                        if !output.push(Ok(bytes[..n].to_vec())) {
                            break;
                        }
                    }
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                    Err(error) => {
                        let _ = output.push(Err(error));
                        break;
                    }
                }
            }
            output.end();
        })?;
    Ok((
        Reader {
            queue: queue.clone(),
            bytes: Vec::new(),
            at: 0,
        },
        queue,
        worker,
    ))
}
pub(super) fn write_pipe(
    mut pipe: Box<dyn Write + Send>,
) -> io::Result<(Writer, Arc<Queue<WriteOp>>, JoinHandle<()>)> {
    let queue = Queue::new();
    let input = queue.clone();
    let worker = std::thread::Builder::new()
        .name("acp-stdin".into())
        .spawn(move || {
            while let Some(op) = input.recv() {
                let (result, ack) = match op {
                    WriteOp::Bytes(bytes, ack) => (pipe.write_all(&bytes), ack),
                    WriteOp::Flush(ack) => (pipe.flush(), ack),
                };
                let failed = result.is_err();
                let _ = ack.send(result);
                if failed {
                    input.close();
                    break;
                }
            }
        })?;
    Ok((
        Writer {
            queue: queue.clone(),
            writes: VecDeque::new(),
            flushing: None,
            accepted: 0,
            flushed: 0,
            closed: false,
        },
        queue,
        worker,
    ))
}

#[cfg(test)]
#[path = "acp_process_io_tests.rs"]
mod tests;
