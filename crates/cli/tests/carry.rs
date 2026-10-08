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
/// with one host, `member`, whose far HOME is `far_home`. A `far.env` in
/// `dir`, when there, is sourced by the wrapper: what the far login's
/// environment adds ([`FAR_CUT_OFF`]).
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
             [ -f '{d}/far.env' ] && . '{d}/far.env'\n\
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

/// The far login behind the same dead proxy, for `khor _relay-check`.
const FAR_CUT_OFF: &str = "export HTTP_PROXY=http://127.0.0.1:9 http_proxy=http://127.0.0.1:9 \
                           HTTPS_PROXY=http://127.0.0.1:9 https_proxy=http://127.0.0.1:9\n";

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
    // A scratch secret store for every askpass and sweep alpha runs.
    let secrets = rr.0.join("secrets").display().to_string();
    let _a = serve_with(&ra.0, "alpha", &[("KHOR_SSH_DIR", &ssh_dir), ("KHOR_SECRET_DIR", &secrets)]);
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

    // Round ROAD-4: `add`'s login flags. The key file is only a path on
    // the row; the passphrase, read from standard input, is kept in the
    // (scratch) store under the road's name, never on the row.
    let bad = khor(&ra.0).args(["carry", "beta", "member", "--password", "--key", "k"]).output().unwrap();
    assert!(!bad.status.success(), "a password and a key at once is refused");
    let key = world.join("client_key").display().to_string();
    let mut keyed = khor(&ra.0)
        .args(["carry", "beta", "member", "--key", &key, "--passphrase"])
        .env("KHOR_SECRET_DIR", &secrets)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    {
        use std::io::Write;
        keyed.stdin.take().unwrap().write_all(b"scratch-pass\n").unwrap();
    }
    let out = keyed.wait_with_output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let row = alpha.devices().unwrap().into_iter().find(|d| d.id == b_id).expect("beta's row");
    assert_eq!(row.ssh_identity.as_deref(), Some(key.as_str()), "the key file, as a path");
    assert!(row.ssh_passphrase && !row.ssh_password);
    let door: String = "member".bytes().map(|b| format!("{b:02x}")).collect();
    assert!(Path::new(&secrets).join(door).exists(), "the passphrase is kept under the road's name");

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
    // Round ROAD-4: a reader of the table alone, for a machine with no
    // road of its own to gamma, composes one through its carrier.
    let composed = alpha.with_satellite_roads(&a_id, vec![khor_node::Road::own(b_id.clone(), khor_node::RoadKind::Direct, None, Some(1))]);
    let g = composed.iter().find(|r| r.to == c_id).expect("a road to gamma through beta");
    assert_eq!((g.road, g.via.as_deref()), (khor_node::RoadKind::Direct, Some(b_id.as_str())));

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

    let (home, id, far_env) = (ra.0.clone(), b_id.clone(), world.join("far.env"));
    let (reaches, cut, paired, started) = std::thread::spawn(move || {
        // ssh here reads the scratch client config, as alpha's resident's does.
        unsafe { std::env::set_var("KHOR_SSH_DIR", &ssh_dir) };
        let n = khor_node::Node::open_as(home, "alpha").unwrap();
        let to = khor_node::satellite::Dest::new("member", None);
        // Round ROAD-4, what `khor add` asks before 配对: control, the
        // far login reaches alpha's relay; then, behind the dead proxy,
        // it reaches none, and says so inside the bound.
        let reaches = n.far_reaches_a_relay(&to);
        std::fs::write(&far_env, FAR_CUT_OFF).unwrap();
        let asked = Instant::now();
        let cut = n.far_reaches_a_relay(&to);
        let cut = (cut, asked.elapsed());
        let started = Instant::now();
        (reaches, cut, n.pair_carried(&to, &id), started)
    })
    .join()
    .unwrap();
    eprintln!("[carry] the far side said it reaches no relay in {:?}", cut.1);
    assert_eq!(reaches, Some(true), "control: the far login reaches alpha's relay");
    assert_eq!(cut.0, Some(false), "behind the dead proxy it reaches none");
    assert!(cut.1 < khor_node::join::RELAY_ASK, "and says so inside the bound");
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

