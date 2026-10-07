//! `khor add` / `khor promote` against a scratch ssh world (lane join-a):
//! unprivileged sshds on loopback ports, scratch host keys and
//! authorized_keys, the client side through `KHOR_SSH_DIR`, and a
//! loopback mirror serving the binary under test. Never the user's
//! `~/.ssh`, Remote Login, the system sshd or any real machine.
//!
//! **The far side's HOME is forced**: sshd to localhost logs in as this
//! same user, so each sshd's `ForceCommand` wrapper exports a scratch
//! HOME (and refuses to run when it is anything else), and the test
//! asserts it over ssh before the first install step. Without that an
//! install would write the real `~/.local/bin/khor`.
//!
//! One test, run in order: the world is a few seconds to build and the
//! cases share it.
use std::collections::BTreeSet;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::Duration;

const SSHD: &str = "/usr/sbin/sshd";

/// A child killed by its own pid when the test ends.
struct Kept(Child);
impl Drop for Kept {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

struct World {
    dir: PathBuf,
    kept: Vec<Kept>,
}

impl Drop for World {
    fn drop(&mut self) {
        // Every far khor this test started, by the pid it wrote.
        for host in HOSTS {
            stop_far(&self.dir, host.0);
        }
        self.kept.clear();
        if std::env::var_os("KHOR_TEST_KEEP").is_none() {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }
}

/// (alias, takes our key, fake `kill -0`): `target` is an ordinary host,
/// `jump` a jump, `pw` takes only a password, `fragile` cannot keep a
/// resident (its `kill -0` always says no, so install.sh reports the
/// serve dead and the add falls back).
const HOSTS: &[(&str, bool, bool)] = &[
    ("target", true, false),
    ("jump", true, false),
    ("pw", false, false),
    ("fragile", true, true),
];

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port()
}

fn far_home(dir: &Path, host: &str) -> PathBuf {
    dir.join(host).join("home")
}

fn stop_far(dir: &Path, host: &str) {
    let pid = far_home(dir, host).join(".khor").join("serve.pid");
    if let Ok(p) = fs::read_to_string(pid) {
        let p = p.trim();
        if !p.is_empty() {
            let _ = Command::new("kill").arg(p).stderr(Stdio::null()).status();
            std::thread::sleep(Duration::from_millis(500));
            let _ = Command::new("kill").args(["-9", p]).stderr(Stdio::null()).status();
        }
    }
}

fn reset_far(dir: &Path, host: &str) {
    stop_far(dir, host);
    let home = far_home(dir, host);
    assert!(home.starts_with(dir), "only a scratch home is emptied");
    let _ = fs::remove_dir_all(&home);
    fs::create_dir_all(&home).unwrap();
}

fn sh(cmd: &mut Command) {
    let st = cmd.status().unwrap();
    assert!(st.success(), "{cmd:?}");
}

/// A static mirror on loopback: answers `/<asset>.gz` with `file`.
fn mirror(file: PathBuf) -> u16 {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for s in l.incoming() {
            let Ok(mut s) = s else { continue };
            let mut buf = [0u8; 2048];
            let n = s.read(&mut buf).unwrap_or(0);
            let req = String::from_utf8_lossy(&buf[..n]);
            let want = file.file_name().unwrap().to_string_lossy().into_owned();
            if req.starts_with(&format!("GET /{want} ")) {
                let body = fs::read(&file).unwrap();
                let _ = write!(s, "HTTP/1.0 200 OK\r\nContent-Length: {}\r\n\r\n", body.len());
                let _ = s.write_all(&body);
            } else {
                let _ = s.write_all(b"HTTP/1.0 404 Not Found\r\nContent-Length: 0\r\n\r\n");
            }
        }
    });
    port
}

