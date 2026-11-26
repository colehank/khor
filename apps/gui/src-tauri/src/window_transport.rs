//! Windows desktop transport. In the Windows 11 reproduction, Tauri's
//! WebView2 custom-protocol requests stalled after the startup burst. The
//! main webview uses this authenticated loopback listener for commands.

use std::io::{self, Read};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use subtle::ConstantTimeEq;
use tauri::Manager;

fn main_window(app: &tauri::AppHandle) -> Result<tauri::Window, String> {
    // **`get_window`, not `get_webview_window`.** A Tauri window that has
    // child webviews at it — the browsing face is added with `add_child`
    // (`lib.rs`) — is no longer returned by `get_webview_window(label)`:
    // that call answers `None`. The `Window` remains available after a
    // browser tab adds a child webview.
    app.get_window("main")
        .ok_or_else(|| "no main window".to_owned())
}

static CLOSE_GUARD_READY: AtomicBool = AtomicBool::new(false);

/// Give the main page a chance to save drafts before retiring the window.
pub fn notify_close_guard(window: &tauri::Window) -> bool {
    CLOSE_GUARD_READY.load(Ordering::Acquire)
        && window.app_handle().get_webview("main").is_some_and(|view| {
            view.eval("window.dispatchEvent(new Event('khor:windows-close-requested'))")
                .is_ok()
        })
}

/// Commands owned by the window, outside the shared command table.
fn window_verb(app: &tauri::AppHandle, cmd: &str) -> Option<Result<String, String>> {
    let window = || main_window(app);
    let answer = match cmd {
        "window_minimize" => window().and_then(|w| w.minimize().map_err(|e| e.to_string())),
        "window_maximize_toggle" => window().and_then(|w| {
            let full = w.is_maximized().map_err(|e| e.to_string())?;
            if full {
                w.unmaximize().map_err(|e| e.to_string())
            } else {
                w.maximize().map_err(|e| e.to_string())
            }
        }),
        // `close`, not `destroy`: closing is a request, and the draft guard
        // hangs on it (`lib.rs`'s own note on `window_close`).
        //
        // **No watchdog here.** An earlier version destroyed the window
        // after four seconds "in case the face never answers", and an
        // independent review was right to flag it: the guard's answer is
        // not on a deadline — `flushAll` has no timeout, and a save that
        // fails opens the unsaved-drafts dialog, where a person may choose
        // to keep working. Force-destroying through that choice trades a
        // stuck window for lost work. The main window's `Destroyed` handler
        // retires its menu and gives editor windows their own close request.
        "window_close" => window().and_then(|w| w.close().map_err(|e| e.to_string())),
        "window_close_guard_ready" => {
            CLOSE_GUARD_READY.store(true, Ordering::Release);
            Ok(())
        }
        // After the native close request, the frontend checks its drafts
        // before invoking destroy. This completes that same close without
        // raising a second close request.
        "window_destroy" => window().and_then(|w| w.destroy().map_err(|e| e.to_string())),
        // **Dragging.** Tauri's own `drag.js` calls
        // `invoke('plugin:window|start_dragging')` on a `data-tauri-drag-region`
        // mousedown, which is the same dead custom-protocol road as every other
        // invoke. `drag_shim` keeps the app's shared drag regions usable
        // as well as the native caption.
        "window_drag" => window().and_then(|w| w.start_dragging().map_err(|e| e.to_string())),
        // Windows has no system buttons standing over the title row, so the
        // row reserves nothing for them (the same zero the skin answers off
        // macOS).
        "traffic_room" => return Some(Ok("0".to_owned())),
        _ => return None,
    };
    Some(answer.map(|()| "null".to_owned()))
}

const TOKEN_HEADER: &str = "x-khor-window-token";

// Tauri uses the bundled HTTP origin on Windows. When custom-protocol is
// disabled, it uses the Vite URL from tauri.conf.json instead.
#[cfg(not(dev))]
const WINDOW_ORIGIN: &str = "http://tauri.localhost";
#[cfg(dev)]
const WINDOW_ORIGIN: &str = "http://localhost:1430";

