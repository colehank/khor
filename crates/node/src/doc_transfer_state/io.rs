//! Private bounded metadata publication. File sync + atomic rename supports
//! process restart; this is not a cross-platform power-loss transaction.
use super::Error;
use std::{
    cell::Cell,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    marker::PhantomData,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{Condvar, Mutex, MutexGuard, TryLockError},
    thread::ThreadId,
    time::{Duration, Instant},
};

pub(super) const MAX_IMAGE: usize = 32 * 1024 * 1024;
const MAGIC: &[u8; 8] = b"khor-d01";
const HEADER: usize = 8 + 8 + 8 + 32;
static LOCAL: Mutex<()> = Mutex::new(());
const ROOT_LIMIT: usize = 64;
const WAITER_LIMIT: usize = 64;
const OPEN_WAIT: Duration = Duration::from_secs(1);
static ROOTS: Mutex<RootAdmission> = Mutex::new(RootAdmission {
    owners: Vec::new(),
    waiters: Vec::new(),
    next_id: 0,
});
static ROOT_CHANGED: Condvar = Condvar::new();
static ROOT_NOTIFIED: tokio::sync::Notify = tokio::sync::Notify::const_new();
thread_local! {
    static METADATA_HELD: Cell<bool> = const { Cell::new(false) };
}
#[cfg(test)]
pub(super) static TEST_SERIAL: Mutex<()> = Mutex::new(());

pub(super) struct Gate {
    _local: MutexGuard<'static, ()>,
    _file: File,
}
impl Drop for Gate {
    fn drop(&mut self) {
        METADATA_HELD.set(false);
    }
}

