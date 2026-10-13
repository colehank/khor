//! The web face's **installation** facts: which port it binds, where its
//! key lives, and how that key is read, minted and rotated.
//!
//! # Why these are here and not in `khor_web`
//!
//! They were in `khor_web` until 2026-09-19, which is where the server
//! that binds the port and checks the key lives — and that is exactly
//! the layer two other things now have to reach:
//!
//! - `khor doctor` reports whether the port is answering and the key is
//!   there ([`crate::doctor`]);
//! - 安全 shows the key and rotates it (`khor lock`'s page).
//!
//! Both are the node's, and **`khor_web` depends on this crate**, so
//! either asking it would be a cycle. What moved is only the part that
//! is about *this installation* — a file in this store, a port an
//! environment variable can move — and not the serving: `khor_web` keeps
//! `listen`, the request loop, and [`matches`]'s constant-time compare,
//! and reads these from here so there is still one spelling of each.
//!
//! # The key is khor's to hand out, never a password to choose
//!
//! 128 random bits from the same mint as the hand-off cookies
//! ([`crate::link::fresh_hex`]), `.khor/web.key`, mode 0600, **never in
//! a synced document**: every CRDT document here lands on every machine
//! by design, so a key that synced would mean pairing a laptop silently
//! handed it the keys to every face in the fleet.

use std::path::{Path, PathBuf};
use std::net::SocketAddr;

use khor_catalog::msg;

/// Where the face listens unless `KHOR_WEB_PORT` says otherwise.
///
/// KHOR on a phone keypad. Far from the development cluster's own range
/// on purpose — 1430 is vite, 1431 the dev bridge, 1442/1443 the smoke
/// run — because a product port a development tool can shadow is a fault
/// nobody diagnoses twice.
pub const WEB_DEFAULT_PORT: u16 = 5467;

/// The port the face uses.
///
/// **One function rather than one line in each caller**: the resident
/// binds this port, `khor web` prints a link naming it and `khor doctor`
/// probes it, and any two of those disagreeing would produce a link that
/// is wrong in the one way nobody checks — it looks exactly like a link
/// that is right.
pub fn web_port() -> u16 {
    std::env::var("KHOR_WEB_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(WEB_DEFAULT_PORT)
}

/// Where this machine keeps its key.
pub fn web_key_path(root: &Path) -> PathBuf {
    root.join(".khor").join("web.key")
}

/// The key as it is on disk right now, or `None` if this machine has no
/// face yet.
///
/// **Read on every request rather than cached.** It costs one 32-byte
/// read against a request whose own work is measured in milliseconds,
/// and it buys the thing the verb promises: `khor web --new` takes
/// effect on the next request, with no message to send to a running
/// serve and no window where the old link still opens.
pub fn web_key_read(root: &Path) -> Result<Option<String>, String> {
    let p = web_key_path(root);
    match std::fs::read_to_string(&p) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(msg::cant_read(p.display(), e)),
        Ok(text) => {
            check_permissions(&p)?;
            let key = text.trim();
            // **An empty file is not a key.** A truncated write leaves
            // one, and `khor_web::key::matches` is an honest comparison:
            // it would say yes to a request that presented nothing,
            // which is every request. Refusing here rather than in the
            // caller keeps the hole closed for callers that have not
            // been written yet.
            if key.is_empty() {
                return Err(msg::web_key_empty(p.display()));
            }
            Ok(Some(key.to_owned()))
        }
    }
}

/// The key, minting one if this machine has never had a face.
pub fn web_key_ensure(root: &Path) -> Result<String, String> {
    match web_key_read(root) {
        Ok(Some(key)) => Ok(key),
        Ok(None) => mint(root),
        Err(e) => Err(e),
    }
}

/// A new key, replacing whatever was there. Every link printed before
/// this call stops working — that is the whole point of the verb, so it
/// is not softened with a grace period: a key you can still use after
/// revoking it is not revoked.
pub fn web_key_rotate(root: &Path) -> Result<String, String> {
    mint(root)
}

fn mint(root: &Path) -> Result<String, String> {
    let dot = root.join(".khor");
    std::fs::create_dir_all(&dot).map_err(|_| msg::cant_make_dir(dot.display()))?;
    let key = crate::link::fresh_hex()?;
    crate::link::write_private(&web_key_path(root), key.as_bytes())?;
    Ok(key)
}

