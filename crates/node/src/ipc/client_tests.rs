//! Real loopback framing across independent tasks, without a second endpoint
//! or an implicit operation replay when negotiation or the reply fails.

use super::{Op, Reply, Target, call_target, lease};
use crate::{deadline, proto};
use std::time::Duration;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    time::Instant,
};

fn target(listener: &TcpListener, max_version: u8, cookie: &str) -> Target {
    Target {
        port: listener.local_addr().unwrap().port(),
        cookie: cookie.into(),
        max_version,
    }
}

async fn respond(listener: TcpListener, leased: bool, cookie: &str, budget: Option<Duration>) {
    assert!(
        deadline::current().is_none(),
        "the receiver must not inherit the callers task local"
    );
    let (mut stream, _) = listener.accept().await.unwrap();
    let request = lease::read(&mut stream).await.unwrap();
    assert_eq!(request.handoff.cookie, cookie);
    assert_eq!(request.by.is_some(), leased);
    if let Some(budget) = budget {
        assert!(request.by.unwrap() <= Instant::now() + budget);
    }
    if leased {
        assert!(
            tokio::time::timeout(Duration::from_millis(20), stream.read_u8())
                .await
                .is_err(),
            "a leased caller keeps its sending direction open after the payload"
        );
    }
    let frame = proto::encode(&Reply::Refused {
        why: "fixture reply".into(),
    })
    .unwrap();
    stream.write_all(&frame).await.unwrap();
    stream.shutdown().await.unwrap();
}

#[tokio::test]
async fn old_capabilities_keep_legacy_half_close_and_new_capabilities_hold_the_lease() {
    for version in [0, 1, lease::VERSION, lease::VERSION + 1] {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let target = target(&listener, version, "cookie");
        let server = tokio::spawn(respond(listener, version >= lease::VERSION, "cookie", None));
        let reply = call_target(target, Op::Hops, || None).await.unwrap();
        assert!(matches!(reply, Reply::Refused { why } if why == "fixture reply"));
        server.await.unwrap();
    }
}

#[tokio::test]
async fn a_refused_connect_refreshes_port_cookie_and_framing_together() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let next_port = listener.local_addr().unwrap().port();
    let old = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let initial = target(&old, 0, "old-cookie");
    drop(old);
    let server = tokio::spawn(respond(listener, true, "new-cookie", None));
    let mut refreshes = 0;
    let reply = call_target(initial, Op::Hops, || {
        refreshes += 1;
        Some(Target {
            port: next_port,
            cookie: "new-cookie".into(),
            max_version: lease::VERSION,
        })
    })
    .await
    .unwrap();
    assert!(matches!(reply, Reply::Refused { .. }));
    assert_eq!(refreshes, 1);
    server.await.unwrap();
}

#[tokio::test]
async fn a_lost_leased_reply_never_refreshes_or_replays_the_operation() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let target = target(&listener, lease::VERSION, "cookie");
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        assert!(lease::read(&mut stream).await.unwrap().by.is_some());
        drop(stream);
    });
    let mut refreshes = 0;
    let result = call_target(target, Op::Hops, || {
        refreshes += 1;
        None
    })
    .await;
    assert!(result.is_err());
    assert_eq!(refreshes, 0);
    server.await.unwrap();
}

#[tokio::test]
async fn the_received_lease_uses_remaining_budget_without_inheriting_the_callers_scope() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let target = target(&listener, lease::VERSION, "cookie");
    let budget = Duration::from_millis(500);
    let server = tokio::spawn(respond(listener, true, "cookie", Some(budget)));
    let result = deadline::scope(
        Instant::now() + budget,
        call_target(target, Op::Hops, || None),
    )
    .await;
    assert!(matches!(result.unwrap(), Reply::Refused { .. }));
    server.await.unwrap();
}
