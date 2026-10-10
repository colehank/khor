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

/// A pen's file, `.pen-<k>` ([`Lease`]): one writer name of this pile on
/// this machine. Not a block, not synced, like the ledger.
const PEN: &str = ".pen-";

/// The pile lock ([`load_pile`]): every open copy holds it shared, a
/// re-seed holds it alone. Not a block, not synced.
const PILE_LOCK: &str = ".pile";

/// Which re-seeding a pile's blocks belong to ([`load_pile`]). Not a
/// block, not synced: each machine re-seeds its own.
const EPOCH_FILE: &str = ".epoch";

/// The epoch this build keeps key-value piles in. A pile with no epoch
/// file is epoch 0, as every pile was before re-seeding; a chat channel
/// stays epoch 0 for good.
pub const EPOCH: u32 = 1;

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
    /// The block files whose contents this copy of the document holds:
    /// what it read, wrote and took in. Compaction deletes these and
    /// nothing else (round NET-6) — a block another writer added after
    /// this copy was read is not in its snapshot.
    held: std::collections::BTreeSet<String>,
    /// The epoch this copy's blocks belong to ([`load_pile`]); what the
    /// wire says beside its version.
    epoch: u32,
    /// An empty pile opened by [`load_pile`]: its epoch file is written
    /// before its first block.
    stamp: bool,
}

/// What `load` returns.
pub struct Loaded<D> {
    pub store: BlockStore,
    pub doc: D,
    /// Files that would not read. Counted, not swallowed: a bad block is
    /// a missing slice of data the UI must be able to mention; one bad
    /// block must not kill the whole document either.
    pub broken: Vec<PathBuf>,
    /// The writer name `doc` holds while it is open ([`Lease`]).
    lease: Lease,
}

/// **One writer name per open copy of a document** (round NET-6).
///
/// Loro names every op by its writer and a counter, and two copies of one
/// document writing under the same name at the same time mint the same
/// names for different ops; on import, an op whose name is already held
/// is skipped, so whichever block merges second loses its writes on every
/// read after — and no sync brings them back, because by version vector
/// every replica already has them. Loro's own rule (`LoroDoc::set_peer_id`,
/// loro 1.13): "Never reuse the same PeerID across concurrent writers …
/// Duplicate PeerIDs can produce conflicting OpIDs and corrupt the
/// document", and no fixed PeerID "without strict single-ownership
/// locking" — which a lease is.
///
/// A process used one name for every copy it opened, and a resident
/// answers requests concurrently: the `Pair` arm's copy of the device
/// table and any other write to it in flight lost one or the other. That
/// is how a pairing's row ended up only in the joiner's copy (the joiner
/// is handed the `Pair` arm's snapshot). So each open copy borrows a name
/// nobody else holds for that directory, and gives it back when it drops.
///
/// **The names are the pile's, not the process's** (PEERS-BRIEF,
/// 2026-10-10). Every process used to bring a fresh name, and every
/// process that wrote left a row in the version vector for good: the
/// owner's devices pile reached 1515 rows, recents 1129 with two ops
/// each, and every row costs on every sync request and (in loro 1.16.2)
/// on every merge. Now a directory that exists keeps a few pens beside
/// its blocks, `.pen-<k>`, each holding one name; a copy takes the lowest
/// pen that nobody holds, in this process or any other, by an OS lock on
/// the file, and holds the lock until it drops. The pile has as many
/// names per machine as copies were ever open at once, however many
/// processes come and go. The lock is taken before anything is read, so
/// the copy reads every block the pen's last holder flushed and its
/// counter carries on from theirs; the lock goes with the process, so a
/// crash frees it.
///
/// A name is minted when its pen is first used, not derived from the
/// machine: a pile that is deleted and made again gets new names, and a
/// machine whose identity was copied to a second machine shares none.
///
/// A directory not on disk yet has no pens. Its copy writes under the
/// caller's name, lent in this process only as before: one row per pile
/// created, and unique, so nobody else can hold it. Any pen that cannot
/// be opened, locked or read falls back the same way: a fresh name is
/// always safe, only unbounded.
///
/// A copy of a directory that exists also holds the pile lock shared, so
/// a re-seed ([`load_pile`]) never rebuilds a pile somebody has open.
struct Lease {
    peer: u64,
    held: Held,
    #[allow(dead_code)]
    pile: Option<fs::File>,
}

