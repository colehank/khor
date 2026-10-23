//! Agent configuration reaches the account's home over authenticated mesh/IPC.
mod util;
use khor_node::{AccountKind, Node, Provider, khor_agent::Action};
use serde_json::{Value, json};
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
        "khor-pi-wire-{tag}-{}",
        khor_node::link::fresh_hex().unwrap()
    ));
    std::fs::create_dir_all(&p).unwrap();
    util::hermetic(&p);
    Scratch(p)
}
struct Serving(tokio::task::JoinHandle<Result<(), String>>);
impl Drop for Serving {
    fn drop(&mut self) {
        self.0.abort();
    }
}
async fn serve(root: &Path, name: &str) -> Serving {
    let node = Node::open_as(root.to_path_buf(), name).unwrap();
    let serving = Serving(tokio::spawn(async move { node.serve().await }));
    timeout(Duration::from_secs(90), async {
        while !root.join(".khor/endpoint.json").exists() {
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
    })
    .await
    .unwrap();
    serving
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn agent_configuration_uses_the_key_home_and_never_copies_the_key() {
    let alpha_root = root("alpha");
    let beta_root = root("beta");
    let alpha = Node::open_as(alpha_root.0.clone(), "alpha").unwrap();
    let account = alpha
        .register_account(
            "fixture",
            Provider::Newapi,
            AccountKind::Bill,
            Some("http://127.0.0.1:1"),
            "fixture-key-stays-on-alpha",
            None,
        )
        .unwrap();
    let _alpha = serve(&alpha_root.0, "alpha").await;
    let beta = Node::open_as(beta_root.0.clone(), "beta").unwrap();
    timeout(Duration::from_secs(45), beta.pair(&alpha.invite().unwrap()))
        .await
        .unwrap()
        .unwrap();
    let _beta = serve(&beta_root.0, "beta").await;
    let action = Action::Configure {
        account: account.id.clone(),
        model: "fixture-model".into(),
    };
    let result = beta
        .khor_agent_on(alpha.device_str(), action.clone())
        .await
        .unwrap();
    assert_eq!(result.config.as_ref().unwrap().home, alpha.device_str());
    assert_eq!(alpha.khor_agent_config().unwrap(), result.config);
    assert_eq!(beta.khor_agent_config().unwrap(), result.config);
    assert!(!beta_root.0.join(".khor/keys").join(&account.id).exists());
    assert!(
        beta.khor_agent_on(beta.device_str(), action.clone())
            .await
            .is_err()
    );
    let manage_option = |request: Value| Action::Manage { request };
    let started=beta.khor_agent_on(alpha.device_str(),manage_option(json!({"operation":"session_set_option","account":account.id,"configId":"auto_compaction","value":false}))).await.unwrap().data.unwrap();
    let option_job = started["job"].as_str().unwrap();
    let completed = timeout(Duration::from_secs(30), async {
        loop {
            let report = beta
                .khor_agent_on(
                    alpha.device_str(),
                    manage_option(json!({"operation":"poll","job":option_job})),
                )
                .await
                .unwrap();
            let data = report.data.as_ref().unwrap();
            if data["status"] == "done" {
                break report;
            }
            assert_ne!(data["status"], "failed", "{data}");
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        completed.config.as_ref().unwrap().options["auto_compaction"],
        false
    );
    assert_eq!(
        beta.khor_agent_config().unwrap().unwrap().options["auto_compaction"],
        false,
        "validated options return to the calling machine"
    );
    // A remote setup keeps both the new credential and OAuth refresh state on alpha.
    let manage = |request: Value| Action::Manage { request };
    let started=beta.khor_agent_on(alpha.device_str(),manage(json!({"operation":"login","provider":"anthropic","authType":"api_key","name":"remote inference"}))).await.unwrap().data.unwrap();
    let job = started["job"].as_str().unwrap();
    let waiting = timeout(Duration::from_secs(20), async {
        loop {
            let state = beta
                .khor_agent_on(
                    alpha.device_str(),
                    manage(json!({"operation":"poll","job":job})),
                )
                .await
                .unwrap()
                .data
                .unwrap();
            if state["status"] == "waiting" {
                break state;
            }
            assert_ne!(state["status"], "failed", "{state}");
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .unwrap();
    beta.khor_agent_on(alpha.device_str(),manage(json!({"operation":"reply","job":job,"ask":waiting["ask"],"value":"fixture-remote-inference-key"}))).await.unwrap();
    let finished = timeout(Duration::from_secs(20), async {
        loop {
            let state = beta
                .khor_agent_on(
                    alpha.device_str(),
                    manage(json!({"operation":"poll","job":job})),
                )
                .await
                .unwrap()
                .data
                .unwrap();
            if state["status"] == "done" {
                break state;
            }
            assert_ne!(state["status"], "failed", "{state}");
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .unwrap();
    assert!(
        !finished
            .to_string()
            .contains("fixture-remote-inference-key")
    );
    let id = finished["account"].as_str().unwrap();
    assert_eq!(finished["accountInfo"]["home"], alpha.device_str());
    assert!(
        alpha_root
            .0
            .join(".khor/agent/credentials")
            .join(id)
            .join("auth.json")
            .is_file()
    );
    assert!(
        !beta_root
            .0
            .join(".khor/agent/credentials")
            .join(id)
            .join("auth.json")
            .exists()
    );
    let request = khor_node::proto::Request::KhorAgent { action };
    assert!(!request.rides_early());
    assert!(!request.is_reading());
    let target = beta
        .devices()
        .unwrap()
        .into_iter()
        .find(|d| d.id == alpha.device_str())
        .unwrap();
    let relays = [khor_net::endpoint::Relay::bare(util::loop_relay())];
    let endpoint = khor_net::endpoint::bind_with(iroh::SecretKey::generate(), &relays, false)
        .await
        .unwrap();
    let address =
        khor_net::endpoint::dial_addr(&target.id, &target.addrs, &[util::loop_relay().into()])
            .unwrap();
    let connection = timeout(
        Duration::from_secs(20),
        endpoint.connect(address, khor_net::endpoint::ALPN),
    )
    .await
    .unwrap()
    .unwrap();
    let (mut send, mut recv) = connection.open_bi().await.unwrap();
    send.write_all(&khor_node::proto::encode(&request).unwrap())
        .await
        .unwrap();
    send.finish().unwrap();
    let bytes = recv.read_to_end(khor_node::proto::MAX_FRAME).await.unwrap();
    let refused: khor_node::proto::Response = khor_node::proto::decode(&bytes).unwrap();
    endpoint.close().await;
    assert!(matches!(
        refused,
        khor_node::proto::Response::Refused { .. }
    ));
}
