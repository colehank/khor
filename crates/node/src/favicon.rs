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
//! # What the page says first, `/favicon.ico` after
//!
//! The icon a site means is the one its page names — `<link rel="icon">`,
//! `shortcut icon`, `apple-touch-icon` — and `/favicon.ico` is only what
//! every browser falls back to. A single-page app answers **every** path
//! with its index page and a 200, `/favicon.ico` included, so asking the
//! root path alone got a page of HTML back from exactly the sites that
//! name an SVG in their `<head>` (lan.cibol.khor.cloud, 2026-09-22). So
//! the page is read here, the same way, and its links are tried first.
//!
//! # A picture, or nothing
//!
//! What comes back is kept only when **its bytes are a picture** — PNG,
//! ICO, GIF, JPEG, WebP, BMP or SVG, told by their first bytes — and its
//! label does not say otherwise. The label alone let a site's HTML
//! through as `image/x-icon`; the bytes alone would take a label that
//! says `text/html` at its word's opposite. A row with no picture wears
//! the generic page mark, which is the honest answer to 「no icon we
//! could get」.

use std::time::Duration;

use base64::Engine;

/// The most an icon may weigh. An ICO with a few sizes is a few KB and
/// an SVG logo a few more; a site answering with a page or a video is
/// not an icon.
const MOST: usize = 128 * 1024;

/// How much of a page is read looking for its `<link>`s: they are in
/// the `<head>`, and a head is never this long.
const HEAD: usize = 256 * 1024;

/// How a row's icon is drawn, in device pixels — what the sizes a page
/// declares are measured against.
const ROW: u32 = 32;

/// What the first bytes of `bytes` say it is, as the `data:` type, or
/// `None` for anything that is not a picture.
fn sniff(bytes: &[u8]) -> Option<&'static str> {
    let at = |magic: &[u8]| bytes.starts_with(magic);
    if at(b"\x89PNG\r\n\x1a\n") {
        return Some("image/png");
    }
    // ICO and its cursor twin: a reserved zero, then type 1 or 2.
    if at(b"\x00\x00\x01\x00") || at(b"\x00\x00\x02\x00") {
        return Some("image/x-icon");
    }
    if at(b"GIF87a") || at(b"GIF89a") {
        return Some("image/gif");
    }
    if at(b"\xff\xd8\xff") {
        return Some("image/jpeg");
    }
    if at(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        return Some("image/webp");
    }
    if at(b"BM") && bytes.len() > 14 {
        return Some("image/bmp");
    }
    // SVG is text: an `<svg` element near the top, and no `<html` — an
    // index page can carry an inline `<svg` too.
    let text = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(bytes);
    let top = String::from_utf8_lossy(&text[..text.len().min(1024)]).to_ascii_lowercase();
    let top = top.trim_start();
    if (top.starts_with("<svg") || top.starts_with("<?xml") || top.starts_with("<!--"))
        && top.contains("<svg")
        && !top.contains("<html")
    {
        return Some("image/svg+xml");
    }
    None
}

/// Whether a label lets the bytes be what they say: an image type, none
/// at all, the generic binary one, or XML for an SVG. `text/html` and
/// every other text is a page, whatever its bytes.
fn label_allows(label: &str, sniffed: &str) -> bool {
    label.is_empty()
        || label.starts_with("image/")
        || label == "application/octet-stream"
        || (sniffed == "image/svg+xml" && matches!(label, "text/xml" | "application/xml"))
}

/// A `data:` URI that is a picture by its own bytes — what [`keep`]
/// accepts and [`kept`] keeps reading back.
fn is_icon(uri: &str) -> bool {
    let Some(rest) = uri.strip_prefix("data:image/") else {
        return false;
    };
    let Some((_, b64)) = rest.split_once(";base64,") else {
        return false;
    };
    base64::engine::general_purpose::STANDARD
        .decode(b64)
        .ok()
        .is_some_and(|b| sniff(&b).is_some())
}

/// One `<link>` a page named as its icon.
#[derive(Debug)]
struct Declared {
    href: String,
    /// Lower is better.
    rank: u32,
}

