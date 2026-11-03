use std::{fs, path::PathBuf, process::Command};

struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn runtime_inspection_is_bounded_json_without_opening_a_user_store() {
    let fixture = Fixture(std::env::temp_dir().join(format!(
        "khor-runtime-info-{}",
        khor_node::link::fresh_hex().unwrap()
    )));
    fs::create_dir(&fixture.0).unwrap();
    let home = fixture.0.join("unopened-home");
    let output = Command::new(env!("CARGO_BIN_EXE_khor"))
        .arg("_runtime_info")
        .env("KHOR_HOME", &home)
        .env_remove("KHOR_RUNTIME_INSTALL_SOURCE")
        .env_remove("KHOR_INSTALL_SOURCE")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.len() <= 16 * 1024);
    let info: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(info["release"]["schema"], 1);
    assert_eq!(info["release"]["version"], khor_node::VERSION);
    assert_eq!(
        info["release"]["contract"]["terminal"],
        khor_node::host::HOST_VERSION
    );
    assert_eq!(info["release"]["contract"]["gui"], "khor-gui/2");
    assert_eq!(
        PathBuf::from(info["executable"].as_str().unwrap()),
        fs::canonicalize(env!("CARGO_BIN_EXE_khor")).unwrap()
    );
    assert!(
        !home.exists(),
        "an activation probe must not create or migrate a user store"
    );
}
