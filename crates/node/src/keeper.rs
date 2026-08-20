//! The keeper: what stands between `khor serve` and dying silently.
//!
//! # Why a supervisor, and why khor's own
//!
//! hinton's serve died at 08:12 and the machine read as "offline" for
//! eleven hours — the box itself was fine (ping 0.13ms, sshd up), nothing
//! restarted the serve, and nothing even *knew*. Its log held six lines
//! of "在听" and not one word about a death: the process was killed with
//! a signal, and a process killed with a signal writes nothing. **Only
//! something standing outside it can say how it died** — that is the
//! whole argument for a supervisor, and it is also why "死前说一句"
//! cannot be done from the inside.
//!
//! systemd user units and launchd do this properly, and on a machine
//! that has them the install may grow that later. But khor's machines
//! include shared HPC boxes where systemd lingering needs an admin and
//! cron is disabled — the same machines that made the install story
//! "one file, no root". So the keeper ships inside that one file.
//!
//! # The shape
//!
//! `khor serve` is the keeper; the actual serve is the same binary
//! re-run with [`INNER_ENV`] set. The keeper spawns it, waits, writes
//! one line about every death, and starts it again — with backoff, so a
//! serve that dies at birth (bad config, taken port) does not become a
//! hot loop. A **clean** exit ends the keeper too: intentional shutdown
//! is not a failure to repair.
//!
//! # Generations
//!
//! Respawning re-executes [`crate::self_exe`], so any restart after an
//! upgrade runs the new version. But a healthy serve never restarts —
//! so the keeper also watches the binary on disk (one stat every couple
//! of seconds) and retires a healthy serve on purpose when the file
//! changes. Before it does, the new binary must answer `version`: on
//! 2026-08-19 a 0-byte file wearing the binary's name took two
//! machines' serve down, and this is the gate that would have refused
//! it. Nothing that cannot answer gets to serve; the old generation
//! keeps running and the refusal is one line in the log.
//!
//! The keeper itself stays on the old code until its own next start.
//! It is deliberately too small for that to matter: every feature
//! lives in the inner serve.
//!
//! # The shield (#76)
//!
//! On an NFS home, replacing the binary can kill the *running* keeper
//! on another machine — silently, twice observed (2026-08-20, SIGBUS
//! suspicion: the text pages of a running image faulting against a
//! replaced file). So a keeper whose binary sits on a network
//! filesystem copies itself to the local disk and re-execs from the
//! copy before doing anything else: same pid (serve.pid, `khor quit`,
//! systemd's PIDFile all keep working), same argv, one extra env var
//! ([`SHIELD_FROM`]) remembering the install path — which stays the
//! watched path, so upgrades land exactly as before, except each new
//! generation is also copied local before it is preflighted and
//! spawned. Nothing khor keeps running executes off the network disk.
//!
//! `KHOR_SHIELD=1` forces the shield on any filesystem (how the tests
//! reach it from a local disk); `KHOR_SHIELD=0` forbids it.
//!
//! # Signals
//!
//! The pid file written by the installer holds the **keeper's** pid, so
//! `kill $(cat serve.pid)` must stop everything: the keeper forwards
//! SIGTERM/SIGINT to the child and exits without respawning. Ctrl-C in a
//! foreground `khor serve` reaches both (same process group) and takes
//! the same path.

use std::sync::atomic::{AtomicI32, Ordering};

use khor_catalog::msg;

/// Set on the child: "you are the serve, not the keeper". An env var
/// rather than a hidden verb so `ps` shows the same honest `khor serve`
/// for both processes, parent over child.
pub const INNER_ENV: &str = "KHOR_SERVE_KEPT";

/// Whether this process is the kept serve itself.
pub fn is_inner() -> bool {
    std::env::var_os(INNER_ENV).is_some()
}

/// Set on the shielded keeper by its own pre-exec self: the install
/// path — the one to watch for new generations — which `current_exe`
/// no longer answers once the process runs from a local copy.
const SHIELD_FROM: &str = "KHOR_SHIELD_FROM";

/// "1" shields on any filesystem (tests), "0" never shields; unset
/// means "shield exactly when the binary sits on a network mount".
const SHIELD_ENV: &str = "KHOR_SHIELD";

