//! `khor doctor`: one checklist, one row per thing that can be wrong
//! (docs/MORE.md 自检).
//!
//! # Three rules this module is built out of
//!
//! **Nothing here is a second probe.** Every row asks the question some
//! existing code already asks — `hops()` for the roads, `hooks_report()`
//! for the hooks, `code::probe()` for the editor, `files::launcher()`
//! for the opener, `vitals::sample()` for the disk, `find_program()` for
//! an agent's command. A doctor that re-derived any of those would be a
//! second opinion about the machine, and the day the two disagree the
//! one on this page is the one nobody can act on.
//!
//! **未量 is not ok.** A check that timed out, or that this build cannot
//! ask at all, answers [`Health::Unmeasured`] — a fourth state, never
//! folded into ok. The whole value of a checklist is that a green row
//! means somebody looked (docs/SESSION.md 认不出就不落词, applied to a
//! reading rather than a word).
//!
//! **One row cannot hold up another.** Each check runs under its own
//! [`CHECK_TIMEOUT`]; a relay that swallows packets costs three seconds
//! and one row, not the page. They are run concurrently for the same
//! reason — a serial walk's worst case is the sum of every timeout, and
//! that is a page nobody waits for.
//!
//! # The sentence is written here, and that is a departure
//!
//! Elsewhere in khor the library answers in facts and the screen picks
//! the words (`khor_core::Hop` against `hop_word`). A check answers with
//! a sentence, because what a row says is not one fact with one shape —
//! 「钥匙别人也读得了」 and 「端口 41000 没人听」 have nothing in common
//! to model. The words still come from the catalog; what moves down here
//! is only the choosing between them. The one thing this costs is that
//! byte formatting happens on this side ([`bytes`]), which the CLI's own
//! `bytes()` calls painting — noted where it is used.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use khor_catalog::cli;

use crate::{Node, find_program};

/// How long one check may take before it answers 未量.
///
/// **Three seconds, and it is a per-check budget rather than a page
/// budget.** The thing this is protecting against is one unreachable
/// relay or one wedged resident, and both of those are a single row.
pub const CHECK_TIMEOUT: Duration = Duration::from_secs(3);

/// Which way a check came out.
///
/// Four, and the fourth is the point: [`Health::Unmeasured`] is「nobody
/// looked」, which reads nothing like「looked and it is fine」 and must
/// not be painted as it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Health {
    Ok,
    Warn,
    Fail,
    Unmeasured,
}

impl Health {
    /// The stable key a face keys off. Serde already spells these; this
    /// is for the CLI, which prints a word rather than JSON.
    pub fn key(self) -> &'static str {
        match self {
            Health::Ok => "ok",
            Health::Warn => "warn",
            Health::Fail => "fail",
            Health::Unmeasured => "unmeasured",
        }
    }

    /// The word a person reads.
    pub fn word(self) -> &'static str {
        match self {
            Health::Ok => cli::doctor_ok(),
            Health::Warn => cli::doctor_warn(),
            Health::Fail => cli::doctor_fail(),
            Health::Unmeasured => cli::doctor_unmeasured(),
        }
    }
}

/// One row of the checklist.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Check {
    /// The stable key — what `khor doctor --fix <key>` takes, and what a
    /// face reaches for. Never translated.
    pub key: String,
    /// Which of [`GROUPS`] this check is filed under, for the page that
    /// folds the healthy ones away. **On the row rather than looked up
    /// by the face**, the way `ConfigRow::section` is: a page that
    /// derived it would be a second table of the same fact.
    pub group: String,
    pub state: Health,
    /// One sentence for a person, already in their language.
    pub said: String,
    /// The key of a remedy khor can run, when there is one. `None` is
    /// the ordinary case: most of what a doctor finds is not khor's to
    /// mend.
    pub fix: Option<String>,
}

/// Every check's key, in the order the page lists them.
///
/// **A list rather than an enum**, because the one thing that walks it
/// is `--fix`, and the one thing that builds it is [`doctor`] — an enum
/// would buy exhaustiveness over a set nothing matches exhaustively.
/// It exists at all so a face and the fix door cannot disagree about
/// what the keys are.
pub const KEYS: &[&str] = &[
    "version", "identity", "store", "serve", "web", "relays", "peers", "roster", "roads", "hooks",
    "agents", "vscode", "disk", "launcher", "terminal",
];

/// The three groups the healthy checks are folded under (docs/MORE.md
/// 第二轮 自检页: 「分三组:本机、网络、工具」).
///
/// **A fact beside the key table, not a layout decision in TS** — the
/// same reasoning `config::SECTIONS` is written with. Which group a
/// check belongs to is a statement about what the check is, and a copy
/// of it in the faces would be two tables of one fact: ones that
/// compile, run, and put 中继 under 本机 the day somebody adds a check.
///
/// **This list is the whole set, and that is all a face reads it for.**
/// It is what the gate walks in both directions; the page draws its
/// headings in the order each group's first check falls in [`KEYS`],
/// so it needs no order from here and cannot fall out of step with one.
/// (`config::SECTIONS` carries an order because its rows are *drawn* in
/// it; these rows are gathered, not walked.)
pub const GROUPS: &[&str] = &["machine", "network", "tools"];

/// Which of [`GROUPS`] a check belongs to.
///
/// **An empty string for a key this does not know**, rather than a
/// fourth bucket or a guess: an unknown check with a plausible group
/// would be filed somewhere a person then trusts. It cannot happen
/// while [`KEYS`] and this match are read together, and the gate below
/// walks them against each other — which is exactly what the empty
/// string is there to let it catch.
pub fn group(key: &str) -> &'static str {
    match key {
        // 本机: what this machine is and what it is made of. `disk` is
        // here rather than under 工具 because a full store is this
        // machine's problem before it is anybody's tool.
        "version" | "identity" | "store" | "disk" => "machine",
        // What this machine's terminals noted about themselves (item 33):
        // a record of this app, not of the network or a tool beside it.
        "terminal" => "machine",
        // 网络: the ones about reaching, or being reached by, another
        // machine.
        "serve" | "web" | "relays" | "peers" | "roster" | "roads" => "network",
        // 工具: the things khor installs *beside* itself — none of them
        // is khor failing, and all of them are khor's to mend.
        "hooks" | "agents" | "vscode" | "launcher" => "tools",
        _ => "",
    }
}

