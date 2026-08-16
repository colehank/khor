//! Turns zh.toml into two generated faces: catalog.rs (compiled into
//! this crate) and catalog.ts (printed by `gen_ts` for the GUI build).
//! One text per name, enforced here: refusing by name only works while
//! names have distinct texts — a duplicate is a build error, not a
//! runtime surprise.

use std::collections::BTreeMap;
use std::fmt::Write as _;

fn main() {
    println!("cargo::rerun-if-changed=zh.toml");
    let text = std::fs::read_to_string("zh.toml").expect("zh.toml must exist");
    let doc: toml::Table = text.parse().expect("zh.toml must parse");
    let mut rs = String::from("// Generated from zh.toml by build.rs - do not edit.\n");
    let mut ts = String::from("// Generated from zh.toml by khor-catalog - do not edit.\n");
    for section in ["state", "category", "avatar", "cli", "msg", "gui"] {
        let table = doc[section].as_table().expect("each section must be a table");
        let mut texts: BTreeMap<&str, &str> = BTreeMap::new();
        for (k, v) in table {
            let v = v.as_str().expect("every value must be a string");
            if let Some(prev) = texts.insert(v, k) {
                panic!(
                    "{section}.{k} repeats the text of {section}.{prev} - \
                     two names, one text; refusal by name needs distinct texts"
                );
            }
        }
        gen_rust(&mut rs, section, table);
        gen_ts(&mut ts, section, table);
    }
    let states = doc["state"].as_table().unwrap();
    assert_eq!(states.len(), 6, "state must hold exactly the six words");
    let out = std::env::var("OUT_DIR").unwrap();
    std::fs::write(format!("{out}/catalog.rs"), rs).unwrap();
    std::fs::write(format!("{out}/catalog.ts"), ts).unwrap();
}

/// Placeholders are {0}..{8}, contiguous from zero.
fn arg_count(v: &str) -> usize {
    (0..9).take_while(|i| v.contains(&format!("{{{i}}}"))).count()
}

fn gen_rust(rs: &mut String, section: &str, table: &toml::Table) {
    writeln!(rs, "pub mod {section} {{").unwrap();
    for (k, v) in table {
        let v = v.as_str().unwrap();
        let n = arg_count(v);
        if n == 0 {
            let name = k.replace('-', "_").to_uppercase();
            writeln!(rs, "    pub const {name}: &str = {v:?};").unwrap();
        } else {
            let name = k.replace('-', "_");
            let args: Vec<String> =
                (0..n).map(|i| format!("a{i}: impl ::std::fmt::Display")).collect();
            let mut fmt = v.replace('{', "{{").replace('}', "}}");
            for i in 0..n {
                fmt = fmt.replace(&format!("{{{{{i}}}}}"), &format!("{{a{i}}}"));
            }
            writeln!(
                rs,
                "    pub fn {name}({}) -> String {{ format!({fmt:?}) }}",
                args.join(", ")
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
    // avatar option — get a lookup and the list of keys behind it. The
    // list is what lets a gate walk the section in the other direction
    // ("every word here names something that exists"); without it a word
    // for an option the code no longer produces stays green forever.
    if section == "state" || section == "category" || section == "avatar" {
        write!(rs, "    pub fn word(key: &str) -> &str {{ match key {{ ").unwrap();
        for (k, v) in table {
            write!(rs, "{:?} => {:?}, ", k, v.as_str().unwrap()).unwrap();
        }
        writeln!(rs, "other => other }} }}").unwrap();
        write!(rs, "    pub const KEYS: [&str; {}] = [", table.len()).unwrap();
        for k in table.keys() {
            write!(rs, "{k:?}, ").unwrap();
        }
        writeln!(rs, "];").unwrap();
    }
    writeln!(rs, "}}").unwrap();
}

fn gen_ts(ts: &mut String, section: &str, table: &toml::Table) {
    writeln!(ts, "export const {section} = {{").unwrap();
    for (k, v) in table {
        let v = v.as_str().unwrap();
        let n = arg_count(v);
        let name = k.replace('-', "_");
        if n == 0 {
            writeln!(ts, "  {name}: {v:?},").unwrap();
        } else {
            let args: Vec<String> = (0..n).map(|i| format!("a{i}: string | number")).collect();
            let mut body = v.replace('\\', "\\\\").replace('`', "\\`").replace("${", "\\${");
            for i in 0..n {
                body = body.replace(&format!("{{{i}}}"), &format!("${{a{i}}}"));
            }
            writeln!(ts, "  {name}: ({}) => `{body}`,", args.join(", ")).unwrap();
        }
    }
    writeln!(ts, "}} as const;").unwrap();
}
