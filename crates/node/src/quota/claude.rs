//! Claude Code's subscription: one [`super::Provider`], two strategies.
//!
//! 1. **`statusline`** (local) — the file Claude Code's statusline hook
//!    writes (`~/.claude/usage-status.json`), used while it is fresh;
//! 2. **`oauth-usage`** (remote) — the user's own stored Claude Code
//!    credential against Anthropic's usage endpoint, behind the chain's
//!    cache and cooldown (`super` says the rules and why).
//!
//! What is Claude's alone is here: where the login is stored (two
//! stores on macOS, not kept in step), the endpoint's shapes, and the
//! plan the credential names.

use std::time::{Duration, SystemTime};

use khor_core::{Quota, QuotaCredits, QuotaTrouble, QuotaWindow, QuotaWindowSpan};
use serde_json::Value;

use super::{Provider, State, Step, Strategy, percent, settle, title_case, unix_seconds};

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

/// The beta header the endpoint requires. Not a version khor chose.
const BETA: &str = "oauth-2025-04-20";

/// How long the statusline hook's file counts as describing now. Past
/// this the local source is skipped rather than served stale — it has no
/// age of its own on screen, so an old one would read as the present.
const LOCAL_FRESH: Duration = Duration::from_secs(30 * 60);
static STATE: State = State::new();

/// Claude Code, described (`super::Provider`).
pub static PROVIDER: Provider = Provider {
    id: "claude",
    vendor: "Claude Code",
    logged_in,
    strategies,
    finish,
    state: &STATE,
};

fn strategies() -> Vec<Strategy> {
    vec![
        Strategy {
            name: "statusline",
            step: Step::Local,
            available: Box::new(|| true),
            fetch: Box::new(|| {
                Box::pin(async { read_local_status().ok_or(QuotaTrouble::Unreadable) })
            }),
        },
        Strategy {
            name: "oauth-usage",
            step: Step::Remote,
            available: Box::new(logged_in),
            fetch: Box::new(|| Box::pin(fetch())),
        },
    ]
}

/// **The plan, which is not part of any strategy's answer.** It lives
/// in the stored credential, so it costs no request, it is the same
/// whichever strategy supplied the windows, and reading it after the
/// caches means changing plan shows up at once. A `plan` inside a cached
/// [`Quota`] is overwritten for that reason and not consulted.
fn finish(quota: &mut Quota) {
    quota.plan = plan_label();
}

/// This machine's Claude Code reading, down the chain.
pub async fn read() -> Result<Quota, QuotaTrouble> {
    PROVIDER.read().await
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
        source: None,
        limit_reached: None,
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
        source: None,
        limit_reached: None,
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
    if super::as_the_account() {
        panic!("a unit test reached the operator's Claude login (quota::stored_credential)");
    }

    #[cfg(target_os = "macos")]
    if super::as_the_account() {
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
    use crate::quota::percent;

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
    const REAL: &str = include_str!("../../tests/fixtures/oauth-usage-2026-09-06.json");

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

    /// **No unit test reads the operator's login**: they run under the
    /// operator's `HOME`, and one that reached the credential would stop
    /// here rather than read it (the tripwire in `stored_credential`).
    #[test]
    #[should_panic(expected = "reached the operator's Claude login")]
    fn unit_tests_never_reach_the_login() {
        let _ = logged_in();
    }
}
