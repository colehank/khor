//! What a file is, and what it looks like when it is not code
//! (工作区批6).
//!
//! # One pipeline, not one renderer per type
//!
//! Word, Excel, PowerPoint, their three legacy binary forms, csv and
//! markdown itself all end up as **markdown**, and markdown is rendered
//! once. That is not a tidy accident — `office_oxide` answers
//! `to_markdown()` for every format it reads, so the alternative would
//! have been six renderers agreeing with each other by hand. What it
//! costs is faithfulness: this is a document's *words and structure*,
//! never its layout, and the face says so rather than implying a
//! preview that is what the file looks like in Word.
//!
//! Images and SVG do not join the pipeline because the browser already
//! decodes them; handing bytes to a `<img>` is less code and a better
//! picture than anything here could produce.
//!
//! # What the dependencies cost, measured off the Cargo.lock diff
//!
//! **11 packages**, all permissive (checked before use, not after):
//! `office_oxide` and its seven (MIT OR Apache-2.0, except `zip` MIT
//! and `zopfli` Apache-2.0), `pulldown-cmark` (MIT), `csv` and
//! `csv-core` (Unlicense/MIT). One crate for six document formats is
//! the reason that number is small.
//!
//! ⚠️ **`office_oxide` is young** — first published 2026-04-28, 0.1.x,
//! and khor took 0.1.11. What was done about that instead of hoping:
//! every Office file on this machine was run through it, **25 of 25
//! parsed, 1–79 ms each**, including Chinese docx and pptx. Three
//! answered with no text at all, and all three are genuinely empty
//! documents (pandoc's reference template, python-docx's and
//! python-pptx's blank defaults) — so the empty answer is correct and
//! the face has a sentence for it rather than a blank pane. The
//! alternatives were measured too and are dead: `dotext` last shipped
//! in 2017, `textract` has one version from 2022.

use std::io::Cursor;

use khor_catalog::msg;

/// How much of a file a preview carries across, in bytes.
///
/// **4 MB, and the number comes off a disk rather than a feeling.**
/// Measured over this person's working directories: every png is under
/// 407 KB (103 of them, p95 355 KB) and every pdf under 1.53 MB (32 of
/// them). One file in all of it is bigger — a 43 MB gif — and that one
/// is a download, not a preview.
///
/// **Here rather than in each face**, because it was in two: the app
/// and `khor read` each held their own copy for a day, with nothing
/// keeping them in step (工作区批6's own debt, closed 2026-09-12).
/// Sixteen slices at [`crate::proto::SLICE`].
pub const LOOK: u64 = 4 * 1024 * 1024;

/// What khor will do with a file when it is opened.
///
/// **Decided from the name, then checked against the bytes where the
/// name is not enough.** An extension is what a person means by
/// double-clicking; sniffing alone would call an empty `.docx` a zip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Paint it as code, or as plain text (`gui_core::files::shape`).
    Text,
    /// Hand the bytes to the browser as an image.
    Image,
    /// An image too, but drawn from markup — which is why it is its own
    /// kind: it must reach the screen through `<img>` rather than as
    /// markup in the document, or the file's own scripts run.
    Svg,
    /// Markdown, whether it started as markdown or was turned into it.
    Doc,
    /// Nothing khor can show. It says so and offers to hand the file
    /// over.
    Other,
}

/// The extension, lowercased, or `""`.
fn ext(path: &str) -> String {
    let name = path.rsplit('/').next().unwrap_or(path);
    match name.rfind('.') {
        Some(i) if i > 0 => name[i + 1..].to_ascii_lowercase(),
        _ => String::new(),
    }
}

/// **Every image type the browsers khor draws on decode natively.**
/// Not a guess at what exists — what is listed is what a `<img>` will
/// show without khor decoding a single byte, so a type added here that
/// the browser cannot read would be a grey box with no sentence.
const IMAGES: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "bmp", "ico", "avif"];