/// Apply the Tauri fallback from document start to every owned Windows
/// webview, including the menu and editors. Normal browser requests and
/// the authenticated loopback commands keep their usual fetch path.
pub fn tauri_message_transport_script() -> String {
    let origin = serde_json::Value::String(WINDOW_ORIGIN.to_owned());
    format!("\n;({})({origin});", include_str!("windows_ipc.js"))
}

fn header<'a>(request: &'a tiny_http::Request, name: &'static str) -> Option<&'a str> {
    let mut values = request.headers().iter().filter(|h| h.field.equiv(name));
    let value = values.next()?.value.as_str();
    values.next().is_none().then_some(value)
}

fn cors<R: Read>(response: tiny_http::Response<R>, origin: &str) -> tiny_http::Response<R> {
    response
        .with_header(tiny_http::Header::from_bytes("Access-Control-Allow-Origin", origin).unwrap())
        .with_header(
            "Access-Control-Allow-Headers: content-type, x-khor-window-token"
                .parse::<tiny_http::Header>()
                .unwrap(),
        )
        .with_header(
            "Access-Control-Allow-Methods: POST, OPTIONS"
                .parse::<tiny_http::Header>()
                .unwrap(),
        )
        .with_header("Vary: Origin".parse::<tiny_http::Header>().unwrap())
}

fn json(
    outcome: Result<String, String>,
    origin: &str,
) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
    let (code, payload) = match outcome {
        Ok(json) => (200, json),
        Err(why) => (
            400,
            serde_json::to_string(&why).unwrap_or_else(|_| "\"error\"".into()),
        ),
    };
    cors(
        tiny_http::Response::from_string(payload)
            .with_status_code(code)
            .with_header(
                "Content-Type: application/json"
                    .parse::<tiny_http::Header>()
                    .unwrap(),
            ),
        origin,
    )
}

/// How many requests the transport answers at once.
///
/// **Not one.** The window polls `sessions` every 2 s, `devices` and
/// `day_strips` on their own beats, and `tabs_list` twice a second — and some
/// commands are slow: the W7 matrix measured `session_create` at 6.4 s and
/// `cli_doctor` at 5.9 s on this platform. A single-threaded loop meant one slow
/// command held every poll behind it, which is what "the app feels stuck" looks
/// like from outside (reported by the owner 2026-11-16, and the same reason the
/// dev bridge has its own note about workers).
///
/// Four is enough for the poll set plus one person-initiated command: the polls
/// are the ones that must never wait, and `tiny_http` hands each thread its own
/// request.
pub const WORKERS: usize = 4;

/// One command's answer, through the shared table.
///
/// The belt under the handler, copied from the dev bridge for the same reason:
/// a panic in one command must be that request's 500, not the death of the
/// transport the whole window depends on.
///
/// **The runtime is the caller's**, built once per worker thread. Building one
/// per request (what this did first) paid a runtime setup on every poll and had
/// nowhere to keep a driver between them.
fn answer(
    rt: &tokio::runtime::Runtime,
    cmd: &str,
    args: &serde_json::Value,
) -> Result<String, String> {
    let root = khor_node::Node::root_from_env();
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        khor_gui_core::api::answer(rt, &root, cmd, args)
    }))
    .unwrap_or_else(|_| Err("panic in handler".to_owned()))
}

