//! Real peer/IPC routing with a second home as the negative control.
mod util;
use khor_node::{
    Node,
    recognition::{Action, Agent},
};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::time::timeout;
struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn root(tag: &str) -> Scratch {
    let p = std::env::temp_dir().join(format!(
        "khor-recognition-wire-{tag}-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(p.join(".codex")).unwrap();
    std::fs::create_dir_all(p.join(".claude")).unwrap();
    util::hermetic(&p);
    Scratch(p)
}
struct Server(tokio::task::JoinHandle<Result<(), String>>);
impl Drop for Server {
    fn drop(&mut self) {
        self.0.abort();
    }
}
async fn serving(root: &Path, name: &str) -> Server {
    let n = Node::open_as(root.to_path_buf(), name).unwrap();
    let task = Server(tokio::spawn(async move { n.serve().await }));
    timeout(Duration::from_secs(90), async {
        while !root.join(".khor/endpoint.json").exists() {
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
    })
    .await
    .unwrap();
    task
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn recognition_is_changed_only_on_the_selected_peer_and_reads_actual_files() {
    let ra = root("a");
    let rb = root("b");
    let _server = serving(&ra.0, "alpha").await;
    let a = Node::open_as(ra.0.clone(), "alpha").unwrap();
    let b = Node::open_as(rb.0.clone(), "beta").unwrap();
    timeout(Duration::from_secs(45), b.pair(&a.invite().unwrap()))
        .await
        .unwrap()
        .unwrap();
    // Client also has a resident: mutations take IPC -> authenticated peer wire.
    let _client = serving(&rb.0, "beta").await;
    let pc = ra.0.join(".codex/config.toml");
    std::fs::write(&pc, "model='fixture'\n[tui]\nterminal_title=['project']\n").unwrap();
    let target = a
        .devices()
        .unwrap()
        .into_iter()
        .find(|d| d.name == "alpha")
        .unwrap()
        .id;
    let ask = |action| b.recognition_on(&target, action);
    let before = ask(Action::Read).await.unwrap();
    assert_eq!(before.entries[1].state, "automatic");
    let after = ask(Action::Enable(Agent::Codex)).await.unwrap();
    assert_eq!(after.entries[1].state, "configured");
    assert!(after.entries[1].managed);
    assert!(
        !rb.0.join(".codex/config.toml").exists(),
        "must not configure the caller"
    );
    assert!(std::fs::read_to_string(&pc).unwrap().contains("thread-id"));
    std::fs::write(&pc, "[tui]\nterminal_title=[]\n").unwrap();
    assert_eq!(ask(Action::Read).await.unwrap().entries[1].state, "changed");
    ask(Action::Disable(Agent::Codex)).await.unwrap();
    assert_eq!(
        std::fs::read_to_string(&pc).unwrap(),
        "[tui]\nterminal_title=[]\n"
    );
    ask(Action::Enable(Agent::Claude)).await.unwrap();
    assert!(!rb.0.join(".claude/settings.json").exists());
    assert_eq!(
        ask(Action::Read).await.unwrap().entries[0].state,
        "configured"
    );
    ask(Action::Disable(Agent::Claude)).await.unwrap();
    assert_eq!(
        ask(Action::Read).await.unwrap().entries[0].state,
        "automatic"
    );
    assert!(
        b.recognition_on("", Action::Enable(Agent::Codex))
            .await
            .is_err()
    );
    // Mutations must neither replay automatically nor ride unauthenticated 0-RTT.
    let mutation = khor_node::proto::Request::Recognition {
        action: Action::Enable(Agent::Codex),
    };
    assert!(!mutation.rides_early());
    let info = b
        .devices()
        .unwrap()
        .into_iter()
        .find(|d| d.id == target)
        .unwrap();
    let before = std::fs::read_to_string(&pc).unwrap();
    let answer = timeout(
        Duration::from_secs(60),
        util::raw_request(
            iroh::SecretKey::generate(),
            &info.id,
            &info.addrs,
            &mutation,
        ),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(
        matches!(answer, khor_node::proto::Response::Refused { .. }),
        "an unpaired machine cannot install config"
    );
    assert_eq!(std::fs::read_to_string(&pc).unwrap(), before);
}
