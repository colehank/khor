//! Headless GUI backend for browser verification: the very functions the
//! tauri commands call, behind a loopback HTTP skin, with serve embedded
//! so peer reports keep flowing. A hand-written frontend mock would
//! drift from Rust; this cannot — it *is* the backend.
//!
//! Dev tool only: binds loopback, no auth, English messages.
//!
//! **The commands are not here.** They are [`khor_gui_core::api`]. What
//! is left in this file is the part that is only true of a dev tool:
//! loopback, CORS wide open, nobody asked for a key. A second server
//! that answers those three differently is the point of moving them —
//! it is not written yet (批⑦), and this file is what a reader should
//! copy nothing from when it is.
//!
//! One request at a time, still. Measured on this server, 2026-08-21:
//! a poll's own service time is ~3ms, but its p99 goes 3.5ms alone →
//! 70ms with two clients → 107ms with four, because the 35–40ms list
//! calls sit in front of it. That is a reason for a **product** server
//! to run workers; it is not a reason to re-time a 4600-line
//! acceptance run that has been green against this loop for weeks.

use std::io::Read;

use khor_node::Node;

fn main() {
    // First thing, before any bridge behaviour: if this process was
    // re-exec'd as a session host (`spawn_host` uses current_exe), be
    // one. Without this a host spawned from the bridge was another
    // bridge, and terminals waited forever (host.rs `main_if_host`).
    khor_node::host::main_if_host(khor_node::Node::root_from_env());
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
        // The belt under every handler: a panic in one request must be
        // that request's 500, not the whole server's death — the bridge
        // runs its loop on main, and one poisoned corner took the whole
        // preview down once (term.rs `plock` has the story).
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            khor_gui_core::api::answer(&rt, &root, &cmd, &args)
        }))
        .unwrap_or_else(|_| Err("panic in handler".to_owned()));
        let (code, payload) = match outcome {
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
