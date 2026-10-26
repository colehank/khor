//! Checked import into a live document, followed by the existing block flush.
//! This boundary does not issue network acknowledgements or promise directory
//! durability across power loss. Commit errors may leave live or disk changes.

use super::*;
use std::fmt;
use std::io;

/// The receiver's advertised durable base and the sender's fixed target.
/// The authenticated transfer layer binds these to its document and body.
#[derive(Clone, Debug)]
pub struct ApplySpec {
    pub expected_epoch: u32,
    pub base_vv: VersionVector,
    pub target_vv: VersionVector,
}

/// A successful checked live import and block/ledger flush, not an Applied ACK.
/// Only `Loaded::apply_checked` can construct this proof of the local boundary.
#[derive(Debug)]
pub struct FlushedImport {
    version: VersionVector,
    epoch: u32,
    report: MergeReport,
    block: Option<PathBuf>,
    retired_pen: Option<u64>,
}

impl FlushedImport {
    pub fn version(&self) -> &VersionVector {
        &self.version
    }
    pub fn epoch(&self) -> u32 {
        self.epoch
    }
    pub fn report(&self) -> &MergeReport {
        &self.report
    }
    pub fn block(&self) -> Option<&Path> {
        self.block.as_deref()
    }
    pub fn retired_pen(&self) -> Option<u64> {
        self.retired_pen
    }
}

/// Candidate/fence failures precede live import. `CommitFailed` can follow a
/// successful live import or a partial disk write; callers must retain/retry
/// their transfer rather than announce success or assume rollback.
#[derive(Debug, PartialEq, Eq)]
pub enum ApplyError {
    WrongEpoch { expected: u32, loaded: u32 },
    StaleBase,
    InvalidVersion,
    SeedFailure(String),
    InvalidBody(String),
    PendingImport,
    MissingTarget,
    MissingWriterLease,
    NeedsReload,
    FenceUnavailable(String),
    FenceLost,
    EpochIo(String),
    InvalidEpoch,
    StaleEpoch { expected: u32, disk: u32 },
    CommitFailed(String),
}

impl fmt::Display for ApplyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ApplyError {}

impl<D: Doc> Loaded<D> {
    /// Validate using an applied-state-only candidate, then import into the
    /// original live document while retaining its pen and real pile fence.
    /// Business writers must not mutate this document through another alias
    /// during this call. Existing pending imports must have been registered.
    pub fn apply_checked(
        &mut self,
        spec: &ApplySpec,
        bytes: &[u8],
    ) -> Result<FlushedImport, ApplyError> {
        if spec.expected_epoch != self.store.epoch {
            return Err(ApplyError::WrongEpoch {
                expected: spec.expected_epoch,
                loaded: self.store.epoch,
            });
        }
        if [&spec.base_vv, &spec.target_vv]
            .iter()
            .any(|vv| vv.iter().any(|(_, n)| *n < 0))
        {
            return Err(ApplyError::InvalidVersion);
        }
        let before = self.doc.version();
        if !before.includes_vv(&spec.base_vv) || !self.store.on_disk.includes_vv(&spec.base_vv) {
            return Err(ApplyError::StaleBase);
        }
        let projected = {
            let candidate = D::open(self.doc.peer_id()).map_err(ApplyError::SeedFailure)?;
            if candidate.version() != VersionVector::default() {
                return Err(ApplyError::SeedFailure(
                    "candidate constructor minted operations".into(),
                ));
            }
            let snapshot = self.doc.snapshot().map_err(ApplyError::SeedFailure)?;
            let seed = candidate
                .merge_report(&snapshot)
                .map_err(ApplyError::SeedFailure)?;
            if !seed.pending.is_empty() || candidate.version() != before {
                return Err(ApplyError::SeedFailure(
                    "snapshot does not reproduce applied version".into(),
                ));
            }
            let report = candidate
                .merge_report(bytes)
                .map_err(ApplyError::InvalidBody)?;
            if !report.pending.is_empty() {
                return Err(ApplyError::PendingImport);
            }
            let version = candidate.version();
            if !covers(&version, &before, spec) {
                return Err(ApplyError::MissingTarget);
            }
            version
        };
        if self.doc.version() != before {
            return Err(ApplyError::NeedsReload);
        }
        self.ensure_apply_fence(spec.expected_epoch)?;
        let own = self.doc.peer_id();
        let own_before = before.get(&own).copied().unwrap_or(0);
        if self.store.pen.is_none()
            && self.store.retired != Some(own)
            && (projected.get(&own).copied().unwrap_or(0) > own_before
                || self.store.pending.get(&own).copied().unwrap_or(0) > own_before)
        {
            // A spare has no persistent pen to retire. If import then flush
            // failed, dropping it could lend this old name again. Reject before
            // touching live. The control fence/epoch now exists, so after this
            // Loaded drops a fresh reload can obtain a persistent pen. This
            // failure may create control metadata, never a document block.
            return Err(ApplyError::MissingWriterLease);
        }
        // No import has happened yet. Retire before importing if either the
        // candidate or registered old pending could advance this pen's writer.
        // A retirement failure therefore cannot return the old pen with new ops.
        let mut retired_pen = self
            .store
            .retire_before_import(&before, &projected)
            .map_err(ApplyError::CommitFailed)?;
        let report = self
            .doc
            .merge_report(bytes)
            .map_err(ApplyError::CommitFailed)?;
        self.store.record_import(&report);
        let actual = self.doc.version();
        // Recheck the real import, including operations unlocked from the old
        // live pending queue. This also retains the ordinary took_in semantics.
        let retired_actual = self
            .store
            .took_in_checked(&before, &actual)
            .map_err(ApplyError::CommitFailed)?;
        retired_pen = retired_pen.or(retired_actual);
        if !covers(&actual, &before, spec) {
            return Err(ApplyError::CommitFailed(
                "live import did not cover required versions".into(),
            ));
        }
        self.check_apply_fence(spec.expected_epoch)
            .map_err(|e| ApplyError::CommitFailed(e.to_string()))?;
        let block = self
            .store
            .flush(&self.doc)
            .map_err(ApplyError::CommitFailed)?;
        self.check_apply_fence(spec.expected_epoch)
            .map_err(|e| ApplyError::CommitFailed(e.to_string()))?;
        if !self.store.on_disk.includes_vv(&actual)
            || !self.store.on_disk.includes_vv(&spec.target_vv)
        {
            return Err(ApplyError::CommitFailed(
                "flush did not cover the applied version".into(),
            ));
        }
        Ok(FlushedImport {
            version: actual,
            epoch: self.store.epoch,
            report,
            block,
            retired_pen,
        })
    }

