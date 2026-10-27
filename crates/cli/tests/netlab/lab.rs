//! The lab's furniture: relays and khor residents placed on patchbay
//! devices, and the reads every scenario takes off them.
//!
//! A resident runs in its device's network namespace (`spawn_command_sync`).
//! Its IPC is loopback TCP, which belongs to that namespace too, so every
//! `Node` call is made from a task on the device's own runtime
//! ([`LabNode::with`]), never from the test's thread.
#![allow(dead_code)]

use std::future::Future;
use std::io::BufRead;
use std::path::Path;
use std::process::Stdio;
use std::time::{Duration, Instant};

use khor_core::{Hop, Link};
use khor_node::Node;
use patchbay::{Device, IpSupport, Lab, LinkCondition, LinkDirection, RouterPreset};

use crate::scratch::{Reaped, Scratch, khor, scratch, serve_pid, signal};

/// A `khor relay --dev` on a device of its own, behind a public router.
pub struct Relay {
    pub url: String,
    pub dev: Device,
    pub _home: Scratch,
    pub _child: Reaped,
}

/// A plain relay (the Beijing shape: HTTP, no QUIC address discovery) on
/// `name`, its link slowed by `rtt` in all (half each way) when given.
pub async fn relay(lab: &Lab, tag: &str, name: &str, rtt: Option<Duration>) -> Relay {
    let dc = lab
        .add_router(&format!("dc-{name}"))
        .preset(RouterPreset::Public)
        .ip_support(IpSupport::DualStack)
        .build()
        .await
        .unwrap();
    let dev = lab.add_device(name).uplink(dc.id()).build().await.unwrap();
    if let Some(rtt) = rtt {
        slow(&dev, rtt).await;
    }
    let home = scratch(&format!("{tag}-{name}"));
    let ip = dev.ip().expect("a relay has IPv4");
    let mut cmd = khor(&home.0);
    cmd.args(["relay", "--dev", "--bind", &format!("{ip}:3340"), "--state"])
        .arg(home.0.join("state"))
        .stdout(Stdio::piped())
        .stderr(std::fs::File::create(home.0.join("relay.log")).unwrap());
    let mut child = dev.spawn_command_sync(cmd).unwrap();
    let mut lines = std::io::BufReader::new(child.stdout.take().unwrap()).lines();
    let first = lines.next().expect("a banner line").unwrap();
    let at = first.find("http://").expect("a url on the first line");
    let url = first[at..].split_whitespace().next().unwrap().to_string();
    // Keep draining: a relay writing into a closed pipe resets its
    // connections (scratch::local_relay).
    std::thread::spawn(move || for _ in lines.map_while(Result::ok) {});
    eprintln!("[netlab] relay {name} at {url}");
    Relay {
        url,
        dev,
        _home: home,
        _child: Reaped(child),
    }
}

/// Stand-ins for n0's relays: HTTPS on 443 and QUIC address discovery on
/// UDP 7842, each on a device of its own, one self-signed certificate
/// naming every one by IP (iroh checks it against the URL's host).
pub struct QadRelays {
    /// For `KHOR_PUBLIC_RELAYS`: `https://ip:443?qad=7842`, comma-separated
    /// (`Relay::parse`).
    pub env: String,
    /// The certificate, for `KHOR_RELAY_CA`.
    pub ca: std::path::PathBuf,
    pub urls: Vec<String>,
    pub devs: Vec<Device>,
    pub _home: Scratch,
    _servers: Vec<tokio::task::JoinHandle<()>>,
}

impl Drop for QadRelays {
    fn drop(&mut self) {
        for s in &self._servers {
            s.abort();
        }
    }
}

pub async fn qad_relays(lab: &Lab, tag: &str, names: &[&str], rtt: Option<Duration>) -> QadRelays {
    qad_relays_on(lab, tag, names, rtt, RouterPreset::PublicV4).await
}

/// [`qad_relays`] on a dual-stack network: each stand-in also answers on
/// its IPv6 address, as a second URL `https://[v6]:443` (iroh asks QAD v6
/// of a relay whose host is a v6 address).
pub async fn qad_relays_dual(lab: &Lab, tag: &str, names: &[&str]) -> QadRelays {
    qad_relays_on(lab, tag, names, None, RouterPreset::Public).await
}

