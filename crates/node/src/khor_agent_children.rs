//! The parent board owns the child links. A read or action uses the
//! captured execution binding, even when a machine has since been renamed.
use super::*;
use crate::delegation::{self, BoundAction, ChildLink, Record, TaskRef, protocol};
use futures::{StreamExt, stream};
use std::collections::HashSet;
use std::time::Duration;

const PAGE: usize = 24;

struct Child {
    run: String,
    key: String,
    link: ChildLink,
}

fn collect(node: &Node, board: &str, runs: &[CommandRecord]) -> Result<Vec<Child>, String> {
    let mut seen = HashSet::new();
    let mut children = Vec::new();
    for run in runs {
        let directory = delegation::children_dir(node.root(), &run.id);
        let names = crate::remote_cancel::names(&directory, crate::remote_cancel::CHILD_LIMIT * 2)?;
        if names.is_empty() {
            continue;
        }
        let saved: Saved =
            crate::remote_cancel::read(&file(node.root(), &run.id), 8 * 1024 * 1024)?;
        if saved.board != board
            || saved.record.id != run.id
            || saved.record.owner != node.device_str()
        {
            continue;
        }
        for key in names {
            let link: ChildLink = crate::remote_cancel::read(&directory.join(&key), 16 * 1024)?;
            if link.binding.caller != node.device()
                || key != delegation::child_key(&link.binding, &link.message)?
            {
                return Err("Child link identity mismatch".into());
            }
            // Permission answers refer to the original task. They do not
            // create a second child.
            if link.message.starts_with("task:") && link.input_parent.is_none() {
                continue;
            }
            let unique = (
                link.binding.home,
                link.binding.agent.clone(),
                link.task.clone().unwrap_or_else(|| link.message.clone()),
            );
            if seen.insert(unique) {
                children.push(Child {
                    run: run.id.clone(),
                    key,
                    link,
                });
            }
            if children.len() >= PAGE {
                return Ok(children);
            }
        }
    }
    Ok(children)
}

async fn call(node: &Node, child: &Child, action: BoundAction) -> Result<Record, String> {
    let answer = node
        .delegation_on(
            &child.link.machine,
            delegation::Action::Bound {
                binding: child.link.binding.clone(),
                action,
            },
        )
        .await?;
    match answer {
        delegation::Answer::Task { record } => Ok(*record),
        _ => Err("Delegation returned no task".into()),
    }
}

fn owns_latest_input(child: &Child, record: &Record) -> bool {
    let parent = child.link.input_parent.clone().unwrap_or_else(|| TaskRef {
        home: child.link.binding.caller,
        id: child.run.clone(),
    });
    record.parent.as_ref() == Some(&parent)
        && latest_input(record).is_some_and(|message| message.message_id == child.link.message)
}

fn latest_input(record: &Record) -> Option<&protocol::Message> {
    record
        .task
        .history
        .as_ref()?
        .iter()
        .rev()
        .find(|message| message.role == protocol::Role::User)
}