struct RootOwner {
    id: u64,
    root: PathBuf,
    thread: ThreadId,
}
struct WaitingRoot {
    id: u64,
    root: PathBuf,
}
struct RootAdmission {
    owners: Vec<RootOwner>,
    waiters: Vec<WaitingRoot>,
    next_id: u64,
}
impl RootAdmission {
    fn id(&mut self) -> Result<u64, Error> {
        self.next_id = self
            .next_id
            .checked_add(1)
            .ok_or("root admission ids exhausted")?;
        Ok(self.next_id)
    }
    fn reserve(&mut self, root: &Path, id: Option<u64>) -> Result<RootPermit, Error> {
        if self.owners.len() >= ROOT_LIMIT {
            return Err("active trust mutation quota exceeded".into());
        }
        let id = match id {
            Some(id) => id,
            None => self.id()?,
        };
        self.owners.push(RootOwner {
            id,
            root: root.into(),
            thread: std::thread::current().id(),
        });
        Ok(RootPermit {
            id,
            root: root.into(),
            wait_remaining: None,
            _thread: PhantomData,
        })
    }
    fn ordinary(
        &mut self,
        root: &Path,
        waiter: &mut Option<RootWaiter>,
    ) -> Result<Option<RootPermit>, Error> {
        // A metadata->root or root->root caller must never become a waiter.
        if METADATA_HELD.get()
            || self
                .owners
                .iter()
                .any(|owner| owner.thread == std::thread::current().id())
        {
            return Err(Error::busy());
        }
        let owned = self.owners.iter().any(|owner| owner.root == root);
        let first = self.waiters.iter().find(|waiting| waiting.root == root);
        let our_turn = first.is_none_or(|first| {
            waiter
                .as_ref()
                .is_some_and(|waiting| waiting.id == first.id)
        });
        if !owned && our_turn {
            let permit = self.reserve(root, waiter.as_ref().map(|waiting| waiting.id))?;
            if let Some(waiter) = waiter.as_mut() {
                self.waiters.retain(|waiting| waiting.id != waiter.id);
                waiter.registered = false;
            }
            return Ok(Some(permit));
        }
        if waiter.is_none() {
            if self.waiters.len() >= WAITER_LIMIT {
                return Err("ordinary root admission quota exceeded".into());
            }
            let id = self.id()?;
            self.waiters.push(WaitingRoot {
                id,
                root: root.into(),
            });
            *waiter = Some(RootWaiter {
                id,
                registered: true,
            });
        }
        Ok(None)
    }
}
fn roots() -> MutexGuard<'static, RootAdmission> {
    ROOTS.lock().unwrap_or_else(|error| error.into_inner())
}
fn root_changed() {
    ROOT_CHANGED.notify_all();
    ROOT_NOTIFIED.notify_waiters();
}
struct RootWaiter {
    id: u64,
    registered: bool,
}
impl Drop for RootWaiter {
    fn drop(&mut self) {
        if self.registered {
            roots().waiters.retain(|waiting| waiting.id != self.id);
            root_changed();
        }
    }
}
/// Process-local ownership before any constructor filesystem side effects.
/// This lease stays on its acquiring thread; it never lives in a returned Node.
pub(crate) struct RootPermit {
    id: u64,
    root: PathBuf,
    // Only ordinary construction can wait for the OS trust fence. Freeze the
    // allowance left after local admission: constructor CPU/filesystem work
    // between admission and attach is not lock contention.
    wait_remaining: Option<Duration>,
    _thread: PhantomData<Rc<()>>,
}
impl RootPermit {
    pub(crate) fn try_acquire(root: &Path) -> Result<Self, Error> {
        let mut admission = roots();
        if admission.owners.iter().any(|owner| owner.root == root) {
            return Err(Error::busy());
        }
        admission.reserve(root, None)
    }
    pub(crate) fn acquire(root: &Path) -> Result<Self, Error> {
        let until = Instant::now() + OPEN_WAIT;
        // The mutex guard drops before a registered waiter's cleanup can lock it.
        let mut waiter = None;
        let mut admission = roots();
        loop {
            if Instant::now() >= until {
                return Err(Error::busy());
            }
            if let Some(mut permit) = admission.ordinary(root, &mut waiter)? {
                permit.wait_remaining = Some(until.saturating_duration_since(Instant::now()));
                drop(admission);
                if permit.wait_remaining.is_some_and(|left| left.is_zero()) {
                    return Err(Error::busy());
                }
                return Ok(permit);
            }
            let remaining = until.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(Error::busy());
            }
            admission = ROOT_CHANGED
                .wait_timeout(admission, remaining)
                .unwrap_or_else(|error| error.into_inner())
                .0;
        }
    }
    pub(crate) async fn acquire_async(root: &Path) -> Result<Self, Error> {
        let until = tokio::time::Instant::now() + OPEN_WAIT;
        let mut waiter = None;
        loop {
            if tokio::time::Instant::now() >= until {
                return Err(Error::busy());
            }
            let notified = ROOT_NOTIFIED.notified();
            tokio::pin!(notified);
            // Register before checking the registry: release cannot be lost.
            notified.as_mut().enable();
            {
                let mut admission = roots();
                if tokio::time::Instant::now() >= until {
                    return Err(Error::busy());
                }
                let ready = admission.ordinary(root, &mut waiter)?;
                if let Some(mut permit) = ready {
                    permit.wait_remaining =
                        Some(until.saturating_duration_since(tokio::time::Instant::now()));
                    drop(admission);
                    if permit.wait_remaining.is_some_and(|left| left.is_zero()) {
                        return Err(Error::busy());
                    }
                    return Ok(permit);
                }
            }
            if tokio::time::timeout_at(until, notified).await.is_err() {
                return Err(Error::busy());
            }
        }
    }
    pub(crate) fn attach(self) -> Result<RootGate, Error> {
        let mut owned = RootGate {
            permit: self,
            file: None,
        };
        // A caller that acquired metadata after ordinary admission may not
        // wait in the opposite lock order. Control/transfer try permits never
        // carry a wait budget, including devices apply under its State lease.
        if owned
            .permit
            .wait_remaining
            .is_some_and(|left| METADATA_HELD.get() || left.is_zero())
        {
            return Err(Error::busy());
        }
        let directory = owned.permit.root.join(".khor/trust-mutation");
        private_dir(&directory)?;
        for (count, entry) in fs::read_dir(&directory).map_err(error)?.enumerate() {
            let entry = entry.map_err(error)?;
            if count >= 2 || !matches!(entry.file_name().to_str(), Some("gate" | "anchor.next")) {
                return Err("unknown trust mutation entry; reconciliation required".into());
            }
            check_file(&entry.path())?;
        }
        let file = open(&directory.join("gate"), true)?;
        if file.metadata().map_err(error)?.len() != 0 {
            return Err("unexpected trust mutation gate bytes".into());
        }
        // Start spending the remaining allowance only when this exact OS
        // lease is contended. Filesystem validation above is not lock waiting.
        let mut waiting_until: Option<Instant> = None;
        loop {
            if waiting_until.is_some_and(|until| Instant::now() >= until) {
                return Err(Error::busy());
            }
            match file.try_lock() {
                Ok(()) => {
                    if let Some(until) = waiting_until {
                        owned.permit.wait_remaining =
                            Some(until.saturating_duration_since(Instant::now()));
                    }
                    break;
                }
                Err(fs::TryLockError::WouldBlock) => {
                    let allowance = owned.permit.wait_remaining.ok_or_else(Error::busy)?;
                    let until = *waiting_until.get_or_insert_with(|| Instant::now() + allowance);
                    let remaining = until.saturating_duration_since(Instant::now());
                    if remaining.is_zero() {
                        return Err(Error::busy());
                    }
                    // Retry only this exact OS lease, never Node construction,
                    // metadata, trust recovery or publication side effects.
                    std::thread::sleep(remaining.min(Duration::from_millis(5)));
                }
                Err(fs::TryLockError::Error(why)) => return Err(error(why).into()),
            }
        }
        owned.file = Some(file);
        Ok(owned)
    }
}
impl Drop for RootPermit {
    fn drop(&mut self) {
        roots().owners.retain(|owner| owner.id != self.id);
        root_changed();
    }
}
/// The OS lease attaches once to the same local reservation used by bootstrap.
pub(crate) struct RootGate {
    permit: RootPermit,
    file: Option<File>,
}
impl Drop for RootGate {
    fn drop(&mut self) {
        // Drop the OS lease before the permit retires its owner and wakes waiters.
        self.file.take();
    }
}
pub(crate) fn root_gate(root: &Path) -> Result<RootGate, Error> {
    RootPermit::try_acquire(root)?.attach()
}