/// The word a person reads for one of [`GROUPS`].
///
/// A match here for the same reason [`name`] is one: these keys are
/// code's, not data's.
pub fn group_name(group: &str) -> &str {
    match group {
        "machine" => cli::doctor_group_machine(),
        "network" => cli::doctor_group_network(),
        "tools" => cli::doctor_group_tools(),
        other => other,
    }
}

/// The word a person reads for one check's key.
///
/// **A match here rather than a `word()` in the catalog**, because the
/// generator only builds those for the sections whose keys are data (a
/// state, a hop, a face option). These keys are code's, so an unknown
/// one echoing would be a row named after a variable; it cannot happen
/// while [`KEYS`] and this match are read together, and the gate below
/// walks them against each other.
pub fn name(key: &str) -> &str {
    match key {
        "version" => cli::doctor_name_version(),
        "identity" => cli::doctor_name_identity(),
        "store" => cli::doctor_name_store(),
        "serve" => cli::doctor_name_serve(),
        "web" => cli::doctor_name_web(),
        "relays" => cli::doctor_name_relays(),
        "peers" => cli::doctor_name_peers(),
        "roster" => cli::doctor_name_roster(),
        "roads" => cli::doctor_name_roads(),
        "hooks" => cli::doctor_name_hooks(),
        "agents" => cli::doctor_name_agents(),
        "vscode" => cli::doctor_name_vscode(),
        "disk" => cli::doctor_name_disk(),
        "launcher" => cli::doctor_name_launcher(),
        "terminal" => cli::doctor_name_terminal(),
        other => other,
    }
}

/// Whether a check means anything on a machine that can (`hosts`) or
/// cannot host a session (docs/IOS.md 第二步).
///
/// **A phone has a checklist of its own, and it is this list minus what
/// is not there**: no `khor` CLI beside the app (version), no web face
/// (web), no claude (hooks), no agents it could start (agents), no VS
/// Code (vscode), nothing that opens a file (launcher). Each of those
/// would answer a warning that is true and useless — 「没有 CLI」 on a
/// phone is not a thing to mend — and a checklist that warns about the
/// ordinary state of the machine teaches people to stop reading it
/// (`check_vscode`'s reason). Dropped rows, not green ones: a row that
/// said 「正常」 about a CLI that does not exist would be a lie.
pub fn applies(key: &str, hosts: bool) -> bool {
    hosts || !matches!(key, "version" | "web" | "hooks" | "agents" | "vscode" | "launcher")
}

/// The whole checklist.
///
/// Concurrent, each row under its own timeout. The order of the answer
/// is [`KEYS`]' order and never the order they finished in: a list that
/// reshuffles itself between two runs is a list nobody can compare.
///
/// On a machine that cannot host, the rows [`applies`] says nothing
/// about are left out **and never run** — a check that is not polled
/// starts nothing, and the version check alone would spawn a CLI that is
/// not there — and the serve row carries no fix: the app *is* the
/// resident there, and the fix starts a `khor serve` that does not exist.
pub async fn doctor(node: &Node) -> Vec<Check> {
    let root = node.root().clone();
    let hosts = node.hosts();
    let run = |key: &str| applies(key, hosts);
    // Each arm is `(key, future)`. `tokio::join!` rather than a Vec of
    // boxed futures: the arms have different types and none of them is
    // built in a loop, so boxing would buy nothing but an allocation.
    let (
        version,
        identity,
        store,
        serve,
        web,
        relays,
        peers,
        roster,
        roads,
        hooks,
        agents,
        vscode,
        disk,
        launcher,
        terminal,
    ) = tokio::join!(
        timed_if(run("version"), "version", check_version()),
        timed_if(run("identity"), "identity", check_identity(root.clone())),
        timed_if(run("store"), "store", check_store(root.clone())),
        timed_if(run("serve"), "serve", check_serve(node)),
        timed_if(run("web"), "web", check_web(root.clone())),
        timed_if(run("relays"), "relays", check_relays(node.relays().to_vec())),
        timed_if(run("peers"), "peers", check_peers(node)),
        timed_if(run("roster"), "roster", check_roster(node)),
        timed_if(run("roads"), "roads", check_roads(node)),
        timed_if(run("hooks"), "hooks", check_hooks(node)),
        timed_if(run("agents"), "agents", check_agents(node)),
        timed_if(run("vscode"), "vscode", check_vscode()),
        timed_if(run("disk"), "disk", check_disk(root.clone())),
        timed_if(run("launcher"), "launcher", check_launcher()),
        timed_if(run("terminal"), "terminal", check_terminal(root.clone())),
    );
    [
        version, identity, store, serve, web, relays, peers, roster, roads, hooks, agents, vscode,
        disk, launcher, terminal,
    ]
    .into_iter()
    .flatten()
    .map(|mut c| {
        if !hosts {
            c.fix = None;
        }
        c
    })
    .collect()
}

/// [`timed`], or nothing at all when the check does not apply here —
/// the future is dropped unpolled, so whatever it would have started
/// never starts.
async fn timed_if(
    run: bool,
    key: &str,
    work: impl Future<Output = (Health, String, Option<String>)>,
) -> Option<Check> {
    if run { Some(timed(key, work).await) } else { None }
}

