//! Shared delegation tools for native A2A HTTP targets and detached ACP hosts.
//! Khor's authenticated mesh can select the connector/execution home.
use crate::{
    Node,
    gui_host::{GuiNote, GuiOp},
    host::{Hello, Welcome, read_frame, write_frame},
};
pub use khor_a2a::{
    AgentTarget, Execution, Executor, Record, SessionRef, Store, TaskRef, dispatch, protocol,
};
use khor_core::{DeviceId, SessionId};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    fs,
    net::TcpStream,
    path::Path,
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    ResolveBinding {
        agent: String,
        task: Option<String>,
    },
    Bound {
        binding: Binding,
        action: BoundAction,
    },
    Agents,
    Connect {
        agent: String,
        url: String,
    },
    Disconnect {
        agent: String,
    },
    Send {
        agent: String,
        message: protocol::Message,
        parent: Option<TaskRef>,
    },
    Get {
        agent: String,
        task: String,
    },
    Wait {
        agent: String,
        task: String,
        revision: u64,
    },
    List {
        agent: String,
        context: Option<String>,
        after: Option<String>,
    },
    Cancel {
        agent: String,
        task: String,
    },
    Answer {
        agent: String,
        task: String,
        ask: u64,
        option: Option<String>,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Answer {
    Binding {
        binding: Binding,
    },
    Agents {
        agents: Vec<Agent>,
    },
    Task {
        record: Box<Record>,
    },
    Tasks {
        records: Vec<Record>,
        #[serde(default)]
        next: Option<String>,
        #[serde(default)]
        cached: bool,
    },
    Message {
        agent: String,
        message: protocol::Message,
    },
}

/// Captured before a side effect. It contains no endpoint credentials or history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Binding {
    pub home: DeviceId,
    pub caller: DeviceId,
    pub agent: String,
    pub protocol: BindingProtocol,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BindingProtocol {
    Native,
    Http { endpoint_digest: String },
}
/// A flat whitelist prevents recursive envelopes and unrelated configuration work.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BoundAction {
    Send {
        message: protocol::Message,
        parent: Option<TaskRef>,
    },
    Answer {
        task: String,
        ask: u64,
        option: Option<String>,
    },
    Get {
        task: String,
    },
    Cancel {
        task: String,
    },
}
impl BoundAction {
    pub(crate) fn into_action(self, agent: String) -> Action {
        match self {
            Self::Send { message, parent } => Action::Send {
                agent,
                message,
                parent,
            },
            Self::Answer { task, ask, option } => Action::Answer {
                agent,
                task,
                ask,
                option,
            },
            Self::Get { task } => Action::Get { agent, task },
            Self::Cancel { task } => Action::Cancel { agent, task },
        }
    }
}

