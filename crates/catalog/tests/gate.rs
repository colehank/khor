//! The words gate: exactly the six state words are covered, every face
//! option a person can pick has a word (and every word names one),
//! unknown keys render instead of panicking, and the TS face really
//! generates.

use khor_core::avatar::{FaceShape, PRESETS, Variant};
use khor_core::{Hop, State};

#[test]
fn every_state_word_is_translated_and_unknown_keys_echo() {
    for s in State::ALL {
        let w = khor_catalog::state::word(s.key());
        assert_ne!(w, s.key(), "{} must map to a word, not echo", s.key());
        assert!(!w.is_ascii(), "{w} should be the word, not the key");
    }
    assert_eq!(khor_catalog::state::word("holodeck"), "holodeck");
}

/// **Every hop reading has a word, and every word names a reading.**
///
/// The second direction is the one that matters here. A word for a
/// reading khor no longer produces is invisible — the screen looks
/// complete while that row can never appear — and this table's whole
/// job is to make an unreadable machine say so rather than wear a
/// neighbour's word (`khor_core::Hop`).
#[test]
fn every_hop_has_a_word_and_every_word_names_a_hop() {
    for h in Hop::ALL {
        let w = khor_catalog::hop::word(h.key());
        assert_ne!(w, h.key(), "{} must map to a word, not echo", h.key());
        assert!(!w.is_ascii(), "{w} should be the word, not the key");
    }
    for key in khor_catalog::hop::KEYS {
        assert!(
            Hop::ALL.iter().any(|h| h.key() == key),
            "hop.{key} is a word for a reading khor cannot produce"
        );
    }
    // The keys are deliberately not `idle`/`unknown`: those exist in the
    // state and category tables, so a face looking a hop up in the wrong
    // one would find a real word and paint it. Here that mistake echoes.
    assert_eq!(khor_catalog::hop::word("idle"), "idle");
    assert_eq!(khor_catalog::hop::word("unknown"), "unknown");
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
        assert_ne!(
            khor_catalog::avatar::word(key),
            *key,
            "{key} must map to a word, not echo"
        );
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

/// **A Chinese sentence wears Chinese punctuation.**
///
/// 2026-09-28, from a full walk of the face: 「锁的是 app 窗口和网页脸,不锁
/// 命令行」, 「换了一台机器:别的机器…」, 「别人也读得了(权限 644)」 — ASCII
/// `,` `:` `(` inside Chinese, all over the catalog. Nobody chose it; the
/// file was typed on a keyboard in its English layout, and a half-width
/// comma between two hanzi is too small to see in review and plain to
/// see on screen.
///
/// **The rule is adjacency, not presence.** A zh string legitimately
/// carries ASCII punctuation wherever the text is not Chinese — a path,
/// a time (`12:00`), a flag, `khor agents add <名字> -- <命令...>`. What
/// is refused is an ASCII `, : ; ( ) ! ?` whose nearest neighbour on
/// either side, skipping whitespace and `{n}` placeholders, is a CJK
/// character (hanzi, CJK punctuation, or a full-width form). Placeholders
/// are skipped rather than counted as Latin because `连不上 {0}: {1}` is
/// a Chinese sentence whatever `{0}` turns out to hold.
///
/// Plus one pairing rule: a bracket opened in one width and closed in the
/// other is red, so a half-converted `（权限 {0})` cannot pass on the
/// strength of its first half.
///
/// Backtick spans are code and are not read.
#[test]
fn zh_sentences_use_full_width_punctuation() {
    // **Waived by name, with the reason.** Each is a template whose
    // ASCII separator belongs to the thing it shows, not to a sentence.
    const WAIVED: [(&str, &str); 1] = [(
        "gui.relays-hint",
        "a URL template: the `:` between host and port is the URL's own",
    )];
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/zh.toml");
    let doc: toml::Table = std::fs::read_to_string(path)
        .expect("zh.toml must exist")
        .parse()
        .expect("zh.toml must parse");
    let mut bad = Vec::new();
    let mut read = 0;
    for (section, table) in &doc {
        let Some(table) = table.as_table() else {
            continue;
        };
        for (key, v) in table {
            let Some(text) = v.as_str() else { continue };
            read += 1;
            let name = format!("{section}.{key}");
            let found = half_width_in_zh(text);
            if WAIVED.iter().any(|(k, _)| *k == name) {
                assert!(
                    !found.is_empty(),
                    "{name} is waived but has nothing to waive any more — delete its line"
                );
                continue;
            }
            for why in found {
                bad.push(format!("{name}: {why}"));
            }
        }
    }
    // The probe: a parse that walked nothing would pass by doing nothing.
    assert!(read > 500, "probe dead: only {read} strings read from zh.toml");
    assert!(
        bad.is_empty(),
        "{} half-width marks inside Chinese sentences in zh.toml (use ，：；（）！？):\n{}",
        bad.len(),
        bad.join("\n")
    );
}

/// The rule's own arm, run on sentences whose answer is known — without
/// it a checker that never fires passes the catalog test above.
#[test]
fn the_punctuation_rule_fires_and_holds_its_fire() {
    for red in [
        "锁的是 app 窗口和网页脸,不锁命令行",
        "换了一台机器:别的机器还认着旧的",
        "别人也读得了(权限 {0})",
        "连不上 {0}: {1}",
        "（权限 {0})",
        "真的吗?",
    ] {
        assert!(!half_width_in_zh(red).is_empty(), "should be red: {red:?}");
    }
    for green in [
        "锁的是 app 窗口和网页脸，不锁命令行",
        "别人也读得了（权限 {0}）",
        "每天 12:00 重置",
        "khor agents add <名字> -- <命令...>",
        "{0} · {1}",
        "用 `a, b` 分开",
        "Max 20x, Pro",
    ] {
        assert!(half_width_in_zh(green).is_empty(), "should be green: {green:?} — {:?}", half_width_in_zh(green));
    }
}

fn is_cjk(c: char) -> bool {
    matches!(c,
        '\u{3000}'..='\u{303F}'   // CJK symbols and punctuation: 、。「」
        | '\u{3400}'..='\u{4DBF}'
        | '\u{4E00}'..='\u{9FFF}'
        | '\u{F900}'..='\u{FAFF}'
        | '\u{FF00}'..='\u{FFEF}' // full-width forms: ，：；（）
    )
}

/// Every ASCII mark in `text` that sits against Chinese, described.
fn half_width_in_zh(text: &str) -> Vec<String> {
    let cs: Vec<char> = text.chars().collect();
    // Which chars are inside a backtick span (code) or a `{n}` placeholder.
    let mut code = vec![false; cs.len()];
    let mut inside = false;
    for (i, &c) in cs.iter().enumerate() {
        if c == '`' {
            inside = !inside;
            code[i] = true;
        } else {
            code[i] = inside;
        }
    }
    let mut slot = vec![false; cs.len()];
    for i in 0..cs.len() {
        if cs[i] == '{' && i + 2 < cs.len() && cs[i + 1].is_ascii_digit() && cs[i + 2] == '}' {
            slot[i..=i + 2].fill(true);
        }
    }
    // The nearest meaningful neighbour, stepping over whitespace and
    // placeholders.
    let near = |from: usize, back: bool| -> Option<char> {
        let mut j = from as isize;
        loop {
            j += if back { -1 } else { 1 };
            if j < 0 || j as usize >= cs.len() {
                return None;
            }
            let k = j as usize;
            if cs[k].is_whitespace() || slot[k] {
                continue;
            }
            return Some(cs[k]);
        }
    };
    let mut out = Vec::new();
    let mut open: Vec<char> = Vec::new();
    for (i, &c) in cs.iter().enumerate() {
        if code[i] {
            continue;
        }
        match c {
            '(' | '（' => open.push(c),
            ')' | '）' => {
                if let Some(o) = open.pop() {
                    if (o == '(') != (c == ')') {
                        out.push(format!("{o:?} closed by {c:?}"));
                    }
                }
            }
            _ => {}
        }
        if !matches!(c, ',' | ':' | ';' | '(' | ')' | '!' | '?') {
            continue;
        }
        let (l, r) = (near(i, true), near(i, false));
        if l.is_some_and(is_cjk) || r.is_some_and(is_cjk) {
            let at: String = cs[i.saturating_sub(6)..(i + 7).min(cs.len())].iter().collect();
            out.push(format!("{c:?} in …{at}…"));
        }
    }
    out
}

#[test]
fn the_ts_face_is_generated_alongside() {
    let ts = include_str!(concat!(env!("OUT_DIR"), "/catalog.ts"));
    assert!(ts.contains("export const state"));
    assert!(ts.contains("export const cli"));
    assert!(ts.contains("export const msg"));
    assert!(ts.contains("export const avatar"));
    assert!(
        ts.contains("${a0}"),
        "placeholders must become template args"
    );
}
