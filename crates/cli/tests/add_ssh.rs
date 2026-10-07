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
//! **The password road is tested on a key's passphrase.** An
//! unprivileged sshd checks a password against the real account, so a
//! generated test password cannot log in anywhere. ssh asks for a key's
//! passphrase through the same `SSH_ASKPASS` round trip it uses for a
//! password, so the `lock` host takes only a passphrase-protected key,
//! and khor answers its askpass from a scratch store (`KHOR_SECRET_DIR`)
//! — never the Keychain.
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
    ports: Vec<(&'static str, u16)>,
    /// The `lock` key's passphrase: generated, never printed.
    phrase: String,
}

impl World {
    fn port(&self, host: &str) -> u16 {
        self.ports.iter().find(|(h, _)| *h == host).unwrap().1
    }
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
/// serve dead and the add falls back), `lock` takes only the
/// passphrase-protected key (the module head).
const HOSTS: &[(&str, bool, bool)] = &[
    ("target", true, false),
    ("jump", true, false),
    ("pw", false, false),
    ("fragile", true, true),
    ("lock", true, false),
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
    let mut b = [0u8; 16];
    fs::File::open("/dev/urandom").unwrap().read_exact(&mut b).unwrap();
    let phrase: String = b.iter().map(|x| format!("{x:02x}")).collect();
    let mut w = World { dir: dir.clone(), kept: vec![], ports: vec![], phrase };
    sh(Command::new("ssh-keygen").args(["-q", "-t", "ed25519", "-N", "", "-C", "add-ssh", "-f"]).arg(client.join("id")));
    sh(Command::new("ssh-keygen")
        .args(["-q", "-t", "ed25519", "-C", "add-ssh-lock", "-N"])
        .arg(&w.phrase)
        .arg("-f")
        .arg(client.join("lock")));
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
        let key = if host == "lock" { "lock.pub" } else { "id.pub" };
        fs::write(&keys, if takes_key { fs::read(client.join(key)).unwrap() } else { vec![] }).unwrap();
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
        if host == "lock" {
            config.push_str(&format!("  IdentityFile {}\n", client.join("lock").display()));
        }
        let pubkey = fs::read_to_string(d.join("hostkey.pub")).unwrap();
        let pubkey: Vec<&str> = pubkey.split_whitespace().take(2).collect();
        known.push_str(&format!("{host} {}\n", pubkey.join(" ")));
        // `target` by address and port too, the way ssh keys a port
        // that is not 22.
        if host == "target" {
            known.push_str(&format!("[127.0.0.1]:{port} {}\n", pubkey.join(" ")));
        }
        w.ports.push((host, port));
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
    node_env(w, mirror, "sponsor")
}

/// A scratch khor node's environment: its own home, name and tmux
/// socket, the scratch ssh world, mirror and secret store.
fn node_env(w: &World, mirror: &str, who: &str) -> Vec<(String, String)> {
    let d = &w.dir;
    [
        ("KHOR_HOME", d.join(who).display().to_string()),
        ("KHOR_NAME", who.to_owned()),
        ("KHOR_NO_MDNS", "1".into()),
        ("KHOR_WEB_PORT", "0".into()),
        ("TMUX_TMPDIR", d.join(format!("{who}-tmux")).display().to_string()),
        ("KHOR_TMUX_SOCKET", format!("khor-add-{who}")),
        ("KHOR_SSH_DIR", d.join("client").display().to_string()),
        ("KHOR_SECRET_DIR", d.join("secrets").display().to_string()),
        ("KHOR_MIRROR", mirror.into()),
        ("LC_ALL", "C".into()),
        ("no_proxy", "*".into()),
        ("NO_PROXY", "*".into()),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v))
    .collect()
}

/// `khor` with `input` on its standard input.
fn khor_in(w: &World, mirror: &str, args: &[&str], input: &str) -> Output {
    let mut c = Command::new(env!("CARGO_BIN_EXE_khor"))
        .args(args)
        .envs(sponsor_env(w, mirror))
        .env_remove("KHOR_SESSION")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    c.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    c.wait_with_output().unwrap()
}

