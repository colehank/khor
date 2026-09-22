//! What a vendor subscription on this machine has left — **one
//! abstraction, a description per vendor, and a chain of ways to read
//! it** (docs/SHELL.md 第九轮).
//!
//! # The shape, and where it comes from
//!
//! Modelled on [CodexBar](https://github.com/steipete/CodexBar) (MIT,
//! Swift): each vendor is **one description** — its name, whether this
//! machine is logged in, the plan the login names — plus **a chain of
//! strategies in priority order**, each of which says whether it can run
//! here, fetches, and on failure hands on to the next; every attempt is
//! kept for diagnosis (`khor quota --why`). All vendors answer in **one
//! snapshot**, [`khor_core::Quota`]. The shape is CodexBar's; the code is
//! written here in Rust and none of it is theirs.
//!
//! Two kinds of strategy, and the difference is the whole of the rules:
//!
//! - **Local** — a file on this disk another tool wrote (Claude Code's
//!   statusline hook, Codex's session records). No network; used while
//!   it is fresh, skipped otherwise.
//! - **Remote** — the vendor's usage endpoint, reached with the login
//!   the vendor's own tool stored. **A non-public endpoint reached with
//!   a credential stored for something else**, and the user ruled on
//!   exactly that (2026-08-21), so the rules below are this layer's and
//!   every vendor keeps them without restating them:
//!   - **said out loud** — a number is shown with whose login it came
//!     through (`crates/catalog`);
//!   - **nothing trusted about the shape** — every field optional, a
//!     shape this build cannot read is 「读不了」, never a zero and
//!     never a panic;
//!   - **the rate-limit discipline is the feature**: five minutes in
//!     memory **and** a disk cache every process on the machine shares
//!     (`khor serve`, the desktop and the CLI would otherwise ask one
//!     account three times over), a ten-minute cooldown after a
//!     failure during which nothing leaves at all, and **no automatic
//!     token refresh** — an expired login is the person's to renew by
//!     running the vendor's own tool, not khor's to mint;
//!   - **a test door per vendor** (`KHOR_CLAUDE_USAGE_URL`,
//!     `KHOR_CODEX_USAGE_URL`) and **a tripwire per vendor** that stops
//!     a unit test before it reads the operator's login.
//!
//! Browser-cookie strategies are deliberately absent (privacy), so a
//! vendor whose only road is a cookie (Cursor) is not here.

pub mod claude;
pub mod codex;

use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Mutex;

use khor_core::{Quota, QuotaTrouble, QuotaWindow};
use serde_json::Value;

pub use claude::{logged_in, read};

/// How long one fetch answers for. Opening and closing a panel must not
/// be a reason to touch a non-public endpoint again.
pub const CACHE_TTL: i64 = 300;
/// How long khor stays away after a failure.
///
/// **Ten minutes because the failure being guarded against is 429**, and
/// continuing to knock while rate limited is how a limit gets extended
/// rather than waited out. During the cooldown a stale reading is served
/// if there is one and the wait is stated if there is not; either way no
/// request leaves.
pub const FAIL_COOLDOWN: i64 = 600;

/// What a strategy's fetch hands back.
pub type Fetched = Pin<Box<dyn Future<Output = Result<Quota, QuotaTrouble>> + Send>>;

/// Where a strategy reads from — which decides which rules wrap it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// A file on this disk; no network, no cache, no cooldown.
    Local,
    /// The vendor's endpoint; behind this layer's cache and cooldown.
    Remote,
}

/// One way to read a vendor, in its place in the chain.
pub struct Strategy {
    /// The name `khor quota --why` prints — an identifier, not prose.
    pub name: &'static str,
    pub step: Step,
    /// Whether it can run here at all (a login to use, a file to read).
    /// Asked before `fetch` and **without the network**.
    pub available: Box<dyn Fn() -> bool + Send + Sync>,
    pub fetch: Box<dyn Fn() -> Fetched + Send + Sync>,
}

