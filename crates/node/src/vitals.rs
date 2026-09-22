//! This machine's live readings (`khor_core::Vitals`).
//!
//! One sample is one answer to "what is this machine doing right now".
//! It is never stored in a replicated document — see the judgment on
//! [`khor_core::Vitals`] — so everything here is about taking a reading
//! honestly, not about keeping one.

use std::path::Path;
use std::sync::{Mutex, OnceLock};

use khor_core::{Fill, Vitals, Volume, WatchedDir};

pub(crate) mod gpu;
pub mod model;

/// Everything a reading is taken from, kept alive between calls.
///
/// # Why it outlives the call at all
///
/// **A CPU percentage is a rate, so it is only ever "over some window",
/// and the window is decided here.** `sysinfo` derives it from the
/// previous reading held inside the `System` it is read from, so how that
/// `System` is managed *is* the choice of window — and the card says
/// "right now", which is a claim about one particular one.
///
/// Measured on this machine (macOS, 2026-08-17, load ≈ 5), four ways of
/// asking the same question:
///
/// ```text
/// fresh System, one refresh           43.6
/// fresh System, two refreshes, no gap  43.6
/// fresh System, two refreshes + 200ms  49.1
/// same System, refreshed 500ms later   48.8
/// ```
///
/// Two things fall out, and the first **corrected a wrong assumption this
/// module was first written on**: a fresh `System` does *not* report
/// `0.0` here — it answers, and it answers plausibly. What it answers is
/// a different question (an average over some long span since the process
/// started), and the two land within six points of each other on an
/// ordinary machine, which is exactly why reading one as the other would
/// never look wrong.
///
/// So the window becomes the gap between whoever asked last and whoever
/// is asking now. The **first** sample in a process has no predecessor
/// and pays [`sysinfo::MINIMUM_CPU_UPDATE_INTERVAL`] once, so that a
/// one-shot `khor devices` reports the present rather than an average
/// since boot.
///
/// # Why the disks are in here too, and what it costs
///
/// `Disks` is kept for a different reason: it is **expensive exactly
/// once**. Measured the same day, same machine, four mounted volumes:
///
/// ```text
/// Disks::new_with_refreshed_list  #0  1.186 s
///                                 #1  14.5 ms
///                                 #2  13.7 ms
/// ```
///
/// A fresh list per sample would pay that 1.2 s on the first reading of
/// every process — and it did, inside a request handler, where it stalled
/// the QUIC connection long enough that two real-connection tests timed
/// out at twenty seconds. Refreshing a list that already exists is the
/// 14 ms path, and the list is refreshed rather than reused verbatim so a
/// volume plugged in after start still shows up.
///
/// **Everything here blocks**, which is why the one caller inside async
/// code goes through `spawn_blocking` (`link.rs`, `Request::Vitals`).
struct Sampler {
    sys: sysinfo::System,
    disks: sysinfo::Disks,
}

static SAMPLER: OnceLock<Mutex<Sampler>> = OnceLock::new();