/// One check, under the budget. A check that overruns answers 未量 with
/// its own name in the sentence rather than being dropped — a row that
/// vanished would read as a clean bill.
async fn timed(key: &str, work: impl Future<Output = (Health, String, Option<String>)>) -> Check {
    match tokio::time::timeout(CHECK_TIMEOUT, work).await {
        Ok((state, said, fix)) => Check {
            key: key.to_owned(),
            group: group(key).to_owned(),
            state,
            said,
            fix,
        },
        Err(_) => Check {
            key: key.to_owned(),
            group: group(key).to_owned(),
            state: Health::Unmeasured,
            said: cli::doctor_timed_out(CHECK_TIMEOUT.as_secs()),
            fix: None,
        },
    }
}

// ── the checks ──────────────────────────────────────────────────────

/// Where the app's terminals note what they saw (item 33).
pub fn face_notes_path(root: &Path) -> PathBuf {
    root.join(".khor").join("face-notes.log")
}

/// The notes file's bound: past it, the older half goes.
const FACE_NOTES_CAP: usize = 64 * 1024;

/// **A line from a terminal, kept where `khor doctor` reads it** (item
/// 33). The owner's phone failed in a way no simulator showed (a drag did
/// nothing until the keyboard came up), and its stderr is a console
/// nobody has open. So the app's terminal notes what it set on attach
/// and what its first touch did, a few lines per terminal entered, never
/// per write; the newest are the `terminal` row's sentence.
pub fn note_face(root: &Path, line: &str) -> std::io::Result<()> {
    use std::io::Write;
    let path = face_notes_path(root);
    let stamp = jiff::Zoned::now().strftime("%m-%d %H:%M:%S").to_string();
    let line: String = line.chars().filter(|c| *c != '\n').take(400).collect();
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(&path)?;
    writeln!(f, "{stamp} {line}")?;
    if f.metadata()?.len() as usize > FACE_NOTES_CAP {
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        let keep = text.len() - FACE_NOTES_CAP / 2;
        let from = text[keep..].find('\n').map_or(text.len(), |i| keep + i + 1);
        std::fs::write(&path, &text[from..])?;
    }
    Ok(())
}

/// How many terminals the `terminal` row keeps (item 45).
const TERMINAL_ENTRIES: usize = 5;

/// Which terminal a face note is about: `term <id>: …`.
fn terminal_of(line: &str) -> Option<&str> {
    line.split_once(" term ")?.1.split_once(": ").map(|(id, _)| id)
}

/// The newest terminals entered, as the face noted them: for each, its
/// newest attach, the first second's fit (item 18 A: whether the
/// scroller ever had a range) and the first touch that followed the
/// attach (item 45: the row showed only the newest attach and the
/// newest touch, which were two different terminals, so the attach of
/// the one the owner had trouble with was gone). Newest first, a line
/// each.
async fn check_terminal(root: PathBuf) -> (Health, String, Option<String>) {
    let text = std::fs::read_to_string(face_notes_path(&root)).unwrap_or_default();
    let lines: Vec<&str> = text.lines().collect();
    let mut seen: Vec<&str> = Vec::new();
    let mut said: Vec<String> = Vec::new();
    for (at, line) in lines.iter().enumerate().rev() {
        if !line.contains(" attach ") {
            continue;
        }
        let Some(id) = terminal_of(line) else { continue };
        if seen.contains(&id) {
            continue;
        }
        seen.push(id);
        let mine = |l: &&&str| terminal_of(l) == Some(id);
        let after = |word: &str| lines[at + 1..].iter().find(|l| l.contains(word) && mine(l)).copied();
        // The fit is noted 1.5 s after the pane is made: usually after the
        // attach, before it when the open was slow.
        let fit = after(": fit at ").or_else(|| lines[..at].iter().rev().find(|l| l.contains(": fit at ") && mine(l)).copied());
        let touch = after(" first touch ");
        // A touch before the attach had landed (item 51) is noted at its
        // lift, which can come before the attach's own line: looked for
        // back to this terminal's previous attach.
        let early = lines[..at]
            .iter()
            .rev()
            .take_while(|l| !(l.contains(" attach ") && mine(l)))
            .find(|l| l.contains(" first touch before the attach") && mine(l))
            .copied();
        said.push(std::iter::once(*line).chain(fit).chain(early).chain(touch).collect::<Vec<_>>().join(" · "));
        if said.len() == TERMINAL_ENTRIES {
            break;
        }
    }
    if said.is_empty() {
        return (Health::Ok, cli::doctor_terminal_none().to_owned(), None);
    }
    (Health::Ok, cli::doctor_terminal_last(said.join("\n")), None)
}

/// Which khor this is, and whether the `khor` a terminal would run is
/// the same build.
///
/// **Two binaries is the ordinary shape on a desktop**: the app is
/// `khor-gui` and the CLI is `khor`, installed side by side, and an
/// upgrade that only took on one of them is invisible from either.
async fn check_version() -> (Health, String, Option<String>) {
    let mine = crate::VERSION.to_owned();
    let Some(exe) = khor_cli_binary() else {
        return (Health::Warn, cli::doctor_version_no_cli().to_owned(), None);
    };
    // `spawn_blocking` with `std::process`, not `tokio::process`: the
    // workspace's tokio is built without the `process` feature, and one
    // spawn on a page that already runs everything concurrently is not
    // worth growing every binary in the tree for.
    let out = tokio::task::spawn_blocking(move || {
        std::process::Command::new(&exe).arg("version").output()
    })
    .await;
    let Ok(Ok(out)) = out else {
        return (Health::Warn, cli::doctor_version_no_cli().to_owned(), None);
    };
    let theirs = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    if theirs.is_empty() {
        return (Health::Warn, cli::doctor_version_no_cli().to_owned(), None);
    }
    if theirs == mine {
        (Health::Ok, mine, None)
    } else {
        (
            Health::Warn,
            cli::doctor_version_split(&theirs, &mine),
            None,
        )
    }
}

