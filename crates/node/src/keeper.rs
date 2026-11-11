//! Supervise serve without retiring its durable terminal or agent hosts.
//!
//! Watch the stable installation pointer; execute immutable node-local images.
//! Each selected image keeps its OS lease through spawn or exec. Shared release
//! packages and legacy shield directories are outside this collector's ownership.

mod startup;

pub(crate) use startup::ready;
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

/// Unix reparents a kept serve to init after the keeper exits.
pub(crate) fn parent_gone() -> bool {
    is_inner() && unsafe { libc::getppid() } == 1
}

/// Set on the shielded keeper by its own pre-exec self: the install
/// path — the one to watch for new generations — which `current_exe`
/// no longer answers once the process runs from a local copy.
const SHIELD_FROM: &str = "KHOR_SHIELD_FROM";

/// The installer's platform pointer survives immutable version changes.
const INSTALL_SOURCE: &str = "KHOR_INSTALL_SOURCE";

/// "1" also names local relocation in the log. "0" keeps this supervisor
/// in place for development; its children still use owned runtime images.
const SHIELD_ENV: &str = "KHOR_SHIELD";

/// Whether this path lives on a filesystem where replacing a file can
/// hurt processes already running from it (#76). Unknown is not classified
/// as network here; runtime storage independently requires a proven local filesystem. The same mounts are the
/// ones whose changes made by another host no watcher here is told of,
/// which a directory watch says (`crate::watch`).
#[cfg(target_os = "linux")]
pub(crate) fn on_network_fs(path: &std::path::Path) -> bool {
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
pub(crate) fn on_network_fs(path: &std::path::Path) -> bool {
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
pub(crate) fn on_network_fs(_path: &std::path::Path) -> bool {
    false
}

/// Move the keeper itself off mutable or shared installation paths before serving.
fn shield() -> Result<std::path::PathBuf, String> {
    let exe = crate::self_exe()?;
    let watched = std::env::var_os(SHIELD_FROM)
        .or_else(|| std::env::var_os(INSTALL_SOURCE))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| exe.clone());
    if khor_runtime::Store::lease_for_program(&exe)
        .map_err(|error| error.to_string())?
        .is_some()
        || std::env::var(SHIELD_ENV).ok().as_deref() == Some("0")
    {
        return Ok(watched);
    }
    let selected = crate::updates::selected_cli_source(&watched)?;
    let image = crate::runtime_image::host_at(&selected)?;
    if on_network_fs(&exe) || std::env::var(SHIELD_ENV).ok().as_deref() == Some("1") {
        let stamp = jiff::Zoned::now().strftime("%Y-%m-%d %H:%M:%S").to_string();
        eprintln!("{}", msg::serve_shielding(&stamp, exe.display()));
    }
    let mut command = std::process::Command::new(image.path());
    command
        .args(std::env::args_os().skip(1))
        .env(SHIELD_FROM, &watched);
    Err(image.exec(&mut command).to_string())
}

fn take_generation(
    watched: &std::path::Path,
) -> Result<(String, crate::runtime_image::HostImage), String> {
    let selected = crate::updates::selected_cli_source(watched)?;
    let image = crate::runtime_image::host_at(&selected)?;
    Ok((image.version().into(), image))
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
    let selected = crate::updates::selected_cli_source(exe).ok()?;
    if !crate::updates::cli_activation_ready(&selected).ok()? {
        return None;
    }
    let m = std::fs::metadata(selected).ok()?;
    Some((m.ino(), m.mtime(), m.len()))
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
    let mut source = crate::runtime_image::running_host()?;
    unsafe {
        libc::signal(libc::SIGTERM, forward as *const () as libc::sighandler_t);
        libc::signal(libc::SIGINT, forward as *const () as libc::sighandler_t);
    }
    let mut backoff = 1u64;
    let mut running = source.version().to_owned();
    loop {
        let mut command = std::process::Command::new(source.path());
        command
            .args(std::env::args_os().skip(1))
            .env(INNER_ENV, "1");
        let mut startup = startup::Startup::for_image(&watched, source.path())?;
        command
            .env_remove(startup::READY_FILE)
            .env_remove(startup::READY_PARENT);
        if let Some(startup) = &startup {
            startup.configure(&mut command);
        }
        let mut child = source.spawn(&mut command).map_err(msg::host_wont_start)?;
        CHILD.store(child.id() as i32, Ordering::SeqCst);
        let born = std::time::Instant::now();
        let mut on_disk = fingerprint(&watched);
        // A change must hold still for two looks before it counts: a
        // download writing the file in place is many changes in a row,
        // and none of them is a generation yet.
        let mut seen: Option<(u64, i64, u64)> = None;
        let mut refused: Option<(u64, i64, u64)> = None;
        let mut swap: Option<(String, crate::runtime_image::HostImage)> = None;
        let mut term_at: Option<std::time::Instant> = None;
        let mut last_look = std::time::Instant::now();
        let status = loop {
            match crate::agent_process::child_exited(&child) {
                Ok(true) => {
                    CHILD.store(0, Ordering::SeqCst);
                    break child.wait().map_err(|error| error.to_string())?;
                }
                Ok(false) => {}
                Err(error) => {
                    CHILD.store(0, Ordering::SeqCst);
                    return Err(error.to_string());
                }
            }
            if let Some(pending) = &startup {
                if pending.acknowledged(child.id()) {
                    match pending.confirm() {
                        Ok(()) => startup = None,
                        Err(why) => eprintln!("could not confirm CLI service startup: {why}"),
                    }
                } else if pending.expired(born) {
                    let _ = child.kill();
                }
            }
            if let Some(t) = term_at
                && t.elapsed() >= SWAP_GRACE
            {
                let _ = child.kill();
            }
            std::thread::sleep(CHILD_POLL);
            if swap.is_some() || last_look.elapsed() < SELF_CHECK {
                continue;
            }
            last_look = std::time::Instant::now();
            if fingerprint(source.path()).is_none() {
                let stamp = jiff::Zoned::now().strftime("%Y-%m-%d %H:%M:%S").to_string();
                eprintln!("{}", msg::serve_copy_swept(&stamp));
                match take_generation(&watched) {
                    Ok(next) => {
                        swap = Some(next);
                        term_at = Some(std::time::Instant::now());
                        let pid = CHILD.load(Ordering::SeqCst);
                        if pid > 0 {
                            unsafe {
                                libc::kill(pid, libc::SIGTERM);
                            }
                        }
                    }
                    Err(why) => eprintln!("{}", msg::serve_swap_refused(&stamp, why)),
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
            match take_generation(&watched) {
                Ok((next, from)) => {
                    if from.generation() == source.generation() {
                        on_disk = Some(now);
                        seen = None;
                        continue;
                    }
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
            drop(startup);
            // Only our serve has stopped. Durable hosts keep their independent leases.
            let mut command = std::process::Command::new(from.path());
            command
                .args(std::env::args_os().skip(1))
                .env(SHIELD_FROM, &watched);
            let err = from.exec(&mut command);
            let stamp = jiff::Zoned::now().strftime("%Y-%m-%d %H:%M:%S").to_string();
            eprintln!("{}", msg::serve_keeper_stayed(&stamp, err));
            running = next;
            source = from;
            backoff = 1;
            continue;
        }
        if let Some(pending) = startup {
            if pending.rollback()? {
                drop(pending);
                let (_, previous) = take_generation(&watched)?;
                let mut command = std::process::Command::new(previous.path());
                command
                    .args(std::env::args_os().skip(1))
                    .env(SHIELD_FROM, &watched);
                return Err(previous.exec(&mut command).to_string());
            }
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
                (None, None) => msg::died_unreadably().to_owned(),
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