async fn qad_relays_on(
    lab: &Lab,
    tag: &str,
    names: &[&str],
    rtt: Option<Duration>,
    preset: RouterPreset,
) -> QadRelays {
    use iroh_relay::server::{
        CertConfig, QuicConfig, RelayConfig, Server, ServerConfig, TlsConfig,
    };
    let mut devs = vec![];
    for name in names {
        let dc = lab
            .add_router(&format!("dc-{name}"))
            .preset(preset)
            .build()
            .await
            .unwrap();
        let dev = lab.add_device(name).uplink(dc.id()).build().await.unwrap();
        if let Some(rtt) = rtt {
            slow(&dev, rtt).await;
        }
        devs.push(dev);
    }
    let ips: Vec<std::net::Ipv4Addr> = devs.iter().map(|d| d.ip().unwrap()).collect();
    let ip6s: Vec<Option<std::net::Ipv6Addr>> = devs.iter().map(|d| d.ip6()).collect();
    let home = scratch(&format!("{tag}-qad"));
    let names: Vec<String> = ips
        .iter()
        .map(|ip| ip.to_string())
        .chain(ip6s.iter().flatten().map(|ip| ip.to_string()))
        .collect();
    let made = rcgen::generate_simple_self_signed(names).unwrap();
    let ca = home.0.join("qad-ca.pem");
    std::fs::write(&ca, made.cert.pem()).unwrap();
    let key = rustls::pki_types::PrivateKeyDer::from(rustls::pki_types::PrivatePkcs8KeyDer::from(
        made.signing_key.serialize_der(),
    ));
    let tls = rustls::ServerConfig::builder_with_provider(std::sync::Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_no_client_auth()
    .with_single_cert(vec![made.cert.der().clone()], key)
    .unwrap();
    let mut servers = vec![];
    let mut urls = vec![];
    for ((dev, ip), ip6) in devs.iter().zip(&ips).zip(&ip6s) {
        // With a v6 address, everything on [::], which takes v4 as well.
        let bind = match ip6 {
            Some(_) => std::net::IpAddr::V6(std::net::Ipv6Addr::UNSPECIFIED),
            None => std::net::IpAddr::V4(*ip),
        };
        let tls = tls.clone();
        let (up_tx, up_rx) = tokio::sync::oneshot::channel();
        let task = dev
            .spawn(move |_| async move {
                let mut relay = RelayConfig::new((bind, 80));
                relay.tls = Some(TlsConfig::new(
                    (bind, 443),
                    CertConfig::Manual {
                        server_config: tls.clone(),
                    },
                ));
                let mut cfg = ServerConfig::default();
                cfg.relay = Some(relay);
                let mut quic = QuicConfig::new((bind, 7842));
                quic.server_config = Some(tls);
                cfg.quic = Some(quic);
                let server = Server::spawn(cfg).await.expect("a QAD stand-in spawns");
                let _ = up_tx.send(());
                std::future::pending::<()>().await;
                drop(server);
            })
            .unwrap();
        up_rx.await.expect("the stand-in stands up");
        servers.push(task);
        urls.push(format!("https://{ip}:443"));
        if let Some(ip6) = ip6 {
            urls.push(format!("https://[{ip6}]:443"));
        }
    }
    let env = urls
        .iter()
        .map(|u| format!("{u}?qad=7842"))
        .collect::<Vec<_>>()
        .join(",");
    eprintln!("[netlab] QAD stand-ins {env}");
    QadRelays {
        env,
        ca,
        urls,
        devs,
        _home: home,
        _servers: servers,
    }
}

impl QadRelays {
    /// The stand-ins as `relays.json` entries of the machine's own list,
    /// QAD port included: the shape of the Beijing relay once it answers
    /// address discovery on UDP 7842.
    pub fn entries(&self) -> Vec<serde_json::Value> {
        self.urls
            .iter()
            .map(|u| serde_json::json!({ "url": u, "qad": 7842 }))
            .collect()
    }
}

/// Adds `rtt` to every packet in and out of `dev`'s first interface.
pub async fn slow(dev: &Device, rtt: Duration) {
    let half = (rtt.as_millis() / 2) as u32;
    let cond = LinkCondition::new().latency_ms(half);
    dev.default_iface()
        .expect("an interface")
        .set_condition(cond, LinkDirection::Both)
        .await
        .unwrap();
}

/// A khor resident on a device, with a scratch home of its own.
pub struct LabNode {
    pub name: String,
    pub id: String,
    pub dev: Device,
    pub home: Scratch,
    pub serve: Option<Reaped>,
}

/// Starts `khor serve` as `name` on `dev`, homed on `relays` only.
pub async fn node(
    tag: &str,
    dev: Device,
    name: &str,
    relays: &[&Relay],
    env: &[(&str, &str)],
) -> LabNode {
    let list = relays
        .iter()
        .map(|r| serde_json::json!({ "url": r.url }))
        .collect();
    node_on(tag, dev, name, list, env).await
}

/// [`node`] with its own list written out as `relays.json` entries
/// (`{"url", "qad", …}`), for relays that are not `khor relay --dev`.
pub async fn node_on(
    tag: &str,
    dev: Device,
    name: &str,
    list: Vec<serde_json::Value>,
    env: &[(&str, &str)],
) -> LabNode {
    let home = scratch(&format!("{tag}-{name}"));
    std::fs::create_dir_all(home.0.join(".khor")).unwrap();
    std::fs::write(
        home.0.join(".khor/relays.json"),
        serde_json::json!({ "relays": list, "only_mine": true }).to_string(),
    )
    .unwrap();
    let mut cmd = khor(&home.0);
    cmd.arg("serve")
        .env("KHOR_NAME", name)
        .env(khor_net::endpoint::LAB_ENV, "1")
        .envs(env.iter().copied())
        .stdout(Stdio::null())
        .stderr(std::fs::File::create(home.0.join("serve.log")).unwrap());
    let child = dev.spawn_command_sync(cmd).unwrap();
    let began = Instant::now();
    while !home.0.join(".khor/endpoint.json").exists() {
        assert!(
            began.elapsed() < Duration::from_secs(60),
            "{name} stands up"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let id = Node::open_as(home.0.clone(), name)
        .unwrap()
        .device_str()
        .to_owned();
    eprintln!("[netlab] {name} up: v4 {:?} v6 {:?}", dev.ip(), dev.ip6());
    LabNode {
        name: name.to_owned(),
        id,
        dev,
        home,
        serve: Some(Reaped(child)),
    }
}

impl LabNode {
    /// Runs `f` with this machine's `Node`, on the device's own runtime.
    pub async fn with<T, F, Fut>(&self, f: F) -> T
    where
        T: Send + 'static,
        F: FnOnce(Node) -> Fut + Send + 'static,
        Fut: Future<Output = T> + Send + 'static,
    {
        let (home, name) = (self.home.0.clone(), self.name.clone());
        self.dev
            .spawn(move |_| async move { f(Node::open_as(home, &name).unwrap()).await })
            .unwrap()
            .await
            .unwrap()
    }

    /// What this machine reads for `peer` now.
    pub async fn link(&self, peer: &LabNode) -> Option<Link> {
        let id = peer.id.clone();
        self.with(move |n| async move {
            n.links()
                .await
                .ok()?
                .into_iter()
                .find(|(d, _)| *d == id)
                .map(|(_, l)| l)
        })
        .await
    }

    /// Polls this machine's word for `peer` until `want` holds, printing
    /// each change of road; `None` when `within` runs out.
    pub async fn until(
        &self,
        peer: &LabNode,
        within: Duration,
        want: impl Fn(&Link) -> bool,
    ) -> Option<(Duration, Link)> {
        let began = Instant::now();
        let mut last: Option<(Hop, Option<String>)> = None;
        loop {
            if let Some(l) = self.link(peer).await {
                let now = (l.hop, l.via.clone());
                if last.as_ref() != Some(&now) {
                    eprintln!(
                        "[netlab] {:>6.1}s {} → {}: {:?} via {:?} rtt {:?} down {:?}",
                        began.elapsed().as_secs_f32(),
                        self.name,
                        peer.name,
                        l.hop,
                        l.via,
                        l.rtt_ms,
                        l.down_ms
                    );
                    last = Some(now);
                }
                if want(&l) {
                    return Some((began.elapsed(), l));
                }
            }
            if began.elapsed() >= within {
                return None;
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }

    /// The resident's own pid.
    pub fn pid(&self) -> i32 {
        serve_pid(&self.home.0).expect("a resident")
    }

    pub fn freeze(&self) {
        signal(self.pid(), "-STOP");
    }

    pub fn thaw(&self) {
        signal(self.pid(), "-CONT");
    }

    /// The journal's lines about `peer` (this machine's own with `None`),
    /// as JSON (`journal::Entry`: `at`, `peer`, `e`, …).
    pub fn journal(&self, peer: Option<&LabNode>) -> Vec<serde_json::Value> {
        let file = match peer {
            Some(p) => format!("{}.jsonl", p.id),
            None => "here.jsonl".to_owned(),
        };
        read_jsonl(&self.home.0.join(".khor/journal").join(file))
    }

    /// The relays this machine publishes, live home first (`endpoint.json`).
    pub fn published_relays(&self) -> Vec<String> {
        let Ok(text) = std::fs::read_to_string(self.home.0.join(".khor/endpoint.json")) else {
            return vec![];
        };
        let v: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
        v["relays"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|r| r.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default()
    }
}

fn read_jsonl(path: &Path) -> Vec<serde_json::Value> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

/// `joiner` pairs with `host`'s ticket, retried for two minutes (both
/// have to be on a relay before either is found there).
pub async fn pair(host: &LabNode, joiner: &LabNode) -> Duration {
    let ticket = host
        .with(|n| async move { n.invite() })
        .await
        .expect("a ticket");
    let began = Instant::now();
    loop {
        let t = ticket.clone();
        let tried = tokio::time::timeout(
            Duration::from_secs(90),
            joiner.with(move |n| async move { n.pair(&t).await }),
        )
        .await
        .expect("pairing must not hang");
        match tried {
            Ok(_) => return began.elapsed(),
            Err(why) if began.elapsed() < Duration::from_secs(120) => {
                eprintln!("[netlab] pair {} → {}: {why}", joiner.name, host.name);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            Err(why) => panic!("{} pairs with {}: {why}", joiner.name, host.name),
        }
    }
}

/// Everyone joins the first; returns once every machine knows every other.
pub async fn mesh(nodes: &[&LabNode]) {
    for joiner in &nodes[1..] {
        let took = pair(nodes[0], joiner).await;
        eprintln!(
            "[netlab] {} paired with {} in {took:?}",
            joiner.name, nodes[0].name
        );
    }
    let ids: Vec<String> = nodes.iter().map(|n| n.id.clone()).collect();
    let began = Instant::now();
    for n in nodes {
        loop {
            let want = ids.clone();
            let known = n
                .with(move |node| async move {
                    node.devices()
                        .is_ok_and(|ds| want.iter().all(|id| ds.iter().any(|d| &d.id == id)))
                })
                .await;
            if known {
                break;
            }
            assert!(
                began.elapsed() < Duration::from_secs(90),
                "{} knows every machine",
                n.name
            );
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }
}

/// A road word with its rtt, for the result lines.
pub fn word(l: &Link) -> String {
    let via = l
        .via
        .as_deref()
        .map(|v| format!(" via {}", &v[..v.len().min(8)]))
        .unwrap_or_default();
    format!(
        "{:?}{via} {} ms",
        l.hop,
        l.rtt_ms.map_or("?".into(), |r| r.to_string())
    )
}

/// One result line, the shape `scripts/netlab.sh` collects for the report.
pub fn result(scenario: &str, verdict: bool, what: impl std::fmt::Display) {
    eprintln!(
        "[netlab-result] {scenario} | {} | {what}",
        if verdict { "green" } else { "red" }
    );
}
