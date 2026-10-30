//! One bounded durable attempt per existing row. No spawn, OS-process probe,
//! independent lock or background registry lives here. Every mutation borrows
//! the already-held row gate; the runtime adapter must supply actual facts.
use super::RowGate;
use crate::{SessionId, session_content::journal::Identity};
use crate::{
    agent_process::Control,
    session_content::journal::{Journal, JournalLease, JournalReservation},
};
use khor_core::DeviceId;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    path::Path,
    sync::{Arc, Weak},
};

const VERSION: u8 = 1;
const BYTES: usize = 16 * 1024;
const NAME: &str = "recovery-attempt.json";

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Arguments {
    pub(super) home: DeviceId,
    pub(super) id: SessionId,
    pub(super) expected_run: Option<String>,
    operation: Operation,
    native: Identity,
    // Hash the effective launch arguments/context, never retain their payload.
    launch_digest: String,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Operation {
    RecoverNative,
    InitialNew,
}
impl Arguments {
    pub(super) fn initial_new(native: Identity, launch: &[u8]) -> Result<Self, String> {
        let mut value = Self::native(native, None, launch)?;
        value.operation = Operation::InitialNew;
        Ok(value)
    }
    pub(super) fn native(
        native: Identity,
        expected_run: Option<String>,
        launch: &[u8],
    ) -> Result<Self, String> {
        let value = Self {
            home: native.home,
            id: crate::adaptor::id_for(&native.native_id),
            expected_run,
            native,
            operation: Operation::RecoverNative,
            launch_digest: blake3::hash(launch).to_hex().to_string(),
        };
        value.validate()?;
        Ok(value)
    }
    fn validate(&self) -> Result<(), String> {
        if self.home != self.native.home
            || self.id != crate::adaptor::id_for(&self.native.native_id)
            || self.native.agent.is_empty()
            || self.native.native_id.is_empty()
            || serde_json::to_vec(&self.native)
                .map_err(|e| e.to_string())?
                .len()
                > 4096
        {
            return Err("recovery arguments need their exact native identity".into());
        }
        hex(&self.launch_digest, 64)?;
        if let Some(run) = &self.expected_run {
            hex(run, 32)?;
        }
        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Token {
    pub(super) epoch: u64,
    nonce: String,
}
impl Token {
    pub(super) fn new(epoch: u64, nonce: String) -> Result<Self, String> {
        let value = Self { epoch, nonce };
        value.validate()?;
        Ok(value)
    }
    fn validate(&self) -> Result<(), String> {
        if self.epoch == 0 {
            return Err("invalid recovery attempt epoch".into());
        }
        hex(&self.nonce, 32)
    }
}

/// Structured identity only. A later Mac/Linux adapter must capture and compare
/// actual boot/start facts; serialization or a PID does not establish ownership.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ProcessStamp {
    pub(super) pid: u32,
    pub(super) parent: u32,
    pub(super) group: u32,
    pub(super) boot: String,
    pub(super) start: ProcessStart,
}
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "os", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum ProcessStart {
    Mac {
        seconds: u64,
        microseconds: u32,
    },
    Linux {
        ticks: u64,
        namespace_device: u64,
        namespace_inode: u64,
    },
}
impl ProcessStamp {
    pub(super) fn validate(&self) -> Result<(), String> {
        if self.pid <= 1
            || self.pid > i32::MAX as u32
            || self.group <= 1
            || self.group > i32::MAX as u32
            || self.parent > i32::MAX as u32
        {
            return Err("invalid recorded process identity".into());
        }
        hex(&self.boot, 32)?;
        match self.start {
            ProcessStart::Mac {
                seconds,
                microseconds,
            } if seconds > 0 && microseconds < 1_000_000 => Ok(()),
            ProcessStart::Linux {
                ticks,
                namespace_inode,
                ..
            } if ticks > 0 && namespace_inode > 0 => Ok(()),
            _ => Err("invalid recorded process birth".into()),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
enum Provider {
    NotStarted,
    Starting,
    Owned {
        process: ProcessStamp,
    },
    Exited {
        pid: u32,
        code: Option<i32>,
    },
    /// Finite same-context evidence, not a fabricated ProcessExit receipt.
    ForegroundAbsent {
        process: ProcessStamp,
    },
    Unknown {
        last: Option<ProcessStamp>,
        actual_pid: Option<u32>,
        reason: UnknownReason,
    },
}
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum UnknownReason {
    BirthUnavailable,
    CleanupUnconfirmed,
}
impl Provider {
    fn no_foreground_owner(&self) -> bool {
        matches!(self, Self::NotStarted | Self::Exited { .. } | Self::ForegroundAbsent { .. })
    }
    fn validate(&self) -> Result<(), String> {
        match self {
            Self::Owned { process } | Self::ForegroundAbsent { process } => process.validate(),
            Self::Unknown {
                last: Some(process),
                ..
            } => process.validate(),
            Self::Exited { pid, .. } if *pid <= 1 || *pid > i32::MAX as u32 => {
                Err("invalid reaped provider PID".into())
            }
            _ => Ok(()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Phase {
    Reserved,
    Starting,
    PublishingBase,
    BasePublished,
    PublishingRun,
    RunPublished,
    Refused,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Recorded {
    Opened { id: SessionId, run_id: String },
    Refused { reason: Reason },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Reason {
    Unavailable,
    InitializationRefused,
    SetupFailed,
    Cancelled,
}

/// The actual Initialize v1 observation for this exact attempt's native source
/// and effective launch digest. It is not permission to spawn or skip guards.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct InitializedSupport {
    load: bool,
    resume: bool,
}
impl From<khor_acp::RecoveryCapabilities> for InitializedSupport {
    fn from(value: khor_acp::RecoveryCapabilities) -> Self {
        Self { load: value.load, resume: value.resume }
    }
}
impl InitializedSupport {
    fn projection(self) -> khor_core::recovery::RecoverySupport {
        use khor_core::recovery::RecoverySupport;
        if self.resume { RecoverySupport::Resume } else if self.load { RecoverySupport::Load } else { RecoverySupport::Unsupported }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Latest {
    token: Token,
    arguments: Arguments,
    run_id: String,
    phase: Phase,
    host: Option<ProcessStamp>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    unverified_host_pid: Option<u32>,
    provider: Provider,
    generation: Option<String>,
    result: Option<Recorded>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    support: Option<InitializedSupport>,
}
/// A caller action is not the execution owner. A Join may advance the action
/// high-water mark while the existing producer continues using its exact key.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum ActionArguments {
    Native { arguments: Arguments },
    Shell { home: DeviceId, id: SessionId, expected_run: String, launch_digest: String },
}
impl ActionArguments {
    pub(super) fn shell(home: DeviceId, id: SessionId, expected_run: String, launch: &[u8]) -> Result<Self, String> {
        let value = Self::Shell { home, id, expected_run, launch_digest: blake3::hash(launch).to_hex().to_string() };
        value.validate()?;
        Ok(value)
    }
    fn scope(&self) -> (DeviceId, &SessionId) {
        match self { Self::Native { arguments } => (arguments.home, &arguments.id), Self::Shell { home, id, .. } => (*home, id) }
    }
    fn validate(&self) -> Result<(), String> {
        match self {
            Self::Native { arguments } => {
                arguments.validate()?;
                if arguments.operation != Operation::RecoverNative { return Err("New cannot be an explicit recovery action".into()); }
            }
            Self::Shell { id, expected_run, launch_digest, .. } => {
                if id.0.is_empty() || id.0.len() > 4096 || expected_run != &id.0 {
                    return Err("a new Shell action must name its exact previous SID".into());
                }
                hex(launch_digest, 64)?;
            }
        }
        Ok(())
    }
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum ActionResult {
    JoinPending,
    Join { current_run: Option<String> },
    ShellPending,
    DirectoryRequired,
    ShellOpened { id: SessionId },
    Refused { reason: String },
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LatestAction { token: Token, arguments: ActionArguments, result: ActionResult }
impl LatestAction {
    fn validate(&self) -> Result<(), String> {
        self.token.validate()?;
        self.arguments.validate()?;
        match (&self.arguments, &self.result) {
            (ActionArguments::Native { .. }, ActionResult::JoinPending) => {},
            (ActionArguments::Native { arguments }, ActionResult::Join { current_run }) if &arguments.expected_run == current_run => {
                if let Some(run) = current_run { hex(run, 32)?; }
            }
            (ActionArguments::Shell { id: previous, .. }, ActionResult::ShellOpened { id }) if id != previous && !id.0.is_empty() && id.0.len() <= 4096 => {}
            (ActionArguments::Shell { .. }, ActionResult::ShellPending | ActionResult::DirectoryRequired) => {}
            (_, ActionResult::Refused { reason }) if !reason.is_empty() && reason.len() <= 256 => {}
            _ => return Err("recovery action and result disagree".into()),
        }
        Ok(())
    }
}
/// None means a never-accepted next token; a retired token cannot become work.
pub(super) enum ActionReplay { Recorded(ActionResult), Retired }
/// Only pure admission validation after a successful document load. Disk,
/// provider and publication failures remain the outer Err, never this refusal.
pub(super) enum InputDecision<T> { Continue(T), Refused(String) }

/// Permission only for the invocation which durably chose Join. It cannot be
/// serialized/cloned or reconstructed by a duplicate/reconnected request.
pub(super) struct JoinCheck { token: Token, arguments: Arguments }
pub(super) enum JoinAdmission { Previous(Reservation), Check(JoinCheck) }
pub(super) fn join_arguments(native: Identity, expected: Option<String>) -> Result<Arguments, String> {
    Arguments::native(native, expected, b"khor/current-execution-join/v1")
}


#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    version: u8,
    high_water: u64,
    // The one current execution, independent of the most recent caller action.
    latest: Option<Latest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    action: Option<LatestAction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cleanup_barrier: Option<Identity>,
}
impl Document {
    fn validate(&self) -> Result<(), String> {
        if let Some(barrier) = &self.cleanup_barrier {
            Arguments::initial_new(barrier.clone(), b"cleanup-barrier")?;
        }
        if self.version != VERSION { return Err("invalid recovery document version".into()); }
        if let Some(action) = &self.action {
            action.validate()?;
            if self.high_water != action.token.epoch || self.latest.as_ref().is_some_and(|execution| {
                let (home, id) = action.arguments.scope();
                execution.token.epoch >= action.token.epoch || execution.arguments.home != home || &execution.arguments.id != id
            }) { return Err("action high-water belongs to another execution row".into()); }
            if self.cleanup_barrier.as_ref().is_some_and(|barrier| match &action.arguments {
                ActionArguments::Native { arguments } => barrier != &arguments.native,
                ActionArguments::Shell { .. } => true,
            }) { return Err("cleanup barrier belongs to another action source".into()); }
        }
        let Some(v) = &self.latest else {
            return if self.action.is_some() || (self.high_water == 0 && self.cleanup_barrier.is_some()) {
                Ok(())
            } else {
                Err("empty recovery document is not an admissible row".into())
            };
        };
        if self.cleanup_barrier.as_ref().is_some_and(|barrier| barrier != &v.arguments.native) {
            return Err("cleanup barrier belongs to another exact native source".into());
        }
        if self.action.is_none() && self.high_water != v.token.epoch {
            return Err("invalid recovery high-water record".into());
        }
        v.token.validate()?;
        v.arguments.validate()?;
        hex(&v.run_id, 32)?;
        if let Some(host) = &v.host {
            host.validate()?;
        }
        if let Some(pid) = v.unverified_host_pid {
            // This is the caller's raw self PID, never a Unix probe target.
            if pid <= 1 || v.host.is_some() || v.arguments.operation != Operation::InitialNew {
                return Err("invalid unverified initial host fact".into());
            }
        }
        let host_recorded = v.host.is_some() || v.unverified_host_pid.is_some();
        if v.support.is_some() && (!host_recorded || matches!(v.provider, Provider::NotStarted | Provider::Starting)) {
            return Err("Initialize facts have no started execution owner".into());
        }
        v.provider.validate()?;
        if let Provider::Unknown {
            last, actual_pid, ..
        } = &v.provider
        {
            if actual_pid.is_some_and(|pid| pid <= 1 || pid > i32::MAX as u32)
                || last
                    .as_ref()
                    .is_some_and(|process| Some(process.pid) != *actual_pid)
            {
                return Err("unknown provider has inconsistent recorded PID facts".into());
            }
        }
        if let Provider::Owned { process } | Provider::ForegroundAbsent { process } = &v.provider {
            if process.group != process.pid
                || v.host.as_ref().is_none_or(|host| {
                    process.parent != host.pid || !super::proc_identity::same_context(host, process)
                })
            {
                return Err("recorded provider has another foreground owner".into());
            }
        }
        if let Some(generation) = &v.generation {
            hex(generation, 32)?;
        }
        match (&v.phase, &v.result) {
            (Phase::RunPublished, Some(Recorded::Opened { id, run_id }))
                if id == &v.arguments.id
                    && run_id == &v.run_id
                    && host_recorded
                    && !matches!(v.provider, Provider::NotStarted | Provider::Starting) => {}
            (Phase::Refused, Some(Recorded::Refused { .. }))
                if v.provider.no_foreground_owner()
                    && (host_recorded || matches!(v.provider, Provider::NotStarted)) => {}
            (Phase::Reserved, None)
                if v.arguments.operation == Operation::RecoverNative
                    && v.host.is_none()
                    && v.unverified_host_pid.is_none()
                    && v.generation.is_none()
                    && matches!(v.provider, Provider::NotStarted) => {}
            (Phase::Starting, None) if host_recorded && v.generation.is_none() => {}
            (Phase::PublishingBase | Phase::BasePublished | Phase::PublishingRun, None)
                if host_recorded
                    && !matches!(v.provider, Provider::NotStarted | Provider::Starting) => {}
            _ => return Err("recovery phase and recorded result disagree".into()),
        }
        Ok(())
    }
}

/// This credential is data, not an unlocked permission to publish. Each action
/// must reload the latest document under RowGate and check the entire key.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AttemptKey {
    token: Token,
    arguments: Arguments,
    run_id: String,
}

impl AttemptKey {
    pub(super) fn scope(&self) -> (DeviceId, &SessionId) {
        (self.arguments.home, &self.arguments.id)
    }
}

/// Only the successful in-process New adapter may provide these facts. The
/// reservation is still the sole File owner and has not appended/bound a base.
pub(super) struct NewInvocation<'a> {
    pub(super) reservation: &'a JournalReservation,
    pub(super) control: &'a Arc<Control>,
    pub(super) actual_pid: Option<u32>,
    pub(super) host: Option<ProcessStamp>,
    pub(super) provider: Option<ProcessStamp>,
    pub(super) support: Option<khor_acp::RecoveryCapabilities>,
}

/// Nonserializable, non-Clone permission for this one live New invocation.
/// Reloading its data key cannot recover the Journal/transport authority.
pub(super) struct InitialNewAuthority {
    key: AttemptKey,
    journal: JournalLease,
    control: Weak<Control>,
    actual_pid: Option<u32>,
    issuing_pid: u32,
}
pub(super) enum Reservation {
    Joined { current_run: Option<String> },
    Start(AttemptKey),
    Recorded(Recorded),
    /// Never label this Pending without independently proving the exact owner.
    Unfinished {
        phase: Phase,
        host: Option<ProcessStamp>,
    },
    Retired,
    UnknownOwner,
}

/// A bounded snapshot to probe outside RowGate; applying its nonserial proof
/// reloads and checks the exact same key and recorded foreground afterwards.
pub(super) struct RegisteredForeground {
    key: AttemptKey,
    pub(super) process: ProcessStamp,
    pub(super) host: ProcessStamp,
    pub(super) slot_ended: bool,
}

/// The only durable writer is the caller's existing short row transaction.
pub(super) struct Held<'a> {
    gate: &'a mut RowGate,
    document: Option<Document>,
    failed: bool,
}
impl<'a> Held<'a> {
    pub(super) fn load(gate: &'a mut RowGate) -> Result<Self, String> {
        let document = read(&gate.dir)?;
        Ok(Self {
            gate,
            document,
            failed: false,
        })
    }

    /// Necessary New exception: its native ID did not exist before the real
    /// New response. This seeds one observed invocation, never a recovery claim.
    pub(super) fn seed_initial_new(
        &mut self,
        arguments: Arguments,
        invocation: NewInvocation<'_>,
    ) -> Result<InitialNewAuthority, String> {
        self.ensure()?;
        arguments.validate()?;
        if arguments.operation != Operation::InitialNew
            || arguments.expected_run.is_some()
            || self.document.is_some()
            || self.gate.read_meta_or_new()?.is_some()
        {
            return Err("initial New cannot replace an existing row or recovery attempt".into());
        }
        if invocation.reservation.identity() != &arguments.native
            || invocation.reservation.has_published_base()?
        {
            return Err("initial New requires its first exact native journal reservation".into());
        }
        invocation.reservation.check_binding(&self.gate.dir)?;
        let issuing_pid = std::process::id();
        if issuing_pid <= 1
            || invocation
                .actual_pid
                .is_some_and(|pid| pid <= 1 || pid > i32::MAX as u32)
        {
            return Err("invalid initial process facts".into());
        }
        if let Some(host) = &invocation.host {
            host.validate()?;
            if host.pid != issuing_pid {
                return Err("initial host identity is not this invocation".into());
            }
        }
        if let Some(provider) = &invocation.provider {
            provider.validate()?;
            if Some(provider.pid) != invocation.actual_pid
                || provider.parent != issuing_pid
                || provider.group != provider.pid
            {
                return Err("initial provider identity differs from its actual transport".into());
            }
        }
        let provider = match (&invocation.host, &invocation.provider) {
            (Some(host), Some(provider)) if super::proc_identity::same_context(host, provider) => {
                Provider::Owned {
                    process: provider.clone(),
                }
            }
            (Some(_), Some(_)) => {
                return Err("initial process facts belong to different kernel contexts".into());
            }
            _ => Provider::Unknown {
                last: invocation.provider.clone(),
                actual_pid: invocation.actual_pid,
                reason: UnknownReason::BirthUnavailable,
            },
        };
        let token = Token::new(1, crate::link::fresh_hex()?)?;
        let run_id = crate::link::fresh_hex()?;
        let key = AttemptKey {
            token: token.clone(),
            arguments: arguments.clone(),
            run_id: run_id.clone(),
        };
        let authority = InitialNewAuthority {
            key,
            journal: invocation.reservation.lease(),
            control: Arc::downgrade(invocation.control),
            actual_pid: invocation.actual_pid,
            issuing_pid,
        };
        let unverified_host_pid = if invocation.host.is_none() {
            Some(issuing_pid)
        } else {
            None
        };
        self.store(Document {
            version: VERSION,
            high_water: 1,
            action: None,
            cleanup_barrier: None,
            latest: Some(Latest {
                token,
                arguments,
                run_id,
                phase: Phase::Starting,
                host: invocation.host,
                unverified_host_pid,
                provider,
                generation: None,
                result: None,
                support: invocation.support.map(InitializedSupport::from),
            }),
        })?;
        Ok(authority)
    }

    pub(super) fn publish_initial_base<T>(
        &mut self,
        authority: &InitialNewAuthority,
        journal: &mut Journal,
        control: &Arc<Control>,
        publish: impl FnOnce(&RowGate, &mut Journal) -> Result<T, String>,
    ) -> Result<T, String> {
        self.check_initial(authority, journal, control, Phase::Starting)?;
        self.base_step(&authority.key, None, |gate| publish(gate, journal))
    }
    pub(super) fn publish_initial_run<T>(
        &mut self,
        authority: &InitialNewAuthority,
        journal: &mut Journal,
        control: &Arc<Control>,
        publish: impl FnOnce(&RowGate, &mut Journal, &str) -> Result<T, String>,
    ) -> Result<T, String> {
        self.check_initial(authority, journal, control, Phase::BasePublished)?;
        self.run_step(&authority.key, |gate, run| publish(gate, journal, run))
    }
    fn check_initial(
        &self,
        authority: &InitialNewAuthority,
        journal: &Journal,
        control: &Arc<Control>,
        phase: Phase,
    ) -> Result<(), String> {
        let value = self.current(&authority.key)?;
        if self.has_cleanup_barrier() {
            return Err("native source has an unconfirmed New cleanup".into());
        }
        let same_transport = authority
            .control
            .upgrade()
            .is_some_and(|owner| Arc::ptr_eq(&owner, control));
        let current_owner = match &value.provider {
            Provider::Owned { process } => Some(process.pid) == authority.actual_pid,
            Provider::Unknown {
                actual_pid,
                reason: UnknownReason::BirthUnavailable,
                ..
            } => *actual_pid == authority.actual_pid,
            _ => false,
        };
        if value.phase != phase
            || value.arguments.operation != Operation::InitialNew
            || authority.issuing_pid != std::process::id()
            || !same_transport
            || !current_owner
            || journal.identity() != &value.arguments.native
            || !journal.owns_lease(&authority.journal)
        {
            return Err("initial New no longer owns this journal and live invocation".into());
        }
        Ok(())
    }
    pub(super) fn registered_foreground(&self, native: &Identity) -> Result<Option<RegisteredForeground>, String> {
        self.ensure()?;
        if self.has_cleanup_barrier() { return Ok(None); }
        let Some(latest) = self.document.as_ref().and_then(|doc| doc.latest.as_ref()) else {
            return Ok(None);
        };
        if &latest.arguments.native != native {
            return Err("registered execution belongs to another full native source".into());
        }
        let Provider::Owned { process } = &latest.provider else { return Ok(None); };
        Ok(Some(RegisteredForeground {
            key: AttemptKey { token: latest.token.clone(), arguments: latest.arguments.clone(), run_id: latest.run_id.clone() },
            process: process.clone(),
            host: latest.host.clone().ok_or("registered foreground has no actual host")?,
            slot_ended: self.slot_ended(latest)?,
        }))
    }

    fn slot_ended(&self, latest: &Latest) -> Result<bool, String> {
        let Some(meta) = self.gate.read_meta_or_new()? else { return Ok(false); };
        Ok(latest.phase == Phase::RunPublished && meta.kind == khor_core::kind::GUI
            && meta.id == latest.arguments.id.0
            && meta.gui_native.as_ref() == Some(&latest.arguments.native)
            && meta.pid == latest.host.as_ref().map(|host| host.pid).or(latest.unverified_host_pid)
            && meta.gui_run.as_ref().is_some_and(|run| run.id == latest.run_id
                && run.state.gui_run.as_ref() == Some(&run.id) && run.state.exit.is_some()))
    }

    pub(super) fn foreground_absent(
        &mut self,
        registered: &RegisteredForeground,
        proof: &super::proc_identity::AbsentForeground,
        host_absent: Option<&super::proc_identity::AbsentForeground>,
    ) -> Result<(), String> {
        let current = self.current(&registered.key)?;
        if current.host.as_ref() != Some(&registered.host)
            || (!self.slot_ended(current)? && !host_absent.is_some_and(|proof| proof.is_for(&registered.host)))
        {
            return Err("the previous Khor execution is still active or unknown".into());
        }
        if self.has_cleanup_barrier() || !proof.is_for(&registered.process) {
            return Err("foreground absence does not match this registered owner".into());
        }
        self.change(&registered.key, |latest| {
            if !matches!(&latest.provider, Provider::Owned { process } if process == &registered.process) {
                return Err("registered foreground changed during its read-only probe".into());
            }
            latest.provider = Provider::ForegroundAbsent { process: registered.process.clone() };
            Ok(())
        })
    }

    pub(super) fn record_support(
        &mut self,
        key: &AttemptKey,
        support: khor_acp::RecoveryCapabilities,
    ) -> Result<(), String> {
        let support = InitializedSupport::from(support);
        self.change(key, |latest| {
            if latest.host.is_none() && latest.unverified_host_pid.is_none()
                || matches!(latest.provider, Provider::NotStarted | Provider::Starting)
            {
                return Err("Initialize facts have no actual started provider owner".into());
            }
            if latest.support.is_some_and(|previous| previous != support) {
                return Err("Initialize facts changed within one startup".into());
            }
            latest.support = Some(support);
            Ok(())
        })
    }

    pub(super) fn check_claim(
        &self,
        key: &AttemptKey,
        arguments: &Arguments,
    ) -> Result<(), String> {
        let latest = self.current(key)?;
        if self.has_cleanup_barrier() || &latest.arguments != arguments
            || latest.phase != Phase::Reserved || !matches!(latest.provider, Provider::NotStarted)
        {
            return Err("recovery ghost does not own these exact reserved arguments".into());
        }
        Ok(())
    }

    fn has_cleanup_barrier(&self) -> bool {
        self.document.as_ref().is_some_and(|doc| doc.cleanup_barrier.is_some())
    }

    /// A colliding real New was refused and its actual cleanup is unconfirmed.
    /// This is not a new provider claim: preserve every old attempt/result and
    /// only quarantine an exactly bound source while its File is still held.
    pub(super) fn quarantine_new(
        &mut self,
        reservation: &JournalReservation,
    ) -> Result<(), String> {
        self.ensure()?;
        if !reservation.holds_file() {
            return Err("New quarantine no longer holds its exact journal File".into());
        }
        let native = reservation.identity();
        let meta = self.gate.read_meta_or_new()?;
        let bound = crate::session_content::journal::bound_identity(&self.gate.dir)?;
        let recorded = meta.as_ref().and_then(|meta| meta.gui_native.as_ref());
        if bound.as_ref().is_some_and(|bound| bound != native)
            || recorded.is_some_and(|recorded| recorded != native)
            || meta.as_ref().is_some_and(|meta| !meta.id.is_empty()
                && meta.id != crate::adaptor::id_for(&native.native_id).0)
            || (bound.as_ref() != Some(native) && recorded != Some(native))
        {
            return Err("unconfirmed New is not proven to be this row's exact source".into());
        }
        reservation.check_binding(&self.gate.dir)?;
        let mut next = self.document.clone().unwrap_or(Document {
            version: VERSION, high_water: 0, latest: None, action: None, cleanup_barrier: None,
        });
        if next.latest.as_ref().is_some_and(|latest| &latest.arguments.native != native)
            || next.cleanup_barrier.as_ref().is_some_and(|barrier| barrier != native)
        {
            return Err("unconfirmed New cannot quarantine another source".into());
        }
        next.cleanup_barrier = Some(native.clone());
        self.store(next)
    }

    /// Parent-side marker revalidation runs after taking this same short gate.
    /// The marker itself may have become visible before RunPublished was stored.
    pub(super) fn ready_run(
        &self,
        native: &Identity,
        ghost_pid: u32,
        expected: Option<&AttemptKey>,
    ) -> Result<&str, String> {
        self.ensure()?;
        if self.has_cleanup_barrier() {
            return Err("GUI Ready has an unconfirmed cleanup barrier".into());
        }
        let latest = self.document.as_ref().and_then(|doc| doc.latest.as_ref())
            .ok_or("GUI Ready has no durable execution result")?;
        if let Some(key) = expected {
            self.current(key)?;
        } else if latest.arguments.operation != Operation::InitialNew {
            return Err("known-native Ready requires its exact recovery token".into());
        }
        let same_host = latest.host.as_ref().map(|host| host.pid)
            .or(latest.unverified_host_pid) == Some(ghost_pid);
        let actual_owner = matches!(latest.provider, Provider::Owned { .. })
            || (latest.arguments.operation == Operation::InitialNew
                && matches!(latest.provider, Provider::Unknown {
                    reason: UnknownReason::BirthUnavailable, ..
                }));
        if &latest.arguments.native != native || !same_host || !actual_owner
            || latest.phase != Phase::RunPublished
            || !matches!(&latest.result, Some(Recorded::Opened { id, run_id })
                if id == &latest.arguments.id && run_id == &latest.run_id)
        {
            return Err("GUI Ready is not this ghost's durable published execution".into());
        }
        Ok(&latest.run_id)
    }

    pub(super) fn settle_initial(
        &mut self,
        authority: &InitialNewAuthority,
        control: &Arc<Control>,
        receipt: Result<&khor_acp::ProcessExit, ()>,
        retire_slot: impl FnOnce(&RowGate, &SessionId, &Identity, &str, u32) -> Result<(), String>,
    ) -> Result<(), String> {
        self.current(&authority.key)?;
        if authority.issuing_pid != std::process::id()
            || !authority.control.upgrade().is_some_and(|owner| Arc::ptr_eq(&owner, control))
        {
            return Err("initial cleanup belongs to another transport owner".into());
        }
        let provider = match (authority.actual_pid, receipt) {
            (Some(pid), Ok(receipt)) => self.confirmed_exit(&authority.key, pid, receipt),
            _ => self.cleanup_unknown(&authority.key),
        };
        let provider = provider.and_then(|()| {
            let latest = self.current(&authority.key)?;
            if latest.result.is_none() && latest.provider.no_foreground_owner() {
                self.refuse(&authority.key, Reason::SetupFailed)
            } else { Ok(()) }
        });
        // The key/Control/epoch were validated under this still-held gate.
        // Even if the attempt file is unwritable, exact same-run Meta cleanup
        // is independently safe and must not be skipped by that first error.
        let slot = retire_slot(self.gate, &authority.key.arguments.id, &authority.key.arguments.native,
            &authority.key.run_id, authority.issuing_pid);
        match (provider, slot) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(why), Ok(())) | (Ok(()), Err(why)) => Err(why),
            (Err(provider), Err(slot)) => Err(format!("{provider}; Khor slot cleanup: {slot}")),
        }
    }

    pub(super) fn replay(
        &self,
        arguments: &Arguments,
        token: &Token,
    ) -> Result<Option<Reservation>, String> {
        self.ensure()?;
        arguments.validate()?;
        token.validate()?;
        if self.has_cleanup_barrier() {
            return Ok(Some(Reservation::UnknownOwner));
        }
        if arguments.operation != Operation::RecoverNative {
            return Err("initial New cannot enter preghost recovery reservation".into());
        }
        if self.document.as_ref().is_some_and(|doc| {
            let Some(latest) = &doc.latest else {
                return doc.action.as_ref().is_none_or(|action| action.arguments.scope() != (arguments.home, &arguments.id));
            };
            let origin = &latest.arguments;
            origin.home != arguments.home
                || origin.id != arguments.id
                || origin.native != arguments.native
        }) {
            return Err("recovery attempt belongs to another home or native row".into());
        }
        let high_water = self.document.as_ref().map_or(0, |doc| doc.high_water);
        if token.epoch < high_water {
            return Ok(Some(Reservation::Retired));
        }
        if token.epoch == high_water {
            if let Some(action) = self.document.as_ref().and_then(|doc| doc.action.as_ref()) {
                if &action.token != token || action.arguments != (ActionArguments::Native { arguments: arguments.clone() }) {
                    return Err("recovery token was reused with different arguments".into());
                }
                return Ok(Some(match &action.result {
                    ActionResult::Join { current_run } => Reservation::Joined { current_run: current_run.clone() },
                    ActionResult::JoinPending | ActionResult::Refused { .. } => Reservation::UnknownOwner,
                    _ => return Err("native token belongs to a Shell action".into()),
                }));
            }
            let latest = &self
                .document
                .as_ref()
                .ok_or("missing latest recovery attempt")?
                .latest.as_ref().ok_or("recovery row is quarantined without an attempt")?;
            if token != &latest.token || arguments != &latest.arguments {
                return Err("recovery token was reused with different arguments".into());
            }
            return Ok(Some(match &latest.result {
                Some(result) => Reservation::Recorded(result.clone()),
                None if matches!(latest.provider, Provider::Unknown { .. }) => {
                    Reservation::UnknownOwner
                }
                None => Reservation::Unfinished {
                    phase: latest.phase,
                    host: latest.host.clone(),
                },
            }));
        }
        if Some(token.epoch) != high_water.checked_add(1) {
            return Err("recovery epoch is not the next offer".into());
        }
        Ok(None)
    }

    pub(super) fn check_native_input(&self, arguments: &Arguments, token: &Token) -> Result<InputDecision<Option<Reservation>>, String> {
        self.ensure()?;
        // replay is immutable and performs no IO: after ensure, every Err in
        // this call is a pure argument/origin/epoch/nonce mismatch.
        Ok(match self.replay(arguments, token) { Ok(value) => InputDecision::Continue(value), Err(reason) => InputDecision::Refused(reason) })
    }
    pub(super) fn check_join_input(&self, arguments: &Arguments, token: &Token) -> Result<InputDecision<Option<Reservation>>, String> {
        self.ensure()?;
        Ok(match self.replay_join(arguments, token) { Ok(value) => InputDecision::Continue(value), Err(reason) => InputDecision::Refused(reason) })
    }
    pub(super) fn check_shell_input(&self, arguments: &ActionArguments, token: &Token) -> Result<InputDecision<Option<ActionReplay>>, String> {
        self.ensure()?;
        // A cleanup barrier is an unknown owner, not invalid caller input.
        if self.has_cleanup_barrier() { return Err("source cleanup is unconfirmed".into()); }
        Ok(match self.action_replay(arguments, token) { Ok(value) => InputDecision::Continue(value), Err(reason) => InputDecision::Refused(reason) })
    }

    /// Replays only an explicitly accepted Join before any launcher lookup.
    /// Native execution tokens still go through their actual effective args.
    pub(super) fn replay_join(&self, arguments: &Arguments, token: &Token) -> Result<Option<Reservation>, String> {
        self.ensure()?; arguments.validate()?; token.validate()?;
        let high = self.document.as_ref().map_or(0, |doc| doc.high_water);
        if token.epoch < high { return Ok(Some(Reservation::Retired)); }
        if token.epoch > high && Some(token.epoch) != high.checked_add(1) { return Err("recovery epoch is not the next offer".into()); }
        if token.epoch == high && self.document.as_ref().is_some_and(|doc| doc.action.is_some()) {
            return self.replay(arguments, token);
        }
        Ok(None)
    }
    pub(super) fn is_execution_token(&self, token: &Token) -> bool {
        self.document.as_ref().is_some_and(|doc| doc.high_water == token.epoch && doc.action.is_none() && doc.latest.as_ref().is_some_and(|execution| execution.token == *token))
    }
    fn join_meta(&self, arguments: &Arguments, legacy: Option<&(Option<String>, Identity)>) -> Result<(), String> {
        let meta = self.gate.read_meta_or_new()?.ok_or("the joined GUI row disappeared")?;
        self.gate.check_native(&arguments.id, Some(&meta), &arguments.native, legacy)?;
        if meta.kind != khor_core::kind::GUI || meta.gui_run.as_ref().map(|run| &run.id) != arguments.expected_run.as_ref() || super::gui_slot_ended(&meta) {
            return Err("the joined GUI execution changed".into());
        }
        Ok(())
    }
    /// Freeze Join intent before its real endpoint handshake. Even a failed
    /// handshake cannot later turn this accepted token into provider startup.
    pub(super) fn begin_join(&mut self, arguments: Arguments, token: Token,
        legacy: Option<&(Option<String>, Identity)>,
    ) -> Result<JoinAdmission, String> {
        if let Some(previous) = self.replay(&arguments, &token)? { return Ok(JoinAdmission::Previous(previous)); }
        self.join_meta(&arguments, legacy)?;
        let mut next = self.document.clone().unwrap_or(Document { version: VERSION, high_water: 0, latest: None, action: None, cleanup_barrier: None });
        next.high_water = token.epoch;
        next.action = Some(LatestAction { token: token.clone(), arguments: ActionArguments::Native { arguments: arguments.clone() }, result: ActionResult::JoinPending });
        self.store(next)?;
        Ok(JoinAdmission::Check(JoinCheck { token, arguments }))
    }
    pub(super) fn finish_join(&mut self, ticket: JoinCheck,
        legacy: Option<&(Option<String>, Identity)>,
        validate_live: impl FnOnce(&mut RowGate) -> Result<(), String>,
    ) -> Result<Reservation, String> {
        let args = ActionArguments::Native { arguments: ticket.arguments.clone() };
        if !matches!(self.action_replay(&args, &ticket.token)?, Some(ActionReplay::Recorded(ActionResult::JoinPending))) {
            return Err("Join checking authority has been retired".into());
        }
        validate_live(self.gate)?;
        self.join_meta(&ticket.arguments, legacy)?;
        let current_run = ticket.arguments.expected_run;
        let mut next = self.document.clone().ok_or("Join intent is missing")?;
        next.action.as_mut().ok_or("Join intent is missing")?.result = ActionResult::Join { current_run: current_run.clone() };
        self.store(next)?;
        Ok(Reservation::Joined { current_run })
    }
    #[cfg(test)]
    fn join(&mut self, arguments: Arguments, token: Token, legacy: Option<&(Option<String>, Identity)>, validate_live: impl FnOnce(&mut RowGate) -> Result<(), String>) -> Result<Reservation, String> {
        match self.begin_join(arguments, token, legacy)? {
            JoinAdmission::Previous(previous) => Ok(previous),
            JoinAdmission::Check(ticket) => self.finish_join(ticket, legacy, validate_live),
        }
    }

    pub(super) fn action_replay(&self, arguments: &ActionArguments, token: &Token) -> Result<Option<ActionReplay>, String> {
        self.ensure()?; arguments.validate()?; token.validate()?;
        if self.has_cleanup_barrier() { return Err("source cleanup is unconfirmed".into()); }
        let high = self.document.as_ref().map_or(0, |doc| doc.high_water);
        if token.epoch < high { return Ok(Some(ActionReplay::Retired)); }
        if token.epoch == high {
            let action = self.document.as_ref().and_then(|doc| doc.action.as_ref()).ok_or("token belongs to another execution action")?;
            if &action.token != token || &action.arguments != arguments { return Err("recovery token was reused with different arguments".into()); }
            return Ok(Some(ActionReplay::Recorded(action.result.clone())));
        }
        if Some(token.epoch) != high.checked_add(1) { return Err("recovery epoch is not the next offer".into()); }
        if self.document.as_ref().is_some_and(|doc| {
            doc.latest.as_ref().is_some_and(|execution| (execution.arguments.home, &execution.arguments.id) != arguments.scope())
                || doc.action.as_ref().is_some_and(|previous| previous.arguments.scope() != arguments.scope())
        }) { return Err("action belongs to another execution row".into()); }
        Ok(None)
    }
    /// This durable pending acceptance precedes existingOnce/shell spawn. An
    /// interrupted response may replay Pending/Unknown, never create another B.
    pub(super) fn reserve_shell(
        &mut self, arguments: ActionArguments, token: Token,
        validate_ended: impl FnOnce(&mut RowGate) -> Result<(), String>,
    ) -> Result<Option<ActionReplay>, String> {
        if !matches!(&arguments, ActionArguments::Shell { .. }) { return Err("not a Shell action".into()); }
        if let Some(previous) = self.action_replay(&arguments, &token)? { return Ok(Some(previous)); }
        if self.document.as_ref().and_then(|doc| doc.action.as_ref()).is_some_and(|previous| matches!(previous.result, ActionResult::ShellPending)) {
            return Err("previous Shell action is unresolved; a new token cannot replay its effects".into());
        }
        validate_ended(self.gate)?;
        let mut next = self.document.clone().unwrap_or(Document { version: VERSION, high_water: 0, latest: None, action: None, cleanup_barrier: None });
        next.high_water = token.epoch;
        next.action = Some(LatestAction { token, arguments, result: ActionResult::ShellPending });
        self.store(next)?;
        Ok(None)
    }
    pub(super) fn finish_shell(&mut self, arguments: &ActionArguments, token: &Token, result: ActionResult) -> Result<(), String> {
        if !matches!(self.action_replay(arguments, token)?, Some(ActionReplay::Recorded(ActionResult::ShellPending))) {
            return Err("Shell action is no longer its accepted pending token".into());
        }
        if !matches!(result, ActionResult::ShellOpened { .. } | ActionResult::DirectoryRequired | ActionResult::Refused { .. }) { return Err("invalid completed Shell result".into()); }
        let mut next = self.document.clone().ok_or("Shell action is missing")?;
        next.action.as_mut().ok_or("Shell action is missing")?.result = result;
        self.store(next)
    }

    pub(super) fn reserve(
        &mut self,
        arguments: Arguments,
        token: Token,
        legacy: Option<&(Option<String>, Identity)>,
    ) -> Result<Reservation, String> {
        if let Some(previous) = self.replay(&arguments, &token)? {
            return Ok(previous);
        }
        // Meta.exit, a dead host, a free journal or absent leader are not a
        // substitute for its actual owned-foreground receipt or complete,
        // same-context registered foreground absence proof.
        if self.document.as_ref().is_some_and(|doc| {
            let Some(latest) = &doc.latest else { return true; };
            !matches!(latest.provider, Provider::Exited { .. } | Provider::ForegroundAbsent { .. })
                && !(latest.phase == Phase::Refused
                    && matches!(latest.provider, Provider::NotStarted))
        }) {
            return Ok(Reservation::UnknownOwner);
        }
        let meta = self.gate.read_meta_or_new()?;
        self.gate
            .check_native(&arguments.id, meta.as_ref(), &arguments.native, legacy)?;
        let current = meta
            .as_ref()
            .and_then(|meta| meta.gui_run.as_ref())
            .map(|run| run.id.as_str());
        if current != arguments.expected_run.as_deref() {
            return Err("recovery offer names another GUI execution".into());
        }
        if self.document.as_ref().and_then(|doc| doc.latest.as_ref()).is_some_and(|latest| {
            current.is_some_and(|current| current != latest.run_id
                && Some(current) != latest.arguments.expected_run.as_deref())
        }) {
            return Ok(Reservation::UnknownOwner);
        }
        if self.document.is_none() && (current.is_some()
            || meta.as_ref().is_some_and(|meta| meta.kind == khor_core::kind::GUI)) {
            return Ok(Reservation::UnknownOwner);
        }
        let run_id = crate::link::fresh_hex()?;
        let key = AttemptKey {
            token: token.clone(),
            arguments: arguments.clone(),
            run_id: run_id.clone(),
        };
        self.store(Document {
            version: VERSION,
            high_water: token.epoch,
            action: None,
            cleanup_barrier: None,
            latest: Some(Latest {
                token,
                arguments,
                run_id,
                phase: Phase::Reserved,
                host: None,
                unverified_host_pid: None,
                provider: Provider::NotStarted,
                generation: None,
                result: None,
                support: None,
            }),
        })?;
        Ok(Reservation::Start(key))
    }
    pub(super) fn spawn_failed(&mut self, key: &AttemptKey) -> Result<(), String> {
        let latest = self.current(key)?;
        if latest.phase != Phase::Reserved || latest.host.is_some() || !matches!(latest.provider, Provider::NotStarted) {
            return Err("failed spawn cannot retire an already claimed ghost".into());
        }
        self.refuse(key, Reason::SetupFailed)
    }

    pub(super) fn claim_host(
        &mut self,
        key: &AttemptKey,
        host: ProcessStamp,
    ) -> Result<(), String> {
        host.validate()?;
        self.change(key, |v| {
            if v.phase != Phase::Reserved {
                return Err("recovery ghost has already claimed this attempt".into());
            }
            v.host = Some(host);
            v.phase = Phase::Starting;
            Ok(())
        })
    }
    pub(super) fn starting_provider(&mut self, key: &AttemptKey) -> Result<(), String> {
        self.change(key, |v| {
            if v.phase != Phase::Starting || !matches!(v.provider, Provider::NotStarted) {
                return Err("provider start has already been issued".into());
            }
            v.provider = Provider::Starting;
            Ok(())
        })
    }
    pub(super) fn owned_provider(
        &mut self,
        key: &AttemptKey,
        provider: ProcessStamp,
    ) -> Result<(), String> {
        provider.validate()?;
        self.change(key, |v| {
            if v.phase != Phase::Starting
                || !matches!(v.provider, Provider::Starting)
                || provider.group != provider.pid
                || v.host.as_ref().is_none_or(|host| {
                    provider.parent != host.pid
                        || !super::proc_identity::same_context(host, &provider)
                })
            {
                return Err("provider does not belong to this starting foreground owner".into());
            }
            v.provider = Provider::Owned { process: provider };
            Ok(())
        })
    }
    pub(super) fn settle_recovery(
        &mut self,
        key: &AttemptKey,
        host: &ProcessStamp,
        actual_pid: Option<u32>,
        receipt: Result<&khor_acp::ProcessExit, ()>,
        retire_slot: impl FnOnce(&RowGate, &SessionId, &Identity, &str, u32) -> Result<(), String>,
    ) -> Result<(), String> {
        let latest = self.current(key)?;
        if latest.arguments.operation != Operation::RecoverNative || latest.host.as_ref() != Some(host)
            || host.pid != std::process::id()
        {
            return Err("recovery cleanup belongs to another actual host".into());
        }
        let provider = match (actual_pid, receipt) {
            (Some(pid), Ok(receipt)) => self.confirmed_exit(key, pid, receipt),
            _ => self.cleanup_unknown(key),
        };
        let provider = provider.and_then(|()| {
            let latest = self.current(key)?;
            if latest.result.is_none() && latest.provider.no_foreground_owner() {
                self.refuse(key, Reason::SetupFailed)
            } else { Ok(()) }
        });
        let slot = retire_slot(self.gate, &key.arguments.id, &key.arguments.native, &key.run_id, host.pid);
        match (provider, slot) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(why), Ok(())) | (Ok(()), Err(why)) => Err(why),
            (Err(provider), Err(slot)) => Err(format!("{provider}; Khor slot cleanup: {slot}")),
        }
    }

    pub(super) fn publish_base<T>(
        &mut self,
        key: &AttemptKey,
        generation: Option<&str>,
        publish: impl FnOnce(&RowGate) -> Result<T, String>,
    ) -> Result<T, String> {
        self.check_live(key, Phase::Starting)?;
        self.base_step(key, generation, publish)
    }
    fn base_step<T>(
        &mut self,
        key: &AttemptKey,
        generation: Option<&str>,
        publish: impl FnOnce(&RowGate) -> Result<T, String>,
    ) -> Result<T, String> {
        if let Some(generation) = generation {
            hex(generation, 32)?;
        }
        // Durable intent fences a crash or callback error after a filesystem
        // side effect. The same token cannot enter this callback a second time.
        self.change(key, |v| {
            v.generation = generation.map(str::to_owned);
            v.phase = Phase::PublishingBase;
            Ok(())
        })?;
        let result = publish(self.gate)?;
        self.change(key, |v| {
            v.phase = Phase::BasePublished;
            Ok(())
        })?;
        Ok(result)
    }
    /// Caller supplies the actual finite-prefix/Ready transaction, with bulk
    /// preparation already finished outside this gate. No spawn or await here.
    pub(super) fn publish_run<T>(
        &mut self,
        key: &AttemptKey,
        publish: impl FnOnce(&RowGate, &str) -> Result<T, String>,
    ) -> Result<T, String> {
        self.check_live(key, Phase::BasePublished)?;
        self.run_step(key, publish)
    }
    fn run_step<T>(
        &mut self,
        key: &AttemptKey,
        publish: impl FnOnce(&RowGate, &str) -> Result<T, String>,
    ) -> Result<T, String> {
        self.change(key, |v| {
            v.phase = Phase::PublishingRun;
            Ok(())
        })?;
        let result = publish(self.gate, &key.run_id)?;
        self.change(key, |v| {
            v.phase = Phase::RunPublished;
            v.result = Some(Recorded::Opened {
                id: v.arguments.id.clone(),
                run_id: v.run_id.clone(),
            });
            Ok(())
        })?;
        Ok(result)
    }
    pub(super) fn confirmed_exit(
        &mut self,
        key: &AttemptKey,
        pid: u32,
        receipt: &khor_acp::ProcessExit,
    ) -> Result<(), String> {
        if pid <= 1 || pid > i32::MAX as u32 {
            return Err("invalid reaped provider PID".into());
        }
        self.change(key, |v| {
            match &v.provider {
                Provider::Owned { process } | Provider::ForegroundAbsent { process } if process.pid == pid => {}
                // Birth capture can fail after a real spawn. The same actual
                // transport can still publish its own confirmed reap receipt.
                Provider::Starting => {}
                Provider::Unknown {
                    last: Some(process),
                    ..
                } if process.pid == pid => {}
                Provider::Unknown {
                    last: None,
                    actual_pid,
                    ..
                } if actual_pid.is_none_or(|actual| actual == pid) => {}
                Provider::Exited { pid: old, code } if *old == pid && *code == receipt.code => {
                    return Ok(());
                }
                _ => return Err("exit receipt is not for this attempt's provider".into()),
            }
            v.provider = Provider::Exited {
                pid,
                code: receipt.code,
            };
            Ok(())
        })
    }
    pub(super) fn cleanup_unknown(&mut self, key: &AttemptKey) -> Result<(), String> {
        self.change(key, |v| {
            let (last, actual_pid) = match &v.provider {
                Provider::Owned { process } => (Some(process.clone()), Some(process.pid)),
                Provider::Unknown {
                    last, actual_pid, ..
                } => (last.clone(), *actual_pid),
                Provider::Starting => (None, None),
                _ => return Err("cleanup uncertainty cannot erase a known clean boundary".into()),
            };
            v.provider = Provider::Unknown {
                last,
                actual_pid,
                reason: UnknownReason::CleanupUnconfirmed,
            };
            Ok(())
        })
    }
    pub(super) fn refuse(&mut self, key: &AttemptKey, reason: Reason) -> Result<(), String> {
        self.change(key, |v| {
            if v.result.is_some() || !v.provider.no_foreground_owner() {
                return Err("refusal has no confirmed clean provider boundary".into());
            }
            v.phase = Phase::Refused;
            v.result = Some(Recorded::Refused { reason });
            Ok(())
        })
    }
    fn check_live(&self, key: &AttemptKey, phase: Phase) -> Result<(), String> {
        let value = self.current(key)?;
        if self.has_cleanup_barrier() {
            return Err("native source has an unconfirmed New cleanup".into());
        }
        if value.arguments.operation != Operation::RecoverNative
            || value.phase != phase
            || !matches!(value.provider, Provider::Owned { .. })
        {
            return Err("recovery publication has no current provider owner".into());
        }
        Ok(())
    }
    fn current(&self, key: &AttemptKey) -> Result<&Latest, String> {
        self.ensure()?;
        let latest = &self
            .document
            .as_ref()
            .ok_or("recovery attempt is missing")?
            .latest.as_ref().ok_or("recovery row has no publishing attempt")?;
        if key.token != latest.token
            || key.arguments != latest.arguments
            || key.run_id != latest.run_id
        {
            return Err("recovery attempt has been retired".into());
        }
        Ok(latest)
    }
    fn change(
        &mut self,
        key: &AttemptKey,
        update: impl FnOnce(&mut Latest) -> Result<(), String>,
    ) -> Result<(), String> {
        self.current(key)?;
        let mut next = self.document.clone().ok_or("recovery attempt is missing")?;
        update(next.latest.as_mut().ok_or("recovery row has no mutable attempt")?)?;
        self.store(next)
    }
    fn store(&mut self, value: Document) -> Result<(), String> {
        self.ensure()?;
        value.validate()?;
        let bytes = serde_json::to_vec(&value).map_err(|e| e.to_string())?;
        if bytes.len() > BYTES {
            return Err("recovery attempt exceeds its metadata budget".into());
        }
        let result = write(self.gate, &bytes);
        if result.is_err() {
            self.failed = true;
        }
        result?;
        self.document = Some(value);
        Ok(())
    }
    fn ensure(&self) -> Result<(), String> {
        if self.failed {
            Err("recovery metadata publication is uncertain".into())
        } else {
            Ok(())
        }
    }
}

/// Read-only projection of a previous observation; no row lock/file creation,
/// provider probe or launch. The action must still re-Initialize and guard.
/// Offer only classifies durable facts; it does not probe processes, acquire
/// coordination files or turn legacy/missing ownership into permission.
pub(super) fn offer_action_allowed(directory: &Path) -> Result<bool, String> {
    let Some(document) = read(directory)? else { return Ok(false); };
    if document.cleanup_barrier.is_some() { return Ok(false); }
    Ok(document.latest.as_ref().is_some_and(|execution| !matches!(execution.provider, Provider::Unknown { .. } | Provider::Starting | Provider::NotStarted)))
}

pub(super) fn matching_support(
    directory: &Path,
    native: &Identity,
    launch: &[u8],
) -> Result<khor_core::recovery::RecoverySupport, String> {
    use khor_core::recovery::RecoverySupport;
    let document = read(directory)?;
    let Some(latest) = document.as_ref().and_then(|doc| doc.latest.as_ref()) else { return Ok(RecoverySupport::Unknown); };
    if &latest.arguments.native != native
        || latest.arguments.launch_digest != blake3::hash(launch).to_hex().to_string()
    {
        return Ok(RecoverySupport::Unknown);
    }
    Ok(latest.support.map_or(RecoverySupport::Unknown, InitializedSupport::projection))
}

pub(super) fn check_unquarantined(directory: &Path) -> Result<(), String> {
    if read(directory)?.as_ref().is_some_and(|doc| doc.cleanup_barrier.is_some()) {
        Err("native source has an unconfirmed New cleanup".into())
    } else {
        Ok(())
    }
}

/// Read-only offer helper: no row lock acquisition, file creation or PID probe.
pub(super) fn next_epoch(directory: &Path) -> Result<u64, String> {
    let value = read(directory)?;
    if value.as_ref().is_some_and(|doc| doc.cleanup_barrier.is_some()) {
        return Err("native source has an unconfirmed New cleanup".into());
    }
    let high = value.map_or(0, |value| value.high_water);
    high.checked_add(1)
        .ok_or_else(|| "recovery attempt epoch is exhausted".into())
}
fn hex(value: &str, bytes: usize) -> Result<(), String> {
    if value.len() == bytes
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        Ok(())
    } else {
        Err("invalid recovery identity encoding".into())
    }
}
fn private_options() -> fs::OpenOptions {
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    }
    options
}
fn regular(file: &fs::File) -> Result<(), String> {
    let meta = file.metadata().map_err(|e| e.to_string())?;
    if !meta.is_file() {
        return Err("recovery metadata is not a regular file".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if meta.nlink() != 1 || meta.uid() != unsafe { libc::geteuid() } {
            return Err("recovery metadata has another owner or link".into());
        }
    }
    Ok(())
}
fn read(directory: &Path) -> Result<Option<Document>, String> {
    let file = match private_options().open(directory.join(NAME)) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    regular(&file)?;
    let mut bytes = Vec::new();
    file.take(BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > BYTES {
        return Err("recovery attempt exceeds its metadata budget".into());
    }
    let value: Document = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    value.validate()?;
    Ok(Some(value))
}
fn write(gate: &RowGate, bytes: &[u8]) -> Result<(), String> {
    // One fixed, bounded atomic staging file, not an append-only token log.
    let part = gate.dir.join("recovery-attempt.part");
    let mut file = private_options()
        .write(true)
        .create(true)
        .truncate(false)
        .open(&part)
        .map_err(|e| e.to_string())?;
    regular(&file)?;
    file.set_len(0).map_err(|e| e.to_string())?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())?;
    drop(file);
    fs::rename(&part, gate.dir.join(NAME)).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    fs::File::open(&gate.dir)
        .and_then(|file| file.sync_all())
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::live::{GuiRun, LiveState, Source};
    use khor_core::State;
    struct Scratch(std::path::PathBuf);
    impl Scratch {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "khor-attempt-{}",
                crate::link::fresh_hex().unwrap()
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn stamp(pid: u32, parent: u32) -> ProcessStamp {
        // Injected bookkeeping inputs, not an assertion about actual OS PIDs.
        ProcessStamp {
            pid,
            parent,
            group: pid,
            boot: "1".repeat(32),
            start: ProcessStart::Mac {
                seconds: 123,
                microseconds: pid,
            },
        }
    }
    fn token(epoch: u64, byte: char) -> Token {
        Token::new(epoch, byte.to_string().repeat(32)).unwrap()
    }
    fn start(value: Reservation) -> AttemptKey {
        match value {
            Reservation::Start(key) => key,
            _ => panic!("expected one new attempt"),
        }
    }
    #[test]
    fn bounded_attempt_tokens_keep_arguments_results_and_unknown_owners_fenced() {
        let supplied = khor_core::recovery::RecoveryToken { epoch: "1".into(), nonce: "a".repeat(32) };
        assert_eq!(super::super::session_recovery::action_token(&supplied).unwrap().epoch, 1);
        for epoch in ["", "0", "01", "+1", "-1", "1.0", "18446744073709551616"] {
            assert!(super::super::session_recovery::action_token(&khor_core::recovery::RecoveryToken {
                epoch: epoch.into(), nonce: "a".repeat(32),
            }).is_err());
        }
        assert!(super::super::session_recovery::action_token(&khor_core::recovery::RecoveryToken {
            epoch: "1".into(), nonce: "A".repeat(32),
        }).is_err());
        let scratch = Scratch::new();
        assert_eq!(next_epoch(&scratch.0).unwrap(), 1);
        assert_eq!(
            fs::read_dir(&scratch.0).unwrap().count(),
            0,
            "read-only offer makes no coordination files"
        );
        let mut gate = RowGate::acquire(&scratch.0).unwrap();
        let native = Identity {
            home: DeviceId([7; 32]),
            agent: "fixture".into(),
            native_id: "full/native-id".into(),
        };
        let args =
            Arguments::native(native.clone(), None, b"effective-launch-and-context").unwrap();
        let one = token(1, 'a');
        let mut held = Held::load(&mut gate).unwrap();
        let first = start(held.reserve(args.clone(), one.clone(), None).unwrap());
        held.check_claim(&first, &args).unwrap();
        assert!(held.check_claim(&first, &Arguments::native(native.clone(), None, b"wrong context").unwrap()).is_err());
        let before = fs::read(held.gate.dir.join(NAME)).unwrap();
        assert!(before.len() <= BYTES);
        assert!(matches!(
            held.reserve(args.clone(), one.clone(), None).unwrap(),
            Reservation::Unfinished {
                phase: Phase::Reserved,
                host: None
            }
        ));
        assert!(matches!(
            held.reserve(args.clone(), token(2, 'b'), None).unwrap(),
            Reservation::UnknownOwner
        ));
        let changed =
            Arguments::native(native.clone(), None, b"different-effective-launch").unwrap();
        assert!(held.reserve(changed, one.clone(), None).is_err());
        let elsewhere = Arguments::native(
            Identity {
                home: DeviceId([8; 32]),
                ..native.clone()
            },
            None,
            b"effective-launch-and-context",
        )
        .unwrap();
        assert!(held.reserve(elsewhere, token(2, 'b'), None).is_err());
        assert!(held.reserve(args.clone(), token(1, 'b'), None).is_err());
        assert!(held.reserve(args.clone(), token(3, 'b'), None).is_err());
        assert!(matches!(held.check_native_input(&args, &token(1, 'b')).unwrap(), InputDecision::Refused(_)));
        assert!(matches!(held.check_native_input(&args, &token(3, 'b')).unwrap(), InputDecision::Refused(_)));
        assert_eq!(fs::read(held.gate.dir.join(NAME)).unwrap(), before);

        held.claim_host(&first, stamp(10, 1)).unwrap();
        assert!(held.check_claim(&first, &args).is_err(), "a second ghost cannot reuse a claimed token");
        assert!(held.claim_host(&first, stamp(11, 1)).is_err());
        held.starting_provider(&first).unwrap();
        assert!(
            held.starting_provider(&first).is_err(),
            "a replayed claim cannot issue another spawn"
        );
        assert!(held.owned_provider(&first, stamp(20, 99)).is_err());
        let advertised = khor_acp::RecoveryCapabilities { load: true, resume: false };
        assert!(held.record_support(&first, advertised).is_err());
        held.owned_provider(&first, stamp(20, 10)).unwrap();
        held.record_support(&first, advertised).unwrap();
        held.record_support(&first, advertised).unwrap();
        let supported = fs::read(held.gate.dir.join(NAME)).unwrap();
        assert!(held.record_support(&first, khor_acp::RecoveryCapabilities { load: false, resume: true }).is_err());
        assert_eq!(supported, fs::read(held.gate.dir.join(NAME)).unwrap());
        assert_eq!(matching_support(&held.gate.dir, &native, b"effective-launch-and-context").unwrap(), khor_core::recovery::RecoverySupport::Load);
        assert_eq!(matching_support(&held.gate.dir, &native, b"changed AgentSpec launch").unwrap(), khor_core::recovery::RecoverySupport::Unknown);
        let mut another_agent = native.clone(); another_agent.agent = "another-agent".into();
        assert_eq!(matching_support(&held.gate.dir, &another_agent, b"effective-launch-and-context").unwrap(), khor_core::recovery::RecoverySupport::Unknown);
        held.publish_base(&first, Some(&"c".repeat(32)), |_| Ok(()))
            .unwrap();
        held.publish_run(&first, |gate, run_id| {
            // This is a phase/row fixture, not an ACP Ready or Journal test.
            gate.write_meta(&crate::live::Meta {
                shell_run: None,
                kind: khor_core::kind::GUI.into(),
                title: "fixture".into(),
                pid: Some(10),
                started_ms: 1,
                gui_native: Some(native.clone()),
                category: None,
                id: args.id.0.clone(),
                vendor_leaf: None,
                place: None,
                gui_run: Some(GuiRun {
                    id: run_id.into(),
                    started_ms: 1,
                    state: LiveState {
                        gui_run: Some(run_id.into()),
                        word: State::Idle,
                        at_ms: 1,
                        exit: None,
                        source: Some(Source::Reported),
                    },
                }),
            })
        })
        .unwrap();
        let result = Recorded::Opened {
            id: args.id.clone(),
            run_id: first.run_id.clone(),
        };
        assert!(
            matches!(held.reserve(args.clone(), one.clone(), None).unwrap(), Reservation::Recorded(value) if value == result)
        );

        held.cleanup_unknown(&first).unwrap();
        let mut meta = held.gate.read_meta().unwrap();
        meta.gui_run.as_mut().unwrap().state.exit = Some(1);
        held.gate.write_meta(&meta).unwrap();
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(held.gate.dir.join("writer.lock"))
            .unwrap();
        file.try_lock().unwrap();
        drop(file);
        let args_two = Arguments::native(
            native.clone(),
            Some(first.run_id.clone()),
            b"explicit-new-retry",
        )
        .unwrap();
        let before = fs::read(held.gate.dir.join(NAME)).unwrap();
        assert!(
            matches!(
                held.reserve(args_two.clone(), token(2, 'b'), None).unwrap(),
                Reservation::UnknownOwner
            ),
            "an exit-present Meta and free journal do not prove provider cleanup"
        );
        assert!(held.refuse(&first, Reason::SetupFailed).is_err());
        assert_eq!(fs::read(held.gate.dir.join(NAME)).unwrap(), before);
        // A stored earlier success is historical idempotency, not another
        // start and not a claim that this execution remains live now.
        assert!(
            matches!(held.reserve(args.clone(), one.clone(), None).unwrap(), Reservation::Recorded(value) if value == result)
        );

        let receipt = khor_acp::ProcessExit { code: Some(0) };
        assert!(held.confirmed_exit(&first, 21, &receipt).is_err());
        held.confirmed_exit(&first, 20, &receipt).unwrap();
        let second = start(held.reserve(args_two.clone(), token(2, 'b'), None).unwrap());
        assert!(matches!(
            held.reserve(args, one, None).unwrap(),
            Reservation::Retired
        ));
        let before = fs::read(held.gate.dir.join(NAME)).unwrap();
        let mut publications = 0;
        assert!(
            held.publish_base(&first, None, |_| {
                publications += 1;
                Ok(())
            })
            .is_err()
        );
        assert!(
            held.publish_run(&first, |_, _| {
                publications += 1;
                Ok(())
            })
            .is_err()
        );
        assert_eq!(
            publications, 0,
            "a retired ghost cannot enter either publication callback"
        );
        assert_eq!(fs::read(held.gate.dir.join(NAME)).unwrap(), before);
        held.refuse(&second, Reason::Cancelled).unwrap();
        assert!(held.claim_host(&second, stamp(30, 1)).is_err());
        assert!(matches!(
            held.reserve(args_two.clone(), token(2, 'b'), None).unwrap(),
            Reservation::Recorded(Recorded::Refused {
                reason: Reason::Cancelled
            })
        ));

        // The document contains exactly the latest result, not an unbounded
        // history of tokens. Exhaustion fails rather than wrapping to epoch 1.
        let mut exhausted = held.document.clone().unwrap();
        exhausted.high_water = u64::MAX;
        exhausted.latest.as_mut().unwrap().token.epoch = u64::MAX;
        held.store(exhausted).unwrap();
        assert!(next_epoch(&scratch.0).is_err());
        assert!(fs::metadata(held.gate.dir.join(NAME)).unwrap().len() <= BYTES as u64);
        let saved = fs::read(held.gate.dir.join(NAME)).unwrap();
        fs::write(held.gate.dir.join(NAME), vec![b'x'; BYTES + 1]).unwrap();
        assert!(next_epoch(&scratch.0).is_err());
        fs::write(held.gate.dir.join(NAME), saved).unwrap();
        drop(held);
        assert!(Held::load(&mut gate).is_ok());
        drop(gate);

        // Publication failure/response loss stays an uncertain accepted
        // prefix. Intent is already durable when a side effect is attempted.
        let interrupted = Scratch::new();
        let mut gate = RowGate::acquire(&interrupted.0).unwrap();
        let mut held = Held::load(&mut gate).unwrap();
        let args = Arguments::native(native, None, b"interrupted-publication").unwrap();
        let key = start(held.reserve(args.clone(), token(1, 'a'), None).unwrap());
        held.claim_host(&key, stamp(10, 1)).unwrap();
        held.starting_provider(&key).unwrap();
        held.owned_provider(&key, stamp(20, 10)).unwrap();
        let mut writes = 0;
        let failed: Result<(), String> = held.publish_base(&key, None, |gate| {
            assert_eq!(
                read(&gate.dir).unwrap().unwrap().latest.unwrap().phase,
                Phase::PublishingBase
            );
            writes += 1;
            Err("failure after a base filesystem side effect".into())
        });
        assert!(failed.is_err());
        assert!(
            held.publish_base(&key, None, |_| {
                writes += 1;
                Ok(())
            })
            .is_err()
        );
        assert_eq!(writes, 1);
        assert!(matches!(
            held.reserve(args.clone(), token(1, 'a'), None).unwrap(),
            Reservation::Unfinished {
                phase: Phase::PublishingBase,
                ..
            }
        ));
        assert!(matches!(
            held.reserve(args.clone(), token(2, 'b'), None).unwrap(),
            Reservation::UnknownOwner
        ));
        held.confirmed_exit(&key, 20, &receipt).unwrap();
        let next = start(held.reserve(args.clone(), token(2, 'b'), None).unwrap());
        let fresh = fs::read(held.gate.dir.join(NAME)).unwrap();
        assert!(held.record_support(&key, khor_acp::RecoveryCapabilities { load: true, resume: true }).is_err());
        assert_eq!(fresh, fs::read(held.gate.dir.join(NAME)).unwrap());
        held.claim_host(&next, stamp(30, 1)).unwrap();
        held.starting_provider(&next).unwrap();
        held.owned_provider(&next, stamp(40, 30)).unwrap();
        held.publish_base(&next, None, |_| Ok(())).unwrap();
        let failed: Result<(), String> = held.publish_run(&next, |gate, _| {
            assert_eq!(
                read(&gate.dir).unwrap().unwrap().latest.unwrap().phase,
                Phase::PublishingRun
            );
            writes += 1;
            Err("failure after a run or Ready filesystem side effect".into())
        });
        assert!(failed.is_err());
        assert!(
            held.publish_run(&next, |_, _| {
                writes += 1;
                Ok(())
            })
            .is_err()
        );
        assert_eq!(writes, 2);
        assert!(matches!(
            held.reserve(args, token(2, 'b'), None).unwrap(),
            Reservation::Unfinished {
                phase: Phase::PublishingRun,
                ..
            }
        ));
        drop(held);
        drop(gate);
        recovery_wire_admission();
        initial_new_cases();
        partial_new_settlement();
        joins_do_not_retire_execution();
        shell_action_is_one_successor();
    }

    fn recovery_wire_admission() {
        use crate::proto::Request;
        let home = DeviceId([0xab; 32]); let id = SessionId("tui/native".into());
        assert_eq!(crate::Node::recovery_home(&home.hex()).unwrap(), home);
        assert!(crate::Node::recovery_home(&home.hex().to_uppercase()).is_err());
        let token = khor_core::recovery::RecoveryToken { epoch: "1".into(), nonce: "a".repeat(32) };
        for request in [
            Request::RecoverSession { home, id: id.clone(), expected_run: Some("c".repeat(32)), token: token.clone() },
            Request::NewShellRun { home, id: SessionId("shell/old".into()), expected_run: Some("shell/old".into()), token, cwd: Some("/explicit/new-directory".into()) },
        ] {
            assert!(!request.is_reading() && !request.rides_early(), "explicit actions are not read retries or 0RTT");
            let encoded = crate::proto::encode_request(&request).unwrap();
            let decoded = crate::proto::question(&encoded).unwrap();
            assert_eq!(serde_json::to_value(request).unwrap(), serde_json::to_value(decoded).unwrap());
        }
        assert!(Request::SessionRecovery { home, id }.is_reading());
    }

    fn joins_do_not_retire_execution() {
        let scratch = Scratch::new();
        let native = Identity { home: DeviceId([7; 32]), agent: "fixture".into(), native_id: "join-writer".into() };
        let original = Arguments::native(native.clone(), None, b"same-config").unwrap();
        let mut gate = RowGate::acquire(&scratch.0).unwrap();
        let mut held = Held::load(&mut gate).unwrap();
        let writer = start(held.reserve(original.clone(), token(1, 'a'), None).unwrap());
        held.claim_host(&writer, stamp(10, 1)).unwrap();
        held.starting_provider(&writer).unwrap();
        held.owned_provider(&writer, stamp(20, 10)).unwrap();
        held.publish_base(&writer, None, |_| Ok(())).unwrap();
        held.publish_run(&writer, |gate, run_id| gate.write_meta(&crate::live::Meta {
            shell_run: None,
            kind: khor_core::kind::GUI.into(), title: "same producer".into(), pid: Some(10), started_ms: 1,
            gui_native: Some(native.clone()), category: None, id: original.id.0.clone(), vendor_leaf: None, place: None,
            gui_run: Some(GuiRun { id: run_id.into(), started_ms: 1, state: LiveState {
                gui_run: Some(run_id.into()), word: State::Idle, at_ms: 1, exit: None, source: Some(Source::Reported),
            }}),
        })).unwrap();
        let join = join_arguments(native, Some(writer.run_id.clone())).unwrap();
        let ticket = match held.begin_join(join.clone(), token(2, 'b'), None).unwrap() { JoinAdmission::Check(ticket) => ticket, _ => panic!("expected one checking ticket") };
        assert!(matches!(held.begin_join(join.clone(), token(2, 'b'), None).unwrap(), JoinAdmission::Previous(Reservation::UnknownOwner)), "pending duplicate cannot handshake or recover");
        let fresh_ticket = match held.begin_join(join.clone(), token(3, 'c'), None).unwrap() { JoinAdmission::Check(ticket) => ticket, _ => panic!("expected explicit new Join intent") };
        let mut checks = 0;
        assert!(held.finish_join(ticket, None, |_| { checks += 1; Ok(()) }).is_err());
        assert_eq!(checks, 0, "a late checking ticket cannot cross a newer action highwater");
        assert!(matches!(held.finish_join(fresh_ticket, None, |_| { checks += 1; Ok(()) }).unwrap(), Reservation::Joined { .. }));
        let after_join = fs::read(held.gate.dir.join(NAME)).unwrap();
        assert!(held.current(&writer).is_ok(), "a Join does not replace producer authority");
        held.record_support(&writer, khor_acp::RecoveryCapabilities { load: true, resume: false }).unwrap();
        held.confirmed_exit(&writer, 20, &khor_acp::ProcessExit { code: Some(0) }).unwrap();
        let mut ended = held.gate.read_meta_or_new().unwrap().unwrap();
        ended.gui_run.as_mut().unwrap().state.exit = Some(0);
        held.gate.write_meta(&ended).unwrap();
        assert!(matches!(held.join(join.clone(), token(3, 'c'), None, |_| { checks += 1; Err("already ended".into()) }).unwrap(), Reservation::Joined { .. }));
        assert_eq!(checks, 1, "duplicate success returns its accepted Join without another live/spawn decision");
        let mut conflicting = join.clone(); conflicting.launch_digest = "f".repeat(64);
        assert!(held.join(conflicting, token(3, 'c'), None, |_| Ok(())).is_err());
        assert_ne!(fs::read(held.gate.dir.join(NAME)).unwrap(), after_join, "the same producer still records actual exit after Join");
        assert!(matches!(held.reserve(join, token(3, 'c'), None).unwrap(), Reservation::Joined { .. }));
        assert!(matches!(held.reserve(original, token(1, 'a'), None).unwrap(), Reservation::Retired));
        assert_eq!(next_epoch(&scratch.0).unwrap(), 4);
        assert!(fs::metadata(held.gate.dir.join(NAME)).unwrap().len() <= BYTES as u64);
    }
    fn shell_action_is_one_successor() {
        let scratch = Scratch::new();
        let mut gate = RowGate::acquire(&scratch.0).unwrap();
        let mut held = Held::load(&mut gate).unwrap();
        let previous = SessionId("shell/previous".into());
        let args = ActionArguments::shell(DeviceId([7; 32]), previous.clone(), previous.0.clone(), b"cwd/defaultshell/emptycommand").unwrap();
        let one = token(1, 'a'); let mut admissions = 0;
        assert!(held.reserve_shell(args.clone(), one.clone(), |_| { admissions += 1; Ok(()) }).unwrap().is_none());
        assert!(matches!(held.reserve_shell(args.clone(), one.clone(), |_| { admissions += 1; Ok(()) }).unwrap(), Some(ActionReplay::Recorded(ActionResult::ShellPending))));
        assert_eq!(admissions, 1, "response loss cannot invoke existingOnce with a second successor");
        assert!(held.reserve_shell(args.clone(), token(2, 'b'), |_| { admissions += 1; Ok(()) }).is_err());
        assert_eq!(admissions, 1, "a fresh token cannot supersede uncertain Shell effects");
        let second = SessionId("shell/fresh-B".into());
        held.finish_shell(&args, &one, ActionResult::ShellOpened { id: second.clone() }).unwrap();
        assert!(matches!(held.action_replay(&args, &one).unwrap(), Some(ActionReplay::Recorded(ActionResult::ShellOpened { id })) if id == second));
        let changed = ActionArguments::shell(DeviceId([7; 32]), previous.clone(), previous.0, b"different-cwd").unwrap();
        assert!(held.action_replay(&changed, &one).is_err());
        let before_refusal = fs::read(held.gate.dir.join(NAME)).unwrap();
        assert!(matches!(held.check_shell_input(&changed, &one).unwrap(), InputDecision::Refused(_)));
        assert_eq!(before_refusal, fs::read(held.gate.dir.join(NAME)).unwrap(), "typed input refusal made no durable transition");
        assert!(held.finish_shell(&args, &one, ActionResult::ShellOpened { id: SessionId("shell/fresh-C".into()) }).is_err());
        assert!(fs::metadata(held.gate.dir.join(NAME)).unwrap().len() <= BYTES as u64);
        let chosen = ActionArguments::shell(DeviceId([7; 32]), SessionId("shell/previous".into()), "shell/previous".into(), b"explicit-new-directory").unwrap();
        let next = token(2, 'b');
        assert!(held.reserve_shell(chosen.clone(), next.clone(), |_| Ok(())).unwrap().is_none());
        held.finish_shell(&chosen, &next, ActionResult::DirectoryRequired).unwrap();
        assert!(matches!(held.action_replay(&chosen, &next).unwrap(), Some(ActionReplay::Recorded(ActionResult::DirectoryRequired))));
        assert!(matches!(held.action_replay(&args, &one).unwrap(), Some(ActionReplay::Retired)));
    }

    fn partial_new_settlement() {
        for blocked in [false, true] {
            partial_new_settlement_with(blocked);
        }
    }
    fn partial_new_settlement_with(blocked: bool) {
        use crate::session_content::journal::Opening;
        let root = Scratch::new();
        let row = root.0.join("row");
        fs::create_dir(&row).unwrap();
        let native = Identity {
            home: DeviceId([7; 32]), agent: "fixture".into(), native_id: "partial-new".into(),
        };
        let reservation = JournalReservation::take(&root.0, native.clone()).unwrap();
        let control = Arc::new(Control::default());
        let authority = {
            let mut gate = RowGate::acquire(&row).unwrap();
            Held::load(&mut gate).unwrap().seed_initial_new(
                Arguments::initial_new(native.clone(), b"actual-args-model").unwrap(),
                NewInvocation { reservation: &reservation, control: &control,
                    actual_pid: Some(20), host: None, provider: None, support: None },
            ).unwrap()
        };
        let mut journal = reservation.into_journal(Opening::New).unwrap();
        journal.note(&serde_json::json!({"text":"complete retained base"})).unwrap();
        journal.prepare_base().unwrap();
        let mut gate = RowGate::acquire(&row).unwrap();
        let mut held = Held::load(&mut gate).unwrap();
        held.publish_initial_base(&authority, &mut journal, &control, |gate, journal| {
            journal.publish_base()?;
            journal.bind(&gate.dir)
        }).unwrap();
        let original = fs::read(native.directory(&root.0).join("current/records.jsonl")).unwrap();
        let failed: Result<(), String> = held.publish_initial_run(
            &authority, &mut journal, &control, |gate, _, run_id| {
                gate.write_meta(&crate::live::Meta {
                    shell_run: None,
                    kind: khor_core::kind::GUI.into(), title: "partial publication".into(),
                    pid: Some(std::process::id()), started_ms: 1, gui_native: Some(native.clone()),
                    gui_run: Some(GuiRun { id: run_id.into(), started_ms: 1, state: LiveState {
                        gui_run: Some(run_id.into()), word: State::Idle, at_ms: 1,
                        exit: None, source: Some(Source::Reported),
                    }}), category: None, id: crate::adaptor::id_for(&native.native_id).0,
                    vendor_leaf: None, place: None,
                })?;
                Err("failure after Meta before Ready/result publication".into())
            },
        );
        assert!(failed.is_err());
        assert!(held.ready_run(&native, std::process::id(), None).is_err());
        if blocked {
            fs::create_dir(row.join("recovery-attempt.part")).unwrap();
        }
        let settled = held.settle_initial(&authority, &control, Err(()), |gate, id, native, run, pid| {
            crate::live::retire_gui_slot_held(gate, id, native, run, pid, 1)
        });
        assert_eq!(settled.is_err(), blocked);
        let meta = held.gate.read_meta().unwrap();
        assert_eq!(meta.gui_run.as_ref().unwrap().state.exit, Some(1));
        let doc = read(&row).unwrap().unwrap();
        let latest = doc.latest.unwrap();
        assert_eq!(latest.phase, Phase::PublishingRun);
        assert!(latest.result.is_none());
        assert!(matches!(latest.provider, Provider::Unknown { reason, .. }
            if reason == if blocked { UnknownReason::BirthUnavailable } else { UnknownReason::CleanupUnconfirmed }));
        assert_eq!(original, fs::read(native.directory(&root.0).join("current/records.jsonl")).unwrap());
        assert!(held.ready_run(&native, std::process::id(), None).is_err());
        drop(held);
        drop(gate);
        if blocked {
            fs::remove_dir(row.join("recovery-attempt.part")).unwrap();
        }
        let mut gate = RowGate::acquire(&row).unwrap();
        let mut held = Held::load(&mut gate).unwrap();
        let mut newer = meta;
        let run = newer.gui_run.as_mut().unwrap();
        run.id = "d".repeat(32);
        run.state.gui_run = Some(run.id.clone());
        run.state.exit = None;
        held.gate.write_meta(&newer).unwrap();
        let before = fs::read(row.join("meta.json")).unwrap();
        held.settle_initial(&authority, &control, Err(()), |gate, id, native, run, pid| {
            crate::live::retire_gui_slot_held(gate, id, native, run, pid, 1)
        }).unwrap();
        assert_eq!(before, fs::read(row.join("meta.json")).unwrap(), "old attempt cannot retire a newer Khor slot");
    }

    fn initial_new_cases() {
        use crate::session_content::journal::Opening;
        let native = Identity {
            home: DeviceId([7; 32]), agent: "fixture".into(), native_id: "barrier-only".into(),
        };
        let empty = Document { version: VERSION, high_water: 0, latest: None, action: None, cleanup_barrier: None };
        assert!(empty.validate().is_err());
        let mut barrier_only = Document { cleanup_barrier: Some(native), ..empty };
        barrier_only.validate().unwrap();
        barrier_only.high_water = 1;
        assert!(barrier_only.validate().is_err());
        {
            let root = Scratch::new();
            let row = root.0.join("row");
            fs::create_dir(&row).unwrap();
            let native = Identity {
                home: DeviceId([7; 32]), agent: "fixture".into(), native_id: "bound-before-meta".into(),
            };
            fs::write(row.join("journal.json"), serde_json::to_vec(&native).unwrap()).unwrap();
            let reservation = JournalReservation::take(&root.0, native.clone()).unwrap();
            let mut gate = RowGate::acquire(&row).unwrap();
            let mut held = Held::load(&mut gate).unwrap();
            let mut conflict = native.clone();
            conflict.agent = "another-agent".into();
            let mut meta = crate::live::Meta {
                shell_run: None,
                kind: khor_core::kind::GUI.into(), title: "collision fixture".into(),
                pid: None, started_ms: 1, gui_native: Some(conflict), gui_run: None,
                category: None, id: crate::adaptor::id_for(&native.native_id).0,
                vendor_leaf: None, place: None,
            };
            held.gate.write_meta(&meta).unwrap();
            assert!(held.quarantine_new(&reservation).is_err());
            assert!(!row.join(NAME).exists(), "matching binding cannot override conflicting Meta source");
            meta.gui_native = Some(native.clone());
            meta.id = "tui/another-row".into();
            held.gate.write_meta(&meta).unwrap();
            assert!(held.quarantine_new(&reservation).is_err());
            assert!(!row.join(NAME).exists(), "matching native cannot override conflicting logical row");
            fs::remove_file(row.join("meta.json")).unwrap();
            held.quarantine_new(&reservation).unwrap();
            let document = read(&row).unwrap().unwrap();
            assert_eq!(document.high_water, 0);
            assert!(document.latest.is_none());
            assert_eq!(document.cleanup_barrier, Some(native.clone()));
            assert!(matches!(held.reserve(
                Arguments::native(native, None, b"explicit retry").unwrap(), token(1, 'a'), None,
            ).unwrap(), Reservation::UnknownOwner));
        }
        for revoke in [false, true] {
            let root = Scratch::new();
            let row = root.0.join("row");
            fs::create_dir(&row).unwrap();
            let native = Identity {
                home: DeviceId([7; 32]),
                agent: "fixture".into(),
                native_id: "first-new-id".into(),
            };
            // Actual journal locks, synthetic controller/birth inputs. This is
            // not evidence that a provider or SDK New was exercised.
            let reservation = JournalReservation::take(&root.0, native.clone()).unwrap();
            let mut other_native = native.clone();
            other_native.native_id = "another-id".into();
            let mut other = Journal::open(&root.0, other_native, Opening::New).unwrap();
            let control = Arc::new(Control::default());
            let wrong_control = Arc::new(Control::default());
            let args = Arguments::initial_new(native.clone(), b"effective-new-context").unwrap();
            let actual_pid = if revoke { None } else { Some(20) };
            let authority = {
                let mut gate = RowGate::acquire(&row).unwrap();
                let mut held = Held::load(&mut gate).unwrap();
                held.seed_initial_new(
                    args.clone(),
                    NewInvocation {
                        reservation: &reservation,
                        control: &control,
                        actual_pid,
                        host: None,
                        provider: None,
                        support: None,
                    },
                )
                .unwrap()
            };
            // Journal initialization/large preparation stays outside RowGate.
            let mut journal = reservation.into_journal(Opening::New).unwrap();
            assert!(journal.owns_lease(&authority.journal));
            journal.bind(&row).unwrap();
            journal
                .note(&serde_json::json!({"text":"accepted new prefix"}))
                .unwrap();
            journal.prepare_base().unwrap();
            {
                let mut gate = RowGate::acquire(&row).unwrap();
                let mut held = Held::load(&mut gate).unwrap();
                let decoded: AttemptKey =
                    serde_json::from_slice(&serde_json::to_vec(&authority.key).unwrap()).unwrap();
                let mut writes = 0;
                assert!(
                    held.publish_base(&decoded, None, |_| {
                        writes += 1;
                        Ok(())
                    })
                    .is_err(),
                    "a data key cannot recreate initial New authority"
                );
                assert!(
                    held.publish_initial_base(&authority, &mut other, &control, |_, _| {
                        writes += 1;
                        Ok(())
                    })
                    .is_err()
                );
                assert!(
                    held.publish_initial_base(&authority, &mut journal, &wrong_control, |_, _| {
                        writes += 1;
                        Ok(())
                    })
                    .is_err()
                );
                assert_eq!(writes, 0);
                held.publish_initial_base(&authority, &mut journal, &control, |_, journal| {
                    journal.publish_base()
                })
                .unwrap();
                assert!(held.ready_run(&native, std::process::id(), None).is_err());
                if revoke {
                    assert!(held.settle_initial(&authority, &wrong_control, Err(()), |_, _, _, _, _| Ok(())).is_err());
                    held.settle_initial(&authority, &control, Err(()), |_, _, _, _, _| Ok(())).unwrap();
                    assert!(
                        held.publish_initial_run(&authority, &mut journal, &control, |_, _, _| {
                            writes += 1;
                            Ok(())
                        })
                        .is_err()
                    );
                    assert_eq!(
                        writes, 0,
                        "cleanupUnknown revokes the still-in-memory authority"
                    );
                } else {
                    held.publish_initial_run(
                        &authority,
                        &mut journal,
                        &control,
                        |gate, _, run_id| {
                            gate.write_meta(&crate::live::Meta {
                                shell_run: None,
                                kind: khor_core::kind::GUI.into(),
                                title: "model new".into(),
                                pid: Some(std::process::id()),
                                started_ms: 1,
                                gui_native: Some(native.clone()),
                                category: None,
                                id: args.id.0.clone(),
                                vendor_leaf: None,
                                place: None,
                                gui_run: Some(GuiRun {
                                    id: run_id.into(),
                                    started_ms: 1,
                                    state: LiveState {
                                        gui_run: Some(run_id.into()),
                                        word: State::Idle,
                                        at_ms: 1,
                                        exit: Some(1),
                                        source: Some(Source::Reported),
                                    },
                                }),
                            })
                        },
                    )
                    .unwrap();
                    assert_eq!(held.ready_run(&native, std::process::id(), None).unwrap(), authority.key.run_id);
                    assert!(held.ready_run(&native, std::process::id() + 1, None).is_err());
                    let recovery = Arguments::native(
                        native.clone(),
                        Some(authority.key.run_id.clone()),
                        b"explicit-recovery",
                    )
                    .unwrap();
                    assert!(
                        matches!(
                            held.reserve(recovery.clone(), token(2, 'b'), None).unwrap(),
                            Reservation::UnknownOwner
                        ),
                        "new works now but its missing birth does not become future recovery proof"
                    );
                    held.settle_initial(
                        &authority,
                        &control,
                        Ok(&khor_acp::ProcessExit { code: Some(0) }),
                        |_, _, _, _, _| Ok(()),
                    )
                    .unwrap();
                    let next = start(held.reserve(recovery, token(2, 'b'), None).unwrap());
                    assert_eq!(
                        next.token.epoch, 2,
                        "InitialNew high-water continues into explicit recovery"
                    );
                    assert!(
                        held.publish_initial_run(&authority, &mut journal, &control, |_, _, _| {
                            writes += 1;
                            Ok(())
                        })
                        .is_err()
                    );
                    assert_eq!(writes, 0);
                }
            }
            let before = fs::read(native.directory(&root.0).join("current/records.jsonl")).unwrap();
            drop(journal);
            let pending = native.directory(&root.0).join("pending");
            fs::create_dir(&pending).unwrap();
            fs::write(pending.join("old-part"), b"do not erase on New collision").unwrap();
            let mut existing = JournalReservation::take(&root.0, native.clone()).unwrap();
            {
                let mut gate = RowGate::acquire(&row).unwrap();
                let mut held = Held::load(&mut gate).unwrap();
                let bytes = fs::read(row.join(NAME)).unwrap();
                assert!(
                    held.seed_initial_new(
                        args,
                        NewInvocation {
                            reservation: &existing,
                            control: &control,
                            actual_pid,
                            host: None,
                            provider: None,
                            support: None,
                        }
                    )
                    .is_err()
                );
                assert_eq!(
                    fs::read(row.join(NAME)).unwrap(),
                    bytes,
                    "New cannot reset a prior/pending high-water"
                );
            }
            assert!(existing.open_journal(Opening::New).is_err());
            assert!(existing.holds_file());
            assert!(JournalReservation::take(&root.0, native.clone()).is_err());
            {
                let mut gate = RowGate::acquire(&row).unwrap();
                let before = read(&row).unwrap().unwrap();
                let old_latest = serde_json::to_vec(&before.latest).unwrap();
                let mut held = Held::load(&mut gate).unwrap();
                held.quarantine_new(&existing).unwrap();
                let after = read(&row).unwrap().unwrap();
                assert_eq!(before.high_water, after.high_water);
                assert_eq!(old_latest, serde_json::to_vec(&after.latest).unwrap());
                assert_eq!(after.cleanup_barrier, Some(native.clone()));
                assert!(matches!(held.reserve(
                    Arguments::native(native.clone(), None, b"barrier retry").unwrap(),
                    token(after.high_water + 1, 'e'), None,
                ).unwrap(), Reservation::UnknownOwner));
                assert!(next_epoch(&row).is_err());
            }
            drop(existing);
            assert_eq!(
                fs::read(native.directory(&root.0).join("current/records.jsonl")).unwrap(),
                before
            );
            assert_eq!(
                fs::read(pending.join("old-part")).unwrap(),
                b"do not erase on New collision"
            );
            let reopened = Journal::open(&root.0, native, Opening::Resume).unwrap();
            assert!(
                !reopened.owns_lease(&authority.journal),
                "the same native ID does not recreate the old Journal lease"
            );
        }
    }
}