/// The six `office_oxide` reads, plus the two text tables khor turns
/// into markdown itself.
const DOCS: &[&str] =
    &["md", "markdown", "docx", "xlsx", "pptx", "doc", "xls", "ppt", "csv", "tsv"];

/// The media type a browser needs to be told before it will decode
/// these bytes.
///
/// **Said by the node because the node is where the table lives.** The
/// face could derive it from the extension it already has, and that
/// would be a third copy of this list — the two that exist are already
/// held together by a test, and a third would need its own.
///
/// Empty for everything that is not drawn from bytes.
pub fn media(path: &str) -> &'static str {
    match ext(path).as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        "avif" => "image/avif",
        "svg" => "image/svg+xml",
        _ => "",
    }
}

/// What to do with a file of this name.
pub fn kind(path: &str) -> Kind {
    let e = ext(path);
    if IMAGES.contains(&e.as_str()) {
        return Kind::Image;
    }
    if e == "svg" {
        return Kind::Svg;
    }
    if DOCS.contains(&e.as_str()) {
        return Kind::Doc;
    }
    Kind::Text
}

/// A file's bytes as markdown, whatever it started as.
///
/// **Answers `Ok("")` for a document with no words in it**, which is a
/// real answer and not a failure: a blank template parses perfectly and
/// contains nothing. The caller says so out loud; a blank pane would
/// read as khor being broken.
pub fn markdown(path: &str, bytes: &[u8]) -> Result<String, String> {
    let e = ext(path);
    if e == "md" || e == "markdown" {
        return String::from_utf8(bytes.to_vec()).map_err(|_| msg::NOT_A_TEXT_FILE.to_owned());
    }
    if e == "csv" || e == "tsv" {
        return table(bytes, if e == "tsv" { b'\t' } else { b',' });
    }
    let Some(f) = office_oxide::DocumentFormat::from_extension(&e) else {
        return Err(msg::NOT_A_TEXT_FILE.to_owned());
    };
    let doc = office_oxide::Document::from_reader(Cursor::new(bytes.to_vec()), f)
        .map_err(|e| msg::cant_read_document(e))?;
    Ok(doc.to_markdown())
}

/// How many rows of a table travel. A spreadsheet is a thing people
/// keep a hundred thousand rows in, and a preview is a screen —
/// **the cut is said out loud** rather than left to look like the end
/// of the data.
const ROWS: usize = 2000;

/// Delimited text as a markdown table.
fn table(bytes: &[u8], sep: u8) -> Result<String, String> {
    let mut rd = csv::ReaderBuilder::new()
        .delimiter(sep)
        // **No header row, on purpose.** A csv's first line is a header
        // often and data sometimes, and khor cannot tell; treating it
        // as data and letting the markdown table's first row be the
        // header shows the same thing either way and loses nothing.
        .has_headers(false)
        // A ragged file is a file to look at, not to refuse.
        .flexible(true)
        .from_reader(bytes);
    let mut out = String::new();
    let mut wide = 0usize;
    let mut n = 0usize;
    let mut more = false;
    for row in rd.records() {
        let row = row.map_err(|e| msg::cant_read_document(e))?;
        if n >= ROWS {
            more = true;
            break;
        }
        let cells: Vec<String> = row.iter().map(|c| c.replace('|', "\\|")).collect();
        if n == 0 {
            wide = cells.len();
        }
        out.push_str("| ");
        out.push_str(&cells.join(" | "));
        out.push_str(" |\n");
        if n == 0 {
            out.push('|');
            for _ in 0..wide.max(1) {
                out.push_str(" --- |");
            }
            out.push('\n');
        }
        n += 1;
    }
    if more {
        out.push_str(&format!("\n{}\n", msg::table_cut(ROWS as u64)));
    }
    Ok(out)
}