/// Whether this path lives on a filesystem where replacing a file can
/// hurt processes already running from it (#76). Unknown reads as
/// local: the shield is medicine, not a default.
#[cfg(target_os = "linux")]
fn on_network_fs(path: &std::path::Path) -> bool {
    use std::os::unix::ffi::OsStrExt;
    let Ok(c) = std::ffi::CString::new(path.as_os_str().as_bytes()) else {
        return false;
    };
    let mut st: libc::statfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statfs(c.as_ptr(), &mut st) } != 0 {
        return false;
    }
    // NFS, SMB2, CIFS, Lustre, Ceph, BeeGFS, 9p.
    matches!(
        st.f_type as u64,
        0x6969 | 0xFE534D42 | 0xFF534D42 | 0x0BD0_0BD0 | 0x00C3_6400 | 0x1983_0326 | 0x0102_1997
    )
}

#[cfg(target_os = "macos")]
fn on_network_fs(path: &std::path::Path) -> bool {
    use std::os::unix::ffi::OsStrExt;
    let Ok(c) = std::ffi::CString::new(path.as_os_str().as_bytes()) else {
        return false;
    };
    let mut st: libc::statfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statfs(c.as_ptr(), &mut st) } != 0 {
        return false;
    }
    let name = unsafe { std::ffi::CStr::from_ptr(st.f_fstypename.as_ptr()) };
    matches!(name.to_bytes(), b"nfs" | b"smbfs" | b"afpfs" | b"webdav")
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn on_network_fs(_path: &std::path::Path) -> bool {
    false
}

/// Where this install's local copies live: per-user (a shared /tmp) and
/// per-install-path (two stores may share one machine in the tests).
fn shield_dir(watched: &std::path::Path) -> std::path::PathBuf {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    watched.hash(&mut h);
    let uid = unsafe { libc::getuid() };
    std::env::temp_dir().join(format!("khor-shield-{uid}-{:016x}", h.finish()))
}

/// Copies the watched binary to the local disk and returns the copy's
/// path, named after the generation's fingerprint. Copy-then-rename so
/// a torn write never wears a finished name; the unlink first because
/// overwriting a file in place is the very harm the shield exists to
/// avoid (a running image survives an unlink, not a rewrite). Old
/// generations are swept by age — running images survive that too.
fn local_copy(watched: &std::path::Path) -> Result<std::path::PathBuf, String> {
    let dir = shield_dir(watched);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            let stale = e
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.elapsed().ok())
                .is_some_and(|age| age > std::time::Duration::from_secs(7 * 24 * 3600));
            if stale {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
    let dst = dir.join(match fingerprint(watched) {
        Some((ino, mtime, len)) => format!("khor-{ino:x}-{mtime:x}-{len:x}"),
        None => format!("khor-{}", std::process::id()),
    });
    let tmp = dir.join(format!(".copy-{}", std::process::id()));
    std::fs::copy(watched, &tmp).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&dst);
    std::fs::rename(&tmp, &dst).map_err(|e| e.to_string())?;
    Ok(dst)
}

/// #76's preventive half. Returns the path to WATCH for new
/// generations — the install path — after having moved this process
/// onto a local copy when that path lives on a network filesystem.
/// The move is an exec: same pid, same args; on any failure the keeper
/// says so once and serves from where it is, shieldless.
fn shield() -> Result<std::path::PathBuf, String> {
    if let Some(orig) = std::env::var_os(SHIELD_FROM) {
        return Ok(std::path::PathBuf::from(orig));
    }
    let exe = crate::self_exe()?;
    let wanted = match std::env::var(SHIELD_ENV).ok().as_deref() {
        Some("0") => false,
        Some(_) => true,
        None => on_network_fs(&exe),
    };
    if !wanted {
        return Ok(exe);
    }
    let stamp = jiff::Zoned::now().strftime("%Y-%m-%d %H:%M:%S").to_string();
    match local_copy(&exe) {
        Ok(copy) => {
            eprintln!("{}", msg::serve_shielding(&stamp, exe.display()));
            use std::os::unix::process::CommandExt;
            let err = std::process::Command::new(&copy)
                .args(std::env::args_os().skip(1))
                .env(SHIELD_FROM, &exe)
                .exec();
            // exec returns only on failure.
            eprintln!("{}", msg::serve_shield_failed(&stamp, err));
            Ok(exe)
        }
        Err(why) => {
            eprintln!("{}", msg::serve_shield_failed(&stamp, &why));
            Ok(exe)
        }
    }
}