/// The identity key: there, readable, and nobody else's to read.
async fn check_identity(root: PathBuf) -> (Health, String, Option<String>) {
    let p = identity_path(&root);
    let Ok(meta) = std::fs::metadata(&p) else {
        return (Health::Fail, cli::doctor_identity_gone(p.display()), None);
    };
    if std::fs::File::open(&p).is_err() {
        return (
            Health::Fail,
            cli::doctor_identity_unreadable(p.display()),
            Some("chmod".to_owned()),
        );
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = meta.permissions().mode() & 0o777;
        if mode != 0o600 {
            return (
                Health::Warn,
                cli::doctor_identity_loose(format_args!("{mode:o}")),
                Some("chmod".to_owned()),
            );
        }
    }
    #[cfg(not(unix))]
    let _ = meta;
    (Health::Ok, cli::doctor_identity_ok().to_owned(), None)
}

/// The store: it opens, it takes a write, and this is how much of the
/// disk it is holding.
async fn check_store(root: PathBuf) -> (Health, String, Option<String>) {
    let probe = root.join(".khor").join("doctor.probe");
    // Written and removed rather than merely stat'd: a read-only store
    // answers every question but the one that matters.
    if let Err(e) = std::fs::write(&probe, b"doctor") {
        return (Health::Fail, cli::doctor_store_shut(e), None);
    }
    let _ = std::fs::remove_file(&probe);
    (
        Health::Ok,
        cli::doctor_store_ok(bytes(store_bytes(&root))),
        None,
    )
}

/// Whether a resident `khor serve` is holding this key and answering.
///
/// Both halves, because they fail apart: a file with a live pid in it
/// and a socket that never answers is a wedged serve, and「没在跑」
/// would send somebody to start a second one.
async fn check_serve(node: &Node) -> (Health, String, Option<String>) {
    let Some(pid) = node.serve_pid() else {
        return (
            Health::Fail,
            cli::doctor_serve_none().to_owned(),
            Some("serve".to_owned()),
        );
    };
    // **Then the machine** (SHELL.md 第五轮 第 0 条): every question
    // here is about this store's serve, and on turing that one was
    // healthy while a second resident, serving a second store, sat on
    // its port. A store with no serve at all stays a `fail` above —
    // that is the worse news and the one with a fix.
    let default_root = Node::root_from_env_default();
    let seen = tokio::task::spawn_blocking(crate::serves::here)
        .await
        .unwrap_or_default();
    let home = std::env::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let residents = crate::serves::own(crate::serves::residents(&seen, &default_root), &home);
    if residents.len() > 1 {
        let list = residents
            .iter()
            .map(|r| cli::doctor_serve_one_of(r.pid, r.root.display()))
            .collect::<Vec<_>>()
            .join(cli::name_separator());
        return (
            Health::Warn,
            cli::doctor_serve_many(residents.len(), list),
            None,
        );
    }
    // The cheapest op the resident answers, and one this page wants
    // anyway (the peers row reads the same answer). An empty table is
    // not a refusal — a machine that has paired with nobody has no rows
    // to answer with.
    let answered = match node.links().await {
        Ok(links) => links.is_empty() || links.iter().any(|(_, l)| l.hop != khor_core::Hop::NoServe),
        Err(_) => false,
    };
    if answered {
        (Health::Ok, cli::doctor_serve_ok(pid), None)
    } else {
        (
            Health::Warn,
            cli::doctor_serve_deaf(pid),
            Some("serve".to_owned()),
        )
    }
}

/// The web face: something listening on its port, and a key on the door.
async fn check_web(root: PathBuf) -> (Health, String, Option<String>) {
    let port = crate::face::web_port();
    let has_key = crate::face::web_key_path(&root).is_file();
    let listening = tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .is_ok();
    match (listening, has_key) {
        (true, true) => (Health::Ok, cli::doctor_web_ok(port), None),
        (true, false) => (Health::Warn, cli::doctor_web_nokey(port), None),
        (false, _) => (Health::Warn, cli::doctor_web_quiet(port), None),
    }
}

/// Every relay this machine would bind with: reachable, and how far.
async fn check_relays(urls: Vec<String>) -> (Health, String, Option<String>) {
    if urls.is_empty() {
        return (Health::Warn, cli::doctor_relays_none().to_owned(), None);
    }
    // **All at once, each on its own clock** (2026-10-01, when the tier
    // became two relays): asked one after another, a slow relay ate the
    // row's whole three seconds and the row said 未量 about a relay that
    // had answered in 10 ms.
    let asked: Vec<_> = urls
        .iter()
        .map(|url| {
            let url = url.clone();
            tokio::spawn(async move { relay_rtt(&url).await })
        })
        .collect();
    let mut rtts = Vec::with_capacity(urls.len());
    for rtt in asked {
        rtts.push(rtt.await.ok().flatten());
    }
    let answered = |u: &str| urls.iter().zip(&rtts).any(|(v, r)| v == u && r.is_some());
    let pairs = khor_net::endpoint::successor_pairs();
    let mut said: Vec<String> = Vec::new();
    let mut reached = 0usize;
    for (url, rtt) in urls.iter().zip(&rtts) {
        match rtt {
            Some(ms) => {
                reached += 1;
                said.push(cli::doctor_relay_ms(url, *ms));
            }
            // **A successor not switched on yet is no fault** (round
            // NET-7): its predecessor answers, and that is where this
            // machine homes until it does.
            None => match pairs.iter().find(|(old, new)| new == url && answered(old)) {
                Some((old, _)) => {
                    reached += 1;
                    said.push(cli::doctor_relay_not_yet(url, old));
                }
                None => said.push(cli::doctor_relay_quiet(url)),
            },
        }
    }
    let state = if reached == urls.len() {
        Health::Ok
    } else if reached == 0 {
        Health::Fail
    } else {
        Health::Warn
    };
    (state, said.join(cli::name_separator()), None)
}

