//! Persistence: a stack of append-only block files anyone can read or write.
//!
//! ```text
//! ~/.khor/chat/<channel>/
//!     u-<author:016x>-<seq:08x>.loro   ← increment block, immutable once written
//!     snap-<seq:08x>.loro              ← compaction product (see `compact`)
//! ```
//!
//! One file per batch, author in the name: two devices writing the same
//! directory never conflict at the filesystem level (the Maildir answer).
//! Loro increments merge in any order, so "immutable files + merge them
//! all" is the entire consistency story.
//!
//! The path is fixed rather than `config_dir()`: the writer is often on
//! another machine (sftp) and cannot know the remote OS's config dir.
//! Two path schemes would eventually disagree, and the symptom would be
//! two chat histories on one machine, each blind to the other.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use loro::VersionVector;

use super::doc::ChatDoc;
use super::plan::{Ledger, Side};

/// Location relative to the home directory. Fixed — see module head.
pub const REL_DIR: &str = ".khor/chat";

/// Block file extension.
const EXT: &str = "loro";

/// The merged-blocks ledger. No `.loro` extension, so it is neither read
/// as a block nor synced to the far side — each machine keeps its own.
const LEDGER: &str = ".merged";

/// Channel names travel through a remote shell (ssh/sftp path building),
/// so this whitelist blocks command injection, not just bad filenames.
/// `.` passes because channel names are usually machine names.
pub fn valid_channel(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name != "."
        && name != ".."
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
}

/// A channel's directory; `None` for invalid names. Never "clean" a name
/// here: two devices cleaning differently would split one channel into
/// two directories. Cleaning has exactly one implementation,
/// [`channel_of_machine`].
pub fn channel_dir(home: &Path, channel: &str) -> Option<PathBuf> {
    valid_channel(channel).then(|| home.join(REL_DIR).join(channel))
}

/// The channel name of a machine. One machine, one channel: every device
/// writes into the same window, so a third machine joins the same
/// conversation instead of spawning pairwise ones.
///
/// `name` must be the machine's **self-reported** hostname — not a
/// user-editable display name, not an ssh alias. Two devices naming one
/// machine differently means two directories that never converge, with
/// no error anywhere.
///
/// Cleaning: clean names pass through unchanged; a changed name gets a
/// fingerprint of the **original** appended. So a cleaned name can never
/// collide with a real one (`a b` → `a-b-<fp>`, distinct from a machine
/// truly named `a-b`), and two originals that clean to the same stem stay
/// apart. Pure function — two machines must compute the same result.
/// `None` when nothing usable remains: report that, don't invent a name
/// someone else could also get.
pub fn channel_of_machine(name: &str) -> Option<String> {
    let name = name.trim().trim_end_matches(".local");
    if name.is_empty() {
        return None;
    }
    if valid_channel(name) {
        return Some(name.to_string());
    }
    // Replace per char, then collapse runs of `-`: a mostly-CJK name
    // should not become a row of dashes.
    let mut cleaned = String::new();
    for c in name.chars() {
        let ok = c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.';
        if ok {
            cleaned.push(c);
        } else if !cleaned.ends_with('-') {
            cleaned.push('-');
        }
    }
    let cleaned = cleaned.trim_matches(['-', '.']).to_string();
    let stem: String = cleaned.chars().take(40).collect();
    let stem = stem.trim_end_matches(['-', '.']);
    // The fingerprint hashes the original, not the cleaned form: that is
    // what keeps two different originals with identical stems apart.
    let out = format!("{}-{}", if stem.is_empty() { "m" } else { stem }, fingerprint(name));
    valid_channel(&out).then_some(out)
}

/// FNV-1a. This needs determinism across machines, not collision
/// resistance — no hash crate.
fn fingerprint(s: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{:08x}", (h ^ (h >> 32)) as u32)
}

/// One channel's stack on disk.
pub struct ChatStore {
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
pub struct Loaded {
    pub store: ChatStore,
    pub doc: ChatDoc,
    /// Files that would not read. Counted, not swallowed: a bad block is
    /// a missing slice of conversation and the UI must be able to say so;
    /// one bad block must not kill the channel either.
    pub broken: Vec<PathBuf>,
}

impl ChatStore {
    /// Reads a channel back from disk. No directory = empty conversation,
    /// not an error.
    pub fn load(dir: &Path, peer: u64) -> Result<Loaded, String> {
        let doc = ChatDoc::new(peer)?;
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
        let mut store = ChatStore {
            dir: dir.to_path_buf(),
            on_disk,
            ledger,
        };
        // Only write the ledger if the directory exists: an empty channel
        // should leave nothing on disk.
        if dir.exists() {
            store.save_ledger()?;
        }
        Ok(Loaded { store, doc, broken })
    }

    /// My side of the books, for [`super::plan::plan`].
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
    /// conversation goes permanently missing, with no error anywhere.
    pub fn absorb(&mut self, doc: &ChatDoc, name: &str, bytes: &[u8]) -> Result<(), String> {
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
        fs::read(self.dir.join(name)).map_err(|e| format!("读不了 {name}: {e}"))
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
    pub fn flush(&mut self, doc: &ChatDoc) -> Result<Option<PathBuf>, String> {
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
            doc.raw().peer_id(),
            self.next_seq(doc.raw().peer_id())?
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
    /// crash in between must cost a few spare KB, not the whole
    /// conversation. The ledger keeps every line, or the next sync would
    /// pull the compacted blocks right back, every time.
    ///
    /// This does not trim history: other devices syncing afterwards still
    /// receive it all (the snapshot carries it). Shallow snapshots are a
    /// different feature with product implications of their own.
    pub fn compact(&mut self, doc: &ChatDoc) -> Result<PathBuf, String> {
        make_dir(&self.dir)?;
        let old = blocks(&self.dir)?;
        let snap = doc.snapshot()?;
        let path = self
            .dir
            .join(format!("snap-{:08x}.{EXT}", self.next_seq(doc.raw().peer_id())?));
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
        return Ok(Vec::new()); // no directory = empty conversation
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
/// block is the document itself — the bytes reconstruct the whole
/// conversation — so the create() default (644 under common umask) leaks
/// it on any shared-home machine. "Not plaintext" is no protection.
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
                if dir.is_dir() { Ok(()) } else { Err(format!("建不了目录: {e}")) }
            });
    }
    #[cfg(not(unix))]
    fs::create_dir_all(dir).map_err(|e| format!("建不了目录: {e}"))
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
            .map_err(|e| format!("写不了 {}: {e}", tmp.display()))?;
        f.write_all(bytes).map_err(|e| format!("写不完: {e}"))?;
        f.sync_all().map_err(|e| format!("落不了盘: {e}"))?;
    }
    fs::rename(&tmp, path).map_err(|e| format!("改不了名: {e}"))
}

fn file_name(p: &Path) -> Option<String> {
    Some(p.file_name()?.to_string_lossy().to_string())
}

fn read_ledger(dir: &Path) -> Ledger {
    // Unreadable = empty ledger, not an error: the cost is one idempotent
    // re-pull, while erroring here would brick a fresh channel.
    fs::read_to_string(dir.join(LEDGER))
        .map(|t| Ledger::parse(&t))
        .unwrap_or_default()
}