/// What a stable new generation is taken FROM: behind the shield, a
/// fresh local copy of it (preflighted as the copy, spawned as the
/// copy); shieldless, the watched path itself.
fn take_generation(
    watched: &std::path::Path,
    shielded: bool,
) -> Result<(String, std::path::PathBuf), String> {
    let from = if shielded { local_copy(watched)? } else { watched.to_path_buf() };
    let version = preflight(&from)?;
    Ok((version, from))
}

/// A child that lived at least this long was up for real, and its death
/// is news rather than a birth defect: the backoff resets.
const LIVED: std::time::Duration = std::time::Duration::from_secs(60);

/// The longest the keeper waits between respawns of a serve that keeps
/// dying young. Long enough not to hammer whatever is killing it, short
/// enough that a machine recovers within a minute of the cause clearing.
const BACKOFF_MAX: u64 = 60;

/// How often the keeper checks whether the child is still there.
const CHILD_POLL: std::time::Duration = std::time::Duration::from_millis(200);

/// How often the keeper glances at the binary on disk: one stat,
/// cheap enough to do forever, short enough that an upgrade lands
/// within seconds.
const SELF_CHECK: std::time::Duration = std::time::Duration::from_secs(2);

/// After the keeper's own TERM at a generation change, the old serve
/// gets this long to leave before SIGKILL.
const SWAP_GRACE: std::time::Duration = std::time::Duration::from_secs(10);

/// What "the same binary" means on disk: installs replace the file
/// (new inode), a rewrite in place changes mtime or length. `None`
/// while the path is momentarily absent (mid-`mv`) — absence is never
/// a generation.
fn fingerprint(exe: &std::path::Path) -> Option<(u64, i64, u64)> {
    use std::os::unix::fs::MetadataExt;
    let m = std::fs::metadata(exe).ok()?;
    Some((m.ino(), m.mtime(), m.len()))
}

/// The new generation proves itself by answering `version` — a
/// truncated download, an empty file, or somebody else's binary all
/// fail here, and the running serve is left alone.
fn preflight(exe: &std::path::Path) -> Result<String, String> {
    let out = std::process::Command::new(exe)
        .arg("version")
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(match out.status.code() {
            Some(code) => msg::died_with_code(code),
            None => msg::DIED_UNREADABLY.to_owned(),
        });
    }
    let version = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    if version.is_empty() {
        return Err(msg::SWAP_NO_ANSWER.to_owned());
    }
    Ok(version)
}

/// The kept child's pid, for the signal handler — which may run at any
/// instant and can touch nothing but an atomic.
static CHILD: AtomicI32 = AtomicI32::new(0);

/// Forwards the signal to the child and leaves. `_exit` because a
/// signal handler must not unwind, and there is nothing to clean up —
/// the child owns every resource worth the name.
extern "C" fn forward(sig: libc::c_int) {
    let child = CHILD.load(Ordering::SeqCst);
    if child > 0 {
        unsafe {
            libc::kill(child, sig);
        }
    }
    unsafe { libc::_exit(128 + sig) }
}

