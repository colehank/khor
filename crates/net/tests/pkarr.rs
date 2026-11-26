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
    let config = iroh_dns_server::config::Config::load(&config)
        .await
        .unwrap();
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
async fn by_id_alone(
    from: &iroh::Endpoint,
    id: iroh::EndpointId,
    within: Duration,
) -> Result<Duration, String> {
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
    relay_cfg.relay = Some(RelayConfig::new(
        "127.0.0.1:0".parse::<std::net::SocketAddr>().unwrap(),
    ));
    let relay = Server::spawn(relay_cfg).await.unwrap();
    let url = format!("http://{}", relay.http_addr().unwrap());
    let entry = Relay {
        url: url.clone(),
        pkarr: Some(pkarr.clone()),
        ..Relay::default()
    };
    assert_eq!(endpoint::registries(std::slice::from_ref(&entry)).len(), 1);

    let alpha_entry = Relay {
        pkarr: Some(format!("{pkarr}/")),
        ..entry.clone()
    };
    let alpha = endpoint::bind_with(iroh::SecretKey::generate(), &[alpha_entry], false)
        .await
        .unwrap();
    answering(&alpha);
    let beta = endpoint::bind_with(
        iroh::SecretKey::generate(),
        std::slice::from_ref(&entry),
        false,
    )
    .await
    .unwrap();

    // The control first, while alpha has had the longest to publish: a
    // machine whose entry names no registry has nothing to look alpha up in.
    let gamma = endpoint::bind_with(iroh::SecretKey::generate(), &[Relay::bare(&url)], false)
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_secs(3)).await;
    let blind = by_id_alone(&gamma, alpha.id(), Duration::from_secs(8)).await;
    eprintln!("[pkarr] no registry: {blind:?}");
    assert!(blind.is_err(), "found alpha with no registry: {blind:?}");

    let found = by_id_alone(&beta, alpha.id(), Duration::from_secs(15)).await;
    eprintln!("[pkarr] through {pkarr}: {found:?}");
    assert!(
        found.is_ok(),
        "beta could not find alpha through the registry: {found:?}"
    );

    for ep in [alpha, beta, gamma] {
        ep.close().await;
    }
    drop(relay);
    let _ = std::fs::remove_dir_all(&dir);
}

struct FaultDomain {
    relay: Option<Server>,
    registry: Option<iroh_dns_server::Server>,
    entry: Relay,
}

impl FaultDomain {
    async fn new(dir: &std::path::Path) -> Self {
        std::fs::create_dir(dir).unwrap();
        let (registry, pkarr) = registry(dir).await;
        let mut config = ServerConfig::default();
        config.relay = Some(RelayConfig::new(
            "127.0.0.1:0".parse::<std::net::SocketAddr>().unwrap(),
        ));
        let relay = Server::spawn(config).await.unwrap();
        let entry = Relay {
            url: format!("http://{}/", relay.http_addr().unwrap()),
            pkarr: Some(pkarr),
            ..Relay::default()
        };
        Self {
            relay: Some(relay),
            registry: Some(registry),
            entry,
        }
    }

    async fn stop_registry(&mut self) {
        self.registry.take().unwrap().shutdown().await.unwrap();
    }

    async fn shutdown(&mut self) {
        if let Some(relay) = self.relay.take() {
            relay.shutdown().await.unwrap();
        }
        if let Some(registry) = self.registry.take() {
            registry.shutdown().await.unwrap();
        }
    }
}

fn fault_environment() {
    // Each test runs serially, before constructing its endpoints.
    for key in [
        "http_proxy",
        "https_proxy",
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
        "all_proxy",
    ] {
        unsafe { std::env::remove_var(key) };
    }
    unsafe {
        std::env::set_var(endpoint::RELAY_ONLY_ENV, "1");
        std::env::set_var(endpoint::NO_MDNS_ENV, "1");
    }
}

fn fault_directory(tag: &str, round: u32) -> std::path::PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "khor-pkarr-{tag}-{}-{round}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir(&dir).unwrap();
    dir
}

async fn fault_bind(entries: &[Relay]) -> iroh::Endpoint {
    endpoint::bind_with(iroh::SecretKey::generate(), entries, false)
        .await
        .unwrap()
}

fn echoing(ep: &iroh::Endpoint) -> tokio::task::JoinHandle<()> {
    let ep = ep.clone();
    tokio::spawn(async move {
        let mut replies = tokio::task::JoinSet::new();
        while let Some(incoming) = ep.accept().await {
            let Ok(conn) = incoming.await else {
                continue;
            };
            replies.spawn(async move {
                while let Ok((mut send, mut recv)) = conn.accept_bi().await {
                    let Ok(body) = recv.read_to_end(16).await else {
                        break;
                    };
                    if send.write_all(&body).await.is_err() || send.finish().is_err() {
                        break;
                    }
                }
            });
        }
        replies.shutdown().await;
    })
}