/// The attributes of one tag's text (`<link …>`), names lowercased.
fn attrs(tag: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let b = tag.as_bytes();
    let mut i = 0;
    while i < b.len() {
        while i < b.len() && (b[i].is_ascii_whitespace() || b[i] == b'/') {
            i += 1;
        }
        let start = i;
        while i < b.len() && !b[i].is_ascii_whitespace() && b[i] != b'=' && b[i] != b'/' {
            i += 1;
        }
        let name = tag[start..i].to_ascii_lowercase();
        while i < b.len() && b[i].is_ascii_whitespace() {
            i += 1;
        }
        let mut value = String::new();
        if i < b.len() && b[i] == b'=' {
            i += 1;
            while i < b.len() && b[i].is_ascii_whitespace() {
                i += 1;
            }
            if i < b.len() && (b[i] == b'"' || b[i] == b'\'') {
                let q = b[i];
                i += 1;
                let vs = i;
                while i < b.len() && b[i] != q {
                    i += 1;
                }
                value = tag[vs..i].to_owned();
                i += 1;
            } else {
                let vs = i;
                while i < b.len() && !b[i].is_ascii_whitespace() {
                    i += 1;
                }
                value = tag[vs..i].to_owned();
            }
        }
        if !name.is_empty() {
            out.push((name, value));
        } else if i == start {
            i += 1;
        }
    }
    out
}

/// How far a declared icon is from what a row draws. Scalable is exact;
/// a size at or above the row costs a little per step up, one below it
/// costs more (it will be blurred); no size said sits between.
fn size_rank(sizes: &str, href: &str, kind: &str) -> u32 {
    let sizes = sizes.to_ascii_lowercase();
    if sizes.split_whitespace().any(|s| s == "any")
        || kind.contains("svg")
        || href
            .to_ascii_lowercase()
            .split(['?', '#'])
            .next()
            .unwrap_or("")
            .ends_with(".svg")
    {
        return 0;
    }
    sizes
        .split_whitespace()
        .filter_map(|s| s.split_once('x').and_then(|(w, _)| w.parse::<u32>().ok()))
        .map(|w| {
            if w >= ROW {
                (w - ROW) / 16
            } else {
                (ROW - w) * 2
            }
        })
        .min()
        .unwrap_or(4)
}

/// The icons a page declares, best first, with the base the page says
/// (`<base href>`) or else the page's own address to resolve them by.
fn declared(html: &str, page: &reqwest::Url) -> Vec<reqwest::Url> {
    let lower = html.to_ascii_lowercase();
    let mut base = page.clone();
    let mut found: Vec<Declared> = Vec::new();
    let mut from = 0;
    while let Some(at) = lower[from..].find('<') {
        let open = from + at;
        let Some(len) = lower[open..].find('>') else {
            break;
        };
        let tag = &html[open + 1..open + len];
        from = open + len + 1;
        let low = tag.to_ascii_lowercase();
        if low.starts_with("base") && low[4..].starts_with(|c: char| c.is_ascii_whitespace()) {
            if let Some((_, h)) = attrs(&tag[4..]).into_iter().find(|(n, _)| n == "href")
                && let Ok(b) = page.join(h.trim())
            {
                base = b;
            }
            continue;
        }
        if !(low.starts_with("link") && low[4..].starts_with(|c: char| c.is_ascii_whitespace())) {
            continue;
        }
        let a = attrs(&tag[4..]);
        let get = |k: &str| {
            a.iter()
                .find(|(n, _)| n == k)
                .map(|(_, v)| v.clone())
                .unwrap_or_default()
        };
        let rel = get("rel").to_ascii_lowercase();
        let rels: Vec<&str> = rel.split_whitespace().collect();
        let touch = rels.iter().any(|r| r.starts_with("apple-touch-icon"));
        if !(touch || rels.contains(&"icon")) {
            continue;
        }
        let href = get("href").trim().to_owned();
        if href.is_empty() {
            continue;
        }
        // A home-screen icon is drawn for a phone's grid, and is the
        // fallback among the page's own, not the first choice.
        let rank = size_rank(&get("sizes"), &href, &get("type")) + if touch { 50 } else { 0 };
        found.push(Declared { href, rank });
    }
    found.sort_by_key(|d| d.rank);
    found
        .into_iter()
        .filter_map(|d| base.join(&d.href).ok())
        .filter(|u| matches!(u.scheme(), "http" | "https"))
        .collect()
}

