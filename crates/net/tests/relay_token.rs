//! A relay that admits by shared key admits the khor endpoint that
//! carries the key, and only it (docs/NET.md 中继, CONNECT.md 分期 0:
//! 客户端带上密钥).
//!
//! The relay is embedded — iroh-relay's own server, gated by a key the
//! way `iroh-relay`'s `access.shared_token` gates it — so nothing here
//! touches a relay anybody else uses. The clients are khor's own bind,
//! which is the half under test: the key has to get from a
//! [`Relay`] entry into the handshake.

use std::sync::Arc;
use std::time::Duration;

use iroh::Watcher as _;
use iroh_relay::server::{Access, AccessControl, ClientRequest, RelayConfig, Server, ServerConfig};
use khor_net::endpoint::{self, ALPN, Relay};
use n0_future::StreamExt as _;
use tokio::time::timeout;

const KEY: &str = "sesame-relay-test";

#[derive(Debug)]
struct SharedKey;

impl AccessControl for SharedKey {
    async fn on_connect(&self, request: &ClientRequest) -> Access {
        if request.auth_token().as_deref() == Some(KEY) {
            Access::Allow
        } else {
            Access::Deny { reason: None }
        }
    }
}

fn clear_proxies() {
    for k in [
        "http_proxy",
        "https_proxy",
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
        "all_proxy",
    ] {
        unsafe { std::env::remove_var(k) };
    }
    // Removing the variables is enough since 2026-10-01: iroh's probes no
    // longer read the macOS system proxy (vendor/iroh/KHOR.md, patch 2).
}

/// How the endpoint's relay came out: `Ok` once it is connected, `Err`
/// with the relay's refusal once it is refused.
async fn relay_verdict(ep: &iroh::Endpoint) -> Result<(), String> {
    let mut stream = ep.home_relay_status().stream();
    let seen = timeout(Duration::from_secs(20), async {
        while let Some(statuses) = stream.next().await {
            for s in &statuses {
                if s.is_connected() {
                    return Ok(());
                }
                if let Some(err) = s.last_error() {
                    return Err(format!("{err:#}"));
                }
            }
        }
        Err("the status stream ended".to_string())
    })
    .await;
    seen.unwrap_or_else(|_| Err("no verdict within 20s".to_string()))
}

/// An endpoint reachable only through the relay: no IP transport, no
/// lookup, the relay map khor would build for the same entries.
async fn relay_only(relays: &[Relay]) -> iroh::Endpoint {
    iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .secret_key(iroh::SecretKey::generate())
        .clear_ip_transports()
        .alpns(vec![ALPN.to_vec()])
        .relay_mode(iroh::endpoint::RelayMode::Custom(endpoint::relay_map(
            relays,
        )))
        .bind()
        .await
        .expect("a relay-only endpoint should bind")
}

#[tokio::test]
async fn a_keyed_relay_admits_the_endpoint_that_carries_the_key_and_no_other() {
    clear_proxies();
    let mut cfg = RelayConfig::new("127.0.0.1:0".parse::<std::net::SocketAddr>().unwrap());
    cfg.access = Arc::new(SharedKey);
    let mut server_cfg = ServerConfig::default();
    server_cfg.relay = Some(cfg);
    let relay = Server::spawn(server_cfg).await.expect("the relay spawns");
    let url = format!("http://{}", relay.http_addr().expect("an http relay"));
    let keyed = [Relay {
        url: url.clone(),
        token: Some(KEY.to_string()),
        ..Relay::default()
    }];

    // The far side has **no IP transport at all**: the relay is the only
    // road to it. Without that, mDNS on this segment could hand either
    // client a direct address, and "reached it" would stop meaning
    // "reached it through the relay" — for the keyless client that is
    // the difference between a sound control and a flaky one.
    let server = relay_only(&keyed).await;
    assert_eq!(
        relay_verdict(&server).await,
        Ok(()),
        "the far side, with the key, must be admitted"
    );
    let server_id = server.addr().id.to_string();
    let echo = tokio::spawn({
        let server = server.clone();
        async move {
            while let Some(incoming) = server.accept().await {
                let Ok(conn) = incoming.await else { continue };
                if let Ok((mut send, mut recv)) = conn.accept_bi().await {
                    let got = recv.read_to_end(1024).await.unwrap_or_default();
                    let _ = send.write_all(&got).await;
                    let _ = send.finish();
                    let _ = conn.closed().await;
                }
            }
        }
    });

    // Control: khor's bind, the same relay, no key — refused at the
    // relay…
    let stranger = endpoint::bind(iroh::SecretKey::generate(), &[Relay::bare(&url)])
        .await
        .unwrap();
    let refused = relay_verdict(&stranger).await;
    assert!(
        refused.is_err(),
        "an endpoint without the key must be refused by the relay, got {refused:?}"
    );
    // …and so it cannot reach the far side, whose only road is that
    // relay.
    let addr = endpoint::dial_addr(&server_id, &[], &[url.clone()]).unwrap();
    let dial = timeout(Duration::from_secs(8), stranger.connect(addr, ALPN)).await;
    assert!(
        !matches!(dial, Ok(Ok(_))),
        "a keyless endpoint must not reach anyone through a keyed relay"
    );
    stranger.close().await;

    // khor's bind with the key: admitted, and it reaches the far side
    // through the relay — there is no other road to it.
    let client = endpoint::bind(iroh::SecretKey::generate(), &keyed)
        .await
        .unwrap();
    assert_eq!(
        relay_verdict(&client).await,
        Ok(()),
        "khor's bind must present the key it was given"
    );
    let addr = endpoint::dial_addr(&server_id, &[], &[url.clone()]).unwrap();
    let conn = timeout(Duration::from_secs(20), client.connect(addr, ALPN))
        .await
        .expect("a relay dial must not hang")
        .expect("the keyed relay should carry the connection");
    let (mut send, mut recv) = conn.open_bi().await.unwrap();
    send.write_all(b"over the keyed relay").await.unwrap();
    send.finish().unwrap();
    assert_eq!(
        recv.read_to_end(1024).await.unwrap(),
        b"over the keyed relay"
    );
    conn.close(0u32.into(), b"done");
    client.close().await;
    echo.abort();
    server.close().await;
}