/// One vendor, described.
pub struct Provider {
    /// The account provider word it answers for (`claude`, `codex`) —
    /// also the name of its disk cache and its line in `--why`.
    pub id: &'static str,
    /// What the vendor calls the tool whose login is read (「Claude
    /// Code」, 「Codex」) — a proper noun, the `{0}` of the words that
    /// say whose login a number came through.
    pub vendor: &'static str,
    /// The command that logs the vendor's tool in again (`claude`,
    /// `codex`) — what a person is sent to run when the login is
    /// missing or expired, since khor never renews it.
    pub command: &'static str,
    /// Whether this machine holds the vendor's login — **no network**.
    pub logged_in: fn() -> bool,
    /// The strategies, in the order they are tried.
    pub strategies: fn() -> Vec<Strategy>,
    /// The last touch on a reading, after any cache: facts about the
    /// login rather than about the reading (the plan), read fresh every
    /// time so a change shows at once instead of after five minutes.
    pub finish: fn(&mut Quota),
    /// This vendor's memory: cache, cooldown, last attempts.
    pub state: &'static State,
}

/// Every vendor this build can read, in the order a list shows them.
pub fn providers() -> [&'static Provider; 2] {
    [&claude::PROVIDER, &codex::PROVIDER]
}

/// The vendor that answers for an account provider word.
pub fn provider(id: &str) -> Option<&'static Provider> {
    providers().into_iter().find(|p| p.id == id)
}

/// A vendor's memory between reads.
pub struct State {
    mem: Mutex<Option<(i64, Quota)>>,
    failed_at: Mutex<Option<i64>>,
    last: Mutex<Vec<Attempt>>,
}

