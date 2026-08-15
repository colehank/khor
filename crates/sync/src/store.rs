//! Persistence for CRDT documents: a stack of append-only block files
//! anyone can read or write.
//!
//! ```text
//! <dir>/
//!     u-<author:016x>-<seq:08x>.loro   ← increment block, immutable once written
//!     snap-<seq:08x>.loro              ← compaction product (see `compact`)
//! ```
//!
//! One file per batch, author in the name: two writers on the same
//! directory never conflict at the filesystem level (the Maildir answer).
//! Loro increments merge in any order, so "immutable files + merge them
//! all" is the entire consistency story.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use khor_catalog::msg;
use loro::VersionVector;

use crate::plan::{Ledger, Side};

/// Block file extension.
const EXT: &str = "loro";

/// The merged-blocks ledger. No `.loro` extension, so it is neither read
/// as a block nor synced to the far side — each machine keeps its own.
const LEDGER: &str = ".merged";

/// What the block store and the wire need from a CRDT document. One
/// implementation per replicated table (chat, devices, …).
pub trait Doc: Sized {
    fn open(peer: u64) -> Result<Self, String>;
    fn peer_id(&self) -> u64;
    fn version(&self) -> VersionVector;
    /// What this doc has beyond `theirs` — the transferable unit: a wire
    /// frame or a file, both work.
    fn changes_since(&self, theirs: &VersionVector) -> Result<Vec<u8>, String>;
    fn snapshot(&self) -> Result<Vec<u8>, String>;
    /// Idempotent: the same bytes twice equal once.
    fn merge(&self, bytes: &[u8]) -> Result<(), String>;
    /// Item count for humans (messages, devices, …).
    fn items(&self) -> usize;
}

/// One document's stack on disk.
pub struct BlockStore {
    dir: PathBuf,
    /// Versions already on disk. `flush` writes only what came after —
    /// without this every flush rewrites all history, and "append-only"
    /// silently turns quadratic.
    on_disk: VersionVector,
    /// Blocks ever merged. Compaction deletes files, never ledger lines —
    /// see the `plan` module head.
    ledger: Ledger,
}

/// What `load` returns.
pub struct Loaded<D> {
    pub store: BlockStore,
    pub doc: D,
    /// Files that would not read. Counted, not swallowed: a bad block is
    /// a missing slice of data the UI must be able to mention; one bad
    /// block must not kill the whole document either.
    pub broken: Vec<PathBuf>,
}

/// Reads a document back from disk. No directory = empty document, not
/// an error.
pub fn load<D: Doc>(dir: &Path, peer: u64) -> Result<Loaded<D>, String> {
    let doc = D::open(peer)?;
    let mut broken = Vec::new();
    let mut ledger = read_ledger(dir);

    for p in blocks(dir)? {
        match fs::read(&p) {
            Ok(bytes) => {
                if doc.merge(&bytes).is_err() {
                    broken.push(p);
                } else if let Some(n) = file_name(&p) {
                    ledger.insert(n);
                }
            }
            Err(_) => broken.push(p),
        }
    }
    let on_disk = doc.version();
    let mut store = BlockStore {
        dir: dir.to_path_buf(),
        on_disk,
        ledger,
    };
    // Only write the ledger if the directory exists: an empty document
    // should leave nothing on disk.
    if dir.exists() {
        store.save_ledger()?;
    }
    Ok(Loaded { store, doc, broken })
}

impl BlockStore {
    /// My side of the books, for [`crate::plan::plan`].
    pub fn side(&self) -> Result<Side, String> {
        Ok(Side::new(
            blocks(&self.dir)?
                .iter()
                .filter_map(|p| file_name(p))
                .collect::<Vec<_>>(),
            self.ledger.names().iter().cloned().collect::<Vec<_>>(),
        ))
    }

    /// Takes in a block pulled from the far side: merge, write, then
    /// ledger — in that order. Reversed, a mid-way failure marks a block
    /// merged that never was; it is never pulled again and a slice of
    /// data goes permanently missing, with no error anywhere.
    pub fn absorb<D: Doc>(&mut self, doc: &D, name: &str, bytes: &[u8]) -> Result<(), String> {
        doc.merge(bytes)?;
        make_dir(&self.dir)?;
        write_atomic(&self.dir.join(name), bytes)?;
        self.ledger.insert(name);
        self.save_ledger()?;
        self.on_disk = doc.version();
        Ok(())
    }

    /// Reads a block to push to the far side.
    pub fn read_block(&self, name: &str) -> Result<Vec<u8>, String> {
        fs::read(self.dir.join(name)).map_err(|e| msg::cant_read(name, e))
    }

    fn save_ledger(&mut self) -> Result<(), String> {
        write_atomic(&self.dir.join(LEDGER), self.ledger.render().as_bytes())
    }

