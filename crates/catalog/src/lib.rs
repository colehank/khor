//! The wording catalog (docs/UX.md 文案): **zh.toml and en.toml** in
//! this crate are the one place user-facing words live. This is their
//! build-time Rust face; `gen_ts` prints the TS face for the GUI build.
//! Keys are stable and travel the wire; the words are looked up at the
//! last moment — no other crate spells a user-facing word in code.
//!
//! **Every entry is a function**, including the wordless ones: a `const`
//! cannot change with the language (批 D3, `build.rs` has the rest).
//! `KEYS` is the exception and stays a `const` array — those are keys.

pub mod lang;

include!(concat!(env!("OUT_DIR"), "/catalog.rs"));
