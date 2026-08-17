//! patterns.toml → Rust, at build time.
//!
//! The generation itself is dull. The three checks around it are not,
//! and each one exists because the failure it catches is **silent**: a
//! table that stops recognising a vendor does not crash, does not warn,
//! and does not show up in a screenshot — it just quietly answers 空闲
//! forever, which is what this whole batch exists to stop happening.
//!
//! 1. **Every regex must compile.** A bad pattern would otherwise be a
//!    panic on a user's machine the first time an agent is opened,
//!    inside a host process nobody is watching.
//! 2. **Every `{name}` must name a real charset.** A typo would
//!    otherwise be a rule that matches a literal brace and can never
//!    fire.
//! 3. **A folded `contains` pattern must already be lowercase.** These
//!    are matched against a lowercased screen, so an uppercase letter in
//!    the pattern makes the rule dead — and dead is invisible. This one
//!    is checked for `contains` only: a regex may legitimately hold an
//!    uppercase escape (`\S`, `\d`) whose case carries meaning.

use std::collections::BTreeMap;
use std::fmt::Write as _;

fn main() {
    println!("cargo:rerun-if-changed=patterns.toml");
    println!("cargo:rerun-if-changed=build.rs");

    let src = std::fs::read_to_string("patterns.toml").expect("patterns.toml is unreadable");
    let doc: toml::Value = src.parse().expect("patterns.toml is not valid TOML");

    let charset: BTreeMap<String, String> = doc
        .get("charset")
        .and_then(|c| c.as_table())
        .map(|t| {
            t.iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        v.as_str().unwrap_or_else(|| panic!("charset {k} is not a string")).to_owned(),
                    )
                })
                .collect()
        })
        .unwrap_or_default();

    let vendors = doc
        .get("vendor")
        .and_then(|v| v.as_array())
        .expect("patterns.toml has no [[vendor]] tables");

    let mut out = String::new();
    out.push_str("// Generated from patterns.toml by build.rs. Do not edit.\n");
    out.push_str("pub(crate) static VENDORS: &[VendorTable] = &[\n");

    for v in vendors {
        let name = v.get("name").and_then(|n| n.as_str()).expect("vendor without a name");
        let default = match v.get("default").and_then(|d| d.as_str()) {
            Some("idle") => "Word::Idle",
            Some("busy") => "Word::Busy",
            Some(other) => panic!("{name}: default must be idle or busy, got {other}"),
            None => panic!("{name}: no default"),
        };
        let debounce = match v.get("idle_debounce_ms").and_then(toml::Value::as_integer) {
            Some(ms) => format!("Some({ms})"),
            None => "None".to_owned(),
        };

        let _ = write!(
            out,
            "    VendorTable {{ name: {name:?}, default: {default}, idle_debounce_ms: {debounce}, rules: &[\n"
        );

        let rules = v
            .get("rule")
            .and_then(|r| r.as_array())
            .unwrap_or_else(|| panic!("{name}: a vendor with no rules answers its default forever"));
        assert!(!rules.is_empty(), "{name}: a vendor with no rules answers its default forever");

        for (i, r) in rules.iter().enumerate() {
            let where_ = format!("{name} rule #{}", i + 1);
            let lines = r
                .get("lines")
                .and_then(toml::Value::as_integer)
                .unwrap_or_else(|| panic!("{where_}: no lines"));
            let scope = match r.get("scope").and_then(|s| s.as_str()) {
                Some("tail") => format!("Scope::Tail({lines})"),
                Some("above_prompt") => format!("Scope::AbovePrompt({lines})"),
                other => panic!("{where_}: unknown scope {other:?}"),
            };
            let fold = match r.get("case").and_then(|c| c.as_str()) {
                Some("fold") => true,
                Some("exact") => false,
                other => panic!("{where_}: case must be fold or exact, got {other:?}"),
            };
            let then = match r.get("then").and_then(|t| t.as_str()) {
                Some("busy") => "Outcome::Busy",
                Some("waiting") => "Outcome::Waiting",
                Some("idle") => "Outcome::Idle",
                Some("keep") => "Outcome::Keep",
                other => panic!("{where_}: unknown outcome {other:?}"),
            };

            let test = match r.get("test").and_then(|t| t.as_str()) {
                Some("contains") => {
                    let p = pattern_of(r, &where_);
                    check_folded_literal(&p, fold, &where_);
                    format!("TestSpec::Contains {{ pattern: {p:?}, fold: {fold} }}")
                }
                Some("contains_all") => {
                    let ps: Vec<String> = r
                        .get("patterns")
                        .and_then(|p| p.as_array())
                        .unwrap_or_else(|| panic!("{where_}: contains_all needs `patterns`"))
                        .iter()
                        .map(|p| p.as_str().expect("pattern is not a string").to_owned())
                        .collect();
                    assert!(ps.len() > 1, "{where_}: contains_all with one pattern is contains");
                    for p in &ps {
                        check_folded_literal(p, fold, &where_);
                    }
                    let joined =
                        ps.iter().map(|p| format!("{p:?}")).collect::<Vec<_>>().join(", ");
                    format!("TestSpec::ContainsAll {{ patterns: &[{joined}], fold: {fold} }}")
                }
                Some("regex") => {
                    let raw = pattern_of(r, &where_);
                    let expanded = expand(&raw, &charset, &where_);
                    // Check 1: it has to compile here, or it panics there.
                    if let Err(e) = regex::Regex::new(&expanded) {
                        panic!("{where_}: pattern does not compile: {e}");
                    }
                    format!("TestSpec::Regex {{ pattern: {expanded:?}, fold: {fold} }}")
                }
                other => panic!("{where_}: unknown test {other:?}"),
            };

            let _ = write!(out, "        RuleSpec {{ scope: {scope}, test: {test}, then: {then} }},\n");
        }
        out.push_str("    ] },\n");
    }
    out.push_str("];\n");

    let dir = std::env::var("OUT_DIR").expect("no OUT_DIR");
    std::fs::write(std::path::Path::new(&dir).join("patterns.rs"), out)
        .expect("cannot write the generated table");
}

