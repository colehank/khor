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

use std::collections::HashMap;
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

/// A meter's records filed under days, which is what `Meters::tally_in`
/// does at the end of a pass — done here so that a single vendor can be
/// asserted about on its own.
fn by_day(t: &Tally, zone: &TimeZone) -> HashMap<String, Tokens> {
    let mut out: HashMap<String, Tokens> = HashMap::new();
    for k in &t.kept {
        out.entry(k.at.to_zoned(zone.clone()).date().to_string())
            .or_default()
            .add(k.tokens);
    }
    out
}

fn claude_meter() -> claude::Claude {
    claude::Claude::at(fixture().join("claude/.claude"))
}

fn codex_meter() -> codex::Codex {
    codex::Codex::at(fixture().join("codex/.codex"))
}

fn gemini_meter() -> gemini::Gemini {
    gemini::Gemini::at(fixture().join("gemini/.gemini"))
}

fn claude_days() -> HashMap<String, Tokens> {
    by_day(&claude_meter().tally(), &plus_eight())
}

fn codex_days() -> HashMap<String, Tokens> {
    by_day(&codex_meter().tally(), &plus_eight())
}

fn gemini_days() -> HashMap<String, Tokens> {
    by_day(&gemini_meter().tally(), &plus_eight())
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
        std::os::unix::fs::symlink(fixture().join("gemini/.gemini"), home.join(".gemini"))
            .unwrap();
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
    let days = claude_days();
    let spent = days.get("2026-08-17").copied().expect("the fixture spends on this day");
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
    let days = claude_days();
    let spent = days.get("2026-08-17").copied().unwrap();
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
    let spent = claude_days().get("2026-08-17").copied().unwrap();
    assert_eq!(spent.cached_input, 277, "…and msg_D's 70 is inside that");
}

