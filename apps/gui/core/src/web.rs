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
use serde::{Deserialize, Serialize};
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

/// The lifecycle reported by the platform's current page navigation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum PageNavigationPhase {
    Opening,
    Started,
    Committed,
    Finished,
    /// The platform reported an end without confirming success.
    Settled,
    Failed,
    Cancelled,
}

/// An original platform error cause. Native integer codes remain strings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct PageNavigationCause {
    pub domain: String,
    pub code: String,
    pub description: String,
    pub url: String,
}

/// A bounded original error chain, including at most four total causes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct PageNavigationError {
    pub domain: String,
    pub code: String,
    pub description: String,
    pub url: String,
    pub underlying: Vec<PageNavigationCause>,
    pub truncated: bool,
}

/// A snapshot of one view's current navigation. Decimal identities preserve
/// native counter precision; an absent estimate means no platform estimate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct PageNavigationState {
    pub tab: String,
    pub view: String,
    pub navigation: String,
    pub revision: String,
    pub phase: PageNavigationPhase,
    pub estimate: Option<f64>,
    pub requested_url: String,
    pub url: String,
    pub exit: String,
    pub proxied: bool,
    pub error: Option<PageNavigationError>,
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

/// Only browser pages may enter a page view. System handoffs use
/// [`system_target`] so a file URI never becomes a browser page.
pub fn page_only(url: &str) -> Result<(), String> {
    let scheme = url.split_once("://").map(|(s, _)| s.to_ascii_lowercase());
    if matches!(scheme.as_deref(), Some("http") | Some("https")) {
        Ok(())
    } else {
        Err(khor_catalog::msg::not_a_page(url))
    }
}

/// The system can open pages and absolute file URIs. Keep this separate
/// from page navigation, and pass accepted targets to the opener unchanged.
pub fn system_target(url: &str) -> Result<(), String> {
    if page_only(url).is_ok() {
        return Ok(());
    }
    let file = url
        .split_once("://")
        .filter(|(scheme, _)| scheme.eq_ignore_ascii_case("file"));
    let valid = file.is_some_and(|(_, rest)| {
        let Some((host, _path)) = rest.split_once('/') else {
            return false;
        };
        // A file authority is a host, never credentials or a port. Brackets
        // retain an IPv6 authority; the OS decides whether it is reachable.
        let host_ok = !host.contains('@')
            && (!host.contains(':') || (host.starts_with('[') && host.ends_with(']')));
        let bytes = url.as_bytes();
        let mut at = 0;
        while at < bytes.len() {
            if bytes[at].is_ascii_whitespace() || bytes[at].is_ascii_control() {
                return false;
            }
            if bytes[at] == b'%' {
                if at + 2 >= bytes.len()
                    || !bytes[at + 1].is_ascii_hexdigit()
                    || !bytes[at + 2].is_ascii_hexdigit()
                {
                    return false;
                }
                at += 2;
            }
            at += 1;
        }
        host_ok
    });
    if valid {
        Ok(())
    } else {
        Err(khor_catalog::msg::not_openable(url))
    }
}

/// Hands an explicit system target to the client's platform opener.
pub fn open_link(url: &str) -> Result<(), String> {
    system_target(url)?;
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
    use super::{page_only, system_target};

    #[test]
    fn system_targets_keep_file_uris_out_of_page_views() {
        for url in [
            "file:///etc/a%20b.txt",
            "FILE:///etc/passwd",
            "file://server/share/a.txt",
            "file:///C:/Work/a.txt",
            "file://[::1]/share/a.txt",
        ] {
            assert!(system_target(url).is_ok(), "system must accept: {url}");
            assert!(page_only(url).is_err(), "page must refuse: {url}");
        }
        for url in ["http://localhost:4321/a", "https://example.com/a"] {
            assert!(system_target(url).is_ok());
            assert!(page_only(url).is_ok());
        }
    }

    #[test]
    fn system_target_validation_never_opens_or_guesses_another_scheme() {
        for url in [
            "mailto:someone@example.com",
            "javascript:alert(1)",
            "vscode://file/etc/passwd",
            "/etc/passwd",
            "",
            "file:relative",
            "file://server",
            "file://user@server/share/a",
            "file://server:80/share/a",
            "file:///a%",
            "file:///a%ZZ",
            "file:///a b",
            "file:///a\n",
        ] {
            assert!(system_target(url).is_err(), "must refuse: {url}");
        }
    }
}

