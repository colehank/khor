use super::{Contract, Info, Release, Role, current, inspect_supported};
use sha2::{Digest, Sha256};
use std::{fs::File, io::Read, path::Path};

const INCOMPATIBLE: &str =
    "runtime protocols or storage format are incompatible; existing sessions were retained";

// Published v0.1.15-rc.3, source 3dbaa9cc53eabd13a2767aa3ee5841df414ab6f5.
// Only these verified executable bytes qualify for the pre-manifest contract.
const LEGACY_PROGRAMS: [&str; 4] = [
    "499dec3513b3af917be3dc866950081a09224880591f1e6950a0524ef8a8a709",
    "827973daf71025dc001a8f20b12831a7e779919d3a3569ac7d144e7f59a7fe94",
    "8dfe7b859801ff3c62f20197e8318bf3b409b718bc7dde29b3683ccb04425428",
    "a4dafc2c94715817f560a22aaae1180ba060d54f7788dd35f4a712ed7dbab8ac",
];

pub(super) fn legacy_info(program: &Path) -> Result<Option<Info>, String> {
    let metadata = std::fs::metadata(program).map_err(|error| error.to_string())?;
    if !metadata.is_file() {
        return Err("activation source is not a regular program".into());
    }
    if metadata.len() > 256 * 1024 * 1024 {
        return Ok(None);
    }
    let mut file = File::open(program)
        .map_err(|error| error.to_string())?
        .take(256 * 1024 * 1024 + 1);
    let mut digest = Sha256::new();
    let mut buffer = [0; 64 * 1024];
    let mut length = 0;
    loop {
        let count = file.read(&mut buffer).map_err(|error| error.to_string())?;
        if count == 0 {
            break;
        }
        length += count as u64;
        digest.update(&buffer[..count]);
    }
    if length != metadata.len() {
        return Err("activation source changed while being verified".into());
    }
    let digest = format!("{:x}", digest.finalize());
    if !LEGACY_PROGRAMS.contains(&digest.as_str()) {
        return Ok(None);
    }
    Ok(Some(Info {
        executable: std::fs::canonicalize(program).map_err(|error| error.to_string())?,
        release: Release {
            schema: 1,
            version: "0.1.15".into(),
            role: Role::Cli,
            portable: true,
            embedded: [("legacy-complete-release".into(), digest)].into(),
            contract: Contract {
                terminal: 5,
                gui: "khor-gui/2".into(),
                node:
                    "khor/0:sha256:7bdf9a8cedcc26b783f3a6714899df875cbcb429e2f1ee64aa5a7d065e75fcda"
                        .into(),
                store: 1,
                journal: 1,
                agent: 1,
            },
        },
    }))
}

pub(super) fn check(other: &Path) -> Result<(), String> {
    let expected = current()?.release.contract;
    let actual = inspect_supported(other)?.release.contract;
    if !expected.accepts(&actual) {
        return Err(INCOMPATIBLE.into());
    }
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    check_local_contracts()?;
    Ok(())
}

#[cfg(not(any(target_os = "ios", target_os = "android")))]
pub(super) fn check_local_contracts() -> Result<(), String> {
    static CHECKED: std::sync::OnceLock<Result<(), String>> = std::sync::OnceLock::new();
    CHECKED
        .get_or_init(|| {
            let root = khor_runtime::local_directory().map_err(|error| error.to_string())?;
            if !root.try_exists().map_err(|error| error.to_string())? {
                return Ok(());
            }
            let expected = current()?.release.contract;
            let store = khor_runtime::Store::open(root, khor_runtime::Limits::default())
                .map_err(|error| error.to_string())?;
            for actual in store.contracts().map_err(|error| error.to_string())? {
                if !expected.accepts(&actual) {
                    return Err(INCOMPATIBLE.into());
                }
            }
            Ok(())
        })
        .clone()
}
