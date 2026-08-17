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

/// One of the three agents that write the Pi format, read from the
/// fixture's copy of its own root.
fn pi_meter(vendor: &'static str) -> pi::PiFormat {
    let root = pi::ROOTS
        .iter()
        .find(|(name, _)| *name == vendor)
        .map(|(_, root)| *root)
        .expect("a vendor this format knows");
    pi::PiFormat::at(vendor, fixture().join("pi").join(root))
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

fn pi_days(vendor: &'static str) -> HashMap<String, Tokens> {
    by_day(&pi_meter(vendor).tally(), &plus_eight())
}

/// One VS Code extension, read from whichever storage location the
/// fixture put it under — which is a different one for each of the three,
/// so finding it at all is part of what this asserts.
fn roo_meter(vendor: &'static str) -> roo::Roo {
    let base = fixture().join("roo");
    let root = roo::roots()
        .into_iter()
        .find(|(name, root)| *name == vendor && base.join(root).exists())
        .map(|(_, root)| base.join(root))
        .expect("the fixture holds exactly one storage location for this vendor");
    roo::Roo::at(vendor, root)
}

fn qwen_meter() -> qwen::Qwen {
    qwen::Qwen::at(fixture().join("qwen/.qwen"))
}

fn junie_meter() -> junie::Junie {
    junie::Junie::at(fixture().join("junie/.junie"))
}

fn amp_meter() -> amp::Amp {
    amp::Amp::at(fixture().join("amp/.local/share/amp"))
}

/// OpenClaw read from one of the four names it has shipped under.
fn openclaw_meter(root: &str) -> openclaw::OpenClaw {
    openclaw::OpenClaw::at(fixture().join("openclaw").join(root))
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
        // The three Pi-format roots live under one fixture directory, so
        // that one home wears all of them the way a real one would.
        for dir in [".pi", ".senpi"] {
            std::os::unix::fs::symlink(fixture().join("pi").join(dir), home.join(dir)).unwrap();
        }
        // `.config` has two tenants — Kimchi and VS Code — so it is a real
        // directory here with a link per child, which is also what a real
        // home looks like. Linking the whole of `.config` to one fixture
        // would have let either vendor hide the other.
        std::fs::create_dir_all(home.join(".config")).unwrap();
        std::os::unix::fs::symlink(
            fixture().join("pi/.config/kimchi"),
            home.join(".config/kimchi"),
        )
        .unwrap();
        std::os::unix::fs::symlink(
            fixture().join("roo/.config/Code"),
            home.join(".config/Code"),
        )
        .unwrap();
        std::fs::create_dir_all(home.join("Library/Application Support")).unwrap();
        std::os::unix::fs::symlink(
            fixture().join("roo/Library/Application Support/Code"),
            home.join("Library/Application Support/Code"),
        )
        .unwrap();
        std::os::unix::fs::symlink(
            fixture().join("roo/.vscode-server"),
            home.join(".vscode-server"),
        )
        .unwrap();
        std::os::unix::fs::symlink(fixture().join("qwen/.qwen"), home.join(".qwen")).unwrap();
        std::os::unix::fs::symlink(fixture().join("junie/.junie"), home.join(".junie")).unwrap();
        for root in ["openclaw", "moltbot"] {
            std::os::unix::fs::symlink(
                fixture().join("openclaw").join(format!(".{root}")),
                home.join(format!(".{root}")),
            )
            .unwrap();
        }
        std::fs::create_dir_all(home.join(".local/share")).unwrap();
        std::os::unix::fs::symlink(
            fixture().join("amp/.local/share/amp"),
            home.join(".local/share/amp"),
        )
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
        tally.unreadable, 3,
        "the renamed one, the one that never says which answer it is, and the one \
         whose parts do not add up to its own total"
    );
    assert!(
        tally.kept.iter().all(|k| k.tokens != tokens(0, 0, 0, 0)),
        "a record read as four zeroes was billed instead of counted"
    );
}

