//! A page's icon, for the corner of its row in the browse sidebar
//! (docs/SHELL.md 第四轮 第 2 条:「在浏览器, 是头像+网页icon」).
//!
//! # Fetched the way the page itself was
//!
//! A borrowed page leaves through another machine's network, and that is
//! the whole point of borrowing it. Fetching its icon from the window
//! that draws the sidebar would send a request for the same site out of
//! *this* machine's network — the leak `page_shim.js` exists to close,
//! reopened by a 16-pixel picture. So the icon is asked for here, through
//! the same proxy the page's webview wears (`proxy`, the borrow's local
//! address), or directly when the page is this machine's own.
//!
//! # Only `/favicon.ico`
//!
//! A page's `<link rel="icon">` lives in its document, which is the
//! webview's and not reachable from here. The root path is what every
//! browser falls back to and what most sites answer; a site that does not
//! is a row wearing the generic page mark, which is the honest answer to
//! 「no icon we could get」.

use std::time::Duration;

use base64::Engine;

/// The most an icon may weigh. An ICO with a few sizes is a few KB; a
/// site answering `/favicon.ico` with a page or a video is not an icon.
const MOST: usize = 64 * 1024;

/// The icon of the site `url` is on, as a `data:` URI. `proxy` is the
/// borrow's `host:port`, or `None` for a page reached directly.
pub async fn icon(url: &str, proxy: Option<&str>) -> Result<String, String> {
    let at = reqwest::Url::parse(url).map_err(|e| format!("not a url: {url} ({e})"))?;
    if !matches!(at.scheme(), "http" | "https") {
        return Err(format!("not a page: {url}"));
    }
    let mut ico = at.clone();
    ico.set_path("/favicon.ico");
    ico.set_query(None);
    ico.set_fragment(None);
    let mut client = reqwest::Client::builder().timeout(Duration::from_secs(5));
    // This machine's own loopback is never a thing to send through a
    // proxy the environment names.
    if matches!(at.host_str(), Some("127.0.0.1" | "localhost" | "[::1]")) && proxy.is_none() {
        client = client.no_proxy();
    }
    if let Some(p) = proxy {
        let through = reqwest::Proxy::all(format!("http://{p}")).map_err(|e| e.to_string())?;
        client = client.proxy(through);
    }
    let client = client.build().map_err(|e| e.to_string())?;
    let got = client.get(ico).send().await.map_err(|e| e.to_string())?;
    if !got.status().is_success() {
        return Err(format!("no icon: {}", got.status()));
    }
    let kind = got
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("image/x-icon")
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_owned();
    // **An image or nothing.** Plenty of sites answer a missing icon with
    // their 404 page and a 200; drawing that as a picture is a broken
    // image in the row, which says less than the generic mark.
    if !kind.starts_with("image/") {
        return Err(format!("not an image: {kind}"));
    }
    let bytes = got.bytes().await.map_err(|e| e.to_string())?;
    if bytes.is_empty() || bytes.len() > MOST {
        return Err(format!("not an icon: {} bytes", bytes.len()));
    }
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(format!("data:{kind};base64,{b64}"))
}

// # Kept on disk (docs/SHELL.md 第五轮 第 8 条)
//
// 「重开 app 就有」: an icon fetched once is written to
// `.khor/favicons.json`, keyed by the site's origin, so a restart draws
// every row's icon before any page is opened again. **Nothing is ever
// fetched to fill it** — only what [`icon`] already got, the way the
// page itself was reached, lands here; reading it back makes no request
// at all, so the rule at the top of this file holds.
//
// Keyed by origin rather than by tab: two tabs on one site are one icon,
// and a tab moving to another site must not carry the old one along.

/// How many sites the file remembers; the one kept longest ago goes
/// first. An icon is at most [`MOST`] bytes before base64, so this is the
/// bound on the file too (~17 MB at the very worst, a few hundred KB in
/// practice — real icons are a few KB).
const KEPT: usize = 200;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Kept {
    /// The `data:` URI [`icon`] answered.
    icon: String,
    /// When it was kept, ms — what [`KEPT`] evicts by.
    at: u64,
}

fn kept_path(root: &std::path::Path) -> std::path::PathBuf {
    root.join(".khor").join("favicons.json")
}

/// The site a page is on, as the key: `scheme://host[:port]`. `None` for
/// anything that is not an http(s) page.
pub fn origin(url: &str) -> Option<String> {
    let at = reqwest::Url::parse(url).ok()?;
    if !matches!(at.scheme(), "http" | "https") {
        return None;
    }
    Some(at.origin().ascii_serialization())
}

fn stored(root: &std::path::Path) -> std::collections::BTreeMap<String, Kept> {
    // Unreadable reads as empty: this is a cache, and a lost one costs a
    // generic mark until the page is opened again.
    std::fs::read_to_string(kept_path(root))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

/// Every kept icon, by origin.
pub fn kept(root: &std::path::Path) -> std::collections::BTreeMap<String, String> {
    stored(root).into_iter().map(|(k, v)| (k, v.icon)).collect()
}

/// Keeps the icon of the site `url` is on. Only a `data:image/` URI is
/// kept — what [`icon`] answers and nothing else.
///
/// **No lock**, unlike `tabs`: one writer in practice (the window), and
/// two racing writers lose one icon, which the next page load puts back.
/// Written beside the file and renamed over it, so a reader never sees
/// half of it.
pub fn keep(root: &std::path::Path, url: &str, icon: &str) -> Result<(), String> {
    let Some(key) = origin(url) else {
        return Err(format!("not a page: {url}"));
    };
    if !icon.starts_with("data:image/") {
        return Err("not an icon".to_owned());
    }
    let mut all = stored(root);
    if all.get(&key).is_some_and(|k| k.icon == icon) {
        return Ok(());
    }
    let at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    all.insert(
        key,
        Kept {
            icon: icon.to_owned(),
            at,
        },
    );
    while all.len() > KEPT {
        let Some(oldest) = all.iter().min_by_key(|(_, v)| v.at).map(|(k, _)| k.clone()) else {
            break;
        };
        all.remove(&oldest);
    }
    let dot = root.join(".khor");
    std::fs::create_dir_all(&dot).map_err(|e| e.to_string())?;
    let text = serde_json::to_vec(&all).map_err(|e| e.to_string())?;
    let p = kept_path(root);
    let tmp = dot.join(format!("favicons.json.{}", std::process::id()));
    std::fs::write(&tmp, text).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &p).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        e.to_string()
    })
}

