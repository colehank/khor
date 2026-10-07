//! Round ROAD-2 T5: khor inside ssh (`khor carry`, `khor _carry`).
//!
//! Alpha and beta are scratch residents, paired over a loopback relay.
//! Then beta is cut off: restarted on a relay nobody runs, with no direct
//! road (`KHOR_RELAY_ONLY`), so iroh reaches it from nowhere. Alpha
//! carries it over a real, unprivileged `sshd` on a high loopback port
//! whose `ForceCommand` wrapper puts the far HOME on beta's scratch home
//! (and refuses to run anywhere else). The ssh files are scratch ones
//! (`KHOR_SSH_DIR`); keys are generated here; no password road is used
//! against this sshd (it would check the real account password).

mod scratch;

use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use khor_node::Hop;
use scratch::*;
use tokio::time::timeout;

/// The scratch ssh world: sshd on `port`, its wrapper, a client config
/// with one host, `member`, whose far HOME is `far_home`.
fn ssh_world(dir: &Path, port: u16, far_home: &Path) {
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
    let (d, h) = (dir.display(), far_home.display());
    let scratch_root = far_home.parent().unwrap().display();
    let wrap = dir.join("far.sh");
    std::fs::write(
        &wrap,
        format!(
            "#!/bin/sh\nexport HOME='{h}' KHOR_NO_MDNS=1 KHOR_WEB_PORT=0 TMUX_TMPDIR='{d}/tmux' KHOR_TMUX_SOCKET=khor-carry LC_ALL=C\n\
             case \"$HOME\" in '{scratch_root}'/*) ;; *) echo 'scratch HOME not set' >&2; exit 97 ;; esac\n\
             exec sh -c \"$SSH_ORIGINAL_COMMAND\"\n"
        ),
    )
    .unwrap();
    Command::new("chmod").arg("755").arg(&wrap).status().unwrap();
    std::fs::write(
        dir.join("sshd_config"),
        format!(
            "Port {port}\nListenAddress 127.0.0.1\nHostKey {d}/host_key\nPidFile {d}/sshd.pid\n\
             AuthorizedKeysFile {d}/authorized_keys\nPasswordAuthentication no\nKbdInteractiveAuthentication no\n\
             UsePAM no\nStrictModes no\nForceCommand {}\n",
            wrap.display()
        ),
    )
    .unwrap();
    let host_pub = std::fs::read_to_string(dir.join("host_key.pub")).unwrap();
    std::fs::write(dir.join("known_hosts"), format!("[127.0.0.1]:{port} {}", host_pub.trim())).unwrap();
    std::fs::write(
        dir.join("config"),
        format!(
            "Host member\n  HostName 127.0.0.1\n  Port {port}\n  IdentityFile {d}/client_key\n  IdentitiesOnly yes\n  \
             IdentityAgent none\n  UserKnownHostsFile {d}/known_hosts\n  GlobalKnownHostsFile /dev/null\n  \
             StrictHostKeyChecking yes\n"
        ),
    )
    .unwrap();
    // The member's khor where `khor add` puts it.
    let bin = far_home.join(".local/bin");
    std::fs::create_dir_all(&bin).unwrap();
    std::os::unix::fs::symlink(env!("CARGO_BIN_EXE_khor"), bin.join("khor")).unwrap();
}

/// Beta with no road of its own: no direct transport, its own relay a
/// port nobody holds, and every relay — its own or a peer's home relay,
/// which iroh dials from the peer's address — behind a proxy nobody runs.
const CUT_OFF: &[(&str, &str)] = &[
    ("KHOR_RELAY_ONLY", "1"),
    ("HTTP_PROXY", "http://127.0.0.1:9"),
    ("http_proxy", "http://127.0.0.1:9"),
    ("HTTPS_PROXY", "http://127.0.0.1:9"),
    ("https_proxy", "http://127.0.0.1:9"),
];

