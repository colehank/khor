//! The `khor` CLI: the library's first consumer. Every verb is one node
//! call — anything the GUI will do must be reachable from here first
//! (docs/KHOR.md: CLI equals GUI).
//!
//! Stable keys are the wire truth; every word a person reads comes from
//! the catalog at print time (docs/UX.md 文案).

use khor_catalog::cli::USAGE;
use khor_catalog::{category, cli, msg, state};
use khor_node::{list, MsgBody, Node, SessionId};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Err(e) = run(&args) {
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

fn run(args: &[String]) -> Result<(), String> {
    let verb = args.first().map(String::as_str).unwrap_or("help");
    let rest = &args[args.len().min(1)..];
    match verb {
        "id" => {
            let n = node()?;
            println!("{}  {}", n.device_str(), n.name());
            Ok(())
        }
        "devices" => {
            let n = node()?;
            for d in n.devices()? {
                let here = if d.name == n.name() { cli::THIS_MACHINE } else { "" };
                let pin = if d.pinned { cli::PINNED_MARK } else { "" };
                println!("{}\t{}…{here}{pin}", d.name, &d.id[..16]);
            }
            Ok(())
        }
        "sessions" => {
            let mode = match rest {
                [] => list::Arrange::Recent,
                [flag, key] if flag == "--by" => list::Arrange::from_key(key).ok_or_else(|| {
                    let all: Vec<&str> =
                        list::Arrange::ALL.iter().map(|a| a.key()).collect();
                    msg::not_a_way_to_arrange(key, all.join(cli::NAME_SEPARATOR))
                })?,
                _ => return Err(USAGE.into()),
            };
            let n = node()?;
            // The heading is printed when the group changes, which is
            // also why the node returns rows already grouped: this loop
            // owns no comparison, it only notices the boundary.
            let mut current: Option<String> = None;
            for a in n.sessions_arranged(mode)? {
                if !a.group.is_empty() && current.as_deref() != Some(a.group.as_str()) {
                    println!("{}", cli::group_header(group_word(&a.group)));
                    current = Some(a.group.clone());
                }
                let v = a.view;
                let s = &v.session;
                let src = match &v.source {
                    // A fresh report reads like local truth; only age
                    // worth knowing gets printed (docs/SESSION.md 离线).
                    Some((name, age)) if *age >= 30_000 => {
                        format!("\t{name} {}", cli::unreachable_for(human_age(*age)))
                    }
                    _ => String::new(),
                };
                // The order already says which rows are pinned; the mark
                // says *why* they lead, which "on top" alone cannot.
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
            // A missing row is invisible; this is the one thing that
            // makes "khor cannot read this vendor any more" visible
            // instead of looking like a quiet machine.
            let behind = n.unreadable_sessions();
            if behind > 0 {
                println!("{}", cli::adaptor_behind(behind));
            }
            Ok(())
        }
        "tell" => {
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
        "send" => {
            let [machine, path] = rest else {
                return Err(USAGE.into());
            };
            let id = node()?.send(machine, std::path::Path::new(path))?;
            println!("{}", cli::summary_sent(&id.0));
            Ok(())
        }
        "accept" => {
            let [id] = rest else {
                return Err(USAGE.into());
            };
            let moved = rt()?.block_on(node()?.accept(&SessionId(id.clone())))?;
            println!("{}", cli::pulled(moved));
            Ok(())
        }
        "log" => {
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
        "run" => {
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
            // The child owns stdout; the id goes to stderr so scripts can
            // still capture the command's own output cleanly.
            eprintln!("session: {}", id.0);
            let code = n.run_ephemeral(&id, &cmd)?;
            std::process::exit(code);
        }
        "open" => {
            let mut tui = false;
            let mut detached = false;
            let mut title: Option<String> = None;
            let mut cmd: Vec<String> = Vec::new();
            let mut it = rest.iter();
            while let Some(a) = it.next() {
                match a.as_str() {
                    "--tui" if cmd.is_empty() => tui = true,
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
            let kind = if tui { khor_node::kind::TUI } else { khor_node::kind::SHELL };
            let title = title.unwrap_or_else(|| {
                std::path::Path::new(&cmd[0])
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| cmd[0].clone())
            });
            let size = tty_size().unwrap_or((80, 24));
            let id = n.open_persistent(kind, &title, &cmd, size)?;
            eprintln!("session: {}", id.0);
            if detached {
                println!("{}", id.0);
                return Ok(());
            }
            attach_verb(&n, &id)
        }
        "attach" => {
            let [sid] = rest else {
                return Err(USAGE.into());
            };
            attach_verb(&node()?, &SessionId(sid.clone()))
        }
        // Internal: the detached host process `open` spawns. Not in the
        // usage text on purpose — nobody types this.
        "_host" => {
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
            khor_node::host::host_main(
                Node::root_from_env(),
                SessionId(sid.clone()),
                size,
                cmd.to_vec(),
            )
        }
        "state" => {
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
                    let sid = std::env::var("KHOR_SESSION")
                        .map_err(|_| msg::WHICH_SESSION.to_string())?;
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
        "seen" => {
            let [id] = rest else {
                return Err(USAGE.into());
            };
            node()?.seen(&SessionId(id.clone()))
        }
        // Two verbs rather than one toggle: a toggle typed twice is a
        // no-op you cannot see, and scripts have no way to say "make
        // sure this is pinned". Both call the one node function the
        // app's button calls.
        verb @ ("pin" | "unpin") => {
            let on = verb == "pin";
            let n = node()?;
            match rest {
                [flag, machine] if flag == "-m" || flag == "--machine" => n.pin_device(machine, on),
                [id] => n.pin_session(&SessionId(id.clone()), on),
                _ => Err(USAGE.into()),
            }
        }
        "close" => {
            let [id] = rest else {
                return Err(USAGE.into());
            };
            node()?.close(&SessionId(id.clone()))
        }
        "serve" => {
            let n = node()?;
            eprintln!("{}", cli::serve_banner(n.name()));
            rt()?.block_on(n.serve())
        }
        // Two shapes, one verb: asking is the default because it is the
        // safe one, and installing has to be typed. `khor hooks` alone
        // is also how somebody checks what `install` just did — a write
        // nobody can verify afterwards is a write on trust.
        "hooks" => {
            let n = node()?;
            match rest {
                [] => {
                    let report = n.hooks_report()?;
                    println!("{}", cli::hooks_file(report.path.display()));
                    let mut incomplete = false;
                    for (event, state) in &report.events {
                        let said = match state {
                            khor_node::adaptor::claude::Installed::Here => {
                                cli::HOOK_HERE.to_owned()
                            }
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
                _ => Err(USAGE.into()),
            }
        }
        "invite" => {
            println!("{}", node()?.invite()?);
            Ok(())
        }
        "pair" => {
            let [ticket] = rest else {
                return Err(USAGE.into());
            };
            let name = rt()?.block_on(node()?.pair(ticket))?;
            println!("{}", cli::paired_ok(name));
            Ok(())
        }
        "sync" => {
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
        "help" | "--help" | "-h" => {
            println!("{USAGE}");
            Ok(())
        }
        other => Err(format!("{}\n{USAGE}", msg::unknown_verb(other))),
    }
}

/// Raw passthrough to a hosted session: keystrokes and size changes go
/// out framed, PTY bytes come back raw. Ctrl-\ detaches; the session
/// stays. One writer thread frames everything so ops never interleave.
#[cfg(unix)]
fn attach_verb(n: &Node, id: &SessionId) -> Result<(), String> {
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
fn attach_verb(_n: &Node, _id: &SessionId) -> Result<(), String> {
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
