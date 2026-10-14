//! Per-machine agent integration. The report is read from vendor configuration,
//! never a saved UI boolean. Codex uses native title settings, not a launcher.
use crate::{
    Node,
    adaptor::{self, claude},
};
use khor_catalog::msg;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
use toml_edit::{DocumentMut, Item};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Agent {
    Claude,
    Codex,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Read,
    Enable(Agent),
    Disable(Agent),
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub agent: Agent,
    /// absent / automatic / configured / changed / error
    pub state: String,
    pub managed: bool,
    pub path: String,
    pub error: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub entries: Vec<Entry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TitleBackup {
    /// Only this value is ours to restore; never back up credentials or the file.
    original: Option<String>,
    had_tui: bool,
    target: PathBuf,
    // Written before replacing vendor config; unfinished installs prove nothing.
    #[serde(default)]
    pending: bool,
    #[serde(default)]
    activated_ms: Option<i64>,
}

/// Only the title setting's observed continuity, never vendor config or secrets.
/// Persisted so a GUI restart does not confuse a model edit with enabling titles.
#[derive(Serialize, Deserialize)]
struct TitleObservation {
    target: PathBuf,
    enabled: bool,
    since_ms: i64,
}
fn observation_path(root: &Path) -> PathBuf {
    root.join(".khor/codex-title-observed.json")
}
fn modified_ms(path: &Path) -> Option<i64> {
    fs::metadata(path)
        .ok()?
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_millis()
        .try_into()
        .ok()
}
fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |t| t.as_millis() as i64)
}

/// Caller holds recognition.lock, shared by configuration writers and observers.
fn observe_title(root: &Path, path: &Path, enabled: bool) -> Option<i64> {
    let target = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let saved = observation_path(root);
    let text = read(&saved).ok()?;
    let previous = text
        .as_deref()
        .map(serde_json::from_str::<TitleObservation>)
        .transpose()
        .ok()?;
    if let Some(previous) = &previous
        && previous.target == target
        && previous.enabled == enabled
    {
        return enabled.then_some(previous.since_ms);
    }
    let mut since_ms = modified_ms(path).unwrap_or_else(now_ms);
    if let Some(previous) = &previous {
        // A normal setting change uses its own write time, so clients launched
        // after that change can bind even if discovery runs a few seconds later.
        // Changed targets/backdated files cannot inherit earlier activation.
        if previous.target != target || since_ms <= previous.since_ms {
            since_ms = since_ms.max(now_ms());
        }
    } else if enabled {
        if let Some(backup) = read(&backup_path(root))
            .ok()
            .flatten()
            .and_then(|s| serde_json::from_str::<TitleBackup>(&s).ok())
            .filter(|b| b.target == target && !b.pending)
        {
            // Pre-observation releases saved the restoration record once, at
            // installation. Migrate that evidence; later model edits leave it alone.
            since_ms = backup
                .activated_ms
                .or_else(|| modified_ms(&backup_path(root)))?;
        }
    }
    let observation = TitleObservation {
        target,
        enabled,
        since_ms,
    };
    let record = serde_json::to_string(&observation).ok()?;
    replace(&saved, text.as_deref(), &record).ok()?;
    enabled.then_some(since_ms)
}

