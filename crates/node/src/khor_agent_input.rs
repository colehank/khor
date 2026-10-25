use std::{
    collections::VecDeque,
    io::Write,
    sync::{Arc, Condvar, Mutex, mpsc},
    thread::JoinHandle,
};
struct Message {
    bytes: Vec<u8>,
    answer: Option<mpsc::SyncSender<Result<(), String>>>,
}
#[derive(Default)]
struct State {
    queue: VecDeque<Message>,
    stopped: bool,
    begun: bool,
    closed: bool,
}
#[derive(Clone)]
pub(super) struct Input(Arc<(Mutex<State>, Condvar)>);
impl Input {
    pub(super) fn spawn(
        mut pipe: Box<dyn Write + Send>,
    ) -> std::io::Result<(Self, JoinHandle<()>)> {
        let input = Self(Arc::new((Mutex::new(State::default()), Condvar::new())));
        let owned = input.clone();
        let thread = std::thread::Builder::new()
            .name("agent-input".into())
            .spawn(move || {
                loop {
                    let message = {
                        let (lock, wake) = &*owned.0;
                        let mut state = lock.lock().unwrap_or_else(|p| p.into_inner());
                        while state.queue.is_empty() && !state.closed {
                            state = wake.wait(state).unwrap_or_else(|p| p.into_inner());
                        }
                        if state.closed {
                            break;
                        }
                        let message = state.queue.pop_front().unwrap();
                        state.begun = true;
                        wake.notify_all();
                        message
                    };
                    let result = pipe
                        .write_all(&message.bytes)
                        .and_then(|_| pipe.flush())
                        .map_err(|e| e.to_string());
                    let failed = result.is_err();
                    if let Some(answer) = message.answer {
                        let _ = answer.send(result);
                    }
                    if failed {
                        break;
                    }
                }
                owned.close();
            })?;
        Ok((input, thread))
    }
    pub(super) fn send(&self, bytes: Vec<u8>) -> Result<(), String> {
        let (answer, result) = mpsc::sync_channel(1);
        let (lock, wake) = &*self.0;
        let mut state = lock.lock().unwrap_or_else(|p| p.into_inner());
        while !state.queue.is_empty() && !state.closed && !state.stopped {
            state = wake.wait(state).unwrap_or_else(|p| p.into_inner());
        }
        if state.closed || state.stopped {
            return Err("agent input closed".into());
        }
        state.queue.push_back(Message {
            bytes,
            answer: Some(answer),
        });
        wake.notify_all();
        drop(state);
        result.recv().map_err(|_| "agent input closed".to_owned())?
    }
    pub(super) fn abort(&self) {
        let (lock, wake) = &*self.0;
        let mut state = lock.lock().unwrap_or_else(|p| p.into_inner());
        if state.closed || state.stopped {
            return;
        }
        state.stopped = true;
        state.queue.clear();
        if state.begun {
            state.queue.push_back(Message {
                bytes: b"{\"type\":\"abort\"}\n".to_vec(),
                answer: None,
            });
        } else {
            state.closed = true;
        }
        wake.notify_all();
    }
    pub(super) fn close(&self) {
        let (lock, wake) = &*self.0;
        let mut state = lock.lock().unwrap_or_else(|p| p.into_inner());
        state.closed = true;
        state.queue.clear();
        wake.notify_all();
    }
}
