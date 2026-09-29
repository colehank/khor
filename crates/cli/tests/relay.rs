//! `khor relay` (docs/NET.md 中继, CONNECT.md 自家 `khor relay`): the
//! binary, spawned on a loopback port under a temporary home, with
//! endpoints standing up against it in this process.
//!
//! **Every far side here has no IP transport at all.** The relay is
//! then the only road to it, so "reached it" means "reached it through
//! the relay", and a refused endpoint cannot sneak round by a LAN
//! address mDNS handed it — without that, the refusals below would be
//! controls that pass for the wrong reason.
//!
//! Nothing here touches a relay anybody else uses, a real identity or
//! a real store.

use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use khor_net::endpoint::{self, ALPN, Relay};
use tokio::time::timeout;

/// A scratch tree that takes itself away however the test leaves.
struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn scratch(tag: &str) -> Scratch {
    let p = std::env::temp_dir().join(format!("khor-relay-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    Scratch(p)
}

const PROXY_VARS: [&str; 6] = [
    "HTTP_PROXY",
    "http_proxy",
    "HTTPS_PROXY",
    "https_proxy",
    "ALL_PROXY",
    "all_proxy",
];

/// The loopback in `NO_PROXY` form: what a proxy must never carry.
const LOOPBACK: &str = "127.0.0.1,localhost,::1";

/// The endpoints in this process go through khor's bind, which honours
/// a proxy from the environment — one set in the shell running the
/// tests would carry these loopback relay connections somewhere else.
fn clear_proxies() {
    for k in PROXY_VARS {
        unsafe { std::env::remove_var(k) };
    }
    // **Removing the variables is not enough on macOS**: iroh's relay
    // probes (net_report's HTTP latency and captive-portal checks) are
    // reqwest clients, and the workspace turns on reqwest's
    // `system-proxy`, so with the variables gone they read the proxy out
    // of System Settings — and hyper-util's reader takes the host and
    // port there and **ignores the exceptions list**. On a machine with
    // a system proxy (the owner's: 127.0.0.1:7897) every probe of a
    // loopback relay goes through it; when that proxy is down or will
    // not deliver to loopback, no relay is ever measured, no home relay
    // is chosen, and an endpoint sits with no verdict at all. `NO_PROXY`
    // is the one knob that reader honours.
    unsafe { std::env::set_var("NO_PROXY", LOOPBACK) };
}

/// A running `khor relay`, killed by pid however the test leaves.
struct Running {
    child: Child,
    url: String,
    banner: Vec<String>,
}

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Starts `khor relay --dev` on a free loopback port with `extra`
/// arguments, under `home`, and waits for the banner.
fn relay(home: &Path, extra: &[&str]) -> Running {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_khor"));
    cmd.args(["relay", "--dev", "--bind", "127.0.0.1:0", "--state"])
        .arg(home.join("state"))
        .args(extra)
        .env("KHOR_HOME", home)
        .env_remove("KHOR_RELAY")
        .env_remove("KHOR_SESSION")
        .stdout(Stdio::piped())
        .stderr(std::fs::File::create(home.join("relay.log")).unwrap());
    for k in PROXY_VARS {
        cmd.env_remove(k);
    }
    cmd.env("NO_PROXY", LOOPBACK);
    let mut child = cmd.spawn().expect("khor relay should spawn");
    let stdout = child.stdout.take().unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        for line in std::io::BufReader::new(stdout).lines() {
            let Ok(line) = line else { return };
            if tx.send(line).is_err() {
                return;
            }
        }
    });
    let mut banner = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut url = None;
    // The banner is five or six lines; the url is on the first. Read
    // until the metering line, which is the last one printed.
    while Instant::now() < deadline {
        let Ok(line) = rx.recv_timeout(Duration::from_millis(200)) else {
            if let Ok(Some(status)) = child.try_wait() {
                panic!(
                    "khor relay exited ({status}) before its banner: {}",
                    std::fs::read_to_string(home.join("relay.log")).unwrap_or_default()
                );
            }
            continue;
        };
        if url.is_none()
            && let Some(at) = line.find("http://")
        {
            url = Some(line[at..].split_whitespace().next().unwrap().to_string());
        }
        let done = line.contains("relay-stats.json");
        banner.push(line);
        if done {
            break;
        }
    }
    Running {
        child,
        url: url.unwrap_or_else(|| panic!("no url in the banner: {banner:?}")),
        banner,
    }
}

