//! Each ordinary Khor conversation owns its config, Pi history and executor slot.
use super::*;
use khor_sync::chat::AgentConversation;

pub(super) fn execution_key(root: &Path, board: &str) -> PathBuf {
    if board == crate::wide::BROADCAST {
        root.to_owned()
    } else {
        directory(root)
            .join("conversations")
            .join(blake3::hash(board.as_bytes()).to_hex().as_str())
    }
}

pub(super) fn executor_path(root: &Path, board: &str) -> PathBuf {
    if board == crate::wide::BROADCAST {
        directory(root).join("executor.lock")
    } else {
        execution_key(root, board).join("executor.lock")
    }
}

pub(super) fn claim_in(root: &Path, board: &str) -> Result<File, String> {
    fs::create_dir_all(directory(root).join("runs")).map_err(|e| e.to_string())?;
    let path = executor_path(root, board);
    fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    file.try_lock().map_err(|_| msg::agent_busy().to_owned())?;
    Ok(file)
}

impl Node {
    pub fn khor_conversation(&self, board: &str) -> Result<AgentConversation, String> {
        if !crate::wide::agent_board(board) {
            return Err(msg::agent_arguments().into());
        }
        self.chat
            .agent_conversation(board)?
            .ok_or_else(|| msg::agent_arguments().into())
    }

    pub fn khor_agent_config_in(&self, board: &str) -> Result<Option<AgentConfig>, String> {
        crate::wide::board(board)?;
        if board != crate::wide::BROADCAST {
            self.khor_conversation(board)?;
        }
        self.chat.agent_config_in(board)
    }

    pub(super) fn create_khor_conversation(
        &self,
        token: &str,
        title: &str,
        cwd: &str,
    ) -> Result<Report, String> {
        self.refuse_unless_hosting()?;
        if !safe_token(token)
            || title.chars().count() > 256
            || title.chars().any(char::is_control)
            || cwd.len() > 4096
            || cwd.contains('\0')
        {
            return Err(msg::agent_arguments().into());
        }
        let _admission = prompt_admission(self.root())?;
        let identity =
            serde_json::to_vec(&(self.device_str(), token)).map_err(|e| e.to_string())?;
        let board = format!("wide/agent-{}", blake3::hash(&identity).to_hex());
        let creation = serde_json::to_vec(&(title.trim(), cwd)).map_err(|e| e.to_string())?;
        let requested = AgentConversation {
            creation: blake3::hash(&creation).to_hex().to_string(),
            home: self.device_str().into(),
            title: title.trim().into(),
            cwd: cwd.into(),
            auto_title: title.trim().is_empty(),
        };
        let config = if let Some(existing) = self.chat.agent_conversation(&board)? {
            if existing.home != requested.home || existing.creation != requested.creation {
                return Err(msg::console_token_conflict().into());
            }
            self.chat.agent_config_in(&board)?
        } else {
            let config = self
                .khor_agent_config()?
                .filter(|c| c.home == self.device_str());
            self.chat
                .set_agent_conversation(&board, &requested, config.as_ref())?;
            config
        };
        let conversation = self.khor_conversation(&board)?;
        let _ = self.emit_row_of(&crate::SessionId(board.clone()));
        Ok(Report {
            config,
            run: None,
            data: Some(json!({"session":board,"conversation":conversation})),
        })
    }

    pub(crate) fn accept_agent_report(
        &self,
        action: &Action,
        report: &Report,
    ) -> Result<(), String> {
        if matches!(action, Action::CreateConversation { .. }) {
            let data = report
                .data
                .as_ref()
                .ok_or_else(|| msg::agent_arguments().to_owned())?;
            let board = data["session"]
                .as_str()
                .filter(|s| crate::wide::agent_board(s))
                .ok_or_else(|| msg::agent_arguments().to_owned())?;
            let conversation: AgentConversation =
                serde_json::from_value(data["conversation"].clone()).map_err(|e| e.to_string())?;
            if let Some(existing) = self.chat.agent_conversation(board)? {
                if existing.home != conversation.home || existing.creation != conversation.creation
                {
                    return Err(msg::console_token_conflict().into());
                }
            } else {
                self.chat
                    .set_agent_conversation(board, &conversation, report.config.as_ref())?;
            }
        } else if configuration_reply(action, report) {
            if let Some(config) = &report.config {
                self.chat.set_agent_config_in(action.board(), config)?;
            }
        }
        Ok(())
    }
}

