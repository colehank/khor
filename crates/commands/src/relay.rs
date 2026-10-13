//! `khor relay`: an iroh relay, embedded in khor (docs/CONNECT.md 中继:
//! 自家 `khor relay`; docs/NET.md 中继).
//!
//! **The server is iroh-relay's own**, the same one the `iroh-relay`
//! binary runs (version-locked to the workspace iroh). What khor adds
//! is only the three things that binary cannot know:
//!
//! - **who gets in** — everyone, holders of a shared key (iroh-relay's
//!   `shared_token`), or **the devices this network trusts**
//!   (`--only-mine`): judged by the endpoint id the relay handshake has
//!   already proven, against the roster the root signed — the same test
//!   every request to this machine passes (`Node::admit`: in the table,
//!   and standing trusted). The last is the one CONNECT.md asks for: a
//!   relay a person runs for their own devices needs no key typed
//!   anywhere, and admits by certificate (分期 2 「khor relay 按证书放行」);
//! - **a meter that keeps totals only** (CONNECT.md 计量只存总数): per
//!   endpoint id, how many connections and how long online, plus the
//!   relay's own byte counters — never who talked to whom;
//! - the words it prints, from the catalog.
//!
//! ⚠️ **Bytes are relay-wide, not per device**, and that is iroh-relay's
//! shape rather than a choice here: its `bytes_sent` / `bytes_recv`
//! counters (`server/metrics.rs`) are one pair for the whole server,
//! incremented in `server/client.rs` with no endpoint attached, and
//! nothing on `Clients` or `AccessControl` sees a packet.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::net::SocketAddr;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use iroh_relay::server::{
    Access, AccessControl, AcmeConfig, CertConfig, ClientRateLimit, ClientRequest, ConnectionId,
    Metrics, QuicConfig, RelayConfig, Server, ServerConfig, TlsConfig,
};
use khor_catalog::cli;
use khor_node::Node;
use serde::{Deserialize, Serialize};

use crate::{out, err};

/// Where a plain-HTTP relay listens when nobody says: the port the
/// Khor tier has always used, and `iroh-relay --dev`'s own default.
const DEV_PORT: u16 = 3340;

/// How long the admitted set is kept before it is worked out again, for
/// `--only-mine`. Bounds the disk reads a stranger can cause by
/// reconnecting in a loop; a machine approved a moment ago waits at most
/// this long to be let in, and one revoked a moment ago is refused
/// within it.
const TABLE_FRESH: Duration = Duration::from_secs(5);

/// The meter's file inside `--state`.
const STATS_FILE: &str = "relay-stats.json";

/// How the relay proves who it is.
enum Mode {
    /// Plain HTTP. The relay still cannot read what it forwards — that
    /// is end-to-end encrypted between the two endpoints — but a shared
    /// key crosses the wire in the clear.
    Dev,
    /// A certificate and key the operator already has (PEM files).
    Tls { cert: PathBuf, key: PathBuf },
    /// Let's Encrypt, for a domain that points at this machine.
    Acme {
        domain: String,
        contact: Option<String>,
    },
}

struct Options {
    mode: Option<Mode>,
    bind: Option<SocketAddr>,
    http: Option<SocketAddr>,
    /// `--quic`: where QUIC address discovery listens (UDP), with the
    /// relay's own certificate — or, beside `--dev`, with the one
    /// [`Options::quic_tls`] names for QUIC alone.
    quic: Option<SocketAddr>,
    /// `--tls-cert`/`--tls-key` beside `--dev --quic`: a certificate for
    /// the QUIC server only, the relay itself staying plain HTTP (the
    /// lead's ruling (b), audit/net-d.md h.2: QAD under the plain
    /// relay's url, whose TCP passes where the TLS relay's is reset).
    quic_tls: Option<(PathBuf, PathBuf)>,
    tokens: Vec<String>,
    rate: Option<NonZeroU32>,
    only_mine: bool,
    metrics: Option<SocketAddr>,
    state: PathBuf,
    stats: bool,
    /// `--tls-check`: read the certificate and key, start nothing.
    tls_check: bool,
}

