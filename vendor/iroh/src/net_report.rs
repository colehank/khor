//! Checks the network conditions from the current host.
//!
//! NetReport is responsible for finding out the network conditions of the current host, like
//! whether it is connected to the internet via IPv4 and/or IPv6, what the NAT situation is
//! etc and reachability to the configured relays.
// Based on <https://github.com/tailscale/tailscale/blob/main/net/netcheck/netcheck.go>

#![cfg_attr(wasm_browser, allow(unused))]

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Debug,
    net::SocketAddr,
    sync::Arc,
};

use defaults::timeouts::PROBES_TIMEOUT;
use iroh_base::RelayUrl;
#[cfg(not(wasm_browser))]
use iroh_dns::dns::DnsResolver;
#[cfg(not(wasm_browser))]
use iroh_relay::{RelayConfig, quic::QuicClient};
use iroh_relay::{
    RelayMap,
    quic::{QUIC_ADDR_DISC_CLOSE_CODE, QUIC_ADDR_DISC_CLOSE_REASON},
};
use n0_error::e;
#[cfg(not(wasm_browser))]
use n0_error::stack_error;
#[cfg(not(wasm_browser))]
use n0_future::task;
use n0_future::{
    StreamExt,
    task::AbortOnDropHandle,
    time::{self, Duration, Instant},
};
use n0_watcher::{Watchable, Watcher};
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tracing::{debug, trace, warn};

use self::reportgen::{ProbeFinished, ProbeReport};
#[cfg(not(wasm_browser))]
use self::reportgen::{QadProbeReport, SocketState};
#[cfg_attr(not(feature = "unstable-net-report"), allow(unreachable_pub))]
pub use self::{
    // exported primarily for use in documentation
    defaults::timeouts::TIMEOUT,
    metrics::Metrics,
    probes::Probe,
    report::{RelayLatencies, Report},
};
pub(crate) use self::{
    options::Options,
    reportgen::{IfStateDetails, QuicConfig},
};

mod defaults;
mod metrics;
mod options;
mod probes;
mod report;
mod reportgen;

#[cfg(not(wasm_browser))]
#[allow(missing_docs)]
#[stack_error(derive, add_meta)]
#[non_exhaustive]
enum QadProbeError {
    #[error("Failed to resolve relay address")]
    GetRelayAddr {
        source: self::reportgen::GetRelayAddrError,
    },
    #[error("Missing host in relay URL")]
    MissingHost,
    #[error("QUIC connection failed")]
    Quic { source: iroh_relay::quic::Error },
    #[error("Receiver dropped")]
    ReceiverDropped,
}

/// Configuration for the net report component.
///
/// Controls which probes and checks are performed when generating network reports.
/// All options default to `true`.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct NetReportConfig {
    /// Run HTTPS latency probes against relay servers.
    ///
    /// HTTPS latency probes perform an empty HTTPS GET request to each configured
    /// relay server and measure latency.
    ///
    /// They are performed in addition to the QUIC address discovery (QAD) probes.
    /// In networks that do not allow QUIC traffic, they are the only way to detect
    /// relay latencies and thus the preferred relay.
    ///
    /// Disabling them is harmless on networks that do allow QUIC traffic, but will
    /// completely prevent finding the home relay on networks that do block QUIC.
    pub https_probes: bool,

    /// Check for captive portals when generating the first report.
    ///
    /// This is done by accessing a well-known URL that is available on each relay
    /// server, `/generate_204`. If a GET request to this URL returns anything else
    /// but a 204 No Content response, we assume we are behind a captive portal.
    ///
    /// When we have detected that we are behind a captive portal, we try to contact
    /// the relay servers more frequently in case the captive portal status changes.
    pub captive_portal_check: bool,

    /// khor patch 3 (vendor/iroh/KHOR.md): a relay to home on whenever the
    /// current report reached it at all, whatever its latency against the
    /// others. `None` keeps iroh's own choice (lowest recent latency).
    pub preferred_relay: Option<RelayUrl>,
    /// khor patch 5 (vendor/iroh/KHOR.md): `(old, new)` pairs — a relay
    /// and the one that replaces it. Whenever the home would be `old`
    /// and the current report reached `new`, the home is `new`.
    pub successors: Vec<(RelayUrl, RelayUrl)>,
    /// khor patch 23 (vendor/iroh/KHOR.md): relays kept behind the rest —
    /// a home only in a report where no other relay answered its HTTPS
    /// probe (n0's public tier behind one's own).
    pub fallback: Vec<RelayUrl>,
    /// khor patch 24 (vendor/iroh/KHOR.md): how long a home that still
    /// answers is kept before latency alone may move it. Zero is
    /// upstream's: any report may move it.
    pub home_dwell: Duration,
}

impl NetReportConfig {
    /// Creates a minimal configuration that disables all optional probes and checks.
    pub fn minimal() -> Self {
        Self {
            https_probes: false,
            captive_portal_check: false,
            preferred_relay: None,
            successors: Vec::new(),
            fallback: Vec::new(),
            home_dwell: Duration::ZERO,
        }
    }
}

impl Default for NetReportConfig {
    fn default() -> Self {
        Self {
            https_probes: true,
            captive_portal_check: true,
            preferred_relay: None,
            successors: Vec::new(),
            fallback: Vec::new(),
            home_dwell: Duration::ZERO,
        }
    }
}

const FULL_REPORT_INTERVAL: Duration = Duration::from_secs(5 * 60);
const ENOUGH_ENDPOINTS: usize = 3;

/// Client to run net_reports.
#[derive(Debug)]
pub(crate) struct Client {
    #[cfg(not(wasm_browser))]
    socket_state: SocketState,
    metrics: Arc<Metrics>,
    probes: BTreeSet<Probe>,
    relay_map: RelayMap,
    #[cfg(not(wasm_browser))]
    qad_conns: QadConns,
    #[cfg(not(wasm_browser))]
    tls_config: rustls::ClientConfig,
    /// Whether to check for captive portals.
    captive_portal_check: bool,
    /// khor patch 3: the relay to home on when reachable.
    preferred_relay: Option<RelayUrl>,
    /// khor patch 5: relays and the ones that replace them.
    successors: Vec<(RelayUrl, RelayUrl)>,
    /// khor patch 23: relays that are a home only when no other answered.
    fallback: Vec<RelayUrl>,
    /// khor patch 24: how long a home that answers is kept against latency.
    home_dwell: Duration,
    /// khor patch 24: the home this client chose last, and since when. Kept
    /// apart from `reports.last`, which a full report clears before it runs.
    home: Option<(RelayUrl, Instant)>,
    /// khor patch 25: when the home connection dropped, per relay
    /// ([`Client::follow_home`]).
    home_drops: Arc<std::sync::Mutex<Vec<(RelayUrl, Instant)>>>,
    /// khor patch 25: until when the successor rule is off for this machine.
    successors_off_until: Option<Instant>,
    /// khor patch 25: the task that fills `home_drops`.
    _follow_home: Option<AbortOnDropHandle<()>>,
    /// A collection of previously generated reports.
    ///
    /// Sometimes it is useful to look at past reports to decide what to do.
    reports: Reports,
}

#[cfg(not(wasm_browser))]
#[derive(Debug, Default)]
struct QadConns {
    v4: Option<(RelayUrl, QadConn)>,
    v6: Option<(RelayUrl, QadConn)>,
}

#[cfg(not(wasm_browser))]
impl QadConns {
    fn clear(&mut self) {
        if let Some((_, conn)) = self.v4.take() {
            conn.conn
                .close(QUIC_ADDR_DISC_CLOSE_CODE, QUIC_ADDR_DISC_CLOSE_REASON);
        }
        if let Some((_, conn)) = self.v6.take() {
            conn.conn
                .close(QUIC_ADDR_DISC_CLOSE_CODE, QUIC_ADDR_DISC_CLOSE_REASON);
        }
    }

