//! The words gate: exactly the six state words are covered, every face
//! option a person can pick has a word (and every word names one),
//! unknown keys render instead of panicking, and the TS face really
//! generates.

use khor_core::avatar::{FaceShape, Variant, PRESETS};
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

/// **Every face option has a word, and every word here names an option
/// that exists.**
///
/// The two directions fail differently, and only one of them is visible.
/// A missing word paints the bare key on a button — `okabe` where
/// Okabe-Ito belongs — which is ugly enough that somebody reports it. A
/// word for an option the code no longer produces is **invisible**: the
/// screen looks complete while that row is unreachable, and the wording
/// file goes on describing a palette khor cannot paint.
///
/// The `axis-` prefix separates a heading from an option; that rule is
/// written in zh.toml beside the keys, and it is a prefix rather than a
/// guess about dashes because a palette id may carry one.
#[test]
fn every_face_option_has_a_word_and_every_word_names_an_option() {
    let options: Vec<&str> = PRESETS
        .iter()
        .map(|p| p.id)
        .chain(Variant::ALL.iter().map(|v| v.key()))
        .chain(FaceShape::ALL.iter().map(|s| s.key()))
        .collect();
    for key in &options {
        // `word` echoes what it does not know, so an echo *is* the
        // missing entry.
        assert_ne!(khor_catalog::avatar::word(key), *key, "{key} must map to a word, not echo");
    }
    for key in khor_catalog::avatar::KEYS {
        if key.starts_with("axis-") {
            continue;
        }
        assert!(
            options.contains(&key),
            "{key} has a word but is not an option anyone can pick"
        );
    }
    assert_eq!(khor_catalog::avatar::word("holodeck"), "holodeck");
}

#[test]
fn the_ts_face_is_generated_alongside() {
    let ts = include_str!(concat!(env!("OUT_DIR"), "/catalog.ts"));
    assert!(ts.contains("export const state"));
    assert!(ts.contains("export const cli"));
    assert!(ts.contains("export const msg"));
    assert!(ts.contains("export const avatar"));
    assert!(ts.contains("${a0}"), "placeholders must become template args");
}