/// **The day is the machine's, not UTC's.**
///
/// One instant, 16:30 UTC, read under two zones — which is the control
/// the assertion needs: under UTC it is still the 17th, and eight hours
/// east it is already the 18th. A tally that ignored the zone would give
/// the same answer twice, and the first assertion alone could not tell.
///
/// Both vendors are checked, because the two read their timestamps
/// through different fields.
#[test]
fn a_day_is_cut_where_the_machine_stands_not_at_utc_midnight() {
    let tallies = [("claude", claude_meter().tally()), ("codex", codex_meter().tally())];
    for (name, tally) in tallies {
        let east = by_day(&tally, &plus_eight());
        let utc = by_day(&tally, &TimeZone::UTC);
        assert!(
            east.contains_key("2026-08-18"),
            "{name}: 16:30Z is past midnight eight hours east — days: {:?}",
            east.keys().collect::<Vec<_>>()
        );
        assert!(
            !utc.contains_key("2026-08-18"),
            "{name}: …and in UTC that same instant is still the 17th, which is what makes \
             the assertion above about the zone: {:?}",
            utc.keys().collect::<Vec<_>>()
        );
        let east_total: u64 = east.values().map(|t| t.output).sum();
        let utc_total: u64 = utc.values().map(|t| t.output).sum();
        assert_eq!(east_total, utc_total, "{name}: the zone moves a day, never a token");
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
    let days = codex_days();
    assert_eq!(
        days.get("2026-08-17").copied(),
        Some(tokens(600, 2400, 0, 150)),
        "two turns: (1000-800, 800, -, 60) and (2000-1600, 1600, -, 90)"
    );
    assert_eq!(
        days.get("2026-08-18").copied(),
        Some(tokens(400, 100, 33, 20)),
        "the turn past local midnight, and the cache-write only newer codex writes"
    );
    assert_ne!(
        days["2026-08-17"].output,
        60 + 150,
        "the running totals were summed instead of the turns"
    );
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
    let days = codex_days();
    let spent = days["2026-08-17"];
    assert_eq!(spent.output, 150, "60 + 90, each written twice and counted once");
    assert_ne!(spent.output, 300, "every line was counted");
    assert_ne!(spent.output, 60, "the second turn was swallowed as a repeat of the first");
}

/// **The vendor's own total decides whether `cached` came out of
/// `input`** — and the fixture holds one record of each kind so that
/// neither answer can be hard-coded.
///
/// `g-C` totals 322, which is `300 + 10 + 5 + 7` — its 100 cached tokens
/// were never added, so they are already inside the 300 and come out of
/// it. `g-D` totals 520, which is `400 + 30` **plus** its 90 cached, so
/// they were counted apart and nothing is subtracted. An implementation
/// that always subtracts bills `g-D` 310 of fresh input; one that never
/// subtracts bills `g-C` 300 and counts its cache hits twice.
#[test]
fn the_vendors_own_total_decides_whether_cached_came_out_of_the_input() {
    // Found by the instant each was written at, so that the assertion
    // below is not looking the record up by the very number it checks.
    let tally = gemini_meter().tally();
    let at = |stamp: &str| {
        let want: jiff::Timestamp = stamp.parse().unwrap();
        tally.kept.iter().find(|k| k.at == want).expect("the fixture holds this record").tokens
    };
    let c = at("2026-08-17T01:00:02Z");
    let d = at("2026-08-17T01:00:03Z");
    assert_eq!(c.input, 207, "300 less the 100 it had cached, plus 7 of tool");
    assert_eq!(d.input, 400, "its total counted the 90 apart, so none of it is in here");
    assert_eq!((c.cached_input, d.cached_input), (100, 90), "both keep what they had cached");
}

/// **Thinking is output, and it is the vendor's arithmetic that says so.**
///
/// `khor_core::Tokens::output` is "what the model produced, reasoning
/// included where a vendor separates the two". Gemini separates them — its
/// own total counts `thoughts` beside `output` — so the two are added.
/// Codex is the other side of the same rule and is asserted about above:
/// there `reasoning_output_tokens` is already inside `output_tokens` and
/// adding them would bill work nobody did.
///
/// Asserted on the day rather than the record so that a change of mind
/// about `thoughts` cannot hide inside a total that happens to match.
#[test]
fn thinking_is_output_and_a_tool_call_is_input() {
    let days = gemini_days();
    let spent = days["2026-08-17"];
    assert_eq!(
        spent,
        tokens(1007, 990, 0, 155),
        "A(200/800/0/100) C(207/100/0/15) D(400/90/0/30) F(200/0/0/10)"
    );
    assert_ne!(spent.output, 155 - 47, "the thinking was dropped");
    assert_ne!(spent.input, 1007 - 7, "the tool tokens were dropped");
    assert_eq!(spent.cache_write, 0, "gemini reports no cache creation at all");
}

/// **One answer is billed once, and the rule spans files.**
///
/// `g-A` is written twice in the same recording — which is what gemini
/// does — and once more in a nested one, exactly as a copied history
/// would. Summing the lines bills it three times.
///
/// The control sits in the nested file too: `g-F` is there and nowhere
/// else, and it *is* billed. Skipping nested files entirely would satisfy
/// the first assertion and fail this one — which is the same pair claude's
/// resume test uses, and the same reason.
#[test]
fn an_answer_written_twice_is_billed_once_across_files_too() {
    let days = gemini_days();
    let spent = days["2026-08-17"];
    assert_eq!(spent.cached_input, 990, "g-A's 800 counted once, not twice or three times");
    assert_ne!(spent.cached_input, 990 + 800, "g-A was billed again from the nested file");
    assert!(
        spent.output >= 10,
        "g-F sits only in the nested file and must still be read: {spent:?}"
    );
}

/// **A renamed field is counted, not billed as nothing.**
///
/// `g-R` carries the API's own spelling (`promptTokenCount`,
/// `candidatesTokenCount`) instead of the CLI's, so every name khor knows
/// reads zero while the record's own total says 1008 were spent. Billing
/// it as zero would be a machine that ran an agent and cost nothing —
/// which no other assertion here can see, since a quiet day looks exactly
/// like that.
///
/// The control is the record above it: an answer carrying no `tokens` at
/// all (`g-quiet`) is a message, not a failure, and must not be counted.
#[test]
fn a_renamed_token_field_is_counted_rather_than_billed_as_nothing() {
    let tally = gemini_meter().tally();
    assert_eq!(
        tally.unreadable, 2,
        "the renamed one and the one that never says which answer it is"
    );
    assert!(
        tally.kept.iter().all(|k| k.tokens != tokens(0, 0, 0, 0)),
        "a record read as four zeroes was billed instead of counted"
    );
}

/// **What it cannot read, it counts — and a half-written line is not
/// that.**
///
/// The fixture holds four unreadable records: a garbled line in the
/// middle of a transcript, an assistant message that bills tokens without
/// saying which message it is, one whose timestamp is not a time, and a
/// codex `token_count` with no turn in it. It also ends `sess-1.jsonl`
/// with an unterminated line — an agent mid-write — which must not count,
/// or the drift alarm goes off every time somebody is working.
///
/// The readable rows are asserted first: an implementation that gave up
/// on the file at the garbled line would report the same count while
/// losing everything after it.
#[test]
fn what_it_cannot_read_it_counts_and_an_unfinished_line_is_not_that() {
    let usage = both("unreadable");
    assert_eq!(
        usage.unreadable, 6,
        "three from claude's tree, one from codex's, two from gemini's"
    );
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
            ("2026-08-17", "gemini", 155),
            ("2026-08-18", "claude", 4),
            ("2026-08-18", "codex", 20),
            ("2026-08-18", "gemini", 20),
        ],
        "oldest day first, then by vendor — and every vendor's own numbers under its own name"
    );
}

