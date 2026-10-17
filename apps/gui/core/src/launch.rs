//! Named startup commands, local to the controlling khor store and independent
//! of the machine they are used on. Reading never executes a command.
use khor_catalog::msg;
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
};
use ts_rs::TS;

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LaunchPreset {
    pub id: String,
    pub name: String,
    pub command: String,
}
static WRITE: Mutex<()> = Mutex::new(());
fn path(root: &Path) -> PathBuf {
    root.join(".khor").join("launch-presets.json")
}
pub fn list(root: &Path) -> Result<Vec<LaunchPreset>, String> {
    match std::fs::read(path(root)) {
        Ok(bytes) => {
            serde_json::from_slice(&bytes).map_err(|e| msg::cant_read(path(root).display(), e))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(msg::cant_read(path(root).display(), e)),
    }
}
fn write(root: &Path, rows: &[LaunchPreset]) -> Result<(), String> {
    let p = path(root);
    let dir = p.parent().unwrap();
    std::fs::create_dir_all(dir).map_err(|e| msg::cant_write(dir.display(), e))?;
    let tmp = dir.join(format!(
        ".launch-presets-{}-{}.tmp",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let result = (|| {
        let bytes = serde_json::to_vec_pretty(rows).map_err(|e| e.to_string())?;
        let mut file = options
            .open(&tmp)
            .map_err(|e| msg::cant_write(tmp.display(), e))?;
        file.write_all(&bytes)
            .and_then(|()| file.sync_all())
            .map_err(|e| msg::cant_write(tmp.display(), e))?;
        std::fs::rename(&tmp, &p).map_err(|e| msg::cant_write(p.display(), e))
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(tmp);
    }
    result
}
pub fn save(root: &Path, preset: LaunchPreset) -> Result<Vec<LaunchPreset>, String> {
    let _held = WRITE.lock().map_err(|e| e.to_string())?;
    if preset.id.is_empty() || preset.name.trim().is_empty() || preset.name.len() > 400 {
        return Err(msg::launch_preset_name().into());
    }
    if preset.command.trim().is_empty()
        || preset.command.contains('\0')
        || preset.command.len() > 32 * 1024
    {
        return Err(msg::launch_command_invalid().into());
    }
    let mut rows = list(root)?;
    let preset = LaunchPreset {
        name: preset.name.trim().into(),
        ..preset
    };
    if rows
        .iter()
        .any(|p| p.id != preset.id && p.name == preset.name)
    {
        return Err(msg::launch_preset_duplicate().into());
    }
    if let Some(old) = rows.iter_mut().find(|p| p.id == preset.id) {
        *old = preset
    } else {
        rows.push(preset)
    }
    write(root, &rows)?;
    Ok(rows)
}
pub fn remove(root: &Path, id: &str) -> Result<Vec<LaunchPreset>, String> {
    let _held = WRITE.lock().map_err(|e| e.to_string())?;
    let mut rows = list(root)?;
    rows.retain(|p| p.id != id);
    write(root, &rows)?;
    Ok(rows)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn presets_are_named_commands_and_corruption_does_not_get_overwritten() {
        let root = std::env::temp_dir().join(format!("khor-launch-presets-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        assert!(list(&root).unwrap().is_empty());
        assert!(!path(&root).exists());
        let p = LaunchPreset {
            id: "one".into(),
            name: "research".into(),
            command: "export A='a b'\ncd ~/work".into(),
        };
        save(&root, p.clone()).unwrap();
        assert_eq!(list(&root).unwrap()[0].command, p.command);
        save(
            &root,
            LaunchPreset {
                name: "renamed".into(),
                ..p.clone()
            },
        )
        .unwrap();
        assert_eq!(list(&root).unwrap().len(), 1);
        assert!(
            save(
                &root,
                LaunchPreset {
                    id: "two".into(),
                    name: "renamed".into(),
                    ..p.clone()
                }
            )
            .is_err()
        );
        // The same command may have another name; only names must be unique.
        save(
            &root,
            LaunchPreset {
                id: "alias".into(),
                name: "alias".into(),
                ..p.clone()
            },
        )
        .unwrap();
        assert_eq!(list(&root).unwrap().len(), 2);
        assert_eq!(remove(&root, "alias").unwrap().len(), 1);
        assert_eq!(list(&root).unwrap()[0].name, "renamed");
        std::fs::write(path(&root), b"broken").unwrap();
        assert!(save(&root, p).is_err());
        assert_eq!(std::fs::read(path(&root)).unwrap(), b"broken");
        let _ = std::fs::remove_dir_all(root);
    }
}
