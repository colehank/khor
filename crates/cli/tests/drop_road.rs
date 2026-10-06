//! **A file dropped onto a terminal, without the terminal** (item 60,
//! `khor_node::drops`): scratch nodes on loopback, each with its own
//! home, so every drop directory here is `<scratch home>/khor-drop` and
//! never the person's own.
//!
//! The four roads: this machine's file onto another machine (the other
//! pulls it), another machine's file onto this one, a file already on
//! the session's machine (nothing moves), and a third machine ordering a
//! file from one machine onto another (the destination pulls from the
//! source directly). Then the refusals: a name already taken, a file
//! over the cap, a file that is not there — none of which leaves a part
//! file behind.

mod scratch;

use std::path::Path;
use std::time::Duration;

use khor_node::drops::{CAP, DropState};
use scratch::*;
use tokio::time::timeout;

/// Every name in a drop directory, part files included.
fn names(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .map(|rd| rd.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect())
        .unwrap_or_default();
    v.sort();
    v
}

async fn dropped(
    n: &khor_node::Node,
    from: &str,
    path: &str,
    to: &str,
) -> (Result<String, String>, Vec<DropState>) {
    let mut seen = Vec::new();
    let out = timeout(
        Duration::from_secs(120),
        n.drop_file(from, path, to, |s| seen.push(s.clone())),
    )
    .await
    .expect("a drop must not hang");
    (out, seen)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
async fn a_dropped_file_lands_where_the_sessions_machine_reads_it() {
    let p = pair("drop").await;
    let (alpha_drop, beta_drop) = (p.ra.0.join("khor-drop"), p.rb.0.join("khor-drop"));
    let payload: Vec<u8> = (0..(3u32 << 20) + 17).map(|i| (i % 253) as u8).collect();

    // This machine's file onto another: alpha pulls it off beta.
    let mine = p.rb.0.join("notes with space.bin");
    std::fs::write(&mine, &payload).unwrap();
    let mine = mine.display().to_string();
    let (out, seen) = dropped(&p.beta, "", &mine, "alpha").await;
    let dest = out.expect("beta's file lands on alpha");
    assert_eq!(Path::new(&dest), alpha_drop.join("notes with space.bin"));
    assert_eq!(std::fs::read(&dest).unwrap(), payload, "byte for byte");
    let last = seen.last().expect("progress was read");
    assert_eq!((last.moved, last.total, last.done), (payload.len() as u64, payload.len() as u64, true));
    assert!(
        seen.windows(2).all(|w| w[0].moved <= w[1].moved),
        "progress only grows: {seen:?}"
    );

    // The same name again: suffixed, the first one untouched.
    let (out, _) = dropped(&p.beta, "", &mine, "alpha").await;
    assert_eq!(Path::new(&out.unwrap()), alpha_drop.join("notes with space-2.bin"));
    assert_eq!(names(&alpha_drop), ["notes with space-2.bin", "notes with space.bin"]);

    // Another machine's file onto this one: beta pulls it off alpha.
    let theirs = p.ra.0.join("far.txt");
    std::fs::write(&theirs, b"from alpha\n").unwrap();
    let (out, _) = dropped(&p.beta, "alpha", &theirs.display().to_string(), "").await;
    let dest = out.expect("alpha's file lands on beta");
    assert_eq!(Path::new(&dest), beta_drop.join("far.txt"));
    assert_eq!(std::fs::read(&dest).unwrap(), b"from alpha\n");

    // Already on the session's machine: nothing moves, either way round.
    let there = theirs.display().to_string();
    let (out, seen) = dropped(&p.beta, "alpha", &there, "alpha").await;
    assert_eq!(out.unwrap(), there);
    assert_eq!(seen.len(), 1);
    let (out, _) = dropped(&p.beta, "", &mine, "").await;
    assert_eq!(out.unwrap(), mine);
    assert_eq!(names(&alpha_drop).len(), 2, "no copy made on alpha");
    assert_eq!(names(&beta_drop), ["far.txt"], "no copy made on beta");

    // The CLI verb, the same road: prints the path over there.
    let out = khor(&p.rb.0)
        .args(["drop", "--on", "alpha"])
        .arg(&mine)
        .output()
        .unwrap();
    assert!(out.status.success(), "khor drop: {}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        alpha_drop.join("notes with space-3.bin").display().to_string()
    );

    // Over the cap: refused from the size the first slice reports, with
    // the cap in the sentence, and nothing left on alpha.
    let big = p.rb.0.join("big.bin");
    std::fs::File::create(&big).unwrap().set_len(CAP + 1).unwrap();
    let (out, _) = dropped(&p.beta, "", &big.display().to_string(), "alpha").await;
    let why = out.expect_err("over the cap is refused");
    assert!(why.contains(&(CAP >> 20).to_string()), "the cap is said: {why}");
    // Not there: refused, and nothing left either.
    let (out, _) = dropped(&p.beta, "", &p.rb.0.join("absent.txt").display().to_string(), "alpha").await;
    assert!(out.is_err(), "a file that is not there is refused");
    assert_eq!(
        names(&alpha_drop),
        ["notes with space-2.bin", "notes with space-3.bin", "notes with space.bin"],
        "no part file and no note left by a refused drop"
    );
}

/// **The asker only orders** (item 60): gamma drops a file of alpha's
/// onto beta, and beta pulls it from alpha directly.
#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
async fn a_third_machine_orders_and_the_destination_pulls_from_the_source() {
    let p = pair("drop3").await;
    let relays: serde_json::Value =
        serde_json::from_slice(&std::fs::read(p.ra.0.join(".khor/relays.json")).unwrap()).unwrap();
    let relay = relays["relays"][0]["url"].as_str().unwrap().to_owned();
    let rg = scratch("drop3-g");
    home_on(&rg.0, &relay);
    let _g = serve(&rg.0, "gamma");
    stood_up(&[&rg.0]).await;
    let gamma = khor_node::Node::open_as(rg.0.clone(), "gamma").unwrap();
    let ticket = p.alpha.invite().unwrap();
    timeout(Duration::from_secs(90), gamma.pair(&ticket))
        .await
        .expect("pairing must not hang")
        .expect("gamma joins through alpha");

    let src = p.ra.0.join("on-alpha.txt");
    std::fs::write(&src, b"alpha's bytes\n").unwrap();
    let src = src.display().to_string();
    // Beta hears of gamma on a round with alpha; until then it refuses
    // gamma as a stranger, which is the only refusal waited out here.
    let began = std::time::Instant::now();
    let dest = loop {
        let _ = timeout(Duration::from_secs(60), p.beta.sync_now()).await;
        match dropped(&gamma, "alpha", &src, "beta").await.0 {
            Ok(dest) => break dest,
            Err(why) if began.elapsed() < Duration::from_secs(90) => {
                eprintln!("[drop3] {:?}: {why}", began.elapsed());
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            Err(why) => panic!("gamma's order lands on beta: {why}"),
        }
    };
    assert_eq!(Path::new(&dest), p.rb.0.join("khor-drop").join("on-alpha.txt"));
    assert_eq!(std::fs::read(&dest).unwrap(), b"alpha's bytes\n");
    assert!(names(&rg.0.join("khor-drop")).is_empty(), "nothing landed on the asker");
}
