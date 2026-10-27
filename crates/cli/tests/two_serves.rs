//! `khor doctor` says so when one machine is running two serves
//! (SHELL.md 第五轮 第 0 条).
//!
//! Measured on turing, 2026-09-22: a keeper from 9 月 17 日 serving a
//! second store (`.khor-hosts/turing`, a second identity for the same
//! machine) beside the resident the installer had just restarted. The
//! old one held port 5467, so the new one's web face never came up —
//! and every row of `khor doctor` read fine, because every row asks
//! about *this* store, and this store's serve was healthy. Nothing
//! looked at the machine.
//!
//! The shape here is that one, on temporary stores: two keepers, two
//! `KHOR_HOME`s, one user. The doctor asked from either store must name
//! the other.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, Instant};

/// A scratch tree that takes itself away however the test leaves (the
/// reasoning is on `crates/node/src/tests.rs::Scratch`).
struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn root(tag: &str) -> Scratch {
    let p = std::env::temp_dir().join(format!("khor-two-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    Scratch(p)
}

/// Kills the keeper by pid however the test leaves; TERM reaches its
/// serve through the keeper's forwarding.
struct Keeper(Child);

impl Drop for Keeper {
    fn drop(&mut self) {
        // Already reaped: its pid may be somebody else's by now.
        if self.0.try_wait().ok().flatten().is_some() {
            return;
        }
        let _ = Command::new("kill")
            .args(["-TERM", &self.0.id().to_string()])
            .status();
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            if self.0.try_wait().ok().flatten().is_some() {
                return;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn keeper(home: &Path, name: &str) -> Keeper {
    Keeper(
        Command::new(env!("CARGO_BIN_EXE_khor"))
            .arg("serve")
            .env("KHOR_HOME", home)
            .env("KHOR_NAME", name)
            // Each its own web port: the collision is not what this
            // test is about, and the doctor must see both whatever
            // the ports do.
            .env("KHOR_WEB_PORT", "0")
            .env("KHOR_NO_MDNS", "1")
            .env_remove("KHOR_SESSION")
            .stderr(fs::File::create(home.join("serve.log")).unwrap())
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap(),
    )
}

fn inner_pid(home: &Path) -> Option<u32> {
    let text = fs::read_to_string(home.join(".khor").join("endpoint.json")).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    v.get("pid")?.as_u64().map(|p| p as u32)
}

fn wait_for(what: &str, secs: u64, mut f: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(secs);
    while Instant::now() < deadline {
        if f() {
            return;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    panic!("timed out waiting for {what}");
}

/// The serve row of `khor doctor --json`, asked from the store `store`
/// by a user whose home directory is `home`.
fn serve_row(store: &Path, home: &Path) -> serde_json::Value {
    let out = Command::new(env!("CARGO_BIN_EXE_khor"))
        .args(["doctor", "--json"])
        .env("KHOR_HOME", store)
        .env("HOME", home)
        .env_remove("KHOR_SESSION")
        .output()
        .unwrap();
    let rows: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "doctor --json did not print JSON ({e}): {}",
            String::from_utf8_lossy(&out.stderr)
        )
    });
    rows.as_array()
        .unwrap()
        .iter()
        .find(|r| r["key"] == "serve")
        .cloned()
        .expect("a serve row")
}

#[test]
fn the_doctor_names_a_second_serve_on_the_same_machine() {
    // turing's shape: one serve on the home itself, one on a side room
    // under `.khor-hosts` — both stores khor's own rule could hand this
    // machine. Plus a control: a third serve on a store elsewhere (a
    // test's, a smoke run's), which is nobody's second identity and must
    // not be counted — smoke's own doctor row went amber over exactly
    // that the day the first version shipped.
    let home = root("home");
    let a = home.0.clone();
    let b = home.0.join(".khor-hosts").join("box-b");
    fs::create_dir_all(&b).unwrap();
    let elsewhere = root("elsewhere");
    let ka = keeper(&a, "box-a");
    let kb = keeper(&b, "box-b");
    let kc = keeper(&elsewhere.0, "box-c");
    wait_for("all three serves to stand up", 30, || {
        inner_pid(&a).is_some() && inner_pid(&b).is_some() && inner_pid(&elsewhere.0).is_some()
    });

    let row = serve_row(&a, &home.0);
    let said = row["said"].as_str().unwrap().to_owned();
    assert_eq!(
        row["state"], "warn",
        "two serves for one machine is not a clean bill: {said}"
    );
    assert!(
        said.contains(&kb.0.id().to_string()),
        "the other serve is named by the pid a person would kill (its keeper's): {said}"
    );
    assert!(
        said.contains(&b.display().to_string()),
        "and by the store it is serving, which is how a person tells the two apart: {said}"
    );
    assert!(
        !said.contains(&kc.0.id().to_string()),
        "a serve on a store this machine's rule would never pick is not counted: {said}"
    );
    // Asked from the other side, the same fact.
    let said_b = serve_row(&b, &home.0)["said"].as_str().unwrap().to_owned();
    assert!(
        said_b.contains(&ka.0.id().to_string()),
        "the doctor asked from the second store names the first: {said_b}"
    );
    drop(kc);
}

/// **A second `khor serve` on one store stands down** (round NET-10).
/// The owner's fleet: a serve started by hand beside the kept one wrote
/// its pid over the keeper's in `serve.pid`, and once it was stopped the
/// file named a dead process while the keeper ran on. The second serve
/// now says who holds the store and leaves, and the file still names the
/// keeper.
#[test]
fn a_second_serve_on_one_store_stands_down() {
    let home = root("again");
    // Off the real relays and off the segment: a loopback relay nobody
    // listens on.
    fs::create_dir_all(home.0.join(".khor")).unwrap();
    fs::write(
        home.0.join(".khor").join("relays.json"),
        r#"{"relays":[{"url":"http://127.0.0.1:9"}],"only_mine":true}"#,
    )
    .unwrap();
    let first = keeper(&home.0, "box-a");
    wait_for("the first serve to stand up", 30, || {
        inner_pid(&home.0).is_some()
    });
    let inner = inner_pid(&home.0);
    let pid_file =
        || fs::read_to_string(home.0.join(".khor").join("serve.pid")).unwrap_or_default();
    assert_eq!(pid_file().trim(), first.0.id().to_string());

    let mut second = Keeper(
        Command::new(env!("CARGO_BIN_EXE_khor"))
            .arg("serve")
            .env("KHOR_HOME", &home.0)
            .env("KHOR_NAME", "box-a")
            .env("KHOR_WEB_PORT", "0")
            .env("KHOR_NO_MDNS", "1")
            .env_remove("KHOR_SESSION")
            .stderr(fs::File::create(home.0.join("second.log")).unwrap())
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap(),
    );
    let mut status = None;
    wait_for("the second serve to stand down", 20, || {
        status = second.0.try_wait().unwrap();
        status.is_some()
    });
    assert!(status.unwrap().success());
    let said = fs::read_to_string(home.0.join("second.log")).unwrap();
    assert!(
        said.contains(&first.0.id().to_string()),
        "it names the serve in the way: {said}"
    );
    assert_eq!(
        pid_file().trim(),
        first.0.id().to_string(),
        "serve.pid still names the keeper"
    );
    assert_eq!(
        inner_pid(&home.0),
        inner,
        "and the resident is the one that was there"
    );
}
