//! The `khor` CLI: the library's first consumer. Every verb is one node
//! call — anything the GUI will do must be reachable from here first
//! (docs/KHOR.md: CLI equals GUI).
//!
//! Stable keys are the wire truth; every word a person reads comes from
//! the catalog at print time (docs/UX.md 文案).

// ── The print layer (SHELL.md 第五轮 第 4 条) ─────────────────────────
//
// **`println!` and `print!` are shadowed for the whole file**, so every
// line this CLI prints goes through [`out`]. Rust ignores SIGPIPE, and
// std's `println!` turns the broken-pipe error that leaves behind into
// a panic — `khor devices | head -1` printed `failed printing to stdout:
// Broken pipe` and exited 101. A reader that went away is not a fault:
// [`out`] leaves the way any Unix tool does when its reader closes.
// Textual scope: these must stay above every use in this file.
macro_rules! println {
    () => {
        out(format_args!("\n"))
    };
    ($($t:tt)*) => {
        out(format_args!("{}\n", format_args!($($t)*)))
    };
}

macro_rules! print {
    ($($t:tt)*) => {
        out(format_args!($($t)*))
    };
}

/// Writes to stdout; a closed reader ends the process the way SIGPIPE
/// ends `cat` (exit by that signal, nothing on the terminal). Any other
/// write error is std's panic, unchanged.
fn out(args: std::fmt::Arguments) {
    use std::io::Write;
    if let Err(e) = std::io::stdout().lock().write_fmt(args) {
        if e.kind() == std::io::ErrorKind::BrokenPipe {
            #[cfg(unix)]
            unsafe {
                libc::signal(libc::SIGPIPE, libc::SIG_DFL);
                libc::raise(libc::SIGPIPE);
            }
            std::process::exit(141);
        }
        panic!("failed printing to stdout: {e}");
    }
}

// **Aliased**, because this file already has a verb called `usage`
// (`khor usage`, what each machine spent). The catalog's is the help
// text; 批 D3 turned it from a const into a function, which is when the
// two names met.
use khor_catalog::cli::usage as usage_text;
use khor_catalog::{avatar, category, cli, msg, state};
use khor_node::{
    AvatarStyle, FaceShape, MsgBody, Node, PRESETS, SessionId, Tokens, UsageDay, Variant, list,
};

fn main() {
    // 语言, before the first word this process can print (docs/MORE.md
    // 语言). A stored choice wins, `auto` reads the environment, and
    // anything that is not zh* — 「read nothing at all」 included — is
    // English (`khor_node::config::apply_lang`).
    //
    // **No node is opened**: the preference is read straight off the
    // store, so this costs one file read and works on a machine whose
    // store refuses — which is exactly when khor most needs to be able
    // to say why.
    khor_node::config::apply_lang(&Node::root_from_env());
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
    /// How far it reaches, and therefore what an agent owes a person
    /// before typing it.
    reach: Reach,
}

/// What typing this word can cost, in the terms `docs/AGENT.md` uses to
/// decide what an agent may do unasked: 看 is free, 动作 is not.
///
/// # Why this is a field and not a list somewhere else
///
/// A list can be missing a row. A field cannot: a `Verb` is a struct
/// literal, and a struct literal short one field does not compile — so
/// the person adding a verb answers this at the moment they are already
/// looking at this line, and a verb that never decided cannot ship.
/// That matters more here than it looks, because of the shape this repo
/// keeps paying for: **a rung that cannot be reached does not become
/// "no value", it becomes the rung beside it.** A lookup that returns
/// "absent" for an unclassified verb would have to answer something,
/// and either answer is wrong — 免批 hands an agent a verb nobody
/// cleared, 待批 asks a person about a verb nobody meant to gate.
///
/// # The rule for `Read`, and it is strict on purpose
///
/// **A word is `Read` only when *every* spelling of it observes.** Five
/// verbs read one way and write another — `face` restyles when handed
/// flags, `agents`/`accounts`/`hooks` write on their `add`/`rm`/
/// `install` subwords, and `web` prints a link **with the key in it**,
/// which is the whole network in one line of stdout. All five are
/// `Act`. A finer rule (「read unless the first argument is one of
/// these」) would let three of them stay free, and it can be wrong in
/// the direction that costs: **one flag missing from that list is one
/// write that never gets asked about**. This one can only be wrong by
/// asking too often.
///
/// # What it does not catch
///
/// **That a `Read` verb stayed a read.** The compiler forces the
/// answer, not its truth: give `top` a `--kill` tomorrow and this field
/// still says `Read` while the gate above it waves the command through.
/// Nothing here can see that; the guard is that the rule above is
/// short enough to hold in your head while you edit a verb.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Reach {
    /// Observes and prints. Free under every mode (`docs/AGENT.md`:
    /// 看（设备状态、session 列表与内容、事件流）一律免批).
    Read,
    /// Changes something — this machine, another machine, the network,
    /// or a file. Goes through whatever mode the task is running under.
    Act,
    /// A calling convention, not a word anybody types. An agent that
    /// reaches for one of these has misread the CLI as its API; the
    /// verbs it wants are the ones the usage lists.
    Inner,
}

impl Reach {
    /// The spelling a gate outside this process matches on. Lowercase
    /// and one word, because the thing reading it is a `case` in
    /// somebody else's language, not a person.
    fn word(self) -> &'static str {
        match self {
            Reach::Read => "read",
            Reach::Act => "act",
            Reach::Inner => "inner",
        }
    }
}

/// Every verb khor answers, in the order the usage text lists them.
const VERBS: &[Verb] = &[
    // Prints with no arguments and **deletes this machine's identity**
    // with `--reset`, which is the most irreversible thing in this
    // table. The rule is per word, not per invocation (`face` below).
    Verb {
        word: "id",
        run: id,
        reach: Reach::Act,
    },
    // Reads with no arguments and restyles with any, so it is an act:
    // the rule is per word, not per invocation.
    //
    // **It was `face` until 2026-09-21**, and the rename is the user's
    // (领队 2026-09-21: 「头像动词改叫 `khor avatar`」). One word cannot
    // mean two things: 「脸」 is khor's *interface* everywhere else in
    // this tree — the desktop app, the browser face, `khor faces` — and
    // this verb is about a **picture**. `face` stays one version as an
    // alias so nobody's muscle memory breaks silently.
    Verb {
        word: "avatar",
        run: avatar_verb,
        reach: Reach::Act,
    },
    Verb {
        word: "face",
        run: avatar_verb,
        reach: Reach::Act,
    },
    Verb {
        word: "devices",
        run: devices,
        reach: Reach::Read,
    },
    Verb {
        word: "top",
        run: top,
        reach: Reach::Read,
    },
    Verb {
        word: "ls",
        run: ls,
        reach: Reach::Read,
    },
    // Lands a file on this machine's disk.
    Verb {
        word: "pull",
        run: pull,
        reach: Reach::Act,
    },
    // Reads a checkout on any machine; writes nothing there and nothing
    // here, which is why it is a read and not an act like `agents`.
    Verb {
        word: "git",
        run: git,
        reach: Reach::Read,
    },
    // Replaces a file on somebody's disk. The most irreversible read-
    // and-write pair in this table, and the mirror of `pull`.
    Verb {
        word: "put",
        run: put,
        reach: Reach::Act,
    },
    // Takes one off it. The only verb here that is irreversible on
    // every machine but this one, which is what its usage line says.
    Verb {
        word: "rm",
        run: rm,
        reach: Reach::Act,
    },
    // The three that make a name where there was none, or move one.
    // Acts for `rm`'s reason from the other end: each of them changes
    // what is on somebody's disk.
    Verb {
        word: "mkdir",
        run: mkdir,
        reach: Reach::Act,
    },
    Verb {
        word: "touch",
        run: touch,
        reach: Reach::Act,
    },
    Verb {
        word: "mv",
        run: mv,
        reach: Reach::Act,
    },
    // Starts a process on somebody's machine — their opener — so it is
    // an act for `code`'s reason, not a read because nothing is written.
    Verb {
        word: "launch",
        run: launch,
        reach: Reach::Act,
    },
    // Walks a directory on somebody's machine and reads the text files
    // in it. Nothing is written anywhere, here or there, so it is a
    // read — the same class as `ls` and `git status`.
    Verb {
        word: "search",
        run: search,
        reach: Reach::Read,
    },
    // Starts a process on somebody's machine and a pipe in front of
    // it. Reading is what the editor then does; opening it is an act.
    Verb {
        word: "code",
        run: code,
        reach: Reach::Act,
    },
    // Turns a document into words on this terminal. Reads only.
    Verb {
        word: "read",
        run: read,
        reach: Reach::Read,
    },
    // The wizard's completion list: a read-only fold over the vendors'
    // own records.
    Verb {
        word: "places",
        run: places,
        reach: Reach::Read,
    },
    // `ls` reads and the other four write the workspace table, so the
    // word is an act: the rule is per word, not per invocation (`face`
    // above).
    Verb {
        word: "ws",
        run: ws,
        reach: Reach::Act,
    },
    Verb {
        word: "borrow",
        run: borrow,
        reach: Reach::Act,
    },
    // `ls` reads and the other three write this machine's own tab
    // table, so the word is an act — the rule is per word, not per
    // invocation (`ws` above). **Nothing here goes out on the wire**:
    // the table is a file on this disk (`khor_node::tabs`), and
    // `open -m` only resolves a name against the device table.
    Verb {
        word: "tabs",
        run: tabs,
        reach: Reach::Act,
    },
    Verb {
        word: "usage",
        run: usage,
        reach: Reach::Read,
    },
    Verb {
        word: "sessions",
        run: sessions,
        reach: Reach::Read,
    },
    Verb {
        word: "tell",
        run: tell,
        reach: Reach::Act,
    },
    Verb {
        word: "send",
        run: send,
        reach: Reach::Act,
    },
    Verb {
        word: "accept",
        run: accept,
        reach: Reach::Act,
    },
    Verb {
        word: "log",
        run: log,
        reach: Reach::Read,
    },
    Verb {
        word: "run",
        run,
        reach: Reach::Act,
    },
    Verb {
        word: "open",
        run: open,
        reach: Reach::Act,
    },
    // Attaching is a keyboard on somebody's session, not a look at it.
    Verb {
        word: "attach",
        run: attach,
        reach: Reach::Act,
    },
    Verb {
        word: "takeover",
        run: takeover,
        reach: Reach::Act,
    },
    // `known`/`sessions` observe; `add`/`rm` write the registry.
    Verb {
        word: "agents",
        run: agents,
        reach: Reach::Act,
    },
    // Listing observes; `add` is handed an API key.
    Verb {
        word: "accounts",
        run: accounts,
        reach: Reach::Act,
    },
    Verb {
        word: "quota",
        run: quota,
        reach: Reach::Read,
    },
    Verb {
        word: "state",
        run: state,
        reach: Reach::Act,
    },
    Verb {
        word: "seen",
        run: seen,
        reach: Reach::Act,
    },
    Verb {
        word: "rename",
        run: rename,
        reach: Reach::Act,
    },
    Verb {
        word: "pin",
        run: pin,
        reach: Reach::Act,
    },
    Verb {
        word: "unpin",
        run: unpin,
        reach: Reach::Act,
    },
    Verb {
        word: "close",
        run: close,
        reach: Reach::Act,
    },
    Verb {
        word: "serve",
        run: serve,
        reach: Reach::Act,
    },
    // Prints a link with the key in it. Reading it aloud is handing
    // over the whole network, which is why printing is not a read.
    Verb {
        word: "web",
        run: web,
        reach: Reach::Act,
    },
    Verb {
        word: "gui",
        run: gui,
        reach: Reach::Act,
    },
    // Reads two files and one loopback port and prints. **A word away
    // from `face`, which restyles this machine's avatar** — the two
    // meanings of 「脸」 in this tree, and the avatar is the one wearing
    // the wrong word (`khor_node::face`'s note, and the ledger).
    Verb {
        word: "faces",
        run: faces,
        reach: Reach::Read,
    },
    Verb {
        word: "quit",
        run: quit,
        reach: Reach::Act,
    },
    // Status observes; `install`/`uninstall` write the rc files.
    Verb {
        word: "hooks",
        run: hooks,
        reach: Reach::Act,
    },
    // Reads twelve things and writes nothing — **until `--fix`**, which
    // chmods a key, rewrites claude's settings or starts a resident.
    // The rule is per word, not per invocation (`face` above).
    Verb {
        word: "doctor",
        run: doctor,
        reach: Reach::Act,
    },
    // `status` observes and the other four write the lock file, so the
    // word is an act: the rule is per word, not per invocation.
    Verb {
        word: "lock",
        run: lock,
        reach: Reach::Act,
    },
    // `get`/`list` observe and `set` writes the preferences file, so the
    // word is an act — the rule is per word, not per invocation (`face`
    // above).
    Verb {
        word: "config",
        run: config,
        reach: Reach::Act,
    },
    // Mints a pairing ticket: this one hands out network access.
    Verb {
        word: "invite",
        run: invite,
        reach: Reach::Act,
    },
    Verb {
        word: "pair",
        run: pair,
        reach: Reach::Act,
    },
    Verb {
        word: "adopt",
        run: adopt,
        reach: Reach::Act,
    },
    Verb {
        word: "sync",
        run: sync,
        reach: Reach::Act,
    },
    Verb {
        word: "mcp",
        run: mcp,
        reach: Reach::Act,
    },
    Verb {
        word: "version",
        run: version,
        reach: Reach::Read,
    },
    Verb {
        word: "root",
        run: root,
        reach: Reach::Read,
    },
    Verb {
        word: "forget",
        run: forget,
        reach: Reach::Act,
    },
    // Takes this machine out of the network — the half `forget` refuses
    // to do (`Node::leave`). An act by the same reading as `forget`: it
    // writes the network's table, and this one empties this machine's.
    Verb {
        word: "leave",
        run: leave,
        reach: Reach::Act,
    },
    // 暂停 / 恢复 (docs/MORE.md 暂停). Acts, like the two above: they
    // write what this machine does next, and a running serve reads that
    // file rather than being told (`crate::pause`).
    Verb {
        word: "pause",
        run: pause,
        reach: Reach::Act,
    },
    Verb {
        word: "resume",
        run: resume,
        reach: Reach::Act,
    },
    Verb {
        word: "watch",
        run: watch,
        reach: Reach::Act,
    },
    Verb {
        word: "--version",
        run: version,
        reach: Reach::Read,
    },
    Verb {
        word: "-V",
        run: version,
        reach: Reach::Read,
    },
    Verb {
        word: "_host",
        run: host,
        reach: Reach::Inner,
    },
    Verb {
        word: "_ghost",
        run: ghost,
        reach: Reach::Inner,
    },
    Verb {
        word: "_cagent",
        run: cagent,
        reach: Reach::Inner,
    },
    Verb {
        word: "_codexagent",
        run: codexagent,
        reach: Reach::Inner,
    },
    Verb {
        word: "_verbs",
        run: verbs,
        reach: Reach::Inner,
    },
    Verb {
        word: "help",
        run: help,
        reach: Reach::Read,
    },
    Verb {
        word: "--help",
        run: help,
        reach: Reach::Read,
    },
    Verb {
        word: "-h",
        run: help,
        reach: Reach::Read,
    },
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
    (
        "_host",
        "internal: the host process `open` spawns, whose arguments are a calling convention",
    ),
    (
        "_ghost",
        "internal: the GUI-session host `open --gui` spawns; same convention, ACP instead of a PTY",
    ),
    (
        "_cagent",
        "internal: khor's own claude ACP shim; the agent a claude GUI session runs (`cagent` module head)",
    ),
    (
        "_codexagent",
        "internal: khor's own codex ACP shim; the agent a codex GUI session runs (`codexagent` module head)",
    ),
    (
        "_verbs",
        "internal: the verb table as JSON, for the gate that decides what an agent may type (`verbs`)",
    ),
    (
        "root",
        "internal: which store on this machine is khor's own, for `scripts/install.sh` — the freshly \
         unpacked binary is the authority on that and the installer asks it rather than reproducing \
         the rule in sh (`khor_root` in scripts/onset-lib.sh is what reproducing it costs). Not a \
         verb a person types: the answer is a path they cannot do anything with, and the one thing \
         they might want it for (which home am I on) is `khor id`.",
    ),
    (
        "face",
        "the old spelling of `avatar`, kept one version as an alias (2026-09-21). Out of the usage \
         on purpose: the usage is what khor teaches, and teaching both spellings of one verb is how \
         a rename never finishes. In this tree 「face」 means khor's interfaces — the desktop app \
         and the web face, which is what `khor faces` lists — so the word had to stop meaning a \
         picture as well.",
    ),
    (
        "help",
        "prints the usage text; a list that lists itself teaches nobody anything",
    ),
    ("--help", "the spelling people try before reading anything"),
    ("-h", "the short spelling of the same"),
    (
        "--version",
        "the spelling people try before finding the verb",
    ),
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
        None => Err(format!("{}\n{}", msg::unknown_verb(word), usage_text())),
    }
}

