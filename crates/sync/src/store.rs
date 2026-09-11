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
use std::sync::atomic::{AtomicU64, Ordering};

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

/// How many documents this process has read back from disk, and how
/// long those reads took in total.
///
/// **The same instrument `khor_node::adaptor::snapshots_taken` is**, and
/// it exists for the same reason: 2026-09-11 a session list was found to
/// be reading one channel's document twice per beat, and the call graph
/// had been read twice without anyone noticing. A counter answers
/// 「how many」 without anybody having to trace it, and the cost test
/// prints it beside the time.
///
/// Nanoseconds rather than a duration so the pair is two plain atomics;
/// the reader divides.
static LOADS: AtomicU64 = AtomicU64::new(0);
static LOAD_NANOS: AtomicU64 = AtomicU64::new(0);

static RELISTS: AtomicU64 = AtomicU64::new(0);

/// Running count of [`load`] calls in this process.
pub fn loads_taken() -> u64 {
    LOADS.load(Ordering::Relaxed)
}

/// How many reads found a listed block already deleted and listed again
/// (see [`load_inner`]).
///
/// **A test that races a compaction needs this to know it raced one.**
/// Without it, a run where the timing never overlapped passes exactly
/// like a run where the re-listing worked, and the two are not the same
/// claim.
pub fn relists_after_a_vanish() -> u64 {
    RELISTS.load(Ordering::Relaxed)
}

/// Total time spent inside [`load`] in this process, in nanoseconds.
pub fn load_nanos() -> u64 {
    LOAD_NANOS.load(Ordering::Relaxed)
}

/// Reads a document back from disk. No directory = empty document, not
/// an error.
///
/// ⚠️ **Not a lookup — every call opens an empty document, reads every
/// block file in `dir`, and merges each one in.** So a caller that wants
/// two things out of one document must ask once and derive both, and a
/// caller on a poll beat pays this per beat. [`loads_taken`] is how that
/// gets checked rather than argued about.
pub fn load<D: Doc>(dir: &Path, peer: u64) -> Result<Loaded<D>, String> {
    let started = std::time::Instant::now();
    let out = load_inner(dir, peer);
    LOADS.fetch_add(1, Ordering::Relaxed);
    LOAD_NANOS.fetch_add(started.elapsed().as_nanos() as u64, Ordering::Relaxed);
    out
}

fn load_inner<D: Doc>(dir: &Path, peer: u64) -> Result<Loaded<D>, String> {
    let (loaded, vanished) = read_once(dir, peer)?;
    if !vanished {
        return Ok(loaded);
    }
    // **A block that was listed and then was not there is compaction,
    // not damage**: [`BlockStore::compact`] writes the snapshot first
    // and deletes the increments after, so a reader that listed before
    // the delete and read after it finds holes — and every byte it
    // missed is inside the snapshot that now sits in the same
    // directory. Listing again is the whole fix.
    //
    // ⚠️ **Without this, compaction is unsafe to run at all**, which is
    // the state this file was in until 2026-09-11: those reads landed in
    // `broken`, which is a count and not an error, so a document would
    // come back missing a slice of itself **and say nothing**. Anyone
    // switching compaction on without this would see it as data that
    // flickers.
    //
    // Once, not in a loop. A second miss means writes are landing faster
    // than a read can finish, and at that point reporting what was found
    // beats spinning — the next read is two seconds away.
    RELISTS.fetch_add(1, Ordering::Relaxed);
    Ok(read_once(dir, peer)?.0)
}

