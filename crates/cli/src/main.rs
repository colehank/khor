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
    Verb { word: "usage", run: usage },
    Verb { word: "sessions", run: sessions },
    Verb { word: "tell", run: tell },
    Verb { word: "send", run: send },
    Verb { word: "accept", run: accept },
    Verb { word: "log", run: log },
    Verb { word: "run", run: run },
    Verb { word: "open", run: open },
    Verb { word: "attach", run: attach },
    Verb { word: "state", run: state },
    Verb { word: "seen", run: seen },
    Verb { word: "pin", run: pin },
    Verb { word: "unpin", run: unpin },
    Verb { word: "close", run: close },
    Verb { word: "serve", run: serve },
    Verb { word: "hooks", run: hooks },
    Verb { word: "invite", run: invite },
    Verb { word: "pair", run: pair },
    Verb { word: "sync", run: sync },
    Verb { word: "_host", run: host },
    Verb { word: "_ghost", run: ghost },
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
    ("help", "prints the usage text; a list that lists itself teaches nobody anything"),
    ("--help", "the spelling people try before reading anything"),
    ("-h", "the short spelling of the same"),
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
    let (entries, truncated) = rt()?.block_on(node()?.ls_of(machine, path))?;
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

fn accept(rest: &[String]) -> Result<(), String> {
    let [id] = rest else {
        return Err(USAGE.into());
    };
    let moved = rt()?.block_on(node()?.accept(&SessionId(id.clone())))?;
    println!("{}", cli::pulled(moved));
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
    let mut it = rest.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--tui" if cmd.is_empty() => tui = true,
            "--gui" if cmd.is_empty() => gui = true,
            "-d" if cmd.is_empty() => detached = true,
            "--title" if cmd.is_empty() => {
                title = Some(it.next().ok_or_else(|| USAGE.to_string())?.clone());
            }
            "--" if cmd.is_empty() => {}
            _ => cmd.push(a.clone()),
        }
    }
    if cmd.is_empty() {
        cmd = vec![std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into())];
    }
    let n = node()?;
    let title = title.unwrap_or_else(|| {
        std::path::Path::new(&cmd[0])
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| cmd[0].clone())
    });
    if gui {
        // No terminal exists to attach to; the id is the deliverable.
        let id = n.open_gui(&title, &cmd)?;
        println!("{}", id.0);
        return Ok(());
    }
    let kind = if tui { khor_node::kind::TUI } else { khor_node::kind::SHELL };
    let size = tty_size().unwrap_or((80, 24));
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
    attach_to(&node()?, &SessionId(sid.clone()))
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
        [id] => n.pin_session(&SessionId(id.clone()), on),
        _ => Err(USAGE.into()),
    }
}

fn close(rest: &[String]) -> Result<(), String> {
    let [id] = rest else {
        return Err(USAGE.into());
    };
    node()?.close(&SessionId(id.clone()))
}

fn serve(_rest: &[String]) -> Result<(), String> {
    let n = node()?;
    eprintln!("{}", cli::serve_banner(n.name()));
    rt()?.block_on(n.serve())
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
fn attach_to(n: &Node, id: &SessionId) -> Result<(), String> {
    use khor_node::host::{connect, write_frame, ClientOp};
    use std::io::{Read, Write};
    use std::sync::atomic::{AtomicBool, Ordering};

    let dir = n
        .session_dir(id)
        .ok_or_else(|| msg::no_such_session(&id.0))?;
    // isatty decides whether this is a terminal; the size is separate —
    // a pty freshly made by script(1) or a GUI reports 0×0, which is a
    // default-worthy size, not a disqualification.
    if unsafe { libc::isatty(0) } != 1 {
        return Err(msg::ATTACH_NEEDS_TTY.into());
    }
    let (cols, rows) = tty_size().unwrap_or((80, 24));
    let mut stream = connect(&dir, cols, rows)?;
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
