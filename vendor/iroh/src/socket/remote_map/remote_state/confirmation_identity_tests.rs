//! A road's quality and confirmation evidence must describe the same connection copy.
use super::*;

const MS: fn(u64) -> Duration = Duration::from_millis;

fn sample(
    reading: Option<Reading>,
    road: &transports::FourTuple,
    received: Instant,
    rtt: Duration,
) -> Reading {
    let mut stats = PathStats::default();
    stats.rtt = rtt;
    stats.latest_rtt = rtt;
    stats.rtt_samples = reading.as_ref().map_or(1, |r| r.samples + 1);
    stats.latest_rtt_sent = Some((received - rtt).into_std());
    stats.latest_rtt_received = Some(received.into_std());
    if let Some(mut reading) = reading {
        assert!(reading.observe(&stats, received));
        reading
    } else {
        Reading::new(road.clone(), &stats, received)
    }
}

fn copies(
    t: Instant,
) -> (
    Measured,
    transports::FourTuple,
    (ConnId, PathId),
    (ConnId, PathId),
) {
    let road = transports::FourTuple::Ip {
        remote: std::net::SocketAddr::from(([127, 0, 0, 1], 17047)),
        local: None,
    };
    let busy_key = (ConnId(470), PathId::ZERO);
    let other_key = (ConnId(471), PathId::ZERO);
    let mut busy = None;
    for before in [1000, 750, 500] {
        busy = Some(sample(busy, &road, t - MS(before), MS(85)));
    }
    let mut other = None;
    for after in [200, 450] {
        other = Some(sample(other, &road, t + MS(after), MS(90)));
    }
    let mut measured = Measured::default();
    measured.rtts.insert(busy_key, busy.unwrap());
    measured.rtts.insert(other_key, other.unwrap());
    (measured, road, busy_key, other_key)
}

#[test]
fn another_copy_cannot_confirm_a_quality_with_no_post_decision_sample() {
    let t = Instant::now();
    let (measured, road, _, _) = copies(t);
    assert_eq!(measured.observation(&road).unwrap().rtt(), MS(85));
    assert_eq!(
        measured.samples_after(&road, t),
        0,
        "two new observations on another copy cannot confirm the representative's old quality"
    );
}

#[test]
fn one_new_representative_sample_cannot_borrow_another_copys_two() {
    let t = Instant::now();
    let (mut measured, road, busy_key, _) = copies(t);
    let busy = measured.rtts.remove(&busy_key).unwrap();
    measured
        .rtts
        .insert(busy_key, sample(Some(busy), &road, t + MS(700), MS(85)));
    assert_eq!(measured.observation(&road).unwrap().rtt(), MS(85));
    assert_eq!(
        measured.samples_after(&road, t),
        1,
        "the representative itself still lacks the second post-decision observation"
    );
}

#[test]
fn a_new_representative_supplies_its_own_confirmation_evidence() {
    let t = Instant::now();
    let (mut measured, road, _, other_key) = copies(t);
    for after in [700, 950] {
        let other = measured.rtts.remove(&other_key).unwrap();
        measured
            .rtts
            .insert(other_key, sample(Some(other), &road, t + MS(after), MS(90)));
    }
    assert_eq!(measured.observation(&road).unwrap().rtt(), MS(90));
    assert_eq!(measured.samples_after(&road, t), 4);
}
