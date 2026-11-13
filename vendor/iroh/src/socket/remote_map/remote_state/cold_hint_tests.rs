//! Cold retry policy preserves newly learned addresses and non-IP routes.

use super::*;
use iroh_base::{RelayUrl, SecretKey};
use std::time::Duration;

#[test]
fn khor_cold_retry_discards_only_ip_sources_before_the_failed_attempt() {
    let before = Instant::now();
    let old = transports::Addr::Ip("127.0.0.1:41001".parse().unwrap());
    let fresh = transports::Addr::Ip("127.0.0.1:41002".parse().unwrap());
    let updated = transports::Addr::Ip("127.0.0.1:41003".parse().unwrap());
    let relay: RelayUrl = "https://relay.example".parse().unwrap();
    let relay = transports::Addr::from((relay, SecretKey::generate().public()));
    let mut paths = RemotePathState::new(Arc::new(SocketMetrics::default()));
    paths.insert_multiple(
        [old.clone(), fresh.clone(), updated.clone(), relay.clone()].into_iter(),
        Source::App,
    );
    for path in paths.paths.values_mut() {
        path.sources
            .insert(Source::App, before - Duration::from_secs(1));
    }
    paths
        .paths
        .get_mut(&fresh)
        .unwrap()
        .sources
        .insert(Source::App, before + Duration::from_millis(1));
    paths.paths.get_mut(&updated).unwrap().sources.insert(
        Source::AddressLookup {
            name: "fresh".into(),
        },
        before + Duration::from_millis(1),
    );
    assert_eq!(paths.discard_ip_hints_before(before), 1);
    let remaining: Vec<_> = paths.addrs().cloned().collect();
    assert!(!remaining.contains(&old));
    assert!(
        remaining.contains(&fresh) && remaining.contains(&updated) && remaining.contains(&relay)
    );
}

#[test]
fn khor_cold_retry_with_only_new_ip_information_keeps_it() {
    let before = Instant::now();
    let fresh = transports::Addr::Ip("[::1]:41002".parse().unwrap());
    let mut paths = RemotePathState::new(Arc::new(SocketMetrics::default()));
    paths.insert_multiple([fresh.clone()].into_iter(), Source::App);
    paths
        .paths
        .get_mut(&fresh)
        .unwrap()
        .sources
        .insert(Source::App, before + Duration::from_millis(1));
    assert_eq!(paths.discard_ip_hints_before(before), 0);
    assert!(paths.addrs().any(|addr| addr == &fresh));
}