/// **A carried road that ends hands back to the relay at once**
/// (NET-DIRECT, vendor/iroh patch 13). Alpha and beta meet on a loopback
/// relay behind a 200 ms slow road, beta on the relay only; alpha carries
/// beta over ssh, which at about 1 ms takes the connection. Then sshd
/// stops listening (no reconnect) and alpha's ssh session is killed: the
/// link ends, and alpha's word for beta reads 中继 within a few seconds,
/// the wire never down. Before, iroh kept the dead carried path selected
/// on its last round trip until its 15 s path idle. When sshd listens
/// again, the reconnect takes the connection back.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_ended_ssh_session_hands_the_connection_to_the_relay_at_once() {
    if !Path::new("/usr/sbin/sshd").exists() {
        eprintln!("no /usr/sbin/sshd here: skipped");
        return;
    }
    let (ra, rb, rr) = (scratch("carry-end-a"), scratch("carry-end-b"), scratch("carry-end-r"));
    let (_relay, direct_url) = local_relay(&rr.0);
    let to: std::net::SocketAddr = direct_url
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .parse()
        .unwrap();
    let (at, _road) = slow_road(
        to,
        Impair {
            rtt: Duration::from_millis(200),
            loss_pct: 0,
            stall: Duration::ZERO,
        },
    )
    .await;
    let url = format!("http://{at}");
    home_on(&ra.0, &url);
    home_on(&rb.0, &url);
    let world = rr.0.join("ssh-world");
    let port = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap().local_addr().unwrap().port();
    ssh_world(&world, port, &rb.0);
    let ssh_dir = world.display().to_string();
    let secrets = rr.0.join("secrets").display().to_string();
    let _a = serve_with(&ra.0, "alpha", &[("KHOR_SSH_DIR", &ssh_dir), ("KHOR_SECRET_DIR", &secrets)]);
    let _b = serve_with(&rb.0, "beta", &[("KHOR_RELAY_ONLY", "1")]);
    stood_up(&[&ra.0, &rb.0]).await;
    let alpha = khor_node::Node::open_as(ra.0.clone(), "alpha").unwrap();
    let beta = khor_node::Node::open_as(rb.0.clone(), "beta").unwrap();
    let ticket = alpha.invite().unwrap();
    timeout(Duration::from_secs(90), beta.pair(&ticket))
        .await
        .expect("pairing must not hang")
        .expect("pairing succeeds");
    let b_id = beta.device_str().to_owned();
    until(Duration::from_secs(60), "alpha lists beta over the relay", || async {
        alpha.ls_of("beta", "/").await.is_ok()
    })
    .await;
    until_reads(&alpha, &b_id, Hop::Relay, Duration::from_secs(30)).await;

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
    let out = khor(&ra.0).args(["carry", "beta", "member"]).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let took = until_reads(&alpha, &b_id, Hop::Ssh, Duration::from_secs(60)).await;
    eprintln!("[carry-end] carried {took:?} after `khor carry`");

    // No reconnect: sshd stops listening (the session it forked lives on).
    drop(sshd);
    let a_pid = serve_pid(&ra.0).expect("alpha's resident");
    let ps = Command::new("ps").args(["-A", "-o", "pid=,ppid=,comm="]).output().unwrap();
    let ssh_pid = String::from_utf8_lossy(&ps.stdout)
        .lines()
        .filter_map(|l| {
            let mut f = l.split_whitespace();
            let (pid, ppid, comm) = (f.next()?.parse::<i32>().ok()?, f.next()?.parse::<i32>().ok()?, f.next()?);
            (ppid == a_pid && (comm == "ssh" || comm.ends_with("/ssh"))).then_some(pid)
        })
        .next()
        .expect("alpha's ssh session");
    signal(ssh_pid, "-KILL");
    let back = until_reads(&alpha, &b_id, Hop::Relay, Duration::from_secs(60)).await;
    eprintln!("[carry-end] back on the relay {back:?} after the ssh session ended");
    let after = alpha.links().await.unwrap().into_iter().find(|(i, _)| *i == b_id).map(|(_, l)| l);
    assert!(after.as_ref().is_some_and(|l| l.down_ms.is_none()), "the wire never went down: {after:?}");
    assert!(
        back < Duration::from_secs(8),
        "an ended ssh session hands back to the relay in a few seconds, not iroh's 15 s path idle: {back:?}"
    );

    // sshd listens again: the carrier's reconnect is offered to the live
    // connection (its carried path was closed), which goes back to ssh.
    let _sshd = Reaped(
        Command::new("/usr/sbin/sshd")
            .args(["-D", "-e", "-f"])
            .arg(world.join("sshd_config"))
            .stderr(std::fs::File::create(world.join("sshd-again.log")).unwrap())
            .spawn()
            .unwrap(),
    );
    let again = until_reads(&alpha, &b_id, Hop::Ssh, Duration::from_secs(90)).await;
    eprintln!("[carry-end] carried again {again:?} after sshd came back");
    let out = khor(&ra.0).args(["carry", "beta", "--off"]).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
}
