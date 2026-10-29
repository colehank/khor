use super::{Provider, read_metadata, write_metadata};
use crate::{KindSurface, Node};
use khor_core::{
    DeviceId, Kind, Millis, ObservationNamespace, ObservationRef, Session, SessionId, kind,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub(crate) const PREFIX: &str = "observe/";
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Observation {
    version: u8,
    pub id: SessionId,
    pub reference: ObservationRef,
    pub title: String,
    pub cwd: Option<String>,
}
impl Observation {
    pub fn id(
        home: DeviceId,
        namespace: ObservationNamespace,
        agent: &str,
        native: &str,
    ) -> SessionId {
        let tuple = serde_json::to_vec(&(1, home, (namespace, agent), native))
            .expect("source identity serializes");
        SessionId(format!("{PREFIX}{}", blake3::hash(&tuple).to_hex()))
    }
    pub fn new(
        owner: DeviceId,
        home: DeviceId,
        namespace: ObservationNamespace,
        agent: String,
        native: String,
        title: String,
        cwd: Option<String>,
        updated_at: Option<Millis>,
    ) -> Self {
        Self {
            version: 1,
            id: Self::id(home, namespace, &agent, &native),
            reference: ObservationRef {
                owner,
                home,
                namespace,
                agent,
                native_id: native,
                imported_at: Millis(crate::live::now_ms().max(0) as u64),
                updated_at,
            },
            title,
            cwd,
        }
    }
    fn path(root: &Path, id: &SessionId) -> PathBuf {
        root.join(".khor/observations").join(format!(
            "{}.observe.json",
            blake3::hash(id.0.as_bytes()).to_hex()
        ))
    }
    pub fn read(root: &Path, id: &SessionId) -> Result<Option<Self>, String> {
        if !id.0.starts_with(PREFIX) {
            return Ok(None);
        }
        let Some(bytes) = read_metadata(&Self::path(root, id))? else {
            return Ok(None);
        };
        let record: Self = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
        record.validate(id)?;
        Ok(Some(record))
    }
    fn validate(&self, id: &SessionId) -> Result<(), String> {
        if self.version != 1
            || self.id != *id
            || self.reference.agent.is_empty()
            || self.reference.native_id.is_empty()
            || Self::id(
                self.reference.home,
                self.reference.namespace,
                &self.reference.agent,
                &self.reference.native_id,
            ) != *id
        {
            return Err("observation record belongs to another source or version".into());
        }
        Ok(())
    }
    pub fn save(&self, root: &Path) -> Result<(), String> {
        self.validate(&self.id)?;
        let bytes = serde_json::to_vec(self).map_err(|error| error.to_string())?;
        write_metadata(&Self::path(root, &self.id), &bytes)
    }
    pub fn remove(root: &Path, id: &SessionId) -> Result<(), String> {
        match std::fs::remove_file(Self::path(root, id)) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.to_string()),
        }
    }
    pub fn all(root: &Path) -> Result<Vec<Self>, String> {
        let entries = match std::fs::read_dir(root.join(".khor/observations")) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(error.to_string()),
        };
        let mut records = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|error| error.to_string())?;
            if !entry
                .file_name()
                .to_string_lossy()
                .ends_with(".observe.json")
            {
                continue;
            }
            let Some(bytes) = read_metadata(&entry.path())? else {
                continue;
            };
            let record: Self = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
            record.validate(&record.id)?;
            if Self::path(root, &record.id) != entry.path() {
                return Err("observation metadata path does not match its identity".into());
            }
            records.push(record);
        }
        Ok(records)
    }
    pub fn session(&self) -> Session {
        Session {
            id: self.id.clone(),
            kind: Kind(kind::OBSERVE.into()),
            title: self.title.clone(),
            home: self.reference.home,
            state: None,
            unread: 0,
            category: Some(self.reference.agent.clone()),
            last: None,
            term: false,
            via: None,
            dead: false,
            unbacked: false,
            place: self.cwd.clone(),
            mux: false,
            doing: None,
            observation: Some(self.reference.clone()),
            can_control: false,
        }
    }
    pub fn native_provider(&self) -> Option<Provider> {
        (self.reference.namespace == ObservationNamespace::Native)
            .then(|| Provider::of(Some(&self.reference.agent)))
            .flatten()
    }
}

pub(crate) struct ObservationKind;
impl KindSurface for ObservationKind {
    fn claims(&self, _node: &Node, id: &SessionId) -> bool {
        id.0.starts_with(PREFIX)
    }
    fn rows(&self, node: &Node) -> Result<Vec<Session>, String> {
        Ok(Observation::all(node.root())?
            .into_iter()
            .map(|record| record.session())
            .collect())
    }
    fn reportable(&self) -> bool {
        true
    }
    fn seen_at(&self, node: &Node, id: &SessionId) -> Result<i64, String> {
        let record = Observation::read(node.root(), id)?.ok_or("observation record is missing")?;
        Ok(record
            .reference
            .updated_at
            .unwrap_or(record.reference.imported_at)
            .0
            .min(i64::MAX as u64) as i64)
    }
    fn close(&self, node: &Node, id: &SessionId) -> Result<(), String> {
        Observation::remove(node.root(), id)
    }
}