/// A loose key refuses to answer, the way a loose identity refuses to
/// start (`khor_net::identity`). On the machines this is for — shared
/// university boxes where every account can reach 127.0.0.1 — the mode
/// bits *are* the boundary between "my face" and "everyone's face", and
/// a key somebody else can read is one they can use silently, from
/// inside, forever.
fn check_permissions(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(path)
            .map_err(|e| msg::cant_read(path.display(), e))?
            .permissions()
            .mode()
            & 0o777;
        if mode & 0o077 != 0 {
            return Err(msg::web_key_too_open(
                format_args!("{mode:o}"),
                path.display(),
            ));
        }
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

// ── Which of khor's faces are up on this machine (docs/MORE.md 头像页) ──
//
// 「本机开着哪几张脸, 当前是哪张」 — the user, 2026-09-20. The 「face」 in
// that sentence is khor's *interface* — the desktop app, the browser
// face — and not the avatar `khor face` restyles. The two meanings sit
// one letter apart in the CLI (`khor face` / `khor faces`) and the
// avatar verb is the one misnamed; renaming it is a separate change and
// is on the ledger.
//
// **Nothing here is a second probe.** The port and the key path are this
// module's own, and the liveness test is the one every other recorded
// pid in this tree gets. `khor doctor`'s `web` row reaches for the same
// two facts, so no machine can be listening on one page and quiet on the
// other.

/// One of khor's faces on this machine.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FaceUp {
    /// The stable key — `app` or `web`, never translated. The word a
    /// person reads is looked up from it (`cli::face_name`), the rule
    /// `doctor::name` already keeps.
    pub key: String,
    pub up: bool,
    /// Where it is, when that is a thing to say: the port, for the web
    /// face. `None` for a face with no address and for one that is not
    /// up — a port printed beside 未运行 is an address nobody can use.
    pub at: Option<String>,
}

/// Every face khor has, in the order a page lists them — **including
/// the ones that are not up**. A list of only the running ones would
/// answer 「本机开着哪几张脸」 with a page that shrinks, and somebody
/// looking for the browser face would have nothing on screen telling
/// them it exists.
pub const FACE_KEYS: &[&str] = &["app", "web"];

/// Where the desktop app records that it is running.
///
/// **A file rather than a walk of the process table.** Matching by name
/// is on the ledger for hitting a class where somebody meant one process
/// (账本: 起进程时选对了工具, 收进程时要沿用它), and it reads forward the
/// same way: on a shared machine a name match counts another account's
/// khor.
pub fn app_pid_path(root: &Path) -> PathBuf {
    root.join(".khor").join("app.json")
}

#[derive(serde::Serialize, serde::Deserialize)]
struct AppPid {
    pid: u32,
}

/// The desktop shell saying it is here. Called once as the window comes
/// up; a crash leaves the file behind, and the pid check below is what
/// makes that harmless rather than a face khor reports forever.
pub fn app_here(root: &Path) -> Result<(), String> {
    let dot = root.join(".khor");
    std::fs::create_dir_all(&dot).map_err(|_| msg::cant_make_dir(dot.display()))?;
    let text = serde_json::to_vec(&AppPid {
        pid: std::process::id(),
    })
    .map_err(|e| e.to_string())?;
    let p = app_pid_path(root);
    std::fs::write(&p, text).map_err(|e| msg::cant_write(p.display(), e))
}

/// Whether the desktop app is running on this machine.
pub fn app_up(root: &Path) -> bool {
    std::fs::read_to_string(app_pid_path(root))
        .ok()
        .and_then(|t| serde_json::from_str::<AppPid>(&t).ok())
        .map(|a| crate::link::pid_alive(a.pid))
        .unwrap_or(false)
}

/// Whether anything answers on the web face's port.
///
/// **Loopback, with a short budget**, because the question here is only
/// 「本机有没有人在听这个端口」. `khor web` probes the address it is about
/// to print instead — a different question with a different right answer
/// on a Mac whose firewall has not been told about khor.
fn web_up(port: u16) -> bool {
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(300)).is_ok()
}

