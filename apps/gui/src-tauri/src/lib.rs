//! The tauri skin: every command is one call into khor-gui-core, which
//! is one call into khor-node — CLI and GUI stay equivalent because they
//! are the same functions (docs/KHOR.md 三条地基).

use khor_gui_core::{DeviceRow, FaceChoices, SessionRow};
use khor_node::Node;

/// `by` is the arrangement key (`khor_node::list::Arrange`). The window
/// passes the one the user last chose; it is that screen's own state,
/// not something the network holds an opinion about.
#[tauri::command]
fn sessions(by: String) -> Result<Vec<SessionRow>, String> {
    khor_gui_core::list_sessions(&Node::root_from_env(), &by)
}

/// The day strips, on their own beat rather than riding the row list
/// (`khor_gui_core::day_strips`). `cells` is how many the strip has room
/// for, measured by the screen.
#[tauri::command]
fn day_strips(
    cells: u32,
) -> Result<std::collections::BTreeMap<String, Vec<khor_gui_core::DayRun>>, String> {
    khor_gui_core::day_strips(&Node::root_from_env(), cells)
}

/// 常去的地方 for the wizard's first question
/// (`khor_gui_core::places`).
#[tauri::command]
async fn places(machine: Option<String>) -> Result<Vec<khor_gui_core::PlaceRow>, String> {
    khor_gui_core::places(&Node::root_from_env(), machine.as_deref().unwrap_or("")).await
}

#[tauri::command]
async fn quota_board() -> Result<Vec<khor_gui_core::AccountRing>, String> {
    khor_gui_core::quota_board(&Node::root_from_env()).await
}

#[tauri::command]
fn register_account(
    name: String,
    provider: String,
    kind: String,
    base_url: Option<String>,
    key: String,
    budget: Option<f64>,
) -> Result<String, String> {
    khor_gui_core::register_account(
        &Node::root_from_env(),
        &name,
        &provider,
        &kind,
        base_url.as_deref(),
        &key,
        budget,
    )
}

#[tauri::command]
fn remove_account(id: String) -> Result<(), String> {
    khor_gui_core::remove_account(&Node::root_from_env(), &id)
}

#[tauri::command]
async fn devices() -> Result<Vec<DeviceRow>, String> {
    khor_gui_core::list_devices(&Node::root_from_env()).await
}

/// The machine's process top and the CPU day its serve recorded.
///
/// **Asked while a card is open, never on the device beat** — the top
/// is a reading somebody is looking at, and `machine` empty means this
/// one (`Node::load_on`). `top` is the row count the card has room for:
/// required, the `cells` rule — a default here would be this side
/// sizing a table it cannot see.
#[tauri::command]
async fn load(
    machine: String,
    top: u32,
    by: Option<String>,
    desc: Option<bool>,
) -> Result<khor_node::Load, String> {
    // The same words the HTTP arm takes, and the same refusal for one
    // it does not know (`api.rs` carries the reasoning): a column read
    // as the default would look exactly like a working sort.
    let by = match by.as_deref() {
        None | Some("") | Some("busiest") => khor_node::TopBy::Busiest,
        Some("pid") => khor_node::TopBy::Pid,
        Some("name") => khor_node::TopBy::Name,
        Some("user") => khor_node::TopBy::User,
        Some("cpu") => khor_node::TopBy::Cpu,
        Some("mem") => khor_node::TopBy::Mem,
        Some("gpu") => khor_node::TopBy::Gpu,
        Some("vram") => khor_node::TopBy::Vram,
        Some("time") => khor_node::TopBy::Time,
        Some(other) => return Err(format!("no such column: {other}")),
    };
    khor_gui_core::load(&Node::root_from_env(), &machine, top, by, desc.unwrap_or(true)).await
}

/// **Blocking, and the first call can take seconds**: reading every
/// transcript on this machine is 18 s cold (`khor_node::usage`). Tauri
/// runs commands on a worker thread rather than on the UI thread, so
/// this does not freeze the window — and every call after the first is
/// a walk of directory entries.
#[tauri::command]
fn usage() -> Result<khor_node::Usage, String> {
    khor_gui_core::usage(&Node::root_from_env())
}

/// The subscription's remaining windows. Answers a shape carrying its own
/// trouble rather than a `Result`, because "khor could not ask" is one of
/// the things this reading has to be able to say out loud — five of them,
/// each with a different next step (`khor_node::quota`), and a `Result`
/// would flatten them into one error string on the way to the face.
#[tauri::command]
async fn quota() -> khor_gui_core::QuotaAnswer {
    khor_gui_core::quota().await
}

#[tauri::command]
fn seen(id: String) -> Result<(), String> {
    khor_gui_core::seen(&Node::root_from_env(), &id)
}

#[tauri::command]
fn close_session(id: String) -> Result<(), String> {
    khor_gui_core::close_session(&Node::root_from_env(), &id)
}

#[tauri::command]
fn rename_session(id: String, title: String) -> Result<(), String> {
    khor_gui_core::rename_session(&Node::root_from_env(), &id, &title)
}

#[tauri::command]
fn tell(machine: String, text: String) -> Result<(), String> {
    khor_gui_core::tell(&Node::root_from_env(), &machine, &text)
}

#[tauri::command]
fn pin_session(id: String, on: bool) -> Result<(), String> {
    khor_gui_core::pin_session(&Node::root_from_env(), &id, on)
}

#[tauri::command]
fn pin_device(machine: String, on: bool) -> Result<(), String> {
    khor_gui_core::pin_device(&Node::root_from_env(), &machine, on)
}

#[tauri::command]
fn forget_device(machine: String) -> Result<String, String> {
    khor_gui_core::forget_device(&Node::root_from_env(), &machine)
}

#[tauri::command]
fn set_watch(machine: String, paths: Vec<String>) -> Result<Vec<String>, String> {
    khor_gui_core::set_watch(&Node::root_from_env(), &machine, &paths)
}

#[tauri::command]
fn face_choices() -> Result<FaceChoices, String> {
    khor_gui_core::face_choices(&Node::root_from_env())
}

/// An axis left out stays where it is — the same shape as `khor face`'s
/// flags, because they are the same call underneath.
#[tauri::command]
fn restyle(
    colors: Option<Vec<String>>,
    variant: Option<String>,
    shape: Option<String>,
) -> Result<(), String> {
    khor_gui_core::restyle(&Node::root_from_env(), colors, variant, shape)
}

/// The three hook commands are about **this machine's** claude: the
/// settings file they read and write is rooted at this node's vendor
/// home, so there is no argument naming a machine and no way to point
/// one of these at somebody else's.
#[tauri::command]
fn hooks_state() -> Result<khor_gui_core::HooksState, String> {
    khor_gui_core::hooks_state(&Node::root_from_env())
}

#[tauri::command]
fn install_hooks() -> Result<khor_gui_core::HooksState, String> {
    khor_gui_core::install_hooks(&Node::root_from_env())
}

#[tauri::command]
fn uninstall_hooks() -> Result<khor_gui_core::HooksState, String> {
    khor_gui_core::uninstall_hooks(&Node::root_from_env())
}

