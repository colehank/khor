//! Execution-home-owned A2A task foundations. See docs/A2A.md.
//!
//! This crate owns durable task semantics, not discovery, authentication,
//! process spawning, HTTP listeners or model access. The host authenticates
//! callers and authorizes delegation before entering this boundary.
pub use a2a as protocol;
pub use khor_core::{DeviceId, SessionId};

mod executor;
mod store;

pub use executor::{Execution, ExecutionFuture, Executor, dispatch};
pub use store::Store;

use a2a::{A2AError, Task};
use serde::{Deserialize, Serialize};

/// A device public key and a configured agent's key, never a launch command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentTarget {
    pub home: DeviceId,
    pub agent: String,
}

/// Task and session identities have different lifetimes. The executor binds
/// a context to a session after opening/resuming that session successfully.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionRef {
    pub home: DeviceId,
    pub id: SessionId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskRef {
    pub home: DeviceId,
    pub id: String,
}

/// Host-owned facts alongside an unmodified canonical A2A Task. These fields
/// are local records, not extra fields added to the standard Task object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Record {
    pub task: Task,
    pub target: AgentTarget,
    pub caller: DeviceId,
    pub parent: Option<TaskRef>,
    /// The context's current session link, not a claim that every earlier
    /// completed task ran inside that session.
    pub context_session: Option<SessionRef>,
    pub revision: u64,
    pub cancellation_requested: bool,
}

/// A repeated message returns the latest receipt with accepted=false. Reading
/// or retrying a receipt never grants a second execution.
#[derive(Debug, Clone, PartialEq)]
pub struct Submission {
    pub record: Record,
    pub accepted: bool,
}

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Corrupt(String),
    Invalid(&'static str),
    NotFound(String),
    Conflict,
    Busy,
    Terminal,
    NotCancelable(String),
    Adapter(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Storage paths and internal parse details stay out of a protocol reply.
    pub fn protocol_error(&self) -> A2AError {
        match self {
            Self::NotFound(id) => A2AError::task_not_found(id),
            Self::NotCancelable(id) => A2AError::task_not_cancelable(id),
            Self::Invalid(why) => A2AError::invalid_params(*why),
            Self::Conflict => A2AError::invalid_params("Message ID or session binding conflict"),
            Self::Busy => A2AError::unsupported_operation("The context is already executing"),
            Self::Terminal => A2AError::unsupported_operation("The task is terminal"),
            _ => A2AError::internal("Delegation storage or executor failure"),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => e.fmt(f),
            Self::Corrupt(e) => write!(f, "Invalid delegation store: {e}"),
            Self::Invalid(e) => e.fmt(f),
            Self::NotFound(id) => write!(f, "Task or context not found: {id}"),
            Self::Conflict => f.write_str("Message ID or session binding conflict"),
            Self::Busy => f.write_str("The context is already executing"),
            Self::Terminal => f.write_str("The task is terminal"),
            Self::NotCancelable(id) => write!(f, "Task cannot be canceled: {id}"),
            Self::Adapter(e) => e.fmt(f),
        }
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}