fn parse(rest: &[String]) -> Result<Options, String> {
    let mut o = Options {
        mode: None,
        bind: None,
        http: None,
        quic: None,
        quic_tls: None,
        tokens: Vec::new(),
        rate: None,
        only_mine: false,
        metrics: None,
        state: crate::command_root().join(".khor").join("relay"),
        stats: false,
        tls_check: false,
    };
    let (mut cert, mut key, mut acme, mut contact, mut dev) = (None, None, None, None, false);
    let mut it = rest.iter();
    let addr = |flag: &str, v: Option<&String>| -> Result<SocketAddr, String> {
        let v = v.ok_or_else(|| cli::relay_flag_needs_value(flag))?;
        v.parse().map_err(|_| cli::relay_addr_bad(v))
    };
    while let Some(flag) = it.next() {
        let mut value = || {
            it.next()
                .cloned()
                .ok_or_else(|| cli::relay_flag_needs_value(flag))
        };
        match flag.as_str() {
            "--dev" => dev = true,
            "--tls-cert" => cert = Some(PathBuf::from(value()?)),
            "--tls-key" => key = Some(PathBuf::from(value()?)),
            "--acme" => acme = Some(value()?),
            "--acme-contact" => contact = Some(value()?),
            "--token" => {
                let t = value()?;
                if t.is_empty() {
                    return Err(cli::relay_flag_needs_value(flag));
                }
                o.tokens.push(t);
            }
            "--rate" => {
                let v = value()?;
                let n: u32 = v.parse().map_err(|_| cli::relay_rate_bad(&v))?;
                o.rate = Some(NonZeroU32::new(n).ok_or_else(|| cli::relay_rate_bad(&v))?);
            }
            "--only-mine" => o.only_mine = true,
            "--metrics" => o.metrics = Some(addr(flag, it.next())?),
            "--bind" => o.bind = Some(addr(flag, it.next())?),
            "--http" => o.http = Some(addr(flag, it.next())?),
            "--quic" => o.quic = Some(addr(flag, it.next())?),
            "--state" => o.state = PathBuf::from(value()?),
            "--stats" => o.stats = true,
            "--tls-check" => o.tls_check = true,
            other => return Err(cli::relay_flag_unknown(other)),
        }
    }
    o.mode = match (dev, cert, key, acme) {
        (true, None, None, None) => Some(Mode::Dev),
        // A certificate beside `--dev` is QUIC's alone, and means nothing
        // without `--quic`: refused as two modes, as before.
        (true, Some(cert), Some(key), None) if o.quic.is_some() => {
            o.quic_tls = Some((cert, key));
            Some(Mode::Dev)
        }
        (false, Some(cert), Some(key), None) => Some(Mode::Tls { cert, key }),
        (false, None, None, Some(domain)) => Some(Mode::Acme { domain, contact }),
        (false, None, None, None) => None,
        _ => return Err(cli::relay_pick_mode().to_owned()),
    };
    if !o.tokens.is_empty() && o.only_mine {
        return Err(cli::relay_token_or_mine().to_owned());
    }
    // QUIC needs a certificate, and a client checks it against the relay
    // url's host: plain HTTP has none of its own to offer.
    if o.quic.is_some() && matches!(o.mode, Some(Mode::Dev)) && o.quic_tls.is_none() {
        return Err(cli::relay_quic_needs_tls().to_owned());
    }
    Ok(o)
}

