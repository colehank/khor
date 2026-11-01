//! Read-only recovery offers and explicit action results shared by Node and GUI.
//! Pure transport data: routing, token admission and provider capability checks
//! belong to the existing execution owner, not these serde types.

use crate::{DeviceId, SessionId};
use serde::{Deserialize, Serialize};

/// Caller-stable action identity. Admission validates the unsigned decimal
/// epoch and exactly 32 lowercase hexadecimal nonce characters before action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct RecoveryToken {
    pub epoch: String,
    pub nonce: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryExecution {
    Live,
    Ended,
    Offline,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryAction {
    Join,
    Native,
    NewShell,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum RecoverySupport {
    Resume,
    Load,
    Unknown,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct NativeRecovery {
    pub agent: String,
    /// The complete qualified provider identity; never derived from a row label.
    pub native_id: String,
    pub support: RecoverySupport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionRecovery {
    pub id: SessionId,
    /// Authoritative execution device, not SessionRow.home's display name.
    pub home: DeviceId,
    pub current_run: Option<String>,
    /// Unsigned decimal action epoch; unavailable when the owner cannot offer it.
    pub next_attempt_epoch: Option<String>,
    pub execution: RecoveryExecution,
    pub action: RecoveryAction,
    pub native: Option<NativeRecovery>,
    pub previous: Option<SessionId>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RecoveryResult {
    /// Published only after the recovered run reaches Ready.
    Opened {
        id: SessionId,
        run_id: String,
    },
    Join {
        id: SessionId,
        current_run: Option<String>,
    },
    Pending {
        id: SessionId,
        attempt_epoch: String,
    },
    Unknown {
        id: SessionId,
        attempt_epoch: String,
        reason: String,
    },
    Unavailable {
        reason: String,
    },
    Refused {
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ShellRunResult {
    /// A fresh shell SID linked to the previous SID, without command replay.
    Opened {
        id: SessionId,
        previous: SessionId,
    },
    /// The recorded cwd is unavailable; selecting another directory is a new
    /// explicit intent, never a mutation of an already accepted action token.
    DirectoryRequired {
        id: SessionId,
    },
    Pending {
        id: SessionId,
        attempt_epoch: String,
    },
    Unknown {
        id: SessionId,
        attempt_epoch: String,
        reason: String,
    },
    Unavailable {
        reason: String,
    },
    Refused {
        reason: String,
    },
}