/// The public anchor is old or new after process interruption, never a
/// truncated identity. The private next file is recovered only for this exact
/// requested value; unknown bytes are preserved for explicit reconciliation.
pub(crate) fn publish_anchor(root: &Path, bytes: &[u8]) -> Result<(), String> {
    if bytes.len() != 65 || bytes.last() != Some(&b'\n') {
        return Err("invalid public anchor publication".into());
    }
    let directory = root.join(".khor/trust-mutation");
    private_dir(&directory)?;
    let next = directory.join("anchor.next");
    let prefix = read_optional(&next)?;
    if let Some(prefix) = &prefix {
        if prefix.len() > bytes.len() || prefix.as_slice() != &bytes[..prefix.len()] {
            return Err("unknown anchor publication; reconciliation required".into());
        }
    }
    let mut file = if prefix.is_some() {
        open(&next, false)?
    } else {
        options().create_new(true).open(&next).map_err(error)?
    };
    file.write_all(bytes).map_err(error)?;
    file.sync_all().map_err(error)?;
    drop(file);
    check_file(&next)?;
    let target = root.join(".khor/root.pub");
    match fs::symlink_metadata(&target) {
        Ok(metadata) if !metadata.is_file() || metadata.file_type().is_symlink() => {
            return Err("unsafe public anchor target".into());
        }
        Ok(_) => (),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
        Err(e) => return Err(error(e)),
    }
    fs::rename(next, target).map_err(error)
}

pub(super) fn has_publication(root: &Path) -> Result<bool, String> {
    match fs::symlink_metadata(root.join("registry.next")) {
        Ok(_) => {
            check_file(&root.join("registry.next"))?;
            Ok(true)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(error(e)),
    }
}

/// Atomic registry reads do not acquire/mutate a transfer owner. Before any
/// trust bootstrap, a critical intent must be recovered under both leases.
pub(super) fn peek(root: &Path) -> Result<Option<Vec<u8>>, String> {
    match fs::symlink_metadata(root) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(error(e)),
        Ok(metadata) => {
            if !metadata.is_dir() || metadata.file_type().is_symlink() {
                return Err("unsafe transfer metadata directory".into());
            }
            private(&metadata)?;
        }
    }
    for (count, entry) in fs::read_dir(root).map_err(error)?.enumerate() {
        let entry = entry.map_err(error)?;
        if count >= 3
            || !matches!(
                entry.file_name().to_str(),
                Some("gate" | "registry" | "registry.next")
            )
        {
            return Err("unrecognized transfer metadata file; reconciliation required".into());
        }
        check_file(&entry.path())?;
    }
    read_optional(&root.join("registry"))?
        .map(|bytes| decode(&bytes).map(|(_, body)| body.to_vec()))
        .transpose()
}

pub(super) fn gate(root: &Path) -> Result<Gate, Error> {
    let local = match LOCAL.try_lock() {
        Ok(guard) => guard,
        Err(TryLockError::Poisoned(error)) => error.into_inner(),
        Err(TryLockError::WouldBlock) => return Err(Error::busy()),
    };
    private_dir(root)?;
    let file = open(root.join("gate").as_path(), true)?;
    if file.metadata().map_err(error)?.len() != 0 {
        return Err("unexpected transfer metadata gate bytes".to_owned().into());
    }
    match file.try_lock() {
        Ok(()) => {}
        Err(fs::TryLockError::WouldBlock) => return Err(Error::busy()),
        Err(fs::TryLockError::Error(why)) => return Err(error(why).into()),
    }
    METADATA_HELD.set(true);
    Ok(Gate {
        _local: local,
        _file: file,
    })
}

pub(super) struct Journal {
    root: PathBuf,
    generation: u64,
}