/// One picture from `url`, as a `data:` URI, or why not.
async fn fetch_icon(client: &reqwest::Client, url: reqwest::Url) -> Result<String, String> {
    let got = client.get(url).send().await.map_err(|e| e.to_string())?;
    if !got.status().is_success() {
        return Err(format!("no icon: {}", got.status()));
    }
    let label = label_of(&got);
    let bytes = got.bytes().await.map_err(|e| e.to_string())?;
    if bytes.is_empty() || bytes.len() > MOST {
        return Err(format!("not an icon: {} bytes", bytes.len()));
    }
    // **An image or nothing.** Plenty of sites answer a missing icon with
    // their index page and a 200 — labelled a page, or labelled an icon,
    // or not labelled at all. Drawing that is a broken image in the row,
    // which says less than the generic mark.
    let Some(kind) = sniff(&bytes) else {
        return Err(format!("not an image: {label}"));
    };
    if !label_allows(&label, kind) {
        return Err(format!("not an image: {label}"));
    }
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(format!("data:{kind};base64,{b64}"))
}

fn label_of(got: &reqwest::Response) -> String {
    got.headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase()
}

/// The page at `url`, as far as its head goes, when it is HTML.
async fn head_of(client: &reqwest::Client, url: reqwest::Url) -> Option<(String, reqwest::Url)> {
    let mut got = client.get(url).send().await.ok()?;
    if !got.status().is_success() {
        return None;
    }
    let label = label_of(&got);
    if !(label.is_empty() || label == "text/html" || label == "application/xhtml+xml") {
        return None;
    }
    let at = got.url().clone();
    let mut body = Vec::new();
    while body.len() < HEAD {
        match got.chunk().await {
            Ok(Some(c)) => body.extend_from_slice(&c),
            _ => break,
        }
    }
    body.truncate(HEAD);
    Some((String::from_utf8_lossy(&body).into_owned(), at))
}

