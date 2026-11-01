use super::*;
use khor_core::{DeviceId, Millis, ObservationNamespace, ObservationRef};
use khor_node::session_content::{ContentLocator, ContentReference, ReadBinding};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn isolate_terminal_probes() {
    static READY: OnceLock<()> = OnceLock::new();
    READY.get_or_init(|| {
        let path = std::env::temp_dir().join("khor-core-readonly-tmux");
        std::fs::create_dir_all(&path).unwrap();
        // This test executable may inspect only its own empty terminal domain.
        unsafe {
            std::env::set_var("TMUX_TMPDIR", path);
            std::env::set_var("KHOR_TMUX_SOCKET", "khor-core-readonly");
            std::env::remove_var("TMUX");
            std::env::remove_var("TMUX_PANE");
        }
    });
}
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let root = std::env::temp_dir().join(format!(
            "khor-core-content-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&root).unwrap();
        Self(root)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
pub(super) struct AdmissionTransport(Option<tokio::task::JoinHandle<()>>);
impl AdmissionTransport {
    pub(super) async fn start(root: &Path) -> Self {
        let node = Arc::new(Node::open(root.to_path_buf()).unwrap());
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .unwrap();
        let port = listener.local_addr().unwrap().port();
        std::fs::write(
            root.join(".khor/endpoint.json"),
            serde_json::to_vec(&serde_json::json!({
                "id":node.device().hex(),"addrs":[],"pid":std::process::id(),
                "ipc_port":port,"ipc_cookie":"private-readonly-test-cookie"
            }))
            .unwrap(),
        )
        .unwrap();
        Self(Some(tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                let node = node.clone();
                tokio::spawn(async move {
                    let mut bytes = Vec::new();
                    (&mut socket)
                        .take(khor_node::proto::MAX_FRAME as u64)
                        .read_to_end(&mut bytes)
                        .await
                        .unwrap();
                    let handoff: khor_node::ipc::Handoff =
                        khor_node::proto::decode(&bytes).unwrap();
                    let reply = match handoff.op {
                        khor_node::ipc::Op::ContentRead { action }
                            if handoff.cookie == "private-readonly-test-cookie"
                                && matches!(
                                    action,
                                    khor_node::session_content::remote::Action::Begin { .. }
                                        | khor_node::session_content::remote::Action::Cancel { .. }
                                ) =>
                        {
                            match node.content_read(action).await {
                                Ok(reply) => khor_node::ipc::Reply::ContentRead { reply },
                                Err(why) => khor_node::ipc::Reply::Refused { why },
                            }
                        }
                        _ => khor_node::ipc::Reply::Refused {
                            why: "fixture only admits readonly begin/cancel".into(),
                        },
                    };
                    socket
                        .write_all(&khor_node::proto::encode(&reply).unwrap())
                        .await
                        .unwrap();
                    socket.shutdown().await.unwrap();
                });
            }
        })))
    }
}
impl Drop for AdmissionTransport {
    fn drop(&mut self) {
        if let Some(task) = self.0.take() {
            task.abort();
        }
    }
}
fn api(
    rt: &tokio::runtime::Runtime,
    root: &Path,
    cmd: &str,
    args: serde_json::Value,
) -> Result<serde_json::Value, String> {
    serde_json::from_str(&crate::api::answer(rt, root, cmd, &args)?)
        .map_err(|error| error.to_string())
}
fn message(text: &str) -> Vec<u8> {
    let mut bytes = serde_json::to_vec(&serde_json::json!({"type":"event_msg","payload":{
        "type":"agent_message","message":text}}))
    .unwrap();
    bytes.push(b'\n');
    bytes
}