    fn ensure_apply_fence(&mut self, expected: u32) -> Result<(), ApplyError> {
        if self.lease.pile.is_some() {
            // Never upgrade or drop a held shared lock to initialize an epoch.
            return self.check_apply_fence(expected);
        }
        make_dir(&self.store.dir).map_err(ApplyError::FenceUnavailable)?;
        let file = open_fence(&self.store.dir.join(PILE_LOCK))
            .map_err(|e| ApplyError::FenceUnavailable(e.to_string()))?;
        let exclusive = self.store.stamp;
        let started = std::time::Instant::now();
        loop {
            let locked = if exclusive {
                file.try_lock()
            } else {
                file.try_lock_shared()
            };
            match locked {
                Ok(()) => break,
                Err(fs::TryLockError::WouldBlock) if started.elapsed() < PILE_WAIT => {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                Err(e) => return Err(ApplyError::FenceUnavailable(e.to_string())),
            }
        }
        check_identity(&file, &self.store.dir.join(PILE_LOCK))?;
        let disk = strict_epoch(&self.store.dir)?;
        if exclusive {
            // A formerly absent pile must still be empty. Do not silently mix
            // the stale Loaded with a pile created while it had no fence.
            let mut entries = fs::read_dir(&self.store.dir)
                .map_err(|e| ApplyError::FenceUnavailable(e.to_string()))?;
            while let Some(entry) = entries.next() {
                let entry = entry.map_err(|e| ApplyError::FenceUnavailable(e.to_string()))?;
                if entry.path().extension().is_some_and(|x| x == EXT) {
                    return Err(ApplyError::NeedsReload);
                }
            }
            if let Some(disk) = disk {
                if disk != expected {
                    return Err(ApplyError::StaleEpoch { expected, disk });
                }
            } else {
                write_atomic(
                    &self.store.dir.join(EPOCH_FILE),
                    format!("{expected}\n").as_bytes(),
                )
                .map_err(ApplyError::CommitFailed)?;
            }
            self.store.stamp = false;
        } else {
            check_epoch(disk, expected, false)?;
        }
        self.lease.pile = Some(file);
        self.check_apply_fence(expected)
    }

    fn check_apply_fence(&self, expected: u32) -> Result<(), ApplyError> {
        let file = self
            .lease
            .pile
            .as_ref()
            .ok_or_else(|| ApplyError::FenceUnavailable("no pile lock".into()))?;
        check_identity(file, &self.store.dir.join(PILE_LOCK))?;
        check_epoch(strict_epoch(&self.store.dir)?, expected, self.store.stamp)
    }
}

fn covers(version: &VersionVector, before: &VersionVector, spec: &ApplySpec) -> bool {
    version.includes_vv(before)
        && version.includes_vv(&spec.base_vv)
        && version.includes_vv(&spec.target_vv)
}

impl BlockStore {
    fn retire_before_import(
        &mut self,
        before: &VersionVector,
        projected: &VersionVector,
    ) -> Result<Option<u64>, String> {
        let Some((_, name)) = &self.pen else {
            return Ok(None);
        };
        let counter = before.get(name).copied().unwrap_or(0);
        if projected.get(name).copied().unwrap_or(0) <= counter
            && self.pending.get(name).copied().unwrap_or(0) <= counter
        {
            return Ok(None);
        }
        self.retire_pen_checked()
    }

