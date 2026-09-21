//! What this machine's Claude subscription has left.
//!
//! Two sources, in order:
//!
//! 1. the local file Claude Code's statusline hook writes
//!    (`~/.claude/usage-status.json`), used while it is fresh;
//! 2. the user's own stored Claude Code credential against Anthropic's
//!    usage endpoint.
//!
//! **The second source is a non-public endpoint reached with a
//! credential the user stored for something else**, and the user ruled
//! on exactly that before this existed (2026-08-21). Three consequences
//! are written into the code rather than remembered:
//!
//! - **It is said out loud.** The credential was stored so `claude`
//!   could run; using it to answer a second question widens what it was
//!   given for, so whichever screen paints a number says whose login it
//!   came through. The words are in `crates/catalog`.
//! - **Nothing is trusted about the response shape.** A non-public
//!   endpoint owes no compatibility, so every field is optional on the
//!   way in and a shape this build cannot read degrades to "cannot
//!   read" — never to a zero, and never to a panic.
//! - **The rate-limit discipline is the feature, not an optimisation.**
//!   A quota that costs the user their account is worse than no quota.
//!   `khor serve` is resident on every machine, the desktop polls, and
//!   the CLI is a third process; each fetching on its own would hit one
//!   account three times over. So: five minutes of memory cache **and**
//!   a disk cache the whole machine shares, a ten-minute cooldown after
//!   a failure during which no request is made at all, and no automatic
//!   token refresh — khor asks the person to run `claude` instead of
//!   quietly minting credentials against an endpoint that never offered
//!   it one.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

use khor_core::{Quota, QuotaCredits, QuotaTrouble, QuotaWindow, QuotaWindowSpan};
use serde_json::Value;

const ENDPOINT: &str = "https://api.anthropic.com/api/oauth/usage";

/// Where the usage is asked, with the door a test uses to stand in for
/// Anthropic (`KHOR_CLAUDE_USAGE_URL`, the sibling of `KHOR_CLAUDE`).
/// **A test door, not a setting**: nothing on a real machine sets it, and
/// what it buys is that `smoke` can read a subscription end to end
/// against a fake it runs itself instead of against the operator's own
/// account (docs/handoff: 测试与 smoke 绝不碰真身份).
fn endpoint_from(door: Option<String>) -> String {
    door.filter(|d| !d.is_empty())
        .unwrap_or_else(|| ENDPOINT.to_owned())
}

fn endpoint() -> String {
    endpoint_from(std::env::var("KHOR_CLAUDE_USAGE_URL").ok())
}

/// **The keychain is the account's, and only a process speaking as the
/// account reads it.** Claude Code's login sits in two stores: the file
/// under `$HOME/.claude` and the login keychain. The file follows `$HOME`
/// and the keychain does not — so a process handed a scratch home (every
/// gate here) still reached the operator's real login through the
/// keychain, and then spent it against Anthropic's endpoint (2026-09-22,
/// s25-investigator: quota 1835 ms, quota_board 5477 ms in a smoke run).
///
/// So the keychain is asked only when `$HOME` *is* the account's home
/// directory (the password database's, not the environment's). A
/// process somebody pointed at another home is, for the purpose of
/// whose login to read, somebody else.
fn keychain_is_ours(home: Option<&Path>, account: Option<&Path>) -> bool {
    let (Some(home), Some(account)) = (home, account) else {
        return false;
    };
    let real = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    real(home) == real(account)
}

/// The account's home directory as the password database has it —
/// **not** `$HOME`, which is exactly the thing a test changes.
#[cfg(unix)]
fn account_home() -> Option<std::path::PathBuf> {
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
    Some(std::path::PathBuf::from(std::ffi::OsStr::from_bytes(
        dir.to_bytes(),
    )))
}

#[cfg(not(unix))]
fn account_home() -> Option<std::path::PathBuf> {
    None
}
/// The beta header the endpoint requires. Not a version khor chose.
const BETA: &str = "oauth-2025-04-20";

/// How long the statusline hook's file counts as describing now. Past
/// this the local source is skipped rather than served stale — it has no
/// age of its own on screen, so an old one would read as the present.
const LOCAL_FRESH: Duration = Duration::from_secs(30 * 60);
/// How long one fetch answers for. Opening and closing a panel must not
/// be a reason to touch a non-public endpoint again.
const CACHE_TTL: i64 = 300;
/// How long khor stays away after a failure.
///
/// **Ten minutes because the failure being guarded against is 429**, and
/// continuing to knock while rate limited is how a limit gets extended
/// rather than waited out. During the cooldown a stale reading is served
/// if there is one and the wait is stated if there is not; either way no
/// request leaves.
const FAIL_COOLDOWN: i64 = 600;

static MEM_CACHE: Mutex<Option<(i64, Quota)>> = Mutex::new(None);
static LAST_FAIL: Mutex<Option<i64>> = Mutex::new(None);

/// This machine's reading, from whichever source can answer — **plus
/// the plan, which is not part of any of them.**
///
/// The plan lives in the stored credential, so it is read here rather
/// than inside a source: it costs no request, it is the same answer
/// whichever source supplied the windows, and reading it outside the
/// caches means changing plan shows up at once instead of after five
/// minutes. A `plan` that came back inside a cached [`Quota`] is
/// overwritten for that reason and not consulted.
pub async fn read() -> Result<Quota, QuotaTrouble> {
    let mut quota = read_windows().await?;
    quota.plan = plan_label();
    // Beside the windows it is a function of, and after any cache, for
    // the same reason `plan` is here: computed where they are is what
    // makes it impossible for the number and the windows to disagree.
    quota.fill = whole_plan_fill(&quota.windows);
    Ok(quota)
}