/// Takes a reading of this machine. **Blocking** — see [`Sampler`].
///
/// `home` is khor's root, and it decides only which filesystem gets
/// reported — see [`disk_under`].
pub fn sample(home: &Path) -> Vitals {
    // The first reading of the pair is taken when the sampler is made
    // (`sampler`): nothing is read out of it, it exists so the reading
    // below has something to subtract. A panic in another sampler left
    // the numbers, not the lock's invariant, in doubt: the worst a
    // poisoned `Sampler` holds is a stale previous reading, which the
    // refresh below replaces.
    let mut s = sampler();

    s.sys.refresh_cpu_usage();
    s.sys.refresh_memory();
    // `true`: a volume that was unmounted since the last reading leaves
    // the list. Keeping it would report a disk that is not there, which
    // is worse than reporting none.
    s.disks.refresh(true);
    let cpu_pct = s.sys.global_cpu_usage();
    let cores = s.sys.cpus().len() as u32;
    let mem = Fill {
        used: s.sys.used_memory(),
        total: s.sys.total_memory(),
    };
    let disk = disk_under(&s.disks, home);
    let volumes = tidy_volumes(volumes_of(&s.disks), home);
    // **Released before the GPU is asked**, and the ordering is the
    // whole reason these are locals rather than fields written inline.
    // The GPU keeps nothing between readings, so it has no use for the
    // `Sampler` — and holding the lock across a call into a vendor
    // driver would make every other sampler in the process wait behind
    // it for nothing.
    drop(s);
    let (gpu, gpus) = gpu::sample_all();
    // The version of the binary doing the sampling — which is the one
    // running on this machine, whatever else is on its disk.
    Vitals {
        cpu_pct,
        cores,
        mem,
        disk,
        gpu,
        version: Some(env!("CARGO_PKG_VERSION").to_owned()),
        gpus,
        volumes,
        uptime_s: sysinfo::System::uptime(),
        model: model::this_machine(),
        system: khor_core::SystemKind::this_machine(),
    }
}

/// The rows for a machine's watch list (2026-09-06 关注目录): each path
/// as given, and the listed volume it sits on. **Blocking**, like
/// [`sample`], and behind the same lock for the same disk list.
///
/// Which volume: the mounted disk whose mount point is the longest
/// prefix of the canonical path — [`disk_under`]'s rule — **and whose
/// device is the directory's own device.** The second half is what
/// keeps a path on a filesystem sysinfo does not list (a pseudo mount,
/// one `keeps_volume` refuses) from wearing the root's numbers: its
/// longest listed prefix is `/`, and the root's fill under `/proc/1`
/// is a reading about the wrong disk. Such a path comes back as the
/// third shape (`missing: false`, no volume) and the faces say so.
pub fn watched(paths: &[String]) -> Vec<WatchedDir> {
    let mut s = sampler();
    s.disks.refresh(true);
    paths.iter().map(|p| watched_one(&s.disks, p)).collect()
}

fn watched_one(disks: &sysinfo::Disks, path: &str) -> WatchedDir {
    let p = Path::new(path);
    let row = |missing: bool, volume: Option<Volume>| WatchedDir {
        path: path.to_owned(),
        missing,
        volume,
    };
    let Ok(meta) = std::fs::metadata(p) else {
        return row(true, None);
    };
    let real = p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    let Some(d) = disks
        .list()
        .iter()
        .filter(|d| real.starts_with(d.mount_point()))
        .max_by_key(|d| d.mount_point().as_os_str().len())
    else {
        return row(false, None);
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if let Ok(m) = std::fs::metadata(d.mount_point())
            && m.dev() != meta.dev()
        {
            return row(false, None);
        }
    }
    let total = d.total_space();
    if total == 0 {
        return row(false, None);
    }
    row(
        false,
        Some(Volume {
            mount: d.mount_point().to_string_lossy().into_owned(),
            fill: Fill {
                used: total.saturating_sub(d.available_space()),
                total,
            },
        }),
    )
}

/// The process-wide sampler, made on first use — see [`Sampler`].
fn sampler() -> std::sync::MutexGuard<'static, Sampler> {
    SAMPLER
        .get_or_init(|| {
            let mut sys = sysinfo::System::new();
            sys.refresh_cpu_usage();
            std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
            Mutex::new(Sampler {
                sys,
                disks: sysinfo::Disks::new_with_refreshed_list(),
            })
        })
        .lock()
        .unwrap_or_else(|p| p.into_inner())
}

