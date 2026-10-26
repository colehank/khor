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
        Ok(Gate {
            local,
            file: Some(file),
        })
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
            create_stage(&dir, &manifest, id)?;
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
    // Only successful mkdir proves ownership. A process crash during the
    // following initialization is explicit InvalidStage until reconciliation.
    create_dir(dir)?;
    if let Err(why) = initialize(dir, manifest, id) {
        if let Err(cleanup) = fs::remove_dir_all(dir)
            && cleanup.kind() != io::ErrorKind::NotFound
        {
            return Err(Error::Cleanup {
                cause: Box::new(why),
                cleanup,
            });
        }
        return Err(why);
    }
    Ok(())
}

fn initialize(dir: &Path, manifest: &Manifest, id: TransferId) -> Result<()> {
    let mut meta = new_file(&dir.join("manifest"))?;
    meta.write_all(&manifest.encode())?;
    meta.sync_all()?;
    new_file(&dir.join("body"))?.sync_all()?;
    let mut journal = new_file(&dir.join("offsets"))?;
    journal.write_all(&checkpoint(id, 0, *blake3::hash(&[]).as_bytes()))?;
    journal.sync_all()?;
    Ok(())
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