/// Which store on this machine is khor's — the one question the
/// install scripts must not answer for themselves.
///
/// **They used to.** `khor_root` in `scripts/onset-lib.sh` reproduced
/// the rule in sh, and the day the `owner` file grew a second line the
/// sh half read the whole file as a hostname, decided the home belonged
/// to somebody else, and sent onset to a directory that did not exist —
/// measured 2026-09-11 on turing, which lost its resident for the few
/// minutes it took to notice. **A rule in two languages is a rule that
/// will disagree with itself**, and that is this repo's own judgment
/// about mirrors with nothing holding them together.
///
/// So the sh half asks this instead, and only falls back to reading the
/// file when there is no khor to ask (a first install, an uninstall
/// after the binary is gone). Prints nothing else: it is consumed by
/// `$(…)`.
///
/// No node is opened. Answering 「which directory」 must work on a store
/// this machine may not be allowed to open — the uninstall path needs
/// the answer precisely when the claim might refuse.
fn root(_rest: &[String]) -> Result<(), String> {
    println!("{}", Node::root_from_env().display());
    Ok(())
}

/// `khor id` — this machine's key and name — and `khor id --reset <名字>`,
/// which throws both away (docs/MORE.md 重置身份).
///
/// **The reset is the same thing as replacing the computer.** Every
/// paired machine goes on knowing the old id and will not know the new
/// one, so this side stops being reachable and pairing has to be done
/// again from a machine that still has a table. Nothing is sent to
/// anybody — there is no longer a key to sign it with.
///
/// The machine's own name is the confirmation, and the check lives in
/// the node (`lock::reset_identity`): a flag here and a dialog over
/// there would be two rules about one irreversible act.
fn id(rest: &[String]) -> Result<(), String> {
    match rest {
        [] => {
            let n = node()?;
            println!("{}  {}", n.device_str(), n.name());
            Ok(())
        }
        [flag, typed] if flag == "--reset" => {
            let root = Node::root_from_env();
            let name = node()?.name().to_owned();
            khor_node::lock::reset_identity(&root, typed, &name)?;
            println!("{}", cli::identity_reset());
            Ok(())
        }
        _ => Err(usage_text().into()),
    }
}

/// `khor lock status|on|off|pin|timeout <分钟>` (docs/MORE.md 安全).
///
/// **It locks the screens, never this terminal.** Behind `khor` there is
/// already somebody logged into this machine; a prompt here would be
/// theatre in front of a shell that can read `.khor/` directly.
///
/// `pin` reads one line from **standard input** rather than taking an
/// argument: an argument lands in shell history, in `ps`, and in every
/// agent transcript that ran the command. An empty line clears the PIN,
/// which turns the lock off with it.
fn lock(rest: &[String]) -> Result<(), String> {
    let root = Node::root_from_env();
    let say = |s: khor_node::lock::LockState| {
        println!(
            "{}\t{}\t{}",
            if s.on {
                cli::lock_on()
            } else {
                cli::lock_off()
            },
            if s.has_pin {
                cli::lock_pin_set()
            } else {
                cli::lock_pin_none()
            },
            if s.timeout_min == 0 {
                cli::lock_idle_never().to_owned()
            } else {
                cli::lock_idle_after(s.timeout_min)
            }
        );
    };
    match rest {
        [] | [_] if rest.first().map(String::as_str) == Some("status") || rest.is_empty() => {
            say(khor_node::lock::state(&root));
            Ok(())
        }
        [word] if word == "on" => {
            say(khor_node::lock::set_on(&root, true)?);
            Ok(())
        }
        [word] if word == "off" => {
            say(khor_node::lock::set_on(&root, false)?);
            Ok(())
        }
        [word] if word == "pin" => {
            let mut typed = String::new();
            std::io::BufRead::read_line(&mut std::io::stdin().lock(), &mut typed)
                .map_err(msg::cant_read_stdin)?;
            say(khor_node::lock::set_pin(
                &root,
                typed.trim_end_matches(['\n', '\r']),
            )?);
            Ok(())
        }
        [word, minutes] if word == "timeout" => {
            let m: u32 = minutes.parse().map_err(|_| usage_text().to_string())?;
            say(khor_node::lock::set_timeout(&root, m)?);
            Ok(())
        }
        _ => Err(usage_text().into()),
    }
}

/// `khor config get|set|list` (docs/MORE.md 设置).
///
/// **No arguments is `list`**, because the first question anybody has
/// about a settings command is 「有哪些」 — and because a bare verb that
/// printed usage would answer it with a page of syntax.
///
/// `list` prints **every** key and the value it currently has, never
/// only the ones somebody has touched: a listing of the file would be
/// empty on a fresh machine, which reads as the command being broken
/// rather than as nothing having been set.
///
/// Both refusals name what was wrong — an unknown key and a value
/// outside that key's ladder are different sentences, because they send
/// a person to two different places (`config::set`).
fn config(rest: &[String]) -> Result<(), String> {
    let root = Node::root_from_env();
    let list = || {
        for (k, v) in khor_node::config::list(&root) {
            println!("{}\t{}", k.name, v);
        }
    };
    match rest {
        [] => {
            list();
            Ok(())
        }
        [word] if word == "list" => {
            list();
            Ok(())
        }
        [word, name] if word == "get" => {
            println!("{}", khor_node::config::get(&root, name)?);
            Ok(())
        }
        // Echoes the value that took, the way `lock` echoes the state it
        // reached: 「存下了」 with nothing after it is a claim nobody can
        // check without a second command.
        [word, name, value] if word == "set" => {
            println!("{}", khor_node::config::set(&root, name, value)?);
            Ok(())
        }
        _ => Err(usage_text().into()),
    }
}

/// `khor tabs`: the browsing tabs this screen has open
/// (docs/SHELL.md 第 2 条:「CLI 至少要能列、能开、能关」).
///
/// # Why this verb exists at all
///
/// The tabs were the webview's own storage until 2026-09-20, which met
/// every part of that ruling except this one — and **CLI 与 GUI 等价**
/// is one of this repo's three standing rules, not a nicety. A list
/// only the window can read is a list nothing else can check, fix or
/// script against.
///
/// # The shape follows `ws`, not `config`
///
/// Four sub-words over a table this machine keeps, with `ls` the one
/// that only reads — the same grammar the workspace table has, so
/// nobody has to learn a second one. **`-m <机器>` names the exit
/// machine by name and becomes its id here**, the one place this verb's
/// grammar meets the table's key, exactly as `ws rm` does.
///
/// # 墓碑：「close 只收 id」，2026-09-21 推翻
///
/// The first version printed and took **only** the id, and the argument
/// was that a row number changes under a person between two commands —
/// the list is a live thing and the GUI is sitting on the other end of
/// it. That much is still true. What it missed is that an id here reads
/// `t1a0c0593bd7-d792-0`, and **nobody retypes that** (验收方,
/// 2026-09-21, reading a real `khor tabs ls`); the usage line even
/// called it 「标签号」, a word for something you can say out loud.
///
/// So `ls` prints both and `close`/`focus` take either
/// ([`tab_named`]). The moving number is the price of a verb a hand can
/// use, and it is paid where it belongs: a script holds the id, and the
/// listing that hands out a number is the same listing it came from.
fn tabs(rest: &[String]) -> Result<(), String> {
    let root = Node::root_from_env();
    // The retired 「置顶的网页 / 置顶的目录」 rows become sidebar pins the
    // first time anything looks at the tabs (`Node::migrate_place_pins`,
    // once per row). A failure here is not this verb's: the tabs are read
    // either way.
    if let Ok(n) = node() {
        let _ = n.migrate_place_pins();
    }
    // One kind today (`page`), and the store is keyed by it so the
    // editor tab can land beside rather than inside
    // (`khor_node::tabs`). Not a flag: a word nobody can use yet is a
    // thing to explain in a usage line that is already long.
    const KIND: &str = "page";
    let list = || {
        let open = khor_node::tabs::list(&root, KIND);
        for (n, t) in (1..).zip(&open.tabs) {
            // **`in-use`, not `pinned-mark`.** The first draft borrowed
            // the workspace table's 「(置顶)」 and a real `khor tabs ls`
            // read 「置顶 https://…」, which says a person put that tab
            // first — a decision nobody made. What this column has to
            // say is 「this is the one you are looking at」, and khor
            // already has that word: `khor face` prints it for the
            // palette in use (catalog `in-use`). One word, one meaning,
            // no synonym minted.
            let front = if t.id == open.active {
                cli::in_use()
            } else {
                ""
            };
            // **The pin rides the same first column**, after the front
            // mark, and it is the table's own 「(置顶)」 this time — here
            // it does mean a person put the tab first (docs/SHELL.md
            // 第二轮). Same column so the number and the id stay where
            // scripts already read them.
            let pinned = if t.pinned { cli::pinned_mark() } else { "" };
            let front = format!("{front}{pinned}");
            // **A number first, and it is the one a person types.**
            // 「标签号」 in the usage line meant `t1a0c…-b391-0` for one
            // build, and nobody is retyping that (验收方, 2026-09-21).
            // The id is still printed, because it is what a script
            // should hold: a number is a row's *place in this listing*
            // and moves when a tab closes, an id never does. Both are
            // accepted below, and they cannot be confused — an id never
            // parses as a number.
            println!("{front}\t{n}\t{}\t{}\t{}", t.id, t.title, t.url);
        }
    };
    match rest {
        [] => {
            list();
            Ok(())
        }
        [word] if word == "ls" => {
            list();
            Ok(())
        }
        // Echoes the id it minted, for `lock`'s reason: 「开好了」 with
        // nothing after it is a claim that needs a second command to
        // check, and this one is what `close` takes.
        [word, flag, machine, url] if word == "open" && (flag == "-m" || flag == "--machine") => {
            let id = khor_node::tabs::open(&root, KIND, &node()?.machine_id(machine)?, url)?;
            println!("{id}");
            Ok(())
        }
        [word, url] if word == "open" => {
            let id = khor_node::tabs::open(&root, KIND, &node()?.machine_id("")?, url)?;
            println!("{id}");
            Ok(())
        }
        [word, which] if word == "close" => {
            khor_node::tabs::close(&root, KIND, &tab_named(&root, KIND, which)?)
        }
        [word, which] if word == "focus" => {
            khor_node::tabs::focus(&root, KIND, &tab_named(&root, KIND, which)?)
        }
        // 置顶 / 取消置顶, the window's row menu said in the terminal
        // (CLI 与 GUI 等价). A pinned tab refuses `close` until it is
        // unpinned — the node says so, for both callers.
        [word, which] if word == "pin" || word == "unpin" => {
            khor_node::tabs::pin(&root, KIND, &tab_named(&root, KIND, which)?, word == "pin")
        }
        _ => Err(usage_text().into()),
    }
}

/// Which tab a person meant: **the number `ls` printed, or the id**.
///
/// A number is that row's place in the listing and is what a hand
/// types; an id is what a script holds, because it does not move when a
/// neighbour closes. They cannot be confused — an id begins with `t`
/// and never parses as a number — so both can be taken without a flag
/// to say which.
///
/// A number outside the listing is refused **by the same sentence a
/// missing id gets**: both mean 「that is not one of the tabs that are
/// open」, and two sentences for one fact would be two things to read.
fn tab_named(root: &std::path::Path, kind: &str, which: &str) -> Result<String, String> {
    let Ok(n) = which.parse::<usize>() else {
        // Not a number: it is an id, and the node refuses an unknown one
        // by name — handing it straight through keeps that one sentence.
        return Ok(which.to_string());
    };
    khor_node::tabs::list(root, kind)
        .tabs
        .into_iter()
        .nth(n.wrapping_sub(1))
        .map(|t| t.id)
        .filter(|_| n >= 1)
        .ok_or_else(|| khor_catalog::msg::no_such_tab(which))
}

/// The verb table, for whatever is about to gate an agent that types
/// khor commands (`docs/AGENT.md` 权限). One JSON object per verb: the
/// word, and how far it reaches.
///
/// # Why khor prints this rather than the gate keeping its own copy
///
/// The gate for the in-app agent runs **outside this process** — it
/// decides whether a command may run before any code in here is
/// reached — so it needs the split in a form it can read. A list
/// compiled into that gate would be a second place to forget a verb,
/// and this repo has paid for that shape twice already: the usage text
/// needed a gate of its own to stay honest, and `mcp.rs` hand-picks six
/// tools out of thirty verbs, so an agent on that path cannot see `top`
/// or `usage` at all. Asking the binary it is about to drive means a
/// verb added to `VERBS` is gated correctly the next time the agent
/// starts, with nothing else edited anywhere.
///
/// **`Inner` rows are printed too, and on purpose.** A gate that never
/// hears about `_host` has to decide what an unknown word is, and the
/// answer it would need is the one this file exists to avoid guessing.
///
/// The JSON is written out rather than serialized because the only
/// values in it are the words from `VERBS`; a test holds those to
/// characters JSON does not escape, which is what makes that safe
/// rather than lucky.
fn verbs(_rest: &[String]) -> Result<(), String> {
    let rows: Vec<String> = VERBS
        .iter()
        .map(|v| {
            format!(
                "{{\"word\":\"{}\",\"reach\":\"{}\"}}",
                v.word,
                v.reach.word()
            )
        })
        .collect();
    println!("[{}]", rows.join(","));
    Ok(())
}

/// This machine's face. It prints what the machine is wearing either way
/// — with no flags because "what am I" is the question somebody runs this
/// to ask, and after a change because a verb that says nothing leaves
/// 做了但没变化 and 失败 looking alike (docs/UX.md 状态呈现).
fn avatar_verb(rest: &[String]) -> Result<(), String> {
    let mut colors: Option<Vec<String>> = None;
    let mut variant: Option<String> = None;
    let mut shape: Option<String> = None;
    // **Which machine, and it is a bare word rather than a flag.** 「CLI
    // 等价：`khor face` 要能指定机器」 (docs/MORE.md 第二轮). Every
    // other verb in this table that acts on a machine takes it that way
    // (`khor top [机器]`, `khor ls <机器>`), and a flag here would be
    // the one place a person has to remember something different.
    let mut machine: Option<String> = None;
    let mut it = rest.iter();
    while let Some(flag) = it.next() {
        let mut value = || it.next().ok_or_else(|| usage_text().to_string()).cloned();
        match flag.as_str() {
            // A factory set by name. Resolved here rather than inside
            // `restyle`, so that the library has exactly one way to be
            // handed a palette: five colors.
            "--palette" => {
                let id = value()?;
                let p = khor_node::preset(&id).ok_or_else(|| {
                    let all: Vec<&str> = PRESETS.iter().map(|p| p.id).collect();
                    msg::no_such_palette(&id, all.join(cli::name_separator()))
                })?;
                colors = Some(p.colors.iter().map(|c| c.to_string()).collect());
            }
            "--colors" => {
                colors = Some(value()?.split(',').map(|c| c.trim().to_owned()).collect());
            }
            "--variant" => variant = Some(value()?),
            "--shape" => shape = Some(value()?),
            // The first bare word is the machine; a second one is a
            // typo, not a second machine, so it is refused rather than
            // quietly ignored.
            name if !name.starts_with("--") && machine.is_none() => {
                machine = Some(name.to_owned());
            }
            _ => return Err(usage_text().into()),
        }
    }
    let n = node()?;
    let who = machine.as_deref();
    let style = match (&colors, &variant, &shape) {
        (None, None, None) => match who {
            None => n.avatar_style(),
            // Refused by name for a machine khor does not have, rather
            // than falling through to this one — a face printed for the
            // wrong machine reads exactly like a face printed for the
            // right one.
            Some(name) => n.avatar_style_of(&n.device_by_name(name)?),
        },
        _ => n.restyle_machine(who, colors.as_deref(), variant.as_deref(), shape.as_deref())?,
    };
    print_face(&style);
    Ok(())
}

