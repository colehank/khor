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
//! One request at a time, still — but for a terminal's drop, which runs
//! on a thread of its own (item 71: its × must reach it). Measured on
//! this server, 2026-08-21:
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
    // 语言 (docs/MORE.md), the same call every other binary makes at
    // startup: the browser face resolves its own, and this is the Rust
    // half behind it — refusals and 诊断 rows cross back as text.
    khor_node::config::apply_lang(&root);
    // This machine's readings, before the first request can ask for
    // them (docs/SHELL.md 第十六轮). `BRIDGE_SLOW_VOLUME_MS` is the
    // `npm run fast` gate's seam: two volumes that do not exist, one of
    // which takes that long to answer — a sleeping disk without a real
    // disk in it.
    match std::env::var("BRIDGE_SLOW_VOLUME_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
    {
        Some(ms) => {
            khor_node::vitals::local::install(
                &root,
                std::sync::Arc::new(SlowVolume(std::time::Duration::from_millis(ms))),
                khor_node::vitals::local::EVERY,
            );
        }
        None => khor_node::vitals::local::start(&root),
    }
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
        let url = req.url().trim_start_matches('/').to_owned();
        let (cmd, query) = url.split_once('?').unwrap_or((url.as_str(), ""));
        let (cmd, query) = (cmd.to_owned(), query.to_owned());
        // Preflight: the vite origin differs from ours.
        if req.method() == &tiny_http::Method::Options {
            let _ = req.respond(with_cors(
                tiny_http::Response::from_string("").with_status_code(204),
            ));
            continue;
        }
        // The raw-bytes route, before the JSON read below — a file body
        // is not UTF-8 and must never pass through a String. Same
        // shared body as the web face's `/api/upload`, so the two
        // faces cannot come to enforce different rules.
        if cmd == "upload" || cmd == "chat_put" || cmd == "term_drop_stage" {
            // `chat_put`: the bytes of a file for an agent conversation,
            // put on the machine it runs on (round CHAT); `upload`: a
            // file staged for the broadcast's composer; `term_drop_stage`: a picked file
            // waiting here for a terminal's drop (item 60 part 3). One
            // body route for all three.
            let outcome = if cmd == "term_drop_stage" {
                khor_gui_core::drop_stage_args(&query)
                    .ok_or_else(|| "term_drop_stage needs a name query arg".to_owned())
                    .and_then(|name| khor_gui_core::term::term_drop_stage(&root, &name, req.as_reader()))
            } else if cmd == "chat_put" {
                khor_gui_core::chat_put_args(&query)
                    .ok_or_else(|| "chat_put needs id and name query args".to_owned())
                    .and_then(|(id, name)| {
                        khor_gui_core::chat::chat_put(&root, &id, &name, &khor_gui_core::into_arg(&query), req.as_reader())
                    })
            } else {
                khor_gui_core::upload_args(&query)
                    .ok_or_else(|| "upload needs a name query arg".to_owned())
                    .and_then(|name| khor_gui_core::stage_upload(&root, &name, req.as_reader()))
            };
            let (code, payload) = match outcome {
                Ok(id) => (200, serde_json::to_string(&id).unwrap_or_default()),
                Err(e) => (
                    400,
                    serde_json::to_string(&e).unwrap_or_else(|_| "\"error\"".into()),
                ),
            };
            let _ = req.respond(with_cors(
                tiny_http::Response::from_string(payload)
                    .with_status_code(code)
                    .with_header(
                        "Content-Type: application/json"
                            .parse::<tiny_http::Header>()
                            .unwrap(),
                    ),
            ));
            continue;
        }
        let mut body = String::new();
        let _ = req.as_reader().read_to_string(&mut body);
        let args: serde_json::Value =
            serde_json::from_str(&body).unwrap_or(serde_json::Value::Null);
        // **A drop runs beside the loop** (item 71): one `term_drop`
        // holds its request for as long as its files take to land, and
        // behind it the pill's progress asks and its × waited until
        // there was nothing left to cancel. Its own thread and runtime;
        // the one-at-a-time rule above stands for everything else.
        if cmd == "term_drop" {
            let root = root.clone();
            std::thread::spawn(move || {
                let outcome = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                    Ok(rt) => answered(&rt, &root, &cmd, &args),
                    Err(e) => Err(e.to_string()),
                };
                let _ = req.respond(json_response(outcome));
            });
            continue;
        }
        let _ = req.respond(json_response(answered(&rt, &root, &cmd, &args)));
    }
}

/// One command's answer. The belt under every handler: a panic in one
/// request must be that request's 500, not the whole server's death —
/// the bridge runs its loop on main, and one poisoned corner took the
/// whole preview down once (term.rs `plock` has the story).
fn answered(
    rt: &tokio::runtime::Runtime,
    root: &std::path::Path,
    cmd: &str,
    args: &serde_json::Value,
) -> Result<String, String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        khor_gui_core::api::answer(rt, root, cmd, args)
    }))
    .unwrap_or_else(|_| Err("panic in handler".to_owned()))
}

fn json_response(
    outcome: Result<String, String>,
) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
    let (code, payload) = match outcome {
        Ok(json) => (200, json),
        Err(e) => (
            400,
            serde_json::to_string(&e).unwrap_or_else(|_| "\"error\"".into()),
        ),
    };
    with_cors(
        tiny_http::Response::from_string(payload)
            .with_status_code(code)
            .with_header(
                "Content-Type: application/json"
                    .parse::<tiny_http::Header>()
                    .unwrap(),
            ),
    )
}

/// The disks `BRIDGE_SLOW_VOLUME_MS` puts under this bridge: one that
/// answers at once and one that answers after the given time. Neither
/// exists, so a gate run never touches a real disk.
struct SlowVolume(std::time::Duration);

impl khor_node::vitals::local::Probe for SlowVolume {
    fn mounts(&self) -> Vec<String> {
        vec![
            "/khor-gate-fast-volume".into(),
            "/khor-gate-slow-volume".into(),
        ]
    }

    fn space(&self, mount: &str) -> Option<(u64, u64)> {
        if mount == "/khor-gate-slow-volume" {
            std::thread::sleep(self.0);
        }
        Some((500 << 30, 200 << 30))
    }
}

fn with_cors<R: Read>(r: tiny_http::Response<R>) -> tiny_http::Response<R> {
    r.with_header(
        "Access-Control-Allow-Origin: *"
            .parse::<tiny_http::Header>()
            .unwrap(),
    )
    .with_header(
        "Access-Control-Allow-Headers: content-type"
            .parse::<tiny_http::Header>()
            .unwrap(),
    )
    .with_header(
        "Access-Control-Allow-Methods: POST, OPTIONS"
            .parse::<tiny_http::Header>()
            .unwrap(),
    )
}
