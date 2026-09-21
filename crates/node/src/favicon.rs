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
