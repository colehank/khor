//! **A registry named on a server's entry is used** (NET-DIRECT, the
//! owner 2026-10-09: publish to every registry, resolve from all at once,
//! first answer wins; audit/net-d.md net-i).
//!
//! Everything is embedded: iroh's own pkarr server (`iroh-dns-server`,
//! the one the Beijing box runs on TCP 3343) and a plain relay, both on
//! loopback. Two relay-only endpoints whose entries name the registry
//! find each other by id alone; a third whose entry names none does not.

use std::time::{Duration, Instant};

use iroh_relay::server::{RelayConfig, Server, ServerConfig};
use khor_net::endpoint::{self, ALPN, Relay};

/// iroh-dns-server on loopback: HTTP for pkarr, its DNS listener on a
/// loopback port nobody asks, no metrics, no mainline.
async fn registry(dir: &std::path::Path) -> (iroh_dns_server::Server, String) {
    let config = dir.join("dns.toml");
    std::fs::write(
        &config,
        format!(
            r#"pkarr_put_rate_limit = "disabled"
data_dir = "{}"

[http]
port = 0
bind_addr = "127.0.0.1"

[dns]
port = 0
bind_addr = "127.0.0.1"
default_soa = "dns1.khor.test hostmaster.khor.test 0 10800 3600 604800 3600"
default_ttl = 30
origins = ["khor.test.", "."]
rr_a = "127.0.0.1"
rr_ns = "ns1.khor.test."

[metrics]
disabled = true

[mainline]
enabled = false
"#,
            dir.join("data").display()
        ),
    )
    .unwrap();
    let config = iroh_dns_server::config::Config::load(&config).await.unwrap();
    let server = iroh_dns_server::Server::bind(config).await.unwrap();
    let url = format!("http://{}/pkarr", server.http_addr().unwrap());
    (server, url)
}

/// Answers every connection it is offered and keeps it open.
fn answering(ep: &iroh::Endpoint) {
    let ep = ep.clone();
    tokio::spawn(async move {
        while let Some(incoming) = ep.accept().await {
            tokio::spawn(async move {
                if let Ok(conn) = incoming.await {
                    conn.closed().await;
                }
            });
        }
    });
}

/// Dials `id` with no address at all: only a lookup can find it.
async fn by_id_alone(from: &iroh::Endpoint, id: iroh::EndpointId, within: Duration) -> Result<Duration, String> {
    let began = Instant::now();
    match tokio::time::timeout(within, from.connect(iroh::EndpointAddr::new(id), ALPN)).await {
        Ok(Ok(_conn)) => Ok(began.elapsed()),
        Ok(Err(e)) => Err(format!("{e:#}")),
        Err(_) => Err(format!("no connection within {within:?}")),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn peers_find_each_other_through_the_registry_their_entries_name() {
    // SAFETY: set before any endpoint reads them; nothing else in this
    // binary writes them.
    for k in ["http_proxy", "https_proxy", "HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "all_proxy"] {
        unsafe { std::env::remove_var(k) };
    }
    unsafe {
        // Relay only: on loopback a direct road would need no lookup.
        std::env::set_var(endpoint::RELAY_ONLY_ENV, "1");
        std::env::set_var(endpoint::NO_MDNS_ENV, "1");
    }
    let dir = std::env::temp_dir().join(format!("khor-pkarr-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let (_registry, pkarr) = registry(&dir).await;

    let mut relay_cfg = ServerConfig::default();
    relay_cfg.relay = Some(RelayConfig::new("127.0.0.1:0".parse::<std::net::SocketAddr>().unwrap()));
    let relay = Server::spawn(relay_cfg).await.unwrap();
    let url = format!("http://{}", relay.http_addr().unwrap());
    let entry = Relay { url: url.clone(), pkarr: Some(pkarr.clone()), ..Relay::default() };
    assert_eq!(endpoint::registries(std::slice::from_ref(&entry)).len(), 1);

    let alpha = endpoint::bind_with(iroh::SecretKey::generate(), std::slice::from_ref(&entry), false).await.unwrap();
    answering(&alpha);
    let beta = endpoint::bind_with(iroh::SecretKey::generate(), std::slice::from_ref(&entry), false).await.unwrap();

    // The control first, while alpha has had the longest to publish: a
    // machine whose entry names no registry has nothing to look alpha up in.
    let gamma = endpoint::bind_with(iroh::SecretKey::generate(), &[Relay::bare(&url)], false).await.unwrap();
    tokio::time::sleep(Duration::from_secs(3)).await;
    let blind = by_id_alone(&gamma, alpha.id(), Duration::from_secs(8)).await;
    eprintln!("[pkarr] no registry: {blind:?}");
    assert!(blind.is_err(), "found alpha with no registry: {blind:?}");

    let found = by_id_alone(&beta, alpha.id(), Duration::from_secs(15)).await;
    eprintln!("[pkarr] through {pkarr}: {found:?}");
    assert!(found.is_ok(), "beta could not find alpha through the registry: {found:?}");

    for ep in [alpha, beta, gamma] {
        ep.close().await;
    }
    drop(relay);
    let _ = std::fs::remove_dir_all(&dir);
}

/// **The live registry** (not a gate; `KHOR_PKARR_LIVE=<registry url>`):
/// two throwaway endpoints, relay only on the Khor tier's Beijing relay,
/// publish to the named registry and meet by id alone. Nothing of a real
/// machine is used: fresh keys, no store.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "touches the live registry and relay; run by hand"]
async fn throwaway_endpoints_meet_through_the_live_registry() {
    let Ok(pkarr) = std::env::var("KHOR_PKARR_LIVE") else {
        panic!("set KHOR_PKARR_LIVE to the registry's url");
    };
    for k in ["http_proxy", "https_proxy", "HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "all_proxy"] {
        unsafe { std::env::remove_var(k) };
    }
    unsafe {
        std::env::set_var(endpoint::RELAY_ONLY_ENV, "1");
        std::env::set_var(endpoint::NO_MDNS_ENV, "1");
    }
    let mut beijing = endpoint::khor_tier().into_iter().find(|r| r.url == "http://101.200.80.77:3340").unwrap();
    beijing.pkarr = Some(pkarr.clone());
    let began = Instant::now();
    let alpha = endpoint::bind_with(iroh::SecretKey::generate(), std::slice::from_ref(&beijing), false).await.unwrap();
    answering(&alpha);
    let beta = endpoint::bind_with(iroh::SecretKey::generate(), std::slice::from_ref(&beijing), false).await.unwrap();
    // Alpha publishes once its home relay is up: until then the lookup
    // answers 404 at once, so the dial is retried each second.
    let mut found = Err(String::new());
    let mut tries = 0;
    while began.elapsed() < Duration::from_secs(30) {
        tries += 1;
        found = by_id_alone(&beta, alpha.id(), Duration::from_secs(10)).await;
        if found.is_ok() {
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    eprintln!("[pkarr-live] {tries} tries");
    eprintln!("[pkarr-live] {} → {}: {found:?}, {:?} after alpha bound", beta.id().fmt_short(), alpha.id().fmt_short(), began.elapsed());
    assert!(found.is_ok(), "{found:?}");
    alpha.close().await;
    beta.close().await;
}