async fn relay() -> (Server, Relay) {
    let mut config = ServerConfig::default();
    config.relay = Some(RelayConfig::new(([127, 0, 0, 1], 0)));
    let server = Server::spawn(config).await.unwrap();
    let relay = Relay::bare(&format!("http://{}", server.http_addr().unwrap()));
    (server, relay)
}

#[tokio::test]
async fn relay_pool_changes_keep_a_running_direct_stream() {
    async fn home(ep: &iroh::Endpoint, url: &iroh::RelayUrl) {
        timeout(Duration::from_secs(15), async {
            let mut status = ep.home_relay_status().stream();
            while let Some(update) = status.next().await {
                if update
                    .iter()
                    .any(|home| home.url() == url && home.is_connected())
                {
                    return;
                }
            }
            panic!("the endpoint shut down while waiting for {url}");
        })
        .await
        .unwrap_or_else(|_| panic!("no connected home at {url}"));
    }

    let (_old_server, old) = relay().await;
    let (_new_server, new) = relay().await;
    let old_url = old.url.parse().unwrap();
    let new_url = new.url.parse().unwrap();
    let server = endpoint::bind(iroh::SecretKey::generate(), &[old])
        .await
        .unwrap();
    let client = endpoint::bind(iroh::SecretKey::generate(), &[])
        .await
        .unwrap();
    home(&server, &old_url).await;
    let server_addr = server
        .bound_sockets()
        .into_iter()
        .find(|addr| addr.is_ipv4())
        .unwrap();
    let server_addr = std::net::SocketAddr::from(([127, 0, 0, 1], server_addr.port()));
    let remote = iroh::EndpointAddr::new(server.id()).with_ip_addr(server_addr);
    let echo = tokio::spawn({
        let server = server.clone();
        async move {
            let conn = server.accept().await.unwrap().await.unwrap();
            let (mut tx, mut rx) = conn.accept_bi().await.unwrap();
            for _ in 0..2 {
                let mut byte = [0];
                rx.read_exact(&mut byte).await.unwrap();
                tx.write_all(&byte).await.unwrap();
            }
            tx.finish().unwrap();
            let _ = tx.stopped().await;
        }
    });

    timeout(Duration::from_secs(30), async {
        let conn = client.connect(remote, ALPN).await.unwrap();
        let (mut tx, mut rx) = conn.open_bi().await.unwrap();
        let mut byte = [0];
        tx.write_all(b"a").await.unwrap();
        rx.read_exact(&mut byte).await.unwrap();
        assert_eq!(byte, *b"a");

        assert!(endpoint::apply_relay_pool(&server, &[new], false, None).await);
        home(&server, &new_url).await;
        assert!(!server.has_configured_relay(&old_url));
        assert_eq!(
            server
                .bound_sockets()
                .iter()
                .find(|addr| addr.is_ipv4())
                .unwrap()
                .port(),
            server_addr.port()
        );
        tx.write_all(b"b").await.unwrap();
        tx.finish().unwrap();
        rx.read_exact(&mut byte).await.unwrap();
        assert_eq!(byte, *b"b");
        echo.await.unwrap();
    })
    .await
    .unwrap();
    client.close().await;
    server.close().await;
}

#[tokio::test]
async fn an_alternate_relay_gets_a_fresh_end_to_end_quic_sample() {
    let (_primary_server, primary) = relay().await;
    let (_standby_server, standby) = relay().await;
    let primary_url: iroh::RelayUrl = primary.url.parse().unwrap();
    let standby_url: iroh::RelayUrl = standby.url.parse().unwrap();

    let target = relay_only(std::slice::from_ref(&primary)).await;
    let client = relay_only(std::slice::from_ref(&primary)).await;
    relay_verdict(&target).await.unwrap();
    let addr = iroh::EndpointAddr::new(target.id()).with_relay_url(primary_url.clone());
    let (out, incoming) = tokio::join!(client.connect(addr, ALPN), async {
        target.accept().await.unwrap().await
    });
    let out = out.unwrap();
    let incoming = incoming.unwrap();
    let preferred = primary.url.clone();
    let relays = [primary, standby];
    assert!(endpoint::apply_relay_pool(&client, &relays, false, Some(&preferred)).await);
    assert!(endpoint::apply_relay_pool(&target, &relays, false, Some(&preferred)).await);

    let validated = || {
        out.paths().iter().any(|p| {
            p.remote_addr() == &iroh::TransportAddr::Relay(standby_url.clone())
                && p.stats().rtt_samples > 0
        })
    };
    assert!(!validated());
    client
        .probe_relay_alternatives(target.id(), primary_url.clone())
        .await;
    tokio::time::sleep(Duration::from_millis(400)).await;
    assert!(
        !validated(),
        "reachable relay without the target cannot pass QUIC validation"
    );

    timeout(Duration::from_secs(12), async {
        while !validated() {
            target
                .probe_relay_alternatives(client.id(), primary_url.clone())
                .await;
            client
                .probe_relay_alternatives(target.id(), primary_url.clone())
                .await;
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
    })
    .await
    .expect("the existing connection should validate the ready standby");
    assert!(incoming.close_reason().is_none());
    client.close().await;
    target.close().await;
}
