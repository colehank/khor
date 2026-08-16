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
        let (code, payload) = match handle(&root, &cmd, &args) {
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

fn handle(root: &std::path::Path, cmd: &str, args: &serde_json::Value) -> Result<String, String> {
    let id = || {
        args.get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing arg: id".to_owned())
    };
    match cmd {
        "sessions" => to_json(&khor_gui_core::list_sessions(root)?),
        "devices" => to_json(&khor_gui_core::list_devices(root)?),
        "seen" => {
            khor_gui_core::seen(root, id()?)?;
            Ok("null".to_owned())
        }
        "close_session" => {
            khor_gui_core::close_session(root, id()?)?;
            Ok("null".to_owned())
        }
        other => Err(format!("no such command: {other}")),
    }
}

fn to_json<T: serde::Serialize>(v: &T) -> Result<String, String> {
    serde_json::to_string(v).map_err(|e| e.to_string())
}
