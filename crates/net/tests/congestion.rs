//! **Which congestion controller, and do stream priorities help, on a
//! slow lossy path?** (round NET-2, piece 6.) A measurement, not a gate:
//! `cargo test -p khor-net --test congestion -- --ignored --nocapture`.
//!
//! Two plain noq endpoints (the QUIC under iroh, same version) talk
//! through a UDP road on loopback that this file controls: a bottleneck
//! of `CC_MBIT` (default 20) with 200 ms of queue, `CC_RTT_MS` (default
//! 300) of round trip and `CC_LOSS_PCT` (default 2) of random loss each
//! way. The far side pours bulk bytes on one stream, the way a transfer
//! or a `yes` in a terminal does; every 250 ms the near side asks a
//! one-byte question on another stream and times the answer, the way a
//! keystroke does. Run once per controller (noq's Cubic — iroh's default
//! — NewReno and BBRv3), and with the answers' stream set above the bulk
//! (`SendStream::set_priority`).
//!
//! A second measurement times what a terminal does most: a screenful
//! after ten seconds of typing, on an otherwise quiet connection.
//!
//! **What it decided.** Round NET-2: khor stays on Cubic. BBRv3 won
//! under loss (27× the throughput at 2 %) and under a bulk pour, but in
//! noq 1.3.0 it paced a burst after quiet by the trickle typing had shown
//! it: a screenful took 0.72–0.91 s against Cubic's 0.30 at 200 ms.
//! Round NET-5: with moq-dev/noq#5 ported into the vendored noq-proto
//! (vendor/noq-proto/KHOR.md), the same screenful takes 0.21 s against
//! Cubic's 0.30 (and 0.61 against 0.61–0.65 at 300 ms with 2 % loss), so
//! khor binds with BBRv3 (`khor_net::endpoint::transport`). Stream
//! priorities changed little: the queue is the network's, not the
//! sender's. Numbers in audit/net-2.md and audit/net-5.md.
//!
//! iroh would learn the endpoints' real loopback addresses through its
//! NAT traversal and go around the road, and iroh 1.2 refuses to turn
//! that off (`QuicTransportConfigBuilder::max_remote_nat_traversal_
//! addresses`), so the road is measured on noq itself; iroh passes the
//! same `TransportConfig` through (`Builder::transport_config`).

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use noq::rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use noq::{ClientConfig, Endpoint, ServerConfig, TransportConfig, congestion};
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