/// **The Pi format needs no arithmetic, and that is the assertion.**
///
/// Its `usage` is already spelled in khor's four names, so the failure
/// this guards against is not a wrong sum — it is somebody "helpfully"
/// making it look like its neighbours, subtracting the cache read out of
/// input the way codex and gemini need. `p-A` is the record that would
/// catch it: 100 of input beside 30 of cache read, which must stay 100.
///
/// `p-D` is the other half: it carries `reasoning: 25`, which this format
/// documents as already inside `output`. Adding it — which is right for
/// gemini and wrong here — turns 40 into 65.
#[test]
fn the_pi_format_is_carried_across_without_arithmetic() {
    let spent = pi_days("pi")["2026-08-17"];
    assert_eq!(
        spent,
        tokens(1171, 30, 5, 593),
        "A(100/30/5/20) B(50/0/0/30) C(7/0/0/1) D(10/0/0/40) E(4/0/0/2) A'(1000/0/0/500)"
    );
    assert_ne!(spent.input, 1171 - 30, "the cache read was subtracted out of input");
    assert_ne!(spent.output, 593 + 25, "p-D's reasoning was added to its output");
}

/// **One message id means one answer *within a session*, not across the
/// tree.**
///
/// `p-A` appears three times: twice under `pi-sess-1` (a resumed session
/// copying its history, which must be billed once) and once under
/// `pi-sess-2` — a different conversation whose first answer happens to
/// carry the same id, because ids in this format are numbered per
/// session. Merging on the bare id would collapse the two conversations'
/// first answers into whichever one produced more, losing the other.
///
/// The two failures are opposite and both are named, and **both numbers
/// were read off a real red rather than worked out on paper**: keying on
/// the bare id loses 100 (the merge keeps whichever `p-A` produced more
/// output, which is the second session's), and dropping the merge
/// altogether adds 100.
#[test]
fn a_message_id_belongs_to_its_session_and_not_to_the_tree() {
    let spent = pi_days("pi")["2026-08-17"];
    assert_ne!(
        spent.input, 1071,
        "the two sessions' p-A collided on a bare id and one of them was dropped"
    );
    assert_ne!(spent.input, 1271, "the resumed file billed p-A a second time");
    assert_eq!(spent.input, 1171);
}

/// **One reader, three vendors, and each row keeps its own name.**
///
/// Pi, Senpi and Kimchi share a parser because upstream says they share a
/// format — so the thing worth asserting is the half that sharing could
/// break: which name the tokens land under. A reader that stamped a
/// constant, or the first vendor's name, would satisfy every other
/// assertion in this file.
#[cfg(unix)]
#[test]
fn three_agents_share_a_reader_and_none_of_them_share_a_name() {
    let usage = both("pi-family");
    for (vendor, output) in [("pi", 593), ("senpi", 2), ("kimchi", 4)] {
        assert_eq!(
            day(&usage, "2026-08-17", vendor).map(|t| t.output),
            Some(output),
            "{vendor} reads its own root and answers under its own name"
        );
    }
}

/// **A task document is a JSON array, and the numbers are inside a string
/// inside it.**
///
/// The fixture is upstream's own test document carried across rather than
/// one written from reading their parser — the numbers, the nesting and
/// both spellings of the clock are theirs. It holds five entries and only
/// three of them are requests: one with a time, one with milliseconds
/// since the epoch (both occur upstream), and one whose payload is not
/// JSON at all. A fourth parses and names none of the four token fields.
///
/// The last two are the point. Upstream skips both in silence; khor
/// counts them, because a request that plainly billed something and could
/// not be read is exactly what `unreadable` is for.
#[test]
fn a_task_document_is_read_whole_and_its_numbers_carried_across() {
    let tally = roo_meter("roocode").tally();
    let spent = by_day(&tally, &plus_eight())["2026-08-17"];
    assert_eq!(
        spent,
        tokens(110, 21, 5, 52),
        "the two readable requests: 100/20/5/50 and 10/1/0/2"
    );
    assert_eq!(
        tally.unreadable, 2,
        "the payload that is not JSON, and the one that names no token field"
    );
    assert_eq!(tally.kept.len(), 2, "the entry that is not a request was not billed");
}

/// **Each extension is found where that extension keeps things, and the
/// three places are not the same place.**
///
/// The fixture puts roocode under Linux's `.config`, cline under macOS's
/// `Library/Application Support` and kilocode under a remote
/// `.vscode-server` — so a reader that knew only the platform it is
/// running on would find one of the three and miss two, which on a real
/// machine is a user whose tokens quietly do not exist.
#[cfg(unix)]
#[test]
fn the_three_extensions_are_found_in_three_different_storage_locations() {
    let usage = both("roo-family");
    for (vendor, output) in [("roocode", 52), ("cline", 3), ("kilocode", 4)] {
        assert_eq!(
            day(&usage, "2026-08-17", vendor).map(|t| t.output),
            Some(output),
            "{vendor} was not found, or was found under somebody else's name"
        );
    }
}

