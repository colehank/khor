//! Manual checks against this machine's actual agent directories.
//!
//! `#[ignore]` because they only mean anything where a real claude or
//! codex has run, and because they are read-only observations of files
//! another program owns — never part of a green build, always available
//! with `cargo test -p khor-node --test real_disk -- --ignored`.
//!
//! Their job is the one thing fixtures structurally cannot do: catch the
//! day the fixtures become fiction. A recorded sample stays green forever
//! by construction, so the vendor changing its file layout is invisible
//! from inside the test suite. These read the real thing and fail when it
//! stops matching.

use std::collections::BTreeSet;
use std::path::PathBuf;

use khor_node::adaptor::{claude::Claude, id_for, tmux::Tmux, Adaptor, Discovery, Procs};

fn real_home() -> PathBuf {
    std::env::home_dir().expect("a home directory")
}

fn fixture(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vendors").join(rel)
}

fn keys_of(path: &PathBuf) -> BTreeSet<String> {
    let text = std::fs::read_to_string(path).expect("readable");
    let v: serde_json::Value = serde_json::from_str(&text).expect("json");
    v.as_object().expect("an object").keys().cloned().collect()
}

/// The fields khor reads out of a claude status file. If claude renames
/// one of these, discovery goes quiet — so this is the list worth
/// pinning against reality rather than against our own recording.
const DEPENDED_ON: [&str; 5] = ["pid", "sessionId", "startedAt", "status", "statusUpdatedAt"];

/// Every field khor depends on is present in every real status file on
/// this machine, and the committed fixture spells them the same way.
///
/// The fixture half matters as much as the real half: a fixture that
/// drifted into a private dialect would keep the unit tests green while
/// describing a file format that no longer exists anywhere.
#[test]
#[ignore]
fn the_fixture_and_this_machines_claude_agree_on_the_schema() {
    let dir = real_home().join(".claude/sessions");
    let real: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("no claude sessions dir at {}: {e}", dir.display()))
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("json"))
        .collect();
    assert!(
        !real.is_empty(),
        "no status files on this machine — the check would pass vacuously"
    );

    for path in &real {
        let keys = keys_of(path);
        for field in DEPENDED_ON {
            assert!(
                keys.contains(field),
                "{} has no {field:?}; khor reads it. Keys present: {keys:?}",
                path.display()
            );
        }
    }

    let fixture_keys = keys_of(&fixture("claude/.claude/sessions/4002.json"));
    for field in DEPENDED_ON {
        assert!(
            fixture_keys.contains(field),
            "the fixture dropped {field:?}; it no longer stands in for a real file"
        );
    }
    // Everything the fixture claims must actually occur out there, or the
    // fixture is describing a format of its own invention.
    let real_keys: BTreeSet<String> =
        real.iter().flat_map(|p| keys_of(p)).collect();
    let invented: Vec<&String> = fixture_keys.difference(&real_keys).collect();
    assert!(invented.is_empty(), "fields the fixture invented: {invented:?}");
}

/// Nothing running on this machine is a session khor cannot read.
///
/// This is the schema-drift alarm proper: a claude that renames a status
/// word, or reshapes the file, shows up here as a non-zero count while
/// every unit test stays green.
#[test]
#[ignore]
fn khor_can_read_every_live_claude_on_this_machine() {
    let claude = Claude::at(real_home().join(".claude"));
    let procs = Procs::snapshot();
    let sweep = claude.sweep(&procs);
    println!(
        "live claude sessions read: {}, unreadable: {}",
        sweep.rows.len(),
        sweep.unmapped
    );
    for row in &sweep.rows {
        // Titles only — the row's own contents are the user's business.
        println!("  {} {:?}", id_for(&row.vendor_session_id).0, row.word);
    }
    assert!(
        !sweep.rows.is_empty(),
        "no live claude found; run one before trusting the count below"
    );
    assert_eq!(
        sweep.unmapped, 0,
        "a live claude session khor could not read — the vendor's format moved"
    );
}

