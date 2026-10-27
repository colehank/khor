//! **A full net report measures the relay that serves no QAD before it
//! settles** (vendor/iroh patch 14, net-g, 2026-10-09). MacLab, once its
//! proxy let n0's QUIC address discovery through: aps1 and euc1 both
//! answered QAD, upstream's full report stopped at those two answers before
//! Beijing's HTTPS probe came back, and the home stayed on aps1 (TLS
//! 0.75–4.1 s through the TUN) with Beijing (18 ms) never measured again.
//!
//! The same shape on loopback, every relay embedded here: two stand-ins
//! for n0's tier serving QAD (`KHOR_PUBLIC_RELAYS`' `url?qad=port`), each
//! behind a road that adds 100 ms each way to both its TCP and its UDP,
//! and one plain relay of the machine's own with no QAD and no delay — the
//! Beijing relay. The home must be the plain one. Certificates are
//! self-signed here and trusted through `KHOR_RELAY_CA`.
//!
//! The stand-ins are `localhost`, which iroh's resolver answers itself
//! with 127.0.0.1 and ::1, so they answer QAD on both families. Upstream's
//! shortcut depends on whether iroh thinks the machine has IPv6
//! (netwatch's `have_v6`, which a ULA such as Tailscale's fd7a:: counts):
//! without, two v4 answers end the report; with, it takes a v6 one too.
//! (An IPv6 literal cannot stand in: iroh hands `[::1]`, brackets and all,
//! to TLS as the server name.) Without patch 14 the home is a stand-in
//! either way (the control, audit/net-d.md 「net-g」).

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use iroh::Watcher as _;
use iroh_relay::server::{CertConfig, QuicConfig, RelayConfig, Server, ServerConfig, TlsConfig};
use khor_net::endpoint::{self, Relay};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::{TcpListener, TcpStream, UdpSocket};

/// What each stand-in's road adds, each way.
const ONE_WAY: Duration = Duration::from_millis(100);

fn clear_proxies() {
    for k in [
        "http_proxy",
        "https_proxy",
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
        "all_proxy",
    ] {
        // SAFETY: set before any endpoint reads the environment; nothing
        // else in this binary writes it meanwhile.
        unsafe { std::env::remove_var(k) };
    }
}

/// A self-signed certificate for `localhost`, its PEM written to `pem` for
/// `KHOR_RELAY_CA`, and the TLS config the stand-ins serve it with.
fn self_signed(pem: &std::path::Path) -> rustls::ServerConfig {
    let made = rcgen::generate_simple_self_signed(vec!["localhost".to_string()]).unwrap();
    std::fs::write(pem, made.cert.pem()).unwrap();
    let key = rustls::pki_types::PrivateKeyDer::from(rustls::pki_types::PrivatePkcs8KeyDer::from(
        made.signing_key.serialize_der(),
    ));
    rustls::ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(vec![made.cert.der().clone()], key)
        .unwrap()
}

/// A stand-in for one of n0's relays: HTTPS and QAD, the self-signed
/// certificate on both.
async fn qad_relay(tls: rustls::ServerConfig) -> Server {
    let ip = IpAddr::from(Ipv4Addr::LOCALHOST);
    let mut relay = RelayConfig::new((ip, 0));
    relay.tls = Some(TlsConfig::new(
        (ip, 0),
        CertConfig::Manual {
            server_config: tls.clone(),
        },
    ));
    let mut cfg = ServerConfig::default();
    cfg.relay = Some(relay);
    let mut quic = QuicConfig::new((ip, 0));
    quic.server_config = Some(tls);
    cfg.quic = Some(quic);
    Server::spawn(cfg).await.expect("the QAD stand-in spawns")
}

/// The machine's own relay: plain HTTP, no QAD, as Beijing's 3340.
async fn plain_relay() -> Server {
    let mut cfg = ServerConfig::default();
    cfg.relay = Some(RelayConfig::new((Ipv4Addr::LOCALHOST, 0)));
    Server::spawn(cfg).await.expect("the plain relay spawns")
}

/// Everything written one way, `ONE_WAY` later, in order.
async fn late_pipe(
    mut from: tokio::net::tcp::OwnedReadHalf,
    mut to: tokio::net::tcp::OwnedWriteHalf,
) {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<(Instant, Vec<u8>)>();
    tokio::spawn(async move {
        let mut buf = vec![0u8; 64 * 1024];
        while let Ok(n) = from.read(&mut buf).await {
            if n == 0
                || tx
                    .send((Instant::now() + ONE_WAY, buf[..n].to_vec()))
                    .is_err()
            {
                break;
            }
        }
    });
    while let Some((due, bytes)) = rx.recv().await {
        tokio::time::sleep_until(due.into()).await;
        if to.write_all(&bytes).await.is_err() {
            break;
        }
    }
    let _ = to.shutdown().await;
}

/// A TCP road to `to` that adds `ONE_WAY` each way, listening on
/// 127.0.0.1 and ::1 at one port (as `localhost` is both); the port.
async fn slow_tcp(to: SocketAddr) -> u16 {
    let (v4, v6) = loop {
        let v4 = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = v4.local_addr().unwrap().port();
        if let Ok(v6) = TcpListener::bind(("::1", port)).await {
            break (v4, v6);
        }
    };
    let port = v4.local_addr().unwrap().port();
    for listener in [v4, v6] {
        tokio::spawn(async move {
            while let Ok((inbound, _)) = listener.accept().await {
                let Ok(outbound) = TcpStream::connect(to).await else {
                    continue;
                };
                let (ir, iw) = inbound.into_split();
                let (or, ow) = outbound.into_split();
                tokio::spawn(late_pipe(ir, ow));
                tokio::spawn(late_pipe(or, iw));
            }
        });
    }
    port
}

