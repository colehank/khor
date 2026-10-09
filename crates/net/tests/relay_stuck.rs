//! **One relay that stops answering must not stop every relay** (net-g,
//! 2026-10-09). MacLab, from about 11:36: a connection to n0's euc1 lost
//! its way back through the proxy's TUN, idled out, and waited forever for
//! the server to answer its websocket Close (iroh awaits
//! `client_sink.close()` with no timeout, and tokio-websockets reads until
//! the server ends the stream). The relay actor then asked that connection
//! whether it had seen a peer, and waited forever too: from then on
//! MacLab sent nothing by relay — no answer to the iPhone, no dial, no
//! home change (audit/net-d.md 「net-g」 g.3).
//!
//! The same on loopback, every relay embedded here: alpha reaches beta's
//! relay through a road that swallows alpha's Close frame and from then on
//! forwards nothing either way, holding both sockets open, as the TUN held
//! euc1's. Once that connection has idled out (60 s), alpha must still
//! reach gamma on a third relay.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use khor_net::endpoint::{self, Relay};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::{TcpListener, TcpStream};

fn clear_proxies() {
    for k in ["http_proxy", "https_proxy", "HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "all_proxy"] {
        // SAFETY: set before any endpoint reads the environment; nothing
        // else in this binary writes it meanwhile.
        unsafe { std::env::remove_var(k) };
    }
}

async fn plain_relay() -> iroh_relay::server::Server {
    let mut cfg = iroh_relay::server::ServerConfig::default();
    cfg.relay = Some(iroh_relay::server::RelayConfig::new((std::net::Ipv4Addr::LOCALHOST, 0)));
    iroh_relay::server::Server::spawn(cfg).await.expect("the relay spawns")
}

/// The length of the websocket frame at the head of `buf`, once all of it
/// is there, and its opcode.
fn frame(buf: &[u8]) -> Option<(usize, u8)> {
    let (b0, b1) = (*buf.first()?, *buf.get(1)?);
    let (mut at, mut len) = (2usize, (b1 & 0x7f) as usize);
    if len == 126 {
        len = u16::from_be_bytes(buf.get(2..4)?.try_into().ok()?) as usize;
        at = 4;
    } else if len == 127 {
        len = u64::from_be_bytes(buf.get(2..10)?.try_into().ok()?) as usize;
        at = 10;
    }
    if b1 & 0x80 != 0 {
        at += 4;
    }
    (buf.len() >= at + len).then_some((at + len, b0 & 0x0f))
}

/// A road to `to` that forwards both ways until the client sends a
/// websocket Close; that frame is swallowed and from then on nothing moves
/// either way, both sockets held open. Its port, and how many closes it
/// swallowed.
async fn swallowing_road(to: std::net::SocketAddr) -> (u16, Arc<AtomicU64>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let swallowed = Arc::new(AtomicU64::new(0));
    let count = swallowed.clone();
    tokio::spawn(async move {
        while let Ok((inbound, _)) = listener.accept().await {
            let Ok(outbound) = TcpStream::connect(to).await else { continue };
            let (mut ir, mut iw) = inbound.into_split();
            let (mut or, mut ow) = outbound.into_split();
            let frozen = Arc::new(tokio::sync::Notify::new());
            let (count, freeze) = (count.clone(), frozen.clone());
            // Client to server: the upgrade request as it comes, then whole
            // frames, until a Close.
            tokio::spawn(async move {
                let (mut buf, mut chunk, mut upgraded) = (Vec::new(), vec![0u8; 65536], false);
                loop {
                    let n = match ir.read(&mut chunk).await {
                        Ok(0) | Err(_) => break,
                        Ok(n) => n,
                    };
                    buf.extend_from_slice(&chunk[..n]);
                    if !upgraded {
                        let Some(end) = buf.windows(4).position(|w| w == b"\r\n\r\n") else { continue };
                        if ow.write_all(&buf[..end + 4]).await.is_err() {
                            break;
                        }
                        buf.drain(..end + 4);
                        upgraded = true;
                    }
                    while let Some((len, opcode)) = frame(&buf) {
                        if opcode == 0x8 {
                            count.fetch_add(1, Ordering::Relaxed);
                            freeze.notify_one();
                            // Held open, silent, for the rest of the test.
                            std::future::pending::<()>().await;
                        }
                        if ow.write_all(&buf[..len]).await.is_err() {
                            return;
                        }
                        buf.drain(..len);
                    }
                }
                let _ = ow.shutdown().await;
            });
            tokio::spawn(async move {
                let mut chunk = vec![0u8; 65536];
                loop {
                    tokio::select! {
                        // Nothing more reaches the client once its Close
                        // was swallowed, not even the server's FIN.
                        () = frozen.notified() => {
                            std::future::pending::<()>().await;
                        }
                        got = or.read(&mut chunk) => match got {
                            Ok(0) | Err(_) => break,
                            Ok(n) => {
                                if iw.write_all(&chunk[..n]).await.is_err() {
                                    break;
                                }
                            }
                        },
                    }
                }
                let _ = iw.shutdown().await;
            });
        }
    });
    (port, swallowed)
}

