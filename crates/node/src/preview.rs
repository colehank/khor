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
//! **15 packages**, all permissive (checked before use, not after):
//! `office_oxide` and its seven (MIT OR Apache-2.0, except `zip` MIT
//! and `zopfli` Apache-2.0), `pulldown-cmark` (MIT), `csv` and
//! `csv-core` (Unlicense/MIT), and `pdfboss-output` with its three
//! (MIT OR Apache-2.0). One crate per *family* rather than per format
//! is the reason that number is small.
//!
//! **The PDF number is the one that was shopped for.** Reading a PDF's
//! words needs its fonts and encodings, so it is never free — but the
//! spread is enormous and was measured off the Cargo.lock diff rather
//! than guessed. `pdf_oxide`, from `office_oxide`'s own author and so
//! the first candidate on the family argument, costs **55 packages**
//! (53 with default features off): it is a whole toolkit, and the
//! rasteriser, the layout engine, the JPEG decoder and the font writer
//! come along whether or not anything renders. `pdf-extract` costs 19.
//! `pdfboss` costs **4**, because text extraction is all it claims to
//! do — which is also all this file wants, and the reason pdf.js and
//! its megabyte of frontend never entered the question.
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

/// The same, for a picture.
///
/// **Bigger because the two failures are not the same failure.** A
/// document cut at four megabytes is still a document: its first four
/// megabytes of words are on screen and the face says the rest was
/// left. A picture cut anywhere is not a picture — half a JPEG decodes
/// to nothing — so the cap is the line between showing it and showing
/// a sentence about it, and that line belongs further out.
///
/// **16 MB, measured rather than felt.** Across this person's
/// directories, 265 images: p50 is 20 KB, p95 is 0.91 MB, p99 is
/// 4.42 MB. Four exceed the document cap and one exceeds this one — a
/// 42 MB gif, which is a download and not a preview, and still gets
/// the sentence. Sixty-four slices at [`crate::proto::SLICE`], which
/// is the cost this buys and the reason it is not simply enormous.
pub const LOOK_IMAGE: u64 = 16 * 1024 * 1024;

/// How much of this file a preview should carry.
///
/// **One question, asked in one place.** The cap and the kind were
/// picked apart in `khor_gui_core::files::preview` and in `khor read`,
/// which is two places to forget that a picture wants the larger one.
pub fn cap(path: &str) -> u64 {
    match kind(path) {
        Kind::Image | Kind::Svg => LOOK_IMAGE,
        _ => LOOK,
    }
}

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
    &["md", "markdown", "docx", "xlsx", "pptx", "doc", "xls", "ppt", "csv", "tsv", "pdf"];

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
    if e == "pdf" {
        return pdf(bytes);
    }
    let Some(f) = office_oxide::DocumentFormat::from_extension(&e) else {
        return Err(msg::NOT_A_TEXT_FILE.to_owned());
    };
    let doc = office_oxide::Document::from_reader(Cursor::new(bytes.to_vec()), f)
        .map_err(|e| msg::cant_read_document(e))?;
    Ok(doc.to_markdown())
}

/// A PDF's words, as markdown.
///
/// **Text, not pages, and that is the same line every other document
/// in this file is on.** A PDF renderer would be a picture of a page,
/// which is what pdf.js would have cost a megabyte of frontend to
/// give; what this road promises is「读的是文字和结构,不是排版」, and
/// `pdfboss` extracts exactly that — headings ranked across the whole
/// document rather than per page, so a title page reads as a heading
/// instead of as its own idea of body size.
///
/// **A PDF with no text layer is not an empty document.** A scan and a
/// figure both come back with nothing, and 「这个文档里没有内容」
/// would be a lie about a twenty-page handbook — so the two are told
/// apart here, by whether the file names a font at all, and the scan
/// gets its own sentence. Measured on this machine: of 68 PDFs, 21
/// carry text and 47 do not, and the 47 are matplotlib's toolbar
/// icons, a paper's figures, and one image-only handbook whose 20
/// pages hold ten JPEGs and no font object at all (which Spotlight
/// also reads as empty). Nothing failed.
fn pdf(bytes: &[u8]) -> Result<String, String> {
    let doc = pdfboss_core::document::Document::load(bytes.to_vec())
        .map_err(msg::cant_read_document)?;
    // Geometric rather than Content: a PDF's content stream emits text
    // in whatever order it was drawn, which for a two-column paper is
    // not the order anybody reads it in.
    let md = pdfboss_output::extract_markdown(&doc, pdfboss_output::ReadingOrder::Geometric)
        .map_err(msg::cant_read_document)?;
    if md.trim().is_empty() && !has_font(bytes) {
        return Err(msg::PDF_IS_PICTURES.to_owned());
    }
    Ok(md)
}

