//! The files landing's data layer: one machine's directory, shaped for
//! the screen. The order, the cap and the "where is this about" all
//! come from the node ([`khor_node::files`]) — this layer renames
//! fields for TS and decides nothing.

use std::path::Path;

use khor_node::Node;
use serde::Serialize;
use ts_rs::TS;

/// A directory answer: where it is about (absolute, the far machine's
/// own spelling — the asker may have said `""` for home), its rows in
/// the node's order, and whether the cap cut it short.
#[derive(Debug, Clone, Serialize, TS)]
pub struct DirListing {
    pub path: String,
    pub entries: Vec<DirRow>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct DirRow {
    pub name: String,
    pub dir: bool,
    #[ts(type = "number")]
    pub size: u64,
    #[ts(type = "number")]
    pub at_ms: u64,
}

/// One machine's directory — the call `khor ls` makes. Async because
/// the machine may be far away and the answer rides a dial.
pub async fn ls(root: &Path, machine: &str, path: &str) -> Result<DirListing, String> {
    let n = Node::open(root.to_path_buf())?;
    let (path, entries, truncated) = n.ls_of(machine, path).await?;
    Ok(DirListing {
        path,
        entries: entries
            .into_iter()
            .map(|e| DirRow {
                name: e.name,
                dir: e.dir,
                size: e.size,
                at_ms: e.at_ms,
            })
            .collect(),
        truncated,
    })
}

/// Takes a file off a machine into this one's downloads directory —
/// the call `khor pull` makes, with the landing the desktop convention
/// picks (`~/Downloads`, falling back to home). Answers where it
/// landed: the place was chosen silently, so it must be said.
pub async fn pull(root: &Path, machine: &str, path: &str) -> Result<String, String> {
    let n = Node::open(root.to_path_buf())?;
    // **Not to a phone** (docs/IOS.md 第二步): its `$HOME` is the app's
    // container, whose top it may not write, and a file that did land
    // there is one no other app — the Files app included — can see. The
    // face draws no 拉过来 there; this is for anything that asks anyway.
    if !n.hosts() {
        return Err(khor_catalog::msg::pull_needs_a_computer().to_owned());
    }
    let dir = dirs_download();
    let (_, dest) = n.pull_path(machine, path, &dir).await?;
    Ok(dest.display().to_string())
}

fn dirs_download() -> std::path::PathBuf {
    let home = std::env::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    let dl = home.join("Downloads");
    if dl.is_dir() { dl } else { home }
}

/// Takes a path off whichever machine holds it — the call `khor rm`
/// makes (`khor_node::Link::remove_path`).
///
/// **This machine's goes to the recycle bin, every other machine's is
/// deleted**, and which of the two it will be is decided in the node.
/// The face has to say which before anybody presses, and it asks
/// [`trash_here`] rather than reading the platform off the browser: the
/// same page is served to a phone looking at a desktop's face.
pub async fn remove(root: &Path, machine: &str, path: &str) -> Result<(), String> {
    Node::open(root.to_path_buf())?
        .remove_path(machine, path)
        .await
}

/// One new directory on whichever machine holds the parent — the call
/// `khor mkdir` makes (`khor_node::Link::mkdir`). One level, and a name
/// already taken is refused.
pub async fn mkdir(root: &Path, machine: &str, path: &str) -> Result<(), String> {
    Node::open(root.to_path_buf())?.mkdir(machine, path).await
}

/// One new empty file, the call `khor touch` makes
/// (`khor_node::Link::touch`). A name already taken is refused and the
/// file wearing it is not emptied.
pub async fn touch(root: &Path, machine: &str, path: &str) -> Result<(), String> {
    Node::open(root.to_path_buf())?.touch(machine, path).await
}

/// Moves a name on whichever machine holds it — the call `khor mv`
/// makes (`khor_node::Link::rename_path`). Crossing directories is the
/// same verb; the target is never overwritten.
pub async fn rename_path(root: &Path, machine: &str, from: &str, to: &str) -> Result<(), String> {
    Node::open(root.to_path_buf())?
        .rename_path(machine, from, to)
        .await
}

/// Opens a path or a page **on the machine that holds it** — the call
/// `khor launch` makes (`khor_node::Link::launch`).
///
/// **Not [`crate::web::open_link`]**, and the pair is worth reading
/// together: that one hands a page to the machine the face is running
/// on, which is right for a link somebody wants to read. This one hands
/// it to the machine the session ran on, which is the only machine the
/// file in that session exists on.
pub async fn launch(root: &Path, machine: &str, target: &str) -> Result<(), String> {
    Node::open(root.to_path_buf())?
        .launch(machine, target)
        .await
}

/// One line of one file that holds the words somebody typed
/// (工作区第六轮 第 47 条). Renamed for TS and nothing else; every
/// judgment — the caps, `.gitignore`, what counts as binary — is
/// `khor_node::files::search`'s.
#[derive(Debug, Clone, Serialize, TS)]
pub struct SearchHit {
    pub path: String,
    #[ts(type = "number")]
    pub line: u32,
    pub text: String,
}

/// What one search answers: the names, then the lines, then whether a
/// cap cut it short.
#[derive(Debug, Clone, Serialize, TS)]
pub struct Found {
    pub names: Vec<String>,
    pub lines: Vec<SearchHit>,
    pub more: bool,
}

/// Everything under one directory on one machine that holds `query` —
/// the call `khor search` makes (`khor_node::Link::search`).
///
/// **The walk happens on the machine that holds the files.** A face
/// that listed the tree and matched here would move a whole project
/// across the wire to look for one word, which is the shape this layer
/// refuses for the git marks and for `touched` as well.
pub async fn search(root: &Path, machine: &str, dir: &str, query: &str) -> Result<Found, String> {
    let n = Node::open(root.to_path_buf())?;
    let found = n.search(machine, dir, query).await?;
    Ok(Found {
        names: found.names,
        lines: found
            .lines
            .into_iter()
            .map(|h| SearchHit {
                path: h.path,
                line: h.line,
                text: h.text,
            })
            .collect(),
        more: found.more,
    })
}

/// Whether the browser this face runs in will draw a PDF itself
/// (`khor_node::preview::PDF_HERE`). False on Linux (WebKitGTK ships no
/// viewer) and Android, where khor shows the PDF's words instead and
/// says that is what they are.
pub fn pdf_here() -> bool {
    khor_node::preview::PDF_HERE
}

/// Whether **this** machine — the one khor's backend is running on —
/// has a recycle bin (`khor_node::files::TRASH_HERE`). False on iOS and
/// Android, where 删除 on a local file is as final as it is on a far
/// one and the sentence must say so.
pub fn trash_here() -> bool {
    khor_node::files::TRASH_HERE
}

/// Whether this machine can show a file in its file manager with the
/// file selected (round CHAT, a conversation's file 「在访达中显示」):
/// a Mac whose opener is its own `open`. Anywhere else the button is not
/// drawn — the same word would be wrong on the next platform, and a
/// smoke run's `KHOR_LAUNCHER` stand-in opens nothing to select in.
pub fn reveal_here() -> bool {
    cfg!(target_os = "macos") && khor_node::files::launcher().as_deref() == Some("open")
}

/// Shows `path`, a file on this machine, in Finder with it selected
/// (`open -R`). The same whitelist every opener call asks first
/// (`khor_node::files::openable`): absolute, and never a flag.
pub fn reveal(path: &str) -> Result<(), String> {
    khor_node::files::openable(path)?;
    if !reveal_here() {
        return Err(khor_catalog::msg::no_launcher(khor_node::files::LAUNCHER_ENV));
    }
    std::process::Command::new("open")
        .arg("-R")
        .arg(path)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| khor_catalog::msg::cant_launch(path, e))
}

