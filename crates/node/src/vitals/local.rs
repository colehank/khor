//! This machine's readings, **taken in the background and read from
//! memory** (docs/SHELL.md 第十六轮).
//!
//! # Why a reading is no longer taken to answer a request
//!
//! It was, until 2026-09-25: `Node::vitals_of` for this machine called
//! [`super::sample`] on the spot, and a sample is one `statfs` per
//! volume. A USB disk that has gone to sleep answers its `statfs` in
//! 3.65 s (measured on this machine, a My Book), so `khor devices` took
//! 10–36 s and the app, which asks for the device table every two
//! seconds, queued its asks behind one process-wide lock. The machine
//! page and the 额度 page «特别慢才加载出来内容» — and nothing in the
//! request was slow except the one disk nobody was looking at.
//!
//! So a request only ever reads what the sampler last wrote down, with
//! its age. **Only the sampler thread waits for a disk**, and even it
//! waits for each volume no longer than [`VOLUME_LIMIT`]: a volume that
//! does not answer in time keeps its last numbers and is named in
//! [`Reading::stale`], and its `statfs` is left running on a thread of
//! its own — never asked a second time while the first is still out, so
//! a disk that never wakes costs one thread, not one per round.
//!
//! # One sampler per home, started on first ask
//!
//! The app opens a fresh `Node` for every command, so the sampler cannot
//! live on a `Node`; it lives here, keyed by khor's root, and the first
//! caller for a root starts it. The resident serve and the app's backend
//! start theirs at startup ([`start`]) so the first screen does not wait
//! for the first round. A sampler whose home is gone (a test's scratch
//! directory) stops at its next beat.
//!
//! # The injection point
//!
//! What the disks are is a [`Probe`]. The real one asks sysinfo; a test
//! installs its own ([`install`]) with volumes that do not exist, one of
//! which takes ten seconds to answer — **never the real disks**, because
//! a test that waited for this machine's own sleeping disk would be
//! measuring the machine, not khor.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

use khor_core::{Vitals, WatchedDir};

/// How often the background sampler takes a reading. The app asks every
/// two seconds and the card's dials move on the vitals beat, so five
/// seconds is the freshest reading anybody can see move.
pub const EVERY: Duration = Duration::from_secs(5);

/// How long one volume's `statfs` may take before the round goes on
/// without it. A healthy volume answers in ~14 ms (measured 2026-08-17,
/// `vitals.rs`); a sleeping USB disk in 3.65 s. The limit sits well
/// clear of the first and well short of the second.
pub const VOLUME_LIMIT: Duration = Duration::from_millis(800);

/// How long the list of mounts may take. The list is one call for all
/// volumes, so it gets more room than one of them; a list that does not
/// come back in time leaves the previous list in force.
const LIST_LIMIT: Duration = Duration::from_secs(2);

/// Where the disk half of a reading comes from.
///
/// **Both calls may block**, and the caller treats them so: each goes on
/// a thread of its own with a limit ([`Reader::table`]).
pub trait Probe: Send + Sync + 'static {
    /// The mount points there are now, as the machine spells them.
    fn mounts(&self) -> Vec<String>;
    /// One volume's `(total, available)` bytes — the `statfs` a sleeping
    /// disk stalls. `None` when the volume has no size to report.
    fn space(&self, mount: &str) -> Option<(u64, u64)>;
}

/// The real disks.
///
/// **macOS lists without touching a disk.** sysinfo's list asks every
/// volume for its name, visibility and locality through CoreFoundation,
/// in a row — so one sleeping disk holds up the *list*, and with it every
/// other volume, before any size is asked (the first real round here
/// came back with no volumes at all after two seconds). So the list is
/// `getfsstat(MNT_NOWAIT)` — the kernel's own table, no I/O — filtered
/// the way sysinfo's visibility filter lands (local, and not hidden from
/// browsing, with the data volume the one hidden mount a person means),
/// and each volume's size is its own `statfs` ([`mac::space`] says why
/// not sysinfo's CoreFoundation keys).
///
/// Elsewhere the list is sysinfo's without sizes (on Linux a parse of
/// `/proc/mounts`), and each size is its own call on its own `Disk`,
/// each behind its own lock, so a volume whose last `statfs` is still
/// out holds up nobody but itself.
struct RealProbe {
    #[cfg_attr(target_os = "macos", allow(dead_code))]
    disks: Mutex<HashMap<String, Arc<Mutex<sysinfo::Disk>>>>,
}

