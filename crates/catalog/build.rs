//! Turns **zh.toml and en.toml** into two generated faces: catalog.rs
//! (compiled into this crate) and catalog.ts (printed by `gen_ts` for
//! the GUI build).
//!
//! # Two files, and the alignment is a build error
//!
//! 「回退是英文」 (docs/MORE.md 语言): a system language that is not zh*
//! gets English, and a key missing from en.toml would fall back to…
//! English, which is the thing that is missing. There is no third place
//! to fall back to, so **a gap is refused at compile time** rather than
//! discovered on somebody's screen as a Chinese sentence in an English
//! app. Three rules, all checked here:
//!
//! 1. **The key sets match**, per section. Adding a line to zh.toml
//!    without adding one to en.toml does not build.
//! 2. **Each entry's placeholders match**, as a set and contiguous from
//!    zero. `{0} 分钟` translated as `minutes` loses the number
//!    silently; here it does not build. Non-contiguous is refused in
//!    both files, because `{2}` with no `{0}` is a typo that renders as
//!    literal braces.
//! 3. **One text per name, within a section, in each file.** Refusing by
//!    name only works while names have distinct texts — and the rule has
//!    to hold in the translation too, or two keys collapse into one
//!    sentence for English readers only.
//!
//! # What the generated code looks like, and why nothing is a `const`
//!
//! A `const` cannot change with the language, so **every entry is a
//! function** — `cli::doctor_ok()` became `cli::doctor_ok()`. The ones
//! that take arguments were already functions and their call sites did
//! not move; the wordless ones cost one rename each across the tree,
//! paid once (批 D3).
//!
//! `KEYS` stays a `const` array: those are keys, not words, and they do
//! not translate.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

/// In the order they are generated. The four whose keys are *data* get a
/// `word()` lookup and a `KEYS` list as well (see `gen_rust`).
const SECTIONS: [&str; 8] = [
    "state", "category", "hop", "avatar", "config", "cli", "msg", "gui",
];

fn main() {
    println!("cargo::rerun-if-changed=zh.toml");
    println!("cargo::rerun-if-changed=en.toml");
    let zh = read("zh.toml");
    let en = read("en.toml");
    let mut rs = String::from("// Generated from zh.toml + en.toml by build.rs - do not edit.\n");
    let mut ts =
        String::from("// Generated from zh.toml + en.toml by khor-catalog - do not edit.\n");
    ts.push_str(TS_HEAD);
    for section in SECTIONS {
        let z = table(&zh, section, "zh.toml");
        let e = table(&en, section, "en.toml");
        align(section, z, e);
        gen_rust(&mut rs, section, z, e);
        gen_ts(&mut ts, section, z, e);
    }
    assert_eq!(
        table(&zh, "state", "zh.toml").len(),
        6,
        "state must hold exactly the six words"
    );
    let out = std::env::var("OUT_DIR").unwrap();
    std::fs::write(format!("{out}/catalog.rs"), rs).unwrap();
    std::fs::write(format!("{out}/catalog.ts"), ts).unwrap();
}

fn read(file: &str) -> toml::Table {
    std::fs::read_to_string(file)
        .unwrap_or_else(|e| panic!("{file} must exist: {e}"))
        .parse()
        .unwrap_or_else(|e| panic!("{file} must parse: {e}"))
}

fn table<'a>(doc: &'a toml::Table, section: &str, file: &str) -> &'a toml::Table {
    doc.get(section)
        .unwrap_or_else(|| panic!("{file} has no [{section}] section"))
        .as_table()
        .unwrap_or_else(|| panic!("{file}'s [{section}] must be a table"))
}

fn text<'a>(table: &'a toml::Table, key: &str, section: &str, file: &str) -> &'a str {
    table[key]
        .as_str()
        .unwrap_or_else(|| panic!("{file}'s {section}.{key} must be a string"))
}

/// Which `{i}` this text carries. A `BTreeSet` rather than a count,
/// because 「{0} 之后」 and 「after {1}」 are the same *count* and two
/// different sentences.
fn slots(v: &str) -> BTreeSet<usize> {
    (0..9).filter(|i| v.contains(&format!("{{{i}}}"))).collect()
}