/// How much of a file gets sniffed for the NUL that means binary. git's
/// own number, and the reason to borrow it rather than pick one: the
/// question「is this text」has no true answer, only a convention, and a
/// convention people already share beats a fresh judgment call.
const SNIFF: usize = 8000;

/// One file as a screen can use it.
#[derive(Debug, Clone, Serialize, TS)]
pub struct FileText {
    /// The whole file's size, whether or not all of it is here — the
    /// only thing that says the view is partial.
    #[ts(type = "number")]
    pub total: u64,
    #[ts(type = "number")]
    pub at_ms: u64,
    /// As much as one slice held, decoded. Empty when `binary`.
    pub text: String,
    /// More of the file exists than `text` holds.
    pub cut: bool,
    /// Not text at all, so nothing was decoded.
    pub binary: bool,
    /// What the file's bytes hash to (`khor_node::files::digest`), or
    /// empty when there is nothing here that could be written back.
    ///
    /// **It is the permission to save, not just an identifier.** A save
    /// hands this back and the far machine refuses unless the file
    /// still hashes to it, so a view holding an empty one has nothing
    /// to offer — which is exactly right for the two kinds of file that
    /// carry one: a binary, and a file the slice cut short. Writing
    /// either of those back would be writing something that is not
    /// what is on screen.
    pub digest: String,
}

