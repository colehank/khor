//! The `khor` CLI: the library's first consumer. Every verb is one node
//! call — anything the GUI will do must be reachable from here first
//! (docs/KHOR.md: CLI equals GUI).
//!
//! Stable keys are the wire truth; every word a person reads comes from
//! the catalog at print time (docs/UX.md 文案).

use khor_catalog::cli::USAGE;
use khor_catalog::{avatar, category, cli, msg, state};
use khor_node::{
    list, AvatarStyle, FaceShape, MsgBody, Node, SessionId, Tokens, UsageDay, Variant, PRESETS,
};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Err(e) = dispatch(&args) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

fn node() -> Result<Node, String> {
    Node::open(Node::root_from_env())
}

fn rt() -> Result<tokio::runtime::Runtime, String> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(msg::runtime_wont_start)
}

/// One verb: the word a person types, and the code that word runs.
///
/// **A verb is declared here exactly once.** This used to be a `match` on
/// the raw string, with the usage text keeping its own list — two places
/// holding one fact, so they drifted, and drift in either direction is
/// invisible: a word `khor help` promises and nothing answers, or a verb
/// nothing documents, both read as a healthy CLI from every angle except
/// a person typing that word. A row here cannot be half-written — there
/// is no way to name a word without giving it code, or to write code
/// without giving it a word.
struct Verb {
    /// What gets typed.
    word: &'static str,
    /// Everything after the word.
    run: fn(&[String]) -> Result<(), String>,
}

/// Every verb khor answers, in the order the usage text lists them.
const VERBS: &[Verb] = &[
    Verb { word: "id", run: id },
    Verb { word: "face", run: face },
    Verb { word: "devices", run: devices },
    Verb { word: "ls", run: ls },
    Verb { word: "pull", run: pull },
    Verb { word: "borrow", run: borrow },
    Verb { word: "usage", run: usage },
    Verb { word: "sessions", run: sessions },
    Verb { word: "tell", run: tell },
    Verb { word: "send", run: send },
    Verb { word: "accept", run: accept },
    Verb { word: "log", run: log },
    Verb { word: "run", run: run },
    Verb { word: "open", run: open },
    Verb { word: "attach", run: attach },
    Verb { word: "takeover", run: takeover },
    Verb { word: "agents", run: agents },
    Verb { word: "accounts", run: accounts },
    Verb { word: "quota", run: quota },
    Verb { word: "state", run: state },
    Verb { word: "seen", run: seen },
    Verb { word: "rename", run: rename },
    Verb { word: "pin", run: pin },
    Verb { word: "unpin", run: unpin },
    Verb { word: "close", run: close },
    Verb { word: "serve", run: serve },
    Verb { word: "web", run: web },
    Verb { word: "quit", run: quit },
    Verb { word: "hooks", run: hooks },
    Verb { word: "invite", run: invite },
    Verb { word: "pair", run: pair },
    Verb { word: "adopt", run: adopt },
    Verb { word: "sync", run: sync },
    Verb { word: "mcp", run: mcp },
    Verb { word: "version", run: version },
    Verb { word: "forget", run: forget },
    Verb { word: "--version", run: version },
    Verb { word: "-V", run: version },
    Verb { word: "_host", run: host },
    Verb { word: "_ghost", run: ghost },
    Verb { word: "_cagent", run: cagent },
    Verb { word: "_codexagent", run: codexagent },
    Verb { word: "help", run: help },
    Verb { word: "--help", run: help },
    Verb { word: "-h", run: help },
];

/// The verbs deliberately absent from the usage text, and why each stays
/// out.
///
/// The gate below refuses a verb that is in neither place, so staying out
/// of the usage becomes a decision somebody writes down rather than an
/// omission that happens. **Delete a line here and that verb goes red** —
/// which is the point: a hidden verb with nothing on record is
/// indistinguishable from one somebody forgot to document.
///
/// It sits beside `VERBS` because that is where a person adding a verb is
/// looking, and it is compiled only for the gate because checking is all
/// it does — khor itself never asks whether a verb is documented.
#[cfg(test)]
const NOT_IN_USAGE: &[(&str, &str)] = &[
    ("_host", "internal: the host process `open` spawns, whose arguments are a calling convention"),
    ("_ghost", "internal: the GUI-session host `open --gui` spawns; same convention, ACP instead of a PTY"),
    ("_cagent", "internal: khor's own claude ACP shim; the agent a claude GUI session runs (`cagent` module head)"),
    ("_codexagent", "internal: khor's own codex ACP shim; the agent a codex GUI session runs (`codexagent` module head)"),
    ("help", "prints the usage text; a list that lists itself teaches nobody anything"),
    ("--help", "the spelling people try before reading anything"),
    ("-h", "the short spelling of the same"),
    ("--version", "the spelling people try before finding the verb"),
    ("-V", "the short spelling of the same"),
];

/// What `khor` alone means. Named so the gate can keep it answerable: a
/// fallback word no row claims turns the bare command into an unknown-verb
/// error, which is the one failure nobody would think to test by hand.
const NOTHING_TYPED: &str = "help";

fn dispatch(args: &[String]) -> Result<(), String> {
    let word = args.first().map(String::as_str).unwrap_or(NOTHING_TYPED);
    let rest = &args[args.len().min(1)..];
    match VERBS.iter().find(|v| v.word == word) {
        Some(v) => (v.run)(rest),
        None => Err(format!("{}\n{USAGE}", msg::unknown_verb(word))),
    }
}

fn id(_rest: &[String]) -> Result<(), String> {
    let n = node()?;
    println!("{}  {}", n.device_str(), n.name());
    Ok(())
}

/// This machine's face. It prints what the machine is wearing either way
/// — with no flags because "what am I" is the question somebody runs this
/// to ask, and after a change because a verb that says nothing leaves
/// 做了但没变化 and 失败 looking alike (docs/UX.md 状态呈现).
fn face(rest: &[String]) -> Result<(), String> {
    let mut colors: Option<Vec<String>> = None;
    let mut variant: Option<String> = None;
    let mut shape: Option<String> = None;
    let mut it = rest.iter();
    while let Some(flag) = it.next() {
        let mut value = || it.next().ok_or_else(|| USAGE.to_string()).cloned();
        match flag.as_str() {
            // A factory set by name. Resolved here rather than inside
            // `restyle`, so that the library has exactly one way to be
            // handed a palette: five colors.
            "--palette" => {
                let id = value()?;
                let p = khor_node::preset(&id).ok_or_else(|| {
                    let all: Vec<&str> = PRESETS.iter().map(|p| p.id).collect();
                    msg::no_such_palette(&id, all.join(cli::NAME_SEPARATOR))
                })?;
                colors = Some(p.colors.iter().map(|c| c.to_string()).collect());
            }
            "--colors" => {
                colors = Some(value()?.split(',').map(|c| c.trim().to_owned()).collect());
            }
            "--variant" => variant = Some(value()?),
            "--shape" => shape = Some(value()?),
            _ => return Err(USAGE.into()),
        }
    }
    let n = node()?;
    let style = match (&colors, &variant, &shape) {
        (None, None, None) => n.avatar_style(),
        _ => n.restyle(colors.as_deref(), variant.as_deref(), shape.as_deref())?,
    };
    print_face(&style);
    Ok(())
}

/// A far machine's directory. Directories wear a trailing slash and no
/// size — the two facts a browse runs on; the order is the node's
/// (directories first, names case-folded), printed as it came.
fn ls(rest: &[String]) -> Result<(), String> {
    let (machine, path) = match rest {
        [machine] => (machine, ""),
        [machine, path] => (machine, path.as_str()),
        _ => return Err(USAGE.into()),
    };
    let (_, entries, truncated) = rt()?.block_on(node()?.ls_of(machine, path))?;
    for e in &entries {
        if e.dir {
            println!("{}/", e.name);
        } else {
            println!("{}\t{}", e.name, e.size);
        }
    }
    if truncated {
        eprintln!("{}", cli::dir_truncated(khor_node::files::MOST_ENTRIES as u64));
    }
    Ok(())
}

/// Takes a file off a machine by path. The landing prints because the
/// default (the current directory) was chosen silently — a file that
/// arrived somewhere unsaid is a file someone will go looking for.
fn pull(rest: &[String]) -> Result<(), String> {
    let (machine, path, dir) = match rest {
        [machine, path] => (machine, path, std::env::current_dir().map_err(|e| e.to_string())?),
        [machine, path, dir] => (machine, path, std::path::PathBuf::from(dir)),
        _ => return Err(USAGE.into()),
    };
    let (moved, dest) = rt()?.block_on(node()?.pull_path(machine, path, &dir))?;
    println!("{}", cli::pulled_to(moved, dest.display()));
    Ok(())
}

/// Borrows a machine's network: the serve stands up a local HTTP CONNECT
/// proxy in front of a lease and prints where it listens. Point a browser
/// or HTTPS_PROXY there and it goes out through the far machine.
fn borrow(rest: &[String]) -> Result<(), String> {
    let [machine] = rest else {
        return Err(USAGE.into());
    };
    let (session, addr) = rt()?.block_on(node()?.borrow(machine))?;
    println!("{}", cli::borrowing(machine, addr, session));
    Ok(())
}

/// Shows a machine the door, network-wide.
///
/// One argument and no flag: this is the same shape as `close`, which
/// also acts on the word alone — typing the machine's name *is* the
/// confirmation, and a `--yes` would only teach people to type it.
fn forget(rest: &[String]) -> Result<(), String> {
    let [machine] = rest else {
        return Err(USAGE.into());
    };
    let name = node()?.forget_device(machine)?;
    println!("{}", msg::forgot(&name));
    Ok(())
}