/// How the endpoint's relay came out: `Ok` once connected, `Err` with
/// the relay's refusal once refused.
async fn verdict(ep: &iroh::Endpoint) -> Result<(), String> {
    use iroh::Watcher as _;
    let mut watch = ep.home_relay_status();
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        for s in watch.get() {
            if s.is_connected() {
                return Ok(());
            }
            if let Some(err) = s.last_error() {
                return Err(format!("{err:#}"));
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Err("no verdict within 20s".to_string())
}

/// An endpoint the relay is the only road to, answering one echo per
/// connection.
async fn far_side(secret: iroh::SecretKey, relays: &[Relay]) -> iroh::Endpoint {
    let ep = iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .secret_key(secret)
        .clear_ip_transports()
        .alpns(vec![ALPN.to_vec()])
        .relay_mode(iroh::endpoint::RelayMode::Custom(endpoint::relay_map(
            relays,
        )))
        .bind()
        .await
        .expect("a relay-only endpoint should bind");
    let server = ep.clone();
    tokio::spawn(async move {
        while let Some(incoming) = server.accept().await {
            let Ok(conn) = incoming.await else { continue };
            tokio::spawn(async move {
                if let Ok((mut send, mut recv)) = conn.accept_bi().await {
                    let got = recv.read_to_end(1024).await.unwrap_or_default();
                    let _ = send.write_all(&got).await;
                    let _ = send.finish();
                    let _ = conn.closed().await;
                }
            });
        }
    });
    ep
}

/// One echo from `from` to `to` through the relay at `url`, or why not.
async fn echo(from: &iroh::Endpoint, to: &iroh::Endpoint, url: &str) -> Result<(), String> {
    let addr = endpoint::dial_addr(&to.addr().id.to_string(), &[], &[url.to_string()])
        .map_err(|e| e.to_string())?;
    let conn = timeout(Duration::from_secs(10), from.connect(addr, ALPN))
        .await
        .map_err(|_| "dial timed out".to_string())?
        .map_err(|e| e.to_string())?;
    let (mut send, mut recv) = conn.open_bi().await.map_err(|e| e.to_string())?;
    send.write_all(b"through khor relay")
        .await
        .map_err(|e| e.to_string())?;
    send.finish().map_err(|e| e.to_string())?;
    let back = recv.read_to_end(1024).await.map_err(|e| e.to_string())?;
    conn.close(0u32.into(), b"done");
    if back == b"through khor relay" {
        Ok(())
    } else {
        Err(format!("the echo came back as {back:?}"))
    }
}

/// `--dev`: two endpoints that have nothing but this relay in common
/// reach each other through it.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_dev_relay_carries_two_endpoints_to_each_other() {
    clear_proxies();
    let home = scratch("dev");
    let r = relay(&home.0, &[]);
    assert!(
        r.url.starts_with("http://127.0.0.1:"),
        "the banner names where it listens: {:?}",
        r.banner
    );
    let relays = [Relay::bare(&r.url)];
    let far = far_side(iroh::SecretKey::generate(), &relays).await;
    assert_eq!(verdict(&far).await, Ok(()), "an open relay admits the far side");
    let near = endpoint::bind(iroh::SecretKey::generate(), &relays)
        .await
        .unwrap();
    assert_eq!(verdict(&near).await, Ok(()), "and khor's own bind");
    assert_eq!(echo(&near, &far, &r.url).await, Ok(()));
    near.close().await;
    far.close().await;
}

/// `--token`: an endpoint without the key is refused at the relay and
/// reaches nobody through it; one with the key gets through.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_token_relay_refuses_the_keyless_and_carries_the_keyed() {
    clear_proxies();
    const KEY: &str = "relay-gate-key";
    let home = scratch("token");
    let r = relay(&home.0, &["--token", KEY]);
    let keyed = [Relay {
        url: r.url.clone(),
        token: Some(KEY.into()),
    }];
    let far = far_side(iroh::SecretKey::generate(), &keyed).await;
    assert_eq!(verdict(&far).await, Ok(()), "the far side holds the key");

    let stranger = endpoint::bind(iroh::SecretKey::generate(), &[Relay::bare(&r.url)])
        .await
        .unwrap();
    let refused = verdict(&stranger).await;
    assert!(
        refused.is_err(),
        "an endpoint without the key must be refused, got {refused:?}"
    );
    assert!(
        echo(&stranger, &far, &r.url).await.is_err(),
        "and must reach nobody through this relay"
    );
    stranger.close().await;

    let wrong = endpoint::bind(
        iroh::SecretKey::generate(),
        &[Relay {
            url: r.url.clone(),
            token: Some("not-the-key".into()),
        }],
    )
    .await
    .unwrap();
    assert!(
        verdict(&wrong).await.is_err(),
        "a wrong key is no key"
    );
    wrong.close().await;

    let near = endpoint::bind(iroh::SecretKey::generate(), &keyed)
        .await
        .unwrap();
    assert_eq!(verdict(&near).await, Ok(()), "the right key is admitted");
    assert_eq!(echo(&near, &far, &r.url).await, Ok(()));
    near.close().await;
    far.close().await;
}

