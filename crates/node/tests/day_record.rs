//! The day strip against a real node: **what gets recorded is the list,
//! not the registry.**
//!
//! The unit tests in `khor_node::day` drive the record directly, so they
//! can say what it does with the words it is handed and nothing about
//! where those words come from. That is the half worth an end-to-end
//! test, because it is the half the whole design turns on: half of what
//! a list shows was never written to disk by anything (`live::face`
//! derives 失败 from a dead process and 中断 from a lost host at read
//! time; `live::settle_done` turns 完成 into 空闲 when a watermark
//! moves), so a recorder wired to `live::write_state` would look correct
//! in every unit test and paint a crashed agent as 忙碌 all day.
//!
//! The proof here is the pair: `state.json` still says 忙碌 while the
//! strip says 失败, in the same instant, from the same node.

mod util;

use std::fs;
use std::path::Path;

use khor_core::{DayCell, DeviceId, SessionId, State};
use khor_node::Node;
use khor_node::day;
use khor_node::live::LiveKind;

/// A scratch tree that **takes itself away however the test leaves**
/// (`crates/node/src/tests.rs::Scratch` has the whole reasoning: a
/// cleanup written at the end of a test is skipped by the assertion
/// that fails, so a red run leaks a directory every time it is red).
/// A copy per file, which is this repo's convention for test helpers —
/// it costs no dev-dependency.
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
    let p = std::env::temp_dir().join(format!("khor-day-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    util::hermetic(&p);
    Scratch(p)
}

/// A pid nobody is using: one that ran and was reaped. Spawned and
/// waited on rather than picked out of the air, so it is a number the
/// OS really did hand out and really has taken back.
fn a_dead_pid() -> u32 {
    let mut child = std::process::Command::new("/bin/sh")
        .args(["-c", "exit 0"])
        .spawn()
        .expect("sh runs");
    let pid = child.id();
    child.wait().expect("it exits");
    pid
}

/// The newest cell of a row's strip — what the right-hand end of the
/// band is showing right now.
fn newest(node: &Node, id: &str) -> DayCell {
    let strips = node.day_strips(day::MINUTES).expect("strips");
    let runs = strips
        .get(id)
        .unwrap_or_else(|| panic!("{id} must have a strip"));
    runs.last().expect("a strip is never empty").cell
}

/// The word on disk in the registry, which is a different question from
/// the word the list shows.
fn word_on_disk(root: &Path, id: &SessionId) -> State {
    let dir = LiveKind::new(root.to_path_buf(), DeviceId([1; 32]))
        .dir_of(id)
        .expect("an id");
    let raw = fs::read(dir.join("state.json")).expect("state.json");
    let v: serde_json::Value = serde_json::from_slice(&raw).expect("json");
    serde_json::from_value(v.get("word").expect("a word").clone()).expect("a state")
}

/// **The whole thesis, in one assertion pair.**
///
/// A session whose process is gone reads 失败 on the list and 忙碌 in
/// the registry — nothing wrote the 失败 anywhere, `live::face` works it
/// out each time it is asked. The strip must carry the first, and the
/// only way it can is by recording what the list said.
#[test]
fn the_strip_records_the_word_the_list_shows_not_the_one_on_disk() {
    let r = root("derived");
    let node = Node::open_as(r.to_path_buf(), "alpha").expect("a node opens");
    let id = SessionId("tui/gone".to_owned());
    // Registering writes 忙碌 and never touches it again; the pid is
    // one the OS has already taken back.
    LiveKind::new(r.to_path_buf(), node.device())
        .register(&id, "tui", "an agent that died", Some(a_dead_pid()), None)
        .expect("registered");
    // Found, not started: on the list because it was imported
    // (docs/SHELL.md 第八轮) — the strip answers for the list's rows.
    node.import(&id.0).expect("imported");

    node.record_day().expect("a sample");

    assert_eq!(
        word_on_disk(&r, &id),
        State::Busy,
        "nothing rewrote the registry — that is the premise of this test"
    );
    assert_eq!(
        newest(&node, &id.0),
        DayCell::Word(State::Failed),
        "the strip must carry the derived word, not the stored one"
    );
}

/// A row the list can see is recorded as its word; a row that does not
/// exist is recorded as **having been looked for and not found**, which
/// is a different cell from the one meaning nobody looked.
///
/// The second half is what keeps a session that started this morning
/// from reading like a morning khor spent switched off.
#[test]
fn a_sampled_minute_tells_a_missing_row_apart_from_a_missing_watcher() {
    let r = root("absent");
    let node = Node::open_as(r.to_path_buf(), "alpha").expect("a node opens");
    let id = SessionId("tui/alive".to_owned());
    LiveKind::new(r.to_path_buf(), node.device())
        .register(&id, "tui", "an agent", Some(std::process::id()), None)
        .expect("registered");
    node.import(&id.0).expect("imported");

    node.record_day().expect("a sample");

    assert_eq!(newest(&node, &id.0), DayCell::Word(State::Busy));

    // A row nothing on this machine has ever listed. It is not in the
    // answer at all — `day_strips` answers for the rows the list has —
    // so ask the record itself, which is what a row appearing later
    // would be cut from.
    let record = day::read(&r);
    let strip = record.strip_of("tui/never-existed", day::MINUTES);
    assert_eq!(
        strip.last().map(|run| run.cell),
        Some(DayCell::Absent),
        "the minute khor sampled says the row was not there"
    );
    assert_eq!(
        strip.first().map(|run| run.cell),
        Some(DayCell::Unwatched),
        "the minutes before khor ever sampled say nobody was looking"
    );
}

/// The record survives the process that wrote it — the sampler is a
/// resident's task and residents restart (`keeper.rs` exists because
/// they do). A second node on the same home reads what the first one
/// recorded, and adds to it rather than replacing it.
///
/// **The verb in the claim happens in the test**: two `Node`s, one after
/// the other, and the assertion is about what the first one wrote.
#[test]
fn a_second_node_on_the_same_home_keeps_what_the_first_recorded() {
    let r = root("restart");
    let id = SessionId("tui/kept".to_owned());
    {
        let node = Node::open_as(r.to_path_buf(), "alpha").expect("a node opens");
        LiveKind::new(r.to_path_buf(), node.device())
            .register(&id, "tui", "an agent", Some(std::process::id()), None)
            .expect("registered");
        node.import(&id.0).expect("imported");
        node.record_day().expect("a sample");
        assert_eq!(newest(&node, &id.0), DayCell::Word(State::Busy));
    }
    let again = Node::open_as(r.to_path_buf(), "alpha").expect("a second node opens");
    assert_eq!(
        newest(&again, &id.0),
        DayCell::Word(State::Busy),
        "a restart must read the record, not start a fresh one"
    );
}
