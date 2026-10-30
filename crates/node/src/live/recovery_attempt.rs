//! One bounded durable attempt per existing row. No spawn, OS-process probe,
//! independent lock or background registry lives here. Every mutation borrows
//! the already-held row gate; the runtime adapter must supply actual facts.
use super::RowGate;
use crate::{SessionId, session_content::journal::Identity};
use khor_core::DeviceId;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    path::Path,
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
}
impl Arguments {
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
    Owned { process: ProcessStamp },
    Exited { pid: u32, code: Option<i32> },
    Unknown { last: Option<ProcessStamp> },
}
impl Provider {
    fn known_clean(&self) -> bool {
        matches!(self, Self::NotStarted | Self::Exited { .. })
    }
    fn validate(&self) -> Result<(), String> {
        match self {
            Self::Owned { process } => process.validate(),
            Self::Unknown {
                last: Some(process),
            } => process.validate(),
            Self::Exited { pid, .. } if *pid == 0 || *pid > i32::MAX as u32 => {
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

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Latest {
    token: Token,
    arguments: Arguments,
    run_id: String,
    phase: Phase,
    host: Option<ProcessStamp>,
    provider: Provider,
    generation: Option<String>,
    result: Option<Recorded>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    version: u8,
    high_water: u64,
    latest: Latest,
}
impl Document {
    fn validate(&self) -> Result<(), String> {
        let v = &self.latest;
        if self.version != VERSION || self.high_water != v.token.epoch {
            return Err("invalid recovery high-water record".into());
        }
        v.token.validate()?;
        v.arguments.validate()?;
        hex(&v.run_id, 32)?;
        if let Some(host) = &v.host {
            host.validate()?;
        }
        v.provider.validate()?;
        if let Provider::Owned { process }
        | Provider::Unknown {
            last: Some(process),
        } = &v.provider
        {
            if process.group != process.pid
                || v.host
                    .as_ref()
                    .is_none_or(|host| process.parent != host.pid || process.boot != host.boot)
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
                    && v.host.is_some()
                    && !matches!(v.provider, Provider::NotStarted | Provider::Starting) => {}
            (Phase::Refused, Some(Recorded::Refused { .. }))
                if v.provider.known_clean()
                    && (v.host.is_some() || matches!(v.provider, Provider::NotStarted)) => {}
            (Phase::Reserved, None)
                if v.host.is_none()
                    && v.generation.is_none()
                    && matches!(v.provider, Provider::NotStarted) => {}
            (Phase::Starting, None) if v.host.is_some() && v.generation.is_none() => {}
            (Phase::PublishingBase | Phase::BasePublished | Phase::PublishingRun, None)
                if v.host.is_some()
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
pub(super) enum Reservation {
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
    pub(super) fn reserve(
        &mut self,
        arguments: Arguments,
        token: Token,
        legacy: Option<&(Option<String>, Identity)>,
    ) -> Result<Reservation, String> {
        self.ensure()?;
        arguments.validate()?;
        token.validate()?;
        if self.document.as_ref().is_some_and(|doc| {
            let origin = &doc.latest.arguments;
            origin.home != arguments.home
                || origin.id != arguments.id
                || origin.native != arguments.native
                || origin.operation != arguments.operation
        }) {
            return Err("recovery attempt belongs to another home or native row".into());
        }
        let high_water = self.document.as_ref().map_or(0, |doc| doc.high_water);
        if token.epoch < high_water {
            return Ok(Reservation::Retired);
        }
        if token.epoch == high_water {
            let latest = &self
                .document
                .as_ref()
                .ok_or("missing latest recovery attempt")?
                .latest;
            if token != latest.token || arguments != latest.arguments {
                return Err("recovery token was reused with different arguments".into());
            }
            return Ok(match &latest.result {
                Some(result) => Reservation::Recorded(result.clone()),
                None => Reservation::Unfinished {
                    phase: latest.phase,
                    host: latest.host.clone(),
                },
            });
        }
        if Some(token.epoch) != high_water.checked_add(1) {
            return Err("recovery epoch is not the next offer".into());
        }
        // Meta.exit, a dead host, a free journal or absent leader are not a
        // substitute for this attempt's actual foreground cleanup receipt.
        if self.document.as_ref().is_some_and(|doc| {
            !matches!(doc.latest.provider, Provider::Exited { .. })
                && !(doc.latest.phase == Phase::Refused
                    && matches!(doc.latest.provider, Provider::NotStarted))
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
        if self.document.is_none() && current.is_some() {
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
            latest: Latest {
                token,
                arguments,
                run_id,
                phase: Phase::Reserved,
                host: None,
                provider: Provider::NotStarted,
                generation: None,
                result: None,
            },
        })?;
        Ok(Reservation::Start(key))
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
                || v.host
                    .as_ref()
                    .is_none_or(|host| provider.parent != host.pid || provider.boot != host.boot)
            {
                return Err("provider does not belong to this starting foreground owner".into());
            }
            v.provider = Provider::Owned { process: provider };
            Ok(())
        })
    }
    pub(super) fn publish_base<T>(
        &mut self,
        key: &AttemptKey,
        generation: Option<&str>,
        publish: impl FnOnce(&RowGate) -> Result<T, String>,
    ) -> Result<T, String> {
        self.check_live(key, Phase::Starting)?;
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
        if pid == 0 || pid > i32::MAX as u32 {
            return Err("invalid reaped provider PID".into());
        }
        self.change(key, |v| {
            match &v.provider {
                Provider::Owned { process } if process.pid == pid => {}
                // Birth capture can fail after a real spawn. The same actual
                // transport can still publish its own confirmed reap receipt.
                Provider::Starting => {}
                Provider::Unknown {
                    last: Some(process),
                } if process.pid == pid => {}
                Provider::Unknown { last: None } => {}
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
            let last = match &v.provider {
                Provider::Owned { process } => Some(process.clone()),
                Provider::Unknown { last } => last.clone(),
                Provider::Starting => None,
                _ => return Err("cleanup uncertainty cannot erase a known clean boundary".into()),
            };
            v.provider = Provider::Unknown { last };
            Ok(())
        })
    }
    pub(super) fn refuse(&mut self, key: &AttemptKey, reason: Reason) -> Result<(), String> {
        self.change(key, |v| {
            if v.result.is_some() || !v.provider.known_clean() {
                return Err("refusal has no confirmed clean provider boundary".into());
            }
            v.phase = Phase::Refused;
            v.result = Some(Recorded::Refused { reason });
            Ok(())
        })
    }
    fn check_live(&self, key: &AttemptKey, phase: Phase) -> Result<(), String> {
        let value = self.current(key)?;
        if value.phase != phase || !matches!(value.provider, Provider::Owned { .. }) {
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
            .latest;
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
        update(&mut next.latest)?;
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

/// Read-only offer helper: no row lock acquisition, file creation or PID probe.
pub(super) fn next_epoch(directory: &Path) -> Result<u64, String> {
    let high = read(directory)?.map_or(0, |value| value.high_water);
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
        assert_eq!(fs::read(held.gate.dir.join(NAME)).unwrap(), before);

        held.claim_host(&first, stamp(10, 1)).unwrap();
        assert!(held.claim_host(&first, stamp(11, 1)).is_err());
        held.starting_provider(&first).unwrap();
        assert!(
            held.starting_provider(&first).is_err(),
            "a replayed claim cannot issue another spawn"
        );
        assert!(held.owned_provider(&first, stamp(20, 99)).is_err());
        held.owned_provider(&first, stamp(20, 10)).unwrap();
        held.publish_base(&first, Some(&"c".repeat(32)), |_| Ok(()))
            .unwrap();
        held.publish_run(&first, |gate, run_id| {
            // This is a phase/row fixture, not an ACP Ready or Journal test.
            gate.write_meta(&crate::live::Meta {
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
        exhausted.latest.token.epoch = u64::MAX;
        held.store(exhausted).unwrap();
        assert!(next_epoch(&scratch.0).is_err());
        assert!(fs::metadata(held.gate.dir.join(NAME)).unwrap().len() <= BYTES as u64);
        let saved = fs::read(held.gate.dir.join(NAME)).unwrap();
        fs::write(held.gate.dir.join(NAME), vec![b'x'; BYTES + 1]).unwrap();
        assert!(next_epoch(&scratch.0).is_err());
        fs::write(held.gate.dir.join(NAME), saved).unwrap();
        drop(held);
        assert!(Held::load(&mut gate).is_ok());

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
                read(&gate.dir).unwrap().unwrap().latest.phase,
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
        held.claim_host(&next, stamp(30, 1)).unwrap();
        held.starting_provider(&next).unwrap();
        held.owned_provider(&next, stamp(40, 30)).unwrap();
        held.publish_base(&next, None, |_| Ok(())).unwrap();
        let failed: Result<(), String> = held.publish_run(&next, |gate, _| {
            assert_eq!(
                read(&gate.dir).unwrap().unwrap().latest.phase,
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
    }
}
