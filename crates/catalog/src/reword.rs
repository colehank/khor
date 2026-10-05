//! A sentence another khor said, read back into its key and arguments
//! and said again in this process's language (round NET-9).
//!
//! # Why the sentence and not a key on the wire
//!
//! A refusal crosses as `Response::Refused { why }`: the far machine's
//! sentence, in the far machine's language. The owner's Mac showed
//! 「那台机器拒绝了这次读取：This machine isn't in the machine table — pair
//! first」, a Chinese frame around an English refusal.
//!
//! The words are the catalog's on both ends, and every entry exists in
//! both languages (`build.rs` refuses a gap). So the sentence already
//! names its key: matching it against both texts of every entry gives
//! the key and the arguments, and the entry's text in this language says
//! it again. That needs nothing on the wire, so it works in both
//! directions with a khor of any age that shares the entry. A key added
//! to the frame instead would be a longer frame, which an older asker
//! refuses whole, losing even the sentence (`khor_node::proto`'s head).
//!
//! # What stays as it was
//!
//! A sentence no entry matches (an OS error, a decode error naming a
//! variant) comes back `None` and is shown as it was said; an argument
//! that is itself a catalog sentence is said again too. A text with too
//! little of its own to be recognised (`{0}`, `{0}：{1}`) is never
//! matched: it would match anything.

use std::sync::OnceLock;

/// The least of its own text, in characters, an entry must have to be
/// matched. Below this an entry is mostly placeholders and would claim
/// sentences it never said.
const OWN_TEXT: usize = 4;

/// How deep arguments are said again: a refusal inside a sentence
/// inside a sentence is as far as khor nests them.
const DEPTH: usize = 3;

#[derive(Debug)]
enum Piece {
    Text(String),
    Slot(usize),
}

struct Entry {
    zh: Vec<Piece>,
    en: Vec<Piece>,
}

fn pieces(text: &str) -> Vec<Piece> {
    let mut out = Vec::new();
    let mut lit = String::new();
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        let b = rest.as_bytes();
        if c == '{' && b.len() >= 3 && b[1].is_ascii_digit() && b[2] == b'}' {
            if !lit.is_empty() {
                out.push(Piece::Text(std::mem::take(&mut lit)));
            }
            out.push(Piece::Slot((b[1] - b'0') as usize));
            rest = &rest[3..];
        } else {
            lit.push(c);
            rest = &rest[c.len_utf8()..];
        }
    }
    if !lit.is_empty() {
        out.push(Piece::Text(lit));
    }
    out
}

fn own_text(p: &[Piece]) -> usize {
    p.iter()
        .map(|x| match x {
            Piece::Text(t) => t.chars().count(),
            Piece::Slot(_) => 0,
        })
        .sum()
}

fn entries() -> &'static [Entry] {
    static ALL: OnceLock<Vec<Entry>> = OnceLock::new();
    ALL.get_or_init(|| {
        crate::TEMPLATES
            .iter()
            .map(|(_, zh, en)| Entry { zh: pieces(zh), en: pieces(en) })
            .filter(|e| own_text(&e.zh) >= OWN_TEXT && own_text(&e.en) >= OWN_TEXT)
            .collect()
    })
}

/// Whether every text of `p` occurs in `said`, in order, with the first
/// at the start and the last at the end where they are there — the
/// cheap refusal most entries meet before [`fill`] is asked.
fn could_be(p: &[Piece], said: &str) -> bool {
    if let Some(Piece::Text(t)) = p.first()
        && !said.starts_with(t.as_str())
    {
        return false;
    }
    if let Some(Piece::Text(t)) = p.last()
        && !said.ends_with(t.as_str())
    {
        return false;
    }
    let mut rest = said;
    for x in p {
        if let Piece::Text(t) = x {
            match rest.find(t.as_str()) {
                Some(at) => rest = &rest[at + t.len()..],
                None => return false,
            }
        }
    }
    true
}

/// How many tries one match may take. A text that is mostly short
/// separators between slots could otherwise be tried every way it splits.
const TRIES: usize = 4096;

/// The arguments `said` fills `p` with, or `false` when it is not this
/// text. A slot takes the shortest run after which the rest still
/// matches, and tries longer ones only if that fails.
fn fill<'a>(p: &[Piece], said: &'a str, args: &mut Vec<(usize, &'a str)>, tries: &mut usize) -> bool {
    if *tries == 0 {
        return false;
    }
    *tries -= 1;
    match p.split_first() {
        None => said.is_empty(),
        Some((Piece::Text(t), rest)) => said
            .strip_prefix(t.as_str())
            .is_some_and(|s| fill(rest, s, args, tries)),
        Some((Piece::Slot(i), rest)) => {
            // The next text must follow the slot, so only the places where
            // it starts are worth trying; a last slot takes the rest.
            let cuts: Vec<usize> = match rest.first() {
                None => vec![said.len()],
                Some(Piece::Text(next)) => said.match_indices(next.as_str()).map(|(at, _)| at).collect(),
                Some(Piece::Slot(_)) => said
                    .char_indices()
                    .map(|(at, _)| at)
                    .chain([said.len()])
                    .collect(),
            };
            for at in cuts {
                args.push((*i, &said[..at]));
                if fill(rest, &said[at..], args, tries) {
                    return true;
                }
                args.pop();
            }
            false
        }
    }
}

