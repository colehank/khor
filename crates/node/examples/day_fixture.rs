//! Builds a khor home holding a day worth looking at, for the browser
//! gate over the day strip (`apps/gui/scripts/check-day.mjs`).
//!
//! # Why this is not a hand-written `day.json`
//!
//! A record made by hand is a 替身, and a strip measured against one
//! proves only that the painter agrees with whoever wrote the fixture.
//! This drives the **real recorder** — `day::Day::observe`, the same
//! call the resident's sampler makes — and only feeds it synthetic
//! timestamps. What lands on disk is whatever the shipping code writes.
//!
//! # Why an example and not a test
//!
//! The gate needs it, and **a gate must not take cargo's artifact lock**
//! (`check-rail`'s head: a check that queues behind a build is a check
//! nobody runs). An example is a build artifact the gate can require,
//! the way `smoke.mjs` requires a built bridge — built once, alongside
//! everything else, and never from inside a check.
//!
//! ```text
//! cargo build -p khor-node --example day_fixture
//! target/debug/examples/day_fixture <home>
//! ```
//!
//! # The trap this fell into once
//!
//! The window is addressed by **absolute** minute, so a record built
//! around minute zero is a day in 1970: the first `Day::load` rolls it
//! forward and every cell falls off the front, leaving a home that looks
//! empty for no visible reason. A fixture has to be built where it will
//! be read — anchored to the clock, not to zero.
//!
//! # And the one after it: a row nobody put on the list
//!
//! Since docs/SHELL.md 第八轮 the session list is the user's choice
//! (`khor_sync::roster`): a terminal or an agent is on it only once khor
//! started it or somebody **imported** it. A `tui` row registered the way
//! a hook registers one is an import *candidate*, not a row. The commit
//! that made that rule taught `tests/day_record.rs` to import its rows
//! and did not teach this fixture, so the three rows below stayed in the
//! registry, off the list — no row to open, no band to read — and the
//! gate saw only the device's own conversation and reported a single
//! word, which read as the day window having moved rather than as rows
//! that never reached the screen. So the rows go through
//! [`Node::import`], the same call the import panel and `khor import`
//! make, and the gate is told which rows to expect (`row <id>` on
//! stdout) so a row that goes missing is named as missing.

use std::path::PathBuf;

use khor_core::{SessionId, State};
use khor_node::Node;
use khor_node::day;
use khor_node::live::LiveKind;

fn main() -> Result<(), String> {
    let home = PathBuf::from(std::env::args().nth(1).ok_or("usage: day_fixture <home>")?);
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).map_err(|e| e.to_string())?;

    let node = Node::open_as(home.clone(), "alpha")?;
    let live = LiveKind::new(home.clone(), node.device());
    let rows = [
        ("tui/agent", "an agent that worked all day"),
        ("tui/crashed", "an agent that died at noon"),
        ("tui/latecomer", "something that started six hours ago"),
        // **The row the gate reads a day with no record on** (2026-09-29).
        // In none of the samples below — but the bridge's resident takes
        // its own sample the moment it is up, so by the time a page asks,
        // this row has a word at the right end like every local session.
        // A day with no record reaches a screen in two other shapes: a
        // peer that never answered (`day::unwatched`, 未查看 end to end)
        // and a row a poll behind the list (no entry). The gate writes
        // those onto this one row's answer on the wire, so the rows the
        // other claims read stay the recorder's own.
        ("tui/quiet", "a session the gate empties the day of"),
    ];
    for (id, title) in rows {
        live.register(
            &SessionId(id.to_owned()),
            "tui",
            title,
            Some(std::process::id()),
            None,
        )?;
        // Imported while this process — the pid on record — is still
        // alive, so each row is a living candidate, which is all the
        // import panel ever offers.
        node.import(id)?;
    }

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_millis() as i64;
    let first = now_ms / 60_000 - day::MINUTES as i64 + 1;
    let mut record = day::Day::empty(now_ms);

    // The shapes are picked so that **every cell kind appears, and the
    // two nothings appear beside each other**: a stretch nobody sampled,
    // and a stretch that was sampled while one of the rows did not
    // exist. A gate whose fixture only produces one word cannot tell
    // "coloured by its word" from "all one colour".
    for m in 0..day::MINUTES as i64 {
        // **The first eight hours: khor was not running at all.** This is
        // what a young install looks like and it is the shape that
        // actually reached a user — a long leading stretch of 不知道 with
        // the coloured part only at the right. It is also the shape that
        // exposed two bugs in a row (an invisible tier read as blank, and
        // then a hairline tier read as a second object), so the fixture
        // has to produce it rather than a tidy full day.
        if m < 480 {
            continue;
        }
        // And half an hour further on with nobody sampling: a restart in
        // the middle of a day khor was otherwise watching. The leading
        // stretch alone would not tell a gap from a beginning.
        if (900..930).contains(&m) {
            continue;
        }
        let mut sample: Vec<(String, State)> = Vec::new();
        sample.push((
            "tui/agent".to_owned(),
            match m {
                // **待批 has to land inside the watched window**, and it
                // did not for one round: it sat at 240–300, the leading
                // eight hours moved to 不知道, and the fixture quietly
                // stopped producing the one word this whole product is
                // built around — the gate went green measuring five words
                // instead of six and said so only in a count nobody was
                // reading. Any change to the window has to be checked
                // against this line.
                600..=660 => State::Blocked,
                1000..=1060 => State::Done,
                1200..=1260 => State::Errored,
                _ => State::Busy,
            },
        ));
        sample.push((
            "tui/crashed".to_owned(),
            if m < 720 { State::Busy } else { State::Failed },
        ));
        // Absent for eighteen hours, then on the list.
        if m >= 1080 {
            sample.push(("tui/latecomer".to_owned(), State::Idle));
        }
        record.observe((first + m) * 60_000, sample);
    }
    record.save(&home)?;
    println!("{}", home.display());
    // What the gate should find on the list. Checked against the list
    // here first, so a rule that keeps a row off it fails this fixture
    // by name instead of reaching the gate as a thin strip.
    let listed = node.sessions()?;
    for (id, _) in rows {
        if !listed.iter().any(|v| v.session.id.0 == id) {
            return Err(format!("{id} was imported and is not on the list"));
        }
        // `quiet` for the one the gate empties: it expects no band under
        // it, and a band that shows up there is the empty row coming back.
        println!("{} {id}", if id == "tui/quiet" { "quiet" } else { "row" });
    }
    Ok(())
}
