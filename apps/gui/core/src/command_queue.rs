//! A connection's commands enter in caller order, then run away from the caller.
//! Submission never takes the socket lock. A reply means the work finished,
//! not merely that it entered the queue. Dropping a reply does not undo input.

use std::{
    collections::VecDeque,
    future::Future,
    pin::Pin,
    sync::{Arc, Condvar, Mutex},
    task::{Context, Poll, Waker},
};

const CLOSED: &str = "attachment is closed";
const CAPACITY: usize = 256;
type Job = Box<dyn FnOnce(bool) + Send>;

#[derive(Default)]
struct QueueState {
    jobs: VecDeque<Job>,
    running: bool,
    closed: bool,
}

#[derive(Default)]
pub(crate) struct CommandQueue(Arc<Mutex<QueueState>>);

impl CommandQueue {
    pub(crate) fn submit<T: Send + 'static>(
        &self,
        work: impl FnOnce() -> Result<T, String> + Send + 'static,
    ) -> Result<Pending<T>, String> {
        let completion = Arc::new(Completion {
            state: Mutex::new((None, None)),
            ready: Condvar::new(),
        });
        let reply = Pending(completion.clone());
        let mut state = lock(&self.0);
        if state.closed {
            return Err(CLOSED.into());
        }
        if state.jobs.len() >= CAPACITY {
            return Err("attachment command queue is full".into());
        }
        state.jobs.push_back(Box::new(move |run| {
            let result = if run {
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(work))
                    .unwrap_or_else(|_| Err("attachment command panicked".into()))
            } else {
                Err(CLOSED.into())
            };
            let wake = {
                let mut state = lock(&completion.state);
                state.0 = Some(result);
                state.1.take()
            };
            completion.ready.notify_all();
            if let Some(wake) = wake {
                wake.wake();
            }
        }));
        if !state.running {
            state.running = true;
            let queue = self.0.clone();
            if let Err(why) = std::thread::Builder::new()
                .name("attachment-commands".into())
                .spawn(move || {
                    loop {
                        let job = {
                            let mut state = lock(&queue);
                            match state.jobs.pop_front() {
                                Some(job) => job,
                                None => {
                                    state.running = false;
                                    return;
                                }
                            }
                        };
                        job(true);
                    }
                })
            {
                state.running = false;
                // No worker could have taken this first job. The caller gets
                // the spawn error; do not leave its captured connection here.
                state.jobs.clear();
                return Err(why.to_string());
            }
        }
        Ok(reply)
    }

    /// The owner also shuts down its socket, interrupting an in-flight write.
    pub(crate) fn close(&self) {
        let jobs = {
            let mut state = lock(&self.0);
            state.closed = true;
            std::mem::take(&mut state.jobs)
        };
        for job in jobs {
            job(false);
        }
    }
}

impl Drop for CommandQueue {
    fn drop(&mut self) {
        self.close();
    }
}

struct Completion<T> {
    state: Mutex<(Option<Result<T, String>>, Option<Waker>)>,
    ready: Condvar,
}

/// Completion of a command submitted synchronously. Native IPC awaits this;
/// synchronous core callers keep their original completion semantics via wait.
pub struct Pending<T>(Arc<Completion<T>>);

impl<T> Pending<T> {
    pub fn wait(self) -> Result<T, String> {
        let mut state = lock(&self.0.state);
        loop {
            if let Some(result) = state.0.take() {
                return result;
            }
            state = self
                .0
                .ready
                .wait(state)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
    }
}

impl<T> Future for Pending<T> {
    type Output = Result<T, String>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = lock(&self.0.state);
        if let Some(result) = state.0.take() {
            Poll::Ready(result)
        } else {
            state.1 = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