// The root admission lock is held while taking these existing execution locks.
pub(super) fn close_claims(root: &Path) -> Result<Option<Vec<File>>, String> {
    let path = directory(root).join("conversations");
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Some(Vec::new())),
        Err(error) => return Err(error.to_string()),
    };
    let mut held = Vec::new();
    for entry in entries {
        let path = entry
            .map_err(|e| e.to_string())?
            .path()
            .join("executor.lock");
        let file = match OpenOptions::new().read(true).write(true).open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.to_string()),
        };
        match file.try_lock() {
            Ok(()) => held.push(file),
            Err(std::fs::TryLockError::WouldBlock) => return Ok(None),
            Err(error) => return Err(error.to_string()),
        }
    }
    Ok(Some(held))
}

pub(super) fn name_conversation(
    root: &Path,
    board: &str,
    title: &str,
    key: &str,
) -> Result<(), String> {
    if board == crate::wide::BROADCAST
        || title.trim().is_empty()
        || title.chars().count() > 80
        || title.chars().any(char::is_control)
    {
        return Ok(());
    }
    let _configuration = configuration_lock(root)?;
    let node = Node::open(root.to_owned())?;
    let mut conversation = node.khor_conversation(board)?;
    if !conversation.auto_title {
        return Ok(());
    }
    conversation.title = if key.is_empty() {
        title.trim().into()
    } else {
        title.trim().replace(key, "[redacted]")
    };
    conversation.auto_title = false;
    node.chat
        .set_agent_conversation(board, &conversation, None)?;
    let _ = node.emit_row_of(&crate::SessionId(board.into()));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::KindSurface;

    struct Scratch(PathBuf);
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn node() -> (Scratch, Node) {
        let root = std::env::temp_dir().join(format!(
            "khor-conversations-{}",
            crate::link::fresh_hex().unwrap()
        ));
        fs::create_dir_all(root.join(".khor")).unwrap();
        fs::write(
            root.join(".khor/relays.json"),
            r#"{"relays":[{"url":"http://127.0.0.1:9"}],"only_mine":true}"#,
        )
        .unwrap();
        let node = Node::open_hosting(root.clone(), "conversations", true).unwrap();
        (Scratch(root), node)
    }
    fn create(node: &Node, token: &str, title: &str) -> String {
        node.create_khor_conversation(token, title, "/fixture")
            .unwrap()
            .data
            .unwrap()["session"]
            .as_str()
            .unwrap()
            .into()
    }

    #[test]
    fn deleting_a_conversation_does_not_hide_its_empty_replacement_or_revive_from_late_output() {
        let (_scratch, node) = node();
        let a = create(&node, "deleted-conversation-token-a", "");
        let mut saved = Saved {
            board: a.clone(),
            fingerprint: "deleted-conversation-fixture".into(),
            record: CommandRecord {
                id: "deleted-conversation-receipt".into(),
                owner: node.device_str().into(),
                status: "done".into(),
                started_at_ms: 1,
                ended_at_ms: Some(2),
                agent: Some(AgentTurn {
                    model: "fixture".into(),
                    approval: None,
                    updated_at_ms: 0,
                    updates: Vec::new(),
                    updates_truncated: false,
                }),
                ..Default::default()
            },
        };
        fs::create_dir_all(directory(node.root()).join("runs")).unwrap();
        save(node.root(), &saved).unwrap();
        node.close(&crate::SessionId(a.clone())).unwrap();
        let b = create(&node, "replacement-conversation-token-b", "");
        assert_ne!(a, b);
        assert_eq!(create(&node, "replacement-conversation-token-b", ""), b);
        let rows = crate::wide::WideKind.rows(&node).unwrap();
        assert!(rows.iter().any(|row| row.id.0 == b));
        assert!(!rows.iter().any(|row| row.id.0 == a));
        assert!(node.chat.log(&b).unwrap().messages.is_empty());

        saved.record.stdout = "late output after deletion".into();
        saved.record.ended_at_ms = Some(3);
        let _ = save(node.root(), &saved);
        let reopened = Node::open(node.root().to_owned()).unwrap();
        let rows = crate::wide::WideKind.rows(&reopened).unwrap();
        assert!(!rows.iter().any(|row| row.id.0 == a));
        assert!(rows.iter().any(|row| row.id.0 == b));
        assert!(reopened.chat.log(&b).unwrap().messages.is_empty());
    }

    #[test]
    fn deleting_a_conversation_with_an_active_receipt_leaves_other_conversations_visible() {
        let (_scratch, node) = node();
        let a = create(&node, "running-conversation-token-a", "");
        let b = create(&node, "running-conversation-token-b", "");
        let saved = Saved {
            board: a.clone(),
            fingerprint: "running-conversation-fixture".into(),
            record: CommandRecord {
                id: "running-conversation-receipt".into(),
                owner: node.device_str().into(),
                status: "running".into(),
                started_at_ms: 1,
                agent: Some(AgentTurn {
                    model: "fixture".into(),
                    approval: None,
                    updated_at_ms: 0,
                    updates: Vec::new(),
                    updates_truncated: false,
                }),
                ..Default::default()
            },
        };
        fs::create_dir_all(directory(node.root()).join("runs")).unwrap();
        save(node.root(), &saved).unwrap();
        node.close(&crate::SessionId(a.clone())).unwrap();
        let rows = crate::wide::WideKind.rows(&node).unwrap();
        assert!(!rows.iter().any(|row| row.id.0 == a));
        assert!(rows.iter().any(|row| row.id.0 == b));
    }

    #[test]
    fn creation_retries_reuse_one_conversation_and_configs_and_receipts_remain_separate() {
        let (_scratch, node) = node();
        let a = create(&node, "conversation-token-a", "");
        let b = create(&node, "conversation-token-b", "My conversation");
        assert_ne!(a, b);
        assert_eq!(create(&node, "conversation-token-a", ""), a);
        assert!(
            node.create_khor_conversation("conversation-token-a", "different", "/fixture")
                .is_err()
        );
        let config = AgentConfig {
            home: node.device_str().into(),
            account: "fixture-account".into(),
            model: "fixture-model".into(),
            options: Default::default(),
        };
        node.chat.set_agent_config_in(&a, &config).unwrap();
        assert_eq!(node.khor_agent_config_in(&a).unwrap(), Some(config));
        assert_eq!(node.khor_agent_config_in(&b).unwrap(), None);
        let _claim = claim_in(node.root(), &a).unwrap();
        let record = CommandRecord {
            id: "fixture-receipt".into(),
            line: "Only A".into(),
            owner: node.device_str().into(),
            status: "done".into(),
            started_at_ms: 1,
            ended_at_ms: Some(2),
            ..Default::default()
        };
        save(
            node.root(),
            &Saved {
                board: a.clone(),
                fingerprint: "receipt-fixture".into(),
                record: record.clone(),
            },
        )
        .unwrap();
        assert_eq!(node.chat.log(&a).unwrap().messages.len(), 1);
        assert!(node.chat.log(&b).unwrap().messages.is_empty());
        assert!(
            node.chat
                .log(crate::wide::BROADCAST)
                .unwrap()
                .messages
                .is_empty()
        );
        let rows = crate::wide::WideKind.rows(&node).unwrap();
        assert_eq!(
            rows.iter()
                .filter(|row| row.id.0 == a || row.id.0 == b)
                .count(),
            2
        );
        let reopened = Node::open(node.root().to_owned()).unwrap();
        assert_eq!(reopened.chat.log(&a).unwrap().messages.len(), 1);
        assert_eq!(
            reopened.khor_conversation(&b).unwrap().title,
            "My conversation"
        );
    }
    #[test]
    fn a_late_create_reply_cannot_replace_a_newer_title_or_config() {
        let (_scratch, node) = node();
        let action = Action::CreateConversation {
            token: "conversation-token-a".into(),
            title: "".into(),
            cwd: "/fixture".into(),
        };
        let report = node
            .create_khor_conversation("conversation-token-a", "", "/fixture")
            .unwrap();
        let board = report.data.as_ref().unwrap()["session"].as_str().unwrap();
        name_conversation(node.root(), board, "Current title", "").unwrap();
        let config = AgentConfig {
            home: node.device_str().into(),
            account: "chosen-account".into(),
            model: "chosen-model".into(),
            options: Default::default(),
        };
        node.chat.set_agent_config_in(board, &config).unwrap();
        node.accept_agent_report(&action, &report).unwrap();
        assert_eq!(
            node.khor_conversation(board).unwrap().title,
            "Current title"
        );
        assert_eq!(node.khor_agent_config_in(board).unwrap(), Some(config));
    }

    #[test]
    fn management_replies_update_their_own_conversation_only() {
        let (_scratch, node) = node();
        let a = create(&node, "conversation-token-a", "");
        let b = create(&node, "conversation-token-b", "");
        let config = AgentConfig {
            home: node.device_str().into(),
            account: "selected-account".into(),
            model: "selected-model".into(),
            options: Default::default(),
        };
        let action = Action::Manage {
            request: json!({"operation":"poll","session":a}),
        };
        let report = Report {
            config: Some(config.clone()),
            run: None,
            data: Some(json!({"status":"done","result":{"options":{},"configOptions":[]}})),
        };
        node.accept_agent_report(&action, &report).unwrap();
        assert_eq!(node.khor_agent_config_in(&a).unwrap(), Some(config));
        assert!(node.khor_agent_config_in(&b).unwrap().is_none());
        assert!(node.khor_agent_config().unwrap().is_none());
    }

    #[test]
    fn executor_slots_are_independent_and_root_close_waits_for_every_slot() {
        let (_scratch, node) = node();
        let a = create(&node, "conversation-token-a", "");
        let b = create(&node, "conversation-token-b", "");
        let first = claim_in(node.root(), &a).unwrap();
        assert!(claim_in(node.root(), &a).is_err());
        let second = claim_in(node.root(), &b).unwrap();
        assert!(close_claims(node.root()).unwrap().is_none());
        drop(first);
        assert!(close_claims(node.root()).unwrap().is_none());
        drop(second);
        assert_eq!(close_claims(node.root()).unwrap().unwrap().len(), 2);
        assert_ne!(
            execution_key(node.root(), &a),
            execution_key(node.root(), &b)
        );
    }
    #[tokio::test]
    async fn approval_answers_reach_only_the_conversation_that_owns_the_run() {
        let (_scratch, node) = node();
        let a = create(&node, "conversation-token-a", "");
        let b = create(&node, "conversation-token-b", "");
        let first = Arc::new(Running::new("run-a".into()));
        let second = Arc::new(Running::new("run-b".into()));
        *first.approval.lock().unwrap() = Some(("ask-7".into(), None));
        *second.approval.lock().unwrap() = Some(("ask-7".into(), None));
        let first_key = execution_key(node.root(), &a);
        let second_key = execution_key(node.root(), &b);
        {
            let mut active = running().lock().unwrap();
            active.insert(first_key.clone(), first.clone());
            active.insert(second_key.clone(), second.clone());
        }
        let answer = |session: String, run: &str, allow| Action::Conversation {
            session,
            action: Box::new(Action::Answer {
                run: run.into(),
                ask: "ask-7".into(),
                allow,
            }),
        };
        let wrong = node.khor_agent_here(answer(b.clone(), "run-a", true)).await;
        let right = node.khor_agent_here(answer(a, "run-a", true)).await;
        let other = node.khor_agent_here(answer(b, "run-b", false)).await;
        running().lock().unwrap().remove(&first_key);
        running().lock().unwrap().remove(&second_key);
        assert!(
            wrong.is_err(),
            "a different conversation cannot answer this run"
        );
        right.unwrap();
        other.unwrap();
        assert_eq!(
            *first.approval.lock().unwrap(),
            Some(("ask-7".into(), Some(true)))
        );
        assert_eq!(
            *second.approval.lock().unwrap(),
            Some(("ask-7".into(), Some(false)))
        );
    }

    #[test]
    fn model_names_are_persisted_once_and_never_replace_explicit_titles() {
        let (_scratch, node) = node();
        let a = create(&node, "conversation-token-a", "");
        let b = create(&node, "conversation-token-b", "Manual title");
        name_conversation(node.root(), &a, "Model title", "").unwrap();
        name_conversation(node.root(), &a, "Later title", "").unwrap();
        name_conversation(node.root(), &b, "Wrong replacement", "").unwrap();
        assert_eq!(node.khor_conversation(&a).unwrap().title, "Model title");
        assert_eq!(node.khor_conversation(&b).unwrap().title, "Manual title");
        assert_eq!(create(&node, "conversation-token-a", ""), a);
    }
}
