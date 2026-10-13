//! Read-only completion for the command console. Ranges use JavaScript's
//! UTF-16 offsets; values replace a whole shell word (including quotes).
use serde::Serialize;
use std::path::{Path, PathBuf};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, TS)]
pub struct Completion {
    pub start: usize,
    pub end: usize,
    pub choices: Vec<Choice>,
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct Choice {
    pub value: String,
    pub label: String,
    pub detail: String,
    pub kind: String,
}

#[derive(Debug)]
struct Word {
    start: usize,
    end: usize,
}

// Completion must accept unfinished quotes, unlike command execution.
fn words(line: &str) -> Vec<Word> {
    let mut out = Vec::new();
    let (mut start, mut quote) = (None, None);
    let mut chars = line.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if c == '\\' && quote != Some('\'') {
            start.get_or_insert(i);
            if chars
                .peek()
                .is_some_and(|(_, c)| c.is_whitespace() || matches!(c, '\'' | '"'))
            {
                chars.next();
            }
        } else if let Some(q) = quote {
            if c == q {
                quote = None;
            }
        } else if c == '\'' || c == '"' {
            start.get_or_insert(i);
            quote = Some(c);
        } else if c.is_whitespace() {
            if let Some(s) = start.take() {
                out.push(Word { start: s, end: i });
            }
        } else {
            start.get_or_insert(i);
        }
    }
    if let Some(s) = start {
        out.push(Word {
            start: s,
            end: line.len(),
        });
    }
    out
}

fn unquote(raw: &str) -> String {
    let (mut out, mut quote) = (String::new(), None);
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' && quote != Some('\'') {
            if chars
                .peek()
                .is_some_and(|c| c.is_whitespace() || matches!(c, '\'' | '"'))
            {
                out.push(chars.next().unwrap());
            } else {
                out.push(c);
            }
        } else if let Some(q) = quote {
            if c == q {
                quote = None;
            } else {
                out.push(c);
            }
        } else if c == '\'' || c == '"' {
            quote = Some(c);
        } else {
            out.push(c);
        }
    }
    out
}

fn quoted(value: &str, force: bool) -> String {
    if value.contains('\\') {
        // Single quotes preserve literal backslashes in parse_line. Adjacent
        // quote segments preserve apostrophes without relying on shell syntax.
        format!("'{}'", value.replace('\'', "'\"'\"'"))
    } else if force
        || value
            .chars()
            .any(|c| c.is_whitespace() || matches!(c, '\'' | '"'))
    {
        format!("\"{}\"", value.replace('"', "\\\""))
    } else {
        value.to_owned()
    }
}

fn byte_at_utf16(line: &str, caret: usize) -> usize {
    let mut units = 0;
    for (i, c) in line.char_indices() {
        if units + c.len_utf16() > caret {
            return i;
        }
        units += c.len_utf16();
    }
    line.len()
}

fn choice(value: String, label: String, detail: String, kind: &str) -> Choice {
    Choice {
        value,
        label,
        detail,
        kind: kind.into(),
    }
}

// Literal subwords and options come from the same grammar as CLI help.
fn literals(usage: &str) -> Vec<String> {
    let mut clean = String::new();
    let mut placeholder = false;
    for c in usage.chars() {
        match c {
            '<' => {
                placeholder = true;
                clean.push(' ');
            }
            '>' => {
                placeholder = false;
                clean.push(' ');
            }
            _ if placeholder => {}
            '[' | ']' | '|' | '(' | ')' => clean.push(' '),
            _ => clean.push(c),
        }
    }
    let command = usage.split_whitespace().next().unwrap_or("");
    clean
        .split_whitespace()
        .skip(1)
        .filter(|s| *s != "..." && *s != command && !s.contains(':') && !s.contains('@'))
        .map(|s| s.trim_end_matches('.').to_owned())
        .filter(|s| !s.is_empty())
        .collect()
}

#[derive(Debug, PartialEq)]
struct PathRequest {
    machine: String,
    typed: String,
    head: String,
    dirs_only: bool,
    local_relative: bool,
}

fn flag_value<'a>(args: &'a [String], flags: &[&str]) -> Option<&'a str> {
    args.windows(2)
        .find(|w| flags.contains(&w[0].as_str()))
        .map(|w| w[1].as_str())
}

