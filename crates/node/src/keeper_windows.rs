//! Windows supervises the resident with owned runtime-image leases. MSI owns
//! installation replacement; there is no POSIX exec or CLI activation pointer.
use khor_catalog::msg;
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant, SystemTime},
};

pub const INNER_ENV: &str = "KHOR_SERVE_KEPT";
const PARENT: &str = "KHOR_SERVE_KEEPER_PID";
static STOP: AtomicBool = AtomicBool::new(false);

pub fn is_inner() -> bool {
    std::env::var_os(INNER_ENV).is_some()
}
pub(crate) fn parent_gone() -> bool {
    is_inner()
        && std::env::var(PARENT)
            .ok()
            .and_then(|s| s.parse().ok())
            .is_some_and(|pid| !crate::win_process::alive(pid))
}
pub(crate) fn on_network_fs(_path: &Path) -> bool {
    false
}
// Startup acknowledgement belongs to the Unix CLI pointer transaction. MSI
// activation is external; this resident still performs all normal initialization.
pub(crate) fn ready() -> Result<(), String> {
    Ok(())
}

unsafe extern "system" fn stop(event: u32) -> i32 {
    use windows_sys::Win32::System::Console::{CTRL_BREAK_EVENT, CTRL_C_EVENT, CTRL_CLOSE_EVENT};
    if matches!(event, CTRL_C_EVENT | CTRL_BREAK_EVENT | CTRL_CLOSE_EVENT) {
        STOP.store(true, Ordering::SeqCst);
        1
    } else {
        0
    }
}

fn fingerprint(path: &Path) -> Option<(PathBuf, SystemTime, u64)> {
    let selected = crate::updates::selected_cli_source(path).ok()?;
    let metadata = std::fs::metadata(&selected).ok()?;
    Some((selected, metadata.modified().ok()?, metadata.len()))
}

pub fn keep() -> Result<(), String> {
    let watched = crate::installed_exe()?;
    let mut image = crate::runtime_image::host_at(&crate::updates::selected_cli_source(&watched)?)?;
    // A GUI-launched background process need not have a console. With one,
    // Ctrl+C is handled by this owner rather than causing a respawn loop.
    unsafe {
        windows_sys::Win32::System::Console::SetConsoleCtrlHandler(Some(stop), 1);
    }
    let mut backoff = 1;
    loop {
        let mut command = std::process::Command::new(image.path());
        command
            .args(std::env::args_os().skip(1))
            .env(INNER_ENV, "1")
            .env(PARENT, std::process::id().to_string());
        let mut child = image.spawn(&mut command).map_err(msg::host_wont_start)?;
        let born = Instant::now();
        let mut installed = fingerprint(&watched);
        let mut seen = None;
        let mut looked = Instant::now();
        let mut handed_over = false;
        let status = loop {
            if STOP.load(Ordering::SeqCst) {
                let _ = child.kill();
                child.wait().map_err(|e| e.to_string())?;
                return Ok(());
            }
            if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
                break status;
            }
            std::thread::sleep(Duration::from_millis(200));
            if looked.elapsed() < Duration::from_secs(2) {
                continue;
            }
            looked = Instant::now();
            let current = fingerprint(&watched);
            if current.is_none() || current == installed {
                seen = None;
                continue;
            }
            if current != seen {
                seen = current;
                continue;
            }
            match crate::runtime_image::host_at(&crate::updates::selected_cli_source(&watched)?) {
                Ok(next) if next.generation() != image.generation() => {
                    // Retire only the resident. Durable PTY/agent hosts own
                    // independent image leases and remain available to rejoin.
                    child.kill().map_err(|e| e.to_string())?;
                    image = next;
                    backoff = 1;
                    handed_over = true;
                    break child.wait().map_err(|e| e.to_string())?;
                }
                Ok(_) => {
                    installed = current;
                    seen = None;
                }
                Err(error) => eprintln!("resident update deferred: {error}"),
            }
        };
        // An installation handover takes the same path as a restart; a clean
        // resident shutdown stays stopped rather than being resurrected.
        if handed_over {
            continue;
        }
        if status.success() {
            return Ok(());
        }
        if born.elapsed() >= Duration::from_secs(60) {
            backoff = 1;
        }
        if !status.success() {
            let stamp = jiff::Zoned::now().strftime("%Y-%m-%d %H:%M:%S").to_string();
            let how = msg::died_with_code(status.code().unwrap_or(1));
            eprintln!("{}", msg::serve_died_restarting(&stamp, &how, backoff));
            std::thread::sleep(Duration::from_secs(backoff));
            backoff = (backoff * 2).min(60);
        }
    }
}