/// Whether this machine has a Claude Code login at all — **asked
/// without touching the network**, which is what makes it usable where
/// it is used.
///
/// Registration calls it to refuse a subscription on a machine that has
/// nothing to read (`khor_node::accounts::register_account`). Calling
/// [`read`] there instead would be wrong twice: it can leave over HTTP,
/// and its answers include states that pass and fail by the minute — a
/// cooldown, a token that expired an hour ago. **A gate on a transient
/// state means the same action gives two different answers two minutes
/// apart, and that is not a refusal, it is a dice roll.**
///
/// **Having no credential at all is the one state that is neither
/// transient nor guessed**: it is exactly what [`QuotaTrouble::NoLogin`]
/// means, and it is the only thing a registering machine can prove on
/// the spot.
pub fn logged_in() -> bool {
    stored_credential().is_ok()
}

/// How full the subscription is **as a whole**, `0..=1`, or `None` when
/// no window speaks for the whole plan.
///
/// # The rule, in three steps, and each keeps the same property
///
/// 1. The **longest unscoped** window — today's readings make that the
///    week.
/// 2. No week, but a five-hour: that one. **Still the whole plan**,
///    just a shorter period.
/// 3. **No unscoped window at all: `None`.** There is no honest number
///    for the whole plan, so none is given — never a scoped window
///    standing in for one, and never a zero.
///
/// **Why the whole-plan window and not the fullest one.** 2026-09-07
/// the user ruled 「list 页、打开后的多环,claude 显示 week_all。现在,是
/// fable。」 — a window *scoped to one model* says how much of that
/// model's slice is gone, and when a line has room for exactly one
/// number, that number has to be about **this subscription**, not about
/// one slice of it. The slices are not lost: each is a pip of its own
/// on the rim.
///
/// **The rule it replaced was answering a different question and
/// answering it well.** 「The fullest one」 existed because `is_active`
/// cannot be trusted — the endpoint had it `false` on the 29% session
/// window and `true` on the weekly ([`QuotaWindow::active`] carries
/// that finding) — and **avoiding a bad signal is not the same as
/// choosing the right window**.
///
/// **Length comes from the order these arrive in, not from a table
/// here**: `parse_windows` sorts them shortest-first, whole plan before
/// any one model's share, so the last unscoped entry *is* the longest
/// one. A second opinion about which span is longer is how two places
/// come to disagree about one order.
///
/// ⚠️ **This lived in the face until 2026-09-07** (`QuotaLanding.tsx`'s
/// `headline`), which was survivable while a subscription was one
/// probed row and stopped being survivable when a subscription could
/// also be a registered account — see [`Quota::fill`] for what having
/// two sources for one picture would have cost.
fn whole_plan_fill(windows: &[QuotaWindow]) -> Option<f32> {
    let whole = windows.iter().rev().find(|w| w.scope.is_none())?;
    Some((whole.used_pct / 100.0).clamp(0.0, 1.0))
}

/// The plan the stored credential names, spelled for a person.
///
/// **Nothing is asked over the network**: `rateLimitTier` sits in the
/// same file `access_token` already reads. The slug is the vendor's, and
/// so is every word that comes out of it — `default_claude_max_20x` is
/// 「Max 20x」 because that is what those pieces say, and a slug that
/// does not fit the shape is passed through untouched.
///
/// `subscriptionType` is the coarser second spelling of the same fact
/// (「max」 with no multiplier) and answers only when the tier is
/// missing — a plan named without its size is still better than none.
fn plan_label() -> Option<String> {
    let raw = stored_credential().ok()?;
    let json: Value = serde_json::from_str(&raw).ok()?;
    let oauth = json.get("claudeAiOauth")?;
    if let Some(tier) = oauth.get("rateLimitTier").and_then(Value::as_str) {
        return Some(plan_from_tier(tier));
    }
    oauth
        .get("subscriptionType")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(title_case)
}

