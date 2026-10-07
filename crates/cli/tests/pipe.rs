//! Round ROAD-1 T2: ssh inside khor (`khor pipe --via <machine>`).
//!
//! Two scratch residents paired over a loopback relay. Beta pipes to a
//! loopback listener **through alpha's exit** — through its own resident
//! (the hand-off port and cookie) and as the verb ssh's `ProxyCommand`
//! runs. The second test puts a real, unprivileged `sshd` on a high
//! loopback port and has a real `ssh` log in through the pipe. Every ssh
//! file is a scratch one; nothing reads a real `~/.ssh`.

mod scratch;

use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

use scratch::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::timeout;

/// A listener that says a banner, then echoes until the input ends and
/// hangs up — the shape of a server that outlives its client's last byte.
async fn echo() -> u16 {
    let l = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let port = l.local_addr().unwrap().port();
    tokio::spawn(async move {
        while let Ok((s, _)) = l.accept().await {
            tokio::spawn(async move {
                let (mut r, mut w) = s.into_split();
                let _ = w.write_all(b"SSH-2.0-Echo\r\n").await;
                let mut got = Vec::new();
                let _ = r.read_to_end(&mut got).await;
                let _ = w.write_all(&got).await;
                let _ = w.shutdown().await;
            });
        }
    });
    port
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_pipe_through_another_machine_carries_both_ways_and_says_why_it_failed() {
    let port = echo().await;
    let p = pair("pipe").await;

    // Through beta's resident: the hand-off port, the cookie.
    let mut out = Vec::new();
    timeout(
        Duration::from_secs(30),
        p.beta.pipe_via("alpha", "127.0.0.1", port, &b"ping\n"[..], &mut out),
    )
    .await
    .expect("the pipe ends when the far end hangs up")
    .expect("the pipe opens");
    assert_eq!(String::from_utf8_lossy(&out), "SSH-2.0-Echo\r\nping\n");

    // A port nobody holds: refused in words, before any byte moves.
    let closed = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap().local_addr().unwrap().port();
    let mut none = Vec::new();
    let why = p
        .beta
        .pipe_via("alpha", "127.0.0.1", closed, &b""[..], &mut none)
        .await
        .expect_err("nothing there to pipe to");
    assert!(why.contains(&format!("127.0.0.1:{closed}")), "names the target: {why}");

    // The verb, as ssh runs it: stdin in, stdout out, exits when the far
    // end hangs up.
    let mut child = khor(&p.rb.0)
        .args(["pipe", "--via", "alpha", "127.0.0.1", &port.to_string()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"from the verb\n").unwrap();
    let mut got = String::new();
    child.stdout.take().unwrap().read_to_string(&mut got).unwrap();
    let status = child.wait().unwrap();
    assert!(status.success(), "{status:?}");
    assert_eq!(got, "SSH-2.0-Echo\r\nfrom the verb\n");

    // With no resident on beta, the verb binds an endpoint of its own and
    // dials alpha's exit itself.
    drop(p._b);
    timeout(Duration::from_secs(20), async {
        while serve_pid(&p.rb.0).is_some_and(|pid| Command::new("kill").args(["-0", &pid.to_string()]).status().is_ok_and(|s| s.success())) {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("beta's resident stops");
    let mut child = khor(&p.rb.0)
        .args(["pipe", "--via", "alpha", "127.0.0.1", &port.to_string()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"no resident\n").unwrap();
    let mut got = String::new();
    child.stdout.take().unwrap().read_to_string(&mut got).unwrap();
    let mut err = String::new();
    child.stderr.take().unwrap().read_to_string(&mut err).unwrap();
    assert!(child.wait().unwrap().success(), "{err}");
    assert_eq!(got, "SSH-2.0-Echo\r\nno resident\n", "{err}");
}

/// A scratch sshd world: host key, the client's key, its authorized_keys,
/// a client config whose ProxyCommand is `khor pipe --via alpha`.
fn ssh_world(dir: &Path, port: u16, khor_home: &Path) -> std::path::PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let keygen = |name: &str| {
        let ok = Command::new("ssh-keygen")
            .args(["-q", "-t", "ed25519", "-N", "", "-C", "", "-f"])
            .arg(dir.join(name))
            .status()
            .unwrap();
        assert!(ok.success());
    };
    keygen("host_key");
    keygen("client_key");
    std::fs::copy(dir.join("client_key.pub"), dir.join("authorized_keys")).unwrap();
    let d = dir.display();
    std::fs::write(
        dir.join("sshd_config"),
        format!(
            "Port {port}\nListenAddress 127.0.0.1\nHostKey {d}/host_key\nPidFile {d}/sshd.pid\n\
             AuthorizedKeysFile {d}/authorized_keys\nPasswordAuthentication no\nKbdInteractiveAuthentication no\n\
             UsePAM no\nStrictModes no\n"
        ),
    )
    .unwrap();
    let host_pub = std::fs::read_to_string(dir.join("host_key.pub")).unwrap();
    std::fs::write(dir.join("known_hosts"), format!("[127.0.0.1]:{port} {}", host_pub.trim())).unwrap();
    std::fs::write(
        dir.join("config"),
        format!(
            "Host intranet\n  HostName 127.0.0.1\n  Port {port}\n  IdentityFile {d}/client_key\n  IdentitiesOnly yes\n  \
             IdentityAgent none\n  UserKnownHostsFile {d}/known_hosts\n  GlobalKnownHostsFile /dev/null\n  \
             StrictHostKeyChecking yes\n  BatchMode yes\n  \
             ProxyCommand env KHOR_HOME={} KHOR_NO_MDNS=1 {} pipe --via alpha %h %p\n",
            khor_home.display(),
            env!("CARGO_BIN_EXE_khor")
        ),
    )
    .unwrap();
    dir.join("config")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_real_ssh_logs_in_through_another_machines_exit() {
    if !Path::new("/usr/sbin/sshd").exists() {
        eprintln!("no /usr/sbin/sshd here: skipped");
        return;
    }
    let port = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap().local_addr().unwrap().port();
    let p = pair("pipe-ssh").await;
    let world = p.rb.0.join("ssh-world");
    let config = ssh_world(&world, port, &p.rb.0);
    let sshd = Reaped(
        Command::new("/usr/sbin/sshd")
            .args(["-D", "-e", "-f"])
            .arg(world.join("sshd_config"))
            .stderr(std::fs::File::create(world.join("sshd.log")).unwrap())
            .spawn()
            .unwrap(),
    );
    timeout(Duration::from_secs(10), async {
        while tokio::net::TcpStream::connect(("127.0.0.1", port)).await.is_err() {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("sshd never listened: {}", std::fs::read_to_string(world.join("sshd.log")).unwrap_or_default()));

    // Waited on with a deadline and killed by pid: a pipe that carries
    // nothing leaves ssh waiting for sshd's two-minute grace.
    let child = Command::new("ssh")
        .arg("-F")
        .arg(&config)
        .args(["intranet", "echo", "through-the-exit"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let pid = child.id();
    let waited = std::thread::spawn(move || child.wait_with_output());
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    while !waited.is_finished() && std::time::Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    if !waited.is_finished() {
        let _ = Command::new("kill").arg(pid.to_string()).status();
    }
    let out = waited.join().unwrap().unwrap();
    let said = String::from_utf8_lossy(&out.stdout);
    let err = String::from_utf8_lossy(&out.stderr);
    drop(sshd);
    assert!(
        out.status.success() && said.trim() == "through-the-exit",
        "ssh through alpha's exit: {said} / {err} / sshd: {}",
        std::fs::read_to_string(world.join("sshd.log")).unwrap_or_default()
    );
}