    fn current_v4(&self) -> Option<ProbeReport> {
        if let Some((_, ref conn)) = self.v4
            && let Some(mut r) = conn.observer.get()
        {
            // grab latest rtt

            use noq_proto::PathId;
            if let Some(latency) = conn.conn.rtt(PathId::ZERO) {
                r.latency = latency;
            }
            return Some(ProbeReport::QadIpv4(r));
        }
        None
    }

    fn current_v6(&self) -> Option<ProbeReport> {
        if let Some((_, ref conn)) = self.v6
            && let Some(mut r) = conn.observer.get()
        {
            // grab latest rtt

            use noq_proto::PathId;
            if let Some(latency) = conn.conn.rtt(PathId::ZERO) {
                r.latency = latency;
            }
            return Some(ProbeReport::QadIpv6(r));
        }
        None
    }

    fn watch_v4(&self) -> impl n0_future::Stream<Item = Option<QadProbeReport>> + Unpin + use<> {
        let watcher = self.v4.as_ref().map(|(_url, conn)| conn.observer.watch());

        if let Some(watcher) = watcher {
            watcher.stream_updates_only().boxed()
        } else {
            n0_future::stream::empty().boxed()
        }
    }

    fn watch_v6(&self) -> impl n0_future::Stream<Item = Option<QadProbeReport>> + Unpin + use<> {
        let watcher = self.v6.as_ref().map(|(_url, conn)| conn.observer.watch());
        if let Some(watcher) = watcher {
            watcher.stream_updates_only().boxed()
        } else {
            n0_future::stream::empty().boxed()
        }
    }
}

#[cfg(not(wasm_browser))]
#[derive(Debug)]
struct QadConn {
    conn: noq::Connection,
    observer: Watchable<Option<QadProbeReport>>,
    _handle: AbortOnDropHandle<Option<()>>,
}

#[derive(Debug)]
struct Reports {
    /// Do a full relay scan, even if last is `Some`.
    next_full: bool,
    /// Some previous reports.
    prev: BTreeMap<Instant, Report>,
    /// Most recent report.
    last: Option<Report>,
    /// Time of last full (non-incremental) report.
    last_full: Instant,
}

impl Default for Reports {
    fn default() -> Self {
        Self {
            next_full: true,
            prev: Default::default(),
            last: Default::default(),
            last_full: Instant::now(),
        }
    }
}

impl Client {
    /// Creates a new net_report client.
    pub(crate) fn new(
        #[cfg(not(wasm_browser))] dns_resolver: DnsResolver,
        relay_map: RelayMap,
        opts: Options,
        metrics: Arc<Metrics>,
    ) -> Self {
        let probes = opts.as_protocols();

        #[cfg(not(wasm_browser))]
        let quic_client = opts
            .quic_config
            .map(|c| iroh_relay::quic::QuicClient::new(c.ep, c.client_config));

        #[cfg(not(wasm_browser))]
        let socket_state = SocketState {
            quic_client,
            dns_resolver,
            proxy_url: opts.proxy_url,
        };

        Client {
            #[cfg(not(wasm_browser))]
            socket_state,
            metrics,
            reports: Reports::default(),
            probes,
            relay_map,
            #[cfg(not(wasm_browser))]
            qad_conns: QadConns::default(),
            #[cfg(not(wasm_browser))]
            tls_config: opts.tls_config,
            captive_portal_check: opts.user_config.captive_portal_check,
            preferred_relay: opts.user_config.preferred_relay.clone(),
            successors: opts.user_config.successors.clone(),
            fallback: opts.user_config.fallback.clone(),
            home_dwell: opts.user_config.home_dwell,
            home: None,
            home_drops: Default::default(),
            successors_off_until: None,
            _follow_home: None,
        }
    }

    /// khor patch 25 (vendor/iroh/KHOR.md): notes every time the home
    /// connection drops — connected, then not, on the same relay — for the
    /// successor rule to read ([`Client::add_report_history_and_set_preferred_relay`]).
    #[cfg(not(wasm_browser))]
    pub(crate) fn follow_home<W>(&mut self, home: W)
    where
        W: Watcher<Value = Vec<crate::endpoint::RelayStatus>> + Unpin + Send + 'static,
    {
        let drops = self.home_drops.clone();
        let task = task::spawn(async move {
            let mut stream = home.stream();
            let mut was: Vec<(RelayUrl, bool)> = Vec::new();
            while let Some(now) = stream.next().await {
                for s in &now {
                    let dropped =
                        !s.is_connected() && was.iter().any(|(u, up)| u == s.url() && *up);
                    if dropped && let Ok(mut d) = drops.lock() {
                        d.push((s.url().clone(), Instant::now()));
                    }
                }
                was = now
                    .iter()
                    .map(|s| (s.url().clone(), s.is_connected()))
                    .collect();
            }
        });
        self._follow_home = Some(AbortOnDropHandle::new(task));
    }

    /// Generates a [`Report`].
    ///
    /// Look at [`Options`] for the different configuration options.
    pub(crate) async fn get_report(
        &mut self,
        if_state: IfStateDetails,
        is_major: bool,
        shutdown_token: CancellationToken,
    ) -> Report {
        let now = Instant::now();

        let mut do_full = is_major
            || self.reports.next_full
            || now.duration_since(self.reports.last_full) > FULL_REPORT_INTERVAL;

        debug!(%do_full, "net_report starting");

        // If the last report had a captive portal and reported no UDP access,
        // it's possible that we didn't get a useful net_report due to the
        // captive portal blocking us. If so, make this report a full (non-incremental) one.
        if !do_full
            && let Some(ref last) = self.reports.last
            && !last.has_udp()
            && last.captive_portal == Some(true)
        {
            do_full = true;
        }
        // khor patch 25: a new network may hold a successor this one reset.
        if is_major {
            self.successors_off_until = None;
        }
        if do_full {
            self.reports.last = None; // causes ProbePlan::new below to do a full (initial) plan
            self.reports.next_full = false;
            self.reports.last_full = now;
            self.metrics.reports_full.inc();
        }
        self.metrics.reports.inc();

        let num_relays = self.relay_map.len();
        let enough_relays = std::cmp::min(num_relays, ENOUGH_ENDPOINTS);
        #[cfg(wasm_browser)]
        let if_state = IfStateDetails::default();
        #[cfg(not(wasm_browser))]
        let if_state = IfStateDetails {
            have_v4: if_state.have_v4,
            have_v6: if_state.have_v6,
        };

        let mut report = Report::default();
        // khor patch 11: a report whose plan is the initial one (no last
        // report, or one that measured nothing; `ProbePlan::with_last_report`)
        // probes every relay even when it is not `do_full`.
        let initial_plan = self.reports.last.as_ref().is_none_or(|l| l.relay_latency.is_empty());
        let mut probes_ended = false;

        // Start the reportgen client to start any needed probes
        let (actor, mut probe_rx) = reportgen::Client::new(
            self.reports.last.clone(),
            self.relay_map.clone(),
            self.probes.clone(),
            self.captive_portal_check,
            if_state.clone(),
            shutdown_token.child_token(),
            #[cfg(not(wasm_browser))]
            self.socket_state.clone(),
            #[cfg(not(wasm_browser))]
            self.tls_config.clone(),
        );

        #[cfg(not(wasm_browser))]
        let reports = self
            .spawn_qad_probes(
                &if_state,
                enough_relays,
                do_full,
                shutdown_token.child_token(),
            )
            .await;

        #[cfg(not(wasm_browser))]
        for r in reports {
            report.update(&r);
        }

        if self.have_enough_reports(&if_state, do_full, num_relays, &report) {
            // check if we already have enough probes immediately after QAD returns
            trace!("have enough probe reports, aborting further probes");
            // shuts down the probes
            drop(actor);
        } else {
            #[cfg(not(wasm_browser))]
            let mut qad_v4_stream = self.qad_conns.watch_v4();
            #[cfg(wasm_browser)]
            let mut qad_v4_stream = n0_future::stream::empty::<Option<()>>();
            #[cfg(not(wasm_browser))]
            let mut qad_v6_stream = self.qad_conns.watch_v6();
            #[cfg(wasm_browser)]
            let mut qad_v6_stream = n0_future::stream::empty::<Option<()>>();

            loop {
                tokio::select! {
                    biased;

                    Some(Some(r)) = qad_v4_stream.next() => {
                        #[cfg(not(wasm_browser))]
                        {
                            trace!(?r, "new report from QAD V4");
                            report.update(&ProbeReport::QadIpv4(r));
                        }
                    }

                    Some(Some(r)) = qad_v6_stream.next() => {
                        #[cfg(not(wasm_browser))]
                        {
                            trace!(?r, "new report from QAD V6");
                            report.update(&ProbeReport::QadIpv6(r));
                        }
                    }

                    maybe_probe = probe_rx.recv() => {
                        let Some(probe_res) = maybe_probe else {
                            probes_ended = true;
                            break;
                        };
                        match probe_res {
                            ProbeFinished::Regular(probe) => match probe {
                                Ok(probe) => {
                                    report.update(&probe);
                                    if self.have_enough_reports(&if_state, do_full, num_relays, &report) {
                                        trace!("have enough probe reports, aborting further probes");
                                        // shuts down the probes
                                        drop(actor);
                                        break;
                                    }
                                }
                                Err(err) => {
                                    trace!("probe failed: {:?}", err);
                                }
                            },
                            #[cfg(not(wasm_browser))]
                            ProbeFinished::CaptivePortal(portal) => {
                                report.captive_portal = portal;
                            }
                        }
                    }
                }
            }
        }
        // khor patch 11: every relay was tried when the report is full, or
        // its plan was the initial one and every probe ran to its end (not
        // cut short by the QAD shortcut). A home missing from such a report
        // did not answer.
        let measured_all = do_full || (initial_plan && probes_ended);
        self.add_report_history_and_set_preferred_relay(&mut report, measured_all);
        debug!(
            ?report,
            duration = ?now.elapsed(),
            "net_report generated",
        );

        report
    }

