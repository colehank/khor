//! Read-only sources and Khor-owned observation identity; never an agent connection.
use crate::adaptor::{self, claude::Utterance};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

#[path = "session_content/reference.rs"]
mod reference;
pub use reference::{ContentLocator, ContentReference, ReadBinding};
#[path = "session_content/requests.rs"]
pub(crate) mod requests;
#[path = "session_content/remote.rs"]
pub mod remote;

#[path = "session_content/journal.rs"]
pub(crate) mod journal;
#[path = "session_content/observations.rs"]
mod observations;
pub use observations::refuse_control;
pub(crate) use observations::{ListedCandidate, Observation, ObservationKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Unavailable {
    NoSource,
    Unreadable,
    RemoteNotConnected,
    AmbiguousSource,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Availability {
    Available {
        following: bool,
        capture: Option<Capture>,
    },
    Unavailable(Unavailable),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Capture {
    pub opening: &'static str,
    pub gaps: u64,
}

pub enum ContentEvent {
    Native(Utterance),
    Update(serde_json::Value),
    Turn { stop: String },
    CaptureGap { opening: String, interrupted: bool },
    PromptOrigin { turn: String, token: String, start: u64, end: u64 },
    Reference,
}
impl From<Utterance> for ContentEvent {
    fn from(value: Utterance) -> Self {
        Self::Native(value)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Provider {
    Claude,
    Codex,
}
impl Provider {
    pub(crate) fn of(category: Option<&str>) -> Option<Self> {
        match category {
            Some("claude") => Some(Self::Claude),
            Some("codex") => Some(Self::Codex),
            _ => None,
        }
    }
    pub(crate) fn content_parser(self) -> fn(&str) -> Vec<ContentEvent> {
        match self {
            Self::Claude => |text| {
                adaptor::claude::utterances_of(text)
                    .into_iter()
                    .map(ContentEvent::Native)
                    .collect()
            },
            Self::Codex => |text| {
                adaptor::codex::utterances_of(text)
                    .into_iter()
                    .map(ContentEvent::Native)
                    .collect()
            },
        }
    }
}
const METADATA_BYTES: u64 = 16 * 1024;
fn read_metadata(path: &Path) -> Result<Option<Vec<u8>>, String> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.to_string()),
    };
    let mut bytes = Vec::new();
    file.take(METADATA_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() as u64 > METADATA_BYTES {
        return Err("observation metadata exceeds its byte budget".into());
    }
    Ok(Some(bytes))
}

pub(crate) fn registered(
    id: &crate::SessionId,
    directory: Option<&Path>,
) -> Result<Option<(Option<Provider>, String)>, String> {
    let Some(directory) = directory else {
        return Ok(None);
    };
    let Some(bytes) = read_metadata(&directory.join("meta.json"))? else {
        return Ok(None);
    };
    let meta: crate::live::Meta =
        serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    if !meta.id.is_empty() && meta.id != id.0 {
        return Err("session metadata belongs to another identity".into());
    }
    let leaf = meta
        .vendor_leaf
        .unwrap_or_else(|| id.0.strip_prefix("tui/").unwrap_or("").to_owned());
    Ok(Some((
        Provider::of(meta.category.as_deref()),
        crate::live::clean_leaf(&leaf),
    )))
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ObservationBinding {
    version: u8,
    home: String,
    session: String,
    provider: Provider,
    native_id: String,
}
fn write_metadata(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if bytes.len() as u64 > METADATA_BYTES {
        return Err("observation metadata exceeds its byte budget".into());
    }
    let previous = read_metadata(path)?;
    if previous.as_deref() == Some(bytes) {
        return Ok(());
    }
    std::fs::create_dir_all(path.parent().expect("binding has a parent"))
        .map_err(|error| error.to_string())?;
    let was = previous
        .as_deref()
        .map(crate::files::digest)
        .unwrap_or_default();
    crate::files::write_whole(
        path.to_str().ok_or("observation path is not UTF-8")?,
        bytes,
        &was,
    )
    .map(|_| ())
}

impl ObservationBinding {
    fn path(root: &Path, id: &crate::SessionId) -> PathBuf {
        root.join(".khor/observations")
            .join(format!("{}.json", blake3::hash(id.0.as_bytes()).to_hex()))
    }
    pub(crate) fn read(
        root: &Path,
        home: &str,
        id: &crate::SessionId,
    ) -> Result<Option<Self>, String> {
        let Some(bytes) = read_metadata(&Self::path(root, id))? else {
            return Ok(None);
        };
        let binding: Self = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
        if binding.version != 1
            || binding.home != home
            || binding.session != id.0
            || binding.native_id.is_empty()
        {
            return Err("observation binding belongs to another source or version".into());
        }
        Ok(Some(binding))
    }
    pub(crate) fn matches(&self, provider: Provider, leaf: &str) -> bool {
        self.provider == provider && crate::live::clean_leaf(&self.native_id) == leaf
    }
    pub(crate) fn resolve(&self, root: &Path) -> Result<Option<NativeTranscript>, Unavailable> {
        NativeTranscript::resolve(
            root,
            self.provider,
            &crate::live::clean_leaf(&self.native_id),
            Some(&self.native_id),
        )
    }
    pub(crate) fn save(
        root: &Path,
        home: &str,
        id: &crate::SessionId,
        source: &NativeTranscript,
    ) -> Result<(), String> {
        let binding = Self {
            version: 1,
            home: home.into(),
            session: id.0.clone(),
            provider: source.provider,
            native_id: source.native_id.clone(),
        };
        let bytes = serde_json::to_vec(&binding).map_err(|error| error.to_string())?;
        write_metadata(&Self::path(root, id), &bytes)
    }
}

pub(crate) type Parser = fn(&str) -> Vec<Utterance>;
#[derive(Debug)]
pub(crate) struct NativeTranscript {
    path: PathBuf,
    provider: Provider,
    native_id: String,
    parse: Parser,
}
impl NativeTranscript {
    pub(crate) fn resolve(
        root: &Path,
        provider: Provider,
        leaf: &str,
        exact: Option<&str>,
    ) -> Result<Option<Self>, Unavailable> {
        Self::resolve_in(
            adaptor::claude_home(root),
            adaptor::codex_home(root),
            provider,
            leaf,
            exact,
        )
    }
    fn resolve_in(
        claude_home: PathBuf,
        codex_home: PathBuf,
        provider: Provider,
        leaf: &str,
        exact: Option<&str>,
    ) -> Result<Option<Self>, Unavailable> {
        Ok(match provider {
            Provider::Claude => adaptor::claude::Claude::at(claude_home)
                .unique_transcript_path(leaf, exact)
                .map_err(|_| Unavailable::AmbiguousSource)?
                .map(|(path, native_id)| Self {
                    path,
                    provider,
                    native_id,
                    parse: adaptor::claude::utterances_of,
                }),
            Provider::Codex => {
                let codex = adaptor::codex::Codex::at(codex_home);
                let Some(native_id) = codex
                    .unique_session_id(leaf, exact)
                    .map_err(|_| Unavailable::AmbiguousSource)?
                else {
                    return Ok(None);
                };
                codex.rollout_path(&native_id).map(|path| Self {
                    path,
                    provider,
                    native_id,
                    parse: adaptor::codex::utterances_of,
                })
            }
        })
    }
    pub(crate) fn native_reference(&self) -> (Provider, &str) {
        (self.provider, &self.native_id)
    }
    pub(crate) fn source_id(&self, home: &str, id: &crate::SessionId) -> String {
        serde_json::to_string(&(1, home, &id.0, self.provider, &self.native_id))
            .expect("source identity serializes")
    }
    pub(crate) fn open(&self) -> std::io::Result<(File, Parser)> {
        let file = File::open(&self.path)?;
        if !file.metadata()?.is_file() {
            return Err(std::io::Error::other("transcript source is not a file"));
        }
        Ok((file, self.parse))
    }
    pub(crate) fn availability(&self) -> Availability {
        match self.open() {
            Ok(_) => Availability::Available {
                following: true,
                capture: None,
            },
            Err(_) => Availability::Unavailable(Unavailable::Unreadable),
        }
    }
}

pub(crate) enum ContentSource {
    Native(NativeTranscript),
    Journal(journal::Source),
}
impl ContentSource {
    pub fn observation_identity(&self) -> (khor_core::ObservationNamespace, &str, &str) {
        match self {
            Self::Native(source) => {
                let (provider, native) = source.native_reference();
                let agent = match provider {
                    Provider::Claude => "claude",
                    Provider::Codex => "codex",
                };
                (khor_core::ObservationNamespace::Native, agent, native)
            }
            Self::Journal(source) => {
                let identity = source.identity();
                (
                    khor_core::ObservationNamespace::Acp,
                    &identity.agent,
                    &identity.native_id,
                )
            }
        }
    }
    pub fn source_id(&self, home: &str, id: &crate::SessionId) -> String {
        match self {
            Self::Native(source) => source.source_id(home, id),
            Self::Journal(source) => source.source_id(id),
        }
    }
    pub fn availability(&self) -> Availability {
        match self {
            Self::Native(source) => source.availability(),
            Self::Journal(source) => match source.open() {
                Ok(_) => Availability::Available {
                    following: true,
                    capture: Some(source.capture()),
                },
                Err(_) => Availability::Unavailable(Unavailable::Unreadable),
            },
        }
    }
    pub fn open(&self) -> std::io::Result<(File, fn(&str) -> Vec<ContentEvent>)> {
        match self {
            Self::Native(source) => {
                let (file, _) = source.open()?;
                let parse = source.provider.content_parser();
                Ok((file, parse))
            }
            Self::Journal(source) => Ok((source.open()?, journal::parse_record)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, UNIX_EPOCH};
    const FIRST: &str = "01a17c1a-a542-7822-b01d-82a832a158e6";
    const SECOND: &str = "01a17c1a-a542-7822-b01d-82a832a158e7";
    const CLAUDE: &str =
        r#"{"type":"assistant","message":{"content":[{"type":"text","text":"claude"}]}}"#;
    const CODEX: &str =
        r#"{"type":"event_msg","payload":{"type":"agent_message","message":"codex"}}"#;
    struct Fixture(PathBuf);
    impl Fixture {
        fn new(name: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("khor-content-{name}-{}", std::process::id()));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
        fn claude(&self, project: &str, native: &str, text: &str) -> PathBuf {
            let path = self
                .0
                .join(format!(".claude/projects/{project}/{native}.jsonl"));
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, format!("{text}\n")).unwrap();
            path
        }
        fn codex(&self, native: &str) {
            let path = self.0.join(format!(
                ".codex/sessions/2026/10/29/rollout-2026-10-29T12-00-00-{native}.jsonl"
            ));
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, format!("{CODEX}\n")).unwrap();
        }
        fn resolve(
            &self,
            provider: Provider,
            exact: Option<&str>,
        ) -> Result<Option<NativeTranscript>, Unavailable> {
            NativeTranscript::resolve_in(
                self.0.join(".claude"),
                self.0.join(".codex"),
                provider,
                &crate::live::clean_leaf(FIRST),
                exact,
            )
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn provider_namespace_prevents_cross_vendor_leaf_collision() {
        let fixture = Fixture::new("namespace");
        fixture.claude("a", FIRST, CLAUDE);
        fixture.codex(FIRST);
        for (provider, line) in [(Provider::Claude, CLAUDE), (Provider::Codex, CODEX)] {
            let source = fixture.resolve(provider, None).unwrap().unwrap();
            assert_eq!(source.provider, provider);
            assert_eq!(source.native_id, FIRST);
            assert_eq!(
                source.availability(),
                Availability::Available {
                    following: true,
                    capture: None
                }
            );
            assert_eq!((source.open().unwrap().1)(line).len(), 1);
        }
    }
    #[test]
    fn distinct_native_ids_sharing_a_leaf_are_ambiguous_for_both_providers() {
        let fixture = Fixture::new("ambiguous");
        fixture.claude("a", FIRST, CLAUDE);
        fixture.claude("a", SECOND, CLAUDE);
        fixture.codex(FIRST);
        fixture.codex(SECOND);
        assert_eq!(
            crate::live::clean_leaf(FIRST),
            crate::live::clean_leaf(SECOND)
        );
        for provider in [Provider::Claude, Provider::Codex] {
            assert_eq!(
                fixture.resolve(provider, None).unwrap_err(),
                Unavailable::AmbiguousSource
            );
            assert_eq!(
                fixture
                    .resolve(provider, Some(FIRST))
                    .unwrap()
                    .unwrap()
                    .native_id,
                FIRST
            );
        }
    }
    #[test]
    fn duplicate_backups_of_one_native_id_keep_the_newest_path() {
        let fixture = Fixture::new("backup");
        let old = fixture.claude("old", FIRST, CLAUDE);
        let latest = fixture.claude("new", FIRST, CLAUDE);
        for (path, seconds) in [(&old, 10), (&latest, 20)] {
            File::open(path)
                .unwrap()
                .set_times(
                    std::fs::FileTimes::new()
                        .set_modified(UNIX_EPOCH + Duration::from_secs(seconds)),
                )
                .unwrap();
        }
        assert_eq!(
            fixture
                .resolve(Provider::Claude, None)
                .unwrap()
                .unwrap()
                .path,
            latest
        );
    }
    #[test]
    fn probe_does_not_parse_and_unreadable_does_not_change_namespace() {
        let fixture = Fixture::new("probe");
        let path = fixture.claude("a", FIRST, "unfinished invalid JSON");
        let source = fixture.resolve(Provider::Claude, None).unwrap().unwrap();
        assert_eq!(
            source.availability(),
            Availability::Available {
                following: true,
                capture: None
            }
        );
        std::fs::remove_file(path).unwrap();
        assert_eq!(
            source.availability(),
            Availability::Unavailable(Unavailable::Unreadable)
        );
    }
    #[test]
    fn unknown_recorded_agent_and_foreign_metadata_cannot_borrow_a_native_leaf() {
        let fixture = Fixture::new("unknown");
        fixture.claude("a", FIRST, CLAUDE);
        let id = crate::SessionId(format!("tui/{}", crate::live::clean_leaf(FIRST)));
        let meta = serde_json::json!({"kind":"gui","title":"fixture","pid":null,"started_ms":0,"id":id.0,"category":"other-agent","vendor_leaf":FIRST,"place":null});
        std::fs::write(fixture.0.join("meta.json"), meta.to_string()).unwrap();
        assert_eq!(registered(&id, Some(&fixture.0)).unwrap().unwrap().0, None);
        assert!(registered(&crate::SessionId("tui/foreign".into()), Some(&fixture.0)).is_err());
    }
    #[test]
    fn binding_survives_node_reopen_and_source_change_resets_page_not_raw_reference() {
        let fixture = Fixture::new("reopen");
        fixture.claude("a", FIRST, CLAUDE);
        fixture.codex(FIRST);
        let id = crate::SessionId(format!("tui/{}", crate::live::clean_leaf(FIRST)));
        let node = crate::Node::open_hosting(fixture.0.clone(), "content-test", false).unwrap();
        let home = node.device_str().to_owned();
        let source = fixture.resolve(Provider::Claude, None).unwrap().unwrap();
        ObservationBinding::save(&fixture.0, &home, &id, &source).unwrap();
        drop(node);
        let node = crate::Node::open_hosting(fixture.0.clone(), "content-test", false).unwrap();
        assert_eq!(
            node.session_content(&id, None).unwrap(),
            Availability::Available {
                following: true,
                capture: None
            }
        );
        let cancelled = std::sync::atomic::AtomicBool::new(false);
        let old = node.transcript_page(&id, None, false, &cancelled).unwrap();
        assert_eq!(old.events.len(), 1);
        let record = old.cursor_before(old.events[0].position.clone());
        let from = old.events[0].position.start;
        assert!(
            !node
                .transcript_record(&id, &record, from, &cancelled)
                .unwrap()
                .bytes
                .is_empty()
        );
        let source = fixture.resolve(Provider::Codex, None).unwrap().unwrap();
        ObservationBinding::save(&fixture.0, &home, &id, &source).unwrap();
        let replaced = node
            .transcript_page(&id, Some(&old.newer), true, &cancelled)
            .unwrap();
        assert!(replaced.reset);
        assert_eq!(replaced.events.len(), 1);
        assert_eq!(
            node.transcript_record(&id, &record, from, &cancelled)
                .err()
                .as_deref(),
            Some("history record changed; reload its page")
        );
        assert!(ObservationBinding::read(&fixture.0, "another-home", &id).is_err());
    }
}

#[cfg(test)]
#[path = "session_content/wire_tests.rs"]
pub(crate) mod wire_tests;