/// Which of khor's interfaces are up on this machine (docs/MORE.md
/// 头像页).
///
/// **It opens no node**, the reason `about` does not: both facts are a
/// file and a loopback port, so this answers on a machine whose store
/// refuses — which is one of the moments somebody asks it.
///
/// **Every face, including the ones that are not up.** A listing that
/// only named the running ones would leave somebody looking for the web
/// face with nothing on screen saying it exists.
fn faces(rest: &[String]) -> Result<(), String> {
    if !rest.is_empty() {
        return Err(usage_text().into());
    }
    for f in khor_node::face::faces(&Node::root_from_env()) {
        let state = if f.up {
            cli::face_up()
        } else {
            cli::face_down()
        };
        match (&f.at, f.up) {
            (Some(at), true) => println!(
                "{}  {}  {}",
                khor_node::face::face_name(&f.key),
                state,
                cli::face_at(at)
            ),
            _ => println!("{}  {}", khor_node::face::face_name(&f.key), state),
        }
    }
    Ok(())
}

/// A far machine's directory. Directories wear a trailing slash and no
/// size — the two facts a browse runs on; the order is the node's
/// (directories first, names case-folded), printed as it came.
fn ls(rest: &[String]) -> Result<(), String> {
    let (machine, path) = match rest {
        [machine] => (machine, ""),
        [machine, path] => (machine, path.as_str()),
        _ => return Err(usage_text().into()),
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
        eprintln!(
            "{}",
            cli::dir_truncated(khor_node::files::MOST_ENTRIES as u64)
        );
    }
    Ok(())
}

/// Takes a file off a machine by path. The landing prints because the
/// default (the current directory) was chosen silently — a file that
/// arrived somewhere unsaid is a file someone will go looking for.
fn pull(rest: &[String]) -> Result<(), String> {
    let (machine, path, dir) = match rest {
        [machine, path] => (
            machine,
            path,
            std::env::current_dir().map_err(|e| e.to_string())?,
        ),
        [machine, path, dir] => (machine, path, std::path::PathBuf::from(dir)),
        _ => return Err(usage_text().into()),
    };
    let (moved, dest) = rt()?.block_on(node()?.pull_path(machine, path, &dir))?;
    println!("{}", cli::pulled_to(moved, dest.display()));
    Ok(())
}

/// Where sessions have run on a machine — 常去的地方, the list the new
/// session wizard completes from (`khor_node::Node::places_on`).
///
/// **Added on 2026-09-12, and it was missing from the day the GUI had
/// it.** docs/KHOR.md's third foundation is that the two faces are
/// equivalent, and this was one of two places that was not true; the
/// other is below. Both were on the ledger from 工作区批1.
fn places(rest: &[String]) -> Result<(), String> {
    let machine = match rest {
        [] => "",
        [machine] => machine.as_str(),
        _ => return Err(usage_text().into()),
    };
    for (path, shown, sessions, _) in rt()?.block_on(node()?.places_on(machine))? {
        // The machine's own spelling first, the real path after: `~` is
        // what a person recognises and the absolute one is what they
        // paste somewhere else.
        println!("{shown}\t{sessions}\t{path}");
    }
    Ok(())
}

/// The workspaces — the directories a person declared they work in
/// (`khor_node::Node::workspaces`), and the four verbs that change the
/// list.
///
/// **Nothing here asks the network for the list.** A workspace is a
/// decision and decisions replicate, so `ls` reads this machine's own
/// copy of the table; the one call that does go out is `add`, and only
/// to ask whether that path over there is a directory.
///
/// `-m` is the flag `pin`/`rename` already use for the same
/// distinction — one verb, several subjects, and the subject is named
/// the same way everywhere rather than each verb inventing its own.
fn ws(rest: &[String]) -> Result<(), String> {
    let (verb, rest) = rest
        .split_first()
        .ok_or_else(|| String::from(usage_text()))?;
    // `-m <机器> <目录>` or a bare `<目录>`, which is this machine.
    let target = |rest: &[String]| -> Result<(String, String), String> {
        match rest {
            [flag, machine, dir] if flag == "-m" || flag == "--machine" => {
                Ok((machine.clone(), here(dir)?))
            }
            [dir] => Ok((String::new(), here(dir)?)),
            _ => Err(usage_text().into()),
        }
    };
    let n = node()?;
    match (verb.as_str(), rest) {
        ("ls", []) => {
            for w in n.workspaces()? {
                let pin = if w.pinned { cli::pinned_mark() } else { "" };
                // **The id stands in where the machine has no name** —
                // it has left the device table, and this column has to
                // stay something a person can paste back into `ws rm`
                // (`Node::machine_id`). The screen leaves it blank
                // instead, which is the difference between a row you
                // read and a row you type at.
                let at = if w.machine.is_empty() {
                    &w.device
                } else {
                    &w.machine
                };
                // The name first because it is what a person recognises,
                // then where it actually is — one row, the same facts
                // the landing's row wears.
                println!("{pin}\t{}\t{at}:{}", w.name, w.path);
            }
            Ok(())
        }
        ("add", rest) => {
            let (machine, dir) = target(rest)?;
            rt()?.block_on(n.ws_add(&machine, &dir))
        }
        // The three writes are keyed by the machine's **id**, not its
        // name (`Node::ws_write` says why), so a typed name becomes one
        // here — the one place the CLI's grammar meets the table's key.
        ("rm", rest) => {
            let (machine, dir) = target(rest)?;
            n.ws_rm(&n.machine_id(&machine)?, &dir)
        }
        ("pin", rest) => {
            let (machine, dir) = target(rest)?;
            n.ws_pin(&n.machine_id(&machine)?, &dir, true)
        }
        ("unpin", rest) => {
            let (machine, dir) = target(rest)?;
            n.ws_pin(&n.machine_id(&machine)?, &dir, false)
        }
        _ => Err(usage_text().into()),
    }
}

/// Takes a path off a machine's disk (工作区批④).
///
/// **This machine's goes to the recycle bin; every other machine's is
/// deleted**, and the usage line says so rather than leaving it to be
/// discovered. A bin on a machine nobody is sitting at is not somewhere
/// anything can be got back from (`khor_node::Link::remove_path`).
///
/// No `-f`, no `-r`: a directory goes whole because that is what 删除
/// means to the person asking, and there is no second spelling of this
/// verb for it to be forced past. What there is instead is one refusal
/// when the path is not there, which is the thing a flag would have
/// silenced.
fn rm(rest: &[String]) -> Result<(), String> {
    let (machine, path) = match rest {
        [flag, machine, path] if flag == "-m" || flag == "--machine" => {
            (machine.clone(), here(path)?)
        }
        [path] => (String::new(), here(path)?),
        _ => return Err(usage_text().into()),
    };
    rt()?.block_on(node()?.remove_path(&machine, &path))
}

/// Makes one directory on a machine (工作区第五轮).
///
/// **No `-p`.** The person typing this is looking at a tree with the
/// parent already on it, and a chain of directories nobody asked for is
/// what a silent `-p` would leave behind after one mistyped name.
fn mkdir(rest: &[String]) -> Result<(), String> {
    let (machine, path) = one_path(rest)?;
    rt()?.block_on(node()?.mkdir(&machine, &path))
}

/// Makes one empty file on a machine (工作区第五轮).
///
/// **Not unix `touch`.** That one updates the mtime of a file already
/// there; this one refuses it, because the caller is 新建文件 and a verb
/// that quietly did nothing would look exactly like one that worked.
/// Nothing khor does needs to stamp a date onto a file, so there is no
/// second meaning to lose.
fn touch(rest: &[String]) -> Result<(), String> {
    let (machine, path) = one_path(rest)?;
    rt()?.block_on(node()?.touch(&machine, &path))
}

/// Moves a name on a machine — rename, move, both (工作区第五轮).
///
/// **No `-f`**, `rm`'s rule: the second spelling of a verb is where a
/// refusal goes to die. A target already there is refused, and the file
/// wearing that name is not touched.
fn mv(rest: &[String]) -> Result<(), String> {
    let (machine, from, to) = match rest {
        [flag, machine, from, to] if flag == "-m" || flag == "--machine" => {
            (machine.clone(), here(from)?, here(to)?)
        }
        [from, to] => (String::new(), here(from)?, here(to)?),
        _ => return Err(usage_text().into()),
    };
    rt()?.block_on(node()?.rename_path(&machine, &from, &to))
}

/// Hands a path or a page to a machine's own opener (工作区第五轮).
///
/// **It opens over there, not here.** That is the whole verb: a file
/// named in a conversation lives on the machine the conversation ran on,
/// and the face looking at it can be a browser on a phone.
///
/// The target is **not** resolved against this terminal's cwd the way
/// every other path verb's is ([`here`]): `./notes.md` here would become
/// this machine's path and then name nothing over there — and unlike a
/// directory that simply is not found, an opener handed a wrong absolute
/// path can succeed at opening the wrong thing.
fn launch(rest: &[String]) -> Result<(), String> {
    let (machine, target) = match rest {
        [flag, machine, target] if flag == "-m" || flag == "--machine" => {
            (machine.clone(), target.clone())
        }
        [target] => (String::new(), target.clone()),
        _ => return Err(usage_text().into()),
    };
    rt()?.block_on(node()?.launch(&machine, &target))
}

/// Looks for words under one directory on a machine (工作区第六轮
/// 第 47 条).
///
/// **Two kinds of hit and one order**, printed exactly as the face
/// draws them: every name first, then every line as `path:line: text`.
/// The pair is what grep and `find` are separately, and the reason they
/// are one verb here is that a person hunting a file does not know yet
/// which of the two they are doing.
///
/// A name hit prints its path alone — there is no line to name — so a
/// bare path and a `path:line:` row are the two shapes, and nothing has
/// to be parsed to tell them apart.
///
/// Prints nothing when nothing matched. **`more` is said out loud**:
/// a list that was cut and a list that is short look identical, which
/// is the rule every capped answer in khor follows.
fn search(rest: &[String]) -> Result<(), String> {
    let (machine, dir, query) = match rest {
        [flag, machine, dir, query] if flag == "-m" || flag == "--machine" => {
            (machine.clone(), here(dir)?, query.clone())
        }
        [dir, query] => (String::new(), here(dir)?, query.clone()),
        _ => return Err(usage_text().into()),
    };
    let found = rt()?.block_on(node()?.search(&machine, &dir, &query))?;
    for path in &found.names {
        println!("{path}");
    }
    for hit in &found.lines {
        println!("{}:{}: {}", hit.path, hit.line, hit.text);
    }
    if found.more {
        eprintln!("{}", cli::search_more());
    }
    Ok(())
}

/// `-m <机器> <路径>` or a bare `<路径>`, which is this machine — the
/// shape `rm` spells inline and the three verbs above share.
fn one_path(rest: &[String]) -> Result<(String, String), String> {
    match rest {
        [flag, machine, path] if flag == "-m" || flag == "--machine" => {
            Ok((machine.clone(), here(path)?))
        }
        [path] => Ok((String::new(), here(path)?)),
        _ => Err(usage_text().into()),
    }
}

/// A directory as typed, made absolute **against this terminal's own
/// cwd**.
///
/// `khor ws add .` is the shape a person actually types, and `.` names
/// nothing on any other machine — so only a local path is resolved
/// here, and a path meant for another machine must already be absolute
/// (`Node::ws_add` refuses it otherwise, in the same words `Ls` uses).
fn here(dir: &str) -> Result<String, String> {
    let p = std::path::Path::new(dir);
    if p.is_absolute() {
        return Ok(dir.to_owned());
    }
    match std::fs::canonicalize(p) {
        Ok(abs) => Ok(abs.display().to_string()),
        // Not this machine's path, or not there: handed on as typed so
        // the refusal comes from the one place that knows which.
        Err(_) => Ok(dir.to_owned()),
    }
}

/// Prints a document's words: markdown, or a Word / Excel / PowerPoint
/// file turned into markdown on the way (`khor_node::preview`).
///
/// **The same pipeline the workspace paints**, which is the point:
/// every format ends as markdown, the app renders that markdown to
/// HTML and this prints it. A `.docx` the app can show and `khor`
/// cannot would be the third foundation broken in the place it is
/// easiest to break it (docs/KHOR.md, CLI 与 GUI 等价).
///
/// **Markdown and not the HTML**, because a terminal reads markdown and
/// a terminal does not read tags — the render is the screen's half of
/// the pipeline, not the document's.
fn read(rest: &[String]) -> Result<(), String> {
    let [machine, path] = rest else {
        return Err(usage_text().into());
    };
    // The same ceiling the app looks through, and **the same
    // function** — the constant lived in two places for a day, and the
    // choice *between* the two constants would have been the same
    // mistake one size up (`khor_node::preview::cap`).
    let look = khor_node::preview::cap(path);
    let (total, _, bytes, cut) = rt()?.block_on(node()?.read_all(machine, path, look))?;
    let md = khor_node::preview::markdown(path, &bytes)?;
    if md.trim().is_empty() {
        // A real answer, not a failure: a blank template parses
        // perfectly and holds nothing. Said on stderr so a pipe gets
        // the empty document it asked for.
        eprintln!("{}", cli::read_empty());
        return Ok(());
    }
    print!("{md}");
    if cut {
        eprintln!("{}", cli::read_cut(total, look));
    }
    Ok(())
}

/// Opens VS Code's web face on a machine's workspace and prints where
/// to point a browser (`khor_node::code`).
///
/// **The link carries the token, and that is said rather than hidden.**
/// The far server refuses every request without it; a person copying
/// this line into a chat is handing over their editor, which is the
/// same warning `khor web` already carries about its own link.
///
/// **Not a session, so there is no row and no `close`.** An editor
/// cannot answer 你怎么样 with any of the six words (docs/UX.md), and a
/// row whose state never changes is a row nobody reads. `--stop` is how
/// it ends, and it is the only way — which is the price of not being a
/// session, paid here where it is spent.
fn code(rest: &[String]) -> Result<(), String> {
    let (stop, rest) = match rest {
        [flag, tail @ ..] if flag == "--stop" => (true, tail),
        _ => (false, rest),
    };
    let (machine, root) = match rest {
        [machine] => (machine, ""),
        [machine, root] => (machine, root.as_str()),
        _ => return Err(usage_text().into()),
    };
    let rt = rt()?;
    let n = node()?;
    if stop {
        rt.block_on(n.code_stop(machine))?;
        println!("{}", cli::code_stopped(machine));
        return Ok(());
    }
    // Asked first so a machine without an editor says so in one round
    // trip, rather than after ten minutes of a download that was never
    // going to happen.
    let editor = rt.block_on(n.code_here(machine))?;
    if editor.version.is_empty() {
        return Err(msg::no_vscode().into());
    }
    // **Said before the wait rather than after it.** This does not ask:
    // the CLI's precedent is `forget` and `takeover`, where typing the
    // verb is the confirmation. But a machine that has no server spends
    // minutes fetching one, and a person watching a silent terminal has
    // no way to tell that from a hang — nor that 640 MB is moving and a
    // licence is being accepted on that machine's behalf.
    if !editor.ready {
        println!("{}", cli::code_fetching());
    }
    let (addr, token) = rt.block_on(n.code_open(machine))?;
    // **The folder rides in the link, not in the launch.** One server
    // per machine opens whichever folder it is asked for, so a second
    // project is a second link and not a second copy of VS Code
    // (`khor_node::code`). No path means whatever it opened last.
    let folder = if root.is_empty() {
        String::new()
    } else {
        format!("&folder={}", enc(root))
    };
    println!(
        "{}",
        cli::code_at(
            format_args!("http://{addr}/?tkn={token}{folder}"),
            editor.version
        )
    );
    Ok(())
}