fn pattern_of(rule: &toml::Value, where_: &str) -> String {
    rule.get("pattern")
        .and_then(|p| p.as_str())
        .unwrap_or_else(|| panic!("{where_}: no pattern"))
        .to_owned()
}

/// Check 2: `{name}` must name a charset in the table.
fn expand(pattern: &str, charset: &BTreeMap<String, String>, where_: &str) -> String {
    let mut out = String::with_capacity(pattern.len());
    let mut rest = pattern;
    while let Some(open) = rest.find('{') {
        let (before, from_brace) = rest.split_at(open);
        out.push_str(before);
        let Some(close) = from_brace.find('}') else {
            out.push_str(from_brace);
            return out;
        };
        let name = &from_brace[1..close];
        // Only an identifier is a charset reference. This is what keeps
        // a repetition count like `{2,}` — cursor's spinner rule has one
        // — from being read as a name and rejected.
        if !name.is_empty() && name.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
            let set = charset
                .get(name)
                .unwrap_or_else(|| panic!("{where_}: {{{name}}} is not a charset in patterns.toml"));
            out.push_str(set);
        } else {
            out.push_str(&from_brace[..=close]);
        }
        rest = &from_brace[close + 1..];
    }
    out.push_str(rest);
    out
}

/// Check 3: a literal matched against a lowercased screen must itself be
/// lowercase, or it can never match and nothing will ever say so.
fn check_folded_literal(pattern: &str, fold: bool, where_: &str) {
    if fold && pattern.to_lowercase() != pattern {
        panic!("{where_}: {pattern:?} is matched case-folded but is not lowercase, so it can never fire");
    }
}
