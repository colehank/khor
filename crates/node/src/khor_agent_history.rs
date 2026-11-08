//! Full redacted ACP display history, independent of sync-document summaries.
//! Per-turn identities preserve multi-machine ownership and existing Pi context.
use super::*;
use crate::session_content::{
    journal::{Identity, Journal, Opening, Outcome, Source},
    reference::{ContentReference, ReadBinding},
};
pub(super) struct FullSource {
    journal: Journal,
    directory: PathBuf,
    run: String,
}
fn identity(saved: &Saved) -> Result<Identity, String> {
    Ok(Identity {
        home: crate::device_id_from_hex(&saved.record.owner)?,
        agent: "khor".into(),
        native_id: saved.record.id.clone(),
    })
}
impl FullSource {
    pub(super) fn begin(root: &Path, saved: &Saved, key: &str) -> Result<Self, String> {
        let identity = identity(saved)?;
        let directory = identity.directory(root);
        let mut journal = Journal::open(root, identity, Opening::New)?;
        journal.commit_base()?;
        let prompt = output::redact(&saved.record.line, key);
        journal.accept(&saved.record.id, &prompt, &[], &[], None)?;
        Ok(Self {
            journal,
            directory,
            run: saved.record.id.clone(),
        })
    }
    pub(super) fn note(&mut self, value: &Value) -> Result<(), String> {
        self.journal
            .note(&json!({"sessionId":self.run,"update":value}))
    }
    pub(super) fn fail(&self) {
        let _ = fs::write(self.directory.join("display-incomplete"), b"incomplete");
    }
    pub(super) fn finish(&mut self, stopped: bool, failed: bool) -> Result<(), String> {
        self.journal.finish(
            &self.run,
            if stopped {
                Outcome::Cancelled
            } else if failed {
                Outcome::Failed
            } else {
                Outcome::Ended
            },
            None,
        )
    }
}
pub(super) fn read(root: &Path, board: &str, run: &str, home: &str) -> Result<Value, String> {
    if run.is_empty()
        || run.len() > 128
        || !run
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_".contains(&byte))
    {
        return Err(msg::agent_arguments().into());
    }
    let saved: Saved =
        serde_json::from_slice(&fs::read(file(root, run)).map_err(|_| "完整记录不可用")?)
            .map_err(|_| "完整记录不可用")?;
    if saved.board != board || saved.record.id != run || saved.record.owner != home {
        return Err(msg::agent_arguments().into());
    }
    let identity = identity(&saved)?;
    if identity.directory(root).join("display-incomplete").exists() {
        return Err("完整记录未能保存，当前仅有展示摘要".into());
    }
    let source = Source::find(root, identity.clone())?
        .ok_or("此旧记录仅保存了展示摘要，原有 Pi 上下文仍保留在执行机器")?;
    let view = crate::SessionId(format!("khor-history/{run}"));
    let binding = ReadBinding::new(
        view.clone(),
        ContentReference {
            home: identity.home,
            namespace: khor_core::ObservationNamespace::Acp,
            agent: identity.agent,
            native_id: identity.native_id,
        },
    )?
    .with_journal_generation(source.generation())?
    .encode()?;
    Ok(json!({"id":view.0,"binding":binding}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    #[test]
    fn full_history_outlives_summary_budget_and_keeps_cross_chunk_redaction() {
        let root = std::env::temp_dir().join(format!(
            "khor-full-history-{}",
            crate::link::fresh_hex().unwrap()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let node = Node::open_hosting(root.clone(), "full-history-test", false).unwrap();
        let saved = Saved {
            board: crate::wide::BROADCAST.into(),
            fingerprint: "fixture".into(),
            record: CommandRecord {
                id: "agent-full-test".into(),
                owner: node.device_str().into(),
                line: "prompt".into(),
                ..Default::default()
            },
        };
        let mut recorder = updates::Recorder::new("fixture-secret");
        recorder.start_source(&root, &saved).unwrap();
        let mut agent: AgentTurn =
            serde_json::from_value(json!({"model":"fixture","approval":null})).unwrap();
        for _ in 0..90 {
            recorder.push(&mut agent,json!({"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"x".repeat(4096)}}));
        }
        for text in ["fixture-", "secret", "FULL_HISTORY_TAIL"] {
            recorder.push(&mut agent,json!({"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":text}}));
        }
        recorder.finish(&mut agent);
        recorder.finish_source(false, false);
        assert!(agent.updates_truncated);
        assert!(agent.updates.iter().map(String::len).sum::<usize>() <= 256 * 1024);
        let source = Source::find(&root, identity(&saved).unwrap())
            .unwrap()
            .unwrap();
        let mut raw = String::new();
        source.open().unwrap().read_to_string(&mut raw).unwrap();
        assert!(raw.contains("FULL_HISTORY_TAIL"));
        assert!(!raw.contains("fixture-secret"));
        assert!(raw.contains("[redacted]"));
        std::fs::create_dir_all(file(&root, &saved.record.id).parent().unwrap()).unwrap();
        std::fs::write(
            file(&root, &saved.record.id),
            serde_json::to_vec(&saved).unwrap(),
        )
        .unwrap();
        assert!(
            read(
                &root,
                crate::wide::BROADCAST,
                &saved.record.id,
                node.device_str()
            )
            .unwrap()["binding"]
                .is_string()
        );
        assert!(read(&root, "another-board", &saved.record.id, node.device_str()).is_err());
        drop(recorder);
        drop(node);
        std::fs::remove_dir_all(root).unwrap();
    }
}