    fn took_in_checked(
        &mut self,
        before: &VersionVector,
        actual: &VersionVector,
    ) -> Result<Option<u64>, String> {
        let Some((_, name)) = &self.pen else {
            return Ok(None);
        };
        if actual.get(name).copied().unwrap_or(0) <= before.get(name).copied().unwrap_or(0) {
            return Ok(None);
        }
        self.retire_pen_checked()
    }

    fn retire_pen_checked(&mut self) -> Result<Option<u64>, String> {
        let Some((path, name)) = self.pen.clone() else {
            return Ok(None);
        };
        write_atomic(&path, format!("{:016x}\n", fresh_name()).as_bytes())?;
        self.pen = None;
        self.retired = Some(name);
        Ok(Some(name))
    }
}

fn strict_epoch(dir: &Path) -> Result<Option<u32>, ApplyError> {
    match fs::read(dir.join(EPOCH_FILE)) {
        Ok(bytes) => std::str::from_utf8(&bytes)
            .map_err(|_| ApplyError::InvalidEpoch)?
            .trim()
            .parse()
            .map(Some)
            .map_err(|_| ApplyError::InvalidEpoch),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(ApplyError::EpochIo(e.to_string())),
    }
}

fn check_epoch(disk: Option<u32>, expected: u32, stamp: bool) -> Result<(), ApplyError> {
    match disk {
        Some(disk) if disk == expected => Ok(()),
        Some(disk) => Err(ApplyError::StaleEpoch { expected, disk }),
        None if expected == 0 && !stamp => Ok(()),
        None => Err(ApplyError::NeedsReload),
    }
}

fn open_fence(path: &Path) -> io::Result<fs::File> {
    let mut opts = fs::OpenOptions::new();
    opts.create(true).truncate(false).read(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(OWNER_ONLY_FILE);
    }
    opts.open(path)
}

fn check_identity(file: &fs::File, path: &Path) -> Result<(), ApplyError> {
    if same_file(file, path).map_err(|e| ApplyError::FenceUnavailable(e.to_string()))? {
        Ok(())
    } else {
        Err(ApplyError::FenceLost)
    }
}

#[cfg(unix)]
fn same_file(file: &fs::File, path: &Path) -> io::Result<bool> {
    use std::os::unix::fs::MetadataExt;
    let held = file.metadata()?;
    let current = fs::metadata(path)?;
    Ok(held.dev() == current.dev() && held.ino() == current.ino())
}

#[cfg(windows)]
fn same_file(file: &fs::File, path: &Path) -> io::Result<bool> {
    use std::ffi::c_void;
    use std::os::windows::io::AsRawHandle;
    #[repr(C)]
    #[derive(PartialEq, Eq)]
    struct FileIdInfo {
        volume: u64,
        id: [u8; 16],
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetFileInformationByHandleEx(
            handle: *mut c_void,
            class: i32,
            info: *mut c_void,
            size: u32,
        ) -> i32;
    }
    fn identity(file: &fs::File) -> io::Result<FileIdInfo> {
        let mut info = FileIdInfo {
            volume: 0,
            id: [0; 16],
        };
        // FileIdInfo (0x12) is the full volume serial and 128-bit file ID.
        // SAFETY: info is a writable, repr(C), 24-byte FILE_ID_INFO and the
        // borrowed file keeps the supplied handle valid for this call.
        let ok = unsafe {
            GetFileInformationByHandleEx(
                file.as_raw_handle(),
                0x12,
                (&mut info as *mut FileIdInfo).cast(),
                std::mem::size_of::<FileIdInfo>() as u32,
            )
        };
        if ok == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(info)
        }
    }
    Ok(identity(file)? == identity(&fs::File::open(path)?)?)
}

#[cfg(not(any(unix, windows)))]
fn same_file(_: &fs::File, _: &Path) -> io::Result<bool> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "pile identity is not supported on this platform",
    ))
}
