//! Read selectors carry content home and exact identity, never metadata ownership.
use khor_core::{DeviceId, ObservationNamespace, ObservationRef, Session, SessionId};
use serde::{Deserialize, Serialize};

const VERSION: u8 = 1;
pub const METADATA_BYTES: usize = 16 * 1024;
const ID_BYTES: usize = 4096;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentReference {
    pub home: DeviceId,
    pub namespace: ObservationNamespace,
    pub agent: String,
    pub native_id: String,
}
impl ContentReference {
    pub fn body_projection(&self, scaffold: &str) -> Result<super::BodyProjection, String> {
        if scaffold.len() > 64 * 1024 {
            return Err("history projection scaffold exceeds its byte budget".into());
        }
        let parser = self.parser()?;
        // Vendor readers consume JSONL. Compact one validated scaffold before
        // passing it to that exact reader; pretty caller whitespace is not a
        // second transcript record, and malformed input cannot claim coverage.
        let compact = {
            let Ok(value) = serde_json::from_str::<serde_json::Value>(scaffold) else {
                return Ok(super::BodyProjection { updates: Vec::new(), complete: false });
            };
            serde_json::to_string(&value).map_err(|error| error.to_string())?
        };
        let events = parser(&compact);
        let mut updates = Vec::new();
        let mut complete = !events.is_empty();
        for event in events {
            use super::ContentEvent;
            match event {
                ContentEvent::Native(utterance) => {
                    if let Some(update) = crate::replayed_update(utterance) {
                        updates.push(serde_json::json!({ "update": update }));
                    } else { complete = false; }
                }
                ContentEvent::Update(update) => updates.push(update),
                ContentEvent::LocalUserEcho | ContentEvent::Turn { .. } => {}
                // A scaffold cannot elevate an origin, gap or arbitrary raw
                // record into a trusted system fact on the viewing client.
                ContentEvent::Reference
                | ContentEvent::PromptOrigin { .. }
                | ContentEvent::CaptureGap { .. } => complete = false,
            }
        }
        Ok(super::BodyProjection { updates, complete })
    }
    pub fn observed(reference: &ObservationRef) -> Self {
        Self {
            home: reference.home,
            namespace: reference.namespace,
            agent: reference.agent.clone(),
            native_id: reference.native_id.clone(),
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        identifier(&self.agent)?;
        identifier(&self.native_id)?;
        if self.namespace == ObservationNamespace::Native
            && super::Provider::of(Some(&self.agent)).is_none()
        {
            return Err("unknown native content provider".into());
        }
        encode(self).map(|_| ())
    }
    pub(crate) fn parser(&self) -> Result<fn(&str) -> Vec<super::ContentEvent>, String> {
        self.validate()?;
        match self.namespace {
            ObservationNamespace::Native => super::Provider::of(Some(&self.agent))
                .map(super::Provider::content_parser)
                .ok_or_else(|| "unknown native content provider".into()),
            ObservationNamespace::Acp => Ok(super::journal::parse_record),
        }
    }
    pub(crate) fn resolve(
        &self,
        root: &std::path::Path,
        actual_home: DeviceId,
    ) -> Result<Option<super::ContentSource>, super::Unavailable> {
        use super::{ContentSource, NativeTranscript, Provider, Unavailable, journal};
        self.validate().map_err(|_| Unavailable::Unreadable)?;
        if self.home != actual_home {
            return Err(Unavailable::RemoteNotConnected);
        }
        match self.namespace {
            ObservationNamespace::Native => {
                let provider = Provider::of(Some(&self.agent)).ok_or(Unavailable::NoSource)?;
                NativeTranscript::resolve(
                    root,
                    provider,
                    &crate::live::clean_leaf(&self.native_id),
                    Some(&self.native_id),
                )
                .map(|source| source.map(ContentSource::Native))
            }
            ObservationNamespace::Acp => journal::Source::find(
                root,
                journal::Identity {
                    home: self.home,
                    agent: self.agent.clone(),
                    native_id: self.native_id.clone(),
                },
            )
            .map(|source| source.map(ContentSource::Journal))
            .map_err(|_| Unavailable::Unreadable),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "at", rename_all = "snake_case", deny_unknown_fields)]
enum Target {
    Home { home: DeviceId },
    Reference { reference: ContentReference },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentLocator {
    version: u8,
    view: SessionId,
    target: Target,
}
impl ContentLocator {
    pub fn of(session: &Session) -> Result<Self, String> {
        let target = match &session.observation {
            Some(reference) => {
                if reference.home != session.home {
                    return Err(
                        "session home differs from its canonical observation reference".into(),
                    );
                }
                Target::Reference {
                    reference: ContentReference::observed(reference),
                }
            }
            None if session.kind.0 == khor_core::kind::OBSERVE => {
                return Err("observation has no canonical content reference".into());
            }
            None => Target::Home { home: session.home },
        };
        let locator = Self {
            version: VERSION,
            view: session.id.clone(),
            target,
        };
        locator.validate()?;
        Ok(locator)
    }
    pub fn observed(view: SessionId, reference: &ObservationRef) -> Result<Self, String> {
        let locator = Self {
            version: VERSION,
            view,
            target: Target::Reference {
                reference: ContentReference::observed(reference),
            },
        };
        locator.validate()?;
        Ok(locator)
    }
    pub fn decode(encoded: &str, view: &SessionId) -> Result<Self, String> {
        let locator: Self = decode(encoded)?;
        locator.validate()?;
        if &locator.view != view {
            return Err("content locator belongs to another view".into());
        }
        Ok(locator)
    }
    pub fn encode(&self) -> Result<String, String> {
        self.validate()?;
        encode(self)
    }
    pub fn home(&self) -> DeviceId {
        match &self.target {
            Target::Home { home } => *home,
            Target::Reference { reference } => reference.home,
        }
    }
    pub(crate) fn reference(&self) -> Option<&ContentReference> {
        match &self.target {
            Target::Reference { reference } => Some(reference),
            Target::Home { .. } => None,
        }
    }
    pub(crate) fn view(&self) -> &SessionId {
        &self.view
    }
    fn validate(&self) -> Result<(), String> {
        version(self.version)?;
        identifier(&self.view.0)?;
        if let Some(reference) = self.reference() {
            reference.validate()?;
        }
        encode(self).map(|_| ())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadBinding {
    version: u8,
    view: SessionId,
    reference: ContentReference,
}
impl ReadBinding {
    pub fn new(view: SessionId, reference: ContentReference) -> Result<Self, String> {
        let binding = Self {
            version: VERSION,
            view,
            reference,
        };
        binding.validate()?;
        Ok(binding)
    }
    pub fn decode(encoded: &str, view: &SessionId) -> Result<Self, String> {
        let binding: Self = decode(encoded)?;
        binding.validate()?;
        if &binding.view != view {
            return Err("content binding belongs to another view".into());
        }
        Ok(binding)
    }
    pub fn encode(&self) -> Result<String, String> {
        self.validate()?;
        encode(self)
    }
    pub fn home(&self) -> DeviceId {
        self.reference.home
    }
    pub fn reference(&self) -> &ContentReference {
        &self.reference
    }
    pub fn view(&self) -> &SessionId {
        &self.view
    }
    pub fn source_id(&self) -> String {
        let bytes = serde_json::to_vec(&(VERSION, "content", &self.view, &self.reference))
            .expect("validated content identity serializes");
        format!("content:1:{}", blake3::hash(&bytes).to_hex())
    }
    pub fn validate(&self) -> Result<(), String> {
        version(self.version)?;
        identifier(&self.view.0)?;
        self.reference.validate()?;
        encode(self).map(|_| ())
    }
}

fn identifier(value: &str) -> Result<(), String> {
    if value.is_empty() || value.len() > ID_BYTES {
        return Err("content identity exceeds its byte budget or is empty".into());
    }
    Ok(())
}
fn version(value: u8) -> Result<(), String> {
    if value != VERSION {
        return Err("unsupported content reference version".into());
    }
    Ok(())
}
fn encode(value: &impl Serialize) -> Result<String, String> {
    let encoded = serde_json::to_string(value).map_err(|e| e.to_string())?;
    if encoded.len() > METADATA_BYTES {
        return Err("content reference exceeds its metadata byte budget".into());
    }
    Ok(encoded)
}
fn decode<T: serde::de::DeserializeOwned>(encoded: &str) -> Result<T, String> {
    if encoded.len() > METADATA_BYTES {
        return Err("content reference exceeds its metadata byte budget".into());
    }
    serde_json::from_str(encoded).map_err(|e| format!("invalid content reference: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scaffold_projection_reuses_the_source_parser_without_promoting_system_facts() {
        let reference = ContentReference::observed(&observed(9));
        let prompt = r#"{"kind":"prompt","turn":"local","text":"marker","images":[],"files":[]}"#;
        let projected = reference.body_projection(prompt).unwrap();
        assert!(projected.complete);
        assert_eq!(projected.updates.len(), 1);
        assert_eq!(projected.updates[0]["update"]["content"]["text"], "marker");
        let echo = r#"{"kind":"update","turn":"local","notification":{"sessionId":"native","update":{"sessionUpdate":"user_message_chunk","content":{"type":"text","text":"marker"}}}}"#;
        let projected = reference.body_projection(echo).unwrap();
        assert!(projected.complete);
        assert!(projected.updates.is_empty());
        for source_only in [
            "{",
            r#"{"kind":"prompt_origin","turn":"local","token":"client","start":1,"end":2}"#,
            r#"{"kind":"gap","opening":"resume","incomplete_record":false,"interrupted_turn":null}"#,
        ] {
            let projected = reference.body_projection(source_only).unwrap();
            assert!(!projected.complete);
            assert!(projected.updates.is_empty());
        }
        assert!(reference.body_projection(&"x".repeat(64 * 1024 + 1)).is_err());
        let native = ContentReference { namespace: ObservationNamespace::Native,
            agent: "claude".into(), ..reference };
        let scaffold = serde_json::to_string_pretty(&serde_json::json!({"type":"user","message":{"role":"user","content":"native marker"}})).unwrap();
        let projected = native.body_projection(&scaffold).unwrap();
        assert!(projected.complete);
        assert_eq!(projected.updates[0]["update"]["content"]["text"], "native marker");
        assert!(!native.body_projection("{").unwrap().complete);
        assert!(!native.body_projection("{}").unwrap().complete);
        assert!(!native.body_projection(r#"{"type":"unknown"}"#).unwrap().complete);
    }
    fn observed(owner: u8) -> ObservationRef {
        ObservationRef {
            owner: DeviceId([owner; 32]),
            home: DeviceId([2; 32]),
            namespace: ObservationNamespace::Acp,
            agent: "any-registry-agent".into(),
            native_id: "opaque/observe/provider-id".into(),
            imported_at: khor_core::Millis(1),
            updated_at: None,
        }
    }
    #[test]
    fn metadata_owner_is_not_part_of_a_read_route_or_source_identity() {
        let view = SessionId("observe/view".into());
        let first = ContentLocator::observed(view.clone(), &observed(9)).unwrap();
        let second = ContentLocator::observed(view.clone(), &observed(7)).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.home(), DeviceId([2; 32]));
        let encoded = first.encode().unwrap();
        assert_eq!(ContentLocator::decode(&encoded, &view).unwrap(), first);
        assert!(ContentLocator::decode(&encoded, &SessionId("different-view".into())).is_err());
        let binding = ReadBinding::new(view, first.reference().unwrap().clone()).unwrap();
        assert_eq!(binding.home(), DeviceId([2; 32]));
        assert_eq!(
            ReadBinding::decode(&binding.encode().unwrap(), binding.view()).unwrap(),
            binding
        );
    }
    #[test]
    fn source_hash_changes_for_each_identity_axis_and_stays_small_for_long_ids() {
        let mut reference = ContentReference::observed(&observed(9));
        reference.native_id = "n".repeat(4000);
        let binding = ReadBinding::new(SessionId("view".into()), reference.clone()).unwrap();
        assert!(binding.source_id().len() < 128);
        for next in [
            ContentReference {
                home: DeviceId([3; 32]),
                ..reference.clone()
            },
            ContentReference {
                agent: "another-agent".into(),
                ..reference.clone()
            },
            ContentReference {
                native_id: "other-id".into(),
                ..reference.clone()
            },
        ] {
            assert_ne!(
                ReadBinding::new(SessionId("view".into()), next)
                    .unwrap()
                    .source_id(),
                binding.source_id()
            );
        }
        let native = ContentReference {
            namespace: ObservationNamespace::Native,
            agent: "codex".into(),
            ..reference.clone()
        };
        let acp = ContentReference {
            namespace: ObservationNamespace::Acp,
            ..native.clone()
        };
        assert_ne!(
            ReadBinding::new(SessionId("view".into()), native)
                .unwrap()
                .source_id(),
            ReadBinding::new(SessionId("view".into()), acp)
                .unwrap()
                .source_id()
        );
        assert_ne!(
            ReadBinding::new(SessionId("other-view".into()), reference)
                .unwrap()
                .source_id(),
            binding.source_id()
        );
    }
    #[test]
    fn unsupported_or_oversized_references_are_rejected_without_guessing_a_provider() {
        let binding = ReadBinding::new(
            SessionId("view".into()),
            ContentReference::observed(&observed(9)),
        )
        .unwrap();
        let mut value: serde_json::Value =
            serde_json::from_str(&binding.encode().unwrap()).unwrap();
        value["version"] = 2.into();
        assert!(ReadBinding::decode(&value.to_string(), binding.view()).is_err());
        assert!(ReadBinding::decode(&"x".repeat(METADATA_BYTES + 1), binding.view()).is_err());
        let mut native = ContentReference::observed(&observed(9));
        native.namespace = ObservationNamespace::Native;
        assert!(native.validate().unwrap_err().contains("provider"));
    }

    #[test]
    fn metadata_budget_counts_the_serialized_form_not_only_raw_string_bytes() {
        let mut reference = ContentReference::observed(&observed(9));
        reference.agent = "\0".repeat(3000);
        reference.native_id = "\0".repeat(3000);
        assert!(
            ReadBinding::new(SessionId("view".into()), reference)
                .unwrap_err()
                .contains("metadata")
        );
    }

    #[test]
    fn canonical_content_opens_without_the_alias_record_or_metadata_owner() {
        use crate::session_content::{
            ContentEvent,
            journal::{Identity, Journal, Opening},
        };
        let tree = crate::testdir::TempTree::at("content-reference");
        std::fs::create_dir_all(tree.path()).unwrap();
        let observation = observed(9);
        let identity = Identity {
            home: observation.home,
            agent: observation.agent.clone(),
            native_id: observation.native_id.clone(),
        };
        let mut writer = Journal::open(&tree.path(), identity, Opening::New).unwrap();
        writer.commit_base().unwrap();
        writer
            .accept("turn", "canonical text", &[], &[], None)
            .unwrap();
        let reference = ContentReference::observed(&observation);
        let binding = ReadBinding::new(
            SessionId("observe/view-with-no-file".into()),
            reference.clone(),
        )
        .unwrap();
        assert!(!tree.path().join(".khor/observations").exists());
        let source = reference
            .resolve(&tree.path(), observation.home)
            .unwrap()
            .unwrap();
        let (mut file, parse) = source.open().unwrap();
        let prepared = crate::adaptor::transcript_page::prepare_read(
            &mut file,
            &binding.source_id(),
            None,
            false,
            &|| false,
        )
        .unwrap();
        let page = crate::adaptor::transcript_page::parse_prepared(
            prepared,
            &binding.source_id(),
            None,
            false,
            parse,
            &|| false,
        )
        .unwrap();
        assert!(page.events.iter().any(|event| matches!(
            &event.payload,
            ContentEvent::Update(update) if update["update"]["content"]["text"] == "canonical text"
        )));
        assert!(reference.resolve(&tree.path(), DeviceId([1; 32])).is_err());
    }
}
