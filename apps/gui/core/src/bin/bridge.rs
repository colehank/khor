//! Headless GUI backend for browser verification: the very functions the
//! tauri commands call, behind a loopback HTTP skin, with serve embedded
//! so peer reports keep flowing. A hand-written frontend mock would
//! drift from Rust; this cannot — it *is* the backend.
//!
//! Dev tool only: binds loopback, no auth, English messages.

use std::io::Read;

use khor_node::Node;

fn main() {
    let root = Node::root_from_env();
    let port: u16 = std::env::var("BRIDGE_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(1431);

    // The sync pump: without serve, reported rows never refresh and the
    // seen watermark never travels. Own thread, own runtime — the HTTP
    // loop below is synchronous.
    let serve_root = root.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        rt.block_on(async {
            match Node::open(serve_root) {
                Ok(n) => {
                    if let Err(e) = n.serve().await {
                        eprintln!("serve ended: {e}");
                    }
                }
                Err(e) => eprintln!("serve did not start: {e}"),
            }
        });
    });

    // A runtime for the one-shot verbs that dial (`pair`). Separate from
    // serve's: this loop is synchronous and blocks on each call.
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");

    let server = tiny_http::Server::http(("127.0.0.1", port)).expect("bind loopback");
    println!("bridge listening on 127.0.0.1:{port}");
    for mut req in server.incoming_requests() {
        let cmd = req.url().trim_start_matches('/').to_owned();
        // Preflight: the vite origin differs from ours.
        if req.method() == &tiny_http::Method::Options {
            let _ = req.respond(with_cors(tiny_http::Response::from_string("").with_status_code(204)));
            continue;
        }
        let mut body = String::new();
        let _ = req.as_reader().read_to_string(&mut body);
        let args: serde_json::Value =
            serde_json::from_str(&body).unwrap_or(serde_json::Value::Null);
        let (code, payload) = match handle(&rt, &root, &cmd, &args) {
            Ok(json) => (200, json),
            Err(e) => (400, serde_json::to_string(&e).unwrap_or_else(|_| "\"error\"".into())),
        };
        let resp = with_cors(
            tiny_http::Response::from_string(payload)
                .with_status_code(code)
                .with_header("Content-Type: application/json".parse::<tiny_http::Header>().unwrap()),
        );
        let _ = req.respond(resp);
    }
}

fn with_cors<R: Read>(r: tiny_http::Response<R>) -> tiny_http::Response<R> {
    r.with_header("Access-Control-Allow-Origin: *".parse::<tiny_http::Header>().unwrap())
        .with_header("Access-Control-Allow-Headers: content-type".parse::<tiny_http::Header>().unwrap())
        .with_header("Access-Control-Allow-Methods: POST, OPTIONS".parse::<tiny_http::Header>().unwrap())
}

fn handle(
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
    match cmd {
        "sessions" => to_json(&khor_gui_core::list_sessions(root, &arg("by")?)?),
        "devices" => to_json(&khor_gui_core::list_devices(root)?),
        "usage" => to_json(&khor_gui_core::usage(root)?),
        "codex_quota" => to_json(&khor_gui_core::codex_quota(root)?),
        "seen" => {
            khor_gui_core::seen(root, &arg("id")?)?;
            Ok("null".to_owned())
        }
        "close_session" => {
            khor_gui_core::close_session(root, &arg("id")?)?;
            Ok("null".to_owned())
        }
        "tell" => {
            khor_gui_core::tell(root, &arg("machine")?, &arg("text")?)?;
            Ok("null".to_owned())
        }
        "pin_session" => {
            khor_gui_core::pin_session(root, &arg("id")?, flag("on")?)?;
            Ok("null".to_owned())
        }
        "pin_device" => {
            khor_gui_core::pin_device(root, &arg("machine")?, flag("on")?)?;
            Ok("null".to_owned())
        }
        "face_choices" => to_json(&khor_gui_core::face_choices(root)?),
        "restyle" => {
            khor_gui_core::restyle(root, opt_colors()?, opt_str("variant")?, opt_str("shape")?)?;
            Ok("null".to_owned())
        }
        "hooks_state" => to_json(&khor_gui_core::hooks_state(root)?),
        "install_hooks" => to_json(&khor_gui_core::install_hooks(root)?),
        "uninstall_hooks" => to_json(&khor_gui_core::uninstall_hooks(root)?),
        "invite" => to_json(&khor_gui_core::invite(root)?),
        "pair" => to_json(&rt.block_on(khor_gui_core::pair(root, &arg("ticket")?))?),
        other => Err(format!("no such command: {other}")),
    }
}

fn to_json<T: serde::Serialize>(v: &T) -> Result<String, String> {
    serde_json::to_string(v).map_err(|e| e.to_string())
}