/// Which khor this binary is.
///
/// **Answering about the machine you are standing at is the small half.**
/// The question that matters in a mesh whose machines install and upgrade
/// themselves is *which of them did not take the last upgrade*, and that
/// is answered by `khor devices` — every machine reports its version
/// beside its readings (`khor_core::Vitals::version`). This verb exists
/// because a person on a machine still has to be able to ask it, and
/// because `--version` is the first thing anybody types.
fn version(_rest: &[String]) -> Result<(), String> {
    println!("{}", env!("CARGO_PKG_VERSION"));
    Ok(())
}

fn devices(_rest: &[String]) -> Result<(), String> {
    let n = node()?;
    // Asked once for the whole list rather than per row: it is one
    // hand-off to the resident, and asking per device would make the
    // readings arrive at different instants — a table where two rows
    // describe two moments.
    let hops = rt()?.block_on(n.hops());
    let all = n.devices()?;
    // id → name, for the satellite lines: their road column names the
    // sponsor they speak through, and a person reads names.
    let names: std::collections::HashMap<&str, &str> =
        all.iter().map(|d| (d.id.as_str(), d.name.as_str())).collect();
    for d in &all {
        // **By id, not by name.** Two machines on one network can carry
        // the same hostname — two homes on one box always do — and the
        // name test then marks both rows 「(本机)」, which is the one
        // thing on this line a person reads to know where they are.
        // Seen live while adding the road reading (2026-08-21): a
        // second home on this Mac made every row claim to be home.
        let me = d.id == n.device_str();
        let here = if me { cli::THIS_MACHINE } else { "" };
        let pin = if d.pinned { cli::PINNED_MARK } else { "" };
        let hop = hops
            .iter()
            .find(|(id, _)| *id == d.id)
            .map(|(_, h)| *h)
            .unwrap_or(khor_node::Hop::Unknown);
        // A satellite has no road of its own — its sponsor speaks for
        // it (附属机器批), so the road column says who through instead
        // of a reading nothing could take.
        let road = match d.sponsor.as_deref() {
            Some(sp) => cli::via_sponsor(names.get(sp).copied().unwrap_or(sp)),
            None => hop_word(hop, me).to_owned(),
        };
        println!(
            "{}\t{}…{here}{pin}\t{}\t{}",
            d.name,
            &d.id[..16],
            road,
            vitals_line(&n, &d.id)
        );
    }
    Ok(())
}

/// The road word for one row — and **nothing at all for this machine**.
///
/// A machine does not reach itself over the network, so any of the five
/// readings would be answering a question nobody asked. Left blank
/// rather than given a sixth word: a word here would be one more thing
/// to read on every row, forever, to learn something already obvious
/// from the 「(本机)」 beside it.
fn hop_word(hop: khor_node::Hop, is_me: bool) -> &'static str {
    if is_me {
        return "";
    }
    khor_catalog::hop::word(hop.key())
}

/// One machine's readings on one line — the CLI half of the machine card,
/// here first because anything the app shows has to be reachable from a
/// terminal (docs/KHOR.md: CLI equals GUI).
fn vitals_line(n: &Node, device_id: &str) -> String {
    let Some((v, age)) = n.vitals_of(device_id) else {
        return cli::VITALS_NEVER.to_owned();
    };
    let mut parts = vec![
        cli::vitals_cpu(format_args!("{:.0}", v.cpu_pct), v.cores),
        cli::vitals_mem(bytes(v.mem.used), bytes(v.mem.total)),
        match v.disk {
            Some(d) => cli::vitals_disk(bytes(d.used), bytes(d.total)),
            None => cli::VITALS_DISK_UNKNOWN.to_owned(),
        },
    ];
    // **Nothing is said when there is no reading**, unlike the disk right
    // above — a machine with no GPU is an ordinary machine, and the word
    // for that would be printed by most desktops to report a non-event
    // (`khor_core::Gpu`). Same again one level in: video memory is its
    // own absence, because a unified-memory machine has none to report
    // rather than none left.
    if let Some(g) = v.gpu {
        parts.push(cli::vitals_gpu(format_args!("{:.0}", g.util_pct), g.cards));
        if let Some(m) = g.mem {
            parts.push(cli::vitals_vram(bytes(m.used), bytes(m.total)));
        }
    }
    // Which khor that machine runs. Silent when unknown, for the GPU's
    // reason one block up — and here the silence carries the answer the
    // question is usually asked for: a machine saying nothing about its
    // version is running one from before khor reported it, which is to
    // say it is behind.
    if let Some(ver) = v.version.as_deref() {
        parts.push(cli::vitals_version(ver));
    }
    // Said for every reading that was not taken just now — which is every
    // machine but this one, and this one only while it is answering.
    if age > 0 {
        parts.push(cli::vitals_taken(human_age(age)));
    }
    parts.join("  ")
}

/// Bytes as a person reads them: 1024-based, one decimal, no space.
///
/// **This is painting, not judgment**, which is why the app has its own
/// copy rather than the number arriving pre-formatted. The wire carries
/// bytes; nothing downstream depends on these two producing the same
/// characters, and a library that returned strings would be deciding what
/// a screen it cannot see has room for.
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

/// What each machine in the network has spent, most recent day first.
///
/// **Every machine, not just this one**, because that is the question
/// somebody with three machines is actually asking — and the answers for
/// the others come from the last time they were reached, wearing their
/// age. A machine khor has never got an answer out of says so rather than
/// showing nothing, which would read as a machine that spent nothing.
///
/// The window is a count of days rather than a date range: "the last
/// week" is what a person asks, and a range is a second thing to get
/// right on the way to it. **It is calendar days and not rows**: seven
/// days in which nothing was spent is an answer, and a version that
/// printed the seven most recent days *with* spending would answer a
/// month-old question while claiming to answer this week's.
fn usage(rest: &[String]) -> Result<(), String> {
    let days = match rest {
        [] => USAGE_DAYS,
        // Zero days is refused rather than answered with nothing: it is
        // not a question, and an empty listing would read as a machine
        // that has spent nothing.
        [flag, n] if flag == "--days" => match n.parse::<usize>() {
            Ok(n) if n > 0 => n,
            _ => return Err(USAGE.into()),
        },
        _ => return Err(USAGE.into()),
    };
    let n = node()?;
    for d in n.devices()? {
        let here = if d.name == n.name() { cli::THIS_MACHINE } else { "" };
        println!("{}{here}", d.name);
        let Some((usage, age)) = n.usage_of(&d.id) else {
            println!("\t{}", cli::USAGE_NEVER);
            continue;
        };
        // Newest first here, and oldest first on the wire. The order a
        // list is *read* in is this screen's business; the order it
        // arrives in is the library's one answer (`khor_node::usage`),
        // and a face reversing it is not a second sort — it is the same
        // sort, looked at from the other end.
        //
        // The dates are `YYYY-MM-DD`, so the window is a string
        // comparison. That is not a shortcut that happens to work: it is
        // the format being chosen so that the order a person reads and
        // the order a machine sorts are the same order.
        let from = khor_node::usage::window_start(days);
        let recent: Vec<&UsageDay> =
            usage.days.iter().rev().filter(|d| d.day >= from).collect();
        if recent.is_empty() {
            println!("\t{}", cli::USAGE_NONE);
        }
        for row in recent {
            println!("\t{}\t{}\t{}", row.day, row.category, tokens_line(&row.tokens));
        }
        if usage.unreadable > 0 {
            println!("\t{}", cli::usage_unreadable(usage.unreadable));
        }
        if age > 0 {
            println!("\t{}", cli::usage_taken(human_age(age)));
        }
    }
    Ok(())
}

/// How far back `khor usage` looks when nobody said. A week is the span
/// somebody has an opinion about; a month of rows is a table nobody
/// reads, and one day cannot show a trend.
const USAGE_DAYS: usize = 7;

/// One day's four numbers. **No total** — the reason is on
/// `khor_core::Tokens`, and it is that a sum here would be almost
/// entirely cache reads.
fn tokens_line(t: &Tokens) -> String {
    [
        cli::tokens_input(count(t.input)),
        cli::tokens_output(count(t.output)),
        cli::tokens_cached(count(t.cached_input)),
        cli::tokens_cache_write(count(t.cache_write)),
    ]
    .join("  ")
}

/// A count as a person reads it: 1000-based, because tokens are counted,
/// not stored. **Not the same function as `bytes` one screen up**, and
/// the difference is the base: 1024 is a property of memory, and a
/// thousand tokens is a thousand tokens.
fn count(n: u64) -> String {
    const UNITS: [&str; 4] = ["", "k", "M", "G"];
    let mut v = n as f64;
    let mut u = 0;
    while v >= 1000.0 && u + 1 < UNITS.len() {
        v /= 1000.0;
        u += 1;
    }
    if u == 0 {
        format!("{n}")
    } else {
        format!("{v:.1}{}", UNITS[u])
    }
}

