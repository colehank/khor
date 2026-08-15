//! The wording catalog (docs/UX.md 文案): zh.toml in this crate is the
//! one place user-facing words live. This is its build-time Rust face;
//! `gen_ts` prints the TS face for the GUI build. Keys are stable and
//! travel the wire; the words are looked up at the last moment — no
//! other crate spells a user-facing word in code.

include!(concat!(env!("OUT_DIR"), "/catalog.rs"));
