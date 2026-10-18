//! A standalone experiment against the production ledger, not a replacement
//! ledger. No terminal, socket or persistent session is opened.
#![allow(dead_code)]
#[path = "../../../crates/node/src/modes.rs"]
mod modes;
#[path = "../../../crates/node/src/answers.rs"]
mod answers;
use std::time::{Duration, Instant};

fn main() {
    let dark = b"\x1b]10;rgb:eeee/eeee/e7e7\x1b\\\x1b]11;rgb:0000/0000/0000\x1b\\";
    let light = b"\x1b]10;rgb:1a1a/1a1a/1414\x1b\\\x1b]11;rgb:f4f4/f4f4/efef\x1b\\";
    let query = b"\x1b]10;?\x1b\\\x1b]11;?\x1b\\";
    for (label, first, second) in [("dark-first", dark.as_slice(), light.as_slice()), ("light-first", light.as_slice(), dark.as_slice())] {
        let mut ledger = answers::Ledger::default();
        let t = Instant::now();
        ledger.saw_output(query, t);
        assert_eq!(ledger.take_input(first, t), first);
        assert!(ledger.take_input(second, t + Duration::from_millis(10)).is_empty());
        let again = t + Duration::from_millis(20);
        ledger.saw_output(query, again);
        assert_eq!(ledger.fallback(again + answers::FALLBACK_AFTER), first);
        println!("{label}: first viewer reaches program; other theme is dropped; fallback keeps first colours");
    }
}
