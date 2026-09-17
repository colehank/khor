//! What a file is, and what it looks like when it is not code
//! (工作区批6).
//!
//! # One pipeline, not one renderer per type
//!
//! Word, Excel, PowerPoint and their three legacy binary forms all end
//! up as **markdown**, and markdown is rendered once. That is not a
//! tidy accident — `office_oxide` answers `to_markdown()` for every
//! format it reads, so the alternative would have been six renderers
//! agreeing with each other by hand. What it costs is faithfulness:
//! this is a document's *words and structure*, never its layout, and
//! the face says so rather than implying a preview that is what the
//! file looks like in Word.
//!
//! **Three things are deliberately not on that pipeline.** Images and
//! SVG, because the browser already decodes them and handing bytes to
//! an `<img>` is less code and a better picture than anything here
//! could produce. PDF, for the same reason, on the platforms whose
//! browser has a viewer ([`PDF_HERE`]). And md, csv and tsv, which are
//! **text** — calling them documents cost them saving, the save
//! contract, the three-way merge and the change gutter, and the
//! rendered view they actually wanted is one the screen can draw from
//! the buffer a person is typing into (工作区批⑤).
//!
//! # What the dependencies cost, measured off the Cargo.lock diff
//!
//! **13 packages**, all permissive (checked before use, not after):
//! `office_oxide` and its seven (MIT OR Apache-2.0, except `zip` MIT
//! and `zopfli` Apache-2.0), `pulldown-cmark` (MIT), and
//! `pdfboss-output` with its three (MIT OR Apache-2.0). One crate per
//! *family* rather than per format is the reason that number is small.
//! It was 15 until 工作区批⑤ gave `csv` and `csv-core` back with the
//! table they built — **a dependency is retired when its last caller
//! is**, and the caller went to the screen.
//!
//! **The four PDF packages now buy the fallback, not the face.** They
//! were shopped for when reading a PDF's words *was* the plan, and the
//! numbers are worth keeping because the choice would be the same
//! today: `pdf_oxide`, from `office_oxide`'s own author and so the
//! first candidate on the family argument, costs **55 packages** (53
//! with default features off) — a whole toolkit, rasteriser and layout
//! engine and JPEG decoder and font writer, whether or not anything
//! renders. `pdf-extract` costs 19. `pdfboss` costs **4**, because text
//! extraction is all it claims to do.
//!
//! What changed is what that extraction is for. 「pdf 你咋转成字符了,
//! 应该直接预览」(the user, 2026-09-14): where the browser draws PDFs
//! the bytes go straight to it, and these four are what Linux and
//! Android get instead of an empty frame. **pdf.js still never entered
//! the question** — a megabyte of frontend to do what three of the five
//! platforms already do natively, and the other two are the two khor
//! would have had to ship it to anyway.
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
    /// Hand the bytes to the browser as a PDF, where the browser has a
    /// PDF viewer ([`PDF_HERE`]); extract its words where it does not.
    ///
    /// **Its own kind rather than an image**, because the two failures
    /// differ: a platform with no viewer still has something to show
    /// (the words), and an image with no decoder has nothing.
    Pdf,
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

/// The six `office_oxide` reads, and only those (工作区批⑤).
///
/// **md, csv and tsv left on 2026-09-15.** They are text files, and
/// calling them documents cost them everything a text file gets:
/// markdown could not be saved, had no save contract, no three-way
/// merge and no change gutter — on the one format this project is
/// written in. They go down the text road now and the *screen* offers
/// a rendered view beside the source (「所有代码、md 文件,打开先是源码,
/// 要有语法高亮。然后对于支持预览的文件,如 md,显示预览」— the user,
/// 2026-09-14), which is a face's job and not a kind.
const DOCS: &[&str] = &["docx", "xlsx", "pptx", "doc", "xls", "ppt"];

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
        // Not an image, but the same contract: bytes the browser
        // decodes itself, and a blob with no media type is a blob the
        // viewer will not open.
        "pdf" => "application/pdf",
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
    if e == "pdf" {
        return Kind::Pdf;
    }
    if DOCS.contains(&e.as_str()) {
        return Kind::Doc;
    }
    Kind::Text
}

/// **Whether the browser khor's face runs in will draw a PDF itself.**
///
/// A compile-time fact, answered here for [`crate::files::TRASH_HERE`]'s
/// reason: the face must not read the platform off `navigator`, because
/// the same page is served to a phone looking at another machine's
/// khor.
///
/// macOS and iOS (WebKit, PDFKit behind it) and Windows (WebView2,
/// Chromium's viewer) draw one. **WebKitGTK does not ship a PDF
/// viewer** and Android's WebView does not either — there the words are
/// what khor can honestly show, and the face says so out loud rather
/// than drawing an empty frame.
pub const PDF_HERE: bool =
    cfg!(any(target_os = "macos", target_os = "ios", target_os = "windows"));