    #[cfg(not(wasm_browser))]
    async fn spawn_qad_probes(
        &mut self,
        if_state: &IfStateDetails,
        enough_relays: usize,
        do_full: bool,
        shutdown_token: CancellationToken,
    ) -> Vec<ProbeReport> {
        use tracing::{Instrument, info_span};

        let Some(ref quic_client) = self.socket_state.quic_client else {
            return Vec::new();
        };

        if do_full {
            // clear out existing connections if we are doing a full reset
            self.qad_conns.clear();
        }

        if let Some((url, conn)) = &self.qad_conns.v4 {
            // verify conn is still around
            if let Some(reason) = conn.conn.close_reason() {
                trace!(?url, "QAD v4 conn closed: {}", reason);
                self.qad_conns.v4.take();
            }
        }
        if let Some((url, conn)) = &self.qad_conns.v6 {
            // verify conn is still around
            if let Some(reason) = conn.conn.close_reason() {
                trace!(?url, "QAD v6 conn closed: {}", reason);
                self.qad_conns.v6.take();
            }
        }

        let v4_report = self.qad_conns.current_v4();
        let v6_report = self.qad_conns.current_v6();
        let needs_v4_probe = v4_report.is_none();
        let needs_v6_probe = v6_report.is_some() != if_state.have_v6;

        let mut reports = Vec::new();

        if let Some(report) = v4_report {
            reports.push(report);
        }
        if let Some(report) = v6_report {
            reports.push(report);
        }

        if !needs_v4_probe && !needs_v6_probe {
            return reports;
        }

        trace!("spawning QAD probes");

        // TODO: randomize choice?
        const MAX_RELAYS: usize = 5;

        let mut v4_buf = JoinSet::new();
        let cancel_v4 = shutdown_token.child_token();
        let mut v6_buf = JoinSet::new();
        let cancel_v6 = shutdown_token.child_token();

        let relays = self.relay_map.relays::<Vec<_>>();
        for relay in relays.into_iter().take(MAX_RELAYS) {
            if if_state.have_v4 && needs_v4_probe {
                trace!(?relay.url, "v4 QAD probe starting");
                let relay = relay.clone();
                let dns_resolver = self.socket_state.dns_resolver.clone();
                let quic_client = quic_client.clone();
                let relay_url = relay.url.clone();
                let inner_token = cancel_v4.child_token();
                v4_buf.spawn(
                    cancel_v4
                        .child_token()
                        .run_until_cancelled_owned(time::timeout(
                            PROBES_TIMEOUT,
                            run_probe_v4(relay, quic_client, dns_resolver, inner_token),
                        ))
                        .instrument(info_span!("QADv4", %relay_url)),
                );
            }
            if if_state.have_v6 && needs_v6_probe {
                trace!(?relay.url, "v6 QAD probe starting");
                let relay = relay.clone();
                let dns_resolver = self.socket_state.dns_resolver.clone();
                let quic_client = quic_client.clone();
                let relay_url = relay.url.clone();
                let inner_token = cancel_v6.child_token();
                v6_buf.spawn(
                    cancel_v6
                        .child_token()
                        .run_until_cancelled_owned(time::timeout(
                            PROBES_TIMEOUT,
                            run_probe_v6(relay, quic_client, dns_resolver, inner_token),
                        ))
                        .instrument(info_span!("QADv6", %relay_url)),
                );
            }
        }

        // We set _pending to true if at least one report was started for each category.
        // If we did not start any report for either category, _pending is set to false right away
        // (it "completed" in the sense that nothing will ever run). If we did start at least one report,
        // _pending is set to true, and will be set to false further down once the first task
        // completed.
        let mut ipv4_pending = !v4_buf.is_empty();
        let mut ipv6_pending = !v6_buf.is_empty();

        while !v4_buf.is_empty() || !v6_buf.is_empty() {
            // We early-abort the tasks once we have at least `enough_relays` reports,
            // and at least one ipv4 and one ipv6 report completed (if they were started, see comment above).

            if reports.len() >= enough_relays && !ipv4_pending && !ipv6_pending {
                debug!("enough probes: {}", reports.len());
                cancel_v4.cancel();
                cancel_v6.cancel();
                break;
            }

            tokio::select! {
                biased;

                _ = shutdown_token.cancelled() => {
                    trace!("qad report cancelled");
                    break;
                }

                val = v4_buf.join_next(), if !v4_buf.is_empty() => {
                    let span = info_span!("QADv4");
                    let _guard = span.enter();
                    // khor patch 26: pending until one brings an address.
                    if qad_family_done(false, v4_buf.len()) {
                        ipv4_pending = false;
                    }
                    match val {
                        Some(Ok(Some(Ok(res)))) => {
                            match res {
                                Ok((r, conn)) => {
                                    ipv4_pending = !qad_family_done(true, v4_buf.len());
                                    debug!(?r, "probe report");
                                    let url = r.relay.clone();
                                    reports.push(ProbeReport::QadIpv4(r));
                                    if self.qad_conns.v4.is_none() {
                                        self.qad_conns.v4.replace((url, conn));
                                    } else {
                                        conn.conn.close(QUIC_ADDR_DISC_CLOSE_CODE, QUIC_ADDR_DISC_CLOSE_REASON);
                                    }
                                }
                                Err(err) => {
                                    debug!("probe failed: {err:#}");
                                }
                            }
                        }
                        Some(Err(err)) => {
                            if err.is_panic() {
                                panic!("probe panicked: {err:#}");
                            }
                            warn!("probe failed: {err:#}");
                        }
                        Some(Ok(None)) => {
                            debug!("probe canceled");
                        }
                        Some(Ok(Some(Err(time::Elapsed { .. })))) => {
                            debug!("probe timed out");
                        }
                        None => {
                            trace!("report canceled");
                        }
                    }
                }
                val = v6_buf.join_next(), if !v6_buf.is_empty() => {
                    let span = info_span!("QADv6");
                    let _guard = span.enter();
                    // khor patch 26: pending until one brings an address.
                    if qad_family_done(false, v6_buf.len()) {
                        ipv6_pending = false;
                    }
                    match val {
                        Some(Ok(Some(Ok(res)))) => {
                            match res {
                                Ok((r, conn)) => {
                                    ipv6_pending = !qad_family_done(true, v6_buf.len());
                                    debug!(?r, "probe report");
                                    let url = r.relay.clone();
                                    reports.push(ProbeReport::QadIpv6(r));
                                    if self.qad_conns.v6.is_none() {
                                        self.qad_conns.v6.replace((url, conn));
                                    } else {
                                        conn.conn.close(QUIC_ADDR_DISC_CLOSE_CODE, QUIC_ADDR_DISC_CLOSE_REASON);
                                    }
                                }
                                Err(err) => {
                                    debug!("probe failed: {err:#}");
                                }
                            }
                        }
                        Some(Err(err)) => {
                            if err.is_panic() {
                                panic!("probe panicked: {err:#}");
                            }
                            warn!("probe failed: {err:#}");
                        }
                        Some(Ok(None)) => {
                            debug!("probe canceled");
                        }
                        Some(Ok(Some(Err(time::Elapsed { .. })))) => {
                            debug!("probe timed out");
                        }
                        None => {
                            trace!("report canceled");
                        }
                    }
                }
                else => {
                    break;
                }
            }
        }

        // make sure to cancel all outstanding reports
        v4_buf.abort_all();
        v6_buf.abort_all();

        reports
    }