/// How many arguments this entry's function takes. Contiguous from zero
/// — [`align`] refuses anything else, so this is the set's size.
fn arg_count(v: &str) -> usize {
    slots(v).len()
}

/// The three rules, for one section. Every panic here is a build error
/// that names the file, the key and what is wrong with it: this message
/// is the whole user interface of the rule.
fn align(section: &str, zh: &toml::Table, en: &toml::Table) {
    for k in zh.keys() {
        let v = text(zh, k, section, "zh.toml");
        if !en.contains_key(k) {
            panic!(
                "en.toml has no {section}.{k} - zh.toml says {v:?}. \
                 Every key needs both: a missing translation would fall back to English, \
                 which is the thing that is missing (docs/MORE.md 语言)"
            );
        }
        let (zs, es) = (slots(v), slots(text(en, k, section, "en.toml")));
        let want: BTreeSet<usize> = (0..zs.len()).collect();
        if zs != want {
            panic!(
                "zh.toml's {section}.{k} has placeholders {zs:?} - they must be {{0}}..{{n}} \
                 with no gaps, or the odd one renders as literal braces"
            );
        }
        if zs != es {
            panic!(
                "{section}.{k} carries {zs:?} in zh.toml and {es:?} in en.toml - \
                 a translation that drops a placeholder loses the value it was carrying, \
                 silently"
            );
        }
    }
    for k in en.keys() {
        if !zh.contains_key(k) {
            panic!(
                "zh.toml has no {section}.{k} - zh.toml is the source, so a key that exists \
                 only in the translation is a word nothing can ever ask for"
            );
        }
    }
    for (file, t) in [("zh.toml", zh), ("en.toml", en)] {
        let mut texts: BTreeMap<&str, &str> = BTreeMap::new();
        for k in t.keys() {
            let v = text(t, k, section, file);
            if let Some(prev) = texts.insert(v, k) {
                panic!(
                    "{file}'s {section}.{k} repeats the text of {section}.{prev} - \
                     two names, one text; refusal by name needs distinct texts"
                );
            }
        }
    }
}

/// The `match` that picks a language, around whatever the two arms
/// produce. Written once here so every generated item spells it the same
/// way.
fn pick(zh: &str, en: &str) -> String {
    format!(
        "match crate::lang::current() {{ \
         crate::lang::Lang::Zh => {zh}, crate::lang::Lang::En => {en} }}"
    )
}

fn gen_rust(rs: &mut String, section: &str, zh: &toml::Table, en: &toml::Table) {
    writeln!(rs, "pub mod {section} {{").unwrap();
    for k in zh.keys() {
        let z = text(zh, k, section, "zh.toml");
        let e = text(en, k, section, "en.toml");
        let n = arg_count(z);
        let name = k.replace('-', "_");
        if n == 0 {
            // **A function, not a const**: a const cannot change with
            // the language, and this is the whole cost of 批 D3.
            writeln!(
                rs,
                "    pub fn {name}() -> &'static str {{ {} }}",
                pick(&format!("{z:?}"), &format!("{e:?}"))
            )
            .unwrap();
        } else {
            let args: Vec<String> = (0..n)
                .map(|i| format!("a{i}: impl ::std::fmt::Display"))
                .collect();
            let fmt = |v: &str| {
                let mut s = v.replace('{', "{{").replace('}', "}}");
                for i in 0..n {
                    s = s.replace(&format!("{{{{{i}}}}}"), &format!("{{a{i}}}"));
                }
                format!("format!({s:?})")
            };
            writeln!(
                rs,
                "    pub fn {name}({}) -> String {{ {} }}",
                args.join(", "),
                pick(&fmt(z), &fmt(e))
            )
            .unwrap();
        }
    }
    // key -> word; unknown keys echo back. An old client meeting a new
    // kind's word must render something, never panic — and for
    // categories the echo is load-bearing rather than a safety net:
    // vendor names (`claude`, `codex`) are proper nouns with no entry
    // here on purpose, so echoing them *is* the translation.
    // The sections whose keys are *data* — a state word, a category, an
    // avatar option, a setting — get a lookup and the list of keys
    // behind it. The list is what lets a gate walk the section in the
    // other direction ("every word here names something that exists");
    // without it a word for an option the code no longer produces stays
    // green forever.
    if matches!(section, "state" | "category" | "hop" | "avatar" | "config") {
        let arm = |t: &toml::Table, file: &str| {
            let mut s = String::from("match key { ");
            for k in t.keys() {
                write!(s, "{:?} => {:?}, ", k, text(t, k, section, file)).unwrap();
            }
            s.push_str("other => other }");
            s
        };
        writeln!(
            rs,
            "    pub fn word(key: &str) -> &str {{ {} }}",
            pick(&arm(zh, "zh.toml"), &arm(en, "en.toml"))
        )
        .unwrap();
        // **Keys, not words**: this list does not translate, so it stays
        // a `const` and every gate that walks it keeps working.
        write!(rs, "    pub const KEYS: [&str; {}] = [", zh.len()).unwrap();
        for k in zh.keys() {
            write!(rs, "{k:?}, ").unwrap();
        }
        writeln!(rs, "];").unwrap();
    }
    writeln!(rs, "}}").unwrap();
}

