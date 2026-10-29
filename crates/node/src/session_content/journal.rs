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

impl Journal {
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
}