    /// Check if we have enough information to consider the current report "good enough".
    fn have_enough_reports(
        &self,
        state: &IfStateDetails,
        do_full: bool,
        num_relays: usize,
        report: &Report,
    ) -> bool {
        #[cfg_attr(wasm_browser, allow(unused_mut))]
        let mut num_ipv4 = 0;
        #[cfg_attr(wasm_browser, allow(unused_mut))]
        let mut num_ipv6 = 0;
        let mut num_https = 0;
        for (typ, _, _) in report.relay_latency.iter() {
            match typ {
                #[cfg(not(wasm_browser))]
                Probe::QadIpv4 => {
                    num_ipv4 += 1;
                }
                #[cfg(not(wasm_browser))]
                Probe::QadIpv6 => {
                    num_ipv6 += 1;
                }
                Probe::Https => {
                    num_https += 1;
                }
            }
        }

        if do_full {
            // khor patch 14: a full report takes the QAD shortcut below only
            // once every relay that serves no QAD has answered its HTTPS
            // probe (or its probes ended, which ends this loop). Upstream
            // stopped at two QAD answers, before such a relay was ever
            // measured, so it could never become home again: MacLab on
            // 2026-10-09, once its proxy let n0's QAD through, left a 5 ms
            // relay for one 1-4 s away for good.
            let unmeasured = self
                .relay_map
                .relays::<Vec<_>>()
                .iter()
                .filter(|r| r.quic.is_none())
                .any(|r| !report.relay_latency.iter().any(|(typ, url, _)| typ == Probe::Https && url == &r.url));
            if unmeasured {
                return num_https >= num_relays;
            }
            // Full report, require more probes
            match (state.have_v4, state.have_v6) {
                (true, true) => {
                    // Both IPv4 and IPv6 are expected to be available
                    if num_ipv4 >= 2 && num_ipv6 >= 1 || num_ipv6 >= 2 && num_ipv4 >= 1 {
                        return true;
                    }
                }
                (true, false) => {
                    // Just Ipv4 is expected
                    if num_ipv4 >= 2 {
                        return true;
                    }
                }
                (false, true) => {
                    // Just Ipv6 is expected
                    if num_ipv6 >= 2 {
                        return true;
                    }
                }
                (false, false) => {}
            }
            if num_https >= num_relays {
                // If we have at least one https probe per relay, we are happy
                return true;
            }
            false
        } else {
            // Incremental reports, here the requirements are reduced further
            match (state.have_v4, state.have_v6) {
                (true, true) => {
                    // Both IPv4 and IPv6 are expected to be available
                    if num_ipv4 >= 1 && num_ipv6 >= 1 {
                        return true;
                    }
                }
                (true, false) => {
                    // Just Ipv4 is expected
                    if num_ipv4 >= 1 {
                        return true;
                    }
                }
                (false, true) => {
                    // Just Ipv6 is expected
                    if num_ipv6 >= 1 {
                        return true;
                    }
                }
                (false, false) => {}
            }
            if num_https >= num_relays {
                // If we have at least one https probe per relay, we are happy
                return true;
            }
            false
        }
    }