fn path_request(command: &str, args: &[String], prefix: &str, _usage: &str) -> Option<PathRequest> {
    // machine:path also works inside a quoted word; URLs are never paths.
    if prefix.contains("://") {
        return None;
    }
    let windows_drive = prefix.as_bytes().get(1) == Some(&b':')
        && prefix
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphabetic)
        && matches!(prefix.as_bytes().get(2), Some(b'/' | b'\\'));
    if !windows_drive && matches!(command, "send" | "tell") {
        if let Some((machine, path)) = prefix.split_once(':') {
            if !machine.is_empty() && !machine.contains('/') {
                return Some(PathRequest {
                    machine: machine.trim_start_matches('@').into(),
                    typed: path.into(),
                    head: format!("{machine}:"),
                    dirs_only: matches!(command, "tell") || prefix.starts_with('@'),
                    local_relative: false,
                });
            }
        }
    }
    let prev = args.last().map(String::as_str).unwrap_or("");
    let explicit_machine = flag_value(args, &["-m", "--machine"]).unwrap_or("");
    // Remove flags and their values when locating positional file arguments.
    let mut positionals = Vec::new();
    let mut skip = false;
    for arg in args {
        if skip {
            skip = false;
            continue;
        }
        if arg.starts_with('-') {
            skip = matches!(
                arg.as_str(),
                "-m" | "--machine"
                    | "--to"
                    | "--on"
                    | "--into"
                    | "--base"
                    | "--key"
                    | "--via"
                    | "--title"
                    | "--agent"
                    | "--resume"
            );
        } else {
            positionals.push(arg.as_str());
        }
    }
    let count = positionals.len();
    let (machine, dirs_only) = if prev == "--into" {
        (
            if command == "send" {
                positionals
                    .first()
                    .copied()
                    .unwrap_or("")
                    .trim_start_matches('@')
                    .split(':')
                    .next()
                    .unwrap_or("")
            } else {
                flag_value(args, &["--on"]).unwrap_or("")
            },
            true,
        )
    } else if prev == "--base" || prev == "--key" {
        ("", false)
    } else {
        match command {
            "ls" if count == 1 => (positionals[0], true),
            "pull" if count == 1 => (positionals[0], false),
            "pull" if count == 2 => ("", true),
            "read" | "code" if count == 1 => (positionals[0], command == "code"),
            "git" if count == 2 => (positionals[1], positionals[0] == "status"),
            "put" if count == 1 => ("", false),
            "put" if count == 2 => (positionals[0], false),
            "ws" if count == 1 && matches!(positionals[0], "add" | "rm" | "pin" | "unpin") => {
                (explicit_machine, true)
            }
            "rm" | "mkdir" | "touch" | "launch" | "drop" if count == 0 => {
                (explicit_machine, command == "mkdir")
            }
            "search" if count == 0 => (explicit_machine, true),
            "mv" if count < 2 => (explicit_machine, false),
            "send" if count >= 1 => ("", false),
            _ => return None,
        }
    };
    if prev.starts_with('-') && !matches!(prev, "--into" | "--base" | "--key") {
        return None;
    }
    if prefix.starts_with('-') {
        return None;
    }
    Some(PathRequest {
        machine: machine.into(),
        typed: prefix.into(),
        head: String::new(),
        dirs_only,
        local_relative: machine.is_empty(),
    })
}

fn needs_machine(command: &str, args: &[String], usage: &str) -> bool {
    let prev = args.last().map(String::as_str).unwrap_or("");
    if matches!(prev, "-m" | "--machine" | "--on" | "--via")
        || (prev.starts_with('-') && usage.contains(&format!("{prev} <machine>")))
    {
        return true;
    }
    if matches!(command, "pin" | "unpin" | "rename") && args.iter().any(|x| x == "-m") {
        return true;
    }
    if command == "code" && args.iter().all(|a| a == "--stop") {
        return true;
    }
    if command == "git" {
        return args.len() == 1;
    }
    if matches!(command, "send" | "tell") {
        return true;
    }
    args.is_empty()
        && (usage.starts_with(&format!("{command} <machine>"))
            || usage.starts_with(&format!("{command} [<machine>]"))
            || usage.starts_with(&format!("{command} [machine]")))
}

fn needs_session(command: &str, args: &[String], usage: &str) -> bool {
    if args.last().is_some_and(|s| s == "--to") {
        return true;
    }
    args.is_empty() && usage.contains("<session") && !matches!(command, "drop" | "open")
}