/// Markdown as HTML.
///
/// **Raw HTML in the source is escaped, not passed through**, and that
/// is what keeps khor out of the sanitiser business entirely. A
/// markdown file is somebody's text — often somebody else's, pulled off
/// a machine khor does not own — and `<script>` inside it is markdown's
/// own feature. Escaping it costs a reader nothing worth having
/// (a `<div>` in a README renders as the words `<div>`, which is what
/// it says) and removes a whole class of holes along with the
/// dependency that would have guarded them.
pub fn html(md: &str) -> String {
    use pulldown_cmark::{Event, Options, Parser, html as render};
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);
    opts.insert(Options::ENABLE_FOOTNOTES);
    // **The escaping is done here, by hand, because there is no option
    // for it.** `pulldown-cmark` passes raw HTML straight through by
    // default; the two events that carry it are turned into text, which
    // is what makes a `<script>` in a README render as the words
    // `<script>`. The test below is what found this — the first
    // spelling of this function trusted the parser and shipped a hole.
    let events = Parser::new_ext(md, opts).map(|e| match e {
        Event::Html(s) | Event::InlineHtml(s) => Event::Text(s),
        other => other,
    });
    let mut out = String::new();
    render::push_html(&mut out, events);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_extension_decides_and_a_dotfile_is_not_an_extension() {
        assert_eq!(kind("/a/b.png"), Kind::Image);
        assert_eq!(kind("/a/B.PNG"), Kind::Image, "and case does not matter");
        assert_eq!(kind("/a/b.svg"), Kind::Svg);
        assert_eq!(kind("/a/b.docx"), Kind::Doc);
        assert_eq!(kind("/a/b.rs"), Kind::Text);
        assert_eq!(kind("/a/Makefile"), Kind::Text, "no extension at all");
        assert_eq!(kind("/a/.gitignore"), Kind::Text, "a leading dot is not one either");
    }

    #[test]
    fn every_image_kind_has_a_media_type_to_be_decoded_by() {
        // The pair that must not drift: a type this module calls an
        // image and then cannot name is a `data:` URL with no media
        // type, which browsers only render by sniffing — and one that
        // sniffs wrong draws a broken-image glyph.
        for e in IMAGES {
            let path = format!("/a/b.{e}");
            assert_eq!(kind(&path), Kind::Image, "{e}");
            assert!(media(&path).starts_with("image/"), "{e} has no media type");
        }
        assert_eq!(media("/a/b.svg"), "image/svg+xml");
        assert_eq!(media("/a/b.rs"), "", "and nothing else claims one");
    }

    #[test]
    fn raw_html_in_markdown_is_escaped_rather_than_rendered() {
        // The whole of khor's sanitiser story, asserted.
        let out = html("hi <script>alert(1)</script>\n");
        assert!(!out.contains("<script"), "a script tag survived: {out}");
        assert!(out.contains("&lt;script"), "and it must still be readable as text: {out}");
    }

    #[test]
    fn a_csv_becomes_a_table_with_a_header_rule() {
        let md = markdown("/a/b.csv", b"one,two\n1,2\n").unwrap();
        assert_eq!(md, "| one | two |\n| --- | --- |\n| 1 | 2 |\n");
        let out = html(&md);
        assert!(out.contains("<table>"), "and the table survives the render: {out}");
    }

    #[test]
    fn a_pipe_inside_a_cell_does_not_break_the_table() {
        // The one character a markdown table cannot carry raw.
        let md = markdown("/a/b.csv", "a|b,c\n".as_bytes()).unwrap();
        assert!(md.starts_with("| a\\|b | c |"), "{md}");
    }

    #[test]
    fn a_tab_separated_file_uses_tabs() {
        let md = markdown("/a/b.tsv", b"one\ttwo\n").unwrap();
        assert!(md.starts_with("| one | two |"), "{md}");
    }

    #[test]
    fn a_markdown_file_comes_back_as_itself() {
        assert_eq!(markdown("/a/b.md", b"# hi\n").unwrap(), "# hi\n");
    }
}
