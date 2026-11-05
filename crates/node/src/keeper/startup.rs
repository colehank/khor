use khor_update::{CliInstallation, Platform};
use std::{
    fs,
    path::Path,
    process::Command,
    time::{Duration, Instant},
};

pub(super) const READY_FILE: &str = "KHOR_SERVE_READY_FILE";
pub(super) const READY_PARENT: &str = "KHOR_SERVE_READY_PARENT";
const STARTUP_LIMIT: Duration = Duration::from_secs(30);

pub(super) struct Startup {
    installation: CliInstallation,
    program: std::path::PathBuf,
    directory: tempfile::TempDir,
    can_rollback: bool,
}

impl Startup {
    pub(super) fn for_image(source: &Path, program: &Path) -> Result<Option<Self>, String> {
        let selected = crate::updates::selected_cli_source(source)?;
        let Some(installation) = CliInstallation::detect(&selected, Platform::current()?)? else {
            return Ok(None);
        };
        if !installation.service_start_pending(program)? {
            return Ok(None);
        }
        let can_rollback = installation
            .receipt()?
            .is_some_and(|r| r.previous.is_some());
        let runtime = khor_runtime::local_directory().map_err(|e| e.to_string())?;
        let directory = tempfile::tempdir_in(runtime).map_err(|e| e.to_string())?;
        Ok(Some(Self {
            installation,
            program: program.into(),
            directory,
            can_rollback,
        }))
    }

    pub(super) fn configure(&self, command: &mut Command) {
        command
            .env(READY_FILE, self.directory.path().join("ready"))
            .env(READY_PARENT, std::process::id().to_string());
    }

    pub(super) fn acknowledged(&self, child: u32) -> bool {
        fs::read_to_string(self.directory.path().join("ready"))
            .is_ok_and(|reply| reply == child.to_string())
    }

    pub(super) fn expired(&self, born: Instant) -> bool {
        self.can_rollback && born.elapsed() >= STARTUP_LIMIT
    }

    pub(super) fn confirm(&self) -> Result<(), String> {
        self.installation
            .finish_service_start(&self.program, true)
            .map(|_| ())
    }

    pub(super) fn rollback(&self) -> Result<bool, String> {
        self.installation.finish_service_start(&self.program, false)
    }
}

// Emitted only after the real resident has initialized its listeners and services.
pub(crate) fn ready() -> Result<(), String> {
    if !super::is_inner() {
        return Ok(());
    }
    // Hosted tasks can inherit environment variables; only the keeper's direct child acknowledges it.
    if std::env::var(READY_PARENT)
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        != Some(unsafe { libc::getppid() } as u32)
    {
        return Ok(());
    }
    if let Some(path) = std::env::var_os(READY_FILE) {
        fs::write(path, std::process::id().to_string()).map_err(|e| e.to_string())?;
    }
    Ok(())
}
