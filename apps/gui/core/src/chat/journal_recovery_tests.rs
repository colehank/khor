use super::*;
use std::{
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
struct Worker(Child);
impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn no_load_core_views_read_canonical_fences_and_reopen_without_agent_replay() {
    let scratch = Scratch(std::env::temp_dir().join(format!(
        "khor-core-journal-{}",
        khor_node::link::fresh_hex().unwrap()
    )));
    std::fs::create_dir(&scratch.0).unwrap();
    let mut worker = Worker(
        Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "chat::journal_recovery_tests::no_load_core_worker",
                "--ignored",
                "--nocapture",
            ])
            .env("KHOR_ACP_STOP_SCRATCH", &scratch.0)
            .env("KHOR_GHOST_VENDOR", "acp:core-capture-fixture")
            .env_remove("KHOR_GHOST_RESUME")
            .env("TMUX_TMPDIR", &scratch.0)
            .env("KHOR_TMUX_SOCKET", "private-core-journal")
            .env_remove("TMUX")
            .env_remove("TMUX_PANE")
            .current_dir(&scratch.0)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(45);
    let status = loop {
        if let Some(status) = worker.0.try_wait().unwrap() {
            break status;
        }
        assert!(
            Instant::now() < deadline,
            "private Core journal worker exceeded 45 seconds"
        );
        std::thread::sleep(Duration::from_millis(5));
    };
    use std::io::Read;
    let mut stdout = String::new();
    let mut stderr = String::new();
    worker
        .0
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut stdout)
        .unwrap();
    worker
        .0
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .unwrap();
    assert!(
        status.success(),
        "Core journal worker failed: {stdout}\n{stderr}"
    );
    assert!(
        stdout.contains("test result: ok. 1 passed; 0 failed; 0 ignored;"),
        "private worker must execute exactly one test: {stdout}"
    );
    print!("{stdout}");
}

