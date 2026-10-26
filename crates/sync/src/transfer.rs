//! Private, resumable byte staging. This module cannot acknowledge a Doc apply.
//!
//! `VerifiedComplete` means only that the declared bytes are present and their
//! digest matches. The document layer must still validate epochs/versions,
//! reject pending imports, preserve concurrent ops and flush before Applied.
//! File syncs/checksummed checkpoints support process-restart recovery; this is
//! not a cross-platform power-loss durability or bounded Loro-memory claim.
//!
//! The root is an exclusively managed private directory. Callers authenticate
//! the source/recipient and canonically encode the document/epoch/base/target
//! (or other purpose) into `binding`; arbitrary JSON is not canonical encoding.

use std::{
    collections::BTreeMap,
    fmt, fs,
    fs::{File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard},
};

mod platform;

pub const CHUNK_BYTES: usize = 256 * 1024;
pub const MAX_METADATA_BYTES: usize = 64 * 1024;
pub const MAX_BLOB_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_RESERVED_BYTES: u64 = 128 * 1024 * 1024;
pub const MAX_OWNERS: usize = 2;
/// Also bound metadata/inode use when retained blobs have empty bodies.
pub const MAX_RETAINED_STAGES: usize = 64;

const MAGIC: &[u8; 8] = b"khor-x01";
const HEADER_BYTES: usize = 8 + 32 + 32 + 8 + 8 + 32;
const RECORD_BYTES: usize = 8 + 32 + 32;
const MAX_RECORDS: usize = (MAX_BLOB_BYTES as usize / CHUNK_BYTES) + 1;
const INITIALIZING: &str = ".initializing";

/// Every field, including opaque purpose binding, participates in the id.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Manifest {
    pub source: [u8; 32],
    pub recipient: [u8; 32],
    pub binding: Vec<u8>,
    pub total_bytes: u64,
    pub digest: [u8; 32],
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct TransferId([u8; 32]);

impl fmt::Display for TransferId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum Error {
    ResourceLimit(&'static str),
    Busy,
    InvalidManifest(&'static str),
    InvalidStage(&'static str),
    InvalidChunk(&'static str),
    OutOfOrder {
        expected: u64,
        received: u64,
    },
    ConflictingDuplicate,
    DigestMismatch,
    Cleanup {
        cause: Box<Error>,
        cleanup: io::Error,
    },
    Io(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ResourceLimit(why) => write!(f, "transfer resource limit: {why}"),
            Self::Busy => f.write_str("transfer owner or quota gate is busy"),
            Self::InvalidManifest(why) => write!(f, "invalid transfer manifest: {why}"),
            Self::InvalidStage(why) => write!(f, "invalid transfer stage: {why}"),
            Self::InvalidChunk(why) => write!(f, "invalid transfer chunk: {why}"),
            Self::OutOfOrder { expected, received } => {
                write!(f, "transfer offset {received}, expected {expected}")
            }
            Self::ConflictingDuplicate => {
                f.write_str("transfer duplicate differs from stored bytes")
            }
            Self::DigestMismatch => f.write_str("transfer body digest mismatch"),
            Self::Cleanup { cause, cleanup } => write!(
                f,
                "{cause}; transfer initialization cleanup failed: {cleanup}"
            ),
            Self::Io(why) => write!(f, "transfer I/O: {why}"),
        }
    }
}

impl std::error::Error for Error {}
impl From<io::Error> for Error {
    fn from(why: io::Error) -> Self {
        Self::Io(why)
    }
}
pub type Result<T> = std::result::Result<T, Error>;

impl Manifest {
    pub fn validate(&self) -> Result<()> {
        if self.binding.is_empty() {
            return Err(Error::InvalidManifest("empty purpose binding"));
        }
        if self.binding.len() > MAX_METADATA_BYTES - HEADER_BYTES {
            return Err(Error::ResourceLimit("manifest metadata exceeds 64 KiB"));
        }
        if self.total_bytes > MAX_BLOB_BYTES {
            return Err(Error::ResourceLimit("blob exceeds 64 MiB"));
        }
        Ok(())
    }

    pub fn id(&self) -> Result<TransferId> {
        self.validate()?;
        Ok(TransferId(*blake3::hash(&self.encode()).as_bytes()))
    }

    fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(HEADER_BYTES + self.binding.len());
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&self.source);
        out.extend_from_slice(&self.recipient);
        out.extend_from_slice(&self.total_bytes.to_be_bytes());
        out.extend_from_slice(&(self.binding.len() as u64).to_be_bytes());
        out.extend_from_slice(&self.digest);
        out.extend_from_slice(&self.binding);
        out
    }

    fn decode(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < HEADER_BYTES || bytes.len() > MAX_METADATA_BYTES || &bytes[..8] != MAGIC {
            return Err(Error::InvalidStage("manifest framing"));
        }
        let binding_len = u64::from_be_bytes(bytes[80..88].try_into().unwrap());
        if binding_len != (bytes.len() - HEADER_BYTES) as u64 {
            return Err(Error::InvalidStage("manifest binding length"));
        }
        let manifest = Self {
            source: bytes[8..40].try_into().unwrap(),
            recipient: bytes[40..72].try_into().unwrap(),
            total_bytes: u64::from_be_bytes(bytes[72..80].try_into().unwrap()),
            digest: bytes[88..120].try_into().unwrap(),
            binding: bytes[120..].to_vec(),
        };
        manifest.validate()?;
        Ok(manifest)
    }
}

/// This is byte verification, deliberately not an `Applied` variant.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Progress {
    Stored { next_offset: u64 },
    VerifiedComplete { id: TransferId, bytes: u64 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Cancelled {
    Removed,
    Absent,
}

#[derive(Clone, Debug)]
pub struct TransferStore {
    root: PathBuf,
}

// The OS locks cover other processes; this table also covers platforms where
// a second file lock in the same process would not exclude the first one.
static LOCAL: Mutex<BTreeMap<PathBuf, TransferId>> = Mutex::new(BTreeMap::new());

struct Gate<'a> {
    local: MutexGuard<'a, BTreeMap<PathBuf, TransferId>>,
    file: Option<File>,
}

impl Drop for Gate<'_> {
    fn drop(&mut self) {
        drop(self.file.take());
    }
}

struct Owner {
    path: PathBuf,
    file: Option<File>,
}

impl Drop for Owner {
    fn drop(&mut self) {
        let mut local = LOCAL.lock().unwrap_or_else(|p| p.into_inner());
        drop(self.file.take());
        local.remove(&self.path);
    }
}