// The web face's browser commands only arrange a network borrow. In the
// desktop app these commands also own a child WebView and its lifecycle.
fn native_command(
    app: &tauri::AppHandle,
    rt: &tokio::runtime::Runtime,
    cmd: &str,
    args: &serde_json::Value,
) -> Option<Result<String, String>> {
    use crate::{
        close_page, hide_page, move_page, open_code_window, open_page, page_state, page_step,
        page_steps, show_page,
    };

    let text = |key| argument::<String>(args, key);
    let tab = || text("tab");
    let webview = || {
        app.get_webview("main")
            .ok_or_else(|| "no main webview".to_owned())
    };
    use crate::updater_commands as updates;
    let events = || {
        app.try_state::<crate::window_events::MainWindowEvents>()
            .ok_or_else(|| "main window event bridge is not ready".to_owned())
    };
    let answer = || -> Result<Option<String>, String> {
        let reply = match cmd {
            "window_event_listen" => encode(events()?.listen(
                app,
                WINDOW_ORIGIN,
                text("event")?,
                argument(args, "target")?,
                argument(args, "handler")?,
            )?),
            "window_event_unlisten" => encode(events()?.unlisten(argument(args, "eventId")?)?),
            "reach_probe" => {
                encode(rt.block_on(crate::reach_probe(text("host")?, argument(args, "port")?))?)
            }
            "add_start" => encode(crate::add_start(text("token")?, argument(args, "ask")?)?),
            "add_read" => encode(crate::add_read(text("token")?)?),
            "promote_start" => encode(crate::promote_start(text("token")?, text("machine")?)?),
            "ssh_config_hosts" => encode(crate::ssh_config_hosts()),
            "open_copy_id" => encode(rt.block_on(crate::open_copy_id(argument(args, "ask")?))?),
            "pick_key_file" => encode(rt.block_on(crate::pick_key_file(app.clone()))),
            "open_settings" => encode(rt.block_on(crate::open_settings(
                app.clone(),
                argument(args, "pane")?,
                argument(args, "machine")?,
            ))?),
            "notify_finished" => encode(crate::notify_finished(
                app.clone(),
                text("title")?,
                text("body")?,
            )?),
            "window_appearance_support" => encode(crate::window_appearance_support()),
            "set_window_appearance" => encode(crate::set_window_appearance(
                app.clone(),
                text("material")?,
            )?),
            "set_window_theme" => {
                crate::set_window_theme(app.clone(), text("theme")?);
                encode(())
            }
            "pick_paths" => encode(crate::pick_desktop_paths(app, false)?),
            "term_pick_folders" => encode(crate::pick_desktop_paths(app, true)?),
            "open_code_window" => encode(rt.block_on(open_code_window(
                app.clone(),
                text("machine")?,
                text("url")?,
                text("title")?,
            ))?),
            "open_page" => encode(rt.block_on(open_page(
                main_window(app)?,
                tab()?,
                text("machine")?,
                text("url")?,
                argument(args, "at")?,
                text("agent")?,
            ))?),
            "move_page" => encode(move_page(main_window(app)?, tab()?, argument(args, "at")?)?),
            "hide_page" => encode(hide_page(app.clone(), tab()?)?),
            "close_page" => encode(close_page(app.clone(), tab()?)?),
            "show_page" => encode(show_page(app.clone(), tab()?)?),
            "page_state" => encode(page_state(tab()?)),
            "page_steps" => encode(rt.block_on(page_steps(app.clone(), tab()?))?),
            "page_step" => encode(rt.block_on(page_step(app.clone(), tab()?, text("way")?))?),
            "file_preview_supported" => encode(crate::file_preview_supported(text("kind")?)),
            "file_preview_open" => {
                encode(rt.block_on(crate::file_preview_open(webview()?, text("id")?))?)
            }
            "file_handoff_supported" => encode(crate::file_handoff_supported()),
            "file_handoff_open" => encode(rt.block_on(crate::file_handoff_open(
                app.clone(),
                webview()?,
                text("id")?,
            ))?),
            "file_handoff_save" => encode(rt.block_on(crate::file_handoff_save(
                app.clone(),
                webview()?,
                text("id")?,
            ))?),
            "file_export_begin" => encode(rt.block_on(crate::file_export_begin(
                webview()?,
                text("id")?,
                text("name")?,
            ))?),
            "file_export_append" => encode(rt.block_on(crate::file_export_append(
                webview()?,
                text("id")?,
                argument(args, "offset")?,
                text("data")?,
            ))?),
            "file_export_finish" => encode(rt.block_on(crate::file_export_finish(
                webview()?,
                text("id")?,
                argument(args, "total")?,
            ))?),
            "save_terminal_palette" => {
                encode(rt.block_on(crate::save_terminal_palette(app.clone(), text("text")?))?)
            }
            "app_update_status" => {
                encode(rt.block_on(updates::app_update_status(app.clone(), webview()?))?)
            }
            "app_update_check" => {
                encode(rt.block_on(updates::app_update_check(app.clone(), webview()?))?)
            }
            "app_update_download" => {
                encode(rt.block_on(updates::app_update_download(app.clone(), webview()?))?)
            }
            "app_update_install" => {
                encode(rt.block_on(updates::app_update_install(app.clone(), webview()?))?)
            }
            "app_update_runtime_cache" => encode(rt.block_on(
                updates::app_update_runtime_cache(webview()?, argument(args, "clean")?),
            )?),
            "app_update_preferences" => encode(rt.block_on(updates::app_update_preferences(
                app.clone(),
                webview()?,
                argument(args, "preferences")?,
            ))?),
            "app_update_companions" => encode(rt.block_on(updates::app_update_companions(
                app.clone(),
                webview()?,
                argument(args, "ids")?,
            ))?),
            "app_update_prepared" => encode(rt.block_on(updates::app_update_prepared(
                app.clone(),
                webview()?,
                text("nonce")?,
                argument(args, "blocked")?,
            ))?),
            "app_update_ui_ready" => {
                encode(rt.block_on(updates::app_update_ui_ready(app.clone(), webview()?))?)
            }
            "machine_update_status" => {
                encode(rt.block_on(updates::machine_update_status(webview()?, text("machine")?))?)
            }
            "machine_update_check" => encode(rt.block_on(updates::machine_update_check(
                webview()?,
                text("machine")?,
                optional_argument(args, "channel")?,
            ))?),
            "machine_update_queue" => {
                encode(rt.block_on(updates::machine_update_queue(app.clone(), webview()?))?)
            }
            "machine_update_start" => encode(rt.block_on(updates::machine_update_start(
                app.clone(),
                webview()?,
                argument(args, "machines")?,
                optional_argument(args, "channel")?,
            ))?),
            "machine_update_cancel" => encode(rt.block_on(updates::machine_update_cancel(
                app.clone(),
                webview()?,
                text("machine")?,
                text("request")?,
            ))?),
            _ => return Ok(None),
        }?;
        Ok(Some(reply))
    };
    answer().transpose()
}

