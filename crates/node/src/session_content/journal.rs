//! One host writes a capture; readers bind to one published display generation.
use khor_core::DeviceId;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, BufWriter, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

const VERSION: u8 = 1;
const MANIFEST_BYTES: u64 = 16 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Identity {
    pub home: DeviceId,
    pub agent: String,
    pub native_id: String,
}
impl Identity {
    pub fn directory(&self, root: &Path) -> PathBuf {
        let key = serde_json::to_vec(&(VERSION, self.home, &self.agent, &self.native_id))
            .expect("journal identity serializes");
        root.join(".khor/acp-journal")
            .join(blake3::hash(&key).to_hex().as_str())
    }
    fn validate(&self) -> Result<(), String> {
        if self.agent.is_empty() || self.native_id.is_empty() {
            return Err("journal requires an agent and full native session identity".into());
        }
        if serde_json::to_vec(self).map_err(|e| e.to_string())?.len() > 4096 {
            return Err("journal identity exceeds its byte budget".into());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Opening {
    New,
    Load,
    Resume,
}
impl Opening {
    pub fn key(self) -> &'static str {
        match self {
            Self::New => "new",
            Self::Load => "load",
            Self::Resume => "resume",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Manifest {
    version: u8,
    pub identity: Identity,
    pub opening: Opening,
    pub gaps: u64,
    active_turn: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    generation: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ActiveGeneration {
    version: u8,
    generation: String,
}

pub(super) fn validate_generation(generation: &str) -> Result<(), String> {
    if generation.len() != 32
        || !generation
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err("invalid journal generation".into());
    }
    Ok(())
}

fn generation_directory(directory: &Path, generation: Option<&str>) -> Result<PathBuf, String> {
    match generation {
        Some(generation) => {
            validate_generation(generation)?;
            Ok(directory.join("generations").join(generation))
        }
        None => Ok(directory.join("current")),
    }
}

fn active_generation(directory: &Path) -> Result<Option<String>, String> {
    let Some(bytes) = super::read_metadata(&directory.join("active.json"))? else {
        return Ok(None);
    };
    let active: ActiveGeneration = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if active.version != VERSION {
        return Err("unsupported journal generation pointer".into());
    }
    validate_generation(&active.generation)?;
    Ok(Some(active.generation))
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Record<'a, T: Serialize> {
    Update {
        turn: Option<&'a str>,
        notification: &'a T,
    },
    Prompt {
        turn: &'a str,
        text: &'a str,
        images: &'a [(String, String)],
        files: &'a [(String, String)],
    },
    PromptOrigin { turn: &'a str, token: &'a str, start: u64, end: u64 },
    Turn {
        turn: &'a str,
        outcome: Outcome,
        detail: Option<&'a str>,
    },
    Gap {
        opening: Opening,
        interrupted_turn: Option<&'a str>,
        incomplete_record: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Outcome {
    Ended,
    Cancelled,
    Failed,
    Unknown,
}

pub(crate) struct Journal {
    directory: PathBuf,
    file: File,
    manifest: Manifest,
    pending: bool,
    poisoned: bool,
    base_prepared: bool,
    _lock: File,
}

#[derive(Debug)]
pub(crate) struct ReadBoundary {
    pub binding: super::ReadBinding,
    pub cursor: crate::adaptor::transcript_page::Cursor,
}

/// Exclusive admission for one exact native conversation. Taking this owner
/// does not append a Gap, remove pending data, re-seat a row or publish a base.
/// The known-native caller must hold it before spawning its ACP provider.
pub(crate) struct JournalReservation {
    directory: PathBuf,
    identity: Identity,
    lock: File,
    generation: Option<String>,
}
impl JournalReservation {
    pub fn take(root: &Path, identity: Identity) -> Result<Self, String> {
        identity.validate()?;
        let directory = identity.directory(root);
        fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))
                .map_err(|e| e.to_string())?;
        }
        let lock = private_options()
            .create(true)
            .open(directory.join("writer.lock"))
            .map_err(|e| e.to_string())?;
        lock.try_lock()
            .map_err(|e| format!("journal already has a writer: {e}"))?;
        let generation = active_generation(&directory)?;
        let current = generation_directory(&directory, generation.as_deref())?;
        match read_manifest(&current)? {
            Some(manifest) if manifest.version == VERSION && manifest.identity == identity && manifest.generation == generation => {}
            Some(_) => return Err("journal manifest belongs to another source or version".into()),
            None if current.exists() || generation.is_some() => return Err("committed journal manifest is missing".into()),
            None => {}
        }
        Ok(Self {
            directory,
            identity,
            lock,
            generation,
        })
    }

    pub fn identity(&self) -> &Identity {
        &self.identity
    }

    pub fn check_binding(&self, directory: &Path) -> Result<(), String> {
        check_binding(directory, &self.identity)
    }

    /// Move the very same physical File owner into the journal. No unlock,
    /// duplicated handle, close/reopen window or second reservation occurs.
    pub fn into_journal(self, opening: Opening) -> Result<Journal, String> {
        Journal::from_reservation(self, opening)
    }
}

pub(crate) fn bound_identity(directory: &Path) -> Result<Option<Identity>, String> {
    let Some(bytes) = super::read_metadata(&directory.join("journal.json"))? else {
        return Ok(None);
    };
    let identity: Identity = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    identity.validate()?;
    Ok(Some(identity))
}

fn check_binding(directory: &Path, expected: &Identity) -> Result<(), String> {
    if let Some(identity) = bound_identity(directory)? {
        if &identity != expected {
            return Err("session already names another journal source".into());
        }
    }
    Ok(())
}

impl Journal {
    pub fn bind(&self, directory: &Path) -> Result<(), String> {
        check_binding(directory, &self.manifest.identity)?;
        let bytes = serde_json::to_vec(&self.manifest.identity).map_err(|e| e.to_string())?;
        super::write_metadata(&directory.join("journal.json"), &bytes)
    }
    pub fn open(root: &Path, identity: Identity, opening: Opening) -> Result<Self, String> {
        JournalReservation::take(root, identity)?.into_journal(opening)
    }
    fn from_reservation(reservation: JournalReservation, opening: Opening) -> Result<Self, String> {
        let JournalReservation {
            directory,
            identity,
            lock,
            generation,
        } = reservation;
        if active_generation(&directory)? != generation {
            return Err("journal generation changed during its reservation".into());
        }
        let pending = directory.join("pending");
        match fs::remove_dir_all(&pending) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.to_string()),
        }
        let current = generation_directory(&directory, generation.as_deref())?;
        let previous = read_manifest(&current)?;
        let mut journal = if let Some(manifest) = previous {
            if manifest.version != VERSION
                || manifest.identity != identity
                || manifest.generation != generation
            {
                return Err("journal manifest belongs to another source or version".into());
            }
            if opening == Opening::Load {
                // A Load replay is complete, not a suffix to compare by text.
                let next_generation = crate::link::fresh_hex()?;
                fs::create_dir(&pending).map_err(|e| e.to_string())?;
                let file = private_options()
                    .create_new(true)
                    .open(pending.join("records.jsonl"))
                    .map_err(|e| e.to_string())?;
                let interrupted = manifest.active_turn;
                let mut next = Self {
                    directory,
                    file,
                    manifest: Manifest {
                        version: VERSION,
                        identity,
                        opening,
                        gaps: manifest.gaps.saturating_add(1),
                        active_turn: None,
                        generation: Some(next_generation),
                    },
                    pending: true,
                    poisoned: false,
                    base_prepared: false,
                    _lock: lock,
                };
                next.append(&Record::<'_, ()>::Gap {
                    opening,
                    interrupted_turn: interrupted.as_deref(),
                    incomplete_record: false,
                })?;
                next
            } else {
                let file = private_options()
                    .append(true)
                    .open(current.join("records.jsonl"))
                    .map_err(|e| e.to_string())?;
                Self {
                    directory,
                    file,
                    manifest,
                    pending: false,
                    poisoned: false,
                    base_prepared: false,
                    _lock: lock,
                }
            }
        } else {
            if current.exists() || generation.is_some() {
                return Err("committed journal manifest is missing".into());
            }
            fs::create_dir(&pending).map_err(|e| e.to_string())?;
            let file = private_options()
                .create_new(true)
                .open(pending.join("records.jsonl"))
                .map_err(|e| e.to_string())?;
            Self {
                directory,
                file,
                manifest: Manifest {
                    version: VERSION,
                    identity,
                    opening,
                    gaps: 0,
                    active_turn: None,
                    generation: None,
                },
                pending: true,
                poisoned: false,
                base_prepared: false,
                _lock: lock,
            }
        };
        if !journal.pending {
            let incomplete_record = journal.finish_partial_record()?;
            let interrupted = journal.manifest.active_turn.take();
            journal.append(&Record::<'_, ()>::Gap {
                opening,
                interrupted_turn: interrupted.as_deref(),
                incomplete_record,
            })?;
            journal.manifest.gaps = journal.manifest.gaps.saturating_add(1);
            journal.persist()?;
        }
        Ok(journal)
    }

