//! A prepared handshake has its own initial route beside a live IP wire.
use super::*;
use khor_net::carry::{Carrier, Links, transport_addr};

async fn endpoint(key: iroh::SecretKey, links: &Links) -> iroh::Endpoint {
    iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .secret_key(key)
        .relay_mode(iroh::RelayMode::Disabled)
        .clear_ip_transports()
        .bind_addr("127.0.0.1:0")
        .unwrap()
        .add_custom_transport(links.transport())
        .alpns(vec![ALPN.to_vec()])
        .bind()
        .await
        .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_prepared_handshake_uses_custom_path_zero_beside_a_live_ip_connection() {
    let (client_key, server_key) = (iroh::SecretKey::generate(), iroh::SecretKey::generate());
    let client_links = Links::new(client_key.public());
    let server_links = Links::new(server_key.public());
    let client = endpoint(client_key, &client_links).await;
    let server = endpoint(server_key, &server_links).await;
    let ip_only = iroh::EndpointAddr::from_parts(
        server.id(),
        server.addr().addrs.into_iter().filter(|addr| addr.is_ip()),
    );
    let (warm, warm_remote) = tokio::time::timeout(Duration::from_secs(5), async {
        tokio::join!(client.connect(ip_only, ALPN), async {
            server.accept().await.unwrap().accept().unwrap().await
        })
    })
    .await
    .unwrap();
    let (warm, warm_remote) = (warm.unwrap(), warm_remote.unwrap());
    assert!(
        warm.paths()
            .iter()
            .any(|path| { path.id() == iroh::endpoint::PathId::ZERO && path.is_ip() })
    );
    let via = iroh::SecretKey::generate().public();
    let (a, b) = tokio::io::duplex(64 << 10);
    let (ar, aw) = tokio::io::split(a);
    let (br, bw) = tokio::io::split(b);
    let local = client_links.attach(server.id(), Carrier::Hop { via }, ar, aw);
    let remote = server_links.attach(client.id(), Carrier::Hop { via }, br, bw);
    let addr = iroh::EndpointAddr::from_parts(server.id(), [transport_addr(&server.id())]);
    let (carried, accepted) = tokio::time::timeout(Duration::from_secs(5), async {
        tokio::join!(connect_prepared_carrier(&client, addr, ALPN), async {
            server.accept().await.unwrap().accept().unwrap().await
        })
    })
    .await
    .unwrap();
    let (carried, accepted) = (carried.unwrap(), accepted.unwrap());
    assert!(carried.paths().iter().any(|path| {
        path.id() == iroh::endpoint::PathId::ZERO
            && path.remote_addr() == &transport_addr(&server.id())
    }));
    assert_eq!(carried.remote_id(), server.id());
    tokio::time::timeout(Duration::from_secs(2), async {
        tokio::join!(
            async {
                let (mut send, mut recv) = carried.open_bi().await.unwrap();
                send.write_all(b"initial-carrier").await.unwrap();
                send.finish().unwrap();
                assert_eq!(recv.read_to_end(64).await.unwrap(), b"initial-carrier");
            },
            async {
                let (mut send, mut recv) = accepted.accept_bi().await.unwrap();
                let bytes = recv.read_to_end(64).await.unwrap();
                send.write_all(&bytes).await.unwrap();
                send.finish().unwrap();
            }
        );
    })
    .await
    .unwrap();
    let wrong = iroh::SecretKey::generate().public();
    let addr = iroh::EndpointAddr::from_parts(wrong, [transport_addr(&server.id())]);
    let mut accepting = Aborts(Some(tokio::spawn({
        let server = server.clone();
        async move {
            if let Some(incoming) = server.accept().await
                && let Ok(connecting) = incoming.accept()
                && let Ok(conn) = connecting.await
            {
                conn.close(0u32.into(), b"identity control complete");
            }
        }
    })));
    let rejected = tokio::time::timeout(
        Duration::from_secs(5),
        connect_prepared_carrier(&client, addr, ALPN),
    )
    .await
    .unwrap()
    .expect_err("pinning a transport cannot bypass endpoint identity");
    if let Some(task) = accepting.0.take() {
        task.abort();
        let _ = task.await;
    }
    eprintln!("[initial-custom-identity-control] {rejected}");
    assert!(matches!(
        rejected,
        iroh::endpoint::ConnectError::Connecting { .. }
    ));
    assert!(warm.close_reason().is_none() && warm_remote.close_reason().is_none());
    carried.close(0u32.into(), b"initial route contract complete");
    warm.close(0u32.into(), b"warm route contract complete");
    client_links.detach_attached(&server.id(), Carrier::Hop { via }, local.serial());
    server_links.detach_attached(&client.id(), Carrier::Hop { via }, remote.serial());
    tokio::time::timeout(Duration::from_secs(2), async {
        tokio::join!(local.ended(), remote.ended());
    })
    .await
    .unwrap();
    tokio::join!(client.close(), server.close());
}

#[tokio::test]
async fn an_initial_custom_address_must_be_explicit_in_the_dial() {
    let client = iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .relay_mode(iroh::RelayMode::Disabled)
        .bind()
        .await
        .unwrap();
    let peer = iroh::SecretKey::generate().public();
    let iroh::TransportAddr::Custom(custom) = transport_addr(&peer) else {
        unreachable!()
    };
    let addr = iroh::EndpointAddr::from_parts(peer, std::iter::empty::<iroh::TransportAddr>());
    let result = client
        .connect_with_opts(
            addr,
            ALPN,
            iroh::endpoint::ConnectOptions::default().with_initial_custom_addr(custom),
        )
        .await;
    assert!(matches!(
        result,
        Err(iroh::endpoint::ConnectWithOptsError::InvalidInitialCustomAddr { .. })
    ));
    client.close().await;
}

async fn answer_sessions(conn: &iroh::endpoint::Connection) {
    let (mut send, mut recv) = conn.accept_bi().await.unwrap();
    let bytes = recv.read_to_end(MAX_FRAME).await.unwrap();
    assert!(matches!(
        proto::question(&bytes).unwrap(),
        Request::Sessions
    ));
    send.write_all(&proto::encode(&Response::SessionRows { rows: vec![] }).unwrap())
        .await
        .unwrap();
    send.finish().unwrap();
}

async fn check_warm_ip(local: &iroh::endpoint::Connection, remote: &iroh::endpoint::Connection) {
    let (response, ()) = tokio::time::timeout(Duration::from_secs(2), async {
        tokio::join!(
            exchange(
                local,
                "warm IP control",
                &Request::Sessions,
                Duration::from_secs(1)
            ),
            answer_sessions(remote),
        )
    })
    .await
    .unwrap();
    assert!(matches!(response, Ok(Response::SessionRows { .. })));
    assert!(local.close_reason().is_none() && remote.close_reason().is_none());
}

/// Two endpoints for the same identity separate the cached old IP route
/// from the installed carrier. The old authenticated IP connection keeps
/// answering, while only the chosen endpoint accepts a new handshake.
/// This is a real Node dial/response contract, not a two-leg relay fixture.
async fn observe_installed(early: bool, direct_answers: bool) {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let key = iroh::SecretKey::generate();
    let direct_links = Links::new(key.public());
    let direct = endpoint(key.clone(), &direct_links).await;
    let links = Links::new(key.public());
    let carried = iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .secret_key(key)
        .relay_mode(iroh::RelayMode::Disabled)
        .clear_ip_transports()
        .add_custom_transport(links.transport())
        .alpns(vec![ALPN.to_vec()])
        .bind()
        .await
        .unwrap();
    let (_root, node, mut peer, initial_ep) = fixture_for(direct.id().to_string()).await;
    initial_ep.close().await;
    let ep = endpoint(node.key.clone(), &node.carry).await;
    peer.addrs = direct
        .addr()
        .addrs
        .iter()
        .filter_map(|addr| match addr {
            iroh::TransportAddr::Ip(addr) => Some(addr.to_string()),
            _ => None,
        })
        .collect();
    assert!(!peer.addrs.is_empty());
    let (warm, warm_remote) = tokio::time::timeout(Duration::from_secs(5), async {
        tokio::join!(ep.connect(direct.addr(), ALPN), async {
            direct.accept().await.unwrap().accept().unwrap().await
        })
    })
    .await
    .unwrap();
    let (warm, warm_remote) = (warm.unwrap(), warm_remote.unwrap());
    check_warm_ip(&warm, &warm_remote).await;
    assert!(node.wires.live(&peer.id, wires::Proto::Main).is_none());
    let via = iroh::SecretKey::generate().public();
    let (a, b) = tokio::io::duplex(64 << 10);
    let (ar, aw) = tokio::io::split(a);
    let (br, bw) = tokio::io::split(b);
    let local = node
        .carry
        .attach(carried.id(), Carrier::Hop { via }, ar, aw);
    let remote = links.attach(ep.id(), Carrier::Hop { via }, br, bw);
    assert!(node.carried_addr(&peer.id).is_some());
    let requests = Arc::new(AtomicUsize::new(0));
    let mut serving = Aborts(Some(tokio::spawn({
        let server = if direct_answers {
            direct.clone()
        } else {
            carried.clone()
        };
        let requests = requests.clone();
        async move {
            let conn = server
                .accept()
                .await
                .unwrap()
                .accept()
                .unwrap()
                .await
                .unwrap();
            answer_sessions(&conn).await;
            requests.fetch_add(1, Ordering::SeqCst);
            conn.closed().await;
        }
    })));
    let by = tokio::time::Instant::now() + Duration::from_secs(1);
    let outcome = tokio::time::timeout_at(
        by + Duration::from_millis(300),
        crate::deadline::scope(by, async {
            if early {
                node.ask_on(&ep, &peer, &Request::Sessions).await
            } else {
                let line = node
                    .line_to_inner(&ep, &peer, wires::Proto::Main, None, false)
                    .await
                    .map_err(|error| unreached_words(&peer.name, error))?;
                node.exchange_on(&line, &Request::Sessions)
                    .await
                    .map_err(Miss::words)
            }
        }),
    )
    .await;
    let answered = matches!(outcome, Ok(Ok(Response::SessionRows { .. })));
    let path = node
        .wires
        .live(&peer.id, wires::Proto::Main)
        .map(|conn| khor_net::path::classify(&conn));
    let carrier_kept = node.carry.has(&carried.id());
    check_warm_ip(&warm, &warm_remote).await;
    let total = requests.load(Ordering::SeqCst);
    // A server still waiting for an unreachable handshake is cancelled
    // before endpoint shutdown. Closing its endpoint first makes accept()
    // return None and hides the actual business assertion behind cleanup.
    serving.0.as_ref().unwrap().abort();
    if let Err(error) = serving.0.as_mut().unwrap().await {
        assert!(
            error.is_cancelled(),
            "scratch response owner must not panic"
        );
    }
    node.carry
        .detach_attached(&carried.id(), Carrier::Hop { via }, local.serial());
    links.detach_attached(&ep.id(), Carrier::Hop { via }, remote.serial());
    tokio::time::timeout(Duration::from_secs(2), async {
        tokio::join!(local.ended(), remote.ended());
    })
    .await
    .unwrap();
    tokio::join!(ep.close(), direct.close(), carried.close());
    let expected = if direct_answers {
        khor_net::path::PathKind::Direct
    } else {
        khor_net::path::PathKind::Carried
    };
    assert!(
        carrier_kept,
        "a losing dial must preserve the installed carrier"
    );
    assert!(
        answered && path == Some(expected),
        "the usable initial route must win within the original caller window: early={early}, direct_answers={direct_answers}, answered={answered}, path={path:?}, requests={total}"
    );
    assert_eq!(total, 1, "the successful reading is not replayed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ordinary_dial_uses_an_installed_carrier_beside_a_live_old_ip_wire() {
    observe_installed(false, false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn early_dial_uses_an_installed_carrier_beside_a_live_old_ip_wire() {
    observe_installed(true, false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_installed_carrier_does_not_delay_a_fast_direct_ordinary_dial() {
    observe_installed(false, true).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_installed_carrier_does_not_delay_a_fast_direct_early_dial() {
    observe_installed(true, true).await;
}