/// The verb.
pub(crate) fn verb(rest: &[String]) -> Result<(), String> {
    let o = parse(rest)?;
    if o.stats {
        return print_stats(&o.state);
    }
    // **`--tls-check`**: the certificate and key read exactly as a relay
    // started with them would read them, and nothing started. What
    // scripts/relay-https.sh runs before it switches anything — and an
    // older khor, which does not know the flag, refuses it by name, so
    // the script also learns whether this binary renews in place.
    if o.tls_check {
        let Some(Mode::Tls { cert, key }) = o.mode.as_ref() else {
            return Err(cli::relay_tls_check_needs_files().to_owned());
        };
        Renewable::read(cert, key)?;
        println!("{}", cli::relay_tls_ok(cert.display(), key.display()));
        return Ok(());
    }
    let Some(mode) = o.mode.as_ref() else {
        return Err(cli::relay_pick_mode().to_owned());
    };
    // Opened before the runtime so a store that refuses says so before
    // anything listens.
    let table = if o.only_mine {
        Some(Arc::new(Table::new(crate::node()?)))
    } else {
        None
    };
    crate::rt()?.block_on(run(&o, mode, table))
}

async fn run(o: &Options, mode: &Mode, table: Option<Arc<Table>>) -> Result<(), String> {
    std::fs::create_dir_all(&o.state).map_err(|e| cli::relay_wont_start(e))?;
    let meter = Arc::new(Meter::load(o.state.join(STATS_FILE)));
    let admit = match &table {
        Some(t) => Admit::Mine(t.clone()),
        None if !o.tokens.is_empty() => Admit::Token(o.tokens.clone()),
        None => Admit::Everyone,
    };
    let (http_bind, tls) = match mode {
        Mode::Dev => (
            o.bind
                .unwrap_or_else(|| SocketAddr::from(([0u16; 8], DEV_PORT))),
            None,
        ),
        Mode::Tls { cert, key } => {
            let (server_config, renewable) = manual_tls(cert, key)?;
            tokio::spawn(renewable.keep_fresh());
            (
                o.http.unwrap_or_else(|| SocketAddr::from(([0u16; 8], 80))),
                Some(TlsConfig::new(
                    o.bind.unwrap_or_else(|| SocketAddr::from(([0u16; 8], 443))),
                    CertConfig::Manual { server_config },
                )),
            )
        }
        Mode::Acme { domain, contact } => {
            let mut acme = AcmeConfig::letsencrypt(true)
                .domains(vec![domain.clone()])
                .cache_path(o.state.join("acme"));
            if let Some(c) = contact {
                acme = acme.contact(vec![format!("mailto:{c}")]);
            }
            (
                o.http.unwrap_or_else(|| SocketAddr::from(([0u16; 8], 80))),
                Some(acme_tls(
                    acme,
                    o.bind.unwrap_or_else(|| SocketAddr::from(([0u16; 8], 443))),
                )?),
            )
        }
    };
    let mut relay = RelayConfig::new(http_bind);
    relay.tls = tls;
    relay.limits.client_rx = o.rate.map(ClientRateLimit::new);
    relay.access = Arc::new(Gate {
        admit,
        meter: meter.clone(),
    });
    let mut cfg = ServerConfig::default();
    cfg.relay = Some(relay);
    // The QUIC server takes the relay's TLS config when given none, the
    // same resolver, so a renewal (`SIGHUP`) reaches it too. Beside
    // `--dev` it has its own, renewed the same way.
    cfg.quic = match (o.quic, &o.quic_tls) {
        (Some(bind), Some((cert, key))) => {
            let (server_config, renewable) = manual_tls(cert, key)?;
            tokio::spawn(renewable.keep_fresh());
            let mut q = QuicConfig::new(bind);
            q.server_config = Some(server_config);
            Some(q)
        }
        (bind, _) => bind.map(QuicConfig::new),
    };
    cfg.metrics_addr = o.metrics;
    let mut server = Server::spawn(cfg)
        .await
        .map_err(|e| cli::relay_wont_start(e))?;
    let _ = meter.bytes.set(server.metrics().server.clone());

    // What a person starting it needs to see: where, who gets in, how
    // fast, and where the numbers go.
    let url = match mode {
        Mode::Dev => server.http_addr().map(|a| format!("http://{a}")),
        Mode::Tls { .. } => server.https_addr().map(|a| format!("https://{a}")),
        Mode::Acme { domain, .. } => Some(match server.https_addr().map(|a| a.port()) {
            Some(443) | None => format!("https://{domain}"),
            Some(p) => format!("https://{domain}:{p}"),
        }),
    };
    println!("{}", cli::relay_up(url.unwrap_or_default()));
    println!(
        "{}",
        match &table {
            Some(t) => cli::relay_admit_mine(t.ids().len()),
            None if !o.tokens.is_empty() => cli::relay_admit_token().to_owned(),
            None => cli::relay_admit_everyone().to_owned(),
        }
    );
    if matches!(mode, Mode::Dev) && !o.tokens.is_empty() {
        println!("{}", cli::relay_key_in_clear());
    }
    println!(
        "{}",
        match o.rate {
            Some(r) => cli::relay_rate(crate::bytes(u64::from(r.get()))),
            None => cli::relay_rate_none().to_owned(),
        }
    );
    if let Some(q) = server.quic_addr() {
        println!("{}", cli::relay_quic(q));
    }
    if let Some(m) = o.metrics {
        println!("{}", cli::relay_metrics(format!("http://{m}/metrics")));
    }
    println!("{}", cli::relay_meter(meter.path.display()));

    let writer = tokio::spawn(meter.clone().keep_written());
    let outcome = tokio::select! {
        _ = stop_signal() => Ok(()),
        ended = server.join() => match ended {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(cli::relay_wont_start(e)),
            Err(e) => Err(cli::relay_wont_start(e)),
        },
    };
    writer.abort();
    meter.write();
    let _ = server.shutdown().await;
    outcome
}