/// One file off any machine, for looking at (`khor_node::Node::
/// read_head`). One slice, never a loop — the node's own head says why
/// and what it measured.
///
/// # Cut is not the same as binary, and one byte decides it
///
/// A slice ends where 256 KB ends, which can be **inside** a multi-byte
/// character. Handed to a decoder that only answers yes or no, that
/// file comes back「not text」and a person is told their Rust source is
/// a binary. `Utf8Error` separates the two exactly:
///
/// - `error_len() == None` — the input ends mid-character and
///   everything before it was valid. That is the slice boundary, not
///   the file. Keep what decoded.
/// - `error_len() == Some(_)` — a byte sequence that cannot be text no
///   matter what follows it.
///
/// # Why a decoder alone is not enough
///
/// NUL is a perfectly valid UTF-8 character, so a file full of them
/// decodes without complaint — and files full of them are exactly what
/// most binaries are. The sniff above is the second half of the answer,
/// and without it the first half passes an executable to a code view.
pub async fn read_text(root: &Path, machine: &str, path: &str) -> Result<FileText, String> {
    let n = Node::open(root.to_path_buf())?;
    let (total, at_ms, bytes) = n.read_head(machine, path).await?;
    Ok(shape(total, at_ms, &bytes))
}

/// The whole judgment, with no machine in it — which is why it is a
/// function of its own. Whether a slice is text, and whether it was
/// cut, is decided from the bytes and the size; a test that had to
/// stand up a node to ask that would be testing the dial instead.
fn shape(total: u64, at_ms: u64, bytes: &[u8]) -> FileText {
    let cut = (bytes.len() as u64) < total;
    let no = |binary| FileText {
        total,
        at_ms,
        text: String::new(),
        cut,
        binary,
        digest: String::new(),
    };
    if bytes.iter().take(SNIFF).any(|b| *b == 0) {
        return no(true);
    }
    let text = match std::str::from_utf8(bytes) {
        Ok(s) => s.to_owned(),
        // The boundary, not the file — see the head of `read_text`.
        // **Only when the slice really ended early.** An *uncut* file
        // that stops mid-character is not a boundary artefact, it is a
        // file whose last bytes are not text; keeping the valid prefix
        // there would put something on screen that is not the file, and
        // since 工作区批4 what is on screen is what a save writes back.
        Err(e) if e.error_len().is_none() && cut => {
            String::from_utf8_lossy(&bytes[..e.valid_up_to()]).into_owned()
        }
        Err(_) => return no(true),
    };
    // **The digest is the invariant, spelled as code.** It is handed
    // out only when `text` is byte-for-byte the whole file, which is
    // precisely when writing it back loses nothing — so「can this be
    // saved」is never a second judgment that could disagree with this
    // one.
    let digest = if cut {
        String::new()
    } else {
        khor_node::files::digest(bytes)
    };
    FileText {
        total,
        at_ms,
        text,
        cut,
        binary: false,
        digest,
    }
}

/// One file shaped for a screen that is not a code view.
#[derive(Debug, Clone, Serialize, TS)]
pub struct Preview {
    /// `image` · `svg` · `pdf` · `doc` · `text` · `other` — what the
    /// screen should do with this. **Decided in the node** so `khor`
    /// and the app never disagree about what a file is.
    pub kind: String,
    #[ts(type = "number")]
    pub total: u64,
    /// `image`, `svg` and `pdf`: the bytes, base64, ready for the
    /// browser to decode. Empty otherwise — **and empty on a `pdf`
    /// means the words are in `html` instead**, because this platform's
    /// browser has no PDF viewer (`khor_node::preview::PDF_HERE`). One
    /// kind, two shapes, and which one it is, is which field is full.
    pub data: String,
    /// The media type those bytes need to be announced with, or empty.
    /// **Carried rather than derived on the far side**: a `data:` URL
    /// with no media type is rendered only by sniffing, and a browser
    /// that sniffs wrong draws a broken-image glyph with no sentence.
    pub media: String,
    /// `doc`, and a `pdf` on a platform that cannot draw one: the file
    /// rendered to HTML, already escaped (`khor_node::preview::html`).
    /// Empty otherwise.
    pub html: String,
    /// The file is bigger than a preview carries and this is its
    /// beginning. **An image that is cut is not shown at all** — half a
    /// picture is a broken picture, where half a document is still a
    /// document.
    pub cut: bool,
    /// The file parsed and holds no words. A real answer, not a
    /// failure: a blank template does exactly this.
    pub empty: bool,
}