/// 连接诊断: per machine the resident has dialled, how long the last
/// dial took to get an answer and where the last few settled
/// (`khor_core::RoadTally`, docs/CONNECT.md 度量与验收).
///
/// **A reading, never a verdict**: a pair of machines that only ever
/// meets on a relay is one of the six places CONNECT.md says cannot be
/// made seamless, and nothing on this row can be mended — so every
/// reading is ok, and only the absence of one is 未量.
async fn check_roads(node: &Node) -> (Health, String, Option<String>) {
    let tallies = match node.road_tallies().await {
        Ok(t) => t,
        Err(crate::link::RoadsUnread::NoServe) => {
            return (
                Health::Unmeasured,
                cli::doctor_roads_no_serve().to_owned(),
                None,
            );
        }
        Err(crate::link::RoadsUnread::Unanswered) => {
            return (
                Health::Unmeasured,
                cli::doctor_roads_unanswered().to_owned(),
                None,
            );
        }
    };
    let devices = node.devices().unwrap_or_default();
    // In the device table's order, so two runs list machines the same
    // way; a tally for a machine that has since left is named by its id.
    let mut rows: Vec<(String, khor_core::RoadTally)> = Vec::new();
    for d in &devices {
        if let Some((_, t)) = tallies.iter().find(|(id, _)| *id == d.id) {
            rows.push((d.shown().to_owned(), *t));
        }
    }
    for (id, t) in &tallies {
        if !devices.iter().any(|d| d.id == *id) {
            rows.push((id.chars().take(8).collect(), *t));
        }
    }
    let said: Vec<String> = rows
        .iter()
        .filter_map(|(who, t)| {
            let ms = t.first_reply_ms?;
            let settled = t.direct + t.relay;
            Some(if settled == 0 {
                cli::doctor_road_fresh(who, ms)
            } else {
                cli::doctor_road(who, ms, t.direct, settled)
            })
        })
        .collect();
    if said.is_empty() {
        return (
            Health::Unmeasured,
            cli::doctor_roads_none().to_owned(),
            None,
        );
    }
    let mut said = said.join(cli::name_separator());
    // How the wires fared, after the per-machine readings: the counts
    // that trace a word on a screen back to what produced it.
    if let Ok(c) = node.wire_counts().await {
        said.push_str(cli::name_separator());
        said.push_str(&cli::doctor_wires(
            c.dialed,
            c.dial_failed,
            c.answers_late,
            c.reads_retried,
        ));
        said.push_str(cli::name_separator());
        said.push_str(&cli::doctor_wires_retired(
            c.retired_silent,
            c.retired_unanswered,
            c.retired_closed,
            c.retired_network,
            c.retired_new_ears,
        ));
        said.push_str(cli::name_separator());
        said.push_str(&cli::doctor_wires_reads(
            c.read_direct,
            c.read_relay,
            c.read_idle,
            c.network_changes,
            c.ops_late,
        ));
        // Whose the failures are: by name, so 「失败 18」 is somebody's.
        let devices = node.devices().unwrap_or_default();
        for (id, failed, unanswered) in &c.by_peer {
            let who = devices
                .iter()
                .find(|d| d.id == *id)
                .map(|d| d.shown().to_owned())
                .unwrap_or_else(|| id.chars().take(8).collect());
            said.push_str(cli::name_separator());
            said.push_str(&cli::doctor_wires_peer(who, failed, unanswered));
        }
    }
    (Health::Ok, said, None)
}

/// Every paired machine: which road, and which khor.
///
/// **The version comes off the readings, not off a question of its
/// own** (`khor_core::Vitals::version` — 「it rides with the readings
/// because it is the same kind of fact」). So a machine khor has not
/// heard from has no version here, and that is an answer rather than a
/// gap.
async fn check_peers(node: &Node) -> (Health, String, Option<String>) {
    let mine = crate::VERSION;
    // A resident that does not answer leaves the road unread on every
    // row — said in those words, never as a road (`Node::links`).
    let roads = node.links().await.ok();
    let Ok(devices) = node.devices() else {
        return (
            Health::Unmeasured,
            cli::doctor_peers_unread().to_owned(),
            None,
        );
    };
    let others: Vec<_> = devices
        .into_iter()
        .filter(|d| d.id != node.device_str())
        .collect();
    if others.is_empty() {
        return (Health::Ok, cli::doctor_peers_alone().to_owned(), None);
    }
    // 暂停 (docs/MORE.md): every peer is mute, and that is the state
    // somebody asked for — walking the rows would paint a whole page of
    // 警告 for a machine doing exactly what it was told. **Ok, not
    // Warn**, for the reason `Health::Unmeasured` exists at all: a
    // checklist that cries about what it was asked to do teaches people
    // to stop reading it.
    if node.paused() {
        return (Health::Ok, cli::doctor_peers_paused().to_owned(), None);
    }
    let mut said = Vec::new();
    let mut worst = Health::Ok;
    for d in &others {
        let road = match &roads {
            Some(roads) => khor_catalog::hop::word(
                roads
                    .iter()
                    .find(|(id, _)| id == &d.id)
                    .map(|(_, l)| l.hop)
                    .unwrap_or(khor_core::Hop::Idle)
                    .key(),
            ),
            None => cli::doctor_road_unread(),
        };
        match node.vitals_of(&d.id).and_then(|(v, _)| v.version) {
            Some(v) if v == mine => said.push(cli::doctor_peer(d.shown(), road, &v)),
            Some(v) => {
                worst = Health::Warn;
                said.push(cli::doctor_peer_behind(d.shown(), road, &v));
            }
            None => {
                worst = Health::Warn;
                said.push(cli::doctor_peer_mute(d.shown(), road));
            }
        }
    }
    (worst, said.join(cli::name_separator()), None)
}

