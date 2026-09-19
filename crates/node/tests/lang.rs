//! 语言 (docs/MORE.md 语言): which of the two khor speaks, and the whole
//! ladder that decides it.
//!
//! # Why this is its own binary, and one test
//!
//! `khor_catalog::lang` is a **process-wide** switch — every word is
//! looked up through it at the moment it is printed, so it has to be
//! one thing per process. That makes it the one piece of state a test
//! cannot touch without touching every other test in the same binary:
//! `khor-node`'s unit tests assert on Chinese sentences, and a test that
//! flipped this under them would fail them at random.
//!
//! So: its own integration binary (its own process), and **one test**
//! rather than several, because tests inside one binary run in parallel
//! with each other and would have the same race among themselves.

use khor_catalog::lang::{self, Lang};

/// A scratch home that takes itself away however the test leaves — the
/// shape the rest of this tree keeps (`tests.rs`'s `Scratch`), so a red
/// run does not leave a directory in `$TMPDIR` every time it is red.
struct Scratch(std::path::PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn scratch(tag: &str) -> Scratch {
    let p = std::env::temp_dir().join(format!("khor-lang-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    Scratch(p)
}

/// **The whole ladder, in the order the ruling puts it**: a stored
/// choice wins; `auto` asks the environment; anything that is not `zh*`
/// — including a machine that says nothing at all — is English.
///
/// Every rung is checked **through a word on a screen**, never by
/// reading the switch back: `lang::current()` returning `En` proves the
/// atomic moved, and proves nothing about whether any sentence khor
/// prints followed it. `state::busy()` is one of the six words, so it is
/// the shortest thing that is a real answer.
#[test]
fn the_language_ladder_reaches_the_words() {
    let r = scratch("ladder");
    let root = r.0.clone();

    // The probe, before anything else. Two languages that came back the
    // same string would make every assertion below agree with itself.
    lang::set(Lang::Zh);
    let zh = khor_catalog::state::busy();
    lang::set(Lang::En);
    let en = khor_catalog::state::busy();
    assert_ne!(zh, en, "probe dead: 忙碌 reads the same in both languages");
    assert!(!zh.is_ascii(), "the Chinese word should not be ascii: {zh}");
    assert!(en.is_ascii(), "the English word should be ascii: {en}");

    // ── 1) an explicit choice wins over the environment ───────────
    //
    // Set against the *opposite* environment each way round, which is
    // the only arrangement that tells 「the store was read」 apart from
    // 「the environment happened to agree」.
    unsafe { std::env::set_var("LC_ALL", "en_US.UTF-8") };
    khor_node::config::set(&root, "lang", "zh").unwrap();
    khor_node::config::apply_lang(&root);
    assert_eq!(
        khor_catalog::state::busy(),
        zh,
        "an English machine with `lang zh` stored must still speak Chinese"
    );

    unsafe { std::env::set_var("LC_ALL", "zh_CN.UTF-8") };
    khor_node::config::set(&root, "lang", "en").unwrap();
    khor_node::config::apply_lang(&root);
    assert_eq!(
        khor_catalog::state::busy(),
        en,
        "a Chinese machine with `lang en` stored must still speak English"
    );

    // ── 2) `auto` asks the environment, in POSIX's own order ──────
    khor_node::config::set(&root, "lang", "auto").unwrap();
    for (var, value, want, why) in [
        ("LC_ALL", "zh_CN.UTF-8", zh, "LC_ALL names Chinese"),
        ("LC_ALL", "en_GB.UTF-8", en, "LC_ALL names English"),
        // A tag with no region, and one with a script — both are zh*.
        ("LC_ALL", "zh", zh, "a bare zh is Chinese"),
        ("LC_ALL", "zh-Hant-TW", zh, "a script subtag is still zh*"),
        // …and the ones that are not a language at all.
        ("LC_ALL", "C", en, "C is no locale, so English"),
        ("LC_ALL", "POSIX", en, "POSIX is no locale, so English"),
        // **Not a prefix match on two letters**: a real locale that
        // merely starts with them is not Chinese.
        ("LC_ALL", "zhu_XX", en, "zhu is not zh"),
        // Anything khor has no words for falls to English rather than
        // to the source language — 「非 zh* 一律英文」.
        (
            "LC_ALL",
            "ja_JP.UTF-8",
            en,
            "an unsupported language is English",
        ),
    ] {
        unsafe { std::env::set_var(var, value) };
        khor_node::config::apply_lang(&root);
        assert_eq!(khor_catalog::state::busy(), want, "{why} ({value})");
    }

    // `LC_ALL` first, then `LC_MESSAGES`, then `LANG` — and an **empty**
    // value is skipped rather than read as an answer, which is what the
    // shells themselves do.
    unsafe {
        std::env::set_var("LC_ALL", "");
        std::env::set_var("LC_MESSAGES", "zh_CN.UTF-8");
        std::env::set_var("LANG", "en_US.UTF-8");
    }
    khor_node::config::apply_lang(&root);
    assert_eq!(
        khor_catalog::state::busy(),
        zh,
        "an empty LC_ALL is not an answer; LC_MESSAGES is the next one asked"
    );
    unsafe {
        std::env::set_var("LC_MESSAGES", "");
        std::env::set_var("LANG", "zh_TW.UTF-8");
    }
    khor_node::config::apply_lang(&root);
    assert_eq!(
        khor_catalog::state::busy(),
        zh,
        "LANG is the last one asked"
    );

    // ── 3) a store nobody can read is `auto`, not a refusal ───────
    //
    // The direction is chosen rather than inherited (`lock::stored`'s
    // own reason): a machine whose store is broken must still be able to
    // say so, in whatever language its environment asks for.
    let broken = scratch("broken");
    std::fs::create_dir_all(broken.0.join(".khor")).unwrap();
    std::fs::write(broken.0.join(".khor").join("config.json"), b"{not json").unwrap();
    unsafe { std::env::set_var("LANG", "en_US.UTF-8") };
    khor_node::config::apply_lang(&broken.0);
    assert_eq!(
        khor_catalog::state::busy(),
        en,
        "an unreadable store falls through to the environment"
    );

    // ── 4) …and a machine that says nothing at all speaks English ─
    //
    // The last rung, and the one the ruling is most explicit about:
    // 「失败会退英文」. On macOS `from_env` asks the system defaults
    // before giving up, so this asserts the *result* rather than the
    // road — a Mac set to Chinese answers Chinese here, which is the
    // right answer for that machine and why this arm checks that khor
    // picked one of the two rather than neither.
    unsafe {
        std::env::remove_var("LC_ALL");
        std::env::remove_var("LC_MESSAGES");
        std::env::remove_var("LANG");
    }
    khor_node::config::apply_lang(&broken.0);
    let bare = khor_catalog::state::busy();
    assert!(
        bare == zh || bare == en,
        "with no locale anywhere khor must still speak one of the two, not echo a key: {bare}"
    );
    if !cfg!(target_os = "macos") {
        assert_eq!(
            bare, en,
            "with no locale and nowhere else to ask, the fallback is English"
        );
    }
}