/// One file off any machine, shaped for whatever it turns out to be.
///
/// **The node decides the kind and does the rendering**; this layer
/// carries the answer and base64s the two kinds the browser draws
/// itself. Doing the render here would be a second renderer for `khor`
/// to disagree with (docs/KHOR.md, CLI 与 GUI 等价).
pub async fn preview(root: &Path, machine: &str, path: &str) -> Result<Preview, String> {
    use base64::Engine as _;
    let n = Node::open(root.to_path_buf())?;
    let kind = khor_node::preview::kind(path);
    let (total, _, bytes, cut) = n
        .read_all(machine, path, khor_node::preview::cap(path))
        .await?;
    let mut out = Preview {
        kind: match kind {
            khor_node::preview::Kind::Image => "image",
            khor_node::preview::Kind::Svg => "svg",
            khor_node::preview::Kind::Pdf => "pdf",
            khor_node::preview::Kind::Doc => "doc",
            khor_node::preview::Kind::Text => "text",
            khor_node::preview::Kind::Other => "other",
        }
        .to_owned(),
        total,
        data: String::new(),
        media: khor_node::preview::media(path).to_owned(),
        html: String::new(),
        cut,
        empty: false,
    };
    match kind {
        khor_node::preview::Kind::Image | khor_node::preview::Kind::Svg
            // Half a picture is a broken picture, so a cut one carries
            // no bytes at all and the face says why.
            if !cut => {
                out.data = base64::engine::general_purpose::STANDARD.encode(&bytes);
            }
        khor_node::preview::Kind::Doc => {
            let md = khor_node::preview::markdown(path, &bytes)?;
            out.empty = md.trim().is_empty();
            out.html = khor_node::preview::html(&md);
        }
        // **The bytes where the browser can draw them, the words where
        // it cannot** (工作区批⑤). Half a PDF is a broken PDF, so a cut
        // one carries no bytes and the face says why — the same rule a
        // picture is under; the words road is left to the fallback,
        // which does not need the whole file to be useful but would
        // then be answering about half a document without saying so.
        khor_node::preview::Kind::Pdf => {
            if khor_node::preview::PDF_HERE {
                if !cut {
                    out.data = base64::engine::general_purpose::STANDARD.encode(&bytes);
                }
            } else {
                let md = khor_node::preview::markdown(path, &bytes)?;
                out.empty = md.trim().is_empty();
                out.html = khor_node::preview::html(&md);
            }
        }
        _ => {}
    }
    Ok(out)
}

/// What a save answers: where the file stands now.
#[derive(Debug, Clone, Serialize, TS)]
pub struct Wrote {
    #[ts(type = "number")]
    pub at_ms: u64,
    /// The new digest, so the view can keep editing and save again
    /// without reading the file back.
    pub digest: String,
}

/// Saves a whole file onto any machine (`khor_node::Node::write_path`).
///
/// `was` is the `digest` the view was handed when it read the file. It
/// is required and never defaulted: an empty one means「there should be
/// no file here」, which is a different and much stronger claim than
/// 「I did not bother to check」, and the second one has no spelling on
/// purpose.
pub async fn write_text(
    root: &Path,
    machine: &str,
    path: &str,
    was: &str,
    text: &str,
) -> Result<Wrote, String> {
    let n = Node::open(root.to_path_buf())?;
    let (at_ms, digest) = n.write_path(machine, path, was, text.as_bytes()).await?;
    Ok(Wrote { at_ms, digest })
}

/// The three sides of a refused save, merged (`khor_node::merge`).
#[derive(Debug, Clone, Serialize, TS)]
pub struct Merged {
    /// The merged text, conflict markers and all.
    pub text: String,
    /// How many places khor would not choose between. `0` means there
    /// is nothing left for the person to decide.
    pub clashes: u32,
}

/// Merges the copy in the box over the copy on the machine, against the
/// copy the person started from.
///
/// **No machine and no path, because nothing is written.** The three
/// texts are already in front of the caller — the base and the edit are
/// the view's own, and 「theirs」 is what `read_text` just answered —
/// so this is a function, not a road to another machine. The save that
/// follows is the ordinary `write_text`, pinned to the digest that came
/// with 「theirs」, and it only happens when somebody presses save.
pub fn merge_text(base: &str, mine: &str, theirs: &str) -> Merged {
    let m = khor_node::merge::three(base, mine, theirs);
    Merged {
        text: m.text,
        clashes: m.clashes,
    }
}

