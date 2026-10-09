//! **QUIC address discovery on khor's own relay** (NET-DIRECT step 2,
//! audit/net-d.md §5.2, h.2 and net-i), on loopback.
//!
//! The lead's ruling (b): QAD is served under the **plain** relay's url
//! (`http://101.200.80.77:3340`, UDP 7842, an IP certificate for QUIC
//! alone), whose TCP is known good, not under the TLS relay
//! (`pk…:3341`), whose TCP is reset in-path from MacLab
//! (audit/pk-3341-dump.txt). The client asks a relay for QAD only on the
//! port its entry names (`Relay::qad`).
//!
//! Its own binary: the environment it sets would clobber relay_https's.

use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use iroh::Watcher as _;
use khor_net::endpoint::{self, Relay};

const PROXY_VARS: [&str; 6] = ["http_proxy", "https_proxy", "HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "all_proxy"];

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A `khor relay`, killed by pid however the test leaves.
struct Running(Child);

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Starts `khor relay` and waits for its banner; the banner's lines are
/// returned so the test can read what it said it serves.
fn relay(home: &Path, args: &[&str]) -> (Running, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_khor"));
    cmd.arg("relay")
        .args(args)
        .arg("--state")
        .arg(home.join("state"))
        .env("KHOR_HOME", home)
        .env_remove("KHOR_RELAY")
        .stdout(Stdio::piped())
        .stderr(std::fs::File::create(home.join("relay.log")).unwrap());
    for k in PROXY_VARS {
        cmd.env_remove(k);
    }
    let mut child = cmd.spawn().unwrap();
    let mut lines = std::io::BufReader::new(child.stdout.take().unwrap()).lines();
    let mut banner = String::new();
    while !banner.contains("relay-stats.json") {
        let Some(Ok(line)) = lines.next() else {
            panic!("khor relay said nothing more: {}", std::fs::read_to_string(home.join("relay.log")).unwrap_or_default());
        };
        banner.push_str(&line);
        banner.push('\n');
    }
    std::thread::spawn(move || lines.for_each(drop));
    (Running(child), banner)
}

fn free_tcp() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port()
}

fn free_udp() -> u16 {
    std::net::UdpSocket::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port()
}

/// A TCP port that takes each connection and closes it at once.
fn dead_tcp() -> u16 {
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    std::thread::spawn(move || l.incoming().for_each(drop));
    port
}

fn home(ep: &iroh::Endpoint) -> Option<String> {
    ep.addr().addrs.iter().find_map(|a| match a {
        iroh::TransportAddr::Relay(u) => Some(u.to_string().trim_end_matches('/').to_owned()),
        _ => None,
    })
}

