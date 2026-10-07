//! **Give an address, every machine tries it** (round ROAD-1, `khor
//! reach`; audit/ROAD-BRIEF.md T1). A person names a host and a port
//! (ssh's 22 unless told); every online machine resolves the name with
//! its own resolver, opens one TCP connection with a short timeout,
//! reads the first line the server says, and looks the name up in its
//! own known_hosts. The fastest machine that got through is the
//! entrance; this machine on a tie, because no hop beats a hop.
//!
//! # What it does not do
//!
//! - **No scan.** One address, one port, only when a person asks. There
//!   is no "khor port" to probe: khor machines are already known by id.
//! - **No keyscan.** The host key comes out of known_hosts — the trust
//!   the user already extended — for `satellite`'s reason: nothing is
//!   probed behind their back. The TCP connection reads the server's
//!   banner and hangs up before the key exchange.
//!
//! # A name is a hint, a key is a machine
//!
//! The same `192.168.1.5` is a different machine on a different LAN. Two
//! machines whose known_hosts hold different keys for the name are
//! talking about two machines, and the view says so
//! (`khor_core::ReachView::disagree`).

use std::time::{Duration, Instant};

use khor_catalog::msg;
use khor_core::{REACH_BANNER_MAX, ReachOutcome, ReachSeen, ReachView};
use tokio::io::AsyncReadExt;

use crate::proto::{Request, Response};

/// How long one connect may take, across every address the name gave.
pub const CONNECT_WITHIN: Duration = Duration::from_secs(3);

/// How long the server has to say its first line once connected. An ssh
/// server says it at once (RFC 4253 §4.2); a web server says nothing
/// first and costs this whole wait, which is why it is short.
pub const BANNER_WITHIN: Duration = Duration::from_millis(1500);

/// How long the asker waits on any one machine's answer: its dial, then
/// its probe ([`CONNECT_WITHIN`] + [`BANNER_WITHIN`] and a lookup) — well
/// inside the hand-off's thirty.
pub const ASK_WITHIN: Duration = Duration::from_secs(20);

/// ssh's port, the default.
pub const SSH_PORT: u16 = 22;

/// **This machine's try** at `host:port`.
pub async fn probe_here(host: &str, port: u16) -> ReachSeen {
    probe_with(host, port, host_key).await
}

/// [`probe_here`] with the known_hosts lookup handed in: a test never
/// reads anybody's known_hosts.
async fn probe_with(host: &str, port: u16, lookup: fn(&str, u16) -> Option<String>) -> ReachSeen {
    let host = bare(host).to_owned();
    let mut seen = ReachSeen {
        outcome: ReachOutcome::NoName,
        ms: None,
        addr: None,
        banner: None,
        host_key: None,
        why: None,
    };
    let key = {
        let host = host.clone();
        tokio::task::spawn_blocking(move || lookup(&host, port))
    };
    let addrs: Vec<std::net::SocketAddr> = match tokio::net::lookup_host((host.as_str(), port)).await {
        Ok(a) => a.collect(),
        Err(_) => Vec::new(),
    };
    if addrs.is_empty() {
        seen.why = Some(msg::reach_no_name(&host));
    } else {
        let by = Instant::now() + CONNECT_WITHIN;
        seen.outcome = ReachOutcome::Timeout;
        seen.why = Some(msg::reach_timeout(&host, CONNECT_WITHIN.as_secs()));
        for a in addrs {
            let left = by.saturating_duration_since(Instant::now());
            if left.is_zero() {
                break;
            }
            seen.addr = Some(a.to_string());
            let started = Instant::now();
            match tokio::time::timeout(left, tokio::net::TcpStream::connect(a)).await {
                Ok(Ok(mut tcp)) => {
                    let ms = started.elapsed().as_micros().div_ceil(1000);
                    seen.ms = Some(u32::try_from(ms).unwrap_or(u32::MAX));
                    seen.banner = banner(&mut tcp).await;
                    // The banner decides: a connect that says no ssh line
                    // may be a proxy's fake address taking anything.
                    if seen.banner.as_deref().is_some_and(|b| b.starts_with("SSH-")) {
                        seen.outcome = ReachOutcome::Reached;
                        seen.why = None;
                    } else {
                        seen.outcome = ReachOutcome::Silent;
                        seen.why = Some(msg::reach_silent(&a.to_string()));
                    }
                    break;
                }
                Ok(Err(e)) => {
                    seen.outcome = ReachOutcome::Refused;
                    seen.why = Some(msg::reach_refused(&a.to_string(), e));
                }
                Err(_) => {}
            }
        }
    }
    // The other IP-form key: ssh writes the address beside the name
    // (`CheckHostIP`), and a person may have typed either.
    seen.host_key = match key.await.ok().flatten() {
        Some(k) => Some(k),
        None => match seen.addr.as_deref().and_then(|a| a.parse::<std::net::SocketAddr>().ok()) {
            Some(a) if a.ip().to_string() != host => {
                let ip = a.ip().to_string();
                tokio::task::spawn_blocking(move || lookup(&ip, port)).await.ok().flatten()
            }
            _ => None,
        },
    };
    seen
}

