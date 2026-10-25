//! Request context for the CLI, slash composer, and future agent tools.
//! Output is delivered at the print boundary; no process stream is redirected.
use serde::Serialize;
use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Clone, Debug, Serialize)]
pub struct CommandSpec {
    pub word: String,
    pub aliases: Vec<String>,
    pub usage: String,
    pub reach: String,
    pub mode: String,
    pub parameters: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct CommandOutcome {
    pub stdout: String,
    pub stderr: String,
    pub data: Option<serde_json::Value>,
    pub session: Option<String>,
    pub exit_code: i32,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CommandProgress {
    pub stream: String,
    pub text: String,
}

pub type ProgressSink = Arc<dyn Fn(CommandProgress) + Send + Sync>;
struct Request {
    root: PathBuf,
    input: Option<std::io::Cursor<Vec<u8>>>,
    outcome: CommandOutcome,
    emit: Option<ProgressSink>,
    stdout_cut: bool,
    stderr_cut: bool,
}
thread_local! {
    pub(crate) static CLI_ROOT: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
    static REQUEST: RefCell<Option<Request>> = const { RefCell::new(None) };
    pub(crate) static START_WEB: Cell<Option<fn(&Path)>> = const { Cell::new(None) };
    pub(crate) static NATIVE_OUTPUT: Cell<Option<fn(bool, &str)>> = const { Cell::new(None) };
    pub(crate) static CLI_EXIT: Cell<i32> = const { Cell::new(0) };
}

pub(crate) fn in_request() -> bool {
    REQUEST.with(|r| r.borrow().is_some())
}
pub(crate) fn command_root() -> PathBuf {
    REQUEST
        .with(|r| r.borrow().as_ref().map(|r| r.root.clone()))
        .or_else(|| CLI_ROOT.with(|root| root.borrow().clone()))
        .unwrap_or_else(khor_node::Node::root_from_env)
}
pub(crate) fn navigate(session: &str) {
    REQUEST.with(|r| {
        if let Some(r) = r.borrow_mut().as_mut() {
            r.outcome.session = Some(session.to_owned());
        }
    });
}
pub(crate) fn emit_output(stderr: bool, args: std::fmt::Arguments<'_>) -> bool {
    const LIMIT: usize = 128 * 1024;
    let event = REQUEST.with(|slot| {
        let mut slot = slot.borrow_mut();
        let r = slot.as_mut()?;
        let (output, cut) = if stderr {
            (&mut r.outcome.stderr, &mut r.stderr_cut)
        } else {
            (&mut r.outcome.stdout, &mut r.stdout_cut)
        };
        if *cut {
            return Some((r.emit.clone(), String::new()));
        }
        let mut text = args.to_string();
        if output.len().saturating_add(text.len()) > LIMIT {
            let mut take = LIMIT.saturating_sub(output.len());
            while !text.is_char_boundary(take) {
                take -= 1;
            }
            text.truncate(take);
            text.push('\n');
            text.push_str(khor_catalog::msg::console_output_cut());
            text.push('\n');
            *cut = true;
        }
        output.push_str(&text);
        Some((r.emit.clone(), text))
    });
    match event {
        None => false,
        Some((sink, text)) => {
            if !text.is_empty() {
                if let Some(sink) = sink {
                    sink(CommandProgress {
                        stream: if stderr { "stderr" } else { "stdout" }.into(),
                        text,
                    });
                }
            }
            true
        }
    }
}
pub(crate) fn record_data(value: serde_json::Value) {
    REQUEST.with(|slot| {
        if let Some(r) = slot.borrow_mut().as_mut() {
            r.outcome.data = Some(value);
        }
    });
}
pub(crate) fn native_output(stderr: bool, args: std::fmt::Arguments<'_>) {
    use std::io::Write;
    NATIVE_OUTPUT.with(|slot| match slot.get() {
        Some(write) => write(stderr, &args.to_string()),
        None if stderr => {
            let _ = std::io::stderr().lock().write_fmt(args);
        }
        None => {
            let _ = std::io::stdout().lock().write_fmt(args);
        }
    });
}
pub(crate) fn err(args: std::fmt::Arguments<'_>) {
    if !emit_output(true, args) {
        native_output(true, args);
    }
}
pub(crate) fn json_output<T: Serialize>(value: &T) -> Result<String, String> {
    let data = serde_json::to_value(value).map_err(|e| e.to_string())?;
    REQUEST.with(|slot| {
        if let Some(r) = slot.borrow_mut().as_mut() {
            r.outcome.data = Some(data.clone());
        }
    });
    serde_json::to_string(&data).map_err(|e| e.to_string())
}
pub(crate) fn input_line(text: &mut String) -> std::io::Result<usize> {
    use std::io::BufRead;
    REQUEST.with(|slot| match slot.borrow_mut().as_mut() {
        Some(r) => r
            .input
            .as_mut()
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    khor_catalog::msg::command_input_required(),
                )
            })?
            .read_line(text),
        None => std::io::stdin().read_line(text),
    })
}
pub(crate) fn input_all(text: &mut String) -> std::io::Result<usize> {
    use std::io::Read;
    REQUEST.with(|slot| match slot.borrow_mut().as_mut() {
        Some(r) => r
            .input
            .as_mut()
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    khor_catalog::msg::command_input_required(),
                )
            })?
            .read_to_string(text),
        None => std::io::stdin().read_to_string(text),
    })
}

