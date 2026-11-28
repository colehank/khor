use std::time::Duration;

use iroh_relay::server::{RelayConfig, Server, ServerConfig};
use khor_net::{
    configuration::NetworkPolicy,
    endpoint::{ALPN, Relay, bind_network},
};

#[tokio::test]
async fn configured_relay_works_and_an_advertised_relay_receives_no_connection() {
    let trap = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let forbidden = format!("http://{}", trap.local_addr().unwrap());
    let mut cfg = ServerConfig::default();
    cfg.relay = Some(RelayConfig::new(
        "127.0.0.1:0".parse::<std::net::SocketAddr>().unwrap(),
    ));
    let relay = Server::spawn(cfg).await.unwrap();
    let allowed = format!("http://{}", relay.http_addr().unwrap());
    let map = [Relay::bare(&allowed)];

    let bind = || {
        let policy = NetworkPolicy {
            mdns: false,
            ..NetworkPolicy::self_hosted()
        };
        let map = &map;
        async move {
            bind_network(
                iroh::SecretKey::generate(),
                map,
                &policy,
                false,
                None,
                (0, 0),
                None,
            )
            .await
            .unwrap()
        }
    };
    let receiver = bind().await;
    let sender = bind().await;
    let home = allowed.parse::<iroh::RelayUrl>().unwrap();
    tokio::time::timeout(Duration::from_secs(8), receiver.online())
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(8), sender.online())
        .await
        .unwrap();
    assert!(
        sender
            .addr()
            .addrs
            .contains(&iroh::TransportAddr::Relay(home.clone()))
    );
    let accept = tokio::spawn({
        let receiver = receiver.clone();
        async move {
            let conn = receiver.accept().await.unwrap().await.unwrap();
            conn.closed().await;
        }
    });

    // A relay in a previously saved row or ticket follows this transport path.
    let target = iroh::EndpointAddr::new(iroh::SecretKey::generate().public())
        .with_relay_url(forbidden.parse().unwrap());
    let attempted = tokio::time::timeout(Duration::from_secs(2), sender.connect(target, ALPN));
    let observed = tokio::time::timeout(Duration::from_secs(2), trap.accept());
    let (attempted, observed) = tokio::join!(attempted, observed);
    assert!(!matches!(attempted, Ok(Ok(_))));
    assert!(
        observed.is_err(),
        "the endpoint connected to an unconfigured relay"
    );

    let target = iroh::EndpointAddr::new(receiver.id()).with_relay_url(home);
    let reachable = tokio::time::timeout(Duration::from_secs(8), sender.connect(target, ALPN))
        .await
        .unwrap()
        .unwrap();
    reachable.close(0u32.into(), b"done");
    accept.await.unwrap();
    sender.close().await;
    receiver.close().await;
}

#[tokio::test]
async fn self_hosted_lookup_finds_a_peer_by_key_and_keeps_ips_private() {
    use iroh::address_lookup::PkarrRelayClient;

    let dir = std::env::temp_dir().join(format!("khor-policy-lookup-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let config_path = dir.join("lookup.toml");
    std::fs::write(
        &config_path,
        include_str!("../../../docs/net-optimization/self-hosted/lookup.toml")
            .replace("/var/lib/khor-lookup", dir.to_str().unwrap())
            .replace("port = 3343", "port = 0")
            .replace("port = 5353", "port = 0"),
    )
    .unwrap();
    let cfg = iroh_dns_server::config::Config::load(&config_path)
        .await
        .unwrap();
    let registry = iroh_dns_server::Server::bind(cfg).await.unwrap();
    let url = format!("http://{}/pkarr", registry.http_addr().unwrap());

    let mut cfg = ServerConfig::default();
    cfg.relay = Some(RelayConfig::new(
        "127.0.0.1:0".parse::<std::net::SocketAddr>().unwrap(),
    ));
    let relay = Server::spawn(cfg).await.unwrap();
    let relay_url = format!("http://{}", relay.http_addr().unwrap());

    let policy = NetworkPolicy {
        mdns: false,
        pkarr: vec![url.clone()],
        ..NetworkPolicy::self_hosted()
    };
    let map = [Relay::bare(&relay_url)];
    let bind = || {
        bind_network(
            iroh::SecretKey::generate(),
            &map,
            &policy,
            false,
            None,
            (0, 0),
            None,
        )
    };
    let receiver = bind().await.unwrap();
    tokio::time::timeout(Duration::from_secs(8), receiver.online())
        .await
        .unwrap();
    let accept = tokio::spawn({
        let receiver = receiver.clone();
        async move {
            let conn = receiver.accept().await.unwrap().await.unwrap();
            conn.closed().await;
        }
    });

    let client = PkarrRelayClient::new(
        url.parse().unwrap(),
        receiver.tls_config().clone(),
        receiver.dns_resolver().unwrap().clone(),
    );
    let info = tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            if let Ok(packet) = client.resolve(receiver.id()).await {
                let info =
                    iroh::address_lookup::EndpointInfo::from_pkarr_signed_packet(&packet).unwrap();
                if info.relay_urls().next().is_some() {
                    break info;
                }
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        info.relay_urls().next().unwrap().host_str(),
        Some("127.0.0.1")
    );
    assert!(info.ip_addrs().next().is_none());

    let sender = bind().await.unwrap();
    tokio::time::timeout(Duration::from_secs(8), sender.online())
        .await
        .unwrap();
    let connected = tokio::time::timeout(
        Duration::from_secs(8),
        sender.connect(iroh::EndpointAddr::new(receiver.id()), ALPN),
    )
    .await
    .unwrap()
    .unwrap();
    connected.close(0u32.into(), b"done");
    accept.await.unwrap();
    sender.close().await;
    receiver.close().await;
    registry.shutdown().await.unwrap();
    let _ = std::fs::remove_dir_all(dir);
}
