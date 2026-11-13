//! A caller-owned window including pending and completed out-of-order slices.
use std::{collections::VecDeque, future::Future, pin::Pin, task::Poll};

struct Slot<F: Future> {
    future: Option<Pin<Box<F>>>,
    output: Option<F::Output>,
}

pub(super) struct Window<F: Future> {
    pending: VecDeque<Slot<F>>,
    next: u64,
    first: bool,
    width: usize,
}

impl<F: Future> Window<F> {
    pub(super) fn new(offset: u64, width: usize) -> Self {
        assert!(width > 0);
        Self {
            pending: VecDeque::with_capacity(width),
            next: offset,
            first: true,
            width,
        }
    }

    pub(super) async fn next(
        &mut self,
        total: Option<u64>,
        mut ask: impl FnMut(u64) -> F,
    ) -> Option<F::Output> {
        let width = if total.is_some() { self.width } else { 1 };
        while self.pending.len() < width && total.map_or(self.first, |total| self.next < total) {
            let at = self.next;
            self.next = self.next.saturating_add(crate::proto::SLICE);
            self.first = false;
            self.pending.push_back(Slot {
                future: Some(Box::pin(ask(at))),
                output: None,
            });
        }
        std::future::poll_fn(|cx| {
            // Poll newly refilled requests even when an earlier result is
            // already ready. Ready responses keep their slot until committed.
            for slot in &mut self.pending {
                if let Some(future) = slot.future.as_mut() {
                    if let Poll::Ready(output) = future.as_mut().poll(cx) {
                        slot.output = Some(output);
                        slot.future = None;
                    }
                }
            }
            if self
                .pending
                .front()
                .is_some_and(|slot| slot.output.is_some())
            {
                Poll::Ready(self.pending.pop_front().and_then(|slot| slot.output))
            } else if self.pending.is_empty() {
                Poll::Ready(None)
            } else {
                Poll::Pending
            }
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    };
    use tokio::sync::Notify;

    use super::*;

    struct Live(Arc<AtomicUsize>);
    impl Live {
        fn new(count: &Arc<AtomicUsize>) -> Self {
            count.fetch_add(1, Ordering::SeqCst);
            Self(count.clone())
        }
    }
    impl Drop for Live {
        fn drop(&mut self) {
            self.0.fetch_sub(1, Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn slow_tail_does_not_hold_back_committed_prefix_refill() {
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            let release = Arc::new(Notify::new());
            let ninth = Arc::new(Notify::new());
            let seen = Arc::new(Mutex::new(Vec::new()));
            let mut window = Window::new(0, 8);
            let ask = |at| {
                let (release, ninth, seen) = (release.clone(), ninth.clone(), seen.clone());
                async move {
                    let index = at / crate::proto::SLICE;
                    seen.lock().unwrap().push(index);
                    if index == 8 {
                        release.notified().await;
                    }
                    if index == 9 {
                        ninth.notify_one();
                    }
                    index
                }
            };
            assert_eq!(window.next(None, ask).await, Some(0));
            for index in 1..8 {
                assert_eq!(
                    window.next(Some(12 * crate::proto::SLICE), ask).await,
                    Some(index)
                );
            }
            let next = window.next(Some(12 * crate::proto::SLICE), ask);
            tokio::pin!(next);
            tokio::select! {
                _ = &mut next => panic!("the uncommitted tail was deliberately held"),
                _ = ninth.notified() => {}
            }
            assert!(seen.lock().unwrap().contains(&9));
            release.notify_one();
            assert_eq!(next.await, Some(8));
        })
        .await
        .expect("logical refill completes without releasing the slow tail first");
    }

    #[tokio::test]
    async fn slow_head_keeps_completed_out_of_order_buffers_bounded() {
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            let release = Arc::new(Notify::new());
            let eighth = Arc::new(Notify::new());
            let seen = Arc::new(Mutex::new(Vec::new()));
            let buffers = Arc::new(AtomicUsize::new(0));
            let mut window = Window::new(0, 8);
            let ask = |at| {
                let (release, eighth, seen, buffers) = (
                    release.clone(),
                    eighth.clone(),
                    seen.clone(),
                    buffers.clone(),
                );
                async move {
                    let index = at / crate::proto::SLICE;
                    seen.lock().unwrap().push(index);
                    if index == 1 {
                        release.notified().await;
                    }
                    let output = (index, Live::new(&buffers));
                    if index == 8 {
                        eighth.notify_one();
                    }
                    output
                }
            };
            assert_eq!(window.next(None, ask).await.unwrap().0, 0);
            {
                let next = window.next(Some(20 * crate::proto::SLICE), ask);
                tokio::pin!(next);
                tokio::select! {
                    _ = &mut next => panic!("the first uncommitted slice was held"),
                    _ = eighth.notified() => {}
                }
                assert_eq!(buffers.load(Ordering::SeqCst), 7);
                assert!(!seen.lock().unwrap().contains(&9));
                release.notify_one();
                assert_eq!(next.await.unwrap().0, 1);
            }
            assert_eq!(
                window
                    .next(Some(20 * crate::proto::SLICE), ask)
                    .await
                    .unwrap()
                    .0,
                2
            );
            assert!(
                seen.lock().unwrap().contains(&9),
                "refill is polled before returning an already-ready prefix"
            );
            assert!(!seen.lock().unwrap().contains(&10));
            drop(window);
            assert_eq!(buffers.load(Ordering::SeqCst), 0);
        })
        .await
        .expect("out-of-order completions are observed without opening a ninth slot");
    }

    #[tokio::test]
    async fn dropping_the_window_drops_all_borrowed_pending_requests() {
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            let count = Arc::new(AtomicUsize::new(0));
            let filled = Arc::new(Notify::new());
            let mut window = Window::new(0, 8);
            let ask = |at| {
                let (count, filled) = (count.clone(), filled.clone());
                async move {
                    let _live = Live::new(&count);
                    if count.load(Ordering::SeqCst) == 8 {
                        filled.notify_one();
                    }
                    std::future::pending::<()>().await;
                    at
                }
            };
            {
                let next = window.next(Some(20 * crate::proto::SLICE), ask);
                tokio::pin!(next);
                tokio::select! {
                    _ = &mut next => panic!("all requests are pending"),
                    _ = filled.notified() => {}
                }
            }
            assert_eq!(count.load(Ordering::SeqCst), 8);
            drop(window);
            assert_eq!(count.load(Ordering::SeqCst), 0);
        })
        .await
        .expect("the caller owns all eight requests");
    }

    #[tokio::test]
    async fn a_completed_resume_offset_opens_no_new_request() {
        let end = 20 * crate::proto::SLICE;
        let asked = AtomicUsize::new(0);
        let mut window = Window::new(end, 8);
        assert_eq!(
            window
                .next(Some(end), |_| {
                    asked.fetch_add(1, Ordering::SeqCst);
                    std::future::ready(0u64)
                })
                .await,
            None
        );
        assert_eq!(asked.load(Ordering::SeqCst), 0);
    }
}
