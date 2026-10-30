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
    fn observed_execution_ended(&self, native: &Identity) -> Result<bool, String> {
        let mut positive = false;
        for id in super::native_aliases(native) {
            let directory = self.dir(&id).ok_or("invalid source alias")?;
            match recovery_attempt::observed_alias_ended(&directory, &id, native)? {
                Some(false) => return Ok(false), Some(true) => positive = true, None => {},
            }
        }
        Ok(positive)
    }

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
        let id = crate::adaptor::id_for(&native.native_id);
        self.prepare_gui_recovery_target(&id, native, launch, expected_run, supplied, None)
    }
    fn prepare_gui_recovery_target(
        &self, id: &SessionId, native: Identity, launch: &[u8], expected_run: Option<&str>, supplied: &RecoveryToken,
        observed: Option<&crate::session_content::Observation>,
    ) -> Result<NativePreparation, String> {
        if native.home != self.device { return Err("recovery source belongs to another execution home".into()); }
        let token = action_token(supplied)?;
        let arguments = recovery_attempt::Arguments::native_at(id.clone(), native.clone(), expected_run.map(str::to_owned), launch)?;
        let directory = self.dir(&id).ok_or_else(|| msg::not_a_session_id(&id.0))?;
        fs::create_dir_all(&directory).map_err(msg::cant_make_session_dir)?;
        let legacy = self.legacy_gui_source(&id)?;
        let previous = {
            let mut gate = RowGate::acquire(&directory)?;
            let meta = gate.read_meta_or_new()?;
            gate.check_native(&id, meta.as_ref(), &native, legacy.as_ref())?;
            let current = meta.as_ref().and_then(|meta| meta.gui_run.as_ref()).map(|run| run.id.as_str());
            let previous = match recovery_attempt::Held::load(&mut gate)?.check_native_input(&arguments, &token)? {
                recovery_attempt::InputDecision::Continue(previous) => previous,
                recovery_attempt::InputDecision::Refused(reason) => return Ok(NativePreparation::Reply(RecoveryResult::Refused { reason })),
            };
            if previous.is_none() && current != expected_run {
                return Ok(NativePreparation::Reply(RecoveryResult::Refused { reason: "recovery offer names another GUI execution".into() }));
            }
            previous
        };
        if let Some(previous) = previous {
            return native_preparation(previous, &id, &supplied.epoch);
        }
        // Preflight the complete manifest and existing binding before even
        // spawning a ghost. The child still acquires and retains its own exact
        // File before provider spawn; the epoch fences this cross-process gap.
        let admission = crate::session_content::journal::JournalReservation::take(&self.root, native.clone())?;
        admission.check_binding(&directory)?;
        if let Some(record) = observed {
            if !self.observed_execution_ended(&native)? { return Err("observed execution ownership is unknown".into()); }
            record.persist_recovery(&self.root, &admission)?;
        }
        self.check_gui_source_at(id, &native)?;
        self.fence_native_aliases(&admission, None)?;
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

struct NativeLaunch {
    native: Identity,
    cwd: std::path::PathBuf,
    title: String,
    cmd: Vec<String>,
    context: khor_acp::Environment,
    encoded: Vec<u8>,
}
impl crate::Node {
    pub(crate) fn check_recovery_token(value: &RecoveryToken) -> Result<(), String> { action_token(value).map(|_| ()) }
    fn recovery_row(&self, id: &SessionId) -> Result<(std::path::PathBuf, super::Meta), String> {
        let directory = self.live.dir(id).ok_or_else(|| msg::not_a_session_id(&id.0))?;
        let meta = self.recovery_meta(id, &directory)?.ok_or("recovery row has no execution metadata")?;
        Ok((directory, meta))
    }
    fn recovery_meta(&self, id: &SessionId, directory: &std::path::Path) -> Result<Option<super::Meta>, String> {
        let meta: super::Meta = match fs::read(directory.join("meta.json")) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|error| error.to_string())?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.to_string()),
        };
        if !meta.id.is_empty() && meta.id != id.0 { return Err("recovery row belongs to another session".into()); }
        if meta.shell_run.as_ref().is_some_and(|run| run.previous.0.is_empty() || run.root.0.is_empty() || run.previous.0.len() > 4096 || run.root.0.len() > 4096 || run.previous == *id || run.root == *id) {
            return Err("invalid stored Shell ancestry".into());
        }
        Ok(Some(meta))
    }
    fn recovery_native(&self, id: &SessionId, directory: &std::path::Path, meta: &super::Meta) -> Result<Option<Identity>, String> {
        let binding = crate::session_content::journal::bound_identity(directory)?;
        if binding.as_ref().zip(meta.gui_native.as_ref()).is_some_and(|(bound, native)| bound != native) {
            return Err("recovery metadata and canonical binding disagree".into());
        }
        let native = binding.or_else(|| meta.gui_native.clone()).or(self.live.legacy_gui_source(id)?.map(|(_, native)| native));
        if native.as_ref().is_some_and(|native| native.home != self.device() || !super::matches_logical_source(id, native)) {
            return Err("recovery identity belongs to another execution home or SID".into());
        }
        if let Some(native) = native.as_ref() { super::check_observed_target(self.root(), id, native)?; }
        Ok(native)
    }
    fn recovery_launch(&self, title: &str, place: Option<&str>, native: Identity, readonly: bool) -> Result<NativeLaunch, String> {
        // Offer never calls user_path(): its cold initialization runs a login
        // shell. A missing observation simply leaves prior support Unknown.
        let path = if readonly { Self::observed_user_path().ok_or("effective launch environment has not been observed")? } else { Self::user_path() };
        let spec = match native.agent.as_str() {
            crate::adaptor::claude::VENDOR => crate::AgentSpec { command: crate::self_exe()?.display().to_string(), args: vec!["_cagent".into()] },
            crate::adaptor::codex::VENDOR => crate::AgentSpec { command: crate::self_exe()?.display().to_string(), args: vec!["_codexagent".into()] },
            name if readonly => khor_sync::store::read_snapshot::<khor_sync::agents::AgentDoc>(&khor_sync::agents::agents_dir(self.root()), self.peer)?.get(name).ok_or("native agent is not registered")?,
            name => self.agent_or_refuse(name)?,
        };
        let cmd = vec![Self::launch_on(&spec, path)?];
        let cwd = std::path::PathBuf::from(place.ok_or("the session has no recorded working directory")?);
        if !cwd.is_dir() { return Err(msg::no_such_dir(cwd.display())); }
        let cwd = fs::canonicalize(cwd).map_err(|error| error.to_string())?;
        let title = title.to_owned();
        let context = crate::gui_host::saved_environment(self.root(), &native.native_id)?;
        let encoded = serde_json::to_vec(&(&cwd, &title, &cmd, &context)).map_err(|error| error.to_string())?;
        Ok(NativeLaunch { native, cwd, title, cmd, context, encoded })
    }

    /// A forwarded reference is data. Identity/title/cwd are re-resolved from
    /// the actual execution home's existing source; this does not start or
    /// probe a provider and does not create an observation/row/attempt.
    fn recovery_observation(&self, id: &SessionId, hint: Option<&khor_core::ObservationRef>) -> Result<crate::session_content::Observation, String> {
        use crate::session_content::{Observation, NativeTranscript, Provider};
        let stored = Observation::read(self.root(), id)?;
        let reference = hint.or(stored.as_ref().map(|record| &record.reference)).ok_or("observation reference is unavailable")?;
        Observation::validate_reference(id, self.device(), reference)?;
        if stored.as_ref().is_some_and(|record| record.reference.home != reference.home || record.reference.namespace != reference.namespace
            || record.reference.agent != reference.agent || record.reference.native_id != reference.native_id) {
            return Err("forwarded observation conflicts with the existing full source".into());
        }
        let (title, cwd) = match reference.namespace {
            khor_core::ObservationNamespace::Native => {
                let provider = Provider::of(Some(reference.agent.as_str())).ok_or("unknown native transcript provider")?;
                let source = NativeTranscript::resolve(self.root(), provider, &super::clean_leaf(&reference.native_id), Some(&reference.native_id))
                    .map_err(|_| "native transcript is unreadable or ambiguous".to_owned())?.ok_or("native transcript is unavailable")?;
                source.open().map_err(|error| error.to_string())?;
                let cwd = source.recorded_cwd(self.root()).ok_or("native transcript has no recorded working directory")?;
                (stored.as_ref().map(|record| record.title.clone()).unwrap_or_else(|| reference.native_id.clone()), cwd.to_string_lossy().into_owned())
            },
            khor_core::ObservationNamespace::Acp => {
                let agents = khor_sync::store::read_snapshot::<khor_sync::agents::AgentDoc>(&khor_sync::agents::agents_dir(self.root()), self.peer)?;
                let listed = match agents.seen(&reference.agent) {
                    Some(khor_sync::agents::Answer::Sessions { home: Some(home), rows }) if home == self.device().0 => rows.into_iter().find(|row| row.id == reference.native_id),
                    _ => None,
                };
                if let Some(row) = listed {
                    (row.title.unwrap_or_else(|| reference.native_id.clone()), row.cwd)
                } else {
                    // Some real owned agents cannot list. A previously ended
                    // exact canonical owner is another existing source, not a
                    // fabricated session/list answer or a new adapter.
                    let native = Identity { home: self.device(), agent: reference.agent.clone(), native_id: reference.native_id.clone() };
                    if !self.live.observed_execution_ended(&native)? { return Err("ACP source has no qualified execution-home record".into()); }
                    crate::session_content::journal::Source::find(self.root(), native.clone())?.ok_or("qualified canonical source is unavailable")?;
                    let mut previous = None;
                    for alias in super::native_aliases(&native) {
                        let directory = self.live.dir(&alias).ok_or("invalid source alias")?;
                        if recovery_attempt::observed_alias_ended(&directory, &alias, &native)? != Some(true) { continue; }
                        let meta = self.recovery_meta(&alias, &directory)?.ok_or("qualified execution metadata disappeared")?;
                        previous = Some((meta.title, meta.place.ok_or("qualified execution has no working directory")?));
                        break;
                    }
                    previous.ok_or("qualified execution-home source disappeared")?
                }
            },
        };
        if cwd.trim().is_empty() { return Err("observation source has no working directory".into()); }
        let mut record = Observation::new(reference.owner, reference.home, reference.namespace, reference.agent.clone(), reference.native_id.clone(), title, Some(cwd), reference.updated_at);
        record.reference = reference.clone();
        Ok(record)
    }

    pub(crate) fn session_recovery_here(&self, home: khor_core::DeviceId, id: &SessionId, hint: Option<&khor_core::ObservationRef>) -> Result<khor_core::recovery::SessionRecovery, String> {
        use khor_core::recovery::{SessionRecovery, RecoveryExecution as Execution, RecoveryAction as Action, RecoverySupport, NativeRecovery};
        if home != self.device() { return Err("recovery offer reached another execution home".into()); }
        if let Some(reference) = hint { crate::session_content::Observation::validate_reference(id, home, reference)?; }
        let directory = self.live.dir(id).ok_or_else(|| msg::not_a_session_id(&id.0))?;
        let Some(meta) = self.recovery_meta(id, &directory)? else {
            let record = self.recovery_observation(id, hint)?;
            let native = Identity { home, agent: record.reference.agent.clone(), native_id: record.reference.native_id.clone() };
            let ended = self.live.observed_execution_ended(&native)?;
            let support = self.recovery_launch(&record.title, record.cwd.as_deref(), native.clone(), true).ok()
                .and_then(|launch| recovery_attempt::matching_support(&directory, &native, &launch.encoded).ok()).unwrap_or(RecoverySupport::Unknown);
            return Ok(SessionRecovery { id: id.clone(), home, current_run: None,
                next_attempt_epoch: recovery_attempt::next_epoch(&directory).ok().map(|epoch| epoch.to_string()),
                execution: if ended { Execution::Ended } else { Execution::Unknown },
                action: if ended { Action::Native } else { Action::None },
                native: Some(NativeRecovery { agent: native.agent, native_id: native.native_id, support }), previous: None,
                reason: if ended { None } else { Some("no confirmed previous execution owner".into()) },
            });
        };
        let current_run = meta.gui_run.as_ref().map(|run| run.id.clone());
        let mut offer = SessionRecovery { id: id.clone(), home, current_run, next_attempt_epoch: recovery_attempt::next_epoch(&directory).ok().map(|epoch| epoch.to_string()), execution: Execution::Unknown, action: Action::None, native: None, previous: None, reason: None };
        if meta.kind == khor_core::kind::SHELL {
            offer.current_run = Some(id.0.clone());
            offer.previous = meta.shell_run.as_ref().map(|run| run.previous.clone());
            offer.execution = if self.live.ending(id).is_some() { Execution::Ended } else { Execution::Unknown };
            if offer.execution == Execution::Ended { offer.action = Action::NewShell; }
            return Ok(offer);
        }
        if meta.kind != khor_core::kind::GUI { offer.reason = Some("this execution kind cannot be recovered here".into()); return Ok(offer); }
        let Some(native) = self.recovery_native(id, &directory, &meta)? else { offer.reason = Some("native identity is unknown".into()); return Ok(offer); };
        // This is the row's existing live observation, not provider ownership or
        // permission to restart. The explicit Join still authenticates its GUI
        // endpoint and rechecks the same Meta/HostFile under the row gate.
        let observed_live = !gui_slot_ended(&meta) && meta.pid.is_some_and(crate::link::pid_running);
        offer.execution = if observed_live { Execution::Live } else if gui_slot_ended(&meta) { Execution::Ended } else { Execution::Unknown };
        let support = self.recovery_launch(&meta.title, meta.place.as_deref(), native.clone(), true).ok().and_then(|launch| recovery_attempt::matching_support(&directory, &native, &launch.encoded).ok()).unwrap_or(RecoverySupport::Unknown);
        let admissible = recovery_attempt::offer_action_allowed(&directory)?;
        offer.action = if observed_live { Action::Join } else if admissible { Action::Native } else { Action::None };
        if !admissible && !observed_live { offer.reason = Some("previous execution ownership is unknown".into()); }
        offer.native = Some(NativeRecovery { agent: native.agent, native_id: native.native_id, support });
        Ok(offer)
    }

    pub(crate) async fn recover_session_here(&self, home: khor_core::DeviceId, id: &SessionId, expected_run: Option<&str>, supplied: RecoveryToken, hint: Option<&khor_core::ObservationRef>) -> Result<RecoveryResult, String> {
        if home != self.device() { return Ok(RecoveryResult::Unavailable { reason: "execution-home-mismatch".into() }); }
        if let Err(reason) = Self::check_recovery_token(&supplied) { return Ok(RecoveryResult::Refused { reason }); }
        if let Some(reference) = hint { if let Err(reason) = crate::session_content::Observation::validate_reference(id, home, reference) { return Ok(RecoveryResult::Refused { reason }); } }
        self.refuse_unless_hosting()?;
        let directory = self.live.dir(id).ok_or_else(|| msg::not_a_session_id(&id.0))?;
        let expected = expected_run.map(str::to_owned);
        let (launch, observed) = if let Some(meta) = self.recovery_meta(id, &directory)? {
            if meta.kind != khor_core::kind::GUI { return Ok(RecoveryResult::Refused { reason: "not-a-native-GUI-execution".into() }); }
            let Some(native) = self.recovery_native(id, &directory, &meta)? else { return Ok(RecoveryResult::Unavailable { reason: "native-identity-unknown".into() }); };
            if let Err(reason) = recovery_attempt::join_arguments_at(id.clone(), native.clone(), expected.clone()) { return Ok(RecoveryResult::Refused { reason }); }
            let join_live = self.live.clone(); let join_id = id.clone(); let join_native = native.clone();
            let join_expected = expected.clone(); let join_token = supplied.clone();
            if let Some(reply) = tokio::task::spawn_blocking(move || join_live.join_recovery_if_live(&join_id, join_native, join_expected, &join_token)).await.map_err(|error| error.to_string())?? { return Ok(reply); }
            let (_, meta) = self.recovery_row(id)?;
            (self.recovery_launch(&meta.title, meta.place.as_deref(), native, false)?, None)
        } else {
            if expected.is_some() { return Ok(RecoveryResult::Refused { reason: "expected-execution-changed".into() }); }
            let record = self.recovery_observation(id, hint)?;
            let native = Identity { home, agent: record.reference.agent.clone(), native_id: record.reference.native_id.clone() };
            if !self.live.observed_execution_ended(&native)? { return Ok(RecoveryResult::Unknown { id: id.clone(), attempt_epoch: supplied.epoch, reason: "observed-execution-owner-unknown".into() }); }
            (self.recovery_launch(&record.title, record.cwd.as_deref(), native, false)?, Some(record))
        };
        let live = self.live.clone(); let root = self.root().to_path_buf(); let id = id.clone();
        // All blocking endpoint/FS/ghost work is outside the async bridge. The
        // short row gates below never contain a connect, process spawn or wait.
        tokio::task::spawn_blocking(move || {
            let arguments = recovery_attempt::Arguments::native_at(id.clone(), launch.native.clone(), expected.clone(), &launch.encoded)?;
            let token = action_token(&supplied)?;
            let legacy = live.legacy_gui_source(&id)?;
            fs::create_dir_all(&directory).map_err(msg::cant_make_session_dir)?;
            {
                let mut gate = RowGate::acquire(&directory)?;
                let meta = gate.read_meta_or_new()?;
                gate.check_native(&id, meta.as_ref(), &launch.native, legacy.as_ref())?;
                let previous = match recovery_attempt::Held::load(&mut gate)?.check_native_input(&arguments, &token)? {
                    recovery_attempt::InputDecision::Continue(previous) => previous,
                    recovery_attempt::InputDecision::Refused(reason) => return Ok(RecoveryResult::Refused { reason }),
                };
                if let Some(previous) = previous {
                    return match native_preparation(previous, &id, &supplied.epoch)? { NativePreparation::Reply(reply) => Ok(reply), NativePreparation::Spawn(_) => Err("replayed action unexpectedly acquired new authority".into()) };
                }
                if meta.as_ref().and_then(|meta| meta.gui_run.as_ref()).map(|run| &run.id) != expected.as_ref() { return Ok(RecoveryResult::Refused { reason: "expected-execution-changed".into() }); }
            }
            if let Some((_, pids)) = live.sighting_of(&crate::adaptor::id_for(&launch.native.native_id)) && !pids.is_empty() { return Ok(RecoveryResult::Refused { reason: "native-session-running-elsewhere".into() }); }
            let codex = crate::adaptor::codex::Codex::at(crate::adaptor::codex_home(&root));
            if codex.full_session_id(&super::clean_leaf(&launch.native.native_id)).as_deref() == Some(launch.native.native_id.as_str()) && codex.has_writer(&launch.native.native_id)? { return Ok(RecoveryResult::Refused { reason: "native-session-has-a-writer".into() }); }
            match live.prepare_gui_recovery_target(&id, launch.native.clone(), &launch.encoded, expected.as_deref(), &supplied, observed.as_ref())? {
                NativePreparation::Reply(reply) => Ok(reply),
                NativePreparation::Spawn(claim) => {
                    let opened = crate::gui_host::spawn_gui_host_claimed(&root, &launch.cwd, &launch.title, &launch.cmd, &launch.native, &launch.context, &claim);
                    // A lost response, timeout or partial publication re-reads
                    // the same token; it never falls back to the CLI fresh ask.
                    match live.prepare_gui_recovery_target(&id, launch.native, &launch.encoded, expected.as_deref(), &supplied, observed.as_ref())? {
                        NativePreparation::Reply(reply) => Ok(reply),
                        NativePreparation::Spawn(_) => Err(format!("accepted recovery action lost its durable result: {opened:?}")),
                    }
                }
            }
        }).await.map_err(|error| error.to_string())?
    }
}