/// Split command arguments without shell expansion or execution. Backslashes
/// remain literal in Windows paths unless they escape a quote or whitespace.
pub fn parse_line(line: &str) -> Result<Vec<String>, String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quote = None;
    let mut started = false;
    let mut chars = line.trim().chars().peekable();
    while let Some(c) = chars.next() {
        match (quote, c) {
            (Some(q), c) if q == c => quote = None,
            (Some('\''), c) => word.push(c),
            (_, '\\') => {
                let escaped = chars
                    .peek()
                    .copied()
                    .filter(|next| next.is_whitespace() || *next == '"' || *next == '\'');
                if let Some(next) = escaped {
                    chars.next();
                    word.push(next);
                } else {
                    word.push(c);
                }
                started = true;
            }
            (None, '\'' | '"') => {
                quote = Some(c);
                started = true;
            }
            (None, c) if c.is_whitespace() => {
                if started {
                    words.push(std::mem::take(&mut word));
                    started = false;
                }
            }
            (_, c) => {
                word.push(c);
                started = true;
            }
        }
    }
    if quote.is_some() {
        return Err(khor_catalog::cli::usage().to_owned());
    }
    if started {
        words.push(word);
    }
    if let Some(first) = words.first_mut() {
        if let Some(verb) = first.strip_prefix('/') {
            *first = verb.into();
        }
    }
    Ok(words)
}

/// Whether an invocation may carry credentials. Callers must keep both its
/// arguments and results local instead of saving them in replicated history.
pub fn private_invocation(argv: &[String]) -> bool {
    matches!(
        argv.first().map(String::as_str),
        Some(
            "root"
                | "lock"
                | "accounts"
                | "web"
                | "code"
                | "pair"
                | "add"
                | "carry"
                | "invite"
                | "id"
        )
    ) || argv.iter().any(|a| {
        let flag = a.split_once('=').map_or(a.as_str(), |(flag, _)| flag);
        matches!(
            flag,
            "--password" | "--passphrase" | "--key" | "--key-stdin" | "--token" | "--tls-key"
        )
    })
}

pub fn execute(root: &Path, argv: &[String], input: Option<&str>) -> CommandOutcome {
    execute_with(root, argv, input, None)
}

/// The observer runs synchronously on the executing thread. Tokio work is
/// awaited on that thread, so its returned progress uses the same context.
pub fn execute_with(
    root: &Path,
    argv: &[String],
    input: Option<&str>,
    emit: Option<ProgressSink>,
) -> CommandOutcome {
    struct Scope(Option<Request>, i32);
    impl Drop for Scope {
        fn drop(&mut self) {
            REQUEST.with(|slot| {
                *slot.borrow_mut() = self.0.take();
            });
            CLI_EXIT.with(|code| code.set(self.1));
        }
    }
    let previous = REQUEST.with(|slot| {
        slot.replace(Some(Request {
            root: root.to_path_buf(),
            input: input.map(|s| std::io::Cursor::new(s.as_bytes().to_vec())),
            outcome: CommandOutcome::default(),
            emit,
            stdout_cut: false,
            stderr_cut: false,
        }))
    });
    let previous_exit = CLI_EXIT.with(|code| code.replace(0));
    let _scope = Scope(previous, previous_exit);
    let result = execute_inner(argv);
    REQUEST.with(|slot| {
        let mut request = slot.borrow_mut();
        let outcome = &mut request.as_mut().expect("execution scope").outcome;
        outcome.exit_code = CLI_EXIT.with(|code| code.get());
        if let Err(error) = result {
            if outcome.exit_code == 0 {
                outcome.exit_code = 1;
            }
            outcome.error = Some(error);
        }
        outcome.clone()
    })
}