/// Ctrl-C, or the TERM a service manager sends.
async fn stop_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let Ok(mut term) = signal(SignalKind::terminate()) else {
            let _ = tokio::signal::ctrl_c().await;
            return;
        };
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = term.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

/// The TLS half of `--acme`: certificates asked of an ACME directory,
/// served on `bind`.
///
/// **Installs ring as the process's rustls provider first.** When the
/// server is spawned the ACME library builds its acceptor with
/// `rustls::ServerConfig::builder()` — the *process default* provider —
/// and this binary's dependency tree turns on both `ring` and
/// `aws-lc-rs`, so rustls refuses to pick one and panics. The first
/// `khor relay --acme` ever started (2026-10-01, the Hong Kong relay)
/// died in a restart loop on exactly that, before it listened.
/// Everything else here names ring explicitly (`manual_tls`, the
/// builder below), which is why the other two modes never met it.
/// Installing twice is refused by rustls and is not an error here.
fn acme_tls(acme: AcmeConfig, bind: SocketAddr) -> Result<TlsConfig, String> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    Ok(TlsConfig::new(
        bind,
        CertConfig::LetsEncrypt {
            acme_config: acme,
            server_config_builder: rustls::ServerConfig::builder_with_provider(Arc::new(
                rustls::crypto::ring::default_provider(),
            ))
            .with_safe_default_protocol_versions()
            .map_err(|e| cli::relay_wont_start(e))?
            .with_no_client_auth(),
        },
    ))
}

/// `--tls-cert`/`--tls-key`: a certificate the operator renews on disk,
/// served through [`Renewable`] so a renewal takes without a restart.
fn manual_tls(cert: &Path, key: &Path) -> Result<(rustls::ServerConfig, Arc<Renewable>), String> {
    let renewable = Arc::new(Renewable::open(cert, key)?);
    let config =
        rustls::ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()
            .map_err(|e| cli::relay_tls_bad(e))?
            .with_no_client_auth()
            .with_cert_resolver(renewable.clone());
    Ok((config, renewable))
}

