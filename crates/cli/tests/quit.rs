//! `khor quit` through the real binary: one word winds the machine's
//! khor down — serve (keeper and inner) and session hosts — and the
//! stop STAYS stopped (a keeper that resurrects after quit is 批19's
//! fix turned against 批22's promise). Files must survive: quit is
//! about processes, `close` is the one that deletes things.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

fn root(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("khor-quit-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

fn inner_pid(home: &PathBuf) -> Option<u32> {
    let text = fs::read_to_string(home.join(".khor").join("endpoint.json")).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    v.get("pid")?.as_u64().map(|p| p as u32)
}

fn alive(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
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

struct Reaper(u32);
impl Drop for Reaper {
    fn drop(&mut self) {
        let _ = Command::new("kill").args(["-9", &self.0.to_string()]).status();
    }
}

#[test]
fn quit_stops_serve_and_hosts_and_the_stop_stays_stopped() {
    let home = root("main");
    let mut keeper = Command::new(env!("CARGO_BIN_EXE_khor"))
        .arg("serve")
        .env("KHOR_HOME", &home)
        .env("KHOR_NAME", "box")
        .env_remove("KHOR_SESSION")
        .stderr(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let _reap = Reaper(keeper.id());
    wait_for("the serve to stand up", 20, || inner_pid(&home).is_some());
    let inner = inner_pid(&home).unwrap();

    // The serve wrote its keeper pid down by itself — no installer ran.
    let recorded: u32 = fs::read_to_string(home.join(".khor/serve.pid"))
        .expect("the keeper must write its own pid file")
        .trim()
        .parse()
        .unwrap();
    assert_eq!(recorded, keeper.id(), "and it must be the keeper's, not the inner's");

    // A session host, planted the way a real one lands: a hostfile in
    // the sessions dir whose pids are a live process in its own group
    // (its OWN group, or quit's group-signal would reach this test).
    use std::os::unix::process::CommandExt;
    let mut sleeper = Command::new("sleep");
    sleeper.arg("300").process_group(0);
    let mut host_proc = sleeper.spawn().unwrap();
    let _reap2 = Reaper(host_proc.id());
    let sdir = home.join(".khor/sessions/shell-quitprobe");
    fs::create_dir_all(&sdir).unwrap();
    fs::write(
        sdir.join("host.json"),
        format!(
            r#"{{"port":0,"cookie":"quitprobe","host_pid":{0},"child_pid":{0}}}"#,
            host_proc.id()
        ),
    )
    .unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_khor"))
        .arg("quit")
        .env("KHOR_HOME", &home)
        .env_remove("KHOR_SESSION")
        .output()
        .unwrap();
    let said = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "quit must exit 0: {said}");
    assert!(
        said.contains(khor_catalog::msg::QUIT_SERVE_STOPPED),
        "quit must say the serve stopped: {said}"
    );
    assert!(
        said.contains(&khor_catalog::msg::quit_hosts(1)),
        "quit must count the host it stopped: {said}"
    );

    // Both serve lives end; reaping the keeper is this test's job (it
    // is our child), and the host leaves too.
    wait_for("the keeper to end", 10, || {
        keeper.try_wait().map(|s| s.is_some()).unwrap_or(false)
    });
    wait_for("the inner to end", 10, || !alive(inner));
    wait_for("the host to end", 10, || {
        host_proc.try_wait().map(|s| s.is_some()).unwrap_or(false)
    });

    // The stop stays stopped: no third life, ever.
    std::thread::sleep(Duration::from_secs(2));
    assert_eq!(inner_pid(&home), Some(inner), "no new serve may appear after quit");
    assert!(!alive(inner));

    // Files survive: quit is about processes.
    assert!(home.join(".khor/sessions/shell-quitprobe/host.json").exists());

    // Control: a second quit finds nothing and says so.
    let out = Command::new(env!("CARGO_BIN_EXE_khor"))
        .arg("quit")
        .env("KHOR_HOME", &home)
        .env_remove("KHOR_SESSION")
        .output()
        .unwrap();
    let said = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success());
    assert!(
        said.contains(khor_catalog::msg::QUIT_NO_SERVE),
        "a quit with nothing to stop must say so: {said}"
    );

    let _ = fs::remove_dir_all(&home);
}
