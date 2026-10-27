//! Opt-in acceptance over an existing isolated vendor conversation. No model turn.
//! KHOR_NATIVE_HANDOFF_HOME must name the private KHOR_HOME used by a probe;
//! KHOR_NATIVE_HANDOFF_SESSION is its full vendor UUID. Never use a live user thread.
use khor_node::{Node, host};
use std::io::Read;
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn main() {
    host::main_if_host(Node::root_from_env());
    let Ok(home) = std::env::var("KHOR_NATIVE_HANDOFF_HOME") else {
        println!("native_handoff ... ignored (requires an isolated probe home)");
        return;
    };
    let root = PathBuf::from(home);
    assert!(
        root.to_string_lossy().contains("khor-acp-audit-"),
        "use an isolated audit root"
    );
    let sid = std::env::var("KHOR_NATIVE_HANDOFF_SESSION").unwrap();
    let vendor = std::env::var("KHOR_NATIVE_HANDOFF_VENDOR").unwrap_or("codex".into());
    assert!(matches!(vendor.as_str(), "codex" | "claude"));
    unsafe {
        std::env::set_var("KHOR_HOME", &root);
        std::env::remove_var("KHOR_VENDOR_HOME");
    }
    let node = Node::open(root.clone()).unwrap();
    let shim = if vendor == "codex" {
        "_codexagent"
    } else {
        "_cagent"
    };
    let cwd = root.parent().unwrap().join("cwd");
    let id = node
        .reopen_gui_as(
            &cwd,
            "native-handoff",
            &[
                std::env::current_exe().unwrap().display().to_string(),
                shim.into(),
            ],
            &vendor,
            &sid,
        )
        .unwrap();
    assert_eq!(id, khor_node::adaptor::id_for(&sid));
    struct Close(Node, khor_core::SessionId);
    impl Drop for Close {
        fn drop(&mut self) {
            let _ = self.0.close(&self.1);
        }
    }
    let clean = Close(node, id.clone());
    let node = &clean.0;
    let dir = node.session_dir(&id).unwrap();
    let original = host::read_host_file(&dir).unwrap();
    node.takeover_term(&id).unwrap();
    let deadline = Instant::now() + Duration::from_secs(20);
    let terminal = loop {
        if let Ok(now) = host::read_host_file(&dir) {
            if now.host_pid != original.host_pid {
                break now;
            }
        }
        assert!(Instant::now() < deadline, "new TUI host never arrived");
        std::thread::sleep(Duration::from_millis(100));
    };
    assert_ne!(terminal.child_pid, original.child_pid);
    // A real TUI asks its attached terminal for cursor/device information.
    // Answer those queries, as xterm does in the GUI, before checking its seat.
    let mut stream = host::connect(&dir, 80, 24).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_millis(100)))
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(4);
    let mut output = Vec::new();
    let mut bytes = [0u8; 8192];
    while Instant::now() < deadline {
        if let Ok(n) = stream.read(&mut bytes) {
            if n == 0 {
                break;
            }
            output.extend_from_slice(&bytes[..n]);
            if output.windows(4).any(|w| w == b"\x1b[6n") {
                host::write_frame(&mut stream, &host::ClientOp::Input(b"\x1b[1;1R".to_vec()))
                    .unwrap();
                output.clear();
            }
        }
    }
    let rows = node.sessions().unwrap();
    assert!(
        rows.iter()
            .any(|r| r.session.id == id && r.session.kind.0 != "gui" && r.session.term),
        "TUI not seated: {:?}; output: {}",
        rows.iter()
            .map(|r| (&r.session.id, &r.session.kind, r.session.dead))
            .collect::<Vec<_>>(),
        String::from_utf8_lossy(&output)
    );
    node.takeover(&id).unwrap();
    assert!(
        node.sessions()
            .unwrap()
            .iter()
            .any(|r| r.session.id == id && r.session.kind.0 == "gui")
    );
    println!("native_handoff {vendor}: GUI -> TUI -> GUI preserved {sid}");
}