#[cfg(test)]
mod kept_tests {
    use super::*;

    struct Scratch(std::path::PathBuf);
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// Kept by origin, read back after the fact, and only icons.
    #[test]
    fn an_icon_is_kept_by_its_site_and_read_back() {
        let s = Scratch(std::env::temp_dir().join(format!("khor-favicons-{}", std::process::id())));
        let root = s.0.clone();
        let _ = std::fs::remove_dir_all(&root);
        assert!(kept(&root).is_empty());
        keep(
            &root,
            "https://far.example/a/b?c",
            "data:image/png;base64,AA==",
        )
        .unwrap();
        // Another page on the same site is the same key.
        keep(
            &root,
            "https://far.example/other",
            "data:image/png;base64,BB==",
        )
        .unwrap();
        let got = kept(&root);
        assert_eq!(got.len(), 1);
        assert_eq!(got["https://far.example"], "data:image/png;base64,BB==");
        assert!(keep(&root, "file:///etc/passwd", "data:image/png;base64,AA==").is_err());
        assert!(keep(&root, "https://far.example/", "<html>").is_err());
        assert_eq!(
            origin("http://127.0.0.1:8080/x").as_deref(),
            Some("http://127.0.0.1:8080")
        );
    }

    /// The file does not grow past [`KEPT`] sites; the oldest goes.
    #[test]
    fn the_oldest_site_goes_first() {
        let s =
            Scratch(std::env::temp_dir().join(format!("khor-favicons-cap-{}", std::process::id())));
        let root = s.0.clone();
        let _ = std::fs::remove_dir_all(&root);
        let mut all = std::collections::BTreeMap::new();
        for i in 0..KEPT {
            all.insert(
                format!("https://s{i}.example"),
                Kept {
                    icon: "data:image/png;base64,AA==".to_owned(),
                    at: 1_000 + i as u64,
                },
            );
        }
        std::fs::create_dir_all(root.join(".khor")).unwrap();
        std::fs::write(kept_path(&root), serde_json::to_vec(&all).unwrap()).unwrap();
        keep(&root, "https://new.example/", "data:image/png;base64,AA==").unwrap();
        let got = kept(&root);
        assert_eq!(got.len(), KEPT);
        assert!(!got.contains_key("https://s0.example"));
        assert!(got.contains_key("https://new.example"));
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;

    use super::*;

    /// One answer on a loopback port, whatever is asked. Returns the port
    /// and the path the request named.
    fn serve_once(
        status: &str,
        kind: &str,
        body: &'static [u8],
    ) -> (u16, std::thread::JoinHandle<String>) {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let head = format!(
            "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        let t = std::thread::spawn(move || {
            let (mut c, _) = l.accept().unwrap();
            let mut buf = [0u8; 2048];
            let n = c.read(&mut buf).unwrap();
            let asked = String::from_utf8_lossy(&buf[..n]).to_string();
            c.write_all(head.as_bytes()).unwrap();
            c.write_all(body).unwrap();
            asked.lines().next().unwrap_or("").to_owned()
        });
        (port, t)
    }

    fn rt() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
    }

    /// The root's `/favicon.ico`, whatever page the tab is on, handed back
    /// as a picture the face can draw without asking the network again.
    #[test]
    fn a_page_gets_its_sites_icon_as_a_data_uri() {
        let (port, asked) = serve_once("200 OK", "image/png", b"\x89PNG-ish");
        // A direct fetch through the loopback: no proxy, and the system's
        // own (this machine sets one) must not be in the way either.
        let got = rt()
            .block_on(icon(&format!("http://127.0.0.1:{port}/a/b?c=d"), None))
            .unwrap();
        assert_eq!(asked.join().unwrap(), "GET /favicon.ico HTTP/1.1");
        assert!(got.starts_with("data:image/png;base64,"), "{got}");
    }

    /// A 200 that is a page is not an icon.
    #[test]
    fn a_page_answering_for_its_icon_is_not_an_icon() {
        let (port, _asked) = serve_once("200 OK", "text/html", b"<html>not here</html>");
        let said = rt()
            .block_on(icon(&format!("http://127.0.0.1:{port}/"), None))
            .unwrap_err();
        assert!(said.contains("text/html"), "{said}");
    }

    /// **Through the proxy when there is one** — the borrow's local port
    /// is what a borrowed page's webview wears, and the icon must leave
    /// the same way. The proxy here is the loopback server itself, so the
    /// request it sees is the absolute-form one a proxy is sent.
    #[test]
    fn a_borrowed_page_is_asked_through_its_borrow() {
        let (port, asked) = serve_once("200 OK", "image/x-icon", b"ICO");
        let got = rt()
            .block_on(icon(
                "http://far.example/x",
                Some(&format!("127.0.0.1:{port}")),
            ))
            .unwrap();
        assert_eq!(
            asked.join().unwrap(),
            "GET http://far.example/favicon.ico HTTP/1.1"
        );
        assert!(got.starts_with("data:image/x-icon;base64,"));
    }
}