fn env_u64(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

#[derive(Clone, Copy)]
struct Road {
    rtt: Duration,
    loss_pct: u32,
    bytes_per_sec: u64,
    queue: Duration,
}

/// One direction of the road: a bottleneck that serialises packets at
/// its rate, drops what would wait longer than its queue, drops
/// `loss_pct` at random, and delivers each the one-way delay later.
struct Lane {
    road: Road,
    free_at: Instant,
    seed: u64,
}

impl Lane {
    fn admit(&mut self, len: usize, now: Instant) -> Option<Instant> {
        // xorshift: a fixed seed, so a run is repeatable.
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        if (self.seed % 10_000) < u64::from(self.road.loss_pct) * 100 {
            return None;
        }
        let start = self.free_at.max(now);
        if start.duration_since(now) > self.road.queue {
            return None;
        }
        self.free_at = start + Duration::from_secs_f64(len as f64 / self.road.bytes_per_sec as f64);
        Some(self.free_at + self.road.rtt / 2)
    }
}

/// The road: a UDP socket the client dials instead of the server. What
/// the client sends goes to the server and back, each way through its
/// own lane.
async fn road(server: SocketAddr, road: Road) -> SocketAddr {
    let sock = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
    let at = sock.local_addr().unwrap();
    let (tx, mut rx) = mpsc::unbounded_channel::<(Instant, Vec<u8>, SocketAddr)>();
    {
        let sock = sock.clone();
        tokio::spawn(async move {
            while let Some((due, bytes, to)) = rx.recv().await {
                tokio::time::sleep_until(due.into()).await;
                let _ = sock.send_to(&bytes, to).await;
            }
        });
    }
    tokio::spawn(async move {
        let now = Instant::now();
        let mut out = Lane {
            road,
            free_at: now,
            seed: 0x9e37_79b9_7f4a_7c15,
        };
        let mut back = Lane {
            road,
            free_at: now,
            seed: 0x2545_f491_4f6c_dd1d,
        };
        let mut client: Option<SocketAddr> = None;
        let mut buf = vec![0u8; 65536];
        loop {
            let Ok((n, from)) = sock.recv_from(&mut buf).await else {
                return;
            };
            let now = Instant::now();
            let (lane, to) = if from == server {
                let Some(c) = client else { continue };
                (&mut back, c)
            } else {
                client = Some(from);
                (&mut out, server)
            };
            if let Some(due) = lane.admit(n, now) {
                let _ = tx.send((due, buf[..n].to_vec(), to));
            }
        }
    });
    at
}

fn transport(cc: &str) -> TransportConfig {
    let mut t = TransportConfig::default();
    match cc {
        "bbr3" => t.congestion_controller_factory(Arc::new(congestion::Bbr3Config::default())),
        "newreno" => {
            t.congestion_controller_factory(Arc::new(congestion::NewRenoConfig::default()))
        }
        _ => t.congestion_controller_factory(Arc::new(congestion::CubicConfig::default())),
    };
    t
}

struct Run {
    cc: &'static str,
    prioritised: bool,
    bytes: u64,
    secs: f64,
    answers: Vec<Duration>,
}

/// A far side and a near side on the road, the far one answering three
/// kinds of stream: `B` pours bulk until the stream is dropped, `E`
/// answers one byte (a keystroke's echo), `U` writes a screenful the way
/// a terminal does — eighty lines of eighty bytes, each its own write.
async fn pair(cc: &'static str, prioritised: bool, road_is: Road) -> (Endpoint, noq::Connection) {
    let ck = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()]).unwrap();
    let cert: CertificateDer<'static> = ck.cert.der().clone();
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(ck.signing_key.serialize_der()));
    let mut server_cfg = ServerConfig::with_single_cert(vec![cert.clone()], key).unwrap();
    // The controller that matters is the sender's: the far side pours.
    server_cfg.transport_config(Arc::new(transport(cc)));
    let server = Endpoint::server(server_cfg, "127.0.0.1:0".parse().unwrap()).unwrap();
    let server_at = server.local_addr().unwrap();
    tokio::spawn(async move {
        while let Some(incoming) = server.accept().await {
            tokio::spawn(async move {
                let Ok(conn) = incoming.await else { return };
                while let Ok((mut send, mut recv)) = conn.accept_bi().await {
                    tokio::spawn(async move {
                        let mut kind = [0u8; 1];
                        if recv.read_exact(&mut kind).await.is_err() {
                            return;
                        }
                        match kind[0] {
                            b'B' => {
                                let chunk = vec![7u8; 64 * 1024];
                                while send.write_all(&chunk).await.is_ok() {}
                            }
                            b'U' => {
                                for _ in 0..80 {
                                    let _ = send.write_all(&[b'x'; 80]).await;
                                    tokio::task::yield_now().await;
                                }
                                let _ = send.finish();
                            }
                            _ => {
                                if prioritised {
                                    let _ = send.set_priority(1);
                                }
                                let _ = send.write_all(b"e").await;
                                let _ = send.finish();
                            }
                        }
                    });
                }
            });
        }
    });

    let at = road(server_at, road_is).await;
    let mut roots = noq::rustls::RootCertStore::empty();
    roots.add(cert).unwrap();
    let mut client_cfg = ClientConfig::with_root_certificates(Arc::new(roots)).unwrap();
    client_cfg.transport_config(Arc::new(transport(cc)));
    let client = Endpoint::client("127.0.0.1:0".parse().unwrap()).unwrap();
    client.set_default_client_config(client_cfg);
    let conn = client.connect(at, "localhost").unwrap().await.unwrap();
    (client, conn)
}

/// One `E` question, timed to its answer.
async fn ask(conn: &noq::Connection) -> Option<Duration> {
    let asked = Instant::now();
    let answered = tokio::time::timeout(Duration::from_secs(10), async {
        let (mut tx, mut rx) = conn.open_bi().await.map_err(|_| ())?;
        tx.write_all(b"E").await.map_err(|_| ())?;
        tx.finish().map_err(|_| ())?;
        let mut one = [0u8; 1];
        rx.read_exact(&mut one).await.map_err(|_| ())
    })
    .await;
    matches!(answered, Ok(Ok(()))).then(|| asked.elapsed())
}

