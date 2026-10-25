use std::sync::{Arc, Condvar, Mutex};

use khor_node::gui_host::WriteStep;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Stage {
    Queued,
    Uploading,
    Committed,
    Finished,
}

struct State {
    stage: Stage,
    stop: Option<Arc<StopSignal>>,
    notified: bool,
}

pub(super) struct Sending {
    pub token: Option<String>,
    state: Mutex<State>,
}

impl Sending {
    pub fn new(token: Option<String>) -> Self {
        Self {
            token,
            state: Mutex::new(State {
                stage: Stage::Queued,
                stop: None,
                notified: false,
            }),
        }
    }

    pub fn before(&self, step: WriteStep) -> Result<(), String> {
        let mut state = self.state.lock().unwrap();
        if state.stop.is_some() {
            return Err(khor_node::drops::cancelled());
        }
        state.stage = match step {
            WriteStep::Part => Stage::Uploading,
            WriteStep::Whole | WriteStep::Commit => Stage::Committed,
        };
        Ok(())
    }

    pub fn cancel(&self, signal: &Arc<StopSignal>) -> Stage {
        let mut state = self.state.lock().unwrap();
        if matches!(state.stage, Stage::Queued | Stage::Uploading) && state.stop.is_none() {
            state.stop = Some(signal.clone());
        }
        state.stage
    }

    pub fn cancellation(&self) -> Option<(Stage, Arc<StopSignal>)> {
        let state = self.state.lock().unwrap();
        state.stop.as_ref().map(|stop| (state.stage, stop.clone()))
    }

    pub fn claim_notice(&self) -> bool {
        let mut state = self.state.lock().unwrap();
        state.stop.is_some() && !std::mem::replace(&mut state.notified, true)
    }

    pub fn claim_receipt(&self) -> bool {
        !std::mem::replace(&mut self.state.lock().unwrap().notified, true)
    }

    pub fn finish(&self) {
        self.state.lock().unwrap().stage = Stage::Finished;
    }
}

#[derive(Clone, Copy)]
pub(super) enum StopAction {
    Local,
    Written,
    Disconnected,
}

#[derive(Default)]
pub(super) struct StopSignal {
    result: Mutex<Option<Result<StopAction, String>>>,
    ready: Condvar,
}

impl StopSignal {
    fn complete(&self, result: Result<StopAction, String>) {
        let mut current = self.result.lock().unwrap();
        if current.is_none() {
            *current = Some(result);
            self.ready.notify_all();
        }
    }

    pub fn wait(&self) -> Result<StopAction, String> {
        let mut result = self.result.lock().unwrap();
        while result.is_none() {
            result = self.ready.wait(result).unwrap();
        }
        result.as_ref().unwrap().clone()
    }
}

/// The queued writer owns completion. Closing a queue drops this guard too,
/// so a cancelled turn cannot wait forever for a Stop that never executes.
pub(super) struct StopWriter(pub Arc<StopSignal>);

impl StopWriter {
    pub fn complete(&self, result: Result<StopAction, String>) {
        self.0.complete(result);
    }
}

impl Drop for StopWriter {
    fn drop(&mut self) {
        self.0.complete(Err("attachment is closed".into()));
    }
}