/// How often a [`Renewable`] reads its files again on its own, besides
/// the `SIGHUP` a renewal hook sends. iroh-relay's own reloading mode
/// reads once a day (`DEFAULT_CERT_RELOAD_INTERVAL`); a quarter of that
/// here, so a renewal whose hook never ran is still served within hours,
/// long before the old certificate's last 30 days run out (acme.sh
/// renews at 60 of Let's Encrypt's 90).
const TLS_REREAD_EVERY: std::time::Duration = std::time::Duration::from_secs(6 * 60 * 60);

/// **A certificate that can be renewed under a running relay** (round
/// NET-7, the Beijing relay's HTTPS). The certificate is one acme.sh
/// obtains by DNS and renews every 60 days, so the relay must take a new
/// one without dropping every client it holds; a restart would.
/// iroh-relay's `CertConfig::Manual` takes any rustls `ServerConfig` and
/// says it may carry "a certificate loader", and its own binary's
/// `Reloading` mode does exactly this with a resolver re-reading the
/// files on an interval (`server::reloading_resolver`). This one also
/// re-reads on `SIGHUP`, which is what the renewal hook sends
/// (`systemctl reload`, `ExecReload=` in the unit), so a renewal is
/// served at once rather than at the next interval.
///
/// A read that fails — a half-written file, a key that does not match
/// the certificate — keeps the certificate already served and says so;
/// only the first read, at start, is fatal.
#[derive(Debug)]
struct Renewable {
    cert: PathBuf,
    key: PathBuf,
    now: std::sync::RwLock<Arc<rustls::sign::CertifiedKey>>,
}

impl Renewable {
    fn open(cert: &Path, key: &Path) -> Result<Renewable, String> {
        Ok(Renewable {
            cert: cert.to_path_buf(),
            key: key.to_path_buf(),
            now: std::sync::RwLock::new(Arc::new(Self::read(cert, key)?)),
        })
    }

    /// The chain and key, read and checked the way a handshake will use
    /// them: parsed, the key loaded by the provider that serves, and the
    /// key the one the certificate names.
    fn read(cert: &Path, key: &Path) -> Result<rustls::sign::CertifiedKey, String> {
        use rustls::pki_types::pem::PemObject;
        use rustls::pki_types::{CertificateDer, PrivateKeyDer};
        let chain = CertificateDer::pem_file_iter(cert)
            .and_then(|i| i.collect::<Result<Vec<_>, _>>())
            .map_err(|e| cli::relay_tls_bad(format!("{}: {e}", cert.display())))?;
        if chain.is_empty() {
            return Err(cli::relay_tls_bad(format!("{}: no certificate", cert.display())));
        }
        let secret = PrivateKeyDer::from_pem_file(key)
            .map_err(|e| cli::relay_tls_bad(format!("{}: {e}", key.display())))?;
        let signing = rustls::crypto::ring::default_provider()
            .key_provider
            .load_private_key(secret)
            .map_err(|e| cli::relay_tls_bad(format!("{}: {e}", key.display())))?;
        let certified = rustls::sign::CertifiedKey::new(chain, signing);
        certified.keys_match().map_err(|e| cli::relay_tls_bad(e))?;
        Ok(certified)
    }

    /// Reads the files again; the certificate served changes only when
    /// the read succeeds.
    fn reread(&self) -> Result<(), String> {
        let fresh = Self::read(&self.cert, &self.key)?;
        *self.now.write().unwrap_or_else(|p| p.into_inner()) = Arc::new(fresh);
        Ok(())
    }

