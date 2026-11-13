//! Unit state fixture for patch 5, starting with a real established connection
//! and a real transmitted PING. The ACK-only tail uses the actual packet-number
//! allocator and retention policy; those tail records are not a wire simulation.
use super::*;

#[derive(Clone, Copy)]
pub(crate) enum Case {
    ReclaimedLargest,
    RetainedLargest,
    NoNewAckEliciting,
    UnsentLargest,
}

pub(crate) fn verify(conn: &mut Connection, sent_at: Instant, case: Case) {
    let path = PathId::ZERO;
    let space = SpaceId::Data;
    assert!(!conn.is_handshaking() && !conn.is_closed());
    let lower = conn.spaces[space].for_path(path).largest_ack_eliciting_sent;
    let mut tail = conn.spaces[space]
        .for_path(path)
        .sent_packets
        .get(lower)
        .expect("the actual transmitted PING remains unacknowledged")
        .clone();
    assert!(tail.ack_eliciting && tail.size > 0);
    // ACK-only packets do not count towards congestion control and contain
    // nothing to retransmit. Preserve the actual path generation.
    tail.time_sent = sent_at;
    tail.size = 0;
    tail.ack_eliciting = false;
    tail.largest_acked.clear();
    tail.retransmits = Default::default();
    tail.path_retransmits = Default::default();
    tail.stream_frames = Default::default();
    let first = conn.spaces[space]
        .for_path(path)
        .get_tx_number(&mut conn.rng);
    assert!(first > lower);
    let _ = conn.spaces[space].for_path(path).sent(first, tail.clone());
    if matches!(case, Case::ReclaimedLargest) {
        // Reach the real bounded-tail reclamation rather than deleting the
        // largest record by hand or acknowledging a number never allocated.
        for _ in 0..2048 {
            let next = conn.spaces[space]
                .for_path(path)
                .get_tx_number(&mut conn.rng);
            let _ = conn.spaces[space].for_path(path).sent(next, tail.clone());
            if conn.spaces[space]
                .for_path(path)
                .sent_packets
                .get(first)
                .is_none()
            {
                break;
            }
        }
        assert!(
            conn.spaces[space]
                .for_path(path)
                .sent_packets
                .get(first)
                .is_none(),
            "the actual retention policy must have reclaimed the ACK-only tail head"
        );
    } else {
        assert!(
            conn.spaces[space]
                .for_path(path)
                .sent_packets
                .get(first)
                .is_some()
        );
    }
    assert!(
        conn.spaces[space]
            .for_path(path)
            .sent_packets
            .get(lower)
            .is_some()
    );
    let next = conn.spaces[space].for_path(path).next_packet_number;
    assert!(
        first < next,
        "the reclaimed packet number was actually allocated"
    );
    let largest = if matches!(case, Case::UnsentLargest) {
        next
    } else {
        first
    };
    let mut ranges = ArrayRangeSet::new();
    ranges.insert_one(largest);
    if !matches!(case, Case::NoNewAckEliciting) {
        ranges.insert_one(lower);
    }
    let ack = frame::Ack {
        largest,
        delay: 0,
        ranges,
        ecn: None,
    };
    let before = conn.path_stats(path).unwrap();
    let received_at = sent_at + Duration::from_millis(40);
    let result = conn.inner_on_ack_received(received_at, space, path, ack.clone());
    if matches!(case, Case::UnsentLargest) {
        assert_eq!(
            result.unwrap_err().code,
            crate::TransportErrorCode::PROTOCOL_VIOLATION,
            "ACK of an unallocated number must be rejected as a protocol violation"
        );
        assert_eq!(
            conn.path_stats(path).unwrap().rtt_samples,
            before.rtt_samples
        );
        return;
    }
    result.expect("these ACK ranges only name allocated packet numbers");
    let after = conn.path_stats(path).unwrap();
    if matches!(case, Case::RetainedLargest) {
        assert_eq!(after.rtt_samples, before.rtt_samples + 1);
        assert_eq!(after.latest_rtt_sent, Some(sent_at));
        assert_eq!(after.latest_rtt_received, Some(received_at));
    } else {
        assert_eq!(
            after.rtt_samples, before.rtt_samples,
            "a stale largest send time or an ACK-only range cannot create an RTT sample"
        );
        assert_eq!(after.latest_rtt_sent, before.latest_rtt_sent);
        assert_eq!(after.latest_rtt_received, before.latest_rtt_received);
    }
    conn.inner_on_ack_received(received_at + Duration::from_millis(1), space, path, ack)
        .expect("a duplicate legal ACK remains valid");
    assert_eq!(
        conn.path_stats(path).unwrap().rtt_samples,
        after.rtt_samples,
        "a duplicate ACK cannot create another sample"
    );
}
