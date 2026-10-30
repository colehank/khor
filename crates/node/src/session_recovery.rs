//! Recovery action orchestration. The existing row/Journal owners remain the
//! only authorities; no independent registry, service or token history lives here.
use super::{LiveKind, RowGate, gui_slot_ended, proc_identity, recovery_attempt};
use crate::{SessionId, session_content::journal::Identity};
use khor_catalog::msg;
use khor_core::recovery::{RecoveryResult, RecoveryToken};
use std::fs;

/// Internal work or an already-determined reply. A replayed token is never
/// converted into a new spawn claim by the legacy convenience wrapper.
enum NativePreparation {
    Spawn(String),
    Reply(RecoveryResult),
}

pub(super) fn action_token(value: &RecoveryToken) -> Result<recovery_attempt::Token, String> {
    if value.epoch.is_empty() || value.epoch.len() > 20 || !value.epoch.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("recovery epoch must be an unsigned decimal string".into());
    }
    let epoch: u64 = value.epoch.parse().map_err(|_| "recovery epoch exceeds its range".to_owned())?;
    if epoch.to_string() != value.epoch {
        return Err("recovery epoch must use its canonical decimal spelling".into());
    }
    recovery_attempt::Token::new(epoch, value.nonce.clone())
}

fn native_preparation(
    value: recovery_attempt::Reservation,
    id: &SessionId,
    epoch: &str,
) -> Result<NativePreparation, String> {
    use recovery_attempt::{Recorded, Reservation};
    let reply = match value {
        Reservation::Start(key) => {
            let encoded = serde_json::to_string(&key).map_err(|error| error.to_string())?;
            if encoded.len() > 16 * 1024 { return Err("GUI claim exceeds its byte budget".into()); }
            return Ok(NativePreparation::Spawn(encoded));
        }
        Reservation::Joined { current_run } => RecoveryResult::Join { id: id.clone(), current_run },
        Reservation::Recorded(Recorded::Opened { id, run_id }) => RecoveryResult::Opened { id, run_id },
        Reservation::Recorded(Recorded::Refused { reason }) => RecoveryResult::Refused { reason: match reason {
            recovery_attempt::Reason::Unavailable => "unavailable",
            recovery_attempt::Reason::InitializationRefused => "initialization-refused",
            recovery_attempt::Reason::SetupFailed => "setup-failed",
            recovery_attempt::Reason::Cancelled => "cancelled",
        }.into() },
        Reservation::Unfinished { host: Some(host), .. }
            if matches!(proc_identity::foreground(&host), proc_identity::Foreground::Present) => {
                RecoveryResult::Pending { id: id.clone(), attempt_epoch: epoch.into() }
            }
        Reservation::Retired => RecoveryResult::Unknown {
            id: id.clone(), attempt_epoch: epoch.into(), reason: "retired-attempt".into(),
        },
        Reservation::Unfinished { .. } | Reservation::UnknownOwner => RecoveryResult::Unknown {
            id: id.clone(), attempt_epoch: epoch.into(), reason: "execution-owner-unknown".into(),
        },
    };
    Ok(NativePreparation::Reply(reply))
}

impl LiveKind {
    /// Existing CLI convenience: choose one new identity for this explicit
    /// operation, then use exactly the stable admission algorithm below.
    pub(crate) fn prepare_gui_recovery(&self, native: Identity, launch: &[u8]) -> Result<String, String> {
        let id = crate::adaptor::id_for(&native.native_id);
        let directory = self.dir(&id).ok_or_else(|| msg::not_a_session_id(&id.0))?;
        fs::create_dir_all(&directory).map_err(msg::cant_make_session_dir)?;
        let (expected, epoch) = {
            let gate = RowGate::acquire(&directory)?;
            let expected = gate.read_meta_or_new()?.and_then(|meta| meta.gui_run.map(|run| run.id));
            (expected, recovery_attempt::next_epoch(&directory)?)
        };
        let token = RecoveryToken { epoch: epoch.to_string(), nonce: crate::link::fresh_hex()? };
        match self.prepare_gui_recovery_stable(native, launch, expected.as_deref(), &token)? {
            NativePreparation::Spawn(claim) => Ok(claim),
            NativePreparation::Reply(RecoveryResult::Refused { reason } | RecoveryResult::Unavailable { reason }) => Err(reason),
            NativePreparation::Reply(_) => Err("GUI recovery has no new exclusive execution authority".into()),
        }
    }