/// Stable WebKit store identity per browsing exit, shared by desktop
/// and iOS. It keeps proxy configuration separate from the app's store.
pub fn page_store_identifier(exit: &str) -> [u8; 16] {
    let fnv = |seed: u64, bytes: &[u8]| {
        bytes.iter().fold(seed, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100_0000_01b3)
        })
    };
    let key = format!("khor-page:{exit}");
    let mut id = [0u8; 16];
    id[..8].copy_from_slice(&fnv(0xcbf2_9ce4_8422_2325, key.as_bytes()).to_le_bytes());
    id[8..].copy_from_slice(&fnv(0x6c62_272e_07bb_0142, key.as_bytes()).to_le_bytes());
    id
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
    let loop_ip = bare
        .parse::<std::net::IpAddr>()
        .is_ok_and(|ip| ip.is_loopback());
    (loop_ip || lower == "localhost" || lower == "localhost.").then_some((host, port))
}

/// A loopback forward belongs to the native pages retaining this lease.
/// Closing the last page stops accepting and closes its in-flight pipes.
#[derive(Clone, Debug)]
pub struct LoopbackLease(std::sync::Arc<LoopbackForward>);

#[derive(Debug)]
struct LoopbackForward {
    target: String,
    proxy: String,
    tasks: Vec<tokio::task::JoinHandle<()>>,
}

impl Drop for LoopbackForward {
    fn drop(&mut self) {
        for task in &self.tasks {
            task.abort();
        }
    }
}

/// A page keeps earlier loopback origins for its back/forward history.
/// Repeated layout/open requests do not accumulate duplicate leases.
#[derive(Default)]
pub struct LoopbackLeases(Vec<LoopbackLease>);

impl LoopbackLeases {
    pub fn retain(&mut self, lease: Option<LoopbackLease>) {
        if let Some(lease) = lease
            && !self
                .0
                .iter()
                .any(|held| std::sync::Arc::ptr_eq(&held.0, &lease.0))
        {
            self.0.push(lease);
        }
    }
}

fn loops()
-> &'static std::sync::Mutex<std::collections::HashMap<u16, std::sync::Weak<LoopbackForward>>> {
    static LOOPS: std::sync::OnceLock<
        std::sync::Mutex<std::collections::HashMap<u16, std::sync::Weak<LoopbackForward>>>,
    > = std::sync::OnceLock::new();
    LOOPS.get_or_init(Default::default)
}