/// Whether the file names a font anywhere.
///
/// **A coarse question answered coarsely, and deliberately.** It is
/// asked only to choose between two sentences about a document that
/// came back empty, so a false yes costs「没有内容」where「都是图」
/// would have been kinder and a false no costs the reverse. Neither is
/// a wrong reading of the file, and the alternative — walking every
/// page's resources for a font dictionary — is a second parse of the
/// whole document to pick a word.
fn has_font(bytes: &[u8]) -> bool {
    bytes.windows(5).any(|w| w == b"/Font")
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

    /// A valid PDF built here rather than a file checked in.
    ///
    /// **Built because the real ones are somebody's documents.** The 68
    /// PDFs this was measured against are the user's papers, figures
    /// and a scanned handbook, and a fixture directory is a place files
    /// stay forever. What the two shapes below need to be is a PDF with
    /// a font and a PDF without one, and that is short enough to write.
    ///
    /// The cross-reference table's offsets are computed rather than
    /// typed: a table that disagrees with the bytes is the one thing a
    /// lenient parser repairs silently, which would make the fixture
    /// prove nothing about the parser reading it.
    fn tiny_pdf(with_font: bool) -> Vec<u8> {
        let content: &[u8] = if with_font {
            b"BT /F1 24 Tf 20 100 Td (khor reads this) Tj ET\n"
        } else {
            b"0 0 1 rg 20 20 160 160 re f\n"
        };
        // The font object is left out entirely, not merely unreferenced
        // — the first spelling of this always wrote it and `has_font`
        // saw the `/Font` in it, so the picture-only case answered as
        // an empty document and the fixture proved the opposite of what
        // it was for.
        let content_obj = if with_font { 5 } else { 4 };
        let resources =
            if with_font { "/Resources<</Font<</F1 4 0 R>>>>" } else { "/Resources<<>>" };
        let mut objs: Vec<String> = vec![
            "<</Type/Catalog/Pages 2 0 R>>".to_owned(),
            "<</Type/Pages/Kids[3 0 R]/Count 1>>".to_owned(),
            format!(
                "<</Type/Page/Parent 2 0 R/MediaBox[0 0 200 200]{resources}/Contents {content_obj} 0 R>>"
            ),
        ];
        if with_font {
            objs.push("<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>".to_owned());
        }
        objs.push(format!("<</Length {}>>", content.len()));

        let mut out: Vec<u8> = b"%PDF-1.4\n".to_vec();
        let mut offsets = Vec::new();
        for (i, body) in objs.iter().enumerate() {
            offsets.push(out.len());
            out.extend_from_slice(format!("{} 0 obj\n{body}\n", i + 1).as_bytes());
            if i + 1 == content_obj {
                out.extend_from_slice(b"stream\n");
                out.extend_from_slice(content);
                out.extend_from_slice(b"endstream\n");
            }
            out.extend_from_slice(b"endobj\n");
        }
        let xref = out.len();
        out.extend_from_slice(
            format!("xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1).as_bytes(),
        );
        for off in &offsets {
            out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
        }
        out.extend_from_slice(
            format!(
                "trailer\n<</Size {}/Root 1 0 R>>\nstartxref\n{xref}\n%%EOF\n",
                objs.len() + 1
            )
            .as_bytes(),
        );
        out
    }

    #[test]
    fn a_pdfs_words_come_back_as_markdown() {
        let md = markdown("a.pdf", &tiny_pdf(true)).expect("reads");
        assert!(md.contains("khor reads this"), "got {md:?}");
    }

    #[test]
    fn a_pdf_of_pictures_says_so_rather_than_saying_it_is_empty() {
        // A scan and a figure both extract nothing. 「这个文档里没有
        // 内容」 about a twenty-page handbook is a flat lie, so the two
        // readings are told apart by whether the file names a font.
        let err = markdown("a.pdf", &tiny_pdf(false)).expect_err("no text to read");
        assert_eq!(err, msg::PDF_IS_PICTURES);
    }

    #[test]
    fn a_pdf_is_a_document_not_a_picture_to_the_router() {
        // The kind decides which screen opens before anything is read;
        // a `.pdf` sent down the image road would reach an `<img>` that
        // renders nothing (`views/FileView`'s `kindOf`).
        assert_eq!(kind("a.pdf"), Kind::Doc);
        assert_eq!(kind("A.PDF"), Kind::Doc);
    }

    #[test]
    fn something_that_is_not_a_pdf_at_all_is_refused_in_words() {
        let err = markdown("a.pdf", b"this is not a pdf").expect_err("refuses");
        assert!(!err.is_empty());
        assert_ne!(err, msg::PDF_IS_PICTURES, "a broken file is not a scan");
    }

    #[test]
    fn a_picture_is_allowed_to_be_bigger_than_a_document() {
        // Half a document still reads; half a JPEG decodes to nothing.
        assert_eq!(cap("a.png"), LOOK_IMAGE);
        assert_eq!(cap("a.svg"), LOOK_IMAGE);
        assert_eq!(cap("a.docx"), LOOK);
        assert_eq!(cap("a.rs"), LOOK);
        assert!(LOOK_IMAGE > LOOK);
    }
}