/// The six chat commands are one attachment registry in gui-core: the
/// window polls for frames because that is the shape both skins can
/// serve (`khor_gui_core::chat` module head).
#[tauri::command]
fn chat_open(id: String) -> Result<bool, String> {
    khor_gui_core::chat::chat_open(&Node::root_from_env(), &id)
}

#[tauri::command]
fn chat_poll(id: String, since: u64) -> Result<khor_gui_core::chat::ChatBatch, String> {
    khor_gui_core::chat::chat_poll(&id, since)
}

#[tauri::command]
fn chat_say(id: String, text: String) -> Result<(), String> {
    khor_gui_core::chat::chat_say(&id, &text)
}

#[tauri::command]
fn chat_answer(id: String, ask: u64, option: Option<String>) -> Result<(), String> {
    khor_gui_core::chat::chat_answer(&id, ask, option)
}

#[tauri::command]
fn open_link(url: String) -> Result<(), String> {
    khor_gui_core::web::open_link(&url)
}

#[tauri::command]
fn chat_stop(id: String) -> Result<(), String> {
    khor_gui_core::chat::chat_stop(&id)
}
#[tauri::command]
fn chat_set_mode(id: String, mode: String) -> Result<(), String> {
    khor_gui_core::chat::chat_set_mode(&id, &mode)
}

#[tauri::command]
fn chat_set_option(id: String, option: String, value: serde_json::Value) -> Result<(), String> {
    khor_gui_core::chat::chat_set_option(&id, &option, &value)
}


#[tauri::command]
fn chat_replay(id: String) -> Result<(), String> {
    khor_gui_core::chat::chat_replay(&id)
}

#[tauri::command]
fn chat_leave(id: String) -> Result<(), String> {
    khor_gui_core::chat::chat_leave(&id)
}

/// A discovered session's recorded past — the vendor's transcript in
/// replay-shaped frames (`khor_gui_core::chat::history`).
#[tauri::command]
fn history(id: String) -> Result<Vec<khor_gui_core::chat::ChatFrame>, String> {
    khor_gui_core::chat::history(&Node::root_from_env(), &id)
}

/// A discovered conversation, **followed**: whatever its transcript has
/// said since `from`, and where to continue. The read-only half of
/// watching a session that is running in its own terminal.
#[tauri::command]
fn history_follow(id: String, from: u64) -> Result<khor_gui_core::chat::Followed, String> {
    khor_gui_core::chat::history_follow(&Node::root_from_env(), &id, from)
}

/// One machine's directory, for the files landing. Async because the
/// machine may be far away (the same reason `pair` is).
#[tauri::command]
async fn ls(machine: String, path: String) -> Result<khor_gui_core::files::DirListing, String> {
    khor_gui_core::files::ls(&Node::root_from_env(), &machine, &path).await
}

/// Takes a file into this machine's downloads; answers where it landed.
#[tauri::command]
async fn pull(machine: String, path: String) -> Result<String, String> {
    khor_gui_core::files::pull(&Node::root_from_env(), &machine, &path).await
}

#[tauri::command]
fn dir_pins() -> Result<Vec<khor_gui_core::files::DirPinRow>, String> {
    khor_gui_core::files::dir_pins(&Node::root_from_env())
}

#[tauri::command]
fn pin_dir(machine: String, path: String, on: bool) -> Result<(), String> {
    khor_gui_core::files::pin_dir(&Node::root_from_env(), &machine, &path, on)
}

/// The seven terminal commands are one attachment registry in gui-core,
/// mirroring chat: the window replays the feed and tails it, because
/// a byte stream is the shape both skins can serve
/// (`khor_gui_core::term` module head — xterm.js is the renderer).
#[tauri::command]
fn term_open(id: String, cols: u16, rows: u16) -> Result<(), String> {
    khor_gui_core::term::term_open(&Node::root_from_env(), &id, cols, rows)
}

#[tauri::command]
fn term_attach(id: String) -> Result<khor_gui_core::term::TermStart, String> {
    khor_gui_core::term::term_attach(&id)
}

#[tauri::command]
fn term_stream(id: String, since: u64) -> Result<khor_gui_core::term::TermChunk, String> {
    khor_gui_core::term::term_stream(&id, since)
}

#[tauri::command]
fn term_key(id: String, keys: String) -> Result<(), String> {
    khor_gui_core::term::term_key(&id, keys.into_bytes())
}

/// Files dropped on a terminal: their paths, shell-quoted, pasted in
/// (`term::term_drop`). The quoting is the registry's because it is
/// about the shell's syntax, and one wrong escape is a command.
///
/// The root rides along because the terminal may not be on this
/// machine: a drop onto a far session sends the files there first and
/// pastes where they landed, and finding that out is a node's job.
/// The command's own arguments are unchanged, so nothing above this
/// line — the frontend, the bridge's table — learns there is a network
/// under it.
#[tauri::command]
fn term_drop(id: String, paths: Vec<String>) -> Result<(), String> {
    khor_gui_core::term::term_drop(&Node::root_from_env(), &id, &paths)
}

#[tauri::command]
fn term_resize(id: String, cols: u16, rows: u16) -> Result<(), String> {
    khor_gui_core::term::term_resize(&id, cols, rows)
}

#[tauri::command]
fn term_leave(id: String) -> Result<(), String> {
    khor_gui_core::term::term_leave(&id)
}

/// Where this person has been, of one kind, most recent first
/// (`khor_gui_core::recents`). The cap is the caller's because it is a
/// question about the surface asking — a strip of tabs and a landing's
/// list want different lengths of the same list.
#[tauri::command]
fn recents(kind: String, limit: u32) -> Result<Vec<khor_gui_core::recents::RecentRow>, String> {
    khor_gui_core::recents::recents(&Node::root_from_env(), &kind, limit as usize)
}

#[tauri::command]
fn recent_visit(kind: String, machine: String, place: String) -> Result<(), String> {
    khor_gui_core::recents::recent_visit(&Node::root_from_env(), &kind, &machine, &place)
}

#[tauri::command]
fn forget_recent(kind: String, machine: String, place: String) -> Result<(), String> {
    khor_gui_core::recents::forget_recent(&Node::root_from_env(), &kind, &machine, &place)
}

#[tauri::command]
fn web_pins() -> Result<Vec<khor_gui_core::web::WebPinRow>, String> {
    khor_gui_core::web::web_pins(&Node::root_from_env())
}

#[tauri::command]
fn pin_web(machine: String, url: String, on: bool) -> Result<(), String> {
    khor_gui_core::web::pin_web(&Node::root_from_env(), &machine, &url, on)
}

/// Gives a page's borrow back.
///
/// **墓碑: this used to close a window too** (2026-09-09). A borrowed page
/// opened in a window of its own until the tab body could hold one, and
/// `open_web` built that window with the borrow's proxy on it. The whole
/// road is gone — the machine card's 借网 opens a browsing tab, and the
/// tab's page is a child webview (`open_page`) — so what is left is the
/// half that was never about windows: handing the network back.
#[tauri::command]
async fn close_web(session: String) -> Result<(), String> {
    khor_gui_core::web::drop_web_borrow(&Node::root_from_env(), &session).await
}