fn path_choices(
    req: &PathRequest,
    listing: &crate::files::DirListing,
    leaf: &str,
    force_quote: bool,
) -> Vec<Choice> {
    let typed_parent = req
        .typed
        .rsplit_once('/')
        .map(|(p, _)| format!("{p}/"))
        .unwrap_or_default();
    listing
        .entries
        .iter()
        .filter(|e| e.name.starts_with(leaf) && (!req.dirs_only || e.dir))
        .map(|e| {
            let parent =
                if req.local_relative && !req.typed.starts_with('/') && !req.typed.starts_with('~')
                {
                    typed_parent.clone()
                } else {
                    format!("{}/", listing.path.trim_end_matches('/'))
                };
            let path = format!(
                "{}{}{}{}",
                req.head,
                parent,
                e.name,
                if e.dir { "/" } else { "" }
            );
            choice(
                quoted(&path, force_quote),
                format!("{}{}", e.name, if e.dir { "/" } else { "" }),
                req.machine.clone(),
                if e.dir { "directory" } else { "file" },
            )
        })
        .collect()
}

/// `caret`, `start`, and `end` are UTF-16 code units, matching textarea
/// selectionStart. This function only reads catalog, devices, sessions and ls.
pub async fn complete(root: &Path, line: &str, caret: usize) -> Result<Completion, String> {
    let caret = byte_at_utf16(line, caret);
    let tokens = words(line);
    let active = tokens
        .iter()
        .position(|w| w.start <= caret && caret <= w.end);
    let (start, end, index) = active
        .map(|i| (tokens[i].start, tokens[i].end, i))
        .unwrap_or((
            caret,
            caret,
            tokens.iter().filter(|w| w.end < caret).count(),
        ));
    let prefix = unquote(&line[start..caret]);
    let force_quote = line[start..end].starts_with(['\'', '"']);
    let mut result = Completion {
        start: line[..start].encode_utf16().count(),
        end: line[..end].encode_utf16().count(),
        choices: Vec::new(),
    };
    let catalog = khor_commands::catalog();
    if index == 0 {
        let slash = line.trim_start().starts_with('/') || line.trim().is_empty();
        let needle = prefix.trim_start_matches('/');
        for spec in &catalog {
            for word in std::iter::once(&spec.word).chain(spec.aliases.iter()) {
                if word.starts_with(needle) {
                    result.choices.push(choice(
                        format!("{}{word}", if slash { "/" } else { "" }),
                        word.clone(),
                        spec.usage.clone(),
                        "command",
                    ));
                }
            }
        }
        return Ok(result);
    }
    let command = unquote(&line[tokens[0].start..tokens[0].end]);
    let command = command.trim_start_matches('/');
    let Some(spec) = catalog
        .iter()
        .find(|s| s.word == command || s.aliases.iter().any(|a| a == command))
    else {
        return Ok(result);
    };
    let args: Vec<String> = tokens
        .iter()
        .take(index)
        .skip(1)
        .map(|w| unquote(&line[w.start..w.end]))
        .collect();
    for literal in literals(&spec.usage) {
        if literal.starts_with(&prefix)
            && (!prefix.starts_with('-') || literal.starts_with('-'))
            && !args.contains(&literal)
        {
            result.choices.push(choice(
                literal.clone(),
                literal,
                spec.usage.clone(),
                "argument",
            ));
        }
    }
    let machines = needs_machine(&spec.word, &args, &spec.usage) || prefix.starts_with('@');
    let sessions = needs_session(&spec.word, &args, &spec.usage);
    if machines || sessions {
        let root = root.to_path_buf();
        let data = tokio::task::spawn_blocking(move || -> Result<_, String> {
            let n = khor_node::Node::open(root)?;
            Ok((
                if machines { n.devices()? } else { Vec::new() },
                if sessions { n.sessions()? } else { Vec::new() },
            ))
        })
        .await
        .map_err(|e| e.to_string())??;
        for d in data.0 {
            let value = format!(
                "{}{}",
                if prefix.starts_with('@') { "@" } else { "" },
                d.name
            );
            if value.starts_with(&prefix) {
                result
                    .choices
                    .push(choice(quoted(&value, force_quote), d.name, d.id, "machine"));
            }
        }
        for v in data.1 {
            let id = v.session.id.0.clone();
            if id.starts_with(&prefix) {
                result
                    .choices
                    .push(choice(id.clone(), id, v.session.title, "session"));
            }
        }
    }
    if let Some(mut req) = path_request(&spec.word, &args, &prefix, &spec.usage) {
        if spec.word == "drop" && args.last().is_some_and(|a| a == "--into") {
            if let Some(session) = flag_value(&args, &["--to"]) {
                let root = root.to_path_buf();
                let session = khor_core::SessionId(session.to_owned());
                req.machine = tokio::task::spawn_blocking(move || {
                    khor_node::Node::open(root)?.far_machine(&session)
                })
                .await
                .map_err(|e| e.to_string())??
                .unwrap_or_default();
                req.local_relative = req.machine.is_empty();
            }
        }
        let (parent, leaf) = req
            .typed
            .rsplit_once('/')
            .map(|(p, l)| (if p.is_empty() { "/" } else { p }, l))
            .unwrap_or(("", req.typed.as_str()));
        let parent = if req.local_relative
            && !req.typed.starts_with('~')
            && !Path::new(parent).is_absolute()
        {
            khor_node::cwd::standing()
                .ok_or_else(|| "No host working directory".to_owned())?
                .join(parent)
                .to_string_lossy()
                .into_owned()
        } else if req.typed.starts_with('~') {
            let home = crate::files::ls(root, &req.machine, "").await?;
            PathBuf::from(home.path)
                .join(parent.trim_start_matches('~').trim_start_matches('/'))
                .to_string_lossy()
                .into_owned()
        } else if !req.local_relative && !parent.is_empty() && !Path::new(parent).is_absolute() {
            let home = crate::files::ls(root, &req.machine, "").await?;
            PathBuf::from(home.path)
                .join(parent)
                .to_string_lossy()
                .into_owned()
        } else {
            parent.to_owned()
        };
        let listing = crate::files::ls(root, &req.machine, &parent).await?;
        result
            .choices
            .extend(path_choices(&req, &listing, leaf, force_quote));
    }
    let mut seen = std::collections::HashSet::new();
    result.choices.retain(|c| seen.insert(c.value.clone()));
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn command_and_metadata_completion_need_no_node_store() {
        let root =
            std::env::temp_dir().join(format!("khor-complete-missing-{}", std::process::id()));
        assert!(!root.exists());
        let result = complete(&root, "/dev", 4).await.unwrap();
        assert_eq!((result.start, result.end), (0, 4));
        assert!(result.choices.iter().any(|c| c.value == "/devices"));
        let result = complete(&root, "/devices --r", 12).await.unwrap();
        assert!(result.choices.iter().any(|c| c.value == "--roads"));
        assert!(!root.exists());
    }

    #[tokio::test]
    async fn actual_node_completes_machine_session_and_quoted_directory_paths() {
        let scratch = std::env::temp_dir().join(format!(
            "khor-complete-fixture-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let root = scratch.join("node");
        let dir = scratch.join("files");
        std::fs::create_dir_all(dir.join("dir with space")).unwrap();
        std::fs::write(dir.join("report.txt"), "unchanged").unwrap();
        let dir = dir.canonicalize().unwrap();
        let node = khor_node::Node::open_as(root.clone(), "completion-host").unwrap();
        let machine = node.name().to_owned();
        let session = node
            .open_ephemeral(khor_core::kind::SHELL, "completion fixture")
            .unwrap();
        let machine_line = format!("/ls {}", &machine[..machine.len().min(4)]);
        let machines = complete(&root, &machine_line, machine_line.encode_utf16().count())
            .await
            .unwrap();
        assert!(
            machines
                .choices
                .iter()
                .any(|c| c.kind == "machine" && c.label == machine)
        );

        let line = format!("/ls {machine} \"{}/dir", dir.display());
        let result = complete(&root, &line, line.encode_utf16().count())
            .await
            .unwrap();
        let selected = result
            .choices
            .iter()
            .find(|c| c.kind == "directory")
            .unwrap();
        let args = khor_commands::parse_line(&format!("/ls {machine} {}", selected.value)).unwrap();
        assert_eq!(args[2], format!("{}/dir with space/", dir.display()));
        assert!(result.choices.iter().all(|c| c.kind != "file"));
        assert_eq!(
            result.start,
            format!("/ls {machine} ").encode_utf16().count()
        );
        assert_eq!(result.end, line.encode_utf16().count());

        let line = format!("/send target \"{machine}:{}/re", dir.display());
        let result = complete(&root, &line, line.encode_utf16().count())
            .await
            .unwrap();
        let selected = result.choices.iter().find(|c| c.kind == "file").unwrap();
        let args = khor_commands::parse_line(&format!("/send target {}", selected.value)).unwrap();
        assert_eq!(args[2], format!("{machine}:{}/report.txt", dir.display()));

        // A relative path stays relative to the host's standing directory.
        let line = "/drop ./Cargo.t";
        let result = complete(&root, line, line.encode_utf16().count())
            .await
            .unwrap();
        assert!(
            result
                .choices
                .iter()
                .any(|c| c.value == "./Cargo.toml" && c.kind == "file")
        );

        let line = format!("/attach {}", session.0);
        let result = complete(&root, &line, line.encode_utf16().count())
            .await
            .unwrap();
        assert!(
            result
                .choices
                .iter()
                .any(|c| c.kind == "session" && c.value == session.0)
        );
        assert_eq!(
            std::fs::read_to_string(dir.join("report.txt")).unwrap(),
            "unchanged"
        );
        assert!(dir.join("dir with space").is_dir());
        drop(node);
        std::fs::remove_dir_all(scratch).unwrap();
    }

    #[test]
    fn utf16_and_unfinished_quotes_preserve_whole_word() {
        let line = "/send mac \"/tmp/😀 new\" tail";
        let caret = line.find(" new").unwrap() + 2;
        let units = line[..caret].encode_utf16().count();
        assert_eq!(byte_at_utf16(line, units), caret);
        let tokens = words(line);
        assert_eq!(unquote(&line[tokens[2].start..caret]), "/tmp/😀 n");
        assert_eq!(&line[tokens[2].start..tokens[2].end], "\"/tmp/😀 new\"");
        assert_eq!(unquote("\"mac:/a b"), "mac:/a b");
        assert_eq!(unquote("a\\ b"), "a b");
        assert_eq!(byte_at_utf16("😀a", 1), 0);
    }
    #[test]
    fn inserted_paths_roundtrip_through_shared_parser() {
        for path in [
            "/tmp/a b",
            "/tmp/a'quote",
            "/tmp/a\\ b",
            "/tmp/a\\'b",
            "/tmp/a\"b",
        ] {
            let inserted = quoted(path, false);
            assert_eq!(
                khor_commands::parse_line(&format!("/drop {inserted}")).unwrap()[1],
                path
            );
            assert_eq!(unquote(&inserted), path);
        }
    }

    #[test]
    fn metadata_options_do_not_leak_placeholder_words() {
        let values = literals(
            "drop [-m <machine>] <local file> [--to <session>|--on <machine>] [--into <dir>]",
        );
        assert_eq!(values, vec!["-m", "--to", "--on", "--into"]);
        assert_eq!(
            literals("sessions [--by recent|category|device|state|place]"),
            vec!["--by", "recent", "category", "device", "state", "place"]
        );
    }
    #[test]
    fn remote_and_local_roles_keep_the_host() {
        let args = vec!["far".into()];
        assert_eq!(
            path_request("ls", &args, "/tmp/a", "").unwrap().machine,
            "far"
        );
        assert!(path_request("ls", &args, "/tmp/a", "").unwrap().dirs_only);
        assert_eq!(
            path_request("send", &[], "@far:/a b", "").unwrap().head,
            "@far:"
        );
        assert!(path_request("launch", &[], "https://example.org", "").is_none());
        assert_eq!(
            path_request("drop", &[], "C:\\Users\\name", "")
                .unwrap()
                .machine,
            ""
        );
        assert!(path_request("search", &["/tmp".into()], "query", "").is_none());
        assert!(path_request("mv", &["/a".into(), "/b".into()], "", "").is_none());
        assert!(path_request("drop", &["--to".into()], "a", "").is_none());
        assert!(
            path_request("put", &["far".into()], "./a", "")
                .unwrap()
                .local_relative
        );
    }
    #[test]
    fn listing_completion_quotes_spaces_and_filters_directory_roles() {
        let listing = crate::files::DirListing {
            path: "/home/far".into(),
            truncated: false,
            entries: vec![
                crate::files::DirRow {
                    name: "a dir".into(),
                    dir: true,
                    size: 0,
                    at_ms: 0,
                    from: None,
                    received_ms: None,
                },
                crate::files::DirRow {
                    name: "a.txt".into(),
                    dir: false,
                    size: 1,
                    at_ms: 0,
                    from: None,
                    received_ms: None,
                },
            ],
        };
        let req = path_request("send", &[], "far:a", "").unwrap();
        let choices = path_choices(&req, &listing, "a", false);
        assert_eq!(choices[0].value, "\"far:/home/far/a dir/\"");
        assert_eq!(choices[1].kind, "file");
        let req = path_request("ls", &["far".into()], "a", "").unwrap();
        assert_eq!(path_choices(&req, &listing, "a", false).len(), 1);
        let req = path_request("drop", &[], "./a", "").unwrap();
        assert_eq!(
            path_choices(&req, &listing, "a", false)[0].value,
            "\"./a dir/\""
        );
    }
}