    pub fn is_pending(&self) -> bool {
        self.pending
    }

    pub fn capture(&self, view: crate::SessionId) -> Result<ReadBoundary, String> {
        use super::reference::{ContentReference, ReadBinding};
        self.ensure_writable()?;
        if self.pending {
            return Err("capture has not committed its opening".into());
        }
        // append flushes each complete record; observing it does not add a durability barrier.
        let binding = ReadBinding::new(
            view,
            ContentReference {
                home: self.manifest.identity.home,
                namespace: khor_core::ObservationNamespace::Acp,
                agent: self.manifest.identity.agent.clone(),
                native_id: self.manifest.identity.native_id.clone(),
            },
        )?
        .with_journal_generation(self.manifest.generation.as_deref())?;
        let mut published = File::open(
            generation_directory(&self.directory, self.manifest.generation.as_deref())?
                .join("records.jsonl"),
        )
        .map_err(|error| error.to_string())?;
        let cursor = crate::adaptor::transcript_page::capture_eof(
            &self.file,
            &mut published,
            &binding.source_id(),
        )?;
        Ok(ReadBoundary { binding, cursor })
    }

    pub fn abort_base(&mut self) -> Result<(), String> {
        if !self.pending {
            return Ok(());
        }
        fs::remove_dir_all(self.directory.join("pending"))
            .map_err(|e| format!("journal pending cleanup failed: {e}"))?;
        self.pending = false;
        self.poisoned = true;
        Ok(())
    }

    pub fn note<T: Serialize>(&mut self, notification: &T) -> Result<(), String> {
        let turn = self.manifest.active_turn.clone();
        self.append(&Record::Update {
            turn: turn.as_deref(),
            notification,
        })
    }

    pub fn commit_base(&mut self) -> Result<(), String> {
        self.prepare_base()?;
        self.publish_base()
    }

    /// Resume adds real notifications to an existing base rather than creating
    /// another replay. Make its finite accepted prefix durable before Ready.
    pub(crate) fn sync_received_prefix(&mut self) -> Result<(), String> {
        self.ensure_writable()?;
        let result = self.file.sync_data().map_err(|error| error.to_string());
        if result.is_err() {
            self.poisoned = true;
        }
        result
    }

    /// Bulk durability work happens before a caller enters its short attempt
    /// publication gate. It does not change the current display-base pointer.
    pub(crate) fn prepare_base(&mut self) -> Result<(), String> {
        if !self.pending {
            return Err("journal base is already committed".into());
        }
        self.ensure_writable()?;
        let result = (|| {
            self.file.sync_all().map_err(|e| e.to_string())?;
            self.persist()?;
            File::open(self.directory.join("pending/manifest.json"))
                .and_then(|file| file.sync_all())
                .map_err(|e| e.to_string())?;
            sync_directory(&self.directory.join("pending"))
        })();
        if result.is_err() {
            self.poisoned = true;
        }
        self.base_prepared = result.is_ok();
        result
    }

