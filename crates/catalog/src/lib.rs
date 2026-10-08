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
mod reword;

pub use reword::{read, reword};

include!(concat!(env!("OUT_DIR"), "/catalog.rs"));

/// **Counted words, singular where the language has one** (`[one]`):
/// English says 「1 minute」, not 「1 minutes」, and Chinese says both the
/// same. Every caller that prints a count of time goes through here.
pub mod count {
    use std::fmt::Display;

    fn pick<N: Display + PartialEq + From<u8>>(n: N, one: fn(N) -> String, many: fn(N) -> String) -> String {
        if n == N::from(1) { one(n) } else { many(n) }
    }

    pub fn seconds(n: u64) -> String {
        pick(n, |n| crate::one::second(n), |n| crate::cli::age_seconds(n))
    }
    pub fn minutes(n: u64) -> String {
        pick(n, |n| crate::one::minute(n), |n| crate::cli::age_minutes(n))
    }
    pub fn hours(n: u64) -> String {
        pick(n, |n| crate::one::hour(n), |n| crate::cli::age_hours(n))
    }
    pub fn days(n: u64) -> String {
        pick(n, |n| crate::one::day(n), |n| crate::cli::age_days(n))
    }
    pub fn up_days(n: u64) -> String {
        pick(n, |n| crate::one::up_day(n), |n| crate::cli::uptime_days(n))
    }
    pub fn up_hours(n: u64) -> String {
        pick(n, |n| crate::one::up_hour(n), |n| crate::cli::uptime_hours(n))
    }
    pub fn lock_idle_after(n: u64) -> String {
        pick(n, |n| crate::one::lock_idle_after(n), |n| crate::cli::lock_idle_after(n))
    }

    #[cfg(test)]
    mod tests {
        #[test]
        fn one_is_singular_in_english_and_the_same_in_chinese() {
            crate::lang::set(crate::lang::Lang::En);
            assert_eq!(super::minutes(1), "1 minute");
            assert_eq!(super::minutes(2), "2 minutes");
            assert_eq!(super::hours(1), "1 hour");
            assert_eq!(super::days(1), "1 day");
            crate::lang::set(crate::lang::Lang::Zh);
            assert_eq!(super::minutes(1), "1 分钟");
            assert_eq!(super::minutes(2), "2 分钟");
        }
    }
}
