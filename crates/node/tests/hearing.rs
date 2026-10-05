//! Round NET-11: the hearing test the deaf watch asks before it takes
//! new ears. Healthy ears echo through their own home relay; closed ears
//! with the relay up do not; with no relay answering at all, the network
//! is down and new ears would not help.

mod util;

use std::time::Duration;

use khor_net::endpoint::{self, Relay};
use khor_node::link::{Hearing, ears_heard};

/// Ears that accept whatever arrives, as a resident's accept loop does.
fn answering(ep: &iroh::Endpoint) {
    let ep = ep.clone();
    tokio::spawn(async move {
        while let Some(incoming) = ep.accept().await {
            tokio::spawn(async move {
                if let Ok(conn) = incoming.await {
                    let _ = conn.closed().await;
                }
            });
        }
    });
}

async fn homed(ep: &iroh::Endpoint) {
    tokio::time::timeout(Duration::from_secs(30), async {
        while !ep.addr().addrs.iter().any(|t| matches!(t, iroh::TransportAddr::Relay(_))) {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("the ears home on the tests' relay");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ears_that_work_echo_and_closed_ones_do_not() {
    util::off_the_segment();
    let url = util::loop_relay().to_owned();
    let map = vec![Relay { url: url.clone(), token: None }];
    let relays = vec![url];

    let ep = endpoint::bind_with(iroh::SecretKey::generate(), &map, false).await.unwrap();
    answering(&ep);
    homed(&ep).await;
    assert_eq!(ears_heard(&ep, &map, false, &relays, true).await, Hearing::Echoed, "healthy ears");

    // Closed under it, the relay still up: what new ears cure.
    ep.close().await;
    assert_eq!(ears_heard(&ep, &map, false, &relays, true).await, Hearing::Unheard, "closed ears, relay up");

    // Nothing answers on the road out: not deafness, the network.
    let dead = vec![Relay { url: "http://127.0.0.1:9".into(), token: None }];
    let lone = endpoint::bind_with(iroh::SecretKey::generate(), &dead, false).await.unwrap();
    assert_eq!(
        ears_heard(&lone, &dead, false, &["http://127.0.0.1:9".to_owned()], false).await,
        Hearing::NoRelay,
        "no relay answers"
    );
    lone.close().await;
}
