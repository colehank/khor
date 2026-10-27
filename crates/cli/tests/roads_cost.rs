//! **What the road column costs the device table** (round NET-12): four
//! scratch residents on loopback, paired into one mesh, watched from one
//! of them. Every change of any machine's road register seen in the table
//! is counted — each is one op every machine copies.
//!
//! The span (`DIAG_SPAN_SECS`, default 900): two thirds quiet, then one
//! machine stopped for a sixth and started again for the last sixth.
//!
//! A probe, not a gate:
//! `cargo test -p khor-cli --test roads_cost -- --ignored --nocapture`.

mod scratch;

use std::collections::HashMap;
use std::time::{Duration, Instant};

use khor_node::Node;
use scratch::*;

fn env_u64(k: &str, d: u64) -> u64 {
    std::env::var(k)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(d)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
#[ignore = "a probe of a quarter hour: run by hand"]
async fn what_the_road_column_writes_on_a_four_machine_mesh() {
    let span = Duration::from_secs(env_u64("DIAG_SPAN_SECS", 900));
    let (rr, homes) = (
        scratch("roads-relay"),
        ["a", "b", "c", "d"].map(|t| scratch(&format!("roads-{t}"))),
    );
    let (_relay, url) = local_relay(&rr.0);
    for h in &homes {
        home_on(&h.0, &url);
    }
    let names = ["alpha", "beta", "gamma", "delta"];
    let mut serves: Vec<Option<Reaped>> = homes
        .iter()
        .zip(names)
        .map(|(h, n)| Some(serve(&h.0, n)))
        .collect();
    stood_up(&homes.iter().map(|h| h.0.as_path()).collect::<Vec<_>>()).await;
    let a = Node::open_as(homes[0].0.clone(), "alpha").unwrap();
    for (h, n) in homes.iter().zip(names).skip(1) {
        let joiner = Node::open_as(h.0.clone(), n).unwrap();
        tokio::time::timeout(Duration::from_secs(90), joiner.pair(&a.invite().unwrap()))
            .await
            .expect("pairing must not hang")
            .unwrap();
    }

    let mut last: HashMap<(String, String), khor_node::Road> = HashMap::new();
    let mut writes = 0usize;
    let mut by_phase = [0usize; 3];
    let began = Instant::now();
    let mut stopped = false;
    let mut restarted = false;
    while began.elapsed() < span {
        let t = began.elapsed().as_secs_f64() / span.as_secs_f64();
        let phase = if t < 2.0 / 3.0 {
            0
        } else if t < 5.0 / 6.0 {
            1
        } else {
            2
        };
        if phase == 1 && !stopped {
            stopped = true;
            let _ = khor(&homes[3].0).arg("quit").output();
            serves[3] = None;
            eprintln!("[roads] {:?} delta stopped", began.elapsed());
        }
        if phase == 2 && !restarted {
            restarted = true;
            let _ = std::fs::remove_file(homes[3].0.join(".khor").join("endpoint.json"));
            serves[3] = Some(serve(&homes[3].0, "delta"));
            eprintln!("[roads] {:?} delta started again", began.elapsed());
        }
        for d in a.devices().unwrap_or_default() {
            for r in a.roads_of(&d) {
                let key = (d.shown().to_owned(), r.to.clone());
                if last.get(&key) != Some(&r) {
                    writes += 1;
                    by_phase[phase] += 1;
                    eprintln!(
                        "[roads] {:>6.0}s {} -> {}: {:?} {:?} {:?}",
                        began.elapsed().as_secs_f64(),
                        key.0,
                        &r.to[..8],
                        r.road,
                        r.relay,
                        r.ms
                    );
                    last.insert(key, r);
                }
            }
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
    let quiet_secs = span.as_secs_f64() * 2.0 / 3.0;
    eprintln!(
        "[roads] {} writes seen in {:?} on four machines: {} in the quiet {:.0} s (first writes included), {} while delta was away, {} after it came back",
        writes, span, by_phase[0], quiet_secs, by_phase[1], by_phase[2]
    );
    let _ = serves;
}