/// `[::1]` as a person may type it, without the brackets the resolver
/// does not want.
fn bare(host: &str) -> &str {
    host.strip_prefix('[').and_then(|h| h.strip_suffix(']')).unwrap_or(host)
}

/// The server's first line, kept printable and short; `None` when it said
/// nothing first.
async fn banner(tcp: &mut tokio::net::TcpStream) -> Option<String> {
    let mut buf = vec![0u8; 256];
    let mut got = 0;
    let by = Instant::now() + BANNER_WITHIN;
    while got < buf.len() && !buf[..got].contains(&b'\n') {
        let left = by.saturating_duration_since(Instant::now());
        match tokio::time::timeout(left, tcp.read(&mut buf[got..])).await {
            Ok(Ok(n)) if n > 0 => got += n,
            _ => break,
        }
    }
    first_line(&buf[..got])
}

fn first_line(bytes: &[u8]) -> Option<String> {
    let line = bytes.split(|b| *b == b'\n').next()?;
    let text: String = line
        .iter()
        .map(|&b| b as char)
        .filter(|c| c.is_ascii_graphic() || *c == ' ')
        .take(REACH_BANNER_MAX)
        .collect();
    let text = text.trim().to_owned();
    (!text.is_empty()).then_some(text)
}

/// The key this machine's known_hosts holds for the name, as ssh files
/// it: bare for port 22, `[host]:port` for any other.
fn host_key(host: &str, port: u16) -> Option<String> {
    let name = if port == SSH_PORT { host.to_owned() } else { format!("[{host}]:{port}") };
    crate::satellite::known_host_id(&name).ok().flatten()
}

/// A host key's hex digest as OpenSSH prints it (`SHA256:<base64>`) —
/// what a person compares against `ssh-keygen -l`.
pub fn shown_key(hex: &str) -> String {
    use base64::Engine;
    let bytes: Vec<u8> = (0..hex.len() / 2)
        .filter_map(|i| u8::from_str_radix(hex.get(2 * i..2 * i + 2)?, 16).ok())
        .collect();
    format!("SHA256:{}", base64::engine::general_purpose::STANDARD_NO_PAD.encode(bytes))
}

/// The port a person typed, or ssh's.
pub fn port_or_ssh(port: Option<u16>) -> u16 {
    port.unwrap_or(SSH_PORT)
}

impl crate::Node {
    /// **Every machine's try at `host:port`**, through the resident when
    /// one holds this key, on an endpoint bound for the occasion
    /// otherwise.
    pub async fn reach_probe(&self, host: &str, port: u16) -> Result<ReachView, String> {
        if let Some(reply) = self
            .via_serve(crate::ipc::Op::ReachProbe {
                host: host.to_owned(),
                port,
            })
            .await
        {
            return match reply? {
                crate::ipc::Reply::ReachProbe { view } => Ok(view),
                crate::ipc::Reply::Refused { why } => Err(why),
                other => Err(msg::serve_non_answer(format_args!("{other:?}"))),
            };
        }
        let ep = khor_net::endpoint::bind_with(self.secret_key().clone(), self.relay_map(), self.relays_n0())
            .await
            .map_err(|e| e.to_string())?;
        let out = self.reach_probe_with(&ep, host, port).await;
        ep.close().await;
        out
    }