/// A file's bytes as markdown, whatever it started as.
///
/// **Answers `Ok("")` for a document with no words in it**, which is a
/// real answer and not a failure: a blank template parses perfectly and
/// contains nothing. The caller says so out loud; a blank pane would
/// read as khor being broken.
pub fn markdown(path: &str, bytes: &[u8]) -> Result<String, String> {
    let e = ext(path);
    // **Already words.** markdown is markdown; a csv or tsv is its own
    // plain text, and since 工作区批⑤ nothing turns it into a table on
    // this side — the screen that wants a table builds one from the
    // buffer it is showing, which is the only copy that can be half
    // typed (`views/FileView`). `khor read` prints the file.
    if e == "md" || e == "markdown" || e == "csv" || e == "tsv" {
        return String::from_utf8(bytes.to_vec()).map_err(|_| msg::NOT_A_TEXT_FILE.to_owned());
    }
    if e == "pdf" {
        return pdf(bytes);
    }
    let Some(f) = office_oxide::DocumentFormat::from_extension(&e) else {
        return Err(msg::NOT_A_TEXT_FILE.to_owned());
    };
    let doc = office_oxide::Document::from_reader(Cursor::new(bytes.to_vec()), f)
        .map_err(msg::cant_read_document)?;
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
        assert_eq!(kind("/a/b.pdf"), Kind::Pdf, "its own kind since 工作区批⑤");
        // **The three that left `Doc` on 2026-09-15.** They are text,
        // and what a text file gets — saving, a save contract, a merge,
        // a change gutter — is what calling them documents cost.
        assert_eq!(kind("/a/b.md"), Kind::Text);
        assert_eq!(kind("/a/b.markdown"), Kind::Text);
        assert_eq!(kind("/a/b.csv"), Kind::Text);
        assert_eq!(kind("/a/b.tsv"), Kind::Text);
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
        // The browser decodes this one too, so it needs naming for the
        // same reason (工作区批⑤).
        assert_eq!(media("/a/b.pdf"), "application/pdf");
        assert_eq!(media("/a/b.rs"), "", "and nothing else claims one");
    }

    #[test]
    fn raw_html_in_markdown_is_escaped_rather_than_rendered() {
        // The whole of khor's sanitiser story, asserted.
        let out = html("hi <script>alert(1)</script>\n");
        assert!(!out.contains("<script"), "a script tag survived: {out}");
        assert!(out.contains("&lt;script"), "and it must still be readable as text: {out}");
    }

    /// **A text file comes back as itself**, whichever of the three it
    /// is (工作区批⑤). A csv used to be turned into a markdown table
    /// here; the screen builds that from its own buffer now, and
    /// `khor read` prints the file — which for a csv *is* its words.
    #[test]
    fn the_text_formats_come_back_as_themselves() {
        assert_eq!(markdown("/a/b.md", b"# hi\n").unwrap(), "# hi\n");
        assert_eq!(markdown("/a/b.csv", b"one,two\n1,2\n").unwrap(), "one,two\n1,2\n");
        assert_eq!(markdown("/a/b.tsv", b"one\ttwo\n").unwrap(), "one\ttwo\n");
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

    /// The kind decides which screen opens before anything is read.
    ///
    /// **A PDF is its own kind since 工作区批⑤**, and it was `Doc`
    /// before: the user asked for the file itself rather than its words
    /// (「pdf 你咋转成字符了,应该直接预览」, 2026-09-14), and where the
    /// browser has no viewer the words are still what khor can honestly
    /// show — two screens, so one kind that carries both and neither of
    /// the two it used to be confused with. It is not an image: an
    /// image with no decoder has nothing left to say.
    #[test]
    fn a_pdf_is_its_own_kind_to_the_router() {
        assert_eq!(kind("a.pdf"), Kind::Pdf);
        assert_eq!(kind("A.PDF"), Kind::Pdf, "and case does not matter");
        assert_ne!(kind("a.pdf"), Kind::Image, "an <img> would render nothing");
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
        // Asserting on constants on purpose: the ordering of these two
        // caps is the claim this test exists to hold.
        #[allow(clippy::assertions_on_constants)]
        {
            assert!(LOOK_IMAGE > LOOK);
        }
    }
}
