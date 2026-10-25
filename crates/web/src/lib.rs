//! The web face: khor's own GUI, served to whatever browser opens this
//! machine's address. A phone, somebody else's laptop, a tablet on the
//! sofa — one address, the whole mesh (docs/KHOR.md 一张网,无中心).
//!
//! It is the same frontend the desktop app ships and the same backend
//! functions it calls ([`khor_gui_core::api`]); what this crate adds is
//! the part a LAN listener needs and an app inside a webview does not:
//! bytes for the page, a key on the door, and workers so two faces do
//! not queue behind each other.
//!
//! # What it deliberately is not
//!
//! **No TLS and nothing on the public internet.** Both are on the
//! ledger, unbuilt on purpose: this face is for the network the machine
//! is already on. Exposing it beyond that is a different product with a
//! different threat model, and half of it — a certificate for an
//! address that is a private IP — cannot be done honestly anyway.
//!
//! **No CORS headers, ever.** The page and the data come from the same
//! origin, so none are needed, and their absence is load-bearing: a
//! foreign page that somehow obtained a key still cannot read a reply,
//! because the browser will not hand it across origins. The dev bridge
//! sends `Access-Control-Allow-Origin: *` and is loopback-only for
//! exactly that reason.

pub mod key;
mod json_request;

mod assets {
    include!(concat!(env!("OUT_DIR"), "/assets.rs"));
}

/// The embedded frontend's receipt, so a running app and a served page can
/// be compared with the archive before installation.
pub fn ui_revision() -> &'static str {
    assets::UI_REVISION
}

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use khor_catalog::msg;

/// Where the face listens unless `KHOR_WEB_PORT` says otherwise.
///
/// **The number itself lives in `khor_node::face`**, and the reason is
/// the dependency graph rather than a preference: `khor doctor` has to
/// probe this port, `khor_web` depends on `khor_node`, so a doctor that
/// asked this crate would be a cycle. It is an installation fact — like
/// the store's own path — rather than a fact about the server that binds
/// it, so it sits with the other installation facts and is re-exported
/// here, where every caller already looks for it.
pub use khor_node::face::WEB_DEFAULT_PORT as DEFAULT_PORT;

/// How many requests the face answers at once.
///
/// **Measured, not guessed** (2026-08-21, dev bridge, one loop): a
/// poll's own service time is ~3ms, but the p99 a terminal feels goes
/// 3.5ms alone → 70ms with two faces → 107ms with four, because the
/// 35–40ms `sessions`/`devices` calls land in front of it. Four workers
/// is one per face in the case this exists for (a phone and a desktop,
/// each with a list pane and a live pane) with room to spare; the
/// registries underneath have always been `Mutex<HashMap>` shared with
/// per-session reader threads, so nothing below this line is newly
/// concurrent.
const WORKERS: usize = 4;

/// A face that is listening.
pub struct Face {
    /// Where it actually bound — the port a link must name.
    pub addr: SocketAddr,
}

/// The port to use, `KHOR_WEB_PORT` overriding [`DEFAULT_PORT`].
///
/// **One function rather than one line in each caller**: the resident
/// binds this port and `khor web` prints a link naming it, and those
/// two disagreeing would produce a link that is wrong in the one way
/// nobody checks — it looks exactly like a link that is right.
pub fn port() -> u16 {
    khor_node::face::web_port()
}

pub use khor_node::face::{link, best_address, answers_at};

/// Starts the face on every interface this machine has.
///
/// **Not loopback-with-a-flag.** The point of the face is a phone that
/// is not this machine, so the address that works is the reachable one;
/// a default nobody can reach, with a switch to make it work, is a
/// feature whose default is off. The key is what makes that safe, and
/// it is required from loopback too: on the shared machines this fleet
/// includes, `127.0.0.1` is reachable by every account on the box.
pub fn listen(root: PathBuf, port: u16) -> Result<Face, String> {
    // Minted before the socket, so that a face which is listening
    // always has a key to check against — the other order has a window
    // where every request is refused for a reason nobody can fix.
    key::ensure(&root)?;

    let server =
        tiny_http::Server::http(("0.0.0.0", port)).map_err(|e| msg::web_cant_listen(port, e))?;
    let addr = match server.server_addr() {
        tiny_http::ListenAddr::IP(a) => a,
        other => return Err(msg::web_cant_listen(port, format_args!("{other:?}"))),
    };

    // Multi-threaded on purpose: `answer` blocks on this runtime for
    // the verbs that dial, and a current-thread runtime that is blocked
    // has a **stopped clock** — every timeout inside it silently grows
    // (#73, `khor_node::reactor`).
    let rt = Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(msg::runtime_wont_start)?,
    );
    let server = Arc::new(server);
    for _ in 0..WORKERS {
        let (server, rt, root) = (server.clone(), rt.clone(), root.clone());
        std::thread::spawn(move || {
            while let Ok(req) = server.recv() {
                serve_one(&rt, &root, req);
            }
        });
    }
    Ok(Face { addr })
}