impl LiveKind {
    fn join_recovery_if_live(&self, id: &SessionId, native: Identity, expected: Option<String>, supplied: &RecoveryToken) -> Result<Option<RecoveryResult>, String> {
        let directory = self.dir(id).ok_or_else(|| msg::not_a_session_id(&id.0))?;
        let arguments = recovery_attempt::join_arguments_at(id.clone(), native.clone(), expected.clone())?;
        let token = action_token(supplied)?;
        let legacy = self.legacy_gui_source(id)?;
        let (ticket, host_pid) = {
            let mut gate = RowGate::acquire(&directory)?;
            let meta = gate.read_meta_or_new()?.ok_or("GUI row disappeared")?;
            gate.check_native(id, Some(&meta), &native, legacy.as_ref())?;
            let mut held = recovery_attempt::Held::load(&mut gate)?;
            let previous = match held.check_join_input(&arguments, &token)? {
                recovery_attempt::InputDecision::Continue(previous) => previous,
                recovery_attempt::InputDecision::Refused(reason) => return Ok(Some(RecoveryResult::Refused { reason })),
            };
            if let Some(previous) = previous {
                return match native_preparation(previous, id, &supplied.epoch)? { NativePreparation::Reply(reply) => Ok(Some(reply)), NativePreparation::Spawn(_) => Err("Join replay issued a spawn".into()) };
            }
            if held.is_execution_token(&token) { return Ok(None); }
            if meta.gui_run.as_ref().map(|run| &run.id) != expected.as_ref() { return Ok(Some(RecoveryResult::Refused { reason: "expected-execution-changed".into() })); }
            if gui_slot_ended(&meta) || !meta.pid.is_some_and(crate::link::pid_running) { return Ok(None); }
            let host_pid = meta.pid.ok_or("live GUI has no host identity")?;
            match held.begin_join(arguments, token, legacy.as_ref())? {
                recovery_attempt::JoinAdmission::Previous(previous) => return match native_preparation(previous, id, &supplied.epoch)? { NativePreparation::Reply(reply) => Ok(Some(reply)), NativePreparation::Spawn(_) => Err("Join replay issued a spawn".into()) },
                recovery_attempt::JoinAdmission::Check(ticket) => (ticket, host_pid),
            }
        };
        let checked = (|| {
            let host = crate::host::read_host_file(&directory)?;
            if host_pid != host.host_pid || host.host_pid != host.child_pid { return Err("live GUI endpoint has another execution owner".into()); }
            let attached = crate::gui_host::connect_at(&format!("127.0.0.1:{}", host.port), &host.cookie)?;
            let mut gate = RowGate::acquire(&directory)?;
            let joined = recovery_attempt::Held::load(&mut gate)?.finish_join(ticket, legacy.as_ref(), |gate| {
                let current = crate::host::read_host_file(&directory)?;
                let meta = gate.read_meta_or_new()?.ok_or("GUI row disappeared")?;
                if current.host_pid != host_pid || current.child_pid != host.child_pid || current.port != host.port || current.cookie != host.cookie || meta.pid != Some(host_pid) { return Err("GUI endpoint changed during authenticated Join".into()); }
                Ok(())
            })?;
            drop(attached);
            match native_preparation(joined, id, &supplied.epoch)? { NativePreparation::Reply(reply) => Ok(reply), NativePreparation::Spawn(_) => Err("Join issued a spawn".into()) }
        })();
        // The durable pending intent is deliberately retained after failure.
        // A duplicate returns Unknown, never a fresh native preparation.
        Ok(Some(match checked { Ok(reply) => reply, Err(reason) => RecoveryResult::Unknown { id: id.clone(), attempt_epoch: supplied.epoch.clone(), reason } }))
    }

