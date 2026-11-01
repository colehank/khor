use std::{path::PathBuf, sync::{Arc, Mutex, atomic::{AtomicU64, Ordering}}};
use khor_node::{Node, ipc::{Handoff, Op, Reply}, watch::{Batch, Opened}};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let root = std::env::temp_dir().join(format!("khor-core-watch-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
        std::fs::create_dir_all(&root).unwrap();
        Self(root)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); }
}
struct Server(Option<tokio::task::JoinHandle<()>>);
impl Drop for Server {
    fn drop(&mut self) { if let Some(task) = &self.0 { task.abort(); } }
}
fn batch() -> Batch {
    Batch { cursor: "private-epoch:7".into(), reset: false, dirs: vec![], workspace: false, coverage: vec![], retry_ms: 1000 }
}
fn answer(rt: &tokio::runtime::Runtime, root: &std::path::Path, cmd: &str, args: serde_json::Value) -> Result<serde_json::Value, String> {
    serde_json::from_str(&super::answer(rt, root, cmd, &args)?).map_err(|error| error.to_string())
}

#[test]
fn http_watch_uses_pinned_identity_and_never_holds_the_serial_bridge() {
    let scratch = Scratch::new();
    let node = Node::open(scratch.0.clone()).unwrap();
    let machine_id = node.device().hex();
    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let mut server = rt.block_on(async {
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        std::fs::write(scratch.0.join(".khor/endpoint.json"), serde_json::to_vec(&serde_json::json!({
            "id":machine_id,"addrs":[],"pid":std::process::id(),"ipc_port":listener.local_addr().unwrap().port(),"ipc_cookie":"private-watch-cookie"
        })).unwrap()).unwrap();
        let calls = calls.clone();
        let pinned = machine_id.clone();
        Server(Some(tokio::spawn(async move {
            for _ in 0..3 {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut bytes = Vec::new();
                (&mut socket).take(khor_node::proto::MAX_FRAME as u64).read_to_end(&mut bytes).await.unwrap();
                let handoff: Handoff = khor_node::proto::decode(&bytes).unwrap();
                assert_eq!(handoff.cookie, "private-watch-cookie");
                let reply = match &handoff.op {
                    Op::WatchBegin { .. } => Reply::WatchOpened { opened: Opened { id: "private-lease".into(), machine_id: pinned.clone(), batch: batch() } },
                    Op::WatchWait { .. } => Reply::WatchChanged { batch: batch() },
                    Op::WatchLeave { .. } => Reply::WatchLeft,
                    _ => panic!("fixture does not answer unrelated commands"),
                };
                calls.lock().unwrap().push(handoff.op);
                socket.write_all(&khor_node::proto::encode(&reply).unwrap()).await.unwrap();
                socket.shutdown().await.unwrap();
            }
        })))
    });
    let opened = answer(&rt, &scratch.0, "dirs_begin", serde_json::json!({
        "machine":"original label","scopes":[{"kind":"workspace","root":"/private/workspace"},{"kind":"entries","path":"/private/entries"}]
    })).unwrap();
    assert_eq!(opened["machine_id"], machine_id);
    assert_eq!(opened["id"], "private-lease");
    assert_eq!(opened["batch"]["retry_ms"], 1000);
    let current = answer(&rt, &scratch.0, "dirs_wait", serde_json::json!({
        "machine_id":machine_id,"id":"private-lease","cursor":"private-epoch:7","wait_ms":20_000
    })).unwrap();
    assert_eq!(current["cursor"], "private-epoch:7");
    assert_eq!(current["retry_ms"], 1000);
    assert_eq!(answer(&rt, &scratch.0, "dirs_leave", serde_json::json!({"machine_id":machine_id,"id":"private-lease"})).unwrap(), serde_json::Value::Null);
    let calls = calls.lock().unwrap();
    assert_eq!(calls.len(), 3);
    assert!(matches!(&calls[0], Op::WatchBegin { machine, scopes } if machine=="original label" && matches!(scopes.as_slice(), [khor_node::watch::Scope::Workspace { root }, khor_node::watch::Scope::Entries { path }] if root=="/private/workspace" && path=="/private/entries")));
    assert!(matches!(&calls[1], Op::WatchWait { machine_id: pin, id, cursor: Some(cursor), wait_ms: 0 } if pin==&machine_id && id=="private-lease" && cursor=="private-epoch:7"));
    assert!(matches!(&calls[2], Op::WatchLeave { machine_id: pin, id } if pin==&machine_id && id=="private-lease"));
    drop(calls);
    rt.block_on(async {
        tokio::time::timeout(std::time::Duration::from_secs(5), server.0.take().unwrap()).await.unwrap().unwrap();
    });
}

#[test]
fn malformed_watch_inputs_are_rejected_before_any_resident_io() {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    let scratch = Scratch::new();
    let root = scratch.0.join("must-remain-unopened");
    for wait in [serde_json::json!(-1), serde_json::json!(1.5), serde_json::json!(u32::MAX as u64 + 1), serde_json::Value::Null] {
        assert!(answer(&rt, &root, "dirs_wait", serde_json::json!({"machine_id":"invalid","id":"unused","cursor":null,"wait_ms":wait})).unwrap_err().contains("wait_ms"));
    }
    assert!(answer(&rt, &root, "dirs_wait", serde_json::json!({"machine_id":"invalid","id":"unused","cursor":7,"wait_ms":20_000})).unwrap_err().contains("cursor"));
    assert!(answer(&rt, &root, "dirs_begin", serde_json::json!({"machine":"unused","scopes":[{"kind":"unknown"}]})).unwrap_err().contains("unknown"));
    assert!(!root.exists(), "malformed payloads reached Node::open");
}