fn argument<T: serde::de::DeserializeOwned>(
    args: &serde_json::Value,
    key: &str,
) -> Result<T, String> {
    let value = args.get(key).ok_or_else(|| format!("missing arg: {key}"))?;
    serde_json::from_value(value.clone()).map_err(|error| format!("invalid {key}: {error}"))
}

fn optional_argument<T: serde::de::DeserializeOwned>(
    args: &serde_json::Value,
    key: &str,
) -> Result<Option<T>, String> {
    args.get(key)
        .filter(|value| !value.is_null())
        .map(|value| {
            serde_json::from_value(value.clone()).map_err(|error| format!("invalid {key}: {error}"))
        })
        .transpose()
}

fn encode(value: impl serde::Serialize) -> Result<String, String> {
    serde_json::to_string(&value).map_err(|error| error.to_string())
}

/// The initialization script that restores window dragging.
///
/// Tauri ships its own (`tauri-2.11.5/src/window/scripts/drag.js`) and it is
/// correct — but it starts the drag with
/// `window.__TAURI_INTERNALS__.invoke('plugin:window|start_dragging')`, which is
/// the custom-protocol IPC that stalls on this platform. With the window
/// borderless there is nothing else to grab, so it has to be started over the
/// same loopback road the other commands take.
///
/// **Capture phase, and it stops propagation**: tauri's listener is on
/// `document` in the bubble phase, and letting both run would start a drag and
/// a double-click maximise at once. Stopping in the capture phase means only one
/// of them ever sees the event.
///
/// The region rules are tauri's own, restated: a clickable element blocks a
/// drag unless it carries the attribute itself, `="false"` disables, `="deep"`
/// makes the whole subtree draggable, and a bare/`"true"` value drags only when
/// the press landed on that very element.
fn initialization_script(address: &str, token: &str) -> String {
    let bridge = serde_json::json!({ "url": format!("http://{address}"), "token": token });
    let origin = serde_json::Value::String(WINDOW_ORIGIN.to_owned());
    format!(
        r#"(function () {{
  if (window !== window.top) return;
  if (window.location.origin !== {origin}) return;
  var bridge = Object.freeze({bridge});
  Object.defineProperty(window, '__KHOR_WINDOW_TRANSPORT__', {{ value: bridge }});
  var ATTR = 'data-tauri-drag-region';
  var CLICKABLE = new Set(['A','BUTTON','INPUT','SELECT','TEXTAREA','LABEL','SUMMARY']);
  var ROLES = new Set(['button','link','menuitem','tab','checkbox','radio','switch','option']);
  function clickable(el) {{
    return CLICKABLE.has(el.tagName)
      || (el.hasAttribute('contenteditable') && el.getAttribute('contenteditable') !== 'false')
      || (el.hasAttribute('tabindex') && el.getAttribute('tabindex') !== '-1')
      || ROLES.has(el.getAttribute('role'));
  }}
  function dragRegion(path) {{
    for (var i = 0; i < path.length; i++) {{
      var el = path[i];
      if (!el || el.nodeType !== 1) continue;
      var attr = el.getAttribute(ATTR);
      if (clickable(el) && attr === null) return false;
      if (attr === null) continue;
      if (attr === 'false') return false;
      if (attr === 'deep') return true;
      if (attr === '' || attr === 'true') return el === path[0];
    }}
    return false;
  }}
  function ask(verb) {{
    fetch(bridge.url + '/' + verb, {{
      method: 'POST', body: '{{}}',
      headers: {{ 'Content-Type': 'application/json', 'X-Khor-Window-Token': bridge.token }}
    }}).catch(() => {{}});
  }}
  document.addEventListener('mousedown', function (e) {{
    if (!e.isTrusted || e.button !== 0 || (e.detail !== 1 && e.detail !== 2)) return;
    var path = e.composedPath ? e.composedPath() : [];
    if (!dragRegion(path)) return;
    e.preventDefault();
    e.stopPropagation();
    ask(e.detail === 2 ? 'window_maximize_toggle' : 'window_drag');
  }}, true);
}})();"#
    )
}