/// The roster this machine trusts by (docs/CONNECT.md 撤销要新鲜): which
/// version, how old — and **how stale**, which is the one reading on this
/// page that says 「the root holder has been away」. Also whether this
/// machine itself is on it, because a machine waiting for approval sees
/// every peer refuse it and nothing else on the page says why.
///
/// The words are `khor root show`'s own, so the two faces say one thing.
pub(crate) async fn check_roster(node: &Node) -> (Health, String, Option<String>) {
    let st = node.root_state();
    if st.root.is_none() {
        return (Health::Warn, cli::root_show_legacy().to_owned(), None);
    }
    match st.standing.as_str() {
        "revoked" => return (Health::Fail, cli::root_show_revoked().to_owned(), None),
        "pending" => return (Health::Warn, cli::root_show_pending().to_owned(), None),
        _ => {}
    }
    let (Some(seq), Some(age)) = (st.seq, st.age_days) else {
        return (Health::Warn, cli::doctor_roster_none().to_owned(), None);
    };
    match st.stale_days {
        Some(n) => (Health::Warn, cli::doctor_roster_stale(n, age), None),
        None => (Health::Ok, cli::root_show_roster(seq, age), None),
    }
}

/// Whether claude on this machine tells khor what it is doing.
async fn check_hooks(node: &Node) -> (Health, String, Option<String>) {
    use crate::adaptor::claude::Installed;
    let Ok(report) = node.hooks_report() else {
        return (
            Health::Unmeasured,
            cli::doctor_hooks_unread().to_owned(),
            None,
        );
    };
    let missing = report
        .events
        .iter()
        .filter(|(_, s)| *s != Installed::Here)
        .count();
    if missing == 0 {
        (Health::Ok, cli::doctor_hooks_ok().to_owned(), None)
    } else {
        (
            Health::Warn,
            cli::doctor_hooks_part(missing),
            Some("hooks".to_owned()),
        )
    }
}

/// Every registered agent's command, against the PATH khor would start
/// it with.
async fn check_agents(node: &Node) -> (Health, String, Option<String>) {
    let Ok(rows) = node.agents() else {
        return (
            Health::Unmeasured,
            cli::doctor_agents_unread().to_owned(),
            None,
        );
    };
    if rows.is_empty() {
        return (Health::Ok, cli::doctor_agents_none().to_owned(), None);
    }
    let path = Node::user_path();
    let missing: Vec<String> = rows
        .iter()
        .filter(|(_, spec)| find_program(&spec.command, path).is_none())
        .map(|(name, _)| name.clone())
        .collect();
    if missing.is_empty() {
        (Health::Ok, cli::doctor_agents_ok(rows.len()), None)
    } else {
        (
            Health::Warn,
            cli::doctor_agents_missing(missing.join(cli::name_separator())),
            None,
        )
    }
}

/// This machine's editor, through the probe the workspace already makes.
///
/// **Never a warning.** A machine without VS Code is an ordinary
/// machine, and a server that has not been fetched yet is fetched on
/// the first press — neither is a thing to mend, so the row states the
/// fact and stays green (docs/UX.md 装饰性说明归零: a checklist that
/// shouts at every ordinary machine teaches people to stop reading it).
async fn check_vscode() -> (Health, String, Option<String>) {
    let editor = tokio::task::spawn_blocking(crate::code::probe)
        .await
        .unwrap_or_default();
    if editor.version.is_empty() {
        (Health::Ok, cli::doctor_vscode_none().to_owned(), None)
    } else if editor.ready {
        (Health::Ok, cli::doctor_vscode_ready(&editor.version), None)
    } else {
        (Health::Ok, cli::doctor_vscode_fetch(&editor.version), None)
    }
}

/// How much room is left on the volume the store sits on.
async fn check_disk(root: PathBuf) -> (Health, String, Option<String>) {
    // The background sampler's reading (`vitals::local`), waited for if
    // this is the first ask — the disks are asked by one thread only.
    let v = tokio::task::spawn_blocking(move || {
        crate::vitals::local::reading_within(&root, std::time::Duration::from_secs(5))
            .map(|r| r.vitals)
            .unwrap_or_else(|| crate::vitals::sample(&root))
    })
    .await;
    let Ok(v) = v else {
        return (
            Health::Unmeasured,
            cli::doctor_disk_unknown().to_owned(),
            None,
        );
    };
    let Some(disk) = v.disk else {
        return (
            Health::Unmeasured,
            cli::doctor_disk_unknown().to_owned(),
            None,
        );
    };
    if disk.total == 0 {
        return (
            Health::Unmeasured,
            cli::doctor_disk_unknown().to_owned(),
            None,
        );
    }
    let left = disk.total.saturating_sub(disk.used);
    let pct = (left as f64 / disk.total as f64) * 100.0;
    let state = if pct < 5.0 {
        Health::Fail
    } else if pct < 10.0 {
        Health::Warn
    } else {
        Health::Ok
    };
    (
        state,
        cli::doctor_disk(bytes(disk.used), bytes(disk.total), bytes(left)),
        None,
    )
}

/// The opener this machine hands a path or a page to.
async fn check_launcher() -> (Health, String, Option<String>) {
    let Some(opener) = crate::files::launcher() else {
        return (Health::Warn, cli::doctor_launcher_none().to_owned(), None);
    };
    if find_program(&opener, Node::user_path()).is_some() {
        (Health::Ok, cli::doctor_launcher_ok(&opener), None)
    } else {
        (Health::Warn, cli::doctor_launcher_missing(&opener), None)
    }
}

