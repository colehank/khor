//! Scenario e: the uplink switches under a live connection, the shape of
//! a laptop leaving Wi-Fi for a phone's hotspot or a phone leaving Wi-Fi
//! for cellular. The connection should survive it (iroh migrates its
//! paths; khor's wire never goes down) and the journal should say so:
//! the new addresses, and the road the connection moved to.

use std::time::{Duration, Instant, SystemTime};

use patchbay::Lab;

use crate::lab::{mesh, node, relay, result, word};
use crate::nat::{Net, router};

/// How long after the switch the far side may take to hear it again.
const BACK: Duration = Duration::from_secs(30);

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn e_wifi_to_cellular_keeps_the_connection_and_journals_it() {
    let s = "e";
    let lab = Lab::new().await.unwrap();
    let r = relay(&lab, s, "relay", None).await;
    let wifi = router(&lab, "wifi", Net::Home).await;
    let cell = router(&lab, "cell", Net::Strict).await;
    let far = router(&lab, "far", Net::Public).await;
    let dm = lab.add_device("m").uplink(wifi.id()).build().await.unwrap();
    let dp = lab.add_device("p").uplink(far.id()).build().await.unwrap();
    let mover = node(s, dm.clone(), "mover", &[&r], &[]).await;
    let peer = node(s, dp, "peer", &[&r], &[]).await;
    tokio::time::sleep(Duration::from_secs(3)).await;
    mesh(&[&mover, &peer]).await;
    let (_, before) = peer
        .until(&mover, Duration::from_secs(90), |l| l.rtt_ms.is_some())
        .await
        .expect("peer reaches mover before the switch");
    // Let the road settle (a direct path, if one opens, within a round).
    tokio::time::sleep(Duration::from_secs(10)).await;
    let before = peer.link(&mover).await.unwrap_or(before);

    let at_ms = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis() as u64;
    let switched = Instant::now();
    dm.iface("eth0").expect("eth0").replug(cell.id()).await.unwrap();
    eprintln!("[netlab] mover now on cell: v4 {:?}", dm.ip());

    // Watch the peer's word for the mover: never down, and heard again.
    let mut down_seen = false;
    let mut back = None;
    let mut last = None;
    while switched.elapsed() < BACK + Duration::from_secs(15) {
        if let Some(l) = peer.link(&mover).await {
            down_seen |= l.down_ms.is_some();
            // Heard since half a second after the switch.
            let since = switched.elapsed().as_millis() as u64;
            if back.is_none() && l.heard_ms.is_some_and(|h| h + 500 < since) {
                back = Some(Duration::from_millis(since - l.heard_ms.unwrap()));
            }
            last = Some(l);
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    let last = last.expect("a link after the switch");
    let after = |e: &serde_json::Value| e["at"].as_u64().is_some_and(|t| t >= at_ms);
    let addrs = mover.journal(None).into_iter().filter(|e| e["e"] == "addrs" && after(e)).count();
    let paths = mover.journal(Some(&peer)).into_iter().filter(|e| e["e"] == "path" && after(e)).count()
        + peer.journal(Some(&mover)).into_iter().filter(|e| e["e"] == "path" && after(e)).count();
    let retired = mover.journal(Some(&peer)).into_iter().filter(|e| e["e"] == "retired" && after(e)).count()
        + peer.journal(Some(&mover)).into_iter().filter(|e| e["e"] == "retired" && after(e)).count();
    let ok = !down_seen && retired == 0 && back.is_some_and(|b| b <= BACK) && addrs > 0 && paths > 0;
    result(
        s,
        ok,
        format!(
            "before {}; heard again {}; now {}; wire down {down_seen}, retired {retired}; journal: {addrs} addrs, {paths} path lines",
            word(&before),
            back.map_or("never".into(), |b| format!("after {b:.1?}")),
            word(&last),
        ),
    );
    assert!(!down_seen && retired == 0, "the wire survives the switch");
    assert!(back.is_some_and(|b| b <= BACK), "the peer hears the mover within {BACK:?}");
    assert!(addrs > 0, "the mover's journal has its new addresses");
    assert!(paths > 0, "a journal has the road the connection moved to");
}