    /// The caller must recheck its attempt epoch around this small publication
    /// step. Once renamed, a completed base is retained even if pointer/row
    /// publication later fails; it is never relabelled as an aborted part.
    pub(crate) fn publish_base(&mut self) -> Result<(), String> {
        if !self.pending {
            return Err("journal base is already committed".into());
        }
        self.ensure_writable()?;
        if !self.base_prepared {
            return Err("journal base has not been prepared".into());
        }
        let target = generation_directory(&self.directory, self.manifest.generation.as_deref())?;
        let result = (|| {
            if target.exists() {
                return Err("journal base target is already published".into());
            }
            if self.manifest.generation.is_some() {
                fs::create_dir_all(self.directory.join("generations"))
                    .map_err(|e| e.to_string())?;
            }
            fs::rename(self.directory.join("pending"), &target).map_err(|e| e.to_string())?;
            self.pending = false;
            self.base_prepared = false;
            sync_directory(target.parent().expect("published base has a parent"))?;
            if let Some(generation) = &self.manifest.generation {
                let active = ActiveGeneration {
                    version: VERSION,
                    generation: generation.clone(),
                };
                super::write_metadata(
                    &self.directory.join("active.json"),
                    &serde_json::to_vec(&active).map_err(|e| e.to_string())?,
                )?;
                File::open(self.directory.join("active.json"))
                    .and_then(|file| file.sync_all())
                    .map_err(|e| e.to_string())?;
            }
            sync_directory(&self.directory)
        })();
        if result.is_err() {
            self.poisoned = true;
        }
        result
    }

    /// Host acceptance is durable before its caller may dispatch a prompt to the agent.
    pub fn accept(
        &mut self,
        turn: &str,
        text: &str,
        images: &[(String, String)],
        files: &[(String, String)],
        token: Option<&str>,
    ) -> Result<(), String> {
        if self.pending {
            return Err("journal bootstrap has not committed".into());
        }
        if self.manifest.active_turn.is_some() {
            return Err("journal already has an active turn".into());
        }
        if turn.is_empty() || turn.len() > 256 {
            return Err("invalid journal turn identity".into());
        }
        if token.is_some_and(|token|token.is_empty() || token.len()>128 || !token.bytes().all(|b|b.is_ascii_alphanumeric() || b"._:-".contains(&b))) {
            return Err("invalid journal client token".into());
        }
        let prompt: Record<'_, ()> = Record::Prompt {
            turn,
            text,
            images,
            files,
        };
        let range = self.append_range(&prompt)?;
        self.manifest.active_turn = Some(turn.to_owned());
        self.persist()?;
        if let Some(token) = token {
            let origin: Record<'_, ()> = Record::PromptOrigin { turn, token, start:range.start, end:range.end };
            self.append(&origin)?;
        }
        Ok(())
    }

    pub fn finish(
        &mut self,
        turn: &str,
        outcome: Outcome,
        detail: Option<&str>,
    ) -> Result<(), String> {
        if self.manifest.active_turn.as_deref() != Some(turn) {
            return Err("journal completion belongs to another turn".into());
        }
        let completion: Record<'_, ()> = Record::Turn {
            turn,
            outcome,
            detail,
        };
        self.append(&completion)?;
        self.manifest.active_turn = None;
        self.persist()
    }

    fn append<T: Serialize>(&mut self, record: &T) -> Result<(), String> {
        self.append_range(record).map(|_| ())
    }

    fn append_range<T: Serialize>(&mut self, record: &T) -> Result<std::ops::Range<u64>, String> {
        self.ensure_writable()?;
        self.base_prepared = false;
        let result = (|| {
            let start = self.file.seek(SeekFrom::End(0))?;
            let mut writer = BufWriter::with_capacity(64 * 1024, &mut self.file);
            serde_json::to_writer(&mut writer, record).map_err(io::Error::other)?;
            writer.write_all(b"\n")?;
            writer.flush()?;
            drop(writer);
            Ok::<_,io::Error>(start..self.file.seek(SeekFrom::End(0))?)
        })();
        match result {
            Ok(range) => Ok(range),
            Err(error) => {
                self.poisoned = true;
                Err(format!("journal append failed; capture must retire: {error}"))
            }
        }
    }

    fn persist(&mut self) -> Result<(), String> {
        self.ensure_writable()?;
        let result = (|| {
            self.file.sync_data().map_err(|e| e.to_string())?;
            let bytes = serde_json::to_vec(&self.manifest).map_err(|e| e.to_string())?;
            let folder = if self.pending {
                self.directory.join("pending")
            } else {
                generation_directory(&self.directory, self.manifest.generation.as_deref())?
            };
            super::write_metadata(&folder.join("manifest.json"), &bytes)
        })();
        if result.is_err() {
            self.poisoned = true;
        }
        result
    }

    fn ensure_writable(&self) -> Result<(), String> {
        if self.poisoned {
            Err("journal writer has failed".into())
        } else {
            Ok(())
        }
    }

    fn finish_partial_record(&mut self) -> Result<bool, String> {
        let length = self.file.metadata().map_err(|e| e.to_string())?.len();
        if length == 0 {
            return Ok(false);
        }
        self.file
            .seek(SeekFrom::End(-1))
            .map_err(|e| e.to_string())?;
        let mut last = [0];
        self.file.read_exact(&mut last).map_err(|e| e.to_string())?;
        if last == [b'\n'] {
            return Ok(false);
        }
        // Preserve interrupted bytes as an explicit raw record instead of joining new JSON onto them.
        self.file.write_all(b"\n").map_err(|e| e.to_string())?;
        Ok(true)
    }
}

