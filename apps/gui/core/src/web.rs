//! The browser landing's data layer (docs/NET.md 借网): pinned pages and
//! the borrow behind opening one. Like [`crate::files`], it renames the
//! node's answers for TS and decides nothing.
//!
//! Opening a page is two halves that live in two skins: the borrow (a
//! lease + a local proxy port) is here, callable from both skins; turning
//! that port into a real browsing window is the tauri skin's alone, since
//! only it has a webview to point at the proxy.

use std::path::Path;

use khor_node::Node;
use serde::Serialize;
use ts_rs::TS;

/// The script every borrowed page runs before its own scripts do.
///
/// **Lives here, in the shared half, on purpose.** Each skin builds its
/// own webview — the tauri skin today, an iOS and an Android plugin in
/// docs/handoff/001.md's units 3 and 4 — and every one of them has to
/// inject this. A copy per skin is a copy that goes stale on four
/// platforms at once, and the thing going stale would be a leak nobody
/// can see.
///
/// The script says what it takes away and what it cannot; read
/// `page_shim.js` before changing what any skin does with it. The short
/// version: two WebKit APIs leave through the device's own network
/// rather than the borrow, and a third (`dns-prefetch`) does too and
/// **is not blocked** — no script can reach it.
///
/// **Injected for all frames, not the main one.** A leak in an iframe
/// is the same leak; `initialization_script` alone would leave every
/// embedded frame untouched.
pub const PAGE_SHIM: &str = include_str!("page_shim.js");

/// Where a borrow's proxy listens, for a window (or a test) to point at.
#[derive(Debug, Clone, Serialize, TS)]
pub struct WebBorrow {
    /// The borrow session's id — closing it collapses the proxy.
    pub session: String,
    /// `127.0.0.1:<port>`, the proxy a webview sets as its `proxy_url`.
    pub addr: String,
}

/// Opens a borrow of `machine`'s network and answers where its proxy
/// listens. The tauri skin then builds a webview window with this address
/// as its proxy; the dev bridge gets the same answer but has no window to
/// open, which is why the window half lives in the skin and not here.
///
/// **Unlisted, and that is the rule rather than a detail** (docs/NET.md
/// 可达性, 一层不占会话列表): a page's borrow is worn by the tab that
/// opened it, so it takes no row in the session list. Only the borrow a
/// person holds by hand (`khor borrow`) is a session, because that one
/// has no other face and nowhere else to hang its lifetime.
pub async fn borrow_web(root: &Path, machine: &str) -> Result<WebBorrow, String> {
    let (session, addr) = Node::open(root.to_path_buf())?
        .borrow_unlisted(machine)
        .await?;
    Ok(WebBorrow { session, addr })
}

/// Gives back a page's borrow: the handle `borrow_web` answered with, and
/// the proxy port goes with it.
///
/// **Not `close_session`** — there is no row to close. The two verbs look
/// interchangeable from the face's side and are not: a close names a
/// session in the list, and this names a borrow that deliberately is not
/// in it.
pub async fn drop_web_borrow(root: &Path, session: &str) -> Result<(), String> {
    Node::open(root.to_path_buf())?.drop_borrow(session).await
}

/// A page's site icon as a `data:` URI, asked **the way the page was**:
/// through its borrow's local proxy (`proxy`, the `addr` `borrow_web`
/// answered), or directly for a page this machine opens itself
/// (`khor_node::favicon` says why the difference matters).
///
/// **What comes back is also kept** (`khor_node::favicon::keep`,
/// docs/SHELL.md 第五轮 第 8 条), so a restart draws it before the page is
/// opened again. A cache that cannot be written is not the answer
/// failing: the icon is still handed back.
pub async fn page_icon(root: &Path, url: &str, proxy: &str) -> Result<String, String> {
    let icon = khor_node::favicon::icon(url, (!proxy.is_empty()).then_some(proxy)).await?;
    let _ = khor_node::favicon::keep(root, url, &icon);
    Ok(icon)
}

/// Every icon kept on this disk, by site origin
/// (`khor_node::favicon::kept`). Read only — nothing is fetched to
/// answer it.
pub fn page_icons(root: &Path) -> std::collections::BTreeMap<String, String> {
    khor_node::favicon::kept(root)
}

/// [`open_link`]'s whitelist on its own: `http` and `https`, refused in
/// words otherwise. Public for the phone's opener (`khor_gui`'s iOS
/// `open_link`, which hands the page to the system rather than to a
/// program), so both roads refuse the same addresses.
pub fn page_only(url: &str) -> Result<(), String> {
    let scheme = url.split_once("://").map(|(s, _)| s.to_ascii_lowercase());
    if matches!(scheme.as_deref(), Some("http") | Some("https")) {
        Ok(())
    } else {
        Err(khor_catalog::msg::not_a_page(url))
    }
}

