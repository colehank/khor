//! The words gate: exactly the six state words are covered, unknown
//! keys render instead of panicking, and the TS face really generates.

use khor_core::State;

#[test]
fn every_state_word_is_translated_and_unknown_keys_echo() {
    for s in State::ALL {
        let w = khor_catalog::state::word(s.key());
        assert_ne!(w, s.key(), "{} must map to a word, not echo", s.key());
        assert!(!w.is_ascii(), "{w} should be the word, not the key");
    }
    assert_eq!(khor_catalog::state::word("holodeck"), "holodeck");
}

#[test]
fn the_ts_face_is_generated_alongside() {
    let ts = include_str!(concat!(env!("OUT_DIR"), "/catalog.ts"));
    assert!(ts.contains("export const state"));
    assert!(ts.contains("export const cli"));
    assert!(ts.contains("export const msg"));
    assert!(ts.contains("${a0}"), "placeholders must become template args");
}