fn build() -> Option<World> {
    if !Path::new(SSHD).exists() || !cfg!(target_os = "macos") && !cfg!(target_os = "linux") {
        return None;
    }
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("add-ssh-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let client = dir.join("client");
    fs::create_dir_all(&client).unwrap();
    let mut w = World { dir: dir.clone(), kept: vec![] };
    sh(Command::new("ssh-keygen").args(["-q", "-t", "ed25519", "-N", "", "-C", "add-ssh", "-f"]).arg(client.join("id")));
    let user = String::from_utf8(Command::new("id").arg("-un").output().unwrap().stdout).unwrap();
    let mut config = String::new();
    let mut known = String::new();
    for &(host, takes_key, fragile) in HOSTS {
        let d = dir.join(host);
        let fake = d.join("fakebin");
        fs::create_dir_all(&fake).unwrap();
        fs::create_dir_all(d.join("home")).unwrap();
        sh(Command::new("ssh-keygen").args(["-q", "-t", "ed25519", "-N", "", "-C", host, "-f"]).arg(d.join("hostkey")));
        let keys = d.join("authorized_keys");
        fs::write(&keys, if takes_key { fs::read(client.join("id.pub")).unwrap() } else { vec![] }).unwrap();
        // No keeper on the far side reaches the real launchd or systemd.
        let mut fakes = vec![("launchctl", "exit 1".to_owned()), ("systemctl", "exit 1".into()), ("loginctl", "exit 1".into())];
        if fragile {
            fakes.push(("kill", "[ \"$1\" = -0 ] && exit 1\nexec /bin/kill \"$@\"".into()));
        }
        for (name, body) in fakes {
            let p = fake.join(name);
            fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
            sh(Command::new("chmod").arg("+x").arg(&p));
        }
        let home = d.join("home");
        let wrap = d.join("wrap.sh");
        let w_dir = dir.display();
        fs::write(
            &wrap,
            format!(
                "#!/bin/sh\nexport HOME='{h}' KHOR_HOME='{h}' KHOR_NO_MDNS=1 KHOR_WEB_PORT=0 KHOR_NAME='{host}-box' TMUX_TMPDIR='{d}/tmux' KHOR_TMUX_SOCKET=khor-add-{host} LC_ALL=C\n\
                 export PATH='{f}':\"$PATH\"\n\
                 case \"$HOME\" in '{w_dir}'/*) ;; *) echo 'scratch HOME not set' >&2; exit 97 ;; esac\n\
                 mkdir -p \"$TMUX_TMPDIR\"; cd \"$HOME\" || exit 97\n\
                 if [ -n \"${{SSH_ORIGINAL_COMMAND:-}}\" ]; then exec /bin/sh -c \"$SSH_ORIGINAL_COMMAND\"; else exec /bin/sh; fi\n",
                h = home.display(),
                d = d.display(),
                f = fake.display(),
            ),
        )
        .unwrap();
        sh(Command::new("chmod").arg("+x").arg(&wrap));
        let port = free_port();
        fs::write(
            d.join("sshd_config"),
            format!(
                "Port {port}\nListenAddress 127.0.0.1\nHostKey {d}/hostkey\nPidFile {d}/sshd.pid\nAuthorizedKeysFile {keys}\n\
                 StrictModes no\nUsePAM no\nPasswordAuthentication {pw}\nKbdInteractiveAuthentication no\nPubkeyAuthentication yes\n\
                 AllowTcpForwarding yes\nForceCommand {wrap}\n",
                d = d.display(),
                keys = keys.display(),
                pw = if takes_key { "no" } else { "yes" },
                wrap = wrap.display(),
            ),
        )
        .unwrap();
        w.kept.push(Kept(
            Command::new(SSHD)
                .arg("-D")
                .arg("-f")
                .arg(d.join("sshd_config"))
                .arg("-E")
                .arg(d.join("sshd.log"))
                .spawn()
                .unwrap(),
        ));
        config.push_str(&format!("Host {host}\n  HostName 127.0.0.1\n  Port {port}\n  HostKeyAlias {host}\n"));
        let pubkey = fs::read_to_string(d.join("hostkey.pub")).unwrap();
        let pubkey: Vec<&str> = pubkey.split_whitespace().take(2).collect();
        known.push_str(&format!("{host} {}\n", pubkey.join(" ")));
    }
    // A host nothing listens for.
    config.push_str(&format!("Host dead\n  HostName 127.0.0.1\n  Port {}\n  HostKeyAlias dead\n", free_port()));
    config.push_str(&format!(
        "Host *\n  User {}\n  IdentityFile {}\n  IdentitiesOnly yes\n  UserKnownHostsFile {}\n  StrictHostKeyChecking yes\n",
        user.trim(),
        client.join("id").display(),
        client.join("known_hosts").display(),
    ));
    fs::write(client.join("config"), config).unwrap();
    fs::write(client.join("known_hosts"), known).unwrap();
    std::thread::sleep(Duration::from_millis(800));
    Some(w)
}

/// `khor` as the scratch sponsor.
fn khor(w: &World, mirror: &str, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_khor"))
        .args(args)
        .envs(sponsor_env(w, mirror))
        .env_remove("KHOR_SESSION")
        .output()
        .unwrap()
}

fn sponsor_env(w: &World, mirror: &str) -> Vec<(String, String)> {
    let d = &w.dir;
    [
        ("KHOR_HOME", d.join("sponsor").display().to_string()),
        ("KHOR_NAME", "sponsor".into()),
        ("KHOR_NO_MDNS", "1".into()),
        ("KHOR_WEB_PORT", "0".into()),
        ("TMUX_TMPDIR", d.join("sponsor-tmux").display().to_string()),
        ("KHOR_TMUX_SOCKET", "khor-add-sponsor".into()),
        ("KHOR_SSH_DIR", d.join("client").display().to_string()),
        ("KHOR_MIRROR", mirror.into()),
        ("LC_ALL", "C".into()),
        ("no_proxy", "*".into()),
        ("NO_PROXY", "*".into()),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v))
    .collect()
}