/// One pass over a document's blocks. The flag is 「a block on the
/// listing was gone by the time we read it」 — see [`load_inner`].
fn read_once<D: Doc>(dir: &Path, peer: u64) -> Result<(Loaded<D>, bool), String> {
    let doc = D::open(peer)?;
    let mut broken = Vec::new();
    let mut ledger = read_ledger(dir);
    let mut vanished = false;
    // Whether anything was learned that the ledger on disk does not
    // already say. See the write at the end.
    let mut grew = false;

    for p in blocks(dir)? {
        match fs::read(&p) {
            Ok(bytes) => {
                if doc.merge(&bytes).is_err() {
                    broken.push(p);
                } else if let Some(n) = file_name(&p) {
                    grew |= ledger.insert(n);
                }
            }
            // Only a missing file is a compaction; anything else is a
            // block this reader genuinely could not read, and those are
            // what `broken` is for.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => vanished = true,
            Err(_) => broken.push(p),
        }
    }
    let on_disk = doc.version();
    let mut store = BlockStore {
        dir: dir.to_path_buf(),
        on_disk,
        ledger,
    };
    // Only write the ledger if the directory exists (an empty document
    // should leave nothing on disk) **and if this read learned a block
    // name the file did not already hold**.
    //
    // ⚠️ **That second half is worth 4 ms per read, which was most of
    // what a session list cost** (2026-09-11). A read that finds nothing
    // new still rewrote the ledger, and `write_atomic` is a temp file, a
    // write and a rename — so the steady state, where every block is
    // already listed, paid three syscalls to save what was already
    // there. Measured per document: `chat/Mac-mini`, **one block, 356
    // bytes, 4.7 ms**, against 0.01 ms for a document whose directory
    // does not exist and therefore skipped this write. The cost was
    // never the data.
    //
    // The ledger only ever grows (compaction deletes blocks but keeps
    // their lines, or a sync would pull them back), so 「nothing new」
    // is exactly 「the file on disk is still right」.
    if dir.exists() && grew {
        store.save_ledger()?;
    }
    Ok((Loaded { store, doc, broken }, vanished))
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
///
/// **The temp name carries the writer, not just the target**, or two
/// writers rename each other's work in progress away. One device
/// runs several khor processes at once (a resident serve, the app's
/// backend, a CLI verb) and they all open the same home, so they write
/// the same ledger. With one temp name per target the sequence is: both
/// create it, both write it, the first renames it into place, and the
/// second's rename finds nothing there — surfacing as "cannot rename: No
/// such file or directory" from whichever verb happened to be running,
/// never as a message about writers colliding. Measured before this
/// line changed: **17 of 40** GUI pin requests failed that way while a
/// CLI loop opened the same home; the pin silently did nothing.
///
/// Threads of one process need it too (the app's backend and its
/// embedded serve are two threads on one pid), hence the counter beside
/// the pid.
///
/// This is the rule block files already follow — author in the name, the
/// module head's Maildir answer — which had simply never reached the
/// temp files.
///
/// A process dying mid-write now leaves its temp behind instead of
/// overwriting one. That is a small orphan, not a corrupt document:
/// temps do not carry the block extension, so nothing reads one back.
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let tmp = path.with_extension(format!(
        "tmp.{}.{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
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

/// A document of **no particular kind**.
///
/// Every loro-backed table in this crate exports, imports and snapshots
/// through the same four functions (`glue`), so a job that only wants to
/// rewrite a pile of blocks as one snapshot never needs to know which
/// table it is looking at. That is the whole reason this exists: the
/// alternative is a list of every document type, kept by hand, whose one
/// failure mode is 「somebody added a table and this list did not」 —
/// silent, and it grows worse the longer nobody notices.
///
/// ⚠️ **Not for reading.** It holds no accessors, so it cannot answer a
/// question about contents; `items` is 0 because a count of nothing in
/// particular is a number that would be read as meaning something.
pub struct AnyDoc(loro::LoroDoc);

impl Doc for AnyDoc {
    fn open(peer: u64) -> Result<Self, String> {
        Ok(Self(crate::glue::with_peer(peer)?))
    }
    fn peer_id(&self) -> u64 {
        self.0.peer_id()
    }
    fn version(&self) -> VersionVector {
        self.0.oplog_vv()
    }
    fn changes_since(&self, theirs: &VersionVector) -> Result<Vec<u8>, String> {
        crate::glue::changes_since(&self.0, theirs)
    }
    fn snapshot(&self) -> Result<Vec<u8>, String> {
        crate::glue::snapshot(&self.0)
    }
    fn merge(&self, bytes: &[u8]) -> Result<(), String> {
        crate::glue::merge(&self.0, bytes)
    }
    fn items(&self) -> usize {
        0
    }
}

/// How many blocks a document may pile up before compacting it is worth
/// the write.
///
/// **Measured, not chosen for roundness** (2026-09-11): this machine's
/// device table had grown to 557 blocks over twelve days and took 20 ms
/// to read; compacted to 13 files it took 4.5 ms. The session list reads
/// ten documents on a 2 s beat, so that pile was most of what the
/// desktop app burned sitting still. 64 is low enough that no document
/// reaches a tenth of 557 and high enough that an ordinary day of
/// writing does not trigger a rewrite.
pub const PILE: usize = 64;

/// What [`compact_dir`] did.
pub struct Compacted {
    pub before: usize,
    pub after: usize,
}

/// Rewrite one document's pile of blocks as a single snapshot, if it has
/// piled up past [`PILE`]. `Ok(None)` means it had not.
///
/// ⚠️ **Exactly one process may call this for a given directory.** Two
/// compactors on one document destroy it: the second reads a directory
/// the first is half way through deleting, gets an empty document, and
/// writes an empty snapshot over everything. That is not a race that
/// careful reading can survive — it is a lost write — and it is why this
/// belongs to the resident serve and to nothing else
/// (`store::tests::a_read_that_races_a_compaction_still_sees_every_line`
/// has the measurement that found it).
///
/// Readers are a different matter and are safe: the snapshot is written
/// before anything is deleted, so a reader either lists it and holds the
/// whole document in one file, or lists before it and has `load`'s
/// re-listing underneath.
pub fn compact_dir(dir: &Path, peer: u64) -> Result<Option<Compacted>, String> {
    let before = blocks(dir)?.len();
    if before <= PILE {
        return Ok(None);
    }
    let loaded: Loaded<AnyDoc> = load(dir, peer)?;
    let mut store = loaded.store;
    store.compact(&loaded.doc)?;
    Ok(Some(Compacted { before, after: blocks(dir)?.len() }))
}

/// Compact every piled-up document under `home`, two directories deep.
///
/// Two deep because that is how this home is laid out: `devices`,
/// `seen`, `recents` and their kind sit directly under `.khor`, while a
/// conversation is `chat/<machine>`. A directory counts as a document
/// when it holds a block file, so anything that is not one is skipped
/// without having to be named.
///
/// Returns what it rewrote, loudest first — a caller that logs this is
/// the only record that compaction ever ran.
///
/// ⚠️ **One process only** — see [`compact_dir`].
pub fn compact_piles_under(home: &Path, peer: u64) -> Vec<(PathBuf, Compacted)> {
    let mut out = Vec::new();
    let Ok(top) = fs::read_dir(home) else {
        return out;
    };
    for e in top.flatten() {
        let p = e.path();
        if !p.is_dir() {
            continue;
        }
        match compact_dir(&p, peer) {
            Ok(Some(c)) => out.push((p.clone(), c)),
            // A directory that is not a document, or one that has not
            // piled up. Both are the ordinary answer.
            Ok(None) => {}
            // ⚠️ **Swallowed on purpose, and this is the one place in
            // this file where that is right.** Compaction is an
            // optimisation with no user waiting on it; a document that
            // will not rewrite today is a document that is still read
            // perfectly well, just slowly. Failing the caller — the
            // resident serve's tick — would trade a slow read for a
            // dead sync.
            Err(_) => continue,
        }
        let Ok(inner) = fs::read_dir(&p) else { continue };
        for e in inner.flatten() {
            let q = e.path();
            if q.is_dir() {
                if let Ok(Some(c)) = compact_dir(&q, peer) {
                    out.push((q, c));
                }
            }
        }
    }
    out.sort_by_key(|(_, c)| std::cmp::Reverse(c.before));
    out
}