fn sessions(rest: &[String]) -> Result<(), String> {
    let mut mode = list::Arrange::Recent;
    let mut all = false;
    let mut it = rest.iter();
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--all" => all = true,
            "--by" => {
                let key = it.next().ok_or_else(|| USAGE.to_string())?;
                mode = list::Arrange::from_key(key).ok_or_else(|| {
                    let keys: Vec<&str> = list::Arrange::ALL.iter().map(|a| a.key()).collect();
                    msg::not_a_way_to_arrange(key, keys.join(cli::NAME_SEPARATOR))
                })?;
            }
            _ => return Err(USAGE.into()),
        }
    }
    let n = node()?;
    // The heading is printed when the group changes, which is also why
    // the node returns rows already grouped: this loop owns no
    // comparison, it only notices the boundary.
    let mut current: Option<String> = None;
    // Old news stays off the default listing and becomes one count at
    // the end (退场批). Skipped here, never dropped by the node: the
    // judgment is the node's (`SessionView::stale`), the folding is
    // each face's, and `--all` is this face unfolding it.
    let mut folded: u64 = 0;
    for a in n.sessions_arranged(mode)? {
        if !all && a.view.stale {
            folded += 1;
            continue;
        }
        if !a.group.is_empty() && current.as_deref() != Some(a.group.as_str()) {
            println!("{}", cli::group_header(group_word(&a.group)));
            current = Some(a.group.clone());
        }
        let v = a.view;
        let s = &v.session;
        let src = match &v.source {
            // A fresh report reads like local truth; only age worth
            // knowing gets printed (docs/SESSION.md 离线).
            Some((name, age)) if *age >= 30_000 => {
                format!("\t{name} {}", cli::unreachable_for(human_age(*age)))
            }
            _ => String::new(),
        };
        // The order already says which rows are pinned; the mark says
        // *why* they lead, which "on top" alone cannot.
        let pin = if v.pinned { format!("\t{}", cli::PINNED_MARK) } else { String::new() };
        // The quiet note (词的保鲜期批): a live word nobody can vouch
        // for, silent past the threshold — said beside the word, which
        // stands untouched. The duration is the word's own age.
        let quiet = if v.quiet {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);
            format!("\t{}", cli::quiet_for(human_age(now.saturating_sub(s.state.at.0))))
        } else {
            String::new()
        };
        println!(
            "{}\t{}\t{}\t{}{pin}{quiet}{}",
            s.id.0,
            state::word(s.state.state.key()),
            cli::unread(s.unread),
            s.title,
            src
        );
    }
    // A hidden row must say it is hidden, or folding reads as deleting.
    if folded > 0 {
        println!("{}", cli::old_sessions_hidden(folded));
    }
    // A missing row is invisible; this is the one thing that makes "khor
    // cannot read this vendor any more" visible instead of looking like
    // a quiet machine.
    let behind = n.unreadable_sessions();
    if behind > 0 {
        println!("{}", cli::adaptor_behind(behind));
    }
    Ok(())
}

fn tell(rest: &[String]) -> Result<(), String> {
    let [machine, text @ ..] = rest else {
        return Err(USAGE.into());
    };
    if text.is_empty() {
        return Err(USAGE.into());
    }
    let id = node()?.tell(machine, &text.join(" "))?;
    println!("{id}");
    Ok(())
}

/// Sends a file, or says a thing — one word for "put this in that
/// machine's conversation" (用户裁决, 传输并入对话批的后续).
///
/// The rule, fixed here and in the usage text: a **single** argument
/// naming an existing regular file makes this a transfer; anything
/// else makes the whole tail one message, joined with spaces (`tell`'s
/// shape). Single on purpose — with more words along, a transfer would
/// have to drop them while a message swallows a file name whole, so
/// only one reading loses nothing. The escape hatch for a lone word
/// that happens to name a file is `tell`, which stays and is always
/// literal — the ambiguity has a one-word way out and never needs
/// quoting tricks here.
fn send(rest: &[String]) -> Result<(), String> {
    let [machine, tail @ ..] = rest else {
        return Err(USAGE.into());
    };
    if tail.is_empty() {
        return Err(USAGE.into());
    }
    let candidate = std::path::Path::new(&tail[0]);
    if tail.len() == 1 && candidate.is_file() {
        let n = node()?;
        // A satellite has no khor to receive an offer: the file goes
        // over the sponsor's own scp, whole, and the verb says what
        // moved (附属机器批; from any other machine this refuses and
        // names the sponsor to run it on).
        if let Some(host) = n.satellite_host_here(machine)? {
            let moved = n.send_to_satellite(&host, candidate)?;
            println!("{}", msg::sat_sent(machine, moved));
            return Ok(());
        }
        let id = n.send(machine, candidate)?;
        println!("{}", cli::summary_sent(&id.0));
        return Ok(());
    }
    let id = node()?.tell(machine, &tail.join(" "))?;
    println!("{id}");
    Ok(())
}

/// The landing prints for the same reason `pull`'s does: the payload's
/// path was chosen silently, and a file that arrived somewhere unsaid
/// is a file someone will go looking for.
fn accept(rest: &[String]) -> Result<(), String> {
    let [id] = rest else {
        return Err(USAGE.into());
    };
    let n = node()?;
    let sid = SessionId(id.clone());
    // The summary travels the CRDT, so `khor send` over there and `khor
    // accept` over here are a race, and accept now waits the sync axis
    // out rather than refusing (`Node::accept`). Announced before it
    // goes quiet: a verb that sits silent through a round gets killed
    // and re-run — the guessing this replaces.
    //
    // Said from here rather than reported from inside, and that is not a
    // preference: when a serve holds the key the wait runs **in the
    // serve process**, whose stderr is not the person's terminal, and
    // `ipc::Reply` is one frame with no room for progress. This end is
    // the only one that has a terminal at all.
    //
    // Only the machines with roads are counted: `sync_device` refuses to
    // dial one that reported none, so those cost nothing and putting
    // them in the total would print a worst case that cannot happen.
    if !n.transfer_is_here(&sid) {
        let asking = n
            .devices()?
            .iter()
            .filter(|d| d.id != n.device_str() && !d.addrs.is_empty())
            .count();
        eprintln!(
            "{}",
            cli::syncing_for_offer(asking, khor_node::link::dial_budget_seconds())
        );
    }
    let (moved, landed) = rt()?.block_on(n.accept(&sid))?;
    // Where the files are is the **receiving** machine's answer, not a
    // path worked out here. This verb routes when the row belongs to
    // another machine (`accept_with`), and the path this end would
    // compute is a real-looking one under this root that nothing was
    // ever written to. `transfer_landing` remains only as the fallback
    // for a resident serve too old to say (`proto::Response::Acted`) —
    // and that case is this machine's own transfer, where the two
    // answers agree.
    let landing: Vec<std::path::PathBuf> = if landed.is_empty() {
        n.transfer_landing(&sid).unwrap_or_default()
    } else {
        landed.iter().map(std::path::PathBuf::from).collect()
    };
    match landing.as_slice() {
        [one] => println!("{}", cli::pulled_to(moved, one.display())),
        [first, ..] => println!(
            "{}",
            cli::pulled_to(moved, first.parent().unwrap_or(first).display())
        ),
        _ => println!("{}", cli::pulled(moved)),
    }
    Ok(())
}

fn log(rest: &[String]) -> Result<(), String> {
    let [machine] = rest else {
        return Err(USAGE.into());
    };
    let log = node()?.log(machine)?;
    if log.broken > 0 {
        eprintln!("{}", cli::broken_blocks(log.broken));
    }
    for m in log.messages {
        println!("{}: {}", m.from.name, render(&m));
    }
    Ok(())
}

fn run(rest: &[String]) -> Result<(), String> {
    let mut tui = false;
    let mut title: Option<String> = None;
    let mut cmd: Vec<String> = Vec::new();
    let mut it = rest.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--tui" if cmd.is_empty() => tui = true,
            "--title" if cmd.is_empty() => {
                title = Some(it.next().ok_or_else(|| USAGE.to_string())?.clone());
            }
            "--" if cmd.is_empty() => {}
            _ => cmd.push(a.clone()),
        }
    }
    if cmd.is_empty() {
        return Err(USAGE.into());
    }
    let n = node()?;
    let kind = if tui { khor_node::kind::TUI } else { khor_node::kind::SHELL };
    let title = title.unwrap_or_else(|| {
        std::path::Path::new(&cmd[0])
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| cmd[0].clone())
    });
    let id = n.open_ephemeral(kind, &title)?;
    // The child owns stdout; the id goes to stderr so scripts can still
    // capture the command's own output cleanly.
    eprintln!("session: {}", id.0);
    let code = n.run_ephemeral(&id, &cmd)?;
    std::process::exit(code);
}

