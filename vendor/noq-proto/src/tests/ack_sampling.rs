//! Patch 5: valid ACK ranges must not invent a send time for a reclaimed packet.
use super::*;
use crate::connection::ack_sampling_contract::{Case, verify};

fn check(case: Case) {
    let _guard = subscribe();
    let mut pair = Pair::default_with_deterministic_pns();
    let (client, _) = pair.connect_with(client_config_with_deterministic_pns());
    pair.client_conn_mut(client).ping();
    // Emit the actual PING, but do not drive the server's reply.
    pair.drive_client();
    let now = pair.time;
    verify(pair.client_conn_mut(client), now, case);
}

#[test]
fn reclaimed_ack_only_largest_does_not_sample_an_older_send_time() {
    check(Case::ReclaimedLargest);
}

#[test]
fn retained_largest_samples_its_send_time_once() {
    check(Case::RetainedLargest);
}

#[test]
fn ack_only_range_without_new_ack_eliciting_packet_takes_no_sample() {
    check(Case::NoNewAckEliciting);
}

#[test]
fn an_unsent_largest_is_rejected_instead_of_testing_stale_rtt() {
    check(Case::UnsentLargest);
}
