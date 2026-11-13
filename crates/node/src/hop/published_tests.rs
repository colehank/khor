//! An actual carried connection may use an offer before its caller resumes.

use super::*;
use khor_net::carry::{Attached, Links, RoadOrder, transport_addr};

const INNER_ALPN: &[u8] = khor_net::endpoint::ALPN;

async fn carried_endpoint(key: iroh::SecretKey, links: &Links, loopback: bool) -> iroh::Endpoint {
    carried_endpoint_selecting(key, links, loopback, Arc::new(RoadOrder::default())).await
}

async fn carried_endpoint_selecting(
    key: iroh::SecretKey,
    links: &Links,
    loopback: bool,
    selector: Arc<dyn iroh::endpoint::transports::PathSelector>,
) -> iroh::Endpoint {
    let mut builder = iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .secret_key(key)
        .relay_mode(iroh::RelayMode::Disabled)
        .clear_ip_transports()
        .add_custom_transport(links.transport())
        .path_selector(selector)
        .alpns(vec![
            INNER_ALPN.to_vec(),
            khor_net::endpoint::TUNNEL_ALPN.to_vec(),
        ]);
    if loopback {
        builder = builder.bind_addr("127.0.0.1:0").unwrap();
    }
    builder.bind().await.unwrap()
}

async fn echo(conn: &iroh::endpoint::Connection) -> bool {
    tokio::time::timeout(Duration::from_secs(1), async {
        let (mut send, mut recv) = conn.open_bi().await.ok()?;
        send.write_all(b"published-carrier").await.ok()?;
        send.finish().ok()?;
        let bytes = recv.read_to_end(64).await.ok()?;
        Some(bytes == b"published-carrier")
    })
    .await
    .ok()
    .flatten()
    .unwrap_or(false)
}

struct Publication {
    _root: tempfile::TempDir,
    node: Arc<Node>,
    ep: iroh::Endpoint,
    middle_ep: iroh::Endpoint,
    target_ep: iroh::Endpoint,
    target_links: Links,
    middle: crate::DeviceInfo,
    target: crate::DeviceInfo,
    outer_local: iroh::endpoint::Connection,
    outer_remote: iroh::endpoint::Connection,
    attached_target: tokio::sync::oneshot::Receiver<Attached>,
    middle_task: Owned<()>,
    echo_task: Owned<()>,
}

impl Publication {
    async fn new() -> Self {
        Self::new_selecting(Arc::new(RoadOrder::default())).await
    }

