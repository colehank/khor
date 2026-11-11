//! The public command locates the application, not a launcher on PATH.
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use khor_catalog::msg;
use khor_node::updates::{self, Component};

const RELEASES: &str = "https://github.com/colehank/khor/releases";

#[cfg(target_os = "macos")]
const CLI_IN_APP: &str = "Contents/MacOS/khor";
#[cfg(target_os = "macos")]
const PROGRAM_IN_APP: &str = "Contents/MacOS/khor-gui";

#[cfg(target_os = "macos")]
fn bundle_of_cli(cli: &Path) -> Option<PathBuf> {
    let cli = std::fs::canonicalize(cli).ok()?;
    let app = cli.ancestors().nth(3)?;
    (cli.ends_with(CLI_IN_APP) && app.extension().is_some_and(|ext| ext == "app"))
        .then(|| app.to_path_buf())
}

#[cfg(not(target_os = "macos"))]
fn packaged_gui(cli: &Path) -> Option<PathBuf> {
    let cli = std::fs::canonicalize(cli).ok()?;
    let directory = cli.parent()?;
    #[cfg(not(target_os = "windows"))]
    if cli != Path::new("/usr/bin/khor") {
        return None;
    }
    #[cfg(target_os = "windows")]
    let (program, marker) = (
        directory.join("khor-gui.exe"),
        directory.join("cli-owner.json"),
    );
    #[cfg(not(target_os = "windows"))]
    let (program, marker) = (
        directory.join("khor-gui"),
        PathBuf::from("/usr/share/khor/cli-owner.json"),
    );
    let ownership: serde_json::Value = serde_json::from_slice(&std::fs::read(marker).ok()?).ok()?;
    if ownership["schema"] != 1 || ownership["component"] != "desktop-app" {
        return None;
    }
    let app = updates::inspect(&program).ok()?;
    let command = updates::inspect(&cli).ok()?;
    (app.component == Component::DesktopApp
        && app.build == command.build
        && app.platform == command.platform
        && app.contract == command.contract)
        .then_some(program)
}

/// App-owned commands follow their application or system package installer.
pub(super) fn owns_cli(cli: &Path) -> bool {
    #[cfg(target_os = "macos")]
    {
        bundle_of_cli(cli).is_some_and(|app| app.join(PROGRAM_IN_APP).is_file())
    }
    #[cfg(not(target_os = "macos"))]
    {
        packaged_gui(cli).is_some()
    }
}

fn installed_apps(cli: &Path) -> Vec<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        let mut apps = Vec::new();
        if let Some(app) = bundle_of_cli(cli) {
            apps.push(app);
        }
        apps.push(PathBuf::from("/Applications/Khor.app"));
        if let Some(home) = std::env::var_os("HOME") {
            apps.push(PathBuf::from(home).join("Applications/Khor.app"));
        }
        apps
    }
    #[cfg(target_os = "windows")]
    {
        let mut apps = Vec::new();
        if let Some(dir) = cli.parent() {
            apps.push(dir.join("khor-gui.exe"));
        }
        for (key, folder) in [("LOCALAPPDATA", "Programs/Khor"), ("ProgramFiles", "Khor")] {
            if let Some(root) = std::env::var_os(key) {
                apps.push(PathBuf::from(root).join(folder).join("khor-gui.exe"));
            }
        }
        apps
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        let mut apps = Vec::new();
        if let Ok(cli) = std::fs::canonicalize(cli)
            && let Some(directory) = cli.parent()
        {
            apps.push(directory.join("khor-gui"));
        }
        if let Some(home) = std::env::var_os("HOME") {
            apps.push(PathBuf::from(home).join(".local/lib/khor/khor-gui"));
        }
        apps.push(PathBuf::from("/usr/lib/khor/khor-gui"));
        apps.push(PathBuf::from("/usr/bin/khor-gui"));
        apps.push(PathBuf::from("/opt/Khor/khor-gui"));
        apps
    }
}

pub(super) fn launch() -> Result<(), String> {
    let cli = khor_node::installed_exe()?;
    let app = installed_apps(&cli)
        .into_iter()
        .find(|app| {
            #[cfg(target_os = "macos")]
            let program = app.join(PROGRAM_IN_APP);
            #[cfg(not(target_os = "macos"))]
            let program = app;
            program.is_file()
                && updates::inspect(&program)
                    .is_ok_and(|info| info.component == Component::DesktopApp)
        })
        .ok_or_else(|| msg::gui_not_installed(RELEASES))?;
    #[cfg(target_os = "macos")]
    {
        // Launch Services reactivates the running instance and provides the
        // same environment as Finder. Do not execute the bundle as a ghost.
        let status = Command::new("/usr/bin/open")
            .arg(&app)
            .stdin(Stdio::null())
            .status()
            .map_err(msg::gui_wont_start)?;
        if !status.success() {
            return Err(msg::gui_wont_start(status));
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let mut command = Command::new(app);
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        #[cfg(unix)]
        unsafe {
            use std::os::unix::process::CommandExt;
            command.pre_exec(|| {
                if libc::setsid() == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        command.spawn().map_err(msg::gui_wont_start)?;
    }
    Ok(())
}