/// How many passwords the scratch store holds.
fn kept(w: &World) -> usize {
    fs::read_dir(w.dir.join("secrets")).map(|d| d.count()).unwrap_or(0)
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
    // …and `lock`'s, through a test askpass that knows the passphrase.
    let ask = dir.join("ask.sh");
    fs::write(&ask, format!("#!/bin/sh\necho '{}'\n", w.phrase)).unwrap();
    sh(Command::new("chmod").arg("700").arg(&ask));
    let home = Command::new("ssh")
        .arg("-F")
        .arg(dir.join("client").join("config"))
        .args(["-o", "BatchMode=no", "lock", "echo $HOME"])
        .env("SSH_ASKPASS", &ask)
        .env("SSH_ASKPASS_REQUIRE", "force")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let home = String::from_utf8_lossy(&home.stdout).trim().to_owned();
    assert_eq!(Path::new(&home), far_home(&dir, "lock"), "the far HOME must be the scratch one; refusing to install");
    fs::remove_file(&ask).unwrap();

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
    assert!(said.contains("downloaded by the target host") && said.contains("Added box1"), "{said}");
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
    assert!(out.status.success() && said.contains("uploaded from this machine") && said.contains("Added box3"), "{said}");

    // 4. It cannot keep a resident: everything put there is taken back,
    //    and it is a satellite.
    let before = tree(&far_home(&dir, "fragile"));
    let out = khor(&w, &m, &["add", "fragile", "box4"]);
    let said = text(&out);
    assert!(out.status.success(), "{said}");
    assert!(said.contains("connected over SSH instead"), "{said}");
    assert_eq!(tree(&far_home(&dir, "fragile")), before, "卫星身上不留东西");
    assert!(rows(&w, &m).iter().any(|(n, road)| n == "box4" && road.contains("sponsor")));

    // 5. The key is refused: said so, with the command that fixes it.
    let out = khor(&w, &m, &["add", "pw"]);
    let said = text(&out);
    assert!(!out.status.success() && said.contains("did not accept this machine's key") && said.contains("ssh-copy-id pw"), "{said}");
    let out = khor(&w, &m, &["add", "pw", "--via", "jump"]);
    assert!(text(&out).contains("ssh-copy-id -o ProxyJump=jump pw"), "{}", text(&out));

    // 6. The broken segment, named.
    let out = khor(&w, &m, &["add", "target", "--via", "dead"]);
    assert!(text(&out).contains("This machine can't connect to the jump host dead"), "{}", text(&out));
    let out = khor(&w, &m, &["add", "dead", "--via", "jump"]);
    assert!(
        text(&out).contains("The jump host jump can't connect to the target host dead: channel 0: open failed"),
        "{}",
        text(&out)
    );
    let out = khor(&w, &m, &["add", "target", "--via", "pw"]);
    assert!(text(&out).contains("This machine can't connect to the jump host pw"), "a jump that refuses the key: {}", text(&out));

    // 7. A port: the address and port alone, no ssh config entry; the
    //    id comes from known_hosts' `[127.0.0.1]:port` line.
    let _ = khor(&w, &m, &["forget", "box3"]);
    reset_far(&dir, "target");
    let port = w.port("target").to_string();
    let out = khor(&w, &m, &["add", "127.0.0.1", "box7", "--port", &port, "--satellite"]);
    assert!(out.status.success(), "{}", text(&out));
    assert!(rows(&w, &m).iter().any(|(n, road)| n == "box7" && road.contains("sponsor")));
    let _ = khor(&w, &m, &["forget", "box7"]);

    // 8. The password road. Keys alone are refused…
    let out = khor(&w, &m, &["add", "lock"]);
    assert!(text(&out).contains("did not accept this machine's key"), "{}", text(&out));
    // …a wrong password is said to be wrong, and is not kept…
    let out = khor_in(&w, &m, &["add", "lock", "box8", "--password", "--satellite"], "not-it\n");
    assert!(!out.status.success() && text(&out).contains("did not accept the saved password"), "{}", text(&out));
    assert_eq!(kept(&w), 0, "a password that failed is not kept");
    // …the right one makes a satellite, kept while the row is…
    let out = khor_in(&w, &m, &["add", "lock", "box8", "--password", "--satellite"], &format!("{}\n", w.phrase));
    assert!(out.status.success(), "{}", text(&out));
    assert!(rows(&w, &m).iter().any(|(n, road)| n == "box8" && road.contains("sponsor")));
    assert_eq!(kept(&w), 1, "a satellite's password is kept");
    assert!(!text(&out).contains(&w.phrase), "the password is never printed");
    let out = khor(&w, &m, &["forget", "box8"]);
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(kept(&w), 0, "forgetting the machine forgets its password");
    // …and a member installed over it, which no longer needs it.
    reset_far(&dir, "lock");
    let out = khor_in(&w, &m, &["add", "lock", "box9", "--password"], &format!("{}\n", w.phrase));
    let said = text(&out);
    assert!(out.status.success() && said.contains("Added box9"), "{said}");
    assert_eq!(kept(&w), 0, "a member's password is dropped once it is in");
    // Every far host is this one machine, and a far serve runs without
    // KHOR_HOME, so the next far khor would take box9's serve for one on
    // its own store (`serves::beside`): one far resident at a time.
    reset_far(&dir, "lock");

    // 9. From another khor machine: `origin`, paired with the sponsor,
    //    runs the add over its own ssh road and is the sponsor; this
    //    machine watches its steps.
    fs::create_dir_all(dir.join("origin")).unwrap();
    let mut serve = Command::new(env!("CARGO_BIN_EXE_khor"));
    serve
        .arg("serve")
        .envs(node_env(&w, &m, "origin"))
        .env_remove("KHOR_SESSION")
        .stdout(Stdio::null())
        .stderr(fs::File::create(dir.join("origin.log")).unwrap());
    let _origin = Kept(serve.spawn().unwrap());
    std::thread::sleep(Duration::from_secs(3));
    let ticket = khor(&w, &m, &["invite"]);
    assert!(ticket.status.success(), "invite: {}", String::from_utf8_lossy(&ticket.stderr));
    let ticket = String::from_utf8_lossy(&ticket.stdout).lines().next().unwrap_or("").to_owned();
    let paired = Command::new(env!("CARGO_BIN_EXE_khor"))
        .args(["pair", "-"])
        .envs(node_env(&w, &m, "origin"))
        .env_remove("KHOR_SESSION")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|mut c| {
            c.stdin.take().unwrap().write_all(format!("{ticket}\n").as_bytes())?;
            c.wait_with_output()
        })
        .unwrap();
    drop(ticket);
    assert!(paired.status.success(), "origin pairs: {}", String::from_utf8_lossy(&paired.stderr));
    let out = khor_in(&w, &m, &["add", "lock", "-m", "origin", "--password"], "x\n");
    assert!(!out.status.success() && text(&out).contains("key login only"), "{}", text(&out));
    reset_far(&dir, "target");
    let out = khor(&w, &m, &["add", "target", "box10", "--via", "jump", "-m", "origin"]);
    let said = text(&out);
    assert!(out.status.success() && said.contains("Added box10"), "{said}");
    assert!(said.contains("Install khor") && said.contains("Pair"), "the steps came back: {said}");
    assert!(far_home(&dir, "target").join(".local/bin/khor").exists());
}
