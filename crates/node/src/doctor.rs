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
    "version", "identity", "store", "serve", "web", "relays", "peers", "hooks", "agents", "vscode",
    "disk", "launcher",
];

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
        "hooks" => cli::doctor_name_hooks(),
        "agents" => cli::doctor_name_agents(),
        "vscode" => cli::doctor_name_vscode(),
        "disk" => cli::doctor_name_disk(),
        "launcher" => cli::doctor_name_launcher(),
        other => other,
    }
}

/// The whole checklist.
///
/// Concurrent, each row under its own timeout. The order of the answer
/// is [`KEYS`]' order and never the order they finished in: a list that
/// reshuffles itself between two runs is a list nobody can compare.
pub async fn doctor(node: &Node) -> Vec<Check> {
    let root = node.root().clone();
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
        hooks,
        agents,
        vscode,
        disk,
        launcher,
    ) = tokio::join!(
        timed("version", check_version()),
        timed("identity", check_identity(root.clone())),
        timed("store", check_store(root.clone())),
        timed("serve", check_serve(node)),
        timed("web", check_web(root.clone())),
        timed("relays", check_relays(node.relays().to_vec())),
        timed("peers", check_peers(node)),
        timed("hooks", check_hooks(node)),
        timed("agents", check_agents(node)),
        timed("vscode", check_vscode()),
        timed("disk", check_disk(root.clone())),
        timed("launcher", check_launcher()),
    );
    vec![
        version, identity, store, serve, web, relays, peers, hooks, agents, vscode, disk, launcher,
    ]
}

/// One check, under the budget. A check that overruns answers 未量 with
/// its own name in the sentence rather than being dropped — a row that
/// vanished would read as a clean bill.
async fn timed(key: &str, work: impl Future<Output = (Health, String, Option<String>)>) -> Check {
    match tokio::time::timeout(CHECK_TIMEOUT, work).await {
        Ok((state, said, fix)) => Check {
            key: key.to_owned(),
            state,
            said,
            fix,
        },
        Err(_) => Check {
            key: key.to_owned(),
            state: Health::Unmeasured,
            said: cli::doctor_timed_out(CHECK_TIMEOUT.as_secs()),
            fix: None,
        },
    }
}

// ── the checks ──────────────────────────────────────────────────────

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
    // The cheapest op the resident answers, and one this page wants
    // anyway (the peers row reads the same answer).
    if node.hops().await.is_empty() {
        // An empty table is not a refusal — a machine that has paired
        // with nobody has no rows to answer with.
        return (Health::Ok, cli::doctor_serve_ok(pid), None);
    }
    let answered = node
        .hops()
        .await
        .iter()
        .any(|(_, h)| *h != khor_core::Hop::NoServe);
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
    let mut said: Vec<String> = Vec::new();
    let mut reached = 0usize;
    for url in &urls {
        match relay_rtt(url).await {
            Some(ms) => {
                reached += 1;
                said.push(cli::doctor_relay_ms(url, ms));
            }
            None => said.push(cli::doctor_relay_quiet(url)),
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

/// Every paired machine: which road, and which khor.
///
/// **The version comes off the readings, not off a question of its
/// own** (`khor_core::Vitals::version` — 「it rides with the readings
/// because it is the same kind of fact」). So a machine khor has not
/// heard from has no version here, and that is an answer rather than a
/// gap.
async fn check_peers(node: &Node) -> (Health, String, Option<String>) {
    let mine = crate::VERSION;
    let roads = node.hops().await;
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
    let mut said = Vec::new();
    let mut worst = Health::Ok;
    for d in &others {
        let hop = roads
            .iter()
            .find(|(id, _)| id == &d.id)
            .map(|(_, h)| *h)
            .unwrap_or(khor_core::Hop::Unknown);
        let road = khor_catalog::hop::word(hop.key());
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
    let v = tokio::task::spawn_blocking(move || crate::vitals::sample(&root)).await;
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
async fn relay_rtt(url: &str) -> Option<u128> {
    let client = reqwest::Client::builder()
        .timeout(CHECK_TIMEOUT)
        .build()
        .ok()?;
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
