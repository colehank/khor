//! Against a hand-written tree, not this machine's own.
//!
//! Every number below is one somebody wrote into
//! `tests/fixtures/usage`, which is what makes them assertions rather
//! than observations — and the fixture holds no working directory, no
//! session name and no message content, because a committed sample of a
//! transcript would be a committed sample of somebody's work.
//!
//! The reading of the real thing lives in `tests/real_disk.rs`, ignored
//! by default, and it is what catches the day this fixture becomes
//! fiction.

use std::path::PathBuf;

use jiff::tz::TimeZone;
use khor_core::{Tokens, Usage};

use super::*;

/// East of UTC by eight hours, spelled as a fixed offset rather than a
/// zone name: naming a zone would make this test depend on the machine
/// having a copy of the time zone database, and nothing here needs
/// daylight saving to be modelled — only that the day is cut somewhere
/// other than at UTC midnight.
fn plus_eight() -> TimeZone {
    TimeZone::fixed(jiff::tz::offset(8))
}

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/usage")
}

fn claude_only() -> Tally {
    claude::Claude::at(fixture().join("claude/.claude")).tally(&plus_eight())
}

fn codex_only() -> Tally {
    codex::Codex::at(fixture().join("codex/.codex")).tally(&plus_eight())
}

fn tokens(input: u64, cached_input: u64, cache_write: u64, output: u64) -> Tokens {
    Tokens { input, cached_input, cache_write, output }
}

fn day(usage: &Usage, day: &str, category: &str) -> Option<Tokens> {
    usage
        .days
        .iter()
        .find(|d| d.day == day && d.category == category)
        .map(|d| d.tokens)
}

/// A whole home, both vendors, one answer.
///
/// `tag` names this test's own directory: the suite runs in parallel, and
/// two tests sharing one temp path would race on tearing it down and
/// rebuilding it — which is a red that says nothing about usage.
fn both(tag: &str) -> Usage {
    Meters::at(&fixture_home(tag)).tally_in(&plus_eight())
}