    /// Adds `r` to the set of recent Reports and mutates `r.preferred_relay` to contain the best recent one.
    fn add_report_history_and_set_preferred_relay(&mut self, r: &mut Report, full: bool) {
        // khor patch 24: the home is this client's own memory. Upstream read
        // it off `reports.last`, which a full report clears before it runs,
        // so every full report chose afresh with no hysteresis at all.
        let prev_relay = self.home.as_ref().map(|(url, _)| url.clone());
        if let Some(ref last) = self.reports.last {
            // If we don't have new information, copy this from the last report
            if r.mapping_varies_by_dest_ipv4.is_none() {
                r.mapping_varies_by_dest_ipv4 = last.mapping_varies_by_dest_ipv4;
            }
            if r.mapping_varies_by_dest_ipv6.is_none() {
                r.mapping_varies_by_dest_ipv6 = last.mapping_varies_by_dest_ipv6;
            }
        }

        let now = Instant::now();
        const MAX_AGE: Duration = Duration::from_secs(5 * 60);

        // relay ID => its best recent latency in last MAX_AGE
        let mut best_recent = RelayLatencies::default();

        // chain the current report as we are still mutating it
        let prevs_iter = self
            .reports
            .prev
            .iter()
            .map(|(a, b)| -> (&Instant, &Report) { (a, b) });

        let mut to_remove = Vec::new();
        for (t, pr) in prevs_iter {
            if now.duration_since(*t) > MAX_AGE {
                to_remove.push(*t);
                continue;
            }
            best_recent.merge(&pr.relay_latency);
        }
        // merge in current run
        best_recent.merge(&r.relay_latency);

        for t in to_remove {
            self.reports.prev.remove(&t);
        }

        // khor patches 3, 5, 11, 23: a relay is reached when its HTTPS probe
        // answered. A QAD answer is UDP and a home is held over TCP: from
        // MacLab, TCP to the Beijing TLS relay is reset in-path
        // (audit/pk-3341-dump.txt) where UDP may pass.
        let https = |url: &RelayUrl| {
            r.relay_latency
                .iter()
                .any(|(probe, u, _)| probe == Probe::Https && u == url)
        };

        // khor patch 25: a successor whose connection dropped twice within
        // SUCCESSOR_DROPS_WITHIN is not taken again for SUCCESSOR_OFF, or
        // until the network changes (`get_report`). MacLab, 2026-10-20
        // 15:32: the TLS relay answered its HTTPS probe, and the home
        // connection to it was reset every 20 s.
        const SUCCESSOR_DROPS_WITHIN: Duration = Duration::from_secs(2 * 60);
        const SUCCESSOR_OFF: Duration = Duration::from_secs(60 * 60);
        if let Ok(mut drops) = self.home_drops.lock() {
            drops.retain(|(_, at)| now.duration_since(*at) <= SUCCESSOR_DROPS_WITHIN);
            let dropping = self
                .successors
                .iter()
                .any(|(_, new)| drops.iter().filter(|(url, _)| url == new).count() >= 2);
            if dropping {
                self.successors_off_until = Some(now + SUCCESSOR_OFF);
                drops.clear();
            }
        }
        let successors_off = self.successors_off_until.is_some_and(|until| now < until);
        let is_successor = |url: &RelayUrl| self.successors.iter().any(|(_, new)| new == url);

        // khor patch 23: the fallback tier is home only in a report where no
        // other relay answered its HTTPS probe (the owner, 2026-09-30: 「n0
        // 可以兜底」). Upstream ranked every relay by latency alone, and n0's
        // answer QAD — one round trip — where ours answer HTTPS alone.
        let ours_answered = r.relay_latency.iter().any(|(probe, url, _)| {
            probe == Probe::Https
                && !self.fallback.contains(url)
                && !(successors_off && is_successor(url))
        });
        let mut candidates: Vec<RelayUrl> = Vec::new();
        for (_, url, _) in r.relay_latency.iter() {
            if !(ours_answered && self.fallback.contains(url)) && !candidates.contains(url) {
                candidates.push(url.clone());
            }
        }
        if successors_off && candidates.iter().any(|u| !is_successor(u)) {
            candidates.retain(|u| !is_successor(u));
        }
        // khor patch 25: and among those, in a report where any answered
        // HTTPS, one that did or the home itself (its connection is the
        // proof). A report with no HTTPS answer at all (an incremental one,
        // or one cut short by the QAD shortcut) ranks them all, as upstream.
        if candidates.iter().any(|u| https(u)) {
            candidates.retain(|u| https(u) || prev_relay.as_ref() == Some(u));
        }

        // Then, pick which currently-alive relay server from the
        // current report has the best latency over the past MAX_AGE.
        r.preferred_relay = None;
        let mut best_any = Duration::default();
        for url in &candidates {
            if let Some(best) = best_recent.get(url)
                && (r.preferred_relay.is_none() || best < best_any)
            {
                best_any = best;
                r.preferred_relay.replace(url.clone());
            }
        }

        // If we're changing our preferred relay but the old one's still
        // accessible and the new one's not much better, just stick with
        // where we are.
        // khor patch 24: the old one is weighed as the new one is, by its
        // best in the last MAX_AGE (upstream took its last sample in this
        // report: one slow probe of the home against the challenger's best
        // moved hinton between aps1 and euc1 a dozen times a day), and a
        // home that answers is not moved by latency within `home_dwell`.
        if let Some(prev) = &prev_relay
            && r.preferred_relay.as_ref() != Some(prev)
            && candidates.contains(prev)
            && let Some(old) = best_recent.get(prev)
        {
            let dwelling = self
                .home
                .as_ref()
                .is_some_and(|(_, since)| now.duration_since(*since) < self.home_dwell);
            if dwelling || best_any > old / 3 * 2 {
                r.preferred_relay = Some(prev.clone());
            }
        }

        // khor patch 11: an incremental report that did not reach the home
        // keeps it. Such a report measures only the relay holding the QAD
        // connection; ranking that one alone moved MacLab's home from
        // Beijing to n0's aps1 25 s after every full report, all night
        // (audit/net-d.md §1.4). A report that tried every relay (`full`:
        // a full one, or an incremental one on the initial plan whose
        // probes all ended) may move the home as before, so a home that
        // died is left within two periodic reports (net-h, audit/net-d.md).
        // Not a home this report has ruled out: a benched successor, or the
        // fallback tier once ours answered.
        if !full
            && let Some(prev) = &prev_relay
            && !https(prev)
            && !(successors_off && is_successor(prev))
            && !(ours_answered && self.fallback.contains(prev))
        {
            r.preferred_relay = Some(prev.clone());
        }

        // khor patch 3: a relay the person chose wins whenever this report
        // reached it — reachability, not latency, is the only test. Once
        // home, any answer keeps it: the home connection is the TCP proof.
        if let Some(chosen) = &self.preferred_relay
            && (https(chosen)
                || (prev_relay.as_ref() == Some(chosen) && r.relay_latency.get(chosen).is_some()))
        {
            r.preferred_relay = Some(chosen.clone());
        }

        // khor patch 5: a home that has a successor this report reached
        // moves to the successor — the same box over TLS, say, measuring
        // no faster than the plain one it replaces. After patch 3, so a
        // person who chose the old one gets the new one too.
        if !successors_off
            && let Some(home) = &r.preferred_relay
            && let Some((_, new)) = self.successors.iter().find(|(old, _)| old == home)
            && https(new)
        {
            r.preferred_relay = Some(new.clone());
        }

        // A report that reached nothing names no home, as upstream's does
        // (iroh then keeps the relay it is on), and forgets none: the dwell
        // runs on once the relays answer again.
        if r.relay_latency.is_empty() {
            r.preferred_relay = None;
        } else if r.preferred_relay != prev_relay {
            self.home = r.preferred_relay.clone().map(|url| (url, now));
        }
        self.reports.prev.insert(now, r.clone());
        self.reports.last = Some(r.clone());
    }
}

/// khor patch 26 (vendor/iroh/KHOR.md): whether one family's QAD probes
/// have done their part — one brought an address, or none is left to.
/// Upstream counted any probe that ended, a failure included: the Beijing
/// relay's host is an IPv4 literal, so its v6 probe failed at once
/// (no AAAA) and cleared the wait before aps1's v6 answer was in, and
/// turing's reports had a public v6 address in one and not the next
/// (420 lines in a day).
#[cfg(not(wasm_browser))]
fn qad_family_done(got_addr: bool, left: usize) -> bool {
    got_addr || left == 0
}

#[cfg(not(wasm_browser))]
async fn run_probe_v4(
    relay: Arc<RelayConfig>,
    quic_client: QuicClient,
    dns_resolver: DnsResolver,
    shutdown_token: CancellationToken,
) -> n0_error::Result<(QadProbeReport, QadConn), QadProbeError> {
    use noq_proto::PathId;

    let relay_addr = reportgen::get_relay_addr_ipv4(&dns_resolver, &relay)
        .await
        .map_err(|source| e!(QadProbeError::GetRelayAddr { source }))?;

    trace!(?relay_addr, "resolved relay server address");
    let host = relay
        .url
        .host_str()
        .ok_or_else(|| e!(QadProbeError::MissingHost))?;
    let conn = quic_client
        .create_conn(relay_addr.into(), host)
        .await
        .map_err(|source| e!(QadProbeError::Quic { source }))?;

    let mut watcher = conn.observed_external_addr();

    // wait for an addr
    let addr = watcher
        .next()
        .await
        .ok_or_else(|| e!(QadProbeError::ReceiverDropped))?;
    let report = QadProbeReport {
        relay: relay.url.clone(),
        addr: SocketAddr::new(addr.ip().to_canonical(), addr.port()),
        latency: conn.rtt(PathId::ZERO).unwrap_or_default(),
    };

    // khor patch 26: the first answer is the observer's first value.
    // Upstream left it `None` until the address changed, so
    // `QadConns::current_v4`/`current_v6` saw nothing, and every report
    // dialled every relay's QAD again beside the connection it held.
    let observer = Watchable::new(Some(report.clone()));
    let endpoint = relay.url.clone();
    let handle = task::spawn(shutdown_token.run_until_cancelled_owned({
        let conn = conn.clone();
        let observer = observer.clone();
        async move {
            while let Some(val) = watcher.next().await {
                // if we've sent to an ipv4 address, but received an observed address
                // that is ivp6 then the address is an [IPv4-Mapped IPv6 Addresses](https://doc.rust-lang.org/beta/std/net/struct.Ipv6Addr.html#ipv4-mapped-ipv6-addresses)
                let val = SocketAddr::new(val.ip().to_canonical(), val.port());
                let latency = conn.rtt(PathId::ZERO).unwrap_or_default();
                observer
                    .set(Some(QadProbeReport {
                        relay: endpoint.clone(),
                        addr: val,
                        latency,
                    }))
                    .ok();
            }
        }
    }));
    let handle = AbortOnDropHandle::new(handle);

    Ok((
        report,
        QadConn {
            conn,
            observer,
            _handle: handle,
        },
    ))
}