fn open(rest: &[String]) -> Result<(), String> {
    let mut tui = false;
    let mut gui = false;
    let mut detached = false;
    let mut title: Option<String> = None;
    let mut cmd: Vec<String> = Vec::new();
    let mut on: Option<String> = None;
    let mut agent: Option<String> = None;
    let mut resume: Option<String> = None;
    let mut it = rest.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--tui" if cmd.is_empty() => tui = true,
            "--gui" if cmd.is_empty() => gui = true,
            "-d" if cmd.is_empty() => detached = true,
            "--on" if cmd.is_empty() => {
                on = Some(it.next().ok_or_else(|| USAGE.to_string())?.clone());
            }
            // A registered ACP agent, by the name its owner gave it
            // (批⑥). Implies `--gui`: a registration says how to start
            // something that speaks the protocol, and there is no other
            // channel for it.
            "--agent" if cmd.is_empty() => {
                agent = Some(it.next().ok_or_else(|| USAGE.to_string())?.clone());
                gui = true;
            }
            // A conversation the agent already has, by the id it gave
            // (`khor agents sessions`). The agent loads it instead of
            // opening a fresh one, so the row that appears is the same
            // conversation rather than a new one beside it.
            "--resume" if cmd.is_empty() => {
                resume = Some(it.next().ok_or_else(|| USAGE.to_string())?.clone());
            }
            "--title" if cmd.is_empty() => {
                title = Some(it.next().ok_or_else(|| USAGE.to_string())?.clone());
            }
            "--" if cmd.is_empty() => {}
            _ => cmd.push(a.clone()),
        }
    }
    let n = node()?;
    // Whether the person named a command at all, read before the agent
    // resolution below fills `cmd` in: the far-agent road takes a name
    // and never a command.
    let named_cmd = !cmd.is_empty();
    // The registration decides the command and the row's 类; a name
    // nobody registered is refused **by name**, with the registry's own
    // sentence, rather than falling back to the shell — a typo that
    // opens a shell session is a session on the wrong thing.
    let vendor = match &agent {
        Some(name) => {
            // The refusal lives on `Node`, not here: the wizard has to
            // refuse the same way, and a judgment written in two faces
            // grows two (docs/KHOR.md — the faces are equivalent
            // because they are one call).
            let spec = n.agent_or_refuse(name)?;
            // Resolved and carrying the user's PATH — see
            // `Node::user_path`. A terminal has that PATH already, so
            // this line changes nothing here; it is the same call the
            // faces make, and one judgment is the point.
            cmd = vec![khor_node::Node::agent_launch(&spec)?];
            Some(name.clone())
        }
        None => None,
    };
    // The default shell is the **target's**, never this machine's: an
    // `--on` with no command crosses empty and the far side fills in
    // its own (the usage's promise — 不给命令就用那台自己的 shell).
    // Filled here, this machine's /bin/zsh went to machines that may
    // not have one — and to a satellite it read as a command the user
    // named, which ran bare and shouldered the tmux persistence aside
    // (measured: the satellite's live tmux probe never ran).
    if cmd.is_empty() && on.is_none() {
        cmd = vec![std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into())];
    }
    let title = title.unwrap_or_else(|| match (&agent, cmd.first()) {
        // A launch JSON has no file name worth reading; the name the
        // person gave the agent is the one they will recognise.
        (Some(name), _) => name.clone(),
        // Remote with the far side's own shell: the machine's name is
        // the only honest title this side knows.
        (None, None) => on.clone().unwrap_or_default(),
        (None, Some(first)) => std::path::Path::new(first)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| first.clone()),
    });
    if gui {
        // **On another machine, by name only.** `--gui --on turing`
        // used to fall through untouched and open a conversation on
        // *this* machine while saying nothing; then it was refused
        // outright. Now it goes over as the agent's *name*
        // (`Node::open_agent_on`, the wizard's own road), and that
        // machine resolves the name against its own registrations and
        // paths. A command spelled here would be this machine's paths
        // on somebody else's disk, so that pairing stays refused — out
        // loud, because a flag accepted and ignored is the worse of the
        // two.
        if let Some(machine) = on {
            if named_cmd {
                return Err(cli::GUI_NOT_ON_ANOTHER_MACHINE.into());
            }
            if resume.is_some() {
                return Err(cli::RESUME_NOT_ON_ANOTHER_MACHINE.into());
            }
            let name = agent.as_deref().unwrap_or(khor_node::adaptor::claude::VENDOR);
            let id = rt()?.block_on(n.open_agent_on(&machine, "", &title, "chat", name))?;
            println!("{}", id.0);
            return Ok(());
        }
        // **A resume needs to know whose conversation it is**, and the
        // only thing that knows is the registration — an id with no
        // agent beside it names nothing this machine can start.
        // Refused rather than guessed: the alternative is opening a
        // fresh conversation on some default and calling it the one
        // that was asked for.
        if resume.is_some() && vendor.is_none() {
            return Err(cli::RESUME_NEEDS_AGENT.into());
        }
        // No terminal exists to attach to; the id is the deliverable.
        let id = match (&vendor, &resume) {
            (Some(v), Some(sid)) => {
                // **Where that conversation lives, not where this
                // process stands.** `session/load` names a cwd and the
                // real adaptor refuses a mismatch (`khor_acp::Handle`),
                // so the address has to come from the row the agent
                // itself handed over. Until 2026-08-31 this passed no
                // cwd at all and resumed at whatever directory the
                // command was typed in — which worked in the one run
                // that happened to be typed in the right place.
                let seen = n
                    .agent_seen_row(v, sid)?
                    .ok_or_else(|| khor_catalog::msg::resume_unknown_session(sid, v))?;
                n.reopen_gui_as(std::path::Path::new(&seen.cwd), &title, &cmd, v, sid)?
            }
            // The name is the user's own answer to "whose session is
            // this" — being told is not the guess `Session::category`
            // forbids (`gui_host`'s vendor door).
            (Some(v), None) => n.open_gui_as(&title, &cmd, v)?,
            (None, _) => n.open_gui(&title, &cmd)?,
        };
        println!("{}", id.0);
        return Ok(());
    }
    let kind = if tui { khor_node::kind::TUI } else { khor_node::kind::SHELL };
    let size = tty_size().unwrap_or((80, 24));
    if let Some(machine) = on {
        // Opened over there, and **defaults resolved over there** — the
        // command was left empty on purpose when the user gave none, so
        // the far machine's own shell answers, not this one's.
        let far: Vec<String> = if rest.iter().any(|a| a == "--") || !cmd.is_empty() {
            cmd.clone()
        } else {
            Vec::new()
        };
        let id = rt()?.block_on(n.open_on(&machine, kind, &title, "", &far, size))?;
        println!("{}", id.0);
        return Ok(());
    }
    let id = n.open_persistent(kind, &title, &cmd, size)?;
    eprintln!("session: {}", id.0);
    if detached {
        println!("{}", id.0);
        return Ok(());
    }
    attach_to(&n, &id)
}

fn attach(rest: &[String]) -> Result<(), String> {
    let [sid] = rest else {
        return Err(USAGE.into());
    };
    let n = node()?;
    let id = SessionId(sid.clone());
    // **A session on another machine is reached, not hosted here.** The
    // row knows whose it is, so the fork is on the row rather than on a
    // flag: a person types the same verb for both, which is the whole
    // point of one list holding every machine's sessions.
    if let Some(machine) = n.far_machine(&id)? {
        let (addr, cookie) = rt()?.block_on(n.reach(&machine, &id))?;
        return attach_at(&n, &id, &addr, &cookie);
    }
    // The bridge, same door the app's terminal uses: a discovered tmux
    // session — or an agent sitting inside one — has no host until
    // someone attaches, and the CLI is someone (`Node::attach_tmux` has
    // the judgment, including refusing rows with no route).
    if !n.is_hosted(&id) {
        n.attach_tmux(&id)?;
    }
    attach_to(&n, &id)
}

/// Internal: the detached host process `open` spawns. Nobody types this,
/// which is why it is on record in `NOT_IN_USAGE` instead of in the
/// usage text.
/// Internal: the GUI-session host `open --gui` spawns (`NOT_IN_USAGE`).
fn ghost(rest: &[String]) -> Result<(), String> {
    let (head, cmd) = match rest.iter().position(|a| a == "--") {
        Some(i) => (&rest[..i], &rest[i + 1..]),
        None => return Err(USAGE.into()),
    };
    let [ready, title] = head else {
        return Err(USAGE.into());
    };
    khor_node::gui_host::gui_host_main(
        khor_node::Node::root_from_env(),
        std::path::PathBuf::from(ready),
        title.clone(),
        cmd.to_vec(),
    )
}

fn host(rest: &[String]) -> Result<(), String> {
    let (head, cmd) = match rest.iter().position(|a| a == "--") {
        Some(i) => (&rest[..i], &rest[i + 1..]),
        None => return Err(USAGE.into()),
    };
    let [sid, cols, rows] = head else {
        return Err(USAGE.into());
    };
    if cmd.is_empty() {
        return Err(USAGE.into());
    }
    let size = (
        cols.parse::<u16>().map_err(|_| USAGE.to_string())?,
        rows.parse::<u16>().map_err(|_| USAGE.to_string())?,
    );
    khor_node::host::host_main(Node::root_from_env(), SessionId(sid.clone()), size, cmd.to_vec())
}

/// 接管 (批C): typing the verb is the confirmation — the CLI's precedent
/// is `close`, which also acts on the word alone. The GUI is where a
/// click is cheap enough to deserve a second look.
fn takeover(rest: &[String]) -> Result<(), String> {
    let [sid] = rest else {
        return Err(USAGE.into());
    };
    let n = node()?;
    let id = SessionId(sid.clone());
    n.takeover(&id)?;
    println!("{}", id.0);
    Ok(())
}

fn cagent(_rest: &[String]) -> Result<(), String> {
    khor_node::cagent::cagent_main(Node::root_from_env())
}

fn codexagent(_rest: &[String]) -> Result<(), String> {
    khor_node::codexagent::codexagent_main(Node::root_from_env())
}

fn state(rest: &[String]) -> Result<(), String> {
    let n = node()?;
    match rest {
        [flag] if flag == "--hook" => {
            use std::io::Read;
            let mut payload = String::new();
            std::io::stdin()
                .read_to_string(&mut payload)
                .map_err(msg::hook_payload_unreadable)?;
            n.claude_hook(&payload)?;
            Ok(())
        }
        [word] => {
            let sid = std::env::var("KHOR_SESSION").map_err(|_| msg::WHICH_SESSION.to_string())?;
            let word = khor_node::State::try_from(word.clone())
                .map_err(|_| msg::not_a_state_word(word))?;
            n.report_state(&SessionId(sid), word)
        }
        [word, flag, sid] if flag == "--session" => {
            let word = khor_node::State::try_from(word.clone())
                .map_err(|_| msg::not_a_state_word(word))?;
            n.report_state(&SessionId(sid.clone()), word)
        }
        _ => Err(USAGE.into()),
    }
}

fn seen(rest: &[String]) -> Result<(), String> {
    let [id] = rest else {
        return Err(USAGE.into());
    };
    node()?.seen(&SessionId(id.clone()))
}

/// Names a session for the whole network, or takes the name back.
///
/// Everything after the id is the name, joined with spaces (`tell`'s
/// shape — a name is words, and quoting should not be homework). **No
/// words is the eraser, on purpose**: the override is withdrawn and the
/// row falls back to the title its kind derives, so unlike `tell` an
/// empty tail is an argument here, not a mistake. Same node call as the
/// GUI's double-click (docs/KHOR.md: CLI equals GUI).
fn rename(rest: &[String]) -> Result<(), String> {
    let [id, name @ ..] = rest else {
        return Err(USAGE.into());
    };
    node()?.rename_session(&SessionId(id.clone()), &name.join(" "))
}