fn serve_one(rt: &tokio::runtime::Runtime, root: &Path, mut req: tiny_http::Request) {
    // The same belt the dev bridge wears: one poisoned corner must be
    // one request's 500, not the death of everybody's face.
    let outcome =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| answer(rt, root, &mut req)));
    let reply = outcome.unwrap_or_else(|_| Reply::refused(500, "panic in handler".to_owned()));
    let _ = req.respond(reply.into_response());
}

struct Reply {
    code: u16,
    kind: &'static str,
    body: Vec<u8>,
    /// A key to remember (记住一次批): set exactly when a page load
    /// presented a valid `?k=`, so the browser holds the credential and
    /// the next visit needs no query string.
    remember: Option<String>,
}

/// The credential cookie's name. The value is the key itself, verified
/// by the very same comparison as the query and header forms — a second
/// channel for one credential, never a second credential.
const COOKIE: &str = "khor_key";

impl Reply {
    fn refused(code: u16, sentence: String) -> Self {
        // Encoded as a JSON string because that is what the frontend's
        // one transport seam unwraps (`api.ts`); a bare sentence would
        // arrive wearing quotation marks.
        let body = serde_json::to_vec(&sentence).unwrap_or_else(|_| b"\"error\"".to_vec());
        Reply {
            code,
            kind: "application/json",
            body,
            remember: None,
        }
    }

    /// The door shut in a browser's face: text a person reads, telling
    /// them where the key is handed out. HTML content type so the
    /// browser renders the sentence instead of downloading it.
    fn refused_page(sentence: &str) -> Self {
        let body = format!(
            "<!doctype html><meta charset=\"utf-8\"><body style=\"font-family:system-ui;display:grid;place-items:center;height:100dvh;margin:0\"><p style=\"max-width:36em;padding:1.5em\">{sentence}</p>"
        );
        Reply {
            code: 403,
            kind: "text/html; charset=utf-8",
            body: body.into_bytes(),
            remember: None,
        }
    }

    fn into_response(self) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
        let header = |h: &str| h.parse::<tiny_http::Header>().expect("a literal header");
        let mut r = tiny_http::Response::from_data(self.body)
            .with_status_code(self.code)
            .with_header(header(&format!("Content-Type: {}", self.kind)));
        // **The key rides in the URL for exactly one page load**, and
        // an address bar is a thing browsers hand onward. This stops
        // the obvious leg of that: any link the face paints — and it
        // paints links an agent wrote — would otherwise carry the key
        // to a stranger's access log in the `Referer`.
        r.add_header(header("Referrer-Policy: no-referrer"));
        r.add_header(header(&format!("X-Khor-UI: {}", ui_revision())));
        if let Some(key) = &self.remember {
            // HttpOnly: the page's own scripts keep their existing
            // channels (localStorage + Authorization) and never need to
            // read this; a cookie no script can read is one script
            // injection cannot steal. SameSite=Strict: the cookie only
            // travels on same-site requests, one more wall in front of
            // the rebinding shapes `check_host` stops. Max-Age a year:
            // "remember once" has to actually remember, and `khor web
            // --new` revokes every copy at any moment by changing what
            // the cookie is compared against. No `Secure` — this face
            // is deliberately plain HTTP on the LAN (module head).
            r.add_header(header(&format!(
                "Set-Cookie: {COOKIE}={key}; Path=/; SameSite=Strict; HttpOnly; Max-Age=31536000"
            )));
        }
        r
    }
}

