//! **khor inside ssh** (round ROAD-2, audit/ROAD-BRIEF.md T5): a member
//! that ssh reaches but that reaches neither the internet nor a relay is
//! a full peer through the ssh session of a machine that can log in to it.
//!
//! - **The carrier** (the machine named in the member's `carrier`
//!   register) keeps `ssh <road> khor _carry <carrier id>` running, with
//!   everything the row keeps for that road (port, jump, password through
//!   the askpass), and reconnects with backoff. The session's stdin and
//!   stdout are the carried link (`khor_net::carry`).
//! - **The member**: `khor _carry` hands its stdin and stdout to its own
//!   resident through a one-shot loopback port with a cookie, the way
//!   `khor pipe` does (`crate::pipe`), and the resident makes it the link
//!   to the carrier.
//!
//! iroh's handshake runs end to end over it, so ssh and the carrier's
//! sshd see ciphertext. **Only the carrier has the road.** Another
//! machine reaches the member through the carrier with the peer hop, or
//! not at all while the hop is off (audit/road-2.md).

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use khor_catalog::msg;
use khor_net::carry::Carrier;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::satellite::Dest;
use crate::{Node, ipc};

/// How often the carrier looks at which members it carries.
const LOOK_EVERY: Duration = Duration::from_secs(5);

/// How long the resident's one-shot port waits for `khor _carry`.
const CLAIM_WITHIN: Duration = Duration::from_secs(10);

/// A session that stayed up this long was a good one: the next failure
/// starts the backoff from the beginning.
const LIVED: Duration = Duration::from_secs(60);

/// How long the add flow's session to a machine not yet paired lives
/// ([`Node::carry_for`]): the pairing it is for, twice, with room.
const FOR_PAIRING: Duration = Duration::from_secs(180);

/// How long that session must have held before the pairing is tried:
/// long enough for the far `khor _carry` to have reached its resident.
const SETTLED: Duration = Duration::from_secs(3);

/// The carrier's wait before the next ssh after `failures` short ones.
fn backoff(failures: u32) -> Duration {
    Duration::from_secs(2u64.saturating_mul(1 << failures.min(5))).min(Duration::from_secs(60))
}

/// The far command: the member's own khor, by the path `khor add` puts it
/// at, on the member's own store (`KHOR_HOME` unset, as for `pair`).
fn far_command(carrier: &str) -> String {
    format!("env -u KHOR_HOME \"$HOME/.local/bin/khor\" _carry {carrier}")
}

impl Node {
    /// Whether this machine carries members over ssh (`ssh-carry`).
    pub(crate) fn carry_on(&self) -> bool {
        crate::config::on(self.root(), "ssh-carry")
    }

    /// Whether this machine asks for and serves the peer hop (`peer-hop`).
    pub(crate) fn hop_on(&self) -> bool {
        crate::config::on(self.root(), "peer-hop")
    }

    /// The carried transport to register at bind; `None` with both
    /// switches off, which is the transport not registered at all.
    pub(crate) fn carrying(&self) -> Option<&khor_net::carry::Links> {
        (self.carry_on() || self.hop_on()).then_some(&self.carry)
    }

    /// The carried address of `id`, when a link to it is up.
    pub(crate) fn carried_addr(&self, id: &str) -> Option<iroh::TransportAddr> {
        let peer: iroh::EndpointId = id.parse().ok()?;
        self.carry
            .has(&peer)
            .then(|| khor_net::carry::transport_addr(&peer))
    }

    /// The handle a carrying task keeps to reach [`let_path_go`]: weak, so
    /// a task never keeps the node alive.
    pub(crate) fn weak(&self) -> std::sync::Weak<Node> {
        self.me.get().cloned().unwrap_or_default()
    }

    /// The word for a wire on a carried path: over this machine's own ssh
    /// session, or forwarded by a third machine — by the link it rides, or
    /// rode until a moment ago ([`khor_net::carry::Links::last_carrier`]).
    pub(crate) fn carried_hop(&self, id: &str) -> khor_core::Hop {
        let carrier = id.parse().ok().and_then(|p| self.carry.last_carrier(&p));
        match carrier {
            Some(Carrier::Hop { .. }) => khor_core::Hop::Forwarded,
            _ => khor_core::Hop::Ssh,
        }
    }