fn execute_inner(argv: &[String]) -> Result<(), String> {
    let word = argv.first().map(String::as_str).unwrap_or("help");
    let verb = super::VERBS
        .iter()
        .find(|v| v.word == word && v.reach != super::Reach::Inner)
        .ok_or_else(|| khor_catalog::msg::unknown_verb(word))?;
    if matches!(word, "serve" | "mcp" | "pipe")
        || word == "relay" && !argv.iter().any(|a| a == "--stats" || a == "--tls-check")
    {
        let rest = &argv[1..];
        validate_stream(word, rest)?;
        let node = super::node()?;
        let root = std::fs::canonicalize(command_root())
            .map_err(|e| khor_catalog::msg::cant_read(command_root().display(), e))?;
        let executable = khor_node::self_exe()?;
        let mut command = vec![
            executable.to_string_lossy().into_owned(),
            "_console".into(),
            root.to_string_lossy().into_owned(),
        ];
        command.extend_from_slice(argv);
        let title = format!("khor {word}");
        let id = node.open_persistent(khor_node::kind::TUI, &title, &command, (80, 24))?;
        navigate(&id.0);
        super::out(format_args!("{}\n", id.0));
        return Ok(());
    }
    (verb.run)(&argv[argv.len().min(1)..])
}

pub(crate) fn validate_stream(word: &str, rest: &[String]) -> Result<(), String> {
    match word {
        "pipe" => match rest {
            [flag, _, _, port] if flag == "--via" && port.parse::<u16>().is_ok() => Ok(()),
            _ => Err(khor_catalog::cli::usage().to_owned()),
        },
        "relay" => super::relay::validate(rest),
        "serve" | "mcp" if rest.is_empty() => Ok(()),
        _ => Err(khor_catalog::cli::usage().to_owned()),
    }
}

pub fn catalog() -> Vec<CommandSpec> {
    super::VERBS
        .iter()
        .filter(|v| {
            v.reach != super::Reach::Inner
                && !matches!(v.word, "face" | "--help" | "-h" | "--version" | "-V")
        })
        .map(|v| {
            let usage = grammar(v.word).to_owned();
            CommandSpec {
                word: v.word.into(),
                aliases: match v.word {
                    "avatar" => vec!["face".into()],
                    "version" => vec!["--version".into(), "-V".into()],
                    "help" => vec!["--help".into(), "-h".into()],
                    _ => vec![],
                },
                parameters: usage
                    .split_whitespace()
                    .skip(1)
                    .map(str::to_owned)
                    .collect(),
                usage,
                reach: v.reach.word().into(),
                mode: match v.word {
                    "run" | "open" | "attach" | "takeover" | "borrow" => "navigation",
                    "serve" | "relay" | "mcp" | "pipe" => "stream",
                    _ => "finite",
                }
                .into(),
            }
        })
        .collect()
}