fn answer(rt: &tokio::runtime::Runtime, root: &Path, req: &mut tiny_http::Request) -> Reply {
    let host = header_of(req, "host").unwrap_or_default();
    if let Err(sentence) = check_host(&host) {
        return Reply::refused(403, sentence);
    }
    let url = req.url().to_owned();
    let (path, query) = url.split_once('?').unwrap_or((url.as_str(), ""));
    match path.strip_prefix("/api/") {
        // The page sits behind the key too (记住一次批). Three doors,
        // one lock: a valid `?k=` serves the page AND plants the
        // cookie (so the next visit needs no query string); a valid
        // cookie or Bearer serves it bare; nothing valid gets a
        // sentence a person can act on — not a blank app that loads
        // and then starves, which is what "public page, locked data"
        // used to hand a keyless visitor.
        None => {
            let fresh = query_key(query)
                .filter(|k| matches!(key::read(root), Ok(Some(real)) if key::matches(k, &real)));
            if fresh.is_none() && check_key(req, root).is_err() {
                return Reply::refused_page(msg::web_door_shut());
            }
            let mut reply = page(path);
            reply.remember = fresh;
            reply
        }
        Some(cmd) => {
            // Every /api/ door wears every lock — the upload included:
            // it writes this machine's disk, which is more reason for
            // the key, not less (docs/NET.md 网页脸的暴露面, one rule
            // relaxed nowhere).
            if let Err(sentence) = check_origin(req, &host) {
                return Reply::refused(403, sentence);
            }
            if let Err(sentence) = check_key(req, root) {
                return Reply::refused(401, sentence);
            }
            // The one raw-bytes route: a browser has no path to name, so
            // the file rides the body whole and the arguments ride the
            // query (a JSON envelope around a gigabyte would mean
            // base64ing it through memory). Streamed to disk with the
            // cap enforced mid-stream (`khor_gui_core::stage_upload`).
            // A file for an agent conversation (round CHAT): the same
            // raw body, put on the machine the conversation runs on.
            if cmd == "chat_put" {
                let Some((id, name)) = khor_gui_core::chat_put_args(query) else {
                    return Reply::refused(400, msg::upload_needs_machine_and_name().to_owned());
                };
                let into = khor_gui_core::into_arg(query);
                return match khor_gui_core::chat::chat_put(root, &id, &name, &into, req.as_reader()) {
                    Ok(path) => Reply {
                        code: 200,
                        kind: "application/json",
                        body: serde_json::to_vec(&path).unwrap_or_default(),
                        remember: None,
                    },
                    Err(sentence) => Reply::refused(400, sentence),
                };
            }
            // A picked file waiting here for a terminal's drop (item 60
            // part 3): the same raw body, staged where this machine's
            // node serves it.
            if cmd == "term_drop_stage_into" {
                let Some((folder,path))=khor_gui_core::drop_stage_into_args(query) else { return Reply::refused(400,"term_drop_stage_into needs folder and path".to_owned()); };
                return match khor_gui_core::term::term_drop_stage_into(root,&folder,&path,req.as_reader()) {
                    Ok(path)=>Reply {code:200,kind:"application/json",body:serde_json::to_vec(&path).unwrap_or_default(),remember:None},
                    Err(why)=>Reply::refused(400,why),
                };
            }
            if cmd == "term_drop_stage" {
                let Some(name) = khor_gui_core::drop_stage_args(query) else {
                    return Reply::refused(400, "term_drop_stage needs a name query arg".to_owned());
                };
                return match khor_gui_core::term::term_drop_stage(root, &name, req.as_reader()) {
                    Ok(path) => Reply {
                        code: 200,
                        kind: "application/json",
                        body: serde_json::to_vec(&path).unwrap_or_default(),
                        remember: None,
                    },
                    Err(sentence) => Reply::refused(400, sentence),
                };
            }
            // A file for the broadcast's composer, staged here and
            // answered by its path; the post that carries it is a JSON
            // command of its own (`post`).
            if cmd == "upload" {
                let Some(name) = khor_gui_core::upload_args(query) else {
                    return Reply::refused(400, msg::upload_needs_machine_and_name().to_owned());
                };
                return match khor_gui_core::stage_upload(root, &name, req.as_reader()) {
                    Ok(path) => Reply {
                        code: 200,
                        kind: "application/json",
                        body: serde_json::to_vec(&path).unwrap_or_default(),
                        remember: None,
                    },
                    Err(sentence) => Reply::refused(400, sentence),
                };
            }
            // Read only once the request is allowed to be here: a body
            // from somebody who has not shown a key is a body khor has
            // no reason to pull into memory.
            match json_request::dispatch(cmd, req.as_reader(), |args| {
                khor_gui_core::api::answer(rt, root, cmd, args)
            }) {
                Ok(json) => Reply {
                    code: 200,
                    kind: "application/json",
                    body: json.into_bytes(),
                    remember: None,
                },
                Err(error) => Reply::refused(error.code, error.message),
            }
        }
    }
}