    /// `SIGHUP`, and every [`TLS_REREAD_EVERY`]: read again, and say what
    /// happened — a renewal that did not take must be in the log.
    async fn keep_fresh(self: Arc<Self>) {
        #[cfg(unix)]
        let mut hup = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup()).ok();
        let mut every = tokio::time::interval(TLS_REREAD_EVERY);
        every.tick().await;
        loop {
            #[cfg(unix)]
            let asked = async {
                match hup.as_mut() {
                    Some(h) => {
                        h.recv().await;
                    }
                    None => std::future::pending::<()>().await,
                }
            };
            #[cfg(not(unix))]
            let asked = std::future::pending::<()>();
            tokio::select! {
                _ = asked => {}
                _ = every.tick() => {}
            }
            let stamp = jiff::Zoned::now().strftime("%Y-%m-%d %H:%M:%S").to_string();
            match self.reread() {
                Ok(()) => println!("{}", cli::relay_tls_reread(&stamp, self.cert.display())),
                Err(e) => eprintln!("{}", cli::relay_tls_reread_failed(&stamp, e)),
            }
        }
    }
}

impl rustls::server::ResolvesServerCert for Renewable {
    fn resolve(&self, _: rustls::server::ClientHello<'_>) -> Option<Arc<rustls::sign::CertifiedKey>> {
        Some(self.now.read().unwrap_or_else(|p| p.into_inner()).clone())
    }
}

// ── who gets in ─────────────────────────────────────────────────────

enum Admit {
    Everyone,
    /// iroh-relay's `shared_token`: the bearer token, or the `token`
    /// query parameter, must be one of these.
    Token(Vec<String>),
    /// Only endpoints this network trusts ([`Table`]).
    Mine(Arc<Table>),
}

/// The devices this network trusts, worked out from this machine's
/// store and kept for [`TABLE_FRESH`].
///
/// **The rule is the request gate's, not a second one** (`Node::admit`):
/// in the device table, and standing admitted — listed in the newest
/// roster that verifies under this machine's root and not revoked (or
/// carrying a certificate newer than a fresh roster), or, in a network
/// from before roots, simply in the table (`Standing::Legacy`). A row
/// the table holds but the roster does not (pending) is refused, and so
/// is a revoked one: **the roster is the one source of trust**
/// (CONNECT.md 名单是信任的唯一出处), and a relay that trusted the table
/// alone would let a stolen, revoked machine keep using it.
///
/// **Read, never written**: pairing and approving stay the only ways
/// in, so this relay admits exactly what the network already trusts.
struct Table {
    node: Node,
    seen: Mutex<Option<(Instant, Arc<HashSet<String>>)>>,
}

impl Table {
    fn new(node: Node) -> Table {
        Table {
            node,
            seen: Mutex::new(None),
        }
    }

    fn ids(&self) -> Arc<HashSet<String>> {
        if let Ok(seen) = self.seen.lock()
            && let Some((at, ids)) = seen.as_ref()
            && at.elapsed() < TABLE_FRESH
        {
            return ids.clone();
        }
        // A table that will not read admits nobody new: failing open
        // would make a broken store an open relay.
        let ids: Arc<HashSet<String>> = Arc::new(
            self.node
                .devices()
                .map(|all| {
                    all.into_iter()
                        .map(|d| d.id)
                        .filter(|id| self.node.standing_of(id).admitted())
                        .collect()
                })
                .unwrap_or_default(),
        );
        if let Ok(mut seen) = self.seen.lock() {
            *seen = Some((Instant::now(), ids.clone()));
        }
        ids
    }
}

#[derive(Debug)]
struct Gate {
    admit: Admit,
    meter: Arc<Meter>,
}

impl std::fmt::Debug for Admit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never the keys themselves.
        f.write_str(match self {
            Admit::Everyone => "Everyone",
            Admit::Token(_) => "Token",
            Admit::Mine(_) => "Mine",
        })
    }
}

impl AccessControl for Gate {
    async fn on_connect(&self, request: &ClientRequest) -> Access {
        let admitted = match &self.admit {
            Admit::Everyone => true,
            Admit::Token(keys) => request
                .auth_token()
                .is_some_and(|t| keys.iter().any(|k| *k == t)),
            Admit::Mine(table) => {
                let (table, id) = (table.clone(), request.endpoint_id().to_string());
                tokio::task::spawn_blocking(move || table.ids().contains(&id))
                    .await
                    .unwrap_or(false)
            }
        };
        if !admitted {
            // A stable key rather than a sentence: it crosses the wire to
            // somebody else's iroh, which shows it in a status nobody
            // reads in our language (`RelayStatus::auth_denied_reason`).
            return Access::Deny {
                reason: Some(
                    match self.admit {
                        Admit::Mine(_) => "not-on-roster",
                        _ => "shared-token",
                    }
                    .to_owned(),
                ),
            };
        }
        self.meter
            .connected(request.endpoint_id().to_string(), request.connection_id());
        Access::Allow
    }