impl TransferStore {
    /// The caller owns the parent and supplies a dedicated staging directory.
    /// Existing non-private directories or links are refused, never chmodded.
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref();
        private_dir(root)?;
        Ok(Self {
            root: root.canonicalize()?,
        })
    }

    fn gate(&self) -> Result<Gate<'static>> {
        let local = LOCAL
            .lock()
            .map_err(|_| Error::InvalidStage("owner table poisoned"))?;
        let file = private_file(&self.root.join("gate"), true)?;
        try_lock(&file)?;
        let gate = Gate {
            local,
            file: Some(file),
        };
        self.reconcile_initialization(&gate)?;
        Ok(gate)
    }

    fn reconcile_initialization(&self, _held: &Gate<'_>) -> Result<()> {
        clear_initialization(&self.root.join(INITIALIZING))
    }

    fn initialize_unpublished(
        &self,
        _held: &Gate<'_>,
        manifest: &Manifest,
        id: TransferId,
    ) -> Result<()> {
        let dir = self.root.join(INITIALIZING);
        #[cfg(test)]
        tests::pause_initialization(&dir, "before_mkdir");
        // Only a successful mkdir grants cleanup authority over this exact
        // reserved namespace. Published ids never enter this cleanup path.
        create_stage(&dir, manifest, id)?;
        let final_dir = self.root.join(id.to_string());
        let publish = (|| {
            let entries = initialization_entries(&dir)?
                .ok_or(Error::InvalidStage("initializer disappeared"))?;
            if entries.len() != 3
                || read_manifest(&dir)? != *manifest
                || fs::read(dir.join("offsets"))?
                    != checkpoint(id, 0, *blake3::hash(&[]).as_bytes())
            {
                return Err(Error::InvalidStage("initializer binding changed"));
            }
            #[cfg(test)]
            tests::pause_initialization(&dir, "before_publish");
            platform::publish_no_replace(&dir, &final_dir)?;
            Ok(())
        })();
        if let Err(cause) = publish {
            return Err(initialization_error(&dir, cause));
        }
        #[cfg(test)]
        tests::pause_initialization(&final_dir, "after_publish");
        Ok(())
    }

    /// All retained stages reserve their declared size, including idle stages.
    /// Dropping a Sink releases its live owner, not its reservation or bytes.
    pub fn reserved_bytes(&self) -> Result<u64> {
        let _gate = self.gate()?;
        self.reservations().map(|(bytes, _)| bytes)
    }

    fn reservations(&self) -> Result<(u64, BTreeMap<TransferId, Manifest>)> {
        let mut bytes = 0u64;
        let mut stages = BTreeMap::new();
        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            let name = entry.file_name();
            let name = name
                .to_str()
                .ok_or(Error::InvalidStage("non-UTF8 stage entry"))?;
            if name == "gate" || name == "owner-0" || name == "owner-1" {
                check_file(&entry.path())?;
                continue;
            }
            if name.len() != 64
                || !name
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            {
                return Err(Error::InvalidStage(
                    "unexpected root entry; explicit reconciliation required",
                ));
            }
            check_dir(&entry.path())?;
            let manifest = read_manifest(&entry.path())?;
            let id = manifest.id()?;
            if id.to_string() != name {
                return Err(Error::InvalidStage("stage id does not bind manifest"));
            }
            bytes = bytes
                .checked_add(manifest.total_bytes)
                .ok_or(Error::ResourceLimit("reservation overflow"))?;
            stages.insert(id, manifest);
            if stages.len() > MAX_RETAINED_STAGES {
                return Err(Error::ResourceLimit("retained stages exceed 64"));
            }
        }
        if bytes > MAX_RESERVED_BYTES {
            return Err(Error::ResourceLimit("retained stages exceed 128 MiB"));
        }
        Ok((bytes, stages))
    }

    /// First-frame bytes use `Sink::append(0, first)` exactly like later chunks.
    pub fn receive(&self, manifest: Manifest) -> Result<Sink> {
        let id = manifest.id()?;
        let mut gate = self.gate()?;
        let (reserved, stages) = self.reservations()?;
        let exists = stages.contains_key(&id);
        if !exists && reserved + manifest.total_bytes > MAX_RESERVED_BYTES {
            return Err(Error::ResourceLimit("retained stages would exceed 128 MiB"));
        }
        if !exists && stages.len() >= MAX_RETAINED_STAGES {
            return Err(Error::ResourceLimit(
                "64 retained stages are already reserved",
            ));
        }
        let (owner_path, mut owner_file) = self.available_owner(id, &gate)?;
        let dir = self.root.join(id.to_string());
        if !exists {
            self.initialize_unpublished(&gate, &manifest, id)?;
        }
        if read_manifest(&dir)? != manifest {
            return Err(Error::InvalidStage("manifest changed"));
        }
        let mut body = private_file(&dir.join("body"), false)?;
        let mut journal = private_file(&dir.join("offsets"), false)?;
        let (offset, hasher) = recover(&mut body, &mut journal, id, manifest.total_bytes)?;
        owner_file.set_len(0)?;
        owner_file.seek(SeekFrom::Start(0))?;
        owner_file.write_all(&id.0)?;
        owner_file.sync_all()?;
        gate.local.insert(owner_path.clone(), id);
        // Nothing fallible after insertion: Owner Drop cannot recurse into a
        // gate still held by a failed receive.
        Ok(Sink {
            manifest,
            id,
            body,
            journal,
            offset,
            hasher,
            poisoned: false,
            _owner: Owner {
                path: owner_path,
                file: Some(owner_file),
            },
        })
    }

    /// Explicitly release an idle stage and its reservation. The complete
    /// manifest must still match; damaged body/checkpoint bytes do not prevent
    /// cancellation. An active owner, including one in another process, wins.
    /// This never rolls back an applied Doc and has no automatic GC policy.
    pub fn cancel(&self, expected: &Manifest) -> Result<Cancelled> {
        let id = expected.id()?;
        let gate = self.gate()?;
        // This is an ordinary File lease, not Owner: while LOCAL is held its
        // Drop cannot recursively lock LOCAL. The gate prevents new owners
        // from entering between the live-owner check and removal.
        let (_slot, _lease) = self.available_owner(id, &gate)?;
        let dir = self.root.join(id.to_string());
        match fs::symlink_metadata(&dir) {
            Err(why) if why.kind() == io::ErrorKind::NotFound => return Ok(Cancelled::Absent),
            Err(why) => return Err(why.into()),
            Ok(_) => {}
        }
        check_dir(&dir)?;
        if read_manifest(&dir)? != *expected {
            return Err(Error::InvalidStage("cancel manifest changed"));
        }
        fs::remove_dir_all(&dir)?;
        Ok(Cancelled::Removed)
    }

    fn available_owner(&self, id: TransferId, gate: &Gate<'_>) -> Result<(PathBuf, File)> {
        let mut available = None;
        for slot in 0..MAX_OWNERS {
            let path = self.root.join(format!("owner-{slot}"));
            if let Some(active) = gate.local.get(&path) {
                if *active == id {
                    return Err(Error::Busy);
                }
                continue;
            }
            let mut file = private_file(&path, true)?;
            match file.try_lock() {
                Ok(()) => {
                    if available.is_none() {
                        available = Some((path, file));
                    }
                }
                Err(fs::TryLockError::WouldBlock) => {
                    let mut active = Vec::new();
                    Read::by_ref(&mut file).take(33).read_to_end(&mut active)?;
                    if active.len() != 32 {
                        return Err(Error::InvalidStage(
                            "busy owner has no complete transfer binding",
                        ));
                    }
                    if active.as_slice() == id.0 {
                        return Err(Error::Busy);
                    }
                }
                Err(fs::TryLockError::Error(why)) => return Err(why.into()),
            }
        }
        available.ok_or(Error::ResourceLimit(
            "two transfer owners are already active",
        ))
    }
}