fn until<T>(mut run: impl FnMut() -> Option<T>) -> T {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(value) = run() {
            return value;
        }
        assert!(
            Instant::now() < deadline,
            "expected Core host fact did not arrive"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}
fn poll_until(
    root: &Path,
    id: &str,
    lease: &str,
    mut run: impl FnMut(&ChatFrame) -> bool,
) -> Vec<ChatFrame> {
    until(|| {
        let frames = chat_poll(root, id, lease, 0).unwrap().frames;
        frames.iter().any(&mut run).then_some(frames)
    })
}
fn capture(root: &Path, id: &str, lease: &str, token: &str) -> (String, String) {
    chat_capture(root, id, lease, token).unwrap();
    let frames = poll_until(
        root,
        id,
        lease,
        |frame| matches!(frame,ChatFrame::Captured {token:actual,..} if actual==token),
    );
    frames
        .into_iter()
        .find_map(|frame| match frame {
            ChatFrame::Captured {
                token: actual,
                result: CapturedContent::Available { binding, cursor },
            } if actual == token => Some((binding, cursor)),
            _ => None,
        })
        .expect("capture must return an available canonical source")
}
async fn page(root: &Path, id: &str, fence: &(String, String), newer: bool) -> HistoryPage {
    let key = history_begin(root, id, &fence.0).await.unwrap();
    history_page(root, id, &fence.0, Some(&fence.1), newer, &key)
        .await
        .unwrap()
}
fn chunks(page: &HistoryPage) -> usize {
    page.entries.iter().filter(|entry|matches!(&entry.frame,ChatFrame::Note {update} if update["update"]["content"]["text"]=="capture identical chunk")).count()
}

#[test]
#[ignore = "isolated real Core attachment and NoLoad ACP host worker"]
fn no_load_core_worker() {
    let scratch = PathBuf::from(std::env::var_os("KHOR_ACP_STOP_SCRATCH").unwrap());
    let root = scratch.join("store");
    std::fs::create_dir(&root).unwrap();
    let agent = scratch.join("agent.py");
    std::fs::write(
        &agent,
        include_str!("../../../../../crates/node/src/fixtures/acp-cancel.py"),
    )
    .unwrap();
    let marker = scratch.join("journal-no-load");
    let ready = scratch.join("core.ready");
    let command = serde_json::json!({"command":"python3","args":[agent,"journal-no-load",marker]})
        .to_string();
    let host = {
        let root = root.clone();
        let ready = ready.clone();
        std::thread::spawn(move || {
            khor_node::gui_host::gui_host_main(
                root,
                ready,
                "private Core journal".into(),
                vec![command],
            )
        })
    };
    let id = until(|| std::fs::read_to_string(&ready).ok())
        .trim()
        .to_owned();
    let a = begin_view(&root, &id).unwrap();
    assert!(chat_open(&root, &id, &a).unwrap());
    let b = begin_view(&root, &id).unwrap();
    assert!(!chat_open(&root, &id, &b).unwrap());
    let facts = poll_until(&root, &id, &a, |f| matches!(f, ChatFrame::Agent { .. }));
    assert!(
        facts
            .iter()
            .any(|f| matches!(f, ChatFrame::Agent { replays: false, .. })),
        "fixture must truly be NoLoad"
    );
    let client_token = "12345678-1234-1234-1234-123456789abc";
    let first = submit_say_with_token(
        &root,
        &id,
        &a,
        "same accepted text".into(),
        vec![("image/png".into(), "i".repeat(150_000))],
        vec![],
        Some(client_token.into()),
    )
    .unwrap();
    first.wait().unwrap();
    poll_until(
        &root,
        &id,
        &a,
        |f| matches!(f,ChatFrame::Turn {stop} if stop=="EndTurn"),
    );
    let fence_a = capture(&root, &id, &a, "0000000000000000000000000000000a");
    let before = chat_poll(&root, &id, &a, 0).unwrap().next;
    submit_say_with_token(
        &root,
        &id,
        &b,
        "same accepted text".into(),
        vec![],
        vec![],
        Some(client_token.into()),
    )
    .unwrap()
    .wait()
    .unwrap();
    until(|| {
        chat_poll(&root, &id, &b, before)
            .unwrap()
            .frames
            .iter()
            .any(|f| matches!(f,ChatFrame::Turn {stop} if stop=="EndTurn"))
            .then_some(())
    });
    let fence_b = capture(&root, &id, &b, "0000000000000000000000000000000b");
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        let _admission = super::history_bridge_tests::AdmissionTransport::start(&root).await;
        let old = page(&root, &id, &fence_a, false).await;
        assert_eq!(chunks(&old), 1);
        let appended = page(&root, &id, &fence_a, true).await;
        assert_eq!(chunks(&appended), 1);
        let all = page(&root, &id, &fence_b, false).await;
        assert_eq!(chunks(&all), 2);
        let origins: Vec<_> = all
            .entries
            .iter()
            .filter_map(|entry| match &entry.frame {
                ChatFrame::PromptRecorded {
                    turn,
                    token,
                    start,
                    end,
                } => Some((turn, token, *start, *end)),
                _ => None,
            })
            .collect();
        assert_eq!(origins.len(), 2);
        assert_ne!(origins[0].0, origins[1].0);
        assert!(origins.iter().all(|origin| origin.1 == client_token));
        let origin = origins
            .iter()
            .find(|origin| origin.3 - origin.2 > 64 * 1024)
            .unwrap();
        let raw = all
            .references
            .iter()
            .find(|r| {
                r.start == origin.2 && r.end == origin.3 && !r.missing_start && !r.missing_end
            })
            .expect("small accepted origin must name the full oversized prompt range");
        let key = history_begin(&root, &id, &fence_b.0).await.unwrap();
        let first = history_record(&root, &id, &fence_b.0, &raw.cursor, raw.start, &key)
            .await
            .unwrap();
        use base64::Engine as _;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(first.data)
            .unwrap();
        assert!(bytes.len() <= 64 * 1024);
        assert!(
            std::str::from_utf8(&bytes)
                .unwrap()
                .starts_with("{\"kind\":\"prompt\"")
        );
        chat_leave(&root, &id, &a).unwrap();
        chat_leave(&root, &id, &b).unwrap();
        let c = begin_view(&root, &id).unwrap();
        assert!(chat_open(&root, &id, &c).unwrap());
        let reopened = capture(&root, &id, &c, "0000000000000000000000000000000c");
        assert_eq!(chunks(&page(&root, &id, &reopened, false).await), 2);
        chat_leave(&root, &id, &c).unwrap();
    });
    let node = Node::open(root.clone()).unwrap();
    let dir = node.session_dir(&SessionId(id)).unwrap();
    let mut closer = khor_node::gui_host::connect(&dir).unwrap().stream;
    write_frame(&mut closer, &GuiOp::Close).unwrap();
    while !matches!(read_frame::<GuiNote>(&mut closer).unwrap(), GuiNote::Gone) {}
    host.join().unwrap().unwrap();
    let received = std::fs::read_to_string(marker.join("received.jsonl")).unwrap();
    assert!(
        !received.contains("session/load") && !received.contains("session/resume"),
        "canonical recovery must not call agent history/control"
    );
}