    pub(crate) fn record_shell_ancestry(&self, id: &SessionId, ancestry: super::ShellRun) -> Result<(), String> {
        if ancestry.previous == *id || ancestry.root == *id || ancestry.previous.0.is_empty() || ancestry.root.0.is_empty() || ancestry.previous.0.len() > 4096 || ancestry.root.0.len() > 4096 { return Err("invalid fresh Shell ancestry".into()); }
        let directory = self.dir(id).ok_or_else(|| msg::not_a_session_id(&id.0))?;
        let gate = RowGate::acquire(&directory)?;
        let mut meta = gate.read_meta_or_new()?.ok_or("new Shell row is missing")?;
        if meta.kind != khor_core::kind::SHELL || meta.id != id.0 { return Err("fresh Shell row changed kind or identity".into()); }
        if let Some(current) = &meta.shell_run {
            return if current == &ancestry { Ok(()) } else { Err("Shell ancestry has already been published".into()) };
        }
        if meta.pid.is_some() { return Err("Shell ancestry must precede its host spawn".into()); }
        meta.shell_run = Some(ancestry);
        gate.write_meta(&meta)
    }
}
fn shell_replay(value: recovery_attempt::ActionReplay, id: &SessionId, epoch: &str) -> khor_core::recovery::ShellRunResult {
    use khor_core::recovery::ShellRunResult;
    use recovery_attempt::{ActionReplay, ActionResult};
    match value {
        ActionReplay::Recorded(ActionResult::ShellOpened { id: next }) => ShellRunResult::Opened { id: next, previous: id.clone() },
        ActionReplay::Recorded(ActionResult::DirectoryRequired) => ShellRunResult::DirectoryRequired { id: id.clone() },
        ActionReplay::Recorded(ActionResult::Refused { reason }) => ShellRunResult::Refused { reason },
        ActionReplay::Retired => ShellRunResult::Unknown { id: id.clone(), attempt_epoch: epoch.into(), reason: "retired-attempt".into() },
        _ => ShellRunResult::Unknown { id: id.clone(), attempt_epoch: epoch.into(), reason: "accepted-Shell-outcome-unknown".into() },
    }
}
impl crate::Node {
    pub(crate) async fn new_shell_run_here(&self, home: khor_core::DeviceId, id: &SessionId, expected_run: Option<&str>, supplied: RecoveryToken, cwd: Option<&str>) -> Result<khor_core::recovery::ShellRunResult, String> {
        use khor_core::recovery::ShellRunResult;
        if home != self.device() { return Ok(ShellRunResult::Unavailable { reason: "execution-home-mismatch".into() }); }
        self.refuse_unless_hosting()?;
        let token = match action_token(&supplied) { Ok(token) => token, Err(reason) => return Ok(ShellRunResult::Refused { reason }) };
        if expected_run != Some(id.0.as_str()) { return Ok(ShellRunResult::Refused { reason: "expected-previous-SID-is-required".into() }); }
        if cwd.is_some_and(|raw| raw.trim().is_empty()) { return Ok(ShellRunResult::Refused { reason: "selected-directory-is-empty".into() }); }
        let (directory, meta) = self.recovery_row(id)?;
        if meta.kind != khor_core::kind::SHELL { return Ok(ShellRunResult::Refused { reason: "not-a-Shell-execution".into() }); }
        let requested = cwd.map(str::to_owned);
        let standing = requested.as_deref().or(meta.place.as_deref());
        // Never turn an absent/blank old directory into directory("")'s home.
        let effective = standing.filter(|path| !path.trim().is_empty()).and_then(|path| crate::shell_start::directory(path).ok()).and_then(|path| fs::canonicalize(path).ok());
        let ancestry = super::ShellRun { previous: id.clone(), root: meta.shell_run.as_ref().map_or_else(|| id.clone(), |run| run.root.clone()) };
        let shell = crate::shell_start::default_shell();
        if shell.trim().is_empty() { return Ok(ShellRunResult::Refused { reason: "default-shell-unavailable".into() }); }
        let encoded = serde_json::to_vec(&(&requested, &meta.place, &effective, &shell, &meta.title, &ancestry.root)).map_err(|error| error.to_string())?;
        let arguments = recovery_attempt::ActionArguments::shell(home, id.clone(), id.0.clone(), &encoded)?;
        {
            let mut gate = RowGate::acquire(&directory)?;
            match recovery_attempt::Held::load(&mut gate)?.check_shell_input(&arguments, &token)? {
                recovery_attempt::InputDecision::Refused(reason) => return Ok(ShellRunResult::Refused { reason }),
                recovery_attempt::InputDecision::Continue(Some(previous)) => return Ok(shell_replay(previous, id, &supplied.epoch)),
                recovery_attempt::InputDecision::Continue(None) => {},
            }
            let previous = recovery_attempt::Held::load(&mut gate)?.reserve_shell(arguments.clone(), token.clone(), |gate| {
                let (current, state) = super::read_pair(&directory)?;
                if current.kind != khor_core::kind::SHELL || current.id != id.0 || current.pid != meta.pid || current.started_ms != meta.started_ms || current.place != meta.place || current.title != meta.title || current.shell_run != meta.shell_run || super::ending_of(&gate.dir, &current, &state).is_none() {
                    return Err("previous Shell is not the same ended execution".into());
                }
                Ok(())
            })?;
            if let Some(previous) = previous { return Ok(shell_replay(previous, id, &supplied.epoch)); }
            if effective.is_none() {
                recovery_attempt::Held::load(&mut gate)?.finish_shell(&arguments, &token, recovery_attempt::ActionResult::DirectoryRequired)?;
                return Ok(ShellRunResult::DirectoryRequired { id: id.clone() });
            }
        }
        let cwd = effective.expect("checked above");
        let once = blake3::hash(&serde_json::to_vec(&(home, id, &supplied)).map_err(|error| error.to_string())?).to_hex().to_string();
        let opened = self.open_shell_here_from(crate::shell_start::ShellStart {
            token: once, title: meta.title, cwd: cwd.display().to_string(), command: String::new(), cols: 80, rows: 24,
        }, Some(ancestry), Some(shell)).await;
        let mut gate = RowGate::acquire(&directory)?;
        let mut held = recovery_attempt::Held::load(&mut gate)?;
        match opened {
            Ok(next) => {
                held.finish_shell(&arguments, &token, recovery_attempt::ActionResult::ShellOpened { id: next.clone() })?;
                Ok(ShellRunResult::Opened { id: next, previous: id.clone() })
            }
            Err(_) => {
                // ExistingOnce/host spawn can have side effects before an Err.
                // Keep the durable pending acceptance; never invent refusal or
                // rerun it just because a response/Ready was not observed.
                Ok(shell_replay(held.action_replay(&arguments, &token)?.ok_or("accepted Shell action disappeared")?, id, &supplied.epoch))
            }
        }
    }
}