fn summary(child: &Child, record: Option<&Record>) -> Value {
    use protocol::TaskState;
    let state = match record.map(|r| &r.task.status.state) {
        Some(TaskState::Submitted) => "submitted",
        Some(TaskState::Working) if record.and_then(delegation::approval).is_some() => "waiting",
        Some(TaskState::Working) => "working",
        Some(TaskState::InputRequired | TaskState::AuthRequired) => "waiting",
        Some(TaskState::Completed) => "completed",
        Some(TaskState::Canceled) => "canceled",
        Some(TaskState::Failed | TaskState::Rejected) => "failed",
        _ => "unavailable",
    };
    let result = record
        .and_then(|r| r.task.status.message.as_ref())
        .map(|message| {
            message
                .parts
                .iter()
                .filter_map(|part| match &part.content {
                    protocol::PartContent::Text(text) => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n")
                .chars()
                .take(16 * 1024)
                .collect::<String>()
        });
    json!({
        "run": child.run,
        "key": child.key,
        "agent": child.link.binding.agent,
        "machine": child.link.machine,
        "home": child.link.binding.home.hex(),
        "task": child.link.task,
        "context": record.map(|r| r.task.context_id.as_str()),
        "cwd": record.and_then(|r| r.task.history.as_ref()).and_then(|history| {
            history.iter().rev().find_map(|message| message.metadata.as_ref()?.get("khor.cwd")?.as_str())
        }),
        "state": state,
        "result": result,
        "can_cancel": record.is_some_and(|r| {
            !r.task.status.state.is_terminal() && !r.cancellation_requested && owns_latest_input(child, r)
                && (child.link.input_parent.is_some() || matches!(&child.link.binding.protocol, delegation::BindingProtocol::Native))
        }),
        "can_retry": record.is_some_and(|r| {
            matches!(r.task.status.state, TaskState::Canceled | TaskState::Failed) && owns_latest_input(child, r)
        }),
    })
}

fn owned_child(node: &Node, board: &str, run: &str, key: &str) -> Result<Child, String> {
    if run.is_empty()
        || run.len() > 128
        || !run
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
        || key.len() != 69
        || !key.ends_with(".json")
        || !key.as_bytes()[..64].iter().all(|b| b.is_ascii_hexdigit())
    {
        return Err("Invalid child reference".into());
    }
    let saved: Saved = crate::remote_cancel::read(&file(node.root(), run), 8 * 1024 * 1024)?;
    if saved.board != board
        || saved.record.id != run
        || saved.record.owner != node.device_str()
        || saved.record.agent.is_none()
    {
        return Err("Child does not belong to this conversation".into());
    }
    let link: ChildLink = crate::remote_cancel::read(
        &delegation::children_dir(node.root(), run).join(key),
        16 * 1024,
    )?;
    if link.binding.caller != node.device()
        || key != delegation::child_key(&link.binding, &link.message)?
    {
        return Err("Child link identity mismatch".into());
    }
    Ok(Child {
        run: run.into(),
        key: key.into(),
        link,
    })
}

fn transcript_messages(record: &Record) -> Vec<Value> {
    let mut messages: Vec<_> = record.task.history.iter().flatten().collect();
    if let Some(current) = &record.task.status.message {
        if !messages
            .iter()
            .any(|message| message.message_id == current.message_id)
        {
            messages.push(current);
        }
    }
    messages.into_iter().map(|message| {
        let metadata = message.metadata.as_ref();
        json!({
            "id": message.message_id,
            "role": if message.role == protocol::Role::User { "user" } else { "agent" },
            "parts": message.parts,
            "updates": metadata.and_then(|values| values.get("khor.acp_updates")),
            "trace_partial": metadata.is_some_and(|values| values.get("khor.acp_trace_partial") == Some(&Value::Bool(true))),
        })
    }).collect()
}

pub(super) async fn read(node: &Node, board: &str, request: &Value) -> Result<Value, String> {
    if let Some(detail) = request.get("detail") {
        if !detail.is_object() {
            return Err("Invalid child reference".into());
        }
        if !request["action"].is_null() {
            return Err("A child transcript is read-only".into());
        }
        let child = owned_child(
            node,
            board,
            detail["run"].as_str().ok_or("Missing child run")?,
            detail["key"].as_str().ok_or("Missing child key")?,
        )?;
        let task = child
            .link
            .task
            .clone()
            .ok_or("Child has no confirmed task")?;
        let record = call(node, &child, BoundAction::Get { task }).await?;
        let mut result = summary(&child, Some(&record));
        result["messages"] = json!(transcript_messages(&record));
        result["artifacts"] = json!(record.task.artifacts);
        result["terminal"] = json!(record.task.status.state.is_terminal());
        return Ok(result);
    }
    let runs: Vec<_> = node
        .chat
        .log(board)?
        .messages
        .into_iter()
        .rev()
        .filter_map(|message| message.command)
        .filter(|run| {
            run.owner == node.device_str()
                && run.agent.is_some()
                && !run.id.is_empty()
                && run.id.len() <= 128
                && run
                    .id
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        })
        .collect();
    let children = collect(node, board, &runs)?;
    // A list label needs only the parent's retained links. Do not contact
    // every child agent or read command output just to paint that count.
    if request["summary_only"] == true && request["action"].is_null() {
        return Ok(json!({
            "count": children.len(),
            "at_least": children.len() >= PAGE,
        }));
    }
    if let Some(action) = request["action"].as_str() {
        let child = children
            .iter()
            .find(|child| request["run"] == child.run && request["key"] == child.key)
            .ok_or("Child no longer belongs to this conversation")?;
        let task = child
            .link
            .task
            .clone()
            .ok_or("Child has no confirmed task")?;
        let record = call(node, child, BoundAction::Get { task: task.clone() }).await?;
        if !owns_latest_input(child, &record) {
            return Err("The task has moved to a newer input".into());
        }
        match action {
            "cancel" if !record.task.status.state.is_terminal() => {
                if matches!(
                    &child.link.binding.protocol,
                    delegation::BindingProtocol::Native
                ) {
                    let changed = call(node, child, BoundAction::Cancel { task }).await?;
                    return Ok(summary(child, Some(&changed)));
                }
                let parent = child.link.input_parent.clone();
                if parent.is_none() {
                    return Err("The original delegated input could not be identified".into());
                }
                let answer = node
                    .delegation_on(
                        &child.link.machine,
                        delegation::Action::Bound {
                            binding: child.link.binding.clone(),
                            action: BoundAction::CancelInput {
                                message: child.link.message.clone(),
                                parent,
                            },
                        },
                    )
                    .await?;
                match answer {
                    delegation::Answer::Input {
                        resolution:
                            delegation::InputResolution::Known {
                                task: confirmed, ..
                            },
                        ..
                    } if confirmed.id == task => return Ok(json!({"task":task})),
                    _ => return Err("Cancellation could not be confirmed for this input".into()),
                }
            }
            "retry"
                if matches!(
                    record.task.status.state,
                    protocol::TaskState::Canceled | protocol::TaskState::Failed
                ) =>
            {
                let token = request["token"]
                    .as_str()
                    .filter(|token| safe_token(token))
                    .ok_or("Invalid child retry token")?;
                let parent = TaskRef {
                    home: node.device(),
                    id: child.run.clone(),
                };
                let mut message = latest_input(&record)
                    .cloned()
                    .ok_or("Child input was not retained")?;
                message.message_id = token.into();
                message.task_id = None;
                message.context_id = Some(record.task.context_id.clone());
                delegation::track_input(
                    node.root(),
                    &parent,
                    &child.link.machine,
                    &child.link.binding,
                    token,
                    None,
                )?;
                let changed = call(
                    node,
                    child,
                    BoundAction::Send {
                        message,
                        parent: Some(parent.clone()),
                    },
                )
                .await?;
                delegation::track_child(
                    node.root(),
                    &parent,
                    &child.link.machine,
                    &child.link.binding,
                    token,
                    Some(&changed.task.id),
                )?;
                return Ok(json!({"task": changed.task.id}));
            }
            _ => return Err("The task cannot perform this action in its current state".into()),
        }
    }
    let rows: Vec<_> = stream::iter(children)
        .map(|child| async move {
            let record = match child.link.task.clone() {
                Some(task) => tokio::time::timeout(
                    Duration::from_secs(3),
                    call(node, &child, BoundAction::Get { task }),
                )
                .await
                .ok()
                .and_then(Result::ok),
                None => None,
            };
            summary(&child, record.as_ref())
        })
        .buffered(8)
        .collect()
        .await;
    let commands = super::commands::summaries(node, board, &runs)?;
    Ok(json!({"children": rows, "commands": commands}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn child_transcript_is_task_scoped_and_deduplicates_the_current_reply() {
        let scratch = tempfile::tempdir().unwrap();
        let node = Node::open_as(scratch.path().into(), "child-reader").unwrap();
        node.register_agent("fixture-agent", &["must-not-spawn".into()])
            .unwrap();
        let target = node.delegation_target("fixture-agent").unwrap();
        let store = node.delegation_store().unwrap();
        let input = protocol::Message::new(
            protocol::Role::User,
            vec![protocol::Part::text("first task only")],
        );
        let first = store.accept(node.device(), &target, &input, None).unwrap();
        let mut execution =
            delegation::Execution::claim(store.clone(), &first.record.task.id, &input.message_id)
                .unwrap();
        let mut output = protocol::Message::new(
            protocol::Role::Agent,
            vec![protocol::Part::text("first result")],
        );
        output.metadata = Some(std::collections::HashMap::from([
            ("khor.acp_updates".into(), json!(["retained-process"])),
            ("khor.acp_trace_partial".into(), json!(true)),
        ]));
        execution
            .update(protocol::TaskState::Completed, Some(output), None)
            .unwrap();
        drop(execution);
        let record = store
            .get(node.device(), &target, &first.record.task.id)
            .unwrap();
        let messages = transcript_messages(&record);
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0]["role"], "user");
        assert_eq!(messages[0]["parts"][0]["text"], "first task only");
        assert_eq!(messages[1]["updates"], json!(["retained-process"]));
        assert_eq!(messages[1]["trace_partial"], true);
        let mut next = protocol::Message::new(
            protocol::Role::User,
            vec![protocol::Part::text("a later unrelated task")],
        );
        next.context_id = Some(record.task.context_id.clone());
        let second = store.accept(node.device(), &target, &next, None).unwrap();
        assert_ne!(second.record.task.id, record.task.id);
        let original = store.get(node.device(), &target, &record.task.id).unwrap();
        assert_eq!(
            transcript_messages(&original),
            messages,
            "Context reuse never changes the earlier task's transcript"
        );
        assert_eq!(transcript_messages(&second.record).len(), 1);
    }

    #[test]
    fn child_reader_rejects_path_and_unicode_references_without_panicking() {
        let scratch = tempfile::tempdir().unwrap();
        let node = Node::open_as(scratch.path().into(), "child-reader").unwrap();
        for (run, key) in [
            ("../other", format!("{}.json", "a".repeat(64))),
            ("valid-run", "../other.json".into()),
            ("valid-run", format!("a{}{}.json", "中".repeat(21), "")),
        ] {
            assert!(owned_child(&node, "parent", run, &key).is_err());
        }
    }
}