/// Percent-encodes a path for a query string. **Hand-rolled and tiny on
/// purpose**: khor has no URL crate, the input is one filesystem path,
/// and the set that must be escaped in a query value is small and
/// fixed. Everything outside the unreserved set goes as `%XX`, which is
/// always correct and only ever uglier than necessary.
fn enc(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for b in path.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                out.push(*b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Writes a local file onto a machine, at a path chosen by the caller —
/// the mirror of `pull`, and what the file view's save does
/// (`khor_node::files::write_whole`).
///
/// **Read first, then write, and refuse if it moved in between.** The
/// digest of what is there now is what the write is pinned to, so a
/// file somebody else changed between those two moments is refused
/// whole rather than half-overwritten. That window is small and real;
/// what it buys is that the common case — two people with the same file
/// open — cannot end with one of them silently gone.
///
/// `--new` is the other claim: **there should be nothing at that path**.
/// It is a separate flag rather than a fallback because「create it」and
/// 「replace it」 are two different intentions, and a verb that quietly
/// did whichever the disk allowed would do the wrong one on the day it
/// mattered.
fn put(rest: &[String]) -> Result<(), String> {
    let mut fresh = false;
    let mut base: Option<String> = None;
    let mut rest = rest;
    loop {
        match rest {
            [flag, tail @ ..] if flag == "--new" => {
                fresh = true;
                rest = tail;
            }
            [flag, file, tail @ ..] if flag == "--base" => {
                base = Some(file.clone());
                rest = tail;
            }
            _ => break,
        }
    }
    // Two claims about the same path that cannot both be true: 「there
    // is nothing there」 and 「there is the copy I started from」.
    if fresh && base.is_some() {
        return Err(cli::put_new_and_base().into());
    }
    let [machine, src, dest] = rest else {
        return Err(usage_text().into());
    };
    let bytes = std::fs::read(src).map_err(|e| msg::cant_read(src, e))?;
    let rt = rt()?;
    let n = node()?;
    if let Some(base) = base {
        return put_over(&rt, &n, machine, dest, &bytes, &base);
    }
    let was = if fresh {
        String::new()
    } else {
        // The read this write is pinned to. Its refusal is the caller's
        // answer too — a path that cannot be read is not a path this
        // verb should be replacing.
        let (_, _, now) = rt.block_on(n.read_head(machine, dest))?;
        khor_node::files::digest(&now)
    };
    let (_, digest) = rt.block_on(n.write_path(machine, dest, &was, &bytes))?;
    println!("{}", cli::put_landed(dest, &digest[..8]));
    Ok(())
}

/// `put --base`: the same write, but pinned to the copy the caller
/// *started from* rather than to whatever is there this second — and
/// when those two differ, merged rather than refused (工作区批7).
///
/// **This is the hole plain `put` has.** Without a base, the digest it
/// pins to is read microseconds before the write, so a change somebody
/// made between the caller's `pull` and their `put` — minutes or hours,
/// the whole time they were editing — is not a conflict to `put`, it is
/// just the old file being replaced. Nothing warns. `--base` names the
/// moment the caller's copy is from, which is the only thing that makes
/// 「it moved under me」 a question khor can answer at all.
///
/// The three outcomes are the file view's three, in a terminal:
/// unchanged over there → written; changed but nowhere the caller
/// touched → merged and written; changed in the same lines → **nothing
/// written**, the merged text with its markers on stdout for the caller
/// to finish, and a non-zero exit.
fn put_over(
    rt: &tokio::runtime::Runtime,
    n: &Node,
    machine: &str,
    dest: &str,
    mine: &[u8],
    base: &str,
) -> Result<(), String> {
    let base_bytes = std::fs::read(base).map_err(|e| msg::cant_read(base, e))?;
    let (total, _, theirs) = rt.block_on(n.read_head(machine, dest))?;
    // **A cut read cannot be merged.** `read_head` answers one slice,
    // and a merge whose 「theirs」 is the first 256 KB of a longer file
    // would write back a truncation that looks like a success.
    if theirs.len() as u64 != total {
        return Err(cli::put_cut_cannot_merge(total, theirs.len()));
    }
    let now = khor_node::files::digest(&theirs);
    if now == khor_node::files::digest(&base_bytes) {
        // It never moved; this is a plain pinned write and it says the
        // plain sentence. Nothing was merged, so nothing claims to be.
        let (_, digest) = rt.block_on(n.write_path(machine, dest, &now, mine))?;
        println!("{}", cli::put_landed(dest, &digest[..8]));
        return Ok(());
    }
    // All three sides have to be text for a line merge to mean
    // anything. A binary that moved is a refusal, not a guess.
    let (base_s, mine_s, theirs_s) = match (
        String::from_utf8(base_bytes),
        std::str::from_utf8(mine),
        String::from_utf8(theirs),
    ) {
        (Ok(b), Ok(m), Ok(t)) => (b, m, t),
        _ => return Err(msg::file_changed_under_you(dest)),
    };
    let merged = khor_node::merge::three(&base_s, mine_s, &theirs_s);
    if merged.clashes > 0 {
        // The half-merged text is the caller's, on stdout, before the
        // refusal goes to stderr: a road that ends 「no」 with nothing
        // in your hands is the road these exits exist to replace.
        print!("{}", merged.text);
        return Err(cli::put_clashes(merged.clashes));
    }
    let (_, digest) = rt.block_on(n.write_path(machine, dest, &now, merged.text.as_bytes()))?;
    println!("{}", cli::put_merged(dest, &digest[..8]));
    Ok(())
}

/// What git already knows about a checkout on any machine, read over the
/// wire (`khor_node::git`).
///
/// **One verb with two words rather than `khor status` and `khor diff`.**
/// khor already has `state`, which reports a *session's* state word, and
/// `status` beside it would be two near-identical verbs for two
/// unrelated vocabularies — the six words belong to sessions and git's
/// six letters belong to files (docs/UX.md; M A D U C R — `khor_node::
/// git`, and `khor git status` prints whichever of them came back,
/// so the two new ones need no verb of their own). Under `git` there is no
/// collision left to have, and the two words read exactly as they do in
/// the tool they came from: 「git 相关操作应该用符合使用习惯的英文」.
///
/// **This verb is why the diff is computed in the node.** Line ranges
/// worked out in the GUI would be a diff `khor` cannot print, and
/// docs/KHOR.md's third foundation is that the two faces are equivalent.
fn git(rest: &[String]) -> Result<(), String> {
    match rest {
        [word, machine, path] if word == "status" => {
            let s = rt()?.block_on(node()?.git_status_on(machine, path))?;
            // The branch first and alone, because it is the one line
            // that is about the checkout rather than about a file — and
            // **nothing at all when there is no name to say**: a
            // detached HEAD would otherwise print a blank first line
            // that reads as「no changes」.
            if !s.branch.is_empty() {
                // **How far from the remote rides on the branch's own
                // line, and never without its date.** khor opens no
                // connection nobody asked for, so it never fetches:
                // 领先/落后 is a fact about the last time somebody
                // else did, and a three-week-old pair looks exactly
                // like a fresh one. Nothing is said when there is no
                // date to say it with — no upstream, or never fetched
                // — because 0/0 is also what being in step answers.
                let far = if s.fetched_ms > 0 && (s.ahead > 0 || s.behind > 0) {
                    // `lasting`, not `human_age`: this is 时长, and
                    // 时长 has to read the way the card reads it —
                    // the face puts the same fetch through `ago()`,
                    // which is `span()`, which `lasting` mirrors.
                    let since = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis() as i64)
                        .unwrap_or(0)
                        - s.fetched_ms;
                    let age = lasting((since.max(0) / 1000) as u64);
                    Some(match (s.ahead, s.behind) {
                        (a, 0) => cli::git_ahead(a, &age),
                        (0, b) => cli::git_behind(b, &age),
                        (a, b) => cli::git_track(a, b, &age),
                    })
                } else {
                    None
                };
                match far {
                    Some(far) => println!("{}\t{far}", s.branch),
                    None => println!("{}", s.branch),
                }
            }
            for (path, mark) in &s.rows {
                println!("{mark}\t{path}");
            }
            if s.more {
                eprintln!("{}", cli::git_status_more());
            }
            Ok(())
        }
        [word, machine, path] if word == "diff" => {
            let d = rt()?.block_on(node()?.git_diff_on(machine, path))?;
            for h in &d.hunks {
                // `at,lines` in the coordinates of the file as it is
                // now, which is the half of a unified header that is
                // true of the copy on this machine's screen. The old
                // side has no start line on the wire on purpose (the
                // gutter never needed one), so printing `-?` would be
                // inventing a number to look familiar.
                println!("@@ {},{} @@", h.at, h.lines);
                for line in &h.was {
                    println!("-{line}");
                }
            }
            if d.more {
                eprintln!("{}", cli::git_more());
            }
            Ok(())
        }
        _ => Err(usage_text().into()),
    }
}

/// Borrows a machine's network: the serve stands up a local HTTP proxy in
/// front of a lease and prints where it listens. Point a browser, or
/// `HTTPS_PROXY` and `HTTP_PROXY`, there and it goes out through the far
/// machine — `https` through CONNECT, plain `http` relayed (`tunnel`).
///
/// **This is the one borrow that is a session of its own**, because a
/// person is holding the port by hand and the row is how they find it
/// again and how they end it. A page opened in the app takes no row
/// (docs/NET.md 可达性).
fn borrow(rest: &[String]) -> Result<(), String> {
    let [machine] = rest else {
        return Err(usage_text().into());
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
        return Err(usage_text().into());
    };
    let name = node()?.forget_device(machine)?;
    println!("{}", msg::forgot(&name));
    Ok(())
}

/// Takes **this** machine out of the network (docs/MORE.md 离开).
///
/// One argument and no flag, exactly like `forget` above: typing the
/// machine's name *is* the confirmation, and the node is what checks it
/// (`Node::leave` — one rule for the verb and the page, the rule
/// `lock::reset_identity` already follows).
///
/// **Three sentences at most, never folded into one.** The act, who
/// heard it, and who did not — the last two are different situations
/// with different things left to do, and a machine in the third line is
/// one somebody has to go and run `khor forget` on.
fn leave(rest: &[String]) -> Result<(), String> {
    let [machine] = rest else {
        return Err(usage_text().into());
    };
    let n = node()?;
    let me = n.shown_name();
    let left = rt()?.block_on(n.leave(machine))?;
    println!("{}", msg::left());
    if !left.told.is_empty() {
        println!("{}", msg::left_told(left.told.join(cli::name_separator())));
    }
    if !left.untold.is_empty() {
        println!(
            "{}",
            msg::left_untold(left.untold.join(cli::name_separator()), &me)
        );
    }
    Ok(())
}

/// Stops this machine talking to the network without leaving it
/// (docs/MORE.md 暂停) — or, with a machine named, stops talking to that
/// one machine (SHELL.md 第五轮 第 3 条): not dialled, not answered, not
/// synced with, while it keeps running on its own. This machine's own
/// name is the same as no name.
fn pause(rest: &[String]) -> Result<(), String> {
    pause_or_resume(rest, true)
}

/// Puts it back (docs/MORE.md 暂停), for this machine or the one named.
fn resume(rest: &[String]) -> Result<(), String> {
    pause_or_resume(rest, false)
}

fn pause_or_resume(rest: &[String], on: bool) -> Result<(), String> {
    match rest {
        [] => {
            khor_node::pause::set_on(&Node::root_from_env(), on)?;
            println!("{}", if on { msg::paused() } else { msg::resumed() });
        }
        [machine] => println!("{}", node()?.set_machine_paused(machine, on)?),
        _ => return Err(usage_text().into()),
    }
    Ok(())
}

/// `khor watch <机器> [目录 ...]` — the directories that machine keeps an
/// eye on, replaced whole; none given clears the list. The CLI twin of
/// the card's editor (docs/KHOR.md 等价): one node call, one sentence
/// of refusal for both.
fn watch(rest: &[String]) -> Result<(), String> {
    let [machine, paths @ ..] = rest else {
        return Err(usage_text().into());
    };
    let n = node()?;
    let kept = n.set_watch(machine, paths)?;
    if kept.is_empty() {
        println!("{}", cli::watching_none(machine));
    } else {
        println!("{}", cli::watching(machine, kept.join(" ")));
    }
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
    // The library's constant, not this crate's own `env!`: the identity
    // card prints the same one (`khor_gui_core::about`), and two
    // spellings of「哪一版」is exactly the drift a version number exists
    // to reveal (`khor_node::VERSION`).
    println!("{}", khor_node::VERSION);
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
        all.iter().map(|d| (d.id.as_str(), d.shown())).collect();
    for d in &all {
        // **By id, not by name.** Two machines on one network can carry
        // the same hostname — two homes on one box always do — and the
        // name test then marks both rows 「(本机)」, which is the one
        // thing on this line a person reads to know where they are.
        // Seen live while adding the road reading (2026-08-21): a
        // second home on this Mac made every row claim to be home.
        let me = d.id == n.device_str();
        let here = if me { cli::this_machine() } else { "" };
        // 暂停 (docs/MORE.md), and 按机器暂停 (SHELL.md 第五轮 第 3
        // 条): what *this* machine decided — about itself on its own
        // row, about talking to that machine on that one's. Whether
        // another machine paused itself is not a thing this one can
        // know; what it sees is the road going quiet.
        let held = if n.machine_paused(&d.id) {
            cli::paused_mark()
        } else {
            ""
        };
        let pin = if d.pinned { cli::pinned_mark() } else { "" };
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
            "{}\t{}…{here}{held}{pin}\t{}\t{}",
            // **The name people gave it** (`DeviceInfo::shown`), which
            // is what every other surface prints. The channel is not a
            // secret, it is just not what this line is for.
            d.shown(),
            &d.id[..16],
            road,
            vitals_line(&n, &d.id)
        );
    }
    Ok(())
}

/// How many rows `khor top` prints. The card asks for its own count;
/// ten is what `top` itself shows above the fold. **Not derived from
/// the terminal**: the width below is read when it can be, but a row
/// count that changed with the window would make two runs on one
/// machine disagree about what「最忙的几个」means.
const TOP_ROWS: u32 = 10;

// ── `khor top`, as the table the card draws ─────────────────────────
//
// **A table, not a log** (2026-09-08, user:「我才发现,机器的二进制程序
// 的表,被搞成了日志.但网页不是.我要的是网页那样的表」).
// docs/KHOR.md:17「CLI 与 GUI 等价」has said so since before any of
// this; what stood here printed five tab-separated fields, and a tab
// stops at multiples of eight, so `74.5M` and `217.9M` on consecutive
// rows put 「命令」 in two different places. **Not a table that is
// slightly off — a table only on the batches whose numbers happen to
// be the same length**, which is exactly why it reads as a log.
//
// Everything below mirrors the card at
// **ff59afe5743857c72979f06477eeaa3cac97c3f5**:
// `COLS` (`apps/gui/src/views/MachineCard.tsx:1355`), `COL_ORDER`
// (:1376), `FILLS` (:1394), `STATUS_WORD` (:1424), `cellOf` (:1897).
// **The sha is half of each reference**: a line number with no version
// does not break when the file moves, it goes on pointing confidently
// at whatever ended up there.