fn title_active_since(root: &Path, path: &Path) -> Option<i64> {
    fs::create_dir_all(root.join(".khor")).ok()?;
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(root.join(".khor/recognition.lock"))
        .ok()?;
    lock.try_lock().ok()?;
    let enabled = read(path)
        .ok()
        .flatten()
        .and_then(|s| parse(&s).ok())
        .is_some_and(|doc| id_title(&doc) && doc.get("profile").is_none());
    observe_title(root, path, enabled)
}
fn backup_path(root: &Path) -> PathBuf {
    root.join(".khor/codex-title.json")
}
fn config_path(home: &Path) -> PathBuf {
    home.join(".codex/config.toml")
}
// Follow an existing symlink and retain it, including when the containing
// directory itself is linked. Refuse dangling links instead of replacing them.
fn write_target(path: &Path) -> Result<PathBuf, String> {
    match fs::symlink_metadata(path) {
        Ok(_) => fs::canonicalize(path).map_err(|e| e.to_string()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let parent = path.parent().ok_or_else(msg::recognition_changed)?;
            fs::canonicalize(parent)
                .map(|p| p.join(path.file_name().unwrap()))
                .map_err(|e| e.to_string())
        }
        Err(e) => Err(e.to_string()),
    }
}
fn read(path: &Path) -> Result<Option<String>, String> {
    match fs::read_to_string(path) {
        Ok(s) => Ok(Some(s)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}
fn parse(text: &str) -> Result<DocumentMut, String> {
    text.parse::<DocumentMut>().map_err(|e| e.to_string())
}
fn title(doc: &DocumentMut) -> Option<&Item> {
    doc.get("tui")?.get("terminal_title")
}
fn id_title(doc: &DocumentMut) -> bool {
    title(doc)
        .and_then(Item::as_array)
        .is_some_and(|a| a.len() == 1 && a.get(0).and_then(|v| v.as_str()) == Some("thread-id"))
}
/// Read every sweep: changing native settings must invalidate an earlier binding.
#[cfg(test)]
fn native_title(home: &Path) -> bool {
    read(&config_path(home))
        .ok()
        .flatten()
        .and_then(|s| parse(&s).ok())
        .is_some_and(|d| id_title(&d))
}

/// A project/profile override may disable native title updates. Fail closed even
/// for a project configuration that Codex might consider untrusted.
pub(crate) fn native_title_for(
    root: &Path,
    home: &Path,
    cwd: Option<&Path>,
    started_ms: i64,
    args: &[std::ffi::OsString],
) -> bool {
    if args.iter().any(|a| {
        let s = a.to_string_lossy();
        s.starts_with("-p") || s == "--profile" || s.starts_with("--profile=")
    }) {
        return false;
    }
    if crate::codex_process::config_values(args).iter().any(|v| {
        v.split_once('=')
            .is_some_and(|(k, _)| k.trim().trim_matches(['\'', '"']) == "profile")
    }) {
        return false;
    }
    let path = config_path(home);
    // Unrelated TOML edits preserve activation. A newly enabled setting still
    // cannot authorize a stale title left in a terminal by a previous client.
    // Process start clocks have second precision on some platforms.
    if title_active_since(root, &path).is_none_or(|since| since / 1000 > started_ms / 1000) {
        return false;
    }

    if let Some(cwd) = cwd {
        for dir in cwd.ancestors() {
            if dir == home {
                continue;
            }
            match read(&dir.join(".codex/config.toml")) {
                Ok(Some(s)) => match parse(&s) {
                    Ok(d) => {
                        if d.get("profile").is_some() || (title(&d).is_some() && !id_title(&d)) {
                            return false;
                        }
                    }
                    Err(_) => return false,
                },
                Err(_) => return false,
                _ => {}
            }
        }
    }
    true
}

/// Preserve mode and unrelated TOML formatting. Check the read version immediately
/// before replacing it; the operation lock serializes khor's own writers.
fn replace(path: &Path, expected: Option<&str>, text: &str) -> Result<(), String> {
    if read(path)?.as_deref() != expected {
        return Err(msg::recognition_changed().to_owned());
    }
    let parent = path.parent().ok_or_else(msg::recognition_changed)?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let tmp = path.with_extension(format!("khor-{}.tmp", std::process::id()));
    let mut opts = fs::OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts.open(&tmp).map_err(|e| e.to_string())?;
    let result = (|| {
        use std::io::Write;
        f.write_all(text.as_bytes()).map_err(|e| e.to_string())?;
        if let Ok(m) = fs::metadata(path) {
            f.set_permissions(m.permissions())
                .map_err(|e| e.to_string())?;
        }
        f.sync_all().map_err(|e| e.to_string())?;
        if read(path)?.as_deref() != expected {
            return Err(msg::recognition_changed().to_owned());
        }
        drop(f);
        fs::rename(&tmp, path).map_err(|e| e.to_string())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}
fn codex_change(root: &Path, home: &Path, enable: bool) -> Result<(), String> {
    let path = write_target(&config_path(home))?;
    let saved = backup_path(root);
    let text = read(&path)?;
    let mut doc = parse(text.as_deref().unwrap_or(""))?;
    let backup = read(&saved)?;
    if enable {
        if id_title(&doc) {
            observe_title(root, &path, doc.get("profile").is_none());
            return Ok(());
        }
        // A changed setting belongs to the user. First explicitly detach the old
        // integration; never silently replace its restoration record.
        if backup.is_some() {
            return Err(msg::recognition_changed().to_owned());
        }
        if !home.join(".codex").is_dir() {
            return Err(msg::recognition_missing().to_owned());
        }
        if doc.get("tui").is_some_and(|v| !v.is_table_like()) {
            return Err(msg::recognition_changed().to_owned());
        }
        observe_title(root, &path, false);
        let previous = TitleBackup {
            original: title(&doc).map(ToString::to_string),
            had_tui: doc.get("tui").is_some(),
            target: path.clone(),
            pending: true,
            activated_ms: None,
        };
        let record = serde_json::to_string(&previous).map_err(|e| e.to_string())?;
        // Save restoration before writing. A crash between writes is recoverable:
        // disable leaves the untouched original in place and clears this record.
        replace(&saved, None, &record)?;
        doc["tui"]["terminal_title"] = toml_edit::value(toml_edit::Array::from_iter(["thread-id"]));
        replace(&path, text.as_deref(), &doc.to_string())?;
        let previous = TitleBackup {
            pending: false,
            activated_ms: modified_ms(&path),
            ..previous
        };
        replace(
            &saved,
            Some(&record),
            &serde_json::to_string(&previous).map_err(|e| e.to_string())?,
        )?;
        observe_title(root, &path, doc.get("profile").is_none());
    } else if let Some(backup) = backup {
        let previous: TitleBackup = serde_json::from_str(&backup).map_err(|e| e.to_string())?;
        if previous.target != path {
            return Err(msg::recognition_changed().to_owned());
        }
        if id_title(&doc) {
            if let Some(original) = previous.original {
                let mut prior = parse(&format!("value ={original}"))?;
                doc["tui"]["terminal_title"] =
                    prior.remove("value").ok_or_else(msg::recognition_changed)?;
            } else if let Some(tui) = doc.get_mut("tui").and_then(Item::as_table_like_mut) {
                tui.remove("terminal_title");
            }
            if !previous.had_tui
                && doc
                    .get("tui")
                    .and_then(Item::as_table_like)
                    .is_some_and(|t| t.is_empty())
            {
                doc.remove("tui");
            }
            replace(&path, text.as_deref(), &doc.to_string())?;
        }
        // If the user changed it meanwhile, their new value is left alone.
        fs::remove_file(saved).map_err(|e| e.to_string())?;
        observe_title(root, &path, id_title(&doc) && doc.get("profile").is_none());
    }
    Ok(())
}
fn entry(root: &Path, home: &Path, agent: Agent) -> Entry {
    let path = match agent {
        Agent::Claude => home.join(".claude/settings.json"),
        Agent::Codex => config_path(home),
    };
    let mut e = Entry {
        agent: agent.clone(),
        state: "absent".into(),
        managed: false,
        path: path.display().to_string(),
        error: None,
    };
    if !path.parent().is_some_and(Path::is_dir) {
        return e;
    }
    let result = (|| -> Result<(), String> {
        match agent {
            Agent::Claude => {
                let report = claude::hooks_report(home)?;
                e.managed = report
                    .events
                    .iter()
                    .any(|(_, s)| !matches!(s, claude::Installed::Missing));
                e.state = if report
                    .events
                    .iter()
                    .all(|(_, s)| matches!(s, claude::Installed::Here))
                {
                    "configured"
                } else if e.managed {
                    "changed"
                } else {
                    "automatic"
                }
                .into();
            }
            Agent::Codex => {
                let doc = parse(read(&path)?.as_deref().unwrap_or(""))?;
                e.managed = read(&backup_path(root))?.is_some();
                e.state = if id_title(&doc) {
                    "configured"
                } else if e.managed {
                    "changed"
                } else {
                    "automatic"
                }
                .into();
            }
        }
        Ok(())
    })();
    if let Err(error) = result {
        e.state = "error".into();
        e.error = Some(error);
    }
    e
}
impl Node {
    pub(crate) fn recognition_here(&self, action: Action) -> Result<Report, String> {
        if !self.hosts() {
            return Err(msg::recognition_computer().to_owned());
        }
        let home = adaptor::vendor_home(&self.root);
        if action != Action::Read {
            fs::create_dir_all(self.root.join(".khor")).map_err(|e| e.to_string())?;
            let lock = fs::OpenOptions::new()
                .create(true)
                .truncate(false)
                .read(true)
                .write(true)
                .open(self.root.join(".khor/recognition.lock"))
                .map_err(|e| e.to_string())?;
            lock.try_lock()
                .map_err(|_| msg::recognition_busy().to_owned())?;
            match action {
                Action::Enable(Agent::Claude) => {
                    if !home.join(".claude").is_dir() {
                        return Err(msg::recognition_missing().to_owned());
                    }
                    self.install_hooks()?;
                }
                Action::Disable(Agent::Claude) => {
                    self.uninstall_hooks()?;
                }
                Action::Enable(Agent::Codex) => codex_change(&self.root, &home, true)?,
                Action::Disable(Agent::Codex) => codex_change(&self.root, &home, false)?,
                Action::Read => {}
            }
        }
        Ok(Report {
            entries: vec![
                entry(&self.root, &home, Agent::Claude),
                entry(&self.root, &home, Agent::Codex),
            ],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Scratch(PathBuf);
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn scratch(tag: &str) -> Scratch {
        let p = std::env::temp_dir().join(format!("khor-recognition-{tag}-{}", std::process::id()));
        fs::create_dir_all(p.join(".codex")).unwrap();
        Scratch(p)
    }
    #[test]
    fn title_roundtrip_preserves_other_edits_and_original_preference() {
        let s = scratch("roundtrip");
        let p = config_path(&s.0);
        fs::write(&p,"# mine\nmodel = 'my-model'\n[tui]\nterminal_title = ['project', 'spinner'] # my title\n").unwrap();
        codex_change(&s.0, &s.0, true).unwrap();
        assert!(native_title(&s.0));
        codex_change(&s.0, &s.0, true).unwrap();
        let installed = fs::read_to_string(&p).unwrap();
        fs::write(&p, installed.replace("my-model", "new-model")).unwrap();
        codex_change(&s.0, &s.0, false).unwrap();
        let restored = fs::read_to_string(&p).unwrap();
        assert!(restored.contains("new-model"));
        assert!(restored.contains("['project', 'spinner'] # my title"));
        assert!(restored.starts_with("# mine"));
        codex_change(&s.0, &s.0, false).unwrap();
        assert_eq!(read(&p).unwrap().unwrap(), restored);
    }
    #[test]
    fn user_title_changes_win_and_existing_configuration_is_not_owned() {
        let s = scratch("changed");
        let p = config_path(&s.0);
        fs::write(&p, "[tui]\nterminal_title=['thread-id']\n").unwrap();
        codex_change(&s.0, &s.0, true).unwrap();
        assert!(!backup_path(&s.0).exists());
        codex_change(&s.0, &s.0, false).unwrap();
        assert!(native_title(&s.0));
        fs::write(&p, "[tui]\nterminal_title=['project']\n").unwrap();
        codex_change(&s.0, &s.0, true).unwrap();
        fs::write(&p, "[tui]\nterminal_title=[]\n").unwrap();
        assert!(codex_change(&s.0, &s.0, true).is_err());
        codex_change(&s.0, &s.0, false).unwrap();
        assert_eq!(read(&p).unwrap().unwrap(), "[tui]\nterminal_title=[]\n");
    }
    #[test]
    fn malformed_settings_are_never_overwritten_and_no_title_is_restored_as_absence() {
        let s = scratch("invalid");
        let p = config_path(&s.0);
        fs::write(&p, "[tui\n").unwrap();
        assert!(codex_change(&s.0, &s.0, true).is_err());
        assert!(!backup_path(&s.0).exists());
        fs::write(&p, "model='x'\n").unwrap();
        codex_change(&s.0, &s.0, true).unwrap();
        codex_change(&s.0, &s.0, false).unwrap();
        assert_eq!(read(&p).unwrap().unwrap(), "model='x'\n");
    }
    #[test]
    fn native_selection_requires_active_unoverridden_configuration() {
        use std::ffi::OsString;
        let s = scratch("overrides");
        let p = config_path(&s.0);
        let project = s.0.join("project");
        fs::create_dir_all(project.join(".codex")).unwrap();
        fs::write(&p, "[tui]\nterminal_title=['thread-id']\n").unwrap();
        let active =
            |args: Vec<OsString>| native_title_for(&s.0, &s.0, Some(&project), i64::MAX, &args);
        assert!(active(vec![]));
        assert!(
            !native_title_for(&s.0, &s.0, Some(&project), 1, &[]),
            "existing TUI has not loaded new config"
        );
        for args in [
            vec!["-p", "work"],
            vec!["-pwork"],
            vec!["--profile=work"],
            vec!["-c", "profile='work'"],
        ] {
            assert!(!active(args.into_iter().map(OsString::from).collect()));
        }
        fs::write(
            project.join(".codex/config.toml"),
            "[tui]\nterminal_title=[]\n",
        )
        .unwrap();
        assert!(!active(vec![]));
        fs::write(
            project.join(".codex/config.toml"),
            "[tui]\nterminal_title=['thread-id']\n",
        )
        .unwrap();
        assert!(active(vec![]));
        fs::write(&p, "profile='work'\n[tui]\nterminal_title=['thread-id']\n").unwrap();
        assert!(!active(vec![]));
        assert!(!crate::codex_process::selection_title_or(
            &["-c".into(), "tui.terminal_title=[]".into()],
            true
        ));
    }

    #[test]
    fn unrelated_edits_preserve_native_title_activation_across_readers() {
        let s = scratch("title-continuity");
        let p = config_path(&s.0);
        fs::write(&p, "model='first'\n[tui]\nterminal_title=['thread-id']\n").unwrap();
        let started = modified_ms(&p).unwrap() + 1;
        assert!(native_title_for(&s.0, &s.0, None, started, &[]));
        // Codex rewrites this file when changing model/status-line preferences.
        // The rewrite happened after the TUI started; the ID setting did not.
        fs::write(&p, "model='second'\n[tui]\nterminal_title=['thread-id']\nstatus_line=['model-with-reasoning']\n").unwrap();
        fs::File::options()
            .write(true)
            .open(&p)
            .unwrap()
            .set_modified(
                std::time::UNIX_EPOCH + std::time::Duration::from_millis((started + 10_000) as u64),
            )
            .unwrap();
        assert!(
            native_title_for(&s.0, &s.0, None, started, &[]),
            "a model edit must not invalidate an active title setting"
        );
        // Each call reads the durable evidence afresh, including after GUI restart.
        assert!(native_title_for(&s.0, &s.0, None, started, &[]));
        fs::write(&p, "[tui]\nterminal_title=[]\n").unwrap();
        assert!(!native_title_for(&s.0, &s.0, None, started, &[]));
        fs::write(&p, "[tui]\nterminal_title=['thread-id']\n").unwrap();
        fs::File::options()
            .write(true)
            .open(&p)
            .unwrap()
            .set_modified(
                std::time::UNIX_EPOCH + std::time::Duration::from_millis((started + 20_000) as u64),
            )
            .unwrap();
        assert!(
            !native_title_for(&s.0, &s.0, None, started, &[]),
            "turning integration back on must not trust an older TUI title"
        );
        assert!(native_title_for(&s.0, &s.0, None, started + 30_000, &[]));
    }

    fn set_modified(path: &Path, ms: i64) {
        fs::File::options()
            .write(true)
            .open(path)
            .unwrap()
            .set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_millis(ms as u64))
            .unwrap();
    }

    #[test]
    fn first_reader_uses_installation_evidence_after_a_model_edit() {
        let s = scratch("installed-title");
        let p = config_path(&s.0);
        fs::write(&p, "model='first'\n").unwrap();
        codex_change(&s.0, &s.0, true).unwrap();
        let installed: TitleBackup =
            serde_json::from_str(&read(&backup_path(&s.0)).unwrap().unwrap()).unwrap();
        assert!(!installed.pending);
        assert!(installed.activated_ms.is_some());
        let started = installed.activated_ms.unwrap() + 2_000;
        // The first discovery may be after the unrelated edit, or after upgrade.
        fs::remove_file(observation_path(&s.0)).unwrap();
        fs::write(&p, "model='second'\n[tui]\nterminal_title=['thread-id']\n").unwrap();
        set_modified(&p, started + 10_000);
        assert!(native_title_for(&s.0, &s.0, None, started, &[]));

        // The actual legacy installation record has no new fields. Its timestamp
        // stayed at installation while Codex later rewrote unrelated preferences.
        fs::remove_file(observation_path(&s.0)).unwrap();
        fs::write(
            backup_path(&s.0),
            serde_json::json!({
                "original": null, "had_tui": true, "target": fs::canonicalize(&p).unwrap(),
            })
            .to_string(),
        )
        .unwrap();
        set_modified(&backup_path(&s.0), started - 2_000);
        assert!(native_title_for(&s.0, &s.0, None, started, &[]));
        assert!(!native_title_for(&s.0, &s.0, None, started - 10_000, &[]));
    }

    #[test]
    fn incomplete_installation_and_late_first_configuration_do_not_bind_old_clients() {
        let s = scratch("pending-title");
        let p = config_path(&s.0);
        fs::write(&p, "[tui]\nterminal_title=['thread-id']\n").unwrap();
        let configured = modified_ms(&p).unwrap();
        fs::create_dir_all(s.0.join(".khor")).unwrap();
        fs::write(
            backup_path(&s.0),
            serde_json::to_string(&TitleBackup {
                original: None,
                had_tui: true,
                target: fs::canonicalize(&p).unwrap(),
                pending: true,
                activated_ms: None,
            })
            .unwrap(),
        )
        .unwrap();
        set_modified(&backup_path(&s.0), configured - 20_000);
        assert!(!native_title_for(
            &s.0,
            &s.0,
            None,
            configured - 10_000,
            &[]
        ));
        assert!(native_title_for(&s.0, &s.0, None, configured + 2_000, &[]));
    }

    #[test]
    fn client_started_after_reenabling_can_bind_before_the_next_discovery() {
        let s = scratch("title-reenable-before-poll");
        let p = config_path(&s.0);
        let base = now_ms() - 30_000;
        fs::write(&p, "[tui]\nterminal_title=[]\n").unwrap();
        set_modified(&p, base);
        assert!(!native_title_for(&s.0, &s.0, None, base + 10_000, &[]));
        fs::write(&p, "[tui]\nterminal_title=['thread-id']\n").unwrap();
        set_modified(&p, base + 15_000);
        // Discovery is now, but this TUI loaded the already-enabled setting earlier.
        assert!(native_title_for(&s.0, &s.0, None, base + 20_000, &[]));
        assert!(!native_title_for(&s.0, &s.0, None, base + 10_000, &[]));
    }

    #[test]
    fn missing_or_invalid_config_clears_continuity_and_evidence_stays_in_node_root() {
        let s = scratch("invalidates-title");
        let root = s.0.join("node");
        let p = config_path(&s.0);
        fs::write(&p, "[tui]\nterminal_title=['thread-id']\n").unwrap();
        let started = now_ms() - 10_000;
        set_modified(&p, started - 10_000);
        assert!(native_title_for(&root, &s.0, None, started, &[]));
        assert!(
            !s.0.join(".khor").exists(),
            "vendor home must not receive khor evidence"
        );
        for text in [
            "[tui\n",
            "profile='work'\n[tui]\nterminal_title=['thread-id']\n",
        ] {
            fs::write(&p, text).unwrap();
            assert!(!native_title_for(&root, &s.0, None, started, &[]));
            fs::write(&p, "[tui]\nterminal_title=['thread-id']\n").unwrap();
            set_modified(&p, started - 10_000);
            assert!(!native_title_for(&root, &s.0, None, started, &[]));
            assert!(native_title_for(&root, &s.0, None, now_ms() + 2_000, &[]));
        }
        fs::remove_file(&p).unwrap();
        assert!(!native_title_for(&root, &s.0, None, now_ms() + 2_000, &[]));
        fs::write(&p, "[tui]\nterminal_title=['thread-id']\n").unwrap();
        set_modified(&p, started - 10_000);
        assert!(!native_title_for(&root, &s.0, None, started, &[]));
    }

    #[cfg(unix)]
    #[test]
    fn retargeted_config_link_does_not_inherit_title_activation() {
        let s = scratch("title-retarget");
        let p = config_path(&s.0);
        let original = s.0.join("original.toml");
        let other = s.0.join("other.toml");
        let started = now_ms() - 10_000;
        for path in [&original, &other] {
            fs::write(path, "[tui]\nterminal_title=['thread-id']\n").unwrap();
            set_modified(path, started - 10_000);
        }
        std::os::unix::fs::symlink(&original, &p).unwrap();
        assert!(native_title_for(&s.0, &s.0, None, started, &[]));
        fs::remove_file(&p).unwrap();
        std::os::unix::fs::symlink(&other, &p).unwrap();
        assert!(!native_title_for(&s.0, &s.0, None, started, &[]));
    }
    #[cfg(unix)]
    #[test]
    fn symlink_and_permissions_survive_but_retargeted_links_are_not_restored() {
        use std::os::unix::{fs::PermissionsExt, fs::symlink};
        let s = scratch("symlink");
        let target = s.0.join("dotfiles.toml");
        let link = config_path(&s.0);
        fs::write(&target, "model='mine'\n").unwrap();
        fs::set_permissions(&target, fs::Permissions::from_mode(0o640)).unwrap();
        symlink(&target, &link).unwrap();
        codex_change(&s.0, &s.0, true).unwrap();
        assert!(fs::symlink_metadata(&link).unwrap().is_symlink());
        assert_eq!(
            fs::metadata(&target).unwrap().permissions().mode() & 0o777,
            0o640
        );
        let other = s.0.join("other.toml");
        fs::write(&other, "[tui]\nterminal_title=['thread-id']\n").unwrap();
        fs::remove_file(&link).unwrap();
        symlink(&other, &link).unwrap();
        assert!(codex_change(&s.0, &s.0, false).is_err());
        fs::remove_file(&link).unwrap();
        symlink(&target, &link).unwrap();
        codex_change(&s.0, &s.0, false).unwrap();
        assert_eq!(read(&target).unwrap().unwrap(), "model='mine'\n");
    }
}
