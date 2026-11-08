use std::{fs, path::PathBuf, process::Command};

struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// An app-bundle signature seals Info.plist as well as the Mach-O. Unsigned
/// integration binaries did not exercise the installed desktop's host path.
#[cfg(target_os = "macos")]
#[test]
fn signed_desktop_hosts_keep_their_bundle_and_runtime_lease() {
    const CHILD: &str = "KHOR_TEST_SIGNED_HOST_BUNDLE";
    if let Some(bundle) = std::env::var_os(CHILD) {
        let bundle = PathBuf::from(bundle);
        let source = bundle.join("Contents/MacOS/khor-gui");
        let image = khor_node::runtime_image::host_at(&source).unwrap();
        assert_ne!(image.path(), source);
        let copied_bundle = image.path().ancestors().nth(3).unwrap();
        let verified = Command::new("/usr/bin/codesign")
            .args(["--verify", "--strict"])
            .arg(copied_bundle)
            .output()
            .unwrap();
        assert!(verified.status.success(), "{verified:?}");
        let mut command = Command::new(image.path());
        command.arg("_runtime_info").stdout(std::process::Stdio::piped());
        let output = image.spawn(&mut command).unwrap().wait_with_output().unwrap();
        assert!(output.status.success(), "{output:?}");
        let info: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(info["release"]["role"], "desktop-host");
        let held = khor_runtime::Store::image_for_program(image.path()).unwrap().unwrap();
        assert_eq!(held.role(), khor_runtime::Role::DesktopHost);
        assert_eq!(held.path(), image.path());
        return;
    }
    let fixture = Fixture(std::env::temp_dir().join(format!(
        "khor-signed-host-{}", khor_node::link::fresh_hex().unwrap()
    )));
    let bundle = fixture.0.join("Khor.app");
    let contents = bundle.join("Contents");
    fs::create_dir_all(contents.join("MacOS")).unwrap();
    fs::write(contents.join("Info.plist"), br#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict><key>CFBundleIdentifier</key><string>io.github.colehank.khor.host-fixture</string><key>CFBundleExecutable</key><string>khor-gui</string><key>CFBundlePackageType</key><string>APPL</string></dict></plist>"#).unwrap();
    fs::copy(env!("CARGO_BIN_EXE_khor"), contents.join("MacOS/khor-gui")).unwrap();
    let sign = Command::new("/usr/bin/codesign")
        .args(["--force", "--sign", "-", "--options", "runtime", "--timestamp=none"])
        .arg(&bundle).output().unwrap();
    assert!(sign.status.success(), "{sign:?}");
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "signed_desktop_hosts_keep_their_bundle_and_runtime_lease", "--nocapture"])
        .env(CHILD, &bundle).env("KHOR_RUNTIME_DIR", fixture.0.join("runtime"))
        .env_remove("KHOR_RUNTIME_INSTALL_SOURCE").env_remove("KHOR_INSTALL_SOURCE")
        .output().unwrap();
    assert!(output.status.success(), "{}\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
}

#[test]
fn managed_host_selection_reads_the_probe_reply_before_joining_its_pipe_owner() {
    let binary = PathBuf::from(env!("CARGO_BIN_EXE_khor"));
    let image = khor_node::runtime_image::host_at(&binary).unwrap();
    assert_ne!(image.path(), binary);
    let mut command = Command::new(image.path());
    command
        .arg("_runtime_info")
        .stdout(std::process::Stdio::piped());
    let output = image
        .spawn(&mut command)
        .unwrap()
        .wait_with_output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let info: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(info["release"]["schema"], 1);
    assert_eq!(info["release"]["platform"], khor_runtime::platform());
    assert_eq!(
        PathBuf::from(info["executable"].as_str().unwrap()),
        image.path()
    );
}

#[test]
fn release_probe_does_not_open_user_or_runtime_stores() {
    let fixture = Fixture(std::env::temp_dir().join(format!(
        "khor-release-info-{}",
        khor_node::link::fresh_hex().unwrap()
    )));
    fs::create_dir(&fixture.0).unwrap();
    let home = fixture.0.join("unopened-home");
    let runtime = fixture.0.join("unopened-runtime");
    let output = Command::new(env!("CARGO_BIN_EXE_khor"))
        .arg("_release_info")
        .env("KHOR_HOME", &home)
        .env("KHOR_RUNTIME_DIR", &runtime)
        .env_remove("KHOR_RUNTIME_INSTALL_SOURCE")
        .env_remove("KHOR_INSTALL_SOURCE")
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let info: khor_node::updates::ProgramInfo = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(info.schema, 1);
    assert_eq!(info.build.version, khor_node::VERSION);
    assert!(output.stdout.len() <= 16 * 1024);
    assert!(!home.exists());
    assert!(!runtime.exists());
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

#[test]
fn activation_check_and_incompatible_local_runtime_never_open_user_data() {
    let fixture = Fixture(std::env::temp_dir().join(format!(
        "khor-runtime-activation-{}",
        khor_node::link::fresh_hex().unwrap()
    )));
    fs::create_dir(&fixture.0).unwrap();
    let home = fixture.0.join("unopened-home");
    let runtime = fixture.0.join("runtime");
    let binary = PathBuf::from(env!("CARGO_BIN_EXE_khor"));
    let probe = Command::new(&binary).arg("_runtime_info").output().unwrap();
    assert!(probe.status.success());
    let info: serde_json::Value = serde_json::from_slice(&probe.stdout).unwrap();
    let mut contract: khor_runtime::Contract =
        serde_json::from_value(info["release"]["contract"].clone()).unwrap();
    let checked = Command::new(&binary)
        .args([std::ffi::OsStr::new("_runtime_check"), binary.as_os_str()])
        .env("KHOR_HOME", &home)
        .env("KHOR_RUNTIME_DIR", &runtime)
        .output()
        .unwrap();
    assert!(checked.status.success(), "{checked:?}");
    assert_eq!(checked.stdout, b"compatible\n");
    assert!(!home.exists());
    assert!(!runtime.exists(), "activation probes do not create stores");

    contract.terminal += 1;
    let store =
        khor_runtime::Store::open(runtime.clone(), khor_runtime::Limits::default()).unwrap();
    let _image = store
        .publish(
            khor_runtime::Role::Cli,
            "incompatible-test-version",
            contract,
            Default::default(),
            khor_runtime::Source::Bytes(b"owned fixture; never executed"),
        )
        .unwrap();
    let blocked = Command::new(&binary)
        .arg("id")
        .env("KHOR_HOME", &home)
        .env("KHOR_RUNTIME_DIR", &runtime)
        .output()
        .unwrap();
    assert_eq!(blocked.status.code(), Some(78), "{blocked:?}");
    assert!(String::from_utf8_lossy(&blocked.stderr).contains("incompatible"));
    assert!(!home.exists(), "refusal must precede Node::open");

    drop(_image);
    let released = Command::new(&binary)
        .args([std::ffi::OsStr::new("_runtime_check"), binary.as_os_str()])
        .env("KHOR_HOME", &home)
        .env("KHOR_RUNTIME_DIR", &runtime)
        .output()
        .unwrap();
    assert!(
        released.status.success(),
        "an idle rollback image must not block activation: {released:?}"
    );
    assert!(
        !home.exists(),
        "checking idle images must not open user data"
    );
}