async fn until<F, Fut>(within: Duration, what: &str, mut f: F) -> Duration
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let started = Instant::now();
    while !f().await {
        assert!(started.elapsed() < within, "{what} within {within:?}");
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    started.elapsed()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_machine_that_reaches_nothing_is_carried_over_ssh() {
    if !Path::new("/usr/sbin/sshd").exists() {
        eprintln!("no /usr/sbin/sshd here: skipped");
        return;
    }
    let (ra, rb, rr) = (scratch("carry-a"), scratch("carry-b"), scratch("carry-r"));
    let (_relay, url) = local_relay(&rr.0);
    home_on(&ra.0, &url);
    home_on(&rb.0, &url);
    let world = rr.0.join("ssh-world");
    let port = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap().local_addr().unwrap().port();
    ssh_world(&world, port, &rb.0);
    let ssh_dir = world.display().to_string();
    let _a = serve_with(&ra.0, "alpha", &[("KHOR_SSH_DIR", &ssh_dir)]);
    let b = serve(&rb.0, "beta");
    stood_up(&[&ra.0, &rb.0]).await;
    let alpha = khor_node::Node::open_as(ra.0.clone(), "alpha").unwrap();
    let beta = khor_node::Node::open_as(rb.0.clone(), "beta").unwrap();
    let ticket = alpha.invite().unwrap();
    timeout(Duration::from_secs(90), beta.pair(&ticket))
        .await
        .expect("pairing must not hang")
        .expect("pairing succeeds");
    let (a_id, b_id) = (alpha.device_str().to_owned(), beta.device_str().to_owned());

    // Beta, cut off ([`CUT_OFF`]).
    let pid = serve_pid(&rb.0).unwrap();
    drop(b);
    signal(pid, "-TERM");
    until(Duration::from_secs(20), "beta's resident stops", || async {
        !Command::new("kill").args(["-0", &pid.to_string()]).status().is_ok_and(|s| s.success())
    })
    .await;
    let _ = std::fs::remove_file(rb.0.join(".khor/endpoint.json"));
    home_on(&rb.0, "http://127.0.0.1:9");
    let _b = serve_with(&rb.0, "beta", CUT_OFF);
    stood_up(&[&rb.0]).await;
    let sshd = Reaped(
        Command::new("/usr/sbin/sshd")
            .args(["-D", "-e", "-f"])
            .arg(world.join("sshd_config"))
            .stderr(std::fs::File::create(world.join("sshd.log")).unwrap())
            .spawn()
            .unwrap(),
    );
    until(Duration::from_secs(10), "sshd listens", || async {
        tokio::net::TcpStream::connect(("127.0.0.1", port)).await.is_ok()
    })
    .await;
    // Control: cut off, alpha cannot list beta.
    tokio::time::sleep(Duration::from_secs(3)).await;
    let control = timeout(Duration::from_secs(20), alpha.ls_of("beta", "/")).await;
    assert!(
        control.as_ref().map_or(true, |r| r.is_err()),
        "control: beta must be unreachable before it is carried: {:?} {:?} / {}",
        control.map(|r| r.map(|(c, e, _)| (c, e.iter().map(|e| e.name.clone()).collect::<Vec<_>>()))),
        alpha.links().await.unwrap_or_default().into_iter().find(|(i, _)| *i == b_id),
        std::fs::read_to_string(rb.0.join("serve.log")).unwrap_or_default()
    );

    // Carried: alpha's ssh runs `khor _carry` on beta.
    let out = khor(&ra.0).args(["carry", "beta", "member"]).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let started = Instant::now();
    while alpha.ls_of("beta", "/").await.is_err() {
        if started.elapsed() > Duration::from_secs(60) {
            let log = |p: &Path| std::fs::read_to_string(p).unwrap_or_default();
            panic!(
                "alpha never listed beta over the carried road\n-- sshd --\n{}\n-- alpha --\n{}\n-- beta --\n{}",
                log(&world.join("sshd.log")),
                log(&ra.0.join("serve.log")),
                log(&rb.0.join("serve.log"))
            );
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    let took = started.elapsed();
    eprintln!("[carry] alpha reached beta {took:?} after `khor carry`");
    until_reads(&alpha, &b_id, Hop::Ssh, Duration::from_secs(20)).await;
    // And the other way: beta's own resident reaches alpha through it.
    until(Duration::from_secs(60), "beta lists alpha over the carried road", || async {
        beta.ls_of("alpha", "/").await.is_ok()
    })
    .await;
    until_reads(&beta, &a_id, Hop::Ssh, Duration::from_secs(20)).await;

    // The far end goes (beta's resident restarts, so `khor _carry` loses
    // it and the ssh session ends): the carrier reconnects on its own.
    let pid = serve_pid(&rb.0).unwrap();
    drop(_b);
    signal(pid, "-TERM");
    until(Duration::from_secs(20), "beta's resident stops again", || async {
        !Command::new("kill").args(["-0", &pid.to_string()]).status().is_ok_and(|s| s.success())
    })
    .await;
    let _ = std::fs::remove_file(rb.0.join(".khor/endpoint.json"));
    let _b = serve_with(&rb.0, "beta", CUT_OFF);
    stood_up(&[&rb.0]).await;
    let back = until(Duration::from_secs(90), "alpha lists beta again after the far end restarts", || async {
        alpha.ls_of("beta", "/").await.is_ok()
    })
    .await;
    eprintln!("[carry] reached again {back:?} after beta's resident came back");
    drop(sshd);

    // Off: alpha stops carrying, and beta is out of reach again.
    let out = khor(&ra.0).args(["carry", "beta", "--off"]).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
}