fn render(p: &[Piece], args: &[(usize, String)]) -> String {
    let mut out = String::new();
    for x in p {
        match x {
            Piece::Text(t) => out.push_str(t),
            Piece::Slot(i) => {
                if let Some((_, a)) = args.iter().find(|(j, _)| j == i) {
                    out.push_str(a);
                }
            }
        }
    }
    out
}

fn said_again(said: &str, lang: crate::lang::Lang, depth: usize) -> Option<String> {
    let mut best: Option<(&Entry, Vec<(usize, &str)>, usize)> = None;
    for e in entries() {
        for p in [&e.zh, &e.en] {
            let mut args = Vec::new();
            if could_be(p, said) && fill(p, said, &mut args, &mut TRIES.clone()) {
                let own = own_text(p);
                if best.as_ref().is_none_or(|(_, _, b)| own > *b) {
                    best = Some((e, args, own));
                }
            }
        }
    }
    let (e, args, _) = best?;
    let args: Vec<(usize, String)> = args
        .into_iter()
        .map(|(i, a)| {
            let again = (depth > 1).then(|| said_again(a, lang, depth - 1)).flatten();
            (i, again.unwrap_or_else(|| a.to_owned()))
        })
        .collect();
    let here = match lang {
        crate::lang::Lang::Zh => &e.zh,
        crate::lang::Lang::En => &e.en,
    };
    Some(render(here, &args))
}

/// `said` in this process's language, when it is a catalog sentence in
/// either language; `None` when no entry says it.
pub fn reword(said: &str) -> Option<String> {
    said_again(said, crate::lang::current(), DEPTH)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lang::Lang;

    /// An entry's text in one language, with these arguments. The tests
    /// never switch the process's language: other tests in this binary
    /// read it at the same time.
    fn said(key: &str, lang: Lang, args: &[&str]) -> String {
        let (_, zh, en) = crate::TEMPLATES.iter().find(|(k, _, _)| *k == key).expect(key);
        let p = pieces(if lang == Lang::Zh { zh } else { en });
        let args: Vec<(usize, String)> = args.iter().enumerate().map(|(i, a)| (i, a.to_string())).collect();
        render(&p, &args)
    }

    /// The owner's machine page: a far refusal in English, read on a
    /// Chinese desk, and the other way round, nested in a local frame.
    #[test]
    fn a_far_refusal_is_said_again_in_this_language() {
        let far = said("msg.not-paired", Lang::En, &[]);
        let here = said("msg.not-paired", Lang::Zh, &[]);
        assert_eq!(said_again(&far, Lang::Zh, DEPTH), Some(here.clone()));
        assert_eq!(said_again(&here, Lang::Zh, DEPTH), Some(here.clone()), "already in this language");
        assert_eq!(
            said_again(&said("msg.invite-expired", Lang::En, &["15"]), Lang::Zh, DEPTH),
            Some(said("msg.invite-expired", Lang::Zh, &["15"]))
        );
        let nested = said("msg.cant-reach-named", Lang::Zh, &["hinton", &here]);
        assert_eq!(
            said_again(&nested, Lang::En, DEPTH),
            Some(said("msg.cant-reach-named", Lang::En, &["hinton", &far])),
            "an argument that is itself a sentence is said again too"
        );
    }

    /// What no entry says comes back `None`: an OS error, a decode error
    /// naming a variant, nothing at all.
    #[test]
    fn what_the_catalog_never_said_is_left_alone() {
        for s in [
            "No such file or directory (os error 2)",
            "unknown variant `Vouch`, expected one of `Pair`, `Sync`",
            "",
            "hinton",
        ] {
            assert_eq!(said_again(s, Lang::Zh, DEPTH), None, "{s:?}");
        }
    }

    /// Every entry, said in one language with words as its arguments —
    /// which is what a far machine's names and paths are — reads back as
    /// the same entry in the other. Two entries may claim one sentence
    /// only when they say the same thing in both languages.
    #[test]
    fn every_entry_reads_back_from_either_language() {
        let mut missed = Vec::new();
        for (key, zh, en) in crate::TEMPLATES {
            let (pz, pe) = (pieces(zh), pieces(en));
            if own_text(&pz) < OWN_TEXT || own_text(&pe) < OWN_TEXT {
                continue;
            }
            let args: Vec<(usize, String)> = (0..9).map(|i| (i, format!("arg{i}"))).collect();
            for (from, to, lang) in [(&pz, &pe, Lang::En), (&pe, &pz, Lang::Zh)] {
                let said = render(from, &args);
                let got = said_again(&said, lang, DEPTH);
                // Another entry with this very text in this language is
                // the same sentence, and either translation is a reading.
                let twins = crate::TEMPLATES.iter().any(|(k, z, e)| {
                    k != key && render(&pieces(if lang == Lang::En { z } else { e }), &args) == said
                });
                if got.as_deref() != Some(render(to, &args).as_str()) && !twins {
                    missed.push(format!("{key}: {:?} -> {got:?}", render(from, &args)));
                }
            }
        }
        assert!(missed.is_empty(), "{} entries do not read back:\n{}", missed.len(), missed.join("\n"));
    }
}