pub(crate) struct Source {
    directory: PathBuf,
    path: PathBuf,
    manifest: Manifest,
}
impl Source {
    pub fn identity(&self) -> &Identity {
        &self.manifest.identity
    }
    pub fn find(root: &Path, identity: Identity) -> Result<Option<Self>, String> {
        identity.validate()?;
        let directory = identity.directory(root);
        let generation = active_generation(&directory)?;
        Self::find_generation(root, identity, generation.as_deref())
    }
    pub(crate) fn find_generation(
        root: &Path,
        identity: Identity,
        generation: Option<&str>,
    ) -> Result<Option<Self>, String> {
        identity.validate()?;
        let directory = identity.directory(root);
        let current = generation_directory(&directory, generation)?;
        let Some(manifest) = read_manifest(&current)? else {
            return if current.exists() {
                Err("committed journal manifest is missing".into())
            } else {
                Ok(None)
            };
        };
        if manifest.version != VERSION
            || manifest.identity != identity
            || manifest.generation.as_deref() != generation
        {
            return Err("journal source identity or generation changed".into());
        }
        Ok(Some(Self {
            directory,
            path: current.join("records.jsonl"),
            manifest,
        }))
    }
    pub(crate) fn generation(&self) -> Option<&str> {
        self.manifest.generation.as_deref()
    }
    pub(crate) fn following(&self) -> bool {
        active_generation(&self.directory)
            .is_ok_and(|active| active.as_deref() == self.generation())
    }
    pub fn registered(
        root: &Path,
        home: DeviceId,
        id: &crate::SessionId,
        directory: Option<&Path>,
    ) -> Result<Option<Self>, String> {
        let Some(directory) = directory else {
            return Ok(None);
        };
        let Some(bytes) = super::read_metadata(&directory.join("journal.json"))? else {
            return Ok(None);
        };
        let identity: Identity = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        if identity.home != home || crate::adaptor::id_for(&identity.native_id) != *id {
            return Err("journal registration belongs to another session or home".into());
        }
        Self::find(root, identity)
    }
    pub fn capture(&self) -> super::Capture {
        super::Capture {
            opening: self.manifest.opening.key(),
            gaps: self.manifest.gaps,
        }
    }
    pub fn source_id(&self, id: &crate::SessionId) -> String {
        if let Some(generation) = self.generation() {
            return serde_json::to_string(&(
                2u8,
                "acp_journal",
                id,
                &self.manifest.identity,
                generation,
            ))
            .expect("journal generation identity serializes");
        }
        serde_json::to_string(&(VERSION, "acp_journal", id, &self.manifest.identity))
            .expect("journal identity serializes")
    }
    pub fn open(&self) -> io::Result<File> {
        let file = File::open(&self.path)?;
        if !file.metadata()?.is_file() {
            return Err(io::Error::other("journal source is not a file"));
        }
        Ok(file)
    }
}

pub(crate) fn parse_record(text: &str) -> Vec<super::ContentEvent> {
    use super::ContentEvent;
    let Ok(mut record) = serde_json::from_str::<serde_json::Value>(text) else {
        return vec![ContentEvent::Reference];
    };
    let Some(object) = record.as_object_mut() else {
        return vec![ContentEvent::Reference];
    };
    match object.get("kind").and_then(|v| v.as_str()) {
        Some("prompt_origin") => {
            let (Some(turn),Some(token),Some(start),Some(end)) = (
                object.get("turn").and_then(|v|v.as_str()), object.get("token").and_then(|v|v.as_str()),
                object.get("start").and_then(|v|v.as_u64()), object.get("end").and_then(|v|v.as_u64()),
            ) else { return vec![ContentEvent::Reference]; };
            if turn.is_empty() || turn.len()>256 || token.is_empty() || token.len()>128 || start>=end {
                return vec![ContentEvent::Reference];
            }
            vec![ContentEvent::PromptOrigin { turn:turn.into(), token:token.into(), start, end }]
        }
        Some("update") => {
            let Some(notification) = object.remove("notification") else {
                return vec![ContentEvent::Reference];
            };
            if serde_json::from_value::<agent_client_protocol::schema::v1::SessionNotification>(
                notification.clone(),
            )
            .is_err()
            {
                return vec![ContentEvent::Reference];
            }
            // This outer turn is written by the journal only after a local
            // prompt is persisted. An optional agent echo is source detail,
            // rather than a second user input; keep its exact raw record.
            if object.get("turn").and_then(|v|v.as_str()).is_some_and(|turn|!turn.is_empty() && turn.len()<=256)
                && notification["update"]["sessionUpdate"] == "user_message_chunk"
            {
                return vec![ContentEvent::LocalUserEcho];
            }
            vec![ContentEvent::Update { update: notification, starts_message: false }]
        }
        Some("prompt") => {
            let Some(text) = object.get("text").and_then(|v| v.as_str()) else {
                return vec![ContentEvent::Reference];
            };
            if object.get("turn").and_then(|v| v.as_str()).is_none()
                || !object.get("images").is_some_and(|v| v.is_array())
                || !object.get("files").is_some_and(|v| v.is_array())
            {
                return vec![ContentEvent::Reference];
            }
            let mut updates = vec![ContentEvent::Update {
                update: serde_json::json!({"update":{"sessionUpdate":"user_message_chunk","content":{"type":"text","text":text}}}),
                starts_message: true,
            }];
            if let Some(images) = object.get("images").and_then(|v| v.as_array()) {
                for image in images {
                    let (Some(mime), Some(data)) = (
                        image.get(0).and_then(|v| v.as_str()),
                        image.get(1).and_then(|v| v.as_str()),
                    ) else {
                        return vec![ContentEvent::Reference];
                    };
                    updates.push(ContentEvent::Update { update: serde_json::json!({"update":{"sessionUpdate":"user_message_chunk","content":{"type":"image","mimeType":mime,"data":data}}}), starts_message: false });
                }
            }
            if let Some(files) = object.get("files").and_then(|v| v.as_array()) {
                for file in files {
                    let (Some(name), Some(text)) = (
                        file.get(0).and_then(|v| v.as_str()),
                        file.get(1).and_then(|v| v.as_str()),
                    ) else {
                        return vec![ContentEvent::Reference];
                    };
                    updates.push(ContentEvent::Update { update: serde_json::json!({"update":{"sessionUpdate":"user_message_chunk","content":{"type":"resource","resource":{"uri":format!("file://{name}"),"text":text}}}}), starts_message: false });
                }
            }
            updates
        }
        Some("turn") => {
            if object.get("turn").and_then(|v| v.as_str()).is_none()
                || object
                    .get("detail")
                    .is_none_or(|v| !v.is_null() && !v.is_string())
            {
                return vec![ContentEvent::Reference];
            }
            let fallback = match object.get("outcome").and_then(|v| v.as_str()) {
                Some("ended") => "EndTurn",
                Some("cancelled") => "Cancelled",
                Some("failed") => "Failed",
                Some("unknown") => "Unknown",
                _ => return vec![ContentEvent::Reference],
            };
            vec![ContentEvent::Turn {
                stop: object
                    .get("detail")
                    .and_then(|v| v.as_str())
                    .unwrap_or(fallback)
                    .to_owned(),
            }]
        }
        Some("gap") => match object.get("opening").and_then(|v| v.as_str()) {
            Some(opening @ ("new" | "load" | "resume"))
                if object
                    .get("incomplete_record")
                    .is_some_and(|v| v.is_boolean())
                    && object
                        .get("interrupted_turn")
                        .is_some_and(|v| v.is_null() || v.is_string()) =>
            {
                vec![ContentEvent::CaptureGap {
                    opening: opening.to_owned(),
                    interrupted: object.get("interrupted_turn").is_some_and(|v| !v.is_null()),
                }]
            }
            _ => vec![ContentEvent::Reference],
        },
        _ => vec![ContentEvent::Reference],
    }
}