// ── the fixes ───────────────────────────────────────────────────────

/// Runs one check's remedy and answers what it did.
///
/// **By key, and refused by name.** A key with no remedy is refused
/// rather than answered with a cheerful nothing — `--fix relays` doing
/// nothing quietly is the shape where somebody believes a thing was
/// mended.
pub fn fix(node: &Node, key: &str) -> Result<String, String> {
    // On a phone the two remedies that start something — a `khor serve`,
    // claude's hooks — have nothing to start (docs/IOS.md); the page
    // offers neither there, and asking anyway is refused by name.
    if !node.hosts() && matches!(key, "serve" | "hooks") {
        return Err(cli::doctor_no_fix(key));
    }
    match key {
        "identity" => {
            let p = identity_path(node.root());
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o600))
                    .map_err(|e| khor_catalog::msg::cant_write(p.display(), e))?;
                Ok(cli::doctor_fixed_chmod().to_owned())
            }
            #[cfg(not(unix))]
            {
                let _ = p;
                Err(cli::doctor_no_fix(key))
            }
        }
        "hooks" => {
            node.install_hooks()?;
            Ok(cli::doctor_fixed_hooks().to_owned())
        }
        "serve" => {
            let exe = khor_cli_binary().ok_or_else(|| cli::doctor_version_no_cli().to_owned())?;
            // Detached, stdio null: this starts a resident, and a
            // resident wired to this terminal dies with it — which is
            // the opposite of what 「起它」 means.
            std::process::Command::new(&exe)
                .arg("serve")
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .map_err(|e| khor_catalog::msg::cant_launch(exe.display(), e))?;
            Ok(cli::doctor_fixed_serve().to_owned())
        }
        other => Err(cli::doctor_no_fix(other)),
    }
}

// ── the small shared facts ──────────────────────────────────────────

/// Where this machine's identity key lives. One spelling, because
/// [`Node::open_as`] and this file must not drift about it.
pub fn identity_path(root: &Path) -> PathBuf {
    root.join(".khor").join("identity.key")
}

/// The `khor` a terminal on this machine would run.
///
/// Beside this binary first: on a desktop the app is `khor-gui` and the
/// CLI is its neighbour, and that pair is what an installer lays down.
/// PATH after, for a machine where they are not neighbours.
fn khor_cli_binary() -> Option<PathBuf> {
    if let Ok(me) = crate::self_exe()
        && let Some(dir) = me.parent()
    {
        let beside = dir.join("khor");
        if beside.is_file() {
            return Some(beside);
        }
    }
    find_program("khor", Node::user_path()).map(PathBuf::from)
}

/// How much of the disk the store is holding, in bytes.
///
/// Walks rather than asks: no platform gives a directory's size without
/// one, and a store is thousands of small files rather than a tree
/// anybody would call deep.
pub fn store_bytes(root: &Path) -> u64 {
    fn walk(dir: &Path, total: &mut u64) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for e in entries.flatten() {
            let Ok(meta) = e.metadata() else { continue };
            if meta.is_dir() {
                walk(&e.path(), total);
            } else {
                *total += meta.len();
            }
        }
    }
    let mut total = 0;
    walk(&root.join(".khor"), &mut total);
    total
}

/// Whether a relay answers, and how long it took.
///
/// An HTTP GET rather than a TCP connect, the same judgment
/// `khor_web::answers_at` makes about the face: a handshake that
/// completes and then never speaks is not reachability. Any status is an
/// answer — what is being asked is whether the host is there, not what
/// it thinks of the path.
///
/// **On the road khor's own relay connection takes**: a proxy from the
/// environment and nothing else — never the system proxy this crate's
/// reqwest reads for the quota page (2026-10-01; iroh's relay client and,
/// since vendor/iroh patch 2, its probes read the same four variables in
/// the same order). Through the system proxy this row measured the
/// proxy's detour, not the relay. Its clock is a little short of the
/// row's, so a silent relay is named as silent rather than timing the
/// whole row out.
/// **One round trip to a relay**, not its handshakes: one GET to warm the
/// connection (DNS, TCP, TLS), then a second on the same connection,
/// timed. `relay_rtt` below times the first, which is what a person
/// waits for on a cold start but reads as a round trip it is not — the
/// owner's Hong Kong relay read 2040 ms that way while its round trip
/// through his proxy was a fraction of it. Same proxy rule as iroh's own
/// relay connection (environment only). `None` when it did not answer
/// within three seconds.
pub(crate) async fn relay_round_trip(url: &str) -> Option<u64> {
    let mut builder = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(3));
    let from_env = ["HTTP_PROXY", "http_proxy", "HTTPS_PROXY", "https_proxy"]
        .into_iter()
        .find_map(|k| std::env::var(k).ok().filter(|v| !v.is_empty()));
    if let Some(proxy) = from_env.and_then(|v| reqwest::Proxy::all(v).ok()) {
        builder = builder.proxy(proxy);
    }
    let client = builder.build().ok()?;
    client.get(url).send().await.ok()?.bytes().await.ok()?;
    let started = Instant::now();
    client.get(url).send().await.ok()?.bytes().await.ok()?;
    u64::try_from(started.elapsed().as_millis()).ok()
}

/// **Whether this machine reaches any of `urls` at all** — what a
/// pairing dial that timed out asks, so its sentence can say which side
/// to check (UX-8): none answering is no network or no relay here; one
/// answering puts the silence on the machine that issued the ticket.
/// Asked of every relay at once, each within [`relay_rtt`]'s budget.
pub(crate) async fn any_relay_answers(urls: Vec<String>) -> bool {
    let asks: Vec<_> = urls
        .into_iter()
        .map(|url| tokio::spawn(async move { relay_rtt(&url).await.is_some() }))
        .collect();
    for ask in asks {
        if ask.await.unwrap_or(false) {
            return true;
        }
    }
    false
}