/// The TS side's one moving part, printed once at the top of the file.
///
/// **Call sites do not move on this side.** `gui.settings` is a property
/// read, and a `Proxy` answers it out of whichever table the face is
/// currently speaking — so the 341 wordless entries stay properties and
/// nothing in `src/` changes shape.
///
/// ⚠️ **A property read is not reactive.** A module-level `const` that
/// reads one captures that language for the life of the page; the ten
/// that did were turned into functions in 批 D3, and `check-type` keeps
/// them that way.
const TS_HEAD: &str = r#"
/** Which of the two this face is speaking. Set before anything renders
    (`src/lang.ts`), and again when 设置 changes it. */
let SPEAKING: "zh" | "en" = "zh";
export function speaking(): "zh" | "en" {
  return SPEAKING;
}
export function speak(lang: "zh" | "en"): void {
  SPEAKING = lang;
}
/** One section, answered out of whichever table is current. The two are
    the same shape — `build.rs` refuses to emit them otherwise — so the
    zh one carries the types. */
function pick<T extends object>(zh: T, en: T): T {
  return new Proxy(zh, {
    get: (_t, k) => (SPEAKING === "zh" ? zh : en)[k as keyof T],
  });
}
"#;

fn gen_ts(ts: &mut String, section: &str, zh: &toml::Table, en: &toml::Table) {
    let body = |t: &toml::Table, file: &str, out: &mut String| {
        for k in zh.keys() {
            let v = text(t, k, section, file);
            let n = arg_count(text(zh, k, section, "zh.toml"));
            let name = k.replace('-', "_");
            if n == 0 {
                writeln!(out, "  {name}: {v:?},").unwrap();
            } else {
                let args: Vec<String> = (0..n).map(|i| format!("a{i}: string | number")).collect();
                let mut lit = v
                    .replace('\\', "\\\\")
                    .replace('`', "\\`")
                    .replace("${", "\\${");
                for i in 0..n {
                    lit = lit.replace(&format!("{{{i}}}"), &format!("${{a{i}}}"));
                }
                writeln!(out, "  {name}: ({}) => `{lit}`,", args.join(", ")).unwrap();
            }
        }
    };
    // **No `as const`.** With two tables the value at a given key is one
    // of two strings, and a literal type would let TS narrow a
    // comparison to an answer that is only right in Chinese.
    writeln!(ts, "const zh_{section} = {{").unwrap();
    body(zh, "zh.toml", ts);
    writeln!(ts, "}};").unwrap();
    // Typed against the Chinese table, which is a structural check tsc
    // does for free on top of `build.rs`'s own.
    writeln!(ts, "const en_{section}: typeof zh_{section} = {{").unwrap();
    body(en, "en.toml", ts);
    writeln!(ts, "}};").unwrap();
    writeln!(
        ts,
        "export const {section} = pick(zh_{section}, en_{section});"
    )
    .unwrap();
}
