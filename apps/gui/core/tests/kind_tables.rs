//! **What a file is must have one answer, and it is written twice.**
//!
//! `khor_node::preview::kind` decides what khor does with a file — the
//! node renders by it, `khor read` refuses by it, and the workspace
//! paints by it. The file view needs the same answer *before* it asks
//! anything, because the two roads it can send a file down differ in
//! what they spend: one decodes bytes as text, the other hands them to
//! a picture. Waiting for the node would mean a blank pane on every
//! file, or a slice spent decoding a png as text.
//!
//! So `views/FileView.tsx` keeps a copy (`kindOf`), and this is what
//! stops the two drifting. **A new image format added to one and not
//! the other is silent in the worst direction**: the view sends a
//! `.avif` down the text road, the node answers with a picture, and the
//! pane shows nothing at all.
//!
//! # Reading source is brittle, so the finders fail loudly
//!
//! Both sides assert they found a plausible number before anything is
//! compared — a regex that stopped matching returns an empty set, and
//! two empty sets agree perfectly (`command_tables` carries the longer
//! version of this reasoning).

use std::collections::BTreeSet;
use std::path::PathBuf;

fn repo_file(rel: &str) -> String {
    let p: PathBuf = [env!("CARGO_MANIFEST_DIR"), rel].iter().collect();
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("reading {}: {e}", p.display()))
}

/// Every quoted word inside the list `needle` is assigned.
///
/// **The `=` is part of the search and that is not fussiness.** The
/// first spelling looked for the first `[` after the name and found the
/// one in `&[&str]` — the *type* — so it compared two empty sets and
/// the loud-finder assertion below is the only reason that was a red
/// and not a green.
fn list_after(src: &str, needle: &str) -> BTreeSet<String> {
    let at = src
        .find(needle)
        .unwrap_or_else(|| panic!("`{needle}` is gone — if it moved, fix this finder"));
    let rest = &src[at..];
    let eq = rest.find('=').expect("the name is assigned something");
    let rest = &rest[eq..];
    let open = rest.find('[').expect("a list follows it");
    let close = rest[open..].find(']').expect("and the list is closed") + open;
    rest[open..close]
        .split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect()
}

#[test]
fn the_node_and_the_file_view_agree_about_what_an_image_is() {
    let rust = list_after(&repo_file("../../../crates/node/src/preview.rs"), "const IMAGES");
    let ts = list_after(&repo_file("../src/views/FileView.tsx"), "const IMAGES");
    assert!(rust.len() >= 6, "the rust finder matched {rust:?}, which is too few to be the table");
    assert!(ts.len() >= 6, "the ts finder matched {ts:?}, which is too few to be the table");
    assert_eq!(
        rust, ts,
        "the node and the file view disagree about which files are pictures — the view sends one \
         down the text road while the node answers with an image, and the pane shows nothing",
    );
}

#[test]
fn the_node_and_the_file_view_agree_about_what_a_document_is() {
    let rust = list_after(&repo_file("../../../crates/node/src/preview.rs"), "const DOCS");
    let ts = list_after(&repo_file("../src/views/FileView.tsx"), "const DOCS");
    assert!(rust.len() >= 8, "the rust finder matched {rust:?}");
    assert!(ts.len() >= 8, "the ts finder matched {ts:?}");
    assert_eq!(
        rust, ts,
        "the node and the file view disagree about which files are documents",
    );
}
