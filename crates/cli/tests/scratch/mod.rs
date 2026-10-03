//! Scratch nodes for the network tests (`wires.rs`, `slow_relay.rs`):
//! temporary homes and identities, a `khor relay --dev` on loopback as
//! the only relay, `khor serve` processes stopped by pid, and a TCP
//! proxy that puts a slow, lossy road in front of that relay.
//!
//! Each test binary compiles this module and uses what it needs.
#![allow(dead_code)]

use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use khor_node::{Hop, Node};
use tokio::time::timeout;

pub struct Scratch(pub PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        // `KHOR_TEST_KEEP`: leave the homes for a person to read.
        if std::env::var_os("KHOR_TEST_KEEP").is_some() {
            return;
        }
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub fn scratch(tag: &str) -> Scratch {
    let p = std::env::temp_dir().join(format!("khor-net-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    Scratch(p)
}

pub const PROXY_VARS: [&str; 6] = [
    "HTTP_PROXY",
    "http_proxy",
    "HTTPS_PROXY",
    "https_proxy",
    "ALL_PROXY",
    "all_proxy",
];

/// A child process, stopped by pid however the test leaves — TERM
/// first, so a serve's keeper takes its serve down with it.
pub struct Reaped(pub Child);

impl Drop for Reaped {
    fn drop(&mut self) {
        let _ = Command::new("kill")
            .args(["-CONT", &self.0.id().to_string()])
            .status();
        let _ = Command::new("kill")
            .args(["-TERM", &self.0.id().to_string()])
            .status();
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            if self.0.try_wait().ok().flatten().is_some() {
                return;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

pub fn khor(home: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_khor"));
    cmd.env("KHOR_HOME", home)
        .env("KHOR_WEB_PORT", "0")
        .env_remove("KHOR_RELAY")
        .env_remove("KHOR_SESSION");
    for k in PROXY_VARS {
        cmd.env_remove(k);
    }
    cmd
}

/// `khor relay --dev` on a free loopback port; its url.
pub fn local_relay(dir: &Path) -> (Reaped, String) {
    let mut child = khor(dir)
        .args(["relay", "--dev", "--bind", "127.0.0.1:0", "--state"])
        .arg(dir.join("state"))
        .stdout(Stdio::piped())
        .stderr(std::fs::File::create(dir.join("relay.log")).unwrap())
        .spawn()
        .unwrap();
    let mut lines = std::io::BufReader::new(child.stdout.take().unwrap()).lines();
    let first = lines.next().expect("a banner line").unwrap();
    let at = first.find("http://").expect("a url on the first line");
    let url = first[at..].split_whitespace().next().unwrap().to_string();
    // **Keep reading what it prints after the first line.** Dropping the
    // pipe here left the relay writing its next line into a closed pipe
    // and resetting every connection after it — invisible while two nodes
    // on loopback went direct, fatal once they had to use the relay.
    let rest = dir.join("relay.out");
    std::thread::spawn(move || {
        use std::io::Write;
        let mut out = std::fs::File::create(rest).ok();
        for line in lines.map_while(Result::ok) {
            if let Some(f) = out.as_mut() {
                let _ = writeln!(f, "{line}");
            }
        }
    });
    (Reaped(child), url)
}

/// A home that binds with that relay and nothing else.
pub fn home_on(dir: &Path, relay: &str) {
    std::fs::create_dir_all(dir.join(".khor")).unwrap();
    std::fs::write(
        dir.join(".khor").join("relays.json"),
        serde_json::json!({ "relays": [{ "url": relay }], "only_mine": true }).to_string(),
    )
    .unwrap();
}

pub fn serve(dir: &Path, name: &str) -> Reaped {
    serve_with(dir, name, &[])
}

/// [`serve`] with extra environment (`KHOR_RELAY_ONLY`, say).
pub fn serve_with(dir: &Path, name: &str, env: &[(&str, &str)]) -> Reaped {
    Reaped(
        khor(dir)
            .arg("serve")
            .envs(env.iter().copied())
            .env("KHOR_NAME", name)
            .stdout(Stdio::null())
            .stderr(
                std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(dir.join("serve.log"))
                    .unwrap(),
            )
            .spawn()
            .unwrap(),
    )
}

/// The resident's own pid, from its endpoint file (the process this
/// test spawned may be a keeper in front of it).
pub fn serve_pid(dir: &Path) -> Option<i32> {
    let f: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.join(".khor/endpoint.json")).ok()?).ok()?;
    f["pid"].as_i64().map(|p| p as i32)
}

pub fn signal(pid: i32, sig: &str) {
    let _ = Command::new("kill").args([sig, &pid.to_string()]).status();
}

pub async fn stood_up(dirs: &[&Path]) {
    timeout(Duration::from_secs(90), async {
        while !dirs.iter().all(|d| d.join(".khor/endpoint.json").exists()) {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("every serve stands up");
}

/// A relay, two serves, paired; the nodes the verbs go through.
pub struct Pair {
    pub _relay: Reaped,
    pub a: Option<Reaped>,
    pub _b: Reaped,
    pub ra: Scratch,
    pub rb: Scratch,
    pub _rr: Scratch,
    pub alpha: Node,
    pub beta: Node,
}

pub async fn pair(tag: &str) -> Pair {
    pair_on(tag, None).await.0
}

/// [`pair`], optionally with every byte between the two forced through
/// the relay (`KHOR_RELAY_ONLY`) and the relay behind a [`slow_road`].
pub async fn pair_on(
    tag: &str,
    road: Option<Impair>,
) -> (Pair, Option<tokio::task::JoinHandle<()>>) {
    let (ra, rb, rr) = (
        scratch(&format!("{tag}-a")),
        scratch(&format!("{tag}-b")),
        scratch(&format!("{tag}-r")),
    );
    let (relay, direct_url) = local_relay(&rr.0);
    let (url, proxy) = match road {
        Some(impair) => {
            let to: std::net::SocketAddr = direct_url
                .trim_start_matches("http://")
                .trim_end_matches('/')
                .parse()
                .unwrap();
            let (at, task) = slow_road(to, impair).await;
            (format!("http://{at}"), Some(task))
        }
        None => (direct_url, None),
    };
    home_on(&ra.0, &url);
    home_on(&rb.0, &url);
    let env: &[(&str, &str)] = if road.is_some() {
        &[("KHOR_RELAY_ONLY", "1")]
    } else {
        &[]
    };
    let a = serve_with(&ra.0, "alpha", env);
    let b = serve_with(&rb.0, "beta", env);
    stood_up(&[&ra.0, &rb.0]).await;
    let alpha = Node::open_as(ra.0.clone(), "alpha").unwrap();
    let beta = Node::open_as(rb.0.clone(), "beta").unwrap();
    if road.is_some() {
        // Both have to be on the relay before either can be found there,
        // and over a slow road the first relay probe takes a while.
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
    let ticket = alpha.invite().unwrap();
    let began = Instant::now();
    let peer = loop {
        let tried = timeout(Duration::from_secs(90), beta.pair(&ticket))
            .await
            .expect("pairing must not hang");
        match tried {
            Ok(peer) => break peer,
            Err(why) if road.is_some() && began.elapsed() < Duration::from_secs(120) => {
                eprintln!("[pair] {:?} in: {why}", began.elapsed());
            }
            Err(why) => panic!("pairing succeeds: {why}"),
        }
    };
    if road.is_some() {
        eprintln!(
            "[pair] paired over the slow road after {:?}",
            began.elapsed()
        );
    }
    assert_eq!(peer, "alpha");
    (
        Pair {
            _relay: relay,
            a: Some(a),
            _b: b,
            ra,
            rb,
            _rr: rr,
            alpha,
            beta,
        },
        proxy,
    )
}

/// What a [`slow_road`] does to every chunk crossing it.
#[derive(Debug, Clone, Copy)]
pub struct Impair {
    /// Added round trip between the two nodes. Both reach the relay
    /// through the road, so a byte from one to the other crosses it twice:
    /// each crossing holds a quarter of this.
    pub rtt: Duration,
    /// The share of chunks that also wait `stall`, in percent.
    pub loss_pct: u32,
    /// How long a "lost" chunk waits on top: TCP under the relay never
    /// drops a byte, so a loss shows up as a retransmission's wait, and
    /// everything behind it waits too (head-of-line, as on a real link).
    pub stall: Duration,
}

/// **A slow, lossy road in front of a relay**: a loopback TCP proxy that
/// holds every chunk for half the added round trip in each direction,
/// and a share of them a retransmission's wait longer, in order. The
/// relay speaks HTTP and WebSocket over TCP, so this is exactly where a
/// cellular link's delay and loss land for a relayed connection.
pub async fn slow_road(
    to: std::net::SocketAddr,
    impair: Impair,
) -> (std::net::SocketAddr, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .unwrap();
    let at = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        let mut seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
            | 1;
        while let Ok((inbound, _)) = listener.accept().await {
            let Ok(outbound) = tokio::net::TcpStream::connect(to).await else {
                continue;
            };
            let _ = inbound.set_nodelay(true);
            let _ = outbound.set_nodelay(true);
            let (ir, iw) = inbound.into_split();
            let (or, ow) = outbound.into_split();
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            tokio::spawn(hold(ir, ow, impair, seed));
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            tokio::spawn(hold(or, iw, impair, seed));
        }
    });
    (at, task)
}

async fn hold(
    mut from: tokio::net::tcp::OwnedReadHalf,
    mut to: tokio::net::tcp::OwnedWriteHalf,
    impair: Impair,
    mut seed: u64,
) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<(tokio::time::Instant, Vec<u8>)>();
    let writer = tokio::spawn(async move {
        while let Some((at, bytes)) = rx.recv().await {
            tokio::time::sleep_until(at).await;
            if to.write_all(&bytes).await.is_err() {
                return;
            }
        }
        let _ = to.shutdown().await;
    });
    let mut buf = vec![0u8; 16 * 1024];
    let mut last = tokio::time::Instant::now();
    loop {
        let n = match from.read(&mut buf).await {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let stalled = seed % 100 < u64::from(impair.loss_pct);
        let at = tokio::time::Instant::now()
            + impair.rtt / 4
            + if stalled {
                impair.stall
            } else {
                Duration::ZERO
            };
        last = last.max(at);
        if tx.send((last, buf[..n].to_vec())).is_err() {
            break;
        }
    }
    drop(tx);
    let _ = writer.await;
}

/// Waits until beta holds a wire to alpha that reads `want`.
pub async fn until_reads(beta: &Node, alpha_id: &str, want: Hop, within: Duration) -> Duration {
    let started = Instant::now();
    loop {
        let links = beta.links().await.unwrap_or_default();
        if links.iter().any(|(id, l)| id == alpha_id && l.hop == want) {
            return started.elapsed();
        }
        assert!(
            started.elapsed() < within,
            "beta never read {want:?} for alpha within {within:?}: {links:?}"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