/// **A document read whole must be replaced, not added to.**
///
/// This is the whole reason `Whole` exists beside `Files`. A task log is a
/// JSON array the extension rewrites as it grows, so the second pass sees
/// the first request again — and a bookkeeper that accumulated the way the
/// append-only one does would bill it twice, with the number climbing
/// every time the user did anything.
///
/// Both halves are asserted, because either alone passes for the wrong
/// reason: a meter that never re-read the file would satisfy "the first
/// request was billed once" and fail to see the second.
#[test]
fn a_rewritten_task_document_is_not_billed_twice() {
    let root = std::env::temp_dir().join(format!("khor-usage-roo-{}", std::process::id()));
    let task = root.join("task-1");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&task).unwrap();
    let log = task.join(roo::LOG);
    let request = |at: &str, tokens: u64| {
        format!(
            r#"{{"type":"say","say":"api_req_started","ts":"{at}","text":"{{\"tokensIn\":{tokens},\"tokensOut\":1}}"}}"#
        )
    };
    std::fs::write(&log, format!("[{}]", request("2026-08-17T01:00:00Z", 10))).unwrap();

    let meter = roo::Roo::at("roocode", root.clone());
    let first = meter.tally();
    assert_eq!(first.kept.len(), 1, "one request in the file, one record");

    // The extension answers again and rewrites the whole array.
    std::fs::write(
        &log,
        format!(
            "[{},{}]",
            request("2026-08-17T01:00:00Z", 10),
            request("2026-08-17T01:00:05Z", 20)
        ),
    )
    .unwrap();
    let second = meter.tally();
    let spent: u64 = second.kept.iter().map(|k| k.tokens.input).sum();
    // The sum first and the count second, so that the failure this exists
    // for is the one that speaks: an accumulating bookkeeper says 40 here,
    // and 40 is the number a user would watch climb.
    assert_eq!(spent, 30, "10 and 20, each once");
    assert_ne!(spent, 40, "the first request was billed again on the second pass");
    assert_eq!(second.kept.len(), 2, "two requests in the file, two records");
    let _ = std::fs::remove_dir_all(&root);
}

/// **The witness gemini uses travels to the fork, and khor parts company
/// with upstream here.**
///
/// Qwen Code writes the API's `usageMetadata` rather than the Gemini
/// CLI's own fields, and the fixture holds one record of each case the
/// witness has to tell apart:
///
/// - 1000 prompt / 800 cached with a total of 1060 — the cached part was
///   never added on its own line, so it is inside the prompt and comes
///   out: 200 fresh.
/// - 400 prompt / 90 cached with a total of 520 — the total *does* count
///   it apart, so it stays: 400 fresh.
/// - 300 prompt / 100 cached with **no total at all** — no witness, and
///   the default is to take it out anyway: 200 fresh.
///
/// **That last one is the case that decides the direction of every error
/// this can make.** Leaving it in would report the same 100 tokens twice
/// — once inside `input`, once in `cached_input` — while
/// `khor_core::Tokens::input` promises input never includes what came out
/// of a cache. Taking it out when it was genuinely separate loses 100
/// from the input column instead, and that is the side khor is allowed to
/// be wrong on.
#[test]
fn the_cached_witness_travels_to_the_gemini_fork() {
    let spent = by_day(&qwen_meter().tally(), &plus_eight())["2026-08-17"];
    assert_eq!(
        spent,
        tokens(900, 990, 0, 125),
        "100 + 200 + 400 + 200 fresh; 25 + 60 + 30 + 10 out"
    );
    assert_ne!(spent.input, 1800, "upstream's reading: no cache read ever comes out");
    assert_ne!(spent.input, 810, "the total that counts the cache apart was overruled");
}

/// **One event, several models, and every one of them billed.**
///
/// A Junie turn that used two models writes them as two rows of one
/// event's `modelUsage`. A reader that took the first row would report
/// 100/50 — a plausible-looking number that is simply short, which is
/// why this asserts the sum and names the wrong one.
///
/// The two unreadable records are the other half: a row naming neither
/// token field, and an event of the right kind carrying no `modelUsage`
/// at all.
#[test]
fn one_junie_event_can_bill_more_than_one_model() {
    let tally = junie_meter().tally();
    let spent = by_day(&tally, &plus_eight())["2026-08-17"];
    assert_eq!(spent, tokens(107, 0, 0, 53), "100/50 and 7/3, both rows of one event");
    assert_ne!(spent.input, 100, "only the first row of the event was billed");
    assert_eq!(tally.kept.len(), 2, "two rows, two records, one instant");
    assert_eq!(
        tally.unreadable, 2,
        "the row naming no token field, and the event with no modelUsage"
    );
}

