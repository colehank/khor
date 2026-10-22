//! Real peers and resident IPC; the peer process owns a private HOME and fake
//! Codex login. No operator credentials, mDNS, or vendor HTTP are used.
mod util;

use khor_node::{
    Node,
    subscriptions::{Action, Answer},
};
use std::{
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::Duration,
};
use tokio::time::timeout;

struct Fixture {
    root: PathBuf,
    peer: Option<Child>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(child) = &mut self.peer {
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

async fn listening(root: &Path) {
    timeout(Duration::from_secs(90), async {
        while !root.join(".khor/endpoint.json").exists() {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("fixture resident did not start");
}

#[test]
#[ignore = "subprocess fixture, started by network_subscription_import"]
fn subscription_peer() {
    let root = std::env::var_os("KHOR_SUBSCRIPTION_PEER").expect("fixture root");
    util::off_the_segment();
    util::own_tmux();
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        Node::open_as(PathBuf::from(root), "subscription-source")
            .unwrap()
            .serve()
            .await
            .unwrap();
    });
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn network_subscription_import() {
    let root =
        std::env::temp_dir().join(format!("khor-network-subscriptions-{}", std::process::id()));
    let mut fixture = Fixture { root, peer: None };
    let source = fixture.root.join("source");
    let receiver = fixture.root.join("receiver");
    let home = fixture.root.join("private-home");
    util::hermetic(&source);
    util::hermetic(&receiver);
    util::own_tmux();
    std::fs::create_dir_all(home.join(".codex/sessions/2026/10/22")).unwrap();
    std::fs::write(
        home.join(".codex/auth.json"),
        r#"{"auth_mode":"chatgpt","tokens":{"access_token":"fixture-secret-never-crosses"}}"#,
    )
    .unwrap();
    let snapshot = serde_json::json!({"timestamp": jiff::Timestamp::now().to_string(), "payload": {"rate_limits": {"primary": {"used_percent":12,"window_minutes":300},"secondary":{"used_percent":34,"window_minutes":10080}}}});
    std::fs::write(
        home.join(".codex/sessions/2026/10/22/rollout-fixture.jsonl"),
        snapshot.to_string(),
    )
    .unwrap();
    let log = std::fs::File::create(fixture.root.join("peer.log")).unwrap();
    fixture.peer = Some(
        Command::new(std::env::current_exe().unwrap())
            .args(["--ignored", "--exact", "subscription_peer", "--nocapture"])
            .env("KHOR_SUBSCRIPTION_PEER", &source)
            .env("HOME", &home)
            .env("KHOR_CODEX_USAGE_URL", "http://127.0.0.1:9/no-vendor")
            .env("KHOR_CLAUDE_USAGE_URL", "http://127.0.0.1:9/no-vendor")
            .env_remove("CODEX_HOME")
            .env_remove("KHOR_HOME")
            .env_remove("KHOR_NAME")
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap(),
    );
    listening(&source).await;
    let remote = Node::open_as(source.clone(), "subscription-source").unwrap();
    let n = Node::open_as(receiver.clone(), "subscription-viewer").unwrap();
    timeout(Duration::from_secs(90), n.pair(&remote.invite().unwrap()))
        .await
        .unwrap()
        .unwrap();
    let server = Node::open_as(receiver.clone(), "subscription-viewer").unwrap();
    let resident = tokio::spawn(async move { server.serve().await });
    listening(&receiver).await;
    let machine = remote.device_str();
    let found = n.subscriptions_on(machine, Action::Discover).await.unwrap();
    let Answer::Candidates { candidates } = found else {
        panic!("discovery reply")
    };
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].provider, "codex");
    assert!(
        remote.accounts().unwrap().is_empty(),
        "discovery must not auto-adopt"
    );
    let import = || Action::Import {
        provider: "codex".into(),
    };
    let Answer::Imported { account } = n.subscriptions_on(machine, import()).await.unwrap() else {
        panic!("import reply")
    };
    assert_eq!(account.home, machine);
    assert_eq!(
        n.accounts().unwrap()[0].id,
        account.id,
        "selected row arrives before background sync"
    );
    let Answer::Imported { account: again } = n.subscriptions_on(machine, import()).await.unwrap()
    else {
        panic!("repeat reply")
    };
    assert_eq!(again.id, account.id);
    assert_eq!(remote.accounts().unwrap().len(), 1);
    assert!(!receiver.join(".codex").exists());
    assert!(!receiver.join(".khor/keys").exists());
    timeout(Duration::from_secs(45), async {
        loop {
            let board = n.quota_board().await.unwrap();
            if board
                .iter()
                .any(|(r, _)| r.account == account.id && r.trouble.is_none())
            {
                assert_eq!(board[0].0.windows.len(), 2);
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("remote quota should reach the viewer over sync");
    std::fs::remove_file(home.join(".codex/auth.json")).unwrap();
    assert!(
        n.subscriptions_on(machine, import()).await.is_err(),
        "removed login is rechecked remotely"
    );
    assert!(n.subscriptions_on("", Action::Discover).await.is_err());
    resident.abort();
    let _ = resident.await;
}