#[cfg(target_os = "macos")]
mod mac {
    /// Every mount in the kernel's table worth a volume row, with no
    /// I/O to any of them (`MNT_NOWAIT`).
    pub(super) fn mounts() -> Vec<String> {
        // SAFETY: the first call sizes the buffer; the second fills at
        // most that many entries and says how many it wrote.
        let table = unsafe {
            let count = libc::getfsstat(std::ptr::null_mut(), 0, libc::MNT_NOWAIT);
            if count < 1 {
                return Vec::new();
            }
            let mut buf: Vec<libc::statfs> = Vec::with_capacity(count as usize);
            let size = count * std::mem::size_of::<libc::statfs>() as libc::c_int;
            let got = libc::getfsstat(buf.as_mut_ptr(), size, libc::MNT_NOWAIT);
            if got < 1 {
                return Vec::new();
            }
            buf.set_len((got as usize).min(count as usize));
            buf
        };
        table
            .iter()
            .filter_map(|s| {
                // SAFETY: `f_mntonname` is a NUL-terminated C string the
                // kernel wrote into a fixed array.
                let name = unsafe { std::ffi::CStr::from_ptr(s.f_mntonname.as_ptr()) };
                let mount = name.to_string_lossy().into_owned();
                let flags = s.f_flags as libc::c_int;
                let local = flags & libc::MNT_LOCAL != 0;
                let shown = flags & libc::MNT_DONTBROWSE == 0 || mount == "/System/Volumes/Data";
                (local && shown).then_some(mount)
            })
            .collect()
    }

    /// One volume's size: its own `statfs` — **the call a sleeping disk
    /// stalls**, and it stalls only this volume.
    ///
    /// ⚠️ Not CoreFoundation's volume keys, which sysinfo reads and which
    /// counted purgeable space as room: those asks do not stall alone.
    /// Measured 2026-09-25 with a My Book attached, four volumes each on
    /// its own thread: through CoreFoundation every one of the four — the
    /// internal data volume included — was still out after nine seconds;
    /// through `statfs` all four answered in under 2 ms together.
    /// A plain `statfs` is `df`'s number: available without the
    /// purgeable space, the pair `disk_under`'s APFS note already
    /// describes.
    pub(super) fn space(mount: &str) -> Option<(u64, u64)> {
        let path = std::ffi::CString::new(mount).ok()?;
        let mut s = std::mem::MaybeUninit::<libc::statfs>::uninit();
        // SAFETY: a NUL-terminated path and a buffer of the right type;
        // the buffer is read only when the call says it filled it.
        let s = unsafe {
            if libc::statfs(path.as_ptr(), s.as_mut_ptr()) != 0 {
                return None;
            }
            s.assume_init()
        };
        let block = s.f_bsize as u64;
        let total = s.f_blocks.saturating_mul(block);
        (total > 0).then(|| (total, s.f_bavail.saturating_mul(block)))
    }
}

impl Probe for RealProbe {
    #[cfg(target_os = "macos")]
    fn mounts(&self) -> Vec<String> {
        mac::mounts()
    }

    #[cfg(target_os = "macos")]
    fn space(&self, mount: &str) -> Option<(u64, u64)> {
        mac::space(mount)
    }

    #[cfg(not(target_os = "macos"))]
    fn mounts(&self) -> Vec<String> {
        let listed: Vec<sysinfo::Disk> =
            sysinfo::Disks::new_with_refreshed_list_specifics(sysinfo::DiskRefreshKind::nothing())
                .into();
        let mut held = lock(&self.disks);
        let mut next = HashMap::new();
        let mut out = Vec::new();
        for d in listed {
            let mount = d.mount_point().to_string_lossy().into_owned();
            if next.contains_key(&mount) {
                continue;
            }
            let slot = held
                .remove(&mount)
                .unwrap_or_else(|| Arc::new(Mutex::new(d)));
            next.insert(mount.clone(), slot);
            out.push(mount);
        }
        *held = next;
        out
    }

    #[cfg(not(target_os = "macos"))]
    fn space(&self, mount: &str) -> Option<(u64, u64)> {
        let disk = lock(&self.disks).get(mount).cloned()?;
        let mut d = lock(&disk);
        d.refresh_specifics(sysinfo::DiskRefreshKind::nothing().with_storage());
        let total = d.total_space();
        (total > 0).then(|| (total, d.available_space()))
    }
}

