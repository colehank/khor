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

use std::path::PathBuf;

use khor_core::{SessionId, State};
use khor_node::day;
use khor_node::live::LiveKind;
use khor_node::Node;

fn main() -> Result<(), String> {
    let home = PathBuf::from(
        std::env::args().nth(1).ok_or("usage: day_fixture <home>")?,
    );
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).map_err(|e| e.to_string())?;

    let node = Node::open_as(home.clone(), "alpha")?;
    let live = LiveKind::new(home.clone(), node.device());
    let rows = [
        ("tui/agent", "an agent that worked all day"),
        ("tui/crashed", "an agent that died at noon"),
        ("tui/latecomer", "something that started six hours ago"),
    ];
    for (id, title) in rows {
        live.register(&SessionId(id.to_owned()), "tui", title, Some(std::process::id()), None)?;
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
        // Two hours in the middle with no resident running at all.
        if (600..720).contains(&m) {
            continue;
        }
        let mut sample: Vec<(String, State)> = Vec::new();
        sample.push((
            "tui/agent".to_owned(),
            match m {
                240..=300 => State::Blocked,
                900..=960 => State::Done,
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
    Ok(())
}
