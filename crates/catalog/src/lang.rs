//! Which language this process is speaking (docs/MORE.md 语言).
//!
//! # Two, and the fallback is English
//!
//! 「自动跟随系统语言，我们支持英语与中文。失败会退英文」 (the user,
//! 2026-09-19). So a system that says `zh*` gets Chinese and **everything
//! else gets English** — including a system that says nothing at all.
//! There is no third answer and no partial one: `build.rs` refuses to
//! compile a key that exists in only one of the two files, so every
//! sentence khor can say exists in both.
//!
//! # A process-wide switch, set once
//!
//! Every word is looked up at the moment it is printed
//! (`khor_catalog`'s generated functions), so this is read on every one
//! of them — an atomic, never a lock. Nothing here decides *what* the
//! language should be beyond the environment: the stored preference is
//! `khor config`'s, which lives a crate away, and
//! `khor_node::config::apply_lang` is where the two meet.

use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Zh,
    En,
}

/// **Chinese until somebody says otherwise**, because zh.toml is the
/// source and a process that never resolved anything (a test, a
/// `build.rs`) should read as the file it is checking.
static SPEAKING: AtomicU8 = AtomicU8::new(0);

pub fn current() -> Lang {
    if SPEAKING.load(Ordering::Relaxed) == 0 {
        Lang::Zh
    } else {
        Lang::En
    }
}

pub fn set(lang: Lang) {
    SPEAKING.store(u8::from(lang == Lang::En), Ordering::Relaxed);
}

/// Whether a locale name asks for Chinese. **`zh` and anything under
/// it** — `zh`, `zh_CN`, `zh-Hant`, `zh_TW.UTF-8` — and nothing else:
/// the ruling is 「非 `zh*` 一律英文」, so this is a prefix test rather
/// than a list of regions khor would have to keep up with.
fn is_zh(tag: &str) -> bool {
    let tag = tag.trim();
    let head = tag.split(['.', '@']).next().unwrap_or(tag);
    head == "zh" || head.starts_with("zh-") || head.starts_with("zh_")
}

/// What this machine's environment asks for.
///
/// **In POSIX's own order** — `LC_ALL` overrides everything, `LC_MESSAGES`
/// is the category that names *words*, `LANG` is the default — and an
/// empty value is skipped rather than read as an answer, which is what
/// the shells themselves do.
///
/// `C` and `POSIX` are not a language; they are 「no locale」, and they
/// fall through to English like every other non-zh answer.
pub fn from_env() -> Lang {
    for name in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        match std::env::var(name) {
            Ok(v) if !v.trim().is_empty() => {
                return if is_zh(&v) { Lang::Zh } else { Lang::En };
            }
            _ => {}
        }
    }
    // **macOS sets none of the three in a GUI session**, and often not
    // in a terminal either: the language lives in the user's defaults,
    // where `AppleLanguages` is an ordered list and the first entry is
    // the one the system is speaking. Asked through `defaults` rather
    // than by linking Foundation, because this crate has no dependencies
    // and taking one for a single string on one platform would put it in
    // every build of every binary.
    //
    // Anything that goes wrong — no such tool, a refusal, unreadable
    // output — is 「读不到」, which the ruling already answers: English.
    #[cfg(target_os = "macos")]
    if let Ok(out) = std::process::Command::new("defaults")
        .args(["read", "-g", "AppleLanguages"])
        .output()
        && out.status.success()
        && let Ok(text) = String::from_utf8(out.stdout)
    {
        // `( "zh-Hans-CN", "en-US" )` — the first quoted tag is the
        // answer. Read by scanning for the first quote pair rather than
        // by parsing a plist: what is needed is one tag, and a parser
        // for a format nothing else here reads is a second thing to
        // keep working.
        if let Some(rest) = text.split_once('"')
            && let Some((first, _)) = rest.1.split_once('"')
        {
            return if is_zh(first) { Lang::Zh } else { Lang::En };
        }
    }
    Lang::En
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **`zh` and everything under it, and nothing else.** The refusals
    /// matter as much as the acceptances: the ruling is 「非 zh* 一律
    /// 英文」, so a locale khor does not know must not be guessed at.
    #[test]
    fn a_locale_is_chinese_only_when_it_says_zh() {
        for yes in [
            "zh",
            "zh_CN",
            "zh_CN.UTF-8",
            "zh-Hans",
            "zh-Hant-TW",
            "zh_TW.Big5@pinyin",
            "  zh_CN.UTF-8  ",
        ] {
            assert!(is_zh(yes), "{yes} asks for Chinese");
        }
        for no in [
            "en_US.UTF-8",
            "C",
            "POSIX",
            "",
            "ja_JP",
            // Not a prefix match on the bare letters: a made-up locale
            // that merely starts with those two letters is not Chinese.
            "zhu_XX",
            "zhuang",
        ] {
            assert!(!is_zh(no), "{no} does not ask for Chinese");
        }
    }
}