/// **A thread that wrote its spending down twice is billed once.**
///
/// This is the assertion the whole `amp` module exists for. Its fixture
/// holds one thread where the two accounts overlap in both the ways
/// upstream distinguishes, plus the case where they do not overlap at
/// all:
///
/// - the ledger event that names its message (`toMessageId: 2`),
/// - the one that names nothing and is recognised only by having the same
///   model and the same numbers as message 3,
/// - message 4, which no event accounts for and which therefore **is**
///   billed,
/// - and a second thread with no ledger at all, read from its messages.
///
/// Both failures are named with the number the red actually produces —
/// **read off the failing run, not worked out on paper**, which is the
/// third time in this batch that arithmetic in somebody's head named a
/// value the test could never reach.
#[test]
fn a_thread_that_wrote_its_spending_twice_is_billed_once() {
    let tally = amp_meter().tally();
    let spent = by_day(&tally, &plus_eight())["2026-08-17"];
    assert_eq!(
        spent,
        tokens(127, 20, 5, 59),
        "thread 1: 100/20/5/50 + 7/0/0/3 + the unmatched 11/0/0/2; thread 2: 9/0/0/4"
    );
    assert_ne!(spent.input, 234, "the ledger and the messages were both billed");
    assert_ne!(spent.input, 116, "the message no event accounted for was dropped");
    assert_eq!(tally.unreadable, 1, "the event that never says which model it was");
}

/// **A record that does not add up to its own total is counted, not
/// billed** — the same door the Gemini family gets, on a vendor that
/// spells the four names khor uses.
///
/// The fixture's four assistant records are one of each case: two that
/// reconcile (one of them carrying a cache write, so the total is shown
/// to span all four rather than the two easy ones), one whose parts sum
/// to 6 against a stated 999, and one that bills without saying when.
/// The last two are the count; the first two are the bill.
#[test]
fn openclaw_checks_its_reading_against_the_total_the_vendor_wrote() {
    let tally = openclaw_meter(".openclaw/agents").tally();
    let spent = by_day(&tally, &plus_eight())["2026-08-17"];
    assert_eq!(spent, tokens(110, 200, 7, 52), "100/200/0/50 and 10/0/7/2");
    assert_eq!(
        tally.unreadable, 2,
        "the one that does not add up, and the one with no time on it"
    );
    assert_ne!(spent.input, 115, "the record that disagrees with its own total was billed");
}

/// **An agent that has been renamed is read under every name it shipped
/// as, and answers under one.**
///
/// A user who upgraded still has spending recorded under the old name; a
/// user who did not still has an agent writing there. The fixture puts
/// one record under `.moltbot` and the rest under `.openclaw`, so a
/// reader that knew only the current name would be short by exactly that
/// record — and short is the failure that looks like a quiet week.
#[cfg(unix)]
#[test]
fn a_renamed_agent_is_read_under_every_name_it_shipped_as() {
    let usage = both("openclaw-names");
    assert_eq!(
        day(&usage, "2026-08-17", "openclaw").map(|t| t.output),
        Some(56),
        "50 + 2 under the current name, 4 under the one it used to have"
    );
    assert_ne!(
        day(&usage, "2026-08-17", "openclaw").map(|t| t.output),
        Some(52),
        "only the current name was looked under"
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
        usage.unreadable, 18,
        "three claude, one codex, three gemini, two pi, two roo, two qwen, two junie, \
         one amp, two openclaw"
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
            ("2026-08-17", "amp", 59),
            ("2026-08-17", "claude", 563),
            ("2026-08-17", "cline", 3),
            ("2026-08-17", "codex", 150),
            ("2026-08-17", "gemini", 155),
            ("2026-08-17", "junie", 53),
            ("2026-08-17", "kilocode", 4),
            ("2026-08-17", "kimchi", 4),
            ("2026-08-17", "openclaw", 56),
            ("2026-08-17", "pi", 593),
            ("2026-08-17", "qwen", 125),
            ("2026-08-17", "roocode", 52),
            ("2026-08-17", "senpi", 2),
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