/// Hands a page to whatever this machine opens pages with.
///
/// **The address comes out of an agent's own words** — a conversation
/// pane paints a link because a model wrote one — so the scheme is a
/// whitelist, not a check for the obviously bad. `http` and `https` are
/// pages; `file:` reads this machine's disk, and a scheme this machine
/// happens to have registered can be an action rather than a page (a
/// mail client sending, an app that takes a command). Anything else is
/// refused with its own text, and the face falls back to showing the
/// address so a person can decide for themselves.
///
/// The opener is the platform's, not a tool the user had to install
/// (docs/STACK.md 零依赖 is about **the user's tools**): `open` and
/// `xdg-open` ship with the desktop that would be showing this window.
pub fn open_link(url: &str) -> Result<(), String> {
    page_only(url)?;
    // A URL is a URL, never a shell word: the opener is executed
    // directly with one argument, so nothing in the address can become
    // a second command.
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(target_os = "windows") {
        "explorer"
    } else {
        "xdg-open"
    };
    std::process::Command::new(opener)
        .arg(url)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod open_link_tests {
    use super::open_link;

    /// The whitelist is the point: what an agent writes is text, and a
    /// scheme this machine has registered can be an action rather than
    /// a page. Only the refusals are asserted here — the allowed side
    /// would open a browser on whoever ran the tests.
    #[test]
    fn only_a_page_is_a_page() {
        for url in [
            "file:///etc/passwd",
            "FILE:///etc/passwd",
            "mailto:someone@example.com",
            "javascript:alert(1)",
            "vscode://file/etc/passwd",
            "/etc/passwd",
            "",
        ] {
            assert!(open_link(url).is_err(), "must refuse: {url}");
        }
    }
}

/// Where a page asks for this device's own loopback — `localhost`,
/// `127.x.x.x`, `[::1]` — the host as written and the port (the scheme's
/// when none is written). `None` for anything else.
pub fn loopback_of(url: &str) -> Option<(String, u16)> {
    let (scheme, rest) = url.split_once("://")?;
    let default = match scheme.to_ascii_lowercase().as_str() {
        "http" => 80,
        "https" => 443,
        _ => return None,
    };
    let authority = rest.split(['/', '?', '#']).next()?;
    let authority = authority.rsplit_once('@').map_or(authority, |(_, a)| a);
    let (host, port) = if let Some(v6) = authority.strip_prefix('[') {
        let (h, after) = v6.split_once(']')?;
        (format!("[{h}]"), after.strip_prefix(':'))
    } else {
        match authority.rsplit_once(':') {
            Some((h, p)) => (h.to_owned(), Some(p)),
            None => (authority.to_owned(), None),
        }
    };
    let port = match port {
        Some(p) => p.parse().ok()?,
        None => default,
    };
    let bare = host.trim_start_matches('[').trim_end_matches(']');
    let lower = host.to_ascii_lowercase();
    let loop_ip = bare.parse::<std::net::IpAddr>().is_ok_and(|ip| ip.is_loopback());
    (loop_ip || lower == "localhost" || lower == "localhost.").then_some((host, port))
}

/// One loopback forward: where its connections go on the far side, and
/// through which borrow's proxy.
type Loop = std::sync::Arc<std::sync::Mutex<(String, String)>>;

fn loops() -> &'static std::sync::Mutex<std::collections::HashMap<u16, Loop>> {
    static L: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<u16, Loop>>> = std::sync::OnceLock::new();
    L.get_or_init(Default::default)
}

/// **A loopback address through a machine is that machine's** (UX-38 §1,
/// the owner 2026-10-06: `http://127.0.0.1:8765/…` on a tab through his
/// Mac showed a blank page on the phone).
///
/// WebKit never sends a loopback address to a proxy: a data store's
/// `proxyConfigurations` carry every other host through the borrow's
/// CONNECT — names, private addresses — but `localhost`, `127.0.0.1` and
/// the device's own addresses go straight out (measured with a logging
/// proxy, macOS and the iOS simulator alike: no CONNECT, the load came
/// from `com.apple.WebKit.Networking`). On a phone that is the phone's own
/// loopback, where nothing listens. No setting changes it (`matchDomains`
/// tried).
///
/// So the phone listens there itself: `127.0.0.1:<port>` on this device
/// pipes every connection through the borrow's proxy as `CONNECT
/// <host>:<port>`, which the machine resolves on its own side — its
/// `localhost`, its dev server. The page keeps its address, its `Host`
/// and its origin, and links to the same port keep working.
///
/// One forward per port, pointed at the page that asked last; asked again
/// it is re-pointed, never bound twice. `Err` with the catalog's words
/// when this device's port is taken.
pub async fn hold_loopback(host: &str, port: u16, proxy: &str) -> Result<(), String> {
    let target = format!("{host}:{port}");
    if let Some(held) = loops().lock().map_err(|e| e.to_string())?.get(&port) {
        if let Ok(mut at) = held.lock() {
            *at = (target, proxy.to_owned());
        }
        return Ok(());
    }
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
        .await
        .map_err(|_| khor_catalog::msg::page_port_taken(port))?;
    let held: Loop = std::sync::Arc::new(std::sync::Mutex::new((target, proxy.to_owned())));
    loops().lock().map_err(|e| e.to_string())?.insert(port, held.clone());
    tokio::spawn(async move {
        while let Ok((sock, _)) = listener.accept().await {
            let at = held.lock().map(|a| a.clone());
            if let Ok((target, proxy)) = at {
                tokio::spawn(async move {
                    let _ = through(sock, &target, &proxy).await;
                });
            }
        }
    });
    Ok(())
}

