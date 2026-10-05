//! Round NET-9: a refusal that crosses machines reads in the language of
//! the machine showing it, not of the one that said it.
//!
//! The owner's Mac, 2026-10-05: 「那台机器拒绝了这次读取：This machine isn't
//! in the machine table — pair first」 — hinton's gate, in hinton's
//! English, inside the Mac's Chinese. Here the far resident speaks
//! English, the near resident English too, and the face asking through
//! it Chinese: the refusal crosses the wire once and the ipc once, and
//! must arrive in Chinese.

mod scratch;

use std::time::Duration;

use khor_catalog::lang::{Lang, set};
use khor_catalog::msg;
use khor_node::Node;
use scratch::*;
use tokio::time::timeout;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_far_refusal_reads_in_this_machines_language() {
    // The one test in this binary: the language is the process's.
    set(Lang::Zh);
    let (ra, rb, rr) = (scratch("words-a"), scratch("words-b"), scratch("words-r"));
    let (_relay, url) = local_relay(&rr.0);
    home_on(&ra.0, &url);
    home_on(&rb.0, &url);
    let english = [("LC_ALL", "en_US.UTF-8")];
    let _a = serve_with(&ra.0, "alpha", &english);
    let _b = serve_with(&rb.0, "beta", &english);
    stood_up(&[&ra.0, &rb.0]).await;
    let alpha = Node::open_as(ra.0.clone(), "alpha").unwrap();
    let beta = Node::open_as(rb.0.clone(), "beta").unwrap();
    timeout(Duration::from_secs(90), beta.pair(&alpha.invite().unwrap()))
        .await
        .expect("pairing must not hang")
        .unwrap();
    // Alpha shows beta the door: its gate now refuses beta by name.
    alpha.forget_device("beta").unwrap();

    let dir = ra.0.display().to_string();
    let why = timeout(Duration::from_secs(60), beta.ls_of("alpha", &dir))
        .await
        .expect("a refusal must not hang")
        .unwrap_err();
    assert!(why.contains(msg::not_paired()), "alpha's refusal, in Chinese: {why}");
    set(Lang::En);
    let theirs = msg::not_paired().to_owned();
    set(Lang::Zh);
    assert!(!why.contains(&theirs), "and not in alpha's English: {why}");
}