/// One directory wearing both vendors' trees, so that the two are read in
/// a single pass the way a real home is.
fn fixture_home(tag: &str) -> PathBuf {
    let home = std::env::temp_dir().join(format!("khor-usage-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(fixture().join("claude/.claude"), home.join(".claude"))
            .unwrap();
        std::os::unix::fs::symlink(fixture().join("codex/.codex"), home.join(".codex")).unwrap();
    }
    home
}

/// **One message is billed once, and the reading kept is a whole one.**
///
/// The fixture writes `msg_A` three times with the output growing 5 → 50
/// → 500, which is what claude does as the blocks of one answer arrive.
/// Summing the lines would bill 555 for an answer that produced 500.
///
/// `msg_E` is the second half of the rule, and it is the half a
/// reasonable implementation gets wrong: its final writing **moves**
/// 100 tokens from cache-write to cache-read (300/0 becoming 200/100).
/// Taking the maximum of each field separately keeps the 300 *and* adds
/// the 100 — 400 tokens of input for an answer that read 300. So the
/// reading with the largest output is taken entire.
#[test]
fn a_message_written_several_times_is_billed_once_and_kept_whole() {
    let t = claude_only();
    let spent = t.by_day.get("2026-08-17").copied().expect("the fixture spends on this day");
    assert_eq!(
        spent,
        tokens(26, 277, 228, 563),
        "A(10/100/20/500) B(3/7/1/9) E(1/100/200/30) G(5/0/0/11) D(7/70/7/13)"
    );
    // Spelled out so the two failure modes above are nameable from the
    // message alone rather than only from the totals.
    assert_ne!(spent.output, 563 + 5 + 50, "the partial writings of msg_A were billed too");
    assert_ne!(spent.cache_write, 228 + 100, "msg_E's fields were maximised one by one");
}

/// **A resumed session copies its history, and khor must not bill it
/// twice.** `msg_A` appears in `sess-1.jsonl` and again in
/// `sess-2.jsonl`, exactly as a real resume writes it — so the
/// deduplication has to span files, not just lines within one.
///
/// The control is in the same file: `sess-2.jsonl` also holds `msg_G`,
/// which nothing else has, and it *is* billed. An implementation that
/// skipped second files entirely would satisfy the first assertion and
/// fail this one.
#[test]
fn a_resumed_session_does_not_bill_its_history_twice() {
    let t = claude_only();
    let spent = t.by_day.get("2026-08-17").copied().unwrap();
    assert_eq!(spent.output, 563, "msg_A's 500 counted once across the two files");
    assert!(spent.output >= 11, "…and msg_G, which only the second file holds, still counted");
    assert_eq!(spent.input, 26, "10 + 3 + 1 + 5 + 7 — msg_A's 10 appearing once");
}

/// **A subagent's tokens are this machine's tokens.** The ledger recorded
/// the transcripts as `projects/<project>/<sid>.jsonl`; the fixture puts
/// `msg_D` under `<sid>/subagents/`, which is where 988 of this machine's
/// 1069 transcripts actually live.
///
/// Asserted through the file walk as well as through the total, because
/// the total alone would also be satisfied by a walk that found the file
/// for some other reason.
#[test]
fn a_subagents_transcript_is_found_and_billed() {
    let nested = jsonl_under(&fixture().join("claude/.claude/projects"));
    assert_eq!(nested.len(), 3, "two session files and one subagent's: {nested:?}");
    assert!(
        nested.iter().any(|p| p.to_string_lossy().contains("subagents")),
        "the walk must go below the session file's own level: {nested:?}"
    );
    let spent = claude_only().by_day.get("2026-08-17").copied().unwrap();
    assert_eq!(spent.cached_input, 277, "…and msg_D's 70 is inside that");
}

/// **The day is the machine's, not UTC's.**
///
/// One instant, 16:30 UTC, read under two zones in one test — which is
/// the control the assertion needs: under UTC it is still the 17th, and
/// eight hours east it is already the 18th. A tally that ignored the zone
/// would give the same answer twice, and the first assertion alone could
/// not tell.
///
/// Both vendors are checked, because the two read their timestamps
/// through different fields and only one shared function.
#[test]
fn a_day_is_cut_where_the_machine_stands_not_at_utc_midnight() {
    for meter in [fixture().join("claude/.claude"), fixture().join("codex/.codex")] {
        let east: Tally = if meter.ends_with(".claude") {
            claude::Claude::at(meter.clone()).tally(&plus_eight())
        } else {
            codex::Codex::at(meter.clone()).tally(&plus_eight())
        };
        let utc: Tally = if meter.ends_with(".claude") {
            claude::Claude::at(meter.clone()).tally(&TimeZone::UTC)
        } else {
            codex::Codex::at(meter.clone()).tally(&TimeZone::UTC)
        };
        assert!(
            east.by_day.contains_key("2026-08-18"),
            "{}: 16:30Z is past midnight eight hours east — days: {:?}",
            meter.display(),
            east.by_day.keys().collect::<Vec<_>>()
        );
        assert!(
            !utc.by_day.contains_key("2026-08-18"),
            "{}: …and in UTC that same instant is still the 17th, which is what makes \
             the assertion above about the zone: {:?}",
            meter.display(),
            utc.by_day.keys().collect::<Vec<_>>()
        );
        let east_total: u64 = east.by_day.values().map(|t| t.output).sum();
        let utc_total: u64 = utc.by_day.values().map(|t| t.output).sum();
        assert_eq!(east_total, utc_total, "{}: the zone moves a day, never a token", meter.display());
    }
}

/// **Codex reports a running total beside each turn, and khor sums the
/// turns.**
///
/// The fixture's second event carries a total of 3150 and a turn of 2090.
/// Summing totals would bill 1060 + 3150 + 3670; summing turns bills
/// 1060 + 2090 + 520. The numbers below are the second, and the third
/// assertion names the first so that an implementation which read the
/// wrong field fails with the reason on the screen.
#[test]
fn a_running_total_is_not_a_sum_of_turns() {
    let t = codex_only();
    assert_eq!(
        t.by_day.get("2026-08-17").copied(),
        Some(tokens(600, 2400, 0, 150)),
        "two turns: (1000-800, 800, -, 60) and (2000-1600, 1600, -, 90)"
    );
    assert_eq!(
        t.by_day.get("2026-08-18").copied(),
        Some(tokens(400, 100, 33, 20)),
        "the turn past local midnight, and the cache-write only newer codex writes"
    );
    let out_17 = t.by_day["2026-08-17"].output;
    assert_ne!(out_17, 60 + 150, "the running totals were summed instead of the turns");
}

/// **Codex writes each event twice; two identical readings in a row are
/// one turn.**
///
/// Without the rule the fixture's first day doubles. The control is that
/// the two turns on that day are *not* merged with each other — they bill
/// different numbers, so an implementation that dropped every repeat
/// regardless would lose one of them.
#[test]
fn a_reading_written_twice_in_a_row_is_one_turn() {
    let t = codex_only();
    let spent = t.by_day["2026-08-17"];
    assert_eq!(spent.output, 150, "60 + 90, each written twice and counted once");
    assert_ne!(spent.output, 300, "every line was counted");
    assert_ne!(spent.output, 60, "the second turn was swallowed as a repeat of the first");
}

/// **What it cannot read, it counts — and a half-written line is not
/// that.**
///
/// The fixture holds four unreadable records: a garbled line in the
/// middle of a transcript, an assistant message that bills tokens without
/// saying which message it is, one whose timestamp is not a time, and a
/// codex `token_count` with no turn in it. It also ends `sess-1.jsonl`
/// with `{"type":"assis` and **no newline** — an agent mid-write — which
/// must not count, or the drift alarm goes off every time somebody is
/// working.
///
/// The readable rows are asserted first: an implementation that gave up
/// on the file at the garbled line would report the same count while
/// losing everything after it.
#[test]
fn what_it_cannot_read_it_counts_and_an_unfinished_line_is_not_that() {
    let usage = both("unreadable");
    assert_eq!(usage.unreadable, 4, "three from claude's tree, one from codex's");
    assert_eq!(
        day(&usage, "2026-08-17", "claude").map(|t| t.output),
        Some(563),
        "the records after the garbled line were still read"
    );
    assert_eq!(
        day(&usage, "2026-08-18", "claude"),
        Some(tokens(2, 0, 0, 4)),
        "msg_F sits after the garbled line and before the unfinished one"
    );
}

/// **A home where no agent has ever run is quiet, not blind.**
///
/// The pair this asserts is the whole reason `unreadable` exists: no rows
/// and no count means nothing was spent, no rows and a count means khor
/// has gone blind. They must not read alike, and here is the first of the
/// two.
#[test]
fn an_empty_home_spends_nothing_and_reports_nothing_unreadable() {
    let empty = std::env::temp_dir().join(format!("khor-usage-empty-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&empty);
    std::fs::create_dir_all(&empty).unwrap();
    let usage = Meters::at(&empty).tally_in(&plus_eight());
    assert_eq!(usage, Usage::default());
    assert!(usage.days.is_empty() && usage.unreadable == 0);
    let _ = std::fs::remove_dir_all(&empty);
}

/// **Each row carries the name of the meter that read it.**
///
/// Both vendors are read from one home in one pass, so the two
/// implementations that would otherwise pass cannot: stamping a constant,
/// and stamping the first meter's name on everything. That is the
/// "everything in one bucket" trap, which a property assertion ("every
/// row has a category") is blind to — so this enumerates.
#[cfg(unix)]
#[test]
fn each_row_carries_the_name_of_the_meter_that_read_it() {
    let usage = both("categories");
    let listed: Vec<(&str, &str, u64)> = usage
        .days
        .iter()
        .map(|d| (d.day.as_str(), d.category.as_str(), d.tokens.output))
        .collect();
    assert_eq!(
        listed,
        vec![
            ("2026-08-17", "claude", 563),
            ("2026-08-17", "codex", 150),
            ("2026-08-18", "claude", 4),
            ("2026-08-18", "codex", 20),
        ],
        "oldest day first, then by vendor — and every vendor's own numbers under its own name"
    );
}

/// The two vendors' names are the ones the session list already uses.
///
/// Not cosmetic: a row in the session list and a row in the spending list
/// are grouped by the same string, so a second spelling would put one
/// agent in two categories on two screens and nothing would report an
/// error.
#[test]
fn the_categories_are_the_ones_the_session_list_already_spells() {
    assert_eq!(claude::VENDOR, crate::adaptor::claude::VENDOR);
    assert_eq!(codex::VENDOR, crate::adaptor::codex::VENDOR);
    assert_eq!((claude::VENDOR, codex::VENDOR), ("claude", "codex"));
}

/// A meter that reads nothing is the closed default, so no test of this
/// module can touch the machine it runs on unless it says so.
#[test]
fn the_empty_registry_reads_nothing() {
    assert_eq!(Meters::empty().tally_in(&plus_eight()), Usage::default());
}


/// **The cache is kept until the files move, and dropped the moment they
/// do.**
///
/// Both halves in one test, because either alone passes for the wrong
/// reason: a cache that never expires satisfies "the second ask read
/// nothing", and no cache at all satisfies "an appended turn shows up".
///
/// "Read nothing" is asserted against this registry's own pass counter
/// rather than against a stopwatch — a timing assertion on a shared
/// machine measures how busy the machine is (docs/handoff 固定 sleep 的
/// 测试，测的是「机器闲不闲」).
#[test]
fn a_cached_answer_survives_until_the_tree_moves_and_not_after() {
    let home = std::env::temp_dir().join(format!("khor-usage-cache-{}", std::process::id()));
    let dir = home.join(".claude/projects/p");
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&dir).unwrap();
    let line = |id: &str, out: u64| {
        format!(
            r#"{{"type":"assistant","timestamp":"2026-08-17T06:00:00Z","message":{{"id":"{id}","usage":{{"input_tokens":1,"cache_read_input_tokens":0,"cache_creation_input_tokens":0,"output_tokens":{out}}}}}}}"#
        )
    };
    let file = dir.join("s.jsonl");
    std::fs::write(&file, format!("{}\n", line("m1", 100))).unwrap();

    let meters = Meters::at(&home);
    let out = |u: &Usage| u.days.iter().map(|d| d.tokens.output).sum::<u64>();

    let first = meters.tally();
    assert_eq!(out(&first), 100, "the one message in the tree");
    let after_first = meters.passes();

    let again = meters.tally();
    assert_eq!(again, first);
    assert_eq!(meters.passes(), after_first, "the second ask must not have re-read anything");

    // A turn is appended, the way an agent appends one.
    std::fs::write(&file, format!("{}\n{}\n", line("m1", 100), line("m2", 7))).unwrap();
    let third = meters.tally();
    assert_eq!(out(&third), 107, "the appended turn is in the answer");
    assert!(meters.passes() > after_first, "…and it got there by reading the file again");

    let _ = std::fs::remove_dir_all(&home);
}