/// `--only-mine`: the devices this network trusts get in with no key —
/// **by the roster, not the device table** (CONNECT.md 名单是信任的唯一
/// 出处). The controls are the three ways of not being trusted: not in
/// the table at all, in the table but waiting (not on the roster), and
/// on the roster once but revoked since. Each is refused and reaches
/// nobody.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_only_mine_relay_admits_the_roster_and_nobody_else() {
    clear_proxies();
    let home = scratch("mine");
    let [a, b, waiting, revoked] = std::array::from_fn(|_| iroh::SecretKey::generate());
    // A fresh home makes its own root at first open and holds it; the
    // other machines are written into its table the way a merged table
    // lands on disk (`crates/node/tests/avatar.rs` does the same), then
    // approved — or not — by that root.
    let node = khor_node::Node::open_as(home.0.clone(), "relay-home").expect("the home opens");
    assert!(node.holds_root(), "a fresh home holds its own root");
    {
        use khor_sync::devices::{DeviceDoc, devices_dir};
        use khor_sync::store::load;
        let loaded = load::<DeviceDoc>(&devices_dir(&home.0), 0x51).unwrap();
        for (key, name) in [(&a, "alpha"), (&b, "beta"), (&waiting, "gamma"), (&revoked, "delta")] {
            loaded
                .doc
                .upsert(&key.public().to_string(), name, &[])
                .unwrap();
        }
        let mut store = loaded.store;
        store.flush(&loaded.doc).unwrap();
    }
    for name in ["alpha", "beta", "delta"] {
        node.approve_here(name).expect("the root holder approves");
    }
    node.revoke_here("delta").expect("and revokes one of them again");
    drop(node);

    let r = relay(&home.0, &["--only-mine"]);
    // The child picked its language off the same environment (a fresh
    // home has no stored choice), so this process speaks it too.
    khor_catalog::lang::set(khor_catalog::lang::from_env());
    let admits = khor_catalog::cli::relay_admit_mine(3);
    assert!(
        r.banner.iter().any(|l| *l == admits),
        "the banner counts who the roster admits (this home, alpha, beta — not the \
         waiting gamma, not the revoked delta): want {admits:?} in {:?}",
        r.banner
    );
    let relays = [Relay::bare(&r.url)];

    let far = far_side(a, &relays).await;
    assert_eq!(verdict(&far).await, Ok(()), "a device on the roster gets in");

    for (key, why) in [
        (iroh::SecretKey::generate(), "not in the table at all"),
        (waiting, "in the table but not on the roster"),
        (revoked, "on the roster once, revoked since"),
    ] {
        let ep = endpoint::bind(key, &relays).await.unwrap();
        let refused = verdict(&ep).await;
        assert!(refused.is_err(), "{why}: must be refused, got {refused:?}");
        assert!(
            echo(&ep, &far, &r.url).await.is_err(),
            "{why}: must reach nobody through this relay"
        );
        ep.close().await;
    }

    let near = endpoint::bind(b, &relays).await.unwrap();
    assert_eq!(verdict(&near).await, Ok(()), "the other device on the roster gets in");
    assert_eq!(echo(&near, &far, &r.url).await, Ok(()));
    near.close().await;
    far.close().await;
}

/// The meter (CONNECT.md 计量只存总数): one device connecting twice is
/// two connections and the time it was on, in the state file — and the
/// file holds totals and nothing about who talked to whom.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_meter_counts_connections_and_online_time_and_nothing_else() {
    clear_proxies();
    let home = scratch("meter");
    let r = relay(&home.0, &[]);
    let relays = [Relay::bare(&r.url)];
    let key = iroh::SecretKey::generate();
    let id = key.public().to_string();
    for _ in 0..2 {
        let ep = endpoint::bind(key.clone(), &relays).await.unwrap();
        assert_eq!(verdict(&ep).await, Ok(()));
        tokio::time::sleep(Duration::from_millis(1500)).await;
        ep.close().await;
    }

    let file = home.0.join("state").join("relay-stats.json");
    let deadline = Instant::now() + Duration::from_secs(20);
    let stats = loop {
        let read = std::fs::read(&file)
            .ok()
            .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok());
        if let Some(v) = &read
            && v["devices"][&id]["connections"] == 2
            && v["devices"][&id]["online_secs"].as_u64().unwrap_or(0) >= 2
        {
            break read.unwrap();
        }
        assert!(
            Instant::now() < deadline,
            "the meter never showed two connections and ≥2s online for {id}: {read:?}"
        );
        tokio::time::sleep(Duration::from_millis(250)).await;
    };
    let online = stats["devices"][&id]["online_secs"].as_u64().unwrap();
    assert!(
        online <= 10,
        "two ~1.5s connections cannot add up to {online}s online"
    );
    // Totals only: these keys and no others — no pair of endpoints, no
    // address, nothing that says who was talking to whom.
    let top: Vec<&str> = stats.as_object().unwrap().keys().map(String::as_str).collect();
    assert_eq!(
        top,
        ["since", "updated", "bytes_sent", "bytes_recv", "devices"],
        "{stats}"
    );
    let per: Vec<&str> = stats["devices"][&id]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(per, ["connections", "online_secs"], "{stats}");

    // `--stats` prints it: the device by its id, and its count.
    let out = Command::new(env!("CARGO_BIN_EXE_khor"))
        .args(["relay", "--stats", "--state"])
        .arg(home.0.join("state"))
        .env("KHOR_HOME", &home.0)
        .output()
        .unwrap();
    let said = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{said}");
    let line = said
        .lines()
        .find(|l| l.contains(&id[..16]))
        .unwrap_or_else(|| panic!("--stats does not name {}: {said}", &id[..16]));
    assert!(line.contains('2'), "the line carries the two connections: {line}");
    drop(r);
}