async fn run(cc: &'static str, prioritised: bool, road_is: Road, secs: u64) -> Run {
    let (client, conn) = pair(cc, prioritised, road_is).await;
    let (mut bulk_tx, mut bulk_rx) = conn.open_bi().await.unwrap();
    bulk_tx.write_all(b"B").await.unwrap();
    let began = Instant::now();
    let until = began + Duration::from_secs(secs);
    let reader = tokio::spawn(async move {
        let mut got = 0u64;
        let mut buf = vec![0u8; 64 * 1024];
        while Instant::now() < until {
            match tokio::time::timeout(Duration::from_millis(500), bulk_rx.read(&mut buf)).await {
                Ok(Ok(Some(n))) => got += n as u64,
                Ok(_) => break,
                Err(_) => {}
            }
        }
        got
    });
    let mut answers = Vec::new();
    // The first seconds are the controller's start-up; questions are
    // timed once the pour has filled the road.
    tokio::time::sleep(Duration::from_secs(3)).await;
    while Instant::now() + Duration::from_secs(2) < until {
        if let Some(took) = ask(&conn).await {
            answers.push(took);
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    let bytes = reader.await.unwrap();
    let secs_ran = began.elapsed().as_secs_f64();
    conn.close(0u32.into(), b"done");
    client.close(0u32.into(), b"done");
    Run {
        cc,
        prioritised,
        bytes,
        secs: secs_ran,
        answers,
    }
}

fn pct(v: &[Duration], p: f64) -> Duration {
    if v.is_empty() {
        return Duration::ZERO;
    }
    let mut v = v.to_vec();
    v.sort();
    v[((v.len() - 1) as f64 * p).round() as usize]
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "a measurement on a simulated road: run by hand"]
async fn controllers_and_priorities_on_a_slow_lossy_road() {
    let road_is = Road {
        rtt: Duration::from_millis(env_u64("CC_RTT_MS", 300)),
        loss_pct: env_u64("CC_LOSS_PCT", 2) as u32,
        bytes_per_sec: env_u64("CC_MBIT", 20) * 1_000_000 / 8,
        queue: Duration::from_millis(200),
    };
    let secs = env_u64("CC_SECS", 20);
    let mut runs = Vec::new();
    for (cc, prioritised) in [
        ("cubic", false),
        ("newreno", false),
        ("bbr3", false),
        ("cubic", true),
        ("bbr3", true),
    ] {
        // `CC_ONLY=cubic,bbr3` narrows the sweep to those controllers,
        // priorities off.
        if let Ok(only) = std::env::var("CC_ONLY")
            && (prioritised || !only.split(',').any(|c| c == cc))
        {
            continue;
        }
        let r = run(cc, prioritised, road_is, secs).await;
        eprintln!(
            "[cc] road rtt {} ms, loss {}%, {} Mbit/s: {:<7} priority {:<3} -> {:.2} Mbit/s; \
             answers under the pour: median {:?}, p90 {:?}, worst {:?} ({} asked)",
            road_is.rtt.as_millis(),
            road_is.loss_pct,
            road_is.bytes_per_sec * 8 / 1_000_000,
            r.cc,
            if r.prioritised { "on" } else { "off" },
            r.bytes as f64 * 8.0 / r.secs / 1e6,
            pct(&r.answers, 0.5),
            pct(&r.answers, 0.9),
            pct(&r.answers, 1.0),
            r.answers.len(),
        );
        runs.push(r);
    }
}

/// **A screenful after typing**: the near side types for ten seconds (a
/// question every 250 ms), then the far side writes a screenful (`U`),
/// timed to its last byte; three times. The terminal's own pattern: an
/// idle connection, then a burst.
async fn screen_after_typing(cc: &'static str, road_is: Road) -> Vec<Duration> {
    let (client, conn) = pair(cc, false, road_is).await;
    let mut took = Vec::new();
    for _ in 0..3 {
        let typing = Instant::now();
        while typing.elapsed() < Duration::from_secs(10) {
            let _ = ask(&conn).await;
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        let asked = Instant::now();
        let (mut tx, mut rx) = conn.open_bi().await.unwrap();
        tx.write_all(b"U").await.unwrap();
        tx.finish().unwrap();
        let screen = rx.read_to_end(1 << 20).await.unwrap();
        assert_eq!(screen.len(), 80 * 80);
        took.push(asked.elapsed());
    }
    conn.close(0u32.into(), b"done");
    client.close(0u32.into(), b"done");
    took
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "a measurement on a simulated road: run by hand"]
async fn a_screenful_after_typing_under_each_controller() {
    let road_is = Road {
        rtt: Duration::from_millis(env_u64("CC_RTT_MS", 300)),
        loss_pct: env_u64("CC_LOSS_PCT", 2) as u32,
        bytes_per_sec: env_u64("CC_MBIT", 20) * 1_000_000 / 8,
        queue: Duration::from_millis(200),
    };
    for cc in ["cubic", "bbr3"] {
        let took = screen_after_typing(cc, road_is).await;
        eprintln!(
            "[screen] rtt {} ms, loss {}%: {:<5} a 6.4 KB screenful after 10 s of typing: {:?}",
            road_is.rtt.as_millis(),
            road_is.loss_pct,
            cc,
            took
        );
    }
}
