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
    Verb { word: "state", run: state },
    Verb { word: "seen", run: seen },
    Verb { word: "pin", run: pin },
    Verb { word: "unpin", run: unpin },
    Verb { word: "close", run: close },
    Verb { word: "serve", run: serve },
    Verb { word: "web", run: web },
    Verb { word: "quit", run: quit },
    Verb { word: "hooks", run: hooks },
    Verb { word: "invite", run: invite },
    Verb { word: "pair", run: pair },
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
    for d in n.devices()? {
        let here = if d.name == n.name() { cli::THIS_MACHINE } else { "" };
        let pin = if d.pinned { cli::PINNED_MARK } else { "" };
        println!("{}\t{}…{here}{pin}\t{}", d.name, &d.id[..16], vitals_line(&n, &d.id));
    }
    Ok(())
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
    let mode = match rest {
        [] => list::Arrange::Recent,
        [flag, key] if flag == "--by" => list::Arrange::from_key(key).ok_or_else(|| {
            let all: Vec<&str> = list::Arrange::ALL.iter().map(|a| a.key()).collect();
            msg::not_a_way_to_arrange(key, all.join(cli::NAME_SEPARATOR))
        })?,
        _ => return Err(USAGE.into()),
    };
    let n = node()?;
    // The heading is printed when the group changes, which is also why
    // the node returns rows already grouped: this loop owns no
    // comparison, it only notices the boundary.
    let mut current: Option<String> = None;
    for a in n.sessions_arranged(mode)? {
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
        println!(
            "{}\t{}\t{}\t{}{pin}{}",
            s.id.0,
            state::word(s.state.state.key()),
            cli::unread(s.unread),
            s.title,
            src
        );
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

fn send(rest: &[String]) -> Result<(), String> {
    let [machine, path] = rest else {
        return Err(USAGE.into());
    };
    let id = node()?.send(machine, std::path::Path::new(path))?;
    println!("{}", cli::summary_sent(&id.0));
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
            "--title" if cmd.is_empty() => {
                title = Some(it.next().ok_or_else(|| USAGE.to_string())?.clone());
            }
            "--" if cmd.is_empty() => {}
            _ => cmd.push(a.clone()),
        }
    }
    let n = node()?;
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
            cmd = vec![spec.launch()];
            Some(name.clone())
        }
        None => None,
    };
    if cmd.is_empty() {
        cmd = vec![std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into())];
    }
    let title = title.unwrap_or_else(|| match &agent {
        // A launch JSON has no file name worth reading; the name the
        // person gave the agent is the one they will recognise.
        Some(name) => name.clone(),
        None => std::path::Path::new(&cmd[0])
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| cmd[0].clone()),
    });
    if gui {
        // **Refused out loud, not ignored.** `--on` used to fall
        // through this branch untouched, so `--gui --on turing` opened
        // a conversation on *this* machine and said nothing — a flag
        // that is accepted and does nothing is worse than one that is
        // rejected, because the screen agrees with the person.
        if on.is_some() {
            return Err(cli::GUI_NOT_ON_ANOTHER_MACHINE.into());
        }
        // No terminal exists to attach to; the id is the deliverable.
        let id = match &vendor {
            // The name is the user's own answer to "whose session is
            // this" — being told is not the guess `Session::category`
            // forbids (`gui_host`'s vendor door).
            Some(v) => n.open_gui_as(&title, &cmd, v)?,
            None => n.open_gui(&title, &cmd)?,
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
        Some((word, tail)) if word == "add" => {
            // `--` is optional in the same way `open`'s is: everything
            // after the name is the command either way, and a person
            // who typed the separator should not be told they are
            // wrong for it.
            let (name, argv) = tail.split_first().ok_or_else(|| USAGE.to_string())?;
            let argv: Vec<String> =
                argv.iter().filter(|a| a.as_str() != "--").cloned().collect();
            n.register_agent(name, &argv)?;
            println!("{}", cli::agents_added(name));
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
        MsgBody::Files(fs) => fs
            .iter()
            .map(|f| cli::file_chip(&f.name, f.size))
            .collect::<Vec<_>>()
            .join(" "),
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