/// Bind before creating the webview. The operating system selects an unused
/// port and the credential goes only into the main webview's startup script.
pub fn start(app: tauri::AppHandle) -> io::Result<String> {
    let mut entropy = [0_u8; 32];
    getrandom::fill(&mut entropy).map_err(|error| io::Error::other(error.to_string()))?;
    let token = entropy
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let server = Arc::new(
        tiny_http::Server::http(("127.0.0.1", 0))
            .map_err(|error| io::Error::other(error.to_string()))?,
    );
    let address = server
        .server_addr()
        .to_ip()
        .ok_or_else(|| io::Error::other("Windows webview listener has no TCP address"))?
        .to_string();
    let script = initialization_script(&address, &token);
    let address: Arc<str> = address.into();
    let token: Arc<str> = token.into();
    let mut worker_count = 0;
    for worker in 0..WORKERS {
        let server = Arc::clone(&server);
        let address = Arc::clone(&address);
        let token = Arc::clone(&token);
        let app = app.clone();
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        match std::thread::Builder::new()
            .name(format!("khor-window-{worker}"))
            .spawn(move || serve(server, &address, &token, &rt, &app))
        {
            Ok(_) => worker_count += 1,
            Err(error) => eprintln!("khor: a window-transport worker did not start: {error}"),
        }
    }
    if worker_count == 0 {
        return Err(io::Error::other(
            "Windows webview transport could not start",
        ));
    }
    Ok(script)
}