/// Two verbs rather than one toggle: a toggle typed twice is a no-op you
/// cannot see, and scripts have no way to say "make sure this is pinned".
/// Both call the one node function the app's button calls.
fn pin(rest: &[String]) -> Result<(), String> {
    pinning(rest, true)
}

fn unpin(rest: &[String]) -> Result<(), String> {
    pinning(rest, false)
}

fn pinning(rest: &[String], on: bool) -> Result<(), String> {
    let n = node()?;
    match rest {
        [flag, machine] if flag == "-m" || flag == "--machine" => n.pin_device(machine, on),
        [flag, machine, path] if flag == "-d" || flag == "--dir" => {
            n.pin_dir(machine, path, on)
        }
        [id] => n.pin_session(&SessionId(id.clone()), on),
        _ => Err(USAGE.into()),
    }
}

fn close(rest: &[String]) -> Result<(), String> {
    let [id] = rest else {
        return Err(USAGE.into());
    };
    // Routed like `attach`: the row says which machine it lives on, and
    // the person types the same word either way.
    let n = node()?;
    rt()?.block_on(n.close_anywhere(&SessionId(id.clone())))
}

/// khor's verbs as an agent's tools (docs/AGENT.md). Speaks MCP on
/// stdin/stdout, so it is started **by** the agent, not by a person —
/// but it is a verb rather than a hidden `_word` because pointing your
/// own claude at khor is a thing to be able to do, and a door nobody
/// can find is a door that only khor may open.
fn mcp(_rest: &[String]) -> Result<(), String> {
    khor_node::mcp::serve_stdio(Node::root_from_env())
}

/// The verb is the keeper; the serve itself is this same binary re-run
/// with the inner mark set (`khor_node::keeper` has why — the short of
/// it: hinton's serve died by signal and said nothing for eleven hours,
/// and only a process standing outside can log a death like that and
/// start the next life).
fn serve(_rest: &[String]) -> Result<(), String> {
    if !khor_node::keeper::is_inner() {
        // The keeper's own pid, where `khor quit` and the installer
        // both look. serve-up writes the same number when it is the
        // parent; this write is for the starts nobody scripted (a GUI,
        // a hand, a foreground terminal).
        let dot = Node::root_from_env().join(".khor");
        let _ = std::fs::create_dir_all(&dot);
        let _ = std::fs::write(dot.join("serve.pid"), std::process::id().to_string());
        return khor_node::keeper::keep();
    }
    // **Paid here, where nobody is waiting.** Finding the user's PATH
    // means running their login shell (0.78s cold, and it runs their rc
    // files) — see `Node::user_path`, which exists because launchd
    // hands this process `/usr/bin:/bin:/usr/sbin:/sbin` and every
    // agent khor can start lives somewhere else. Doing it now rather
    // than on the first spawn keeps that second out of the middle of
    // somebody pressing 问一次.
    khor_node::Node::warm_user_path();
    let n = node()?;
    eprintln!("{}", cli::serve_banner(n.name()));
    start_web_face(&Node::root_from_env());
    rt()?.block_on(n.serve())
}

/// Brings up the browser face beside the resident, and **keeps serving
/// if it cannot come up**.
///
/// A busy port must not cost this machine its place in the mesh —
/// syncing, answering peers and hosting sessions are what a node is
/// for, and none of them needs a browser. So the failure is loud on
/// stderr and not fatal.
///
/// That leaves a state where the node is up and the face is not, which
/// is why `khor web` asks the address a real question before printing
/// it rather than assuming this succeeded (`khor_web::answers_at`).
fn start_web_face(root: &std::path::Path) {
    match khor_web::listen(root.to_path_buf(), khor_web::port()) {
        Ok(face) => eprintln!("{}", cli::web_banner(face.addr)),
        Err(e) => eprintln!("{e}"),
    }
}

/// The browser face's link — the same GUI the app shows, opened from a
/// phone or anybody's laptop on this network.
///
/// **It starts nothing.** The face belongs to the resident (`serve`),
/// because "open it and the mesh is there" has to be true at the moment
/// somebody reaches for their phone, and a face that waits for a person
/// to run a command is absent exactly then. So this verb hands out the
/// key and the address, and refuses to print a link when nothing is
/// listening rather than printing one that fails in the browser.
///
/// `--new` mints a fresh key, which is also how a link is taken back:
/// there is no list of issued links to revoke one from, and there does
/// not need to be — the key *is* the link.
fn web(rest: &[String]) -> Result<(), String> {
    let fresh = match rest {
        [] => false,
        [flag] if flag == "--new" => true,
        _ => return Err(USAGE.into()),
    };
    let root = Node::root_from_env();
    let key = if fresh {
        khor_web::key::rotate(&root)?
    } else {
        khor_web::key::ensure(&root)?
    };
    if fresh {
        println!("{}", msg::WEB_FACE_ROTATED);
    }
    // Rotating is a local act and has already happened; only the
    // printing of an address depends on somebody being up to answer it.
    let port = khor_web::port();
    let Some(ips) = node()?.local_ips()? else {
        return Err(msg::WEB_FACE_NOT_SERVING.to_owned());
    };
    let Some(ip) = khor_web::best_address(&ips) else {
        return Err(msg::WEB_FACE_NOT_SERVING.to_owned());
    };
    // **The address about to be printed is the address that gets
    // probed.** Checking loopback instead would pass on a Mac whose
    // firewall has not been told about khor — loopback is exempt from
    // it and nothing else is — and the verb would hand somebody a link
    // that their phone silently cannot open.
    if !khor_web::answers_at(std::net::SocketAddr::new(ip, port)) {
        return Err(msg::web_face_down(std::net::SocketAddr::new(ip, port)));
    }
    let url = khor_web::link(ip, port, &key);
    println!("{}", msg::WEB_FACE_HERE);
    println!("  {url}");
    print!("{}", web_qr(&url));
    println!("{}", msg::WEB_FACE_KEY_NOTE);
    Ok(())
}

/// The link as a scannable block, for the phone that is going to open it.
///
/// **Colors, not characters.** A terminal renderer that prints `██` for
/// a dark module is betting on the reader having a light background; on
/// a dark one every module is inverted and most scanners refuse. So each
/// module carries its own black or white, and the code scans out of any
/// terminal.
///
/// Two rows per line through the upper half block: a character cell is
/// about twice as tall as it is wide, so one module per cell would come
/// out stretched, and stretched is the other way a code fails to scan.
fn web_qr(url: &str) -> String {
    let Ok(code) = qrcode::QrCode::new(url.as_bytes()) else {
        // A URL too long to encode is not a reason to withhold the link
        // that was already printed above.
        return String::new();
    };
    let width = code.width();
    let modules = code.to_colors();
    // The quiet zone is part of the symbol, not decoration: without a
    // light margin a scanner cannot find the finder patterns.
    let quiet = 4usize;
    let dark = |x: isize, y: isize| -> bool {
        if x < 0 || y < 0 || x as usize >= width || y as usize >= width {
            return false;
        }
        modules[y as usize * width + x as usize] == qrcode::Color::Dark
    };
    let span = width + quiet * 2;
    let mut out = String::new();
    let mut row = 0isize;
    while row < span as isize {
        for col in 0..span as isize {
            let (x, y) = (col - quiet as isize, row - quiet as isize);
            let upper = dark(x, y);
            let lower = dark(x, y + 1);
            // Foreground paints the upper half, background the lower.
            let fg = if upper { "30" } else { "97" };
            let bg = if lower { "40" } else { "107" };
            out.push_str(&format!("\x1b[{fg};{bg}m\u{2580}"));
        }
        out.push_str("\x1b[0m\n");
        row += 2;
    }
    out
}

#[cfg(test)]
mod web_qr_tests {
    use super::web_qr;

    /// Reads the painted block back into modules. Dark is black — `30`
    /// on top, `40` underneath — and everything this test knows about
    /// the picture, a scanner knows too.
    fn read_back(block: &str) -> Vec<Vec<bool>> {
        let mut rows: Vec<Vec<bool>> = Vec::new();
        for line in block.lines() {
            let (mut upper, mut lower) = (Vec::new(), Vec::new());
            for cell in line.trim_end_matches("\x1b[0m").split('\x1b').skip(1) {
                let Some(codes) = cell.strip_prefix('[').and_then(|c| c.split_once('m')).map(|(c, _)| c)
                else {
                    continue;
                };
                let Some((fg, bg)) = codes.split_once(';') else { continue };
                upper.push(fg == "30");
                lower.push(bg == "40");
            }
            rows.push(upper);
            rows.push(lower);
        }
        rows
    }

    /// **The picture is the code.** The half-block packing is the part
    /// written here rather than by the library, and every way it can be
    /// wrong — inverted colors, rows paired off by one, a quiet zone on
    /// three sides — produces a block that still *looks* like a QR code
    /// in a terminal and cannot be scanned. So the block is read back
    /// and compared module for module against what the encoder said.
    #[test]
    fn the_painted_block_is_the_same_code_the_encoder_made() {
        let url = "http://192.168.1.20:5467/?k=9516d9585187f1e2e7d7b6793c46f905";
        let code = qrcode::QrCode::new(url.as_bytes()).expect("a URL this size encodes");
        let width = code.width();
        let modules = code.to_colors();
        let quiet = 4;

        let painted = read_back(&web_qr(url));
        assert!(painted.len() >= width + quiet * 2, "the block is shorter than the symbol");
        for y in 0..width {
            for x in 0..width {
                assert_eq!(
                    painted[y + quiet][x + quiet],
                    modules[y * width + x] == qrcode::Color::Dark,
                    "module ({x},{y}) came out wrong — the picture is not this code"
                );
            }
        }
        // The margin, on all four sides. A scanner needs it to find the
        // finder patterns, and it is the easiest half of this to drop.
        for y in 0..quiet {
            assert!(painted[y].iter().all(|d| !d), "row {y} of the quiet zone is not blank");
        }
        for row in painted.iter().take(width + quiet).skip(quiet) {
            assert!(row[..quiet].iter().all(|d| !d), "the left margin is not blank");
            assert!(row[width + quiet..].iter().all(|d| !d), "the right margin is not blank");
        }
    }