/// One column of the process table.
///
/// **The order is the card's, and the card's is the user's**
/// (2026-09-05:「PID · 状态 · USER · 时长 · CPU时间 · CPU · GPU ·
/// 内存 · 显存 · 读写 · 命令」): the pid you type next, then what the
/// process *is*, then the two times, then the quantities, and 「命令」
/// last because it is the only column whose content has no bound — so
/// it is the one that can take width without opening a canyon between
/// two numbers.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Col {
    Pid,
    Status,
    User,
    Time,
    CpuTime,
    Cpu,
    Gpu,
    Mem,
    Vram,
    Io,
    Cmd,
}

const TOP_ORDER: [Col; 11] = [
    Col::Pid,
    Col::Status,
    Col::User,
    Col::Time,
    Col::CpuTime,
    Col::Cpu,
    Col::Gpu,
    Col::Mem,
    Col::Vram,
    Col::Io,
    Col::Cmd,
];

impl Col {
    /// The head word — every one of them out of the catalog, none of
    /// them written here. (The old listing printed a literal `pid`,
    /// lowercase, which is not a word `zh.toml` has.)
    fn head(self) -> &'static str {
        match self {
            Col::Pid => cli::top_pid(),
            Col::Status => cli::top_status(),
            Col::User => cli::top_user(),
            Col::Time => cli::top_time(),
            Col::CpuTime => cli::top_cputime(),
            Col::Cpu => cli::vitals_name_cpu(),
            Col::Gpu => cli::vitals_name_gpu(),
            Col::Mem => cli::vitals_name_mem(),
            Col::Vram => cli::vitals_name_vram(),
            Col::Io => cli::top_io(),
            Col::Cmd => cli::top_cmd(),
        }
    }

    /// Quantities right, words left — the card's `align`.
    fn right(self) -> bool {
        matches!(
            self,
            Col::Time | Col::CpuTime | Col::Cpu | Col::Gpu | Col::Mem | Col::Vram | Col::Io
        )
    }

    /// **Whether this is a column a machine might not be able to fill
    /// at all** — the seven the card latches in `Seen`. Four of them
    /// are「an older khor over there does not send this field」and
    /// three are「this machine has nothing to put in it」, and on screen
    /// they behave the same way, so they are one mechanism.
    fn optional(self) -> bool {
        matches!(
            self,
            Col::Status | Col::User | Col::Time | Col::CpuTime | Col::Gpu | Col::Vram | Col::Io
        )
    }

    /// What counts as one row filling it — the card's `FILLS`.
    ///
    /// **`User` is the odd one**: it is a `String` on the wire and its
    /// absence is spelled `""`, because a machine that will not name an
    /// account has no id to send either. So the emptiness has to be
    /// read, not the null.
    fn filled_by(self, r: &khor_node::TopRow) -> bool {
        match self {
            Col::Status => r.status.is_some(),
            Col::User => !r.user.is_empty(),
            Col::Time => r.run_secs.is_some(),
            Col::CpuTime => r.cpu_ms.is_some(),
            Col::Gpu => r.gpu_pct.is_some(),
            Col::Vram => r.gpu_mem.is_some(),
            Col::Io => r.io_bytes.is_some(),
            _ => true,
        }
    }

    /// What one cell says — the card's `cellOf`.
    ///
    /// **An empty string is 「读不到」 and it is deliberate.** Every one
    /// of these that can be absent says nothing rather than zero: a
    /// `0%` where the machine never looked is the neighbouring-value
    /// mistake this tree keeps paying for, and on a table it would be
    /// compared and believed.
    ///
    /// **In a terminal the empty cell is still padded to its column's
    /// width** (`lay_out` below), and that is not decoration. On the
    /// card the columns are a CSS grid, so an empty cell holds its
    /// place by itself; here nothing holds it, and a zero-width cell
    /// slides every column to its right one place left — the row then
    /// reads as though the *next* reading belonged to this head.
    /// **That is not one missing cell, it is one invented one.**
    fn cell(self, r: &khor_node::TopRow) -> String {
        match self {
            Col::Pid => r.pid.to_string(),
            // A token this build has no word for prints as the token —
            // never as a neighbour's word. See `status_word`.
            Col::Status => r
                .status
                .as_deref()
                .map(status_word)
                .unwrap_or("")
                .to_owned(),
            Col::User => r.user.clone(),
            Col::Time => r.run_secs.map(lasting).unwrap_or_default(),
            Col::CpuTime => r.cpu_ms.map(cpu_clock).unwrap_or_default(),
            // **`round`, not `{:.0}`** — Rust's formatter rounds halves
            // to even and `Math.round` rounds them up, so the card and
            // the terminal disagreed on every reading landing on a
            // half. Nobody would ever have noticed, which is the point.
            Col::Cpu => format!("{}%", r.cpu_pct.round() as i64),
            Col::Gpu => r
                .gpu_pct
                .map(|v| format!("{}%", v.round() as i64))
                .unwrap_or_default(),
            Col::Mem => bytes(r.mem),
            Col::Vram => r.gpu_mem.map(bytes).unwrap_or_default(),
            Col::Io => r.io_bytes.map(bytes).unwrap_or_default(),
            Col::Cmd => cmd_text(r),
        }
    }
}

/// **The name and its identifying word, as one field** — the card
/// merged those two columns into 「命令」 on 2026-09-04 (「表的进程和
/// 命令合并为一列,就叫命令」), and docs/KHOR.md says the terminal prints
/// what the card draws. It also gains something here: this listing used
/// to print the name alone, so three rows of `python3` arrived
/// indistinguishable in a terminal even though the word had crossed the
/// wire.
///
/// **Not printed twice when it repeats the name** — turing's two vLLM
/// workers have `argv[0] == name`, and `VLLM::EngineCore
/// VLLM::EngineCore` reads as a bug. What tells those two apart is the
/// pid in the first column.
fn cmd_text(r: &khor_node::TopRow) -> String {
    match r.arg.as_deref() {
        Some(a) if a != r.name => format!("{} {}", r.name, a),
        _ => r.name.clone(),
    }
}

/// The word for a status token, or the token itself — the card's
/// `STATUS_WORD`.
///
/// **A token with no word is printed as it stands**: `unknown-13` in a
/// terminal is something a person can look up, and it is the honest
/// answer for a status a newer node knows about and this build does
/// not. What it must never do is fall through to 「运行」 — the node
/// goes to some trouble not to say that word wrongly
/// (`khor_node::top::status_token`), and a lookup with a default would
/// undo all of it here.
fn status_word(token: &str) -> &str {
    match token {
        "run" => cli::top_st_run(),
        "sleep" => cli::top_st_sleep(),
        "idle" => cli::top_st_idle(),
        "spawning" => cli::top_st_spawning(),
        "diskwait" => cli::top_st_diskwait(),
        "dead" => cli::top_st_dead(),
        "zombie" => cli::top_st_zombie(),
        "stopped" => cli::top_st_stopped(),
        "tracing" => cli::top_st_tracing(),
        "parked" => cli::top_st_parked(),
        "wakekill" => cli::top_st_wakekill(),
        "waking" => cli::top_st_waking(),
        "suspended" => cli::top_st_suspended(),
        "lockblocked" => cli::top_st_lockblocked(),
        "unclear" => cli::top_st_unclear(),
        other => other,
    }
}

/// A duration in the coarsest unit that still says something — the
/// card's `span()` (`apps/gui/src/words.ts:273` at
/// ff59afe5743857c72979f06477eeaa3cac97c3f5).
///
/// **Deliberately not `human_age`, and this is not a duplication
/// somebody forgot to clean up.** `human_age` turns over at 120s / 2h /
/// 48h; the card turns over at 60s / 1h / 24h. Fed the same ninety
/// seconds the two say 「90 秒」 and 「1 分钟」 — **the same catalog
/// words, a different reading**, and nothing but a test can tell them
/// apart, because both are perfectly sensible sentences.
///
/// `human_age` has its own callers and its own reason (it dates
/// *messages*, where a stickier ladder reads better). This is 时长, and
/// 时长 has to agree with the card. **The mirror is guarded** —
/// `the_duration_ladder_turns_over_where_the_card_does`, which is the
/// only thing on this tree that would ever notice it drifting.
///
/// The card's `span` takes milliseconds and immediately floors them to
/// seconds; `run_secs` is already whole seconds, so taking seconds here
/// is the same function with the round trip removed.
fn lasting(secs: u64) -> String {
    if secs < 60 {
        return cli::age_seconds(secs);
    }
    let m = secs / 60;
    if m < 60 {
        return cli::age_minutes(m);
    }
    let h = m / 60;
    if h < 24 {
        return cli::age_hours(h);
    }
    cli::age_days(h / 24)
}

/// Accumulated CPU as a **stopwatch**, where 时长 beside it is a
/// duration in words — the card's `cpuClock`.
///
/// The two columns are both times and they are not the same time: one
/// is the wall clock, the other core-seconds, and a process on four
/// cores has burned four times the first. Printed in the same shape
/// they read as a contradiction (「起了 2 小时,烧了 8 小时」 looks like
/// a bug); printed as 「2 小时」 and 「8:03:41」 they read as two kinds
/// of thing, which is what they are.
///
/// **Past a hundred hours the seconds go.** hinton's `go.d.plugin` came
/// back at `1374:35:10` — ten characters, and on the card it spilled
/// out of its column. The seconds on a figure that size are not a
/// reading anybody uses; the magnitude is.
fn cpu_clock(ms: u64) -> String {
    // `(ms + 500) / 1000` is `Math.round(ms / 1000)` for a count that
    // cannot be negative, without going through a float.
    let s = (ms + 500) / 1000;
    let h = s / 3600;
    let m = (s % 3600) / 60;
    if h >= 100 {
        return format!("{h}:{m:02}");
    }
    if h > 0 {
        format!("{h}:{m:02}:{:02}", s % 60)
    } else {
        format!("{}:{:02}", s / 60, s % 60)
    }
}

/// How wide a string is on a screen. `状态` is two chars and four
/// columns; `.len()` says six and `chars().count()` says two, and both
/// are wrong in the direction that only shows up on the rows carrying
/// Chinese.
fn cols_of(s: &str) -> usize {
    unicode_width::UnicodeWidthStr::width(s)
}

/// Which columns this batch of rows can fill.
///
/// **The question is 「can any row fill me」, not 「does this machine
/// have the hardware」** — turing reports two RTX A5000s and not one of
/// its two hundred processes carries a per-process figure, so asking
/// about the cards drew two permanently empty columns.
///
/// **The card latches this and this does not**, and the difference is
/// not a simplification. The card re-asks every five seconds, so a
/// column that vanished between two polls would delete a head the
/// person was in the middle of pressing; one batch printed once has no
/// second poll to disagree with.
fn shown(rows: &[khor_node::TopRow]) -> Vec<Col> {
    TOP_ORDER
        .into_iter()
        .filter(|c| !c.optional() || rows.iter().any(|r| c.filled_by(r)))
        .collect()
}

/// Cut a cell down to `room` display columns, ending in an ellipsis.
///
/// A wide character that would straddle the last column is left out
/// rather than half-drawn, so the result can come back one column
/// narrower than asked — never one wider, which is the direction that
/// wraps the line.
fn cut_to(s: &str, room: usize) -> String {
    if cols_of(s) <= room {
        return s.to_owned();
    }
    let keep = room.saturating_sub(1);
    let mut out = String::new();
    let mut used = 0;
    for ch in s.chars() {
        let w = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if used + w > keep {
            break;
        }
        out.push(ch);
        used += w;
    }
    out.push('…');
    out
}

/// The head line and one line a row, laid out in columns.
///
/// `room` is the terminal's width in display columns, or `None` when
/// stdout will not say (`tty_size`). **It is only ever used to shorten
/// 「命令」**, and when it is absent nothing is shortened — see `top`.
///
/// **What the padding costs, and when this decision expires.** Spaces
/// instead of tabs break `khor top | cut -f`. Taken knowingly, on three
/// counts: nothing on this tree consumes that output (2026-09-08, the
/// verbs `apps/gui/scripts/smoke.mjs` runs were listed and `top` is not
/// among them — the same search finds `devices`, which *is* there, so
/// the empty answer is an answer and not a bad search); what was asked
/// for is a table a **person** reads; and the right door for a machine
/// is a `--json`, not a field separator that only lines up by luck.
/// **Either half of that can stop being true**: give `khor top` a
/// `--json` and the tab is gone for good, or find one real caller
/// piping it into `cut` and it was never free. Neither will go red —
/// this paragraph is the only thing that will tell the next person
/// where to look.
fn lay_out(rows: &[khor_node::TopRow], room: Option<usize>) -> Vec<String> {
    let cols = shown(rows);
    let mut grid: Vec<Vec<String>> = Vec::with_capacity(rows.len() + 1);
    grid.push(cols.iter().map(|c| c.head().to_owned()).collect());
    for r in rows {
        grid.push(cols.iter().map(|c| c.cell(r)).collect());
    }
    // Each column as wide as the widest thing in it, head included —
    // the card's tracks are content-sized too, and it is what leaves
    // 「命令」 the most room.
    let mut w: Vec<usize> = (0..cols.len())
        .map(|i| grid.iter().map(|row| cols_of(&row[i])).max().unwrap_or(0))
        .collect();
    // 「命令」 is the only column that can be shortened, because it is
    // the only one whose content has no bound. **And only against a
    // width somebody actually reported**: `room` is `None` down a pipe,
    // and a pipe has a person on the other end often enough
    // (`| less`, `| head`) that guessing a width there would hand the
    // reader the very thing being fixed.
    if let (Some(room), Some(last)) = (room, cols.len().checked_sub(1)) {
        debug_assert_eq!(
            cols[last],
            Col::Cmd,
            "「命令」 is never optional, so it is always last"
        );
        let fixed: usize = w[..last].iter().sum::<usize>() + 2 * last;
        let budget = room.saturating_sub(fixed);
        // **Below its own head word, stop cutting and let the line
        // wrap.** Not because a cut could not fit — measured at 62 and
        // 60 columns on this Mac (2026-09-08 20:37), a two-column
        // 「命令」 does bring the line inside a 60-wide window. It is
        // that two columns hold one character and an ellipsis, which
        // names no process; the text is worth more wrapped than
        // truncated to nothing.
        //
        // (An earlier draft of this comment said 「no cut can rescue
        // the line」. That was a confident sentence and it was false —
        // and it was false in the direction that stops the next person
        // from re-opening the question. The reason is a judgement about
        // what a two-column column is worth, not an arithmetic fact.)
        if budget >= cols_of(cols[last].head()) && budget < w[last] {
            for row in grid.iter_mut().skip(1) {
                row[last] = cut_to(&row[last], budget);
            }
            w[last] = budget;
        }
    }
    grid.iter()
        .map(|row| {
            let mut line = String::new();
            for (i, s) in row.iter().enumerate() {
                if i > 0 {
                    line.push_str("  ");
                }
                let pad = " ".repeat(w[i].saturating_sub(cols_of(s)));
                if cols[i].right() {
                    line.push_str(&pad);
                    line.push_str(s);
                } else {
                    line.push_str(s);
                    // No trailing run of spaces on the last column:
                    // nothing follows it to line up with, and they
                    // would ride along into anything copied out.
                    if i + 1 < row.len() {
                        line.push_str(&pad);
                    }
                }
            }
            line
        })
        .collect()
}