#[cfg(not(wasm_browser))]
async fn run_probe_v6(
    relay: Arc<RelayConfig>,
    quic_client: QuicClient,
    dns_resolver: DnsResolver,
    shutdown_token: CancellationToken,
) -> n0_error::Result<(QadProbeReport, QadConn), QadProbeError> {
    use noq_proto::PathId;

    let relay_addr = reportgen::get_relay_addr_ipv6(&dns_resolver, &relay)
        .await
        .map_err(|source| e!(QadProbeError::GetRelayAddr { source }))?;

    trace!(?relay_addr, "resolved relay server address");
    let host = relay
        .url
        .host_str()
        .ok_or_else(|| e!(QadProbeError::MissingHost))?;
    // khor patch 22 (vendor/iroh/KHOR.md): an IPv6 literal's host is `[2001:db8::1]` in a
    // URL, and with the brackets it is no TLS server name: the handshake never started.
    let host = host.trim_start_matches('[').trim_end_matches(']');
    let conn = quic_client
        .create_conn(relay_addr.into(), host)
        .await
        .map_err(|source| e!(QadProbeError::Quic { source }))?;

    let mut watcher = conn.observed_external_addr();

    // wait for an addr
    let addr = watcher
        .next()
        .await
        .ok_or_else(|| e!(QadProbeError::ReceiverDropped))?;
    let report = QadProbeReport {
        relay: relay.url.clone(),
        addr: SocketAddr::new(addr.ip().to_canonical(), addr.port()),
        latency: conn.rtt(PathId::ZERO).unwrap_or_default(),
    };

    // khor patch 26: the first answer is the observer's first value.
    // Upstream left it `None` until the address changed, so
    // `QadConns::current_v4`/`current_v6` saw nothing, and every report
    // dialled every relay's QAD again beside the connection it held.
    let observer = Watchable::new(Some(report.clone()));
    let endpoint = relay.url.clone();
    let handle = task::spawn(shutdown_token.run_until_cancelled_owned({
        let observer = observer.clone();
        let conn = conn.clone();
        async move {
            while let Some(val) = watcher.next().await {
                // if we've sent to an ipv4 address, but received an observed address
                // that is ivp6 then the address is an [IPv4-Mapped IPv6 Addresses](https://doc.rust-lang.org/beta/std/net/struct.Ipv6Addr.html#ipv4-mapped-ipv6-addresses)
                let val = SocketAddr::new(val.ip().to_canonical(), val.port());
                let latency = conn.rtt(PathId::ZERO).unwrap_or_default();
                observer
                    .set(Some(QadProbeReport {
                        relay: endpoint.clone(),
                        addr: val,
                        latency,
                    }))
                    .ok();
            }
        }
    }));
    let handle = AbortOnDropHandle::new(handle);

    Ok((
        report,
        QadConn {
            conn,
            observer,
            _handle: handle,
        },
    ))
}

#[cfg(test)]
mod test_utils {
    //! Creates a relay server against which to perform tests

    use iroh_relay::{RelayConfig, RelayQuicConfig, server};

    pub(crate) async fn relay() -> (server::Server, RelayConfig) {
        let server = server::Server::spawn(server::testing::server_config())
            .await
            .expect("should serve relay");
        let quic = Some(RelayQuicConfig::new(
            server.quic_addr().expect("server should run quic").port(),
        ));
        let endpoint_desc =
            RelayConfig::new(server.https_url().expect("should work as relay"), quic);

        (server, endpoint_desc)
    }

    /// Create a [`crate::RelayMap`] of the given size.
    ///
    /// This function uses [`relay`]. Note that the returned map uses internal order that will
    /// often _not_ match the order of the servers.
    pub(crate) async fn relay_map(relays: usize) -> (Vec<server::Server>, crate::RelayMap) {
        let mut servers = Vec::with_capacity(relays);
        let mut endpoints = Vec::with_capacity(relays);
        for _ in 0..relays {
            let (relay_server, endpoint) = relay().await;
            servers.push(relay_server);
            endpoints.push(endpoint);
        }
        (servers, crate::RelayMap::from_iter(endpoints))
    }
}

#[cfg(all(test, with_crypto_provider))]
mod tests {
    use std::net::{Ipv4Addr, SocketAddr};

    use iroh_base::RelayUrl;
    use iroh_dns::dns::DnsResolver;
    use iroh_relay::tls::{CaTlsConfig, default_provider};
    use n0_error::{Result, StdResultExt};
    use n0_tracing_test::traced_test;
    use tokio_util::sync::CancellationToken;

    use super::*;
    use crate::net_report::probes::Probe;

    #[tokio::test(flavor = "multi_thread")]
    #[traced_test]
    async fn test_basic() -> Result<()> {
        let (server, relay) = test_utils::relay().await;
        let client_config = iroh_relay::tls::make_dangerous_client_config();
        let ep = noq::Endpoint::client(SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 0)).anyerr()?;
        let quic_addr_disc = QuicConfig {
            ep: ep.clone(),
            client_config,
            ipv4: true,
            ipv6: true,
        };
        let relay_map = RelayMap::from(relay);

        let resolver = DnsResolver::new();
        let tls_config = CaTlsConfig::insecure_skip_verify()
            .client_config(default_provider())
            .expect("infallible");
        let opts = Options::new(tls_config).quic_config(Some(quic_addr_disc.clone()));
        let mut client = Client::new(
            resolver.clone(),
            relay_map.clone(),
            opts.clone(),
            Default::default(),
        );
        let if_state = IfStateDetails::fake();

        // Note that the ProbePlan will change with each iteration.
        for i in 0..5 {
            let cancel = CancellationToken::new();
            println!("--round {i}");
            let r = client
                .get_report(if_state.clone(), false, cancel.child_token())
                .await;

            assert!(r.has_udp(), "want UDP");
            dbg!(&r);
            assert!(
                !r.relay_latency.is_empty(),
                "expected at least 1 key in RelayLatency; got none",
            );
            assert!(
                r.relay_latency.iter().next().is_some(),
                "expected key 1 in RelayLatency; got {:?}",
                r.relay_latency
            );
            assert!(r.global_v4.is_some(), "expected globalV4 set");
            assert!(r.preferred_relay.is_some(),);
            cancel.cancel();
        }

        drop(client);
        ep.wait_idle().await;
        server.shutdown().await?;

