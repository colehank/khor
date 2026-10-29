//! One host writes a capture; readers consume only its published current directory.
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
    _lock: File,
}

#[derive(Debug)]
pub(crate) struct ReadBoundary {
    pub binding: super::ReadBinding,
    pub cursor: crate::adaptor::transcript_page::Cursor,
}

impl Journal {
    pub fn bind(&self, directory: &Path) -> Result<(), String> {
        if let Some(bytes) = super::read_metadata(&directory.join("journal.json"))? {
            let identity: Identity = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            if identity != self.manifest.identity {
                return Err("session already names another journal source".into());
            }
        }
        let bytes = serde_json::to_vec(&self.manifest.identity).map_err(|e| e.to_string())?;
        super::write_metadata(&directory.join("journal.json"), &bytes)
    }
    pub fn open(root: &Path, identity: Identity, opening: Opening) -> Result<Self, String> {
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
        let pending = directory.join("pending");
        match fs::remove_dir_all(&pending) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.to_string()),
        }
        let current = directory.join("current");
        let manifest = read_manifest(&current)?;
        let mut journal = if let Some(manifest) = manifest {
            if manifest.version != VERSION || manifest.identity != identity {
                return Err("journal manifest belongs to another source or version".into());
            }
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
                _lock: lock,
            }
        } else {
            // A published directory without its manifest is corrupt, not an empty capture.
            if current.exists() {
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
                },
                pending: true,
                poisoned: false,
                _lock: lock,
            }
        };
        if !journal.pending {
            let incomplete_record = journal.finish_partial_record()?;
            let interrupted = journal.manifest.active_turn.take();
            let gap: Record<'_, ()> = Record::Gap {
                opening,
                interrupted_turn: interrupted.as_deref(),
                incomplete_record,
            };
            journal.append(&gap)?;
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
        let binding = ReadBinding::new(view, ContentReference {
            home: self.manifest.identity.home,
            namespace: khor_core::ObservationNamespace::Acp,
            agent: self.manifest.identity.agent.clone(),
            native_id: self.manifest.identity.native_id.clone(),
        })?;
        let mut published = File::open(self.directory.join("current/records.jsonl"))
            .map_err(|error| error.to_string())?;
        let cursor = crate::adaptor::transcript_page::capture_eof(
            &self.file, &mut published, &binding.source_id(),
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
        if !self.pending {
            return Err("journal base is already committed".into());
        }
        self.ensure_writable()?;
        let result = (|| {
            self.file.sync_all().map_err(|e| e.to_string())?;
            self.persist()?;
            sync_directory(&self.directory.join("pending"))?;
            fs::rename(
                self.directory.join("pending"),
                self.directory.join("current"),
            )
            .map_err(|e| e.to_string())?;
            self.pending = false;
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
        let prompt: Record<'_, ()> = Record::Prompt {
            turn,
            text,
            images,
            files,
        };
        self.append(&prompt)?;
        self.manifest.active_turn = Some(turn.to_owned());
        self.persist()
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
        self.ensure_writable()?;
        let result = (|| {
            let mut writer = BufWriter::with_capacity(64 * 1024, &mut self.file);
            serde_json::to_writer(&mut writer, record).map_err(io::Error::other)?;
            writer.write_all(b"\n")?;
            writer.flush()
        })();
        if let Err(error) = result {
            self.poisoned = true;
            return Err(format!(
                "journal append failed; capture must retire: {error}"
            ));
        }
        Ok(())
    }

    fn persist(&mut self) -> Result<(), String> {
        self.ensure_writable()?;
        let result = (|| {
            self.file.sync_data().map_err(|e| e.to_string())?;
            let bytes = serde_json::to_vec(&self.manifest).map_err(|e| e.to_string())?;
            let folder = self
                .directory
                .join(if self.pending { "pending" } else { "current" });
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
    path: PathBuf,
    manifest: Manifest,
}
impl Source {
    pub fn identity(&self) -> &Identity {
        &self.manifest.identity
    }
    pub fn find(root: &Path, identity: Identity) -> Result<Option<Self>, String> {
        identity.validate()?;
        let current = identity.directory(root).join("current");
        let Some(manifest) = read_manifest(&current)? else {
            return if current.exists() {
                Err("committed journal manifest is missing".into())
            } else {
                Ok(None)
            };
        };
        if manifest.version != VERSION || manifest.identity != identity {
            return Err("journal source identity changed".into());
        }
        Ok(Some(Self {
            path: current.join("records.jsonl"),
            manifest,
        }))
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
            vec![ContentEvent::Update(notification)]
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
            let mut updates = vec![ContentEvent::Update(
                serde_json::json!({"update":{"sessionUpdate":"user_message_chunk","content":{"type":"text","text":text}}}),
            )];
            if let Some(images) = object.get("images").and_then(|v| v.as_array()) {
                for image in images {
                    let (Some(mime), Some(data)) = (
                        image.get(0).and_then(|v| v.as_str()),
                        image.get(1).and_then(|v| v.as_str()),
                    ) else {
                        return vec![ContentEvent::Reference];
                    };
                    updates.push(ContentEvent::Update(serde_json::json!({"update":{"sessionUpdate":"user_message_chunk","content":{"type":"image","mimeType":mime,"data":data}}})));
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
                    updates.push(ContentEvent::Update(serde_json::json!({"update":{"sessionUpdate":"user_message_chunk","content":{"type":"resource","resource":{"uri":format!("file://{name}"),"text":text}}}})));
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
                .accept("early", "not sent", &[], &[])
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
    fn existing_capture_is_exclusive_and_reopen_preserves_unknown_turn_and_partial_bytes() {
        let f = Fixture::new();
        let mut journal = Journal::open(&f.0, f.identity(), Opening::Resume).unwrap();
        journal.commit_base().unwrap();
        journal
            .accept("unknown-turn", "accepted", &[], &[])
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
                .accept("turn", "received but not dispatched", &[], &[])
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
            matches!(&events[0],ContentEvent::Update(update) if update["update"]["content"]["text"]=="words")
        );
        assert!(
            matches!(&events[1],ContentEvent::Update(update) if update["update"]["content"]["data"]=="pixels")
        );
        assert!(
            matches!(&events[2],ContentEvent::Update(update) if update["update"]["content"]["resource"]["text"]=="body")
        );
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
