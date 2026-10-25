//! Setup reaches a registered executable, remains root-scoped, and creates no session.
use khor_node::{Node, khor_agent::Action};
use serde_json::{Value, json};
use std::{path::PathBuf, time::Duration};
mod util;
struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn scratch() -> Scratch {
    let path = std::env::temp_dir().join(format!(
        "khor-acp-setup-{}",
        khor_node::link::fresh_hex().unwrap()
    ));
    std::fs::create_dir_all(&path).unwrap();
    util::hermetic(&path);
    Scratch(path)
}
async fn action(node: &Node, request: Value) -> Result<Value, String> {
    node.khor_agent_on(node.device_str(), Action::Manage { request })
        .await
        .map(|r| r.data.unwrap())
}
async fn status(node: &Node, id: &str, want: &str) -> Value {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let state = action(node, json!({"operation":"acp_poll","job":id}))
                .await
                .unwrap();
            if state["status"] == want {
                return state;
            }
            assert_ne!(state["status"], "failed", "{state}");
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
    })
    .await
    .unwrap()
}
#[tokio::test]
async fn setup_can_retry_invalid_answers_cancel_and_refuse_a_different_home() {
    let root = scratch();
    let other_root = scratch();
    let node = Node::open_as(root.0.clone(), "setup").unwrap();
    let other = Node::open_as(other_root.0.clone(), "other").unwrap();
    let stub = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("acp-stub");
    assert!(stub.is_file(), "build khor-acp's test stub first");
    node.register_agent(
        "fixture",
        &[stub.display().to_string(), "--extended".into()],
    )
    .unwrap();
    assert!(
        action(
            &node,
            json!({"operation":"acp_initialize","agent":"not-registered","command":"arbitrary"})
        )
        .await
        .is_err()
    );
    let started = action(
        &node,
        json!({"operation":"acp_authenticate","agent":"fixture","methodId":"login"}),
    )
    .await
    .unwrap();
    let id = started["job"].as_str().unwrap();
    let waiting = status(&node, id, "waiting").await;
    assert!(
        action(&other, json!({"operation":"acp_poll","job":id}))
            .await
            .is_err()
    );
    action(&node,json!({"operation":"acp_reply","job":id,"ask":waiting["ask"],"response":{"action":"accept","content":{"count":99}}})).await.unwrap();
    assert!(status(&node, id, "waiting").await["error"].is_string());
    action(&node,json!({"operation":"acp_reply","job":id,"ask":waiting["ask"],"response":{"action":"accept","content":{"count":2}}})).await.unwrap();
    let done = status(&node, id, "done").await;
    assert_eq!(done["result"]["authMethods"][0]["id"], "login");
    assert!(action(&node,json!({"operation":"acp_reply","job":id,"ask":waiting["ask"],"response":{"action":"accept"}})).await.is_err());
    let started = action(
        &node,
        json!({"operation":"acp_authenticate","agent":"fixture","methodId":"login"}),
    )
    .await
    .unwrap();
    let id = started["job"].as_str().unwrap();
    let _ = status(&node, id, "waiting").await;
    action(&node, json!({"operation":"acp_cancel","job":id}))
        .await
        .unwrap();
    let _ = status(&node, id, "cancelled").await;
}
