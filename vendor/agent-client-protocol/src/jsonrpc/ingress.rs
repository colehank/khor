//! Per-physical-input ownership, retained through adapters until dispatch/drop.
use futures::task::AtomicWaker;
use std::sync::{Arc, Mutex};
use std::task::Poll;

/// Encoded ingress ownership for one physical transport; not a process RSS limit.
#[derive(Debug)]
pub struct IngressBudget {
    book: Arc<Book>,
}
/// A payload-free view of the same ingress owner. Cloning cannot create a producer.
#[derive(Clone, Debug)]
pub struct IngressObserver(Arc<Book>);
/// Current and lifetime-high encoded ownership; line/decode terms overlap as documented.
#[derive(Clone, Copy, Debug, Default)]
pub struct IngressStats {
    /// Encoded byte allowance; a larger single frame is an explicit exclusive case.
    pub window_bytes: usize,
    /// Maximum admitted frames, including the currently dispatched frame.
    pub window_frames: usize,
    /// Actual bytes charged, never min(frame size, window size).
    pub charged_bytes: usize,
    /// Number of live frame charges, including the one being parsed.
    pub charged_frames: usize,
    /// Charged bytes already parsed into frames, not a measurement of Rust heap expansion.
    pub decoded_frame_bytes: usize,
    /// Complete undecoded line currently held by the reader; may be waiting for admission.
    pub undecoded_line_bytes: usize,
    /// Largest actual charged byte total.
    pub peak_charged_bytes: usize,
    /// Largest actual charged frame count.
    pub peak_charged_frames: usize,
    /// Largest parsed-frame encoded charge total.
    pub peak_decoded_frame_bytes: usize,
    /// Largest completed line before parsing. The upstream partial-line buffer has no size cap.
    pub largest_line_bytes: usize,
    /// Largest individually admitted frame above the window, charged at its full size.
    pub largest_oversize_bytes: usize,
    /// Whether this producer is currently waiting for capacity.
    pub waiting: bool,
    /// Whether the physical input owner has ended (queued frames may still hold charges).
    pub closed: bool,
}
impl IngressStats {
    /// Charged bytes in raw-to-decode transfer; the charge is not lost between phases.
    pub fn reserved_parse_bytes(&self) -> usize {
        self.charged_bytes - self.decoded_frame_bytes
    }
}
#[derive(Debug)]
struct Book {
    state: Mutex<IngressStats>,
    available: AtomicWaker,
}
impl Book {
    fn state(&self) -> std::sync::MutexGuard<'_, IngressStats> {
        self.state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }
}
impl Default for IngressBudget {
    fn default() -> Self {
        Self::new(8 * 1024 * 1024, 256).expect("nonzero ingress constants")
    }
}
impl IngressBudget {
    /// Select one input's encoded byte/frame allowance. No global history-size limit is imposed.
    pub fn new(bytes: usize, frames: usize) -> Result<Self, crate::Error> {
        if bytes == 0 || frames == 0 {
            return Err(crate::util::internal_error(
                "ingress limits must be nonzero",
            ));
        }
        Ok(Self {
            book: Arc::new(Book {
                state: Mutex::new(IngressStats {
                    window_bytes: bytes,
                    window_frames: frames,
                    ..IngressStats::default()
                }),
                available: AtomicWaker::new(),
            }),
        })
    }
    /// Observe this owner without gaining admission or a second producer.
    pub fn observer(&self) -> IngressObserver {
        IngressObserver(self.book.clone())
    }
    pub(super) fn line(&self, bytes: usize) -> Line {
        let mut state = self.book.state();
        state.undecoded_line_bytes = bytes;
        state.largest_line_bytes = state.largest_line_bytes.max(bytes);
        Line(self.book.clone())
    }
    pub(super) async fn admit(&self, bytes: usize) -> Result<FrameAdmission, crate::Error> {
        futures::future::poll_fn(|cx| {
            // One physical producer owns this waker. Register before checking
            // capacity so a last-credit drop cannot race us into a lost sleep.
            self.book.available.register(cx.waker());
            let mut state = self.book.state();
            if state.closed {
                drop(state);
                drop(self.book.available.take());
                return Poll::Ready(Err(crate::util::internal_error("ingress is closed")));
            }
            let fits = state.charged_frames < state.window_frames
                && if bytes > state.window_bytes {
                    // Preserve legal giant single frames at their full size,
                    // exclusively; the window never becomes a history-size cap.
                    state.charged_frames == 0
                } else {
                    state
                        .charged_bytes
                        .checked_add(bytes)
                        .is_some_and(|total| total <= state.window_bytes)
                };
            if !fits {
                state.waiting = true;
                return Poll::Pending;
            }
            state.waiting = false;
            let charged_bytes = state
                .charged_bytes
                .checked_add(bytes)
                .ok_or_else(|| crate::util::internal_error("ingress byte accounting overflow"))?;
            let charged_frames = state
                .charged_frames
                .checked_add(1)
                .ok_or_else(|| crate::util::internal_error("ingress frame accounting overflow"))?;
            state.charged_bytes = charged_bytes;
            state.charged_frames = charged_frames;
            state.peak_charged_bytes = state.peak_charged_bytes.max(charged_bytes);
            state.peak_charged_frames = state.peak_charged_frames.max(charged_frames);
            if bytes > state.window_bytes {
                state.largest_oversize_bytes = state.largest_oversize_bytes.max(bytes);
            }
            drop(state);
            drop(self.book.available.take());
            Poll::Ready(Ok(FrameAdmission(Arc::new(Receipt {
                book: self.book.clone(),
                bytes,
                decoded: false,
            }))))
        })
        .await
    }
}
impl Drop for IngressBudget {
    fn drop(&mut self) {
        let mut state = self.book.state();
        state.closed = true;
        state.waiting = false;
        drop(state);
        self.book.available.wake();
    }
}
impl IngressObserver {
    /// Read resource counters without inspecting a payload or RPC identity.
    pub fn stats(&self) -> IngressStats {
        *self.0.state()
    }
}
pub(super) struct Line(Arc<Book>);
impl Drop for Line {
    fn drop(&mut self) {
        self.0.state().undecoded_line_bytes = 0;
    }
}
/// Opaque ownership of one input frame; clones share its lifetime, not a second charge.
#[derive(Clone, Debug)]
pub struct FrameAdmission(Arc<Receipt>);
#[derive(Debug)]
struct Receipt {
    book: Arc<Book>,
    bytes: usize,
    decoded: bool,
}
impl FrameAdmission {
    pub(super) fn into_frame(mut self) -> Self {
        let receipt = Arc::get_mut(&mut self.0).expect("unpublished admission is unique");
        receipt.decoded = true;
        let mut state = receipt.book.state();
        state.decoded_frame_bytes += receipt.bytes;
        state.peak_decoded_frame_bytes = state
            .peak_decoded_frame_bytes
            .max(state.decoded_frame_bytes);
        drop(state);
        self
    }
}
impl Drop for Receipt {
    fn drop(&mut self) {
        let mut state = self.book.state();
        state.charged_bytes -= self.bytes;
        state.charged_frames -= 1;
        if self.decoded {
            state.decoded_frame_bytes -= self.bytes;
        }
        drop(state);
        self.book.available.wake();
    }
}
