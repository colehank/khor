//! **The Beijing relay's move to HTTPS, on loopback** (round NET-7).
//! Nothing here touches the real relay or a real certificate: every
//! certificate is self-signed here, every relay is a scratch `khor relay`
//! on 127.0.0.1, and the client trusts the test's certificate through
//! `KHOR_RELAY_CA`.
//!
//! - the client side: a plain relay and its TLS successor
//!   (`KHOR_RELAY_SUCCESSOR`, the loopback twin of `KHOR_SUCCESSORS`):
//!   home on the plain one while the TLS one is down, move the moment it
//!   answers, keep the wires through a certificate renewal (`SIGHUP`, no
//!   restart), and fall back when it stops;
//! - `scripts/relay-https.sh` against a scratch root, with a stand-in
//!   `acme.sh` that installs and renews the way acme.sh does and a
//!   stand-in `systemctl` that really runs the unit's `ExecStart`: set
//!   up, run again changing nothing, renew in place.

use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use khor_net::endpoint::{self, Relay};

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn scratch(tag: &str) -> Scratch {
    let p = std::env::temp_dir().join(format!("khor-https-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    Scratch(p)
}

const PROXY_VARS: [&str; 6] = ["http_proxy", "https_proxy", "HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "all_proxy"];

/// A self-signed certificate for `names`, written as `<stem>.pem` and
/// `<stem>.key`; the certificate is its own root, so trusting it is
/// trusting the relay.
fn self_signed(dir: &Path, stem: &str, names: &[&str]) -> (PathBuf, PathBuf) {
    let made = rcgen::generate_simple_self_signed(names.iter().map(|n| n.to_string()).collect::<Vec<_>>()).unwrap();
    let (cert, key) = (dir.join(format!("{stem}.pem")), dir.join(format!("{stem}.key")));
    std::fs::write(&cert, made.cert.pem()).unwrap();
    std::fs::write(&key, made.signing_key.serialize_pem()).unwrap();
    (cert, key)
}

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port()
}

/// A running `khor relay`, killed by pid however the test leaves; what it
/// prints after its banner is kept in `out`.
struct Running {
    child: Child,
    out: PathBuf,
}

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn relay(home: &Path, args: &[&str]) -> Running {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_khor"));
    cmd.arg("relay")
        .args(args)
        .arg("--state")
        .arg(home.join("state"))
        .env("KHOR_HOME", home)
        .env_remove("KHOR_RELAY")
        .stdout(Stdio::piped())
        .stderr(std::fs::File::create(home.join("relay.log")).unwrap());
    for k in PROXY_VARS {
        cmd.env_remove(k);
    }
    let mut child = cmd.spawn().unwrap();
    let out = home.join("relay.out");
    let lines = std::io::BufReader::new(child.stdout.take().unwrap()).lines();
    let to = out.clone();
    std::thread::spawn(move || {
        use std::io::Write;
        let mut f = std::fs::File::create(to).unwrap();
        for line in lines.map_while(Result::ok) {
            let _ = writeln!(f, "{line}");
        }
    });
    let began = Instant::now();
    while !std::fs::read_to_string(&out).unwrap_or_default().contains("relay-stats.json") {
        if let Ok(Some(status)) = child.try_wait() {
            panic!("khor relay exited ({status}): {}", std::fs::read_to_string(home.join("relay.log")).unwrap_or_default());
        }
        assert!(began.elapsed() < Duration::from_secs(30), "no banner from khor relay");
        std::thread::sleep(Duration::from_millis(50));
    }
    Running { child, out }
}

/// The relay this endpoint is homed on now, as its own address says.
fn home(ep: &iroh::Endpoint) -> Option<String> {
    ep.addr().addrs.iter().find_map(|a| match a {
        iroh::TransportAddr::Relay(u) => Some(u.to_string().trim_end_matches('/').to_owned()),
        _ => None,
    })
}

/// Waits until `ep` is homed on `want` and connected there, telling iroh
/// the network changed at the start and every 10 s after — not more
/// often: a change restarts the report under way, and one a second never
/// lets a report with a TLS probe in it finish.
async fn until_home(ep: &iroh::Endpoint, want: &str, within: Duration) -> Duration {
    use iroh::Watcher as _;
    let began = Instant::now();
    ep.network_change().await;
    let mut nudged = Instant::now();
    loop {
        let connected = ep.home_relay_status().get().iter().any(|s| s.is_connected());
        if home(ep).as_deref() == Some(want) && connected {
            return began.elapsed();
        }
        assert!(
            began.elapsed() < within,
            "not homed on {want} within {within:?}: home {:?}",
            home(ep)
        );
        if nudged.elapsed() > Duration::from_secs(10) {
            ep.network_change().await;
            nudged = Instant::now();
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// An endpoint answering one echo per connection.
fn answer_echoes(ep: &iroh::Endpoint) {
    let server = ep.clone();
    tokio::spawn(async move {
        while let Some(incoming) = server.accept().await {
            let Ok(conn) = incoming.await else { continue };
            tokio::spawn(async move {
                if let Ok((mut send, mut recv)) = conn.accept_bi().await {
                    let got = recv.read_to_end(1024).await.unwrap_or_default();
                    let _ = send.write_all(&got).await;
                    let _ = send.finish();
                    let _ = conn.closed().await;
                }
            });
        }
    });
}

/// One echo from `from` to `to` through `to`'s relay at `url`.
async fn echo(from: &iroh::Endpoint, to: &iroh::Endpoint, url: &str) -> Result<(), String> {
    let addr = endpoint::dial_addr(&to.addr().id.to_string(), &[], &[url.to_string()]).map_err(|e| e.to_string())?;
    let conn = tokio::time::timeout(Duration::from_secs(10), from.connect(addr, endpoint::ALPN))
        .await
        .map_err(|_| "dial timed out".to_string())?
        .map_err(|e| e.to_string())?;
    let (mut send, mut recv) = conn.open_bi().await.map_err(|e| e.to_string())?;
    send.write_all(b"over https").await.map_err(|e| e.to_string())?;
    send.finish().map_err(|e| e.to_string())?;
    let back = recv.read_to_end(1024).await.map_err(|e| e.to_string())?;
    conn.close(0u32.into(), b"done");
    (back == b"over https").then_some(()).ok_or_else(|| format!("came back as {back:?}"))
}

/// A relay-only endpoint of this build with `relays` and their successors.
async fn bind(relays: &[Relay]) -> iroh::Endpoint {
    // SAFETY: tests here set the environment before any endpoint reads it
    // and nothing else in this binary writes it meanwhile.
    unsafe { std::env::set_var("KHOR_RELAY_ONLY", "1") };
    let ep = endpoint::bind_preferring(iroh::SecretKey::generate(), relays, false, None).await.unwrap();
    answer_echoes(&ep);
    ep
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_tls_relay_takes_over_renews_in_place_and_gives_way_when_it_stops() {
    for k in PROXY_VARS {
        // SAFETY: as in `bind`.
        unsafe { std::env::remove_var(k) };
    }
    let s = scratch("client");
    let (cert, key) = self_signed(&s.0, "first", &["127.0.0.1"]);
    // SAFETY: as in `bind`.
    unsafe { std::env::set_var(endpoint::RELAY_CA_ENV, &cert) };

    let (plain_home, tls_home) = (s.0.join("plain"), s.0.join("tls"));
    std::fs::create_dir_all(&plain_home).unwrap();
    std::fs::create_dir_all(&tls_home).unwrap();
    let plain_port = free_port();
    let _plain = relay(&plain_home, &["--dev", "--bind", &format!("127.0.0.1:{plain_port}"), "--token", "plain-key"]);
    let plain = format!("http://127.0.0.1:{plain_port}");
    let tls_port = free_port();
    let tls = format!("https://127.0.0.1:{tls_port}");
    // SAFETY: as in `bind`.
    unsafe { std::env::set_var(endpoint::RELAY_SUCCESSOR_ENV, format!("{plain}|{tls}")) };
    let relays = [
        Relay { url: plain.clone(), token: Some("plain-key".into()) },
        Relay { url: tls.clone(), token: Some("tls-key".into()) },
    ];

    // The TLS relay not up yet: home on the plain one, as before.
    let near = bind(&relays).await;
    let far = bind(&relays).await;
    until_home(&near, &plain, Duration::from_secs(30)).await;
    until_home(&far, &plain, Duration::from_secs(30)).await;
    // An older khor, which knows only the plain relay.
    let old = {
        // SAFETY: as in `bind`.
        unsafe { std::env::remove_var(endpoint::RELAY_SUCCESSOR_ENV) };
        let ep = bind(&relays[..1]).await;
        unsafe { std::env::set_var(endpoint::RELAY_SUCCESSOR_ENV, format!("{plain}|{tls}")) };
        ep
    };
    until_home(&old, &plain, Duration::from_secs(30)).await;

    // The TLS relay starts answering: both move, on their own.
    let started = Instant::now();
    let tls_relay = relay(
        &tls_home,
        &[
            "--tls-cert",
            cert.to_str().unwrap(),
            "--tls-key",
            key.to_str().unwrap(),
            "--bind",
            &format!("127.0.0.1:{tls_port}"),
            "--http",
            "127.0.0.1:0",
            "--token",
            "tls-key",
        ],
    );
    let moved = until_home(&near, &tls, Duration::from_secs(60)).await;
    until_home(&far, &tls, Duration::from_secs(60)).await;
    eprintln!("[https] moved to the TLS relay {moved:?} after it started ({:?} with its start)", started.elapsed());
    assert_eq!(echo(&near, &far, &tls).await, Ok(()), "two of this build meet on the TLS relay");
    assert_eq!(echo(&near, &old, &plain).await, Ok(()), "and reach an older khor where it is");
    // The other way is not asserted: an older khor holds no key for the
    // TLS relay, and reaches a machine homed there only through the plain
    // relay while that machine is still connected to it — which it was
    // here, having just dialled the older one. Measured once, not relied
    // on: audit/net-7.md has the owner give every machine this build
    // before the TLS relay is switched on.

    // Renewal: new files, SIGHUP, no restart; the wires stay.
    let pid = tls_relay.child.id();
    let (cert2, key2) = self_signed(&s.0, "second", &["127.0.0.1"]);
    std::fs::copy(&cert2, &cert).unwrap();
    std::fs::copy(&key2, &key).unwrap();
    assert!(Command::new("kill").args(["-HUP", &pid.to_string()]).status().unwrap().success());
    let began = Instant::now();
    while !std::fs::read_to_string(&tls_relay.out).unwrap_or_default().contains(&cert.display().to_string()) {
        assert!(began.elapsed() < Duration::from_secs(10), "the relay never said it read the certificate again");
        std::thread::sleep(Duration::from_millis(50));
    }
    // What the relay serves now is the second certificate: an endpoint
    // trusting only that one gets in.
    // SAFETY: as in `bind`.
    unsafe { std::env::set_var(endpoint::RELAY_CA_ENV, &cert2) };
    let fresh = bind(&relays).await;
    until_home(&fresh, &tls, Duration::from_secs(30)).await;
    assert_eq!(echo(&near, &fresh, &tls).await, Ok(()), "a wire made after the renewal");
    assert_eq!(echo(&far, &near, &tls).await, Ok(()), "and the ones from before it");
    assert_eq!(tls_relay.child.id(), pid);

    // The TLS relay stops: back to the plain one.
    drop(tls_relay);
    let back = until_home(&near, &plain, Duration::from_secs(60)).await;
    until_home(&far, &plain, Duration::from_secs(60)).await;
    assert_eq!(echo(&near, &far, &plain).await, Ok(()), "and they meet there again");
    eprintln!("[https] back on the plain relay {back:?} after the TLS one stopped");
    for ep in [near, far, old, fresh] {
        ep.close().await;
    }
}

/// The script's key is the one compiled into khor beside the Beijing TLS
/// relay: a mismatch would be a relay nobody of this build can enter.
#[test]
fn the_script_admits_by_the_key_khor_carries() {
    let script = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../scripts/relay-https.sh")).unwrap();
    let carried = endpoint::KHOR_RELAYS
        .iter()
        .find(|(url, _)| *url == endpoint::BEIJING_TLS)
        .and_then(|(_, key)| *key)
        .unwrap();
    assert!(script.contains(&format!("TOKEN=\"{carried}\"")), "the script's TOKEN is not khor's");
    let (domain, port) = endpoint::BEIJING_TLS.trim_start_matches("https://").split_once(':').unwrap();
    assert!(script.contains(&format!("KHOR_RH_DOMAIN:-{domain}")), "the script's domain is not khor's");
    assert!(script.contains(&format!("KHOR_RH_PORT:-{port}")), "the script's port is not khor's");
}

/// A stand-in `acme.sh`: `--install-cert` copies the certificate it holds
/// to the paths given and runs the reload command, remembering both, as
/// acme.sh does; `--cron` (a renewal) copies again and runs it again.
const FAKE_ACME: &str = r#"#!/bin/bash
set -e
src="$FAKE_ACME_SRC"
if [ "$1" = "--cron" ]; then
	. "$src/installed"
else
	while [ $# -gt 0 ]; do
		case "$1" in
		--fullchain-file) chain="$2"; shift ;;
		--key-file) key="$2"; shift ;;
		--reloadcmd) reload="$2"; shift ;;
		esac
		shift
	done
	printf 'chain=%q\nkey=%q\nreload=%q\n' "$chain" "$key" "$reload" >"$src/installed"
fi
cp "$src/fullchain.pem" "$chain"
cp "$src/key.pem" "$key"
bash -c "$reload"
"#;

/// A stand-in `systemctl` for one unit: `restart` runs its `ExecStart`
/// with its environment file, `is-active` asks that process,
/// `try-reload-or-restart` sends it `ExecReload`'s `SIGHUP`.
const FAKE_SYSTEMCTL: &str = r#"#!/bin/bash
unit="$FAKE_ROOT/etc/systemd/system/khor-relay-https.service"
pidf="$FAKE_ROOT/relay.pid"
alive() { [ -f "$pidf" ] && kill -0 "$(cat "$pidf")" 2>/dev/null; }
case "$1" in
daemon-reload|enable) ;;
is-active) [ "${@: -1}" = "khor-relay-https.service" ] && alive ;;
restart)
	alive && kill "$(cat "$pidf")" && sleep 0.3
	set -a; . "$(sed -n 's/^EnvironmentFile=//p' "$unit")"; set +a
	cmd="$(sed -n 's/^ExecStart=//p' "$unit")"
	cmd="${cmd//\$\{KHOR_RELAY_TOKEN\}/$KHOR_RELAY_TOKEN}"
	nohup $cmd >>"$FAKE_ROOT/relay.out" 2>>"$FAKE_ROOT/relay.log" &
	echo $! >"$pidf"
	;;
reload) alive && kill -HUP "$(cat "$pidf")" ;;
*) echo "fake systemctl: $*" >&2; exit 1 ;;
esac
"#;