    fn on_disconnect(&self, _endpoint_id: iroh::EndpointId, connection_id: ConnectionId) {
        self.meter.disconnected(connection_id);
    }
}

// ── the meter ───────────────────────────────────────────────────────

/// What is written to `relay-stats.json`: totals since the file was
/// first written, carried across restarts.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct Stats {
    /// When this file's counting began.
    pub since: String,
    /// When it was last written.
    pub updated: String,
    /// Bytes this relay sent to its clients, all clients together.
    pub bytes_sent: u64,
    /// Bytes it received from them.
    pub bytes_recv: u64,
    /// Per endpoint id (hex), what it added up to.
    pub devices: BTreeMap<String, DeviceStats>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DeviceStats {
    /// Connections admitted.
    pub connections: u64,
    /// Seconds connected, over all of them.
    pub online_secs: u64,
}

#[derive(Debug)]
struct Meter {
    path: PathBuf,
    /// What the file said when this relay started.
    base: Stats,
    now: Mutex<Counted>,
    /// iroh-relay's own counters, once the server exists.
    bytes: OnceLock<Arc<Metrics>>,
    changed: tokio::sync::Notify,
}

#[derive(Debug, Default)]
struct Counted {
    /// Per endpoint: connections admitted, milliseconds of the ones
    /// that have ended.
    ended: HashMap<String, (u64, u128)>,
    /// Connections still open: whose, and since when.
    open: HashMap<ConnectionId, (String, Instant)>,
}

fn stamp() -> String {
    jiff::Zoned::now().strftime("%Y-%m-%d %H:%M:%S").to_string()
}

impl Meter {
    /// Picks up where the file left off. A file that does not parse is
    /// counted from zero rather than refused — a relay that will not
    /// start over its own bookkeeping is the worse failure — and the
    /// next write replaces it.
    fn load(path: PathBuf) -> Meter {
        let base = std::fs::read(&path)
            .ok()
            .and_then(|b| serde_json::from_slice::<Stats>(&b).ok())
            .unwrap_or_else(|| Stats {
                since: stamp(),
                ..Stats::default()
            });
        Meter {
            path,
            base,
            now: Mutex::default(),
            bytes: OnceLock::new(),
            changed: tokio::sync::Notify::new(),
        }
    }

    fn connected(&self, id: String, conn: ConnectionId) {
        if let Ok(mut c) = self.now.lock() {
            c.ended.entry(id.clone()).or_default().0 += 1;
            c.open.insert(conn, (id, Instant::now()));
        }
        self.changed.notify_one();
    }

    fn disconnected(&self, conn: ConnectionId) {
        if let Ok(mut c) = self.now.lock()
            && let Some((id, since)) = c.open.remove(&conn)
        {
            c.ended.entry(id).or_default().1 += since.elapsed().as_millis();
        }
        self.changed.notify_one();
    }

    /// The file as it should read right now: the base, what ended in
    /// this run, and what is still open counted up to this moment.
    fn snapshot(&self) -> Stats {
        let mut out = self.base.clone();
        out.updated = stamp();
        if let Some(m) = self.bytes.get() {
            out.bytes_sent += m.bytes_sent.get();
            out.bytes_recv += m.bytes_recv.get();
        }
        let Ok(c) = self.now.lock() else { return out };
        let mut ms: HashMap<&str, u128> = HashMap::new();
        for (id, (conns, ended)) in &c.ended {
            out.devices.entry(id.clone()).or_default().connections += conns;
            *ms.entry(id).or_default() += ended;
        }
        for (id, since) in c.open.values() {
            *ms.entry(id).or_default() += since.elapsed().as_millis();
        }
        for (id, total) in ms {
            out.devices.entry(id.to_owned()).or_default().online_secs +=
                u64::try_from(total / 1000).unwrap_or(u64::MAX);
        }
        out
    }