    /// A code that cannot be made costs the link nothing: the address is
    /// printed before this is called, and a person can still type it.
    #[test]
    fn an_unencodable_link_is_no_picture_rather_than_no_link() {
        assert_eq!(web_qr(&"x".repeat(10_000)), "");
    }
}

/// Processes only, files stay — the opposite end of `close`, which is
/// per-session and deletes what that session received. The next
/// `khor serve` (or the guardian's next boot) brings everything back.
fn quit(rest: &[String]) -> Result<(), String> {
    let [] = rest else {
        return Err(USAGE.into());
    };
    let (served, hosts) = node()?.quit()?;
    println!("{}", if served { msg::QUIT_SERVE_STOPPED } else { msg::QUIT_NO_SERVE });
    if hosts > 0 {
        println!("{}", msg::quit_hosts(hosts));
    }
    Ok(())
}

/// The ACP agent registry (`khor_sync::agents`): what this person has
/// told khor about, said once and true on every machine.
///
/// Three modes on one word, `hooks`'s shape. `add` takes the command
/// after `--` as argv rather than as one string: a path with a space in
/// it survives that and does not survive re-splitting, and argv is
/// exactly what the child is spawned from.
/// `khor accounts` — the registered API accounts, and 登记/rm
/// (`Node::register_account`). The key comes in on stdin (`--key-stdin`)
/// or on the line (`--key`); either way it lands in `.khor/keys/` on
/// this machine and nowhere else (`accounts` module head).
fn accounts(rest: &[String]) -> Result<(), String> {
    let n = node()?;
    match rest.split_first() {
        None => {
            let names: std::collections::HashMap<String, String> =
                n.devices()?.into_iter().map(|d| (d.id, d.name)).collect();
            let listed = n.accounts()?;
            if listed.is_empty() {
                println!("{}", cli::ACCOUNTS_NONE);
            }
            for a in listed {
                println!(
                    "{}	{}	{}	{}	{}",
                    a.id,
                    a.name,
                    a.provider.word(),
                    names.get(&a.home).cloned().unwrap_or(a.home),
                    a.base_url.unwrap_or_default()
                );
            }
            Ok(())
        }
        Some((word, tail)) if word == "add" => {
            let mut it = tail.iter();
            let name = it.next().ok_or_else(|| USAGE.to_string())?.clone();
            let mut provider: Option<String> = None;
            let mut base_url: Option<String> = None;
            let mut key: Option<String> = None;
            let mut budget: Option<f64> = None;
            while let Some(flag) = it.next() {
                let mut value = || it.next().ok_or_else(|| USAGE.to_string()).cloned();
                match flag.as_str() {
                    "--provider" => provider = Some(value()?),
                    "--base-url" => base_url = Some(value()?),
                    "--key" => key = Some(value()?),
                    "--key-stdin" => {
                        let mut text = String::new();
                        std::io::stdin().read_line(&mut text).map_err(|e| e.to_string())?;
                        key = Some(text.trim().to_owned());
                    }
                    "--budget" => {
                        budget = Some(value()?.parse::<f64>().map_err(|_| cli::ACCOUNT_BUDGET_NUMBER.to_string())?);
                    }
                    _ => return Err(USAGE.into()),
                }
            }
            let provider = provider.ok_or_else(|| USAGE.to_string())?;
            let provider = khor_node::Provider::parse(&provider)
                .ok_or_else(|| msg::no_such_provider(&provider))?;
            let key = key.ok_or_else(|| USAGE.to_string())?;
            let a = n.register_account(&name, provider, base_url.as_deref(), &key, budget)?;
            println!("{}", a.id);
            Ok(())
        }
        Some((word, tail)) if word == "rm" => {
            let [which] = tail else { return Err(USAGE.into()) };
            // By id or by name — a name is what a person remembers, and
            // it is refused when two accounts share it rather than
            // guessed between.
            let all = n.accounts()?;
            let id = if all.iter().any(|a| &a.id == which) {
                which.clone()
            } else {
                let named: Vec<&khor_node::Account> = all.iter().filter(|a| &a.name == which).collect();
                match named.as_slice() {
                    [one] => one.id.clone(),
                    [] => return Err(msg::no_such_account(which)),
                    _ => return Err(cli::account_name_ambiguous(which)),
                }
            };
            n.remove_account(&id)
        }
        Some(_) => Err(USAGE.into()),
    }
}

/// `khor quota` — every registered account's reading, this machine's
/// read now and the others' as last carried here (`Node::quota_board`).
fn quota(rest: &[String]) -> Result<(), String> {
    if !rest.is_empty() {
        return Err(USAGE.into());
    }
    let n = node()?;
    let names: std::collections::HashMap<String, String> =
        n.devices()?.into_iter().map(|d| (d.id, d.name)).collect();
    let board = rt()?.block_on(n.quota_board())?;
    if board.is_empty() {
        println!("{}", cli::ACCOUNTS_NONE);
    }
    for (r, age_ms) in board {
        let home = names.get(&r.home).cloned().unwrap_or_else(|| r.home.clone());
        let figure = match (&r.trouble, r.spent, r.limit) {
            (Some(t), _, _) => cli::quota_trouble(t),
            (None, Some(s), Some(l)) => format!("{s:.2}/{l:.2} {}", r.currency.clone().unwrap_or_default()),
            (None, Some(s), None) if r.unlimited => format!("{s:.2} {} {}", r.currency.clone().unwrap_or_default(), cli::QUOTA_UNLIMITED),
            (None, Some(s), None) => format!("{s:.2} {}", r.currency.clone().unwrap_or_default()),
            (None, None, _) => cli::QUOTA_NO_NUMBER.to_owned(),
        };
        let share = r.fill.map(|f| format!("{:.0}%", f * 100.0)).unwrap_or_default();
        println!("{}	{}	{}	{}	{}	{}s", r.name, r.provider, home, figure, share, age_ms / 1000);
    }
    Ok(())
}

fn agents(rest: &[String]) -> Result<(), String> {
    let n = node()?;
    match rest.split_first() {
        None => {
            let listed = n.agents()?;
            if listed.is_empty() {
                // An empty registry and a broken one look identical;
                // this line is the difference, and it is also the one
                // thing a person cannot guess.
                println!("{}", cli::AGENTS_NONE);
            }
            for (name, spec) in listed {
                println!("{name}\t{}", spec.typed());
            }
            Ok(())
        }
        // The shipped snapshot of the ACP registry. A separate word
        // rather than a flag on the listing above because they answer
        // different questions — "what have I told khor about" and "what
        // is there" — and folding them into one list would put things
        // this person has never used beside the ones they run daily.
        Some((word, tail)) if word == "known" => {
            if !tail.is_empty() {
                return Err(USAGE.into());
            }
            use khor_node::agent_registry::{KNOWN, STARTABLE, TAKEN};
            println!("{}", cli::agents_known_head(TAKEN, KNOWN.len(), STARTABLE));
            for k in KNOWN {
                // The argv is printed for the startable ones, because
                // what khor would run is the one thing a person cannot
                // find out any other way, and「khor 起得来」without it
                // is a claim they have to take on trust.
                match k.argv {
                    Some(argv) => println!(
                        "{}\t{}\t{}\t{}",
                        k.id,
                        k.name,
                        cli::AGENTS_KNOWN_STARTABLE,
                        argv.join(" ")
                    ),
                    None => println!(
                        "{}\t{}\t{}",
                        k.id,
                        k.name,
                        cli::AGENTS_KNOWN_BYHAND
                    ),
                }
            }
            Ok(())
        }
        // **Ask the agent itself what it has.** The only discovery
        // path for an agent khor did not write a disk reader for —
        // and one that starts a real process, so it is a verb a person
        // types rather than something a list poll does behind them.
        // **Reads what was remembered; `--ask` is what starts a
        // process.** Asking is the expensive half — a whole agent
        // spawns — so it is a thing a person types, and the answer is
        // kept and synced so no other machine has to spend it again.
        Some((word, tail)) if word == "sessions" => {
            let (name, ask) = match tail {
                [name] => (name, false),
                [name, flag] if flag == "--ask" => (name, true),
                _ => return Err(USAGE.into()),
            };
            let seen = if ask {
                Some(n.refresh_agent_sessions(name)?)
            } else {
                n.agent_seen(name)?
            };
            // **Four answers, and each one is a different sentence.**
            // Never asked / asked and it cannot / it has none / here
            // they are. The first two were one answer on this feature's
            // first run, and the person's next move differs for each:
            // ask, stop asking, open a new one, pick one.
            let rows = match seen {
                None => {
                    println!("{}", cli::agents_sessions_unasked(name));
                    return Ok(());
                }
                Some(khor_node::AgentAnswer::Cannot) => {
                    println!("{}", cli::agents_sessions_cannot(name));
                    return Ok(());
                }
                Some(khor_node::AgentAnswer::Sessions { rows }) => rows,
            };
            if rows.is_empty() {
                println!("{}", cli::agents_sessions_none(name));
                return Ok(());
            }
            println!(
                "{}",
                if ask {
                    cli::agents_sessions_asked(name, rows.len())
                } else {
                    // The date of the freshest thing it told us, which
                    // is the closest honest answer to「how old is
                    // this」without storing a clock reading of our own.
                    let at = rows.iter().filter_map(|r| r.updated_at.as_deref()).max();
                    cli::agents_sessions_head(name, rows.len(), at.unwrap_or_default())
                }
            );
            for r in rows {
                // Title where there is one, cwd where there is not: an
                // id alone is not something a person recognises, and
                // the cwd is the next best thing they do.
                let said = r.title.unwrap_or(r.cwd);
                println!("{}\t{}\t{}", r.id, said, r.updated_at.unwrap_or_default());
            }
            Ok(())
        }
        Some((word, tail)) if word == "add" => {
            // `--` is optional in the same way `open`'s is: everything
            // after the name is the command either way, and a person
            // who typed the separator should not be told they are
            // wrong for it.
            let (name, argv) = tail.split_first().ok_or_else(|| USAGE.to_string())?;
            let argv: Vec<String> =
                argv.iter().filter(|a| a.as_str() != "--").cloned().collect();
            // **`@id` expands out of the snapshot.** The sigil is what
            // keeps this from being a guess: without it, a single-word
            // argv that happens to match a registry id would be
            // silently replaced by something else entirely, and a
            // person who has a binary called `gemini` on their PATH
            // would find khor running npx instead. `@` is not a command
            // on any platform, so asking for the snapshot is a thing
            // you can only do on purpose.
            //
            // **The expansion itself lives on `Node`** (2026-08-31):
            // the wizard expands `@id` too now, and this repo's own
            // judgment is that a rule written on two faces grows two.
            let argv = match argv.split_first() {
                Some((first, rest)) if rest.is_empty() && first.starts_with('@') => {
                    n.known_argv(&first[1..])?
                }
                _ => argv,
            };
            n.register_agent(name, &argv)?;
            println!("{}", cli::agents_added(name));
            // **Asked once, here, because this is a moment the person
            // is already waiting on this agent.** A failure to answer
            // is not a failure to register: the registration stands,
            // and the question can be put again later.
            if let Ok(khor_node::AgentAnswer::Sessions { rows }) = n.refresh_agent_sessions(name) {
                if !rows.is_empty() {
                    println!("{}", cli::agents_sessions_asked(name, rows.len()));
                }
            }
            Ok(())
        }
        Some((word, tail)) if word == "rm" => {
            let [name] = tail else { return Err(USAGE.into()) };
            // Said before the removal, because after it the answer is
            // the same either way — and a typo that prints 「不再登记」
            // reads as a success.
            let had = n.agent(name)?.is_some();
            n.forget_agent(name)?;
            println!(
                "{}",
                if had { cli::agents_forgotten(name) } else { cli::agents_was_not_there(name) }
            );
            Ok(())
        }
        Some(_) => Err(USAGE.into()),
    }
}