/// The vendors' names are the ones the session list already uses.
///
/// Not cosmetic: a row in the session list and a row in the spending list
/// are grouped by the same string, so a second spelling would put one
/// agent in two categories on two screens and nothing would report an
/// error.
///
/// Gemini is asserted against a literal instead, and the difference is the
/// point: it has no adaptor to agree with, because a session row needs a
/// live process and spending does not. The literal is what a gemini
/// adaptor will have to match on the day there is one.
#[test]
fn the_categories_are_the_ones_the_session_list_already_spells() {
    assert_eq!(claude::VENDOR, crate::adaptor::claude::VENDOR);
    assert_eq!(codex::VENDOR, crate::adaptor::codex::VENDOR);
    assert_eq!(
        (claude::VENDOR, codex::VENDOR, gemini::VENDOR),
        ("claude", "codex", "gemini")
    );
}

/// A meter that reads nothing is the closed default, so no test of this
/// module can touch the machine it runs on unless it says so.
#[test]
fn the_empty_registry_reads_nothing() {
    assert_eq!(Meters::empty().tally_in(&plus_eight()), Usage::default());
}

/// **The answer is kept until the files move, and re-folded the moment
/// they do.**
///
/// Both halves in one test, because either alone passes for the wrong
/// reason: an answer that never expires satisfies "the second ask did no
/// work", and no cache at all satisfies "an appended turn shows up".
///
/// "Did no work" is asserted against this registry's own counter rather
/// than against a stopwatch — a timing assertion on a shared machine
/// measures how busy the machine is (docs/handoff 固定 sleep 的测试，
/// 测的是「机器闲不闲」).
#[test]
fn a_kept_answer_survives_until_the_tree_moves_and_not_after() {
    let home = std::env::temp_dir().join(format!("khor-usage-cache-{}", std::process::id()));
    let dir = home.join(".claude/projects/p");
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("s.jsonl");
    std::fs::write(&file, format!("{}\n", claude_line("m1", 100))).unwrap();

    let meters = Meters::at(&home);
    let out = |u: &Usage| u.days.iter().map(|d| d.tokens.output).sum::<u64>();

    let first = meters.tally();
    assert_eq!(out(&first), 100, "the one message in the tree");
    let after_first = meters.passes();

    let again = meters.tally();
    assert_eq!(again, first);
    assert_eq!(meters.passes(), after_first, "the second ask must not have re-folded anything");

    // A turn is appended, the way an agent appends one.
    append(&file, &claude_line("m2", 7));
    let third = meters.tally();
    assert_eq!(out(&third), 107, "the appended turn is in the answer");
    assert!(meters.passes() > after_first, "…and it got there by reading the file again");

    let _ = std::fs::remove_dir_all(&home);
}

