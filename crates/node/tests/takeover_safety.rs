//! An unbound native writer must never trigger a partial takeover.
use std::fs::{self, OpenOptions};

#[test]
fn a_shared_codex_writer_is_refused_before_any_host_is_touched() {
    const ID: &str = "019fd755-e464-7aaa-8aaa-aaaaaaaaaaaa";
    let root = std::env::temp_dir().join(format!("khor-takeover-writer-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let sessions = root.join(".codex/sessions/2026/08/06");
    let locks = root.join(".codex/thread-writer-locks");
    fs::create_dir_all(&sessions).unwrap();
    fs::create_dir_all(&locks).unwrap();
    fs::write(sessions.join(format!("rollout-2026-08-06T21-49-18-{ID}.jsonl")),
        format!("{}\n", serde_json::json!({"type":"session_meta","payload":{"id":ID,"cwd":root,"originator":"codex-tui"}}))).unwrap();
    fs::write(locks.join(".coordination.lock"), "").unwrap();
    let writer = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(locks.join(format!("{ID}.lock")))
        .unwrap();
    writer.lock().unwrap();
    let node = khor_node::Node::open(root.clone()).unwrap();
    let id = khor_node::adaptor::id_for(ID);
    assert_eq!(
        node.takeover(&id).unwrap_err(),
        khor_catalog::msg::takeover_writer_busy(&id.0)
    );
    assert_eq!(
        node.reopen_gui_as(&root, "held thread", &["unused".into()], "codex", ID)
            .unwrap_err(),
        khor_catalog::msg::takeover_writer_busy(ID)
    );
    // A separately opened handle still cannot own this writer afterwards.
    let observer = OpenOptions::new()
        .read(true)
        .open(locks.join(format!("{ID}.lock")))
        .unwrap();
    assert!(matches!(
        observer.try_lock_shared(),
        Err(std::fs::TryLockError::WouldBlock)
    ));
    drop(node);
    drop(writer);
    let _ = fs::remove_dir_all(root);
}