/// WebKit bypasses a data store's CONNECT proxy for loopback URLs, even
/// with explicit matchDomains (native macOS probe 2026-11-08). Bind only
/// that loopback origin and pipe it through the chosen machine's proxy.
/// The URL, Host, origin and TLS remain unchanged.
///
/// A live port can be shared only by the same target AND proxy. Another
/// exit is refused instead of silently redirecting already-open pages.
/// Drop every lease before replacing that route. An occupied local port
/// is also refused: borrowing must never fall through to a local service.
pub async fn hold_loopback(host: &str, port: u16, proxy: &str) -> Result<LoopbackLease, String> {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(250);
    loop {
        if let Some(lease) = try_hold_loopback(host, port, proxy)? {
            return Ok(lease);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(khor_catalog::msg::page_port_taken(port));
        }
        // The last owner aborted this port's old task. Its socket closes
        // when Tokio retires that task; never race it with a new route.
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
}

/// None means an unowned listener is still retiring, never a live conflict.
fn try_hold_loopback(host: &str, port: u16, proxy: &str) -> Result<Option<LoopbackLease>, String> {
    let target = format!("{}:{port}", host.to_ascii_lowercase());
    let mut loops = loops().lock().map_err(|e| e.to_string())?;
    loops.retain(|at, held| *at == port || held.strong_count() > 0);
    let retiring = loops.contains_key(&port);
    if let Some(held) = loops.get(&port).and_then(std::sync::Weak::upgrade) {
        return if held.target == target && held.proxy == proxy {
            Ok(Some(LoopbackLease(held)))
        } else {
            Err(khor_catalog::msg::page_port_taken(port))
        };
    }
    let bare = host.trim_start_matches('[').trim_end_matches(']');
    let addresses: Vec<std::net::IpAddr> =
        if bare.eq_ignore_ascii_case("localhost") || bare.eq_ignore_ascii_case("localhost.") {
            vec![
                std::net::Ipv4Addr::LOCALHOST.into(),
                std::net::Ipv6Addr::LOCALHOST.into(),
            ]
        } else {
            let address: std::net::IpAddr = bare.parse().map_err(|_| "not a loopback address")?;
            if !address.is_loopback() {
                return Err("not a loopback address".into());
            }
            vec![address]
        };
    // Reserve every address before publishing or spawning anything. In
    // particular localhost must not reach an unrelated IPv6 listener.
    let mut listeners = Vec::new();
    for address in addresses {
        let listener = match std::net::TcpListener::bind((address, port)) {
            Ok(listener) => listener,
            Err(error) if retiring && error.kind() == std::io::ErrorKind::AddrInUse => {
                return Ok(None);
            }
            Err(_) => return Err(khor_catalog::msg::page_port_taken(port)),
        };
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        listeners.push(tokio::net::TcpListener::from_std(listener).map_err(|e| e.to_string())?);
    }
    let tasks = listeners.into_iter().map(|listener| {
        let (target, proxy) = (target.clone(), proxy.to_owned());
        tokio::spawn(async move {
            let mut pipes = tokio::task::JoinSet::new();
            loop {
                tokio::select! {
                    accepted = listener.accept() => {
                        let Ok((sock, _)) = accepted else { break };
                        let (target, proxy) = (target.clone(), proxy.clone());
                        pipes.spawn(async move { let _ = through(sock, &target, &proxy).await; });
                    }
                    _ = pipes.join_next(), if !pipes.is_empty() => {}
                }
            }
        })
    }).collect();
    let held = std::sync::Arc::new(LoopbackForward {
        target,
        proxy: proxy.to_owned(),
        tasks,
    });
    loops.insert(port, std::sync::Arc::downgrade(&held));
    Ok(Some(LoopbackLease(held)))
}

pub fn loopback_held(port: u16) -> bool {
    loops()
        .lock()
        .is_ok_and(|loops| loops.get(&port).is_some_and(|held| held.strong_count() > 0))
}

/// One connection, through the proxy as a CONNECT: the pipe once the
/// proxy says `200`, else the connection dropped — which the page hears as
/// a failed load, and says so.
async fn through(mut sock: tokio::net::TcpStream, target: &str, proxy: &str) -> Result<(), String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut up = tokio::net::TcpStream::connect(proxy)
        .await
        .map_err(|e| e.to_string())?;
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
    tokio::io::copy_bidirectional(&mut sock, &mut up)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod loopback_tests {
    use super::{hold_loopback, loopback_of, page_store_identifier};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[test]
    fn loopback_addresses_are_told_apart() {
        assert_eq!(
            loopback_of("http://127.0.0.1:8765/x?y"),
            Some(("127.0.0.1".into(), 8765))
        );
        assert_eq!(
            loopback_of("http://localhost/"),
            Some(("localhost".into(), 80))
        );
        assert_eq!(
            loopback_of("https://LOCALHOST:5173"),
            Some(("LOCALHOST".into(), 5173))
        );
        assert_eq!(
            loopback_of("http://[::1]:3000/"),
            Some(("[::1]".into(), 3000))
        );
        assert_eq!(
            loopback_of("http://127.1.2.3:9/"),
            Some(("127.1.2.3".into(), 9))
        );
        for url in [
            "http://172.23.250.213:8765/",
            "http://x.localhost:1/",
            "https://example.com/",
            "file:///etc",
            "http://localhost.example/",
        ] {
            assert_eq!(loopback_of(url), None, "{url}");
        }
    }

    #[test]
    fn browser_stores_keep_exits_separate_and_preserve_the_phone_identity() {
        assert_ne!(page_store_identifier(""), page_store_identifier("home"));
        assert_ne!(page_store_identifier("home"), page_store_identifier("work"));
        // The previous iOS store_for used exactly these two FNV-1a seeds.
        assert_eq!(
            page_store_identifier("home"),
            [
                122, 107, 205, 55, 220, 70, 247, 199, 93, 13, 72, 35, 107, 212, 187, 146
            ]
        );
    }

    /// The page connects to this device's port; what arrives is the far
    /// side's answer, asked through the proxy as the page's own host —
    /// and another page on that route shares the listener.
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
                    let line = String::from_utf8_lossy(&head)
                        .lines()
                        .next()
                        .unwrap()
                        .to_owned();
                    let _ = asked_tx.send(line);
                    let mut up = tokio::net::TcpStream::connect(far_at).await.unwrap();
                    s.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
                        .await
                        .unwrap();
                    let _ = tokio::io::copy_bidirectional(&mut s, &mut up).await;
                });
            }
        });
        // A free port on this device for the page's address.
        let port = {
            let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            l.local_addr().unwrap().port()
        };
        let first = hold_loopback("localhost", port, &proxy_at).await.unwrap();
        let mut page = tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .unwrap();
        let mut got = String::new();
        page.read_to_string(&mut got).await.unwrap();
        assert_eq!(got, "far side");
        assert_eq!(
            asked.recv().await.unwrap(),
            format!("CONNECT localhost:{port} HTTP/1.1")
        );
        // The same route is shared without changing the first page.
        let second = hold_loopback("localhost", port, &proxy_at).await.unwrap();
        drop(first);
        let mut page = tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .unwrap();
        let mut got = String::new();
        page.read_to_string(&mut got).await.unwrap();
        assert_eq!(
            asked.recv().await.unwrap(),
            format!("CONNECT localhost:{port} HTTP/1.1")
        );
        drop(second);
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                if let Ok(listener) = std::net::TcpListener::bind(("127.0.0.1", port)) {
                    drop(listener);
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn a_taken_port_is_said() {
        let taken = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = taken.local_addr().unwrap().port();
        let said = hold_loopback("localhost", port, "127.0.0.1:9")
            .await
            .unwrap_err();
        assert!(said.contains(&port.to_string()), "{said}");
    }

    #[tokio::test]
    async fn an_open_page_cannot_be_retargeted_by_another_exit() {
        let port = {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            listener.local_addr().unwrap().port()
        };
        let _first = hold_loopback("127.0.0.1", port, "127.0.0.1:19001")
            .await
            .unwrap();
        assert!(
            hold_loopback("127.0.0.1", port, "127.0.0.1:19002")
                .await
                .is_err(),
            "a second page must not change the first page's exit"
        );
    }

    #[tokio::test]
    async fn closing_the_last_page_closes_an_inflight_pipe() {
        let proxy = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let free = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = free.local_addr().unwrap().port();
        drop(free);
        let lease = hold_loopback("127.0.0.1", port, &proxy.local_addr().unwrap().to_string())
            .await
            .unwrap();
        let mut page = tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .unwrap();
        let (mut upstream, _) = proxy.accept().await.unwrap();
        // Wait until the forward owns both ends, then close its last page.
        assert_eq!(upstream.read_u8().await.unwrap(), b'C');
        drop(lease);
        let closed = tokio::time::timeout(std::time::Duration::from_secs(2), page.read_u8())
            .await
            .unwrap();
        assert_eq!(
            closed.unwrap_err().kind(),
            std::io::ErrorKind::UnexpectedEof
        );
    }

    #[tokio::test]
    async fn a_closed_route_can_be_replaced_without_a_false_port_conflict() {
        let free = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = free.local_addr().unwrap().port();
        drop(free);
        let first = hold_loopback("127.0.0.1", port, "127.0.0.1:19001")
            .await
            .unwrap();
        drop(first);
        let _replacement = hold_loopback("127.0.0.1", port, "127.0.0.1:19002")
            .await
            .unwrap();
    }

    #[cfg(target_os = "macos")]
    #[tokio::test]
    #[ignore = "requires KHOR_WEBKIT_PROBE, an isolated native WKWebView fixture"]
    async fn native_webkit_reaches_the_borrowed_loopback_origin() {
        let program = std::env::var_os("KHOR_WEBKIT_PROBE").expect("native fixture required");
        for host in ["127.0.0.1", "localhost", "[::1]"] {
            let proxy = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let proxy_port = proxy.local_addr().unwrap().port();
            let free = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let port = free.local_addr().unwrap().port();
            drop(free);
            #[derive(Debug, Default)]
            struct Facts {
                accepted: usize,
                connect: Vec<String>,
                served: usize,
                idle: usize,
                errors: Vec<String>,
            }
            async fn header(stream: &mut tokio::net::TcpStream) -> Result<Option<String>, String> {
                let mut bytes = Vec::new();
                while !bytes.ends_with(b"\r\n\r\n") {
                    let mut byte = [0];
                    if stream.read(&mut byte).await.map_err(|e| e.to_string())? == 0 {
                        return if bytes.is_empty() {
                            Ok(None)
                        } else {
                            Err("partial HTTP header".into())
                        };
                    }
                    bytes.push(byte[0]);
                    if bytes.len() > 8192 {
                        return Err("oversized HTTP header".into());
                    }
                }
                String::from_utf8(bytes)
                    .map(Some)
                    .map_err(|e| e.to_string())
            }
            let facts = std::sync::Arc::new(std::sync::Mutex::new(Facts::default()));
            let recording = facts.clone();
            let target = format!("{host}:{port}");
            // A browser may preconnect without sending a request. The real
            // borrow proxy accepts concurrently; this fixture must too.
            let answering = tokio::spawn(async move {
                let mut clients = tokio::task::JoinSet::new();
                loop {
                    tokio::select! {
                        accepted = proxy.accept() => {
                            let Ok((mut stream, _)) = accepted else { break };
                            recording.lock().unwrap().accepted += 1;
                            let (facts, target) = (recording.clone(), target.clone());
                            clients.spawn(async move {
                                let result: Result<(), String> = async {
                                    let Some(head) = header(&mut stream).await? else {
                                        facts.lock().unwrap().idle += 1;
                                        return Ok(());
                                    };
                                    let line = head.lines().next().unwrap_or_default().to_owned();
                                    facts.lock().unwrap().connect.push(line.clone());
                                    if line != format!("CONNECT {target} HTTP/1.1") {
                                        return Err(format!("wrong proxy target: {line}"));
                                    }
                                    stream.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n").await.map_err(|e| e.to_string())?;
                                    let Some(request) = header(&mut stream).await? else {
                                        facts.lock().unwrap().idle += 1;
                                        return Ok(());
                                    };
                                    if !request.starts_with("GET / ") {
                                        return Err(format!("wrong HTTP request: {}", request.lines().next().unwrap_or_default()));
                                    }
                                    let host = request.lines().filter_map(|line| line.split_once(':')).find_map(|(key, value)| key.eq_ignore_ascii_case("host").then(|| value.trim()));
                                    if host != Some(target.as_str()) {
                                        return Err(format!("wrong HTTP origin: {host:?}, expected {target}"));
                                    }
                                    stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 13\r\nConnection: close\r\n\r\nBORROWED_EXIT").await.map_err(|e| e.to_string())?;
                                    facts.lock().unwrap().served += 1;
                                    Ok(())
                                }.await;
                                if let Err(error) = result { facts.lock().unwrap().errors.push(error); }
                            });
                        }
                        _ = clients.join_next(), if !clients.is_empty() => {}
                    }
                }
            });
            let lease = hold_loopback(host, port, &format!("127.0.0.1:{proxy_port}"))
                .await
                .unwrap();
            let mut process = tokio::process::Command::new(&program);
            process.args([
                "default",
                &format!("http://{host}:{port}/"),
                &proxy_port.to_string(),
            ]);
            process.kill_on_drop(true);
            let output = tokio::time::timeout(std::time::Duration::from_secs(30), process.output())
                .await
                .unwrap()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
            answering.abort();
            let _ = answering.await;
            let facts = facts.lock().unwrap();
            eprintln!("native loopback {host}: {facts:?}");
            assert_eq!(
                result["body"], "BORROWED_EXIT",
                "{host}: {result}; {facts:?}"
            );
            assert!(facts.errors.is_empty(), "{host}: {facts:?}");
            assert!(facts.served > 0, "{host}: {facts:?}");
            drop(lease);
        }
    }
}