/// Whether this device's `port` is already piped to a machine.
pub fn loopback_held(port: u16) -> bool {
    loops().lock().is_ok_and(|l| l.contains_key(&port))
}

/// One connection, through the proxy as a CONNECT: the pipe once the
/// proxy says `200`, else the connection dropped — which the page hears as
/// a failed load, and says so.
async fn through(mut sock: tokio::net::TcpStream, target: &str, proxy: &str) -> Result<(), String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut up = tokio::net::TcpStream::connect(proxy).await.map_err(|e| e.to_string())?;
    up.write_all(format!("CONNECT {target} HTTP/1.1\r\nHost: {target}\r\n\r\n").as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        if up.read(&mut byte).await.map_err(|e| e.to_string())? == 0 || head.len() > 8 * 1024 {
            return Err("the proxy hung up".into());
        }
        head.push(byte[0]);
    }
    let line = String::from_utf8_lossy(&head);
    if line.split_whitespace().nth(1) != Some("200") {
        return Err(line.lines().next().unwrap_or_default().to_owned());
    }
    tokio::io::copy_bidirectional(&mut sock, &mut up).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod loopback_tests {
    use super::{hold_loopback, loopback_of};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[test]
    fn loopback_addresses_are_told_apart() {
        assert_eq!(loopback_of("http://127.0.0.1:8765/x?y"), Some(("127.0.0.1".into(), 8765)));
        assert_eq!(loopback_of("http://localhost/"), Some(("localhost".into(), 80)));
        assert_eq!(loopback_of("https://LOCALHOST:5173"), Some(("LOCALHOST".into(), 5173)));
        assert_eq!(loopback_of("http://[::1]:3000/"), Some(("[::1]".into(), 3000)));
        assert_eq!(loopback_of("http://127.1.2.3:9/"), Some(("127.1.2.3".into(), 9)));
        for url in ["http://172.23.250.213:8765/", "http://x.localhost:1/", "https://example.com/", "file:///etc", "http://localhost.example/"] {
            assert_eq!(loopback_of(url), None, "{url}");
        }
    }

    /// The page connects to this device's port; what arrives is the far
    /// side's answer, asked through the proxy as the page's own host —
    /// and a second ask re-points the same port.
    #[tokio::test]
    async fn the_port_pipes_through_the_proxy_to_the_far_side() {
        // The far side: a server on a port of its own.
        let far = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let far_at = far.local_addr().unwrap();
        tokio::spawn(async move {
            while let Ok((mut s, _)) = far.accept().await {
                let _ = s.write_all(b"far side").await;
            }
        });
        // The borrow's proxy: answers a CONNECT for the asked host by
        // dialling the far side, and remembers what it was asked.
        let proxy = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let proxy_at = proxy.local_addr().unwrap().to_string();
        let (asked_tx, mut asked) = tokio::sync::mpsc::unbounded_channel::<String>();
        tokio::spawn(async move {
            while let Ok((mut s, _)) = proxy.accept().await {
                let asked_tx = asked_tx.clone();
                tokio::spawn(async move {
                    let mut head = Vec::new();
                    let mut b = [0u8; 1];
                    while !head.ends_with(b"\r\n\r\n") {
                        s.read_exact(&mut b).await.unwrap();
                        head.push(b[0]);
                    }
                    let line = String::from_utf8_lossy(&head).lines().next().unwrap().to_owned();
                    let _ = asked_tx.send(line);
                    let mut up = tokio::net::TcpStream::connect(far_at).await.unwrap();
                    s.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n").await.unwrap();
                    let _ = tokio::io::copy_bidirectional(&mut s, &mut up).await;
                });
            }
        });
        // A free port on this device for the page's address.
        let port = {
            let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            l.local_addr().unwrap().port()
        };
        hold_loopback("localhost", port, &proxy_at).await.unwrap();
        let mut page = tokio::net::TcpStream::connect(("127.0.0.1", port)).await.unwrap();
        let mut got = String::new();
        page.read_to_string(&mut got).await.unwrap();
        assert_eq!(got, "far side");
        assert_eq!(asked.recv().await.unwrap(), format!("CONNECT localhost:{port} HTTP/1.1"));
        // Asked again for the same port: re-pointed, not bound twice.
        hold_loopback("127.0.0.1", port, &proxy_at).await.unwrap();
        let mut page = tokio::net::TcpStream::connect(("127.0.0.1", port)).await.unwrap();
        let mut got = String::new();
        page.read_to_string(&mut got).await.unwrap();
        assert_eq!(asked.recv().await.unwrap(), format!("CONNECT 127.0.0.1:{port} HTTP/1.1"));
    }

    #[tokio::test]
    async fn a_taken_port_is_said() {
        let taken = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = taken.local_addr().unwrap().port();
        let said = hold_loopback("localhost", port, "127.0.0.1:9").await.unwrap_err();
        assert!(said.contains(&port.to_string()), "{said}");
    }
}