#[cfg(test)]
mod read_text_tests {
    use super::*;

    /// A slice ends where 256 KB ends, and a character can straddle
    /// that. **This is the case that decides whether a person is told
    /// their source file is a binary**, and it is built here the way it
    /// happens in life: filler up to one byte short of the boundary,
    /// then a three-byte character that cannot fit.
    #[test]
    fn a_character_cut_by_the_slice_is_not_a_binary_file() {
        let slice = khor_node::proto::SLICE as usize;
        let mut whole = vec![b'a'; slice - 1];
        whole.extend("中".as_bytes());
        let head = &whole[..slice];

        let f = shape(whole.len() as u64, 0, head);
        assert!(!f.binary, "a cut character must not read as binary");
        assert!(f.cut, "and it must say it was cut");
        assert_eq!(
            f.text.len(),
            slice - 1,
            "what decoded is everything before the character"
        );
        assert!(
            f.text.chars().all(|c| c == 'a'),
            "and no replacement character was kept"
        );
    }

    /// NUL is a valid UTF-8 character, so a decoder alone says yes to a
    /// file that is binary in exactly the way most binaries are.
    #[test]
    fn nul_makes_it_binary_even_though_it_decodes() {
        let bytes = b"\x7fELF\0\0\0 and more".to_vec();
        assert!(
            std::str::from_utf8(&bytes).is_ok(),
            "the premise: this decodes"
        );
        let f = shape(bytes.len() as u64, 0, &bytes);
        assert!(f.binary, "the sniff is what catches it");
        assert!(f.text.is_empty(), "and nothing is handed to a code view");
    }

    /// The ordinary case, and the one that proves `cut` is not just
    /// always true.
    #[test]
    fn a_whole_small_file_is_text_and_says_it_is_whole() {
        let bytes = "fn main() {}\n中文也在这里\n".as_bytes();
        let f = shape(bytes.len() as u64, 7, bytes);
        assert!(!f.binary);
        assert!(!f.cut, "everything arrived");
        assert_eq!(f.text, String::from_utf8(bytes.to_vec()).unwrap());
        assert_eq!(f.at_ms, 7, "the change contract rides through untouched");
    }

    /// A byte sequence no continuation can rescue, away from the end.
    /// Told apart from the case above by `error_len`, which is the
    /// whole of the distinction.
    #[test]
    fn invalid_bytes_in_the_middle_are_binary() {
        let bytes = vec![b'o', b'k', 0xC3, 0x28, b'm', b'o', b'r', b'e'];
        let f = shape(bytes.len() as u64, 0, &bytes);
        assert!(
            f.binary,
            "not a boundary — nothing that follows makes this text"
        );
    }

    /// The same incomplete character as the first test, but the file
    /// really does end there. **The slice is not what cut it, so there
    /// is nothing to be lenient about** — and being lenient would put
    /// text on screen that a save would then write back over bytes it
    /// never saw (工作区批4).
    #[test]
    fn an_incomplete_character_at_the_end_of_a_whole_file_is_binary() {
        let bytes = vec![b'h', b'i', 0xE4, 0xB8];
        let f = shape(bytes.len() as u64, 0, &bytes);
        assert!(!f.cut, "the premise: everything the file has is here");
        assert!(
            f.binary,
            "and what it has is not text all the way to the end"
        );
        assert!(f.digest.is_empty(), "so there is nothing to offer a save");
    }

    /// The invariant the save rests on, asserted rather than described:
    /// a digest exists exactly when the text on screen is the whole
    /// file, byte for byte.
    #[test]
    fn a_digest_is_handed_out_only_when_the_text_is_the_whole_file() {
        let whole = "fn main() {}\n".as_bytes();
        let f = shape(whole.len() as u64, 0, whole);
        assert_eq!(f.digest, khor_node::files::digest(whole));
        assert_eq!(f.text.as_bytes(), whole, "and it is the same bytes");

        // Cut: the text is a prefix, so writing it back would truncate.
        let g = shape(whole.len() as u64 + 10, 0, whole);
        assert!(g.cut);
        assert!(g.digest.is_empty(), "a prefix must not be savable");

        // Binary: nothing was decoded at all.
        let h = shape(4, 0, b"\x7fE\0F");
        assert!(h.binary);
        assert!(h.digest.is_empty());
    }
}