/// Return the complete response for preflight and rejected requests. Only
/// authenticated POSTs proceed to a command or read an attachment body.
fn admission(
    request: &tiny_http::Request,
    address: &str,
    token: &str,
) -> Option<tiny_http::Response<io::Empty>> {
    if header(request, "Origin") != Some(WINDOW_ORIGIN) || header(request, "Host") != Some(address)
    {
        return Some(tiny_http::Response::empty(403));
    }
    let method = request.method();
    let status = if method == &tiny_http::Method::Options {
        204
    } else if method != &tiny_http::Method::Post {
        405
    } else if header(request, TOKEN_HEADER)
        .is_some_and(|value| bool::from(value.as_bytes().ct_eq(token.as_bytes())))
    {
        return None;
    } else {
        403
    };
    Some(cors(tiny_http::Response::empty(status), WINDOW_ORIGIN))
}

fn serve(
    server: Arc<tiny_http::Server>,
    address: &str,
    token: &str,
    rt: &tokio::runtime::Runtime,
    app: &tauri::AppHandle,
) {
    // `recv` hands this thread the next request, or an error when the server is
    // gone; several workers may call it at once, which is the point.
    while let Ok(mut request) = server.recv() {
        if let Some(response) = admission(&request, address, token) {
            let _ = request.respond(response);
            continue;
        }
        let url = request.url().trim_start_matches('/').to_owned();
        let (cmd, query) = url.split_once('?').unwrap_or((url.as_str(), ""));
        let cmd = cmd.to_owned();
        let query = query.to_owned();
        // Named on stderr when `KHOR_GUI_TRACE` is set, so which commands the
        // page asks for — and which it never asks for — is answerable without
        // a devtools session (whose isolated world cannot see invoke callbacks).
        if std::env::var_os("KHOR_GUI_TRACE").is_some() {
            eprintln!("[transport] {cmd}");
        }
        // The window's own verbs first: they are not the store's to answer.
        if let Some(outcome) = window_verb(app, &cmd) {
            let _ = request.respond(json(outcome, WINDOW_ORIGIN));
            continue;
        }
        // The raw-body routes carry a file, which is not UTF-8 and must
        // never pass through a String. Each keeps the road the dev bridge
        // gives it (`bridge.rs`), so the two faces cannot drift.
        let raw = matches!(
            cmd.as_str(),
            "upload" | "chat_put" | "term_drop_stage" | "term_drop_stage_into"
        );
        if raw {
            let root = khor_node::Node::root_from_env();
            let outcome: Result<String, String> = if cmd == "term_drop_stage_into" {
                khor_gui_core::drop_stage_into_args(&query)
                    .ok_or_else(|| "term_drop_stage_into needs folder and path".to_owned())
                    .and_then(|(folder, path)| {
                        khor_gui_core::term::term_drop_stage_into(
                            &root,
                            &folder,
                            &path,
                            request.as_reader(),
                        )
                        .map(|id| serde_json::to_string(&id).unwrap_or_default())
                    })
            } else if cmd == "term_drop_stage" {
                khor_gui_core::drop_stage_args(&query)
                    .ok_or_else(|| "term_drop_stage needs a name query arg".to_owned())
                    .and_then(|name| {
                        khor_gui_core::term::term_drop_stage(&root, &name, request.as_reader())
                            .map(|id| serde_json::to_string(&id).unwrap_or_default())
                    })
            } else if cmd == "chat_put" {
                khor_gui_core::chat_put_args(&query)
                    .ok_or_else(|| "chat_put needs id, lease and name query args".to_owned())
                    .and_then(|(id, lease, name)| {
                        khor_gui_core::chat::chat_put(
                            &root,
                            &id,
                            &lease,
                            &name,
                            &khor_gui_core::into_arg(&query),
                            request.as_reader(),
                        )
                        .map(|paths| serde_json::to_string(&paths).unwrap_or_default())
                    })
            } else {
                khor_gui_core::upload_args(&query)
                    .ok_or_else(|| "upload needs a name query arg".to_owned())
                    .and_then(|name| {
                        khor_gui_core::stage_upload(&root, &name, request.as_reader())
                            .map(|id| serde_json::to_string(&id).unwrap_or_default())
                    })
            };
            let _ = request.respond(json(outcome, WINDOW_ORIGIN));
            continue;
        }
        let mut body = String::new();
        if let Err(error) = request.as_reader().read_to_string(&mut body) {
            let _ = request.respond(json(Err(error.to_string()), WINDOW_ORIGIN));
            continue;
        }
        let args: serde_json::Value = if body.trim().is_empty() {
            serde_json::Value::Null
        } else {
            match serde_json::from_str(&body) {
                Ok(value) => value,
                Err(error) => {
                    let _ = request.respond(json(Err(error.to_string()), WINDOW_ORIGIN));
                    continue;
                }
            }
        };
        let outcome =
            native_command(app, rt, &cmd, &args).unwrap_or_else(|| answer(rt, &cmd, &args));
        let _ = request.respond(json(outcome, WINDOW_ORIGIN));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::net::TcpStream;
    use std::time::Duration;

    fn exchange(
        method: &str,
        host: Option<&str>,
        origin: Option<&str>,
        token: Option<&str>,
    ) -> String {
        let server = tiny_http::Server::http(("127.0.0.1", 0)).unwrap();
        let address = server.server_addr().to_ip().unwrap();
        let worker = std::thread::spawn(move || {
            let request = server
                .recv_timeout(Duration::from_secs(3))
                .unwrap()
                .unwrap();
            if let Some(response) = admission(&request, &address.to_string(), "test-token") {
                request.respond(response).unwrap();
            } else {
                request
                    .respond(tiny_http::Response::from_string("admitted"))
                    .unwrap();
            }
        });
        let mut socket = TcpStream::connect(address).unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut head = format!(
            "{method} /sessions HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nContent-Length: 0\r\n",
            host.map(str::to_owned)
                .unwrap_or_else(|| address.to_string())
        );
        if let Some(origin) = origin {
            head.push_str(&format!("Origin: {origin}\r\n"));
        }
        if let Some(token) = token {
            head.push_str(&format!("{TOKEN_HEADER}: {token}\r\n"));
        }
        socket.write_all((head + "\r\n").as_bytes()).unwrap();
        let mut response = String::new();
        socket.read_to_string(&mut response).unwrap();
        worker.join().unwrap();
        response
    }

    #[test]
    fn window_transport_admits_only_the_window_and_its_credential() {
        let admitted = exchange("POST", None, Some(WINDOW_ORIGIN), Some("test-token"));
        assert!(admitted.starts_with("HTTP/1.1 200 "));
        assert!(admitted.ends_with("admitted"));
        for (host, origin, token) in [
            (None, Some(WINDOW_ORIGIN), None),
            (None, Some(WINDOW_ORIGIN), Some("wrong-token")),
            (None, None, Some("test-token")),
            (None, Some("https://example.com"), Some("test-token")),
            (Some("example.com"), Some(WINDOW_ORIGIN), Some("test-token")),
        ] {
            let rejected = exchange("POST", host, origin, token);
            assert!(rejected.starts_with("HTTP/1.1 403 "), "{rejected}");
        }
    }

    #[test]
    fn window_transport_answers_preflight_without_running_a_command() {
        let preflight = exchange("OPTIONS", None, Some(WINDOW_ORIGIN), None);
        assert!(preflight.starts_with("HTTP/1.1 204 "), "{preflight}");
        assert!(preflight.contains(&format!("Access-Control-Allow-Origin: {WINDOW_ORIGIN}")));
        let rejected = exchange("GET", None, Some(WINDOW_ORIGIN), Some("test-token"));
        assert!(rejected.starts_with("HTTP/1.1 405 "), "{rejected}");
    }
}