enum Held {
    /// A pen: its directory and number in this process's table, and the
    /// open file whose lock keeps every other process off it.
    Pen((PathBuf, usize), #[allow(dead_code)] fs::File),
    /// A name lent in this process only, for a directory with no pens.
    Spare((PathBuf, u64)),
}

/// How long a copy waits for a re-seed to let go of the pile. A re-seed
/// of the owner's devices pile (1515 writers) is well under a second.
const PILE_WAIT: std::time::Duration = std::time::Duration::from_secs(10);

/// More than this many copies of one pile open at once on one machine
/// and the next takes a spare name rather than a pen.
const PENS: usize = 64;

/// Pens out in this process, per directory: a second open of a locked
/// file in the same process is not refused on every platform.
static PENS_OUT: std::sync::Mutex<std::collections::BTreeMap<PathBuf, std::collections::BTreeSet<usize>>> =
    std::sync::Mutex::new(std::collections::BTreeMap::new());

/// For each directory and caller's name: every name lent for it, and
/// whether it is out.
type Lent = std::collections::BTreeMap<(PathBuf, u64), Vec<(u64, bool)>>;
static LENT: std::sync::Mutex<Lent> = std::sync::Mutex::new(std::collections::BTreeMap::new());

/// splitmix64: distinct inputs, well-spread names.
fn mix(seed: u64) -> u64 {
    let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// A name nobody else has: this process, this instant, a count. Not
/// `u64::MAX`, which loro keeps for itself.
fn fresh_name() -> u64 {
    static MINTED: AtomicU64 = AtomicU64::new(1);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let n = MINTED.fetch_add(1, Ordering::Relaxed);
    mix((u64::from(std::process::id()) << 40) ^ nanos ^ mix(n)).min(u64::MAX - 1)
}

impl Lease {
    fn take(dir: &Path, peer: u64) -> Lease {
        if !dir.is_dir() {
            return Lease::spare(dir, peer);
        }
        // Waits while a re-seed holds it: the copy then reads the new
        // pile, and takes a pen of the new pile's. Polled, not blocking:
        // a lock that never comes (a wedged NFS lock daemon) must not
        // hang every read, so after a while the copy goes on without it.
        let pile = open_owner_only(&dir.join(PILE_LOCK)).filter(|f| {
            let began = std::time::Instant::now();
            loop {
                match f.try_lock_shared() {
                    Ok(()) => return true,
                    Err(fs::TryLockError::WouldBlock) if began.elapsed() < PILE_WAIT => {
                        std::thread::sleep(std::time::Duration::from_millis(5))
                    }
                    Err(_) => return false,
                }
            }
        });
        let mut taken = None;
        for k in 0..PENS {
            match Lease::pen(dir, k) {
                Some(Ok(lease)) => {
                    taken = Some(lease);
                    break;
                }
                Some(Err(())) => break,
                None => {}
            }
        }
        let mut lease = taken.unwrap_or_else(|| Lease::spare(dir, peer));
        lease.pile = pile;
        lease
    }

    /// Pen `k`: `None` if somebody holds it, `Some(Err)` if pens do not
    /// work here at all.
    fn pen(dir: &Path, k: usize) -> Option<Result<Lease, ()>> {
        let slot = (dir.to_path_buf(), k);
        {
            let mut out = PENS_OUT.lock().unwrap_or_else(|p| p.into_inner());
            if !out.entry(dir.to_path_buf()).or_default().insert(k) {
                return None;
            }
        }
        let give_back = |r| {
            if let Some(ks) = PENS_OUT.lock().unwrap_or_else(|p| p.into_inner()).get_mut(dir) {
                ks.remove(&k);
            }
            r
        };
        let Some(mut file) = open_owner_only(&dir.join(format!("{PEN}{k}"))) else {
            return give_back(Some(Err(())));
        };
        match file.try_lock() {
            Ok(()) => {}
            Err(fs::TryLockError::WouldBlock) => return give_back(None),
            Err(fs::TryLockError::Error(_)) => return give_back(Some(Err(()))),
        }
        // Under the lock: the name the pen holds, or a new one written
        // and synced before any op can carry it.
        let mut text = String::new();
        let _ = std::io::Read::read_to_string(&mut file, &mut text);
        let peer = match u64::from_str_radix(text.trim(), 16) {
            Ok(peer) => peer,
            Err(_) => {
                let peer = fresh_name();
                use std::io::Seek;
                let written = file.set_len(0).is_ok()
                    && file.seek(std::io::SeekFrom::Start(0)).is_ok()
                    && file.write_all(format!("{peer:016x}\n").as_bytes()).is_ok()
                    && file.sync_all().is_ok();
                if !written {
                    return give_back(Some(Err(())));
                }
                peer
            }
        };
        Some(Ok(Lease { peer, held: Held::Pen(slot, file), pile: None }))
    }

    fn spare(dir: &Path, peer: u64) -> Lease {
        let key = (dir.to_path_buf(), peer);
        let mut lent = LENT.lock().unwrap_or_else(|p| p.into_inner());
        let names = lent.entry(key.clone()).or_insert_with(|| vec![(peer, false)]);
        let peer = match names.iter_mut().find(|(_, out)| !out) {
            Some((name, out)) => {
                *out = true;
                *name
            }
            None => {
                let name = fresh_name();
                names.push((name, true));
                name
            }
        };
        Lease { peer, held: Held::Spare(key), pile: None }
    }
}

/// A lock file: opened read-write, made owner-only if it is new.
fn open_owner_only(path: &Path) -> Option<fs::File> {
    let mut opts = fs::OpenOptions::new();
    opts.create(true).truncate(false).read(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(OWNER_ONLY_FILE);
    }
    opts.open(path).ok()
}

impl Drop for Lease {
    fn drop(&mut self) {
        match &self.held {
            Held::Pen((dir, k), _) => {
                if let Some(ks) = PENS_OUT.lock().unwrap_or_else(|p| p.into_inner()).get_mut(dir) {
                    ks.remove(k);
                }
            }
            Held::Spare(key) => {
                let mut lent = LENT.lock().unwrap_or_else(|p| p.into_inner());
                if let Some(names) = lent.get_mut(key)
                    && let Some(slot) = names.iter_mut().find(|(name, _)| *name == self.peer)
                {
                    slot.1 = false;
                }
            }
        }
    }
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
pub fn load<D: Doc + 'static>(dir: &Path, peer: u64) -> Result<Loaded<D>, String> {
    let started = std::time::Instant::now();
    let out = load_cached(dir, peer);
    LOADS.fetch_add(1, Ordering::Relaxed);
    LOAD_NANOS.fetch_add(started.elapsed().as_nanos() as u64, Ordering::Relaxed);
    out
}

/// **A key-value pile, re-seeded if it is older than this build**
/// (PEERS-BRIEF step 3, 2026-10-10). Every process used to write under a
/// name of its own, and loro keeps every name that ever wrote for good:
/// the owner's devices pile carried 1515 rows in its version vector,
/// recents 1129, on every sync request and every merge. Pens stop the
/// growth; only a rebuild sheds the rows already there.
///
/// So a pile whose epoch file is older than [`EPOCH`] is rebuilt the
/// first time it is opened with nobody else holding it: every block read,
/// the state copied key by key into a fresh document written under one
/// new name, that document written as one snapshot, the old blocks, the
/// old pens and the ledger deleted, the new name put in pen 0, and the
/// epoch written last. The history goes; the contents stay, and the
/// rebuild is refused unless the copy reads back equal.
///
/// Under the pile lock, held alone: every open copy holds it shared
/// ([`Lease`]), so no copy that read the old blocks can write after them,
/// and a copy opened meanwhile waits and reads the new pile. Somebody
/// holding the pile means not now; the next open tries again, and until
/// then the copy says its old epoch on the wire.
///
/// Two machines re-seed on their own, each under its own name. For a map
/// of last-writer-wins values that converges like any two concurrent
/// writes (`store::tests::two_replicas_reseeded_apart_agree_after_a_round`).
/// The wire refuses to merge across epochs, so an old replica never
/// pushes the old history back in (`wire::answer`). Chat channels are not
/// piles in this sense: lists, per channel, and loaded with [`load`].
pub fn load_pile<D: Doc + 'static>(dir: &Path, peer: u64) -> Result<Loaded<D>, String> {
    if read_epoch(dir) < EPOCH && dir.is_dir() {
        reseed(dir);
    }
    let mut loaded = load(dir, peer)?;
    if loaded.store.epoch < EPOCH && loaded.store.held.is_empty() && loaded.broken.is_empty() {
        // Nothing in it yet: it starts in this build's epoch.
        loaded.store.epoch = EPOCH;
        loaded.store.stamp = true;
    }
    Ok(loaded)
}

static RESEEDS: AtomicU64 = AtomicU64::new(0);

/// How many piles this process has re-seeded ([`load_pile`]).
pub fn reseeds_done() -> u64 {
    RESEEDS.load(Ordering::Relaxed)
}

/// A pile's epoch: what its epoch file says, 0 with none.
fn read_epoch(dir: &Path) -> u32 {
    fs::read_to_string(dir.join(EPOCH_FILE))
        .ok()
        .and_then(|t| t.trim().parse().ok())
        .unwrap_or(0)
}

/// The rebuild [`load_pile`] describes. `false`: not done, for any
/// reason; the pile is then as it was.
fn reseed(dir: &Path) -> bool {
    let Some(pile) = open_owner_only(&dir.join(PILE_LOCK)) else {
        return false;
    };
    if pile.try_lock().is_err() || read_epoch(dir) >= EPOCH {
        return false;
    }
    // A copy whose pile lock would not take may still hold a pen.
    let mut pens = Vec::new();
    for e in fs::read_dir(dir).into_iter().flatten().flatten() {
        if e.file_name().to_string_lossy().starts_with(PEN) {
            match open_owner_only(&e.path()) {
                Some(f) if f.try_lock().is_ok() => pens.push((e.path(), f)),
                _ => return false,
            }
        }
    }
    let Ok(listed) = blocks(dir) else {
        return false;
    };
    let old = loro::LoroDoc::new();
    let mut read = Vec::new();
    let mut pending = Vec::new();
    for p in listed {
        // A block that will not read stays where it is, as a load leaves it.
        let Ok(bytes) = fs::read(&p) else { continue };
        if let Ok(status) = old.import(&bytes) {
            pending.extend(status.pending);
            read.push(p);
        }
    }
    // A change whose ancestors are in no block is not in the state, and
    // the rebuild would delete the only copy of it.
    let have = old.oplog_vv();
    if pending.iter().flat_map(|r| r.iter()).any(|(peer, (_, end))| have.get(peer).copied().unwrap_or(0) < *end) {
        return false;
    }
    let name = fresh_name();
    let new = loro::LoroDoc::new();
    if !read.is_empty() {
        if new.set_peer_id(name).is_err() || copy_doc(&old, &new).is_none() || old.get_deep_value() != new.get_deep_value() {
            return false;
        }
        let Ok(snap) = new.export(loro::ExportMode::Snapshot) else {
            return false;
        };
        let snap_path = dir.join(format!("snap-{name:016x}-{:08x}.{EXT}", 0));
        if write_atomic(&snap_path, &snap).is_err() {
            return false;
        }
        for p in read.into_iter().filter(|p| *p != snap_path) {
            let _ = fs::remove_file(p);
        }
        let _ = fs::remove_file(dir.join(LEDGER));
        for (p, _) in &pens {
            let _ = fs::remove_file(p);
        }
        if write_atomic(&dir.join(format!("{PEN}0")), format!("{name:016x}\n").as_bytes()).is_err() {
            return false;
        }
        RESEEDS.fetch_add(1, Ordering::Relaxed);
    }
    KEPT.lock().unwrap_or_else(|p| p.into_inner()).remove(dir);
    write_atomic(&dir.join(EPOCH_FILE), format!("{EPOCH}\n").as_bytes()).is_ok()
}

/// Every root container of `from`, written into `to` value by value.
/// `None` for a kind no pile uses; the caller compares the two anyway.
fn copy_doc(from: &loro::LoroDoc, to: &loro::LoroDoc) -> Option<()> {
    use loro::{Container, LoroValue};
    let LoroValue::Map(roots) = from.get_value() else {
        return None;
    };
    for (name, id) in roots.iter() {
        let LoroValue::Container(id) = id else {
            return None;
        };
        match from.get_container(id.clone())? {
            Container::Map(m) => copy_map(&m, &to.get_map(name.as_str()))?,
            Container::List(l) => copy_list(&l, &to.get_list(name.as_str()))?,
            Container::Text(t) => to.get_text(name.as_str()).insert(0, &t.to_string()).ok()?,
            _ => return None,
        }
    }
    to.commit();
    Some(())
}

fn copy_map(from: &loro::LoroMap, to: &loro::LoroMap) -> Option<()> {
    use loro::{Container, LoroList, LoroMap, LoroText, ValueOrContainer};
    let mut ok = Some(());
    from.for_each(|k, v| {
        if ok.is_none() {
            return;
        }
        ok = match v {
            ValueOrContainer::Value(v) => to.insert(k, v).ok(),
            ValueOrContainer::Container(Container::Map(m)) => {
                to.insert_container(k, LoroMap::new()).ok().and_then(|n| copy_map(&m, &n))
            }
            ValueOrContainer::Container(Container::List(l)) => {
                to.insert_container(k, LoroList::new()).ok().and_then(|n| copy_list(&l, &n))
            }
            ValueOrContainer::Container(Container::Text(t)) => {
                to.insert_container(k, LoroText::new()).ok().and_then(|n| n.insert(0, &t.to_string()).ok())
            }
            ValueOrContainer::Container(_) => None,
        };
    });
    ok
}

fn copy_list(from: &loro::LoroList, to: &loro::LoroList) -> Option<()> {
    use loro::{Container, LoroList, LoroMap, LoroText, ValueOrContainer};
    let mut ok = Some(());
    from.for_each(|v| {
        if ok.is_none() {
            return;
        }
        ok = match v {
            ValueOrContainer::Value(v) => to.push(v).ok(),
            ValueOrContainer::Container(Container::Map(m)) => {
                to.push_container(LoroMap::new()).ok().and_then(|n| copy_map(&m, &n))
            }
            ValueOrContainer::Container(Container::List(l)) => {
                to.push_container(LoroList::new()).ok().and_then(|n| copy_list(&l, &n))
            }
            ValueOrContainer::Container(Container::Text(t)) => {
                to.push_container(LoroText::new()).ok().and_then(|n| n.insert(0, &t.to_string()).ok())
            }
            ValueOrContainer::Container(_) => None,
        };
    });
    ok
}

/// What a directory's blocks were when a read of them was kept: each
/// block's name, length and modification time. Blocks are immutable once
/// written, so the same list is the same document.
type Fingerprint = Vec<(String, u64, u128)>;

fn fingerprint(dir: &Path) -> Result<Fingerprint, String> {
    Ok(blocks(dir)?
        .into_iter()
        .filter_map(|p| {
            let meta = fs::metadata(&p).ok()?;
            let at = meta.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_nanos();
            Some((file_name(&p)?, meta.len(), at))
        })
        .collect())
}

/// One kept read: the blocks it was made of, the whole document as one
/// snapshot, and the files that would not read.
struct Kept {
    fingerprint: Fingerprint,
    kind: std::any::TypeId,
    snapshot: std::sync::Arc<Vec<u8>>,
    broken: Vec<PathBuf>,
}

static KEPT: std::sync::LazyLock<std::sync::Mutex<std::collections::HashMap<PathBuf, Kept>>> =
    std::sync::LazyLock::new(Default::default);

static KEPT_HITS: AtomicU64 = AtomicU64::new(0);

/// How many [`load`]s were answered from a kept read of an unchanged
/// directory.
pub fn loads_kept() -> u64 {
    KEPT_HITS.load(Ordering::Relaxed)
}

/// **A read of a directory nothing has written to since is not made
/// again** (the owner's Mac app, 2026-10-05: 70–260 % CPU, the main thread
/// and a network worker both inside loro's import, under the session list
/// and every sync round's opening). Every load used to open an empty
/// document and merge each block in, every block an import with its own
/// diff — up to [`PILE`] of them per document, ten documents a list beat,
/// every chat channel a round. Now the first read of a directory keeps
/// the document as one snapshot beside the list of blocks it was made
/// of; while that list is unchanged (a write, a compaction or a sync adds
/// or removes a file), the next read is one import of that snapshot into
/// a fresh copy under its own lease. The list is taken before the read
/// and again after it, and a read the two disagree about is not kept.
fn load_cached<D: Doc + 'static>(dir: &Path, peer: u64) -> Result<Loaded<D>, String> {
    // The name first, the listing after: a block the name's last holder
    // flushed just before giving it back must be on the listing, or this
    // copy starts its counter below theirs.
    let lease = Lease::take(dir, peer);
    let before = fingerprint(dir)?;
    let kind = std::any::TypeId::of::<D>();
    let kept = KEPT
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .get(dir)
        .filter(|k| k.kind == kind && k.fingerprint == before)
        .map(|k| (k.snapshot.clone(), k.broken.clone()));
    if let Some((snapshot, broken)) = kept {
        let doc = D::open(lease.peer)?;
        if doc.merge(&snapshot).is_ok() {
            KEPT_HITS.fetch_add(1, Ordering::Relaxed);
            // As a read holds what it merged: never a block that would
            // not read, which a compaction must leave where it is.
            let held = before
                .iter()
                .map(|(name, _, _)| name.clone())
                .filter(|name| !broken.iter().any(|b| file_name(b).as_deref() == Some(name.as_str())))
                .collect();
            let store = BlockStore {
                dir: dir.to_path_buf(),
                on_disk: doc.version(),
                ledger: read_ledger(dir),
                held,
                epoch: read_epoch(dir),
                stamp: false,
            };
            return Ok(Loaded { store, doc, broken, lease });
        }
    }
    let loaded: Loaded<D> = load_inner(dir, lease)?;
    if !before.is_empty()
        && fingerprint(dir).is_ok_and(|after| after == before)
        && let Ok(snapshot) = loaded.doc.snapshot()
    {
        KEPT.lock().unwrap_or_else(|p| p.into_inner()).insert(
            dir.to_path_buf(),
            Kept { fingerprint: before, kind, snapshot: std::sync::Arc::new(snapshot), broken: loaded.broken.clone() },
        );
    }
    Ok(loaded)
}

