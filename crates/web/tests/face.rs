//! The locks on the face, against a face that is really listening.
//!
//! Every refusal here is paired with the request that differs from it
//! in **one** way and must succeed. A negative assertion on its own
//! cannot tell "refused" from "the probe never reached anything", and
//! this file is entirely negative assertions — so each one carries its
//! own proof that the door was working when it was tried.
//!
//! The client is written out longhand rather than pulled in: these
//! tests need to send a `Host` header that lies and an `Origin` header
//! that lies, which is exactly what a well-behaved HTTP client exists
//! to prevent.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};

/// A home nobody else is using, removed when the test ends.
struct Home(std::path::PathBuf);

impl Home {
    fn new(tag: &str) -> Home {
        let dir = std::env::temp_dir().join(format!("khor-web-test-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a home to test in");
        Home(dir)
    }
}

impl Drop for Home {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// One request, one reply, spelled out. Returns the status line and the
/// body — the two things every assertion below is about.
fn request(addr: SocketAddr, path: &str, headers: &[(&str, &str)]) -> (u16, String) {
    let mut sock = TcpStream::connect(addr).expect("the face is listening");
    // 90s, not 5: on this Mac, a freshly compiled test binary is a new
    // face to macOS, and its first network-stack touch hangs waiting on
    // an authorization prompt nobody over ssh can click (#73; the
    // template is `crates/node/tests/transfer.rs`). Measured here
    // 2026-08-21: every request that stays inside this crate answers in
    // ~1ms, and the first one that reaches `Node::open` took **42.9s**.
    // At 5s that read like a server that never answers, which cost a
    // wrong diagnosis before the number was actually taken.
    let wait = std::time::Duration::from_secs(90);
    sock.set_read_timeout(Some(wait)).unwrap();
    let mut req = format!("POST {path} HTTP/1.1\r\n");
    let mut has_host = false;
    for (k, v) in headers {
        if k.eq_ignore_ascii_case("host") {
            has_host = true;
        }
        req.push_str(&format!("{k}: {v}\r\n"));
    }
    if !has_host {
        req.push_str(&format!("Host: {addr}\r\n"));
    }
    req.push_str("Content-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}");
    sock.write_all(req.as_bytes()).expect("sending");

    // **Read to `Content-Length`, never to EOF.** `read_to_end` waits
    // for the server to close, and a keep-alive server that answered
    // instantly does not — so a perfectly good reply read as a five
    // second timeout, and the first thing that looked like was a
    // server that never answers. It cost a wrong diagnosis and a
    // change to the server that had to be taken back out.
    let mut raw = Vec::new();
    let mut buf = [0u8; 4096];
    let text = loop {
        match sock.read(&mut buf) {
            Ok(0) => break String::from_utf8_lossy(&raw).into_owned(),
            Ok(n) => raw.extend_from_slice(&buf[..n]),
            Err(e) => panic!("reading after {} bytes: {e}", raw.len()),
        }
        let text = String::from_utf8_lossy(&raw).into_owned();
        let Some((head, body)) = text.split_once("\r\n\r\n") else {
            continue;
        };
        let want: usize = head
            .lines()
            .find_map(|l| l.strip_prefix("Content-Length: "))
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(0);
        if body.len() >= want {
            break text;
        }
    };
    let code: u16 = text
        .split_whitespace()
        .nth(1)
        .and_then(|c| c.parse().ok())
        .unwrap_or_else(|| panic!("no status line in: {text:.200}"));
    let body = text.split_once("\r\n\r\n").map(|(_, b)| b.to_owned()).unwrap_or_default();
    (code, body)
}

fn face(tag: &str) -> (Home, SocketAddr, String) {
    let home = Home::new(tag);
    // Port 0: the OS picks a free one, so two of these can run at once
    // and neither can collide with a developer's own face.
    let face = khor_web::listen(home.0.clone(), 0).expect("the face starts");
    let key = khor_web::key::read(&home.0).expect("readable").expect("minted at listen");
    let addr = SocketAddr::from(([127, 0, 0, 1], face.addr.port()));
    (home, addr, key)
}

fn bearer(key: &str) -> String {
    format!("Bearer {key}")
}

/// **The probe can say yes.** Everything else in this file is a
/// refusal, and a probe that always answered "unreachable" would make
/// every one of them pass while testing nothing.
#[test]
fn a_listening_face_answers_and_a_closed_port_does_not() {
    let (_home, addr, _key) = face("reach");
    assert!(khor_web::answers_at(addr), "a face that is listening must answer at {addr}");

    // A port with nothing on it. Bound and dropped, so it is a port
    // that was free a moment ago rather than a number picked by hand
    // that something else might own.
    let closed = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let dead = closed.local_addr().unwrap();
    drop(closed);
    assert!(!khor_web::answers_at(dead), "nothing is listening at {dead}");
}

/// The page is public and the data is not — one request apart.
#[test]
fn the_page_needs_no_key_and_the_data_does() {
    let (_home, addr, key) = face("public");
    let (code, body) = request(addr, "/", &[]);
    assert_eq!(code, 200, "the shell must load before anybody has a key");
    assert!(body.contains("<div id=\"root\">"), "that was not the app's page: {body:.120}");

    let (code, _) = request(addr, "/api/devices", &[]);
    assert_eq!(code, 401, "the same visitor must not get data");
    let (code, _) = request(addr, "/api/devices", &[("Authorization", &bearer(&key))]);
    assert_eq!(code, 200, "…and must get it with the key — otherwise the 401 proves nothing");
}

/// A key that is not the key. The pair rules out "this endpoint refuses
/// everything", which is what a broken handler looks like from here.
#[test]
fn a_wrong_key_is_refused_and_the_right_one_is_not() {
    let (_home, addr, key) = face("wrongkey");
    let wrong = "0".repeat(key.len());
    let (code, body) = request(addr, "/api/devices", &[("Authorization", &bearer(&wrong))]);
    assert_eq!(code, 401, "a wrong key opened the door");
    assert!(!body.trim().is_empty(), "a refusal with nothing in it tells the person nothing");

    let (code, _) = request(addr, "/api/devices", &[("Authorization", &bearer(&key))]);
    assert_eq!(code, 200);
}

/// The cross-origin lock: a page somewhere else, holding a real key.
#[test]
fn another_page_is_refused_even_holding_the_key() {
    let (_home, addr, key) = face("origin");
    let auth = bearer(&key);
    let (code, _) = request(
        addr,
        "/api/devices",
        &[("Authorization", &auth), ("Origin", "http://evil.example")],
    );
    assert_eq!(code, 403, "a foreign page with the key got through");

    let ours = format!("http://{addr}");
    let (code, _) = request(addr, "/api/devices", &[("Authorization", &auth), ("Origin", &ours)]);
    assert_eq!(code, 200, "our own page must still be served — else the 403 proves nothing");
}

/// The rebinding lock. The request carries a real key and a name that
/// is not an address, which is the shape a rebound page arrives in.
#[test]
fn a_name_that_is_not_an_address_is_refused() {
    let (_home, addr, key) = face("host");
    let auth = bearer(&key);
    let (code, _) = request(addr, "/api/devices", &[("Authorization", &auth), ("Host", "evil.example")]);
    assert_eq!(code, 403, "a rebound name with the key got through");

    let real = addr.to_string();
    let (code, _) = request(addr, "/api/devices", &[("Authorization", &auth), ("Host", &real)]);
    assert_eq!(code, 200);
}

/// **`khor web --new` takes the old links back, on a face that never
/// restarted.** The verb's whole promise is in that last clause: a key
/// change that needed a restart would leave every printed link live for
/// as long as somebody left the resident running.
#[test]
fn a_new_key_retires_the_old_one_without_a_restart() {
    let (home, addr, old) = face("rotate");
    let (code, _) = request(addr, "/api/devices", &[("Authorization", &bearer(&old))]);
    assert_eq!(code, 200, "the old key worked to begin with");

    let new = khor_web::key::rotate(&home.0).expect("rotating");
    assert_ne!(new, old, "rotate handed back the same key");

    let (code, _) = request(addr, "/api/devices", &[("Authorization", &bearer(&old))]);
    assert_eq!(code, 401, "the old link still opens — nothing was revoked");
    let (code, _) = request(addr, "/api/devices", &[("Authorization", &bearer(&new))]);
    assert_eq!(code, 200, "the new key must work, or this test would pass on a dead face");
}

/// The key never lands anywhere that travels. Every document khor syncs
/// reaches every machine in the network, so a key that got written into
/// one would be handed to every device somebody ever pairs.
#[test]
fn the_key_is_nowhere_that_syncs() {
    let (home, _addr, key) = face("nosync");
    // A node, so the home holds the documents this is really about.
    // Without it the search walks a directory containing the key and
    // nothing else, and the guard at the bottom of this test says so
    // rather than letting it pass — which is how this line got here.
    khor_node::Node::open(home.0.clone()).expect("a node in this home");
    let mut checked = 0;
    let mut stack = vec![home.0.clone()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("listing the home") {
            let path = entry.expect("an entry").path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            // The key's own file is where it belongs; every other file
            // under the home is fair game for this search.
            if path == khor_web::key::path(&home.0) {
                continue;
            }
            checked += 1;
            let bytes = std::fs::read(&path).unwrap_or_default();
            assert!(
                !String::from_utf8_lossy(&bytes).contains(&key),
                "the key is written into {}",
                path.display()
            );
        }
    }
    // Without this the test passes on an empty home, which is exactly
    // what it would find if `listen` had failed to write anything.
    assert!(checked > 0, "no other files under the home — this search proved nothing");
}