// ── the page inside a browsing tab (docs/NET.md 借网, 单元 2) ──
//
// **A page cannot be drawn by the face that draws everything else.** khor's
// whole interface is one webview rendering a React app; an arbitrary site
// cannot be put inside that page (`iframe` is refused by most sites, reads
// back neither title nor address, and the proxy can only be set when a
// webview is built). So a browsing tab's page is a **second webview**, a
// child of the same window, positioned over the rectangle the tab body
// occupies. The frontend owns the rectangle because only it knows the
// layout; this side owns the webview because only it can make one.
//
// **One child per tab, hidden rather than closed on a tab switch.** That is
// what makes it a tab rather than a bookmark: a page keeps its scroll, its
// half-filled form and its live connection while you look at another one.
//
// The upstream bug that used to rule this out — a window showing only its
// last child — was fixed in tauri 2024-11 (#11616, `build_as_child`), and
// the fix is in the version this builds against. The issue is still open
// upstream; it was never triaged.

/// Where a tab's page sits inside the window, in the frontend's own CSS
/// pixels — which are tauri's logical pixels, so nothing here does DPI
/// arithmetic and nothing can disagree about the scale factor.
#[derive(Debug, Clone, Copy, serde::Deserialize)]
struct PageBox {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    /// The face's own viewport height, sent with every rectangle.
    ///
    /// **It is here to close a coordinate mismatch, measured rather than
    /// assumed** (2026-09-09, macOS). A child webview's y is flipped by
    /// wry against the height tauri reports for the window — and on macOS
    /// tauri reports the **frame**, titlebar included: `inner_size` and
    /// `outer_size` both answered 1080x720 for a window whose content
    /// area is 688 tall. The flip therefore lands the page one titlebar
    /// too high: the face asked for y=93 and the page painted at 61,
    /// covering the address bar it was supposed to sit under.
    ///
    /// The face is the one side that knows the true content height, so it
    /// says so and this side takes the difference. **Not a 28 or a 32
    /// written down here**: that number is the titlebar of one window
    /// style on one platform, and it is zero on the platforms whose two
    /// sizes agree — a constant would be right today and wrong on the
    /// first window that is not this one.
    view: f64,
}

/// How much lower the page must sit than the face asked, because the two
/// sides measure the window differently. Zero wherever they agree.
#[cfg(desktop)]
fn frame_lift(window: &tauri::Window, at: PageBox) -> f64 {
    let Ok(inner) = window.inner_size() else { return 0.0 };
    let scale = window.scale_factor().unwrap_or(1.0);
    if scale <= 0.0 || at.view <= 0.0 {
        return 0.0;
    }
    let told = inner.height as f64 / scale;
    // Never negative: a face that measured a viewport *taller* than the
    // window is a face mid-resize, and pulling the page up on that would
    // be a jump the next rectangle undoes.
    (told - at.view).max(0.0)
}

/// One thing a page told us about itself, addressed to the tab it belongs
/// to. The tab id rides along because the face has several and the event
/// arrives with no other way to tell them apart.
#[derive(Debug, Clone, serde::Serialize)]
struct PageWord {
    tab: String,
    text: String,
}

/// Which exit machine each live page webview was built against, by label.
///
/// **This exists to make one specific failure impossible.** A webview's
/// proxy is fixed when it is built and there is no runtime setter (checked
/// against wry: `with_proxy_config` on the builder is the only door). So a
/// page webview reused for a different exit machine would keep leaving
/// through the old one — **not failing, succeeding through the wrong
/// network**, which is the shape docs/NET.md 可达性 rules out. Knowing what
/// each one was built against lets `open_page` rebuild instead of reuse.
fn page_exits() -> &'static std::sync::Mutex<std::collections::HashMap<String, String>> {
    static EXITS: std::sync::OnceLock<
        std::sync::Mutex<std::collections::HashMap<String, String>>,
    > = std::sync::OnceLock::new();
    EXITS.get_or_init(Default::default)
}

/// How a browsing tab presents itself to the sites it opens.
///
/// **A browser has to look like one, and this is not vanity.** A webview
/// inside an app identifies itself as bare WebKit: the same string Safari
/// sends, minus the `Version/… Safari/…` tail — and sites read that tail.
///
/// Measured 2026-09-09 against baidu through a borrow. With the tail it
/// answers with its real homepage, 713 KB. Without it, 227 bytes whose
/// whole body is
/// `location.replace(location.href.replace("https://","http://"))` — a
/// deliberate downgrade of clients it does not recognise. HSTS puts that
/// straight back on https, which serves the same 227 bytes, and the tab
/// spins: **2179 navigations in one sitting, white the whole time**,
/// because that page has nothing in it. From the outside it reads as「打
/// 不开」, and khor is not doing any of it — `open_page` was called 15
/// times while the page navigated 1254.
///
/// **The build number is taken out of the string, never written down.**
/// What Safari appends is `Safari/<the same build as AppleWebKit/…>`, and
/// that build is already in what the face reports — so this cannot go
/// stale the way a constant would, and it stays right on a machine whose
/// WebKit is not this one's. A face whose agent carries no `AppleWebKit/`
/// gets no override at all: inventing an identity for a client we cannot
/// read is worse than presenting the honest one, and on Linux and Windows
/// the honest one already names a browser.
fn as_a_browser(agent: &str) -> Option<String> {
    if agent.is_empty() || agent.contains("Safari/") {
        return None;
    }
    let build = agent.split("AppleWebKit/").nth(1)?.split_whitespace().next()?;
    Some(format!("{agent} Safari/{build}"))
}

/// A tab id as a webview label. Tab ids are minted from a timestamp and a
/// counter so they are already tame; this is the same guard the borrow
/// window label uses, for the same reason — a label is a key, and a key
/// with a separator in it is a key that can collide.
fn page_label(tab: &str) -> String {
    format!("page-{}", tab.replace(['/', ':'], "-"))
}