pub fn refuse_control(id: &SessionId) -> Result<(), String> {
    if id.0.starts_with(PREFIX) {
        Err(
            "This is a read-only observation; resume explicitly to control the original session"
                .into(),
        )
    } else {
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ListedCandidate {
    pub home: DeviceId,
    pub agent: String,
    pub native_id: String,
}
impl ListedCandidate {
    pub fn encode(&self) -> String {
        use base64::Engine;
        let bytes = serde_json::to_vec(self).expect("candidate reference serializes");
        format!(
            "{}{}",
            crate::ACP_CANDIDATE,
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
        )
    }
    pub fn parse(text: &str) -> Result<Self, String> {
        use base64::Engine;
        let encoded = text
            .strip_prefix(crate::ACP_CANDIDATE)
            .ok_or("not an ACP observation candidate")?;
        if encoded.len() > 16 * 1024 {
            return Err("candidate reference exceeds its byte budget".into());
        }
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(encoded)
            .map_err(|error| error.to_string())?;
        let candidate: Self = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
        if candidate.agent.is_empty() || candidate.native_id.is_empty() {
            return Err("candidate identity is empty".into());
        }
        Ok(candidate)
    }
}

impl Node {
    pub(crate) fn import_listed_observation(&self, id: &str) -> Result<SessionId, String> {
        let candidate = ListedCandidate::parse(id)?;
        let Some(crate::AgentAnswer::Sessions {
            home: Some(home),
            rows,
        }) = self.agent_seen(&candidate.agent)?
        else {
            return Err(
                "The session list has no verified source machine; refresh it before importing"
                    .into(),
            );
        };
        let home = DeviceId(home);
        if home != candidate.home {
            return Err("The session list source changed; refresh before importing".into());
        }
        let seen = rows
            .into_iter()
            .find(|row| row.id == candidate.native_id)
            .ok_or("The listed session is no longer in this source snapshot")?;
        let updated = seen
            .updated_at
            .as_deref()
            .and_then(|value| value.parse::<jiff::Timestamp>().ok())
            .and_then(|time| u64::try_from(time.as_millisecond()).ok())
            .map(Millis);
        self.register_observation(Observation::new(
            self.device(),
            home,
            ObservationNamespace::Acp,
            candidate.agent,
            candidate.native_id,
            seen.title.unwrap_or_default(),
            (!seen.cwd.is_empty()).then_some(seen.cwd),
            updated,
        ))
    }

    pub(crate) fn import_observed_row(&self, row: &Session) -> Result<SessionId, String> {
        let reference = row.observation.as_ref().ok_or("not an observation row")?;
        let mut record = Observation::new(
            self.device(),
            reference.home,
            reference.namespace,
            reference.agent.clone(),
            reference.native_id.clone(),
            row.title.clone(),
            row.place.clone(),
            reference.updated_at,
        );
        record.reference.imported_at = reference.imported_at;
        self.register_observation(record)
    }

    pub(crate) fn import_native_observation(&self, row: &Session) -> Result<SessionId, String> {
        if row.home != self.device() {
            return Err(
                "The full native identity must be resolved on its content home before importing"
                    .into(),
            );
        }
        let source = self
            .selected_content(&row.id, true)
            .map_err(|reason| format!("content source unavailable: {reason:?}"))?
            .ok_or("No verified native identity is available for this agent session")?;
        let (provider, native) = source.native_reference();
        let agent = match provider {
            Provider::Claude => "claude",
            Provider::Codex => "codex",
        };
        self.register_observation(Observation::new(
            self.device(),
            self.device(),
            ObservationNamespace::Native,
            agent.to_owned(),
            native.to_owned(),
            row.title.clone(),
            row.place.clone(),
            row.activity_at(),
        ))
    }

    pub(crate) fn register_observation(&self, record: Observation) -> Result<SessionId, String> {
        record.save(&self.root)?;
        let loaded = self.roster_loaded()?;
        loaded
            .doc
            .put(&record.id.0, khor_sync::roster::Entry::Import)?;
        let mut store = loaded.store;
        store.flush(&loaded.doc)?;
        self.emit(crate::NodeEvent::Row(record.session()));
        Ok(record.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture(PathBuf);
    impl Fixture {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir()
                .join(format!("khor-observation-{name}-{}", std::process::id()));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
        fn node(&self) -> Node {
            Node::open_hosting(self.0.clone(), "observation-test", false).unwrap()
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn remember(node: &Node, home: Option<DeviceId>, native: &str) {
        let loaded = node.agents_loaded().unwrap();
        loaded
            .doc
            .set_seen(
                "fixture-agent",
                &crate::AgentAnswer::Sessions {
                    home: home.map(|home| home.0),
                    rows: vec![crate::AgentSeen {
                        id: native.to_owned(),
                        cwd: "/fixture/project".into(),
                        title: Some("Saved conversation".into()),
                        updated_at: None,
                    }],
                },
            )
            .unwrap();
        let mut store = loaded.store;
        store.flush(&loaded.doc).unwrap();
    }
    #[test]
    fn importing_observes_exact_home_without_starting_or_controlling_an_agent() {
        let fixture = Fixture::new("import");
        let node = fixture.node();
        node.register_agent("fixture-agent", &["uninstalled-observation-fixture".into()])
            .unwrap();
        let home = DeviceId([42; 32]);
        let native = "native/session:with complete identity";
        remember(&node, Some(home), native);
        let candidate = node
            .import_candidates()
            .unwrap()
            .into_iter()
            .find(|row| row.agent.as_deref() == Some("fixture-agent"))
            .unwrap();
        let alias = node.import(&candidate.id).unwrap();
        assert_eq!(
            alias,
            Observation::id(home, ObservationNamespace::Acp, "fixture-agent", native)
        );
        assert_ne!(alias, crate::adaptor::id_for(native));
        let record = Observation::read(&fixture.0, &alias).unwrap().unwrap();
        assert_eq!(record.reference.home, home);
        assert_eq!(record.reference.owner, node.device());
        assert_eq!(record.reference.native_id, native);
        let other_owner = DeviceId([55; 32]);
        let copy = Observation::new(
            other_owner,
            home,
            ObservationNamespace::Acp,
            "fixture-agent".into(),
            native.into(),
            "Other metadata owner".into(),
            None,
            None,
        );
        assert_eq!(copy.id, alias);
        let devices = node.devices_loaded().unwrap();
        devices
            .doc
            .upsert(&other_owner.hex(), "other-observer", &[])
            .unwrap();
        let mut store = devices.store;
        store.flush(&devices.doc).unwrap();
        node.cache_peer_rows(&other_owner.hex(), "other-observer", &[copy.session()])
            .unwrap();
        assert!(
            node.cached_peer_rows()
                .iter()
                .any(|(name, _, row)| name == "other-observer" && row.id == alias),
            "the duplicate is present in the actual peer input, not filtered as an unknown device"
        );
        let listed = node.sessions().unwrap();
        assert_eq!(
            listed
                .iter()
                .filter(|view| view.session.id == alias)
                .count(),
            1
        );
        assert_eq!(
            listed
                .iter()
                .find(|view| view.session.id == alias)
                .unwrap()
                .session
                .observation
                .as_ref()
                .unwrap()
                .owner,
            node.device()
        );
        let row = node
            .sessions()
            .unwrap()
            .into_iter()
            .find(|view| view.session.id == alias)
            .unwrap()
            .session;
        assert_eq!(row.state, None);
        assert!(!row.can_control && !row.term && !row.mux);
        assert_eq!(row.activity_at(), Some(record.reference.imported_at));
        assert!(
            !crate::live::sessions_dir(&fixture.0).exists(),
            "import must not create an agent host"
        );
        assert!(node.takeover(&alias).unwrap_err().contains("read-only"));
        assert!(
            node.takeover_term(&alias)
                .unwrap_err()
                .contains("read-only")
        );
        assert!(node.attach_tmux(&alias).unwrap_err().contains("read-only"));
        assert!(
            node.report_state(&alias, khor_core::State::Busy)
                .unwrap_err()
                .contains("read-only")
        );
        assert!(
            node.run_ephemeral(&alias, &["uninstalled-observation-fixture".into()])
                .unwrap_err()
                .contains("read-only")
        );
        remember(&node, Some(DeviceId([43; 32])), "another session");
        assert_eq!(
            Observation::read(&fixture.0, &alias)
                .unwrap()
                .unwrap()
                .reference,
            record.reference
        );
        let source = fixture.0.join("external-writer-record");
        std::fs::write(&source, "retained").unwrap();
        node.close(&alias).unwrap();
        assert!(Observation::read(&fixture.0, &alias).unwrap().is_none());
        assert_eq!(std::fs::read_to_string(&source).unwrap(), "retained");
        assert!(
            node.sessions()
                .unwrap()
                .iter()
                .all(|view| view.session.id != alias)
        );
    }
    #[test]
    fn native_agent_import_creates_an_observer_without_adopting_its_terminal() {
        let fixture = Fixture::new("native-import");
        let native = "01a17c1a-a542-7822-b01d-82a832a158e6";
        let transcript = fixture
            .0
            .join(format!(".claude/projects/fixture/{native}.jsonl"));
        std::fs::create_dir_all(transcript.parent().unwrap()).unwrap();
        std::fs::create_dir_all(fixture.0.join(".codex")).unwrap();
        std::fs::write(
            &transcript,
            concat!(
                r#"{"type":"assistant","message":{"content":[{"type":"text","text":"retained"}]}}"#,
                "\n"
            ),
        )
        .unwrap();
        let node = Node::open_hosting(fixture.0.clone(), "observation-test", true).unwrap();
        let original = crate::adaptor::id_for(native);
        node.live
            .register(
                &original,
                kind::TUI,
                "External Claude",
                None,
                Some("claude"),
            )
            .unwrap();
        let candidate = node
            .import_candidates()
            .unwrap()
            .into_iter()
            .find(|row| row.id == original.0)
            .unwrap();
        assert_eq!(candidate.form, kind::OBSERVE);
        let alias = node.import(&candidate.id).unwrap();
        assert_ne!(alias, original);
        let reference = Observation::read(&fixture.0, &alias)
            .unwrap()
            .unwrap()
            .reference;
        assert_eq!(reference.namespace, ObservationNamespace::Native);
        assert_eq!(reference.native_id, native);
        assert_eq!(reference.home, node.device());
        assert_eq!(
            node.session_content(&alias, None).unwrap(),
            super::super::Availability::Available { following: true }
        );
        let page = node
            .transcript_page(
                &alias,
                None,
                false,
                &std::sync::atomic::AtomicBool::new(false),
            )
            .unwrap();
        assert!(
            matches!(page.events.as_slice(), [crate::adaptor::transcript_page::Event { utterance: crate::adaptor::claude::Utterance::Agent(text), .. }] if text == "retained")
        );
        assert!(
            node.require_interactive_owner(&original)
                .unwrap_err()
                .contains("outside Khor")
        );
        node.close(&alias).unwrap();
        assert!(
            node.live.on_record(&original).is_some(),
            "deleting observation leaves the hook's record"
        );
        assert!(
            transcript.exists(),
            "deleting observation preserves vendor history"
        );
        assert!(!crate::host::host_file_path(&node.live.dir_of(&original).unwrap()).exists());
    }

    #[test]
    fn unknown_and_changed_list_home_cannot_be_imported_as_local() {
        let fixture = Fixture::new("home");
        let node = fixture.node();
        node.register_agent("fixture-agent", &["uninstalled-observation-fixture".into()])
            .unwrap();
        remember(&node, None, "exact-native");
        assert!(
            node.import_candidates()
                .unwrap()
                .iter()
                .all(|row| row.agent.is_none())
        );
        let unknown = ListedCandidate {
            home: node.device(),
            agent: "fixture-agent".into(),
            native_id: "exact-native".into(),
        }
        .encode();
        assert!(
            node.import(&unknown)
                .unwrap_err()
                .contains("verified source machine")
        );
        remember(&node, Some(DeviceId([42; 32])), "exact-native");
        let old = node
            .import_candidates()
            .unwrap()
            .into_iter()
            .find(|row| row.agent.is_some())
            .unwrap()
            .id;
        remember(&node, Some(DeviceId([43; 32])), "exact-native");
        assert!(node.import(&old).unwrap_err().contains("source changed"));
        assert!(Observation::all(&fixture.0).unwrap().is_empty());
    }
    #[test]
    fn identity_uses_unambiguous_namespace_and_full_native_id() {
        let home = DeviceId([7; 32]);
        assert_ne!(
            Observation::id(home, ObservationNamespace::Native, "claude", "same"),
            Observation::id(home, ObservationNamespace::Acp, "claude", "same")
        );
        assert_ne!(
            Observation::id(home, ObservationNamespace::Acp, "a:b", "c"),
            Observation::id(home, ObservationNamespace::Acp, "a", "b:c")
        );
        assert_ne!(
            Observation::id(home, ObservationNamespace::Acp, "agent", "same:a"),
            Observation::id(home, ObservationNamespace::Acp, "agent", "same?a")
        );
        let native_literal = "observe/provider-native-id";
        let encoded = ListedCandidate {
            home,
            agent: "agent".into(),
            native_id: native_literal.into(),
        }
        .encode();
        assert_eq!(
            ListedCandidate::parse(&encoded).unwrap().native_id,
            native_literal
        );
        assert_eq!(
            Observation::id(home, ObservationNamespace::Acp, "agent", "same"),
            Observation::new(
                DeviceId([8; 32]),
                home,
                ObservationNamespace::Acp,
                "agent".into(),
                "same".into(),
                String::new(),
                None,
                None
            )
            .id
        );
    }
}