/// `max` → `Max`. One word, first letter up, and nothing else touched.
fn title_case(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// `default_claude_max_20x` → 「Max 20x」; `default_claude_pro` → 「Pro」.
///
/// **A slug that does not fit comes back whole.** Guessing at an
/// unfamiliar tier would put a plan name on the screen that the vendor
/// never used, and a reader cannot tell an invented name from a real
/// one — whereas a raw slug is visibly the endpoint's own word.
fn plan_from_tier(tier: &str) -> String {
    let Some(rest) = tier.strip_prefix("default_claude_") else {
        return tier.to_owned();
    };
    let (plan, size) = match rest.split_once('_') {
        Some((plan, size)) => (plan, Some(size)),
        None => (rest, None),
    };
    if !matches!(plan, "pro" | "max") {
        return tier.to_owned();
    }
    match size {
        // The multiplier is only a multiplier if it looks like one.
        Some(size)
            if size.ends_with('x')
                && size[..size.len() - 1].chars().all(|c| c.is_ascii_digit()) =>
        {
            format!("{} {size}", title_case(plan))
        }
        Some(_) => tier.to_owned(),
        None => title_case(plan),
    }
}

/// This machine's windows, from whichever source can answer.
async fn read_windows() -> Result<Quota, QuotaTrouble> {
    if let Some(quota) = read_local_status() {
        return Ok(quota);
    }
    let now = jiff::Timestamp::now().as_second();
    // Memory alone is not enough: `khor serve`, the desktop and the CLI
    // are three processes with three memory caches, and any one of them
    // restarting has an empty one and goes straight out to the endpoint.
    // The disk cache is what makes a machine's fetches one fetch.
    let cached = MEM_CACHE
        .lock()
        .unwrap()
        .clone()
        .or_else(|| read_disk_cache(now));
    if let Some((taken, quota)) = &cached
        && now - taken < CACHE_TTL
    {
        return Ok(stamp(quota.clone(), *taken));
    }
    if let Some(failed_at) = *LAST_FAIL.lock().unwrap() {
        let waited = now - failed_at;
        if waited < FAIL_COOLDOWN {
            return match cached {
                Some((taken, quota)) => Ok(stamp(quota, taken)),
                None => Err(QuotaTrouble::Cooling {
                    minutes: (FAIL_COOLDOWN - waited) / 60 + 1,
                }),
            };
        }
    }
    match fetch().await {
        Ok(quota) => {
            *MEM_CACHE.lock().unwrap() = Some((now, quota.clone()));
            *LAST_FAIL.lock().unwrap() = None;
            write_disk_cache(now, &quota);
            Ok(quota)
        }
        Err(trouble) => {
            // A refused credential is not a rate limit, and cooling off
            // would not fix it — the person has to run `claude`. Only
            // the failures that time might heal start a cooldown.
            if !matches!(trouble, QuotaTrouble::NoLogin | QuotaTrouble::Stale) {
                *LAST_FAIL.lock().unwrap() = Some(now);
            }
            match cached {
                Some((taken, quota)) => Ok(stamp(quota, taken)),
                None => Err(trouble),
            }
        }
    }
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

/// The cache every process on this machine shares.
///
/// Under `KHOR_HOME` rather than the OS cache directory, which is not
/// tidiness: khor already treats that root as "this installation", so
/// two homes on one machine — which is how this repo tests and how a
/// person runs two accounts — get two caches without anything having to
/// know they are different. It is disposable; losing it costs one fetch.
fn disk_cache_path() -> PathBuf {
    crate::Node::root_from_env()
        .join(".khor")
        .join("claude-usage.json")
}

fn read_disk_cache(now: i64) -> Option<(i64, Quota)> {
    read_cache_at(&disk_cache_path(), now)
}

/// Path injected so tests never touch the real one and never each other's.
fn read_cache_at(path: &Path, now: i64) -> Option<(i64, Quota)> {
    let json: Value = serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()?;
    let taken = json.get("fetched_at")?.as_i64()?;
    // A clock that jumped backwards — a timezone change, a laptop waking
    // — makes `now - taken` negative, and this entry would then look
    // fresh forever and never refresh again. Treat it as absent.
    if taken > now {
        return None;
    }
    let quota: Quota = serde_json::from_value(json.get("quota")?.clone()).ok()?;
    Some((taken, quota))
}

fn write_disk_cache(now: i64, quota: &Quota) {
    write_cache_at(&disk_cache_path(), now, quota);
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

/// The file Claude Code's statusline hook writes, if it is recent.
///
/// **Absent on a machine whose owner never installed that hook**, which
/// is most of them — this source is a shortcut when it happens to be
/// there, not a fallback that makes the endpoint optional.
fn read_local_status() -> Option<Quota> {
    let path = std::env::home_dir()?
        .join(".claude")
        .join("usage-status.json");
    let written = std::fs::metadata(&path).ok()?.modified().ok()?;
    if SystemTime::now().duration_since(written).ok()? > LOCAL_FRESH {
        return None;
    }
    let json: Value = serde_json::from_str(&std::fs::read_to_string(&path).ok()?).ok()?;
    // The hook nests the windows; the endpoint does not. One parser for
    // both, handed whichever object actually holds them.
    let windows = parse_windows(json.get("rate_limits").unwrap_or(&json))?;
    let as_of = written
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs() as i64);
    // The hook's file is about windows and says nothing about money.
    Some(Quota {
        windows,
        as_of,
        // Both filled by `read`: a source knows only what its own
        // payload said, and the share is a reading of the windows
        // rather than a thing any source reports.
        fill: None,
        plan: None,
        credits: None,
    })
}

async fn fetch() -> Result<Quota, QuotaTrouble> {
    let token = access_token()?;
    let response = reqwest::Client::new()
        .get(endpoint())
        .bearer_auth(&token)
        .header("anthropic-beta", BETA)
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(|_| QuotaTrouble::Unreachable)?;
    match response.status().as_u16() {
        401 | 403 => return Err(QuotaTrouble::Stale),
        429 => return Err(QuotaTrouble::Cooling { minutes: 10 }),
        code if !(200..300).contains(&code) => return Err(QuotaTrouble::Unreachable),
        _ => {}
    }
    let json: Value = response
        .json()
        .await
        .map_err(|_| QuotaTrouble::Unreadable)?;
    let windows = parse_windows(&json).ok_or(QuotaTrouble::Unreadable)?;
    Ok(Quota {
        windows,
        as_of: None,
        // Filled by `read`, which has the credential; a source knows
        // only what its own payload said.
        plan: None,
        fill: None,
        credits: parse_credits(&json),
    })
}

/// The access token Claude Code left behind.
fn access_token() -> Result<String, QuotaTrouble> {
    let raw = stored_credential()?;
    let json: Value = serde_json::from_str(&raw).map_err(|_| QuotaTrouble::Stale)?;
    let oauth = json.get("claudeAiOauth").ok_or(QuotaTrouble::Stale)?;
    if let Some(expires) = oauth.get("expiresAt").and_then(unix_seconds)
        && expires < jiff::Timestamp::now().as_second()
    {
        return Err(QuotaTrouble::Stale);
    }
    oauth
        .get("accessToken")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or(QuotaTrouble::Stale)
}

/// Claude Code's credential, from whichever store holds the newer one.
///
/// **Two stores on macOS and they are not kept in step**: the keychain
/// and `~/.claude/.credentials.json`. Preferring one outright produces
/// the worst possible failure — an expired token in the preferred store
/// while a valid one sits in the other, reported to the user as "your
/// login expired" when it has not. So both are read and the later
/// `expiresAt` wins; neither is treated as authoritative.
fn stored_credential() -> Result<String, QuotaTrouble> {
    let mut found: Vec<String> = Vec::new();

    // **A tripwire for this crate's own tests.** They run with the
    // operator's `HOME`, so a test that reached this line would be reading
    // the operator's Claude login — the one thing no test may touch. It
    // stops the test instead of reading (`unit_tests_never_reach_the_login`).
    #[cfg(test)]
    if keychain_is_ours(std::env::home_dir().as_deref(), account_home().as_deref()) {
        panic!("a unit test reached the operator's Claude login (quota::stored_credential)");
    }

    #[cfg(target_os = "macos")]
    if keychain_is_ours(std::env::home_dir().as_deref(), account_home().as_deref()) {
        // A process with no GUI session cannot read the keychain at all
        // ("User interaction is not allowed"). That is not an error here
        // — it just means this store has nothing to offer and the file
        // answers instead.
        if let Ok(out) = std::process::Command::new("security")
            .args([
                "find-generic-password",
                "-s",
                "Claude Code-credentials",
                "-w",
            ])
            .output()
            && out.status.success()
        {
            let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !text.is_empty() {
                found.push(text);
            }
        }
    }

    if let Some(home) = std::env::home_dir()
        && let Ok(text) = std::fs::read_to_string(home.join(".claude").join(".credentials.json"))
    {
        found.push(text);
    }

    found
        .into_iter()
        .max_by_key(|raw| expiry_of(raw).unwrap_or(i64::MIN))
        .ok_or(QuotaTrouble::NoLogin)
}

/// When a stored credential expires, for choosing between stores.
/// Unreadable counts as oldest, so it never displaces a valid one.
fn expiry_of(raw: &str) -> Option<i64> {
    serde_json::from_str::<Value>(raw)
        .ok()?
        .get("claudeAiOauth")?
        .get("expiresAt")
        .and_then(unix_seconds)
}

/// A percentage from a number or a string, clamped.
///
/// Clamped rather than rejected: a value outside 0–100 is a misread, and
/// a bar drawn past full is a more confusing answer than a full one.
fn percent(v: &Value) -> Option<f32> {
    let n = match v {
        Value::Number(n) => n.as_f64()?,
        Value::String(s) => s.trim().parse().ok()?,
        _ => return None,
    };
    Some(n.clamp(0.0, 100.0) as f32)
}

/// Unix seconds from an RFC 3339 string, or from seconds or milliseconds.
fn unix_seconds(v: &Value) -> Option<i64> {
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
fn settle(used_pct: f32, resets_at: Option<i64>, now: i64) -> (f32, Option<i64>) {
    match resets_at {
        Some(at) if at < now => (0.0, None),
        other => (used_pct, other),
    }
}

/// The `limits[]` array: **every entry says what it is.**
///
/// This is the shape the endpoint grew, and it is why this file stopped
/// dropping windows it does not recognise. Each entry carries its own
/// `percent`, `resets_at`, `group` (how long) and `scope` (what it is
/// limited to), so a limit khor has never seen is still a fully
/// described number rather than a bare one — see [`QuotaWindowSpan`].
///
/// **`kind` is deliberately not read.** It is `session` / `weekly_all` /
/// `weekly_scoped`, which fuses length and limit back together — the
/// exact fusion this change took apart. `group` and `scope` answer the
/// two questions separately, and reading `kind` as well would be a third
/// opinion that can disagree with them.
fn from_limits(v: &Value, now: i64) -> Vec<QuotaWindow> {
    let Some(limits) = v.get("limits").and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for limit in limits {
        // One entry that cannot be read costs that entry, never the
        // array — the same posture the key path has always had.
        let Some(group) = limit.get("group").and_then(Value::as_str) else {
            continue;
        };
        let Some(used_pct) = ["percent", "utilization", "used_percentage", "used_pct"]
            .iter()
            .find_map(|f| limit.get(*f).and_then(percent))
        else {
            continue;
        };
        let span = match group {
            "session" => QuotaWindowSpan::FiveHour,
            "weekly" => QuotaWindowSpan::SevenDay,
            other => QuotaWindowSpan::Other(other.to_owned()),
        };
        // The model's own display name, and only that: `scope.model.id`
        // was null on every entry seen, and a scope with no model
        // (`surface`, one day) has no name to draw, so it is not a
        // scoped window this build can label.
        let scope = limit
            .get("scope")
            .and_then(|s| s.get("model"))
            .and_then(|m| m.get("display_name"))
            .and_then(Value::as_str)
            .map(str::to_owned);
        let (used_pct, resets_at) =
            settle(used_pct, limit.get("resets_at").and_then(unix_seconds), now);
        out.push(QuotaWindow {
            span,
            scope,
            used_pct,
            resets_at,
            active: limit.get("is_active").and_then(Value::as_bool),
            severity: limit
                .get("severity")
                .and_then(Value::as_str)
                .map(str::to_owned),
        });
    }
    out
}

/// The four top-level keys, which are all an older payload (and the
/// statusline hook's file) has.
///
/// **The two model names here are the endpoint's own key words**, read
/// off `seven_day_opus` rather than chosen by khor; the capital is
/// typesetting. On a payload that has `limits[]` these lose to it, where
/// the name arrives as `display_name` already spelled.
fn from_top_keys(v: &Value, now: i64) -> Vec<QuotaWindow> {
    const KEYS: &[(&str, QuotaWindowSpan, Option<&str>)] = &[
        ("five_hour", QuotaWindowSpan::FiveHour, None),
        ("seven_day", QuotaWindowSpan::SevenDay, None),
        (
            "seven_day_sonnet",
            QuotaWindowSpan::SevenDay,
            Some("Sonnet"),
        ),
        ("seven_day_opus", QuotaWindowSpan::SevenDay, Some("Opus")),
    ];
    let mut out = Vec::new();
    for (key, span, scope) in KEYS {
        let Some(window) = v.get(key).filter(|w| !w.is_null()) else {
            continue;
        };
        // Three names for one number have been seen in the wild; a
        // fourth costs this row and nothing else.
        let Some(used_pct) = ["utilization", "used_percentage", "used_pct"]
            .iter()
            .find_map(|f| window.get(*f).and_then(percent))
        else {
            continue;
        };
        let (used_pct, resets_at) = settle(
            used_pct,
            window.get("resets_at").and_then(unix_seconds),
            now,
        );
        out.push(QuotaWindow {
            span: span.clone(),
            scope: scope.map(str::to_owned),
            used_pct,
            resets_at,
            active: None,
            severity: None,
        });
    }
    out
}

/// Money, out of whichever of the endpoint's two spellings answers.
///
/// **`extra_usage` and `spend` are the same credits twice**: 921 with
/// `decimal_places: 2` and 921 with `exponent: 2` are both 9.21 USD.
/// This reads `extra_usage` because that is the block carrying the
/// switch and the reason, and falls back to `spend` for a figure the
/// first one left out.
///
/// ⚠️ **A minor-unit amount with no exponent is not read at all.** The
/// exponent is what turns 921 into 9.21, and assuming 2 would be a
/// hundred-fold error on a currency that does not have two decimals —
/// an amount off by 100 is worse than an amount missing, because a
/// person acts on it.
fn parse_credits(v: &Value) -> Option<QuotaCredits> {
    let extra = v.get("extra_usage").filter(|e| !e.is_null());
    let spend = v.get("spend").filter(|s| !s.is_null());
    let (extra, spend) = (extra?, spend);
    // `used_credits` is in minor units, and `decimal_places` says how
    // many. Both or neither.
    let major = |amount: Option<f64>, places: Option<i64>| -> Option<f64> {
        Some(amount? / 10f64.powi(i32::try_from(places?).ok()?))
    };
    let used = major(
        extra.get("used_credits").and_then(Value::as_f64),
        extra.get("decimal_places").and_then(Value::as_i64),
    )
    .or_else(|| {
        let used = spend?.get("used")?;
        major(
            used.get("amount_minor").and_then(Value::as_f64),
            used.get("exponent").and_then(Value::as_i64),
        )
    });
    let money_at = |block: Option<&Value>, key: &str| -> Option<f64> {
        let m = block?.get(key).filter(|v| !v.is_null())?;
        // Either a bare number in the major unit, or the minor-unit
        // object `spend` uses.
        m.as_f64().or_else(|| {
            major(
                m.get("amount_minor").and_then(Value::as_f64),
                m.get("exponent").and_then(Value::as_i64),
            )
        })
    };
    Some(QuotaCredits {
        enabled: extra
            .get("is_enabled")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        disabled_reason: extra
            .get("disabled_reason")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(str::to_owned),
        used,
        balance: money_at(spend, "balance"),
        monthly_limit: money_at(Some(extra), "monthly_limit").or_else(|| money_at(spend, "limit")),
        currency: extra
            .get("currency")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .or_else(|| {
                spend?
                    .get("used")?
                    .get("currency")?
                    .as_str()
                    .map(str::to_owned)
            }),
        ever_enabled: extra
            .get("credits_ever_enabled")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    })
}

/// Where a window sits in the order [`Quota::windows`] promises:
/// shortest first, and the whole plan before one model's share of it.
fn seat(w: &QuotaWindow) -> (u8, bool, String) {
    let length = match w.span {
        QuotaWindowSpan::FiveHour => 0,
        QuotaWindowSpan::SevenDay => 1,
        // A length khor cannot order goes last, for the same reason it
        // does not go on the ring: there is no order to put it in.
        QuotaWindowSpan::Other(_) => 2,
    };
    (
        length,
        w.scope.is_some(),
        w.scope.clone().unwrap_or_default(),
    )
}

/// The windows out of either source's JSON.
///
/// **Every step here is allowed to give up on one window without losing
/// the others**, which is the whole posture toward an endpoint that owes
/// khor nothing: a renamed field costs that row, not the panel.
///
/// **Both sources, `limits[]` first, merged on identity.** A payload
/// that has both says the same thing twice — the top-level `seven_day`
/// and `limits[]`'s `weekly_all` are one window — and the merge is a key
/// comparison (`span` + `scope`), not a name match, which is the whole
/// reason those two fields were split apart. `limits[]` wins a tie
/// because it is the entry that says what it is; a top-level key with no
/// entry beside it (an older payload, the statusline file) is still
/// carried.
fn parse_windows(v: &Value) -> Option<Vec<QuotaWindow>> {
    let now = jiff::Timestamp::now().as_second();
    let mut windows = from_limits(v, now);
    for w in from_top_keys(v, now) {
        if !windows
            .iter()
            .any(|had| had.span == w.span && had.scope == w.scope)
        {
            windows.push(w);
        }
    }
    windows.sort_by_key(seat);
    (!windows.is_empty()).then_some(windows)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One file per test: sharing one would make them tread on each
    /// other, and none of them may touch the real cache.
    fn tmp_cache(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("khor-quota-{}-{tag}.json", std::process::id()))
    }

    fn quota_at(used: f32) -> Quota {
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

    /// **The whole plan's window, not the fullest one and not a model's
    /// slice.**
    ///
    /// The fixture is the shape the endpoint actually answers, in the
    /// order `parse_windows` leaves it: a five-hour, the week, and the
    /// week *scoped to one model*. Each of the three numbers is
    /// distinct and the scoped one is **the fullest**, so the two rules
    /// this replaced answer differently here — 「the fullest」 would say
    /// 71%, 「the last window」 would say 71%, and only 「the longest
    /// unscoped」 says 39%. A fixture whose scoped window was the
    /// emptiest could not tell them apart.
    #[test]
    fn the_share_is_the_longest_window_that_covers_the_whole_plan() {
        let windows = vec![
            window(QuotaWindowSpan::FiveHour, None, 4.0),
            window(QuotaWindowSpan::SevenDay, None, 39.0),
            window(QuotaWindowSpan::SevenDay, Some("Fable"), 71.0),
        ];
        assert_eq!(whole_plan_fill(&windows), Some(0.39));
        // Step two: no week, and the five-hour speaks for the plan.
        assert_eq!(whole_plan_fill(&windows[..1]), Some(0.04));
        // Step three: **nothing**, never a scoped window standing in
        // for the plan and never a zero — a zero is a drawing of
        // 「nothing used」, which is a claim this has no basis for.
        assert_eq!(whole_plan_fill(&windows[2..]), None);
        assert_eq!(whole_plan_fill(&[]), None);
    }

    /// A reading is a share of one, not a percentage, and it cannot
    /// leave that range: the endpoint has answered past 100 (`percent`
    /// clamps for the same reason) and an arc drawn past full is a
    /// worse answer than a full one.
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

    /// The disk cache exists to save a *request*, not a millisecond: the
    /// three processes on one machine have three memory caches, and any
    /// one of them restarting would otherwise go straight to the
    /// endpoint on the same account.
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
    fn the_endpoints_shape_parses() {
        let body = serde_json::json!({
            "five_hour": {"utilization": 34.0, "resets_at": "2099-01-01T00:00:00+00:00"},
            "seven_day": {"utilization": 61.5, "resets_at": "2099-01-03T00:00:00+00:00"},
            "seven_day_opus": null
        });
        let windows = parse_windows(&body).expect("two readable windows");
        assert_eq!(windows.len(), 2);
        assert_eq!(windows[0].span, QuotaWindowSpan::FiveHour);
        assert!((windows[0].used_pct - 34.0).abs() < 1e-6);
    }

    /// The hook writes a different field name and nests the object. Same
    /// parser, or the two shapes drift apart one fix at a time.
    #[test]
    fn the_statusline_hooks_shape_parses_too() {
        let body = serde_json::json!({
            "rate_limits": {
                "five_hour": {"used_percentage": 12, "resets_at": "2099-01-01T00:00:00Z"},
                "seven_day": {"used_percentage": 40, "resets_at": "2099-01-02T00:00:00Z"}
            }
        });
        assert_eq!(parse_windows(&body["rate_limits"]).unwrap().len(), 2);
    }

    #[test]
    fn a_window_past_its_reset_reads_empty_rather_than_stale() {
        let body = serde_json::json!({
            "five_hour": {"utilization": 88.0, "resets_at": "2020-01-01T00:00:00Z"}
        });
        let windows = parse_windows(&body).unwrap();
        assert_eq!(windows[0].used_pct, 0.0);
        assert!(windows[0].resets_at.is_none());
    }

    /// A shape khor cannot read is not a quota of zero.
    #[test]
    fn an_unreadable_shape_yields_nothing_rather_than_zero() {
        assert!(parse_windows(&serde_json::json!({"five_hour": {"who_knows": 5}})).is_none());
        assert!(parse_windows(&serde_json::json!({})).is_none());
    }

    /// The keychain and the file are not kept in step. Preferring one
    /// produces the worst failure there is here: telling someone their
    /// login expired while a valid one sits in the other store.
    #[test]
    fn the_newer_credential_wins_between_stores() {
        let old = r#"{"claudeAiOauth":{"accessToken":"old","expiresAt":1000000000000}}"#;
        let new = r#"{"claudeAiOauth":{"accessToken":"new","expiresAt":2000000000000}}"#;
        assert!(expiry_of(old).unwrap() < expiry_of(new).unwrap());
        // Unreadable counts as oldest, so it cannot displace a valid one.
        assert_eq!(expiry_of("not json"), None);
    }

    /// Percentages arrive as numbers and as strings, and out-of-range is
    /// a misread rather than a fuller-than-full window.
    /// **The real thing, byte for byte** — one capture of
    /// `api.anthropic.com/api/oauth/usage`, 2026-09-06, the day it
    /// started reporting a per-model week. It carries no token and no
    /// account: percentages, reset times, and the endpoint's own code
    /// names.
    ///
    /// It is here rather than hand-shaped because the hand-shaped
    /// version is the thing that drifts, and because two of the
    /// assertions below are about parts nobody would think to invent —
    /// the ten code-name buckets, and a `nimbus_quill` that is *not*
    /// null and still must not become a window.
    const REAL: &str = include_str!("../tests/fixtures/oauth-usage-2026-09-06.json");

    /// A fixed instant an hour before that capture's earliest reset, so
    /// these tests say the same thing next year rather than starting to
    /// read the payload as rolled over.
    ///
    /// ⚠️ **An hour, not a second.** This was first written as
    /// `1_788_706_800` under a comment claiming 14:00Z; that number is
    /// 15:00Z — the five-hour window's reset instant exactly. Every
    /// test still passed, because [`settle`] zeroes on `at < now` and
    /// not on `at == now`: the fixture was sitting **on** the boundary
    /// it was supposed to be an hour away from, and one second's drift
    /// in either the constant or that comparison would have zeroed a
    /// window these tests assert is 29% full. A fixture on a boundary
    /// is the same mistake as a teardown target on one.
    const BEFORE_RESET: i64 = 1_788_703_200; // 2026-09-06T14:00:00Z

    fn real() -> Value {
        serde_json::from_str(REAL).expect("the captured payload is JSON")
    }

    #[test]
    fn the_endpoint_names_three_windows_and_ten_code_names_that_are_not_windows() {
        let windows = from_limits(&real(), BEFORE_RESET);
        assert_eq!(
            windows
                .iter()
                .map(|w| (w.span.clone(), w.scope.clone(), w.used_pct))
                .collect::<Vec<_>>(),
            vec![
                (QuotaWindowSpan::FiveHour, None, 29.0),
                (QuotaWindowSpan::SevenDay, None, 30.0),
                (QuotaWindowSpan::SevenDay, Some("Fable".into()), 29.0),
            ],
        );
        // `nimbus_quill` is a top-level bucket with a readable 0% in it,
        // and it is **not** in `limits[]` — so it is not a window, and
        // the reason is not that it was unreadable.
        assert!(percent(&real()["nimbus_quill"]["utilization"]).is_some());
    }

    /// The whole point of reading `limits[]`: **the name is the
    /// endpoint's.** A build that spelled 「Fable」 anywhere of its own
    /// passes the test above and fails this one.
    #[test]
    fn a_per_model_window_is_named_by_the_payload_and_not_by_khor() {
        let renamed: Value = serde_json::from_str(&REAL.replace("Fable", "Zephyr")).unwrap();
        let scopes: Vec<_> = from_limits(&renamed, BEFORE_RESET)
            .into_iter()
            .filter_map(|w| w.scope)
            .collect();
        assert_eq!(scopes, vec!["Zephyr".to_owned()]);
    }

    /// The premise that changed. A limit whose length khor cannot order
    /// used to be dropped because it was a bare percentage; it now says
    /// its own group, percent and reset, so it is kept — off the ring
    /// (no order to put it in) and last in the list.
    #[test]
    fn a_length_khor_cannot_order_is_kept_and_wears_the_endpoints_own_word() {
        let body = serde_json::json!({
            "limits": [
                {"group": "weekly", "percent": 30, "resets_at": "2099-01-02T00:00:00Z", "scope": null},
                {"group": "monthly", "percent": 4, "resets_at": "2099-02-01T00:00:00Z", "scope": null},
            ]
        });
        let windows = parse_windows(&body).expect("both");
        assert_eq!(windows[0].span, QuotaWindowSpan::SevenDay);
        assert_eq!(windows[1].span, QuotaWindowSpan::Other("monthly".into()));
        assert_eq!(windows[1].used_pct, 4.0);
    }

    /// The regression: a payload from before `limits[]`, and the
    /// statusline hook's file, still yield the four keys — with the two
    /// model windows now spelled as a week scoped to a model rather than
    /// as two more kinds.
    #[test]
    fn an_older_payload_with_no_limits_still_yields_its_four_keys() {
        let body = serde_json::json!({
            "five_hour": {"utilization": 34.0, "resets_at": "2099-01-01T00:00:00Z"},
            "seven_day": {"utilization": 61.5, "resets_at": "2099-01-03T00:00:00Z"},
            "seven_day_sonnet": {"utilization": 12.0, "resets_at": "2099-01-03T00:00:00Z"},
            "seven_day_opus": {"utilization": 5.0, "resets_at": "2099-01-03T00:00:00Z"}
        });
        assert_eq!(
            parse_windows(&body)
                .expect("four")
                .iter()
                .map(|w| (w.span.clone(), w.scope.clone()))
                .collect::<Vec<_>>(),
            vec![
                (QuotaWindowSpan::FiveHour, None),
                (QuotaWindowSpan::SevenDay, None),
                (QuotaWindowSpan::SevenDay, Some("Opus".into())),
                (QuotaWindowSpan::SevenDay, Some("Sonnet".into())),
            ],
        );
    }

    /// **One window, two names for it, one row.** The top-level
    /// `seven_day` and `limits[]`'s `weekly_all` are the same window;
    /// the merge is `(span, scope)` equality, which is the reason those
    /// two fields were split apart in the first place.
    #[test]
    fn a_window_both_sources_report_comes_out_once_and_from_limits() {
        let windows = parse_windows(&real()).expect("windows");
        assert_eq!(
            windows
                .iter()
                .filter(|w| w.span == QuotaWindowSpan::SevenDay && w.scope.is_none())
                .count(),
            1,
        );
        // Three, not five: the two sources overlap on the five-hour and
        // the seven-day windows, and Fable is only in `limits[]`.
        assert_eq!(windows.len(), 3);
        // And `limits[]` won the tie — it is the entry carrying the two
        // extra facts, which the key path cannot supply.
        assert!(windows.iter().any(|w| w.active == Some(true)));
    }

    #[test]
    fn the_windows_come_out_shortest_first_and_the_whole_plan_before_one_models_share() {
        let windows = parse_windows(&real()).expect("windows");
        let seats: Vec<_> = windows.iter().map(seat).collect();
        let mut sorted = seats.clone();
        sorted.sort();
        assert_eq!(seats, sorted);
        assert_eq!(windows[0].span, QuotaWindowSpan::FiveHour);
        assert_eq!(windows[2].scope, Some("Fable".into()));
    }

    /// Carried, not painted (see [`QuotaWindow::active`]). The key path
    /// says neither, and saying `false` there would be inventing an
    /// answer the endpoint did not give.
    #[test]
    fn the_two_extra_facts_come_from_limits_and_are_absent_from_the_key_path() {
        let from_new = from_limits(&real(), BEFORE_RESET);
        assert_eq!(from_new[1].active, Some(true));
        assert_eq!(from_new[1].severity.as_deref(), Some("normal"));
        let old = serde_json::json!({"five_hour": {"utilization": 1.0}});
        let from_old = from_top_keys(&old, BEFORE_RESET);
        assert_eq!(from_old[0].active, None);
        assert_eq!(from_old[0].severity, None);
    }

    /// The capture's own reset strings carry a fractional second and a
    /// `+00:00` offset, which is a shape the previous tests never saw.
    #[test]
    fn a_reset_time_with_a_fraction_and_an_offset_reads() {
        assert_eq!(
            unix_seconds(&serde_json::json!("2026-09-12T08:00:00.400168+00:00")),
            Some(1_789_200_000),
        );
    }

    #[test]
    fn a_tier_slug_is_de_slugged_and_an_unfamiliar_one_is_not_guessed_at() {
        assert_eq!(plan_from_tier("default_claude_max_20x"), "Max 20x");
        assert_eq!(plan_from_tier("default_claude_max_5x"), "Max 5x");
        assert_eq!(plan_from_tier("default_claude_pro"), "Pro");
        // **Whole, not half.** Every one of these has a piece this
        // build recognises, and guessing from that piece would put a
        // plan name on the screen the vendor never used.
        assert_eq!(
            plan_from_tier("enterprise_claude_max_20x"),
            "enterprise_claude_max_20x"
        );
        assert_eq!(
            plan_from_tier("default_claude_team_20x"),
            "default_claude_team_20x"
        );
        assert_eq!(
            plan_from_tier("default_claude_max_enormous"),
            "default_claude_max_enormous"
        );
        assert_eq!(plan_from_tier(""), "");
    }

    /// Money, and the reason the exponent is not assumed: the capture
    /// says 921 twice, in two blocks, each with its own way of saying
    /// 「shift two places」.
    #[test]
    fn the_two_spellings_of_the_same_credits_read_as_the_same_money() {
        let credits = parse_credits(&real()).expect("the capture has both blocks");
        assert_eq!(credits.used, Some(9.21));
        assert_eq!(credits.currency.as_deref(), Some("USD"));
        assert!(!credits.enabled);
        assert_eq!(credits.disabled_reason.as_deref(), Some("out_of_credits"));
        assert!(credits.ever_enabled);
        // Neither was reported, and neither may be drawn as zero.
        assert_eq!(credits.balance, None);
        assert_eq!(credits.monthly_limit, None);

        // `spend` alone says the same money a different way.
        let only_spend = serde_json::json!({
            "extra_usage": {"is_enabled": true},
            "spend": {"used": {"amount_minor": 921, "currency": "USD", "exponent": 2}},
        });
        assert_eq!(parse_credits(&only_spend).unwrap().used, Some(9.21));
    }

    /// An amount whose exponent nobody stated is **not read**, because
    /// assuming two decimals is a hundred-fold error on a currency that
    /// does not have them — and a wrong figure about money is acted on.
    #[test]
    fn a_minor_unit_amount_with_no_exponent_is_left_unread() {
        let body = serde_json::json!({
            "extra_usage": {"is_enabled": false, "used_credits": 921.0},
            "spend": {"used": {"amount_minor": 921}},
        });
        assert_eq!(parse_credits(&body).unwrap().used, None);
    }

    #[test]
    fn a_payload_with_no_credits_block_says_so_rather_than_zero() {
        assert!(parse_credits(&serde_json::json!({"five_hour": null})).is_none());
        assert!(parse_credits(&serde_json::json!({"extra_usage": null})).is_none());
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

    /// The test door answers when it is set, and Anthropic's own address
    /// when it is not — the default must not move.
    #[test]
    fn the_usage_endpoint_has_a_test_door_and_a_fixed_default() {
        assert_eq!(endpoint_from(None), ENDPOINT);
        assert_eq!(
            endpoint_from(Some("http://127.0.0.1:9/usage".into())),
            "http://127.0.0.1:9/usage"
        );
        assert_eq!(
            endpoint_from(Some(String::new())),
            ENDPOINT,
            "an empty door is no door"
        );
    }

    /// **A process with a scratch `$HOME` does not read the keychain** —
    /// the keychain is the account's, and that process is not speaking as
    /// the account. The same home, spelled the same, is the account.
    #[test]
    fn only_the_accounts_own_home_reads_the_keychain() {
        let real = Path::new("/Users/someone");
        assert!(keychain_is_ours(Some(real), Some(real)));
        assert!(!keychain_is_ours(
            Some(Path::new("/tmp/khor-smoke/home")),
            Some(real)
        ));
        assert!(
            !keychain_is_ours(None, Some(real)),
            "no home at all is not the account"
        );
        assert!(
            !keychain_is_ours(Some(real), None),
            "an account the system cannot name is not this one"
        );
    }

    /// **No unit test reads the operator's login**: they run under the
    /// operator's `HOME`, and one that reached the credential would stop
    /// here rather than read it (the tripwire in `stored_credential`).
    #[test]
    #[should_panic(expected = "reached the operator's Claude login")]
    fn unit_tests_never_reach_the_login() {
        let _ = logged_in();
    }
}
