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

/// Round ROAD-3 T6: **the peer hop.** Gamma is cut off as beta is above
/// and carried by beta over ssh; alpha has no road to gamma of its own.
/// Alpha reaches it through beta: beta's roadbook says it reaches gamma
/// (`ssh`), alpha reaches beta, and beta splices alpha's stream to its
/// own stream to gamma. Control: with alpha's `peer-hop` off, gamma
/// stays out of reach though beta's road is in the table.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_third_machine_reaches_a_carried_member_through_its_carrier() {
    if !Path::new("/usr/sbin/sshd").exists() {
        eprintln!("no /usr/sbin/sshd here: skipped");
        return;
    }
    let (ra, rb, rc, rr) = (scratch("hop-a"), scratch("hop-b"), scratch("hop-c"), scratch("hop-r"));
    let (_relay, url) = local_relay(&rr.0);
    for r in [&ra, &rb, &rc] {
        home_on(&r.0, &url);
    }
    let world = rr.0.join("ssh-world");
    let port = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap().local_addr().unwrap().port();
    ssh_world(&world, port, &rc.0);
    let ssh_dir = world.display().to_string();
    // The control first: alpha asks for no hop (read live, so it can be
    // turned on below without a restart; the transport is registered
    // because `ssh-carry` is on).
    let out = khor(&ra.0).args(["config", "set", "peer-hop", "off"]).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let _a = serve(&ra.0, "alpha");
    let _b = serve_with(&rb.0, "beta", &[("KHOR_SSH_DIR", &ssh_dir)]);
    let c = serve(&rc.0, "gamma");
    stood_up(&[&ra.0, &rb.0, &rc.0]).await;
    let alpha = khor_node::Node::open_as(ra.0.clone(), "alpha").unwrap();
    let beta = khor_node::Node::open_as(rb.0.clone(), "beta").unwrap();
    let gamma = khor_node::Node::open_as(rc.0.clone(), "gamma").unwrap();
    for joiner in [&beta, &gamma] {
        let ticket = alpha.invite().unwrap();
        timeout(Duration::from_secs(90), joiner.pair(&ticket))
            .await
            .expect("pairing must not hang")
            .expect("pairing succeeds");
    }
    let ids = [&alpha, &beta, &gamma].map(|n| n.device_str().to_owned());
    let [a_id, b_id, c_id] = ids.clone();
    for n in [&alpha, &beta, &gamma] {
        until(Duration::from_secs(60), "every machine knows the other two", || async {
            let known = n.devices().unwrap_or_default();
            ids.iter().all(|id| known.iter().any(|d| &d.id == id))
        })
        .await;
    }

    // Gamma, cut off, and carried by beta.
    let pid = serve_pid(&rc.0).unwrap();
    drop(c);
    signal(pid, "-TERM");
    until(Duration::from_secs(20), "gamma's resident stops", || async {
        !Command::new("kill").args(["-0", &pid.to_string()]).status().is_ok_and(|s| s.success())
    })
    .await;
    let _ = std::fs::remove_file(rc.0.join(".khor/endpoint.json"));
    home_on(&rc.0, "http://127.0.0.1:9");
    let _c = serve_with(&rc.0, "gamma", CUT_OFF);
    stood_up(&[&rc.0]).await;
    let _sshd = Reaped(
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
    let out = khor(&rb.0).args(["carry", "gamma", "member"]).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    until(Duration::from_secs(60), "beta lists gamma over its ssh", || async {
        beta.ls_of("gamma", "/").await.is_ok()
    })
    .await;
    // Beta's road to gamma in the table, as alpha reads it.
    let wrote = until(Duration::from_secs(90), "alpha reads beta's ssh road to gamma", || async {
        let all = alpha.devices().unwrap_or_default();
        all.iter()
            .find(|d| d.id == b_id)
            .is_some_and(|b| alpha.roads_of(b).iter().any(|r| r.to == c_id && r.road == khor_node::RoadKind::Ssh))
    })
    .await;
    eprintln!("[hop] beta's road to gamma reached alpha's table {wrote:?} after gamma was carried");

    // Control: no hop asked, no road.
    let control = timeout(Duration::from_secs(20), alpha.ls_of("gamma", "/")).await;
    assert!(
        control.as_ref().map_or(true, |r| r.is_err()),
        "control: with peer-hop off alpha must not reach gamma"
    );

    let out = khor(&ra.0).args(["config", "set", "peer-hop", "on"]).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let started = Instant::now();
    while alpha.ls_of("gamma", "/").await.is_err() {
        if started.elapsed() > Duration::from_secs(60) {
            let log = |p: &Path| std::fs::read_to_string(p).unwrap_or_default();
            panic!(
                "alpha never listed gamma through beta\n-- alpha --\n{}\n-- beta --\n{}\n-- gamma --\n{}",
                log(&ra.0.join("serve.log")),
                log(&rb.0.join("serve.log")),
                log(&rc.0.join("serve.log"))
            );
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    eprintln!("[hop] alpha reached gamma through beta {:?} after peer-hop on", started.elapsed());
    until_reads(&alpha, &c_id, Hop::Forwarded, Duration::from_secs(20)).await;
    // Gamma's own resident reaches alpha back over the same hop.
    until(Duration::from_secs(60), "gamma lists alpha through beta", || async {
        gamma.ls_of("alpha", "/").await.is_ok()
    })
    .await;
    until_reads(&gamma, &a_id, Hop::Forwarded, Duration::from_secs(20)).await;
    // Alpha writes its road to gamma with the middle named.
    let named = until(Duration::from_secs(60), "alpha's road to gamma names beta", || async {
        let all = alpha.devices().unwrap_or_default();
        all.iter().find(|d| d.id == a_id).is_some_and(|me| {
            alpha.roads_of(me).iter().any(|r| r.to == c_id && r.via.as_deref() == Some(b_id.as_str()))
        })
    })
    .await;
    eprintln!("[hop] alpha's register named beta as the middle {named:?} after it settled");
}

/// Round ROAD-3, the add flow's fallback (`Node::pair_carried`, called by
/// `khor add` when khor runs there but did not connect back): beta has
/// khor and a resident but no road at all, and is not paired. Alpha's
/// resident carries it over ssh before it is anyone's (`ipc::Op::
/// CarryFor`; beta's resident takes a link from a machine not in its
/// table), the far `khor pair -` rides that link, and once paired alpha
/// carries it for good (`carrier` on the row).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_machine_that_cannot_connect_back_pairs_over_the_ssh_it_was_added_by() {
    if !Path::new("/usr/sbin/sshd").exists() {
        eprintln!("no /usr/sbin/sshd here: skipped");
        return;
    }
    let (ra, rb, rr) = (scratch("pairc-a"), scratch("pairc-b"), scratch("pairc-r"));
    let (_relay, url) = local_relay(&rr.0);
    home_on(&ra.0, &url);
    home_on(&rb.0, "http://127.0.0.1:9");
    let world = rr.0.join("ssh-world");
    let port = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap().local_addr().unwrap().port();
    ssh_world(&world, port, &rb.0);
    let ssh_dir = world.display().to_string();
    let _a = serve_with(&ra.0, "alpha", &[("KHOR_SSH_DIR", &ssh_dir)]);
    let _b = serve_with(&rb.0, "beta", CUT_OFF);
    stood_up(&[&ra.0, &rb.0]).await;
    let _sshd = Reaped(
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
    let alpha = khor_node::Node::open_as(ra.0.clone(), "alpha").unwrap();
    let b_id = khor_node::Node::open_as(rb.0.clone(), "beta").unwrap().device_str().to_owned();
    let a_id = alpha.device_str().to_owned();

    let (home, id) = (ra.0.clone(), b_id.clone());
    let started = Instant::now();
    let paired = std::thread::spawn(move || {
        // ssh here reads the scratch client config, as alpha's resident's does.
        unsafe { std::env::set_var("KHOR_SSH_DIR", &ssh_dir) };
        let n = khor_node::Node::open_as(home, "alpha").unwrap();
        n.pair_carried(&khor_node::satellite::Dest::new("member", None), &id)
    })
    .join()
    .unwrap();
    let log = |p: &Path| std::fs::read_to_string(p).unwrap_or_default();
    assert!(
        paired,
        "beta paired over the carried road\n-- sshd --\n{}\n-- alpha --\n{}\n-- beta --\n{}",
        log(&world.join("sshd.log")),
        log(&ra.0.join("serve.log")),
        log(&rb.0.join("serve.log"))
    );
    eprintln!("[carry] paired over the carried road in {:?}", started.elapsed());
    let row = alpha.devices().unwrap().into_iter().find(|d| d.id == b_id).expect("beta's row");
    assert_eq!(row.carrier.as_deref(), Some(a_id.as_str()), "alpha carries beta for good");
    assert_eq!(row.ssh_host.as_deref(), Some("member"));
    until(Duration::from_secs(60), "alpha lists beta over the carried road", || async {
        alpha.ls_of("beta", "/").await.is_ok()
    })
    .await;
    until_reads(&alpha, &b_id, Hop::Ssh, Duration::from_secs(20)).await;
}
