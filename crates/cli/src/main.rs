//! The `khor` CLI: the library's first consumer. Every verb is one node
//! call — anything the GUI will do must be reachable from here first
//! (docs/KHOR.md: CLI equals GUI).
//!
//! State words print as their stable keys until the catalog lands; the
//! keys are the wire truth, the catalog is wording.

use khor_node::{MsgBody, Node, SessionId};

const USAGE: &str = "\
用法: khor <动词>
  id                    本机身份
  devices               网里的设备
  sessions              session 列表
  tell <机器> <话...>    给机器的窗口留言
  send <机器> <文件>     把文件给那台机器(摘要先到,对方许可后才传)
  accept <传输号>        许可并收下全量(在收文件的那台机器上跑)
  log <机器>            看那个窗口的消息
  run [--tui] [--title 名] -- <命令...>
                        临时 session:跟着这个终端活,全程网里可见
  open [-d] [--tui] [--title 名] [-- 命令]
                        持久 session:终端关了还在,能重连(默认接上;-d 只开)
  attach <session>      接上一个持久 session(Ctrl-\\ 断开)
  state <词|--hook>     (给进程钩子用)汇报状态;--hook 从标准输入读 Claude 事件
  seen <session>        标已读
  close <session>       关掉(对话删收下的文件;临时 session 停掉进程)
  serve                 常驻:应答别人、代跑本机动词、每 5 秒同步全网
  invite                出一张一次性配对票(要求 serve 在跑)
  pair <票>             凭票入网
  sync                  立即和全网各同步一趟";

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
        .map_err(|e| format!("起不了运行时: {e}"))
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
                let here = if d.name == n.name() { "(本机)" } else { "" };
                println!("{}\t{}…{here}", d.name, &d.id[..16]);
            }
            Ok(())
        }
        "sessions" => {
            let n = node()?;
            for v in n.sessions()? {
                let s = &v.session;
                let src = match &v.source {
                    // A fresh report reads like local truth; only age
                    // worth knowing gets printed (docs/SESSION.md 离线).
                    Some((name, age)) if *age >= 30_000 => {
                        format!("\t{name} {}没联系上", human_age(*age))
                    }
                    _ => String::new(),
                };
                println!("{}\t{}\t未读 {}\t{}{}", s.id.0, s.state.state.key(), s.unread, s.title, src);
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
            println!("摘要已发,等对方许可。传输号: {}", id.0);
            Ok(())
        }
        "accept" => {
            let [id] = rest else {
                return Err(USAGE.into());
            };
            let moved = rt()?.block_on(node()?.accept(&SessionId(id.clone())))?;
            println!("收下了,这一趟拉了 {moved} 字节");
            Ok(())
        }
        "log" => {
            let [machine] = rest else {
                return Err(USAGE.into());
            };
            let log = node()?.log(machine)?;
            if log.broken > 0 {
                eprintln!("有 {} 个块读不出来,这段对话缺了一截", log.broken);
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
                        .map_err(|e| format!("读不了钩子载荷: {e}"))?;
                    n.claude_hook(&payload)?;
                    Ok(())
                }
                [word] => {
                    let sid = std::env::var("KHOR_SESSION").map_err(|_| {
                        "不知道汇报给哪个 session:设 KHOR_SESSION 或用 --session".to_string()
                    })?;
                    let state = khor_node::State::try_from(word.clone())
                        .map_err(|_| format!("不是六词之一: {word}"))?;
                    n.report_state(&SessionId(sid), state)
                }
                [word, flag, sid] if flag == "--session" => {
                    let state = khor_node::State::try_from(word.clone())
                        .map_err(|_| format!("不是六词之一: {word}"))?;
                    n.report_state(&SessionId(sid.clone()), state)
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
        "close" => {
            let [id] = rest else {
                return Err(USAGE.into());
            };
            node()?.close(&SessionId(id.clone()))
        }
        "serve" => {
            let n = node()?;
            eprintln!("khor serve: {} 在听,配对与同步都归它", n.name());
            rt()?.block_on(n.serve())
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
            println!("配上了: {name}。它认识的机器现在都在你的设备表里");
            Ok(())
        }
        "sync" => {
            let outcomes = rt()?.block_on(node()?.sync_now())?;
            if outcomes.is_empty() {
                println!("网里只有本机,没得同步");
            }
            for (name, verdict) in outcomes {
                match verdict {
                    Ok(what) => println!("{name}: {what}"),
                    Err(e) => println!("{name}: 没同步上——{e}"),
                }
            }
            Ok(())
        }
        "help" | "--help" | "-h" => {
            println!("{USAGE}");
            Ok(())
        }
        other => Err(format!("不认识的动词: {other}\n{USAGE}")),
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
        .ok_or_else(|| format!("没有这个 session: {}", id.0))?;
    // isatty decides whether this is a terminal; the size is separate —
    // a pty freshly made by script(1) or a GUI reports 0×0, which is a
    // default-worthy size, not a disqualification.
    if unsafe { libc::isatty(0) } != 1 {
        return Err("attach 要在终端里跑".into());
    }
    let (cols, rows) = tty_size().unwrap_or((80, 24));
    let mut stream = connect(&dir, cols, rows)?;
    eprintln!("接上了: {}(Ctrl-\\ 断开,session 不会跟着走)", id.0);

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
        eprintln!("已断开,session 还在: khor attach {}", id.0);
    } else {
        // The stream ended on the host's side — say what the row says.
        match n.sessions().ok().and_then(|v| v.into_iter().find(|v| v.session.id == *id)) {
            Some(v) => match v.session.state.state.key() {
                "done" | "failed" | "idle" => {
                    eprintln!("session 收场了: {}", v.session.state.state.key())
                }
                w => eprintln!("连接断了,session 好像还在({w}): khor attach {}", id.0),
            },
            None => eprintln!("session 关了"),
        }
    }
    // The stdin thread may still be blocked on read(2); leaving is how
    // this process lets go of it.
    std::process::exit(0);
}

#[cfg(not(unix))]
fn attach_verb(_n: &Node, _id: &SessionId) -> Result<(), String> {
    Err("这一版还接不上(Windows 的 attach 挂账)".into())
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
            return Err("拿不到终端参数".into());
        }
        let saved = t;
        libc::cfmakeraw(&mut t);
        if libc::tcsetattr(0, libc::TCSANOW, &t) != 0 {
            return Err("终端切不进原始模式".into());
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

fn human_age(ms: u64) -> String {
    let s = ms / 1000;
    if s < 120 {
        format!("{s} 秒")
    } else if s < 7200 {
        format!("{} 分钟", s / 60)
    } else if s < 172_800 {
        format!("{} 小时", s / 3600)
    } else {
        format!("{} 天", s / 86_400)
    }
}

fn render(m: &khor_node::Message) -> String {
    if m.retracted {
        return "[已撤回]".into();
    }
    match &m.body {
        MsgBody::Text(t) => t.clone(),
        MsgBody::Files(fs) => fs
            .iter()
            .map(|f| format!("[文件 {} {}B]", f.name, f.size))
            .collect::<Vec<_>>()
            .join(" "),
        MsgBody::Unknown(k) => format!("[这一版还读不出这条:{k}]"),
    }
}