/// One file out of the binary. `/` is the page; anything unnamed is a
/// 404 rather than the page, because this app has no client-side routes
/// — an unknown path is a mistake, and answering it with the shell
/// would make a typo look like a blank screen.
fn page(path: &str) -> Reply {
    let wanted = if path == "/" { "/index.html" } else { path };
    match assets::ASSETS.iter().find(|(p, _)| *p == wanted) {
        Some((p, bytes)) => Reply {
            code: 200,
            kind: content_type(p),
            body: bytes.to_vec(),
            remember: None,
        },
        None => Reply::refused(404, msg::web_no_such_page(path)),
    }
}

/// What vite emits, and nothing else. A table rather than a crate: the
/// set of extensions in `dist` is small, known, and checked by the test
/// below — a guessing library would be a package for six answers.
fn content_type(path: &str) -> &'static str {
    match path.rsplit_once('.').map(|(_, ext)| ext) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("json") => "application/json",
        Some("png") => "image/png",
        Some("woff2") => "font/woff2",
        // The terminal's symbols fallback (`apps/gui/src/assets/fonts`).
        Some("ttf") => "font/ttf",
        Some("ico") => "image/x-icon",
        _ => "application/octet-stream",
    }
}

/// **The lock against DNS rebinding.**
///
/// The attack: a page at `evil.com` whose name resolves, on the second
/// look, to this machine's LAN address. The browser then treats that
/// page as same-origin with the face and can read every reply — but the
/// request it sends still says `Host: evil.com`, because that is the
/// name it was asked for.
///
/// So: a **name** is never acceptable, only an address. `localhost` and
/// `*.local` are the two exceptions and both are safe for the same
/// reason — neither is resolvable by anybody off this network, so
/// neither can be pointed at us by a stranger.
///
/// What it does not stop: somebody on this LAN, or another account on
/// this machine, who has the key. That is the key's job, and neither
/// lock covers the other's ground.
fn check_host(host: &str) -> Result<(), String> {
    let name = host
        .rsplit_once(':')
        .map_or(host, |(h, _)| h)
        .trim_matches(['[', ']']);
    let acceptable =
        name == "localhost" || name.ends_with(".local") || name.parse::<std::net::IpAddr>().is_ok();
    if acceptable {
        Ok(())
    } else {
        Err(msg::web_bad_host(name))
    }
}

/// Same origin or no origin. A browser sends `Origin` on every request
/// this frontend makes; a non-browser (curl, a script, the CLI) sends
/// none and is judged by the key alone.
fn check_origin(req: &tiny_http::Request, host: &str) -> Result<(), String> {
    match header_of(req, "origin") {
        None => Ok(()),
        Some(origin) => {
            let ours = format!("http://{host}");
            if origin == ours {
                Ok(())
            } else {
                Err(msg::web_bad_origin(&origin))
            }
        }
    }
}