fn grammar(word: &str) -> &'static str {
    match word {
        "id" => "id [--reset <machine>]",
        "avatar" => {
            "avatar [<machine>] [--palette <palette>] [--colors <color> <color> <color> <color> <color>] [--variant <variant>] [--shape <shape>]"
        }
        "devices" => "devices [--roads]",
        "top" => "top [<machine>]",
        "ls" => "ls <machine> [<path>]",
        "pull" => "pull <machine> <file> [<directory>]",
        "drop" => "drop [-m <machine>] <file> [--to <session>|--on <machine>] [--into <dir>]",
        "git" => "git status <machine> <path> | git diff <machine> <file>",
        "put" => "put [--new|--base <file>] <machine> <file> <path>",
        "rm" => "rm [-m <machine>] <path>",
        "mkdir" => "mkdir [-m <machine>] <path>",
        "touch" => "touch [-m <machine>] <path>",
        "mv" => "mv [-m <machine>] <path> <path>",
        "launch" => "launch [-m <machine>] <path|url>",
        "search" => "search [-m <machine>] <directory> <word>",
        "code" => "code [--stop] <machine> [<path>]",
        "read" => "read <machine> <file>",
        "places" => "places [<machine>]",
        "ws" => "ws ls | ws add|rm|pin|unpin [-m <machine>] <directory>",
        "borrow" => "borrow <machine>",
        "reach" => "reach <host> [--port <port>]",
        "pipe" => "pipe --via <machine> <host> <port>",
        "carry" => {
            "carry <machine> <host> [--port <port>] [--via <host>] [--password|--key <file> [--passphrase]] | carry <machine> --off"
        }
        "tabs" => "tabs [ls] | tabs open [-m <machine>] <url> | tabs close|focus|pin|unpin <tab>",
        "usage" => "usage [--days <number>]",
        "sessions" => "sessions [--by recent|category|device|state|place] [--all]",
        "find" => "find <word>",
        "import" => "import [<number|id>...]",
        "tell" => "tell <machine>|@<machine>[:<dir>]... <words>...",
        "send" => {
            "send <machine>|@<machine>[:<dir>]... <file>|<machine>:<path>|<words>... [--into <dir>]"
        }
        "retry" => "retry <transfer>",
        "log" => "log [<machine>]",
        "run" => "run [--tui] [--title <title>] -- <command>...",
        "open" => {
            "open [-d] [--tui|--gui] [--agent <agent> [--resume <id>]] [--on <machine>] [--title <title>] [-- <command>...]"
        }
        "attach" => "attach <session>",
        "takeover" => "takeover <session>",
        "agents" => {
            "agents [known|sessions <agent> [--ask]|add <agent> -- <command>...|rm <agent>]"
        }
        "accounts" => {
            "accounts [ls|add <name> --provider <provider> [--kind sub|bill] [--base-url <url>] [--budget <number>] [--key <key>|--key-stdin]|rm <account>]"
        }
        "quota" => "quota [--why]",
        "state" => "state <state> [--session <session>] | state --hook",
        "seen" => "seen <session>",
        "rename" => "rename [-m] <session|machine> [<name>...]",
        "pin" => "pin [-m] <session|machine>",
        "unpin" => "unpin [-m] <session|machine>",
        "close" => "close <session>",
        "serve" => "serve",
        "relay" => {
            "relay [--dev|--tls-cert <file> --tls-key <file>|--acme <host> [--acme-contact <email>]] [--token <key>|--only-mine] [--rate <number>] [--metrics <address>] [--bind <address>] [--http <address>] [--quic <address>] [--state <directory>] [--stats|--tls-check]"
        }
        "web" => "web [--new]",
        "gui" => "gui",
        "faces" => "faces",
        "quit" => "quit",
        "hooks" => "hooks [install|uninstall]",
        "doctor" => "doctor [--json] [--fix <check>]",
        "journal" => {
            "journal [<machine>] [--minutes <number>] [--report] [--json] [--on <machine>]"
        }
        "lock" => "lock [status|on|off|pin|timeout <number>]",
        "config" => "config [list|get <key>|set <key> <value>]",
        "invite" => "invite",
        "pair" => "pair <ticket|->",
        "add" => {
            "add <host> [<name>] [--port <port>] [--via <host>] [-m <machine>] [--password|--key <file> [--passphrase]] [--no-install]"
        }
        "promote" => "promote <machine>",
        "pending" => "pending [--json]",
        "approve" => "approve <machine>",
        "revoke" => "revoke <machine>",
        "sync" => "sync",
        "mcp" => "mcp",
        "version" => "version",
        "root" => "root [--verify-owner|show [--json]|words <machine>|recover <words>...|init]",
        "forget" => "forget <machine>",
        "leave" => "leave <machine>",
        "pause" => "pause [<machine>]",
        "resume" => "resume [<machine>]",
        "watch" => "watch <machine> [<path>...]",
        "help" => "help",
        _ => unreachable!("a public command must have grammar: {word}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tokenizer_preserves_arguments_without_shell_expansion() {
        assert_eq!(
            parse_line(r#"/put lab 'a file.txt' "C:\Users\Name\a file.txt""#).unwrap(),
            ["put", "lab", "a file.txt", r"C:\Users\Name\a file.txt"]
        );
        assert_eq!(
            parse_line(r#"tell lab '$HOME' "$(touch /tmp/no)""#).unwrap(),
            ["tell", "lab", "$HOME", "$(touch /tmp/no)"]
        );
        assert_eq!(
            parse_line("config set x ''").unwrap(),
            ["config", "set", "x", ""]
        );
        assert!(parse_line("tell 'unfinished").is_err());
    }
    #[test]
    fn every_public_verb_has_one_catalog_entry_and_internal_verbs_stay_private() {
        let rows = catalog();
        assert_eq!(rows.len(), 73);
        for row in &rows {
            assert!(!row.word.starts_with('_'));
            assert!(row.usage.starts_with(&row.word));
        }
        for v in super::super::VERBS {
            if v.reach == super::super::Reach::Inner {
                continue;
            }
            assert_eq!(
                rows.iter()
                    .filter(|r| r.word == v.word || r.aliases.iter().any(|a| a == v.word))
                    .count(),
                1,
                "{}",
                v.word
            );
        }
        let result = execute(Path::new("/unused"), &["_host".into()], None);
        assert!(result.error.is_some());
    }
    #[test]
    fn root_is_scoped_and_output_is_request_local() {
        let root = Path::new("/tmp/khor-explicit-command-root");
        let result = execute(root, &["root".into()], None);
        assert_eq!(result.stdout.trim(), root.to_str().unwrap());
        assert!(result.error.is_none());
        assert_eq!(command_root(), khor_node::Node::root_from_env());
        let version = execute(root, &["--version".into()], None);
        assert_eq!(version.stdout.trim(), khor_node::VERSION);
        assert!(!version.stdout.contains("explicit-command-root"));
    }

    #[test]
    fn root_owner_receipt_is_strict_scoped_and_read_only() {
        struct Scratch(PathBuf);
        impl Drop for Scratch {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "khor-command-owner-{}",
            khor_node::link::fresh_hex().unwrap()
        ));
        let scratch = Scratch(root.clone());
        let store = root.join(".khor");
        std::fs::create_dir_all(&store).unwrap();
        let fingerprint = khor_node::hardware::fingerprint();
        let owner = format!(
            "machine {}\nname previous-name\n",
            fingerprint.as_deref().unwrap_or("unverifiable-test-machine")
        );
        std::fs::write(store.join("owner"), &owner).unwrap();
        let args = ["root".into(), "--verify-owner".into()];
        let verified = execute(&root, &args, None);
        assert_eq!(
            verified.error.is_none(),
            fingerprint.is_some(),
            "{:?}",
            verified.error
        );
        let expected = if fingerprint.is_some() {
            format!("khor-store-owner-v1\n{}\n", root.display())
        } else {
            String::new()
        };
        assert_eq!(verified.stdout, expected);
        assert_eq!(std::fs::read_to_string(store.join("owner")).unwrap(), owner);
        assert_eq!(std::fs::read_dir(&store).unwrap().count(), 1);
        assert!(!store.join("identity.key").exists());

        for rest in [
            vec!["--verify-owner", "extra"],
            vec!["--verify-owner", "--json"],
            vec!["--verify-owner", "--verify-owner"],
            vec!["--verify-owner=true"],
        ] {
            let args = std::iter::once("root")
                .chain(rest)
                .map(str::to_owned)
                .collect::<Vec<_>>();
            let result = execute(&root, &args, None);
            assert!(result.error.is_some(), "{args:?}");
            assert!(result.stdout.is_empty(), "{args:?}");
        }
        let foreign = format!(
            "machine not-{}\nname previous-name\n",
            fingerprint.as_deref().unwrap_or("known")
        );
        std::fs::write(store.join("owner"), &foreign).unwrap();
        let denied = execute(&root, &args, None);
        assert!(denied.error.is_some());
        assert!(denied.stdout.is_empty());
        assert_eq!(std::fs::read_to_string(store.join("owner")).unwrap(), foreign);
        assert!(!store.join("identity.key").exists());
        std::fs::remove_file(store.join("owner")).unwrap();
        let missing = execute(&root, &args, None);
        assert!(missing.error.is_some());
        assert!(missing.stdout.is_empty());
        assert_eq!(std::fs::read_dir(&store).unwrap().count(), 0);
        assert!(!store.join("identity.key").exists());
        drop(scratch);
    }
    #[test]
    fn concurrent_requests_keep_roots_inputs_and_output_separate() {
        let base = std::env::temp_dir().join(format!(
            "khor-command-scope-{}",
            khor_node::link::fresh_hex().unwrap()
        ));
        let barrier = Arc::new(std::sync::Barrier::new(4));
        let workers: Vec<_> = (0..4)
            .map(|i| {
                let root = base.join(i.to_string());
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    let theme = if i % 2 == 0 { "dark" } else { "light" };
                    let set = execute(
                        &root,
                        &["config".into(), "set".into(), "theme".into(), theme.into()],
                        None,
                    );
                    assert!(set.error.is_none(), "{:?}", set.error);
                    assert_eq!(set.stdout.trim(), theme);
                    let got = execute(
                        &root,
                        &["config".into(), "get".into(), "theme".into()],
                        None,
                    );
                    assert_eq!(got.stdout.trim(), theme);
                    let root_result = execute(&root, &["root".into()], None);
                    assert_eq!(root_result.stdout.trim(), root.to_str().unwrap());
                    let pin = execute(&root, &["lock".into(), "pin".into()], Some("739251\n"));
                    assert!(pin.error.is_none(), "{:?}", pin.error);
                    assert!(!pin.stdout.contains("739251"));
                    assert!(!pin.stderr.contains("739251"));
                    assert!(khor_node::lock::check_pin(&root, "739251").unwrap());
                    assert_eq!(command_root(), khor_node::Node::root_from_env());
                })
            })
            .collect();
        for worker in workers {
            worker.join().unwrap();
        }
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn required_input_and_bad_stream_arguments_return_without_starting_a_session() {
        let root = Path::new("/unused");
        let missing = execute(root, &["lock".into(), "pin".into()], None);
        assert!(
            missing
                .error
                .unwrap()
                .contains(khor_catalog::msg::command_input_required())
        );
        for args in [
            vec!["pipe", "--via", "lab", "host", "NaN"],
            vec!["relay"],
            vec!["relay", "--unknown"],
            vec!["serve", "--unknown"],
        ] {
            let args: Vec<String> = args.into_iter().map(str::to_owned).collect();
            let result = execute(root, &args, None);
            assert!(result.error.is_some(), "{args:?}");
            assert!(result.session.is_none());
        }
    }

    #[test]
    fn progress_and_results_share_a_utf8_safe_output_limit() {
        let root = PathBuf::from(format!("/{}", "界".repeat(80_000)));
        let events = Arc::new(std::sync::Mutex::new(Vec::<CommandProgress>::new()));
        let keep = events.clone();
        let result = execute_with(
            &root,
            &["root".into()],
            None,
            Some(Arc::new(move |event| keep.lock().unwrap().push(event))),
        );
        assert!(result.error.is_none());
        assert!(
            result
                .stdout
                .contains(khor_catalog::msg::console_output_cut())
        );
        assert!(result.stdout.len() < 129 * 1024);
        assert_eq!(
            events
                .lock()
                .unwrap()
                .iter()
                .map(|e| e.text.as_str())
                .collect::<String>(),
            result.stdout
        );
    }

    #[test]
    fn credential_commands_are_private_even_when_they_only_read() {
        for verb in [
            "root", "lock", "accounts", "web", "code", "pair", "add", "carry", "invite", "id",
        ] {
            assert!(private_invocation(&[verb.into()]));
        }
        assert!(private_invocation(&[
            "relay".into(),
            "--token".into(),
            "secret".into()
        ]));
        assert!(private_invocation(&[
            "relay".into(),
            "--token=secret".into()
        ]));
        // Starting code returns a URL containing the server's access key.
        assert!(private_invocation(&[
            "code".into(),
            "lab".into(),
            "/repo".into()
        ]));
        assert!(private_invocation(&[
            "code".into(),
            "--stop".into(),
            "lab".into()
        ]));
        assert!(!private_invocation(&["sessions".into()]));
        assert!(!private_invocation(&["devices".into(), "--roads".into()]));
    }
    #[test]
    fn failed_tls_checks_and_native_exit_state_cannot_report_success() {
        CLI_EXIT.with(|code| code.set(42));
        let result = execute(
            Path::new("/unused"),
            &[
                "relay".into(),
                "--tls-check".into(),
                "--tls-cert".into(),
                "/missing-khor-console-cert.pem".into(),
                "--tls-key".into(),
                "/missing-khor-console-key.pem".into(),
            ],
            None,
        );
        assert_eq!(result.exit_code, 1);
        assert!(result.error.is_some());
        assert!(result.session.is_none());
        assert_eq!(CLI_EXIT.with(|code| code.get()), 42);
        let clean = execute(Path::new("/unused"), &["version".into()], None);
        assert_eq!(clean.exit_code, 0);
        let native = execute_with(
            Path::new("/unused"),
            &["version".into()],
            None,
            Some(Arc::new(|_| CLI_EXIT.with(|code| code.set(7)))),
        );
        assert_eq!(native.exit_code, 7);
        CLI_EXIT.with(|code| code.set(0));
    }
}