/// khor can read this machine's own tmux, and everything it reports
/// either becomes a row or is already one.
///
/// **Read-only, and deliberately so**: this looks at the user's default
/// server, where their real work is, so it runs `list-panes` and nothing
/// else. It never creates or kills a session — the automatic tests do
/// that, on servers of their own (`adaptor::tmux`'s `Server`).
///
/// What it is for is the half a closed test cannot reach: the duplicate
/// rule against real agents. Fixtures can prove khor drops a session
/// holding a process khor was told about; only this can prove it drops
/// the ones actually running here.
#[test]
#[ignore]
fn khor_can_read_this_machines_tmux_and_lists_nothing_twice() {
    let procs = Procs::snapshot();
    let listing = Tmux::default_server().sweep(&procs);
    println!(
        "tmux sessions seen: {}, unreadable: {}, processes: {}",
        listing.rows.len(),
        listing.unmapped,
        procs.len()
    );
    assert!(
        !listing.rows.is_empty(),
        "no tmux sessions on this machine — every assertion below would pass vacuously"
    );
    assert_eq!(
        listing.unmapped, 0,
        "a tmux session khor could not describe — either tmux's output moved or a pane is gone"
    );

    // Now the same question the session list asks: which of these hold
    // something a vendor adaptor already named?
    let found = Discovery::at(&real_home())
        .with_tmux(Tmux::default_server())
        .with_procs(procs.clone())
        .sweep();
    let claimed: Vec<u32> =
        found.rows.iter().flat_map(|f| f.sighting.pids.iter().copied()).collect();
    println!(
        "agent rows: {}, live processes they account for: {}",
        found.rows.len(),
        claimed.len()
    );
    assert!(
        !claimed.is_empty(),
        "no running agent on this machine — the drop below would prove nothing"
    );

    let mut kept = 0;
    let mut dropped = 0;
    for m in &found.multiplexed {
        let inside: Vec<u32> =
            m.holds.iter().copied().filter(|p| claimed.contains(p)).collect();
        if inside.is_empty() {
            kept += 1;
            println!("  row   {} {:?}", m.found.id().0, m.found.sighting.word);
        } else {
            dropped += 1;
            println!("  drop  {} — holds {inside:?}, already listed", m.found.id().0);
        }
    }
    assert!(
        dropped > 0,
        "not one tmux session on this machine holds a listed agent, yet {} agents are \
         running — the join is not working, or they are all outside tmux",
        claimed.len()
    );
    println!("kept {kept}, dropped {dropped}");
}

/// khor can read every spending record on this machine, and what that
/// costs.
///
/// Two things at once because they are one walk. The count of unreadable
/// records is the drift alarm — a vendor reshaping its files shows up
/// here while every fixture test stays green — and the timing is the
/// number that decides whether this may sit on a polling path (it may
/// not; see the figure it prints).
///
/// Read-only throughout: it opens the user's real transcripts and adds
/// up four numbers per record. **Nothing it prints is content** — days,
/// vendor names and totals only, because a test that dumped a transcript
/// would be a test that copied somebody's work into a build log.
///
/// **The days are cut where the machine stands, and that is a correction.**
/// This used to pass UTC, reasoning that a fixed boundary would line up
/// with a checker written in another language. Held against a real one it
/// did the opposite: `tokscale` pins a bucket timezone on its first scan
/// and then **refuses to move** — the attempt answers `cannot change
/// scanner.bucketTimezone from Asia/Singapore to UTC: historical submitted
/// day rows are monotonic` — so the side that has to give is this one.
/// Cutting where the machine stands also prints the days khor's own
/// screens show (`khor_core::UsageDay`), so what is compared is what the
/// user sees rather than a third thing neither tool displays.
///
/// The totals below are unaffected either way: a record is counted once
/// whichever day it lands on, which is what makes them the right thing to
/// compare first when two tools disagree.
///
/// **Every row is printed, not just the last few**, and with the raw
/// integers rather than anything rounded — the point of this output is to
/// be diffed against another tool's, and a comparison that can only see
/// four days cannot tell a vendor's arithmetic changing from a quiet
/// week. What it costs is a page of output on a machine with a year of
/// history, which is cheap for the one thing it buys: the numbers khor
/// would put on the screen, in a form somebody can check.
#[test]
#[ignore]
fn khor_can_read_every_spending_record_on_this_machine() {
    use std::time::Instant;

    let home = real_home();
    let zone = jiff::tz::TimeZone::system();
    let t = Instant::now();
    let usage = khor_node::usage::Meters::at(&home).tally_in(&zone);
    let elapsed = t.elapsed();

    let mut total = khor_core::Tokens::default();
    for d in &usage.days {
        total.add(d.tokens);
    }
    // Named where the system can say so, and otherwise by the offset in
    // force right now — which is the thing that actually decides where a
    // day is cut, and the thing the other tool has to be holding too.
    let now = jiff::Timestamp::now().to_zoned(zone.clone());
    println!(
        "days cut in {} (UTC{} today)",
        zone.iana_name().unwrap_or("the system zone"),
        now.offset()
    );
    println!(
        "days: {}, unreadable: {}, one cold pass: {:.2}s",
        usage.days.len(),
        usage.unreadable,
        elapsed.as_secs_f64()
    );
    println!(
        "totals  input {}  cached {}  cache-write {}  output {}",
        total.input, total.cached_input, total.cache_write, total.output
    );
    // Tab-separated, one row per day per vendor, oldest first — the order
    // the library answers in, so that two runs (or two tools) diff
    // line-for-line without either side sorting first.
    println!("day\tcategory\tinput\tcached\tcache_write\toutput");
    for d in &usage.days {
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}",
            d.day, d.category, d.tokens.input, d.tokens.cached_input, d.tokens.cache_write,
            d.tokens.output
        );
    }
    // Per vendor as well, because a per-day diff that goes red everywhere
    // says nothing about which vendor's reading moved.
    let mut per_vendor: std::collections::BTreeMap<&str, khor_core::Tokens> =
        std::collections::BTreeMap::new();
    for d in &usage.days {
        per_vendor.entry(d.category.as_str()).or_default().add(d.tokens);
    }
    for (category, t) in &per_vendor {
        println!(
            "total\t{category}\t{}\t{}\t{}\t{}",
            t.input, t.cached_input, t.cache_write, t.output
        );
    }

    assert!(
        !usage.days.is_empty(),
        "no agent has ever run under {} — every assertion below would pass vacuously",
        home.display()
    );
    assert_eq!(
        usage.unreadable, 0,
        "a spending record khor could not read — a vendor's format moved"
    );
    // The one property that holds on every machine and would break on the
    // two mistakes that matter: summing the repeats of one claude message
    // inflates output, and summing codex's running total inflates input.
    // Neither can be checked against a constant here, so what is checked
    // is that the answer is not empty and not absurd.
    assert!(total.output > 0 && total.input > 0, "{total:?}");
}

