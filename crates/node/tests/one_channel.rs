//! **One channel** (STORE-BRIEF phase 5): the gates for a message pushed
//! to a machine. Three scratch homes, each with its own serve, on
//! loopback with a scratch relay and no mDNS (`util::hermetic`):
//!
//! - alpha posts `@beta:~/in x` → it lands on beta at `~/in/x`, the
//!   directory made, with nobody pressing anything;
//! - alpha posts `@beta gamma:/path/y` → beta pulls `y` from gamma, not
//!   through alpha, and its record says so;
//! - alpha posts `@beta` with beta stopped → refused at once, naming
//!   beta, and nothing is written to the broadcast;
//! - a phone-shaped alpha (`hosts = false`) pushes the same way, and a
//!   push it cuts (its grace over, `link::cut_pushes`) fails on the
//!   target with 「发送中断：手机已挂起」.
//!
//! (A third machine taking the file as an offer is `act.rs`'s gate.)
//! Every await under a timeout.

mod util;

use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use khor_node::Node;
use khor_node::wide::BROADCAST;
use khor_sync::chat::Step;
use tokio::time::timeout;

struct Scratch(std::path::PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

impl std::ops::Deref for Scratch {
    type Target = std::path::Path;

    fn deref(&self) -> &std::path::Path {
        &self.0
    }
}

fn root(tag: &str) -> Scratch {
    let p = std::env::temp_dir().join(format!("khor-one-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    util::hermetic(&p);
    Scratch(p)
}

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

async fn sync(n: &Node) -> Vec<(String, Result<String, String>)> {
    timeout(Duration::from_secs(45), n.sync_now())
        .await
        .expect("sync must not hang")
        .unwrap()
}

fn heard(out: &[(String, Result<String, String>)], name: &str) -> bool {
    out.iter().any(|(n, r)| n == name && r.is_ok())
}

/// Waits until `path` holds exactly `bytes`.
async fn lands(path: &Path, bytes: &[u8], what: &str) {
    timeout(Duration::from_secs(60), async {
        while fs::read(path).ok().as_deref() != Some(bytes) {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("{what}: {} never held the bytes", path.display()));
}

/// The message `id` as `n` reads it.
fn message(n: &Node, id: &str) -> Option<khor_sync::chat::Message> {
    n.log(BROADCAST).unwrap().messages.into_iter().find(|m| m.id == id)
}

/// Three serving machines that all reach each other: beta and gamma
/// paired through alpha, then rounds until each has heard the other.
async fn three(ra: &Path, rb: &Path, rc: &Path, alpha_hosts: bool) -> (Node, Node, Node, Vec<tokio::task::JoinHandle<()>>) {
    let mut serves = Vec::new();
    for (r, name, hosts) in [(ra, "alpha", alpha_hosts), (rb, "beta", true), (rc, "gamma", true)] {
        let s = Node::open_hosting(r.to_path_buf(), name, hosts).unwrap();
        // Resumable, so a test can take a machine off the network the way
        // a stopped one is (`link::deafen`).
        serves.push(tokio::spawn(async move {
            let _ = s.serve_resumable().await;
        }));
        wait_for_endpoint_file(r).await;
    }
    let a = Node::open_hosting(ra.to_path_buf(), "alpha", alpha_hosts).unwrap();
    let b = Node::open_as(rb.to_path_buf(), "beta").unwrap();
    let c = Node::open_as(rc.to_path_buf(), "gamma").unwrap();
    for n in [&b, &c] {
        let ticket = a.invite().unwrap();
        timeout(Duration::from_secs(45), n.pair(&ticket))
            .await
            .expect("pairing must not hang")
            .unwrap();
    }
    let began = Instant::now();
    loop {
        let (ob, oc) = (sync(&b).await, sync(&c).await);
        if heard(&ob, "gamma") && heard(&oc, "beta") && heard(&ob, "alpha") {
            break;
        }
        assert!(began.elapsed() < Duration::from_secs(90), "beta and gamma reach each other: {ob:?} {oc:?}");
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
    sync(&a).await;
    (a, b, c, serves)
}

/// **Pushed, from here and from a third machine, and refused when the
/// target is not there** (the module head's first three gates).
#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
async fn a_push_lands_where_it_names_and_an_absent_target_is_refused() {
    let (ra, rb, rc) = (root("a"), root("b"), root("c"));
    let (a, b, c, serves) = three(&ra, &rb, &rc, true).await;

    // ── alpha posts `@beta:~/in x` ─────────────────────────
    let x: Vec<u8> = (0..700_000u32).map(|i| (i % 241) as u8).collect();
    let src = ra.join("x");
    fs::write(&src, &x).unwrap();
    assert!(!rb.join("in").exists(), "control: the directory is not there before");
    let id = timeout(Duration::from_secs(60), a.post("@beta:~/in here it is", std::slice::from_ref(&src)))
        .await
        .expect("a post must not hang")
        .unwrap();
    lands(&rb.join("in").join("x"), &x, "pushed to beta's ~/in").await;
    assert!(!rb.join("khor-drop").join("x").exists(), "where it was named, not also the default");
    let m = message(&b, &id).expect("the message is on beta");
    assert_eq!((m.to.len(), m.to[0].device.as_str(), m.to[0].path.as_str()), (1, b.device_str(), "~/in"));
    assert_eq!(m.note, "here it is");
    // beta's record of it, written by beta, read on alpha once it syncs.
    timeout(Duration::from_secs(60), async {
        loop {
            sync(&a).await;
            if message(&a, &id).and_then(|m| m.fates.get(b.device_str()).cloned()).is_some_and(|f| f.step == Step::Landed) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    })
    .await
    .expect("alpha reads beta's landing");
    assert!(!rc.join("khor-drop").join("x").exists(), "gamma was not a target: nothing landed there");

    // ── alpha posts `@beta gamma:/…/y`: beta pulls from gamma ──
    let y: Vec<u8> = (0..500_000u32).map(|i| (i % 239) as u8).collect();
    let far = rc.join("y");
    fs::write(&far, &y).unwrap();
    let typed = format!("@beta gamma:{}", far.display());
    let id = timeout(Duration::from_secs(60), a.post(&typed, &[]))
        .await
        .expect("a post must not hang")
        .unwrap();
    lands(&rb.join("khor-drop").join("y"), &y, "beta's copy of gamma's file").await;
    let fate = timeout(Duration::from_secs(30), async {
        loop {
            if let Some(f) = message(&b, &id).and_then(|m| m.fates.get(b.device_str()).cloned())
                && f.step == Step::Landed
            {
                break f;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("beta records the landing");
    assert_eq!(fate.via, c.device_str(), "pulled from gamma, the holder, not through alpha");
    let m = message(&b, &id).unwrap();
    let khor_node::MsgBody::Files(files) = &m.body else { panic!("a files message") };
    assert_eq!(files[0].holder, c.device_str(), "the file names its holder, resolved when composed");
    assert!(!ra.join("y").exists() && !ra.join("khor-drop").join("y").exists(), "nothing went through alpha's disk");

    // A file on a machine that is not there: refused when composed.
    let typed = "@beta gamma:/nowhere/z".to_owned();
    let refused = timeout(Duration::from_secs(60), a.post(&typed, &[])).await.expect("must not hang").unwrap_err();
    assert!(refused.contains("gamma") && refused.contains("/nowhere/z"), "{refused}");

    // ── alpha posts `@beta` with beta stopped ──────────────
    let before = a.log(BROADCAST).unwrap().messages.len();
    assert!(khor_node::link::deafen(&rb).await, "beta's serve is resumable");
    let began = Instant::now();
    let refused = timeout(Duration::from_secs(60), a.post("@beta are you there", &[]))
        .await
        .expect("a refusal must not hang")
        .unwrap_err();
    assert_eq!(refused, khor_catalog::msg::post_target_down("beta"), "the sentence names the machine");
    assert_eq!(a.log(BROADCAST).unwrap().messages.len(), before, "nothing was written to the broadcast");
    eprintln!("[one] refusal said in {:?}", began.elapsed());
    // Control: the same words to nobody in particular still go.
    timeout(Duration::from_secs(60), a.post("are you there", &[])).await.expect("must not hang").unwrap();
    assert_eq!(a.log(BROADCAST).unwrap().messages.len(), before + 1, "control: a post with no target is written");

    for s in serves {
        s.abort();
    }
}

/// **A phone-shaped sender pushes the same way** (`hosts = false`): it
/// serves the bytes while the push runs, and the count of pushes in
/// flight is back to nothing once they end.
#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
async fn a_phone_shaped_machine_pushes_the_same_way() {
    let (ra, rb, rc) = (root("pa"), root("pb"), root("pc"));
    let (a, _b, _c, serves) = three(&ra, &rb, &rc, false).await;
    // `cut_pushes` reaches the serve on alpha's home through its door,
    // which only a resumable serve opens (`three` starts them so).
    // The node itself is phone-shaped. (Its row in the table is not
    // asserted: a round's quota reading reopens a node on this home with
    // the build's own answer, which in a test build is a computer's.)
    assert!(!a.hosts(), "control: alpha is phone-shaped");
    let photo: Vec<u8> = (0..900_000u32).map(|i| (i % 233) as u8).collect();
    let src = ra.join("photo.heic");
    fs::write(&src, &photo).unwrap();
    timeout(Duration::from_secs(60), a.post("@beta @gamma:~/pics", std::slice::from_ref(&src)))
        .await
        .expect("a post must not hang")
        .unwrap();
    lands(&rb.join("khor-drop").join("photo.heic"), &photo, "beta's").await;
    lands(&rc.join("pics").join("photo.heic"), &photo, "gamma's, in its own directory").await;
    timeout(Duration::from_secs(30), async {
        while khor_node::post::pushes_in_flight() > 0 {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("every push ends");

    // **The grace runs out mid-push**: a file big enough to be moving
    // when the phone is told so; beta ends its pull and says why.
    let big: Vec<u8> = (0..96_000_000u32).map(|i| (i % 229) as u8).collect();
    let src = ra.join("video.mov");
    fs::write(&src, &big).unwrap();
    let id = timeout(Duration::from_secs(60), a.post("@beta", std::slice::from_ref(&src)))
        .await
        .expect("a post must not hang")
        .unwrap();
    timeout(Duration::from_secs(60), async {
        loop {
            let moving = message(&_b, &id)
                .and_then(|m| m.fates.get(_b.device_str()).cloned())
                .is_some_and(|f| f.step == Step::Moving && f.moved > 0);
            if moving {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("beta's pull is moving");
    assert!(khor_node::post::pushes_in_flight() > 0, "control: the push is still running on alpha");
    assert!(timeout(Duration::from_secs(10), khor_node::link::cut_pushes(&ra)).await.unwrap(), "alpha's serve took the cut");
    let fate = timeout(Duration::from_secs(30), async {
        loop {
            if let Some(f) = message(&_b, &id).and_then(|m| m.fates.get(_b.device_str()).cloned())
                && f.step == Step::Failed
            {
                break f;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("beta records the cut");
    assert_eq!(fate.why, khor_catalog::msg::push_cut_suspended(), "the cut says why, in the catalog's words");
    assert!(!rb.join("khor-drop").join("video.mov").exists(), "nothing landed half");
    for s in serves {
        s.abort();
    }
}