    /// Writes what the doc has beyond `on_disk` as one new block; `None`
    /// when there is nothing new.
    ///
    /// The emptiness test is version-vector equality, not
    /// `delta.is_empty()`: loro exports a non-empty header block even with
    /// zero new ops, and judging by emptiness piles up thousands of
    /// "empty" files while everything appears to work.
    pub fn flush<D: Doc>(&mut self, doc: &D) -> Result<Option<PathBuf>, String> {
        let now = doc.version();
        if now == self.on_disk {
            return Ok(None);
        }
        let delta = doc.changes_since(&self.on_disk)?;
        if delta.is_empty() {
            return Ok(None);
        }
        make_dir(&self.dir)?;
        let path = self.dir.join(format!(
            "u-{:016x}-{:08x}.{EXT}",
            doc.peer_id(),
            self.next_seq(doc.peer_id())?
        ));
        write_atomic(&path, &delta)?;
        // Own blocks enter the ledger too: the far side may push one right
        // back, and an unledgered copy would be re-pulled on every sync.
        if let Some(n) = file_name(&path) {
            self.ledger.insert(n);
        }
        self.save_ledger()?;
        self.on_disk = doc.version();
        Ok(Some(path))
    }

    /// Next unused sequence number for this author, counted from disk,
    /// not memory: two processes of one device (GUI + a CLI run) counting
    /// in memory would collide, and the collision silently overwrites a
    /// block.
    fn next_seq(&self, peer: u64) -> Result<u32, String> {
        let prefix = format!("u-{peer:016x}-");
        let mut max = None::<u32>;
        if let Ok(rd) = fs::read_dir(&self.dir) {
            for e in rd.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                let Some(rest) = name.strip_prefix(&prefix) else {
                    continue;
                };
                let Some(num) = rest.strip_suffix(&format!(".{EXT}")) else {
                    continue;
                };
                if let Ok(n) = u32::from_str_radix(num, 16) {
                    max = Some(max.map_or(n, |m: u32| m.max(n)));
                }
            }
        }
        Ok(max.map_or(0, |m| m + 1))
    }

    /// Compaction: write current state as one snapshot block, then delete
    /// the increments it covers. Write-then-delete, never reversed — a
    /// crash in between must cost a few spare KB, not the whole document.
    /// The ledger keeps every line, or the next sync would pull the
    /// compacted blocks right back, every time.
    pub fn compact<D: Doc>(&mut self, doc: &D) -> Result<PathBuf, String> {
        make_dir(&self.dir)?;
        let old = blocks(&self.dir)?;
        let snap = doc.snapshot()?;
        let path = self
            .dir
            .join(format!("snap-{:08x}.{EXT}", self.next_seq(doc.peer_id())?));
        write_atomic(&path, &snap)?;
        for p in old {
            let _ = fs::remove_file(p);
        }
        if let Some(n) = file_name(&path) {
            self.ledger.insert(n);
        }
        self.save_ledger()?;
        self.on_disk = doc.version();
        Ok(path)
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }
}

/// All blocks, sorted by name. Order doesn't affect the merge result
/// (increments commute); sorting exists so two loads behave identically
/// and failures reproduce.
fn blocks(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let Ok(rd) = fs::read_dir(dir) else {
        return Ok(Vec::new()); // no directory = empty document
    };
    let mut out: Vec<PathBuf> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == EXT))
        .collect();
    out.sort();
    Ok(out)
}

/// Blocks get copied to other machines and their mode travels along. A
/// block is the document itself — the bytes reconstruct all of it — so
/// the create() default (644 under common umask) leaks it on any
/// shared-home machine. "Not plaintext" is no protection.
#[cfg(unix)]
const OWNER_ONLY_FILE: u32 = 0o600;
/// Directories likewise: 755 lets anyone cd in and list.
#[cfg(unix)]
const OWNER_ONLY_DIR: u32 = 0o700;

fn make_dir(dir: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        // .mode() applies only to directories actually created here; a
        // pre-existing 755 directory stays 755.
        return fs::DirBuilder::new()
            .recursive(true)
            .mode(OWNER_ONLY_DIR)
            .create(dir)
            .or_else(|e| {
                if dir.is_dir() { Ok(()) } else { Err(msg::cant_make_dir(e)) }
            });
    }
    #[cfg(not(unix))]
    fs::create_dir_all(dir).map_err(msg::cant_make_dir)
}

/// tmp + sync + rename: readers see a whole block or none. The sync
/// matters — without it the rename can reach disk before the content, and
/// a crash leaves a right-sized block full of zeros that reads fine.
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension("tmp");
    {
        let mut opts = fs::OpenOptions::new();
        opts.create(true).write(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(OWNER_ONLY_FILE);
        }
        let mut f = opts
            .open(&tmp)
            .map_err(|e| msg::cant_write(tmp.display(), e))?;
        f.write_all(bytes).map_err(msg::cant_finish_write)?;
        f.sync_all().map_err(msg::cant_sync_disk)?;
    }
    fs::rename(&tmp, path).map_err(msg::cant_rename)
}

fn file_name(p: &Path) -> Option<String> {
    Some(p.file_name()?.to_string_lossy().to_string())
}

fn read_ledger(dir: &Path) -> Ledger {
    // Unreadable = empty ledger, not an error: the cost is one idempotent
    // re-pull, while erroring here would brick a fresh document.
    fs::read_to_string(dir.join(LEDGER))
        .map(|t| Ledger::parse(&t))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests;