/// The process's one real probe.
pub fn real_probe() -> Arc<dyn Probe> {
    static REAL: OnceLock<Arc<RealProbe>> = OnceLock::new();
    REAL.get_or_init(|| {
        Arc::new(RealProbe {
            disks: Mutex::new(HashMap::new()),
        })
    })
    .clone()
}

/// One row of a disk table: a mount point, its size if it has ever
/// answered, and whether that size is from an earlier round.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mount {
    pub mount: String,
    pub space: Option<(u64, u64)>,
    /// Did not answer within [`VOLUME_LIMIT`] this round: `space` is the
    /// last answer it gave, or `None` if it never gave one.
    pub stale: bool,
}

/// Where one volume's size will land.
type Answer = mpsc::Receiver<Option<(u64, u64)>>;

/// Reads disk tables through one probe, remembering what each volume
/// last said and which `statfs` calls are still out.
pub struct Reader {
    probe: Arc<dyn Probe>,
    mounts: Vec<String>,
    listing: Option<mpsc::Receiver<Vec<String>>>,
    pending: HashMap<String, Answer>,
    last: HashMap<String, (u64, u64)>,
}

impl Reader {
    pub fn new(probe: Arc<dyn Probe>) -> Reader {
        Reader {
            probe,
            mounts: Vec::new(),
            listing: None,
            pending: HashMap::new(),
            last: HashMap::new(),
        }
    }

    /// One round: the mounts, then every volume's size **at once**, each
    /// given until `limit` from the start of the sizes. Blocks for at
    /// most `LIST_LIMIT + limit`, whatever the disks do.
    pub fn table(&mut self, limit: Duration) -> Vec<Mount> {
        let listing = self.listing.take().unwrap_or_else(|| {
            let probe = self.probe.clone();
            off(move || probe.mounts())
        });
        match listing.recv_timeout(LIST_LIMIT) {
            Ok(mut mounts) => {
                let mut seen = std::collections::HashSet::new();
                mounts.retain(|m| seen.insert(m.clone()));
                self.mounts = mounts;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => self.listing = Some(listing),
            Err(mpsc::RecvTimeoutError::Disconnected) => {}
        }

        let mut out_before = std::mem::take(&mut self.pending);
        let asks: Vec<(String, Answer)> = self
            .mounts
            .iter()
            .map(|m| {
                let rx = out_before.remove(m).unwrap_or_else(|| {
                    let (probe, mount) = (self.probe.clone(), m.clone());
                    off(move || probe.space(&mount))
                });
                (m.clone(), rx)
            })
            .collect();
        // Whatever was still out for a volume that has left the list is
        // dropped with its receiver: the thread finishes into nothing.
        drop(out_before);

        let deadline = Instant::now() + limit;
        let mut table = Vec::with_capacity(asks.len());
        for (mount, rx) in asks {
            let left = deadline.saturating_duration_since(Instant::now());
            match rx.recv_timeout(left) {
                Ok(Some(space)) => {
                    self.last.insert(mount.clone(), space);
                    table.push(Mount {
                        mount,
                        space: Some(space),
                        stale: false,
                    });
                }
                Ok(None) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                    self.last.remove(&mount);
                    table.push(Mount {
                        mount,
                        space: None,
                        stale: false,
                    });
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    let space = self.last.get(&mount).copied();
                    self.pending.insert(mount.clone(), rx);
                    table.push(Mount {
                        mount,
                        space,
                        stale: true,
                    });
                }
            }
        }
        let listed = &self.mounts;
        self.last.retain(|m, _| listed.contains(m));
        table
    }
}

/// Runs `f` on a thread of its own and hands back where its answer will
/// land. A thread that cannot be made answers nothing, which reads as a
/// volume with no size — the same as one that failed.
fn off<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> mpsc::Receiver<T> {
    let (tx, rx) = mpsc::channel();
    let _ = std::thread::Builder::new()
        .name("khor-statfs".into())
        .spawn(move || {
            let _ = tx.send(f());
        });
    rx
}

/// What a request gets: the last reading, how old it is, and which
/// volumes in it did not answer in time.
#[derive(Debug, Clone)]
pub struct Reading {
    pub vitals: Vitals,
    pub age_ms: u64,
    /// Mount points (of volumes worth a row) whose numbers are from an
    /// earlier round, or that have no numbers yet.
    pub stale: Vec<String>,
}

