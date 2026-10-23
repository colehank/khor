//! Agent configuration reaches the account's home over authenticated mesh/IPC.
mod util;
use khor_node::{AccountKind, Node, Provider, khor_agent::Action};
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
