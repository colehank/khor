use crate::{Record, Result, SessionRef, Store};
use a2a::{Artifact, Message, Part, Role, TaskState};
use std::{fs::File, future::Future, pin::Pin};

/// One claimed input turn. Keeping this value alive holds the context's
/// cross-process lease. No second task can drive its session concurrently.
pub struct Execution {
    store: Store,
    record: Record,
    message: Message,
    turn: u64,
    _lease: File,
}

impl Execution {
    /// Called by the trusted execution host after delegation authorization.
    pub fn claim(store: Store, task: &str, message: &str) -> Result<Self> {
        let (lease, turn, record, message) = store.claim(task, message)?;
        Ok(Self {
            store,
            record,
            message,
            turn,
            _lease: lease,
        })
    }
    pub fn record(&self) -> &Record {
        &self.record
    }
    pub fn message(&self) -> &Message {
        &self.message
    }

    pub fn bind_session(&mut self, session: SessionRef) -> Result<()> {
        self.record = self
            .store
            .bind_session(&self.record.task.id, self.turn, session)?;
        Ok(())
    }
    pub fn cancellation_requested(&self) -> Result<bool> {
        Ok(self
            .store
            .get(
                self.record.caller,
                &self.record.target,
                &self.record.task.id,
            )?
            .cancellation_requested)
    }
    pub fn update(
        &mut self,
        state: TaskState,
        message: Option<Message>,
        artifact: Option<Artifact>,
    ) -> Result<()> {
        self.record = self.store.executor_update(
            &self.record.task.id,
            self.turn,
            state,
            message,
            artifact,
        )?;
        Ok(())
    }
}

pub type ExecutionFuture<'a> = Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;

/// Adapter boundary for ACP sessions or another A2A implementation. A normal
/// return must leave a terminal or input/auth-required state. Before publishing
/// input/auth-required, quiesce execution and return; the answer is a new
/// accepted message. These states do not grant approval or provide credentials.
/// Implementations stop their actual operation before confirming Canceled.
pub trait Executor: Send + Sync {
    fn execute<'a>(&'a self, execution: &'a mut Execution) -> ExecutionFuture<'a>;
}

/// Explicit dispatch, intentionally separate from storage and receipt reads.
/// The caller owns runtime scheduling; there is no background auto-start hook.
pub async fn dispatch(
    store: Store,
    task: &str,
    message: &str,
    executor: &dyn Executor,
) -> Result<Record> {
    let mut execution = Execution::claim(store, task, message)?;
    let result = executor.execute(&mut execution).await;
    // An IO error can follow a successful rename. Read the authoritative
    // receipt before replacing an apparently unfinished outcome.
    execution.record = execution.store.get(
        execution.record.caller,
        &execution.record.target,
        &execution.record.task.id,
    )?;
    if !execution.record.task.status.state.is_terminal() {
        let failure = match result {
            Err(error) => Some(error.to_string()),
            Ok(()) if execution.record.task.status.state == TaskState::Working => {
                Some("Executor returned without completing or suspending the task".into())
            }
            Ok(()) => None,
        };
        if let Some(failure) = failure {
            execution.update(
                TaskState::Failed,
                Some(Message::new(Role::Agent, vec![Part::text(failure)])),
                None,
            )?;
        }
    }
    Ok(execution.record.clone())
}
