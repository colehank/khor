//! A known road returning to its pre-fault quality must not wait for a transient median to age out.
use super::*;

const MS: fn(u64) -> Duration = Duration::from_millis;

fn stats(count: u64, sent: Instant, received: Instant, quality: Duration) -> PathStats {
    let mut stats = PathStats::default();
    stats.rtt_samples = count;
    stats.rtt = quality;
    stats.latest_rtt = quality;
    stats.latest_rtt_sent = Some(sent.into_std());
    stats.latest_rtt_received = Some(received.into_std());
    stats
}

fn fault(t: Instant, observed: bool) -> (Measured, transports::FourTuple, (ConnId, PathId)) {
    let road = transports::FourTuple::Ip {
        remote: std::net::SocketAddr::from(([127, 0, 0, 1], 17046)),
        local: None,
    };
    let key = (ConnId(46), PathId::ZERO);
    let old = stats(u64::from(observed), t - MS(3085), t - MS(3000), MS(85));
    let mut reading = Reading::new(road.clone(), &old, t);
    reading.expire(t);
    let mut measured = Measured::default();
    measured.rtts.insert(key, reading);
    measured.mark_silent(t, &[(road.clone(), t - MS(50))], |_| true);
    // A second genuine observation supplies the history needed for the
    // episode, but must not turn an unobserved initial estimate into history.
    let heard = [(road.clone(), (t + MS(1)).into_std())];
    if observed {
        assert_eq!(
            measured
                .recovery_probes(t + MS(250), &[], &heard, |_| true)
                .len(),
            1
        );
    }
    (measured, road, key)
}

fn observe(
    measured: &mut Measured,
    key: (ConnId, PathId),
    sent: Instant,
    received: Instant,
    quality: Duration,
) {
    let road = measured.rtts[&key].road.clone();
    let previous = measured.road(&road).unwrap().0;
    let next = stats(measured.rtts[&key].samples + 1, sent, received, quality);
    assert!(
        measured
            .rtts
            .get_mut(&key)
            .unwrap()
            .observe(&next, received)
    );
    measured.recovery_quality_step(key, quality, previous, received);
}

#[test]
fn returning_to_known_quality_refreshes_a_sparse_transient_window_once() {
    let t = Instant::now();
    let (mut measured, road, key) = fault(t, true);
    observe(&mut measured, key, t + MS(300), t + MS(425), MS(100));
    observe(&mut measured, key, t + MS(550), t + MS(676), MS(101));
    assert_eq!(measured.road(&road).unwrap().0, MS(101));
    let returned = t + MS(800);
    observe(&mut measured, key, returned, t + MS(910), MS(85));
    assert_eq!(
        measured.road(&road).unwrap().0,
        MS(85),
        "a real post-fault round trip returned to the known regime"
    );
    assert_eq!(measured.rtts[&key].quality_since, Some(returned));
    assert_eq!(measured.recovery[&road].quality_steps, 1);
    assert_eq!(
        measured.recovery[&road].attempts, 1,
        "refreshing quality cannot refill probe budget"
    );
    observe(&mut measured, key, t + MS(1050), t + MS(1160), MS(84));
    assert_eq!(
        measured.rtts[&key].quality_since,
        Some(returned),
        "small changes cannot repeatedly restart the epoch"
    );
    assert_eq!(measured.recovery[&road].quality_steps, 1);
}

#[test]
fn a_quality_still_above_the_known_regime_does_not_refresh_it() {
    let t = Instant::now();
    let (mut measured, road, key) = fault(t, true);
    observe(&mut measured, key, t + MS(300), t + MS(425), MS(100));
    observe(&mut measured, key, t + MS(550), t + MS(676), MS(101));
    observe(&mut measured, key, t + MS(800), t + MS(910), MS(86));
    assert!(measured.rtts[&key].quality_since.is_none());
    assert_eq!(measured.road(&road).unwrap().0, MS(100));
}

#[test]
fn one_returned_pulse_cannot_hide_subsequent_slow_round_trips() {
    let t = Instant::now();
    let (mut measured, road, key) = fault(t, true);
    observe(&mut measured, key, t + MS(300), t + MS(425), MS(100));
    observe(&mut measured, key, t + MS(550), t + MS(676), MS(101));
    observe(&mut measured, key, t + MS(800), t + MS(910), MS(85));
    observe(&mut measured, key, t + MS(1050), t + MS(1185), MS(110));
    assert_eq!(
        measured.road(&road).unwrap().0,
        MS(110),
        "new slow samples must still stop an unjustified move"
    );
    assert_eq!(
        measured.samples_after(&road, t + MS(910)),
        1,
        "the confirmation still needs two post-decision ticks"
    );
}

#[test]
fn an_expired_episode_cannot_refresh_from_an_old_baseline() {
    let t = Instant::now();
    let (mut measured, road, key) = fault(t, true);
    observe(&mut measured, key, t + MS(300), t + MS(425), MS(100));
    observe(&mut measured, key, t + MS(550), t + MS(676), MS(101));
    observe(&mut measured, key, t + MS(2300), t + MS(2410), MS(85));
    assert!(measured.rtts[&key].quality_since.is_none());
    assert_eq!(measured.recovery[&road].quality_steps, 0);
}

#[test]
fn a_healthy_road_keeps_its_original_sparse_median() {
    let t = Instant::now();
    let (mut measured, road, key) = fault(t, true);
    measured.silent.clear();
    measured.recovery.clear();
    observe(&mut measured, key, t + MS(300), t + MS(425), MS(100));
    observe(&mut measured, key, t + MS(550), t + MS(676), MS(101));
    observe(&mut measured, key, t + MS(800), t + MS(910), MS(85));
    assert_eq!(measured.road(&road).unwrap().0, MS(100));
    assert!(measured.rtts[&key].quality_since.is_none());
}