/// A UDP road to `to` that adds `ONE_WAY` each way, on 127.0.0.1 and ::1
/// at one port; the port, and how many datagrams came back to a v4 and
/// to a v6 client.
async fn slow_udp(to: SocketAddr) -> (u16, [Arc<AtomicU64>; 2]) {
    let (v4, v6) = loop {
        let v4 = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let port = v4.local_addr().unwrap().port();
        if let Ok(v6) = UdpSocket::bind(("::1", port)).await {
            break (v4, v6);
        }
    };
    let port = v4.local_addr().unwrap().port();
    let counts = [Arc::new(AtomicU64::new(0)), Arc::new(AtomicU64::new(0))];
    for (front, counted) in [(v4, counts[0].clone()), (v6, counts[1].clone())] {
        let front = Arc::new(front);
        tokio::spawn(async move {
            let mut uplinks: HashMap<SocketAddr, Arc<UdpSocket>> = HashMap::new();
            let mut buf = vec![0u8; 65536];
            while let Ok((n, client)) = front.recv_from(&mut buf).await {
                let up = match uplinks.get(&client) {
                    Some(up) => up.clone(),
                    None => {
                        let up = Arc::new(UdpSocket::bind((to.ip(), 0)).await.unwrap());
                        up.connect(to).await.unwrap();
                        let (up2, front2, counted) = (up.clone(), front.clone(), counted.clone());
                        tokio::spawn(async move {
                            let mut buf = vec![0u8; 65536];
                            while let Ok(n) = up2.recv(&mut buf).await {
                                let (bytes, front3, counted) =
                                    (buf[..n].to_vec(), front2.clone(), counted.clone());
                                tokio::spawn(async move {
                                    tokio::time::sleep(ONE_WAY).await;
                                    if front3.send_to(&bytes, client).await.is_ok() {
                                        counted.fetch_add(1, Ordering::Relaxed);
                                    }
                                });
                            }
                        });
                        uplinks.insert(client, up.clone());
                        up
                    }
                };
                let bytes = buf[..n].to_vec();
                tokio::spawn(async move {
                    tokio::time::sleep(ONE_WAY).await;
                    let _ = up.send(&bytes).await;
                });
            }
        });
    }
    (port, counts)
}

/// The relay this endpoint is homed on, as its own address says.
fn home(ep: &iroh::Endpoint) -> Option<String> {
    ep.addr().addrs.iter().find_map(|a| match a {
        iroh::TransportAddr::Relay(u) => Some(u.to_string().trim_end_matches('/').to_owned()),
        _ => None,
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_full_report_measures_the_relay_without_qad_before_it_settles() {
    clear_proxies();
    let dir = std::env::temp_dir().join(format!("khor-qad-home-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let pem = dir.join("ca.pem");
    let tls = self_signed(&pem);

    let mut stand_ins = Vec::new();
    let mut named = Vec::new();
    let mut answered = Vec::new();
    for _ in 0..2 {
        let server = qad_relay(tls.clone()).await;
        let https = slow_tcp(server.https_addr().unwrap()).await;
        let (qad, back) = slow_udp(server.quic_addr().unwrap()).await;
        named.push((format!("https://localhost:{https}"), qad));
        answered.push(back);
        stand_ins.push(server);
    }
    let plain_server = plain_relay().await;
    let plain = format!("http://{}", plain_server.http_addr().unwrap());

    // SAFETY: as in `clear_proxies`.
    unsafe {
        std::env::set_var(endpoint::RELAY_CA_ENV, &pem);
        std::env::set_var(endpoint::NO_MDNS_ENV, "1");
        std::env::set_var(
            endpoint::PUBLIC_RELAYS_ENV,
            named
                .iter()
                .map(|(url, qad)| format!("{url}?qad={qad}"))
                .collect::<Vec<_>>()
                .join(","),
        );
    }
    let began = Instant::now();
    let ep = endpoint::bind_with(
        iroh::SecretKey::generate(),
        &[Relay::bare(plain.clone())],
        false,
    )
    .await
    .unwrap();

    // The first report is a full one; wait for the home it picks to connect.
    let settled = loop {
        let connected = ep
            .home_relay_status()
            .get()
            .iter()
            .any(|s| s.is_connected());
        if let (Some(h), true) = (home(&ep), connected) {
            break h;
        }
        assert!(
            began.elapsed() < Duration::from_secs(30),
            "no home within 30 s"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    };
    let took = began.elapsed();
    // Held for a while: the incremental reports after it keep it.
    tokio::time::sleep(Duration::from_secs(5)).await;
    let held = home(&ep);
    let qad_back: Vec<[u64; 2]> = answered
        .iter()
        .map(|[v4, v6]| [v4.load(Ordering::Relaxed), v6.load(Ordering::Relaxed)])
        .collect();
    eprintln!(
        "[qad-home] home {settled} after {took:?}, {held:?} 5 s on; plain {plain}; stand-ins {named:?}; \
         QAD datagrams back [v4, v6] {qad_back:?}"
    );
    assert!(
        qad_back.iter().all(|[v4, _]| *v4 > 0),
        "both stand-ins answered QAD on v4: {qad_back:?}"
    );
    assert_eq!(
        settled, plain,
        "the full report's home is the relay without QAD, measured fastest"
    );
    assert_eq!(held.as_deref(), Some(plain.as_str()), "and it stays home");

    ep.close().await;
    drop(stand_ins);
    drop(plain_server);
    let _ = std::fs::remove_dir_all(&dir);
}