fn load_inner<D: Doc>(dir: &Path, lease: Lease) -> Result<Loaded<D>, String> {
    let (loaded, vanished) = read_once(dir, lease)?;
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
    Ok(read_once(dir, loaded.lease)?.0)
}

/// One pass over a document's blocks. The flag is 「a block on the
/// listing was gone by the time we read it」 — see [`load_inner`].
fn read_once<D: Doc>(dir: &Path, lease: Lease) -> Result<(Loaded<D>, bool), String> {
    let doc = D::open(lease.peer)?;
    let mut broken = Vec::new();
    let mut ledger = read_ledger(dir);
    let mut vanished = false;
    // Whether anything was learned that the ledger on disk does not
    // already say. See the write at the end.
    let mut grew = false;
    let mut held = std::collections::BTreeSet::new();

    for p in blocks(dir)? {
        match fs::read(&p) {
            Ok(bytes) => {
                if doc.merge(&bytes).is_err() {
                    broken.push(p);
                } else if let Some(n) = file_name(&p) {
                    grew |= ledger.insert(&n);
                    held.insert(n);
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
        held,
        epoch: read_epoch(dir),
        stamp: false,
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
    Ok((Loaded { store, doc, broken, lease }, vanished))
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
        self.make_dir()?;
        write_atomic(&self.dir.join(name), bytes)?;
        self.ledger.insert(name);
        self.held.insert(name.to_owned());
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

    /// The directory, made if it is not there; and an empty pile's epoch
    /// file before its first block, so no reader ever takes a block of
    /// this build's for an old pile.
    fn make_dir(&mut self) -> Result<(), String> {
        make_dir(&self.dir)?;
        if self.stamp {
            if !self.dir.join(EPOCH_FILE).exists() {
                write_atomic(&self.dir.join(EPOCH_FILE), format!("{}\n", self.epoch).as_bytes())?;
            }
            self.stamp = false;
        }
        Ok(())
    }

    /// The epoch this copy's blocks belong to: [`EPOCH`] for a pile
    /// opened by [`load_pile`] (unless it could not be re-seeded yet), 0
    /// for a chat channel.
    pub fn epoch(&self) -> u32 {
        self.epoch
    }

    /// The document's name, as the wire and the journal say it: its
    /// directory's.
    pub fn name(&self) -> String {
        file_name(&self.dir).unwrap_or_default()
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
        self.make_dir()?;
        let path = self.dir.join(format!(
            "u-{:016x}-{:08x}.{EXT}",
            doc.peer_id(),
            self.next_seq(doc.peer_id())?
        ));
        write_atomic(&path, &delta)?;
        // Own blocks enter the ledger too: the far side may push one right
        // back, and an unledgered copy would be re-pulled on every sync.
        if let Some(n) = file_name(&path) {
            self.ledger.insert(&n);
            self.held.insert(n);
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
        Ok(self.next_number(&format!("u-{peer:016x}-")))
    }

    /// The next number after every name starting `prefix` **on disk or in
    /// the ledger** (round NET-6). Disk alone let a number come round once
    /// compaction had deleted the blocks that used it: a new block took a
    /// name the far side's ledger already marks merged, and a snapshot took
    /// the name of the one before it — and the delete that follows a
    /// compaction then took the new snapshot too
    /// (`store::tests::compacting_twice_keeps_every_word`). The ledger keeps
    /// every name, so a number counted against it is never reused.
    fn next_number(&self, prefix: &str) -> u32 {
        let number = |name: &str| {
            u32::from_str_radix(name.strip_prefix(prefix)?.strip_suffix(&format!(".{EXT}"))?, 16).ok()
        };
        let on_disk = fs::read_dir(&self.dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| number(&e.file_name().to_string_lossy()));
        let ledgered = self.ledger.names().iter().filter_map(|n| number(n)).collect::<Vec<_>>();
        on_disk.chain(ledgered).max().map_or(0, |m| m + 1)
    }

    /// Compaction: write current state as one snapshot block, then delete
    /// the increments it covers. Write-then-delete, never reversed — a
    /// crash in between must cost a few spare KB, not the whole document.
    /// The ledger keeps every line, or the next sync would pull the
    /// compacted blocks right back, every time.
    ///
    /// **Only the increments it covers**: the ones this copy holds. A
    /// block another writer flushed since this copy was read is on the
    /// listing and not in the snapshot, and deleting it lost its writes
    /// for good (`store::tests::a_block_written_while_compacting_is_not_deleted_unread`).
    pub fn compact<D: Doc>(&mut self, doc: &D) -> Result<PathBuf, String> {
        self.make_dir()?;
        let old: Vec<PathBuf> = blocks(&self.dir)?
            .into_iter()
            .filter(|p| file_name(p).is_some_and(|n| self.held.contains(&n)))
            .collect();
        let snap = doc.snapshot()?;
        // The writer in the name as well as the number, as increments
        // have it: two writers' snapshots never share a name.
        let prefix = format!("snap-{:016x}-", doc.peer_id());
        let path = self.dir.join(format!("{prefix}{:08x}.{EXT}", self.next_number(&prefix)));
        write_atomic(&path, &snap)?;
        for p in old.into_iter().filter(|p| *p != path) {
            if let Some(n) = file_name(&p) {
                self.held.remove(&n);
            }
            let _ = fs::remove_file(p);
        }
        if let Some(n) = file_name(&path) {
            self.ledger.insert(&n);
            self.held.insert(n);
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
    // **Absent is empty; anything else is an error**, and the two used
    // to be one branch. A store whose directory has been replaced by a
    // file, or whose permissions stop a read, answered「no blocks」 —
    // and a document with no blocks is a document with nothing in it,
    // so a damaged store read back as 「you have no recents」, 「no
    // machines are paired」, 「there are no accounts」. Silent, and in
    // the one direction this repo never accepts.
    //
    // Found by `tests/files_wire.rs`, whose control writes a file where
    // a store's directory belongs and asserts the read refuses. That
    // assertion had been red on `main` for long enough that it was
    // being carried as「a pre-existing red」rather than read
    // (2026-09-12).
    let rd = match fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Vec::new()); // never written to = empty document
        }
        Err(e) => return Err(msg::cant_list_dir(dir.display(), e)),
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
        fs::DirBuilder::new()
            .recursive(true)
            .mode(OWNER_ONLY_DIR)
            .create(dir)
            .or_else(|e| {
                if dir.is_dir() {
                    Ok(())
                } else {
                    Err(msg::cant_make_dir(e))
                }
            })
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
    Ok(Some(Compacted {
        before,
        after: blocks(dir)?.len(),
    }))
}

/// The lease file that makes 「one compactor」 enforceable rather than
/// merely stated.
const LEASE: &str = "compacting";

/// How long a lease stands before it is treated as abandoned. Long
/// enough that no honest compaction is mistaken for a corpse (the
/// longest measured is well under a second), short enough that a machine
/// that died mid-rewrite does not freeze compaction for an afternoon.
const LEASE_STALE_MS: u64 = 5 * 60 * 1000;

/// Take the compaction lease for `home`, or find that somebody else
/// holds it.
///
/// ⚠️ **This is not belt-and-braces, it is the thing that makes
/// compaction safe on this fleet.** Three of these machines share one
/// `.khor` over NFS (turing, hinton, lecun), so 「the resident serve
/// owns compaction」 is three residents owning it, and two compactors on
/// one document destroy it — the second reads a directory the first is
/// half way through deleting, gets an empty document, and writes an
/// empty snapshot over everything. Measured, not feared:
/// `store::tests::a_read_that_races_a_compaction_still_sees_every_line`
/// was written as two tight loops first and did exactly that.
///
/// `create_new` is `O_EXCL`, which NFSv3 and later implement as an
/// atomic exclusive create — the one primitive that survives a shared
/// filesystem without a lock daemon.
///
/// **Measured on the filesystem it has to work on, 2026-09-11**, because
/// 「NFS implements this atomically」 is a claim about a server, not
/// about a standard. turing and hinton, both on the shared mount, raced
/// for the same 300 lock files at once:
///
/// ```text
/// turing  won 157
/// hinton  won 143
/// both claimed the same one:  0
/// ```
///
/// The split is what makes it evidence rather than a coincidence: had
/// one machine simply been faster every time, zero overlaps would mean
/// nothing.
///
/// **What it does not cover**: the window between a holder dying and its
/// lease going stale, during which nothing is compacted. That is by
/// design — not compacting is the safe failure — and a different
/// filesystem would need the reading above taken again.
fn take_lease(home: &Path) -> Option<PathBuf> {
    let path = home.join(LEASE);
    let mine = || {
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map(|mut f| {
                use std::io::Write as _;
                // The stamp is for a person reading the directory after
                // something went wrong; the staleness test below uses the
                // file's own mtime, which no writer can get wrong.
                let _ = writeln!(f, "{}", now_ms());
            })
            .is_ok()
    };
    if mine() {
        return Some(path);
    }
    // Somebody holds it, or somebody died holding it. Only the second
    // is ours to clear, and only after it has been still for long
    // enough that no live compaction could look like it.
    let age = fs::metadata(&path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.elapsed().ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    if age > LEASE_STALE_MS {
        let _ = fs::remove_file(&path);
        if mine() {
            return Some(path);
        }
    }
    None
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Whether any document under `home` has piled up past [`PILE`], two
/// directories deep — the same shape [`compact_piles_under`] walks, and
/// reading only.
///
/// It exists so the ordinary tick, which finds nothing, costs directory
/// listings and no writes at all.
fn anything_piled(home: &Path) -> bool {
    let piled = |d: &Path| blocks(d).map(|b| b.len() > PILE).unwrap_or(false);
    let Ok(top) = fs::read_dir(home) else {
        return false;
    };
    for e in top.flatten() {
        let p = e.path();
        if !p.is_dir() {
            continue;
        }
        if piled(&p) {
            return true;
        }
        let Ok(inner) = fs::read_dir(&p) else {
            continue;
        };
        for e in inner.flatten() {
            let q = e.path();
            if q.is_dir() && piled(&q) {
                return true;
            }
        }
    }
    false
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
    // **Look before taking the lease.** The resident calls this on its
    // sync tick — every five seconds, for the whole life of the
    // process — and on an ordinary tick there is nothing piled up at
    // all. Taking the lease first meant creating and deleting a file on
    // every one of those ticks, and on the shared-home machines that is
    // two NFS round trips to learn that there was no work.
    //
    // The window between looking and taking is harmless: whoever gets
    // the lease re-checks each directory's count inside `compact_dir`,
    // so a pile somebody else just flattened is simply skipped.
    if !anything_piled(home) {
        return out;
    }
    // **Nobody else may be doing this right now.** See `take_lease` —
    // on the shared-home machines this is the only thing standing
    // between 「one compactor」 and a destroyed document.
    let Some(lease) = take_lease(home) else {
        return out;
    };
    let Ok(top) = fs::read_dir(home) else {
        let _ = fs::remove_file(&lease);
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
        let Ok(inner) = fs::read_dir(&p) else {
            continue;
        };
        for e in inner.flatten() {
            let q = e.path();
            if q.is_dir()
                && let Ok(Some(c)) = compact_dir(&q, peer)
            {
                out.push((q, c));
            }
        }
    }
    let _ = fs::remove_file(&lease);
    out.sort_by_key(|(_, c)| std::cmp::Reverse(c.before));
    out
}