/// The filesystem a path sits on: the mounted disk whose mount point is
/// the longest prefix of it.
///
/// **A machine has many filesystems and no single "the disk".** Summing
/// them would fold a plugged-in backup drive into the number somebody
/// reads as "am I running out of room" — and this very machine keeps its
/// build output on an external volume, so that is not a hypothetical.
/// Taking the root would be wrong for the same reason from the other
/// direction. The one with a reason behind it is the filesystem khor's
/// own data is written to, which is the one that fills up when khor keeps
/// working.
///
/// `None` when no mount point contains the path — the caller paints
/// nothing rather than a zero, because `0 of 0` reads as an empty disk.
fn disk_under(disks: &sysinfo::Disks, home: &Path) -> Option<Fill> {
    // Canonicalized so the comparison is between real paths: a home
    // reached through a symlink (this repo's `target/` is one) lives on
    // the filesystem the link points at, not the one holding the link.
    let path = home.canonicalize().unwrap_or_else(|_| home.to_path_buf());
    disks
        .list()
        .iter()
        .filter(|d| path.starts_with(d.mount_point()))
        .max_by_key(|d| d.mount_point().as_os_str().len())
        .map(|d| Fill {
            // **Used is derived from what is free, not read as its own
            // number**, and the two are genuinely different on APFS.
            // Measured here 2026-08-17 on `/System/Volumes/Data`:
            // `df` says 228.2G total, 184.1G used, 18.6G available —
            // which does not add up, because purgeable space belongs to
            // neither column. The pair a person can act on is
            // (what is left, out of how much), so `used` is the
            // complement of available and the two halves of this `Fill`
            // always sum to the total, which is what a bar draws.
            used: d.total_space().saturating_sub(d.available_space()),
            total: d.total_space(),
        })
}

/// Whether a mount point is a volume a person means by 「硬盘存储」.
///
/// A blacklist rather than a whitelist, because real volumes mount in
/// too many places to enumerate (`/`, `/home`, `/data`, `/mnt`,
/// `/Volumes/…`) while the noise is a short known family:
/// system-managed service volumes and virtual filesystems.
/// `/System/Volumes/Data` is the one system-managed exception — on
/// macOS it *is* the person's data volume, the filesystem everything
/// they touch lives on.
///
/// Which way this can be wrong: a genuinely interesting volume mounted
/// under a rejected prefix (say `/private/var/…`) stays off the list;
/// the single-volume reading (`Vitals::disk`) still reports it when
/// khor's home is there, so the loss is a missing row, never a wrong
/// one.
pub(crate) fn keeps_volume(mount: &str) -> bool {
    if mount == "/" || mount == "/System/Volumes/Data" {
        return true;
    }
    const NOISE: [&str; 9] = [
        "/System/",
        "/private/",
        "/dev",
        "/proc",
        "/sys",
        "/run",
        "/boot",
        "/snap",
        "/var/lib/",
    ];
    !NOISE.iter().any(|p| mount.starts_with(p))
}

/// Every volume worth a row, raw — [`tidy_volumes`] finishes the list.
fn volumes_of(disks: &sysinfo::Disks) -> Vec<Volume> {
    disks
        .list()
        .iter()
        .filter_map(|d| {
            let total = d.total_space();
            if total == 0 {
                return None;
            }
            let mount = d.mount_point().to_string_lossy().into_owned();
            keeps_volume(&mount).then(|| Volume {
                mount,
                // Used as the complement of available — `disk_under`'s
                // APFS note is the reason, and two derivations of one
                // number on one screen would disagree.
                fill: Fill {
                    used: total.saturating_sub(d.available_space()),
                    total,
                },
            })
        })
        .collect()
}

