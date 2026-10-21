//! **A machine that refuses a dial is resting, not unreachable** (net-dial
//! B4). A paused machine (or a phone in the background) drops every
//! incoming connection, which the dialler hears at once as a refusal. It
//! used to be written and counted like a dial that timed out: 「不通」, a
//! backoff from five seconds, the row greyed as unreachable — and four
//! machines dialled one sleeping machine 593 times in a day.
//!
//! alpha and beta both serve; alpha pauses; beta's ask is refused, and
//! beta's journal says refused (not a failure with a backoff) and its row
//! for alpha is not unreachable. alpha resumes and dials in, and beta
//! answers with a round at once — the word is cleared and nothing waits
//! for the ten minutes the refusal bought. The control is the first ask.
//!
//! Temporary stores and temporary identities only.

mod util;

use std::fs;
use std::path::Path;
use std::time::Duration;

use khor_node::journal::{By, What};
use khor_node::Node;
use tokio::time::timeout;

struct Scratch(std::path::PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn root(tag: &str) -> Scratch {
    let p = std::env::temp_dir().join(format!("khor-refused-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    util::hermetic(&p);
    Scratch(p)
}

/// 90 s for issue #73's first-touch stall; `files_wire.rs` has why.
async fn wait_for_endpoint_file(root: &Path) {
    let path = root.join(".khor").join("endpoint.json");
    timeout(Duration::from_secs(90), async {
        while !path.exists() {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("serve should write endpoint.json");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_refused_dial_is_resting_and_the_machine_is_visited_when_it_dials_back() {
    let ra = root("a");
    let rb = root("b");
    let sa = Node::open_as(ra.0.clone(), "alpha").unwrap();
    let serve_a = tokio::spawn(async move { sa.serve().await });
    wait_for_endpoint_file(&ra.0).await;

    let a = Node::open_as(ra.0.clone(), "alpha").unwrap();
    let b = Node::open_as(rb.0.clone(), "beta").unwrap();
    let ticket = a.invite().unwrap();
    timeout(Duration::from_secs(60), b.pair(&ticket))
        .await
        .expect("pairing must not hang")
        .unwrap();
    let alpha_id = a.device_str().to_owned();
    let dir = ra.0.join("browse");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("apple"), b"x").unwrap();
    let path = dir.to_str().unwrap().to_owned();
    let sb = Node::open_as(rb.0.clone(), "beta").unwrap();
    let serve_b = tokio::spawn(async move { sb.serve().await });
    wait_for_endpoint_file(&rb.0).await;
    let ask = || async {
        timeout(Duration::from_secs(60), b.ls_of("alpha", &path))
            .await
            .expect("an ask must come back, answered or refused")
    };

    assert!(ask().await.is_ok(), "control: alpha answers beta");
    // alpha has beta's roads (beta's resident wrote them and its first
    // round carried them): a machine with none is never dialled, which is
    // the one-shot pairing's shape and not this test's.
    let beta_id = b.device_str().to_owned();
    let roads = timeout(Duration::from_secs(60), async {
        loop {
            let a_now = Node::open_as(ra.0.clone(), "alpha").unwrap();
            if a_now.devices().unwrap_or_default().iter().any(|d| d.id == beta_id && !d.addrs.is_empty()) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
    })
    .await;
    assert!(roads.is_ok(), "alpha learnt beta's roads");

    khor_node::pause::set_on(&ra.0, true).unwrap();
    tokio::time::sleep(Duration::from_secs(2)).await;
    let t = std::time::Instant::now();
    let why = ask().await.unwrap_err();
    eprintln!("[refused] {:?}: {why}", t.elapsed());
    assert!(t.elapsed() < Duration::from_secs(8), "a refusal is heard at once, not waited out: {:?}", t.elapsed());
    assert!(why.contains(khor_catalog::msg::peer_resting()), "said as resting: {why}");

    let dials: Vec<What> = khor_node::journal::read(&rb.0, Some(&alpha_id), 0)
        .into_iter()
        .map(|e| e.what)
        .filter(|w| matches!(w, What::Dial { won: None, .. }))
        .collect();
    assert!(!dials.is_empty(), "the refused dial is written");
    for d in &dials {
        assert!(
            matches!(d, What::Dial { asleep: true, retry_s: None, .. }),
            "written as refused, with no backoff: {d:?}"
        );
    }
    let row = b.links().await.unwrap().into_iter().find(|(id, _)| *id == alpha_id).map(|(_, l)| l);
    assert_eq!(row.and_then(|l| l.down_ms), None, "resting is not unreachable");

    // alpha resumes: its pump dials beta, and beta answers with a round of
    // its own at once — not ten minutes later.
    let since = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    khor_node::pause::set_on(&ra.0, false).unwrap();
    let back = timeout(Duration::from_secs(60), async {
        loop {
            let after: Vec<What> = khor_node::journal::read(&rb.0, Some(&alpha_id), since)
                .into_iter()
                .map(|e| e.what)
                .collect();
            let inbound = after.iter().position(|w| matches!(w, What::Inbound));
            if let Some(i) = inbound
                && after[i..]
                    .iter()
                    .any(|w| matches!(w, What::Dial { by: By::Forced, won: Some(_), .. }))
            {
                return;
            }
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
    })
    .await;
    if back.is_err() {
        for (who, r, peer) in [("beta", &rb.0, &alpha_id), ("alpha", &ra.0, &beta_id)] {
            for e in khor_node::journal::read(r, Some(peer), since) {
                eprintln!("[{who}] +{}ms {}", e.at.saturating_sub(since), khor_node::journal::words(&e, &|s| s.to_owned()));
            }
        }
    }
    assert!(back.is_ok(), "alpha dialled in and beta visited it at once");
    assert!(ask().await.is_ok(), "and alpha answers again");

    serve_a.abort();
    serve_b.abort();
}
