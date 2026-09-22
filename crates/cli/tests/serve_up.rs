//! The login pull-up (`~/.local/bin/khor-serve-up`) sends the machine
//! that owns `$HOME` to `$HOME` — and onset ships it (SHELL.md 第五轮
//! 第 0 条).
//!
//! turing, 2026-09-22: every login ran the copy install.sh wrote on
//! 2026-08-21, which compared the whole `~/.khor/owner` to the hostname.
//! The owner file grew a second line on 9-11, so the comparison has
//! failed ever since — on the owner too — and each login with no live
//! resident behind `.khor-hosts/turing/.khor/serve.pid` started one
//! there: a second identity for the same machine, up since 9-17 beside
//! the real one. onset had upgraded the binary a dozen times since and
//! never touched the script.
//!
//! Nothing here reaches a real store: `HOME` is a scratch directory, the
//! identity in it is minted for the test, and the "serve" the script
//! starts is a stand-in that only writes down the `KHOR_HOME` it was
//! handed.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        // The stand-in serve sleeps; its pid is in whichever pid file
        // the script wrote. By pid, never by name.
        for pidf in [
            self.0.join(".khor").join("serve.pid"),
            self.0
                .join(".khor-hosts")
                .join(gethostname())
                .join(".khor")
                .join("serve.pid"),
        ] {
            if let Ok(pid) = fs::read_to_string(&pidf) {
                let _ = Command::new("kill").arg(pid.trim()).status();
            }
        }
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn gethostname() -> String {
    let out = Command::new("hostname").output().unwrap();
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

/// A home whose `~/.khor` this machine has claimed (two-line owner
/// stamp, the format since 9-11), with `khor-serve-up` beside a `khor`
/// that answers `root` truthfully and stands in for `serve`.
fn home_with(tag: &str, serve_up: &str) -> Scratch {
    let home = std::env::temp_dir().join(format!("khor-serveup-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    fs::create_dir_all(home.join(".local").join("bin")).unwrap();
    let home = Scratch(home);
    let claimed = Command::new(env!("CARGO_BIN_EXE_khor"))
        .arg("id")
        .env("KHOR_HOME", &home.0)
        .env("HOME", &home.0)
        .env_remove("KHOR_SESSION")
        .output()
        .unwrap();
    assert!(claimed.status.success(), "khor id: {claimed:?}");
    assert!(
        fs::read_to_string(home.0.join(".khor").join("owner"))
            .unwrap()
            .lines()
            .count()
            >= 2,
        "the owner stamp the fleet has carried since 9-11 is two lines"
    );
    let bin = home.0.join(".local").join("bin");
    let stand_in = format!(
        "#!/bin/sh\ncase \"$1\" in\nroot) exec '{}' root ;;\nserve) printf '%s' \"$KHOR_HOME\" > \"$HOME/started-with\"; exec sleep 30 ;;\nesac\nexit 2\n",
        env!("CARGO_BIN_EXE_khor")
    );
    fs::write(bin.join("khor"), stand_in).unwrap();
    fs::write(bin.join("khor-serve-up"), serve_up).unwrap();
    for f in ["khor", "khor-serve-up"] {
        fs::set_permissions(bin.join(f), fs::Permissions::from_mode(0o755)).unwrap();
    }
    home
}

/// Runs the pull-up the way `.profile` does, and returns the store the
/// serve was started for.
fn pull_up(home: &Path) -> PathBuf {
    let ran = Command::new(home.join(".local").join("bin").join("khor-serve-up"))
        .env("HOME", home)
        .env_remove("KHOR_HOME")
        .env_remove("KHOR_SESSION")
        .status()
        .unwrap();
    assert!(ran.success());
    let marker = home.join("started-with");
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if let Ok(s) = fs::read_to_string(&marker)
            && !s.is_empty()
        {
            return PathBuf::from(s);
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!("the pull-up never started a serve");
}

/// What `onset.sh` ships as `khor-serve-up`.
fn shipped() -> String {
    let onset = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/onset.sh");
    let out = Command::new("sh")
        .arg(&onset)
        .arg("--print-serve-up")
        .output()
        .unwrap();
    assert!(
        out.status.success() && !out.stdout.is_empty(),
        "onset must ship the login pull-up it would install: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

/// turing's `~/.local/bin/khor-serve-up`, byte for byte as read on
/// 2026-09-22 (install.sh of 2026-08-21).
const FLEET_2026_08_21: &str = r#"#!/bin/sh
# Start this host's khor serve unless it is already running.
set -u
BIN="$(dirname "$0")/khor"
R="$HOME"
if [ -f "$HOME/.khor/owner" ] && [ "$(cat "$HOME/.khor/owner")" != "$(hostname)" ]; then
    R="$HOME/.khor-hosts/$(hostname)"
fi
PIDF="$R/.khor/serve.pid"
mkdir -p "$R/.khor"
if [ -f "$PIDF" ] && kill -0 "$(cat "$PIDF")" 2>/dev/null; then
    exit 0
fi
if command -v setsid >/dev/null 2>&1; then
    KHOR_HOME="$R" setsid nohup "$BIN" serve >> "$R/.khor/serve.log" 2>&1 &
else
    KHOR_HOME="$R" nohup "$BIN" serve >> "$R/.khor/serve.log" 2>&1 &
fi
echo $! > "$PIDF"
"#;

#[test]
fn the_pull_up_onset_ships_sends_the_owner_to_its_own_store() {
    let home = home_with("new", &shipped());
    assert_eq!(
        pull_up(&home.0),
        home.0,
        "the machine that owns $HOME serves $HOME, not a side room of its own"
    );
}

/// The control, and the field bug in one line: the script the fleet
/// actually carries sends the owner to `.khor-hosts/<itself>`.
#[test]
fn the_pull_up_the_fleet_carried_sends_the_owner_to_a_second_store() {
    let home = home_with("fleet", FLEET_2026_08_21);
    assert_eq!(
        pull_up(&home.0),
        home.0.join(".khor-hosts").join(gethostname())
    );
}