impl Journal {
    pub(super) fn load(
        root: &Path,
        validate: impl Fn(&[u8]) -> Result<(), String>,
    ) -> Result<(Self, Option<Vec<u8>>), String> {
        // Only three recognized controls; unknown files are never GC targets.
        for (count, entry) in fs::read_dir(root).map_err(error)?.enumerate() {
            if count >= 3 {
                return Err("transfer metadata has unexpected entries".into());
            }
            let entry = entry.map_err(error)?;
            if !matches!(
                entry.file_name().to_str(),
                Some("gate" | "registry" | "registry.next")
            ) {
                return Err("unrecognized transfer metadata file; reconciliation required".into());
            }
            check_file(&entry.path())?;
        }
        let current = read_optional(&root.join("registry"))?;
        let (mut generation, mut body) = match current {
            None => (0, None),
            Some(bytes) => {
                let (generation, body) = decode(&bytes)?;
                validate(body)?;
                (generation, Some(body.to_vec()))
            }
        };
        let next = root.join("registry.next");
        if let Some(bytes) = read_optional(&next)? {
            let expected = generation
                .checked_add(1)
                .ok_or("metadata generation overflow")?;
            let mut prefix = Vec::from(MAGIC.as_slice());
            prefix.extend_from_slice(&expected.to_be_bytes());
            if bytes[..bytes.len().min(prefix.len())] != prefix[..bytes.len().min(prefix.len())] {
                return Err(
                    "unrecognized metadata publication; preserve for reconciliation".into(),
                );
            }
            let partial = if bytes.len() < 24 {
                if bytes.len() > 16 {
                    let mut partial_length = [0u8; 8];
                    partial_length[..bytes.len() - 16].copy_from_slice(&bytes[16..]);
                    if u64::from_be_bytes(partial_length) > MAX_IMAGE as u64 {
                        return Err(
                            "invalid partial metadata length; preserve for reconciliation".into(),
                        );
                    }
                }
                true
            } else {
                let length = u64::from_be_bytes(bytes[16..24].try_into().unwrap());
                if length > MAX_IMAGE as u64 {
                    return Err("metadata publication exceeds budget".into());
                }
                bytes.len() < HEADER + length as usize
            };
            if partial {
                // This exact, private, unpublished next-generation control has
                // never returned success. Keep the previously committed image.
                // Bad complete checksums/unknown generations are NOT removed.
                check_file(&next)?;
                fs::remove_file(&next).map_err(error)?;
            } else {
                let (next_generation, next_body) = decode(&bytes)?;
                if next_generation != expected {
                    return Err("unexpected metadata generation".into());
                }
                validate(next_body)?;
                open(&next, false)?.sync_all().map_err(error)?;
                fs::rename(&next, root.join("registry")).map_err(error)?;
                generation = next_generation;
                body = Some(next_body.to_vec());
            }
        }
        Ok((
            Self {
                root: root.to_path_buf(),
                generation,
            },
            body,
        ))
    }

    pub(super) fn save(&mut self, body: &[u8]) -> Result<(), String> {
        if body.len() > MAX_IMAGE {
            return Err("document transfer metadata exceeds 32 MiB".into());
        }
        let generation = self
            .generation
            .checked_add(1)
            .ok_or("metadata generation overflow")?;
        let next = self.root.join("registry.next");
        let mut options = options();
        let mut file = options.create_new(true).open(&next).map_err(error)?;
        let mut header = Vec::from(MAGIC.as_slice());
        header.extend_from_slice(&generation.to_be_bytes());
        header.extend_from_slice(&(body.len() as u64).to_be_bytes());
        let mut checksum = blake3::Hasher::new();
        checksum.update(&header);
        checksum.update(body);
        header.extend_from_slice(checksum.finalize().as_bytes());
        file.write_all(&header).map_err(error)?;
        file.write_all(body).map_err(error)?;
        file.sync_all().map_err(error)?;
        drop(file);
        check_file(&next)?;
        #[cfg(all(test, unix))]
        crate::link::documents::query_concurrency_tests::publication_ready(
            &self.root,
            METADATA_HELD.get(),
            OPEN_WAIT,
            (&header, body),
        );
        fs::rename(&next, self.root.join("registry")).map_err(error)?;
        self.generation = generation;
        Ok(())
    }
}

fn decode(bytes: &[u8]) -> Result<(u64, &[u8]), String> {
    if bytes.len() < HEADER || &bytes[..8] != MAGIC {
        return Err("invalid transfer metadata header".into());
    }
    let generation = u64::from_be_bytes(bytes[8..16].try_into().unwrap());
    let length = u64::from_be_bytes(bytes[16..24].try_into().unwrap());
    if generation == 0 || length > MAX_IMAGE as u64 || bytes.len() != HEADER + length as usize {
        return Err("invalid transfer metadata length or generation".into());
    }
    let body = &bytes[HEADER..];
    let mut checksum = blake3::Hasher::new();
    checksum.update(&bytes[..24]);
    checksum.update(body);
    if checksum.finalize().as_bytes() != &bytes[24..56] {
        return Err("transfer metadata checksum mismatch; preserve for reconciliation".into());
    }
    Ok((generation, body))
}

fn read_optional(path: &Path) -> Result<Option<Vec<u8>>, String> {
    match fs::symlink_metadata(path) {
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(why) => return Err(error(why)),
        Ok(_) => {}
    }
    let file = open(path, false)?;
    if file.metadata().map_err(error)?.len() > (MAX_IMAGE + HEADER) as u64 {
        return Err("transfer metadata file exceeds budget".into());
    }
    let mut bytes = Vec::new();
    file.take((MAX_IMAGE + HEADER + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(error)?;
    if bytes.len() > MAX_IMAGE + HEADER {
        return Err("transfer metadata file grew past budget".into());
    }
    Ok(Some(bytes))
}

fn options() -> OpenOptions {
    let mut options = OpenOptions::new();
    options.read(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(0x00200000); // FILE_FLAG_OPEN_REPARSE_POINT
    }
    options
}

fn open(path: &Path, create: bool) -> Result<File, String> {
    let file = options().create(create).open(path).map_err(error)?;
    let metadata = file.metadata().map_err(error)?;
    private(&metadata)?;
    if !metadata.is_file() {
        return Err("transfer metadata control is not a file".into());
    }
    check_file(path)?;
    Ok(file)
}

fn check_file(path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path).map_err(error)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err("unsafe transfer metadata file".into());
    }
    private(&metadata)
}

