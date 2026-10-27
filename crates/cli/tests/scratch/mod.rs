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
        reap_hosts(&self.0);
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Stops every session host recorded under a scratch home, by the pids
/// in its own host files — a session a test opened outlives the test's
/// serves, and must not outlive the test.
pub fn reap_hosts(home: &Path) {
    let Ok(dirs) = std::fs::read_dir(home.join(".khor").join("sessions")) else {
        return;
    };
    for d in dirs.flatten() {
        let Ok(text) = std::fs::read_to_string(d.path().join("host.json")) else {
            continue;
        };
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else {
            continue;
        };
        for key in ["child_pid", "host_pid"] {
            if let Some(pid) = v[key].as_i64().filter(|p| *p > 1) {
                let _ = Command::new("kill")
                    .args(["-TERM", &pid.to_string()])
                    .status();
            }
        }
    }
}

pub fn scratch(tag: &str) -> Scratch {
    own_tmux();
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

/// **This test process's own tmux, never the person's.** A scratch node
/// on the default socket can list, and attach to, the real sessions on
/// this machine (2026-10-03: a probe line was typed into the owner's own
/// pane that way). Every process the harness starts, and this one, talk
/// to a server under a directory of their own (`TMUX_TMPDIR`) on a
/// socket of their own (`KHOR_TMUX_SOCKET`), and are not told they sit
/// inside a tmux (`TMUX`, `TMUX_PANE`). The directory is short: a
/// socket path past ~104 bytes cannot be bound.
pub fn own_tmux() -> &'static Path {
    static DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    DIR.get_or_init(|| {
        // One fixed directory for every test run rather than one per
        // process: nothing accumulates, and no test here starts a tmux
        // server on it, only asks it.
        let dir = std::env::temp_dir().join("khor-test-tmux");
        std::fs::create_dir_all(&dir).unwrap();
        // SAFETY: once per test binary, before any session is opened
        // from it; the values only ever point at this process's own.
        unsafe {
            std::env::set_var("TMUX_TMPDIR", &dir);
            std::env::set_var("KHOR_TMUX_SOCKET", TMUX_SOCKET);
            std::env::remove_var("TMUX");
            std::env::remove_var("TMUX_PANE");
        }
        dir
    })
}

/// The scratch tmux socket's name (`own_tmux`).
pub const TMUX_SOCKET: &str = "khor-scratch";

/// **No scratch node on the segment** (`KHOR_NO_MDNS`): every machine on
/// this Wi-Fi decodes every node's mDNS announcement, the person's own app
/// included (term-cpu.md). Set for this process's own binds and handed to
/// every process the harness starts ([`khor_from`]).
pub fn off_the_segment() {
    static DONE: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    DONE.get_or_init(|| {
        // SAFETY: once per test binary, before its first bind; the value
        // only ever turns this process's own announcing off.
        unsafe { std::env::set_var(khor_net::endpoint::NO_MDNS_ENV, "1") };
    });
}

pub fn khor(home: &Path) -> Command {
    khor_from(home, Path::new(env!("CARGO_BIN_EXE_khor")))
}

/// [`khor`], from another build of khor — an older one, for a test of
/// two versions talking (`sync_mixed.rs`).
pub fn khor_from(home: &Path, bin: &Path) -> Command {
    off_the_segment();
    let mut cmd = Command::new(bin);
    cmd.env("KHOR_HOME", home)
        .env("KHOR_WEB_PORT", "0")
        .env(khor_net::endpoint::NO_MDNS_ENV, "1")
        .env("TMUX_TMPDIR", own_tmux())
        .env("KHOR_TMUX_SOCKET", TMUX_SOCKET)
        .env_remove("TMUX")
        .env_remove("TMUX_PANE")
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
    serve_from(dir, name, env, Path::new(env!("CARGO_BIN_EXE_khor")))
}