/// **What one pass costs, and why the answer is not "cache it".**
///
/// Printed rather than asserted, for the reason `cost.rs` gives: a
/// threshold here is a test that fails on somebody else's laptop. The
/// figure this exists to keep honest is the ratio between a full pass and
/// the GUI's poll interval — if a pass ever becomes cheap enough to sit
/// on the poll, the caching in `khor_node::usage` stops being necessary,
/// and if it becomes far more expensive, the cache needs to become
/// incremental rather than whole.
#[test]
#[ignore]
fn one_spending_pass_is_timed_against_the_tree_it_walked() {
    use std::time::Instant;

    let home = real_home();
    let mut files = 0usize;
    let mut bytes = 0u64;
    let mut stack = vec![home.join(".claude/projects"), home.join(".codex/sessions")];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            match e.file_type() {
                Ok(t) if t.is_dir() => stack.push(p),
                Ok(t) if t.is_file() => {
                    if p.extension().and_then(|x| x.to_str()) == Some("jsonl") {
                        files += 1;
                        bytes += e.metadata().map(|m| m.len()).unwrap_or(0);
                    }
                }
                _ => {}
            }
        }
    }
    let zone = jiff::tz::TimeZone::UTC;
    let meters = khor_node::usage::Meters::at(&home);
    let took = |f: &dyn Fn()| {
        let t = Instant::now();
        f();
        t.elapsed().as_secs_f64()
    };
    // Three different questions, and the design rests on the gap between
    // them: reading every byte, reading only what was appended (nothing,
    // here) and folding again, and deciding from the directory entries
    // that there is nothing to fold.
    let cold = took(&|| {
        let _ = meters.tally_in(&zone);
    });
    let refold = took(&|| {
        let _ = meters.tally_in(&zone);
    });
    let _ = meters.tally();
    let kept = took(&|| {
        let _ = meters.tally();
    });
    println!(
        "{files} files, {:.1} MiB — cold {cold:.2}s, tail-only re-fold {refold:.2}s, \
         unchanged {kept:.3}s",
        bytes as f64 / (1024.0 * 1024.0),
    );
    assert!(files > 0, "nothing to time under {}", home.display());
    // Not a threshold on the machine — a threshold on the *shape* of the
    // answer, which holds anywhere: whatever a cold pass costs, not
    // paying it again is the whole design (`khor_node::usage`). A
    // hard-coded second count here would go red on somebody else's
    // laptop; this goes red if the incremental bookkeeping stops working.
    assert!(
        refold < cold,
        "re-reading nothing must cost less than reading everything: {cold:.2}s then {refold:.2}s"
    );
}
