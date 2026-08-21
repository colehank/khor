//! Turns `apps/gui/dist` into a table of bytes compiled into khor.
//!
//! Same arrangement as khor-catalog's zh.toml and khor-detect's pattern
//! table: a file the build reads once, never a file the product opens at
//! run time. The shipped khor is one binary that needs nothing on the
//! machine it lands on (docs/KHOR.md), and a face that lived beside it
//! as loose files would be a face that arrives broken exactly when
//! somebody copied only the binary.
//!
//! # A missing `dist` stops the build, on purpose
//!
//! The three softer options were all worse:
//!
//! - a cargo feature, default off — then the default build is **a
//!   different product from the shipped one**, and the batch's own
//!   acceptance ("open the LAN address, get the whole GUI") would be
//!   proving something about a binary nobody ships;
//! - an empty table with an apologetic page — same, plus it *runs*: a
//!   khor that installs, starts, serves, and is blank;
//! - committing `dist` — build output in the tree, and the one fact
//!   (what the frontend is) would have two sources that drift.
//!
//! The price is real and is the intended one: `cargo build -p khor-cli`
//! wants `npm run build` to have happened once. That is a command with
//! an error message pointing at it, which is the cheapest kind of
//! missing prerequisite.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

fn main() {
    let dist: PathBuf = [env!("CARGO_MANIFEST_DIR"), "..", "..", "apps", "gui", "dist"]
        .iter()
        .collect();
    println!("cargo::rerun-if-changed={}", dist.display());

    // The entry point specifically, not the directory: a `dist` left
    // half-written by an interrupted build is a directory that exists
    // and answers nothing, and "it is there" would have passed.
    if !dist.join("index.html").is_file() {
        panic!(
            "the web face has no frontend to serve: {} is missing.\n\
             Build it once — `cd apps/gui && npm run build` — and this build will find it.",
            dist.join("index.html").display()
        );
    }

    let mut files = Vec::new();
    collect(&dist, &dist, &mut files);
    files.sort();

    let mut rs = String::from("// Generated from apps/gui/dist by build.rs - do not edit.\n");
    rs.push_str("pub static ASSETS: &[(&str, &[u8])] = &[\n");
    for (web_path, disk) in &files {
        writeln!(rs, "    ({web_path:?}, include_bytes!({:?})),", disk.display()).unwrap();
    }
    rs.push_str("];\n");

    let out = std::env::var("OUT_DIR").expect("OUT_DIR");
    std::fs::write(Path::new(&out).join("assets.rs"), rs).expect("writing assets.rs");
}

/// Every file under `dir`, keyed by the path a browser asks for.
fn collect(root: &Path, dir: &Path, out: &mut Vec<(String, PathBuf)>) {
    let listing = std::fs::read_dir(dir).unwrap_or_else(|e| panic!("reading {}: {e}", dir.display()));
    for entry in listing {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            collect(root, &path, out);
            continue;
        }
        let rel = path.strip_prefix(root).expect("under dist");
        // Web paths, not this machine's: vite writes `assets/x.js` and
        // the page asks for `/assets/x.js`. Windows would spell the
        // separator the other way and every request would 404 on the
        // one platform nobody builds releases from by hand.
        let web = format!("/{}", rel.to_string_lossy().replace('\\', "/"));
        out.push((web, path));
    }
}