#[cfg(test)]
mod navigation_contract_tests {
    use super::{PageNavigationPhase, PageNavigationState};

    #[test]
    fn native_identity_and_error_codes_survive_the_json_boundary() {
        let value = serde_json::json!({
            "tab":"tab", "view":"18446744073709551615", "navigation":"9007199254740993",
            "revision":"9007199254740994", "phase":"failed", "estimate":null,
            "requested_url":"https://requested.example", "url":"https://actual.example",
            "exit":"home", "proxied":true,
            "error":{"domain":"NSURLErrorDomain","code":"-9223372036854775808",
                "description":"连接中断", "url":"https://failed.example",
                "underlying":[], "truncated":false}
        });
        let state: PageNavigationState = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(state.phase, PageNavigationPhase::Failed);
        assert_eq!(serde_json::to_value(state).unwrap(), value);
        assert_eq!(
            serde_json::to_value(PageNavigationPhase::Settled).unwrap(),
            "settled"
        );
    }

    #[test]
    fn the_http_skin_has_no_native_page_snapshot_or_node_work() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        // This path must not be opened or created just to report native absence.
        let root = std::path::Path::new("/does-not-exist/khor-native-page-state");
        assert_eq!(
            crate::api::answer(&rt, root, "page_state", &serde_json::json!({"tab":"tab"})).unwrap(),
            "null"
        );
        for invalid in [
            serde_json::json!({}),
            serde_json::json!({"tab":42}),
            serde_json::json!({"tab":null}),
        ] {
            assert!(crate::api::answer(&rt, root, "page_state", &invalid).is_err());
        }
    }
}
