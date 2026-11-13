//! Path probes should produce a timely sample without changing ordinary ACK policy.
use super::*;

#[test]
fn explicit_path_probe_is_answered_within_one_network_round_trip() {
    let _guard = subscribe();
    let (mut pair, client, server) = setup_ack_frequency_test(Duration::from_millis(30));
    let before = pair
        .client_conn_mut(client)
        .path_stats(PathId::ZERO)
        .unwrap();
    let began = pair.time;

    pair.client_conn_mut(client)
        .ping_path(PathId::ZERO)
        .unwrap();
    pair.drive_client();
    pair.time += Duration::from_millis(10);
    pair.drive_server();
    assert_eq!(pair.server_conn_mut(server).stats().frame_rx.ping, 1);
    pair.time += Duration::from_millis(10);
    pair.drive_client();

    let after = pair
        .client_conn_mut(client)
        .path_stats(PathId::ZERO)
        .unwrap();
    assert_eq!(after.rtt_samples, before.rtt_samples + 1);
    assert_eq!(after.latest_rtt_sent, Some(began));
    assert_eq!(
        after.latest_rtt_received,
        Some(began + Duration::from_millis(20))
    );
    assert_eq!(after.frame_tx.ping - before.frame_tx.ping, 1);
}

#[test]
fn ordinary_connection_ping_keeps_negotiated_delayed_ack() {
    let _guard = subscribe();
    let (mut pair, client, server) = setup_ack_frequency_test(Duration::from_millis(30));
    let before = pair
        .client_conn_mut(client)
        .path_stats(PathId::ZERO)
        .unwrap();
    let acks = pair.server_conn_mut(server).stats().frame_tx.acks;

    pair.client_conn_mut(client).ping();
    pair.drive_client();
    pair.time += Duration::from_millis(10);
    pair.drive_server();
    assert_eq!(pair.server_conn_mut(server).stats().frame_rx.ping, 1);
    assert_eq!(pair.server_conn_mut(server).stats().frame_tx.acks, acks);
    pair.time += Duration::from_millis(10);
    pair.drive_client();

    let after = pair
        .client_conn_mut(client)
        .path_stats(PathId::ZERO)
        .unwrap();
    assert_eq!(after.rtt_samples, before.rtt_samples);
    assert_eq!(after.frame_tx.immediate_ack, before.frame_tx.immediate_ack);
    assert_eq!(after.frame_tx.ping - before.frame_tx.ping, 1);
}

#[test]
fn path_ping_during_handshake_does_not_send_one_rtt_only_ack_request() {
    let _guard = subscribe();
    let mut pair = Pair::default_with_deterministic_pns();
    let mut config = client_config_with_deterministic_pns();
    Arc::get_mut(&mut config.transport)
        .unwrap()
        .mtu_discovery_config(None);
    let client = pair.begin_connect(config);
    assert!(pair.client_conn_mut(client).is_handshaking());
    pair.client_conn_mut(client)
        .ping_path(PathId::ZERO)
        .unwrap();
    pair.drive();
    let server = pair.server.assert_accept();
    pair.finish_connect(client, server);
    assert!(!pair.client_conn_mut(client).is_handshaking());
    assert_eq!(
        pair.client_conn_mut(client).stats().frame_tx.immediate_ack,
        0
    );
}

#[test]
fn resumed_path_ping_does_not_put_ack_request_in_zero_rtt() {
    let _guard = subscribe();
    let mut pair = Pair::default_with_deterministic_pns();
    let mut config = client_config_with_deterministic_pns();
    Arc::get_mut(&mut config.transport)
        .unwrap()
        .mtu_discovery_config(None);
    let first = pair.begin_connect(config.clone());
    pair.drive();
    pair.server.assert_accept();
    let now = pair.time;
    pair.client_conn_mut(first)
        .close(now, VarInt(0), Bytes::new());
    pair.drive();

    let client = pair.begin_connect(config);
    assert!(pair.client_conn_mut(client).has_0rtt());
    assert!(pair.client_conn_mut(client).is_handshaking());
    let stream = pair.client_streams(client).open(Dir::Uni).unwrap();
    const MSG: &[u8] = b"path probe must preserve accepted early bytes";
    pair.client_send(client, stream).write(MSG).unwrap();
    pair.client_conn_mut(client)
        .ping_path(PathId::ZERO)
        .unwrap();
    // No server step: these frames can only use Initial/0-RTT keys.
    pair.drive_client();
    let sent = pair.client_conn_mut(client).stats().frame_tx;
    assert!(sent.stream > 0);
    assert_eq!(sent.immediate_ack, 0);

    pair.drive();
    assert!(pair.client_conn_mut(client).accepted_0rtt());
    let server = pair.server.assert_accept();
    let mut recv = pair.server_recv(server, stream);
    let mut chunks = recv.read(false).unwrap();
    assert_matches!(chunks.next(usize::MAX),
        Ok(Some(chunk)) if chunk.offset == 0 && chunk.bytes == MSG);
    let _ = chunks.finalize();
}