/// An endpoint answering one echo per connection.
fn answer_echoes(ep: &iroh::Endpoint) {
    let server = ep.clone();
    tokio::spawn(async move {
        while let Some(incoming) = server.accept().await {
            let Ok(conn) = incoming.await else { continue };
            tokio::spawn(async move {
                if let Ok((mut send, mut recv)) = conn.accept_bi().await {
                    let got = recv.read_to_end(1024).await.unwrap_or_default();
                    let _ = send.write_all(&got).await;
                    let _ = send.finish();
                    let _ = conn.closed().await;
                }
            });
        }
    });
}

/// One echo from `from` to `to` through the relay at `url`.
async fn echo(from: &iroh::Endpoint, to: &iroh::Endpoint, url: &str) -> Result<Duration, String> {
    let began = Instant::now();
    let addr = endpoint::dial_addr(&to.addr().id.to_string(), &[], &[url.to_string()]).map_err(|e| e.to_string())?;
    let conn = tokio::time::timeout(Duration::from_secs(10), from.connect(addr, endpoint::ALPN))
        .await
        .map_err(|_| "dial timed out".to_string())?
        .map_err(|e| e.to_string())?;
    let (mut send, mut recv) = conn.open_bi().await.map_err(|e| e.to_string())?;
    send.write_all(b"still there").await.map_err(|e| e.to_string())?;
    send.finish().map_err(|e| e.to_string())?;
    let back = tokio::time::timeout(Duration::from_secs(10), recv.read_to_end(1024))
        .await
        .map_err(|_| "no answer within 10 s".to_string())?
        .map_err(|e| e.to_string())?;
    conn.close(0u32.into(), b"done");
    (back == b"still there").then_some(began.elapsed()).ok_or_else(|| format!("came back as {back:?}"))
}

/// A relay-only endpoint homed on `home`.
async fn on(home: &str) -> iroh::Endpoint {
    let ep = endpoint::bind_with(iroh::SecretKey::generate(), &[Relay { url: home.to_owned(), token: None }], false)
        .await
        .unwrap();
    answer_echoes(&ep);
    ep
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_relay_that_never_answers_its_close_stops_no_other_relay() {
    clear_proxies();
    // SAFETY: as in `clear_proxies`.
    unsafe {
        std::env::set_var("KHOR_RELAY_ONLY", "1");
        std::env::set_var(endpoint::NO_MDNS_ENV, "1");
    }
    let (r1, r2, r3) = (plain_relay().await, plain_relay().await, plain_relay().await);
    let url = |s: &iroh_relay::server::Server| format!("http://{}", s.http_addr().unwrap());
    let (port, swallowed) = swallowing_road(r2.http_addr().unwrap()).await;
    let through_road = format!("http://127.0.0.1:{port}");

    let alpha = on(&url(&r1)).await;
    let beta = on(&url(&r2)).await;
    let gamma = on(&url(&r3)).await;
    // On alpha's home and never near beta's relay: beta itself would route
    // to alpha through beta's relay, where the held connection still makes
    // alpha look present.
    let delta = on(&url(&r1)).await;
    for ep in [&alpha, &beta, &gamma, &delta] {
        tokio::time::timeout(Duration::from_secs(20), async {
            while !ep.addr().addrs.iter().any(|a| matches!(a, iroh::TransportAddr::Relay(_))) {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .expect("homed within 20 s");
    }

    // Alpha reaches beta through the road: a connection to beta's relay
    // that is not alpha's home.
    let took = echo(&alpha, &beta, &through_road).await.expect("alpha reaches beta through the road");
    eprintln!("[stuck] alpha → beta through the road: {took:?}");

    // That connection idles out after 60 s and closes; the road swallows the
    // Close and holds the connection open, silent.
    let began = Instant::now();
    while swallowed.load(Ordering::Relaxed) == 0 {
        assert!(began.elapsed() < Duration::from_secs(90), "alpha never closed its connection to beta's relay");
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    eprintln!("[stuck] alpha closed the idle connection {:?} later; the road swallowed it", began.elapsed());
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Alpha must still reach gamma, on a relay it holds no connection to.
    let reached = echo(&alpha, &gamma, &url(&r3)).await;
    eprintln!("[stuck] alpha → gamma on a third relay afterwards: {reached:?}");
    // And alpha still answers on its home (MacLab's iPhone, 13:07: packets
    // in on the home relay, nothing back).
    let back = echo(&delta, &alpha, &url(&r1)).await;
    eprintln!("[stuck] delta → alpha on alpha's home: {back:?}");
    assert!(reached.is_ok(), "alpha reaches gamma: {reached:?}");
    assert!(back.is_ok(), "delta reaches alpha on alpha's home: {back:?}");

    for ep in [alpha, beta, gamma, delta] {
        ep.close().await;
    }
}
