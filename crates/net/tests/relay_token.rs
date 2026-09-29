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
use iroh_relay::server::{
    Access, AccessControl, ClientRequest, RelayConfig, Server, ServerConfig,
};
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
    // **Removing the variables is not enough on macOS**: iroh's relay
    // probes (net_report's HTTP latency and captive-portal checks) are
    // reqwest clients, and the workspace turns on reqwest's
    // `system-proxy`, so with the variables gone they read the proxy out
    // of System Settings — and hyper-util's reader takes the host and
    // port there and **ignores the exceptions list**. On a machine with
    // a system proxy (the owner's: 127.0.0.1:7897) every probe of a
    // loopback relay goes through it; when that proxy is down or will
    // not deliver to loopback, no relay is ever measured, no home relay
    // is chosen, and an endpoint sits with no verdict at all. `NO_PROXY`
    // is the one knob that reader honours.
    unsafe { std::env::set_var("NO_PROXY", "127.0.0.1,localhost,::1") };
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
