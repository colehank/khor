//! A close prepared before a replacement must not close its offered path.

use super::*;
use futures::StreamExt as _;
use khor_net::carry::{Attached, Links, RoadOrder, transport_addr};

const ALPN: &[u8] = b"khor/carried-path-epoch-test";

async fn bound(key: iroh::SecretKey, links: &Links) -> iroh::Endpoint {
    iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .secret_key(key)
        .relay_mode(iroh::RelayMode::Disabled)
        .clear_ip_transports()
        .bind_addr("127.0.0.1:0")
        .unwrap()
        .add_custom_transport(links.transport())
        .path_selector(Arc::new(RoadOrder::default()))
        .alpns(vec![ALPN.to_vec()])
        .bind()
        .await
        .unwrap()
}

async fn echo(conn: &iroh::endpoint::Connection) -> bool {
    tokio::time::timeout(Duration::from_secs(1), async {
        let (mut send, mut recv) = conn.open_bi().await.ok()?;
        send.write_all(b"path-epoch").await.ok()?;
        send.finish().ok()?;
        Some(recv.read_to_end(64).await.ok()? == b"path-epoch")
    })
    .await
    .ok()
    .flatten()
    .unwrap_or(false)
}

struct Epoch {
    _root: tempfile::TempDir,
    node: Arc<Node>,
    ep: iroh::Endpoint,
    remote: iroh::Endpoint,
    other: Links,
    conn: iroh::endpoint::Connection,
    answering: Owned<()>,
    stream: Option<(Attached, Attached)>,
}

impl Epoch {
    async fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join(".khor")).unwrap();
        std::fs::write(
            root.path().join(".khor/relays.json"),
            r#"{"relays":[{"url":"http://127.0.0.1:9"}],"only_mine":true}"#,
        )
        .unwrap();
        let node = Arc::new(Node::open_hosting(root.path().to_path_buf(), "epoch", false).unwrap());
        let ep = bound(node.key.clone(), &node.carry).await;
        let key = iroh::SecretKey::generate();
        let other = Links::new(key.public());
        let remote = bound(key, &other).await;
        *node.ears.lock().unwrap() = Some(Arc::new(ep.clone()));
        // Start on an actual IP-only dial. The Custom address is offered
        // after the encrypted connection is already established.
        let addr = iroh::EndpointAddr::from_parts(
            remote.id(),
            remote
                .addr()
                .addrs
                .into_iter()
                .filter(|addr| matches!(addr, iroh::TransportAddr::Ip(_))),
        );
        let (conn, incoming) = tokio::time::timeout(Duration::from_secs(5), async {
            tokio::join!(ep.connect(addr, ALPN), async {
                remote.accept().await.unwrap().accept().unwrap().await
            })
        })
        .await
        .unwrap();
        let (conn, incoming) = (conn.unwrap(), incoming.unwrap());
        let answering = Owned(Some(tokio::spawn(async move {
            while let Ok((mut send, mut recv)) = incoming.accept_bi().await {
                let Ok(bytes) = recv.read_to_end(64).await else {
                    break;
                };
                if send.write_all(&bytes).await.is_err() {
                    break;
                }
                let _ = send.finish();
            }
        })));
        assert!(echo(&conn).await);
        assert_eq!(
            khor_net::path::classify(&conn),
            khor_net::path::PathKind::Direct
        );
        Self {
            _root: root,
            node,
            ep,
            remote,
            other,
            conn,
            answering,
            stream: None,
        }
    }

    async fn attach(&mut self) {
        assert!(self.stream.is_none());
        let (near, far) = tokio::io::duplex(1 << 20);
        let (nr, nw) = tokio::io::split(near);
        let (fr, fw) = tokio::io::split(far);
        self.stream = Some((
            self.node
                .carry
                .attach(self.remote.id(), Carrier::Ssh, nr, nw),
            self.other.attach(self.ep.id(), Carrier::Ssh, fr, fw),
        ));
        self.ep
            .offer_addrs(iroh::EndpointAddr::from_parts(
                self.remote.id(),
                [transport_addr(&self.remote.id())],
            ))
            .await;
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let paths = khor_net::path::path_facts(&self.conn);
                if paths
                    .iter()
                    .any(|path| path.carried && path.quality.is_some())
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("the actual Custom path opened and exchanged measured packets");
    }

    async fn end_stream(&mut self) {
        self.node.carry.detach(&self.remote.id(), Carrier::Ssh);
        self.other.detach(&self.ep.id(), Carrier::Ssh);
        if let Some((near, far)) = self.stream.take() {
            tokio::time::timeout(Duration::from_secs(2), async {
                tokio::join!(near.ended(), far.ended());
            })
            .await
            .unwrap();
        }
    }

    async fn close(mut self) {
        self.end_stream().await;
        self.answering.0.as_ref().unwrap().abort();
        if let Err(error) = self.answering.0.take().unwrap().await {
            assert!(error.is_cancelled());
        }
        *self.node.ears.lock().unwrap() = None;
        tokio::join!(self.ep.close(), self.remote.close());
    }
}

async fn observe_close(replacement: bool) {
    let mut f = Epoch::new().await;
    f.attach().await;
    f.end_stream().await;
    let (entered, release) = f.node.wires.gate_carried_close_for_test();
    let mut closing = Owned(Some(tokio::spawn({
        let (node, links, peer) = (Arc::downgrade(&f.node), f.node.carry.clone(), f.remote.id());
        async move { crate::carry::let_path_go(&node, &links, peer).await }
    })));
    ready(entered).await;
    if replacement {
        f.attach().await;
    }
    let mut events = f.conn.path_events();
    release.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(2), closing.0.take().unwrap())
        .await
        .unwrap()
        .unwrap();
    // A real message to that same remote actor is a processing barrier
    // after close_carried's enqueue; it does not reopen any path.
    assert!(f.ep.remote_info(f.remote.id()).await.is_some());
    let closed = tokio::time::timeout(Duration::from_secs(2), async {
        while let Some(event) = events.next().await {
            match event {
                iroh::endpoint::PathEvent::Closed {
                    remote_addr: iroh::TransportAddr::Custom(_),
                    ..
                } => return true,
                iroh::endpoint::PathEvent::Lagged { .. } => {
                    panic!("path observation must not lag")
                }
                _ => {}
            }
        }
        false
    })
    .await
    .unwrap_or(false);
    let carried = khor_net::path::path_facts(&f.conn)
        .iter()
        .any(|path| path.carried);
    let answered = echo(&f.conn).await;
    let wire_open = f.conn.close_reason().is_none();
    f.close().await;

    assert!(
        answered && wire_open,
        "path retirement preserves the actual IP wire"
    );
    if replacement {
        assert!(
            !closed && carried,
            "the late close must preserve the already offered replacement: closed={closed}, carried={carried}"
        );
    } else {
        assert!(
            closed && !carried,
            "without a replacement the real Custom path must close"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_late_close_preserves_an_already_offered_replacement() {
    observe_close(true).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_close_without_replacement_retires_custom_and_keeps_the_ip_wire() {
    observe_close(false).await;
}

#[path = "queued_close_tests.rs"]
mod queued_close;