impl State {
    pub const fn new() -> Self {
        State {
            mem: Mutex::new(None),
            failed_at: Mutex::new(None),
            last: Mutex::new(Vec::new()),
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

/// What one strategy did on one read — a cause, never a sentence
/// (`khor quota --why` says it in the catalog's words).
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// It cannot run here (no login, no file).
    Unavailable,
    /// It answered, and its answer is the reading.
    Used,
    /// A remote strategy not asked: the machine's cached reading, this
    /// many seconds old, answered instead.
    Cached { age: i64 },
    /// A remote strategy not asked: cooling after a failure.
    Cooling { minutes: i64 },
    /// It was tried and did not answer.
    Failed(QuotaTrouble),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Attempt {
    pub strategy: &'static str,
    pub outcome: Outcome,
}

impl Provider {
    /// This vendor's reading, down the chain.
    pub async fn read(&'static self) -> Result<Quota, QuotaTrouble> {
        let cache = disk_cache_path(self.id);
        let now = jiff::Timestamp::now().as_second();
        let (read, attempts) = run(&(self.strategies)(), self.state, &cache, now).await;
        *self.state.last.lock().unwrap() = attempts;
        let mut quota = read?;
        (self.finish)(&mut quota);
        // Beside the windows it is a function of, and after any cache:
        // computed where they are is what makes it impossible for the
        // number and the windows to disagree.
        quota.fill = whole_plan_fill(&quota.windows);
        Ok(quota)
    }

    /// What the last read tried, strategy by strategy.
    pub fn last_attempts(&self) -> Vec<Attempt> {
        self.state.last.lock().unwrap().clone()
    }
}

/// The chain itself, with every rule of this layer and nothing of any
/// vendor's — **the seam the rules are tested at** (`state` and `cache`
/// handed in, so a test owns both).
///
/// Locals first, in order, each used only if it answers. The first
/// remote is where the cache and the cooldown sit: a fresh cached
/// reading answers for every remote in the chain, and a cooldown keeps
/// all of them from leaving. A remote that fails hands on to the next;
/// the last failure is the answer when nothing is cached.
pub async fn run(
    chain: &[Strategy],
    state: &State,
    cache: &Path,
    now: i64,
) -> (Result<Quota, QuotaTrouble>, Vec<Attempt>) {
    let mut attempts = Vec::new();
    let mut gate_checked = false;
    let mut cached: Option<(i64, Quota)> = None;
    let mut last = QuotaTrouble::NoLogin;
    for s in chain {
        if !(s.available)() {
            attempts.push(Attempt {
                strategy: s.name,
                outcome: Outcome::Unavailable,
            });
            continue;
        }
        if s.step == Step::Local {
            match (s.fetch)().await {
                Ok(mut quota) => {
                    quota.source = Some(s.name.to_owned());
                    attempts.push(Attempt {
                        strategy: s.name,
                        outcome: Outcome::Used,
                    });
                    return (Ok(quota), attempts);
                }
                Err(why) => attempts.push(Attempt {
                    strategy: s.name,
                    outcome: Outcome::Failed(why),
                }),
            }
            continue;
        }
        if !gate_checked {
            gate_checked = true;
            // Memory alone is not enough: three processes, three memory
            // caches, and any one of them restarting would otherwise go
            // straight out. The disk cache makes a machine's fetches one.
            cached = state
                .mem
                .lock()
                .unwrap()
                .clone()
                .or_else(|| read_cache_at(cache, now));
            if let Some((taken, quota)) = &cached
                && now - taken < CACHE_TTL
            {
                attempts.push(Attempt {
                    strategy: s.name,
                    outcome: Outcome::Cached { age: now - taken },
                });
                return (Ok(stamp(quota.clone(), *taken)), attempts);
            }
            if let Some(failed_at) = *state.failed_at.lock().unwrap() {
                let waited = now - failed_at;
                if waited < FAIL_COOLDOWN {
                    let minutes = (FAIL_COOLDOWN - waited) / 60 + 1;
                    attempts.push(Attempt {
                        strategy: s.name,
                        outcome: Outcome::Cooling { minutes },
                    });
                    return (
                        match cached {
                            Some((taken, quota)) => Ok(stamp(quota, taken)),
                            None => Err(QuotaTrouble::Cooling { minutes }),
                        },
                        attempts,
                    );
                }
            }
        }
        match (s.fetch)().await {
            Ok(mut quota) => {
                quota.source = Some(s.name.to_owned());
                *state.mem.lock().unwrap() = Some((now, quota.clone()));
                *state.failed_at.lock().unwrap() = None;
                write_cache_at(cache, now, &quota);
                attempts.push(Attempt {
                    strategy: s.name,
                    outcome: Outcome::Used,
                });
                return (Ok(quota), attempts);
            }
            Err(why) => {
                // A refused login is not a rate limit, and cooling off
                // would not fix it — the person has to run the vendor's
                // tool. Only the failures time might heal cool.
                if !matches!(why, QuotaTrouble::NoLogin | QuotaTrouble::Stale) {
                    *state.failed_at.lock().unwrap() = Some(now);
                }
                attempts.push(Attempt {
                    strategy: s.name,
                    outcome: Outcome::Failed(why.clone()),
                });
                last = why;
            }
        }
    }
    let answer = match cached {
        Some((taken, quota)) if gate_checked => Ok(stamp(quota, taken)),
        _ => Err(last),
    };
    (answer, attempts)
}

/// How full the subscription is **as a whole**, `0..=1`, or `None` when
/// no window speaks for the whole plan.
///
/// 1. The **longest unscoped** window — the week, today.
/// 2. No week, but a five-hour: that one. Still the whole plan.
/// 3. **No unscoped window at all: `None`** — never a scoped window
///    standing in for the plan, and never a zero.
///
/// Why the whole plan's window and not the fullest: 2026-09-07 the user
/// ruled 「list 页、打开后的多环,claude 显示 week_all」 — one number on a
/// line is about **this subscription**, not one model's slice of it;
/// the slices are pips of their own on the rim. **Length comes from the
/// order the windows arrive in** (each vendor sorts them shortest-first,
/// whole plan before a model's share), so the last unscoped entry is the
/// longest; a second opinion about which span is longer is how two
/// places come to disagree about one order.
pub(crate) fn whole_plan_fill(windows: &[QuotaWindow]) -> Option<f32> {
    let whole = windows.iter().rev().find(|w| w.scope.is_none())?;
    Some((whole.used_pct / 100.0).clamp(0.0, 1.0))
}

/// Mark a reading with when it was taken.
///
/// A cache hit and a fresh fetch are otherwise the same object, and
/// "18%" from ten minutes ago is a different sentence from "18%" now.
fn stamp(quota: Quota, as_of: i64) -> Quota {
    Quota {
        as_of: Some(as_of),
        ..quota
    }
}

/// The cache every process on this machine shares, per vendor.
///
/// Under `KHOR_HOME` rather than the OS cache directory, which is not
/// tidiness: khor already treats that root as "this installation", so
/// two homes on one machine — which is how this repo tests and how a
/// person runs two accounts — get two caches without anything having to
/// know they are different. It is disposable; losing it costs one fetch.
fn disk_cache_path(id: &str) -> PathBuf {
    crate::Node::root_from_env()
        .join(".khor")
        .join(format!("{id}-usage.json"))
}

/// Path injected so tests never touch the real one and never each other's.
fn read_cache_at(path: &Path, now: i64) -> Option<(i64, Quota)> {
    let json: Value = serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()?;
    let taken = json.get("fetched_at")?.as_i64()?;
    // A clock that jumped backwards makes `now - taken` negative, and this
    // entry would then look fresh forever. Treat it as absent.
    if taken > now {
        return None;
    }
    let quota: Quota = serde_json::from_value(json.get("quota")?.clone()).ok()?;
    Some((taken, quota))
}

fn write_cache_at(path: &Path, now: i64, quota: &Quota) {
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    // A failed write costs the next process a fetch; it does not cost
    // this caller its answer, so it is not worth failing over.
    let _ = std::fs::write(
        path,
        serde_json::json!({ "fetched_at": now, "quota": quota }).to_string(),
    );
}

/// A percentage from a number or a string, clamped.
///
/// Clamped rather than rejected: a value outside 0–100 is a misread, and
/// a bar drawn past full is a more confusing answer than a full one.
pub(crate) fn percent(v: &Value) -> Option<f32> {
    let n = match v {
        Value::Number(n) => n.as_f64()?,
        Value::String(s) => s.trim().parse().ok()?,
        _ => return None,
    };
    Some(n.clamp(0.0, 100.0) as f32)
}

/// Unix seconds from an RFC 3339 string, or from seconds or milliseconds.
pub(crate) fn unix_seconds(v: &Value) -> Option<i64> {
    match v {
        Value::String(s) => s.parse::<jiff::Timestamp>().ok().map(|t| t.as_second()),
        Value::Number(n) => {
            let n = n.as_i64()?;
            // Anything past the year 2286 in seconds is milliseconds.
            Some(if n > 10_000_000_000 { n / 1000 } else { n })
        }
        _ => None,
    }
}

/// A reading that has rolled over is empty now, and its reset time is in
/// the past and says nothing about the next one.
pub(crate) fn settle(used_pct: f32, resets_at: Option<i64>, now: i64) -> (f32, Option<i64>) {
    match resets_at {
        Some(at) if at < now => (0.0, None),
        other => (used_pct, other),
    }
}

/// `max` → `Max`. One word, first letter up, and nothing else touched.
pub(crate) fn title_case(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// **The account's home directory decides whose login is read.** A
/// vendor's login lives under `$HOME` (and, for Claude on macOS, in the
/// login keychain too). The keychain does not follow `$HOME`, so a
/// process handed a scratch home — every gate here — could still reach
/// the operator's real login through it (2026-09-22). So the keychain is
/// asked only when `$HOME` *is* the account's home directory (the
/// password database's, not the environment's); and a unit test that
/// would read a login under the operator's own home stops instead.
pub(crate) fn home_is_the_accounts(home: Option<&Path>, account: Option<&Path>) -> bool {
    let (Some(home), Some(account)) = (home, account) else {
        return false;
    };
    let real = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    real(home) == real(account)
}

/// The account's home directory as the password database has it —
/// **not** `$HOME`, which is exactly the thing a test changes.
#[cfg(unix)]
pub(crate) fn account_home() -> Option<PathBuf> {
    use std::ffi::CStr;
    use std::os::unix::ffi::OsStrExt;
    let mut pwd: libc::passwd = unsafe { std::mem::zeroed() };
    let mut buf = vec![0 as libc::c_char; 4096];
    let mut out: *mut libc::passwd = std::ptr::null_mut();
    let rc = unsafe {
        libc::getpwuid_r(
            libc::getuid(),
            &mut pwd,
            buf.as_mut_ptr(),
            buf.len(),
            &mut out,
        )
    };
    if rc != 0 || out.is_null() || pwd.pw_dir.is_null() {
        return None;
    }
    let dir = unsafe { CStr::from_ptr(pwd.pw_dir) };
    Some(PathBuf::from(std::ffi::OsStr::from_bytes(dir.to_bytes())))
}

#[cfg(not(unix))]
pub(crate) fn account_home() -> Option<PathBuf> {
    None
}

/// Whether this process is speaking as the account (its `$HOME` is the
/// account's own).
pub(crate) fn as_the_account() -> bool {
    home_is_the_accounts(std::env::home_dir().as_deref(), account_home().as_deref())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use khor_core::QuotaWindowSpan;

    use super::*;

    fn tmp_cache(tag: &str) -> crate::testdir::TempTree {
        crate::testdir::TempTree::at(&format!("quota-{tag}.json"))
    }

    pub(crate) fn quota_at(used: f32) -> Quota {
        Quota {
            windows: vec![QuotaWindow {
                span: QuotaWindowSpan::FiveHour,
                scope: None,
                used_pct: used,
                resets_at: None,
                active: None,
                severity: None,
            }],
            as_of: None,
            plan: None,
            credits: None,
            fill: None,
            source: None,
            limit_reached: None,
        }
    }

    fn window(span: QuotaWindowSpan, scope: Option<&str>, used_pct: f32) -> QuotaWindow {
        QuotaWindow {
            span,
            scope: scope.map(str::to_owned),
            used_pct,
            resets_at: None,
            active: None,
            severity: None,
        }
    }

    fn rt() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
    }

    /// A strategy that answers what it is told and counts every ask.
    fn counted(
        name: &'static str,
        step: Step,
        available: bool,
        answer: Result<Quota, QuotaTrouble>,
    ) -> (Strategy, Arc<AtomicUsize>) {
        let asked = Arc::new(AtomicUsize::new(0));
        let n = asked.clone();
        (
            Strategy {
                name,
                step,
                available: Box::new(move || available),
                fetch: Box::new(move || {
                    n.fetch_add(1, Ordering::SeqCst);
                    let a = answer.clone();
                    Box::pin(async move { a })
                }),
            },
            asked,
        )
    }

    /// **A fresh local reading answers, and the endpoint is asked 0
    /// times**; a stale one (the local strategy declining) hands on,
    /// and the endpoint is asked once.
    #[test]
    fn a_fresh_local_reading_keeps_the_endpoint_unasked() {
        let cache = tmp_cache("chain-local");
        let state = State::new();
        let (local, _) = counted("local", Step::Local, true, Ok(quota_at(6.0)));
        let (remote, asked) = counted("remote", Step::Remote, true, Ok(quota_at(9.0)));
        let (got, attempts) = rt().block_on(run(&[local, remote], &state, &cache, 1_000));
        assert_eq!(asked.load(Ordering::SeqCst), 0);
        assert_eq!(got.unwrap().source.as_deref(), Some("local"));
        assert_eq!(attempts.len(), 1);

        let (local, _) = counted("local", Step::Local, true, Err(QuotaTrouble::Unreadable));
        let (remote, asked) = counted("remote", Step::Remote, true, Ok(quota_at(9.0)));
        let (got, attempts) = rt().block_on(run(&[local, remote], &state, &cache, 1_000));
        assert_eq!(asked.load(Ordering::SeqCst), 1, "a stale local hands on");
        let got = got.unwrap();
        assert_eq!(got.source.as_deref(), Some("remote"));
        assert!((got.windows[0].used_pct - 9.0).abs() < 1e-6);
        assert_eq!(attempts[1].outcome, Outcome::Used);
    }

    /// Within five minutes the machine's reading answers and nothing
    /// leaves — from memory, and from disk for a process that has none.
    #[test]
    fn a_cached_reading_answers_and_nothing_leaves() {
        let cache = tmp_cache("chain-cache");
        let state = State::new();
        let (remote, asked) = counted("remote", Step::Remote, true, Ok(quota_at(9.0)));
        let _ = rt().block_on(run(&[remote], &state, &cache, 1_000));
        assert_eq!(asked.load(Ordering::SeqCst), 1);
        let (remote, asked) = counted("remote", Step::Remote, true, Ok(quota_at(50.0)));
        let fresh_process = State::new();
        let (got, attempts) = rt().block_on(run(
            &[remote],
            &fresh_process,
            &cache,
            1_000 + CACHE_TTL - 1,
        ));
        assert_eq!(asked.load(Ordering::SeqCst), 0);
        assert_eq!(got.unwrap().as_of, Some(1_000));
        assert_eq!(attempts[0].outcome, Outcome::Cached { age: CACHE_TTL - 1 });
    }

    /// **A failure cools**: the next read inside ten minutes asks 0
    /// times and says it is cooling — it does not draw a zero.
    #[test]
    fn a_failure_cools_and_says_so_rather_than_zero() {
        let cache = tmp_cache("chain-cool");
        let state = State::new();
        let (remote, asked) = counted("remote", Step::Remote, true, Err(QuotaTrouble::Unreachable));
        let (got, _) = rt().block_on(run(&[remote], &state, &cache, 1_000));
        assert_eq!(got.unwrap_err(), QuotaTrouble::Unreachable);
        assert_eq!(asked.load(Ordering::SeqCst), 1);
        let (remote, asked) = counted("remote", Step::Remote, true, Ok(quota_at(9.0)));
        let (got, attempts) = rt().block_on(run(&[remote], &state, &cache, 1_060));
        assert_eq!(asked.load(Ordering::SeqCst), 0, "cooling: nothing leaves");
        assert!(matches!(got, Err(QuotaTrouble::Cooling { .. })), "{got:?}");
        assert!(matches!(attempts[0].outcome, Outcome::Cooling { .. }));
        // An expired login does not cool: running the tool fixes it, and
        // the next read must see that at once.
        let state = State::new();
        let (remote, _) = counted("remote", Step::Remote, true, Err(QuotaTrouble::Stale));
        let _ = rt().block_on(run(&[remote], &state, &tmp_cache("chain-stale"), 1_000));
        assert!(state.failed_at.lock().unwrap().is_none());
    }

    /// A strategy that cannot run here is said and passed, never asked.
    #[test]
    fn an_unavailable_strategy_is_passed_without_being_asked() {
        let cache = tmp_cache("chain-skip");
        let state = State::new();
        let (remote, asked) = counted("remote", Step::Remote, false, Ok(quota_at(9.0)));
        let (got, attempts) = rt().block_on(run(&[remote], &state, &cache, 1_000));
        assert_eq!(asked.load(Ordering::SeqCst), 0);
        assert_eq!(got.unwrap_err(), QuotaTrouble::NoLogin);
        assert_eq!(attempts[0].outcome, Outcome::Unavailable);
    }

    /// **The whole plan's window, not the fullest one and not a model's
    /// slice.** Three distinct numbers and the scoped one the fullest,
    /// so the rules this replaced answer differently here.
    #[test]
    fn the_share_is_the_longest_window_that_covers_the_whole_plan() {
        let windows = vec![
            window(QuotaWindowSpan::FiveHour, None, 4.0),
            window(QuotaWindowSpan::SevenDay, None, 39.0),
            window(QuotaWindowSpan::SevenDay, Some("Fable"), 71.0),
        ];
        assert_eq!(whole_plan_fill(&windows), Some(0.39));
        assert_eq!(whole_plan_fill(&windows[..1]), Some(0.04));
        assert_eq!(whole_plan_fill(&windows[2..]), None);
        assert_eq!(whole_plan_fill(&[]), None);
    }

    #[test]
    fn the_share_stays_within_one() {
        assert_eq!(
            whole_plan_fill(&[window(QuotaWindowSpan::FiveHour, None, 140.0)]),
            Some(1.0)
        );
        assert_eq!(
            whole_plan_fill(&[window(QuotaWindowSpan::FiveHour, None, -3.0)]),
            Some(0.0)
        );
    }

    #[test]
    fn a_fresh_process_reads_the_machines_last_fetch() {
        let now = jiff::Timestamp::now().as_second();
        let path = tmp_cache("restart");
        write_cache_at(&path, now, &quota_at(15.0));
        let (taken, back) = read_cache_at(&path, now).expect("what was just written");
        assert_eq!(taken, now);
        assert!((back.windows[0].used_pct - 15.0).abs() < 1e-6);
    }

    #[test]
    fn a_cache_from_the_future_is_dropped_rather_than_trusted_forever() {
        let now = jiff::Timestamp::now().as_second();
        let path = tmp_cache("future");
        write_cache_at(&path, now + 3600, &quota_at(1.0));
        assert!(read_cache_at(&path, now).is_none());
    }

    #[test]
    fn a_served_reading_carries_when_it_was_taken() {
        assert_eq!(
            stamp(quota_at(1.0), 1_700_000_000).as_of,
            Some(1_700_000_000)
        );
    }

    #[test]
    fn percentages_are_read_leniently_and_clamped() {
        assert_eq!(percent(&serde_json::json!("12.5")), Some(12.5));
        assert_eq!(percent(&serde_json::json!(140)), Some(100.0));
        assert_eq!(percent(&serde_json::json!(-3)), Some(0.0));
        assert_eq!(percent(&serde_json::json!(true)), None);
    }

    #[test]
    fn instants_are_read_as_rfc3339_seconds_or_milliseconds() {
        assert_eq!(
            unix_seconds(&serde_json::json!(1_700_000_000)),
            Some(1_700_000_000)
        );
        assert_eq!(
            unix_seconds(&serde_json::json!(1_700_000_000_000i64)),
            Some(1_700_000_000)
        );
        assert_eq!(
            unix_seconds(&serde_json::json!("2023-11-14T22:13:20Z")),
            Some(1_700_000_000)
        );
        assert_eq!(unix_seconds(&serde_json::json!("not a time")), None);
    }

    /// **A process with a scratch `$HOME` is not the account** — the
    /// keychain is the account's, and that process is not speaking as it.
    #[test]
    fn only_the_accounts_own_home_is_the_account() {
        let real = Path::new("/Users/someone");
        assert!(home_is_the_accounts(Some(real), Some(real)));
        assert!(!home_is_the_accounts(
            Some(Path::new("/tmp/khor-smoke/home")),
            Some(real)
        ));
        assert!(!home_is_the_accounts(None, Some(real)));
        assert!(!home_is_the_accounts(Some(real), None));
    }
}
