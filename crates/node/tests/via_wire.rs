//! Carrying somebody else's datagrams over a real wire (docs/handoff
//! 批18; 账本: 网络类改动必须真连,还要有对照组).
//!
//! One serve owns alpha's key and is the **exit**. A plain UDP socket
//! stands in for "a machine only alpha's network can reach" — it is put
//! into alpha's device table under an id of its own, which is exactly
//! what alpha would hold for a neighbour on its LAN. beta, paired, asks
//! alpha for a road to that id and gets its datagrams carried there and
//! back.
//!
//! Three controls, because the road is easy to fake green:
//!
//! - a machine **alpha has never heard of** must be refused, or the exit
//!   is an open relay wearing khor's name;
//! - an **unpaired** key asking for the same road must be refused
//!   against the very serve beta just used, so the refusal is the
//!   pairing gate and not a dead endpoint;
//! - a datagram from **somebody other than the target** must not come
//!   back up the road, or the road is a way to inject packets into the
//!   asker's endpoint.

use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use khor_node::tunnel;
use khor_node::Node;
use tokio::time::timeout;

mod util;
use util::raw_tunnel;

fn root(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("khor-via-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

async fn wait_for_endpoint_file(root: &PathBuf) {
    let path = root.join(".khor").join("endpoint.json");
    timeout(Duration::from_secs(10), async {
        while !path.exists() {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("serve should write endpoint.json within 10s");
}

/// Writes one length-prefixed datagram the way `khor_node::roads` does.
async fn put(send: &mut iroh::endpoint::SendStream, bytes: &[u8]) {
    send.write_all(&(bytes.len() as u16).to_be_bytes()).await.unwrap();
    send.write_all(bytes).await.unwrap();
}

/// Reads one back, or says what it was waiting for.
async fn get(recv: &mut iroh::endpoint::RecvStream, what: &str) -> Vec<u8> {
    use tokio::io::AsyncReadExt;
    let mut len = [0u8; 2];
    timeout(Duration::from_secs(10), recv.read_exact(&mut len))
        .await
        .unwrap_or_else(|_| panic!("timed out waiting for {what}"))
        .unwrap();
    let mut buf = vec![0u8; u16::from_be_bytes(len) as usize];
    timeout(Duration::from_secs(10), recv.read_exact(&mut buf))
        .await
        .unwrap_or_else(|_| panic!("timed out reading {what}"))
        .unwrap();
    buf
}

#[tokio::test]
async fn a_paired_machine_is_carried_to_a_neighbour_only_the_exit_can_reach() {
    let ra = root("a");
    let rb = root("b");
    let sa = Node::open_as(ra.clone(), "alpha").unwrap();
    let serve_a = tokio::spawn(async move { sa.serve().await });
    wait_for_endpoint_file(&ra).await;

    let a = Node::open_as(ra.clone(), "alpha").unwrap();
    let b = Node::open_as(rb.clone(), "beta").unwrap();
    let ticket = a.invite().unwrap();
    timeout(Duration::from_secs(15), b.pair(&ticket))
        .await
        .expect("pairing must not hang")
        .unwrap();

    // The stand-in neighbour: a socket alpha can reach, wearing an id of
    // its own in alpha's table. This is exactly the shape of a machine
    // on alpha's LAN — an id, and an address only alpha can use.
    let far = tokio::net::UdpSocket::bind(("127.0.0.1", 0)).await.unwrap();
    let far_addr = far.local_addr().unwrap();
    let far_id = iroh::SecretKey::generate().public().to_string();
    a.remember(&far_id, "neighbour", &[far_addr.to_string()]).unwrap();

    let alpha = b
        .devices()
        .unwrap()
        .into_iter()
        .find(|d| d.name == "alpha")
        .expect("pairing put alpha in beta's table");

    // beta asks alpha to carry for the neighbour.
    let borrow = timeout(Duration::from_secs(15), b.tunnel_to("alpha"))
        .await
        .expect("dialling the tunnel must not hang")
        .unwrap();
    let (mut send, mut recv) = timeout(
        Duration::from_secs(15),
        borrow.open(&format!("{}{far_id}", tunnel::UDP_PREFIX)),
    )
    .await
    .expect("opening the road must not hang")
    .expect("a paired ask for a known neighbour must be carried");

    // Out: what beta writes reaches the neighbour, byte for byte.
    put(&mut send, b"knock-knock").await;
    let mut buf = [0u8; 64];
    let (n, from) = timeout(Duration::from_secs(10), far.recv_from(&mut buf))
        .await
        .expect("the neighbour must hear beta within 10s")
        .unwrap();
    assert_eq!(&buf[..n], b"knock-knock", "carried out verbatim");

    // Back: what the neighbour answers reaches beta. It answers to the
    // exit's own socket, which is the whole point — it never learns that
    // beta exists.
    far.send_to(b"who-is-there", from).await.unwrap();
    assert_eq!(get(&mut recv, "the neighbour's answer").await, b"who-is-there");

    // Control: a stranger's datagram to the same exit socket must not be
    // carried up. Sent after a real one, so the road is known to work at
    // the moment this is refused.
    let stranger = tokio::net::UdpSocket::bind(("127.0.0.1", 0)).await.unwrap();
    stranger.send_to(b"let-me-in", from).await.unwrap();
    far.send_to(b"still-me", from).await.unwrap();
    assert_eq!(
        get(&mut recv, "the neighbour's second answer").await,
        b"still-me",
        "the stranger's datagram must not be on this road at all"
    );

    // Control: a stranger alpha **can reach right now**.
    //
    // The obvious version of this — asking for a freshly generated id —
    // passes whether or not the exit checks its table, because a machine
    // nobody has heard of has no address either way. It was written that
    // way first and stayed green with the check deleted: the assertion
    // was true for a reason that had nothing to do with the gate.
    //
    // So the stranger dials alpha first. Alpha refuses it at the app
    // layer, but the connection happened, and alpha now holds a live
    // path to it — everything an exit needs to carry for somebody. The
    // only thing standing between that and an open relay is the table.
    let mallory = iroh::SecretKey::generate();
    let mallory_id = mallory.public().to_string();
    let _ = timeout(
        Duration::from_secs(15),
        util::raw_request(
            mallory,
            &alpha.id,
            &alpha.addrs,
            &khor_node::proto::Request::Vitals,
        ),
    )
    .await
    .expect("the stranger's dial must not hang");

    let err = timeout(
        Duration::from_secs(15),
        borrow.open(&format!("{}{mallory_id}", tunnel::UDP_PREFIX)),
    )
    .await
    .expect("must not hang")
    .expect_err("an exit that carries for machines it never let in is an open relay");
    assert!(err.contains(&mallory_id), "the refusal names who it will not carry for: {err}");

    // Control: an unpaired key asking for the road beta just used.
    let status = timeout(
        Duration::from_secs(15),
        raw_tunnel(
            iroh::SecretKey::generate(),
            &alpha.id,
            &alpha.addrs,
            &format!("{}{far_id}", tunnel::UDP_PREFIX),
        ),
    )
    .await
    .expect("must not hang")
    .unwrap();
    assert_eq!(status, tunnel::REFUSED, "the pairing gate holds on this road too");

    serve_a.abort();
    let _ = fs::remove_dir_all(&ra);
    let _ = fs::remove_dir_all(&rb);
}
