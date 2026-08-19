//! The keeper through the real binary (docs/handoff 批19; 账本:
//! 网络类改动必须真连,还要有对照组). hinton's serve died by signal and
//! said nothing for eleven hours — so the assertions here are exactly
//! the three things that failure was missing: the death is **survived**
//! (a new serve stands up), the death is **named** (a line in the log
//! says signal 9), and an **intentional** stop stays stopped — a keeper
//! that resurrects what an operator just killed is a different bug
//! wearing the fix's name.

use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

fn root(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("khor-keeper-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

/// The pid the *inner* serve wrote about itself — endpoint.json is the
/// serve's own voice, so this is how the test tells the lives apart.
fn inner_pid(home: &PathBuf) -> Option<u32> {
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

fn alive(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// An assertion that panics must not leak a serve tree.
struct Reaper(u32);
impl Drop for Reaper {
    fn drop(&mut self) {
        let _ = Command::new("kill").args(["-9", &self.0.to_string()]).status();
    }
}

#[test]
fn a_killed_serve_comes_back_named_and_a_stopped_one_stays_stopped() {
    let home = root("main");
    let log = home.join("serve.log");
    let mut keeper = Command::new(env!("CARGO_BIN_EXE_khor"))
        .arg("serve")
        .env("KHOR_HOME", &home)
        .env("KHOR_NAME", "box")
        .env_remove("KHOR_SESSION")
        .stderr(fs::File::create(&log).unwrap())
        .stdout(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let keeper_pid = keeper.id();
    let _reap = Reaper(keeper_pid);

    wait_for("the first serve to stand up", 20, || inner_pid(&home).is_some());
    let first = inner_pid(&home).unwrap();
    assert_ne!(first, keeper_pid, "the serve must be a child, or nothing is keeping it");

    // The failure being reproduced: a death by signal, which writes
    // nothing of its own.
    Command::new("kill").args(["-9", &first.to_string()]).status().unwrap();

    wait_for("a new serve to stand up in the old one's place", 20, || {
        inner_pid(&home).is_some_and(|pid| pid != first && alive(pid))
    });
    let second = inner_pid(&home).unwrap();

    // The death is named, from outside — the vantage point hinton's log
    // did not have.
    let mut said = String::new();
    fs::File::open(&log).unwrap().read_to_string(&mut said).unwrap();
    assert!(
        said.contains(&khor_catalog::msg::died_by_signal(9)),
        "the log must say how it died: {said}"
    );

    // Control: an intentional stop stays stopped. TERM lands on the
    // keeper — the pid the installer's pid file would hold — and both
    // lives must end, with no third.
    Command::new("kill").args(["-TERM", &keeper_pid.to_string()]).status().unwrap();
    // The keeper is this test's own child, so a bare `kill -0` would say
    // "alive" about its zombie forever (账本: 判 agent 死活要证据 —
    // pid_alive can't tell a zombie from a process). `try_wait` reaps.
    wait_for("the keeper and its serve to leave", 10, || {
        keeper.try_wait().map(|s| s.is_some()).unwrap_or(false) && !alive(second)
    });
    std::thread::sleep(Duration::from_secs(2));
    let after = inner_pid(&home).unwrap();
    assert!(
        after == second && !alive(after),
        "an operator's kill must not be repaired: a new life after TERM is a keeper gone rogue"
    );

    let _ = fs::remove_dir_all(&home);
}