/// **The door.** Required from every address including loopback: the
/// machines this fleet includes are shared, and on a shared box
/// `127.0.0.1` is not a boundary — it is a hallway.
///
/// The credential may arrive as a Bearer header (the page's scripts) or
/// as the remembered cookie (a bare visit, 记住一次批) — two channels,
/// one credential, one comparison. `khor web --new` revokes every
/// outstanding cookie by changing what both are compared against; the
/// cookie is never a second trust root, and it excuses nothing else:
/// Host and Origin are checked before this is ever asked.
fn check_key(req: &tiny_http::Request, root: &Path) -> Result<(), String> {
    let real = match key::read(root)? {
        Some(k) => k,
        None => return Err(msg::web_no_key_yet().to_owned()),
    };
    let presented = header_of(req, "authorization")
        .and_then(|h| h.strip_prefix("Bearer ").map(str::to_owned))
        .or_else(|| cookie_key(req))
        .unwrap_or_default();
    if presented.is_empty() {
        return Err(msg::web_no_key().to_owned());
    }
    if key::matches(&presented, &real) {
        Ok(())
    } else {
        Err(msg::web_wrong_key().to_owned())
    }
}

/// The remembered key out of the Cookie header, if one rode along.
fn cookie_key(req: &tiny_http::Request) -> Option<String> {
    let jar = header_of(req, "cookie")?;
    jar.split(';').find_map(|pair| {
        let (name, value) = pair.trim().split_once('=')?;
        (name == COOKIE && !value.is_empty()).then(|| value.to_owned())
    })
}

/// The `k` off a page URL's query string — the form `khor web` prints.
/// Values are plain hex, so no decoding is owed.
fn query_key(query: &str) -> Option<String> {
    query.split('&').find_map(|pair| {
        let (name, value) = pair.split_once('=')?;
        (name == "k" && !value.is_empty()).then(|| value.to_owned())
    })
}

fn header_of(req: &tiny_http::Request, name: &str) -> Option<String> {
    // Compared case-insensitively by hand rather than with
    // `field.equiv`, which wants a `&'static str` and would make every
    // header name in this file a literal at the call site.
    req.headers()
        .iter()
        .find(|h| h.field.as_str().as_str().eq_ignore_ascii_case(name))
        .map(|h| h.value.as_str().to_owned())
}

#[cfg(test)]
mod tests {
    use super::{assets, check_host, content_type};

    /// **The face has a face.** An empty asset table compiles, serves
    /// 404 for everything, and reads exactly like a build that worked —
    /// the same shape as a finder that stopped matching and returned an
    /// empty set.
    #[test]
    fn the_frontend_really_got_embedded() {
        assert!(super::ui_revision().starts_with("khor-ui-assets:/assets/index-"));
        assert!(assets::ASSETS.iter().any(|(p, _)| p.ends_with(".js") && super::ui_revision().contains(*p)));
        assert!(
            assets::ASSETS.iter().any(|(p, _)| *p == "/index.html"),
            "no /index.html in the embedded table: {:?}",
            assets::ASSETS.iter().map(|(p, _)| *p).collect::<Vec<_>>()
        );
        let (_, html) = assets::ASSETS
            .iter()
            .find(|(p, _)| *p == "/index.html")
            .unwrap();
        assert!(
            html.len() > 100,
            "index.html is {} bytes — that is not a page",
            html.len()
        );
        assert!(
            assets::ASSETS.iter().any(|(p, _)| p.ends_with(".js")),
            "a page with no script is a page with no app"
        );
    }

    /// Every embedded file is a kind this server can name. The default
    /// arm exists for safety, not as an answer: a `.woff` or a `.webp`
    /// arriving as `application/octet-stream` is a font a browser
    /// refuses to use and an image it will not draw.
    #[test]
    fn nothing_shipped_falls_through_to_the_default_type() {
        for (p, _) in assets::ASSETS {
            assert_ne!(
                content_type(p),
                "application/octet-stream",
                "{p} has no content type — add its extension to `content_type`"
            );
        }
    }

    #[test]
    fn only_an_address_may_name_this_machine() {
        for good in [
            "192.168.1.7:5467",
            "10.0.0.2",
            "127.0.0.1:5467",
            "localhost:5467",
            "macmini.local:5467",
            "[::1]:5467",
        ] {
            assert!(check_host(good).is_ok(), "must accept {good}");
        }
        // The rebinding shape: a name that resolves here, arriving as
        // the name. Also the plain mistake of a reverse proxy nobody
        // configured.
        for bad in ["evil.example", "khor.example.com:5467", "attacker.io"] {
            assert!(check_host(bad).is_err(), "must refuse {bad}");
        }
    }
}

#[cfg(test)]
#[path = "http_input_tests.rs"]
mod http_input_tests;