fn private_dir(path: &Path) -> Result<(), String> {
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    match builder.create(path) {
        Ok(()) => {}
        Err(why) if why.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(why) => return Err(error(why)),
    }
    let metadata = fs::symlink_metadata(path).map_err(error)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err("unsafe transfer metadata directory".into());
    }
    private(&metadata)
}

fn private(metadata: &fs::Metadata) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.uid() != unsafe { libc::geteuid() }
            || metadata.mode() & 0o077 != 0
            || (metadata.is_file() && metadata.nlink() != 1)
        {
            return Err("transfer metadata must be owner-only without hard links".into());
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err("transfer metadata reparse point".into());
        }
    }
    Ok(())
}
fn error(why: std::io::Error) -> String {
    format!("document transfer metadata: {why}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Outside(PathBuf);
    impl Drop for Outside {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            Self(std::env::temp_dir().join(format!(
                "khor-doc-journal-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            )))
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn image(generation: u64, body: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::from(MAGIC.as_slice());
        bytes.extend_from_slice(&generation.to_be_bytes());
        bytes.extend_from_slice(&(body.len() as u64).to_be_bytes());
        let mut hash = blake3::Hasher::new();
        hash.update(&bytes);
        hash.update(body);
        bytes.extend_from_slice(hash.finalize().as_bytes());
        bytes.extend_from_slice(body);
        bytes
    }
    fn next(root: &Path, bytes: &[u8]) {
        let mut f = options()
            .create_new(true)
            .open(root.join("registry.next"))
            .unwrap();
        f.write_all(bytes).unwrap();
        f.sync_all().unwrap();
    }
    #[test]
    fn a_complete_unpublished_image_recovers_and_every_partial_prefix_keeps_the_committed_image() {
        let _serial = TEST_SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        for length in 0..HEADER + 7 {
            let fixture = Fixture::new();
            let _gate = gate(&fixture.0).unwrap();
            let (mut journal, _) = Journal::load(&fixture.0, |_| Ok(())).unwrap();
            journal.save(b"previous").unwrap();
            let candidate = image(2, b"current");
            next(&fixture.0, &candidate[..length]);
            let (_, body) = Journal::load(&fixture.0, |_| Ok(())).unwrap();
            assert_eq!(body.as_deref(), Some(b"previous".as_slice()));
            assert!(!fixture.0.join("registry.next").exists());
        }
        let fixture = Fixture::new();
        let _gate = gate(&fixture.0).unwrap();
        let (mut journal, _) = Journal::load(&fixture.0, |_| Ok(())).unwrap();
        journal.save(b"previous").unwrap();
        next(&fixture.0, &image(2, b"current"));
        let (_, body) = Journal::load(&fixture.0, |_| Ok(())).unwrap();
        assert_eq!(body.as_deref(), Some(b"current".as_slice()));
    }
    #[test]
    fn complete_corruption_unknown_files_and_semantically_bad_publications_are_preserved() {
        let _serial = TEST_SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        for case in 0..3 {
            let fixture = Fixture::new();
            let _gate = gate(&fixture.0).unwrap();
            let (mut journal, _) = Journal::load(&fixture.0, |_| Ok(())).unwrap();
            journal.save(b"previous").unwrap();
            let previous = fs::read(fixture.0.join("registry")).unwrap();
            let mut bytes = image(2, b"current");
            match case {
                0 => bytes[24] ^= 1,
                1 => {
                    fs::write(fixture.0.join("unknown"), b"keep").unwrap();
                }
                _ => {}
            }
            next(&fixture.0, &bytes);
            assert!(
                Journal::load(&fixture.0, |body| if case == 2 && body == b"current" {
                    Err("semantic rejection".into())
                } else {
                    Ok(())
                })
                .is_err()
            );
            assert_eq!(fs::read(fixture.0.join("registry.next")).unwrap(), bytes);
            assert_eq!(fs::read(fixture.0.join("registry")).unwrap(), previous);
        }
    }
    #[cfg(unix)]
    #[test]
    fn metadata_links_never_touch_their_targets() {
        let _serial = TEST_SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        use std::os::unix::fs::symlink;
        for hard in [false, true] {
            let fixture = Fixture::new();
            let _gate = gate(&fixture.0).unwrap();
            let target = fixture.0.with_extension("outside");
            let mut file = options().create_new(true).open(&target).unwrap();
            let _outside = Outside(target.clone());
            file.write_all(b"preserve outside target").unwrap();
            drop(file);
            let linked = fixture.0.join("registry.next");
            if hard {
                fs::hard_link(&target, &linked).unwrap();
            } else {
                symlink(&target, &linked).unwrap();
            }
            assert!(Journal::load(&fixture.0, |_| Ok(())).is_err());
            assert_eq!(fs::read(&target).unwrap(), b"preserve outside target");
            assert!(fs::symlink_metadata(&linked).is_ok());
            fs::remove_file(&linked).unwrap();
            fs::remove_file(&target).unwrap();
        }
    }
    #[test]
    fn ordinary_open_waits_for_a_same_root_control_lease_then_reuses_its_anchor() {
        use std::sync::mpsc;
        use std::time::Duration;

        let _serial = TEST_SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let fixture = Fixture::new();
        let first = crate::Node::open_hosting(fixture.0.clone(), "ordinary-open", false).unwrap();
        assert!(first.root_born());
        let anchor = fs::read(fixture.0.join(".khor/root.pub")).unwrap();
        assert!(!anchor.is_empty());
        drop(first);

        let control = root_gate(&fixture.0).unwrap();
        let root = fixture.0.clone();
        let (started_tx, started_rx) = mpsc::channel();
        let (proceed_tx, proceed_rx) = mpsc::channel();
        let (attempting_tx, attempting_rx) = mpsc::channel();
        let (returned_tx, returned_rx) = mpsc::channel();
        let handshake = Duration::from_secs(2);
        let worker = std::thread::spawn(move || {
            let _ = started_tx.send(());
            if proceed_rx.recv_timeout(handshake).is_err() {
                return Err("ordinary-open worker did not receive its start handshake".to_owned());
            }
            let _ = attempting_tx.send(());
            let opened =
                crate::Node::open_hosting(root.clone(), "ordinary-open", false).and_then(|node| {
                    let anchor =
                        fs::read(root.join(".khor/root.pub")).map_err(|error| error.to_string())?;
                    Ok((node.root_born(), anchor))
                });
            let _ = returned_tx.send(opened.clone());
            opened
        });

        let started = started_rx.recv_timeout(handshake).is_ok();
        let proceeded = started && proceed_tx.send(()).is_ok();
        drop(proceed_tx);
        let attempting = proceeded && attempting_rx.recv_timeout(handshake).is_ok();
        let early = if attempting {
            Some(returned_rx.recv_timeout(Duration::from_millis(100)))
        } else {
            None
        };
        // Always release the actual owner and join before any assertion can unwind.
        drop(control);
        let joined = worker.join();
        match &early {
            Some(Ok(Err(error))) => eprintln!("Q039_EARLY_OPEN_ERROR: {error}"),
            Some(Ok(Ok(_))) => eprintln!("Q039_EARLY_OPEN_RETURNED_OK"),
            Some(Err(mpsc::RecvTimeoutError::Timeout)) => {
                eprintln!("Q039_HELD_WINDOW_TIMEOUT_AFTER_ATTEMPT_HANDSHAKE");
            }
            Some(Err(mpsc::RecvTimeoutError::Disconnected)) => {
                eprintln!("Q039_OPEN_WORKER_DISCONNECTED");
            }
            None => eprintln!("Q039_OPEN_ATTEMPT_HANDSHAKE_MISSING"),
        }
        assert!(
            started && proceeded && attempting,
            "ordinary-open handshake did not complete"
        );
        assert!(
            matches!(early, Some(Err(mpsc::RecvTimeoutError::Timeout))),
            "ordinary Node open returned while the same-root control lease was held: {early:?}"
        );
        let (born, reopened_anchor) = joined
            .expect("ordinary-open worker panicked")
            .expect("ordinary Node open failed after the control lease was released");
        assert!(!born, "ordinary reopening must not create another root");
        assert_eq!(
            reopened_anchor, anchor,
            "ordinary reopening changed the trust anchor"
        );
    }

    fn private_trust_root(root: &Path) {
        private_dir(root).unwrap();
        private_dir(&root.join(".khor")).unwrap();
    }

    struct HeldRoot {
        release: Option<std::sync::mpsc::Sender<()>>,
        worker: Option<std::thread::JoinHandle<()>>,
    }
    impl HeldRoot {
        fn new(root: PathBuf) -> Self {
            private_trust_root(&root);
            let (held_tx, held_rx) = std::sync::mpsc::channel();
            let (release_tx, release_rx) = std::sync::mpsc::channel();
            let worker = std::thread::spawn(move || {
                let _gate = root_gate(&root).unwrap();
                let _ = held_tx.send(());
                let _ = release_rx.recv_timeout(Duration::from_secs(10));
            });
            let held = Self {
                release: Some(release_tx),
                worker: Some(worker),
            };
            assert!(held_rx.recv_timeout(Duration::from_secs(2)).is_ok());
            held
        }
    }
    impl Drop for HeldRoot {
        fn drop(&mut self) {
            if let Some(release) = self.release.take() {
                let _ = release.send(());
            }
            if let Some(worker) = self.worker.take() {
                let joined = worker.join();
                assert!(
                    joined.is_ok() || std::thread::panicking(),
                    "root owner panicked"
                );
            }
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn cancelled_async_open_retires_its_waiter_and_a_late_notification_cannot_construct() {
        use std::task::Poll;
        let _serial = TEST_SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let fixture = Fixture::new();
        let held = HeldRoot::new(fixture.0.clone());
        let mut cancelled = Box::pin(crate::Node::open_hosting_async(
            fixture.0.clone(),
            "cancelled-open".into(),
            false,
        ));
        fn require_send<T: Send>(_: &T) {}
        require_send(&cancelled);
        assert!(matches!(futures::poll!(cancelled.as_mut()), Poll::Pending));
        drop(cancelled);
        assert!(!fixture.0.join(".khor/identity.key").exists());
        // The canceled request must not consume one of the fixed64 waiter slots.
        let mut pending = Vec::new();
        for _ in 0..64 {
            let mut opening = Box::pin(crate::Node::open_hosting_async(
                fixture.0.clone(),
                "queued-open".into(),
                false,
            ));
            assert!(matches!(futures::poll!(opening.as_mut()), Poll::Pending));
            pending.push(opening);
        }
        let overflow =
            crate::Node::open_hosting_async(fixture.0.clone(), "over-quota".into(), false).await;
        assert!(
            overflow
                .err()
                .expect("the65th waiter must be refused")
                .contains("quota")
        );
        drop(pending);
        assert!(!fixture.0.join(".khor/identity.key").exists());
        drop(held);
        let reopened =
            crate::Node::open_hosting_async(fixture.0.clone(), "admitted-open".into(), false)
                .await
                .unwrap();
        assert!(reopened.root_born());
        assert!(
            reopened
                .devices()
                .unwrap()
                .iter()
                .any(|device| device.id == reopened.device_str())
        );
        drop(reopened);

        let expired = Fixture::new();
        let held = HeldRoot::new(expired.0.clone());
        let mut opening = Box::pin(crate::Node::open_hosting_async(
            expired.0.clone(),
            "expired-open".into(),
            false,
        ));
        assert!(matches!(futures::poll!(opening.as_mut()), Poll::Pending));
        tokio::time::sleep(Duration::from_millis(1100)).await;
        // Notify after the fixed deadline, before polling the waiting future again.
        drop(held);
        assert!(
            opening.await.is_err(),
            "a late free root extended the admission deadline"
        );
        assert!(!expired.0.join(".khor/identity.key").exists());
    }

    #[test]
    fn ordinary_admission_does_not_charge_nonwaiting_constructor_work() {
        let _serial = TEST_SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let fixture = Fixture::new();
        private_trust_root(&fixture.0);
        let directory = fixture.0.join(".khor/trust-mutation");
        private_dir(&directory).unwrap();
        let path = directory.join("gate");
        // Physical lock proof, not a warmed constructor or a Node retry.
        let free = open(&path, true).unwrap();
        free.try_lock().expect("new private OS gate is unowned");
        drop(free);
        let admitted = RootPermit::acquire(&fixture.0).unwrap();
        let started = Instant::now();
        // Deliberately model slow CPU/filesystem initialization between the
        // actual admission and attach. Nobody contends or waits for any lock.
        std::thread::sleep(OPEN_WAIT + Duration::from_millis(50));
        assert!(started.elapsed() >= OPEN_WAIT);
        assert!(!METADATA_HELD.get());
        let free = open(&path, false).unwrap();
        free.try_lock()
            .expect("the same physical gate remains unowned");
        drop(free);
        let attached = admitted.attach();
        eprintln!(
            "[N-NONWAIT] physical_gate_free=true metadata_held=false delay_ms={} attached={}",
            started.elapsed().as_millis(),
            attached.is_ok()
        );
        assert!(
            attached.is_ok(),
            "nonwaiting constructor work consumed the lock waiting allowance: {:?}",
            attached.err()
        );
    }

    #[test]
    fn ordinary_attach_spends_only_remaining_wait_and_control_never_waits() {
        let _serial = TEST_SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let fixture = Fixture::new();
        private_trust_root(&fixture.0);
        drop(root_gate(&fixture.0).unwrap());
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(fixture.0.join(".khor/trust-mutation/gate"))
            .unwrap();
        file.try_lock().unwrap();

        let mut expired = RootPermit::acquire(&fixture.0).unwrap();
        assert!(
            expired.wait_remaining.is_some(),
            "ordinary admission must retain its remaining allowance"
        );
        // An actually exhausted local-admission allowance may not be restarted
        // by attach. Constructor elapsed time alone does not exhaust it.
        expired.wait_remaining = Some(Duration::ZERO);
        let started = Instant::now();
        let error = expired
            .attach()
            .err()
            .expect("held OS fence with exhausted budget");
        assert_eq!(error.kind, crate::proto::DocTransferFailure::Busy);
        assert!(
            started.elapsed() < OPEN_WAIT,
            "attach reset the admission budget"
        );

        // OS contention receives only what local admission left, not a fresh
        // OPEN_WAIT. A held physical file makes the timeout decision exact.
        let mut partly_spent = RootPermit::acquire(&fixture.0).unwrap();
        let remaining = Duration::from_millis(20);
        partly_spent.wait_remaining = Some(remaining);
        let started = Instant::now();
        assert_eq!(
            partly_spent.attach().err().unwrap().kind,
            crate::proto::DocTransferFailure::Busy
        );
        assert!(started.elapsed() >= remaining);
        assert!(started.elapsed() < OPEN_WAIT, "OS wait reset the allowance");

        let control = RootPermit::try_acquire(&fixture.0).unwrap();
        assert!(control.wait_remaining.is_none());
        let started = Instant::now();
        assert_eq!(
            control.attach().err().unwrap().kind,
            crate::proto::DocTransferFailure::Busy
        );
        assert!(
            started.elapsed() < OPEN_WAIT,
            "control waited for the OS gate"
        );

        let ordinary = RootPermit::acquire(&fixture.0).unwrap();
        let metadata = gate(&fixture.0.join("metadata")).unwrap();
        let started = Instant::now();
        assert_eq!(
            ordinary.attach().err().unwrap().kind,
            crate::proto::DocTransferFailure::Busy
        );
        assert!(
            started.elapsed() < OPEN_WAIT,
            "metadata -> root waited after admission"
        );
        drop(metadata);
        drop(file);

        // Exhaustion is checked even for a free gate. This is spent lock wait,
        // not the old absolute idle-deadline premise corrected by the gap test.
        let mut expired_free = RootPermit::acquire(&fixture.0).unwrap();
        expired_free.wait_remaining = Some(Duration::ZERO);
        assert_eq!(
            expired_free
                .attach()
                .err()
                .expect("expired even when free")
                .kind,
            crate::proto::DocTransferFailure::Busy
        );
        drop(root_gate(&fixture.0).unwrap());
    }

    #[test]
    fn ordinary_reentry_and_deadline_preserve_control_and_external_try_leases() {
        let _serial = TEST_SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let fixture = Fixture::new();
        private_trust_root(&fixture.0);
        let nested = fixture.0.join("nested");
        {
            let _owner = root_gate(&fixture.0).unwrap();
            assert!(crate::Node::open_hosting(fixture.0.clone(), "reentry", false).is_err());
            assert!(crate::Node::open_hosting(nested.clone(), "other-root", false).is_err());
            assert!(!nested.exists());
        }
        {
            let _metadata = gate(&fixture.0.join("metadata")).unwrap();
            assert!(crate::Node::open_hosting(nested.clone(), "metadata-owner", false).is_err());
            assert!(!nested.exists());
        }
        let without_runtime = fixture.0.join("without-runtime");
        let missing = futures::executor::block_on(crate::Node::open_async(without_runtime.clone()));
        assert!(
            missing
                .err()
                .expect("async opening requires an existing runtime")
                .contains("Tokio runtime")
        );
        assert!(!without_runtime.exists());

        let held = HeldRoot::new(fixture.0.clone());
        let started = Instant::now();
        assert!(crate::Node::try_open(fixture.0.clone()).is_err());
        assert!(
            started.elapsed() < OPEN_WAIT,
            "control construction entered the ordinary queue"
        );
        let root = fixture.0.clone();
        let (done_tx, done_rx) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            let started = Instant::now();
            let opened = crate::Node::open_hosting(root, "deadline-open", false).map(drop);
            let _ = done_tx.send((opened.clone(), started.elapsed()));
            opened
        });
        let timed = done_rx.recv_timeout(Duration::from_secs(3));
        drop(held);
        let joined = worker.join();
        let (opened, elapsed) =
            timed.expect("ordinary admission did not reach its finite deadline");
        assert!(opened.is_err());
        assert!(
            elapsed >= OPEN_WAIT,
            "ordinary construction failed instead of waiting"
        );
        assert!(joined.expect("ordinary worker panicked").is_err());
        assert!(!fixture.0.join(".khor/identity.key").exists());

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(fixture.0.join(".khor/trust-mutation/gate"))
            .unwrap();
        file.try_lock().unwrap();
        let permit = RootPermit::acquire(&fixture.0).unwrap();
        let error = permit
            .attach()
            .err()
            .expect("an unknown OS owner remains fail-closed");
        assert_eq!(error.kind, crate::proto::DocTransferFailure::Busy);
        drop(file);
        drop(root_gate(&fixture.0).unwrap());
    }

    #[test]
    fn node_opening_on_a_multithread_localset_keeps_sync_async_and_try_as_results() {
        let _serial = TEST_SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let fixture = Fixture::new();
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(tokio::task::LocalSet::new().run_until(async {
            let first = crate::Node::open_hosting(fixture.0.clone(), "local-open", false).unwrap();
            assert!(first.root_born());
            let anchor = fs::read(fixture.0.join(".khor/root.pub")).unwrap();
            drop(first);
            let reopened =
                crate::Node::open_hosting_async(fixture.0.clone(), "local-open".into(), false)
                    .await
                    .unwrap();
            assert!(!reopened.root_born());
            assert_eq!(fs::read(fixture.0.join(".khor/root.pub")).unwrap(), anchor);
            drop(reopened);
            let _control = root_gate(&fixture.0).unwrap();
            assert!(crate::Node::try_open(fixture.0.clone()).is_err());
        }));
    }
}