#[test]
fn readonly_bridge_uses_locator_binding_and_the_node_lease_owner() {
    use base64::Engine as _;
    isolate_terminal_probes();
    let root = Scratch::new();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let transport = rt.block_on(AdmissionTransport::start(&root.0));
    let node = Node::open(root.0.clone()).unwrap();
    let native = "01a17c1a-a542-7822-b01d-82a832a158e6";
    let path = root.0.join(format!(
        ".codex/sessions/2026/10/29/rollout-2026-10-29T12-00-00-{native}.jsonl"
    ));
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let large = message(&"中🙂".repeat(22000));
    let mut content = message("through the real core projection");
    content.extend_from_slice(&large);
    std::fs::write(&path, content).unwrap();
    let id = "observe/core-with-no-alias";
    let locator = ContentLocator::observed(
        SessionId(id.into()),
        &ObservationRef {
            owner: DeviceId([77; 32]),
            home: node.device(),
            namespace: ObservationNamespace::Native,
            agent: "codex".into(),
            native_id: native.into(),
            imported_at: Millis(1),
            updated_at: None,
        },
    )
    .unwrap()
    .encode()
    .unwrap();
    let capability = api(
        &rt,
        &root.0,
        "session_content",
        serde_json::json!({"id":id,"locator":locator}),
    )
    .unwrap();
    assert_eq!(capability["kind"], "available");
    let binding = capability["binding"].as_str().unwrap();
    let key = api(
        &rt,
        &root.0,
        "history_begin",
        serde_json::json!({"id":id,"binding":binding}),
    )
    .unwrap();
    let page = api(
        &rt,
        &root.0,
        "history_page",
        serde_json::json!({"id":id,"binding":binding,
        "requestKey":key,"cursor":null,"newer":false}),
    )
    .unwrap();
    assert!(page["encoded_bytes"].as_u64().unwrap() <= 512 * 1024);
    assert!(page["entries"].as_array().unwrap().iter().any(
        |entry| entry["frame"]["update"]["update"]["content"]["text"]
            == "through the real core projection"
    ));
    let reference = &page["references"][0];
    let key = api(
        &rt,
        &root.0,
        "history_begin",
        serde_json::json!({"id":id,"binding":binding}),
    )
    .unwrap();
    let chunk = api(
        &rt,
        &root.0,
        "history_record",
        serde_json::json!({"id":id,"binding":binding,
        "requestKey":key,"cursor":reference["cursor"],"from":reference["start"]}),
    )
    .unwrap();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(chunk["data"].as_str().unwrap())
        .unwrap();
    assert_eq!(bytes.as_slice(), &large[..64 * 1024]);
    let key = api(
        &rt,
        &root.0,
        "history_begin",
        serde_json::json!({"id":id,"binding":binding}),
    )
    .unwrap();
    api(
        &rt,
        &root.0,
        "history_cancel",
        serde_json::json!({"id":id,"binding":binding,"requestKey":key}),
    )
    .unwrap();
    let error = api(
        &rt,
        &root.0,
        "history_page",
        serde_json::json!({"id":id,"binding":binding,
        "requestKey":key,"cursor":null,"newer":false}),
    )
    .unwrap_err();
    assert!(error.contains("cancelled or expired"), "{error}");
    drop(transport);
}

#[test]
fn malformed_or_cross_view_selectors_fail_before_opening_a_node() {
    isolate_terminal_probes();
    let root = Scratch::new();
    let untouched = root.0.join("never-opened");
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let binding = ReadBinding::new(
        SessionId("original".into()),
        ContentReference {
            home: DeviceId([2; 32]),
            namespace: ObservationNamespace::Acp,
            agent: "any-agent".into(),
            native_id: "opaque/source".into(),
        },
    )
    .unwrap()
    .encode()
    .unwrap();
    let error = api(
        &rt,
        &untouched,
        "history_begin",
        serde_json::json!({"id":"another","binding":binding}),
    )
    .unwrap_err();
    assert!(error.contains("another view"), "{error}");
    let error = api(
        &rt,
        &untouched,
        "session_content",
        serde_json::json!({"id":"original","locator":"invalid"}),
    )
    .unwrap_err();
    assert!(error.contains("invalid content reference"), "{error}");
    assert!(
        !untouched.exists(),
        "invalid selectors must not create/reopen Node state"
    );
}