/// What a machine is busy with — the CLI half of the machine card's
/// lower half (docs/KHOR.md: CLI equals GUI). The top as a table, then
/// the CPU day as one line of blocks, six buckets to a block so a day
/// fits a terminal; a block nobody measured is a dot, the band's gap.
fn top(rest: &[String]) -> Result<(), String> {
    let machine = match rest {
        [] => "",
        [machine] => machine.as_str(),
        _ => return Err(usage_text().into()),
    };
    let load =
        rt()?.block_on(node()?.load_on(machine, TOP_ROWS, khor_node::TopBy::Busiest, true))?;
    // **Measured, not guessed** (2026-09-08): down a pipe the ioctl
    // fails with `ENOTTY`; on a pty `script(1)` just made it succeeds
    // and answers 0×0, which `tty_size` already refuses; on a real
    // window it answers the width. `COLUMNS` is not a third way — this
    // shell does not export it, and the value it holds is `0`, so
    // reading it would parse fine and lay out a table no columns wide.
    let room = tty_size().map(|(cols, _)| cols as usize);
    for line in lay_out(&load.top, room) {
        println!("{line}");
    }
    println!("{}  {}", cli::top_day(), day_line(&load.cpu_day));
    Ok(())
}

/// A day of CPU as blocks — the mean of each run of six buckets (half an
/// hour), one glyph per run, a dot where none of the six was measured.
///
/// **This line is a fixed 66 display columns and does not answer to the
/// terminal's width**: the label is 16, the separator 2, and there are
/// 48 glyphs (measured 2026-09-08 20:37, not worked out). The table
/// above it fits whatever window it is given — at 62 columns the table
/// came back exactly 62 wide — and **this line is then the only thing
/// that wraps.**
///
/// **Left alone on purpose.** The count is a meaning, not a layout
/// number: one glyph is half an hour, and a day is 48 of them. It is
/// also narrower than what it replaced — the tab-separated version put
/// the blocks at column 24 and ran to 72.
///
/// **When to re-open it**: somebody actually reading this in a window
/// under 66 columns. There is no such person today, which is the whole
/// reason this is not being done. If there is one, the move is to
/// coarsen the bucket — an hour to a glyph is 24 of them and a 42-column
/// line — **not to truncate**, because a day band with its tail cut off
/// still looks like a day.
fn day_line(day: &khor_node::CpuDay) -> String {
    const BLOCKS: [char; 8] = [
        '\u{2581}', '\u{2582}', '\u{2583}', '\u{2584}', '\u{2585}', '\u{2586}', '\u{2587}',
        '\u{2588}',
    ];
    day.buckets
        .chunks(6)
        .map(|run| {
            let seen: Vec<f32> = run.iter().flatten().copied().collect();
            if seen.is_empty() {
                return '\u{00b7}';
            }
            let mean = seen.iter().sum::<f32>() / seen.len() as f32;
            let step = ((mean / 100.0) * (BLOCKS.len() - 1) as f32).round() as usize;
            BLOCKS[step.min(BLOCKS.len() - 1)]
        })
        .collect()
}

#[cfg(test)]
mod top_table {
    use super::*;
    use khor_node::TopRow;

    /// Three rows chosen so that every shape the table has to survive
    /// is in them: a status with a word and one without, a user the
    /// machine would not name, an absent 读写 and an absent CPU时间,
    /// and readings that land on all three rungs of the 时长 ladder.
    fn fixture() -> Vec<TopRow> {
        let row = |pid, name: &str, arg: Option<&str>, user: &str, cpu_pct, mem| TopRow {
            pid,
            name: name.to_owned(),
            user: user.to_owned(),
            cpu_pct,
            mem,
            gpu_pct: None,
            gpu_mem: None,
            run_secs: None,
            arg: arg.map(str::to_owned),
            ppid: None,
            status: None,
            cpu_ms: None,
            io_bytes: None,
        };
        vec![
            TopRow {
                status: Some("run".into()),
                run_secs: Some(3_600),
                cpu_ms: Some(3_600_000),
                io_bytes: Some(1024 * 1024),
                ..row(
                    12_392,
                    "khor",
                    Some("serve"),
                    "zgh",
                    70.0,
                    110 * 1024 * 1024,
                )
            },
            TopRow {
                status: Some("spawning".into()),
                run_secs: Some(90),
                cpu_ms: Some(0),
                io_bytes: None,
                ..row(7, "python3", None, "zgh", 0.4, 512)
            },
            TopRow {
                // A status this build has no word for. It must arrive
                // as itself; the one thing it must never do is borrow a
                // neighbour's word.
                status: Some("unknown-13".into()),
                run_secs: Some(90_000),
                cpu_ms: None,
                io_bytes: Some(0),
                ..row(
                    96_042,
                    "Google Chrome Helper",
                    None,
                    "",
                    11.0,
                    100 * 1024 * 1024,
                )
            },
        ]
    }

    /// Every cell the fixture should produce, written out here rather
    /// than asked of the code that produces them.
    ///
    /// **A function, not a `const`** since 批 D3: a word depends on the
    /// language now, so it cannot be computed before the program runs.
    fn head() -> [&'static str; 9] {
        [
            cli::top_pid(),
            cli::top_status(),
            cli::top_user(),
            cli::top_time(),
            cli::top_cputime(),
            cli::vitals_name_cpu(),
            cli::vitals_name_mem(),
            cli::top_io(),
            cli::top_cmd(),
        ]
    }
    /// Quantities right, words left — written out here too, so a column
    /// that quietly changed sides would go red rather than be agreed
    /// with.
    const RIGHT: [bool; 9] = [false, false, false, true, true, true, true, true, false];

    fn want_cells() -> Vec<Vec<String>> {
        let s = |v: &[&str]| v.iter().map(|x| (*x).to_owned()).collect::<Vec<_>>();
        vec![
            s(&head()),
            s(&[
                "12392",
                cli::top_st_run(),
                "zgh",
                &cli::age_hours(1),
                "1:00:00",
                "70%",
                "110.0M",
                "1.0M",
                "khor serve",
            ]),
            s(&[
                "7",
                cli::top_st_spawning(),
                "zgh",
                &cli::age_minutes(1),
                "0:00",
                "0%",
                "512B",
                "",
                "python3",
            ]),
            s(&[
                "96042",
                "unknown-13",
                "",
                &cli::age_days(1),
                "",
                "11%",
                "100.0M",
                "0B",
                "Google Chrome Helper",
            ]),
        ]
    }

    /// Lay `want_cells` out the way the table should have: each column
    /// as wide as its widest entry, two spaces between, right-aligned
    /// where `RIGHT` says, and no trailing run on the last one.
    fn want_lines(cells: &[Vec<String>]) -> Vec<String> {
        let n = cells[0].len();
        let w: Vec<usize> = (0..n)
            .map(|i| cells.iter().map(|r| cols_of(&r[i])).max().unwrap())
            .collect();
        cells
            .iter()
            .map(|row| {
                let mut line = String::new();
                for (i, c) in row.iter().enumerate() {
                    if i > 0 {
                        line.push_str("  ");
                    }
                    let pad = " ".repeat(w[i] - cols_of(c));
                    if RIGHT[i] {
                        line.push_str(&pad);
                        line.push_str(c);
                    } else {
                        line.push_str(c);
                        if i + 1 < n {
                            line.push_str(&pad);
                        }
                    }
                }
                line
            })
            .collect()
    }

    /// The first `cols` display columns of a line. A wide character
    /// that would straddle the edge is left out, so this can come back
    /// one column short — which is fine for comparing two lines with it,
    /// because it is short in the same place on both.
    fn upto(line: &str, cols: usize) -> String {
        let mut out = String::new();
        let mut used = 0;
        for ch in line.chars() {
            let w = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
            if used + w > cols {
                break;
            }
            out.push(ch);
            used += w;
        }
        out
    }

    /// **The claim in the form it can be falsified**: under one batch of
    /// rows every column occupies the same span of display columns on
    /// every line, head included.
    ///
    /// Checked by building the whole table a second way — from cells
    /// written out above, with widths taken as display columns — and
    /// comparing the two texts. An implementation that padded by
    /// `.len()` or by `chars().count()` renders lines that are not this
    /// text, and the assertion prints both.
    #[test]
    fn every_column_lands_on_the_same_display_columns_on_every_line() {
        let cells = want_cells();
        // ⚠️ **The arm without which this test is green on a broken
        // implementation.** Padding by length only comes apart on cells
        // where length and width disagree; an all-ASCII fixture would
        // pass a table padded any way at all. `新建中` is three chars,
        // nine bytes and six columns — if a later edit swaps these rows
        // for English ones, this fails here rather than going quietly
        // green downstairs.
        assert!(
            cells
                .iter()
                .flatten()
                .any(|c| cols_of(c) != c.chars().count() && cols_of(c) != c.len()),
            "no cell in this fixture is a wide character, so the comparison below cannot tell a display-width layout from a byte-length one"
        );
        let want = want_lines(&cells);
        let got = lay_out(&fixture(), None);
        assert_eq!(
            got,
            want,
            "\ngot:\n{}\nwant:\n{}\n",
            got.join("\n"),
            want.join("\n")
        );
    }

    /// A column no row can fill is not drawn, and the ones that are keep
    /// the card's order.
    #[test]
    fn a_column_no_row_can_fill_is_not_drawn() {
        // The fixture reads no GPU at all — the machine this was
        // written on cannot, and neither can turing's two hundred
        // processes on two A5000s.
        assert_eq!(
            shown(&fixture()),
            vec![
                Col::Pid,
                Col::Status,
                Col::User,
                Col::Time,
                Col::CpuTime,
                Col::Cpu,
                Col::Mem,
                Col::Io,
                Col::Cmd
            ]
        );
        // One row filling it is enough, and it comes back **in the
        // card's place** — between CPU and 内存, not appended.
        let mut with_gpu = fixture();
        with_gpu[0].gpu_pct = Some(200.0);
        let cols = shown(&with_gpu);
        assert_eq!(cols.iter().position(|c| *c == Col::Gpu), Some(6));
        assert_eq!(cols[7], Col::Mem);
        // And a batch with nothing optional in it still draws the four
        // that are never optional — an empty `Seen` is not an empty
        // table.
        let bare = TopRow {
            status: None,
            run_secs: None,
            cpu_ms: None,
            io_bytes: None,
            user: String::new(),
            ..fixture()[1].clone()
        };
        assert_eq!(shown(&[bare]), vec![Col::Pid, Col::Cpu, Col::Mem, Col::Cmd]);
    }

    /// The three places the 时长 ladder turns over, against the card's
    /// (`apps/gui/src/words.ts:273` at
    /// ff59afe5743857c72979f06477eeaa3cac97c3f5).
    ///
    /// **No number here is one somebody worked out.** Each side is a
    /// catalog word applied to a figure read off that function, so this
    /// compares two readings of the same table rather than a reading
    /// against a literal — the kind of constant that goes red saying
    /// somebody else's name.
    #[test]
    fn the_duration_ladder_turns_over_where_the_card_does() {
        assert_eq!(lasting(0), cli::age_seconds(0));
        assert_eq!(lasting(59), cli::age_seconds(59));
        assert_eq!(lasting(60), cli::age_minutes(1));
        assert_eq!(lasting(3_599), cli::age_minutes(59));
        assert_eq!(lasting(3_600), cli::age_hours(1));
        assert_eq!(lasting(86_399), cli::age_hours(23));
        assert_eq!(lasting(86_400), cli::age_days(1));
        // **The arm that says why this function exists.** `human_age`
        // is the other ladder in this file and it turns over elsewhere;
        // ninety seconds is inside the gap. Delete `lasting` and route
        // 时长 through `human_age` — the obvious tidy-up — and this is
        // the line that goes red. Nothing else would: both answers are
        // sensible sentences made of the same catalog words.
        assert_ne!(lasting(90), human_age(90 * 1000));
    }

    /// 「命令」 is the only column that gets shortened, and only against
    /// a width somebody reported.
    #[test]
    fn only_the_command_column_is_cut_and_only_against_a_real_width() {
        let rows = fixture();
        let loose = lay_out(&rows, None);
        // **The control arm.** A width far past what the table needs
        // must change nothing — otherwise「it cut when I gave it a
        // width」would be true of every width, and the test below would
        // be measuring `Some` rather than the arithmetic.
        assert_eq!(
            lay_out(&rows, Some(500)),
            loose,
            "a width the table already fits under must not cut anything"
        );

        let tight = lay_out(&rows, Some(70));
        assert_ne!(
            tight, loose,
            "70 columns is narrower than this table's own command column"
        );
        // Everything left of 「命令」 is untouched — the cut takes width
        // off the last column and off no other. The head line ends in
        // 「命令」 with nothing padded after it, so the width of
        // everything before it is that line's width less that word's.
        let before = cols_of(&loose[0]) - cols_of(cli::top_cmd());
        for (a, b) in tight.iter().zip(&loose) {
            assert!(cols_of(a) <= cols_of(b));
            assert_eq!(
                upto(a, before),
                upto(b, before),
                "the cut moved a column left of 「命令」"
            );
        }
        assert!(
            tight.iter().skip(1).any(|l| l.ends_with('…')),
            "nothing was actually cut"
        );
        for l in tight.iter().skip(1) {
            assert!(
                cols_of(l) <= 70,
                "a cut line is still wider than the terminal: {l:?}"
            );
        }
        // Narrower than the fixed columns alone: cutting the last one
        // cannot rescue the line, so it is left whole rather than
        // reduced to an ellipsis.
        assert_eq!(lay_out(&rows, Some(20)), loose);
    }

