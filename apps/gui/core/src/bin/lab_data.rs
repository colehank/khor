//! Canonical data for frontend-only Labs. Domain output comes from the real
//! avatar generator and filesystem listing; this binary never opens a node.
use khor_core::avatar::{AvatarSeed, AvatarStyle, avatar};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

struct Workspace(PathBuf);

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn copy_tree(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let target = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn directory_data(
    path: &Path,
    relative: &str,
    directories: &mut BTreeMap<String, Value>,
    documents: &mut BTreeMap<String, String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (_, entries, truncated) =
        khor_node::files::list_dir(path.to_str().ok_or("Invalid fixture path")?)?;
    directories.insert(
        relative.to_owned(),
        json!({"entries":entries,"truncated":truncated}),
    );
    for entry in entries {
        let child = if relative.is_empty() {
            entry.name.clone()
        } else {
            format!("{relative}/{}", entry.name)
        };
        if entry.dir {
            directory_data(&path.join(&entry.name), &child, directories, documents)?;
        } else {
            documents.insert(child, fs::read_to_string(path.join(entry.name))?);
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let path = std::env::temp_dir().join(format!("khor-lab-data-{}-{stamp}", std::process::id()));
    fs::create_dir(&path)?;
    let workspace = Workspace(path);
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../dev/lab/data/workspace");
    for name in ["project", "research-notes", "design-system", "资料库"] {
        copy_tree(&source, &workspace.0.join(name))?;
    }
    let mut directories = BTreeMap::new();
    let mut documents = BTreeMap::new();
    directory_data(&workspace.0, "", &mut directories, &mut documents)?;
    let ids: Vec<_> = std::iter::once("e".repeat(64))
        .chain((2..=6).map(|index| format!("{index:x}").repeat(64)))
        .collect();
    let avatars: BTreeMap<_, _> = ids
        .iter()
        .map(|id| {
            let seed = AvatarSeed::from_id_hex(id).expect("synthetic device id");
            let face = avatar(&seed, &AvatarStyle::default());
            (id.clone(), face)
        })
        .collect();
    // A scanable, deliberately invalid invitation: the Lab never opens a node
    // or issues a credential that could admit a real machine.
    let token = format!("khorLAB_ONLY_{}", "NOT_A_PAIRING_CREDENTIAL".repeat(8));
    let pairing_ticket = khor_gui_core::Ticket {
        qr: khor_gui_core::qr::encode(&token),
        token,
        minutes: khor_node::link::invite_window_minutes(),
    };
    let previous = khor_catalog::lang::current();
    khor_catalog::lang::set(khor_catalog::lang::Lang::Zh);
    let zh = khor_gui_core::config_rows(&workspace.0);
    khor_catalog::lang::set(khor_catalog::lang::Lang::En);
    let en = khor_gui_core::config_rows(&workspace.0);
    khor_catalog::lang::set(previous);
    println!(
        "{}",
        json!({"schema":1,"machine_ids":ids,"avatars":avatars,"directories":directories,"documents":documents,"pairing_ticket":pairing_ticket,"settings":{"zh":zh,"en":en}})
    );
    Ok(())
}
