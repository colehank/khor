//! **A slow pong is not a dead relay** (vendor/iroh patch 15, net-g,
//! 2026-10-09). Upstream gave each periodic ping 3× the last pong time (at
//! least 0.5 s). Through the proxy's TUN to n0's aps1 the pongs swing
//! 124–976 ms, so a healthy home connection was dropped and redialled
//! three times in 6.5 minutes, each time telling the peers this machine
//! had left (audit/net-d.md 「net-g」 g.4).
//!
//! The same on loopback: an embedded relay behind a road that holds every
//! other pong for 700 ms and the rest for 40 ms. Nothing else is delayed,
//! so the relay's own pings to the client keep their pace. Over two
//! minutes the client must keep its one connection.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use khor_net::endpoint::{self, Relay};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::{TcpListener, TcpStream};

const FAST: Duration = Duration::from_millis(40);
const SLOW: Duration = Duration::from_millis(700);
/// The relay protocol's pong frame type (iroh-relay `FrameType::Pong`).
const PONG: u8 = 10;

/// The length of the websocket frame at the head of `buf`, once all of it
/// is there, and where its payload starts.
fn frame(buf: &[u8]) -> Option<(usize, usize)> {
    let b1 = *buf.get(1)?;
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
    (buf.len() >= at + len).then_some((at + len, at))
}

/// A road to `to` whose pongs, server to client, take `FAST` and `SLOW` in
/// turn; every other frame passes at once (frames may overtake a held
/// pong: each is a message of its own). Its port, and how many relay
/// connections (websocket upgrades) it carried.
async fn jittery_road(to: std::net::SocketAddr) -> (u16, Arc<AtomicU64>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let carried = Arc::new(AtomicU64::new(0));
    let count = carried.clone();
    tokio::spawn(async move {
        while let Ok((inbound, _)) = listener.accept().await {
            let Ok(outbound) = TcpStream::connect(to).await else { continue };
            let count = count.clone();
            let (mut ir, mut iw) = inbound.into_split();
            let (mut or, mut ow) = outbound.into_split();
            tokio::spawn(async move {
                let _ = tokio::io::copy(&mut ir, &mut ow).await;
                let _ = ow.shutdown().await;
            });
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();
            tokio::spawn(async move {
                while let Some(bytes) = rx.recv().await {
                    if iw.write_all(&bytes).await.is_err() {
                        break;
                    }
                }
                let _ = iw.shutdown().await;
            });
            tokio::spawn(async move {
                let (mut buf, mut chunk, mut upgraded, mut pongs) = (Vec::new(), vec![0u8; 65536], false, 0u64);
                loop {
                    let n = match or.read(&mut chunk).await {
                        Ok(0) | Err(_) => break,
                        Ok(n) => n,
                    };
                    buf.extend_from_slice(&chunk[..n]);
                    if !upgraded {
                        let Some(end) = buf.windows(4).position(|w| w == b"\r\n\r\n") else { continue };
                        // A relay connection, not a latency probe's GET.
                        if buf.starts_with(b"HTTP/1.1 101") {
                            count.fetch_add(1, Ordering::Relaxed);
                        }
                        let _ = tx.send(buf.drain(..end + 4).collect());
                        upgraded = true;
                    }
                    while let Some((len, payload)) = frame(&buf) {
                        let bytes: Vec<u8> = buf.drain(..len).collect();
                        if bytes.get(payload) == Some(&PONG) {
                            let wait = if pongs % 2 == 0 { FAST } else { SLOW };
                            pongs += 1;
                            let tx = tx.clone();
                            tokio::spawn(async move {
                                tokio::time::sleep(wait).await;
                                let _ = tx.send(bytes);
                            });
                        } else {
                            let _ = tx.send(bytes);
                        }
                    }
                }
            });
        }
    });
    (port, carried)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_slow_pong_keeps_the_relay_connection() {
    for k in ["http_proxy", "https_proxy", "HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "all_proxy"] {
        // SAFETY: set before any endpoint reads the environment; nothing
        // else in this binary writes it meanwhile.
        unsafe { std::env::remove_var(k) };
    }
    // SAFETY: as above.
    unsafe {
        std::env::set_var("KHOR_RELAY_ONLY", "1");
        std::env::set_var(endpoint::NO_MDNS_ENV, "1");
    }
    let mut cfg = iroh_relay::server::ServerConfig::default();
    cfg.relay = Some(iroh_relay::server::RelayConfig::new((std::net::Ipv4Addr::LOCALHOST, 0)));
    let relay = iroh_relay::server::Server::spawn(cfg).await.expect("the relay spawns");
    let (port, carried) = jittery_road(relay.http_addr().unwrap()).await;
    let url = format!("http://127.0.0.1:{port}");

    let ep = endpoint::bind_with(iroh::SecretKey::generate(), &[Relay { url, token: None }], false)
        .await
        .unwrap();
    let began = Instant::now();
    let watch = Duration::from_secs(
        std::env::var("KHOR_JITTER_S").ok().and_then(|s| s.parse().ok()).unwrap_or(120),
    );
    while began.elapsed() < watch {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    let n = carried.load(Ordering::Relaxed);
    eprintln!("[jitter] connections to the relay in {watch:?}: {n}");
    assert_eq!(n, 1, "the home connection was redialled {} times", n.saturating_sub(1));
    ep.close().await;
}