    /// Every status the node can send has its own word, and one it
    /// cannot arrives as itself.
    ///
    /// **What this does not cover**: a token `khor_node::top` grows
    /// later. That list is `pub(crate)` over there and this cannot
    /// walk it, so what is guarded is「no arm points at the wrong
    /// word」, not「no token is missing」.
    #[test]
    fn every_status_token_has_its_own_word() {
        let pairs = [
            ("run", cli::top_st_run()),
            ("sleep", cli::top_st_sleep()),
            ("idle", cli::top_st_idle()),
            ("spawning", cli::top_st_spawning()),
            ("diskwait", cli::top_st_diskwait()),
            ("dead", cli::top_st_dead()),
            ("zombie", cli::top_st_zombie()),
            ("stopped", cli::top_st_stopped()),
            ("tracing", cli::top_st_tracing()),
            ("parked", cli::top_st_parked()),
            ("wakekill", cli::top_st_wakekill()),
            ("waking", cli::top_st_waking()),
            ("suspended", cli::top_st_suspended()),
            ("lockblocked", cli::top_st_lockblocked()),
            ("unclear", cli::top_st_unclear()),
        ];
        for (token, word) in pairs {
            assert_eq!(status_word(token), word, "{token}");
            assert_ne!(status_word(token), token, "{token} came back untranslated");
        }
        // Two arms pointing at one word is the way a wrong arm hides:
        // both readings look like words, and the row says the other
        // one's fact.
        let mut words: Vec<&str> = pairs.iter().map(|(_, w)| *w).collect();
        words.sort_unstable();
        words.dedup();
        assert_eq!(words.len(), pairs.len(), "two tokens share a word");
        assert_eq!(status_word("unknown-13"), "unknown-13");
    }
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
        return cli::vitals_never().to_owned();
    };
    let mut parts = vec![
        cli::vitals_cpu(format_args!("{:.0}", v.cpu_pct), v.cores),
        cli::vitals_mem(bytes(v.mem.used), bytes(v.mem.total)),
    ];
    // The volumes one by one when the machine surveyed them
    // (2026-09-02:「硬盘存储」明细); the single home-volume reading is
    // the older shape's fallback — and 「说不上」 belongs only to that
    // fallback: an empty survey is an old reporter, not an empty
    // machine.
    if v.volumes.is_empty() {
        parts.push(match v.disk {
            Some(d) => cli::vitals_disk(bytes(d.used), bytes(d.total)),
            None => cli::vitals_disk_unknown().to_owned(),
        });
    } else {
        for vol in &v.volumes {
            parts.push(cli::vitals_vol(
                vol_word(&vol.mount),
                bytes(vol.fill.used),
                bytes(vol.fill.total),
            ));
        }
    }
    // **Nothing is said when there is no reading**, unlike the disk right
    // above — a machine with no GPU is an ordinary machine, and the word
    // for that would be printed by most desktops to report a non-event
    // (`khor_core::Gpu`). Same again one level in: video memory is its
    // own absence, because a unified-memory machine has none to report
    // rather than none left.
    if v.gpus.is_empty() {
        if let Some(g) = v.gpu {
            parts.push(cli::vitals_gpu(format_args!("{:.0}", g.util_pct), g.cards));
            if let Some(m) = g.mem {
                parts.push(cli::vitals_vram(bytes(m.used), bytes(m.total)));
            }
        }
    } else {
        // One sentence per card (2026-09-02:「多个GPU，应该分多个环」),
        // its memory inside the same sentence — the GUI draws one ring
        // per card and smoke counts the two faces' pieces against each
        // other, so a second piece here would be a phantom ring there.
        // A card with no name gets the generic word, both ends' rule
        // (`khor_core::GpuCard::name`).
        for c in &v.gpus {
            let name = if c.name.is_empty() {
                cli::vitals_name_gpu()
            } else {
                c.name.as_str()
            };
            parts.push(match c.mem {
                Some(m) => cli::vitals_gpu_card_mem(
                    name,
                    format_args!("{:.0}", c.util_pct),
                    bytes(m.used),
                    bytes(m.total),
                ),
                None => cli::vitals_gpu_card(name, format_args!("{:.0}", c.util_pct)),
            });
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
    // The directories it was asked to watch, one sentence each in one
    // of the three shapes (`khor_core::WatchedDir`), from the answer to
    // `Request::Watched` — its own record and its own age; the card
    // draws the same rows under its disk, one unit each, so the count
    // here and there stays one count. Nothing is said for a machine
    // that never answered: an older khor refuses the op, and 「no rows」
    // must not read as 「nothing watched」.
    // **Only the rows still on the list.** The answer is the machine's
    // last word and the list is the setting as replicated; between a
    // save that took a path off and the next visit, the answer still
    // carries it. The card draws its rows from the list and lets the
    // answer fill them in, so it drops that path at once — and the two
    // faces are held to one count.
    if let Some((rows, _)) = n.watched_of(device_id) {
        let listed = n.watch_of(device_id);
        for r in rows.iter().filter(|r| listed.contains(&r.path)) {
            parts.push(match (&r.volume, r.missing) {
                (Some(vol), _) => cli::vitals_watch(
                    &r.path,
                    vol_word(&vol.mount),
                    bytes(vol.fill.total.saturating_sub(vol.fill.used)),
                    bytes(vol.fill.total),
                ),
                (None, true) => cli::vitals_watch_missing(&r.path),
                (None, false) => cli::vitals_watch_unlisted(&r.path),
            });
        }
    }
    // Said for every reading that was not taken just now — which is every
    // machine but this one, and this one only while it is answering.
    if age > 0 {
        parts.push(cli::vitals_taken(human_age(age)));
    }
    parts.join("  ")
}

/// A volume's short name: the last step of its mount point, with the
/// two spellings of "the main disk" both drawn as `/` — macOS's
/// `/System/Volumes/Data` IS the root a person means. Painting, not
/// judgment, so the GUI keeps a copy of its own the way `bytes` below
/// has one; the wire carries the honest mount.
fn vol_word(mount: &str) -> &str {
    if mount == "/System/Volumes/Data" || mount == "/" {
        return "/";
    }
    mount.rsplit('/').find(|s| !s.is_empty()).unwrap_or(mount)
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
            _ => return Err(usage_text().into()),
        },
        _ => return Err(usage_text().into()),
    };
    let n = node()?;
    for d in n.devices()? {
        let here = if d.id == n.device_str() {
            cli::this_machine()
        } else {
            ""
        };
        println!("{}{here}", d.shown());
        let Some((usage, age)) = n.usage_of(&d.id) else {
            println!("\t{}", cli::usage_never());
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
        let recent: Vec<&UsageDay> = usage.days.iter().rev().filter(|d| d.day >= from).collect();
        if recent.is_empty() {
            println!("\t{}", cli::usage_none());
        }
        for row in recent {
            println!(
                "\t{}\t{}\t{}",
                row.day,
                row.category,
                tokens_line(&row.tokens)
            );
            // The models under that day, one indent deeper. **Absent
            // rather than empty when the answering machine predates the
            // breakdown** (`khor_core::Usage::models`), which is why
            // this is a filter over what arrived and never a row of its
            // own saying 「no models」 — that would be a claim about the
            // spending where the truth is a claim about the answerer.
            //
            // ⚠️ **This layout is for a person to read, not for the
            // next screen to copy.** Whoever draws the 花了多少 page
            // reads `Usage::models`, which states its own ordering and
            // its own exhaustiveness; a face that reproduced these tabs
            // would be depending on a shape nothing keeps stable.
            for m in usage
                .models
                .iter()
                .filter(|m| m.day == row.day && m.category == row.category)
            {
                let name = m.model.as_deref().unwrap_or(cli::usage_model_unnamed());
                println!("\t\t{}\t{}", name, tokens_line(&m.tokens));
            }
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
                let key = it.next().ok_or_else(|| usage_text().to_string())?;
                mode = list::Arrange::from_key(key).ok_or_else(|| {
                    let keys: Vec<&str> = list::Arrange::ALL.iter().map(|a| a.key()).collect();
                    msg::not_a_way_to_arrange(key, keys.join(cli::name_separator()))
                })?;
            }
            _ => return Err(usage_text().into()),
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
        let pin = if v.pinned {
            format!("\t{}", cli::pinned_mark())
        } else {
            String::new()
        };
        // The quiet note (词的保鲜期批): a live word nobody can vouch
        // for, silent past the threshold — said beside the word, which
        // stands untouched. The duration is the word's own age.
        let quiet = if v.quiet {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);
            format!(
                "\t{}",
                cli::quiet_for(human_age(now.saturating_sub(s.state.at.0)))
            )
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
        return Err(usage_text().into());
    };
    if text.is_empty() {
        return Err(usage_text().into());
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
        return Err(usage_text().into());
    };
    if tail.is_empty() {
        return Err(usage_text().into());
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
        return Err(usage_text().into());
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
        return Err(usage_text().into());
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
                title = Some(it.next().ok_or_else(|| usage_text().to_string())?.clone());
            }
            "--" if cmd.is_empty() => {}
            _ => cmd.push(a.clone()),
        }
    }
    if cmd.is_empty() {
        return Err(usage_text().into());
    }
    let n = node()?;
    let kind = if tui {
        khor_node::kind::TUI
    } else {
        khor_node::kind::SHELL
    };
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
                on = Some(it.next().ok_or_else(|| usage_text().to_string())?.clone());
            }
            // A registered ACP agent, by the name its owner gave it
            // (批⑥). Implies `--gui`: a registration says how to start
            // something that speaks the protocol, and there is no other
            // channel for it.
            "--agent" if cmd.is_empty() => {
                agent = Some(it.next().ok_or_else(|| usage_text().to_string())?.clone());
                gui = true;
            }
            // A conversation the agent already has, by the id it gave
            // (`khor agents sessions`). The agent loads it instead of
            // opening a fresh one, so the row that appears is the same
            // conversation rather than a new one beside it.
            "--resume" if cmd.is_empty() => {
                resume = Some(it.next().ok_or_else(|| usage_text().to_string())?.clone());
            }
            "--title" if cmd.is_empty() => {
                title = Some(it.next().ok_or_else(|| usage_text().to_string())?.clone());
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
                return Err(cli::gui_not_on_another_machine().into());
            }
            if resume.is_some() {
                return Err(cli::resume_not_on_another_machine().into());
            }
            let name = agent
                .as_deref()
                .unwrap_or(khor_node::adaptor::claude::VENDOR);
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
            return Err(cli::resume_needs_agent().into());
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
    let kind = if tui {
        khor_node::kind::TUI
    } else {
        khor_node::kind::SHELL
    };
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
        return Err(usage_text().into());
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
        None => return Err(usage_text().into()),
    };
    let [ready, title] = head else {
        return Err(usage_text().into());
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
        None => return Err(usage_text().into()),
    };
    let [sid, cols, rows] = head else {
        return Err(usage_text().into());
    };
    if cmd.is_empty() {
        return Err(usage_text().into());
    }
    let size = (
        cols.parse::<u16>().map_err(|_| usage_text().to_string())?,
        rows.parse::<u16>().map_err(|_| usage_text().to_string())?,
    );
    khor_node::host::host_main(
        Node::root_from_env(),
        SessionId(sid.clone()),
        size,
        cmd.to_vec(),
    )
}

/// 接管 (批C): typing the verb is the confirmation — the CLI's precedent
/// is `close`, which also acts on the word alone. The GUI is where a
/// click is cheap enough to deserve a second look.
fn takeover(rest: &[String]) -> Result<(), String> {
    let [sid] = rest else {
        return Err(usage_text().into());
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
            let sid =
                std::env::var("KHOR_SESSION").map_err(|_| msg::which_session().to_string())?;
            let word = khor_node::State::try_from(word.clone())
                .map_err(|_| msg::not_a_state_word(word))?;
            n.report_state(&SessionId(sid), word)
        }
        [word, flag, sid] if flag == "--session" => {
            let word = khor_node::State::try_from(word.clone())
                .map_err(|_| msg::not_a_state_word(word))?;
            n.report_state(&SessionId(sid.clone()), word)
        }
        _ => Err(usage_text().into()),
    }
}

fn seen(rest: &[String]) -> Result<(), String> {
    let [id] = rest else {
        return Err(usage_text().into());
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
    // `-m`, the flag `pin` already uses for the same distinction — one
    // verb, two subjects, and the subject is named the same way in both
    // places rather than each verb inventing its own.
    if let [flag, machine, name @ ..] = rest
        && (flag == "-m" || flag == "--machine")
    {
        let n = node()?;
        // Resolved to an id here, because `rename_machine` takes an
        // id on purpose: the name is the thing being changed, and
        // resolving 「rename A to B」 through a table that may
        // already hold a B renames the wrong row.
        let (_, id) = n.resolve_machine(machine)?;
        println!("{}", n.rename_machine(&id.hex(), &name.join(" "))?);
        return Ok(());
    }
    let [id, name @ ..] = rest else {
        return Err(usage_text().into());
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
        // **Retired** (docs/SHELL.md 第六轮): a pinned directory is a pinned
        // workspace now — the list the fresh tab's 「置顶的目录」 column used
        // to stand in for. Refused, non-zero, with the verbs that do it.
        [flag, machine, path] if flag == "-d" || flag == "--dir" => {
            Err(khor_catalog::msg::dir_pin_retired(machine, path))
        }
        [id] => n.pin_session(&SessionId(id.clone()), on),
        _ => Err(usage_text().into()),
    }
}

fn close(rest: &[String]) -> Result<(), String> {
    let [id] = rest else {
        return Err(usage_text().into());
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
/// Opens the desktop app's window.
///
/// The desktop app is its own binary, `khor-gui` (the tauri crate, named
/// by cargo after its package), installed **beside** `khor` the way
/// `_ghost` and `_host` are found beside it: one directory, no registry,
/// nothing to configure. Spawned into its own session so the window
/// outlives the terminal that asked for it; the CLI returns at once
/// because a window is not something to wait on. A missing binary is said
/// in words that name where to put it, never as a generic "not found".
///
/// **Only `khor-gui` is looked for, never tauri's `Khor`.** tauri also
/// emits a copy named after `productName`, and on the case-insensitive
/// APFS this Mac runs, `Khor` *is* `khor` — the same inode as the CLI
/// (measured: both 28 MB, one inode; the real app is the 34 MB
/// `khor-gui`). A fallback to that name would have this verb exec the
/// CLI itself and call it a window.
fn gui(rest: &[String]) -> Result<(), String> {
    if !rest.is_empty() {
        return Err(usage_text().into());
    }
    let me = khor_node::self_exe()?;
    let dir = me
        .parent()
        .ok_or_else(|| msg::gui_not_installed(me.display()))?;
    let bin = dir.join("khor-gui");
    if !bin.is_file() {
        return Err(msg::gui_not_installed(dir.display()));
    }
    let mut c = std::process::Command::new(&bin);
    c.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(unix)]
    unsafe {
        use std::os::unix::process::CommandExt;
        c.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
    c.spawn()
        .map_err(|e| msg::gui_wont_start(format_args!("{e}")))?;
    Ok(())
}

fn web(rest: &[String]) -> Result<(), String> {
    let fresh = match rest {
        [] => false,
        [flag] if flag == "--new" => true,
        _ => return Err(usage_text().into()),
    };
    let root = Node::root_from_env();
    let key = if fresh {
        khor_web::key::rotate(&root)?
    } else {
        khor_web::key::ensure(&root)?
    };
    if fresh {
        println!("{}", msg::web_face_rotated());
    }
    // Rotating is a local act and has already happened; only the
    // printing of an address depends on somebody being up to answer it.
    let port = khor_web::port();
    let Some(ips) = node()?.local_ips()? else {
        return Err(msg::web_face_not_serving().to_owned());
    };
    let Some(ip) = khor_web::best_address(&ips) else {
        return Err(msg::web_face_not_serving().to_owned());
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
    println!("{}", msg::web_face_here());
    println!("  {url}");
    print!("{}", web_qr(&url));
    println!("{}", msg::web_face_key_note());
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
                let Some(codes) = cell
                    .strip_prefix('[')
                    .and_then(|c| c.split_once('m'))
                    .map(|(c, _)| c)
                else {
                    continue;
                };
                let Some((fg, bg)) = codes.split_once(';') else {
                    continue;
                };
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
        assert!(
            painted.len() >= width + quiet * 2,
            "the block is shorter than the symbol"
        );
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
        for (y, row) in painted.iter().take(quiet).enumerate() {
            assert!(
                row.iter().all(|d| !d),
                "row {y} of the quiet zone is not blank"
            );
        }
        for row in painted.iter().take(width + quiet).skip(quiet) {
            assert!(
                row[..quiet].iter().all(|d| !d),
                "the left margin is not blank"
            );
            assert!(
                row[width + quiet..].iter().all(|d| !d),
                "the right margin is not blank"
            );
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
        return Err(usage_text().into());
    };
    let (served, hosts) = node()?.quit()?;
    println!(
        "{}",
        if served {
            msg::quit_serve_stopped()
        } else {
            msg::quit_no_serve()
        }
    );
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
            let names: std::collections::HashMap<String, String> = n
                .devices()?
                .into_iter()
                .map(|d| (d.id.clone(), d.shown().to_owned()))
                .collect();
            let listed = n.accounts()?;
            if listed.is_empty() {
                println!("{}", cli::accounts_none());
            }
            for a in listed {
                // The kind rides beside the provider because together
                // they are what this row *is* — 「claude sub」 and
                // 「claude bill」 are two different things to register
                // and two different things to read, and a listing that
                // showed only the vendor would spell them the same.
                println!(
                    "{}	{}	{}	{}	{}	{}",
                    a.id,
                    a.name,
                    a.provider.word(),
                    a.kind.word(),
                    names.get(&a.home).cloned().unwrap_or(a.home),
                    a.base_url.unwrap_or_default()
                );
            }
            Ok(())
        }
        Some((word, tail)) if word == "add" => {
            let mut it = tail.iter();
            let name = it.next().ok_or_else(|| usage_text().to_string())?.clone();
            let mut provider: Option<String> = None;
            let mut base_url: Option<String> = None;
            let mut key: Option<String> = None;
            let mut budget: Option<f64> = None;
            let mut kind: Option<String> = None;
            while let Some(flag) = it.next() {
                let mut value = || it.next().ok_or_else(|| usage_text().to_string()).cloned();
                match flag.as_str() {
                    "--provider" => provider = Some(value()?),
                    "--kind" => kind = Some(value()?),
                    "--base-url" => base_url = Some(value()?),
                    "--key" => key = Some(value()?),
                    "--key-stdin" => {
                        let mut text = String::new();
                        std::io::stdin()
                            .read_line(&mut text)
                            .map_err(|e| e.to_string())?;
                        key = Some(text.trim().to_owned());
                    }
                    "--budget" => {
                        budget = Some(
                            value()?
                                .parse::<f64>()
                                .map_err(|_| cli::account_budget_number().to_string())?,
                        );
                    }
                    _ => return Err(usage_text().into()),
                }
            }
            let provider = provider.ok_or_else(|| usage_text().to_string())?;
            let provider = khor_node::Provider::parse(&provider)
                .ok_or_else(|| msg::no_such_provider(&provider))?;
            // **Absent is 「bill」, and a word that is present is
            // parsed** — an unknown one is refused by name rather than
            // falling back to the default, which would register the
            // wrong kind of thing under a word the person chose on
            // purpose.
            let kind = match kind {
                Some(word) => {
                    khor_node::AccountKind::parse(&word).ok_or_else(|| msg::no_such_kind(&word))?
                }
                None => khor_node::AccountKind::Bill,
            };
            // A subscription has no key to give, and the node refuses
            // one that is given — so the *requirement* lifts here while
            // the refusal stays over there, in one voice with the face.
            let key = match key {
                Some(key) => key,
                None if kind.is_sub() => String::new(),
                None => return Err(usage_text().into()),
            };
            let a = n.register_account(&name, provider, kind, base_url.as_deref(), &key, budget)?;
            println!("{}", a.id);
            Ok(())
        }
        Some((word, tail)) if word == "rm" => {
            let [which] = tail else {
                return Err(usage_text().into());
            };
            // By id or by name — a name is what a person remembers, and
            // it is refused when two accounts share it rather than
            // guessed between.
            let all = n.accounts()?;
            let id = if all.iter().any(|a| &a.id == which) {
                which.clone()
            } else {
                let named: Vec<&khor_node::Account> =
                    all.iter().filter(|a| &a.name == which).collect();
                match named.as_slice() {
                    [one] => one.id.clone(),
                    [] => return Err(msg::no_such_account(which)),
                    _ => return Err(cli::account_name_ambiguous(which)),
                }
            };
            n.remove_account(&id)
        }
        Some(_) => Err(usage_text().into()),
    }
}

/// `khor quota` — every registered account's reading, this machine's
/// read now and the others' as last carried here (`Node::quota_board`).
fn quota(rest: &[String]) -> Result<(), String> {
    if let [why] = rest
        && why == "--why"
    {
        return quota_why();
    }
    if !rest.is_empty() {
        return Err(usage_text().into());
    }
    let n = node()?;
    let names: std::collections::HashMap<String, String> =
        n.devices()?.into_iter().map(|d| (d.id, d.name)).collect();
    let board = rt()?.block_on(n.quota_board())?;
    if board.is_empty() {
        println!("{}", cli::accounts_none());
    }
    for (r, age_ms) in board {
        let home = names
            .get(&r.home)
            .cloned()
            .unwrap_or_else(|| r.home.clone());
        let currency = r.currency.clone().unwrap_or_default();
        let figure = match (&r.trouble, r.spent, r.limit, r.remaining) {
            (Some(t), _, _, _) => cli::quota_trouble(t),
            (None, Some(s), Some(l), _) => format!("{s:.2}/{l:.2} {currency}"),
            (None, Some(s), None, _) if r.unlimited => {
                format!("{s:.2} {currency} {}", cli::quota_unlimited())
            }
            (None, Some(s), None, _) => format!("{s:.2} {currency}"),
            // A wallet says what is left and nothing about what went.
            (None, None, _, Some(left)) => {
                cli::quota_remaining(format_args!("{left:.2} {currency}"))
            }
            // **A subscription has no 「spent」 at all**, so 「没有数」
            // would be false here — the number is in the next column,
            // where the share is. What goes in this seat is the plan,
            // the one fact this row has that a billed account has not
            // (empty when the credential did not name one).
            (None, None, _, None) if !r.windows.is_empty() => r.plan.clone().unwrap_or_default(),
            (None, None, _, None) => cli::quota_no_number().to_owned(),
        };
        let share = r
            .fill
            .map(|f| format!("{:.0}%", f * 100.0))
            .unwrap_or_default();
        println!(
            "{}	{}	{}	{}	{}	{}s",
            r.name,
            r.provider,
            home,
            figure,
            share,
            age_ms / 1000
        );
    }
    Ok(())
}

/// `khor quota --why` — this machine's vendors, each read once down its
/// chain (the chain's own cache and cooldown hold, so this asks no more
/// than any other read), and what every strategy did (docs/SHELL.md
/// 第九轮 第 1 条).
fn quota_why() -> Result<(), String> {
    use khor_node::quota::Outcome;
    let rt = rt()?;
    for p in khor_node::quota::providers() {
        let _ = rt.block_on(p.read());
        println!("{}", p.vendor);
        for a in p.last_attempts() {
            let said = match a.outcome {
                Outcome::Unavailable => cli::quota_why_unavailable().to_owned(),
                Outcome::Used => cli::quota_why_used().to_owned(),
                Outcome::Cached { age } => cli::quota_why_cached(age),
                Outcome::Cooling { minutes } => cli::quota_why_cooling(minutes),
                Outcome::Failed(why) => {
                    cli::quota_why_failed(khor_node::accounts::sub_trouble(&why))
                }
            };
            println!("  {:<12} {said}", a.strategy);
        }
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
                println!("{}", cli::agents_none());
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
                return Err(usage_text().into());
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
                        cli::agents_known_startable(),
                        argv.join(" ")
                    ),
                    None => println!("{}\t{}\t{}", k.id, k.name, cli::agents_known_byhand()),
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
                _ => return Err(usage_text().into()),
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
            let (name, argv) = tail.split_first().ok_or_else(|| usage_text().to_string())?;
            let argv: Vec<String> = argv
                .iter()
                .filter(|a| a.as_str() != "--")
                .cloned()
                .collect();
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
            if let Ok(khor_node::AgentAnswer::Sessions { rows }) = n.refresh_agent_sessions(name)
                && !rows.is_empty()
            {
                println!("{}", cli::agents_sessions_asked(name, rows.len()));
            }
            Ok(())
        }
        Some((word, tail)) if word == "rm" => {
            let [name] = tail else {
                return Err(usage_text().into());
            };
            // Said before the removal, because after it the answer is
            // the same either way — and a typo that prints 「不再登记」
            // reads as a success.
            let had = n.agent(name)?.is_some();
            n.forget_agent(name)?;
            println!(
                "{}",
                if had {
                    cli::agents_forgotten(name)
                } else {
                    cli::agents_was_not_there(name)
                }
            );
            Ok(())
        }
        Some(_) => Err(usage_text().into()),
    }
}