/// Opens (or re-points) the page a browsing tab is showing.
///
/// Answers with the borrow, because the face is what closes with
/// it. Async because the borrow dials.
#[cfg(desktop)]
#[tauri::command]
async fn open_page(
    window: tauri::Window,
    tab: String,
    machine: String,
    url: String,
    at: PageBox,
    agent: String,
) -> Result<khor_gui_core::web::WebBorrow, String> {
    use tauri::{Emitter, LogicalPosition, LogicalSize, Manager, WebviewUrl};
    let borrow = khor_gui_core::web::borrow_web(&Node::root_from_env(), &machine).await?;
    let target: tauri::Url = url.parse().map_err(|_| format!("not a url: {url}"))?;
    let proxy: tauri::Url =
        format!("http://{}", borrow.addr).parse().map_err(|e| format!("{e}"))?;
    let label = page_label(&tab);
    let top = at.y + frame_lift(&window, at);

    // Reuse only when it is the same exit. A different one means the
    // webview in front of us is wired to a proxy it can no longer be
    // talked out of, so it goes.
    let same_exit = page_exits().lock().map(|m| m.get(&label) == Some(&machine)).unwrap_or(false);
    if let Some(live) = window.app_handle().get_webview(&label) {
        if same_exit {
            // **Only navigate when it is somewhere else.** Coming back to
            // a tab mounts its body afresh and asks again for the address
            // it is already on; navigating to where you already are is a
            // reload, and a reload on every tab switch is exactly the
            // thing keeping a webview per tab exists to avoid.
            if live.url().map(|now| now != target).unwrap_or(true) {
                live.navigate(target).map_err(|e| e.to_string())?;
            }
            live.set_position(LogicalPosition::new(at.x, top)).map_err(|e| e.to_string())?;
            live.set_size(LogicalSize::new(at.w, at.h)).map_err(|e| e.to_string())?;
            live.show().map_err(|e| e.to_string())?;
            return Ok(borrow);
        }
        let _ = live.close();
    }

    // Every event goes to **this** window and no further. A page webview
    // is somebody else's site; broadcasting khor's own events into it
    // would be handing a stranger the app's internal traffic.
    let face = window.label().to_string();
    let (t_tab, l_tab) = (tab.clone(), tab.clone());
    let (t_face, l_face) = (face.clone(), face.clone());
    let mut builder = tauri::webview::WebviewBuilder::new(&label, WebviewUrl::External(target))
        .proxy_url(proxy);
    if let Some(seem) = as_a_browser(&agent) {
        builder = builder.user_agent(&seem);
    }
    let builder = builder
        .on_document_title_changed(move |view, title| {
            let _ = view
                .app_handle()
                .emit_to(t_face.clone(), "page:title", PageWord { tab: t_tab.clone(), text: title });
        })
        .on_page_load(move |view, payload| {
            let word = match payload.event() {
                tauri::webview::PageLoadEvent::Started => "started",
                tauri::webview::PageLoadEvent::Finished => "finished",
            };
            let app = view.app_handle();
            let _ = app.emit_to(
                l_face.clone(),
                "page:load",
                PageWord { tab: l_tab.clone(), text: word.to_owned() },
            );
            // **On arrival, not on departure.** This used to fire for
            // both halves of a load, so every navigation reported the
            // tab's place twice — and the first of the two is a place the
            // tab has not reached yet. A navigation that starts and never
            // finishes would have left the address bar naming somewhere
            // the tab is not, which is this repo's oldest failure shape:
            // a step that cannot be taken does not become "no answer", it
            // becomes the answer next to it.
            //
            // It is also not free. A page in a redirect loop reported
            // 2484 times in one sitting (baidu, 2026-09-09, before
            // `as_a_browser`), and each report is a render of khor's own
            // face.
            //
            // **Where the tab is, taken from what actually loaded here.**
            // This used to be reported from `on_navigation`, which fires
            // for a `target="_blank"` link too — and that link's page
            // never loads in *this* webview, it becomes a tab of its own.
            // So one click moved both: the new tab opened, and the tab
            // that was left behind followed it. Measured 2026-09-09.
            //
            // ⚠️ **A page that moves itself with `pushState` does not
            // load anything**, so it does not reach here and the address
            // stays where it was. Real browsers follow that too; the hook
            // for it is a script in the page, not this callback.
            if matches!(payload.event(), tauri::webview::PageLoadEvent::Finished) {
                let _ = app.emit_to(
                    l_face.clone(),
                    "page:went",
                    PageWord { tab: l_tab.clone(), text: payload.url().to_string() },
                );
            }
        });
    // **Kept, and deliberately silent.** `on_navigation` answers whether
    // a navigation may happen at all, which is where a scheme allowlist
    // would live — `file:` and whatever else a borrowed page must not be
    // able to reach through this window. It used to also report the
    // address, and that was wrong: it fires for a `target="_blank"` link
    // whose page never loads here (see `on_page_load`).
    let builder = builder.on_navigation(|_to| true);
    // **A link that asks for a new window gets a new tab**, which is the
    // only answer that leaves the decision where it belongs. Whether a
    // link opens where you are or beside it is the page author's call —
    // `target="_blank"` on one link and not another is a distinction they
    // made on purpose — and khor's job is to carry it, not to overrule it
    // in either direction.
    //
    // Without this the default answer is to deny, and **denying is
    // silent**: measured on 2026-09-09, baidu's results are such links
    // and pressing one looked like a page that had stopped responding.
    // The first fix here navigated in place, which worked and was still
    // khor deciding — corrected the same evening.
    //
    // `Deny` and an event, rather than `Create`: the new tab is the
    // *face's* to mint, because the tab strip, the store and the borrow
    // ledger are all on that side and a webview created behind their back
    // would belong to no tab at all.
    let popped = window.app_handle().clone();
    let (p_tab, p_face) = (tab.clone(), face.clone());
    let builder = builder.on_new_window(move |to, _features| {
        let _ = popped.emit_to(
            p_face.clone(),
            "page:popup",
            PageWord { tab: p_tab.clone(), text: to.to_string() },
        );
        tauri::webview::NewWindowResponse::Deny
    });

    window
        .add_child(builder, LogicalPosition::new(at.x, top), LogicalSize::new(at.w, at.h))
        .map_err(|e| e.to_string())?;
    if let Ok(mut m) = page_exits().lock() {
        m.insert(label, machine);
    }
    Ok(borrow)
}

/// Keeps a page over its tab body as the layout moves under it. Called by
/// the side that can see the layout; there is no way for this side to know
/// a rectangle changed.
#[cfg(desktop)]
#[tauri::command]
fn move_page(window: tauri::Window, tab: String, at: PageBox) -> Result<(), String> {
    use tauri::{LogicalPosition, LogicalSize, Manager};
    let Some(view) = window.app_handle().get_webview(&page_label(&tab)) else { return Ok(()) };
    let top = at.y + frame_lift(&window, at);
    view.set_position(LogicalPosition::new(at.x, top)).map_err(|e| e.to_string())?;
    view.set_size(LogicalSize::new(at.w, at.h)).map_err(|e| e.to_string())?;
    Ok(())
}

/// Takes a page off the screen without ending it.
///
/// **Two jobs, one call.** A tab switch hides the page you are leaving, and
/// so does opening anything that has to float over it: a native child
/// webview always paints above the DOM, so khor's own menus and the address
/// bar's completions would otherwise be drawn underneath the site.
#[cfg(desktop)]
#[tauri::command]
fn hide_page(app: tauri::AppHandle, tab: String) -> Result<(), String> {
    use tauri::Manager;
    let Some(view) = app.get_webview(&page_label(&tab)) else { return Ok(()) };
    view.hide().map_err(|e| e.to_string())
}