    async fn new_selecting(selector: Arc<dyn iroh::endpoint::transports::PathSelector>) -> Self {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join(".khor")).unwrap();
        std::fs::write(
            root.path().join(".khor/relays.json"),
            r#"{"relays":[{"url":"http://127.0.0.1:9"}],"only_mine":true}"#,
        )
        .unwrap();
        let node =
            Arc::new(Node::open_hosting(root.path().to_path_buf(), "published", false).unwrap());
        let ep = carried_endpoint_selecting(node.key.clone(), &node.carry, true, selector).await;
        assert_eq!(ep.id().to_string(), node.device_str());
        let middle_ep = endpoint().await;
        let key = iroh::SecretKey::generate();
        let target_links = Links::new(key.public());
        let target_ep = carried_endpoint(key, &target_links, false).await;
        {
            let loaded = node.devices_loaded().unwrap();
            loaded
                .doc
                .upsert(&middle_ep.id().to_string(), "middle", &[])
                .unwrap();
            loaded
                .doc
                .upsert(&target_ep.id().to_string(), "target", &[])
                .unwrap();
            let mut store = loaded.store;
            store.flush(&loaded.doc).unwrap();
        }
        let rows = node.devices().unwrap();
        let middle = rows
            .iter()
            .find(|d| d.id == middle_ep.id().to_string())
            .unwrap()
            .clone();
        let target = rows
            .iter()
            .find(|d| d.id == target_ep.id().to_string())
            .unwrap()
            .clone();
        let (outer_local, outer_remote) = tokio::time::timeout(Duration::from_secs(5), async {
            tokio::join!(
                ep.connect(middle_ep.addr(), khor_net::endpoint::TUNNEL_ALPN),
                async { middle_ep.accept().await.unwrap().accept().unwrap().await }
            )
        })
        .await
        .unwrap();
        let (outer_local, outer_remote) = (outer_local.unwrap(), outer_remote.unwrap());
        node.wires
            .adopt(&middle.id, Proto::Tunnel, outer_local.clone());
        let (attached, attached_target) = tokio::sync::oneshot::channel();
        let middle_task = Owned(Some(tokio::spawn({
            let conn = outer_remote.clone();
            let links = target_links.clone();
            let (me, via, to) = (ep.id(), middle_ep.id(), target_ep.id());
            async move {
                let (mut send, mut recv) = conn.accept_bi().await.unwrap();
                let target = crate::tunnel::read_target(&mut recv).await.unwrap();
                assert_eq!(target, Ask::To(to).words());
                send.write_all(&[crate::tunnel::OK]).await.unwrap();
                // The fixture adapts this real tunnel directly to B's
                // transport; it does not claim two physical relay legs.
                let up = links.attach(me, Carrier::Hop { via }, recv, send);
                attached.send(up).unwrap();
            }
        })));
        let echo_task = Owned(Some(tokio::spawn({
            let ep = target_ep.clone();
            async move {
                let conn = ep.accept().await.unwrap().accept().unwrap().await.unwrap();
                while let Ok((mut send, mut recv)) = conn.accept_bi().await {
                    let Ok(bytes) = recv.read_to_end(64).await else {
                        break;
                    };
                    if send.write_all(&bytes).await.is_err() {
                        break;
                    }
                    let _ = send.finish();
                }
            }
        })));
        Self {
            _root: root,
            node,
            ep,
            middle_ep,
            target_ep,
            target_links,
            middle,
            target,
            outer_local,
            outer_remote,
            attached_target,
            middle_task,
            echo_task,
        }
    }

    fn opening(&self) -> Owned<bool> {
        let (node, ep, target, middle) = (
            self.node.clone(),
            self.ep.clone(),
            self.target.clone(),
            self.middle.clone(),
        );
        Owned(Some(tokio::spawn(async move {
            let picked = vec![middle.id.clone()];
            node.open_hop(&ep, &target, &[middle], &picked).await
        })))
    }

    async fn close(mut self, attachment: u64) {
        self.node.carry.detach_attached(
            &self.target_ep.id(),
            Carrier::Hop {
                via: self.middle_ep.id(),
            },
            attachment,
        );
        self.target_links.detach(
            &self.ep.id(),
            Carrier::Hop {
                via: self.middle_ep.id(),
            },
        );
        tokio::time::timeout(Duration::from_secs(2), async {
            while asking(self.node.device_str(), &self.target.id) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        tokio::time::timeout(Duration::from_secs(2), self.middle_task.0.as_mut().unwrap())
            .await
            .unwrap()
            .unwrap();
        self.echo_task.0.as_ref().unwrap().abort();
        if let Err(error) = self.echo_task.0.as_mut().unwrap().await {
            assert!(error.is_cancelled());
        }
        self.ep.close().await;
        self.target_ep.close().await;
        self.middle_ep.close().await;
    }
}

async fn observe(cancelled: bool) {
    let mut f = Publication::new().await;
    let (entered, release) = f.node.wires.gate_hop_offered_for_test();
    let mut opening = f.opening();
    let serial = ready(entered).await;
    let remote_attached = tokio::time::timeout(Duration::from_secs(5), &mut f.attached_target)
        .await
        .unwrap()
        .unwrap();
    let conn = tokio::time::timeout(
        Duration::from_secs(5),
        f.ep.connect(
            iroh::EndpointAddr::from_parts(f.target_ep.id(), [transport_addr(&f.target_ep.id())]),
            INNER_ALPN,
        ),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(
        echo(&conn).await,
        "the published carrier actually carried data"
    );
    assert_eq!(
        khor_net::path::classify(&conn),
        khor_net::path::PathKind::Carried
    );
    f.node.wires.adopt(&f.target.id, Proto::Main, conn.clone());
    if cancelled {
        cancel(&mut opening).await;
        drop(release);
    } else {
        release.send(()).unwrap();
        assert!(finish(&mut opening).await);
    }
    let kept = f.node.carry.has(&f.target_ep.id());
    let owned = asking(f.node.device_str(), &f.target.id);
    let answered = echo(&conn).await;
    let wire_open = conn.close_reason().is_none()
        && f.outer_local.close_reason().is_none()
        && f.outer_remote.close_reason().is_none();
    f.close(serial).await;
    tokio::time::timeout(Duration::from_secs(2), remote_attached.ended())
        .await
        .unwrap();
    assert!(wire_open);
    assert!(
        kept && owned && answered,
        "an in-use published carrier needs its node owner: kept={kept}, owned={owned}, echoed={answered}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cancelling_a_publisher_preserves_the_actual_connection_using_its_carrier() {
    observe(true).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn normal_publication_keeps_the_same_actual_carried_connection() {
    observe(false).await;
}

#[path = "incoming_tests.rs"]
mod incoming;

#[path = "sole_peer_tests.rs"]
mod sole_peer;

#[path = "race_tests.rs"]
mod race;

#[path = "pending_use_tests.rs"]
mod pending_use;

#[path = "fresh_middle_tests.rs"]
mod fresh_middle;

#[path = "early_fallback_tests.rs"]
mod early_fallback;

#[path = "pending_drop_tests.rs"]
mod pending_drop;

#[path = "normal_release_tests.rs"]
mod normal_release;
