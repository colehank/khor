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

use khor_core::{Quota, QuotaTrouble, QuotaWindow, QuotaWindowKind};
use serde_json::Value;

const ENDPOINT: &str = "https://api.anthropic.com/api/oauth/usage";
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

/// This machine's reading, from whichever source can answer.
pub async fn read() -> Result<Quota, QuotaTrouble> {
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
    let _ = std::fs::write(path, serde_json::json!({ "fetched_at": now, "quota": quota }).to_string());
}

/// The file Claude Code's statusline hook writes, if it is recent.
///
/// **Absent on a machine whose owner never installed that hook**, which
/// is most of them — this source is a shortcut when it happens to be
/// there, not a fallback that makes the endpoint optional.
fn read_local_status() -> Option<Quota> {
    let path = std::env::home_dir()?.join(".claude").join("usage-status.json");
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
    Some(Quota { windows, as_of })
}

async fn fetch() -> Result<Quota, QuotaTrouble> {
    let token = access_token()?;
    let response = reqwest::Client::new()
        .get(ENDPOINT)
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
    let json: Value = response.json().await.map_err(|_| QuotaTrouble::Unreadable)?;
    let windows = parse_windows(&json).ok_or(QuotaTrouble::Unreadable)?;
    Ok(Quota {
        windows,
        as_of: None,
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

    #[cfg(target_os = "macos")]
    {
        // A process with no GUI session cannot read the keychain at all
        // ("User interaction is not allowed"). That is not an error here
        // — it just means this store has nothing to offer and the file
        // answers instead.
        if let Ok(out) = std::process::Command::new("security")
            .args(["find-generic-password", "-s", "Claude Code-credentials", "-w"])
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

/// The windows out of either source's JSON.
///
/// **Every step here is allowed to give up on one window without losing
/// the others**, which is the whole posture toward an endpoint that owes
/// khor nothing: a renamed field costs that row, not the panel.
fn parse_windows(v: &Value) -> Option<Vec<QuotaWindow>> {
    const KEYS: &[(&str, QuotaWindowKind)] = &[
        ("five_hour", QuotaWindowKind::FiveHour),
        ("seven_day", QuotaWindowKind::SevenDay),
        ("seven_day_sonnet", QuotaWindowKind::SevenDaySonnet),
        ("seven_day_opus", QuotaWindowKind::SevenDayOpus),
    ];
    let now = jiff::Timestamp::now().as_second();
    let mut windows = Vec::new();
    for (key, kind) in KEYS {
        let Some(window) = v.get(key).filter(|w| !w.is_null()) else {
            continue;
        };
        // Three names for one number have been seen in the wild; a
        // fourth costs this row and nothing else.
        let Some(mut used_pct) = ["utilization", "used_percentage", "used_pct"]
            .iter()
            .find_map(|f| window.get(*f).and_then(percent))
        else {
            continue;
        };
        let mut resets_at = window.get("resets_at").and_then(unix_seconds);
        // The window already rolled over: it is empty now, and the reset
        // time is in the past and says nothing about the next one.
        if let Some(at) = resets_at
            && at < now
        {
            used_pct = 0.0;
            resets_at = None;
        }
        windows.push(QuotaWindow {
            kind: *kind,
            used_pct,
            resets_at,
        });
    }
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
                kind: QuotaWindowKind::FiveHour,
                used_pct: used,
                resets_at: None,
            }],
            as_of: None,
        }
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
        assert_eq!(stamp(quota_at(1.0), 1_700_000_000).as_of, Some(1_700_000_000));
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
        assert_eq!(windows[0].kind, QuotaWindowKind::FiveHour);
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
    #[test]
    fn percentages_are_read_leniently_and_clamped() {
        assert_eq!(percent(&serde_json::json!("12.5")), Some(12.5));
        assert_eq!(percent(&serde_json::json!(140)), Some(100.0));
        assert_eq!(percent(&serde_json::json!(-3)), Some(0.0));
        assert_eq!(percent(&serde_json::json!(true)), None);
    }

    #[test]
    fn instants_are_read_as_rfc3339_seconds_or_milliseconds() {
        assert_eq!(unix_seconds(&serde_json::json!(1_700_000_000)), Some(1_700_000_000));
        assert_eq!(unix_seconds(&serde_json::json!(1_700_000_000_000i64)), Some(1_700_000_000));
        assert_eq!(
            unix_seconds(&serde_json::json!("2023-11-14T22:13:20Z")),
            Some(1_700_000_000)
        );
        assert_eq!(unix_seconds(&serde_json::json!("not a time")), None);
    }
}