    /// The fan-out: this machine and every other one at once. A
    /// satellite is left out (it is not khor; its sponsor's own try
    /// stands for the network it sits in), and so is a machine the
    /// roster revoked.
    pub(crate) async fn reach_probe_with(
        &self,
        ep: &iroh::Endpoint,
        host: &str,
        port: u16,
    ) -> Result<ReachView, String> {
        if bare(host).trim().is_empty() {
            return Err(msg::reach_no_host().into());
        }
        let me = self.device_str().to_owned();
        let others: Vec<_> = self
            .devices()?
            .into_iter()
            .filter(|d| d.id != me && d.sponsor.is_none())
            .filter(|d| self.standing_of(&d.id) != crate::Standing::Revoked)
            .collect();
        let req = Request::Probe {
            host: host.to_owned(),
            port,
        };
        let asks = others
            .iter()
            .map(|d| {
                let req = req.clone();
                async move {
                    if crate::pause::peer_on(self.root(), &d.id) {
                        return unasked(msg::reach_paused(d.shown()));
                    }
                    match tokio::time::timeout(ASK_WITHIN, self.ask_on(ep, d, &req)).await {
                        Ok(Ok(Response::Probed { mut seen })) => {
                            // Said in that machine's language; said again in this one's.
                            seen.why = seen.why.map(|w| khor_catalog::reword(&w).unwrap_or(w));
                            seen
                        }
                        Ok(Ok(Response::Refused { why })) if why.contains("Probe") => ReachSeen {
                            outcome: ReachOutcome::Predates,
                            ..unasked(msg::reach_predates(d.shown()))
                        },
                        Ok(Ok(Response::Refused { why })) => unasked(why),
                        Ok(Ok(other)) => unasked(msg::serve_non_answer(format_args!("{other:?}"))),
                        Ok(Err(why)) => unasked(why),
                        Err(_) => unasked(msg::machine_unreachable_timeout(d.shown())),
                    }
                }
            })
            .collect();
        let (here, there) = tokio::join!(probe_here(host, port), crate::link::join_all(asks));
        let mut rows = vec![(me.clone(), here)];
        rows.extend(others.iter().map(|d| d.id.clone()).zip(there));
        Ok(ReachView {
            host: host.to_owned(),
            port,
            rows,
            entrance: None,
            disagree: false,
        }
        .pick(&me))
    }
}

fn unasked(why: String) -> ReachSeen {
    ReachSeen {
        outcome: ReachOutcome::Unasked,
        ms: None,
        addr: None,
        banner: None,
        host_key: None,
        why: Some(why),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_banner_is_its_first_line_printable_and_short() {
        assert_eq!(first_line(b"SSH-2.0-OpenSSH_9.6\r\nrest").as_deref(), Some("SSH-2.0-OpenSSH_9.6"));
        assert_eq!(first_line(b"\x1b[31mhi\x07\n").as_deref(), Some("[31mhi"));
        assert_eq!(first_line(b""), None);
        assert_eq!(first_line(&[b'a'; 400]).map(|s| s.len()), Some(REACH_BANNER_MAX));
    }

    #[test]
    fn a_key_is_shown_as_openssh_prints_it() {
        assert_eq!(shown_key(&"00".repeat(32)), format!("SHA256:{}", "A".repeat(43)));
    }

    /// One listener that says an ssh banner, one port nobody holds: the
    /// two outcomes a person sees most, off loopback only.
    #[tokio::test]
    async fn a_listener_is_reached_with_its_banner_and_a_closed_port_is_refused() {
        let l = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let port = l.local_addr().unwrap().port();
        tokio::spawn(async move {
            use tokio::io::AsyncWriteExt;
            while let Ok((mut s, _)) = l.accept().await {
                let _ = s.write_all(b"SSH-2.0-Fake_1.0\r\n").await;
            }
        });
        let none = |_: &str, _: u16| None;
        let seen = probe_with("127.0.0.1", port, none).await;
        assert_eq!(seen.outcome, ReachOutcome::Reached, "{seen:?}");
        assert_eq!(seen.banner.as_deref(), Some("SSH-2.0-Fake_1.0"));
        assert!(seen.ms.is_some());
        let closed = {
            let l = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
            l.local_addr().unwrap().port()
        };
        let seen = probe_with("127.0.0.1", closed, none).await;
        assert_eq!(seen.outcome, ReachOutcome::Refused, "{seen:?}");
        // A listener that says nothing: connected, not reached.
        let quiet = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let qport = quiet.local_addr().unwrap().port();
        tokio::spawn(async move {
            let mut held = Vec::new();
            while let Ok((s, _)) = quiet.accept().await {
                held.push(s);
            }
        });
        let seen = probe_with("127.0.0.1", qport, none).await;
        assert_eq!((seen.outcome, seen.banner.as_deref()), (ReachOutcome::Silent, None), "{seen:?}");
        // No `no-name` case here: a resolver behind a fake-ip proxy
        // answers every name, `.invalid` included (measured: 198.18.0.23),
        // so the only loopback-safe way to it is a lookup that is never made.
    }
}