async fn relay_rtt(url: &str) -> Option<u128> {
    let mut builder = reqwest::Client::builder()
        .no_proxy()
        .timeout(CHECK_TIMEOUT.saturating_sub(Duration::from_millis(400)));
    let from_env = ["HTTP_PROXY", "http_proxy", "HTTPS_PROXY", "https_proxy"]
        .into_iter()
        .find_map(|k| std::env::var(k).ok().filter(|v| !v.is_empty()));
    if let Some(proxy) = from_env.and_then(|v| reqwest::Proxy::all(v).ok()) {
        builder = builder.proxy(proxy);
    }
    let client = builder.build().ok()?;
    let started = Instant::now();
    client.get(url).send().await.ok()?;
    Some(started.elapsed().as_millis())
}

/// Bytes as a person reads them: 1024-based, one decimal, no space.
///
/// ⚠️ **The CLI has its own copy and calls this painting** (`bytes()` in
/// `crates/cli`), on the argument that a library returning strings is
/// deciding what a screen it cannot see has room for. That argument
/// holds everywhere else and does not hold here: a check's `said` **is**
/// a sentence, written on this side by contract (the head of this file),
/// so the number in it has to be formatted on this side too. Nothing
/// compares the two outputs; if they ever drift, two screens print the
/// same size two ways, which is a blemish rather than a wrong answer.
fn bytes(n: u64) -> String {
    const UNITS: [&str; 5] = ["B", "K", "M", "G", "T"];
    let mut v = n as f64;
    let mut u = 0;
    while v >= 1024.0 && u + 1 < UNITS.len() {
        v /= 1024.0;
        u += 1;
    }
    if u == 0 {
        format!("{n}{}", UNITS[0])
    } else {
        format!("{v:.1}{}", UNITS[u])
    }
}

#[cfg(test)]
mod face_note_tests {
    use super::*;

    /// The terminal row says the newest terminals, each with its attach
    /// and its first touch, and the notes stay under their bound however
    /// many entries there were.
    #[test]
    fn the_terminal_row_reads_the_newest_entry_and_the_notes_stay_bounded() {
        let root = std::env::temp_dir().join(format!("khor-face-notes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join(".khor")).unwrap();
        let row = || {
            tokio::runtime::Builder::new_current_thread()
                .build()
                .unwrap()
                .block_on(check_terminal(root.clone()))
        };
        assert_eq!(row().1, cli::doctor_terminal_none());
        note_face(&root, "term a: attach 10B; alternate; mouse drag").unwrap();
        note_face(&root, "term a: first touch drag after 900 ms: wheel to the program, 6 wheels, 0 reports").unwrap();
        note_face(&root, "term b: attach 20B; normal; mouse none").unwrap();
        let (state, said, fix) = row();
        assert_eq!((state, fix), (Health::Ok, None));
        assert!(said.starts_with(&cli::doctor_terminal_last("")) && said.contains("term b: attach"), "{said}");
        assert!(said.lines().nth(1).is_some_and(|l| l.contains("term a: attach 10B") && l.contains("6 wheels")), "{said}");
        note_face(&root, "term b: first touch tap after 300 ms: keyboard up").unwrap();
        assert!(row().1.contains("term b: first touch tap"), "{}", row().1);
        // Per terminal (item 45): b's attach and touch on one line, a's on
        // the next; a touch after another terminal's attach is still its own.
        note_face(&root, "term a: attach 30B; normal; mouse none").unwrap();
        note_face(&root, "term b: first touch drag after 2000 ms: native, moved 100 px").unwrap();
        note_face(&root, "term a: fit at 9 ms (44x48), range > 0 at never ms, last range 0").unwrap();
        note_face(&root, "term a: first touch tap after 500 ms: keyboard up").unwrap();
        let said = row().1;
        let rows: Vec<&str> = said.lines().collect();
        assert_eq!(rows.len(), 2, "{said}");
        assert!(rows[0].contains("term a: attach 30B") && rows[0].contains("term a: fit at 9 ms") && rows[0].contains("term a: first touch tap after 500"), "{said}");
        assert!(rows[1].contains("term b: attach 20B") && rows[1].contains("term b: first touch tap after 300"), "{said}");
        assert!(!said.contains("term a: attach 10B") && !said.contains("drag after 2000"), "{said}");
        // A touch before the attach, noted before it (item 51), is its
        // terminal's too; one before an earlier attach is not.
        note_face(&root, "term d: first touch before the attach, 90 ms into the pane: drag").unwrap();
        note_face(&root, "term d: attach 1B").unwrap();
        note_face(&root, "term e: first touch before the attach, 10 ms into the pane: tap").unwrap();
        note_face(&root, "term e: attach 1B").unwrap();
        note_face(&root, "term e: attach 2B").unwrap();
        let said = row().1;
        assert!(said.lines().any(|l| l.contains("term d: attach 1B") && l.contains("term d: first touch before the attach, 90 ms")), "{said}");
        assert!(!said.contains("term e: first touch before"), "{said}");
        for i in 0..8 {
            note_face(&root, &format!("term t{i}: attach 1B")).unwrap();
        }
        assert_eq!(row().1.lines().count(), TERMINAL_ENTRIES, "{}", row().1);

        for i in 0..2000 {
            note_face(&root, &format!("term c: attach {i}; {}", "x".repeat(80))).unwrap();
        }
        let text = std::fs::read_to_string(face_notes_path(&root)).unwrap();
        assert!(text.len() <= FACE_NOTES_CAP, "{}", text.len());
        assert!(text.lines().last().unwrap().contains("attach 1999;"));
        assert!(text.lines().next().unwrap().contains("term c: attach"), "cut at a line's start");
        let _ = std::fs::remove_dir_all(&root);
    }
}
