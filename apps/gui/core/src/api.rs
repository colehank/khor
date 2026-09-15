//! **One table, however many servers.** Every command this crate
//! answers over HTTP is named here exactly once. Today one skin
//! dispatches through it — the dev bridge (`src/bin/bridge.rs`,
//! loopback and no auth, for browser verification); the LAN listener a
//! phone opens is the next one and is why these arms left that file
//! ahead of it (批⑦). The tauri app is the third caller and reaches the
//! same functions through its own handler list.
//!
//! Copying the forty-two arms into the second server would have made
//! **three** tables of one fact, and `tests/command_tables.rs` has the
//! reason that is worse than it sounds: each way two tables can
//! disagree is silent in the direction nobody is looking. That gate can
//! compare two; the third would have been a table nothing checks at
//! all.
//!
//! This layer decides nothing. It reads arguments off JSON, refuses the
//! malformed ones by name, and hands the answer back as a JSON string —
//! the judgments are the functions it calls.

pub fn answer(
    rt: &tokio::runtime::Runtime,
    root: &std::path::Path,
    cmd: &str,
    args: &serde_json::Value,
) -> Result<String, String> {
    let arg = |name: &str| {
        args.get(name)
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("missing arg: {name}"))
            .map(|s| s.to_owned())
    };
    // Missing is an error, not a silent false: a bool that defaults would
    // turn a typo'd argument name into "unpin", which looks like the
    // button working and doing the opposite.
    let flag = |name: &str| {
        args.get(name)
            .and_then(|v| v.as_bool())
            .ok_or_else(|| format!("missing bool arg: {name}"))
    };
    // Absent means "leave that axis alone" (`restyle`), so absent is not
    // an error here — but **present and the wrong shape still is**. A
    // malformed value quietly read as absent would report a change that
    // took while nothing moved.
    let opt_str = |name: &str| -> Result<Option<String>, String> {
        match args.get(name) {
            None | Some(serde_json::Value::Null) => Ok(None),
            Some(v) => v
                .as_str()
                .map(|s| Some(s.to_owned()))
                .ok_or_else(|| format!("arg is not a string: {name}")),
        }
    };
    let opt_colors = || -> Result<Option<Vec<String>>, String> {
        match args.get("colors") {
            None | Some(serde_json::Value::Null) => Ok(None),
            Some(serde_json::Value::Array(items)) => items
                .iter()
                .map(|c| c.as_str().map(str::to_owned).ok_or_else(|| "a color is not a string".to_owned()))
                .collect::<Result<Vec<_>, _>>()
                .map(Some),
            Some(_) => Err("arg is not a list: colors".to_owned()),
        }
    };
    // Whole and in range is the frame contract; a fraction or a
    // negative quietly truncated would answer a different poll than the
    // one asked.
    let num = |name: &str| {
        args.get(name)
            .and_then(|v| v.as_u64())
            .ok_or_else(|| format!("missing number arg: {name}"))
    };
    match cmd {
        "sessions" => to_json(&crate::list_sessions(root, &arg("by")?)?),
        // `cells` is required rather than defaulted: it is a measured
        // pixel width, and a default would be this layer inventing a
        // resolution for a screen it cannot see.
        "day_strips" => to_json(&crate::day_strips(root, num("cells")? as u32)?),
        // `machine` absent means this one — the wizard's default; a
        // name is the machine the session is about to be born on.
        "places" => {
            to_json(&rt.block_on(crate::places(root, &opt_str("machine")?.unwrap_or_default()))?)
        }
        // One read of one local table, no arguments: a workspace is a
        // decision and decisions replicate, so there is no machine to
        // ask (`workspaces` module head).
        "workspaces" => to_json(&crate::workspaces::workspaces(root)?),
        // `machine` empty is this one, the way every path verb here
        // spells it.
        "ws_add" => {
            rt.block_on(crate::workspaces::ws_add(
                root,
                &opt_str("machine")?.unwrap_or_default(),
                &arg("path")?,
            ))?;
            Ok("null".to_owned())
        }
        // **The two writes take a `device`, not a `machine`**: a row
        // whose machine has left the table has no name and is exactly
        // the row that has to stay removable (`workspaces::ws_rm`).
        "ws_rm" => {
            crate::workspaces::ws_rm(root, &arg("device")?, &arg("path")?)?;
            Ok("null".to_owned())
        }
        // `on` is explicit rather than a toggle, for `pin_dir`'s reason.
        "ws_pin" => {
            crate::workspaces::ws_pin(root, &arg("device")?, &arg("path")?, flag("on")?)?;
            Ok("null".to_owned())
        }
        // When that directory last changed. Refuses out loud; the face
        // paints nothing where the time would have been, which is a
        // blank corner and not a wrong date.
        "touched" => to_json(&rt.block_on(crate::workspaces::touched(
            root,
            &opt_str("machine")?.unwrap_or_default(),
            &arg("path")?,
        ))?),
        // One slice of a file, for reading. `pull` is the other verb
        // and it spends a disk; this one spends a screen.
        "read_text" => to_json(&rt.block_on(crate::files::read_text(
            root,
            &arg("machine")?,
            &arg("path")?,
        ))?),
        // What differs from HEAD: one question per workspace, and one
        // per file opened. Both refuse out loud rather than answering
        // empty — an empty answer is「clean」, which is the one reading
        // that must never be a guess (`crate::git`).
        "git_status" => to_json(&rt.block_on(crate::git::status(
            root,
            &arg("machine")?,
            &arg("dir")?,
        ))?),
        "git_diff" => to_json(&rt.block_on(crate::git::diff(
            root,
            &arg("machine")?,
            &arg("path")?,
        ))?),
        // Saving an edit. **`was` is required even when empty** — same
        // shape and the same reason as `rename_session`'s `title`
        // above: an empty one is a real claim (「there should be no
        // file here」), and a missing argument read as "" would turn a
        // frontend typo into a save that clobbers.
        // VS Code on a machine. `code_here` is a probe with no side
        // effect; `code_open` starts a process over there and can take
        // minutes on a first run, which is why the face asks before it
        // calls this and never the other way round.
        // A file shaped for a screen that is not a code view: an
        // image, an SVG, or a document rendered to HTML. One call,
        // because what a file turns out to be is the node's answer and
        // not a question the face can ask first.
        "preview" => to_json(&rt.block_on(crate::files::preview(
            root,
            &arg("machine")?,
            &arg("path")?,
        ))?),
        "code_here" => to_json(&rt.block_on(crate::code::here(root, &arg("machine")?))?),
        "code_open" => {
            to_json(&rt.block_on(crate::code::open(root, &arg("machine")?, &arg("dir")?))?)
        }
        "code_stop" => {
            rt.block_on(crate::code::stop(root, &arg("machine")?))?;
            Ok("null".to_owned())
        }
        "write_text" => to_json(&rt.block_on(crate::files::write_text(
            root,
            &arg("machine")?,
            &arg("path")?,
            &arg("was")?,
            &arg("text")?,
        ))?),
        // No machine argument: the three texts come from the caller and
        // nothing is written (`crate::files::merge_text`).
        "merge_text" => to_json(&crate::files::merge_text(
            &arg("base")?,
            &arg("mine")?,
            &arg("theirs")?,
        )),
        "devices" => to_json(&rt.block_on(crate::list_devices(root))?),
        // `top` is required, the `cells` rule: how many rows fit is the
        // screen's fact, and a default here would be this layer sizing
        // a table it cannot see.
        // `by`/`desc` rank the table on the machine it is about, before
        // the cut (`khor_core::TopBy`): a shared box runs ten thousand
        // processes, and the alternative is either all of them on the
        // wire or a cut ordered by the wrong column. Absent means the
        // node's own busiest-first, which is what the enum defaults to;
        // a word this build does not know is **refused by name** rather
        // than quietly read as that default — a face asking for 显存 and
        // silently getting CPU is the neighbouring-value mistake, and
        // it would look exactly like a working sort.
        "load" => {
            let by = match opt_str("by")?.as_deref() {
                None | Some("") | Some("busiest") => khor_core::TopBy::Busiest,
                Some("pid") => khor_core::TopBy::Pid,
                Some("name") => khor_core::TopBy::Name,
                Some("user") => khor_core::TopBy::User,
                Some("cpu") => khor_core::TopBy::Cpu,
                Some("mem") => khor_core::TopBy::Mem,
                Some("gpu") => khor_core::TopBy::Gpu,
                Some("vram") => khor_core::TopBy::Vram,
                Some("time") => khor_core::TopBy::Time,
                Some("status") => khor_core::TopBy::Status,
                // Two words for two different times, and the pair is
                // the reason both are spelled out: `time` is the wall
                // clock a process has been up, `cputime` is the CPU it
                // has burned. A machine on four cores can have burned
                // four times the first.
                Some("cputime") => khor_core::TopBy::CpuTime,
                Some("io") => khor_core::TopBy::Io,
                Some(other) => return Err(format!("no such column: {other}")),
            };
            let desc = args.get("desc").and_then(|v| v.as_bool()).unwrap_or(true);
            to_json(&rt.block_on(crate::load(root, &arg("machine")?, num("top")? as u32, by, desc))?)
        }
        // The 额度 page: one ring per registered account, wherever its
        // key lives (`Node::quota_board`).
        "quota_board" => to_json(&rt.block_on(crate::quota_board(root))?),
        "register_account" => {
            // `budget` absent is "no budget"; present and not a number
            // is refused, the `restyle` rule (a malformed value quietly
            // read as absent would register an account with no ceiling
            // while the form said one).
            let budget = match args.get("budget") {
                None | Some(serde_json::Value::Null) => None,
                Some(v) => Some(v.as_f64().ok_or_else(|| "arg is not a number: budget".to_owned())?),
            };
            // `kind` is required rather than defaulted to 「bill」 for
            // `budget`'s reason one step further: a page that failed to
            // send it would register a *billed* account for somebody
            // who picked 订阅, and the refusal they would then read is
            // 「密钥是空的」 — a sentence about a box the form never
            // showed them.
            to_json(&crate::register_account(
                root,
                &arg("name")?,
                &arg("provider")?,
                &arg("kind")?,
                opt_str("base_url")?.as_deref(),
                &arg("key")?,
                budget,
            )?)
        }
        "remove_account" => {
            crate::remove_account(root, &arg("id")?)?;
            Ok("null".to_owned())
        }
        // 置顶 / 取消置顶. `flag`, not a defaulting bool: a typo'd
        // argument name read as `false` would look like the menu
        // working and doing the opposite — the reason `flag` exists.
        "pin_account" => {
            crate::pin_account(root, &arg("id")?, flag("on")?)?;
            Ok("null".to_owned())
        }
        // 编辑. **`key` absent means「leave the stored key alone」**, and
        // that is why it is `opt_str` rather than `arg`: an edit of the
        // name must not make somebody paste a secret again. Present and
        // the wrong shape is still refused — a malformed key read as
        // absent would report a rotation that never happened.
        "update_account" => {
            let budget = match args.get("budget") {
                None | Some(serde_json::Value::Null) => None,
                Some(v) => Some(v.as_f64().ok_or_else(|| "arg is not a number: budget".to_owned())?),
            };
            crate::update_account(
                root,
                &arg("id")?,
                &arg("name")?,
                opt_str("base_url")?.as_deref(),
                opt_str("key")?.as_deref(),
                budget,
                opt_str("unit")?.as_deref(),
            )?;
            Ok("null".to_owned())
        }
        "usage" => to_json(&crate::usage(root)?),
        // No `root`: a subscription is an account's, not an
        // installation's. And no `?` — every failure here is a reason
        // the person can act on, so it rides back as data.
        "quota" => to_json(&rt.block_on(crate::quota())),
        "seen" => {
            crate::seen(root, &arg("id")?)?;
            Ok("null".to_owned())
        }
        "close_session" => {
            crate::close_session(root, &arg("id")?)?;
            Ok("null".to_owned())
        }
        // `title` must be present even when empty — an empty string is
        // the eraser (fall back to the derived title), and a missing
        // argument silently read as "" would turn a frontend typo into
        // renames that quietly erase.
        "rename_session" => {
            crate::rename_session(root, &arg("id")?, &arg("title")?)?;
            Ok("null".to_owned())
        }
        // Same shape and the same reason `title` is required above: a
        // missing argument read as "" would turn a frontend typo into a
        // rename that quietly erases.
        "rename_machine" => {
            let shown = crate::rename_machine(root, &arg("id")?, &arg("name")?)?;
            serde_json::to_string(&shown).map_err(|e| e.to_string())
        }
        "tell" => {
            crate::tell(root, &arg("machine")?, &arg("text")?)?;
            Ok("null".to_owned())
        }
        "pin_session" => {
            crate::pin_session(root, &arg("id")?, flag("on")?)?;
            Ok("null".to_owned())
        }
        "pin_device" => {
            crate::pin_device(root, &arg("machine")?, flag("on")?)?;
            Ok("null".to_owned())
        }
        "forget_device" => to_json(&crate::forget_device(root, &arg("machine")?)?),
        "set_watch" => {
            // A list of strings, read by position like `colors`; a
            // malformed entry is refused by name rather than dropped —
            // a path that silently vanished from the list would report
            // a save that took while a directory went unwatched.
            let paths: Vec<String> = match args.get("paths") {
                Some(serde_json::Value::Array(items)) => items
                    .iter()
                    .map(|p| p.as_str().map(str::to_owned).ok_or_else(|| "a path is not a string".to_owned()))
                    .collect::<Result<Vec<_>, _>>()?,
                _ => return Err("arg is not a list: paths".to_owned()),
            };
            to_json(&crate::set_watch(root, &arg("machine")?, &paths)?)
        }
        "face_choices" => to_json(&crate::face_choices(root)?),
        "restyle" => {
            crate::restyle(root, opt_colors()?, opt_str("variant")?, opt_str("shape")?)?;
            Ok("null".to_owned())
        }
        "hooks_state" => to_json(&crate::hooks_state(root)?),
        "install_hooks" => to_json(&crate::install_hooks(root)?),
        "uninstall_hooks" => to_json(&crate::uninstall_hooks(root)?),
        "chat_open" => to_json(&crate::chat::chat_open(root, &arg("id")?)?),
        "history_follow" => to_json(&crate::chat::history_follow(
            root,
            &arg("id")?,
            args.get("from").and_then(serde_json::Value::as_u64).unwrap_or(0),
        )?),
        "chat_poll" => to_json(&crate::chat::chat_poll(&arg("id")?, num("since")?)?),
        "chat_say" => {
            // `images` absent is the ordinary turn. Read as an array of
            // `[mime, base64]` pairs rather than an object so the two
            // ends agree by position, the way everything else here
            // does.
            let images: Vec<(String, String)> = match args.get("images") {
                Some(serde_json::Value::Array(a)) => a
                    .iter()
                    .filter_map(|p| {
                        let pair = p.as_array()?;
                        Some((pair.first()?.as_str()?.to_owned(), pair.get(1)?.as_str()?.to_owned()))
                    })
                    .collect(),
                _ => Vec::new(),
            };
            let files: Vec<(String, String)> = match args.get("files") {
                Some(serde_json::Value::Array(a)) => a
                    .iter()
                    .filter_map(|p| {
                        let pair = p.as_array()?;
                        Some((pair.first()?.as_str()?.to_owned(), pair.get(1)?.as_str()?.to_owned()))
                    })
                    .collect(),
                _ => Vec::new(),
            };
            crate::chat::chat_say_with(&arg("id")?, &arg("text")?, images, files)?;
            Ok("null".to_owned())
        }
        "chat_answer" => {
            crate::chat::chat_answer(&arg("id")?, num("ask")?, opt_str("option")?)?;
            Ok("null".to_owned())
        }
        "open_link" => {
            crate::web::open_link(&arg("url")?)?;
            Ok("null".to_owned())
        }
        "chat_stop" => {
            crate::chat::chat_stop(&arg("id")?)?;
            Ok("null".to_owned())
        }
        "chat_set_mode" => {
            crate::chat::chat_set_mode(&arg("id")?, &arg("mode")?)?;
            Ok("null".to_owned())
        }
        // `value` is handed over as it came — a string or a bool — and
        // sorted by shape one layer down, where the two shapes are
        // named (`chat::chat_set_option`).
        "chat_set_option" => {
            let value = args.get("value").ok_or_else(|| "missing arg: value".to_owned())?;
            crate::chat::chat_set_option(&arg("id")?, &arg("option")?, value)?;
            Ok("null".to_owned())
        }
        "chat_replay" => {
            crate::chat::chat_replay(&arg("id")?)?;
            Ok("null".to_owned())
        }
        "chat_leave" => {
            crate::chat::chat_leave(&arg("id")?)?;
            Ok("null".to_owned())
        }
        "history" => to_json(&crate::chat::history(root, &arg("id")?)?),
        "ls" => to_json(&rt.block_on(crate::files::ls(root, &arg("machine")?, &arg("path")?))?),
        "pull" => to_json(&rt.block_on(crate::files::pull(root, &arg("machine")?, &arg("path")?))?),
        "dir_pins" => to_json(&crate::files::dir_pins(root)?),
        // Taking a path off a disk. `machine` empty is this one, which
        // is also the only one with a recycle bin behind it.
        "remove" => {
            rt.block_on(crate::files::remove(
                root,
                &opt_str("machine")?.unwrap_or_default(),
                &arg("path")?,
            ))?;
            Ok("null".to_owned())
        }
        // Whether this machine has a bin at all — asked rather than
        // guessed from the browser (`files::trash_here`).
        "trash_here" => to_json(&crate::files::trash_here()),
        // Whether this browser draws PDFs itself — asked for
        // `trash_here`'s reason, never read off `navigator`.
        "pdf_here" => to_json(&crate::files::pdf_here()),
        "pin_dir" => {
            crate::files::pin_dir(root, &arg("machine")?, &arg("path")?, flag("on")?)?;
            Ok("null".to_owned())
        }
        "term_open" => {
            crate::term::term_open(root, &arg("id")?, num("cols")? as u16, num("rows")? as u16)?;
            Ok("null".to_owned())
        }
        "term_attach" => to_json(&crate::term::term_attach(&arg("id")?)?),
        "term_stream" => to_json(&crate::term::term_stream(&arg("id")?, num("since")?)?),
        "term_key" => {
            crate::term::term_key(&arg("id")?, arg("keys")?.into_bytes())?;
            Ok("null".to_owned())
        }
        "term_drop" => {
            let paths: Vec<String> = args
                .get("paths")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect())
                .unwrap_or_default();
            crate::term::term_drop(root, &arg("id")?, &paths)?;
            Ok("null".to_owned())
        }
        "term_resize" => {
            crate::term::term_resize(&arg("id")?, num("cols")? as u16, num("rows")? as u16)?;
            Ok("null".to_owned())
        }
        "term_leave" => {
            crate::term::term_leave(&arg("id")?)?;
            Ok("null".to_owned())
        }
        // Where this person has been, one kind at a time — the two
        // landings never show each other's places (`crate::recents`).
        "recents" => to_json(&crate::recents::recents(root, &arg("kind")?, num("limit")? as usize)?),
        "recent_visit" => {
            crate::recents::recent_visit(root, &arg("kind")?, &arg("machine")?, &arg("place")?)?;
            Ok("null".to_owned())
        }
        "forget_recent" => {
            crate::recents::forget_recent(root, &arg("kind")?, &arg("machine")?, &arg("place")?)?;
            Ok("null".to_owned())
        }
        "web_pins" => to_json(&crate::web::web_pins(root)?),
        "pin_web" => {
            crate::web::pin_web(root, &arg("machine")?, &arg("url")?, flag("on")?)?;
            Ok("null".to_owned())
        }
        // The other half of that: giving the borrow back. Both roads need
        // it — the bridge opens real borrows, so a bridge that could not
        // close them would leak a proxy port per page it ever opened.
        "close_web" => {
            rt.block_on(crate::web::drop_web_borrow(root, &arg("session")?))?;
            Ok("null".to_owned())
        }
        // The page commands, for the bridge's half of them. There is no
        // window here to parent a webview to, so `open_page` answers the
        // borrow and stops, and the ones that only move a webview around
        // have nothing to move. **Answering rather than refusing** keeps the bridge and
        // the app equivalent for everything a gate can reach: a refusal
        // would make every gate that walks the browsing landing red for
        // a reason that is about the bridge, not about the product.
        "open_page" => to_json(&rt.block_on(crate::web::borrow_web(root, &arg("machine")?))?),
        "move_page" | "hide_page" | "close_page" | "show_page" | "page_step" => {
            Ok("null".to_owned())
        }
        "takeover" => {
            crate::takeover(root, &arg("id")?)?;
            Ok("null".to_owned())
        }
        "takeover_term" => {
            crate::takeover_term(root, &arg("id")?)?;
            Ok("null".to_owned())
        }
        "open_session" => to_json(&rt.block_on(crate::open_session(
            root,
            &arg("dir")?,
            &opt_str("title")?.unwrap_or_default(),
            &arg("form")?,
            &opt_str("agent")?.unwrap_or_default(),
            &opt_str("machine")?.unwrap_or_default(),
        ))?),
        "open_on" => to_json(&rt.block_on(crate::open_term_on(
            root,
            &arg("machine")?,
            &opt_str("title")?.unwrap_or_default(),
            &opt_str("cwd")?.unwrap_or_default(),
        ))?),
        // The path names a file on the machine answering this call —
        // meaningful from the app and the loopback bridge, where that
        // machine is the one the person sits at. The browser upload is
        // not here: it is a raw-bytes route, not a JSON command
        // (`send_upload` behind `/api/upload`), and this table carries
        // JSON commands.
        "send_path" => to_json(&crate::send_path(
            root,
            &arg("machine")?,
            std::path::Path::new(&arg("path")?),
        )?),
        "machine_log" => to_json(&crate::machine_log(root, &arg("id")?)?),
        "accept_transfer" => to_json(&rt.block_on(crate::accept_transfer(root, &arg("msg")?))?),
        "agents" => to_json(&crate::agents(root)?),
        // The shipped ACP registry snapshot, and when it was taken.
        // Two arms rather than one object because the list is 40 rows
        // and the provenance is three scalars — a face redraws the
        // first on every keystroke of its filter and the second never.
        "agents_known" => to_json(&crate::agents::agents_known()),
        "agents_known_taken" => to_json(&crate::agents::agents_known_taken()),
        // **By id, never by argv.** The command is looked up on this
        // side (`Node::register_known_agent`); an arm that accepted the
        // argv would be an arm that spawns whatever it is handed, and
        // this table's second caller is a listener a phone opens.
        "register_known_agent" => {
            crate::agents::register_known(root, &arg("id")?)?;
            Ok("null".to_owned())
        }
        // **These two are the same question and must stay two arms.**
        // `agent_seen` reads what was recorded (0.017s); `agent_ask`
        // starts the agent to ask it (1.5s warm, 5.64s cold). One arm with a
        // flag would put the expensive one a typo away from a poll, and
        // the whole judgment here is that asking only ever happens
        // behind something a person did.
        "agent_seen" => to_json(&crate::agents::agent_seen(root, &arg("name")?)?),
        "agent_ask" => to_json(&crate::agents::agent_ask(root, &arg("name")?)?),
        // Not `open_session` with a flag: a conversation that exists
        // brings its own directory and its own name, so the wizard's
        // answers do not apply to it (`agents::resume_session`).
        "resume_session" => {
            to_json(&crate::agents::resume_session(root, &arg("agent")?, &arg("session")?)?)
        }
        "invite" => to_json(&crate::invite(root)?),
        "pair" => to_json(&rt.block_on(crate::pair(root, &arg("ticket")?))?),
        other => Err(format!("no such command: {other}")),
    }
}

fn to_json<T: serde::Serialize>(v: &T) -> Result<String, String> {
    serde_json::to_string(v).map_err(|e| e.to_string())
}
