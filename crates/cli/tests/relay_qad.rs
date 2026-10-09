//! **QUIC address discovery on khor's own relay** (NET-DIRECT step 2,
//! audit/net-d.md §5.2 and h.2), on loopback.
//!
//! **Prep, not built yet (net-h, 2026-10-09).** `#[ignore]`d until a lane
//! builds it and reshapes it for the lead's ruling (b): QAD is served
//! under the **plain** relay's url (`http://101.200.80.77:3340`, UDP 7842,
//! an IP certificate), whose TCP is known good, not under the TLS relay
//! (`pk…:3341`), whose TCP is reset in-path from MacLab
//! (audit/pk-3341-dump.txt). What it checks as written:
//! - `khor relay --quic` with `--dev` is refused (for (b) this changes:
//!   `--dev` takes `--quic` together with a certificate for QUIC alone);
//! - a client told by `KHOR_RELAY_QAD` learns its address from the relay;
//! - a successor that answered only QAD — its TCP dead — does not become
//!   home. vendor/iroh patch 5 now counts only an HTTPS answer; iroh's own
//!   ranking (lowest latency of any probe, QAD included) may still pick it.
//!   That is the trap h.2 describes, which ruling (b) avoids; this case
//!   measures it and is expected red until then.
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "net-h prep: not built yet; reshape for ruling (b), audit/net-d.md h.2"]
async fn the_relay_answers_qad_and_a_successor_reached_only_by_qad_is_not_home() {
    // SAFETY (every set_var/remove_var here): set before any endpoint
    // reads it, and nothing else in this binary writes it meanwhile.
    for k in PROXY_VARS {
        unsafe { std::env::remove_var(k) };
    }
    let s = Scratch(std::env::temp_dir().join(format!("khor-qad-{}", std::process::id())));
    let _ = std::fs::remove_dir_all(&s.0);
    let (plain_home, tls_home) = (s.0.join("plain"), s.0.join("tls"));
    std::fs::create_dir_all(&plain_home).unwrap();
    std::fs::create_dir_all(&tls_home).unwrap();
    let made = rcgen::generate_simple_self_signed(vec!["127.0.0.1".to_owned()]).unwrap();
    let (cert, key) = (s.0.join("relay.pem"), s.0.join("relay.key"));
    std::fs::write(&cert, made.cert.pem()).unwrap();
    std::fs::write(&key, made.signing_key.serialize_pem()).unwrap();
    unsafe { std::env::set_var(endpoint::RELAY_CA_ENV, &cert) };

    // Without a certificate there is no QUIC to serve.
    let refused = Command::new(env!("CARGO_BIN_EXE_khor"))
        .args(["relay", "--dev", "--quic", "127.0.0.1:0"])
        .env("KHOR_HOME", &plain_home)
        .output()
        .unwrap();
    assert!(!refused.status.success(), "--quic with --dev must be refused");

    let plain_port = free_tcp();
    let (_plain, _) = relay(&plain_home, &["--dev", "--bind", &format!("127.0.0.1:{plain_port}")]);
    let plain = format!("http://127.0.0.1:{plain_port}");
    let (tls_port, quic_port) = (free_tcp(), free_udp());
    let (_tls, banner) = relay(
        &tls_home,
        &[
            "--tls-cert",
            cert.to_str().unwrap(),
            "--tls-key",
            key.to_str().unwrap(),
            "--bind",
            &format!("127.0.0.1:{tls_port}"),
            "--http",
            "127.0.0.1:0",
            "--quic",
            &format!("127.0.0.1:{quic_port}"),
        ],
    );
    assert!(banner.contains(&format!("UDP 127.0.0.1:{quic_port}")), "the banner names the QAD port:\n{banner}");
    let tls = format!("https://127.0.0.1:{tls_port}");
    // The same relay seen through a TCP port that is dead: QAD still
    // answers on UDP, HTTPS never does.
    let walled = format!("https://127.0.0.1:{}", dead_tcp());
    unsafe {
        std::env::set_var("KHOR_RELAY_ONLY", "1");
        std::env::set_var(endpoint::RELAY_QAD_ENV, format!("{tls}|{quic_port},{walled}|{quic_port}"));
        std::env::set_var(endpoint::RELAY_SUCCESSOR_ENV, format!("{plain}|{tls},{plain}|{walled}"));
    }

    // Reached both ways: QAD teaches the address, and home is the
    // successor as before.
    let open = endpoint::bind_preferring(iroh::SecretKey::generate(), &[Relay::bare(&plain), Relay::bare(&tls)], false, None)
        .await
        .unwrap();
    let seen = learnt(&open, Duration::from_secs(20)).await;
    assert_eq!(*seen.ip(), std::net::Ipv4Addr::LOCALHOST, "the relay saw this machine on loopback");
    let began = Instant::now();
    while home(&open).as_deref() != Some(&tls) {
        assert!(began.elapsed() < Duration::from_secs(60), "not homed on the TLS relay: {:?}", home(&open));
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    eprintln!("[qad] learnt {seen} from khor relay --quic; home {tls} after {:?}", began.elapsed());

    // Reached by QAD only: the address is learnt, the home stays plain.
    let walled_ep =
        endpoint::bind_preferring(iroh::SecretKey::generate(), &[Relay::bare(&plain), Relay::bare(&walled)], false, None)
            .await
            .unwrap();
    let seen = learnt(&walled_ep, Duration::from_secs(20)).await;
    eprintln!("[qad] behind the dead TCP port: learnt {seen}");
    let began = Instant::now();
    while home(&walled_ep).is_none() {
        assert!(began.elapsed() < Duration::from_secs(10), "no home after the first report");
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    // Two periodic reports (20–26 s each) after the first.
    let began = Instant::now();
    while began.elapsed() < Duration::from_secs(55) {
        assert_eq!(home(&walled_ep).as_deref(), Some(plain.as_str()), "a successor reached only by QAD became home");
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    open.close().await;
    walled_ep.close().await;
}
