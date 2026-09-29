//! The roster is the one source of trust (docs/CONNECT.md 身份, 分期 2):
//! three real machines-in-miniature — three `khor serve` processes on
//! three temporary stores, driven through the `khor` binary the way a
//! person would.
//!
//! alpha is the first machine, so it makes the root. beta joins through
//! alpha's ticket and is signed on the spot. gamma joins through beta,
//! which does not hold the root, so gamma waits: every machine refuses
//! it until alpha approves it. Then alpha revokes beta, and beta is
//! refused. Each "refused" is checked against its control — the same
//! request, from a machine that is on the roster, going through.
//!
//! # What this test does not touch
//!
//! No real relay and no real store: every store is a temporary
//! directory that is also `HOME`, and each one's relay list is a single
//! loopback address nothing listens on, marked as the whole list
//! (`only_mine`) — so the built-in relay and n0's are never dialled.
//! The machines reach each other directly over loopback, which is what
//! the tickets and the device table carry.

use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use khor_catalog::{cli, msg};

/// A scratch tree that takes itself away however the test leaves (the
/// reasoning is on `crates/node/src/tests.rs::Scratch`).
struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// One machine: its store (which is also its `HOME`) and its name.
struct Machine {
    home: Scratch,
    name: &'static str,
}

fn machine(name: &'static str) -> Machine {
    let p = std::env::temp_dir().join(format!("khor-roster-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(p.join(".khor")).unwrap();
    // The whole relay list: one loopback port nobody listens on. Nothing
    // outside this machine is dialled.
    fs::write(
        p.join(".khor").join("relays.json"),
        r#"{"relays":["http://127.0.0.1:9"],"only_mine":true}"#,
    )
    .unwrap();
    Machine {
        home: Scratch(p),
        name,
    }
}

impl Machine {
    fn command(&self) -> Command {
        let mut c = Command::new(env!("CARGO_BIN_EXE_khor"));
        c.env("KHOR_HOME", &self.home.0)
            .env("HOME", &self.home.0)
            .env("KHOR_NAME", self.name)
            .env("KHOR_WEB_PORT", "0")
            // The words asserted below are the test process's, which
            // speaks the catalog's source language; so do the machines.
            .env("LC_ALL", "zh_CN.UTF-8")
            .env("LANG", "zh_CN.UTF-8")
            .env_remove("KHOR_SESSION")
            .env_remove("KHOR_RELAY");
        c
    }

    /// One CLI verb, under a watchdog: a hang is a failure with a name,
    /// not a test that never ends. Answers (success, stdout, stderr).
    fn run(&self, args: &[&str]) -> (bool, String, String) {
        let mut child = self
            .command()
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        // Generous: the first network touch of a fresh binary on macOS
        // can stall tens of seconds (issue #73, `tests/pairing.rs`).
        let deadline = Instant::now() + Duration::from_secs(120);
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                let out = child.wait_with_output().unwrap();
                return (
                    status.success(),
                    String::from_utf8_lossy(&out.stdout).into_owned(),
                    String::from_utf8_lossy(&out.stderr).into_owned(),
                );
            }
            if Instant::now() > deadline {
                let _ = Command::new("kill")
                    .args(["-KILL", &child.id().to_string()])
                    .status();
                panic!("{} khor {args:?} did not finish in 120s", self.name);
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    fn ok(&self, args: &[&str]) -> String {
        let (ok, out, err) = self.run(args);
        assert!(ok, "{}: khor {args:?} failed: {err}", self.name);
        out
    }

    fn state(&self) -> serde_json::Value {
        serde_json::from_str(&self.ok(&["root", "show", "--json"])).unwrap()
    }

    fn pending(&self) -> Vec<serde_json::Value> {
        serde_json::from_str::<serde_json::Value>(&self.ok(&["pending", "--json"]))
            .unwrap()
            .as_array()
            .unwrap()
            .clone()
    }

    /// `khor sync`'s line about one machine.
    fn sync_line(&self, about: &str) -> String {
        let out = self.ok(&["sync"]);
        // A success line is `name: what`; a failure is the catalog's
        // `sync-failed-line`, whose colon is the language's own.
        let failed = cli::sync_failed_line(about, '\u{0}');
        let failed = failed.split('\u{0}').next().unwrap();
        out.lines()
            .find(|l| l.starts_with(&format!("{about}:")) || l.starts_with(failed))
            .unwrap_or_else(|| panic!("{}: no line about {about} in: {out}", self.name))
            .to_owned()
    }

    fn serve(&self) -> Keeper {
        let log = fs::File::create(self.home.0.join("serve.log")).unwrap();
        let k = Keeper(
            self.command()
                .arg("serve")
                .stderr(log)
                .stdout(Stdio::null())
                .spawn()
                .unwrap(),
        );
        let file = self.home.0.join(".khor").join("endpoint.json");
        let deadline = Instant::now() + Duration::from_secs(120);
        while !file.exists() {
            assert!(
                Instant::now() < deadline,
                "{}'s serve did not stand up: {}",
                self.name,
                fs::read_to_string(self.home.0.join("serve.log")).unwrap_or_default()
            );
            std::thread::sleep(Duration::from_millis(100));
        }
        k
    }

    fn id(&self) -> String {
        self.ok(&["id"]).split_whitespace().next().unwrap().to_owned()
    }

    fn has(&self, file: &str) -> bool {
        self.home.0.join(".khor").join(file).exists()
    }
}

/// Stops a keeper by its pid however the test leaves; TERM reaches its
/// serve through the keeper's forwarding (`tests/two_serves.rs`).
struct Keeper(Child);

impl Drop for Keeper {
    fn drop(&mut self) {
        let _ = Command::new("kill")
            .args(["-TERM", &self.0.id().to_string()])
            .status();
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            if self.0.try_wait().ok().flatten().is_some() {
                return;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn refused(about: &str) -> String {
    cli::sync_failed_line(about, msg::not_in_roster())
}

#[test]
fn the_roster_decides_who_is_in_across_three_machines() {
    let (a, b, c) = (machine("alpha"), machine("beta"), machine("gamma"));
    let _ka = a.serve();
    let _kb = b.serve();
    let _kc = c.serve();

    // Each started alone, so each made a root of its own.
    let root = a.state()["root"].as_str().unwrap().to_owned();
    assert!(a.state()["holds"].as_bool().unwrap(), "alpha holds its root");
    assert_ne!(b.state()["root"].as_str().unwrap(), root);

    // ── alpha ↔ beta: alpha holds the root, so beta is signed on the spot.
    let ticket = a.ok(&["invite"]);
    b.ok(&["pair", ticket.trim()]);
    let sb = b.state();
    assert_eq!(sb["root"].as_str(), Some(root.as_str()), "beta took alpha's root");
    assert_eq!(sb["holds"].as_bool(), Some(false), "and gave up its own");
    assert!(!b.has("root.key"), "its own key is gone");
    assert_eq!(sb["standing"].as_str(), Some("trusted"), "beta has its certificate");
    assert!(b.pending().is_empty());
    // Control for every refusal below: beta, on the roster, syncs.
    let line = b.sync_line("alpha");
    assert_ne!(line, refused("alpha"), "a signed machine syncs: {line}");
    assert!(!line.contains(msg::not_in_roster()), "{line}");

    // ── beta ↔ gamma: beta does not hold the root, so gamma waits.
    let ticket = b.ok(&["invite"]);
    let (ok, _, hint) = c.run(&["pair", ticket.trim()]);
    assert!(ok, "the pairing itself goes through: {hint}");
    assert!(hint.contains(cli::pair_pending_hint()), "and says it waits: {hint}");
    let sc = c.state();
    assert_eq!(sc["root"].as_str(), Some(root.as_str()), "gamma took the network's root");
    assert_eq!(sc["standing"].as_str(), Some("pending"));
    let gamma = c.id();
    for (who, m) in [("beta", &b), ("gamma", &c)] {
        assert!(
            m.pending().iter().any(|r| r["id"] == gamma.as_str()),
            "{who} notes gamma as waiting"
        );
    }
    // beta carries gamma's row to alpha (until then alpha has never
    // heard of gamma and says so — `not_paired`, the other refusal).
    b.ok(&["sync"]);
    // gamma's sync is refused by both, by name: paired, not on the roster.
    let out = c.ok(&["sync"]);
    assert!(out.contains(&refused("alpha")), "alpha refuses gamma: {out}");
    assert!(out.contains(&refused("beta")), "beta refuses gamma: {out}");
    // Nobody but alpha can let it in.
    let (ok, _, err) = b.run(&["approve", "gamma"]);
    assert!(!ok && err.contains(msg::approve_needs_root()), "{err}");

    // ── alpha approves gamma, which it learned of from beta's table.
    assert!(
        a.pending().iter().any(|r| r["id"] == gamma.as_str()),
        "alpha sees gamma waiting"
    );
    let said = a.ok(&["approve", "gamma"]);
    assert!(said.contains("gamma"), "{said}");
    // approve pushed the roster; gamma is in on both.
    let line = c.sync_line("alpha");
    assert!(!line.contains(msg::not_in_roster()), "alpha answers gamma now: {line}");
    let line = c.sync_line("beta");
    assert!(!line.contains(msg::not_in_roster()), "and so does beta: {line}");
    assert_eq!(c.state()["standing"].as_str(), Some("trusted"));
    assert!(a.pending().is_empty() && c.pending().is_empty());

    // ── alpha revokes beta: refused from now on.
    let said = a.ok(&["revoke", "beta"]);
    assert!(said.contains("beta"), "{said}");
    assert_eq!(b.sync_line("alpha"), refused("alpha"), "beta is refused by name");
    // Control: gamma, still on the roster, still goes through.
    let line = c.sync_line("alpha");
    assert!(!line.contains(msg::not_in_roster()), "{line}");
    // …and gamma, which has the new roster, no longer syncs with beta.
    assert_eq!(
        c.sync_line("beta"),
        cli::sync_failed_line("beta", msg::peer_not_in_roster())
    );
}
