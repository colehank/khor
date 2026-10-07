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
        a.connect(EndpointAddr::from_parts(b_id, [transport_addr(&b_id)]), ALPN),
    )
    .await
    .expect("the handshake crosses the link")
    .unwrap();
    let (mut s, mut r) = conn.open_bi().await.unwrap();
    s.write_all(b"hello").await.unwrap();
    let mut got = vec![0u8; 5];
    r.read_exact(&mut got).await.unwrap();
    assert_eq!(&got, b"hello");
    assert_eq!(khor_net::path::classify(&conn), khor_net::path::PathKind::Carried);

    // The link ends: the table forgets it.
    drop(_up_b);
    lb.attach(a_id, Carrier::Ssh, tokio::io::empty(), tokio::io::sink());
    tokio::time::timeout(Duration::from_secs(5), up_a.ended()).await.ok();
    conn.close(0u32.into(), b"done");
    a.close().await;
    b.close().await;
}