/// [`serve_with`], from another build of khor.
pub fn serve_from(dir: &Path, name: &str, env: &[(&str, &str)], bin: &Path) -> Reaped {
    Reaped(
        khor_from(dir, bin)
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
    pair_built(tag, road, Path::new(env!("CARGO_BIN_EXE_khor"))).await
}

/// [`pair_on`], alpha's resident from another build of khor.
pub async fn pair_built(
    tag: &str,
    road: Option<Impair>,
    alpha_bin: &Path,
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
    let a = serve_from(&ra.0, "alpha", env, alpha_bin);
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
            tokio::spawn(hold(ir, ow, impair, seed, &ROAD_TO_RELAY, None));
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            tokio::spawn(hold(or, iw, impair, seed, &ROAD_FROM_RELAY, None));
        }
    });
    (at, task)
}

/// A [`slow_road`] that can be **cut**: every connection through it
/// closed at once and new ones refused until [`Cut::heal`] — an answer
/// in flight is lost, not late (round NET-7, the create made once).
#[allow(dead_code)]
#[derive(Clone, Default)]
pub struct Cut(std::sync::Arc<std::sync::atomic::AtomicBool>);

#[allow(dead_code)]
impl Cut {
    pub fn cut(&self) {
        self.0.store(true, std::sync::atomic::Ordering::SeqCst);
    }
    pub fn heal(&self) {
        self.0.store(false, std::sync::atomic::Ordering::SeqCst);
    }
    fn is_cut(&self) -> bool {
        self.0.load(std::sync::atomic::Ordering::SeqCst)
    }
}

/// [`slow_road`], with a [`Cut`] that closes it.
#[allow(dead_code)]
pub async fn cuttable_road(
    to: std::net::SocketAddr,
    impair: Impair,
) -> (std::net::SocketAddr, Cut, tokio::task::JoinHandle<()>) {
    let cut = Cut::default();
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .unwrap();
    let at = listener.local_addr().unwrap();
    let held = cut.clone();
    let task = tokio::spawn(async move {
        let mut seed = 0x9E37_79B9_7F4A_7C15u64;
        while let Ok((inbound, _)) = listener.accept().await {
            if held.is_cut() {
                continue;
            }
            let Ok(outbound) = tokio::net::TcpStream::connect(to).await else {
                continue;
            };
            let _ = inbound.set_nodelay(true);
            let _ = outbound.set_nodelay(true);
            let (ir, iw) = inbound.into_split();
            let (or, ow) = outbound.into_split();
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            tokio::spawn(hold(
                ir,
                ow,
                impair,
                seed,
                &ROAD_TO_RELAY,
                Some(held.clone()),
            ));
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            tokio::spawn(hold(
                or,
                iw,
                impair,
                seed,
                &ROAD_FROM_RELAY,
                Some(held.clone()),
            ));
        }
    });
    (at, cut, task)
}

/// Bytes every [`slow_road`] in this process has carried towards its
/// relay, and back from it — what a probe reads to see traffic a node's
/// own wire count does not (the relay's own protocol, dials that never
/// became a wire).
#[allow(dead_code)]
pub static ROAD_TO_RELAY: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
#[allow(dead_code)]
pub static ROAD_FROM_RELAY: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

async fn hold(
    mut from: tokio::net::tcp::OwnedReadHalf,
    mut to: tokio::net::tcp::OwnedWriteHalf,
    impair: Impair,
    mut seed: u64,
    counter: &'static std::sync::atomic::AtomicU64,
    cut: Option<Cut>,
) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<(tokio::time::Instant, Vec<u8>)>();
    let stop = cut.clone();
    let writer = tokio::spawn(async move {
        while let Some((at, bytes)) = rx.recv().await {
            tokio::time::sleep_until(at).await;
            // Cut: what was still on the road never arrives.
            if stop.as_ref().is_some_and(Cut::is_cut) {
                return;
            }
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
        if cut.as_ref().is_some_and(Cut::is_cut) {
            break;
        }
        counter.fetch_add(n as u64, std::sync::atomic::Ordering::Relaxed);
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
