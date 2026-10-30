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
                .map(|c| {
                    c.as_str()
                        .map(str::to_owned)
                        .ok_or_else(|| "a color is not a string".to_owned())
                })
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
        "khor_agent" => to_json(&rt.block_on(crate::khor_agent(
            root,
            &arg("machine")?,
            &arg("action")?,
            args.get("payload").ok_or("missing payload")?,
        ))?),
        "console_catalog" => to_json(&crate::console::catalog()),
        "console_complete" => to_json(&rt.block_on(crate::console_complete::complete(
            root,
            &arg("line")?,
            num("caret")? as usize,
        ))?),
        "console_run" => to_json(&crate::console::run(
            root,
            &arg("line")?,
            &arg("token")?,
            opt_str("input")?.as_deref(),
        )?),
        "sessions" => to_json(&crate::list_sessions(root, &arg("by")?)?),
        // `cells` is required rather than defaulted: it is a measured
        // pixel width, and a default would be this layer inventing a
        // resolution for a screen it cannot see.
        "day_strips" => to_json(&crate::day_strips(root, num("cells")? as u32)?),
        // One read of one local table, no arguments: a workspace is a
        // decision and decisions replicate, so there is no machine to
        // ask (`workspaces` module head).
        "workspaces" => to_json(&crate::workspaces::workspaces(root)?),
        // ── the browsing tabs (docs/SHELL.md 第 2 条) ────────────────
        //
        // **A file on this disk, not this window's storage**, so
        // `khor tabs` can list, open and close the same rows. The five
        // below are the whole surface, and every one of them answers
        // with the list as it now stands — a caller that had to ask
        // again to find out what its own write did would be two reads
        // out of step for as long as the round trip takes.
        //
        // `kind` is always given: `page` today, and the store is keyed
        // by it so a second browsing kind lands beside rather than
        // inside (`khor_node::tabs`).
        "tabs_list" => to_json(&crate::tabs::list(root, &arg("kind")?)),
        // Both places may be empty — that is the fresh tab the 「+」
        // mints, and it is a real state rather than a half-made row.
        "tabs_open" => to_json(&crate::tabs::open(
            root,
            &arg("kind")?,
            &opt_str("device")?.unwrap_or_default(),
            &opt_str("url")?.unwrap_or_default(),
            &opt_str("session")?.unwrap_or_default(),
            &opt_str("after")?.unwrap_or_default(),
            opt_str("view")?.as_deref(),
            opt_str("request")?.as_deref(),
        )?),
        "tabs_go" => to_json(&crate::tabs::go(
            root,
            &arg("kind")?,
            &arg("id")?,
            &opt_str("device")?.unwrap_or_default(),
            &opt_str("url")?.unwrap_or_default(),
            &opt_str("title")?.unwrap_or_default(),
            opt_str("view")?.as_deref(),
        )?),
        "tabs_focus" => to_json(&crate::tabs::focus(root, &arg("kind")?, &arg("id")?)?),
        "tabs_close" => to_json(&crate::tabs::close(root, &arg("kind")?, &arg("id")?)?),
        "tabs_pin" => to_json(&crate::tabs::pin(
            root,
            &arg("kind")?,
            &arg("id")?,
            flag("on")?,
        )?),
        // The whole column, for the one gesture that cannot be spelled
        // as a close and an open: a drag that reorders. The ids are
        // still the node's to mint.
        "tabs_reorder" => {
            let ids: Vec<String> =
                serde_json::from_value(args.get("ids").cloned().unwrap_or(serde_json::Value::Null))
                    .map_err(|e| format!("arg is not a tab id list: ids ({e})"))?;
            to_json(&crate::tabs::reorder(
                root,
                &arg("kind")?,
                &arg("session")?,
                &ids,
            )?)
        }
        "tabs_save" => {
            // **Shape-checked by serde rather than field by field**, and
            // a malformed one is refused out loud: this is the only
            // argument on this layer that is a structure, and a
            // half-read column would be a list of tabs with rows
            // silently missing.
            let want: crate::tabs::TabsOpen = serde_json::from_value(
                args.get("open").cloned().unwrap_or(serde_json::Value::Null),
            )
            .map_err(|e| format!("arg is not a tab list: open ({e})"))?;
            to_json(&crate::tabs::save_all(root, &arg("kind")?, &want)?)
        }
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
        // `on` is explicit rather than a toggle: a toggle sent twice is a
        // no-op nobody can see.
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
        "dirs_begin" => {
            let scopes: Vec<crate::files::DirWatchScope> = serde_json::from_value(
                args.get("scopes").cloned().ok_or("missing arg: scopes")?,
            ).map_err(|error| error.to_string())?;
            to_json(&rt.block_on(crate::files::dirs_begin(root, &arg("machine")?, scopes))?)
        }
        "dirs_wait" => {
            // Validate the same wire shape as native, but never hold the
            // serial HTTP bridge while waiting for a filesystem event.
            u32::try_from(num("wait_ms")?).map_err(|_| "wait_ms exceeds u32".to_owned())?;
            to_json(&rt.block_on(crate::files::dirs_wait(
                root, &arg("machine_id")?, &arg("id")?, opt_str("cursor")?.as_deref(), 0,
            ))?)
        }
        "dirs_leave" => {
            rt.block_on(crate::files::dirs_leave(root, &arg("machine_id")?, &arg("id")?))?;
            Ok("null".into())
        }
        "stat_text" => to_json(&rt.block_on(crate::files::stat_text(
            root,
            &arg("machine")?,
            &arg("path")?,
        ))?),
        "read_text" => to_json(&rt.block_on(crate::files::read_text(
            root,
            &arg("machine")?,
            &arg("path")?,
        ))?),
        // What differs from HEAD: one question per workspace, and one
        // per file opened. Both refuse out loud rather than answering
        // empty — an empty answer is「clean」, which is the one reading
        // that must never be a guess (`crate::git`).
        "git_status" => {
            to_json(&rt.block_on(crate::git::status(root, &arg("machine")?, &arg("dir")?))?)
        }
        // Whether that directory is a repository at all — one stat,
        // asked beside `git_status` so the row's corner never waits on
        // the walk (docs/SHELL.md 第五轮 第 6 条).
        "checkout" => {
            to_json(&rt.block_on(crate::git::checkout(root, &arg("machine")?, &arg("dir")?))?)
        }
        "git_diff" => {
            to_json(&rt.block_on(crate::git::diff(root, &arg("machine")?, &arg("path")?))?)
        }
        // One file's history (round UX-4): its log, its blame, a revision.
        "git_log" => {
            to_json(&rt.block_on(crate::git::log(root, &arg("machine")?, &arg("path")?))?)
        }
        "git_history" => to_json(&rt.block_on(crate::git::history(
            root,
            &arg("machine")?,
            &arg("dir")?,
            opt_str("cursor")?.as_deref(),
            u32::try_from(num("limit")?).map_err(|_| "limit exceeds u32".to_owned())?,
        ))?),
        "git_blame" => {
            to_json(&rt.block_on(crate::git::blame(root, &arg("machine")?, &arg("path")?))?)
        }
        "git_show" => to_json(&rt.block_on(crate::git::show(
            root,
            &arg("machine")?,
            &arg("path")?,
            &arg("rev")?,
        ))?),
        // One commit of a repository and what it changed (the commit page).
        "git_commit" => to_json(&rt.block_on(crate::git::commit_show(
            root,
            &arg("machine")?,
            &arg("dir")?,
            &arg("rev")?,
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
        "preview" => {
            to_json(&rt.block_on(crate::files::preview(root, &arg("machine")?, &arg("path")?))?)
        }
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
            to_json(&rt.block_on(crate::load(
                root,
                &arg("machine")?,
                num("top")? as u32,
                by,
                desc,
            ))?)
        }
        // The 额度 page: one ring per registered account, wherever its
        // key lives (`Node::quota_board`).
        "quota_board" => to_json(&rt.block_on(crate::quota_board(root))?),
        "subscription_candidates" => {
            to_json(&rt.block_on(crate::subscriptions::discover(root, &arg("machine")?))?)
        }
        "import_subscription" => to_json(&rt.block_on(crate::subscriptions::import(
            root,
            &arg("machine")?,
            &arg("provider")?,
        ))?),
        "register_account" => {
            // `budget` absent is "no budget"; present and not a number
            // is refused, the `restyle` rule (a malformed value quietly
            // read as absent would register an account with no ceiling
            // while the form said one).
            let budget = match args.get("budget") {
                None | Some(serde_json::Value::Null) => None,
                Some(v) => Some(
                    v.as_f64()
                        .ok_or_else(|| "arg is not a number: budget".to_owned())?,
                ),
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
        // 改名: the 额度 row's double-click — the name and nothing else.
        "rename_account" => {
            crate::rename_account(root, &arg("id")?, &arg("name")?)?;
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
                Some(v) => Some(
                    v.as_f64()
                        .ok_or_else(|| "arg is not a number: budget".to_owned())?,
                ),
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
        "seen_snapshot" => {
            let messages: Vec<crate::SeenMessage> = serde_json::from_value(
                args.get("messages")
                    .cloned()
                    .ok_or_else(|| "missing arg: messages".to_owned())?,
            )
            .map_err(|e| e.to_string())?;
            crate::seen_snapshot(root, &arg("id")?, &messages)?;
            Ok("null".to_owned())
        }
        "seen" => {
            let at_ms = match args.get("at_ms") {
                None | Some(serde_json::Value::Null) => None,
                Some(v) => Some(
                    v.as_i64()
                        .ok_or_else(|| "invalid read timestamp".to_owned())?,
                ),
            };
            crate::seen(root, &arg("id")?, at_ms)?;
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
        "post" => {
            let files: Vec<String> = match args.get("files") {
                Some(v) => serde_json::from_value(v.clone()).map_err(|e| e.to_string())?,
                None => Vec::new(),
            };
            to_json(&rt.block_on(crate::post(root, &arg("typed")?, &files))?)
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
                    .map(|p| {
                        p.as_str()
                            .map(str::to_owned)
                            .ok_or_else(|| "a path is not a string".to_owned())
                    })
                    .collect::<Result<Vec<_>, _>>()?,
                _ => return Err("arg is not a list: paths".to_owned()),
            };
            to_json(&crate::set_watch(root, &arg("machine")?, &paths)?)
        }
        "set_receive_dir" => to_json(&crate::set_receive_dir(
            root,
            &arg("machine")?,
            &arg("dir")?,
        )?),
        "relays" => to_json(&crate::relays(root)?),
        "road_share" => to_json(&rt.block_on(crate::road_share(root))?),
        "connections" => to_json(&rt.block_on(crate::connections(root))?),
        "journal" => to_json(&crate::journal(root, &arg("machine")?, num("minutes")?)?),
        "journal_report" => to_json(&crate::journal_report(root, num("minutes")?)?),
        "proxy_line" => {
            let port = match args.get("port") {
                None | Some(serde_json::Value::Null) => None,
                Some(v) => Some(
                    v.as_u64()
                        .and_then(|p| u16::try_from(p).ok())
                        .ok_or_else(|| "arg is not a port: port".to_owned())?,
                ),
            };
            to_json(&crate::proxy_line(&arg("machine")?, &arg("host")?, port))
        }
        "face_here" => {
            rt.block_on(crate::face_here(
                root,
                matches!(args.get("visible"), Some(serde_json::Value::Bool(true))),
            ))?;
            Ok("null".to_owned())
        }
        "visit" => {
            rt.block_on(crate::visit(root, &arg("machine")?))?;
            Ok("null".to_owned())
        }
        "rejoin_now" => {
            rt.block_on(crate::rejoin_now(root, &arg("machine")?))?;
            Ok("null".to_owned())
        }
        "set_preferred_relay" => {
            let url = args.get("url").and_then(|v| v.as_str()).map(str::to_owned);
            crate::set_preferred_relay(root, url)?;
            Ok("null".to_owned())
        }
        "set_relays" => {
            // Read by position and refused by name, the same as `paths`
            // above and for a sharper version of the same reason: a
            // relay is only consulted after a hole punch has already
            // failed, so an entry dropped on the way in shows up weeks
            // later as「有时候连不上」and points at the network.
            let wants: Vec<crate::RelayWant> = match args.get("relays") {
                Some(serde_json::Value::Array(items)) => items
                    .iter()
                    .map(|u| {
                        serde_json::from_value(u.clone())
                            .map_err(|_| "a relay is not {url, token}".to_owned())
                    })
                    .collect::<Result<Vec<_>, _>>()?,
                _ => return Err("arg is not a list: relays".to_owned()),
            };
            to_json(&crate::set_relays(root, &wants, flag("only_mine")?)?)
        }
        "face_choices" => to_json(&crate::face_choices(
            root,
            &opt_str("machine")?.unwrap_or_default(),
        )?),
        "restyle" => {
            crate::restyle(
                root,
                &opt_str("machine")?.unwrap_or_default(),
                opt_colors()?,
                opt_str("variant")?,
                opt_str("shape")?,
            )?;
            Ok("null".to_owned())
        }
        "agent_recognition" => to_json(&rt.block_on(crate::agent_recognition(
            root,
            &arg("machine")?,
            &arg("action")?,
            opt_str("agent")?.as_deref(),
        ))?),
        "hooks_state" => to_json(&crate::hooks_state(root)?),
        "install_hooks" => to_json(&crate::install_hooks(root)?),
        "uninstall_hooks" => to_json(&crate::uninstall_hooks(root)?),
        "attachment_begin" => {
            let kind =
                serde_json::from_value(args.get("kind").cloned().ok_or("missing arg: kind")?)
                    .map_err(|e| format!("invalid attachment kind: {e}"))?;
            to_json(&crate::attachment_begin(root, &arg("id")?, kind)?)
        }
        "chat_open" => to_json(&crate::chat::chat_open(root, &arg("id")?, &arg("lease")?)?),
        "session_content" => to_json(&rt.block_on(crate::chat::session_content(root,&arg("id")?,&arg("locator")?))?),
        "history_begin" => to_json(&rt.block_on(crate::chat::history_begin(root,&arg("id")?,&arg("binding")?))?),
        "history_cancel" => {
            rt.block_on(crate::chat::history_cancel(root,&arg("id")?,&arg("binding")?,&arg("requestKey")?))?;
            Ok("null".to_owned())
        }
        "history_record" => to_json(&rt.block_on(crate::chat::history_record(
            root,&arg("id")?,&arg("binding")?,&arg("cursor")?,num("from")?,&arg("requestKey")?,
        ))?),
        "history_export_start" => to_json(&crate::history_export::start(
            root, &arg("transfer")?, &arg("id")?, &arg("binding")?, &arg("cursor")?, num("start")?, num("end")?,
        )?),
        "history_projection" => to_json(&rt.block_on(crate::chat::history_projection(
            root,
            &arg("id")?,
            &arg("binding")?,
            &arg("requestKey")?,
            &arg("cursor")?,
            num("start")?,
            num("end")?,
            opt_str("scaffold")?,
        ))?),
        "history_page" => to_json(&rt.block_on(crate::chat::history_page(
            root,&arg("id")?,&arg("binding")?,opt_str("cursor")?.as_deref(),
            args.get("newer").and_then(serde_json::Value::as_bool).unwrap_or(false),&arg("requestKey")?,
        ))?),
        "chat_poll" => to_json(&crate::chat::chat_poll(root, &arg("id")?, &arg("lease")?, num("since")?)?),
        // The bridge answers one request at a time: a held wait would
        // hold every other call behind it, so it is the poll here.
        "chat_wait" => to_json(&crate::chat::chat_poll(
            root,
            &arg("id")?,
            &arg("lease")?,
            num("since")?,
        )?),
        // Files already on this machine (a drop's or a picker's paths),
        // put where the conversation's machine keeps files (round CHAT).
        "chat_put_paths" => {
            let paths: Vec<String> = match args.get("paths") {
                Some(serde_json::Value::Array(a)) => a
                    .iter()
                    .filter_map(|p| p.as_str().map(str::to_owned))
                    .collect(),
                _ => return Err("missing list arg: paths".to_owned()),
            };
            to_json(&crate::chat::chat_put_paths(
                root,
                &arg("id")?,
                &arg("lease")?,
                &paths,
                opt_str("into")?.as_deref().unwrap_or(""),
            )?)
        }
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
                        Some((
                            pair.first()?.as_str()?.to_owned(),
                            pair.get(1)?.as_str()?.to_owned(),
                        ))
                    })
                    .collect(),
                _ => Vec::new(),
            };
            let files: Vec<(String, String)> = match args.get("files") {
                Some(serde_json::Value::Array(a)) => a
                    .iter()
                    .filter_map(|p| {
                        let pair = p.as_array()?;
                        Some((
                            pair.first()?.as_str()?.to_owned(),
                            pair.get(1)?.as_str()?.to_owned(),
                        ))
                    })
                    .collect(),
                _ => Vec::new(),
            };
            // **Handed to a thread**: the bridge answers one request at a
            // time, and a turn carrying photos to a far machine takes as
            // long as the road does — every poll behind it would wait.
            // How it went reaches the face on the conversation itself:
            // the batch's `sending`, and an `Unsent` frame if it did not
            // go (`chat::chat_say_with`).
            let (id, text) = (arg("id")?, arg("text")?);
            let token = match args.get("token") {
                None | Some(serde_json::Value::Null) => None,
                Some(serde_json::Value::String(token)) => Some(token.clone()),
                _ => return Err("token must be a string or null".into()),
            };
            let pending = crate::chat::submit_say_with_token(
                root,
                &id,
                &arg("lease")?,
                text,
                images,
                files,
                token,
            )?;
            std::thread::spawn(move || {
                let _ = pending.wait();
            });
            Ok("null".to_owned())
        }
        "chat_answer" => {
            crate::chat::chat_answer(
                root,
                &arg("id")?,
                &arg("lease")?,
                num("ask")?,
                opt_str("option")?,
            )?;
            Ok("null".to_owned())
        }
        "page_icons" => to_json(&crate::web::page_icons(root)),
        "page_icon" => to_json(&rt.block_on(crate::web::page_icon(
            root,
            &arg("url")?,
            &opt_str("proxy")?.unwrap_or_default(),
        ))?),
        "open_link" => {
            crate::web::open_link(&arg("url")?)?;
            Ok("null".to_owned())
        }
        "chat_stop" => {
            crate::chat::chat_stop(root, &arg("id")?, &arg("lease")?)?;
            Ok("null".to_owned())
        }
        "chat_set_mode" => {
            crate::chat::chat_set_mode(root, &arg("id")?, &arg("lease")?, &arg("mode")?)?;
            Ok("null".to_owned())
        }
        // `value` is handed over as it came — a string or a bool — and
        // sorted by shape one layer down, where the two shapes are
        // named (`chat::chat_set_option`).
        "chat_respond" => {
            crate::chat::chat_respond(
                root,
                &arg("id")?,
                &arg("lease")?,
                num("ask")?,
                args.get("response").ok_or("missing response")?,
            )?;
            Ok("null".into())
        }
        "chat_control" => {
            crate::chat::chat_control(
                root,
                &arg("id")?,
                &arg("lease")?,
                &arg("token")?,
                &arg("operation")?,
                args.get("payload").unwrap_or(&serde_json::Value::Null),
            )?;
            Ok("null".into())
        }
        "chat_set_option" => {
            let value = args
                .get("value")
                .ok_or_else(|| "missing arg: value".to_owned())?;
            crate::chat::chat_set_option(
                root,
                &arg("id")?,
                &arg("lease")?,
                &arg("option")?,
                value,
            )?;
            Ok("null".to_owned())
        }
        "chat_capture" => {
            crate::chat::chat_capture(root, &arg("id")?, &arg("lease")?, &arg("token")?)?;
            Ok("null".to_owned())
        }
        "chat_leave" => {
            crate::chat::chat_leave(root, &arg("id")?, &arg("lease")?)?;
            Ok("null".to_owned())
        }
        "ls" => to_json(&rt.block_on(crate::files::ls(root, &arg("machine")?, &arg("path")?))?),
        "pull" => {
            to_json(&rt.block_on(crate::files::pull(root, &arg("machine")?, &arg("path")?))?)
        }
        "file_handoff_start" => to_json(&crate::file_handoff::start(
            root,
            &arg("id")?,
            &arg("machine")?,
            &arg("path")?,
        )?),
        "file_handoff_state" => to_json(&crate::file_handoff::state(root, &arg("id")?)?),
        "file_handoff_cancel" => {
            crate::file_handoff::cancel(root, &arg("id")?)?;
            Ok("null".to_owned())
        }
        "pull_temp" => to_json(&rt.block_on(crate::files::pull_temp(
            root,
            &arg("machine")?,
            &arg("path")?,
        ))?),
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
        // Making a name where there was none, and moving one. Each
        // refuses a name already taken rather than writing over it;
        // `machine` empty is this one, the way every path verb here
        // spells it.
        "mkdir" => {
            rt.block_on(crate::files::mkdir(
                root,
                &opt_str("machine")?.unwrap_or_default(),
                &arg("path")?,
            ))?;
            Ok("null".to_owned())
        }
        "touch" => {
            rt.block_on(crate::files::touch(
                root,
                &opt_str("machine")?.unwrap_or_default(),
                &arg("path")?,
            ))?;
            Ok("null".to_owned())
        }
        "rename_path" => {
            rt.block_on(crate::files::rename_path(
                root,
                &opt_str("machine")?.unwrap_or_default(),
                &arg("from")?,
                &arg("to")?,
            ))?;
            Ok("null".to_owned())
        }
        // Opening a path or a page **on the machine that holds it**,
        // which is what tells this apart from `open_link` below: that
        // one opens on the machine the face is running on.
        "launch" => {
            rt.block_on(crate::files::launch(
                root,
                &opt_str("machine")?.unwrap_or_default(),
                &arg("target")?,
            ))?;
            Ok("null".to_owned())
        }
        // Looking for words under one directory on one machine. A read:
        // it walks that machine's project and writes nothing anywhere.
        "search" => to_json(&rt.block_on(crate::files::search(
            root,
            &opt_str("machine")?.unwrap_or_default(),
            &arg("dir")?,
            &arg("query")?,
        ))?),
        // Whether this machine has a bin at all — asked rather than
        // guessed from the browser (`files::trash_here`).
        "trash_here" => to_json(&crate::files::trash_here()),
        // Finder with the file selected, this machine only (round CHAT).
        "reveal_here" => to_json(&crate::files::reveal_here()),
        "reveal" => {
            crate::files::reveal(&arg("path")?)?;
            to_json(&())
        }
        // Whether this browser draws PDFs itself — asked for
        // `trash_here`'s reason, never read off `navigator`.
        "pdf_here" => to_json(&crate::files::pdf_here()),
        "term_open" => {
            // `history` is a newer face's (item 56); an older one says nothing.
            let history = args
                .get("history")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
                .min(u32::MAX as u64) as u32;
            crate::term::term_open(
                root,
                &arg("id")?,
                &arg("lease")?,
                num("cols")? as u16,
                num("rows")? as u16,
                history,
            )?;
            Ok("null".to_owned())
        }
        "term_attach" => to_json(&crate::term::term_attach(
            root,
            &arg("id")?,
            &arg("lease")?,
        )?),
        "term_ready" => {
            crate::term::term_ready(root, &arg("id")?, &arg("lease")?)?;
            to_json(&())
        }
        // A dead terminal row's page (P5-SHELL): its last screen or its
        // commands. A read.
        "term_ended" => to_json(&crate::term::ended(root, &arg("id")?)?),
        "term_stream" => to_json(&crate::term::term_stream(
            root,
            &arg("id")?,
            &arg("lease")?,
            num("since")?,
        )?),
        // A machine's commands, searched (roadmap 2.6). A read.
        "term_history" => to_json(&rt.block_on(crate::term::history(
            root,
            &opt_str("machine")?.unwrap_or_default(),
            &arg("text")?,
            &arg("dir")?,
            flag("failed")?,
        ))?),
        // A tmux row's front pane history (item 31); `since` is the
        // anchor the last answer gave, or absent for all of it.
        "term_pane_history" => {
            let since = match args.get("since") {
                None | Some(serde_json::Value::Null) => None,
                Some(v) => Some(
                    serde_json::from_value(v.clone())
                        .map_err(|e| format!("arg is not an anchor: since ({e})"))?,
                ),
            };
            to_json(&rt.block_on(crate::term::pane_history(
                root,
                &arg("id")?,
                &arg("lease")?,
                since,
            ))?)
        }
        // ⌘K kept by the feed and the host's ring (term-attach).
        "term_forget" => to_json(&crate::term::term_forget(
            root,
            &arg("id")?,
            &arg("lease")?,
            flag("redraw")?,
        )?),
        // A tmux row's prompts, moved between by tmux (item 21).
        "term_prompt" => to_json(&rt.block_on(crate::term::prompt(
            root,
            &arg("id")?,
            &arg("lease")?,
            flag("up")?,
        ))?),
        // The wheel over a shell's own tmux client (item 58).
        "term_scroll" => {
            // Signed: up is positive.
            let rows = args
                .get("rows")
                .and_then(|v| v.as_i64())
                .ok_or("missing number arg: rows")?;
            to_json(&rt.block_on(crate::term::scroll(
                root,
                &arg("id")?,
                &arg("lease")?,
                rows.clamp(-1000, 1000) as i32,
            ))?)
        }
        // The bridge answers one request at a time, so it never holds one
        // open: here a wait is the stream's answer as it stands.
        "term_wait" => to_json(&crate::term::term_stream(
            root,
            &arg("id")?,
            &arg("lease")?,
            num("since")?,
        )?),
        "term_key" => {
            crate::term::term_key(root, &arg("id")?, &arg("lease")?, arg("keys")?.into_bytes())?;
            Ok("null".to_owned())
        }
        "term_drop_items" => {
            let items: Vec<crate::term::DropItem> = serde_json::from_value(
                args.get("items")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null),
            )
            .map_err(|e| e.to_string())?;
            to_json(&crate::term::term_drop_items(
                root,
                &arg("id")?,
                &arg("lease")?,
                &items,
                args.get("into").and_then(|v| v.as_str()),
            ))
        }
        "term_drop_folder" => {
            let dirs: Vec<String> = serde_json::from_value(
                args.get("dirs")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!([])),
            )
            .map_err(|e| e.to_string())?;
            to_json(&crate::term::term_drop_folder(root, &arg("name")?, &dirs)?)
        }
        "term_drop_forget" => {
            let paths: Vec<String> = serde_json::from_value(
                args.get("paths")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!([])),
            )
            .map_err(|e| e.to_string())?;
            crate::term::term_drop_forget(root, &paths);
            to_json(&())
        }
        "term_drop" => {
            let paths: Vec<String> = args
                .get("paths")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default();
            let from = args.get("from").and_then(|v| v.as_str());
            let into = args.get("into").and_then(|v| v.as_str());
            to_json(&crate::term::term_drop(
                root,
                &arg("id")?,
                &arg("lease")?,
                &paths,
                from,
                into,
            ))
        }
        "term_drop_progress" => to_json(&crate::term::term_drop_progress(
            root,
            &arg("id")?,
            &arg("lease")?,
        )?),
        // The pill's × while files go in (item 71). Behind the bridge it
        // reaches a drop under way because `term_drop` runs on a thread
        // of its own there (`bin/bridge.rs`).
        "term_drop_cancel" => {
            let staged: Vec<String> = args
                .get("staged")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default();
            crate::term::term_drop_cancel(root, &arg("id")?, &arg("lease")?, &staged)?;
            Ok("null".to_owned())
        }
        "term_resize" => {
            crate::term::term_resize(
                root,
                &arg("id")?,
                &arg("lease")?,
                num("cols")? as u16,
                num("rows")? as u16,
            )?;
            Ok("null".to_owned())
        }
        "term_leave" => {
            crate::term::term_leave(root, &arg("id")?, &arg("lease")?)?;
            Ok("null".to_owned())
        }
        // Where this person has been, one kind at a time — the two
        // landings never show each other's places (`crate::recents`).
        "recents" => to_json(&crate::recents::recents(
            root,
            &arg("kind")?,
            num("limit")? as usize,
        )?),
        "recent_visit" => {
            crate::recents::recent_visit(
                root,
                &arg("kind")?,
                &arg("machine")?,
                &arg("place")?,
                opt_str("session")?.as_deref(),
            )?;
            Ok("null".to_owned())
        }
        "forget_recent" => {
            if let Some(device) = opt_str("device")? {
                crate::recents::forget_recent_by_device(
                    root,
                    &arg("kind")?,
                    &device,
                    &arg("place")?,
                )?;
            } else {
                crate::recents::forget_recent(
                    root,
                    &arg("kind")?,
                    &arg("machine")?,
                    &arg("place")?,
                )?;
            }
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
        // **An empty machine is 「go direct」** — this machine's own network,
        // no borrow — and the app answers it with an empty borrow without
        // asking the node (`src-tauri` `open_page`). The bridge asked the
        // node to borrow from a machine named "" and printed its refusal
        // on every page opened here; it answers the same as the app now.
        "open_page" => {
            let machine = arg("machine")?;
            if machine.is_empty() {
                to_json(&crate::web::WebBorrow {
                    session: String::new(),
                    addr: String::new(),
                })
            } else {
                to_json(&rt.block_on(crate::web::borrow_web(root, &machine))?)
            }
        }
        "page_state" => {
            let _tab = arg("tab")?;
            to_json(&Option::<crate::web::PageNavigationState>::None)
        }
        // `page_deck` is the iPhone's native deck over its page (UX-13);
        // here there is neither, so it is answered like the other moves.
        "move_page" | "hide_page" | "close_page" | "show_page" | "page_step" | "page_deck"
        | "page_ground" => Ok("null".to_owned()),
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
            opt_str("token")?.as_deref(),
        ))?),
        "launch_presets" => to_json(&crate::launch::list(root)?),
        "launch_preset_save" => to_json(&crate::launch::save(
            root,
            serde_json::from_value(args.get("preset").cloned().ok_or("missing preset")?)
                .map_err(|e| e.to_string())?,
        )?),
        "launch_preset_delete" => to_json(&crate::launch::remove(root, &arg("id")?)?),
        "open_shell" => to_json(
            &rt.block_on(crate::open_shell(
                root,
                &arg("machine")?,
                &arg("cwd")?,
                &arg("command")?,
                &opt_str("title")?.unwrap_or_default(),
                args.get("cols")
                    .and_then(|v| v.as_u64())
                    .zip(args.get("rows").and_then(|v| v.as_u64()))
                    .map(|(c, r)| (c.min(500) as u16, r.min(200) as u16)),
                &arg("token")?,
            ))?,
        ),
        "open_on" => to_json(&rt.block_on(crate::open_term_on(
            root,
            &arg("machine")?,
            &opt_str("title")?.unwrap_or_default(),
            &opt_str("cwd")?.unwrap_or_default(),
            // The face's terminal size, when it sends one: both or none.
            match (
                args.get("cols").and_then(|v| v.as_u64()),
                args.get("rows").and_then(|v| v.as_u64()),
            ) {
                (Some(c), Some(r)) => {
                    Some((c.min(u16::MAX as u64) as u16, r.min(u16::MAX as u64) as u16))
                }
                _ => None,
            },
            opt_str("token")?.as_deref(),
        ))?),
        "machine_log" => to_json(&crate::machine_log(root, &arg("id")?)?),
        "retry_transfer" => to_json(&rt.block_on(crate::retry_transfer(root, &arg("msg")?))?),
        "stop_transfer" => to_json(&rt.block_on(crate::stop_transfer(root, &arg("msg")?))?),
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
        // 导入 (docs/SHELL.md 第八轮): the candidates, and bringing some
        // onto the list. `ids` in the order the person picked them.
        // 全局搜索 (docs/SHELL.md 第十轮): one word, the node's own list.
        "find" => to_json(&crate::find(root, &arg("word")?)?),
        "import_candidates" => to_json(&crate::import::import_candidates(root)?),
        "import_sessions" => {
            let ids: Vec<String> = args
                .get("ids")
                .and_then(|v| v.as_array())
                .ok_or_else(|| "missing arg: ids".to_owned())?
                .iter()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect();
            to_json(&crate::import::import_sessions(root, &ids)?)
        }
        "resume_session" => to_json(&crate::agents::resume_session(
            root,
            &arg("agent")?,
            &arg("session")?,
        )?),
        "session_recovery" => to_json(&rt.block_on(crate::agents::session_recovery(
            root,
            &arg("home_id")?,
            &arg("id")?,
        ))?),
        "recover_session" => {
            let home_id = arg("home_id")?;
            let id = arg("id")?;
            let expected_run = opt_str("expected_run")?;
            let token: crate::agents::RecoveryToken = serde_json::from_value(
                args.get("token")
                    .cloned()
                    .ok_or_else(|| "missing arg: token".to_owned())?,
            )
            .map_err(|error| format!("invalid recovery token: {error}"))?;
            to_json(&rt.block_on(crate::agents::recover_session(
                root,
                &home_id,
                &id,
                expected_run.as_deref(),
                token,
            ))?)
        }
        "new_shell_run" => {
            let home_id = arg("home_id")?;
            let id = arg("id")?;
            let expected_run = opt_str("expected_run")?;
            let cwd = opt_str("cwd")?;
            let token: crate::agents::RecoveryToken = serde_json::from_value(
                args.get("token")
                    .cloned()
                    .ok_or_else(|| "missing arg: token".to_owned())?,
            )
            .map_err(|error| format!("invalid recovery token: {error}"))?;
            to_json(&rt.block_on(crate::agents::new_shell_run(
                root,
                &home_id,
                &id,
                expected_run.as_deref(),
                token,
                cwd.as_deref(),
            ))?)
        }
        // 安全 (docs/MORE.md). The lock is a local preference read
        // straight off the store, so none of these opens a node except
        // the reset, which needs this machine's name.
        "lock_state" => to_json(&crate::lock_state(root)),
        "set_lock" => to_json(&crate::set_lock(root, flag("on")?)?),
        "set_lock_pin" => to_json(&crate::set_lock_pin(root, &arg("pin")?)?),
        "set_lock_timeout" => to_json(&crate::set_lock_timeout(root, num("minutes")? as u32)?),
        // **A wrong PIN answers `false`, not an error.** Only khor being
        // unable to read its own record is a refusal.
        "unlock" => to_json(&crate::unlock(root, &arg("pin")?)?),
        "web_key" => to_json(&crate::web_key(root)?),
        "rotate_web_key" => to_json(&crate::rotate_web_key(root)?),
        "reset_identity" => {
            crate::reset_identity(root, &arg("name")?)?;
            Ok("null".to_owned())
        }
        // 诊断 (docs/MORE.md). Thirteen checks, each under its own
        // timeout — so this arm can take three seconds and never more.
        "doctor" => to_json(&rt.block_on(crate::doctor(root))?),
        // …and one check's remedy, by the check's own key.
        "doctor_fix" => to_json(&crate::doctor_fix(root, &arg("key")?)?),
        // Which khor this is and which store it is talking to. No node
        // is opened (`crate::about`), which is why it answers on a
        // machine whose store refuses — the moment the answer matters
        // most.
        "about" => to_json(&crate::about(root)),
        // 设置 (docs/MORE.md). Like the lock, a local preference read
        // straight off the store — no node is opened by either arm.
        "config" => to_json(&crate::config_rows(root)),
        "set_config" => to_json(&crate::set_config(root, &arg("name")?, &arg("value")?)?),
        "invite" => to_json(&crate::invite(root)?),
        // 加入网络's 扫码 (docs/IOS.md 第三步): whether this face can read
        // a pairing ticket's QR code with a camera. **A browser cannot**
        // — the scanner is the phone app's (`tauri-plugin-barcode-
        // scanner`), and the app answers for itself — so every HTTP skin
        // answers no, and the face draws no entry rather than one that
        // refuses (the step-2 rule: absent, not refused).
        "scanner_here" => to_json(&false),
        // 待批 (docs/CONNECT.md 根不在场): who waits for the root holder,
        // and the holder's one act on them. `approve` pushes the roster,
        // so it dials — async for `pair`'s reason.
        "pending_list" => to_json(&crate::waiting(root)?),
        "approve" => to_json(&rt.block_on(crate::approve(root, &arg("machine")?))?),
        // The root's words are not served here — `root_words` is app-only
        // (`tests/command_tables.rs`). The first-start prompt's two
        // questions (`root_unshown`, `root_words_seen`) went with the
        // prompt on 2026-10-01: the words are on demand.
        "pair" => to_json(&rt.block_on(crate::pair(root, &arg("ticket")?))?),
        // 下拉刷新 (the redesign): a sync round with every machine, now.
        // Async for `pair`'s reason: it dials.
        "sync_now" => to_json(&rt.block_on(crate::sync_now(root))?),
        // 离开这张网 (docs/MORE.md 离开). Async for `pair`'s reason: it
        // goes out over the wire before it touches this disk.
        "leave" => to_json(&rt.block_on(crate::leave(root, &arg("machine")?))?),
        // 暂停 / 接回来 (docs/MORE.md 暂停). No runtime: it writes one
        // small file, and the resident serve is what reads it.
        "set_paused" => {
            crate::set_paused(root, flag("on")?)?;
            to_json(&crate::about(root))
        }
        // 按机器暂停 (SHELL.md 第五轮 第 3 条): one other machine, by name.
        // Answers the sentence; the row's grey comes from the next
        // device list, which reads the same file.
        "set_machine_paused" => to_json(&crate::set_machine_paused(
            root,
            &arg("machine")?,
            flag("on")?,
        )?),
        other => Err(format!("no such command: {other}")),
    }
}

#[cfg(test)]
#[path = "api/watch_bridge_tests.rs"]
mod watch_bridge_tests;

fn to_json<T: serde::Serialize>(v: &T) -> Result<String, String> {
    serde_json::to_string(v).map_err(|e| e.to_string())
}