    /// **`khor carry <machine> <ssh host>`**: this machine carries the
    /// member over ssh by `road` from now on; `None` stops it. `secret`,
    /// when given, is the road's password or key passphrase, kept in this
    /// machine's secret store as `khor add` keeps one (round ROAD-4); the
    /// sweep drops it once this machine no longer carries the row.
    pub fn set_carried(
        &self,
        machine: &str,
        road: Option<&Dest>,
        secret: Option<&str>,
    ) -> Result<String, String> {
        let (channel, id) = self.resolve(machine)?;
        let id = id.hex();
        let loaded = self.devices_loaded()?;
        let row = loaded
            .doc
            .all()
            .into_iter()
            .find(|d| d.id == id)
            .ok_or_else(|| msg::carry_not_a_machine(machine))?;
        if row.sponsor.is_some() {
            return Err(msg::carry_is_satellite(row.shown()));
        }
        let kept = match (road, secret) {
            (Some(r), Some(s)) if r.asks() => {
                crate::secret::keep(self.root(), &r.secret_key(), s, crate::now_ms())?;
                Some(r.secret_key())
            }
            _ => None,
        };
        let me = self.device_str().to_owned();
        let carrier = road.map(|_| me.as_str());
        let written = loaded
            .doc
            .set_carried(&id, carrier, road.map(Dest::as_row).as_ref())
            .and_then(|()| {
                let mut store = loaded.store;
                store.flush(&loaded.doc)
            });
        if let Err(e) = written {
            if let Some(k) = kept {
                let _ = crate::secret::drop_kept(self.root(), &k);
            }
            return Err(e);
        }
        Ok(match road {
            Some(r) => msg::carry_set(&channel, r),
            None => msg::carry_unset(&channel),
        })
    }

    /// **`khor _carry <carrier>`, the member's end**: this process's
    /// stdin and stdout become its resident's link to `carrier`, until
    /// either side hangs up.
    pub async fn carry_in<I, O>(&self, carrier: &str, input: I, output: O) -> Result<(), String>
    where
        I: AsyncRead + Unpin,
        O: AsyncWrite + Unpin,
    {
        carrier
            .parse::<iroh::EndpointId>()
            .map_err(|_| msg::carry_not_a_machine(carrier))?;
        if !self.carry_on() {
            return Err(msg::carry_off().to_owned());
        }
        let Some(reply) = self
            .via_serve(ipc::Op::CarryIn {
                carrier: carrier.to_owned(),
            })
            .await
        else {
            return Err(msg::carry_needs_serve().to_owned());
        };
        let (addr, cookie) = match reply? {
            ipc::Reply::Piping { addr, cookie } => (addr, cookie),
            ipc::Reply::Refused { why } => return Err(why),
            other => return Err(msg::serve_non_answer(format_args!("{other:?}"))),
        };
        let mut tcp = tokio::net::TcpStream::connect(&addr)
            .await
            .map_err(msg::serve_unreachable)?;
        tcp.write_all(format!("{cookie}\n").as_bytes())
            .await
            .map_err(msg::handoff_failed)?;
        let _ = tcp.set_nodelay(true);
        let (r, w) = tcp.into_split();
        crate::pipe::splice(input, output, r, w).await;
        Ok(())
    }

    /// The resident's half of [`Self::carry_in`]: a loopback port that
    /// makes the one connection saying the cookie first the link to
    /// `carrier`. Only a machine in the table is carried.
    ///
    /// **A machine not in the table yet is carried too** (round ROAD-3):
    /// that is the add flow, which carries a machine before it pairs over
    /// the link. Only a local process holding the hand-off cookie asks
    /// this, and that is someone who logged in here; what the link then
    /// carries still meets iroh's handshake and the roster's gate.
    pub(crate) async fn carry_hosted(&self, carrier: &str) -> Result<(String, String), String> {
        let peer: iroh::EndpointId = carrier
            .parse()
            .map_err(|_| msg::carry_not_a_machine(carrier))?;
        if !self.carry_on() || self.carrying().is_none() {
            return Err(msg::carry_off().to_owned());
        }
        if self
            .devices()?
            .iter()
            .any(|d| d.id == carrier && d.sponsor.is_some())
        {
            return Err(msg::carry_not_a_machine(carrier));
        }
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .map_err(|e| e.to_string())?;
        let addr = listener
            .local_addr()
            .map_err(|e| e.to_string())?
            .to_string();
        let cookie = crate::link::fresh_hex()?;
        let want = format!("{cookie}\n");
        let (links, wires, id, node) = (
            self.carry.clone(),
            self.wires.clone(),
            carrier.to_owned(),
            self.weak(),
        );
        tokio::spawn(async move {
            let claimed = tokio::time::timeout(CLAIM_WITHIN, async {
                loop {
                    let Ok((mut sock, _)) = listener.accept().await else {
                        return None;
                    };
                    let mut said = vec![0u8; want.len()];
                    if sock.read_exact(&mut said).await.is_ok() && said == want.as_bytes() {
                        return Some(sock);
                    }
                }
            })
            .await;
            if let Ok(Some(sock)) = claimed {
                let _ = sock.set_nodelay(true);
                let (r, w) = sock.into_split();
                let attached = links.attach(peer, Carrier::Ssh, r, w);
                offer_path(&node, peer).await;
                wires.forgive(&id);
                wires.note(
                    Some(&id),
                    crate::journal::What::Carry {
                        up: true,
                        why: None,
                    },
                );
                attached.ended().await;
                let_path_go(&node, &links, peer).await;
                wires.note(
                    Some(&id),
                    crate::journal::What::Carry {
                        up: false,
                        why: None,
                    },
                );
            }
        });
        Ok((addr, cookie))
    }

