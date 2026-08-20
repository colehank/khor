//! The relay restarts; an endpoint older than it must find its way
//! back (批23). 2026-08-20, live: the aliyun relay rebooted three
//! times and the one serve predating it stayed unreachable for ~50
//! minutes — restarting that serve healed the mesh instantly. This is
//! that failure with the relay embedded, so the test can kill and
//! resurrect it on the same port without sawing the fleet's bridge.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use iroh_relay::server::{RelayConfig, Server, ServerConfig};
use tokio::time::timeout;

const ALPN: &[u8] = b"khor-test/0";

/// How long a stranded endpoint gets to find the reborn relay before
/// the verdict is "it does not come back". Generous against backoff,
/// tiny against the 50 minutes the fleet actually waited.
const WAY_BACK: Duration = Duration::from_secs(60);

async fn relay_on(addr: SocketAddr) -> Server {
    let mut cfg = ServerConfig::default();
    cfg.relay = Some(RelayConfig::new(addr));
    Server::spawn(cfg).await.expect("the embedded relay should spawn")
}

/// An endpoint that knows exactly one relay and no discovery — the
/// hermetic shape of a khor serve homed on the self-hosted tier.
/// Minimal, not Empty: the crypto provider is the one mandatory thing.
async fn bind_on(relay_url: &str) -> iroh::Endpoint {
    let url: iroh::RelayUrl = relay_url.parse().unwrap();
    let mut c = iroh::RelayConfig::from(url);
    c.quic = None;
    iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .secret_key(iroh::SecretKey::generate())
        .alpns(vec![ALPN.to_vec()])
        .relay_mode(iroh::endpoint::RelayMode::Custom(
            vec![Arc::new(c)].into_iter().collect(),
        ))
        .bind()
        .await
        .expect("endpoint should bind")
}

async fn wait_homed(ep: &iroh::Endpoint) {
    timeout(Duration::from_secs(20), async {
        while !ep.addr().addrs.iter().any(|a| matches!(a, iroh::TransportAddr::Relay(_))) {
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    })
    .await
    .expect("the endpoint should home on the relay within 20s");
}

/// A relay-only dial from a FRESH endpoint: the caller a stranded serve
/// must remain reachable by.
async fn reachable_via(relay_url: &str, id: &iroh::EndpointId) -> bool {
    let client = bind_on(relay_url).await;
    let mut addr = iroh::EndpointAddr::new(*id);
    addr = addr.with_relay_url(relay_url.parse().unwrap());
    let outcome = timeout(Duration::from_secs(5), client.connect(addr, ALPN)).await;
    let ok = matches!(&outcome, Ok(Ok(_)));
    if let Ok(Ok(conn)) = outcome {
        conn.close(0u32.into(), b"probe done");
    }
    client.close().await;
    ok
}

#[tokio::test]
async fn an_endpoint_survives_its_relay_restarting() {
    // The proxy carve-out from relay.rs: a machine-wide proxy would
    // carry the relay client's HTTP and test the wrong network.
    for k in ["http_proxy", "https_proxy", "HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "all_proxy"] {
        unsafe { std::env::remove_var(k) };
    }

    let first = relay_on("127.0.0.1:0".parse().unwrap()).await;
    let port = first.http_addr().expect("http relay must have an addr").port();
    let url = format!("http://127.0.0.1:{port}");

    let server = bind_on(&url).await;
    let id = server.addr().id;
    wait_homed(&server).await;
    let echo = tokio::spawn(async move {
        while let Some(incoming) = server.accept().await {
            if let Ok(conn) = incoming.await {
                let _ = conn.closed().await;
            }
        }
    });

    // Baseline: the road works while the relay lives.
    assert!(reachable_via(&url, &id).await, "the relay road must work before the restart");

    // The relay dies and comes back on the same port — a reboot, not a
    // migration. Nothing tells the stranded endpoint.
    first.shutdown().await.expect("relay shutdown");
    let _second = relay_on(format!("127.0.0.1:{port}").parse().unwrap()).await;

    // The verdict: within WAY_BACK the old endpoint must be reachable
    // again through the reborn relay, with nobody restarting it.
    let deadline = tokio::time::Instant::now() + WAY_BACK;
    let mut back = false;
    while tokio::time::Instant::now() < deadline {
        if reachable_via(&url, &id).await {
            back = true;
            break;
        }
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
    assert!(
        back,
        "an endpoint must find its way back to a restarted relay within {WAY_BACK:?} — \
         on 2026-08-20 the fleet waited 50 minutes and the answer was a human"
    );
    echo.abort();
}