async fn payload(conn: &iroh::endpoint::Connection) {
    tokio::time::timeout(Duration::from_secs(2), async {
        let (mut send, mut recv) = conn.open_bi().await.unwrap();
        send.write_all(b"registry-fault!!").await.unwrap();
        send.finish().unwrap();
        assert_eq!(recv.read_to_end(16).await.unwrap(), b"registry-fault!!");
    })
    .await
    .expect("an actual payload must complete within the original foreground2s");
}

async fn cold_connection(
    from: &iroh::Endpoint,
    id: iroh::EndpointId,
) -> iroh::endpoint::Connection {
    tokio::time::timeout(
        Duration::from_secs(15),
        from.connect(iroh::EndpointAddr::new(id), ALPN),
    )
    .await
    .expect("id-only cold discovery must keep the existing15s cap")
    .expect("an independent allowed registry must discover the peer")
}

fn current_home(ep: &iroh::Endpoint) -> Option<String> {
    ep.addr().addrs.iter().find_map(|addr| match addr {
        iroh::TransportAddr::Relay(url) => Some(url.to_string()),
        _ => None,
    })
}

/// Observe the registry itself, without warming any endpoint's address lookup.
async fn published_record(url: &str, id: iroh::EndpointId, relay: &str) -> (u16, bool) {
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
    let (authority, path) = url
        .strip_prefix("http://")
        .unwrap()
        .split_once('/')
        .unwrap();
    tokio::time::timeout(Duration::from_millis(250), async {
        let mut stream = tokio::net::TcpStream::connect(authority).await.ok()?;
        let request = format!(
            "GET /{path}/{} HTTP/1.1\r\nHost: {authority}\r\nConnection: close\r\n\r\n",
            id.to_z32()
        );
        stream.write_all(request.as_bytes()).await.ok()?;
        let mut bytes = Vec::new();
        stream.take(65536).read_to_end(&mut bytes).await.ok()?;
        let status = std::str::from_utf8(bytes.split(|b| *b == b'\n').next()?)
            .ok()?
            .split_whitespace()
            .nth(1)?
            .parse::<u16>()
            .ok()?;
        let at = bytes.windows(4).position(|part| part == b"\r\n\r\n")? + 4;
        let has_relay = bytes[at..]
            .windows(relay.len())
            .any(|part| part == relay.as_bytes());
        Some((status, status == 200 && has_relay))
    })
    .await
    .ok()
    .flatten()
    .unwrap_or((0, false))
}