pub struct Sink {
    manifest: Manifest,
    id: TransferId,
    body: File,
    journal: File,
    offset: u64,
    hasher: blake3::Hasher,
    poisoned: bool,
    _owner: Owner,
}

impl Sink {
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }
    pub fn next_offset(&self) -> u64 {
        self.offset
    }

    pub fn progress(&self) -> Result<Progress> {
        if self.poisoned {
            return Err(Error::InvalidStage("uncertain write; drop and reopen"));
        }
        if self.offset != self.manifest.total_bytes {
            return Ok(Progress::Stored {
                next_offset: self.offset,
            });
        }
        if self.hasher.finalize().as_bytes() != &self.manifest.digest {
            return Err(Error::DigestMismatch);
        }
        Ok(Progress::VerifiedComplete {
            id: self.id,
            bytes: self.offset,
        })
    }

    /// Canonical chunks are full-sized except the final chunk. That also bounds
    /// the checkpoint journal to 257 records, even for hostile tiny writes.
    pub fn append(&mut self, at: u64, bytes: &[u8]) -> Result<Progress> {
        if self.poisoned {
            return Err(Error::InvalidStage("uncertain write; drop and reopen"));
        }
        if at > self.offset {
            return Err(Error::OutOfOrder {
                expected: self.offset,
                received: at,
            });
        }
        if bytes.len() > CHUNK_BYTES {
            return Err(Error::InvalidChunk("chunk exceeds 256 KiB"));
        }
        let end = at
            .checked_add(bytes.len() as u64)
            .ok_or(Error::InvalidChunk("offset overflow"))?;
        if end > self.manifest.total_bytes {
            return Err(Error::InvalidChunk("chunk exceeds declared total"));
        }
        if end != self.manifest.total_bytes && bytes.len() != CHUNK_BYTES {
            return Err(Error::InvalidChunk("non-final chunk is not 256 KiB"));
        }
        if bytes.is_empty() {
            return self.progress();
        }
        if at < self.offset {
            if end > self.offset {
                return Err(Error::InvalidChunk("duplicate overlaps unstored bytes"));
            }
            let mut stored = vec![0; bytes.len()];
            self.body.seek(SeekFrom::Start(at))?;
            self.body.read_exact(&mut stored)?;
            if stored != bytes {
                return Err(Error::ConflictingDuplicate);
            }
            return self.progress();
        }
        // No successful checkpoint follows an uncertain write/sync. On error
        // the caller must drop/reopen; recover checks data against its journal.
        self.poisoned = true;
        self.body.seek(SeekFrom::Start(at))?;
        self.body.write_all(bytes)?;
        self.body.sync_all()?;
        let mut next_hasher = self.hasher.clone();
        next_hasher.update(bytes);
        let record = checkpoint(self.id, end, *next_hasher.finalize().as_bytes());
        self.journal.seek(SeekFrom::End(0))?;
        self.journal.write_all(&record)?;
        self.journal.sync_all()?;
        self.offset = end;
        self.hasher = next_hasher;
        self.poisoned = false;
        self.progress()
    }

    /// A bounded read for the later Doc layer; it does not allocate a full Vec.
    /// Only verified bytes may be read through this API.
    pub fn read_verified(&mut self, at: u64, into: &mut [u8]) -> Result<usize> {
        if !matches!(self.progress()?, Progress::VerifiedComplete { .. }) {
            return Err(Error::InvalidStage("body has not been verified complete"));
        }
        if into.len() > CHUNK_BYTES {
            return Err(Error::InvalidChunk("read buffer exceeds 256 KiB"));
        }
        if at > self.offset {
            return Err(Error::InvalidChunk("read offset exceeds total"));
        }
        self.body.seek(SeekFrom::Start(at))?;
        Ok(self.body.read(into)?)
    }
}

fn checkpoint(id: TransferId, offset: u64, prefix: [u8; 32]) -> [u8; RECORD_BYTES] {
    let mut record = [0; RECORD_BYTES];
    record[..8].copy_from_slice(&offset.to_be_bytes());
    record[8..40].copy_from_slice(&prefix);
    let mut hash = blake3::Hasher::new();
    hash.update(b"khor-transfer-offset-v1");
    hash.update(&id.0);
    hash.update(&record[..40]);
    record[40..].copy_from_slice(hash.finalize().as_bytes());
    record
}

fn recover(
    body: &mut File,
    journal: &mut File,
    id: TransferId,
    total: u64,
) -> Result<(u64, blake3::Hasher)> {
    if journal.metadata()?.len() > (MAX_RECORDS * RECORD_BYTES) as u64 {
        return Err(Error::InvalidStage("checkpoint journal exceeds budget"));
    }
    let mut raw = Vec::new();
    journal.read_to_end(&mut raw)?;
    let mut accepted = 0;
    let mut offset = 0;
    let mut prefix = *blake3::hash(&[]).as_bytes();
    for record in raw.chunks_exact(RECORD_BYTES) {
        let end = u64::from_be_bytes(record[..8].try_into().unwrap());
        let digest: [u8; 32] = record[8..40].try_into().unwrap();
        if checkpoint(id, end, digest).as_slice() != record {
            break;
        }
        if accepted == 0 {
            if end != 0 || digest != prefix {
                return Err(Error::InvalidStage("missing initial checkpoint"));
            }
        } else if end <= offset
            || end > total
            || end - offset > CHUNK_BYTES as u64
            || (end != total && end - offset != CHUNK_BYTES as u64)
        {
            return Err(Error::InvalidStage("invalid checkpoint offset"));
        }
        accepted += RECORD_BYTES;
        offset = end;
        prefix = digest;
    }
    if accepted == 0 {
        return Err(Error::InvalidStage("no valid checkpoint"));
    }
    if body.metadata()?.len() < offset {
        return Err(Error::InvalidStage("checkpoint exceeds stored body"));
    }
    let mut hasher = blake3::Hasher::new();
    body.seek(SeekFrom::Start(0))?;
    let mut remaining = offset;
    let mut buffer = vec![0; CHUNK_BYTES];
    while remaining > 0 {
        let n = remaining.min(CHUNK_BYTES as u64) as usize;
        body.read_exact(&mut buffer[..n])?;
        hasher.update(&buffer[..n]);
        remaining -= n as u64;
    }
    if hasher.finalize().as_bytes() != &prefix {
        return Err(Error::InvalidStage("stored prefix digest mismatch"));
    }
    // Reconcile only the unconfirmed tail of this exact, exclusively owned id.
    body.set_len(offset)?;
    body.sync_all()?;
    journal.set_len(accepted as u64)?;
    journal.sync_all()?;
    Ok((offset, hasher))
}