pub(crate) fn validate_answer(
    action: &Action,
    home: DeviceId,
    caller: DeviceId,
    answer: &Answer,
) -> Result<(), String> {
    let valid = match (action, answer) {
        (Action::ResolveBinding { agent, .. }, Answer::Binding { binding }) => {
            binding.home == home && binding.caller == caller && &binding.agent == agent
        }
        (Action::Bound { binding, action }, Answer::Task { record }) => {
            let expected_task = match action {
                BoundAction::Get { task }
                | BoundAction::Cancel { task }
                | BoundAction::Answer { task, .. } => Some(task),
                BoundAction::Send { message, .. } => message.task_id.as_ref(),
            };
            record.target.home == binding.home
                && record.caller == binding.caller
                && record.target.agent == binding.agent
                && expected_task.is_none_or(|task| task == &record.task.id)
        }
        (
            Action::Bound {
                binding,
                action: BoundAction::Send { .. },
            },
            Answer::Message { agent, .. },
        ) => &binding.agent == agent,
        (Action::ResolveBinding { .. } | Action::Bound { .. }, _) => false,
        _ => true,
    };
    if valid {
        Ok(())
    } else {
        Err("Delegation reply identity mismatch".into())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub name: String,
    pub available: bool,
    pub why: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card: Option<protocol::AgentCard>,
}
impl Node {
    pub fn delegation_store(&self) -> Result<Store, String> {
        self.refuse_unless_hosting()?;
        Store::open(self.root().join(".khor/a2a"), self.device()).map_err(|e| e.to_string())
    }
    pub fn delegation_target(&self, agent: &str) -> Result<AgentTarget, String> {
        self.refuse_unless_hosting()?;
        self.agent_or_refuse(agent)?;
        Ok(AgentTarget {
            home: self.device(),
            agent: agent.into(),
        })
    }
    /// `caller` is supplied by the authenticated transport, never by Action.
    pub(crate) fn delegation_here(
        &self,
        caller: DeviceId,
        action: Action,
    ) -> Result<Answer, String> {
        self.refuse_unless_hosting()?;
        let action = match action {
            Action::ResolveBinding { agent, task } => {
                let protocol = if crate::a2a_http::is_target(&agent) {
                    crate::a2a_http::resolve_binding(self, caller, &agent, task.as_deref())?
                } else {
                    if let Some(task) = task {
                        self.delegation_store()?
                            .get(
                                caller,
                                &AgentTarget {
                                    home: self.device(),
                                    agent: agent.clone(),
                                },
                                &task,
                            )
                            .map_err(|e| e.to_string())?;
                    } else {
                        self.delegation_target(&agent)?;
                    }
                    BindingProtocol::Native
                };
                return Ok(Answer::Binding {
                    binding: Binding {
                        home: self.device(),
                        caller,
                        agent,
                        protocol,
                    },
                });
            }
            Action::Bound { binding, action } => {
                if binding.home != self.device() || binding.caller != caller {
                    return Err("Delegation binding identity mismatch".into());
                }
                match &binding.protocol {
                    BindingProtocol::Http { endpoint_digest }
                        if crate::a2a_http::is_target(&binding.agent) =>
                    {
                        return crate::a2a_http::handle_bound(
                            self,
                            caller,
                            &binding.agent,
                            action,
                            endpoint_digest,
                        );
                    }
                    BindingProtocol::Native if !crate::a2a_http::is_target(&binding.agent) => {
                        action.into_action(binding.agent)
                    }
                    _ => return Err("Delegation binding protocol mismatch".into()),
                }
            }
            action => action,
        };
        match &action {
            Action::Connect { agent, url } => {
                return Ok(Answer::Agents {
                    agents: vec![crate::a2a_http::describe(
                        self.connect_a2a(agent, url, None)?,
                    )],
                });
            }
            Action::Disconnect { agent } => {
                self.disconnect_a2a(agent)?;
                return Ok(Answer::Agents { agents: vec![] });
            }
            Action::Send { agent, .. }
            | Action::Get { agent, .. }
            | Action::Wait { agent, .. }
            | Action::List { agent, .. }
            | Action::Cancel { agent, .. }
            | Action::Answer { agent, .. }
                if crate::a2a_http::is_target(agent) =>
            {
                return crate::a2a_http::handle(self, caller, action);
            }
            _ => {}
        }
        let store = self.delegation_store()?;
        store.recover().map_err(|e| e.to_string())?;
        if matches!(action, Action::Agents) {
            let mut agents: Vec<_> = self
                .agents()?
                .into_iter()
                .map(|(name, spec)| {
                    let why = Node::agent_launch(&spec).err();
                    Agent {
                        name,
                        available: why.is_none(),
                        why,
                        protocol: Some("ACP".into()),
                        card: None,
                    }
                })
                .collect();
            agents.extend(
                self.a2a_agents()?
                    .into_iter()
                    .map(crate::a2a_http::describe),
            );
            return Ok(Answer::Agents { agents });
        }

        let agent = match &action {
            Action::ResolveBinding { .. }
            | Action::Bound { .. }
            | Action::Agents
            | Action::Connect { .. }
            | Action::Disconnect { .. } => unreachable!(),
            Action::Send { agent, .. }
            | Action::Get { agent, .. }
            | Action::Wait { agent, .. }
            | Action::List { agent, .. }
            | Action::Cancel { agent, .. }
            | Action::Answer { agent, .. } => agent,
        };
        // Existing receipts remain readable and cancelable after unregistering.
        let target = AgentTarget {
            home: self.device(),
            agent: agent.clone(),
        };
        let read = |task: &str| store.get(caller, &target, task).map_err(|e| e.to_string());
        let record = match action {
            Action::ResolveBinding { .. }
            | Action::Bound { .. }
            | Action::Agents
            | Action::Connect { .. }
            | Action::Disconnect { .. } => unreachable!(),
            Action::Send {
                agent,
                message,
                parent,
            } => {
                self.delegation_target(&agent)?;
                if parent.as_ref().is_some_and(|p| p.home != caller) {
                    return Err(
                        "Parent task must belong to the authenticated calling device".into(),
                    );
                }
                if message.context_id.is_some()
                    && message
                        .metadata
                        .as_ref()
                        .is_some_and(|m| m.contains_key("khor.cwd"))
                {
                    return Err("A continued context keeps its working directory; omit cwd".into());
                }
                if message
                    .parts
                    .iter()
                    .any(|p| !matches!(p.content, protocol::PartContent::Text(_)))
                {
                    return Err("ACP delegation currently accepts text parts only".into());
                }
                let submission = store
                    .accept(caller, &target, &message, parent)
                    .map_err(|e| e.to_string())?;
                let mut record = submission.record;
                // Only an explicitly retried Submitted input may be dispatched.
                // Working/terminal deduplication is a read, never a second prompt.
                if record.task.status.state == protocol::TaskState::Submitted {
                    if let Err(why) = self.start_delegation(&store, &record, &message.message_id) {
                        if let Ok(mut execution) =
                            Execution::claim(store.clone(), &record.task.id, &message.message_id)
                        {
                            execution
                                .update(
                                    protocol::TaskState::Failed,
                                    Some(agent_message(&why)),
                                    None,
                                )
                                .map_err(|e| e.to_string())?;
                        } else {
                            return Err(why);
                        }
                    }
                    record = read(&record.task.id)?;
                }
                record
            }
            Action::Get { task, .. } => read(&task)?,
            Action::Wait { task, revision, .. } => {
                let until = Instant::now() + Duration::from_secs(8);
                loop {
                    let record = read(&task)?;
                    if record.revision != revision
                        || record.task.status.state.is_terminal()
                        || approval(&record).is_some()
                        || Instant::now() >= until
                    {
                        break record;
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
            }
            Action::List { context, after, .. } => {
                let records = store
                    .list(caller, &target, context.as_deref(), after.as_deref(), 50)
                    .map_err(|e| e.to_string())?;
                let next = if records.len() == 50 {
                    records.last().map(|r| r.task.id.clone())
                } else {
                    None
                };
                return Ok(Answer::Tasks {
                    records,
                    next,
                    cached: false,
                });
            }
            Action::Cancel { task, .. } => {
                store
                    .request_cancel(caller, &target, &task)
                    .map_err(|e| e.to_string())?;
                // The host observes intent for this exact task. An unscoped
                // GuiOp::Stop could arrive after completion and stop the next
                // task or a human's turn in the same live session instead.
                read(&task)?
            }
            Action::Answer {
                task, ask, option, ..
            } => {
                let record = read(&task)?;
                let pending = approval(&record).ok_or("This task has no pending permission")?;
                if pending["ask"].as_u64() != Some(ask)
                    || option.as_ref().is_some_and(|id| {
                        !pending["options"].as_array().is_some_and(|options| {
                            options.iter().any(|o| o[0].as_str() == Some(id))
                        })
                    })
                {
                    return Err("The permission or option is no longer current".into());
                }
                let session = record
                    .context_session
                    .as_ref()
                    .ok_or("Task has no session")?;
                host_op(self.root(), &session.id, &GuiOp::Answer { ask, option })?;
                // Wait for the host's receipt, not merely a successful socket write.
                let until = Instant::now() + Duration::from_secs(3);
                loop {
                    let updated = read(&task)?;
                    if approval(&updated).and_then(|a| a["ask"].as_u64()) != Some(ask) {
                        break updated;
                    }
                    if Instant::now() >= until {
                        return Err(
                            "Permission answer has not been confirmed; read task status".into()
                        );
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
            }
        };
        Ok(Answer::Task {
            record: Box::new(record),
        })
    }

    fn start_delegation(
        &self,
        store: &Store,
        record: &Record,
        message: &str,
    ) -> Result<(), String> {
        // Serialize session creation across request handlers/processes. The ACP
        // host separately owns the execution lease for the entire prompt.
        let path = self.root().join(".khor/a2a").join(format!(
            "start-{}.lock",
            blake3::hash(record.task.context_id.as_bytes()).to_hex()
        ));
        let mut options = fs::OpenOptions::new();
        options.create(true).truncate(false).read(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let gate = options.open(path).map_err(|e| e.to_string())?;
        gate.lock().map_err(|e| e.to_string())?;
        let current = store
            .get(record.caller, &record.target, &record.task.id)
            .map_err(|e| e.to_string())?;
        if current.task.status.state != protocol::TaskState::Submitted {
            return Ok(());
        }
        let session = if let Some(session) = &current.context_session {
            session.id.clone()
        } else {
            let spec = self.agent_or_refuse(&record.target.agent)?;
            let launch = Node::agent_launch(&spec)?;
            let cwd = current
                .task
                .history
                .as_ref()
                .and_then(|h| h.last())
                .and_then(|m| m.metadata.as_ref())
                .and_then(|m| m.get("khor.cwd"))
                .and_then(Value::as_str)
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| self.root().to_path_buf());
            if !cwd.is_absolute() || !cwd.is_dir() {
                return Err(
                    "Delegation cwd must be an existing absolute directory on the target machine"
                        .into(),
                );
            }
            self.open_gui_at(
                &cwd,
                &record.target.agent,
                &[launch],
                Some(&record.target.agent),
                false,
            )?
        };
        let mut connection = connect_host(self.root(), &session)?;
        write_frame(
            &mut connection,
            &GuiOp::Delegate {
                task: current.task.id.clone(),
                message: message.into(),
            },
        )?;
        // The socket acknowledgement carries only task identity; the durable
        // receipt remains authoritative if the reply is lost.
        loop {
            match read_frame::<GuiNote>(&mut connection)? {
                GuiNote::Delegated { task, why } if task == current.task.id => {
                    return why.map_or(Ok(()), Err);
                }
                GuiNote::Gone => return Err("ACP session ended before accepting the task".into()),
                _ => {}
            }
        }
    }
}

pub(crate) fn approval(record: &Record) -> Option<&Value> {
    record
        .task
        .status
        .message
        .as_ref()?
        .metadata
        .as_ref()?
        .get("khor.approval")
}
fn agent_message(text: &str) -> protocol::Message {
    protocol::Message::new(protocol::Role::Agent, vec![protocol::Part::text(text)])
}
fn connect_host(root: &Path, session: &SessionId) -> Result<TcpStream, String> {
    let live = crate::live::LiveKind::new(root.to_path_buf(), DeviceId([0; 32]));
    let dir = live.dir_of(session).ok_or("Unknown ACP session")?;
    let host = crate::host::read_host_file(&dir)?;
    let mut stream =
        TcpStream::connect_timeout(&([127, 0, 0, 1], host.port).into(), Duration::from_secs(2))
            .map_err(|e| e.to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| e.to_string())?;
    write_frame(
        &mut stream,
        &Hello {
            cookie: host.cookie,
            cols: 0,
            rows: 0,
        },
    )?;
    let welcome: Welcome = read_frame(&mut stream)?;
    if !welcome.ok {
        return Err(welcome.why);
    }
    Ok(stream)
}
fn host_op(root: &Path, session: &SessionId, op: &GuiOp) -> Result<(), String> {
    write_frame(&mut connect_host(root, session)?, op)
}

/// Held in the detached GUI host, so closing a caller or restarting the node
/// does not abandon a live agent. Errors fail the task and cancel the prompt.
pub(crate) struct HostedTask {
    execution: Execution,
    output: String,
    pending: Option<Value>,
    published: Instant,
}
impl HostedTask {
    pub fn claim(
        root: &Path,
        session: &SessionId,
        task: &str,
        message: &str,
    ) -> Result<(Self, String), String> {
        let node = Node::open(root.to_path_buf())?;
        let mut execution =
            Execution::claim(node.delegation_store()?, task, message).map_err(|e| e.to_string())?;
        if let Err(error) = execution.bind_session(SessionRef {
            home: node.device(),
            id: session.clone(),
        }) {
            let _ = execution.update(
                protocol::TaskState::Failed,
                Some(agent_message(&error.to_string())),
                None,
            );
            return Err(error.to_string());
        }
        let text = execution
            .message()
            .parts
            .iter()
            .filter_map(|p| match &p.content {
                protocol::PartContent::Text(text) => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");
        Ok((
            Self {
                execution,
                output: String::new(),
                pending: None,
                published: Instant::now(),
            },
            text,
        ))
    }
    pub fn note(&mut self, json: &str) -> Result<(), String> {
        let value: Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
        let update = &value["update"];
        if update["sessionUpdate"] == "agent_message_chunk" {
            if let Some(text) = update["content"]["text"].as_str() {
                if self.output.len() + text.len() > 96 * 1024 {
                    return Err("Delegated output exceeded 96 KiB".into());
                }
                self.output.push_str(text);
            }
        }
        if self.published.elapsed() >= Duration::from_millis(300) {
            self.publish(protocol::TaskState::Working)?;
        }
        Ok(())
    }
    pub fn ask(
        &mut self,
        ask: u64,
        title: &str,
        options: &[(String, String)],
    ) -> Result<(), String> {
        self.pending = Some(json!({"ask":ask,"title":title,"options":options}));
        self.publish(protocol::TaskState::Working)
    }
    pub fn answered(&mut self) -> Result<(), String> {
        self.pending = None;
        self.publish(protocol::TaskState::Working)
    }
    pub fn cancel_requested(&self) -> bool {
        self.execution.cancellation_requested().unwrap_or(true)
    }
    pub fn finish(&mut self, stop: &str) -> Result<(), String> {
        let state = match stop {
            "EndTurn" => protocol::TaskState::Completed,
            "Cancelled" => protocol::TaskState::Canceled,
            _ => {
                self.output.push_str(&format!("\nACP ended: {stop}"));
                protocol::TaskState::Failed
            }
        };
        self.pending = None;
        self.publish(state)
    }
    fn publish(&mut self, state: protocol::TaskState) -> Result<(), String> {
        let mut message = agent_message(&self.output);
        if let Some(pending) = &self.pending {
            message.metadata = Some([("khor.approval".into(), pending.clone())].into());
        }
        self.execution
            .update(state, Some(message), None)
            .map_err(|e| e.to_string())?;
        self.published = Instant::now();
        Ok(())
    }
}
impl Drop for HostedTask {
    fn drop(&mut self) {
        if !self.execution.record().task.status.state.is_terminal() {
            let _ = self.execution.update(
                protocol::TaskState::Failed,
                Some(agent_message(
                    "ACP host was interrupted; work was not replayed",
                )),
                None,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delegation_uses_the_existing_registry_and_this_nodes_private_store() {
        let root = std::env::temp_dir().join(format!("khor-node-a2a-{}", protocol::new_task_id()));
        let node = Node::open_as(root.clone(), "delegation-fixture").unwrap();
        assert!(node.delegation_target("fixture-agent").is_err());
        // This deliberately absent command must never be spawned by discovery.
        node.register_agent("fixture-agent", &["khor-fixture-must-not-run".into()])
            .unwrap();
        let target = node.delegation_target("fixture-agent").unwrap();
        assert_eq!(target.home, node.device());
        let store = node.delegation_store().unwrap();
        let message = protocol::Message::new(
            protocol::Role::User,
            vec![protocol::Part::text("record only")],
        );
        let accepted = store
            .accept(node.device(), &target, &message, None)
            .unwrap();
        assert_eq!(
            accepted.record.task.status.state,
            protocol::TaskState::Submitted
        );
        assert!(root.join(".khor/a2a/tasks.json").is_file());
        assert!(!root.join(".khor/agent/runs").exists());
        assert_eq!(
            node.agent("fixture-agent").unwrap().unwrap().command,
            "khor-fixture-must-not-run"
        );
        // Official optional-field JSON must survive both positional transports.
        fn round_trip<T: serde::Serialize + serde::de::DeserializeOwned>(value: T) {
            let decoded: T = crate::proto::decode(&crate::proto::encode(&value).unwrap()).unwrap();
            assert_eq!(
                serde_json::to_value(value).unwrap(),
                serde_json::to_value(decoded).unwrap()
            );
        }
        let action = Action::Send {
            agent: "fixture-agent".into(),
            message,
            parent: None,
        };
        let answer = Answer::Task {
            record: Box::new(accepted.record),
        };
        round_trip(crate::proto::Request::Delegation {
            action: action.clone(),
        });
        round_trip(crate::proto::Response::Delegation {
            answer: answer.clone(),
        });
        round_trip(crate::ipc::Op::Delegation {
            machine: "here".into(),
            action,
        });
        round_trip(crate::ipc::Reply::Delegation { answer });
        let first = TaskRef {
            home: node.device(),
            id: "first-root-run".into(),
        };
        let next = TaskRef {
            home: node.device(),
            id: "next-root-run".into(),
        };
        assert_eq!(
            original_parent(
                &root,
                &first,
                &Binding {
                    home: node.device(),
                    caller: node.device(),
                    agent: "fixture-agent".into(),
                    protocol: BindingProtocol::Native
                },
                "same-input"
            )
            .unwrap(),
            first
        );
        assert_eq!(
            original_parent(
                &root,
                &next,
                &Binding {
                    home: node.device(),
                    caller: node.device(),
                    agent: "fixture-agent".into(),
                    protocol: BindingProtocol::Native
                },
                "same-input"
            )
            .unwrap(),
            first
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct ChildLink {
    pub(crate) machine: String,
    pub(crate) binding: Binding,
    pub(crate) message: String,
    pub(crate) task: Option<String>,
}
pub(crate) fn children_dir(root: &Path, run: &str) -> std::path::PathBuf {
    root.join(".khor/agent/children")
        .join(blake3::hash(run.as_bytes()).to_hex().as_str())
}
/// Bind before sending. A lost response keeps an unresolved link; cancellation
/// never replays Send/Answer or claims an unknown task has stopped.
pub(crate) fn track_child(
    root: &Path,
    parent: &TaskRef,
    machine: &str,
    binding: &Binding,
    message: &str,
    task: Option<&str>,
) -> Result<(), String> {
    let dir = children_dir(root, &parent.id);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    if binding.caller != parent.home {
        return Err("Child binding caller mismatch".into());
    }
    let fingerprint = serde_json::to_vec(&(binding, message)).map_err(|e| e.to_string())?;
    let leaf = blake3::hash(&fingerprint).to_hex().to_string();
    let path = dir.join(format!("{leaf}.json"));
    let names = crate::remote_cancel::names(&dir, crate::remote_cancel::CHILD_LIMIT * 2)?;
    if !path.exists() && names.len() >= crate::remote_cancel::CHILD_LIMIT {
        return Err("Child link resource limit".into());
    }
    let prior = if path.try_exists().map_err(|e| e.to_string())? {
        let prior: ChildLink = crate::remote_cancel::read(&path, 16 * 1024)?;
        if prior.binding != *binding
            || prior.message != message
            || task.is_some_and(|task| prior.task.as_ref().is_some_and(|previous| previous != task))
        {
            return Err("Child link identity mismatch".into());
        }
        Some(prior)
    } else {
        None
    };
    let link = ChildLink {
        machine: machine.into(),
        binding: binding.clone(),
        message: message.into(),
        task: task
            .map(str::to_owned)
            .or_else(|| prior.and_then(|p| p.task)),
    };
    let temporary = dir.join(format!("{}.tmp", protocol::new_task_id()));
    let bytes = serde_json::to_vec(&link).map_err(|e| e.to_string())?;
    if bytes.len() > 16 * 1024 {
        return Err("Child link resource limit".into());
    }
    crate::link::write_private(&temporary, &bytes)?;
    fs::File::open(&temporary)
        .and_then(|f| f.sync_all())
        .map_err(|e| e.to_string())?;
    fs::rename(temporary, &path).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    fs::File::open(&dir)
        .and_then(|f| f.sync_all())
        .map_err(|e| e.to_string())?;
    Ok(())
}
/// Official A2A types omit optional JSON fields. Positional MessagePack cannot
/// safely carry those structs directly; carry canonical JSON at this boundary.
pub(crate) mod wire_json {
    use serde::{Deserialize, Deserializer, Serialize, Serializer, de::DeserializeOwned};
    pub fn serialize<T: Serialize, S: Serializer>(
        value: &T,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            return value.serialize(serializer);
        }
        let json = serde_json::to_string(value).map_err(serde::ser::Error::custom)?;
        serializer.serialize_str(&json)
    }
    pub fn deserialize<'de, T: DeserializeOwned, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<T, D::Error> {
        if deserializer.is_human_readable() {
            return T::deserialize(deserializer);
        }
        let json = String::deserialize(deserializer)?;
        serde_json::from_str(&json).map_err(serde::de::Error::custom)
    }
}

/// A retry from a later Pi turn keeps the original parent in the accepted
/// message fingerprint. The new turn separately tracks the same child for Stop.
pub(crate) fn original_parent(
    root: &Path,
    parent: &TaskRef,
    binding: &Binding,
    message: &str,
) -> Result<TaskRef, String> {
    let dir = root.join(".khor/agent/messages");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let key = serde_json::to_vec(&(binding, message)).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{}.json", blake3::hash(&key).to_hex()));
    match fs::read(&path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| e.to_string()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // A Pi execution home has one active main turn; only that turn
            // writes this index. Readers never start or replay a message.
            let temporary = dir.join(format!("{}.tmp", protocol::new_task_id()));
            crate::link::write_private(
                &temporary,
                &serde_json::to_vec(parent).map_err(|e| e.to_string())?,
            )?;
            fs::rename(temporary, path).map_err(|e| e.to_string())?;
            Ok(parent.clone())
        }
        Err(error) => Err(error.to_string()),
    }
}
