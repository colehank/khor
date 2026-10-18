//! Exact rendered receipts, through the same node and API used by the GUI.
use khor_node::{CommandRecord, Node, SeenMessage, SessionId};
use std::{fs, path::PathBuf};
static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "khor-seen-snapshot-{}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn command(node: &Node, id: &str, at: i64) -> CommandRecord {
    CommandRecord {
        id: format!("cmd-test-{id}-12345678"),
        line: "/version".into(),
        owner: node.device_str().into(),
        owner_name: node.name().into(),
        status: "running".into(),
        started_at_ms: at,
        ended_at_ms: None,
        stdout: String::new(),
        stderr: String::new(),
        data_json: None,
        session: None,
        error: None,
    }
}
fn row(node: &Node) -> khor_node::SessionView {
    node.sessions()
        .unwrap()
        .into_iter()
        .find(|r| r.session.id.0 == khor_node::wide::BROADCAST)
        .unwrap()
}
fn version(record: &CommandRecord) -> SeenMessage {
    SeenMessage {
        id: record.id.clone(),
        at_ms: record.ended_at_ms.unwrap_or(record.started_at_ms),
    }
}

#[test]
fn same_millisecond_later_message_stays_unread_and_cli_seen_still_clears_all() {
    let scratch = Scratch::new();
    let node = Node::open_as(scratch.0.clone(), "snapshot-test").unwrap();
    let board = SessionId(khor_node::wide::BROADCAST.into());
    let first = command(&node, "first", 100);
    node.record_command(&first).unwrap();
    let rendered = vec![version(&first)];
    let later = command(&node, "later", 100);
    node.record_command(&later).unwrap();
    khor_gui_core::seen_snapshot(&scratch.0, &board.0, &rendered).unwrap();
    assert_eq!(row(&node).session.unread, 1);
    node.seen_snapshot(&board, &[version(&later)]).unwrap();
    assert_eq!(row(&node).session.unread, 0);
    let third = command(&node, "third", 101);
    node.record_command(&third).unwrap();
    assert_eq!(row(&node).session.unread, 1);
    node.seen(&board).unwrap();
    assert_eq!(
        row(&node).session.unread,
        0,
        "CLI seen retains broad watermark semantics"
    );
}

#[test]
fn same_id_new_command_result_requires_its_own_rendered_receipt() {
    let scratch = Scratch::new();
    let node = Node::open_as(scratch.0.clone(), "snapshot-test").unwrap();
    let board = SessionId(khor_node::wide::BROADCAST.into());
    let mut command = command(&node, "updated", 100);
    node.record_command(&command).unwrap();
    node.seen_snapshot(&board, &[version(&command)]).unwrap();
    assert_eq!(row(&node).session.unread, 0);
    let stale = version(&command);
    command.status = "done".into();
    command.ended_at_ms = Some(101);
    command.stdout = "new result".into();
    node.record_command(&command).unwrap();
    node.seen_snapshot(&board, &[stale]).unwrap();
    assert_eq!(
        row(&node).session.unread,
        1,
        "an old rendered version must not acknowledge new output"
    );
    node.seen_snapshot(&board, &[version(&command)]).unwrap();
    assert_eq!(row(&node).session.unread, 0);
}

#[test]
fn malformed_snapshot_is_atomic_and_cannot_pre_acknowledge_a_future_update() {
    let scratch = Scratch::new();
    let node = Node::open_as(scratch.0.clone(), "snapshot-test").unwrap();
    let board = SessionId(khor_node::wide::BROADCAST.into());
    let command = command(&node, "validation", 100);
    node.record_command(&command).unwrap();
    let valid = version(&command);
    let mut future = valid.clone();
    future.at_ms += 1;
    assert!(
        node.seen_snapshot(&board, &[valid.clone(), future])
            .is_err()
    );
    assert_eq!(
        row(&node).session.unread,
        1,
        "all versions must validate before any mark is written"
    );
    assert!(
        node.seen_snapshot(
            &board,
            &[SeenMessage {
                id: "missing".into(),
                at_ms: 100
            }]
        )
        .is_err()
    );
    assert!(
        node.seen_snapshot(
            &board,
            &[SeenMessage {
                id: valid.id,
                at_ms: -1
            }]
        )
        .is_err()
    );
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = khor_gui_core::api::answer(
        &rt,
        &scratch.0,
        "seen_snapshot",
        &serde_json::json!({"id": board.0, "messages": [version(&command)]}),
    );
    assert_eq!(result.unwrap(), "null");
    assert_eq!(row(&node).session.unread, 0);
}

#[test]
fn failed_and_interrupted_receipts_keep_distinct_row_states() {
    for (status, state) in [
        ("failed", khor_core::State::Failed),
        ("interrupted", khor_core::State::Errored),
    ] {
        let scratch = Scratch::new();
        let node = Node::open_as(scratch.0.clone(), "snapshot-test").unwrap();
        let mut command = command(&node, status, 100);
        command.status = status.into();
        command.ended_at_ms = Some(101);
        node.record_command(&command).unwrap();
        assert_eq!(row(&node).session.state.state, state);
    }
}

#[test]
fn persisted_note_succeeds_when_read_acknowledgement_storage_fails() {
    let scratch = Scratch::new();
    let node = Node::open_as(scratch.0.clone(), "snapshot-test").unwrap();
    let seen = scratch.0.join(".khor/seen");
    if seen.is_dir() {
        fs::remove_dir_all(&seen).unwrap();
    }
    fs::write(&seen, "not a directory").unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let id = rt.block_on(node.note("accepted note", &[])).unwrap();
    let log = node.log(khor_node::wide::BROADCAST).unwrap();
    assert_eq!(log.messages.len(), 1);
    assert_eq!(log.messages[0].id, id);
    assert_eq!(
        log.messages[0].body,
        khor_node::MsgBody::Text("accepted note".into())
    );
}

#[test]
fn gui_seen_api_preserves_the_observed_row_timestamp() {
    let scratch = Scratch::new();
    let node = Node::open_as(scratch.0.clone(), "bounded-row-test").unwrap();
    node.record_command(&command(&node, "observed-row", 100)).unwrap();
    node.record_command(&command(&node, "arrived-in-flight", 200)).unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let args = serde_json::json!({"id": khor_node::wide::BROADCAST, "at_ms": 100});
    khor_gui_core::api::answer(&rt, &scratch.0, "seen", &args).unwrap();
    assert_eq!(row(&node).session.unread, 1, "HTTP must pass the timestamp through to the node");
    for at in [serde_json::json!(-1), serde_json::json!("200"), serde_json::json!(1.5)] {
        let args = serde_json::json!({"id": khor_node::wide::BROADCAST, "at_ms": at});
        assert!(khor_gui_core::api::answer(&rt, &scratch.0, "seen", &args).is_err());
    }
    let old_client = serde_json::json!({"id": khor_node::wide::BROADCAST});
    khor_gui_core::api::answer(&rt, &scratch.0, "seen", &old_client).unwrap();
    assert_eq!(row(&node).session.unread, 0, "older clients retain their broad mark semantics");
}