impl Drop for Journal {
    fn drop(&mut self) {
        if self.pending {
            let _ = fs::remove_dir_all(self.directory.join("pending"));
        }
    }
}

pub(crate) fn read_manifest(current: &Path) -> Result<Option<Manifest>, String> {
    let file = match File::open(current.join("manifest.json")) {
        Ok(file) => file,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    let mut bytes = Vec::new();
    file.take(MANIFEST_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MANIFEST_BYTES {
        return Err("journal manifest exceeds its byte budget".into());
    }
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|e| e.to_string())
}

fn private_options() -> OpenOptions {
    let mut options = OpenOptions::new();
    options.read(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options
}

fn sync_directory(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|e| e.to_string())?;
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            Self(std::env::temp_dir().join(format!(
                "khor-journal-{}-{}",
                std::process::id(),
                crate::link::fresh_hex().unwrap()
            )))
        }
        fn identity(&self) -> Identity {
            Identity {
                home: DeviceId([3; 32]),
                agent: "acp:test".into(),
                native_id: "provider/full/id".into(),
            }
        }
        fn current(&self) -> PathBuf {
            self.identity().directory(&self.0).join("current")
        }
        fn records(&self) -> String {
            fs::read_to_string(self.current().join("records.jsonl")).unwrap()
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn native_reservation_keeps_the_same_exclusive_file_owner_through_journal_open() {
        let f = Fixture::new();
        let reservation = JournalReservation::take(&f.0, f.identity()).unwrap();
        #[cfg(unix)]
        let descriptor = {
            use std::os::fd::AsRawFd;
            reservation.lock.as_raw_fd()
        };
        assert!(JournalReservation::take(&f.0, f.identity()).is_err());
        assert!(!f.identity().directory(&f.0).join("pending").exists());
        let journal = reservation.into_journal(Opening::New).unwrap();
        #[cfg(unix)]
        {
            use std::os::fd::AsRawFd;
            assert_eq!(journal._lock.as_raw_fd(), descriptor);
        }
        assert!(JournalReservation::take(&f.0, f.identity()).is_err());
        drop(journal);
        assert!(JournalReservation::take(&f.0, f.identity()).is_ok());
    }

    #[test]
    fn dropping_a_preinit_reservation_keeps_old_records_and_pending_data_unchanged() {
        let f = Fixture::new();
        let mut journal = Journal::open(&f.0, f.identity(), Opening::New).unwrap();
        journal
            .note(&serde_json::json!({"text":"retained"}))
            .unwrap();
        journal.commit_base().unwrap();
        drop(journal);
        let directory = f.identity().directory(&f.0);
        fs::create_dir(directory.join("pending")).unwrap();
        fs::write(
            directory.join("pending/unpublished"),
            b"previous interrupted attempt",
        )
        .unwrap();
        let names = [
            "current/records.jsonl",
            "current/manifest.json",
            "pending/unpublished",
        ];
        let before = names.map(|name| fs::read(directory.join(name)).unwrap());
        let row = f.0.join("row");
        fs::create_dir(&row).unwrap();
        let mut another = f.identity();
        another.agent = "another-agent".into();
        fs::write(
            row.join("journal.json"),
            serde_json::to_vec(&another).unwrap(),
        )
        .unwrap();
        let reservation = JournalReservation::take(&f.0, f.identity()).unwrap();
        assert!(
            reservation
                .check_binding(&row)
                .unwrap_err()
                .contains("another journal source")
        );
        // This models dropping before ACP initialization succeeds, not a claim
        // that a real provider process was exercised by this unit fixture.
        drop(reservation);
        assert_eq!(
            names.map(|name| fs::read(directory.join(name)).unwrap()),
            before
        );
        assert_eq!(
            fs::read(row.join("journal.json")).unwrap(),
            serde_json::to_vec(&another).unwrap()
        );
        assert!(JournalReservation::take(&f.0, f.identity()).is_ok());
    }
    #[test]
    fn capture_requires_a_committed_current_file_and_preserves_canonical_identity() {
        let f = Fixture::new();
        let mut journal = Journal::open(&f.0, f.identity(), Opening::New).unwrap();
        let view = crate::SessionId("view".into());
        assert!(journal.capture(view.clone()).unwrap_err().contains("committed"));
        journal.note(&serde_json::json!({"text":"same"})).unwrap();
        journal.commit_base().unwrap();
        let fence = journal.capture(view.clone()).unwrap();
        assert_eq!(fence.binding.view(), &view);
        assert_eq!(fence.binding.reference().home, f.identity().home);
        assert_eq!(fence.binding.reference().native_id, f.identity().native_id);
        assert_eq!(fence.binding.reference().namespace, khor_core::ObservationNamespace::Acp);
        assert!(fence.cursor.is_for_source(&fence.binding.source_id()));
        assert!(fence.binding.encode().unwrap().len() <= super::super::reference::METADATA_BYTES);
        let path = f.current().join("records.jsonl");
        let content = fs::read(&path).unwrap();
        fs::rename(&path, path.with_extension("retired")).unwrap();
        fs::write(&path, content).unwrap();
        assert!(journal.capture(view.clone()).unwrap_err().contains("identity changed"));
        journal.poisoned = true;
        assert!(journal.capture(view).unwrap_err().contains("failed"));
    }

    #[test]
    fn base_is_atomic_and_pending_recovery_does_not_claim_committed_content() {
        let f = Fixture::new();
        let mut journal = Journal::open(&f.0, f.identity(), Opening::Load).unwrap();
        journal.note(&serde_json::json!({"text":"base"})).unwrap();
        assert!(!f.current().exists());
        assert!(
            journal
                .accept("early", "not sent", &[], &[], None)
                .unwrap_err()
                .contains("bootstrap")
        );
        drop(journal);
        assert!(!f.identity().directory(&f.0).join("pending").exists());
        let mut journal = Journal::open(&f.0, f.identity(), Opening::Load).unwrap();
        journal
            .note(&serde_json::json!({"text":"complete"}))
            .unwrap();
        journal.commit_base().unwrap();
        assert!(f.current().is_dir());
        assert!(f.records().contains("complete"));
        assert!(!f.records().contains("base"));
        assert!(!journal.is_pending());
        assert_eq!(
            read_manifest(&f.current()).unwrap().unwrap().opening,
            Opening::Load
        );
        assert!(journal.commit_base().is_err());
    }

    #[test]
    fn load_publishes_a_complete_new_base_without_redirecting_old_bindings() {
        let f = Fixture::new();
        let view = crate::SessionId("same-logical-view".into());
        let mut first = Journal::open(&f.0, f.identity(), Opening::New).unwrap();
        first.note(&serde_json::json!({"text":"X"})).unwrap();
        first.commit_base().unwrap();
        let old = first.capture(view.clone()).unwrap();
        let old_records = f.records();
        let old_manifest = fs::read(f.current().join("manifest.json")).unwrap();
        drop(first);

        let mut loaded = Journal::open(&f.0, f.identity(), Opening::Load).unwrap();
        for text in ["X", "X", "Y"] {
            loaded.note(&serde_json::json!({"text":text})).unwrap();
        }
        loaded.prepare_base().unwrap();
        assert_eq!(f.records(), old_records);
        assert_eq!(
            fs::read(f.current().join("manifest.json")).unwrap(),
            old_manifest
        );
        assert!(loaded.capture(view.clone()).is_err());
        assert!(!f.identity().directory(&f.0).join("active.json").exists());
        loaded.publish_base().unwrap();

        let next = loaded.capture(view.clone()).unwrap();
        assert_ne!(next.binding.source_id(), old.binding.source_id());
        assert!(next.cursor.is_for_source(&next.binding.source_id()));
        let old_source = old
            .binding
            .resolve_exact(&f.0, f.identity().home)
            .unwrap()
            .unwrap();
        let new_source = next
            .binding
            .resolve_exact(&f.0, f.identity().home)
            .unwrap()
            .unwrap();
        let read = |source: &super::super::ContentSource| {
            let (mut file, _) = source.open().unwrap();
            let mut text = String::new();
            file.read_to_string(&mut text).unwrap();
            text
        };
        assert_eq!(read(&old_source), old_records);
        let raw = read(&new_source);
        // These are three exact raw notifications, not three invented message
        // boundaries or a text-based reconciliation of a replayed prefix.
        let texts: Vec<String> = raw
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
            .filter(|record| record["kind"] == "update")
            .map(|record| record["notification"]["text"].as_str().unwrap().to_owned())
            .collect();
        assert_eq!(texts, ["X", "X", "Y"]);
        assert_eq!(f.records(), old_records);
        assert_eq!(
            fs::read(f.current().join("manifest.json")).unwrap(),
            old_manifest
        );
        let current = Source::find(&f.0, f.identity()).unwrap().unwrap();
        assert!(current.following());
        assert!(current.generation().is_some());
        assert!(
            !Source::find_generation(&f.0, f.identity(), None)
                .unwrap()
                .unwrap()
                .following()
        );

        // Resume has no replay base. Its actual early notes are appended to
        // the selected generation without replacing or deduplicating them.
        let selected = next.binding.source_id();
        drop(loaded);
        let mut resumed = Journal::open(&f.0, f.identity(), Opening::Resume).unwrap();
        assert!(!resumed.is_pending());
        resumed
            .note(&serde_json::json!({"text":"received-during-resume"}))
            .unwrap();
        let resumed_capture = resumed.capture(view).unwrap();
        assert_eq!(resumed_capture.binding.source_id(), selected);
        assert!(read(&new_source).contains("received-during-resume"));
        assert_eq!(read(&old_source), old_records);
        let completed = read(&new_source);
        drop(resumed);
        let mut later = Journal::open(&f.0, f.identity(), Opening::Load).unwrap();
        later.note(&serde_json::json!({"text":"Z"})).unwrap();
        later.commit_base().unwrap();
        let later_capture = later
            .capture(resumed_capture.binding.view().clone())
            .unwrap();
        assert_ne!(later_capture.binding.source_id(), selected);
        assert_eq!(
            read(&new_source),
            completed,
            "a version-two binding also keeps its exact previous generation"
        );
        assert!(!current.following());
        assert_eq!(read(&old_source), old_records);
    }

    #[test]
    fn aborted_or_failed_load_keeps_the_previous_published_base_and_completed_generations() {
        let f = Fixture::new();
        let mut first = Journal::open(&f.0, f.identity(), Opening::New).unwrap();
        first.note(&serde_json::json!({"text":"retained"})).unwrap();
        first.commit_base().unwrap();
        drop(first);
        let before = f.records();
        {
            let mut aborted = Journal::open(&f.0, f.identity(), Opening::Load).unwrap();
            aborted
                .note(&serde_json::json!({"text":"never published"}))
                .unwrap();
            aborted.prepare_base().unwrap();
            aborted
                .note(&serde_json::json!({"text":"arrived after the prepared fence"}))
                .unwrap();
            assert!(
                aborted
                    .publish_base()
                    .unwrap_err()
                    .contains("not been prepared")
            );
            assert_eq!(f.records(), before);
            aborted.prepare_base().unwrap();
            aborted.abort_base().unwrap();
        }
        assert_eq!(f.records(), before);
        let directory = f.identity().directory(&f.0);
        assert!(!directory.join("active.json").exists());

        let mut failed = Journal::open(&f.0, f.identity(), Opening::Load).unwrap();
        failed
            .note(&serde_json::json!({"text":"complete but not selected"}))
            .unwrap();
        failed.prepare_base().unwrap();
        let generation = failed.manifest.generation.clone().unwrap();
        // Atomic pointer publication is made to fail after the complete base
        // rename. It must neither select a partial file nor delete old data.
        fs::create_dir(directory.join("active.json")).unwrap();
        assert!(failed.publish_base().is_err());
        assert!(!failed.is_pending());
        assert!(failed.capture(crate::SessionId("view".into())).is_err());
        drop(failed);
        assert_eq!(f.records(), before);
        let retained = generation_directory(&directory, Some(&generation)).unwrap();
        assert!(retained.join("manifest.json").is_file());
        assert!(
            fs::read_to_string(retained.join("records.jsonl"))
                .unwrap()
                .contains("complete but not selected")
        );
        assert!(!directory.join("pending").exists());
    }

    #[test]
    fn prompt_attachments_and_identical_updates_survive_without_user_echo() {
        let f = Fixture::new();
        let mut journal = Journal::open(&f.0, f.identity(), Opening::New).unwrap();
        journal.commit_base().unwrap();
        let image = "a".repeat(150_000);
        journal
            .accept(
                "turn-1",
                "你好",
                &[("image/png".into(), image.clone())],
                &[("文档.md".into(), "正文".into())],
                None,
            )
            .unwrap();
        let delta = serde_json::json!({"sessionUpdate":"agent_message_chunk","text":"same"});
        journal.note(&delta).unwrap();
        journal.note(&delta).unwrap();
        assert!(journal.finish("wrong", Outcome::Ended, None).is_err());
        journal.finish("turn-1", Outcome::Ended, None).unwrap();
        let records: Vec<serde_json::Value> = f
            .records()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(records.len(), 4);
        assert_eq!(records[0]["images"][0][1], image);
        assert_eq!(records[0]["files"][0][1], "正文");
        assert_eq!(records[1], records[2]);
        assert_eq!(records[3]["outcome"], "ended");
    }

    #[test]
    fn origin_records_preserve_large_prompt_ranges_without_deduplicating_reused_tokens() {
        let f=Fixture::new();
        let mut journal=Journal::open(&f.0,f.identity(),Opening::New).unwrap();
        journal.commit_base().unwrap();
        let image="图".repeat(70_000);
        for turn in ["first-turn","second-turn"] {
            journal.accept(turn,"same words",&[("image/png".into(),image.clone())],&[],Some("same-client-token")).unwrap();
            journal.finish(turn,Outcome::Ended,Some("EndTurn")).unwrap();
        }
        let bytes=std::fs::read(f.current().join("records.jsonl")).unwrap();
        let mut offset=0;let mut count=0;
        for line in bytes.split_inclusive(|b|*b==b'\n') {
            let record:serde_json::Value=serde_json::from_slice(line).unwrap();
            if record["kind"]=="prompt_origin" {
                count+=1;assert!(line.len()<512);
                let start=record["start"].as_u64().unwrap() as usize;
                let end=record["end"].as_u64().unwrap() as usize;
                assert_eq!(end,offset);assert!(end-start>64*1024);
                let prompt:serde_json::Value=serde_json::from_slice(&bytes[start..end]).unwrap();
                assert_eq!(prompt["kind"],"prompt");assert_eq!(prompt["turn"],record["turn"]);
                assert_eq!(prompt["images"][0][1],image);
                assert!(matches!(&parse_record(std::str::from_utf8(line).unwrap())[0],super::super::ContentEvent::PromptOrigin {token,..} if token=="same-client-token"));
            }
            offset+=line.len();
        }
        assert_eq!(count,2,"client token is correlation, never a deduplication identity");
    }

    #[test]
    fn failed_acceptance_manifest_does_not_publish_a_prompt_origin() {
        let f=Fixture::new();
        let mut journal=Journal::open(&f.0,f.identity(),Opening::New).unwrap();
        journal.commit_base().unwrap();
        let manifest=f.current().join("manifest.json");
        std::fs::remove_file(&manifest).unwrap();
        std::fs::create_dir(&manifest).unwrap();
        assert!(journal.accept("not-dispatched","original unsent input",&[],&[],Some("client-token")).is_err());
        let records=f.records();
        let rows:Vec<serde_json::Value>=records.lines().map(|line|serde_json::from_str(line).unwrap()).collect();
        assert_eq!(rows.len(),1);assert_eq!(rows[0]["kind"],"prompt");
        assert_eq!(rows[0]["text"],"original unsent input");
        assert!(!records.contains("prompt_origin"));
        assert!(journal.capture(crate::SessionId("failed-view".into())).unwrap_err().contains("failed"));
        assert!(journal.note(&serde_json::json!({"not":"dispatched"})).unwrap_err().contains("failed"));
    }

    #[test]
    fn local_turn_echoes_keep_raw_records_without_duplicating_canonical_prompts() {
        let f=Fixture::new();
        let mut journal=Journal::open(&f.0,f.identity(),Opening::New).unwrap();
        let user=serde_json::json!({"sessionId":f.identity().native_id,"update":{"sessionUpdate":"user_message_chunk","content":{"type":"text","text":"same text"}}});
        // A replay/bootstrap user chunk has no local prompt provenance.
        journal.note(&user).unwrap();journal.commit_base().unwrap();
        for turn in ["local-one","local-two"] {
            journal.accept(turn,"same text",&[],&[],None).unwrap();
            journal.note(&user).unwrap();journal.finish(turn,Outcome::Ended,Some("EndTurn")).unwrap();
        }
        let bytes=std::fs::read(f.current().join("records.jsonl")).unwrap();
        let mut file=std::fs::File::open(f.current().join("records.jsonl")).unwrap();
        let page=crate::adaptor::transcript_page::read_page(&mut file,"owned-journal",None,parse_record,&||false).unwrap();
        let users=page.events.iter().filter(|event|matches!(&event.payload,super::super::ContentEvent::Update { update, .. } if update["update"]["sessionUpdate"]=="user_message_chunk")).count();
        assert_eq!(users,3,"OWNED_ECHO_DUPLICATION: bootstrap plus two different recorded turns must remain three user messages");
        let echoes:Vec<_>=page.events.iter().filter(|event|matches!(&event.payload,super::super::ContentEvent::LocalUserEcho)).collect();
        assert_eq!(echoes.len(),2,"every associated echo remains an inspectable raw record");
        let mut turns=Vec::new();
        for echo in echoes {
            let raw:serde_json::Value=serde_json::from_slice(&bytes[echo.position.start as usize..echo.position.end as usize]).unwrap();
            assert_eq!(raw["kind"],"update");assert_eq!(raw["notification"],user);
            turns.push(raw["turn"].as_str().unwrap().to_owned());
        }
        assert_eq!(turns,["local-one","local-two"]);
    }

    #[test]
    fn existing_capture_is_exclusive_and_reopen_preserves_unknown_turn_and_partial_bytes() {
        let f = Fixture::new();
        let mut journal = Journal::open(&f.0, f.identity(), Opening::Resume).unwrap();
        journal.commit_base().unwrap();
        journal
            .accept("unknown-turn", "accepted", &[], &[], None)
            .unwrap();
        assert!(
            Journal::open(&f.0, f.identity(), Opening::Resume)
                .err()
                .unwrap()
                .contains("writer")
        );
        drop(journal);
        let mut file = OpenOptions::new()
            .append(true)
            .open(f.current().join("records.jsonl"))
            .unwrap();
        file.write_all(b"{\"interrupted\":").unwrap();
        drop(file);
        let before = f.records();
        let journal = Journal::open(&f.0, f.identity(), Opening::Resume).unwrap();
        assert!(!journal.is_pending());
        let after = f.records();
        assert!(after.starts_with(&before));
        assert!(after.contains("\"interrupted_turn\":\"unknown-turn\""));
        assert!(after.contains("\"incomplete_record\":true"));
        let manifest = read_manifest(&f.current()).unwrap().unwrap();
        assert_eq!(manifest.gaps, 1);
        assert!(manifest.active_turn.is_none());
    }

    #[test]
    fn agent_namespace_and_manifest_are_not_inferred_from_a_directory() {
        let f = Fixture::new();
        let mut other = f.identity();
        other.agent = "acp:another".into();
        assert_ne!(other.directory(&f.0), f.identity().directory(&f.0));
        fs::create_dir_all(f.current()).unwrap();
        assert!(
            Journal::open(&f.0, f.identity(), Opening::Load)
                .err()
                .unwrap()
                .contains("manifest is missing")
        );
    }

    #[test]
    fn failed_acceptance_retires_the_writer_before_dispatch_is_allowed() {
        let f = Fixture::new();
        let mut journal = Journal::open(&f.0, f.identity(), Opening::New).unwrap();
        journal.commit_base().unwrap();
        fs::remove_file(f.current().join("manifest.json")).unwrap();
        fs::create_dir(f.current().join("manifest.json")).unwrap();
        assert!(
            journal
                .accept("turn", "received but not dispatched", &[], &[], None)
                .is_err()
        );
        assert!(
            journal
                .note(&serde_json::json!({"text":"must not continue"}))
                .unwrap_err()
                .contains("failed")
        );
    }

    #[test]
    fn manifest_read_has_a_metadata_budget_without_a_history_size_limit() {
        let f = Fixture::new();
        fs::create_dir_all(f.current()).unwrap();
        fs::write(
            f.current().join("manifest.json"),
            vec![b' '; MANIFEST_BYTES as usize + 1],
        )
        .unwrap();
        assert!(
            read_manifest(&f.current())
                .unwrap_err()
                .contains("byte budget")
        );
    }

    #[test]
    fn journal_parser_preserves_prompt_blocks_original_stop_and_unparsed_records() {
        use super::super::ContentEvent;
        let prompt = serde_json::json!({"kind":"prompt","turn":"turn","text":"words","images":[["image/png","pixels"]],"files":[["note.md","body"]]});
        let events = parse_record(&prompt.to_string());
        assert_eq!(events.len(), 3);
        assert!(
            matches!(&events[0],ContentEvent::Update { update, .. } if update["update"]["content"]["text"]=="words")
        );
        assert!(
            matches!(&events[1],ContentEvent::Update { update, .. } if update["update"]["content"]["data"]=="pixels")
        );
        assert!(
            matches!(&events[2],ContentEvent::Update { update, .. } if update["update"]["content"]["resource"]["text"]=="body")
        );
        assert!(matches!(&events[0],ContentEvent::Update { starts_message:true, .. }));
        assert!(events[1..].iter().all(|event|matches!(event,ContentEvent::Update { starts_message:false, .. })),
            "attachments belong to the same recorded prompt");
        let chunk = serde_json::json!({"kind":"update","notification":{"sessionId":"s","update":{
            "sessionUpdate":"agent_message_chunk","messageId":"provider-message",
            "content":{"type":"text","text":"same text"}}}});
        assert!(matches!(&parse_record(&chunk.to_string())[0],ContentEvent::Update { update, starts_message:false }
            if update==&chunk["notification"]),"SDK messageId and original payload stay intact");
        let turn = r#"{"kind":"turn","turn":"turn","outcome":"ended","detail":"MaxTokens"}"#;
        assert!(matches!(&parse_record(turn)[0],ContentEvent::Turn {stop} if stop=="MaxTokens"));
        for damaged in [
            "{\"interrupted\":",
            r#"{"kind":"prompt","text":"would lose attachments"}"#,
        ] {
            assert!(matches!(
                parse_record(damaged).as_slice(),
                [ContentEvent::Reference]
            ));
        }
    }

    #[test]
    fn reader_opens_committed_journal_without_becoming_another_writer() {
        let f = Fixture::new();
        let mut writer = Journal::open(&f.0, f.identity(), Opening::Load).unwrap();
        assert!(Source::find(&f.0, f.identity()).unwrap().is_none());
        writer.note(&serde_json::json!({"sessionId":"provider/full/id","update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"recorded"}}})).unwrap();
        writer.commit_base().unwrap();
        let source = Source::find(&f.0, f.identity()).unwrap().unwrap();
        let mut file = source.open().unwrap();
        let page = crate::adaptor::transcript_page::read_page(
            &mut file,
            &source.source_id(&crate::SessionId("alias".into())),
            None,
            parse_record,
            &|| false,
        )
        .unwrap();
        assert_eq!(page.events.len(), 1);
        assert_eq!(
            source.capture(),
            super::super::Capture {
                opening: "load",
                gaps: 0
            }
        );
        assert!(
            Journal::open(&f.0, f.identity(), Opening::Resume).is_err(),
            "reading must not release or replace the live writer"
        );
    }
}
