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
        .map(|rd| {
            rd.flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
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
    dropped_into(n, from, path, to, None).await
}

async fn dropped_into(
    n: &khor_node::Node,
    from: &str,
    path: &str,
    to: &str,
    into: Option<&str>,
) -> (Result<String, String>, Vec<DropState>) {
    let mut seen = Vec::new();
    let out = timeout(
        Duration::from_secs(120),
        n.drop_file(from, path, to, into, |s| seen.push(s.clone())),
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
    assert_eq!(
        (last.moved, last.total, last.done),
        (payload.len() as u64, payload.len() as u64, true)
    );
    assert!(
        seen.windows(2).all(|w| w[0].moved <= w[1].moved),
        "progress only grows: {seen:?}"
    );

    // The same name again: suffixed, the first one untouched.
    let (out, _) = dropped(&p.beta, "", &mine, "alpha").await;
    assert_eq!(
        Path::new(&out.unwrap()),
        alpha_drop.join("notes with space-2.bin")
    );
    assert_eq!(
        names(&alpha_drop),
        ["notes with space-2.bin", "notes with space.bin"]
    );

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
    assert!(
        out.status.success(),
        "khor drop: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        alpha_drop
            .join("notes with space-3.bin")
            .display()
            .to_string()
    );

    // Over the cap: refused from the size the first slice reports, with
    // the cap in the sentence, and nothing left on alpha.
    let big = p.rb.0.join("big.bin");
    std::fs::File::create(&big)
        .unwrap()
        .set_len(CAP + 1)
        .unwrap();
    let (out, _) = dropped(&p.beta, "", &big.display().to_string(), "alpha").await;
    let why = out.expect_err("over the cap is refused");
    assert!(
        why.contains(&(CAP >> 20).to_string()),
        "the cap is said: {why}"
    );
    // Not there: refused, and nothing left either.
    let (out, _) = dropped(
        &p.beta,
        "",
        &p.rb.0.join("absent.txt").display().to_string(),
        "alpha",
    )
    .await;
    assert!(out.is_err(), "a file that is not there is refused");
    assert_eq!(
        names(&alpha_drop),
        [
            "notes with space-2.bin",
            "notes with space-3.bin",
            "notes with space.bin"
        ],
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
    assert_eq!(
        Path::new(&dest),
        p.rb.0.join("khor-drop").join("on-alpha.txt")
    );
    assert_eq!(std::fs::read(&dest).unwrap(), b"alpha's bytes\n");
    assert!(
        names(&rg.0.join("khor-drop")).is_empty(),
        "nothing landed on the asker"
    );
}

/// **A file picked on a phone** (item 60 part 3): the phone's app stages
/// the picked bytes under its own store (`khor_gui_core::term::
/// term_drop_stage`, `<store>/.khor/uploads/drop/<dir>/<name>`) and drops
/// that path onto the session's machine, which pulls it from the phone
/// by `FetchPath` — so the node a phone runs (hosting nothing, its serve
/// in the app's own process, resumable) must answer it.
#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
async fn a_phone_s_picked_file_is_pulled_by_the_session_s_machine() {
    let p = pair("drop-phone").await;
    let relays: serde_json::Value =
        serde_json::from_slice(&std::fs::read(p.ra.0.join(".khor/relays.json")).unwrap()).unwrap();
    let relay = relays["relays"][0]["url"].as_str().unwrap().to_owned();
    let rp = scratch("drop-phone-p");
    home_on(&rp.0, &relay);
    let served = khor_node::Node::open_hosting(rp.0.clone(), "phone", false).unwrap();
    let serve = tokio::spawn(async move { served.serve_resumable().await });
    stood_up(&[&rp.0]).await;
    let phone = khor_node::Node::open_hosting(rp.0.clone(), "phone", false).unwrap();
    let ticket = p.alpha.invite().unwrap();
    timeout(Duration::from_secs(90), phone.pair(&ticket))
        .await
        .expect("pairing must not hang")
        .expect("the phone joins through alpha");

    let staged = rp.0.join(".khor/uploads/drop/1-1-0/IMG 0001.png");
    std::fs::create_dir_all(staged.parent().unwrap()).unwrap();
    let photo: Vec<u8> = (0..(1u32 << 20) + 5).map(|i| (i % 251) as u8).collect();
    std::fs::write(&staged, &photo).unwrap();
    let (out, seen) = dropped(&phone, "", &staged.display().to_string(), "alpha").await;
    let dest = out.expect("the phone's file lands on alpha");
    assert_eq!(
        Path::new(&dest),
        p.ra.0.join("khor-drop").join("IMG 0001.png")
    );
    assert_eq!(std::fs::read(&dest).unwrap(), photo, "byte for byte");
    assert!(
        seen.last().is_some_and(|s| s.done),
        "progress was read to the end"
    );
    assert!(
        names(&rp.0.join("khor-drop")).is_empty(),
        "nothing landed on the phone"
    );
    serve.abort();
}

/// **「放到当前目录」** (item 60 part 5): the file lands in the directory
/// the asker names on the session's machine, in place of its drop
/// directory — pulled there from another machine, copied there from the
/// session's own machine, left as it is when it already sits there.
#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
async fn a_drop_into_the_shells_directory_lands_there() {
    let p = pair("drop-into").await;
    let (alpha_drop, beta_drop) = (p.ra.0.join("khor-drop"), p.rb.0.join("khor-drop"));
    let work = p.ra.0.join("work dir");
    std::fs::create_dir_all(&work).unwrap();
    let into = work.display().to_string();

    // beta's file into alpha's directory: alpha pulls it there.
    let mine = p.rb.0.join("notes.txt");
    std::fs::write(&mine, b"from beta\n").unwrap();
    let mine = mine.display().to_string();
    let (out, _) = dropped_into(&p.beta, "", &mine, "alpha", Some(&into)).await;
    assert_eq!(
        Path::new(&out.expect("lands in the named directory")),
        work.join("notes.txt")
    );
    assert_eq!(
        std::fs::read(work.join("notes.txt")).unwrap(),
        b"from beta\n"
    );
    let (out, _) = dropped_into(&p.beta, "", &mine, "alpha", Some(&into)).await;
    assert_eq!(
        Path::new(&out.unwrap()),
        work.join("notes-2.txt"),
        "a taken name is suffixed"
    );

    // alpha's own file, ordered by beta into alpha's directory: copied.
    let theirs = p.ra.0.join("far.txt");
    std::fs::write(&theirs, b"from alpha\n").unwrap();
    let (out, seen) = dropped_into(
        &p.beta,
        "alpha",
        &theirs.display().to_string(),
        "alpha",
        Some(&into),
    )
    .await;
    assert_eq!(
        Path::new(&out.expect("copied on alpha")),
        work.join("far.txt")
    );
    assert_eq!(
        std::fs::read(work.join("far.txt")).unwrap(),
        b"from alpha\n"
    );
    assert!(theirs.exists(), "the original stays");
    assert_eq!(seen.last().map(|s| s.done), Some(true));
    // A file already in that directory: nothing moves.
    let in_place = work.join("far.txt").display().to_string();
    let (out, _) = dropped_into(&p.beta, "alpha", &in_place, "alpha", Some(&into)).await;
    assert_eq!(out.unwrap(), in_place);

    // This machine's own file into its own directory: copied here.
    let here_dir = p.rb.0.join("here");
    std::fs::create_dir_all(&here_dir).unwrap();
    let (out, _) = dropped_into(
        &p.beta,
        "",
        &mine,
        "",
        Some(&here_dir.display().to_string()),
    )
    .await;
    assert_eq!(Path::new(&out.unwrap()), here_dir.join("notes.txt"));

    // A directory that is not there, or not absolute: refused, nothing made.
    let gone = p.ra.0.join("gone").display().to_string();
    let (out, _) = dropped_into(&p.beta, "", &mine, "alpha", Some(&gone)).await;
    assert!(out.expect_err("no such directory").contains("gone"));
    assert!(!p.ra.0.join("gone").exists(), "never made for the asker");
    let (out, _) = dropped_into(&p.beta, "", &mine, "alpha", Some("work dir")).await;
    assert!(out.is_err(), "a relative directory is refused");

    // The CLI verb's --into.
    let out = khor(&p.rb.0)
        .args(["drop", "--on", "alpha", "--into"])
        .arg(&into)
        .arg(&mine)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "khor drop --into: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        work.join("notes-3.txt").display().to_string()
    );

    assert_eq!(
        names(&work),
        ["far.txt", "notes-2.txt", "notes-3.txt", "notes.txt"],
        "no part file left"
    );
    assert!(
        names(&alpha_drop).is_empty() && names(&beta_drop).is_empty(),
        "nothing in the drop directories"
    );
}

/// **A drop cancelled mid-pull** (item 71, the pill's ×): the pull
/// stops at its next slice wherever it runs, its part file goes, nothing
/// lands, and the asker answers the cancel at once. Behind a slow road,
/// so the pull is still going when the cancel comes.
///
/// - **Far**: beta's file onto alpha — alpha pulls, beta orders, and the
///   cancel is an order to alpha (`Request::DropStop`).
/// - **Here**: alpha's file onto beta — beta's own resident pulls, and
///   the cancel is told to it by token (`ipc::Op::DropStop`).
///
/// Then the same file dropped again lands whole: a cancel ends one job.
#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
async fn a_cancelled_drop_stops_its_pull_and_leaves_nothing() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    let (p, _road) = pair_on(
        "drop-stop",
        Some(Impair {
            rtt: Duration::from_millis(200),
            loss_pct: 0,
            stall: Duration::ZERO,
        }),
    )
    .await;
    let (alpha_drop, beta_drop) = (p.ra.0.join("khor-drop"), p.rb.0.join("khor-drop"));
    let payload: Vec<u8> = (0..(24u32 << 20)).map(|i| (i % 249) as u8).collect();
    let on_beta = p.rb.0.join("big on beta.bin");
    std::fs::write(&on_beta, &payload).unwrap();
    let on_alpha = p.ra.0.join("big on alpha.bin");
    std::fs::write(&on_alpha, &payload).unwrap();

    /// A drop cancelled once some of it has landed: what it answered, and
    /// how far it had got when the cancel came.
    async fn cancelled(
        n: &khor_node::Node,
        from: &str,
        path: &str,
        to: &str,
    ) -> (Result<String, String>, u64) {
        let stop = Arc::new(AtomicBool::new(false));
        let mut at = 0;
        let flag = stop.clone();
        let out = timeout(
            Duration::from_secs(120),
            n.drop_file_until(
                from,
                path,
                to,
                None,
                |s| {
                    if s.moved > 0 && !stop.load(Ordering::Relaxed) {
                        at = s.moved;
                        stop.store(true, Ordering::Relaxed);
                    }
                },
                move || flag.load(Ordering::Relaxed),
            ),
        )
        .await
        .expect("a cancelled drop must not hang");
        (out, at)
    }
    /// The drop directory once nothing is left in it, or what is.
    async fn emptied(dir: &Path) -> Vec<String> {
        let began = std::time::Instant::now();
        while !names(dir).is_empty() && began.elapsed() < Duration::from_secs(20) {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        names(dir)
    }
    let total = payload.len() as u64;

    // Far.
    let (out, at) = cancelled(&p.beta, "", &on_beta.display().to_string(), "alpha").await;
    assert_eq!(
        out,
        Err(khor_node::drops::cancelled()),
        "the asker answers the cancel"
    );
    assert!(at > 0 && at < total, "cancelled mid-pull: {at} of {total}");
    assert_eq!(
        emptied(&alpha_drop).await,
        Vec::<String>::new(),
        "alpha's pull stopped: no part file, no file"
    );
    tokio::time::sleep(Duration::from_secs(3)).await;
    assert_eq!(
        names(&alpha_drop),
        Vec::<String>::new(),
        "and nothing lands after"
    );

    // Here.
    let (out, at) = cancelled(&p.beta, "alpha", &on_alpha.display().to_string(), "").await;
    assert_eq!(out, Err(khor_node::drops::cancelled()));
    assert!(at > 0 && at < total, "cancelled mid-pull: {at} of {total}");
    assert_eq!(
        emptied(&beta_drop).await,
        Vec::<String>::new(),
        "beta's own pull stopped: no part file, no file"
    );
    tokio::time::sleep(Duration::from_secs(3)).await;
    assert_eq!(
        names(&beta_drop),
        Vec::<String>::new(),
        "and nothing lands after"
    );

    // Dropped again, uncancelled: whole, under its own name.
    let (out, _) = dropped(&p.beta, "", &on_beta.display().to_string(), "alpha").await;
    let dest = out.expect("a later drop of the same file lands");
    assert_eq!(Path::new(&dest), alpha_drop.join("big on beta.bin"));
    assert_eq!(std::fs::read(&dest).unwrap(), payload, "byte for byte");
}