/// **A pass reads only what was appended, and the answer is the same as a
/// reader that started from scratch.**
///
/// This is the assertion the incremental bookkeeping exists for, and it
/// is written as an equality against a **second meter opened on the same
/// tree** — which has read nothing before and therefore has no offsets to
/// be wrong about. An off-by-one in the offset shows up as one record
/// billed twice or lost, and either way the two disagree.
///
/// The half-written tail is here too, because it is the case the offset
/// rule and the drift alarm share: a line with no newline must be neither
/// counted **nor consumed**, or the finished line would never be read.
#[test]
fn reading_only_the_tail_gives_the_same_answer_as_reading_it_all() {
    let home = std::env::temp_dir().join(format!("khor-usage-tail-{}", std::process::id()));
    let dir = home.join(".claude/projects/p");
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("s.jsonl");
    std::fs::write(&file, format!("{}\n", claude_line("m1", 100))).unwrap();

    let kept = claude::Claude::at(home.join(".claude"));
    let fresh =
        || by_day(&claude::Claude::at(home.join(".claude")).tally(), &TimeZone::UTC);
    let seen = |m: &claude::Claude| by_day(&m.tally(), &TimeZone::UTC);
    let out = |d: &HashMap<String, Tokens>| d.values().map(|t| t.output).sum::<u64>();
    assert_eq!(seen(&kept), fresh(), "the two agree before anything is appended");

    // A line arrives whole.
    append(&file, &claude_line("m2", 7));
    assert_eq!(seen(&kept), fresh(), "and after one append");
    assert_eq!(out(&seen(&kept)), 107, "which is 100 + 7, not 100 and not 207");

    // Half a line arrives. Neither reader may count it, and the one that
    // has been following along must not consume it either.
    std::fs::write(
        &file,
        format!(
            "{}\n{}\n{}",
            claude_line("m1", 100),
            claude_line("m2", 7),
            r#"{"type":"assistant","timestamp":"2026-08-17T06:00:00Z","message":{"id":"m3","usa"#
        ),
    )
    .unwrap();
    assert_eq!(seen(&kept), fresh(), "a half-written line is nobody's record yet");
    assert_eq!(out(&seen(&kept)), 107);

    // …and once it is finished it counts, which is what proves the
    // unfinished line was left unconsumed rather than skipped forever.
    std::fs::write(
        &file,
        format!(
            "{}\n{}\n{}\n",
            claude_line("m1", 100),
            claude_line("m2", 7),
            claude_line("m3", 1000)
        ),
    )
    .unwrap();
    assert_eq!(seen(&kept), fresh());
    assert_eq!(out(&seen(&kept)), 1107, "the finished line was picked up on the next pass");

    let _ = std::fs::remove_dir_all(&home);
}

/// **A transcript that was replaced rather than appended to is read again
/// from the beginning**, and one that was deleted stops being billed.
///
/// Both are the ways an offset can be wrong about a file. A shorter file
/// is not the file that was read — logs do not lose their beginning — and
/// a meter that trusted its offset would either seek past the end and see
/// nothing or keep billing contents that are gone.
#[test]
fn a_replaced_transcript_is_read_again_and_a_deleted_one_stops_counting() {
    let home = std::env::temp_dir().join(format!("khor-usage-replace-{}", std::process::id()));
    let dir = home.join(".claude/projects/p");
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("s.jsonl");
    let meter = claude::Claude::at(home.join(".claude"));
    let out = |m: &claude::Claude| {
        by_day(&m.tally(), &TimeZone::UTC).values().map(|t| t.output).sum::<u64>()
    };

    std::fs::write(&file, format!("{}\n{}\n", claude_line("m1", 100), claude_line("m2", 200)))
        .unwrap();
    assert_eq!(out(&meter), 300);

    // Rotated: shorter, and holding something else entirely.
    std::fs::write(&file, format!("{}\n", claude_line("m9", 5))).unwrap();
    assert_eq!(out(&meter), 5, "the old contents are gone, so they are not billed");

    std::fs::remove_file(&file).unwrap();
    assert_eq!(out(&meter), 0, "a transcript that is gone bills nothing");

    let _ = std::fs::remove_dir_all(&home);
}