fn text(o: &Output) -> String {
    format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr))
}

/// What the sponsor's table says, one `(name, road)` per row.
fn rows(w: &World, mirror: &str) -> BTreeSet<(String, String)> {
    let out = khor(w, mirror, &["devices"]);
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut f = l.split('\t');
            Some((f.next()?.to_owned(), f.nth(1)?.to_owned()))
        })
        .collect()
}

/// Everything under a scratch home, as relative paths.
fn tree(home: &Path) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut stack = vec![home.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in fs::read_dir(&d).into_iter().flatten().flatten() {
            let p = e.path();
            out.insert(p.strip_prefix(home).unwrap().display().to_string());
            if p.is_dir() && !p.is_symlink() {
                stack.push(p);
            }
        }
    }
    out
}

#[test]
fn add_over_ssh_in_a_scratch_world() {
    let Some(w) = build() else {
        eprintln!("no {SSHD}: skipped");
        return;
    };
    let dir = w.dir.clone();
    // The asset the mirror serves: the binary under test.
    let asset = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => "khor-macos-arm64",
        ("macos", _) => "khor-macos-x86_64",
        ("linux", "aarch64") => "khor-linux-aarch64",
        _ => "khor-linux-x86_64",
    };
    let gz = dir.join(format!("{asset}.gz"));
    sh(Command::new("sh").arg("-c").arg(format!(
        "gzip -c '{}' > '{}'",
        env!("CARGO_BIN_EXE_khor"),
        gz.display()
    )));
    let m = format!("http://127.0.0.1:{}", mirror(gz));
    let dead_mirror = format!("http://127.0.0.1:{}", free_port());

    // **The far HOME is scratch, through the jump too, or nothing runs.**
    for (via, host) in [(None, "target"), (Some("jump"), "target"), (None, "fragile")] {
        let mut c = Command::new("ssh");
        c.args(["-F"]).arg(dir.join("client").join("config")).args(["-o", "BatchMode=yes"]);
        if let Some(j) = via {
            c.args(["-J", j]);
        }
        let home = c.arg(host).arg("echo $HOME").output().unwrap();
        let home = String::from_utf8_lossy(&home.stdout).trim().to_owned();
        assert_eq!(Path::new(&home), far_home(&dir, host), "the far HOME must be the scratch one; refusing to install");
    }

    // The sponsor: a resident on a scratch home.
    fs::create_dir_all(dir.join("sponsor")).unwrap();
    let mut serve = Command::new(env!("CARGO_BIN_EXE_khor"));
    serve
        .arg("serve")
        .envs(sponsor_env(&w, &m))
        .env_remove("KHOR_SESSION")
        .stdout(Stdio::null())
        .stderr(fs::File::create(dir.join("sponsor.log")).unwrap());
    let _sponsor = Kept(serve.spawn().unwrap());
    std::thread::sleep(Duration::from_secs(3));

    // 1. A member, through a jump: fetched from the mirror over there.
    let out = khor(&w, &m, &["add", "target", "box1", "--via", "jump"]);
    let said = text(&out);
    assert!(out.status.success(), "{said}");
    assert!(said.contains("downloaded there") && said.contains("Added box1"), "{said}");
    assert!(far_home(&dir, "target").join(".local/bin/khor").exists());
    assert!(rows(&w, &m).iter().any(|(n, road)| n == "box1" && !road.contains("sponsor")), "{:?}", rows(&w, &m));

    // 2. A satellite through the jump, then 转正: one row by that name,
    //    a member now.
    let _ = khor(&w, &m, &["forget", "box1"]);
    reset_far(&dir, "target");
    let out = khor(&w, &m, &["add", "target", "box2", "--via", "jump", "--satellite"]);
    assert!(out.status.success(), "{}", text(&out));
    assert!(tree(&far_home(&dir, "target")).is_empty(), "a satellite is left nothing");
    assert!(rows(&w, &m).iter().any(|(n, road)| n == "box2" && road.contains("sponsor")));
    let out = khor(&w, &m, &["promote", "box2"]);
    assert!(out.status.success(), "{}", text(&out));
    let named: Vec<_> = rows(&w, &m).into_iter().filter(|(n, _)| n == "box2").collect();
    assert_eq!(named.len(), 1, "the satellite row went: {named:?}");
    assert!(!named[0].1.contains("sponsor"), "box2 speaks for itself now: {named:?}");

    // 3. It cannot fetch: this machine sends its own program.
    let _ = khor(&w, &m, &["forget", "box2"]);
    reset_far(&dir, "target");
    let out = khor(&w, &dead_mirror, &["add", "target", "box3"]);
    let said = text(&out);
    assert!(out.status.success() && said.contains("sent from this machine") && said.contains("Added box3"), "{said}");

    // 4. It cannot keep a resident: everything put there is taken back,
    //    and it is a satellite.
    let before = tree(&far_home(&dir, "fragile"));
    let out = khor(&w, &m, &["add", "fragile", "box4"]);
    let said = text(&out);
    assert!(out.status.success(), "{said}");
    assert!(said.contains("added as a satellite instead"), "{said}");
    assert_eq!(tree(&far_home(&dir, "fragile")), before, "卫星身上不留东西");
    assert!(rows(&w, &m).iter().any(|(n, road)| n == "box4" && road.contains("sponsor")));

    // 5. Password only: said so, with the command that fixes it.
    let out = khor(&w, &m, &["add", "pw"]);
    let said = text(&out);
    assert!(!out.status.success() && said.contains("only takes a password") && said.contains("ssh-copy-id pw"), "{said}");
    let out = khor(&w, &m, &["add", "pw", "--via", "jump"]);
    assert!(text(&out).contains("ssh-copy-id -o ProxyJump=jump pw"), "{}", text(&out));

    // 6. The broken segment, named.
    let out = khor(&w, &m, &["add", "target", "--via", "dead"]);
    assert!(text(&out).contains("This machine can't reach the jump dead"), "{}", text(&out));
    let out = khor(&w, &m, &["add", "dead", "--via", "jump"]);
    assert!(text(&out).contains("The jump jump can't reach dead: channel 0: open failed"), "{}", text(&out));
    let out = khor(&w, &m, &["add", "target", "--via", "pw"]);
    assert!(text(&out).contains("This machine can't reach the jump pw"), "a jump that refuses the key: {}", text(&out));
}
