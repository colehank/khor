//! The codex shim's scheduler half (docs/AGENT.md), hermetically: with
//! `KHOR_AGENT` set by the opener, `thread/start` must carry khor's MCP
//! server in the config override and the brief in
//! `developerInstructions` — the probed doors (`codexagent`'s module
//! head). Its own binary because the flag is process env, and the
//! ordinary-session test asserts its absence.

const SID: &str = "01a01e00-0000-7000-8000-000000000043";

#[tokio::test]
async fn a_scheduler_session_carries_khor_tools_into_thread_start() {
    let dir = std::env::temp_dir().join(format!("khor-codexsched-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("cwd")).unwrap();

    let fake = dir.join("codex.py");
    std::fs::write(
        &fake,
        r#"#!/usr/bin/env python3
import json, sys
SID = "01a01e00-0000-7000-8000-000000000043"
def emit(obj):
    sys.stdout.write(json.dumps(obj) + "\n")
    sys.stdout.flush()
for line in sys.stdin:
    m = json.loads(line)
    method = m.get("method")
    if method == "initialize":
        emit({"jsonrpc": "2.0", "id": m["id"], "result": {}})
    elif method == "thread/start":
        open("fake.params", "w").write(json.dumps(m["params"]))
        emit({"jsonrpc": "2.0", "id": m["id"], "result": {"thread": {"id": SID}}})
"#,
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    unsafe {
        std::env::set_var("KHOR_HOME", dir.join("home"));
        std::env::set_var("KHOR_CODEX", &fake);
        std::env::set_var("KHOR_AGENT", "1");
    }

    let exe = env!("CARGO_BIN_EXE_khor");
    let (handle, _events) =
        khor_acp::start(&format!("{exe} _codexagent"), dir.join("cwd")).await.unwrap();
    assert_eq!(handle.session().0.to_string(), SID);

    let params: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("cwd/fake.params")).unwrap())
            .unwrap();
    let khor = &params["config"]["mcp_servers"]["khor"];
    assert_eq!(khor["args"], serde_json::json!(["mcp"]), "khor's verbs as tools: {params}");
    assert!(
        khor["env"]["KHOR_HOME"].as_str().is_some_and(|h| !h.is_empty()),
        "the store is named, not inherited: {params}"
    );
    assert!(
        params["developerInstructions"].as_str().is_some_and(|b| !b.is_empty()),
        "the brief must ride developerInstructions: {params}"
    );
    drop(handle);
    let _ = std::fs::remove_dir_all(&dir);
}
