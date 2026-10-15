use super::*;
use std::collections::BTreeSet;

/// A browser directory is staged once, keeping its hierarchy and empty
/// directories. All paths are inside a freshly-created upload token.
pub fn term_drop_folder(root: &Path, name: &str, dirs: &[String]) -> Result<String, String> {
    let mut parents = BTreeSet::new();
    for dir in dirs {
        khor_node::tree::relative(dir)?;
        let mut path = String::new();
        for part in dir.split('/') {
            if !path.is_empty() {
                path.push('/');
            }
            path.push_str(part);
            parents.insert(path.clone());
        }
    }
    let manifest = khor_node::tree::Manifest::checked(
        parents
            .iter()
            .map(|path| khor_node::tree::Entry {
                path: path.clone(),
                dir: true,
                size: 0,
                at_ms: 0,
                mode: 0,
            })
            .collect(),
    )?;
    let path = term_drop_stage(root, name, &mut std::io::empty())?;
    let outcome = (|| -> Result<(), String> {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
        std::fs::create_dir(&path).map_err(|e| e.to_string())?;
        for dir in parents {
            std::fs::create_dir_all(Path::new(&path).join(khor_node::tree::relative(&dir)?))
                .map_err(|e| e.to_string())?;
        }
        let budget = Path::new(&path).parent().unwrap().join("budget");
        std::fs::write(budget, format!("0 {}", manifest.entries.len())).map_err(|e| e.to_string())
    })();
    if let Err(why) = outcome {
        term_drop_forget(root, std::slice::from_ref(&path));
        return Err(why);
    }
    Ok(path)
}

pub fn term_drop_stage_into(
    root: &Path,
    folder: &str,
    relative: &str,
    body: &mut dyn std::io::Read,
) -> Result<String, String> {
    use std::io::Write;
    // Serializing the budget and its write also handles two browser
    // upload requests arriving at once. No state lives in the webview.
    static WRITE: Mutex<()> = Mutex::new(());
    let _write = plock(&WRITE);
    let token = staged_dir(root, folder).ok_or_else(|| khor_catalog::msg::tree_bad_path(folder))?;
    let rel = khor_node::tree::relative(relative)?;
    let budget = token.join("budget");
    let saved = std::fs::read_to_string(&budget).map_err(|e| e.to_string())?;
    let mut fields = saved.split_whitespace();
    let bytes = fields
        .next()
        .unwrap_or("")
        .parse::<u64>()
        .map_err(|e| e.to_string())?;
    let count = fields
        .next()
        .unwrap_or("")
        .parse::<usize>()
        .map_err(|e| e.to_string())?;
    if count >= khor_node::tree::MAX_ENTRIES {
        return Err(khor_catalog::msg::tree_too_many(
            khor_node::tree::MAX_ENTRIES,
        ));
    }
    let path = Path::new(folder).join(rel);
    let parent = path
        .parent()
        .ok_or_else(|| khor_catalog::msg::tree_bad_path(relative))?;
    // Every directory was declared when the selection was staged. This
    // also prevents an undeclared path from bypassing the entry budget.
    if !parent.is_dir() {
        return Err(khor_catalog::msg::tree_bad_path(relative));
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    let result = (|| -> Result<(), String> {
        let mut moved = 0u64;
        let mut buf = [0u8; 64 * 1024];
        loop {
            let got = body.read(&mut buf).map_err(|e| e.to_string())?;
            if got == 0 {
                break;
            }
            moved += got as u64;
            if moved > khor_node::drops::CAP.saturating_sub(bytes) {
                return Err(khor_catalog::msg::tree_too_big(khor_node::drops::CAP >> 20));
            }
            file.write_all(&buf[..got]).map_err(|e| e.to_string())?;
        }
        std::fs::write(&budget, format!("{} {}", bytes + moved, count + 1))
            .map_err(|e| e.to_string())
    })();
    drop(file);
    if let Err(why) = result {
        let _ = std::fs::remove_file(&path);
        return Err(why);
    }
    Ok(path.display().to_string())
}

/// iOS holds the directory's security scope while this bounded copy
/// runs off the UI thread. Afterwards only app-owned paths are exposed.
pub fn term_drop_stage_native(root: &Path, source: &Path) -> Result<String, String> {
    let name = khor_node::drops::name_of(&source.display().to_string())?;
    let placeholder = term_drop_stage(root, &name, &mut std::io::empty())?;
    let dir = Path::new(&placeholder).parent().unwrap();
    std::fs::remove_file(&placeholder).map_err(|e| e.to_string())?;
    let outcome = khor_node::drops::copy_into(source, dir, &khor_node::drops::Job::default())
        .map(|p| p.display().to_string());
    if outcome.is_err() {
        let _ = std::fs::remove_dir_all(dir);
    }
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn staged_directories_keep_structure_and_cannot_escape_their_token() {
        let root = std::env::temp_dir().join(format!("khor-stage-tree-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let folder = term_drop_folder(&root, "data", &["nested/empty".into()]).unwrap();
        term_drop_stage_into(&root, &folder, "nested/a.csv", &mut &b"x,y\n"[..]).unwrap();
        assert!(Path::new(&folder).join("nested/empty").is_dir());
        assert_eq!(
            std::fs::read(Path::new(&folder).join("nested/a.csv")).unwrap(),
            b"x,y\n"
        );
        assert!(term_drop_stage_into(&root, &folder, "../outside", &mut &b"no"[..]).is_err());
        assert!(
            term_drop_stage_into(&root, &folder, "nested/a.csv", &mut &b"overwritten"[..]).is_err()
        );
        assert!(term_drop_stage_into(&root, &folder, "undeclared/a", &mut &b"no"[..]).is_err());
        term_drop_forget(&root, &[folder]);
        assert_eq!(std::fs::read_dir(stage_root(&root)).unwrap().count(), 0);
        let _ = std::fs::remove_dir_all(root);
    }
}