/// Waits for a net report with a QAD answer in it: the address the relay
/// saw.
async fn learnt(ep: &iroh::Endpoint, within: Duration) -> std::net::SocketAddrV4 {
    let began = Instant::now();
    loop {
        if let Some(a) = ep.net_report().get().and_then(|r| r.global_v4) {
            return a;
        }
        assert!(began.elapsed() < within, "no QAD answer within {within:?}");
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// A self-signed certificate for `127.0.0.1` (an IP, as Beijing's will
/// be), trusted through `KHOR_RELAY_CA`, and the scratch directory.
fn scratch_with_cert(tag: &str) -> (Scratch, PathBuf, PathBuf) {
    // SAFETY (every set_var/remove_var in this binary): set before any
    // endpoint reads it, and the tests here run one at a time
    // (`serial` below).
    for k in PROXY_VARS {
        unsafe { std::env::remove_var(k) };
    }
    let s = Scratch(std::env::temp_dir().join(format!("khor-qad-{tag}-{}", std::process::id())));
    let _ = std::fs::remove_dir_all(&s.0);
    std::fs::create_dir_all(&s.0).unwrap();
    let made = rcgen::generate_simple_self_signed(vec!["127.0.0.1".to_owned()]).unwrap();
    let (cert, key) = (s.0.join("relay.pem"), s.0.join("relay.key"));
    std::fs::write(&cert, made.cert.pem()).unwrap();
    std::fs::write(&key, made.signing_key.serialize_pem()).unwrap();
    unsafe { std::env::set_var(endpoint::RELAY_CA_ENV, &cert) };
    (s, cert, key)
}

/// `khor relay --dev` serving QAD with a certificate for QUIC alone; its
/// url and QUIC port.
fn plain_with_qad(home: &Path, cert: &Path, key: &Path) -> (Running, String, u16) {
    std::fs::create_dir_all(home).unwrap();
    let (port, quic) = (free_tcp(), free_udp());
    let (relay, banner) = relay(
        home,
        &[
            "--dev",
            "--bind",
            &format!("127.0.0.1:{port}"),
            "--quic",
            &format!("127.0.0.1:{quic}"),
            "--tls-cert",
            cert.to_str().unwrap(),
            "--tls-key",
            key.to_str().unwrap(),
        ],
    );
    assert!(banner.contains(&format!("UDP 127.0.0.1:{quic}")), "the banner names the QAD port:\n{banner}");
    assert!(banner.contains(&format!("http://127.0.0.1:{port}")), "the relay itself stays plain:\n{banner}");
    (relay, format!("http://127.0.0.1:{port}"), quic)
}

fn refused(home: &Path, args: &[&str]) -> bool {
    !Command::new(env!("CARGO_BIN_EXE_khor"))
        .arg("relay")
        .args(args)
        .env("KHOR_HOME", home)
        .output()
        .unwrap()
        .status
        .success()
}

static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_plain_relay_answers_qad_under_its_own_url_and_stays_home() {
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (s, cert, key) = scratch_with_cert("plain");
    let (c, k) = (cert.to_str().unwrap(), key.to_str().unwrap());

    // A certificate beside --dev is QUIC's alone: without --quic it is two
    // modes, and --quic without one has nothing to serve with.
    assert!(refused(&s.0, &["--dev", "--quic", "127.0.0.1:0"]), "--dev --quic with no certificate");
    assert!(refused(&s.0, &["--dev", "--tls-cert", c, "--tls-key", k]), "--dev with a certificate and no --quic");

    let (_plain, url, quic) = plain_with_qad(&s.0.join("plain"), &cert, &key);
    let entry = Relay { url: url.clone(), qad: Some(quic), ..Relay::default() };
    let ep = endpoint::bind_with(iroh::SecretKey::generate(), &[entry], false).await.unwrap();
    let began = Instant::now();
    let seen = learnt(&ep, Duration::from_secs(20)).await;
    assert_eq!(*seen.ip(), std::net::Ipv4Addr::LOCALHOST, "the relay saw this machine on loopback");
    eprintln!("[qad] learnt {seen} from {url} (QAD on UDP {quic}) after {:?}", began.elapsed());
    while home(&ep).as_deref() != Some(url.as_str()) {
        assert!(began.elapsed() < Duration::from_secs(20), "not homed on the plain relay: {:?}", home(&ep));
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    // Two periodic reports (20–26 s each) after the first: the QAD
    // connection keeps the home measured, and it stays.
    let held = Instant::now();
    while held.elapsed() < Duration::from_secs(55) {
        assert_eq!(home(&ep).as_deref(), Some(url.as_str()), "the home moved off the plain relay");
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    ep.close().await;

    // The same relay with no QAD port in its entry is asked none: the
    // home still comes, and no address is learnt from it.
    let ep = endpoint::bind_with(iroh::SecretKey::generate(), &[Relay::bare(&url)], false).await.unwrap();
    let began = Instant::now();
    while home(&ep).as_deref() != Some(url.as_str()) {
        assert!(began.elapsed() < Duration::from_secs(20), "not homed without QAD: {:?}", home(&ep));
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert_eq!(ep.net_report().get().and_then(|r| r.global_v4), None, "no QAD asked, none answered");
    ep.close().await;
}

/// **The trap h.2 described, measured** (not a gate): a relay whose UDP
/// answers QAD while its TCP is dead, beside a plain relay with no QAD.
/// iroh ranks the home by the lowest latency of any probe, QAD included,
/// so the walled one may win a home it can never hold. Ruling (b) keeps
/// production out of this shape (QAD only under a url whose TCP passes);
/// this prints what the home does for 60 s.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "measurement of the h.2 trap; red is the finding"]
async fn a_relay_reached_only_by_qad_does_not_win_home() {
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let (s, cert, key) = scratch_with_cert("walled");
    let (_qad, _, quic) = plain_with_qad(&s.0.join("qad"), &cert, &key);
    let plain_home = s.0.join("plain");
    std::fs::create_dir_all(&plain_home).unwrap();
    let plain_port = free_tcp();
    let (_plain, _) = relay(&plain_home, &["--dev", "--bind", &format!("127.0.0.1:{plain_port}")]);
    let plain = format!("http://127.0.0.1:{plain_port}");
    // The QAD server seen through a TCP port that is dead: QAD answers on
    // UDP, the relay never does.
    let walled = format!("http://127.0.0.1:{}", dead_tcp());
    eprintln!("[qad] plain {plain}, walled {walled} (QAD on UDP {quic})");
    let ep = endpoint::bind_with(
        iroh::SecretKey::generate(),
        &[Relay::bare(&plain), Relay { url: walled.clone(), qad: Some(quic), ..Relay::default() }],
        false,
    )
    .await
    .unwrap();
    let seen = learnt(&ep, Duration::from_secs(20)).await;
    eprintln!("[qad] behind the dead TCP port: learnt {seen}");
    let began = Instant::now();
    let mut walled_for = Duration::ZERO;
    let mut last = None;
    while began.elapsed() < Duration::from_secs(60) {
        let now = home(&ep);
        if now != last {
            eprintln!("[qad] {:>5.1} s home {now:?}", began.elapsed().as_secs_f64());
            last = now.clone();
        }
        if now.as_deref() == Some(walled.as_str()) {
            walled_for += Duration::from_millis(250);
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    eprintln!("[qad] homed on the walled relay for {walled_for:?} of 60 s");
    ep.close().await;
    assert_eq!(walled_for, Duration::ZERO, "a relay reached only by QAD became home");
}