/// Puts a page back after `hide_page` took it away.
///
/// **Separate from `open_page` on purpose.** Coming back from a menu is
/// not opening anything: it must not dial a borrow, must not navigate,
/// and must not cost the page a frame of reload. A page that is not there
/// is not an error here — the tab may have been closed while a menu was
/// up, and nothing about that is worth a refusal.
#[cfg(desktop)]
#[tauri::command]
fn show_page(app: tauri::AppHandle, tab: String) -> Result<(), String> {
    use tauri::Manager;
    let Some(view) = app.get_webview(&page_label(&tab)) else { return Ok(()) };
    view.show().map_err(|e| e.to_string())
}

/// Back, forward and reload for one tab's page.
///
/// **Two of the three are script, and that is a gap rather than a
/// choice.** tauri has `reload` and nothing else: there is no `go_back`,
/// no `go_forward` and — the one that shows — no `can_go_back`, so the
/// arrows cannot grey out when there is nowhere to go. Pressing one at
/// the start of a history does nothing and says nothing, which is the
/// honest state of it until the platform grows the question.
#[cfg(desktop)]
#[tauri::command]
fn page_step(app: tauri::AppHandle, tab: String, way: String) -> Result<(), String> {
    use tauri::Manager;
    let Some(view) = app.get_webview(&page_label(&tab)) else { return Ok(()) };
    match way.as_str() {
        "back" => view.eval("history.back()").map_err(|e| e.to_string()),
        "forward" => view.eval("history.forward()").map_err(|e| e.to_string()),
        "reload" => view.reload().map_err(|e| e.to_string()),
        // Refused by name, the `load` rule: a word this build does not
        // know, read as one of the three, would look like a working
        // button doing somebody else's job.
        other => Err(format!("no such step: {other}")),
    }
}

/// Ends a page: the webview goes and the tab stops having one.
///
/// **Not the borrow.** One exit machine is one borrow however many tabs
/// point at it, so the last tab to stop naming a machine is what gives its
/// network back (`views/borrows`), and that is a count only the face keeps.
#[cfg(desktop)]
#[tauri::command]
fn close_page(app: tauri::AppHandle, tab: String) -> Result<(), String> {
    use tauri::Manager;
    let label = page_label(&tab);
    if let Ok(mut m) = page_exits().lock() {
        m.remove(&label);
    }
    let Some(view) = app.get_webview(&label) else { return Ok(()) };
    view.close().map_err(|e| e.to_string())
}

// ── khor's own overlays, in a window of their own (单元 2, 浮层) ──
//
// **A native child webview always paints above the DOM.** That is not a
// z-index anybody can raise; it is another layer of the window. So the
// address bar's completions — drawn in khor's own page, one row under the
// address they complete — came out *underneath* the site the tab is
// showing. Two answers were tried in front of somebody and both were
// judged wrong: taking the page away while the menu is up blanked the
// whole site, and pushing the page's top down shoved the site out from
// under the eyes of the person reading it. 「不对,应该是浮在页面上的」.
//
// **What browsers do is a second window**, and it is the only answer that
// makes the menu genuinely float: an operating-system window sits above
// another window's child views by construction. It carries the same React
// list, rendered by the same component (`components/OmniMenu`), so there
// is one menu and not two that drift apart.
//
// Three properties are what make it read as a menu rather than a window,
// and each is doing a job:
//
// - **`parent`** — the platform then keeps it above the face, moves it
//   with the face and destroys it with the face. Without a parent the app
//   would stop exiting when its window is closed (a hidden window is
//   still a window), and the alternative — `always_on_top` — would have
//   floated khor's completions over other applications.
// - **`focusable(false)`** — the caret has to stay in the address box.
//   This menu is driven from that box's keyboard (↑↓/Tab/Enter all belong
//   to the field), and a window that took focus would blur the field
//   mid-completion. On macOS this is `canBecomeKeyWindow` answering no
//   (tao 0.35, `platform_impl/macos/window.rs`).
// - **`accept_first_mouse(true)`** — the other half of that one. Every
//   click into a window that cannot become key is a "first mouse" click,
//   and a view that does not accept one swallows it. Without this the
//   rows would be visible and unpressable.

/// Where an overlay has to appear, in the face's own CSS pixels, with the
/// two viewport measurements that turn them into screen ones.
#[derive(Debug, Clone, Copy, serde::Deserialize)]
struct Spot {
    x: f64,
    y: f64,
    w: f64,
    /// The face's viewport height and width. **Both are here to be
    /// subtracted from the window frame, and neither is decoration** —
    /// see `on_screen`, which is where the arithmetic and the reason for
    /// it live. `PageBox.view` carries the same measurement for the same
    /// kind of reason.
    view: f64,
    across: f64,
}

/// The window khor's own overlays are drawn in. One, reused: building a
/// webview costs a page load, and a menu that had to load a page every
/// time somebody pressed the address bar would arrive after they had
/// finished typing.
#[cfg(desktop)]
const MENU: &str = "menu";

/// Where a `Spot` lands on the screen, in physical pixels, and how wide.
///
/// **Both offsets are measured rather than written down.** The face
/// reports the size of its own viewport; this side asks the platform for
/// the size of the whole window frame; the difference is the chrome,
/// whatever the platform decided that is — a titlebar on macOS, a
/// titlebar and three borders on Windows, nothing at all where the window
/// is undecorated. A constant here would be right on the machine it was
/// measured on and wrong on the first window that is not that one, which
/// is the mistake `PageBox.view` exists to have already made once.
///
/// The side borders are what let the top be worked out: chrome is
/// left+right+bottom+top, the sides are half of what the width is short
/// by, and the bottom border is the same as a side one on every window
/// style this runs in. What is left over is the titlebar.
#[cfg(desktop)]
fn on_screen(window: &tauri::Window, at: Spot) -> Result<(f64, f64, f64), String> {
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    let frame = window.outer_size().map_err(|e| e.to_string())?;
    let origin = window.outer_position().map_err(|e| e.to_string())?;
    if scale <= 0.0 || at.view <= 0.0 || at.across <= 0.0 {
        return Err("the face has not measured itself yet".into());
    }
    // Never negative: a face that measured a viewport bigger than the
    // window is a face caught mid-resize, and a menu placed on that would
    // jump to somewhere the next rectangle undoes.
    let side = ((frame.width as f64 - at.across * scale) / 2.0).max(0.0);
    let down = ((frame.height as f64 - at.view * scale) - side).max(0.0);
    Ok((
        origin.x as f64 + side + at.x * scale,
        origin.y as f64 + down + at.y * scale,
        at.w * scale,
    ))
}

/// The rows the menu is showing and the face that asked for them.
///
/// **The face's label rides along because a pick has to go home.** The
/// menu window is one window serving whichever face opened it, and an
/// event broadcast to every webview would hand khor's traffic to the site
/// in the browsing tab (`open_page` says why that matters).
#[cfg(desktop)]
fn menu_held() -> &'static std::sync::Mutex<Option<(String, serde_json::Value)>> {
    static HELD: std::sync::OnceLock<
        std::sync::Mutex<Option<(String, serde_json::Value)>>,
    > = std::sync::OnceLock::new();
    HELD.get_or_init(Default::default)
}