    /// Replaces the file whole, through a temporary beside it, so a
    /// reader never sees half of one.
    fn write(&self) {
        let Ok(bytes) = serde_json::to_vec_pretty(&self.snapshot()) else {
            return;
        };
        let tmp = self.path.with_extension(format!("tmp{}", std::process::id()));
        if std::fs::write(&tmp, bytes).is_ok() {
            let _ = std::fs::rename(&tmp, &self.path);
        }
        let _ = std::fs::remove_file(&tmp);
    }

    /// Writes a second after anything changed, and once a minute
    /// regardless — the minute is what keeps an open connection's
    /// online time from being lost to a kill that skips the final write.
    async fn keep_written(self: Arc<Self>) {
        loop {
            tokio::select! {
                () = self.changed.notified() => {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
                () = tokio::time::sleep(Duration::from_secs(60)) => {}
            }
            self.write();
        }
    }
}

fn print_stats(state: &Path) -> Result<(), String> {
    let path = state.join(STATS_FILE);
    let Ok(bytes) = std::fs::read(&path) else {
        println!("{}", cli::relay_stats_none(path.display()));
        return Ok(());
    };
    let stats: Stats =
        serde_json::from_slice(&bytes).map_err(|e| cli::relay_stats_unreadable(e))?;
    println!(
        "{}",
        cli::relay_stats_since(
            &stats.since,
            crate::bytes(stats.bytes_sent),
            crate::bytes(stats.bytes_recv)
        )
    );
    if stats.devices.is_empty() {
        println!("{}", cli::relay_stats_no_device());
    }
    for (id, d) in &stats.devices {
        println!(
            "{}",
            cli::relay_stats_device(
                id.chars().take(16).collect::<String>(),
                d.connections,
                crate::lasting(d.online_secs)
            )
        );
    }
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;

    /// `khor relay --acme` panicked the moment its server was spawned:
    /// "Could not automatically determine the process-level
    /// CryptoProvider". Red without the install in `acme_tls` when run
    /// alone (`cargo test -p khor-cli --bin khor acme_mode`).
    ///
    /// Nothing leaves the machine: the ACME directory is a dead loopback
    /// port, so the order loop fails quietly in the background, and both
    /// listeners are on loopback ports the system picks.
    #[tokio::test]
    async fn the_acme_mode_spawns_its_server_in_a_process_with_no_rustls_provider() {
        let dir = std::env::temp_dir().join(format!("khor-relay-acme-{}", std::process::id()));
        let loopback = SocketAddr::from(([127, 0, 0, 1], 0));
        let acme = AcmeConfig::new("http://127.0.0.1:9/directory".to_owned())
            .domains(vec!["relay.example".to_owned()])
            .cache_path(dir.join("acme"));
        let mut relay = RelayConfig::new(loopback);
        relay.tls = Some(acme_tls(acme, loopback).expect("the TLS half should build"));
        let mut cfg = ServerConfig::default();
        cfg.relay = Some(relay);
        let mut server = Server::spawn(cfg)
            .await
            .expect("an ACME relay should spawn and listen");
        assert!(server.https_addr().is_some(), "it listens for TLS");
        let _ = server.shutdown().await;
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// Validate a terminal-owned relay before creating its session.
pub(crate) fn validate(rest: &[String]) -> Result<(), String> {
    let options = parse(rest)?;
    if !options.stats && !options.tls_check && options.mode.is_none() { return Err(cli::relay_pick_mode().to_owned()); }
    Ok(())
}