/// Three shapes, one verb: asking is the default because it is the safe
/// one, and both writes have to be typed. `khor hooks` alone is also how
/// somebody checks what `install` or `uninstall` just did — a write
/// nobody can verify afterwards is a write on trust.
/// `khor doctor [--json] [--fix <key>]` — the checklist
/// (docs/MORE.md 自检).
///
/// **One line per check, tab-separated**, which is the shape every other
/// listing verb here prints: name, state word, sentence. The `--fix`
/// hint rides a fourth field rather than a line of its own, so a row
/// stays a row for anything reading this with `cut`.
///
/// `--json` prints the node's answer verbatim — the same rows the GUI
/// page draws, so the two faces cannot come to disagree about what was
/// checked.
fn doctor(rest: &[String]) -> Result<(), String> {
    let n = node()?;
    let mut json = false;
    let mut fix: Option<String> = None;
    let mut it = rest.iter();
    while let Some(word) = it.next() {
        match word.as_str() {
            "--json" => json = true,
            "--fix" => fix = Some(it.next().ok_or_else(|| usage_text().to_string())?.clone()),
            _ => return Err(usage_text().into()),
        }
    }
    if let Some(key) = fix {
        println!("{}", khor_node::doctor::fix(&n, &key)?);
        return Ok(());
    }
    let rows = rt()?.block_on(khor_node::doctor::doctor(&n));
    if json {
        println!(
            "{}",
            serde_json::to_string(&rows).map_err(|e| e.to_string())?
        );
        return Ok(());
    }
    for c in &rows {
        // The fix is named by its **check's** key, because that is what
        // `--fix` takes — printing the remedy's own key would be a word
        // this verb does not answer to.
        let fixable = if c.fix.is_some() {
            format!("\t{}", cli::doctor_fixable(&c.key))
        } else {
            String::new()
        };
        println!(
            "{}\t{}\t{}{fixable}",
            khor_node::doctor::name(&c.key),
            c.state.word(),
            c.said
        );
    }
    Ok(())
}

fn hooks(rest: &[String]) -> Result<(), String> {
    let n = node()?;
    match rest {
        [] => {
            let report = n.hooks_report()?;
            println!("{}", cli::hooks_file(report.path.display()));
            let mut incomplete = false;
            for (event, state) in &report.events {
                let said = match state {
                    khor_node::adaptor::claude::Installed::Here => cli::hook_here().to_owned(),
                    khor_node::adaptor::claude::Installed::Missing => {
                        incomplete = true;
                        cli::hook_missing().to_owned()
                    }
                    khor_node::adaptor::claude::Installed::Elsewhere(cmd) => {
                        incomplete = true;
                        cli::hook_elsewhere(cmd)
                    }
                };
                println!("{event}\t{said}");
            }
            if incomplete {
                println!("{}", cli::hooks_worth_installing());
            }
            Ok(())
        }
        [word] if word == "install" => {
            let done = n.install_hooks()?;
            println!("{}", cli::hooks_file(done.path.display()));
            let list = |v: &[&str]| v.join(cli::name_separator());
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
                println!("{}", cli::hooks_restart_claude());
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
                println!("{}", cli::hooks_none_to_remove());
            } else {
                println!(
                    "{}",
                    cli::hooks_removed(done.removed.join(cli::name_separator()))
                );
                println!("{}", cli::hooks_restart_claude());
            }
            Ok(())
        }
        _ => Err(usage_text().into()),
    }
}

fn invite(_rest: &[String]) -> Result<(), String> {
    // The ticket goes to stdout so it can be piped; how long it lasts
    // goes to stderr, because a window nobody is told about is a refusal
    // nobody can explain later.
    println!("{}", node()?.invite()?);
    eprintln!(
        "{}",
        cli::invite_window(khor_node::link::invite_window_minutes())
    );
    Ok(())
}

fn pair(rest: &[String]) -> Result<(), String> {
    let [ticket] = rest else {
        return Err(usage_text().into());
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
        _ => return Err(usage_text().into()),
    };
    let n = node()?;
    let (_, name, tmux) = n.adopt_satellite(host, name)?;
    println!("{}", cli::adopted(&name, n.name()));
    println!(
        "{}",
        if tmux {
            cli::sat_tmux_yes()
        } else {
            cli::sat_tmux_no()
        }
    );
    Ok(())
}

fn sync(_rest: &[String]) -> Result<(), String> {
    let outcomes = rt()?.block_on(node()?.sync_now())?;
    if outcomes.is_empty() {
        println!("{}", cli::nothing_to_sync());
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
    println!("{}", usage_text());
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
    use khor_node::host::{ClientOp, connect_at, write_frame};
    use std::io::{Read, Write};
    use std::sync::atomic::{AtomicBool, Ordering};

    // isatty decides whether this is a terminal; the size is separate —
    // a pty freshly made by script(1) or a GUI reports 0×0, which is a
    // default-worthy size, not a disqualification.
    if unsafe { libc::isatty(0) } != 1 {
        return Err(msg::attach_needs_tty().into());
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
            while let Ok(nread) = stdin.read(&mut buf) {
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
                    if tx
                        .send(ClientOp::Resize {
                            cols: now.0,
                            rows: now.1,
                        })
                        .is_err()
                    {
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
                if out
                    .write_all(&buf[..nread])
                    .and_then(|()| out.flush())
                    .is_err()
                {
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
        match n
            .sessions()
            .ok()
            .and_then(|v| v.into_iter().find(|v| v.session.id == *id))
        {
            Some(v) => match v.session.state.state.key() {
                key @ ("done" | "failed" | "idle") => {
                    eprintln!("{}", cli::session_settled(state::word(key)))
                }
                key => eprintln!("{}", cli::link_dropped(state::word(key), &id.0)),
            },
            None => eprintln!("{}", cli::session_closed()),
        }
    }
    // The stdin thread may still be blocked on read(2); leaving is how
    // this process lets go of it.
    std::process::exit(0);
}

#[cfg(not(unix))]
fn attach_to(_n: &Node, _id: &SessionId) -> Result<(), String> {
    Err(msg::attach_windows_later().into())
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
            return Err(msg::no_termios().into());
        }
        let saved = t;
        libc::cfmakeraw(&mut t);
        if libc::tcsetattr(0, libc::TCSANOW, &t) != 0 {
            return Err(msg::no_raw_mode().into());
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
    let mark = |on: bool| {
        if on {
            format!("\t{}", cli::in_use())
        } else {
            String::new()
        }
    };
    println!("{}", cli::group_header(avatar::axis_palette()));
    println!("{}", style.palette.colors().join("\t"));
    let worn = style.palette.preset_id();
    for p in PRESETS {
        println!(
            "{}\t{}{}",
            p.id,
            avatar::word(p.id),
            mark(worn == Some(p.id))
        );
    }
    println!("{}", cli::group_header(avatar::axis_variant()));
    for v in Variant::ALL {
        println!(
            "{}\t{}{}",
            v.key(),
            avatar::word(v.key()),
            mark(v == style.variant)
        );
    }
    println!("{}", cli::group_header(avatar::axis_shape()));
    for s in FaceShape::ALL {
        println!(
            "{}\t{}{}",
            s.key(),
            avatar::word(s.key()),
            mark(s == style.shape)
        );
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
        return cli::group_pinned().to_owned();
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
    group
        .strip_prefix(list::GROUP_DEVICE)
        .unwrap_or(group)
        .to_owned()
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
        return cli::retracted().into();
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
    use super::{NOT_IN_USAGE, NOTHING_TYPED, Reach, VERBS};
    // **Aliased**, because this file already has a verb called `usage`
    // (`khor usage`, what each machine spent). The catalog's is the help
    // text; 批 D3 turned it from a const into a function, which is when the
    // two names met.
    use khor_catalog::cli::usage as usage_text;

    /// The words the usage promises, read back out of the text a person
    /// reads.
    ///
    /// An entry starts at the list's left margin — two spaces, then the
    /// word; a description that wraps is indented past it. Reading the
    /// prose is the point: fed from a second, tidier list, this would
    /// only prove the two lists agree, and the text somebody actually
    /// reads could still say anything.
    fn promised() -> Vec<&'static str> {
        usage_text()
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

    /// `Inner` and the `_` prefix are two spellings of one decision, and
    /// they are written in different places — the row says `Inner`, the
    /// word says `_`. Either can be edited without the other, and both
    /// mistakes are quiet: a public verb marked `Inner` is one an agent
    /// is refused for no stated reason, and an `_` word marked `Act` is
    /// a calling convention offered to an agent as if it were a verb.
    #[test]
    fn inner_is_exactly_the_underscored_verbs() {
        for v in VERBS {
            assert_eq!(
                v.reach == Reach::Inner,
                v.word.starts_with('_'),
                "`{}` is {:?} and {} underscored; those must agree",
                v.word,
                v.reach,
                if v.word.starts_with('_') {
                    "is"
                } else {
                    "is not"
                }
            );
        }
    }

    /// `verbs` writes its JSON by hand, which is safe exactly as long as
    /// no word needs escaping. That is true today and nothing enforces
    /// it, so this does — a verb spelled with a quote would otherwise
    /// ship JSON that the gate reading it cannot parse, and the failure
    /// would land over there, in a language away from the cause.
    #[test]
    fn every_verb_can_be_written_into_json_unescaped() {
        for v in VERBS {
            assert!(
                !v.word.contains('"')
                    && !v.word.contains('\\')
                    && !v.word.contains(|c: char| c.is_control()),
                "`{}` needs escaping, and `verbs` does not escape",
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