/// Three shapes, one verb: asking is the default because it is the safe
/// one, and both writes have to be typed. `khor hooks` alone is also how
/// somebody checks what `install` or `uninstall` just did — a write
/// nobody can verify afterwards is a write on trust.
fn hooks(rest: &[String]) -> Result<(), String> {
    let n = node()?;
    match rest {
        [] => {
            let report = n.hooks_report()?;
            println!("{}", cli::hooks_file(report.path.display()));
            let mut incomplete = false;
            for (event, state) in &report.events {
                let said = match state {
                    khor_node::adaptor::claude::Installed::Here => cli::HOOK_HERE.to_owned(),
                    khor_node::adaptor::claude::Installed::Missing => {
                        incomplete = true;
                        cli::HOOK_MISSING.to_owned()
                    }
                    khor_node::adaptor::claude::Installed::Elsewhere(cmd) => {
                        incomplete = true;
                        cli::hook_elsewhere(cmd)
                    }
                };
                println!("{event}\t{said}");
            }
            if incomplete {
                println!("{}", cli::HOOKS_WORTH_INSTALLING);
            }
            Ok(())
        }
        [word] if word == "install" => {
            let done = n.install_hooks()?;
            println!("{}", cli::hooks_file(done.path.display()));
            let list = |v: &[&str]| v.join(cli::NAME_SEPARATOR);
            if !done.added.is_empty() {
                println!("{}", cli::hooks_added(list(&done.added)));
            }
            if !done.repointed.is_empty() {
                println!("{}", cli::hooks_repointed(list(&done.repointed)));
            }
            if !done.unchanged.is_empty() {
                println!("{}", cli::hooks_already(list(&done.unchanged)));
            }
            if !done.added.is_empty() || !done.repointed.is_empty() {
                println!("{}", cli::HOOKS_RESTART_CLAUDE);
            }
            Ok(())
        }
        [word] if word == "uninstall" => {
            let done = n.uninstall_hooks()?;
            println!("{}", cli::hooks_file(done.path.display()));
            // Both outcomes are said out loud. "Removed nothing" is not
            // silence: nobody can check somebody else's settings file by
            // hand, and a second run that printed the same thing as the
            // first would be the only way to see that twice equals once.
            if done.removed.is_empty() {
                println!("{}", cli::HOOKS_NONE_TO_REMOVE);
            } else {
                println!("{}", cli::hooks_removed(done.removed.join(cli::NAME_SEPARATOR)));
                println!("{}", cli::HOOKS_RESTART_CLAUDE);
            }
            Ok(())
        }
        _ => Err(USAGE.into()),
    }
}

fn invite(_rest: &[String]) -> Result<(), String> {
    // The ticket goes to stdout so it can be piped; how long it lasts
    // goes to stderr, because a window nobody is told about is a refusal
    // nobody can explain later.
    println!("{}", node()?.invite()?);
    eprintln!("{}", cli::invite_window(khor_node::link::invite_window_minutes()));
    Ok(())
}

fn pair(rest: &[String]) -> Result<(), String> {
    let [ticket] = rest else {
        return Err(USAGE.into());
    };
    let name = rt()?.block_on(node()?.pair(ticket))?;
    println!("{}", cli::paired_ok(name));
    Ok(())
}

/// Leads an ssh-only machine into the network as this machine's
/// satellite (附属机器批) — `Node::adopt_satellite` does the probing
/// and the minting; this prints the report, and the tmux line rides
/// along so the terminal's nature is no surprise later.
fn adopt(rest: &[String]) -> Result<(), String> {
    let (host, name) = match rest {
        [host] => (host, None),
        [host, name] => (host, Some(name.as_str())),
        _ => return Err(USAGE.into()),
    };
    let n = node()?;
    let (_, name, tmux) = n.adopt_satellite(host, name)?;
    println!("{}", cli::adopted(&name, n.name()));
    println!("{}", if tmux { cli::SAT_TMUX_YES } else { cli::SAT_TMUX_NO });
    Ok(())
}

fn sync(_rest: &[String]) -> Result<(), String> {
    let outcomes = rt()?.block_on(node()?.sync_now())?;
    if outcomes.is_empty() {
        println!("{}", cli::NOTHING_TO_SYNC);
    }
    for (name, verdict) in outcomes {
        match verdict {
            Ok(what) => println!("{name}: {what}"),
            Err(e) => println!("{}", cli::sync_failed_line(name, e)),
        }
    }
    Ok(())
}

fn help(_rest: &[String]) -> Result<(), String> {
    println!("{USAGE}");
    Ok(())
}

/// Raw passthrough to a hosted session: keystrokes and size changes go
/// out framed, PTY bytes come back raw. Ctrl-\ detaches; the session
/// stays. One writer thread frames everything so ops never interleave.
#[cfg(unix)]
/// The machine a session lives on, when that is not this one. `None`
/// means "mine" — including for a row this machine has never heard of,
/// which then fails the ordinary local way with the ordinary local
/// words.
fn attach_to(n: &Node, id: &SessionId) -> Result<(), String> {
    let dir = n
        .session_dir(id)
        .ok_or_else(|| msg::no_such_session(&id.0))?;
    let hf = khor_node::host::read_host_file(&dir)?;
    attach_at(n, id, &format!("127.0.0.1:{}", hf.port), &hf.cookie)
}

/// The attach loop, against an address that is already resolved.
///
/// Local and remote share every line below the address: the terminal
/// protocol is the same one either way, and the only thing a far
/// session changes is what is on the other end of the socket (a pipe
/// the resident serve bound, `Node::reach`). Splitting anywhere lower
/// than this would mean two copies of the raw-mode dance.
fn attach_at(n: &Node, id: &SessionId, addr: &str, cookie: &str) -> Result<(), String> {
    use khor_node::host::{connect_at, write_frame, ClientOp};
    use std::io::{Read, Write};
    use std::sync::atomic::{AtomicBool, Ordering};

    // isatty decides whether this is a terminal; the size is separate —
    // a pty freshly made by script(1) or a GUI reports 0×0, which is a
    // default-worthy size, not a disqualification.
    if unsafe { libc::isatty(0) } != 1 {
        return Err(msg::ATTACH_NEEDS_TTY.into());
    }
    let (cols, rows) = tty_size().unwrap_or((80, 24));
    let mut stream = connect_at(addr, cookie, cols, rows)?;
    eprintln!("{}", cli::attached(&id.0));

    let saved = raw_on()?;
    let detached = std::sync::Arc::new(AtomicBool::new(false));
    let (tx, rx) = std::sync::mpsc::channel::<ClientOp>();
    {
        let mut w = stream.try_clone().map_err(|e| e.to_string())?;
        std::thread::spawn(move || {
            for op in rx {
                if write_frame(&mut w, &op).is_err() {
                    break;
                }
            }
        });
    }
    {
        let tx = tx.clone();
        let detached = detached.clone();
        let s = stream.try_clone().map_err(|e| e.to_string())?;
        std::thread::spawn(move || {
            let mut stdin = std::io::stdin();
            let mut buf = [0u8; 1024];
            loop {
                let Ok(nread) = stdin.read(&mut buf) else { break };
                if nread == 0 {
                    break;
                }
                let chunk = &buf[..nread];
                if let Some(pos) = chunk.iter().position(|&b| b == 0x1c) {
                    if pos > 0 {
                        let _ = tx.send(ClientOp::Input(chunk[..pos].to_vec()));
                    }
                    detached.store(true, Ordering::SeqCst);
                    let _ = s.shutdown(std::net::Shutdown::Both);
                    break;
                }
                if tx.send(ClientOp::Input(chunk.to_vec())).is_err() {
                    break;
                }
            }
        });
    }
    {
        std::thread::spawn(move || {
            let mut last = (cols, rows);
            loop {
                std::thread::sleep(std::time::Duration::from_millis(300));
                let Some(now) = tty_size() else { continue };
                if now != last {
                    last = now;
                    if tx.send(ClientOp::Resize { cols: now.0, rows: now.1 }).is_err() {
                        break;
                    }
                }
            }
        });
    }
    let mut out = std::io::stdout();
    let mut buf = [0u8; 8192];
    loop {
        match stream.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(nread) => {
                if out.write_all(&buf[..nread]).and_then(|()| out.flush()).is_err() {
                    break;
                }
            }
        }
    }
    raw_off(&saved);
    if detached.load(Ordering::SeqCst) {
        eprintln!("{}", cli::detached(&id.0));
    } else {
        // The stream ended on the host's side — say what the row says.
        match n.sessions().ok().and_then(|v| v.into_iter().find(|v| v.session.id == *id)) {
            Some(v) => match v.session.state.state.key() {
                key @ ("done" | "failed" | "idle") => {
                    eprintln!("{}", cli::session_settled(state::word(key)))
                }
                key => eprintln!("{}", cli::link_dropped(state::word(key), &id.0)),
            },
            None => eprintln!("{}", cli::SESSION_CLOSED),
        }
    }
    // The stdin thread may still be blocked on read(2); leaving is how
    // this process lets go of it.
    std::process::exit(0);
}

