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

/// Everything after the stamp is stable; the stamp is not — so grep the
/// log for the tail. Built from the catalog message itself (no words
/// spelled here) by formatting with sentinels and slicing them off.
fn after_stamp(with_sentinel: &str, sentinel: char) -> String {
    let tail = with_sentinel.split(sentinel).next_back().unwrap();
    tail.trim_start_matches(']').trim_start().to_owned()
}

#[test]
fn a_new_binary_on_disk_takes_over_and_a_broken_one_is_refused() {
    let home = root("swap");
    // The keeper runs from a copy it owns, so replacing "the binary on
    // disk" replaces the very path its self_exe resolves to — the shape
    // of a real upgrade — without touching the shared cargo artifact.
    let bin = home.join("khor-copy");
    fs::copy(env!("CARGO_BIN_EXE_khor"), &bin).unwrap();
    let log = home.join("serve.log");
    let mut keeper = Command::new(&bin)
        .arg("serve")
        .env("KHOR_HOME", &home)
        .env("KHOR_NAME", "box")
        .env_remove("KHOR_SESSION")
        .stderr(fs::File::create(&log).unwrap())
        .stdout(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let _reap = Reaper(keeper.id());
    wait_for("the first serve to stand up", 20, || inner_pid(&home).is_some());
    let first = inner_pid(&home).unwrap();

    let read_log = || {
        let mut said = String::new();
        fs::File::open(&log).unwrap().read_to_string(&mut said).unwrap();
        said
    };

    // Control first: a broken new generation must be refused, and the
    // running serve must not pay for it. Rename in, like an installer's
    // `mv` — same inode dance, wrong contents (and not executable,
    // which is one of the ways a broken install is broken).
    let junk = home.join("junk");
    fs::write(&junk, b"not a binary\n").unwrap();
    fs::rename(&junk, &bin).unwrap();
    let refused_line = {
        let m = khor_catalog::msg::serve_swap_refused("\u{1}", "\u{2}");
        after_stamp(m.split('\u{2}').next().unwrap(), '\u{1}')
    };
    wait_for("the refusal to be written down", 20, || read_log().contains(&refused_line));
    assert_eq!(
        inner_pid(&home),
        Some(first),
        "a refused generation must not cost the running serve its life"
    );
    assert!(alive(first), "the old serve must still be there after a refusal");

    // The real thing: a healthy binary lands, the serve hands over.
    let next = home.join("next");
    fs::copy(env!("CARGO_BIN_EXE_khor"), &next).unwrap();
    fs::rename(&next, &bin).unwrap();
    wait_for("the new generation to stand up", 30, || {
        inner_pid(&home).is_some_and(|pid| pid != first && alive(pid))
    });
    let second = inner_pid(&home).unwrap();
    assert!(!alive(first), "the old generation must be gone, not doubled");
    let v = env!("CARGO_PKG_VERSION");
    let swap_line = after_stamp(&khor_catalog::msg::serve_swapping("\u{1}", v, v), '\u{1}');
    assert!(
        read_log().contains(&swap_line),
        "the handover must be named in the log: {}",
        read_log()
    );
    // A handover is not a death: the keeper's own TERM must not be
    // written down as one, or every upgrade reads like a crash.
    assert!(
        !read_log().contains(&khor_catalog::msg::died_by_signal(15)),
        "the keeper's own TERM dressed as a death: {}",
        read_log()
    );

    Command::new("kill").args(["-TERM", &keeper.id().to_string()]).status().unwrap();
    wait_for("the keeper and its serve to leave", 10, || {
        keeper.try_wait().map(|s| s.is_some()).unwrap_or(false) && !alive(second)
    });
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn a_shielded_keeper_serves_from_a_local_copy_and_still_swaps() {
    // #76: on an NFS home, replacing the binary killed the RUNNING
    // keeper on another machine — silently, twice. The medicine: a
    // keeper whose binary sits on a network disk re-execs from a local
    // copy, so the install path can be replaced without touching any
    // running image. KHOR_SHIELD=1 forces that verdict on a local disk;
    // everything downstream must keep working through the exec — the
    // pid (TERM below lands on the pre-exec id, which is what serve.pid
    // holds), the disk watch, and the handover.
    let home = root("shield");
    let bin = home.join("khor-copy");
    fs::copy(env!("CARGO_BIN_EXE_khor"), &bin).unwrap();
    let log = home.join("serve.log");
    let mut keeper = Command::new(&bin)
        .arg("serve")
        .env("KHOR_HOME", &home)
        .env("KHOR_NAME", "box")
        .env("KHOR_SHIELD", "1")
        .env_remove("KHOR_SESSION")
        .stderr(fs::File::create(&log).unwrap())
        .stdout(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let _reap = Reaper(keeper.id());
    wait_for("the serve to stand up", 20, || inner_pid(&home).is_some());
    let first = inner_pid(&home).unwrap();

    let read_log = || {
        let mut said = String::new();
        fs::File::open(&log).unwrap().read_to_string(&mut said).unwrap();
        said
    };
    // The shield names itself before the exec; the stable text is what
    // follows the path argument.
    let shield_line = khor_catalog::msg::serve_shielding("\u{1}", "\u{2}")
        .split('\u{2}')
        .next_back()
        .unwrap()
        .to_owned();
    assert!(
        read_log().contains(&shield_line),
        "KHOR_SHIELD=1 must be heard and named: {}",
        read_log()
    );

    // The handover still works from behind the shield: replace the
    // watched install path, and the new generation must stand up —
    // proof the keeper kept watching the install path rather than the
    // copy it runs from.
    let next = home.join("next");
    fs::copy(env!("CARGO_BIN_EXE_khor"), &next).unwrap();
    fs::rename(&next, &bin).unwrap();
    wait_for("the new generation to stand up", 30, || {
        inner_pid(&home).is_some_and(|pid| pid != first && alive(pid))
    });
    let second = inner_pid(&home).unwrap();
    assert!(!alive(first), "the old generation must be gone, not doubled");
    assert!(
        !read_log().contains(&khor_catalog::msg::died_by_signal(15)),
        "a shielded handover is still a handover, not a death: {}",
        read_log()
    );

    // The exec kept the pid: TERM at the pre-exec id must still stop
    // everything — serve.pid, `khor quit` and systemd's PIDFile all
    // point there.
    Command::new("kill").args(["-TERM", &keeper.id().to_string()]).status().unwrap();
    wait_for("the keeper and its serve to leave", 10, || {
        keeper.try_wait().map(|s| s.is_some()).unwrap_or(false) && !alive(second)
    });
    let _ = fs::remove_dir_all(&home);
}

#[test]
fn an_inner_whose_keeper_died_leaves_instead_of_squatting() {
    // 2026-08-20 (#76): a keeper died silently on an NFS binary swap
    // and its inner squatted for an hour — healthy-looking, holding
    // the endpoint key, in the way of every next serve. The inner must
    // notice it was orphaned (ppid becomes 1) and leave on its own.
    let home = root("orphan");
    let mut keeper = Command::new(env!("CARGO_BIN_EXE_khor"))
        .arg("serve")
        .env("KHOR_HOME", &home)
        .env("KHOR_NAME", "box")
        .env_remove("KHOR_SESSION")
        .stderr(fs::File::create(home.join("serve.log")).unwrap())
        .stdout(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let _reap = Reaper(keeper.id());
    wait_for("the serve to stand up", 20, || inner_pid(&home).is_some());
    let inner = inner_pid(&home).unwrap();

    // The keeper dies the way it did in the field: no forwarding, no
    // last words — straight to KILL.
    Command::new("kill").args(["-9", &keeper.id().to_string()]).status().unwrap();
    keeper.wait().unwrap();

    // The inner notices on its own tick and leaves — nobody signals it.
    wait_for("the orphaned inner to leave by itself", 15, || !alive(inner));
    let mut said = String::new();
    fs::File::open(home.join("serve.log")).unwrap().read_to_string(&mut said).unwrap();
    let expected = {
        let m = khor_catalog::msg::serve_orphaned("\u{1}");
        m.split('\u{1}').next_back().unwrap().trim_start_matches(']').trim_start().to_owned()
    };
    assert!(said.contains(&expected), "the departure must be named: {said}");

    let _ = fs::remove_dir_all(&home);
}