fn run_script(root: &Path, env: &[(&str, String)]) -> (bool, String) {
    let out = Command::new("bash")
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/../../scripts/relay-https.sh"))
        .envs(env.iter().map(|(k, v)| (*k, v.as_str())))
        .env("PATH", format!("{}:{}", root.join("bin").display(), std::env::var("PATH").unwrap()))
        .output()
        .unwrap();
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    (out.status.success(), text)
}

fn curl_ping(ca: &Path, domain: &str, port: u16) -> bool {
    Command::new("curl")
        .args(["-fsS", "--max-time", "3", "--cacert"])
        .arg(ca)
        .args(["--resolve", &format!("{domain}:{port}:127.0.0.1"), &format!("https://{domain}:{port}/ping")])
        .env_remove("https_proxy")
        .env_remove("HTTPS_PROXY")
        .env_remove("all_proxy")
        .env_remove("ALL_PROXY")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
fn the_setup_script_installs_renews_in_place_and_runs_twice_without_change() {
    let s = scratch("script");
    let root = s.0.join("root");
    let (bin, src) = (root.join("bin"), s.0.join("acme"));
    for d in [&bin, &src] {
        std::fs::create_dir_all(d).unwrap();
    }
    let write_exe = |p: PathBuf, text: &str| {
        std::fs::write(&p, text).unwrap();
        assert!(Command::new("chmod").arg("+x").arg(&p).status().unwrap().success());
    };
    write_exe(bin.join("acme.sh"), FAKE_ACME);
    write_exe(bin.join("systemctl"), FAKE_SYSTEMCTL);
    // The khor the unit runs: a copy, so a rebuild under the test does not
    // restart it (a resident whose binary changes restarts by design).
    let khor = root.join("usr/local/bin/khor");
    std::fs::create_dir_all(khor.parent().unwrap()).unwrap();
    std::fs::copy(env!("CARGO_BIN_EXE_khor"), &khor).unwrap();

    let domain = "bj.relay.test";
    let (cert, key) = self_signed(&s.0, "issued", &[domain]);
    std::fs::copy(&cert, src.join("fullchain.pem")).unwrap();
    std::fs::copy(&key, src.join("key.pem")).unwrap();
    let port = free_port();
    let user = String::from_utf8(Command::new("id").arg("-un").output().unwrap().stdout).unwrap().trim().to_owned();
    let group = String::from_utf8(Command::new("id").arg("-gn").output().unwrap().stdout).unwrap().trim().to_owned();
    let env = [
        ("KHOR_RH_ROOT", root.display().to_string()),
        ("KHOR_RH_DOMAIN", domain.to_owned()),
        ("KHOR_RH_PORT", port.to_string()),
        ("KHOR_RH_BIND", "127.0.0.1".to_owned()),
        ("KHOR_RH_CAPTIVE", "127.0.0.1:0".to_owned()),
        ("KHOR_RH_USER", user),
        ("KHOR_RH_GROUP", group),
        ("KHOR_RH_KHOR", khor.display().to_string()),
        ("KHOR_RH_ACME", bin.join("acme.sh").display().to_string()),
        ("FAKE_ACME_SRC", src.display().to_string()),
        ("FAKE_ROOT", root.display().to_string()),
        ("CURL_CA_BUNDLE", cert.display().to_string()),
        ("KHOR_HOME", s.0.join("home").display().to_string()),
    ];
    let stop = || {
        if let Ok(pid) = std::fs::read_to_string(root.join("relay.pid")) {
            let _ = Command::new("kill").arg(pid.trim()).status();
        }
    };

    let (ok, first) = run_script(&root, &env);
    eprintln!("[script] first run:\n{first}");
    if !ok {
        stop();
        panic!("the first run failed:\n{first}\n{}", std::fs::read_to_string(root.join("relay.log")).unwrap_or_default());
    }
    assert!(first.contains("在本机答了"), "the relay answered on this machine");
    let pid = std::fs::read_to_string(root.join("relay.pid")).unwrap();
    assert!(curl_ping(&cert, domain, port), "and answers a client that trusts the certificate");

    let (ok, second) = run_script(&root, &env);
    eprintln!("[script] second run:\n{second}");
    assert!(ok && second.contains("什么都没改"), "a second run changes nothing:\n{second}");
    assert_eq!(std::fs::read_to_string(root.join("relay.pid")).unwrap(), pid, "nor restarts the relay");

    // A renewal, as acme.sh's cron runs one: new files, the hook, no restart.
    let (cert2, key2) = self_signed(&s.0, "renewed", &[domain]);
    std::fs::copy(&cert2, src.join("fullchain.pem")).unwrap();
    std::fs::copy(&key2, src.join("key.pem")).unwrap();
    let renew = Command::new(bin.join("acme.sh"))
        .arg("--cron")
        .envs(env.iter().map(|(k, v)| (*k, v.as_str())))
        .env("PATH", format!("{}:{}", bin.display(), std::env::var("PATH").unwrap()))
        .output()
        .unwrap();
    let said = format!("{}{}", String::from_utf8_lossy(&renew.stdout), String::from_utf8_lossy(&renew.stderr));
    let began = Instant::now();
    while !curl_ping(&cert2, domain, port) {
        if began.elapsed() > Duration::from_secs(10) {
            stop();
            panic!("the renewed certificate is not served: {said}\n{}", std::fs::read_to_string(root.join("relay.out")).unwrap_or_default());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(!curl_ping(&cert, domain, port), "and the old one no longer is");
    assert_eq!(std::fs::read_to_string(root.join("relay.pid")).unwrap(), pid, "without a restart");
    let unit = std::fs::read_to_string(root.join("etc/systemd/system/khor-relay-https.service")).unwrap();
    assert!(unit.contains("ExecReload=/bin/kill -HUP $MAINPID"), "{unit}");
    stop();
}
