//! Round ROAD-2: two endpoints whose only road is a carried link (one
//! in-memory duplex stream), the shape of khor inside ssh.

use std::time::Duration;

use iroh::endpoint::presets;
use iroh::{Endpoint, EndpointAddr, RelayMode, SecretKey};
use khor_net::carry::{Carrier, Links, RoadOrder, transport_addr};

const ALPN: &[u8] = b"carry/test";

async fn bound(key: SecretKey, links: &Links) -> Endpoint {
    Endpoint::builder(presets::Minimal)
        .secret_key(key)
        .relay_mode(RelayMode::Disabled)
        .clear_ip_transports()
        .add_custom_transport(links.transport())
        .path_selector(std::sync::Arc::new(RoadOrder::default()))
        .alpns(vec![ALPN.to_vec()])
        .bind()
        .await
        .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_carried_link_is_a_road_both_ways() {
    let (ka, kb) = (SecretKey::generate(), SecretKey::generate());
    let (a_id, b_id) = (ka.public(), kb.public());
    let (la, lb) = (Links::new(a_id), Links::new(b_id));
    let (a, b) = (bound(ka, &la).await, bound(kb, &lb).await);
    let (near, far) = tokio::io::duplex(1 << 20);
    let (nr, nw) = tokio::io::split(near);
    let (fr, fw) = tokio::io::split(far);
    let up_a = la.attach(b_id, Carrier::Ssh, nr, nw);
    let _up_b = lb.attach(a_id, Carrier::Ssh, fr, fw);
    assert!(la.has(&b_id) && lb.has(&a_id));

    let server = b.clone();
    tokio::spawn(async move {
        let conn = server.accept().await.unwrap().await.unwrap();
        let (mut s, mut r) = conn.accept_bi().await.unwrap();
        let mut buf = vec![0u8; 5];
        r.read_exact(&mut buf).await.unwrap();
        s.write_all(&buf).await.unwrap();
        s.finish().unwrap();
        let _ = conn.closed().await;
    });
    let conn = tokio::time::timeout(
        Duration::from_secs(10),
        a.connect(
            EndpointAddr::from_parts(b_id, [transport_addr(&b_id)]),
            ALPN,
        ),
    )
    .await
    .expect("the handshake crosses the link")
    .unwrap();
    let (mut s, mut r) = conn.open_bi().await.unwrap();
    s.write_all(b"hello").await.unwrap();
    let mut got = vec![0u8; 5];
    r.read_exact(&mut got).await.unwrap();
    assert_eq!(&got, b"hello");
    assert_eq!(
        khor_net::path::classify(&conn),
        khor_net::path::PathKind::Carried
    );

    // The link ends: the table forgets it.
    drop(_up_b);
    lb.attach(a_id, Carrier::Ssh, tokio::io::empty(), tokio::io::sink());
    tokio::time::timeout(Duration::from_secs(5), up_a.ended())
        .await
        .ok();
    conn.close(0u32.into(), b"done");
    a.close().await;
    b.close().await;
}

/// Round ROAD-3 (T6): **the relay answers first, and the carried road is
/// opened all the same.** iroh 1.2 opens extra paths after the handshake
/// only for relays (when path 0 is not one); a carried address on the same
/// dial was never opened once the relay won the race. Patched in
/// `vendor/iroh` (KHOR.md, patch 6). Then (NET-DIRECT step 4) the faster
/// of the two carries it: here the relay, as the carried link is slow.
/// The other way round is `carry::tests` (`choose`).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_carried_road_is_opened_when_the_relay_answers_first_and_the_faster_carries() {
    use iroh_relay::server::{RelayConfig, Server, ServerConfig};
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
    let mut cfg = ServerConfig::default();
    cfg.relay = Some(RelayConfig::new(
        "127.0.0.1:0".parse::<std::net::SocketAddr>().unwrap(),
    ));
    let relay = Server::spawn(cfg).await.unwrap();
    let url: iroh::RelayUrl = format!("http://127.0.0.1:{}", relay.http_addr().unwrap().port())
        .parse()
        .unwrap();
    let on_relay = |key: SecretKey, links: &Links| {
        let mut c = iroh::RelayConfig::from(url.clone());
        c.quic = None;
        Endpoint::builder(presets::Minimal)
            .secret_key(key)
            .relay_mode(RelayMode::Custom(
                vec![std::sync::Arc::new(c)].into_iter().collect(),
            ))
            .clear_ip_transports()
            .add_custom_transport(links.transport())
            .path_selector(std::sync::Arc::new(RoadOrder::default()))
            .alpns(vec![ALPN.to_vec()])
            .bind()
    };
    let (ka, kb) = (SecretKey::generate(), SecretKey::generate());
    let (a_id, b_id) = (ka.public(), kb.public());
    let (la, lb) = (Links::new(a_id), Links::new(b_id));
    let (a, b) = (
        on_relay(ka, &la).await.unwrap(),
        on_relay(kb, &lb).await.unwrap(),
    );
    tokio::time::timeout(Duration::from_secs(20), async {
        while !b
            .addr()
            .addrs
            .iter()
            .any(|x| matches!(x, iroh::TransportAddr::Relay(_)))
        {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("b homes on the loopback relay");

    // A slow link: every byte waits 120 ms each way, so the relay's
    // answer arrives first.
    let (near, far) = slow_duplex(Duration::from_millis(120));
    let (nr, nw) = tokio::io::split(near);
    let (fr, fw) = tokio::io::split(far);
    let up_a = la.attach(b_id, Carrier::Ssh, nr, nw);
    let up_b = lb.attach(a_id, Carrier::Ssh, fr, fw);

    let server = b.clone();
    tokio::spawn(async move {
        while let Some(incoming) = server.accept().await {
            tokio::spawn(async move {
                let Ok(conn) = incoming.await else { return };
                while let Ok((mut s, mut r)) = conn.accept_bi().await {
                    tokio::spawn(async move {
                        let got = r.read_to_end(64).await.unwrap_or_default();
                        let _ = s.write_all(&got).await;
                        let _ = s.finish();
                    });
                }
            });
        }
    });
    let to_b = EndpointAddr::from_parts(
        b_id,
        [
            transport_addr(&b_id),
            iroh::TransportAddr::Relay(url.clone()),
        ],
    );
    let conn = tokio::time::timeout(Duration::from_secs(10), a.connect(to_b, ALPN))
        .await
        .expect("a dial")
        .unwrap();
    let echo = |conn: iroh::endpoint::Connection| async move {
        let (mut s, mut r) = conn.open_bi().await.ok()?;
        s.write_all(b"ping").await.ok()?;
        s.finish().ok()?;
        r.read_to_end(64).await.ok()
    };
    let first = khor_net::path::path_facts(&conn);
    assert!(
        first.iter().any(|p| p.selected && p.relay),
        "the relay answered first (the race this is about): {first:?}"
    );
    let opened = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if khor_net::path::path_facts(&conn).iter().any(|p| p.carried) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await;
    assert!(
        opened.is_ok(),
        "the carried road is opened though the relay answered first: {:?}",
        khor_net::path::path_facts(&conn)
    );
    // Both measured: the relay (loopback) beats the carried road (240 ms a
    // round trip) by far more than the margin, so the relay carries it.
    tokio::time::sleep(Duration::from_secs(3)).await;
    assert_eq!(echo(conn.clone()).await.as_deref(), Some(&b"ping"[..]));
    assert_eq!(
        khor_net::path::classify(&conn),
        khor_net::path::PathKind::Relay,
        "the faster road carries it: {:?}",
        khor_net::path::path_facts(&conn)
    );
    drop((up_a, up_b));
    conn.close(0u32.into(), b"done");
    a.close().await;
    b.close().await;
}

/// A duplex stream pair whose bytes each take `delay` to cross.
fn slow_duplex(delay: Duration) -> (tokio::io::DuplexStream, tokio::io::DuplexStream) {
    let (near, near_far) = tokio::io::duplex(1 << 20);
    let (far, far_near) = tokio::io::duplex(1 << 20);
    let (nr, nw) = tokio::io::split(near_far);
    let (fr, fw) = tokio::io::split(far_near);
    tokio::spawn(delayed(nr, fw, delay));
    tokio::spawn(delayed(fr, nw, delay));
    (near, far)
}

async fn delayed<R, W>(mut r: R, mut w: W, delay: Duration)
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
    W: tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<(tokio::time::Instant, Vec<u8>)>();
    tokio::spawn(async move {
        while let Some((at, bytes)) = rx.recv().await {
            tokio::time::sleep_until(at).await;
            if w.write_all(&bytes).await.is_err() {
                return;
            }
        }
    });
    let mut buf = vec![0u8; 64 << 10];
    while let Ok(n) = r.read(&mut buf).await {
        if n == 0
            || tx
                .send((tokio::time::Instant::now() + delay, buf[..n].to_vec()))
                .is_err()
        {
            return;
        }
    }
}
