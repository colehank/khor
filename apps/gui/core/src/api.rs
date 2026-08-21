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
        "devices" => to_json(&crate::list_devices(root)?),
        "usage" => to_json(&crate::usage(root)?),
        "seen" => {
            crate::seen(root, &arg("id")?)?;
            Ok("null".to_owned())
        }
        "close_session" => {
            crate::close_session(root, &arg("id")?)?;
            Ok("null".to_owned())
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
        "face_choices" => to_json(&crate::face_choices(root)?),
        "restyle" => {
            crate::restyle(root, opt_colors()?, opt_str("variant")?, opt_str("shape")?)?;
            Ok("null".to_owned())
        }
        "hooks_state" => to_json(&crate::hooks_state(root)?),
        "install_hooks" => to_json(&crate::install_hooks(root)?),
        "uninstall_hooks" => to_json(&crate::uninstall_hooks(root)?),
        "chat_open" => to_json(&crate::chat::chat_open(root, &arg("id")?)?),
        "chat_poll" => to_json(&crate::chat::chat_poll(&arg("id")?, num("since")?)?),
        "chat_say" => {
            crate::chat::chat_say(&arg("id")?, &arg("text")?)?;
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
        "pin_dir" => {
            crate::files::pin_dir(root, &arg("machine")?, &arg("path")?, flag("on")?)?;
            Ok("null".to_owned())
        }
        "term_open" => {
            crate::term::term_open(root, &arg("id")?, num("cols")? as u16, num("rows")? as u16)?;
            Ok("null".to_owned())
        }
        "term_poll" => to_json(&crate::term::term_poll(
            &arg("id")?,
            num("since")?,
            num("back")? as usize,
        )?),
        "term_key" => {
            crate::term::term_key(&arg("id")?, arg("keys")?.into_bytes())?;
            Ok("null".to_owned())
        }
        "term_paste" => {
            crate::term::term_paste(&arg("id")?, &arg("text")?)?;
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
        "web_pins" => to_json(&crate::web::web_pins(root)?),
        "pin_web" => {
            crate::web::pin_web(root, &arg("machine")?, &arg("url")?, flag("on")?)?;
            Ok("null".to_owned())
        }
        // The bridge answers the borrow half only — it has no webview to
        // point at the proxy, so it returns where the proxy listens and
        // the window half stays the tauri skin's (`web::borrow_web`).
        "open_web" => to_json(&rt.block_on(crate::web::borrow_web(root, &arg("machine")?))?),
        "takeover" => {
            crate::takeover(root, &arg("id")?)?;
            Ok("null".to_owned())
        }
        "takeover_term" => {
            crate::takeover_term(root, &arg("id")?)?;
            Ok("null".to_owned())
        }
        "open_session" => to_json(&crate::open_session(
            root,
            &arg("dir")?,
            &opt_str("title")?.unwrap_or_default(),
            &arg("form")?,
            &opt_str("agent")?.unwrap_or_default(),
        )?),
        "agents" => to_json(&crate::agents(root)?),
        "invite" => to_json(&crate::invite(root)?),
        "pair" => to_json(&rt.block_on(crate::pair(root, &arg("ticket")?))?),
        other => Err(format!("no such command: {other}")),
    }
}

fn to_json<T: serde::Serialize>(v: &T) -> Result<String, String> {
    serde_json::to_string(v).map_err(|e| e.to_string())
}