/// Runs the serve under the keeper, forever. Returns only when the serve
/// ends cleanly (exit 0) or a child cannot even be spawned.
pub fn keep() -> Result<(), String> {
    // Before anything else: off the network disk (#76). On success this
    // is an exec and the pid survives it; either way `watched` is the
    // install path — where upgrades land — and `source` is what serves.
    let watched = shield()?;
    let shielded = std::env::var_os(SHIELD_FROM).is_some();
    let mut source = crate::self_exe()?;
    unsafe {
        libc::signal(libc::SIGTERM, forward as libc::sighandler_t);
        libc::signal(libc::SIGINT, forward as libc::sighandler_t);
    }
    let mut backoff = 1u64;
    let mut running = env!("CARGO_PKG_VERSION").to_owned();
    loop {
        let mut child = std::process::Command::new(&source)
            .args(std::env::args_os().skip(1))
            .env(INNER_ENV, "1")
            .spawn()
            .map_err(msg::host_wont_start)?;
        CHILD.store(child.id() as i32, Ordering::SeqCst);
        let born = std::time::Instant::now();
        let on_disk = fingerprint(&watched);
        // A change must hold still for two looks before it counts: a
        // download writing the file in place is many changes in a row,
        // and none of them is a generation yet.
        let mut seen: Option<(u64, i64, u64)> = None;
        let mut refused: Option<(u64, i64, u64)> = None;
        let mut swap: Option<(String, std::path::PathBuf)> = None;
        let mut term_at: Option<std::time::Instant> = None;
        let mut last_look = std::time::Instant::now();
        let status = loop {
            if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
                break status;
            }
            if let Some(t) = term_at {
                if t.elapsed() >= SWAP_GRACE {
                    let _ = child.kill();
                }
            }
            std::thread::sleep(CHILD_POLL);
            if swap.is_some() || last_look.elapsed() < SELF_CHECK {
                continue;
            }
            last_look = std::time::Instant::now();
            // The local copy this generation runs from can be swept by
            // a /tmp cleaner while the watched install is unchanged;
            // hand over to a fresh copy of the same generation before
            // spawns start failing. If even that copy cannot be made,
            // fall back to serving straight off the watched path.
            if source != watched && fingerprint(&source).is_none() {
                let stamp = jiff::Zoned::now().strftime("%Y-%m-%d %H:%M:%S").to_string();
                eprintln!("{}", msg::serve_copy_swept(&stamp));
                let next = local_copy(&watched).unwrap_or_else(|_| watched.clone());
                swap = Some((running.clone(), next));
                term_at = Some(std::time::Instant::now());
                let pid = CHILD.load(Ordering::SeqCst);
                if pid > 0 {
                    unsafe {
                        libc::kill(pid, libc::SIGTERM);
                    }
                }
                continue;
            }
            let Some(now) = fingerprint(&watched) else {
                seen = None;
                continue;
            };
            if Some(now) == on_disk {
                seen = None;
                continue;
            }
            if refused == Some(now) {
                continue;
            }
            if seen != Some(now) {
                seen = Some(now);
                continue;
            }
            let stamp = jiff::Zoned::now().strftime("%Y-%m-%d %H:%M:%S").to_string();
            match take_generation(&watched, shielded) {
                Ok((next, from)) => {
                    eprintln!("{}", msg::serve_swapping(&stamp, &running, &next));
                    swap = Some((next, from));
                    term_at = Some(std::time::Instant::now());
                    let pid = CHILD.load(Ordering::SeqCst);
                    if pid > 0 {
                        unsafe {
                            libc::kill(pid, libc::SIGTERM);
                        }
                    }
                }
                Err(why) => {
                    eprintln!("{}", msg::serve_swap_refused(&stamp, &why));
                    refused = Some(now);
                }
            }
        };
        CHILD.store(0, Ordering::SeqCst);
        if let Some((next, from)) = swap {
            // The keeper's own TERM: a handover, not a death and not an
            // operator's stop — no death line, no backoff, and the exit
            // code (clean or signal 15) means nothing here.
            running = next;
            source = from;
            backoff = 1;
            continue;
        }
        if status.success() {
            return Ok(());
        }
        // How it died, from outside — the one vantage point that still
        // works when the death was a signal.
        let how = {
            use std::os::unix::process::ExitStatusExt;
            match (status.code(), status.signal()) {
                (Some(code), _) => msg::died_with_code(code),
                (None, Some(sig)) => msg::died_by_signal(sig),
                (None, None) => msg::DIED_UNREADABLY.to_owned(),
            }
        };
        if born.elapsed() >= LIVED {
            backoff = 1;
        }
        // 报故障要带时间戳: this line lands in a log somebody reads
        // hours later, next to lines from other lifetimes.
        let stamp = jiff::Zoned::now().strftime("%Y-%m-%d %H:%M:%S").to_string();
        eprintln!("{}", msg::serve_died_restarting(&stamp, &how, backoff));
        std::thread::sleep(std::time::Duration::from_secs(backoff));
        backoff = (backoff * 2).min(BACKOFF_MAX);
    }
}