fn create_stage(dir: &Path, manifest: &Manifest, id: TransferId) -> Result<()> {
    create_dir(dir)?;
    #[cfg(test)]
    tests::pause_after_stage_mkdir(dir);
    if let Err(why) = initialize(dir, manifest, id) {
        return Err(initialization_error(dir, why));
    }
    Ok(())
}

fn initialize(dir: &Path, manifest: &Manifest, id: TransferId) -> Result<()> {
    let mut meta = new_file(&dir.join("manifest"))?;
    let encoded = manifest.encode();
    #[cfg(test)]
    tests::partial_control(&mut meta, &encoded, dir, "manifest_partial")?;
    meta.write_all(&encoded)?;
    meta.sync_all()?;
    #[cfg(test)]
    tests::pause_initialization(dir, "manifest_synced");
    drop(meta);
    let body = new_file(&dir.join("body"))?;
    #[cfg(test)]
    tests::pause_initialization(dir, "body_created");
    body.sync_all()?;
    drop(body);
    let mut journal = new_file(&dir.join("offsets"))?;
    let initial = checkpoint(id, 0, *blake3::hash(&[]).as_bytes());
    #[cfg(test)]
    tests::partial_control(&mut journal, &initial, dir, "offsets_partial")?;
    journal.write_all(&initial)?;
    journal.sync_all()?;
    #[cfg(test)]
    tests::pause_initialization(dir, "offsets_synced");
    Ok(())
}

fn initialization_error(dir: &Path, cause: Error) -> Error {
    match clear_initialization(dir) {
        Ok(()) => cause,
        Err(cleanup) => Error::Cleanup {
            cause: Box::new(cause),
            cleanup: io::Error::other(cleanup),
        },
    }
}

// Validate the complete, bounded listing before any unlink. Interrupted
// cleanup is another legal subset; unexpected bytes/names always survive.
fn clear_initialization(dir: &Path) -> Result<()> {
    let Some(entries) = initialization_entries(dir)? else {
        return Ok(());
    };
    #[cfg(test)]
    tests::pause_initialization(dir, "cleanup_before_unlink");
    for path in entries {
        fs::remove_file(path)?;
        #[cfg(test)]
        tests::pause_initialization(dir, "cleanup_after_unlink");
    }
    fs::remove_dir(dir)?;
    Ok(())
}

fn initialization_entries(dir: &Path) -> Result<Option<Vec<PathBuf>>> {
    if dir.file_name() != Some(std::ffi::OsStr::new(INITIALIZING)) {
        return Err(Error::InvalidStage(
            "cleanup requires exact initializer namespace",
        ));
    }
    let metadata = match fs::symlink_metadata(dir) {
        Ok(meta) => meta,
        Err(why) if why.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(why) => return Err(why.into()),
    };
    if !metadata.is_dir() || is_link(&metadata) {
        return Err(Error::InvalidStage("initializer is not a plain directory"));
    }
    let root = fs::symlink_metadata(
        dir.parent()
            .ok_or(Error::InvalidStage("initializer has no parent"))?,
    )?;
    check_initialization_private(&metadata, &root)?;
    let mut paths = Vec::new();
    for entry in fs::read_dir(dir)?.take(4) {
        let entry = entry?;
        if paths.len() == 3 {
            return Err(Error::InvalidStage("initializer has excess controls"));
        }
        let limit = match entry.file_name().to_str() {
            Some("manifest") => MAX_METADATA_BYTES as u64,
            Some("body") => 0,
            Some("offsets") => RECORD_BYTES as u64,
            _ => return Err(Error::InvalidStage("unknown initializer control")),
        };
        let path = entry.path();
        let meta = fs::symlink_metadata(&path)?;
        if !meta.is_file() || is_link(&meta) {
            return Err(Error::InvalidStage(
                "initializer control is not a plain file",
            ));
        }
        check_initialization_private(&meta, &root)?;
        let file = File::open(&path)?;
        platform::check_control_file(&file)?;
        let held = file.metadata()?;
        check_initialization_private(&held, &root)?;
        if held.len() > limit {
            return Err(Error::InvalidStage("initializer control exceeds its bound"));
        }
        paths.push(path);
    }
    Ok(Some(paths))
}

fn check_initialization_private(meta: &fs::Metadata, root: &fs::Metadata) -> Result<()> {
    check_private(meta)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if meta.uid() != root.uid() {
            return Err(Error::InvalidStage("initializer owner differs from root"));
        }
    }
    let _ = root;
    Ok(())
}

fn is_link(meta: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if meta.file_attributes() & 0x400 != 0 {
            return true;
        }
    }
    meta.file_type().is_symlink()
}

fn read_manifest(dir: &Path) -> Result<Manifest> {
    let mut bytes = Vec::new();
    private_file(&dir.join("manifest"), false)?
        .take((MAX_METADATA_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    Manifest::decode(&bytes)
}

fn try_lock(file: &File) -> Result<()> {
    match file.try_lock() {
        Ok(()) => Ok(()),
        Err(fs::TryLockError::WouldBlock) => Err(Error::Busy),
        Err(fs::TryLockError::Error(why)) => Err(why.into()),
    }
}

fn create_dir(path: &Path) -> Result<()> {
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path)?;
    Ok(())
}

fn private_dir(path: &Path) -> Result<()> {
    match create_dir(path) {
        Ok(()) => Ok(()),
        Err(Error::Io(why)) if why.kind() == io::ErrorKind::AlreadyExists => check_dir(path),
        Err(why) => Err(why),
    }
}

fn check_dir(path: &Path) -> Result<()> {
    let meta = fs::symlink_metadata(path)?;
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Err(Error::InvalidStage("stage root is not a directory"));
    }
    check_private(&meta)
}

fn check_file(path: &Path) -> Result<()> {
    let meta = fs::symlink_metadata(path)?;
    if !meta.is_file() || meta.file_type().is_symlink() {
        return Err(Error::InvalidStage("stage entry is not a regular file"));
    }
    check_private(&meta)
}

fn check_private(meta: &fs::Metadata) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if meta.permissions().mode() & 0o077 != 0 {
            return Err(Error::InvalidStage("stage permissions are not private"));
        }
    }
    let _ = meta;
    Ok(())
}