struct Snap {
    vitals: Vitals,
    at: Instant,
    stale: Vec<String>,
}

#[derive(Default)]
struct State {
    snap: Option<Snap>,
    /// The watch list the last asker wanted rows for.
    watch: Vec<String>,
    rows: HashMap<String, WatchedDir>,
    rows_at: Option<Instant>,
    /// Somebody wants a round now (the watch list changed).
    poke: bool,
}

struct Local {
    home: PathBuf,
    every: Duration,
    state: Mutex<State>,
    changed: Condvar,
}

fn locals() -> MutexGuard<'static, HashMap<PathBuf, Arc<Local>>> {
    static LOCALS: OnceLock<Mutex<HashMap<PathBuf, Arc<Local>>>> = OnceLock::new();
    lock(LOCALS.get_or_init(|| Mutex::new(HashMap::new())))
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

/// The sampler for `home`, started with `probe` if there is none yet.
/// The second half says whether this call started it.
fn ensure(
    home: &Path,
    probe: impl FnOnce() -> Arc<dyn Probe>,
    every: Duration,
) -> (Arc<Local>, bool) {
    let mut all = locals();
    if let Some(l) = all.get(home) {
        return (l.clone(), false);
    }
    let local = Arc::new(Local {
        home: home.to_path_buf(),
        every,
        state: Mutex::new(State::default()),
        changed: Condvar::new(),
    });
    all.insert(home.to_path_buf(), local.clone());
    drop(all);
    let (runner, probe) = (local.clone(), probe());
    let started = std::thread::Builder::new()
        .name("khor-vitals".into())
        .spawn(move || run(runner, probe));
    if started.is_err() {
        locals().remove(home);
    }
    (local, true)
}

/// Starts this home's sampler with the real disks, if it is not running.
pub fn start(home: &Path) {
    ensure(home, real_probe, EVERY);
}

/// Starts this home's sampler with `probe` instead of the real disks —
/// the test seam, and the dev bridge's. Refused (`false`) when a sampler
/// already runs for this home: swapping the disks under a running one
/// would mix two machines into one reading.
pub fn install(home: &Path, probe: Arc<dyn Probe>, every: Duration) -> bool {
    ensure(home, move || probe, every).1
}

/// This machine's last reading. **Never blocks on a disk**; `None` until
/// the first round is done — «还没问过» is a state of its own
/// (docs/SESSION.md 离线), not a zero.
pub fn reading(home: &Path) -> Option<Reading> {
    let (local, _) = ensure(home, real_probe, EVERY);
    let state = lock(&local.state);
    snap_of(&state)
}

/// [`reading`], waiting up to `wait` for the first round. For a caller
/// that prints once and leaves (`khor devices`), or that answers another
/// machine, where 「还没问过」 would be a strange thing to say about
/// itself. Blocks — on a blocking thread only.
pub fn reading_within(home: &Path, wait: Duration) -> Option<Reading> {
    let (local, _) = ensure(home, real_probe, EVERY);
    let state = lock(&local.state);
    let (state, _) = local
        .changed
        .wait_timeout_while(state, wait, |s| s.snap.is_none())
        .unwrap_or_else(|p| p.into_inner());
    snap_of(&state)
}

fn snap_of(state: &State) -> Option<Reading> {
    state.snap.as_ref().map(|s| Reading {
        vitals: s.vitals.clone(),
        age_ms: s.at.elapsed().as_millis() as u64,
        stale: s.stale.clone(),
    })
}

/// This machine's watched directories as the sampler last found them,
/// and how old that is. Asking with a list the sampler has not seen
/// wakes it for a round; until that round is done the rows it has no
/// answer for are left out, and `None` means it has none at all yet.
pub fn watched(home: &Path, paths: &[String]) -> Option<(Vec<WatchedDir>, u64)> {
    let (local, _) = ensure(home, real_probe, EVERY);
    let mut state = lock(&local.state);
    if state.watch != paths {
        state.watch = paths.to_vec();
        state.poke = true;
        local.changed.notify_all();
    }
    rows_of(&state, paths)
}