    fn prepare_gui_recovery_stable(
        &self,
        native: Identity,
        launch: &[u8],
        expected_run: Option<&str>,
        supplied: &RecoveryToken,
    ) -> Result<NativePreparation, String> {
        if native.home != self.device { return Err("recovery source belongs to another execution home".into()); }
        let token = action_token(supplied)?;
        let arguments = recovery_attempt::Arguments::native(native.clone(), expected_run.map(str::to_owned), launch)?;
        let id = crate::adaptor::id_for(&native.native_id);
        let directory = self.dir(&id).ok_or_else(|| msg::not_a_session_id(&id.0))?;
        fs::create_dir_all(&directory).map_err(msg::cant_make_session_dir)?;
        let legacy = self.legacy_gui_source(&id)?;
        let previous = {
            let mut gate = RowGate::acquire(&directory)?;
            let meta = gate.read_meta_or_new()?;
            gate.check_native(&id, meta.as_ref(), &native, legacy.as_ref())?;
            let current = meta.as_ref().and_then(|meta| meta.gui_run.as_ref()).map(|run| run.id.as_str());
            let previous = recovery_attempt::Held::load(&mut gate)?.replay(&arguments, &token)?;
            if previous.is_none() && current != expected_run {
                return Err("recovery offer names another GUI execution".into());
            }
            previous
        };
        if let Some(previous) = previous {
            return native_preparation(previous, &id, &supplied.epoch);
        }
        self.check_gui_source(&native)?;
        // Preflight the complete manifest and existing binding before even
        // spawning a ghost. The child still acquires and retains its own exact
        // File before provider spawn; the epoch fences this cross-process gap.
        let admission = crate::session_content::journal::JournalReservation::take(&self.root, native.clone())?;
        admission.check_binding(&directory)?;
        // Capture only the registered target under the short gate; all OS
        // facts are read after dropping it, never across a spawn or await.
        let registered = {
            let mut gate = RowGate::acquire(&directory)?;
            recovery_attempt::Held::load(&mut gate)?.registered_foreground(&native)?
        };
        if let Some(registered) = registered {
            let host_absent = if registered.slot_ended { None } else {
                match proc_identity::foreground(&registered.host) {
                    proc_identity::Foreground::Absent(proof) => Some(proof),
                    proc_identity::Foreground::Present => return Err("the previous Khor execution is still active".into()),
                    proc_identity::Foreground::Unknown => return Err("the previous Khor execution owner is unknown".into()),
                }
            };
            match proc_identity::foreground(&registered.process) {
                proc_identity::Foreground::Absent(proof) => {
                    let mut gate = RowGate::acquire(&directory)?;
                    recovery_attempt::Held::load(&mut gate)?.foreground_absent(&registered, &proof, host_absent.as_ref())?;
                }
                proc_identity::Foreground::Present => return Err("registered provider foreground is still present".into()),
                proc_identity::Foreground::Unknown => return Err("registered provider foreground is unknown".into()),
            }
        }
        let value = {
            let mut gate = RowGate::acquire(&directory)?;
            let meta = gate.read_meta_or_new()?;
            gate.check_native(&id, meta.as_ref(), &native, legacy.as_ref())?;
            if meta.as_ref().is_some_and(|meta| !gui_slot_ended(meta) && meta.pid.is_some_and(crate::link::pid_running)) {
                return Err("GUI row still has a live execution".into());
            }
            // The supplied expected run and token stay unchanged across all
            // preflight I/O. reserve rechecks them in the current row snapshot.
            recovery_attempt::Held::load(&mut gate)?.reserve(arguments, token, legacy.as_ref())?
        };
        native_preparation(value, &id, &supplied.epoch)
    }
}