/// The icon of the page at `url`, as a `data:` URI. `proxy` is the
/// borrow's `host:port`, or `None` for a page reached directly — and
/// **every** ask, the page and each icon tried, leaves that one way.
pub async fn icon(url: &str, proxy: Option<&str>) -> Result<String, String> {
    let at = reqwest::Url::parse(url).map_err(|e| format!("not a url: {url} ({e})"))?;
    if !matches!(at.scheme(), "http" | "https") {
        return Err(format!("not a page: {url}"));
    }
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
    let mut page = at.clone();
    page.set_fragment(None);
    let mut tries = match head_of(&client, page).await {
        Some((html, landed)) => declared(&html, &landed),
        None => Vec::new(),
    };
    let mut ico = at.clone();
    ico.set_path("/favicon.ico");
    ico.set_query(None);
    ico.set_fragment(None);
    if !tries.contains(&ico) {
        tries.push(ico);
    }
    let mut last = String::from("no icon");
    for u in tries {
        match fetch_icon(&client, u).await {
            Ok(icon) => return Ok(icon),
            Err(why) => last = why,
        }
    }
    Err(last)
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
///
/// **Checked on the way out**: an icon kept before the bytes were read
/// (a site's HTML kept as `image/x-icon`) is dropped here rather than
/// drawn, and the row wears the generic mark until its page loads again
/// and fetches a real one.
pub fn kept(root: &std::path::Path) -> std::collections::BTreeMap<String, String> {
    stored(root)
        .into_iter()
        .filter(|(_, v)| is_icon(&v.icon))
        .map(|(k, v)| (k, v.icon))
        .collect()
}

/// Keeps the icon of the site `url` is on. Only a `data:image/` URI whose
/// bytes are a picture is kept — what [`icon`] answers and nothing else.
///
/// **No lock**, unlike `tabs`: one writer in practice (the window), and
/// two racing writers lose one icon, which the next page load puts back.
/// Written beside the file and renamed over it, so a reader never sees
/// half of it.
pub fn keep(root: &std::path::Path, url: &str, icon: &str) -> Result<(), String> {
    let Some(key) = origin(url) else {
        return Err(format!("not a page: {url}"));
    };
    if !is_icon(icon) {
        return Err("not an icon".to_owned());
    }
    let mut all = stored(root);
    all.retain(|_, v| is_icon(&v.icon));
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
            "data:image/png;base64,iVBORw0KGgo=",
        )
        .unwrap();
        // Another page on the same site is the same key.
        keep(
            &root,
            "https://far.example/other",
            "data:image/png;base64,iVBORw0KGgoB",
        )
        .unwrap();
        let got = kept(&root);
        assert_eq!(got.len(), 1);
        assert_eq!(
            got["https://far.example"],
            "data:image/png;base64,iVBORw0KGgoB"
        );
        assert!(
            keep(
                &root,
                "file:///etc/passwd",
                "data:image/png;base64,iVBORw0KGgo="
            )
            .is_err()
        );
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
                    icon: "data:image/png;base64,iVBORw0KGgo=".to_owned(),
                    at: 1_000 + i as u64,
                },
            );
        }
        std::fs::create_dir_all(root.join(".khor")).unwrap();
        std::fs::write(kept_path(&root), serde_json::to_vec(&all).unwrap()).unwrap();
        keep(
            &root,
            "https://new.example/",
            "data:image/png;base64,iVBORw0KGgo=",
        )
        .unwrap();
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

    fn rt() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
    }

    /// A page that names no icon: the root's `/favicon.ico`, whatever
    /// page the tab is on, handed back as a picture the face can draw
    /// without asking the network again.
    #[test]
    fn a_page_gets_its_sites_icon_as_a_data_uri() {
        let (port, seen) = serve(vec![
            ("/favicon.ico", "image/png", b"\x89PNG\r\n\x1a\n-ish"),
            ("*", "text/html", INDEX),
        ]);
        // A direct fetch through the loopback: no proxy, and the system's
        // own (this machine sets one) must not be in the way either.
        let got = rt()
            .block_on(icon(&format!("http://127.0.0.1:{port}/a/b?c=d"), None))
            .unwrap();
        let seen = seen.lock().unwrap().clone();
        assert_eq!(
            seen.last().map(String::as_str),
            Some("GET /favicon.ico HTTP/1.1"),
            "{seen:?}"
        );
        assert!(got.starts_with("data:image/png;base64,"), "{got}");
    }

    /// A 200 that is a page is not an icon.
    #[test]
    fn a_page_answering_for_its_icon_is_not_an_icon() {
        let (port, _seen) = serve(vec![("*", "text/html", b"<html>not here</html>")]);
        let said = rt()
            .block_on(icon(&format!("http://127.0.0.1:{port}/"), None))
            .unwrap_err();
        assert!(said.contains("text/html"), "{said}");
    }

    /// **Through the proxy when there is one** — the borrow's local port
    /// is what a borrowed page's webview wears, and the icon must leave
    /// the same way. The proxy here is the loopback server itself, so the
    /// requests it sees are the absolute-form ones a proxy is sent.
    #[test]
    fn a_borrowed_page_is_asked_through_its_borrow() {
        let (port, seen) = serve(vec![
            ("/favicon.ico", "image/x-icon", b"\x00\x00\x01\x00ICO"),
            ("*", "text/html", INDEX),
        ]);
        let got = rt()
            .block_on(icon(
                "http://far.example/x",
                Some(&format!("127.0.0.1:{port}")),
            ))
            .unwrap();
        let seen = seen.lock().unwrap().clone();
        assert_eq!(
            seen.last().map(String::as_str),
            Some("GET http://far.example/favicon.ico HTTP/1.1"),
            "{seen:?}"
        );
        assert!(got.starts_with("data:image/x-icon;base64,"));
    }

    /// Every connection answered from a table, by path, until the test
    /// ends. Answers the port and every request line seen. A path the
    /// table does not name gets the table's `"*"` entry — a SPA answering
    /// everything with its index page.
    type Route = (&'static str, &'static str, &'static [u8]);
    fn serve(routes: Vec<Route>) -> (u16, std::sync::Arc<std::sync::Mutex<Vec<String>>>) {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let log = seen.clone();
        std::thread::spawn(move || {
            for c in l.incoming() {
                let Ok(mut c) = c else { continue };
                let mut buf = [0u8; 4096];
                let n = c.read(&mut buf).unwrap_or(0);
                let line = String::from_utf8_lossy(&buf[..n])
                    .lines()
                    .next()
                    .unwrap_or("")
                    .to_owned();
                log.lock().unwrap().push(line.clone());
                let target = line.split(' ').nth(1).unwrap_or("/");
                let path = match target.find("://") {
                    Some(i) => {
                        let rest = &target[i + 3..];
                        rest.find('/').map(|j| &rest[j..]).unwrap_or("/")
                    }
                    None => target,
                };
                let path = path.split('?').next().unwrap_or(path);
                let (_, kind, body) = routes
                    .iter()
                    .find(|r| r.0 == path)
                    .or_else(|| routes.iter().find(|r| r.0 == "*"))
                    .copied()
                    .unwrap_or(("", "text/plain", b"nothing" as &[u8]));
                let kind = if kind.is_empty() {
                    String::new()
                } else {
                    format!("Content-Type: {kind}\r\n")
                };
                let head = format!(
                    "HTTP/1.1 200 OK\r\n{kind}Content-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = c.write_all(head.as_bytes());
                let _ = c.write_all(body);
            }
        });
        (port, seen)
    }

    const SVG: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 8 8"><circle cx="4" cy="4" r="4"/></svg>"#;
    const INDEX: &[u8] =
        b"<!doctype html><html><head><title>spa</title></head><body>app</body></html>";

    /// **The page's own icon first** — the site that sent this fix: its
    /// `/favicon.ico` is the SPA's index page with a 200, and the icon
    /// it means is an SVG named in its `<link rel="icon">`.
    #[test]
    fn the_icon_a_page_declares_comes_before_the_root_path() {
        let (port, seen) = serve(vec![
            (
                "/",
                "text/html; charset=utf-8",
                br#"<!doctype html><html><head><link rel="icon" href="/assets/logo-icon.svg" /><title>spa</title></head></html>"#,
            ),
            ("/assets/logo-icon.svg", "image/svg+xml", SVG),
            ("*", "text/html", INDEX),
        ]);
        let got = rt()
            .block_on(icon(&format!("http://127.0.0.1:{port}/"), None))
            .unwrap();
        assert!(got.starts_with("data:image/svg+xml;base64,"), "{got}");
        let b64 = got.split_once(',').unwrap().1;
        let back = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .unwrap();
        assert_eq!(back, SVG);
        let seen = seen.lock().unwrap().clone();
        assert!(
            seen.iter().any(|l| l.contains("/assets/logo-icon.svg")),
            "{seen:?}"
        );
    }

    /// Relative to the page, `shortcut icon` and `apple-touch-icon`
    /// count, and among several the one nearest a row's size wins.
    #[test]
    fn a_relative_icon_of_the_right_size_is_picked() {
        let (port, _seen) = serve(vec![
            (
                "/docs/page",
                "text/html",
                br#"<html><head>
                <link rel="apple-touch-icon" href="touch.png">
                <LINK REL='shortcut icon' sizes="16x16" href='small.png'>
                <link href="big.png" sizes="256x256" rel="icon">
                <link rel="icon" sizes="32x32" href="icons/right.png">
                </head></html>"#,
            ),
            (
                "/docs/icons/right.png",
                "image/png",
                b"\x89PNG\r\n\x1a\nright",
            ),
            ("/docs/small.png", "image/png", b"\x89PNG\r\n\x1a\nsmall"),
            ("/docs/big.png", "image/png", b"\x89PNG\r\n\x1a\nbig"),
            ("/docs/touch.png", "image/png", b"\x89PNG\r\n\x1a\ntouch"),
            ("*", "text/html", INDEX),
        ]);
        let got = rt()
            .block_on(icon(&format!("http://127.0.0.1:{port}/docs/page"), None))
            .unwrap();
        let back = base64::engine::general_purpose::STANDARD
            .decode(got.split_once(',').unwrap().1)
            .unwrap();
        assert!(
            back.ends_with(b"right"),
            "{}",
            String::from_utf8_lossy(&back)
        );
    }

    /// **Nothing a picture, nothing kept**: a page that declares no icon,
    /// on a site that answers every path with its index page — even one
    /// that labels it an image, and one that labels it nothing.
    #[test]
    fn a_site_with_no_icon_has_none() {
        for kind in ["text/html", "image/x-icon", ""] {
            let (port, _seen) = serve(vec![
                ("/favicon.ico", kind, INDEX),
                ("*", "text/html", INDEX),
            ]);
            let said = rt()
                .block_on(icon(&format!("http://127.0.0.1:{port}/"), None))
                .unwrap_err();
            assert!(!said.is_empty(), "{kind}");
        }
    }

    /// A page's link pointing at something that is not a picture falls
    /// back to the root path, which is.
    #[test]
    fn a_declared_icon_that_is_not_one_falls_back() {
        let (port, _seen) = serve(vec![
            ("/", "text/html", br#"<link rel="icon" href="/gone.svg">"#),
            ("/gone.svg", "image/svg+xml", INDEX),
            ("/favicon.ico", "image/x-icon", b"\x00\x00\x01\x00ico"),
            ("*", "text/html", INDEX),
        ]);
        let got = rt()
            .block_on(icon(&format!("http://127.0.0.1:{port}/"), None))
            .unwrap();
        assert!(got.starts_with("data:image/x-icon;base64,"), "{got}");
    }

    /// Both asks — the page and its icon — leave through the borrow.
    #[test]
    fn a_borrowed_pages_link_is_read_through_its_borrow() {
        let (port, seen) = serve(vec![
            ("/x", "text/html", br#"<link rel="icon" href="/i.svg">"#),
            ("/i.svg", "image/svg+xml", SVG),
            ("*", "text/html", INDEX),
        ]);
        let got = rt()
            .block_on(icon(
                "http://far.example/x",
                Some(&format!("127.0.0.1:{port}")),
            ))
            .unwrap();
        assert!(got.starts_with("data:image/svg+xml;base64,"), "{got}");
        let seen = seen.lock().unwrap().clone();
        assert!(
            seen.iter().all(|l| l.contains("http://far.example/")),
            "{seen:?}"
        );
        assert!(
            seen.iter().any(|l| l.contains("http://far.example/i.svg")),
            "{seen:?}"
        );
    }

    /// **Not one ask of a borrowed page leaves from here** — the page's
    /// HTML included, which is read only for its `<link>`s. The page is
    /// on a real loopback site that counts what reaches it directly; the
    /// borrow is a second server answering as the proxy. The site must
    /// hear nothing, and the proxy both the page and its icon.
    #[test]
    fn nothing_of_a_borrowed_page_is_asked_directly() {
        let (site, direct) = serve(vec![
            ("/x", "text/html", br#"<link rel="icon" href="/i.svg">"#),
            ("/i.svg", "image/svg+xml", SVG),
            ("*", "text/html", INDEX),
        ]);
        let (borrow, through) = serve(vec![
            ("/x", "text/html", br#"<link rel="icon" href="/i.svg">"#),
            ("/i.svg", "image/svg+xml", SVG),
            ("*", "text/html", INDEX),
        ]);
        let got = rt()
            .block_on(icon(
                &format!("http://127.0.0.1:{site}/x"),
                Some(&format!("127.0.0.1:{borrow}")),
            ))
            .unwrap();
        assert!(got.starts_with("data:image/svg+xml;base64,"), "{got}");
        let direct = direct.lock().unwrap().clone();
        assert_eq!(direct.len(), 0, "asked directly: {direct:?}");
        let through = through.lock().unwrap().clone();
        assert!(
            through
                .iter()
                .any(|l| l.starts_with(&format!("GET http://127.0.0.1:{site}/x "))),
            "the page's HTML went round the borrow: {through:?}"
        );
        assert!(through.iter().any(|l| l.contains("/i.svg")), "{through:?}");
    }

    /// An icon kept before this rule that is not a picture is dropped on
    /// read, and a new one like it is refused.
    #[test]
    fn a_kept_page_is_not_an_icon() {
        let s = std::env::temp_dir().join(format!("khor-favicons-bad-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&s);
        let html = base64::engine::general_purpose::STANDARD.encode(INDEX);
        let bad = format!("data:image/x-icon;base64,{html}");
        let mut all = std::collections::BTreeMap::new();
        all.insert(
            "https://bad.example".to_owned(),
            Kept {
                icon: bad.clone(),
                at: 1,
            },
        );
        std::fs::create_dir_all(s.join(".khor")).unwrap();
        std::fs::write(kept_path(&s), serde_json::to_vec(&all).unwrap()).unwrap();
        let got = kept(&s);
        assert!(keep(&s, "https://worse.example/", &bad).is_err());
        let _ = std::fs::remove_dir_all(&s);
        assert!(got.is_empty(), "{got:?}");
    }
}