fn file_options() -> OpenOptions {
    let mut options = OpenOptions::new();
    options.read(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options
}

fn new_file(path: &Path) -> Result<File> {
    Ok(file_options().create_new(true).open(path)?)
}

fn private_file(path: &Path, create: bool) -> Result<File> {
    if create {
        match new_file(path) {
            Ok(file) => return Ok(file),
            Err(Error::Io(why)) if why.kind() == io::ErrorKind::AlreadyExists => {}
            Err(why) => return Err(why),
        }
    }
    check_file(path)?;
    Ok(file_options().open(path)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Compiled only into this unit-test executable. The owned child alone has
    // these environment variables; ordinary tests and production have no hook.
    pub(super) fn pause_after_stage_mkdir(dir: &Path) {
        let fault = std::env::var("KHOR_TRANSFER_INIT_FAULT").unwrap_or_default();
        if selected_root(dir) && matches!(fault.as_str(), "existing_manifest" | "unknown_control") {
            // A real create_new collision inside our successfully-created
            // namespace exercises ordinary I/O-error cleanup.
            new_file(&dir.join("manifest")).unwrap();
            if fault == "unknown_control" {
                new_file(&dir.join("keep"))
                    .unwrap()
                    .write_all(b"sentinel")
                    .unwrap();
            }
        }
        pause_initialization(dir, "after_mkdir");
    }

    fn selected_root(dir: &Path) -> bool {
        let Some(root) = std::env::var_os("KHOR_TRANSFER_INIT_PAUSE_ROOT") else {
            return false;
        };
        dir.parent() == Some(Path::new(&root))
    }

    fn selected_phase(dir: &Path, phase: &str) -> bool {
        selected_root(dir)
            && std::env::var("KHOR_TRANSFER_INIT_PHASE").unwrap_or_else(|_| "after_mkdir".into())
                == phase
    }

    pub(super) fn partial_control(
        file: &mut File,
        bytes: &[u8],
        dir: &Path,
        phase: &str,
    ) -> io::Result<()> {
        if selected_phase(dir, phase) {
            file.write_all(&bytes[..bytes.len() / 2])?;
            pause_initialization(dir, phase);
            file.seek(SeekFrom::Start(0))?;
        }
        Ok(())
    }

    pub(super) fn pause_initialization(dir: &Path, phase: &str) {
        if !selected_phase(dir, phase) {
            return;
        }
        let ready = PathBuf::from(
            std::env::var_os("KHOR_TRANSFER_INIT_READY").expect("owned initializer ready path"),
        );
        let temporary = ready.with_extension("preparing");
        fs::write(&temporary, dir.file_name().unwrap().as_encoded_bytes()).unwrap();
        fs::rename(&temporary, &ready).unwrap();
        // Fuse only: the parent must kill/reap while create_stage still owns
        // the real gate and slot, before manifest initialization can run.
        std::thread::sleep(std::time::Duration::from_secs(30));
    }

    struct InitScratch(PathBuf);
    impl InitScratch {
        fn new() -> Self {
            let base = std::env::var_os("KHOR_TRANSFER_SCRATCH")
                .map(PathBuf::from)
                .unwrap_or_else(std::env::temp_dir);
            fs::create_dir_all(&base).unwrap();
            let root = base.join(format!(
                "khor-transfer-init-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            create_dir(&root).unwrap();
            Self(root)
        }
    }
    impl Drop for InitScratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    struct InitWorker(Option<std::process::Child>);
    impl Drop for InitWorker {
        fn drop(&mut self) {
            if let Some(child) = &mut self.0 {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }

    fn init_manifest(binding: &[u8], body: &[u8]) -> Manifest {
        Manifest {
            source: [41; 32],
            recipient: [42; 32],
            binding: binding.to_vec(),
            total_bytes: body.len() as u64,
            digest: *blake3::hash(body).as_bytes(),
        }
    }

    #[test]
    fn a_crashed_initializer_does_not_block_new_reservations() {
        use std::{
            process::{Command, Stdio},
            time::{Duration, Instant},
        };
        let scratch = InitScratch::new();
        let root = scratch.0.join("stages");
        let store = TransferStore::open(&root).unwrap();
        let retained_body = b"retained user bytes";
        let retained = init_manifest(b"retained control", retained_body);
        let mut existing = store.receive(retained.clone()).unwrap();
        existing.append(0, retained_body).unwrap();
        drop(existing);
        let retained_dir = root.join(retained.id().unwrap().to_string());
        let before: Vec<_> = ["manifest", "body", "offsets"]
            .into_iter()
            .map(|name| (name, fs::read(retained_dir.join(name)).unwrap()))
            .collect();
        let ready = scratch.0.join("init-ready");
        let log = File::create(scratch.0.join("initializer.log")).unwrap();
        let mut worker = InitWorker(Some(
            Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "transfer::tests::initialization_crash_worker",
                    "--ignored",
                    "--nocapture",
                ])
                .env("KHOR_TRANSFER_INIT_PAUSE_ROOT", &root)
                .env("KHOR_TRANSFER_INIT_READY", &ready)
                .stdout(Stdio::from(log.try_clone().unwrap()))
                .stderr(Stdio::from(log))
                .spawn()
                .unwrap(),
        ));
        let child_pid = worker.0.as_ref().unwrap().id();
        let start = Instant::now();
        while !ready.exists() {
            assert!(
                worker.0.as_mut().unwrap().try_wait().unwrap().is_none(),
                "initializer exited before the real mkdir barrier"
            );
            assert!(
                start.elapsed() < Duration::from_secs(5),
                "owned initializer did not reach create_stage mkdir barrier"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        let actual_name = fs::read_to_string(&ready).unwrap();
        assert_eq!(Path::new(&actual_name).components().count(), 1);
        let actual = root.join(&actual_name);
        assert!(actual.is_dir());
        assert_eq!(
            fs::read_dir(&actual).unwrap().count(),
            0,
            "the worker must be paused before manifest creation"
        );
        assert!(
            matches!(store.reserved_bytes(), Err(Error::Busy)),
            "the actual initializer must retain the global OS gate"
        );
        worker.0.as_mut().unwrap().kill().unwrap();
        let status = worker.0.as_mut().unwrap().wait().unwrap();
        assert!(
            !status.success(),
            "worker must be killed, not finish initialization"
        );
        worker.0 = None;
        for slot in 0..MAX_OWNERS {
            let file = private_file(&root.join(format!("owner-{slot}")), false).unwrap();
            assert!(
                file.try_lock().is_ok(),
                "killed initializer must release its real owner slot"
            );
        }
        let reserved = store.reserved_bytes();
        let fresh_body = b"new transfer after real initialization crash";
        let fresh = store
            .receive(init_manifest(b"fresh after crash", fresh_body))
            .and_then(|mut sink| sink.append(0, fresh_body));
        eprintln!(
            "[Q039-INIT] killed_pid={child_pid} status={status} actual_created={actual_name} reserved={reserved:?} fresh={fresh:?}"
        );
        for (name, bytes) in &before {
            assert_eq!(
                fs::read(retained_dir.join(name)).unwrap(),
                *bytes,
                "recovery must preserve published retained {name}"
            );
        }
        assert!(
            fresh.is_ok(),
            "a new reservation must recover after the real initializer crash without manual removal: {fresh:?}"
        );
        assert_eq!(reserved.unwrap(), retained_body.len() as u64);
        assert!(matches!(fresh.unwrap(), Progress::VerifiedComplete { .. }));
        let resumed = store.receive(retained).unwrap();
        assert!(matches!(
            resumed.progress().unwrap(),
            Progress::VerifiedComplete { .. }
        ));
    }

    #[test]
    #[ignore = "actually spawned, killed and reaped by the initialization parent"]
    fn initialization_crash_worker() {
        let root = PathBuf::from(
            std::env::var_os("KHOR_TRANSFER_INIT_PAUSE_ROOT").expect("owned initializer root"),
        );
        let store = TransferStore::open(&root).unwrap();
        let received = store.receive(init_manifest(
            b"initializer to kill",
            b"never received body bytes",
        ));
        match std::env::var("KHOR_TRANSFER_INIT_FAULT").as_deref() {
            Ok("existing_manifest") => assert!(
                matches!(received, Err(Error::Io(why)) if why.kind() == io::ErrorKind::AlreadyExists)
            ),
            Ok("unknown_control") => match received {
                Err(Error::Cleanup { cause, cleanup }) => {
                    assert!(
                        matches!(*cause, Error::Io(why) if why.kind() == io::ErrorKind::AlreadyExists)
                    );
                    assert!(cleanup.to_string().contains("unknown initializer control"));
                }
                _ => panic!("initialization must retain both errors"),
            },
            _ => drop(received.unwrap()),
        }
    }

    fn phase_worker(root: &Path, phase: &str, fault: &str) -> (InitWorker, PathBuf, PathBuf) {
        use std::process::{Command, Stdio};
        let ready = root
            .parent()
            .unwrap()
            .join(format!("{phase}-{fault}-ready"));
        let log_path = ready.with_extension("log");
        let log = File::create(&log_path).unwrap();
        let child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "transfer::tests::initialization_crash_worker",
                "--ignored",
                "--nocapture",
            ])
            .env("KHOR_TRANSFER_INIT_PAUSE_ROOT", root)
            .env("KHOR_TRANSFER_INIT_READY", &ready)
            .env("KHOR_TRANSFER_INIT_PHASE", phase)
            .env("KHOR_TRANSFER_INIT_FAULT", fault)
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap();
        (InitWorker(Some(child)), ready, log_path)
    }

    fn kill_at(root: &Path, phase: &str) -> PathBuf {
        use std::time::{Duration, Instant};
        let (mut worker, ready, log) = phase_worker(root, phase, "");
        let start = Instant::now();
        while !ready.exists() {
            assert!(
                worker.0.as_mut().unwrap().try_wait().unwrap().is_none(),
                "worker exited: {}",
                fs::read_to_string(&log).unwrap()
            );
            assert!(
                start.elapsed() < Duration::from_secs(5),
                "worker missed {phase} barrier"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        let actual = root.join(fs::read_to_string(ready).unwrap());
        assert!(matches!(
            TransferStore::open(root).unwrap().reserved_bytes(),
            Err(Error::Busy)
        ));
        let pid = worker.0.as_ref().unwrap().id();
        worker.0.as_mut().unwrap().kill().unwrap();
        let status = worker.0.as_mut().unwrap().wait().unwrap();
        assert!(!status.success());
        worker.0 = None;
        for slot in 0..MAX_OWNERS {
            assert!(
                private_file(&root.join(format!("owner-{slot}")), false)
                    .unwrap()
                    .try_lock()
                    .is_ok()
            );
        }
        eprintln!(
            "[Q039-INIT-PHASE] phase={phase} killed_pid={pid} status={status} actual={}",
            actual.display()
        );
        actual
    }

    #[test]
    fn every_initialization_crash_phase_releases_gate_and_recovers() {
        for phase in [
            "before_mkdir",
            "manifest_partial",
            "manifest_synced",
            "body_created",
            "offsets_partial",
            "offsets_synced",
            "before_publish",
            "after_publish",
        ] {
            let scratch = InitScratch::new();
            let root = scratch.0.join("stages");
            let store = TransferStore::open(&root).unwrap();
            let retained = init_manifest(b"retain through phases", b"retained bytes");
            let mut sink = store.receive(retained.clone()).unwrap();
            sink.append(0, b"retained bytes").unwrap();
            drop(sink);
            let published = root.join(retained.id().unwrap().to_string());
            let before: Vec<_> = ["manifest", "body", "offsets"]
                .map(|name| fs::read(published.join(name)).unwrap())
                .into();
            let actual = kill_at(&root, phase);
            match phase {
                "before_mkdir" => assert!(!actual.exists()),
                "manifest_partial" => assert!(
                    fs::metadata(actual.join("manifest")).unwrap().len() < HEADER_BYTES as u64
                ),
                "body_created" => assert_eq!(fs::metadata(actual.join("body")).unwrap().len(), 0),
                "offsets_partial" => assert_eq!(
                    fs::metadata(actual.join("offsets")).unwrap().len(),
                    (RECORD_BYTES / 2) as u64
                ),
                "after_publish" => assert_eq!(
                    actual.file_name().unwrap(),
                    init_manifest(b"initializer to kill", b"never received body bytes")
                        .id()
                        .unwrap()
                        .to_string()
                        .as_str()
                ),
                _ => assert!(actual.is_dir()),
            }
            let expected = b"retained bytes".len() as u64
                + if phase == "after_publish" {
                    b"never received body bytes".len() as u64
                } else {
                    0
                };
            assert_eq!(store.reserved_bytes().unwrap(), expected, "{phase}");
            assert!(!root.join(INITIALIZING).exists());
            for (name, bytes) in ["manifest", "body", "offsets"].into_iter().zip(before) {
                assert_eq!(
                    fs::read(published.join(name)).unwrap(),
                    bytes,
                    "{phase}/{name}"
                );
            }
            let mut fresh = store
                .receive(init_manifest(b"fresh phases", b"new"))
                .unwrap();
            assert!(matches!(
                fresh.append(0, b"new").unwrap(),
                Progress::VerifiedComplete { .. }
            ));
            drop(fresh);
            assert!(matches!(
                store.receive(retained).unwrap().progress().unwrap(),
                Progress::VerifiedComplete { .. }
            ));
            if phase == "after_publish" {
                let mut resumed = store
                    .receive(init_manifest(
                        b"initializer to kill",
                        b"never received body bytes",
                    ))
                    .unwrap();
                assert_eq!(resumed.next_offset(), 0);
                assert!(matches!(
                    resumed.append(0, b"never received body bytes").unwrap(),
                    Progress::VerifiedComplete { .. }
                ));
            }
        }
    }

    #[test]
    fn a_second_crash_during_cleanup_recovers_the_remaining_controls() {
        let scratch = InitScratch::new();
        let root = scratch.0.join("stages");
        let store = TransferStore::open(&root).unwrap();
        kill_at(&root, "offsets_synced");
        assert_eq!(fs::read_dir(root.join(INITIALIZING)).unwrap().count(), 3);
        kill_at(&root, "cleanup_after_unlink");
        assert_eq!(fs::read_dir(root.join(INITIALIZING)).unwrap().count(), 2);
        assert_eq!(store.reserved_bytes().unwrap(), 0);
        let mut fresh = store
            .receive(init_manifest(b"after cleanup crash", b"ok"))
            .unwrap();
        assert!(matches!(
            fresh.append(0, b"ok").unwrap(),
            Progress::VerifiedComplete { .. }
        ));
    }

    #[test]
    fn normal_initialization_io_failure_cleans_only_valid_owned_controls() {
        use std::time::{Duration, Instant};
        for fault in ["existing_manifest", "unknown_control"] {
            let scratch = InitScratch::new();
            let root = scratch.0.join("stages");
            let store = TransferStore::open(&root).unwrap();
            let (mut worker, _, log) = phase_worker(&root, "disabled", fault);
            let start = Instant::now();
            let status = loop {
                if let Some(status) = worker.0.as_mut().unwrap().try_wait().unwrap() {
                    break status;
                }
                assert!(start.elapsed() < Duration::from_secs(5));
                std::thread::sleep(Duration::from_millis(5));
            };
            worker.0 = None;
            assert!(status.success(), "{}", fs::read_to_string(log).unwrap());
            if fault == "unknown_control" {
                assert!(store.reserved_bytes().is_err());
                assert_eq!(
                    fs::read(root.join(INITIALIZING).join("keep")).unwrap(),
                    b"sentinel"
                );
                assert_eq!(
                    fs::read(root.join(INITIALIZING).join("manifest")).unwrap(),
                    b""
                );
            } else {
                assert!(!root.join(INITIALIZING).exists());
                assert_eq!(store.reserved_bytes().unwrap(), 0);
                // Both actual owner slots are available after ordinary error.
                let first = store
                    .receive(init_manifest(b"first owner", b"one"))
                    .unwrap();
                let second = store
                    .receive(init_manifest(b"second owner", b"two"))
                    .unwrap();
                assert_eq!(store.reserved_bytes().unwrap(), 6);
                drop((first, second));
            }
        }
    }

    fn namespace_bytes(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
        let mut entries: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .map(|entry| {
                let path = entry.unwrap().path();
                let meta = fs::symlink_metadata(&path).unwrap();
                let bytes = if meta.file_type().is_symlink() {
                    fs::read_link(&path)
                        .unwrap()
                        .as_os_str()
                        .as_encoded_bytes()
                        .to_vec()
                } else if meta.is_file() {
                    fs::read(&path).unwrap()
                } else {
                    Vec::new()
                };
                (path, bytes)
            })
            .collect();
        entries.sort();
        entries
    }

    #[test]
    fn unsafe_initializer_controls_are_preserved_before_any_unlink() {
        for control in [
            "unknown",
            "fourth",
            "nested",
            "body_bytes",
            "large_manifest",
            "large_offsets",
        ] {
            let scratch = InitScratch::new();
            let root = scratch.0.join("stages");
            let store = TransferStore::open(&root).unwrap();
            let dir = root.join(INITIALIZING);
            create_dir(&dir).unwrap();
            new_file(&dir.join("manifest"))
                .unwrap()
                .write_all(b"bounded partial metadata")
                .unwrap();
            match control {
                "unknown" => {
                    new_file(&dir.join("keep"))
                        .unwrap()
                        .write_all(b"sentinel")
                        .unwrap();
                }
                "fourth" => {
                    for name in ["body", "offsets", "keep"] {
                        new_file(&dir.join(name)).unwrap();
                    }
                }
                "nested" => create_dir(&dir.join("body")).unwrap(),
                "body_bytes" => {
                    new_file(&dir.join("body"))
                        .unwrap()
                        .write_all(b"must not erase body bytes")
                        .unwrap();
                }
                "large_manifest" => new_file(&dir.join("body")).unwrap().set_len(0).unwrap(),
                "large_offsets" => new_file(&dir.join("offsets"))
                    .unwrap()
                    .set_len(RECORD_BYTES as u64 + 1)
                    .unwrap(),
                _ => unreachable!(),
            }
            if control == "large_manifest" {
                file_options()
                    .open(dir.join("manifest"))
                    .unwrap()
                    .set_len(MAX_METADATA_BYTES as u64 + 1)
                    .unwrap();
            }
            let before = namespace_bytes(&dir);
            assert!(store.reserved_bytes().is_err(), "{control}");
            assert_eq!(namespace_bytes(&dir), before, "{control}");
            assert!(
                store
                    .receive(init_manifest(b"refuse unsafe cleanup", b"new"))
                    .is_err()
            );
            assert_eq!(namespace_bytes(&dir), before);
            assert!(store.cancel(&init_manifest(b"absent", b"")).is_err());
            assert_eq!(namespace_bytes(&dir), before);
        }
    }

    #[cfg(unix)]
    #[test]
    fn initializer_links_and_nonprivate_entries_keep_their_targets_and_modes() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        for control in [
            "file_link",
            "hardlink",
            "directory_link",
            "directory_mode",
            "file_mode",
        ] {
            let scratch = InitScratch::new();
            let root = scratch.0.join("stages");
            let store = TransferStore::open(&root).unwrap();
            let sentinel = scratch.0.join("sentinel");
            new_file(&sentinel)
                .unwrap()
                .write_all(b"retained sentinel")
                .unwrap();
            let dir = root.join(INITIALIZING);
            if control == "directory_link" {
                symlink(&scratch.0, &dir).unwrap();
            } else {
                create_dir(&dir).unwrap();
                match control {
                    "file_link" => symlink(&sentinel, dir.join("manifest")).unwrap(),
                    "hardlink" => fs::hard_link(&sentinel, dir.join("manifest")).unwrap(),
                    "directory_mode" => {
                        fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap()
                    }
                    "file_mode" => {
                        new_file(&dir.join("manifest")).unwrap();
                        fs::set_permissions(
                            dir.join("manifest"),
                            fs::Permissions::from_mode(0o644),
                        )
                        .unwrap();
                    }
                    _ => unreachable!(),
                }
            }
            assert!(store.reserved_bytes().is_err(), "{control}");
            assert!(fs::symlink_metadata(&dir).is_ok());
            assert_eq!(fs::read(&sentinel).unwrap(), b"retained sentinel");
            match control {
                "directory_link" => assert_eq!(fs::read_link(&dir).unwrap(), scratch.0),
                "file_link" => assert_eq!(fs::read_link(dir.join("manifest")).unwrap(), sentinel),
                "hardlink" => assert_eq!(
                    fs::read(dir.join("manifest")).unwrap(),
                    b"retained sentinel"
                ),
                "directory_mode" => assert_eq!(
                    fs::metadata(&dir).unwrap().permissions().mode() & 0o777,
                    0o755
                ),
                "file_mode" => assert_eq!(
                    fs::metadata(dir.join("manifest"))
                        .unwrap()
                        .permissions()
                        .mode()
                        & 0o777,
                    0o644
                ),
                _ => unreachable!(),
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn initializer_file_owner_must_match_the_supplied_root_owner() {
        use std::os::unix::fs::MetadataExt;
        let scratch = InitScratch::new();
        let file = new_file(&scratch.0.join("manifest")).unwrap();
        let actual = file.metadata().unwrap();
        let system_root = fs::metadata("/").unwrap();
        if actual.uid() != system_root.uid() {
            assert!(matches!(
                check_initialization_private(&actual, &system_root),
                Err(Error::InvalidStage("initializer owner differs from root"))
            ));
        }
        assert!(check_initialization_private(&actual, &fs::metadata(&scratch.0).unwrap()).is_ok());
    }

    #[test]
    fn publication_never_replaces_an_existing_file_or_directory() {
        for target in ["empty_directory", "sentinel_directory", "file"] {
            let scratch = InitScratch::new();
            let source = scratch.0.join(INITIALIZING);
            create_dir(&source).unwrap();
            new_file(&source.join("manifest"))
                .unwrap()
                .write_all(b"source control")
                .unwrap();
            let destination = scratch.0.join("published");
            if target == "file" {
                new_file(&destination)
                    .unwrap()
                    .write_all(b"target file")
                    .unwrap();
            } else {
                create_dir(&destination).unwrap();
                if target == "sentinel_directory" {
                    new_file(&destination.join("keep"))
                        .unwrap()
                        .write_all(b"target sentinel")
                        .unwrap();
                }
            }
            assert!(
                platform::publish_no_replace(&source, &destination).is_err(),
                "{target}"
            );
            assert_eq!(
                fs::read(source.join("manifest")).unwrap(),
                b"source control"
            );
            match target {
                "file" => assert_eq!(fs::read(destination).unwrap(), b"target file"),
                "empty_directory" => assert_eq!(fs::read_dir(destination).unwrap().count(), 0),
                _ => assert_eq!(
                    fs::read(destination.join("keep")).unwrap(),
                    b"target sentinel"
                ),
            }
        }
    }

    #[test]
    fn damaged_published_stages_are_never_initializer_cleanup_targets() {
        for state in ["empty", "corrupt", "sentinel"] {
            let scratch = InitScratch::new();
            let root = scratch.0.join("stages");
            let store = TransferStore::open(&root).unwrap();
            let manifest = init_manifest(b"existing incomplete final", b"body");
            let final_dir = root.join(manifest.id().unwrap().to_string());
            create_dir(&final_dir).unwrap();
            if state == "sentinel" {
                new_file(&final_dir.join("keep"))
                    .unwrap()
                    .write_all(b"final sentinel")
                    .unwrap();
            }
            if state == "corrupt" {
                new_file(&final_dir.join("manifest"))
                    .unwrap()
                    .write_all(b"broken published metadata")
                    .unwrap();
                new_file(&final_dir.join("body"))
                    .unwrap()
                    .write_all(b"published user bytes")
                    .unwrap();
            }
            let before = namespace_bytes(&final_dir);
            assert!(store.reserved_bytes().is_err());
            assert!(store.receive(manifest.clone()).is_err());
            assert!(store.cancel(&manifest).is_err());
            assert!(final_dir.is_dir());
            assert_eq!(namespace_bytes(&final_dir), before);
        }
    }

    #[test]
    fn a_failed_stage_mkdir_never_cleans_an_existing_directory() {
        let base = std::env::var_os("KHOR_TRANSFER_SCRATCH")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        fs::create_dir_all(&base).unwrap();
        let root = base.join(format!(
            "khor-transfer-existing-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        struct Cleanup(PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }
        create_dir(&root).unwrap();
        let cleanup = Cleanup(root.clone());
        fs::write(root.join("keep"), b"not created by initialize").unwrap();
        let manifest = Manifest {
            source: [1; 32],
            recipient: [2; 32],
            binding: b"existing stage".to_vec(),
            total_bytes: 0,
            digest: *blake3::hash(&[]).as_bytes(),
        };
        assert!(matches!(
            create_stage(&root, &manifest, manifest.id().unwrap()),
            Err(Error::Io(_))
        ));
        assert_eq!(
            fs::read(root.join("keep")).unwrap(),
            b"not created by initialize"
        );
        drop(cleanup);
    }

    #[test]
    fn an_uncertain_checkpoint_write_cannot_ack_another_chunk_until_reopened() {
        let base = std::env::var_os("KHOR_TRANSFER_SCRATCH")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        fs::create_dir_all(&base).unwrap();
        let root = base.join(format!(
            "khor-transfer-io-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        struct Cleanup(PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }
        let cleanup = Cleanup(root.clone());
        let store = TransferStore::open(&root).unwrap();
        let bytes = vec![3; CHUNK_BYTES + 5];
        let meta = Manifest {
            source: [1; 32],
            recipient: [2; 32],
            binding: b"checkpoint-I/O-fault".to_vec(),
            total_bytes: bytes.len() as u64,
            digest: *blake3::hash(&bytes).as_bytes(),
        };
        let id = meta.id().unwrap();
        let mut sink = store.receive(meta.clone()).unwrap();
        // Real read-only file handle makes checkpoint persistence fail after
        // body sync, without a production fault hook or fake success path.
        sink.journal = File::open(root.join(id.to_string()).join("offsets")).unwrap();
        assert!(matches!(
            sink.append(0, &bytes[..CHUNK_BYTES]),
            Err(Error::Io(_))
        ));
        assert!(matches!(sink.progress(), Err(Error::InvalidStage(_))));
        assert!(matches!(
            sink.append(0, &bytes[..CHUNK_BYTES]),
            Err(Error::InvalidStage(_))
        ));
        drop(sink);
        let mut reopened = store.receive(meta).unwrap();
        assert_eq!(reopened.next_offset(), 0);
        reopened.append(0, &bytes[..CHUNK_BYTES]).unwrap();
        assert!(matches!(
            reopened
                .append(CHUNK_BYTES as u64, &bytes[CHUNK_BYTES..])
                .unwrap(),
            Progress::VerifiedComplete { .. }
        ));
        drop(reopened);
        drop(cleanup);
    }
}
