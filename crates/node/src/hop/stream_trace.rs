//! Opt-in observation of the actual carried byte streams. No payload is logged.
use super::*;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

struct Tap<T> {
    inner: T,
    peer: EndpointId,
    role: &'static str,
    wire: Option<usize>,
    stream: String,
    read: u64,
    written: u64,
    last_read_at: Option<u64>,
    last_write_at: Option<u64>,
    flushed_written: Option<u64>,
    pending: [bool; 4],
}

impl<T> Tap<T> {
    fn new(
        inner: T,
        peer: EndpointId,
        role: &'static str,
        wire: Option<usize>,
        stream: String,
    ) -> Self {
        Self {
            inner,
            peer,
            role,
            wire,
            stream,
            read: 0,
            written: 0,
            last_read_at: None,
            last_write_at: None,
            flushed_written: None,
            pending: [false; 4],
        }
    }

    fn record(&mut self, index: usize, operation: &str, state: &str, bytes: usize) {
        let was_pending = self.pending[index];
        if state == "pending" && was_pending {
            return;
        }
        self.pending[index] = state == "pending";
        if bytes > 0 {
            let at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_millis() as u64);
            if index == 0 {
                self.read = self.read.saturating_add(bytes as u64);
                self.last_read_at = Some(at);
            } else if index == 1 {
                self.written = self.written.saturating_add(bytes as u64);
                self.last_write_at = Some(at);
            }
            // Keep exact counters without serializing every successful I/O.
            // A subsequent wait/flush/end reports the accumulated progress.
            return;
        }
        if operation == "flush" && state == "ready" {
            if self.flushed_written == Some(self.written) && !was_pending {
                return;
            }
            self.flushed_written = Some(self.written);
        }
        khor_net::carry::observe_lifecycle(self.peer, "hop_stream_io", || {
            serde_json::json!({
                "role": self.role, "wire": self.wire, "stream": self.stream,
                "operation": operation, "state": state, "bytes": bytes,
                "read": self.read, "written": self.written,
                "last_read_at": self.last_read_at, "last_write_at": self.last_write_at,
            })
        });
    }
}

impl<T: AsyncRead + Unpin> AsyncRead for Tap<T> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let before = buf.filled().len();
        let result = Pin::new(&mut self.inner).poll_read(cx, buf);
        match &result {
            Poll::Pending => self.record(0, "read", "pending", 0),
            Poll::Ready(Ok(())) => self.record(0, "read", "ready", buf.filled().len() - before),
            Poll::Ready(Err(_)) => self.record(0, "read", "error", 0),
        }
        result
    }
}

impl<T: AsyncWrite + Unpin> AsyncWrite for Tap<T> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        let result = Pin::new(&mut self.inner).poll_write(cx, bytes);
        match &result {
            Poll::Pending => self.record(1, "write", "pending", 0),
            Poll::Ready(Ok(n)) => self.record(1, "write", "ready", *n),
            Poll::Ready(Err(_)) => self.record(1, "write", "error", 0),
        }
        result
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let result = Pin::new(&mut self.inner).poll_flush(cx);
        match &result {
            Poll::Pending => self.record(2, "flush", "pending", 0),
            Poll::Ready(Ok(())) => self.record(2, "flush", "ready", 0),
            Poll::Ready(Err(_)) => self.record(2, "flush", "error", 0),
        }
        result
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let result = Pin::new(&mut self.inner).poll_shutdown(cx);
        match &result {
            Poll::Pending => self.record(3, "shutdown", "pending", 0),
            Poll::Ready(Ok(())) => self.record(3, "shutdown", "ready", 0),
            Poll::Ready(Err(_)) => self.record(3, "shutdown", "error", 0),
        }
        result
    }
}