/// [`watched`], waiting up to `wait` for a round that covers every path.
pub fn watched_within(
    home: &Path,
    paths: &[String],
    wait: Duration,
) -> Option<(Vec<WatchedDir>, u64)> {
    if paths.is_empty() {
        return Some((Vec::new(), 0));
    }
    let (local, _) = ensure(home, real_probe, EVERY);
    let mut state = lock(&local.state);
    if state.watch != paths {
        state.watch = paths.to_vec();
        state.poke = true;
        local.changed.notify_all();
    }
    let (state, _) = local
        .changed
        .wait_timeout_while(state, wait, |s| {
            s.rows_at.is_none() || !paths.iter().all(|p| s.rows.contains_key(p))
        })
        .unwrap_or_else(|p| p.into_inner());
    rows_of(&state, paths)
}

fn rows_of(state: &State, paths: &[String]) -> Option<(Vec<WatchedDir>, u64)> {
    if paths.is_empty() {
        return Some((Vec::new(), 0));
    }
    let at = state.rows_at?;
    let rows = paths
        .iter()
        .filter_map(|p| state.rows.get(p).cloned())
        .collect();
    Some((rows, at.elapsed().as_millis() as u64))
}

/// The sampler's whole life: a round, a wait, again — until its home is
/// gone.
fn run(local: Arc<Local>, probe: Arc<dyn Probe>) {
    let mut reader = Reader::new(probe);
    loop {
        if !local.home.exists() {
            let mut all = locals();
            if all.get(&local.home).is_some_and(|l| Arc::ptr_eq(l, &local)) {
                all.remove(&local.home);
            }
            return;
        }
        let table = reader.table(VOLUME_LIMIT);
        let vitals = super::compose(&local.home, &table);
        let stale = table
            .iter()
            .filter(|m| m.stale && super::keeps_volume(&m.mount))
            .map(|m| m.mount.clone())
            .collect();
        let watch = {
            let mut state = lock(&local.state);
            state.snap = Some(Snap {
                vitals,
                at: Instant::now(),
                stale,
            });
            local.changed.notify_all();
            state.watch.clone()
        };
        // After the reading is out, not before: a watched path on a slow
        // disk makes this part slow, and it must not make the reading
        // wait for it.
        let rows: HashMap<String, WatchedDir> = watch
            .iter()
            .map(|p| (p.clone(), super::watched_one(&table, p)))
            .collect();
        let state = lock(&local.state);
        let mut state = state;
        state.rows = rows;
        state.rows_at = Some(Instant::now());
        local.changed.notify_all();
        let (mut state, _) = local
            .changed
            .wait_timeout_while(state, local.every, |s| !s.poke)
            .unwrap_or_else(|p| p.into_inner());
        state.poke = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Volumes that do not exist, one of which answers late — the probe
    /// the reader's rules are proved on, so no real disk is asked.
    struct Fake {
        slow: Duration,
    }

    impl Probe for Fake {
        fn mounts(&self) -> Vec<String> {
            vec![
                "/khor-fast".into(),
                "/khor-slow".into(),
                "/khor-fast".into(),
            ]
        }
        fn space(&self, mount: &str) -> Option<(u64, u64)> {
            if mount == "/khor-slow" {
                std::thread::sleep(self.slow);
            }
            Some((100, 40))
        }
    }

    /// A late volume is marked stale with no numbers the first time, and
    /// **its `statfs` is not asked again while the first is still out**.
    #[test]
    fn a_late_volume_is_stale_and_is_not_asked_twice() {
        let mut r = Reader::new(Arc::new(Fake {
            slow: Duration::from_millis(600),
        }));
        let t = Instant::now();
        let first = r.table(Duration::from_millis(100));
        assert!(
            t.elapsed() < Duration::from_millis(400),
            "{:?}",
            t.elapsed()
        );
        assert_eq!(
            first,
            vec![
                Mount {
                    mount: "/khor-fast".into(),
                    space: Some((100, 40)),
                    stale: false
                },
                Mount {
                    mount: "/khor-slow".into(),
                    space: None,
                    stale: true
                },
            ]
        );
        assert!(r.pending.contains_key("/khor-slow"));
        // The same call is still out; a second round waits on it rather
        // than starting another, and once it lands the numbers are fresh.
        std::thread::sleep(Duration::from_millis(700));
        let second = r.table(Duration::from_millis(100));
        assert_eq!(second[1].space, Some((100, 40)));
        assert!(!second[1].stale);
    }
}