async fn wait_published(urls: &[&str], ep: &iroh::Endpoint, wanted_home: Option<&str>) {
    let started = Instant::now();
    let mut first = None;
    let mut attempts = 0;
    let result = tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            if let Some(home) = current_home(ep)
                && wanted_home.is_none_or(|wanted| wanted == home)
            {
                let mut statuses = Vec::new();
                let mut ready = true;
                for url in urls {
                    let (status, published) = published_record(url, ep.id(), &home).await;
                    statuses.push(status);
                    ready &= published;
                }
                attempts += 1;
                first.get_or_insert(statuses.clone());
                if ready {
                    eprintln!("[registry-publication] sources={} attempts={attempts} first={first:?} last={statuses:?} elapsed_us={}", urls.len(), started.elapsed().as_micros());
                    break;
                }
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }).await;
    if result.is_err() {
        use iroh::Watcher;
        let statuses = ep.home_relay_status().get();
        let states: Vec<_> = statuses
            .iter()
            .map(|status| {
                (
                    status.url().to_string(),
                    status.is_connected(),
                    status.last_error().map(|error| format!("{error:#}")),
                )
            })
            .collect();
        let observed = current_home(ep);
        let mut records = Vec::new();
        if let Some(expected) = wanted_home {
            for url in urls {
                records.push(published_record(url, ep.id(), expected).await);
            }
        }
        eprintln!(
            "[registry-timeout] wanted={wanted_home:?} observed={observed:?} home_states={states:?} last_records={records:?} first={first:?} attempts={attempts}"
        );
    }
    result.expect("the advertised Relay must actually be published within the predeclared20s");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cold_peers_use_the_other_registry_and_warm_connections_need_neither() {
    fault_environment();
    for round in 1..=3 {
        let dir = fault_directory("cold", round);
        let mut domains = [
            FaultDomain::new(&dir.join("a")).await,
            FaultDomain::new(&dir.join("b")).await,
        ];
        let entries = domains.each_ref().map(|domain| domain.entry.clone());
        assert_eq!(endpoint::registries(&entries).len(), 2);
        domains[0].stop_registry().await;
        let alpha = fault_bind(&entries).await;
        let replies = echoing(&alpha);
        wait_published(&[entries[1].pkarr.as_deref().unwrap()], &alpha, None).await;
        let beta = fault_bind(&entries).await;
        let started = Instant::now();
        let conn = cold_connection(&beta, alpha.id()).await;
        payload(&conn).await;
        let cold_us = started.elapsed().as_micros();

        domains[1].stop_registry().await;
        payload(&conn).await;
        let blind = fault_bind(&entries).await;
        let absent = by_id_alone(&blind, alpha.id(), Duration::from_secs(8)).await;
        assert!(
            absent.is_err(),
            "a fresh peer has no remaining discovery source"
        );
        payload(&conn).await;
        eprintln!(
            "[registry-fault] cold round={round} cold_content_us={cold_us} warm_payloads=3 no_registry_negative=true"
        );
        for ep in [blind, beta, alpha] {
            ep.close().await;
        }
        replies.await.unwrap();
        for domain in &mut domains {
            domain.shutdown().await;
        }
        std::fs::remove_dir_all(dir).unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_whole_relay_and_registry_domain_can_fail_without_restarting_the_peer() {
    fault_environment();
    for round in 1..=3 {
        let dir = fault_directory("whole", round);
        let mut domains = [
            FaultDomain::new(&dir.join("a")).await,
            FaultDomain::new(&dir.join("b")).await,
        ];
        let entries = domains.each_ref().map(|domain| domain.entry.clone());
        let alpha = fault_bind(&entries).await;
        let replies = echoing(&alpha);
        wait_published(
            &entries
                .each_ref()
                .map(|entry| entry.pkarr.as_deref().unwrap()),
            &alpha,
            None,
        )
        .await;
        let beta = fault_bind(&entries).await;
        let conn = cold_connection(&beta, alpha.id()).await;
        payload(&conn).await;
        let old_home = current_home(&alpha).expect("the working peer publishes its Relay");
        let failed = entries
            .iter()
            .position(|entry| entry.url == old_home)
            .unwrap();
        let live_home = entries[1 - failed].url.clone();
        let started = Instant::now();
        let (fresh, content_us) = tokio::time::timeout(Duration::from_secs(30), async {
            domains[failed].shutdown().await;
            wait_published(
                &[entries[1 - failed].pkarr.as_deref().unwrap()],
                &alpha,
                Some(&live_home),
            )
            .await;
            let fresh = fault_bind(&entries).await;
            let replacement = cold_connection(&fresh, alpha.id()).await;
            payload(&replacement).await;
            (fresh, started.elapsed().as_micros())
        })
        .await
        .expect("whole-domain failure to id-only content must stay within30s");
        assert_eq!(current_home(&alpha).as_deref(), Some(live_home.as_str()));
        eprintln!(
            "[registry-fault] whole round={round} recovery_content_us={content_us} changed_home=true server_restarted=false"
        );
        for ep in [fresh, beta, alpha] {
            ep.close().await;
        }
        replies.await.unwrap();
        for domain in &mut domains {
            domain.shutdown().await;
        }
        std::fs::remove_dir_all(dir).unwrap();
    }
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
    unsafe {
        std::env::set_var(endpoint::RELAY_ONLY_ENV, "1");
        std::env::set_var(endpoint::NO_MDNS_ENV, "1");
    }
    let mut beijing = endpoint::khor_tier()
        .into_iter()
        .find(|r| r.url == "http://101.200.80.77:3340")
        .unwrap();
    beijing.pkarr = Some(pkarr.clone());
    let began = Instant::now();
    let alpha = endpoint::bind_with(
        iroh::SecretKey::generate(),
        std::slice::from_ref(&beijing),
        false,
    )
    .await
    .unwrap();
    answering(&alpha);
    let beta = endpoint::bind_with(
        iroh::SecretKey::generate(),
        std::slice::from_ref(&beijing),
        false,
    )
    .await
    .unwrap();
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
    eprintln!(
        "[pkarr-live] {} → {}: {found:?}, {:?} after alpha bound",
        beta.id().fmt_short(),
        alpha.id().fmt_short(),
        began.elapsed()
    );
    assert!(found.is_ok(), "{found:?}");
    alpha.close().await;
    beta.close().await;
}