/// The normal branch retains the original I/O types and hot path.
pub(super) fn attach_if(
    links: &khor_net::carry::Links,
    peer: EndpointId,
    carrier: Carrier,
    wire: Option<usize>,
    role: &'static str,
    pair: (SendStream, RecvStream),
    allow: impl FnOnce(Option<Carrier>) -> bool,
) -> Option<khor_net::carry::Attached> {
    let (send, recv) = pair;
    if std::env::var_os("KHOR_NET_DIAGNOSTICS").is_none() {
        return links.attach_if(peer, carrier, recv, send, allow);
    }
    let send_id = format!("{:?}", send.id());
    let recv_id = format!("{:?}", recv.id());
    let attached = links.attach_if(
        peer,
        carrier,
        Tap::new(recv, peer, role, wire, recv_id.clone()),
        Tap::new(send, peer, role, wire, send_id.clone()),
        allow,
    )?;
    khor_net::carry::observe_lifecycle(peer, "hop_stream_bound", || {
        serde_json::json!({"role": role, "wire": wire, "attachment": attached.serial(),
            "send_stream": send_id, "recv_stream": recv_id})
    });
    Some(attached)
}

/// Wait for one complete transport frame without publishing its stream.
/// The existing u16 framing bounds this preparation to at most 65537 bytes.
pub(super) async fn first_frame(recv: &mut RecvStream) -> Option<Vec<u8>> {
    let mut header = [0; 2];
    recv.read_exact(&mut header).await.ok()?;
    let len = usize::from(u16::from_be_bytes(header));
    if len == 0 {
        return None;
    }
    let mut prefix = vec![0; len + 2];
    prefix[..2].copy_from_slice(&header);
    recv.read_exact(&mut prefix[2..]).await.ok()?;
    Some(prefix)
}

/// Preserve the first frame when a prepared receiver commits its stream.
pub(super) fn attach_prefixed(
    links: &khor_net::carry::Links,
    peer: EndpointId,
    carrier: Carrier,
    wire: Option<usize>,
    role: &'static str,
    pair: (SendStream, RecvStream),
    prefix: Vec<u8>,
) -> khor_net::carry::Attached {
    use tokio::io::AsyncReadExt;
    let (send, recv) = pair;
    if std::env::var_os("KHOR_NET_DIAGNOSTICS").is_none() {
        return links.attach(
            peer,
            carrier,
            std::io::Cursor::new(prefix).chain(recv),
            send,
        );
    }
    let send_id = format!("{:?}", send.id());
    let recv_id = format!("{:?}", recv.id());
    let attached = links.attach(
        peer,
        carrier,
        Tap::new(
            std::io::Cursor::new(prefix).chain(recv),
            peer,
            role,
            wire,
            recv_id.clone(),
        ),
        Tap::new(send, peer, role, wire, send_id.clone()),
    );
    khor_net::carry::observe_lifecycle(peer, "hop_stream_bound", || {
        serde_json::json!({"role": role, "wire": wire, "attachment": attached.serial(),
            "send_stream": send_id, "recv_stream": recv_id, "prepared_frame": true})
    });
    attached
}

/// Same two copies, finish calls and cancellation choice as the normal splice.
pub(super) async fn splice(
    a: (SendStream, RecvStream),
    b: (SendStream, RecvStream),
    peer: EndpointId,
    wires: (Option<usize>, usize),
) {
    if std::env::var_os("KHOR_NET_DIAGNOSTICS").is_none() {
        return super::splice(a, b).await;
    }
    let tap_send = |stream: SendStream, role, wire| {
        let id = format!("{:?}", stream.id());
        Tap::new(stream, peer, role, wire, id)
    };
    let tap_recv = |stream: RecvStream, role, wire| {
        let id = format!("{:?}", stream.id());
        Tap::new(stream, peer, role, wire, id)
    };
    let ((a_send, a_recv), (b_send, b_recv)) = (a, b);
    let (mut a_send, mut a_recv) = (
        tap_send(a_send, "middle_in", wires.0),
        tap_recv(a_recv, "middle_in", wires.0),
    );
    let (mut b_send, mut b_recv) = (
        tap_send(b_send, "middle_out", Some(wires.1)),
        tap_recv(b_recv, "middle_out", Some(wires.1)),
    );
    let there = async {
        let _ = tokio::io::copy(&mut a_recv, &mut b_send).await;
        let _ = b_send.inner.finish();
        b_send.record(3, "finish", "called", 0);
    };
    let back = async {
        let _ = tokio::io::copy(&mut b_recv, &mut a_send).await;
        let _ = a_send.inner.finish();
        a_send.record(3, "finish", "called", 0);
    };
    tokio::select! {
        () = there => {}
        () = back => {}
    }
}