/// Which faces are up, in [`FACE_KEYS`] order.
///
/// **None on a phone** (docs/IOS.md 第二步): its app is not 「the desktop
/// app」 and it serves no web face, so both rows would be 未运行 about
/// things that do not exist there — and the port probe would be asked of
/// a loopback nothing listens on.
pub fn faces(root: &Path) -> Vec<FaceUp> {
    if !khor_core::hosts_here() {
        return Vec::new();
    }
    let port = web_port();
    FACE_KEYS
        .iter()
        .map(|&key| match key {
            "web" => {
                let up = web_up(port);
                FaceUp {
                    key: key.to_owned(),
                    up,
                    at: up.then(|| port.to_string()),
                }
            }
            _ => FaceUp {
                key: key.to_owned(),
                up: app_up(root),
                at: None,
            },
        })
        .collect()
}

/// The word a person reads for one face's key.
///
/// A match here rather than a generated `word()`, `doctor::name`'s
/// reason: the generator only builds those for the sections whose keys
/// are data, and these keys are code's.
pub fn face_name(key: &str) -> &str {
    match key {
        "app" => khor_catalog::cli::face_name_app(),
        // **Borrowed from the 自检 row rather than spelled twice**: one
        // fact, one word — and the catalog refuses two keys in one
        // section holding one text anyway, which is that rule wearing a
        // compile error.
        "web" => khor_catalog::cli::doctor_name_web(),
        other => other,
    }
}

/// The address to hand to another device, key included.
pub fn link(ip: std::net::IpAddr, port: u16, key: &str) -> String {
    match ip {
        std::net::IpAddr::V4(_) => format!("http://{ip}:{port}/?k={key}"),
        // Brackets or the port reads as part of the address, and the
        // link a person copies is one that does not open.
        std::net::IpAddr::V6(_) => format!("http://[{ip}]:{port}/?k={key}"),
    }
}

/// Which of this machine's addresses to put in front of a person.
///
/// A phone is not this machine, so a routable address beats loopback —
/// but loopback is returned rather than nothing when it is all there
/// is, because a link that works from this desk is still a working
/// link, and "no address" would read as "the face is broken".
pub fn best_address(ips: &[std::net::IpAddr]) -> Option<std::net::IpAddr> {
    let rank = |ip: &std::net::IpAddr| match ip {
        std::net::IpAddr::V4(v4) if v4.is_private() => 0,
        std::net::IpAddr::V4(v4) if !v4.is_loopback() => 1,
        std::net::IpAddr::V4(_) => 3,
        // Last: a global v6 address usually works and is unreadable to
        // type from a phone, and this string is meant to be read.
        std::net::IpAddr::V6(_) => 2,
    };
    ips.iter().min_by_key(|ip| rank(ip)).copied()
}

/// Whether the face really answers **at this address** — a whole
/// request and a whole reply, from outside the process.
///
/// `khor web` says "the face is here", so the claim gets checked before
/// it is printed. Two things this had to learn the hard way, both
/// measured on the developer's Mac 2026-08-21:
///
/// - **Probe the address that will be printed, not loopback.** A
///   loopback probe passed while every request to the machine's own LAN
///   address hung — macOS's application firewall exempts loopback and
///   silently swallows the rest for a binary nobody has approved. The
///   verb would have printed a confident link to a face no phone could
///   open, which is the worst of the three possible answers.
/// - **A connection is not an answer.** In that same state the TCP
///   handshake completed — `curl` printed `Connected` — and then
///   nothing ever came back. `connect_timeout` returning `Ok` would
///   have called that reachable.
pub fn answers_at(addr: SocketAddr) -> bool {
    use std::io::{Read, Write};
    let wait = std::time::Duration::from_secs(2);
    let Ok(mut sock) = std::net::TcpStream::connect_timeout(&addr, wait) else {
        return false;
    };
    let _ = sock.set_read_timeout(Some(wait));
    let _ = sock.set_write_timeout(Some(wait));
    // The page, not an API call: this asks whether a browser pointed
    // here would get something, and the page is the first thing one
    // asks for.
    if sock
        .write_all(
            format!("GET / HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n").as_bytes(),
        )
        .is_err()
    {
        return false;
    }
    let mut head = [0u8; 15];
    let mut got = 0;
    while got < head.len() {
        match sock.read(&mut head[got..]) {
            Ok(0) | Err(_) => return false,
            Ok(n) => got += n,
        }
    }
    // Any HTTP status line is an answer — this probe exists to catch
    // the firewall that completes the handshake and then swallows every
    // byte, and a keyless probe now earns the refusal page (记住一次批:
    // the page sits behind the key), which is this face answering in
    // full. Pinning 200 here would report a healthy face as absent.
    head.starts_with(b"HTTP/1.1 ")
}