/// One claude assistant line, billing `out` and nothing else.
fn claude_line(id: &str, out: u64) -> String {
    format!(
        r#"{{"type":"assistant","timestamp":"2026-08-17T06:00:00Z","message":{{"id":"{id}","usage":{{"input_tokens":0,"cache_read_input_tokens":0,"cache_creation_input_tokens":0,"output_tokens":{out}}}}}}}"#
    )
}

/// Appends a line, the way an agent does — the file is not rewritten.
fn append(path: &std::path::Path, line: &str) {
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new().append(true).open(path).unwrap();
    writeln!(f, "{line}").unwrap();
}

/// **The same tail rule on the vendor where getting it wrong duplicates
/// rather than loses.**
///
/// Claude's meter is idempotent under re-reading — a message read twice
/// collapses on its own id — so an offset that was too small would hide
/// there. Codex's turns are a list, and reading a byte twice bills a turn
/// twice, so this is where the bookkeeping is actually load-bearing.
///
/// Compared against a meter opened fresh on the same tree, for the reason
/// the claude version gives: a fresh reader has no offsets to be wrong
/// about.
#[test]
fn a_codex_turn_read_from_the_tail_is_billed_once() {
    let home = std::env::temp_dir().join(format!("khor-usage-codex-{}", std::process::id()));
    let dir = home.join(".codex/sessions/2026/08/17");
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("rollout-x.jsonl");
    std::fs::write(&file, format!("{}\n", codex_line(100, 100))).unwrap();

    let kept = codex::Codex::at(home.join(".codex"));
    let fresh = || by_day(&codex::Codex::at(home.join(".codex")).tally(), &TimeZone::UTC);
    let out = |d: &HashMap<String, Tokens>| d.values().map(|t| t.output).sum::<u64>();

    assert_eq!(out(&by_day(&kept.tally(), &TimeZone::UTC)), 100);
    append(&file, &codex_line(30, 130));
    let seen = by_day(&kept.tally(), &TimeZone::UTC);
    assert_eq!(seen, fresh(), "the tail reader and a fresh one must agree");
    assert_eq!(out(&seen), 130, "100 + 30 — not 230, and not 100");

    // The written-twice rule has to survive the boundary too: the repeat
    // lands in a later pass than the turn it repeats.
    append(&file, &codex_line(30, 130));
    let seen = by_day(&kept.tally(), &TimeZone::UTC);
    assert_eq!(seen, fresh(), "…including across a pass boundary");
    assert_eq!(out(&seen), 130, "the repeat is the same turn written again");

    let _ = std::fs::remove_dir_all(&home);
}

/// One codex `token_count` event: a turn of `out` output tokens, with the
/// session's running total at `running`.
fn codex_line(out: u64, running: u64) -> String {
    format!(
        r#"{{"timestamp":"2026-08-17T06:00:00Z","type":"event_msg","payload":{{"type":"token_count","info":{{"total_token_usage":{{"input_tokens":0,"cached_input_tokens":0,"output_tokens":0,"reasoning_output_tokens":0,"total_tokens":{running}}},"last_token_usage":{{"input_tokens":0,"cached_input_tokens":0,"output_tokens":{out},"reasoning_output_tokens":0,"total_tokens":{out}}}}}}}}}"#
    )
}

/// **A window of N days reaches back N-1 days from today**, and one day
/// is today alone.
///
/// The span is measured back out with a different operation than the one
/// that produced it (a date difference, not a subtraction), so an
/// implementation that got the arithmetic wrong in one direction does not
/// get to agree with itself. And the endpoints are asserted as
/// relationships rather than against a literal date, which would go red
/// tomorrow.
#[test]
fn a_window_of_n_days_reaches_back_n_minus_one() {
    let today = super::today();
    assert_eq!(window_start(1), today.to_string(), "one day is today alone");
    for days in [2usize, 7, 30] {
        let start: jiff::civil::Date = window_start(days).parse().unwrap();
        let span = today.since(start).unwrap();
        assert_eq!(
            span.get_days(),
            days as i32 - 1,
            "a window of {days} days starts {} days back",
            days - 1
        );
        assert!(start < today, "a window of more than one day starts before today");
    }
    // Zero is not a window; it is treated as one day rather than given a
    // meaning of its own, and the caller refuses it before getting here.
    assert_eq!(window_start(0), window_start(1));
}