/// Whether the menu is meant to be on screen right now.
///
/// **It exists because showing is two steps and hiding is one.** The
/// window is shown by `size_menu`, which arrives after the menu window has
/// laid its list out — and a `hide_menu` can land in that gap. Without
/// this, a menu dismissed while its last render was still in flight would
/// reappear by itself.
/// How wide the box that owns the menu is, in physical pixels.
///
/// **Kept here because the window cannot be asked.** `set_size` goes
/// through the event loop, so a `size_menu` arriving hard on the heels of
/// a `show_menu` reads back the width the window had *before* the show —
/// and writing that back with the new height pinned the menu at one pixel
/// wide. Measured 2026-09-09: the window server listed it 1x151, in front
/// and correctly placed, and invisible for all practical purposes.
#[cfg(desktop)]
fn menu_wide() -> &'static std::sync::atomic::AtomicU32 {
    static WIDE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);
    &WIDE
}

#[cfg(desktop)]
fn menu_up() -> &'static std::sync::atomic::AtomicBool {
    static UP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    &UP
}

/// Builds the overlay window, once, while the app is starting.
///
/// **At startup rather than on the first press, and the reason is that a
/// failure here has to be visible.** Built lazily, a window that would not
/// build turned into a refused command, and a refused command turned into
/// a face quietly drawing its menu the old way — the same silence that hid
/// a missing capabilities file for a day (`onPageWord`). Here it says so
/// out loud, and everything downstream reads "no window" as "draw it in
/// the page", which is a state rather than an error.
///
/// It also buys the thing lazy building cost: the first press of the
/// address bar no longer waits for a page load.
#[cfg(desktop)]
fn hang_menu_window(app: &tauri::AppHandle) {
    // **Not a child window, and that was measured rather than chosen.**
    // The first shape of this hung the overlay off the face with
    // `parent`, which reads better — the platform then keeps it above the
    // face, moves it along and destroys it with the face. Two things
    // broke:
    //
    //   - `orderOut:` **takes a child off its parent** on macOS, so the
    //     first hide detached it and every show after that put it behind
    //     the face. From the outside that is 「只有第一次点会弹出」.
    //   - `addChildWindow:` binds the child's window level to the
    //     parent's, so `always_on_top` could not fix it: the window
    //     server listed both at layer 0 with the menu second.
    //
    // A level of its own is the shape that survives being hidden. What it
    // costs is the two things the parent was giving: the app has to close
    // this window itself (see `run`, or a hidden window keeps it alive
    // after its face is gone), and the menu no longer follows a window
    // being dragged — it is repositioned on every show, so what shows is
    // dragging the window *while a completion list is up*.
    let built = tauri::WebviewWindowBuilder::new(
        app,
        MENU,
        // **The same page as the face, told which half of itself to be.**
        // A second HTML entry point would be a second place for the
        // menu's look to be defined, and the two would drift; the query
        // is read once at boot (`main.tsx`) and picks a root.
        tauri::WebviewUrl::App("index.html?face=menu".into()),
    )
    .title("")
    .decorations(false)
    .resizable(false)
    .skip_taskbar(true)
    .focusable(false)
    .accept_first_mouse(true)
    .shadow(true)
    .always_on_top(true)
    // Built invisible and shown by `size_menu`: a window sized before its
    // list has been laid out would flash at whatever size it was built.
    .visible(false)
    .inner_size(1.0, 1.0)
    .build();
    if let Err(e) = built {
        eprintln!("khor: the overlay window did not build, menus stay in the page: {e}");
    }
}

/// Puts khor's completions on screen under the box that owns them.
///
/// Answers whether the float took it. **Not an error when it did not**:
/// a platform with no overlay window draws its menus in the page, which
/// is not a failure but the other half of this control, and the face has
/// one to fall back to (`useFloatedMenu`).
#[cfg(desktop)]
#[tauri::command]
fn show_menu(window: tauri::Window, at: Spot, rows: serde_json::Value) -> Result<bool, String> {
    use tauri::{Emitter, Manager, PhysicalPosition, PhysicalSize};
    let app = window.app_handle().clone();
    let Some(menu) = app.get_webview_window(MENU) else { return Ok(false) };
    let (x, y, w) = on_screen(&window, at)?;
    let face = window.label().to_string();
    if let Ok(mut held) = menu_held().lock() {
        *held = Some((face, rows.clone()));
    }
    menu_up().store(true, std::sync::atomic::Ordering::Relaxed);
    // The width is this side's to set — it is the box's width, which only
    // the face can see. The height is the list's, and the list is the one
    // that knows (`size_menu`).
    let wide = (w.max(1.0) as u32).max(1);
    menu_wide().store(wide, std::sync::atomic::Ordering::Relaxed);
    let tall = menu.inner_size().map_err(|e| e.to_string())?.height.max(1);
    menu.set_position(PhysicalPosition::new(x, y)).map_err(|e| e.to_string())?;
    menu.set_size(PhysicalSize::new(wide, tall)).map_err(|e| e.to_string())?;
    app.emit_to(MENU, "menu:rows", rows).map_err(|e| e.to_string())?;
    Ok(true)
}