        Ok(())
    }

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn test_add_report_history_set_preferred_relay() -> Result {
        fn relay_url(i: u16) -> RelayUrl {
            format!("http://{i}.com").parse().unwrap()
        }

        // report returns a *Report from (relay host, Duration)+ pairs.
        fn report(a: impl IntoIterator<Item = (&'static str, u64)>) -> Option<Report> {
            let mut report = Report::default();
            for (s, d) in a {
                assert!(s.starts_with('d'), "invalid relay server key");
                let id: u16 = s[1..].parse().unwrap();
                report.relay_latency.update_relay(
                    relay_url(id),
                    Duration::from_secs(d),
                    Probe::QadIpv4,
                );
            }

            Some(report)
        }
        struct Step {
            /// Delay in seconds
            after: u64,
            r: Option<Report>,
        }
        struct Test {
            name: &'static str,
            steps: Vec<Step>,
            /// want PreferredRelay on final step
            want_relay: Option<RelayUrl>,
            // wanted len(c.prev)
            want_prev_len: usize,
        }

        let tests = [
            Test {
                name: "first_reading",
                steps: vec![Step {
                    after: 0,
                    r: report([("d1", 2), ("d2", 3)]),
                }],
                want_prev_len: 1,
                want_relay: Some(relay_url(1)),
            },
            Test {
                name: "with_two",
                steps: vec![
                    Step {
                        after: 0,
                        r: report([("d1", 2), ("d2", 3)]),
                    },
                    Step {
                        after: 1,
                        r: report([("d1", 4), ("d2", 3)]),
                    },
                ],
                want_prev_len: 2,
                want_relay: Some(relay_url(1)), // t0's d1 of 2 is still best
            },
            Test {
                name: "but_now_d1_gone",
                steps: vec![
                    Step {
                        after: 0,
                        r: report([("d1", 2), ("d2", 3)]),
                    },
                    Step {
                        after: 1,
                        r: report([("d1", 4), ("d2", 3)]),
                    },
                    Step {
                        after: 2,
                        r: report([("d2", 3)]),
                    },
                ],
                want_prev_len: 3,
                want_relay: Some(relay_url(2)), // only option
            },
            Test {
                name: "d1_is_back",
                steps: vec![
                    Step {
                        after: 0,
                        r: report([("d1", 2), ("d2", 3)]),
                    },
                    Step {
                        after: 1,
                        r: report([("d1", 4), ("d2", 3)]),
                    },
                    Step {
                        after: 2,
                        r: report([("d2", 3)]),
                    },
                    Step {
                        after: 3,
                        r: report([("d1", 4), ("d2", 3)]),
                    }, // same as 2 seconds ago
                ],
                want_prev_len: 4,
                want_relay: Some(relay_url(1)), // t0's d1 of 2 is still best
            },
            Test {
                name: "things_clean_up",
                steps: vec![
                    Step {
                        after: 0,
                        r: report([("d1", 1), ("d2", 2)]),
                    },
                    Step {
                        after: 1,
                        r: report([("d1", 1), ("d2", 2)]),
                    },
                    Step {
                        after: 2,
                        r: report([("d1", 1), ("d2", 2)]),
                    },
                    Step {
                        after: 3,
                        r: report([("d1", 1), ("d2", 2)]),
                    },
                    Step {
                        after: 10 * 60,
                        r: report([("d3", 3)]),
                    },
                ],
                want_prev_len: 1, // t=[0123]s all gone. (too old, older than 10 min)
                want_relay: Some(relay_url(3)), // only option
            },
            Test {
                name: "preferred_relay_hysteresis_no_switch",
                steps: vec![
                    Step {
                        after: 0,
                        r: report([("d1", 4), ("d2", 5)]),
                    },
                    Step {
                        after: 1,
                        r: report([("d1", 4), ("d2", 3)]),
                    },
                ],
                want_prev_len: 2,
                want_relay: Some(relay_url(1)), // 2 didn't get fast enough
            },
            Test {
                name: "preferred_relay_hysteresis_do_switch",
                steps: vec![
                    Step {
                        after: 0,
                        r: report([("d1", 4), ("d2", 5)]),
                    },
                    Step {
                        after: 1,
                        r: report([("d1", 4), ("d2", 1)]),
                    },
                ],
                want_prev_len: 2,
                want_relay: Some(relay_url(2)), // 2 got fast enough
            },
        ];
        let resolver = DnsResolver::new();
        let tls_config = CaTlsConfig::insecure_skip_verify()
            .client_config(default_provider())
            .expect("infallible");
        for mut tt in tests {
            println!("test: {}", tt.name);
            let relay_map = RelayMap::empty();
            let opts = Options::new(tls_config.clone());
            let mut client = Client::new(resolver.clone(), relay_map, opts, Default::default());
            for s in &mut tt.steps {
                // trigger the timer
                tokio::time::advance(Duration::from_secs(s.after)).await;
                client.add_report_history_and_set_preferred_relay(s.r.as_mut().unwrap(), true);
            }
            let last_report = tt.steps.last().unwrap().r.clone().unwrap();
            let got = client.reports.prev.len();
            let want = tt.want_prev_len;
            assert_eq!(got, want, "prev length");
            let got = &last_report.preferred_relay;
            let want = &tt.want_relay;
            assert_eq!(got, want, "preferred_relay");
        }

        Ok(())
    }

    // khor patches 23-25 (vendor/iroh/KHOR.md): which relay is home.

    fn khor_url(name: &str) -> RelayUrl {
        format!("http://{name}.com").parse().unwrap()
    }

    fn khor_client(set: impl FnOnce(&mut NetReportConfig)) -> Client {
        let mut config = NetReportConfig::default();
        set(&mut config);
        let tls_config = CaTlsConfig::insecure_skip_verify()
            .client_config(default_provider())
            .expect("infallible");
        let opts = Options::new(tls_config).net_report_config(config);
        Client::new(
            DnsResolver::new(),
            RelayMap::empty(),
            opts,
            Default::default(),
        )
    }

    /// One report of `(relay, probe, ms)` answers and the home it picks.
    /// `full` is a report that tried every relay; like `get_report`, it
    /// starts with no last report.
    fn khor_step(c: &mut Client, full: bool, answers: &[(&str, Probe, u64)]) -> Option<String> {
        if full {
            c.reports.last = None;
        }
        let mut r = Report::default();
        for (name, probe, ms) in answers {
            r.relay_latency
                .update_relay(khor_url(name), Duration::from_millis(*ms), *probe);
        }
        c.add_report_history_and_set_preferred_relay(&mut r, full);
        r.preferred_relay
            .map(|u| u.host_str().unwrap().trim_end_matches(".com").to_owned())
    }

    const MIN: Duration = Duration::from_secs(60);

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn khor_the_fallback_tier_is_home_only_when_no_other_relay_answered() {
        use Probe::*;
        let mut c = khor_client(|c| {
            c.fallback = vec![khor_url("aps1")];
            c.home_dwell = 10 * MIN;
        });
        // n0's aps1 measures far nearer over QAD than our own over HTTPS.
        let both = [
            ("bj", Https, 300),
            ("aps1", QadIpv4, 20),
            ("aps1", Https, 80),
        ];
        assert_eq!(
            khor_step(&mut c, true, &both).as_deref(),
            Some("bj"),
            "our own answered"
        );
        tokio::time::advance(5 * MIN).await;
        let n0_only = [("aps1", QadIpv4, 20), ("aps1", Https, 80)];
        assert_eq!(
            khor_step(&mut c, true, &n0_only).as_deref(),
            Some("aps1"),
            "nothing else answered"
        );
        tokio::time::advance(MIN).await;
        // Back the moment ours answers, dwell or not: the tier is not latency.
        assert_eq!(
            khor_step(&mut c, true, &both).as_deref(),
            Some("bj"),
            "ours answers again"
        );
        // Ours reached by UDP alone is not ours answering.
        let mut c = khor_client(|c| c.fallback = vec![khor_url("aps1")]);
        let qad_only = [
            ("bj", QadIpv4, 300),
            ("aps1", QadIpv4, 20),
            ("aps1", Https, 80),
        ];
        assert_eq!(khor_step(&mut c, true, &qad_only).as_deref(), Some("aps1"));
    }

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn khor_the_home_is_weighed_by_its_best_recent_latency_too() {
        use Probe::*;
        let mut c = khor_client(|_| {});
        let first = [("d1", Https, 400), ("d2", Https, 700)];
        assert_eq!(khor_step(&mut c, true, &first).as_deref(), Some("d1"));
        tokio::time::advance(Duration::from_secs(25)).await;
        // One slow sample of the home against the challenger's best:
        // upstream weighed the home by this sample alone (350 < 900 * 2/3).
        let spike = [("d1", Https, 900), ("d2", Https, 350)];
        assert_eq!(
            khor_step(&mut c, false, &spike).as_deref(),
            Some("d1"),
            "350 is not a third under d1's 400"
        );
    }

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn khor_a_home_that_answers_is_kept_for_the_dwell() {
        use Probe::*;
        let mut c = khor_client(|c| c.home_dwell = 10 * MIN);
        assert_eq!(
            khor_step(&mut c, true, &[("d1", Https, 500), ("d2", Https, 900)]).as_deref(),
            Some("d1")
        );
        let faster = [("d1", Https, 500), ("d2", Https, 10)];
        for _ in 0..3 {
            tokio::time::advance(3 * MIN).await;
            assert_eq!(
                khor_step(&mut c, true, &faster).as_deref(),
                Some("d1"),
                "inside the dwell"
            );
        }
        tokio::time::advance(2 * MIN).await;
        assert_eq!(
            khor_step(&mut c, true, &faster).as_deref(),
            Some("d2"),
            "past it"
        );
        // A home that stops answering is left at once.
        tokio::time::advance(MIN).await;
        assert_eq!(
            khor_step(&mut c, true, &[("d1", Https, 500)]).as_deref(),
            Some("d1")
        );
    }

    /// hinton, 2026-10-20: its own relays are v4 and it reaches only n0's,
    /// over CERNET's v6; it changed home 12 times that day. Relay round
    /// trips from its journal (改走中继), in the order measured: aps1
    /// 177-1216 ms, euc1 428-5152 ms, one over 3 s a probe that timed out.
    /// Each sample holds for 150 s (aps1) or 75 s (euc1), so the two
    /// cross. Two hours of reports every 25 s, a full one every 5 min,
    /// each measuring both (an incremental report re-probed every relay's
    /// QAD: `QadConn::observer`, patch 26). Upstream: 8 changes.
    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn khor_hinton_changes_home_rarely_between_two_far_relays() {
        use Probe::*;
        const APS1: [u64; 19] = [
            177, 314, 487, 463, 479, 488, 593, 310, 42101, 352, 462, 265, 394, 1036, 382, 1216,
            599, 219, 374,
        ];
        const EUC1: [u64; 33] = [
            783, 890, 567, 442, 584, 638, 628, 552, 864, 796, 798, 428, 771, 772, 749, 529, 523,
            574, 1230, 611, 538, 506, 515, 560, 847, 509, 772, 543, 2610, 2013, 5152, 2100, 3054,
        ];
        let mut c = khor_client(|c| {
            c.fallback = vec![khor_url("aps1"), khor_url("euc1")];
            c.home_dwell = 10 * MIN;
        });
        let mut homes = Vec::new();
        for i in 0..(2 * 60 * 60 / 25) {
            let full = i % 12 == 0;
            let mut answers = Vec::new();
            for (name, ms) in [
                ("aps1", APS1[(i / 6) % APS1.len()]),
                ("euc1", EUC1[(i / 3) % EUC1.len()]),
            ] {
                if ms <= 3000 {
                    answers.push((name, QadIpv6, ms));
                    if full {
                        answers.push((name, Https, ms * 2));
                    }
                }
            }
            let home = khor_step(&mut c, full, &answers);
            if homes.last() != Some(&home) {
                homes.push(home);
            }
            tokio::time::advance(Duration::from_secs(25)).await;
        }
        let changes = homes.len() - 1;
        println!("hinton, two hours: {changes} changes: {homes:?}");
        // What is left is aps1 missing a full report (the 42 s sample):
        // away, and back once the dwell is over.
        assert!(
            changes <= 3,
            "{changes} changes of home in two hours: {homes:?}"
        );
    }

    /// MacLab, 2026-10-20 15:32: home moved to the TLS successor, whose
    /// connection was reset every 20 s; back to plain at 15:35, to TLS
    /// again at 15:37.
    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn khor_a_successor_that_keeps_dropping_is_left_for_an_hour() {
        use Probe::*;
        let mut c = khor_client(|c| c.successors = vec![(khor_url("plain"), khor_url("pk"))]);
        let both = [
            ("plain", QadIpv4, 5),
            ("plain", Https, 20),
            ("pk", Https, 25),
        ];
        assert_eq!(khor_step(&mut c, true, &both).as_deref(), Some("pk"));
        for _ in 0..2 {
            tokio::time::advance(Duration::from_secs(20)).await;
            c.home_drops
                .lock()
                .unwrap()
                .push((khor_url("pk"), Instant::now()));
        }
        tokio::time::advance(Duration::from_secs(5)).await;
        // The next report, incremental, hears plain over QAD alone.
        assert_eq!(
            khor_step(&mut c, false, &[("plain", QadIpv4, 5)]).as_deref(),
            Some("plain"),
            "two drops in 40 s"
        );
        tokio::time::advance(Duration::from_secs(25)).await;
        assert_eq!(khor_step(&mut c, true, &both).as_deref(), Some("plain"));
        for _ in 0..11 {
            tokio::time::advance(5 * MIN).await;
            assert_eq!(
                khor_step(&mut c, true, &both).as_deref(),
                Some("plain"),
                "for an hour"
            );
        }
        tokio::time::advance(5 * MIN).await;
        assert_eq!(
            khor_step(&mut c, true, &both).as_deref(),
            Some("pk"),
            "then tried again"
        );
        // One drop is not a pattern.
        c.home_drops
            .lock()
            .unwrap()
            .push((khor_url("pk"), Instant::now()));
        tokio::time::advance(Duration::from_secs(25)).await;
        assert_eq!(khor_step(&mut c, true, &both).as_deref(), Some("pk"));
    }

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn khor_reached_is_an_https_answer() {
        use Probe::*;
        // Patch 3: a preferred relay that answered QAD alone is not home.
        let mut c = khor_client(|c| c.preferred_relay = Some(khor_url("mine")));
        let udp = [("mine", QadIpv4, 5), ("other", Https, 50)];
        assert_eq!(khor_step(&mut c, true, &udp).as_deref(), Some("other"));
        tokio::time::advance(MIN).await;
        let tcp = [
            ("mine", QadIpv4, 5),
            ("mine", Https, 900),
            ("other", Https, 50),
        ];
        assert_eq!(khor_step(&mut c, true, &tcp).as_deref(), Some("mine"));
        // Home already, it is kept by any answer.
        tokio::time::advance(MIN).await;
        assert_eq!(khor_step(&mut c, true, &udp).as_deref(), Some("mine"));
        // Patch 11: an incremental report that heard the home over QAD
        // alone keeps it, as one that did not hear it at all.
        let mut c = khor_client(|_| {});
        assert_eq!(
            khor_step(&mut c, true, &[("d1", Https, 100), ("d2", Https, 900)]).as_deref(),
            Some("d1")
        );
        tokio::time::advance(Duration::from_secs(25)).await;
        let qad = [("d1", QadIpv4, 900), ("d2", QadIpv4, 10)];
        assert_eq!(khor_step(&mut c, false, &qad).as_deref(), Some("d1"));
    }

    #[test]
    fn khor_a_failed_qad_probe_does_not_end_the_wait_for_an_address() {
        assert!(
            !qad_family_done(false, 2),
            "Beijing's v6 failed, aps1's still out"
        );
        assert!(qad_family_done(true, 2), "one brought an address");
        assert!(qad_family_done(false, 0), "none left to bring one");
    }

    /// Patch 26: the QAD connection a report opened answers the next one;
    /// upstream's observer stayed empty and every report dialled again.
    #[tokio::test(flavor = "multi_thread")]
    async fn khor_the_qad_connection_is_reused() -> Result<()> {
        let (server, relay) = test_utils::relay().await;
        let client_config = iroh_relay::tls::make_dangerous_client_config();
        let ep = noq::Endpoint::client(SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 0)).anyerr()?;
        let quic = QuicConfig {
            ep: ep.clone(),
            client_config,
            ipv4: true,
            ipv6: false,
        };
        let tls_config = CaTlsConfig::insecure_skip_verify()
            .client_config(default_provider())
            .expect("infallible");
        let opts = Options::new(tls_config).quic_config(Some(quic));
        let mut client = Client::new(
            DnsResolver::new(),
            RelayMap::from(relay),
            opts,
            Default::default(),
        );
        let if_state = IfStateDetails {
            have_v4: true,
            have_v6: false,
        };
        let first = client
            .get_report(if_state.clone(), false, CancellationToken::new())
            .await;
        assert!(
            first.global_v4.is_some(),
            "the first report has an address: {first:?}"
        );
        let held = client.qad_conns.current_v4();
        assert!(held.is_some(), "the connection it opened has one too");
        let second = client
            .get_report(if_state, false, CancellationToken::new())
            .await;
        assert_eq!(second.global_v4, first.global_v4);
        drop(client);
        ep.wait_idle().await;
        server.shutdown().await?;
        Ok(())
    }
}