    /// **The carrier**: one ssh session up to each member this machine
    /// carries, for as long as the row says so and the switch is on.
    pub(crate) async fn carrier(self: Arc<Self>) {
        let mut running: HashMap<String, (Dest, tokio::task::AbortHandle)> = HashMap::new();
        let mut tick = tokio::time::interval(LOOK_EVERY);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tick.tick().await;
            let me = self.device_str();
            let want: HashMap<String, Dest> = self
                .devices()
                .unwrap_or_default()
                .into_iter()
                .filter(|d| d.carrier.as_deref() == Some(me) && d.sponsor.is_none())
                .filter(|d| !crate::pause::peer_on(self.root(), &d.id))
                .filter_map(|d| Some((d.id.clone(), Dest::of(&d)?)))
                .collect();
            running.retain(|id, (dest, task)| {
                let keep = want.get(id) == Some(dest) && !task.is_finished();
                if !keep {
                    task.abort();
                }
                keep
            });
            for (id, dest) in want {
                if running.contains_key(&id) {
                    continue;
                }
                let Ok(peer) = id.parse::<iroh::EndpointId>() else {
                    continue;
                };
                let one = self.one_carry(peer, dest.clone());
                let task = tokio::spawn(one.run(None));
                running.insert(id, (dest, task.abort_handle()));
            }
        }
    }

    fn one_carry(&self, peer: iroh::EndpointId, to: Dest) -> OneCarry {
        OneCarry {
            links: self.carry.clone(),
            wires: self.wires.clone(),
            node: self.weak(),
            me: self.device_str().to_owned(),
            peer,
            to,
        }
    }

    /// **The add flow's carry** (round ROAD-3, `ipc::Op::CarryFor`): an
    /// ssh session to a machine that has khor but did not connect back,
    /// before it is paired, so the pairing can ride it. It lives for
    /// [`FOR_PAIRING`], or until another session to it takes over (the
    /// carrier's, once the row names this machine). Answers once the
    /// session has held [`SETTLED`].
    pub(crate) async fn carry_for(&self, id: &str, to: Dest) -> Result<(), String> {
        let peer: iroh::EndpointId = id.parse().map_err(|_| msg::carry_not_a_machine(id))?;
        if !self.carry_on() || self.carrying().is_none() {
            return Err(msg::carry_off().to_owned());
        }
        let to_host = to.host.clone();
        if !self.carry.has(&peer) {
            let one = self.one_carry(peer, to);
            tokio::spawn(one.run(Some(Instant::now() + FOR_PAIRING)));
        }
        let started = Instant::now();
        let mut up_since = None;
        while started.elapsed() < SETTLED * 6 {
            up_since = if self.carry.has(&peer) {
                up_since.or(Some(Instant::now()))
            } else {
                None
            };
            if up_since.is_some_and(|t| t.elapsed() >= SETTLED) {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        Err(msg::carry_not_up(&to_host))
    }
}

/// **A carried link ended: let iroh drop its path now** (vendor/iroh patch
/// 13), so the selector chooses again among the rest at once rather than
/// keeping the dead path on its last round trip until iroh's 15 s path
/// idle. Nothing when a newer link to the machine took its place (it rides
/// the same path) or no resident is serving. The ears are read at the end,
/// not when the link came up: fresh ears move the links to a new endpoint.
pub(crate) async fn let_path_go(
    node: &std::sync::Weak<Node>,
    links: &khor_net::carry::Links,
    peer: iroh::EndpointId,
) {
    if links.has(&peer) {
        return;
    }
    if let Some(ep) = node.upgrade().and_then(|n| n.current_ears()) {
        ep.close_carried(peer).await;
    }
}

/// **A carried link came up: open its path on the live connections**
/// (vendor/iroh patch 12), the other half of [`let_path_go`]. A link that
/// ended had its path closed, so a reconnect is offered at once, not at
/// the next dial.
pub(crate) async fn offer_path(node: &std::sync::Weak<Node>, peer: iroh::EndpointId) {
    if let Some(ep) = node.upgrade().and_then(|n| n.current_ears()) {
        ep.offer_addrs(iroh::EndpointAddr::from_parts(
            peer,
            [khor_net::carry::transport_addr(&peer)],
        ))
        .await;
    }
}

/// One ssh session kept up to one machine: what it needs of the node.
struct OneCarry {
    links: khor_net::carry::Links,
    wires: Arc<crate::wires::Wires>,
    node: std::sync::Weak<Node>,
    me: String,
    peer: iroh::EndpointId,
    to: Dest,
}

impl OneCarry {
    /// Keeps the session up, reconnecting with backoff. `until`: the add
    /// flow's session, which also ends when another link to the machine
    /// took its place.
    async fn run(self, until: Option<Instant>) {
        let OneCarry {
            links,
            wires,
            node,
            me,
            peer,
            to,
        } = self;
        let id = peer.to_string();
        let mut failures = 0u32;
        loop {
            let started = Instant::now();
            let mut ssh = tokio::process::Command::from(to.command("ssh"));
            ssh.args([
                "-T",
                "-o",
                "ServerAliveInterval=15",
                "-o",
                "ServerAliveCountMax=3",
            ])
            .args(["-o", "ConnectTimeout=10"])
            .arg(&to.host)
            .arg(far_command(&me))
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);
            if let Ok(mut child) = ssh.spawn()
                && let (Some(w), Some(r)) = (child.stdin.take(), child.stdout.take())
            {
                // What ssh (or the far `khor _carry`) last said, for the
                // resident's log when the session ends.
                let said = child.stderr.take().map(|mut e| {
                    tokio::spawn(async move {
                        let mut all = Vec::new();
                        let _ = e.read_to_end(&mut all).await;
                        String::from_utf8_lossy(&all)
                            .trim()
                            .lines()
                            .last()
                            .unwrap_or("")
                            .to_owned()
                    })
                });
                let attached = links.attach(peer, Carrier::Ssh, r, w);
                offer_path(&node, peer).await;
                wires.forgive(&id);
                wires.note(
                    Some(&id),
                    crate::journal::What::Carry {
                        up: true,
                        why: None,
                    },
                );
                let ended = attached.ended();
                tokio::pin!(ended);
                let mut gone = false;
                tokio::select! {
                    () = &mut ended => gone = true,
                    _ = child.wait() => {}
                }
                let _ = child.start_kill();
                let _ = child.wait().await;
                // ssh exited first: its pipes are closed, and the link ends
                // as soon as the reader sees it.
                if !gone {
                    let _ = tokio::time::timeout(Duration::from_secs(2), &mut ended).await;
                }
                let_path_go(&node, &links, peer).await;
                let mut why = None;
                if let Some(said) = said
                    && let Ok(Ok(line)) = tokio::time::timeout(Duration::from_secs(2), said).await
                    && !line.is_empty()
                {
                    eprintln!("carry {}: {line}", to.host);
                    why = Some(line);
                }
                wires.note(Some(&id), crate::journal::What::Carry { up: false, why });
            }
            if let Some(until) = until
                && (Instant::now() >= until || links.has(&peer))
            {
                return;
            }
            failures = if started.elapsed() >= LIVED {
                0
            } else {
                failures.saturating_add(1)
            };
            tokio::time::sleep(backoff(failures)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_carrier_backs_off_to_a_minute() {
        let s = |n| backoff(n).as_secs();
        assert_eq!(
            [s(0), s(1), s(2), s(3), s(4), s(5), s(9)],
            [2, 4, 8, 16, 32, 60, 60]
        );
    }

    /// Round ROAD-4: a member this machine carries logs in by its road as
    /// a satellite does, so the secret that road asks for is kept from
    /// the sweep — and is not once another machine carries it.
    #[test]
    fn a_carried_members_secret_is_kept_from_the_sweep() {
        let dir = std::env::temp_dir().join(format!("khor-carry-secret-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let n = crate::Node::open_as(dir.clone(), "carrier").unwrap();
        let me = n.device_str().to_owned();
        let member = "cd".repeat(32);
        let road = Dest::new("box", None)
            .on_port(Some(2222))
            .with_identity(Some("/k"))
            .with_passphrase(true);
        let carried_by = |carrier: &str| {
            let loaded = n.devices_loaded().unwrap();
            loaded.doc.upsert(&member, "box", &[]).unwrap();
            loaded
                .doc
                .set_carried(&member, Some(carrier), Some(&road.as_row()))
                .unwrap();
            let mut store = loaded.store;
            store.flush(&loaded.doc).unwrap();
            n.devices().unwrap()
        };
        let kept = n.secrets_in_use(&carried_by(&me));
        assert!(kept.contains("box:2222"), "carried here: {kept:?}");
        let other = "ef".repeat(32);
        assert!(
            n.secrets_in_use(&carried_by(&other)).is_empty(),
            "another machine keeps its own"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_far_command_runs_the_members_own_khor() {
        assert_eq!(
            far_command("ab"),
            "env -u KHOR_HOME \"$HOME/.local/bin/khor\" _carry ab"
        );
    }
}