/// The menu window says how tall its list came out, and is shown at that
/// height.
///
/// **It has to be the one to say it.** The height of a list of rows is a
/// layout fact, and working one out on this side would mean writing down
/// a row height — a number that goes wrong the first time somebody changes
/// the type scale, and goes wrong silently, as a menu with its last row
/// cut off.
#[cfg(desktop)]
#[tauri::command]
fn size_menu(app: tauri::AppHandle, h: f64) -> Result<(), String> {
    use tauri::{Manager, PhysicalSize};
    let Some(menu) = app.get_webview_window(MENU) else { return Ok(()) };
    let scale = menu.scale_factor().map_err(|e| e.to_string())?;
    // **Not read back off the window** — see `menu_wide`, which is the
    // account of what reading it back cost.
    let wide = menu_wide().load(std::sync::atomic::Ordering::Relaxed);
    menu.set_size(PhysicalSize::new(wide, (h * scale).max(1.0) as u32))
        .map_err(|e| e.to_string())?;
    if menu_up().load(std::sync::atomic::Ordering::Relaxed) {
        menu.show().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// What the menu has to show right now, pulled by the menu window when it
/// mounts.
///
/// **A pull as well as an event, because the two cannot be ordered.** The
/// window is built and the rows are chosen in the same breath, and a
/// webview still loading its page cannot be listening yet — an event sent
/// then reaches nobody, and the first press of the address bar would open
/// an empty menu.
#[cfg(desktop)]
#[tauri::command]
fn menu_rows() -> Result<serde_json::Value, String> {
    Ok(menu_held()
        .lock()
        .ok()
        .and_then(|h| h.as_ref().map(|(_, rows)| rows.clone()))
        .unwrap_or(serde_json::Value::Null))
}

/// Somebody pressed a row. It goes back to the face that opened the menu,
/// and to no other webview.
#[cfg(desktop)]
#[tauri::command]
fn menu_pick(app: tauri::AppHandle, key: String) -> Result<(), String> {
    use tauri::Emitter;
    let face = menu_held().lock().ok().and_then(|h| h.as_ref().map(|(f, _)| f.clone()));
    let Some(face) = face else { return Ok(()) };
    app.emit_to(face, "menu:pick", key).map_err(|e| e.to_string())
}

/// Takes the menu off the screen. **Hidden rather than closed**, for
/// `open_page`'s reason turned around: the window is reused, and closing
/// it would cost the next press of the address bar a page load.
#[cfg(desktop)]
#[tauri::command]
fn hide_menu(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    menu_up().store(false, std::sync::atomic::Ordering::Relaxed);
    let Some(menu) = app.get_webview_window(MENU) else { return Ok(()) };
    menu.hide().map_err(|e| e.to_string())
}

// **The mobile halves are not refusals, and that is the difference from
// the page ones below.** A phone has no second window to hang a menu in,
// and it does not need one: it has no page webview either, so nothing is
// painting over khor's own overlays and the completions drawn in the page
// are the right ones. `false` is that answer — "the float did not take
// it" — and the face renders the list where it always did.
#[cfg(not(desktop))]
#[tauri::command]
fn show_menu(at: Spot, rows: serde_json::Value) -> Result<bool, String> {
    let _ = (at, rows);
    Ok(false)
}

#[cfg(not(desktop))]
#[tauri::command]
fn size_menu(h: f64) -> Result<(), String> {
    let _ = h;
    Ok(())
}

#[cfg(not(desktop))]
#[tauri::command]
fn menu_rows() -> Result<serde_json::Value, String> {
    Ok(serde_json::Value::Null)
}

#[cfg(not(desktop))]
#[tauri::command]
fn menu_pick(key: String) -> Result<(), String> {
    let _ = key;
    Ok(())
}

#[cfg(not(desktop))]
#[tauri::command]
fn hide_menu() -> Result<(), String> {
    Ok(())
}

// **The mobile halves are refusals, and unlike the last one that stood
// here they are true.** `add_child` is gated `all(desktop, feature =
// "unstable")` and there is no mobile equivalent: a page on a phone needs a
// native plugin adding a WKWebView or a WebView by hand (定形 单元 3/4).
// The one this file used to carry refused Linux and Windows for a thing
// they could already do; this one refuses a thing that genuinely is not
// there yet.
#[cfg(not(desktop))]
#[tauri::command]
async fn open_page(
    tab: String,
    machine: String,
    url: String,
    at: PageBox,
    agent: String,
) -> Result<khor_gui_core::web::WebBorrow, String> {
    let _ = (tab, machine, url, at, agent);
    Err(khor_catalog::msg::PAGE_NEEDS_DESKTOP.into())
}

#[cfg(not(desktop))]
#[tauri::command]
fn move_page(tab: String, at: PageBox) -> Result<(), String> {
    let _ = (tab, at);
    Ok(())
}

#[cfg(not(desktop))]
#[tauri::command]
fn hide_page(tab: String) -> Result<(), String> {
    let _ = tab;
    Ok(())
}

#[cfg(not(desktop))]
#[tauri::command]
fn close_page(tab: String) -> Result<(), String> {
    let _ = tab;
    Ok(())
}

#[cfg(not(desktop))]
#[tauri::command]
fn show_page(tab: String) -> Result<(), String> {
    let _ = tab;
    Ok(())
}

#[cfg(not(desktop))]
#[tauri::command]
fn page_step(tab: String, way: String) -> Result<(), String> {
    let _ = (tab, way);
    Ok(())
}

/// Answers with the ticket **and** the window it is good for: the window
/// belongs to the library that enforces it, not to the dialog that
/// prints it (`khor_gui_core::Ticket`).
/// 接管 (批C): ends the session's terminal side; the conversation
/// continues here (`khor_gui_core::takeover`).
#[tauri::command]
fn takeover(id: String) -> Result<(), String> {
    khor_gui_core::takeover(&Node::root_from_env(), &id)
}

#[tauri::command]
fn takeover_term(id: String) -> Result<(), String> {
    khor_gui_core::takeover_term(&Node::root_from_env(), &id)
}

/// The wizard's door (会话身份批B): a fresh agent session, born in the
/// chosen directory, as a conversation (`chat`) or a terminal (`term`);
/// `agent` is the wizard's 智能体 answer (claude when absent).
#[tauri::command]
async fn open_session(
    dir: String,
    title: Option<String>,
    form: String,
    agent: Option<String>,
    machine: Option<String>,
) -> Result<String, String> {
    khor_gui_core::open_session(
        &Node::root_from_env(),
        &dir,
        title.as_deref().unwrap_or(""),
        &form,
        agent.as_deref().unwrap_or(""),
        machine.as_deref().unwrap_or(""),
    )
    .await
}

/// The machine card's 在这台开终端 — `open --on`, through the same
/// core call the bridge answers with (docs/KHOR.md equivalence).
#[tauri::command]
async fn open_on(machine: String, title: Option<String>, cwd: Option<String>) -> Result<String, String> {
    khor_gui_core::open_term_on(
        &Node::root_from_env(),
        &machine,
        title.as_deref().unwrap_or(""),
        cwd.as_deref().unwrap_or(""),
    )
    .await
}

/// A machine conversation's readout — `khor log`, shaped for the pane.
#[tauri::command]
fn machine_log(id: String) -> Result<khor_gui_core::MachineLog, String> {
    khor_gui_core::machine_log(&Node::root_from_env(), &id)
}

/// The waiting file message's 收下 — `khor accept`, same core call.
#[tauri::command]
async fn accept_transfer(msg: String) -> Result<Vec<String>, String> {
    khor_gui_core::accept_transfer(&Node::root_from_env(), &msg).await
}

/// A path this machine already has, straight to `khor send`'s call —
/// what a tauri file drop hands over, and what the bridge answers for
/// browser verification. The picker flow is `pick_and_send` below.
#[tauri::command]
async fn send_path(machine: String, path: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        khor_gui_core::send_path(&Node::root_from_env(), &machine, std::path::Path::new(&path))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// The composer's 发文件, desktop form: a native picker answers with a
/// real local path and the path goes straight to `khor send`'s own call
/// — no upload exists here, which is the whole advantage of this form
/// (the browser faces stage bytes first; `khor_gui_core::send_upload`).
/// `None` = the person cancelled the picker, which is not an error and
/// must not wear one.
///
/// Rust-side dialog on purpose (`DialogExt`, the official v2 plugin):
/// the webview's own <input type=file> deliberately never reveals a
/// path, and driving the plugin from here keeps the frontend free of
/// the plugin's JS package and its capability grant.
#[tauri::command]
async fn pick_and_send(app: tauri::AppHandle, machine: String) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let Some(picked) = app.dialog().file().blocking_pick_file() else {
        return Ok(None);
    };
    let path = picked.into_path().map_err(|e| e.to_string())?;
    // Digesting a large file is real work; off the async runtime.
    tokio::task::spawn_blocking(move || {
        khor_gui_core::send_path(&Node::root_from_env(), &machine, &path)
    })
    .await
    .map_err(|e| e.to_string())?
    .map(Some)
}

/// The wizard's 智能体 list (批⑥): every ACP agent this person named.
/// khor's own two are not in it — they are not registrations, and the
/// face puts them beside this list rather than inside it.
#[tauri::command]
fn agents() -> Result<Vec<khor_gui_core::AgentRow>, String> {
    khor_gui_core::agents(&Node::root_from_env())
}

/// The ACP registry snapshot khor ships with, and when it was taken —
/// the same data `khor agents known` prints. The wizard is the only
/// way somebody who never opens a terminal finds out these exist.
#[tauri::command]
fn agents_known() -> Vec<khor_gui_core::agents::KnownAgent> {
    khor_gui_core::agents::agents_known()
}

#[tauri::command]
fn agents_known_taken() -> (String, usize, usize) {
    khor_gui_core::agents::agents_known_taken()
}

/// Attaches a snapshot entry, **by id**: the command is looked up on
/// the far side, never sent from the face.
#[tauri::command]
fn register_known_agent(id: String) -> Result<(), String> {
    khor_gui_core::agents::register_known(&Node::root_from_env(), &id)
}

/// What this agent was recorded as saying — no process (0.017s).
#[tauri::command]
fn agent_seen(name: String) -> Result<khor_gui_core::agents::AgentSaid, String> {
    khor_gui_core::agents::agent_seen(&Node::root_from_env(), &name)
}

/// Puts the question to the agent itself. **Starts it — measured
/// 1.5s with the package cached, 5.64s without** — so it hangs off a
/// person's press and nothing else.
#[tauri::command]
fn agent_ask(name: String) -> Result<khor_gui_core::agents::AgentSaid, String> {
    khor_gui_core::agents::agent_ask(&Node::root_from_env(), &name)
}

/// Opens a conversation the agent already has, at **its own** cwd —
/// not a fresh one, and not the wizard's 目录 answer.
#[tauri::command]
fn resume_session(agent: String, session: String) -> Result<String, String> {
    khor_gui_core::agents::resume_session(&Node::root_from_env(), &agent, &session)
}

#[tauri::command]
fn invite() -> Result<khor_gui_core::Ticket, String> {
    khor_gui_core::invite(&Node::root_from_env())
}

/// Async on purpose: pairing dials, and a dial can sit on the timeout —
/// a blocking command would freeze the window for those seconds.
#[tauri::command]
async fn pair(ticket: String) -> Result<String, String> {
    khor_gui_core::pair(&Node::root_from_env(), &ticket).await
}

pub fn run() {
    // If this process was re-exec'd as a session host, be one — the app
    // spawns hosts through its own binary (`host.rs main_if_host`).
    khor_node::host::main_if_host(Node::root_from_env());

    // The app embeds serve — the desktop is a full node, not a client
    // (one mesh, no client/server split). Running `khor serve` on the
    // same home at the same time is the known conflict; handover is on
    // the ledger.
    std::thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        rt.block_on(async {
            match Node::open(Node::root_from_env()) {
                Ok(n) => {
                    if let Err(e) = n.serve().await {
                        eprintln!("serve ended: {e}");
                    }
                }
                Err(e) => eprintln!("serve did not start: {e}"),
            }
        });
    });

    // The browser face rides with the resident, and the app is the
    // resident on a desktop somebody is sitting at. Not fatal if it
    // cannot bind: a busy port must not cost this machine its place in
    // the mesh, and `khor web` checks a live socket before printing a
    // link rather than trusting that this succeeded.
    match khor_web::listen(Node::root_from_env(), khor_web::port()) {
        Ok(face) => eprintln!("khor web face: {}", face.addr),
        Err(e) => eprintln!("{e}"),
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // The window khor's own overlays are drawn in, hung off the
            // face and hidden until something has a list to show
            // (`hang_menu_window`).
            #[cfg(desktop)]
            hang_menu_window(app.handle());
            #[cfg(not(desktop))]
            let _ = app;
            Ok(())
        })
        // **The overlay window has to be closed by hand.** It has a level
        // of its own rather than a parent (`hang_menu_window` says why),
        // and tauri leaves an app running while any window is open — so
        // without this, closing khor's window would leave the process
        // alive with nothing on screen.
        .on_window_event(|window, event| {
            #[cfg(desktop)]
            if window.label() == "main" {
                use tauri::Manager;
                let menu = || window.app_handle().get_webview_window(MENU);
                match event {
                    // **The overlay window has to be closed by hand.** It
                    // has a level of its own rather than a parent
                    // (`hang_menu_window` says why), and tauri leaves an
                    // app running while any window is open — so without
                    // this, closing khor's window would leave the process
                    // alive with nothing on screen.
                    tauri::WindowEvent::CloseRequested { .. } => {
                        if let Some(menu) = menu() {
                            let _ = menu.close();
                        }
                    }
                    // **A menu belongs to the face it hangs under, and
                    // follows it out of the room.** The overlay floats at
                    // its own window level, which is what keeps it above
                    // the page — and would also have kept it above
                    // whatever somebody switched to mid-completion. It
                    // goes away with the face and comes back with it.
                    //
                    // `menu_up` is deliberately not touched: it records
                    // whether anybody still *wants* a menu, and switching
                    // apps is not an answer to that question. The face
                    // asks and unasks; this only draws or stops drawing.
                    tauri::WindowEvent::Focused(here) => {
                        if let Some(menu) = menu() {
                            if menu_up().load(std::sync::atomic::Ordering::Relaxed) {
                                let _ = if *here { menu.show() } else { menu.hide() };
                            }
                        }
                    }
                    _ => {}
                }
            }
            #[cfg(not(desktop))]
            let _ = (window, event);
        })
        .invoke_handler(tauri::generate_handler![
            sessions,
            day_strips,
            places,
            devices,
            load,
            quota_board,
            register_account,
            remove_account,
            usage,
            quota,
            seen,
            close_session,
            rename_session,
            tell,
            pin_session,
            pin_device,
            forget_device,
            set_watch,
            face_choices,
            restyle,
            hooks_state,
            install_hooks,
            uninstall_hooks,
            chat_open,
            chat_poll,
            chat_say,
            chat_answer,
            chat_stop,
            chat_set_mode,
            chat_set_option,
            open_link,
            chat_replay,
            chat_leave,
            history,
            history_follow,
            ls,
            pull,
            dir_pins,
            pin_dir,
            term_open,
            term_attach,
            term_stream,
            term_key,
            term_drop,
            term_resize,
            term_leave,
            recents,
            recent_visit,
            forget_recent,
            web_pins,
            pin_web,
            close_web,
            open_page,
            move_page,
            hide_page,
            close_page,
            show_page,
            page_step,
            show_menu,
            hide_menu,
            size_menu,
            menu_rows,
            menu_pick,
            open_session,
            open_on,
            machine_log,
            accept_transfer,
            send_path,
            pick_and_send,
            agents,
            agents_known,
            agents_known_taken,
            register_known_agent,
            agent_seen,
            agent_ask,
            resume_session,
            takeover,
            takeover_term,
            invite,
            pair
        ])
        .run(tauri::generate_context!())
        .expect("tauri run");
}
