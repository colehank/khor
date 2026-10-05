//! Round NET-11: a restarted resident binds the UDP ports it had, so its
//! row in the device table — its addresses, ports included — is left as
//! it was, and a restart writes no block every machine has to copy.

mod scratch;

use std::path::Path;
use std::time::Duration;

use khor_node::Node;
use scratch::*;

fn blocks(root: &Path) -> usize {
    std::fs::read_dir(khor_sync::devices::devices_dir(root))
        .map(|rd| rd.flatten().filter(|e| e.path().extension().is_some_and(|x| x == "loro")).count())
        .unwrap_or(0)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_restarted_resident_keeps_its_ports_and_its_row() {
    let (rd, rr) = (scratch("pin-desk"), scratch("pin-relay"));
    let (_relay, url) = local_relay(&rr.0);
    home_on(&rd.0, &url);
    let mut rows = Vec::new();
    let mut counts = Vec::new();
    for _ in 0..3 {
        let serve = serve(&rd.0, "desk");
        stood_up(&[&rd.0]).await;
        let me = Node::open_as(rd.0.clone(), "desk").unwrap();
        rows.push(me.devices().unwrap().into_iter().find(|d| d.name == "desk").unwrap().addrs);
        counts.push(blocks(&rd.0));
        let _ = khor(&rd.0).arg("quit").output();
        drop(serve);
        let _ = std::fs::remove_file(rd.0.join(".khor").join("endpoint.json"));
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
    assert_eq!(rows[1], rows[0], "the second start's addresses are the first's");
    assert_eq!(rows[2], rows[0], "and the third's");
    assert_eq!(counts[2], counts[0], "no block was written for a restart: {counts:?}");
}