#[cfg(not(unix))]
fn attach_to(_n: &Node, _id: &SessionId) -> Result<(), String> {
    Err(msg::ATTACH_WINDOWS_LATER.into())
}

#[cfg(unix)]
fn tty_size() -> Option<(u16, u16)> {
    unsafe {
        let mut ws: libc::winsize = std::mem::zeroed();
        if libc::ioctl(1, libc::TIOCGWINSZ, &mut ws) == 0 && ws.ws_col > 0 {
            Some((ws.ws_col, ws.ws_row))
        } else {
            None
        }
    }
}

#[cfg(not(unix))]
fn tty_size() -> Option<(u16, u16)> {
    None
}

#[cfg(unix)]
fn raw_on() -> Result<libc::termios, String> {
    unsafe {
        let mut t: libc::termios = std::mem::zeroed();
        if libc::tcgetattr(0, &mut t) != 0 {
            return Err(msg::NO_TERMIOS.into());
        }
        let saved = t;
        libc::cfmakeraw(&mut t);
        if libc::tcsetattr(0, libc::TCSANOW, &t) != 0 {
            return Err(msg::NO_RAW_MODE.into());
        }
        Ok(saved)
    }
}

#[cfg(unix)]
fn raw_off(saved: &libc::termios) {
    unsafe {
        libc::tcsetattr(0, libc::TCSANOW, saved);
    }
}

/// What this machine is wearing, and what else it could wear.
///
/// The five colors get a line of their own rather than only a factory
/// name: once a slot has been changed by hand the style belongs to no
/// factory set, and a listing that could only name sets would have
/// nothing at all to say about the machine in front of it.
///
/// Keys are printed beside the words because the keys are what the flags
/// take. A listing of words alone would be a menu you cannot order from.
fn print_face(style: &AvatarStyle) {
    let mark = |on: bool| if on { format!("\t{}", cli::IN_USE) } else { String::new() };
    println!("{}", cli::group_header(avatar::AXIS_PALETTE));
    println!("{}", style.palette.colors().join("\t"));
    let worn = style.palette.preset_id();
    for p in PRESETS {
        println!("{}\t{}{}", p.id, avatar::word(p.id), mark(worn == Some(p.id)));
    }
    println!("{}", cli::group_header(avatar::AXIS_VARIANT));
    for v in Variant::ALL {
        println!("{}\t{}{}", v.key(), avatar::word(v.key()), mark(v == style.variant));
    }
    println!("{}", cli::group_header(avatar::AXIS_SHAPE));
    for s in FaceShape::ALL {
        println!("{}\t{}{}", s.key(), avatar::word(s.key()), mark(s == style.shape));
    }
}

/// A group key as a person reads it.
///
/// The prefix says which kind of thing the rest is, so this dispatches
/// instead of guessing (`khor_node::list` module head): a state key and
/// a category key get looked up, a machine name is printed as it is.
/// Without the prefixes a machine called `busy` would come out 忙碌.
fn group_word(group: &str) -> String {
    if group == list::GROUP_PINNED {
        return cli::GROUP_PINNED.to_owned();
    }
    if let Some(key) = group.strip_prefix(list::GROUP_STATE) {
        return state::word(key).to_owned();
    }
    if let Some(name) = group.strip_prefix(list::GROUP_CATEGORY) {
        // The empty category is the row nobody could place; every other
        // name echoes if the catalog has no entry, which is how vendor
        // names come through as themselves.
        let key = if name.is_empty() { "unknown" } else { name };
        return category::word(key).to_owned();
    }
    group.strip_prefix(list::GROUP_DEVICE).unwrap_or(group).to_owned()
}

fn human_age(ms: u64) -> String {
    let s = ms / 1000;
    if s < 120 {
        cli::age_seconds(s)
    } else if s < 7200 {
        cli::age_minutes(s / 60)
    } else if s < 172_800 {
        cli::age_hours(s / 3600)
    } else {
        cli::age_days(s / 86_400)
    }
}

fn render(m: &khor_node::Message) -> String {
    if m.retracted {
        return cli::RETRACTED.into();
    }
    match &m.body {
        MsgBody::Text(t) => t.clone(),
        // The transfer id rides the line: with no transfer row on the
        // list any more (传输并入对话批), this is where the handle
        // `khor accept` wants is read from.
        MsgBody::Files(fs) => format!(
            "{}\ttransfer/{}",
            fs.iter()
                .map(|f| cli::file_chip(&f.name, f.size))
                .collect::<Vec<_>>()
                .join(" "),
            m.id
        ),
        MsgBody::Unknown(k) => cli::unknown_body(k),
    }
}

/// **The usage text and the dispatch table say the same words.**
///
/// `VERBS` makes half of this structural — a word cannot exist without
/// code, nor code without a word. The half no compiler can hold is the
/// usage text: it is prose, and prose is where the drift lived. These
/// tests are the other half.
///
/// **Nothing here runs a verb.** They read `word` and never touch `run`,
/// which is the only reason a gate may be exhaustive over a list holding
/// `serve` and `open`: calling them to prove they exist would start a
/// server and open a session on whatever machine runs the tests.
///
/// They live inside the binary because a `[[bin]]` crate has nothing to
/// import — and being unreachable from `tests/` is the same property that
/// keeps `VERBS` the only place a verb is declared.
#[cfg(test)]
mod gate {
    use super::{NOTHING_TYPED, NOT_IN_USAGE, VERBS};
    use khor_catalog::cli::USAGE;

    /// The words the usage promises, read back out of the text a person
    /// reads.
    ///
    /// An entry starts at the list's left margin — two spaces, then the
    /// word; a description that wraps is indented past it. Reading the
    /// prose is the point: fed from a second, tidier list, this would
    /// only prove the two lists agree, and the text somebody actually
    /// reads could still say anything.
    fn promised() -> Vec<&'static str> {
        USAGE
            .lines()
            .filter_map(|l| l.strip_prefix("  "))
            .filter(|l| !l.starts_with(' '))
            .filter_map(|l| l.split_whitespace().next())
            .collect()
    }

    #[test]
    fn the_usage_promises_nothing_khor_cannot_answer() {
        let promised = promised();
        for word in &promised {
            assert!(
                VERBS.iter().any(|v| v.word == *word),
                "the usage promises `{word}` and nothing answers it"
            );
        }
        // Last, because it is the vaguer complaint of the two: a parser
        // that quietly stopped matching would leave the loop above with
        // nothing to walk and this test green. The count is derived from
        // the two tables — never a number typed in here.
        assert_eq!(
            promised.len(),
            VERBS.len() - NOT_IN_USAGE.len(),
            "the usage did not parse into one word per listed verb: {promised:?}"
        );
    }

    #[test]
    fn every_verb_is_promised_or_on_record_as_hidden() {
        let promised = promised();
        for v in VERBS {
            let hidden = NOT_IN_USAGE.iter().any(|(word, _)| *word == v.word);
            assert!(
                promised.contains(&v.word) || hidden,
                "`{}` is a verb the usage never mentions and the register never explains",
                v.word
            );
            assert!(
                !(promised.contains(&v.word) && hidden),
                "`{}` is in the usage and also on record as kept out of it",
                v.word
            );
        }
    }

    #[test]
    fn the_register_explains_only_verbs_khor_has() {
        for (word, _) in NOT_IN_USAGE {
            assert!(
                VERBS.iter().any(|v| v.word == *word),
                "the register explains `{word}`, which is not a verb"
            );
        }
    }

    /// Two rows claiming one word: dispatch takes the first, and the
    /// second is code nobody can reach by typing.
    #[test]
    fn no_two_rows_claim_the_same_word() {
        for (i, v) in VERBS.iter().enumerate() {
            assert!(
                !VERBS[..i].iter().any(|e| e.word == v.word),
                "`{}` is claimed twice; the second row is unreachable",
                v.word
            );
        }
    }

    #[test]
    fn khor_with_no_verb_lands_on_one() {
        assert!(
            VERBS.iter().any(|v| v.word == NOTHING_TYPED),
            "`khor` alone falls back to `{NOTHING_TYPED}`, which no row claims"
        );
    }
}