/// Dedupes, drops the sealed twin, and puts the volume khor lives on
/// first — split from the collection so a hand-built list can prove
/// each rule (a `sysinfo::Disks` cannot be built by hand). Shared with
/// the satellite parser, whose lines come over ssh instead of sysinfo.
///
/// The sealed twin: on macOS `/` is a snapshot sharing one APFS
/// container with `/System/Volumes/Data`, so with Data on the list `/`
/// is the same space twice under another name.
pub(crate) fn tidy_volumes(mut volumes: Vec<Volume>, home: &Path) -> Vec<Volume> {
    let home = home.canonicalize().unwrap_or_else(|_| home.to_path_buf());
    let mut seen = std::collections::HashSet::new();
    volumes.retain(|v| seen.insert(v.mount.clone()));
    if volumes.iter().any(|v| v.mount == "/System/Volumes/Data") {
        volumes.retain(|v| v.mount != "/");
    }
    let home_mount = volumes
        .iter()
        .filter(|v| home.starts_with(&v.mount))
        .max_by_key(|v| v.mount.len())
        .map(|v| v.mount.clone());
    volumes.sort_by_key(|v| (Some(&v.mount) != home_mount.as_ref(), v.mount.clone()));
    volumes
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The reading is of a real machine, so the assertions are the ones
    /// that hold on every machine — a hard-coded number here would go red
    /// on somebody else's laptop (the rule `tests/real_disk.rs` follows).
    #[test]
    fn a_reading_describes_the_machine_it_was_taken_on() {
        let v = sample(Path::new("."));
        assert!(v.cores >= 1, "a machine has at least one core: {}", v.cores);
        assert!(v.mem.total > 0, "a machine has memory: {:?}", v.mem);
        assert!(
            v.mem.used <= v.mem.total,
            "used memory cannot exceed the total: {:?}",
            v.mem
        );
        assert!(
            (0.0..=100.0).contains(&v.cpu_pct),
            "a whole-machine percentage is a percentage: {}",
            v.cpu_pct
        );
    }

    /// A machine doing work says so.
    ///
    /// **This does not discriminate between the two sampling designs, and
    /// saying so is the point.** It was written to prove the process-wide
    /// sampler was necessary, and swapping in a fresh `System` per call
    /// left it green — which is how the measurement in [`SAMPLER`] came to
    /// be taken and how the assumption in that comment came to be
    /// corrected. What is left here still guards something real: a
    /// sampler that returns a constant, a zero, or the wrong field fails
    /// it. The window it measures over is **not** testable on a shared
    /// machine, and pretending otherwise would be a green that means
    /// nothing on a quiet day.
    ///
    /// The load comes from this test's own thread rather than from any
    /// process it starts: borrowing the machine's existing work would make
    /// the assertion depend on what else is running.
    #[test]
    fn a_pinned_core_shows_up_in_the_reading() {
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = stop.clone();
        let spin = std::thread::spawn(move || {
            let mut n: u64 = 0;
            while !flag.load(std::sync::atomic::Ordering::Relaxed) {
                n = n.wrapping_add(1);
            }
            n
        });
        let v = sample(Path::new("."));
        stop.store(true, std::sync::atomic::Ordering::Relaxed);
        let _ = spin.join();
        assert!(
            v.cpu_pct > 0.0,
            "one core was pinned; the reading says {}",
            v.cpu_pct
        );
    }

    /// A path on no filesystem at all has no disk to report, and that is
    /// an absence rather than a zero.
    ///
    /// **The positive half runs first and in the same test**, because a
    /// `disk_under` that always answered `None` would satisfy the negative
    /// assertion on its own — an absence and a broken lookup read alike
    /// (docs/handoff 否定式断言之前先证明测法还活着).
    #[test]
    fn a_path_under_no_mount_point_reports_no_disk() {
        let disks = sysinfo::Disks::new_with_refreshed_list();

        let found = disk_under(&disks, Path::new(".")).expect("this directory is on a filesystem");
        assert!(
            found.total > 0,
            "a mounted filesystem has a size: {found:?}"
        );
        assert!(
            found.used <= found.total,
            "used cannot exceed the total: {found:?}"
        );

        // Not a prefix of any mount point on any platform: mount points
        // are absolute paths, and this is not one.
        assert_eq!(
            disk_under(&disks, Path::new("khor-no-such-relative-path")),
            None
        );
    }

    /// A watched directory that is there sits on a volume with a size;
    /// one that is not there says so and names no volume. **The positive
    /// half runs first**, `a_path_under_no_mount_point_reports_no_disk`'s
    /// rule: an answer of 「missing」 for everything would pass the
    /// negative half alone.
    #[test]
    fn a_watched_directory_names_its_volume_and_a_missing_one_says_so() {
        let here = std::env::current_dir().unwrap().display().to_string();
        let rows = watched(&[here.clone(), "/khor-no-such-dir-anywhere".into()]);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].path, here);
        assert!(!rows[0].missing);
        let vol = rows[0]
            .volume
            .as_ref()
            .expect("this directory is on a listed volume");
        assert!(
            vol.fill.total > 0 && vol.fill.used <= vol.fill.total,
            "{vol:?}"
        );
        assert!(rows[1].missing && rows[1].volume.is_none(), "{:?}", rows[1]);
    }

    /// The third shape, on the one path every Mac has on a filesystem
    /// sysinfo does not list: `/dev` is devfs. It exists, so it is not
    /// missing; its longest listed prefix is `/`, so without the device
    /// check it would come back wearing the root's numbers — which is
    /// the reading this shape exists to refuse. Measured here only: a
    /// Linux `/dev` is devtmpfs and may or may not be in the list.
    #[cfg(target_os = "macos")]
    #[test]
    fn a_directory_on_an_unlisted_filesystem_names_no_volume_rather_than_the_root() {
        let rows = watched(&["/dev".into()]);
        assert!(!rows[0].missing, "/dev is there");
        assert!(
            rows[0].volume.is_none(),
            "/dev came back on {:?} — the root's numbers under a pseudo mount",
            rows[0].volume
        );
    }

    /// The predicate keeps what a person means by 「硬盘」 and drops the
    /// machinery — including keeping the one system-managed path that
    /// IS the person's data volume.
    #[test]
    fn a_volume_is_kept_by_what_it_means_not_where_it_mounts() {
        for keep in [
            "/",
            "/System/Volumes/Data",
            "/home",
            "/data",
            "/mnt/scratch",
            "/Volumes/disk0",
        ] {
            assert!(keeps_volume(keep), "{keep} is a person's volume");
        }
        for noise in [
            "/System/Volumes/VM",
            "/private/var/vm",
            "/dev",
            "/proc",
            "/run/lock",
            "/boot/efi",
            "/snap/core",
            "/var/lib/docker",
        ] {
            assert!(!keeps_volume(noise), "{noise} is machinery");
        }
    }

    /// The tidy pass: duplicates collapse, the sealed `/` twin leaves
    /// when Data is present, and the volume khor's home fills leads.
    /// (The hand-built homes do not exist, so `canonicalize` falls back
    /// to the literal path — the branch every platform takes here.)
    #[test]
    fn tidied_volumes_lead_with_home_and_drop_the_sealed_twin() {
        let vol = |mount: &str, used| Volume {
            mount: mount.to_owned(),
            fill: Fill { used, total: 100 },
        };
        let raw = vec![
            vol("/", 1),
            vol("/System/Volumes/Data", 2),
            vol("/Volumes/disk0", 3),
            vol("/Volumes/disk0", 3),
        ];
        let out = tidy_volumes(raw, Path::new("/System/Volumes/Data/Users/x"));
        assert_eq!(
            out.iter().map(|v| v.mount.as_str()).collect::<Vec<_>>(),
            ["/System/Volumes/Data", "/Volumes/disk0"],
        );

        let out = tidy_volumes(vec![vol("/", 1), vol("/data", 2)], Path::new("/data/khor"));
        assert_eq!(
            out.iter().map(|v| v.mount.as_str()).collect::<Vec<_>>(),
            ["/data", "/"],
            "home's own volume leads even against the root",
        );
    }
}
