//! The `khor` CLI: the library's first consumer. Every verb is one node
//! call — anything the GUI will do must be reachable from here first
//! (docs/KHOR.md: CLI 与 GUI 等价).
//!
//! State words print as their stable keys until the catalog lands; the
//! keys are the wire truth, the catalog is wording.

use khor_node::{MsgBody, Node, SessionId};

const USAGE: &str = "\
用法: khor <动词>
  id                    本机身份
  sessions              session 列表
  say <机器> <话...>    给机器的窗口留言
  log <机器>            看那个窗口的消息
  seen <session>        标已读
  close <session>       关掉(对话连历史与收下的文件一起删)";

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

fn run(args: &[String]) -> Result<(), String> {
    let verb = args.first().map(String::as_str).unwrap_or("help");
    let rest = &args[args.len().min(1)..];
    match verb {
        "id" => {
            let n = node()?;
            println!("{}  {}", n.device_str(), n.name());
            Ok(())
        }
        "sessions" => {
            let n = node()?;
            for s in n.sessions()? {
                println!("{}\t{}\t未读 {}\t{}", s.id.0, s.state.state.key(), s.unread, s.title);
            }
            Ok(())
        }
        "say" => {
            let [machine, text @ ..] = rest else {
                return Err(USAGE.into());
            };
            if text.is_empty() {
                return Err(USAGE.into());
            }
            let id = node()?.say(machine, &text.join(" "))?;
            println!("{id}");
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
        "help" | "--help" | "-h" => {
            println!("{USAGE}");
            Ok(())
        }
        other => Err(format!("不认识的动词: {other}\n{USAGE}")),
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
