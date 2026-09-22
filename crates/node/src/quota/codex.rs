//! Codex's subscription (a ChatGPT plan): one [`super::Provider`], two
//! strategies (docs/SHELL.md 第九轮).
//!
//! 1. **`rollout`** (local) — the `rate_limits` snapshot Codex writes
//!    into its session records (`~/.codex/sessions/**/rollout-*.jsonl`,
//!    on each `token_count` event), used while it is fresh. **Only a
//!    fallback in truth**: it is whatever the last turn saw — measured
//!    2026-09-22, a 09-06 snapshot read 6% while the endpoint read 9%.
//! 2. **`wham-usage`** (remote) — the ChatGPT login Codex stored
//!    (`~/.codex/auth.json`, `auth_mode = chatgpt`) against
//!    `chatgpt.com/backend-api/wham/usage`, with `Authorization: Bearer
//!    <access_token>` and `ChatGPT-Account-Id: <account_id>`.
//!
//! **A non-public endpoint**, tried once by hand with the user's own
//! login on 2026-09-22 (HTTP 200). What came back: `plan_type`;
//! `rate_limit.{allowed, limit_reached, primary_window,
//! secondary_window}`, each window `{used_percent,
//! limit_window_seconds, reset_after_seconds, reset_at}`;
//! `credits.{has_credits, unlimited, balance}`; and blocks that were
//! empty that day (`code_review_rate_limit`, `additional_rate_limits`,
//! `model_usage`, `promo`), which are not read. Every field here is
//! optional; a shape this build cannot read is 「读不了」, never 0.
//!
//! The token is **never refreshed here**: an expired one is Codex's to
//! renew (`codex login`), the same rule Claude's login keeps.

use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::Duration;

use base64::Engine;
use khor_core::{Quota, QuotaCredits, QuotaTrouble, QuotaWindow, QuotaWindowSpan};
use serde_json::Value;

use super::{Provider, State, Step, Strategy, percent, settle, title_case, unix_seconds};

const ENDPOINT: &str = "https://chatgpt.com/backend-api/wham/usage";

/// Where the usage is asked, with the door a test uses to stand in for
/// OpenAI (`KHOR_CODEX_USAGE_URL`, the sibling of
/// `KHOR_CLAUDE_USAGE_URL`). A test door, not a setting.
fn endpoint_from(door: Option<String>) -> String {
    door.filter(|d| !d.is_empty())
        .unwrap_or_else(|| ENDPOINT.to_owned())
}

fn endpoint() -> String {
    endpoint_from(std::env::var("KHOR_CODEX_USAGE_URL").ok())
}

/// How long a session record's snapshot counts as describing now. The
/// same half hour Claude's statusline file gets: past it the snapshot is
/// skipped, not served stale — it would read as the present.
const LOCAL_FRESH: i64 = 30 * 60;

/// How much of the newest record is read looking for the last snapshot.
/// A turn writes one every time it counts tokens, so the last one is
/// near the end.
const TAIL: u64 = 512 * 1024;

static STATE: State = State::new();

/// Codex, described (`super::Provider`).
pub static PROVIDER: Provider = Provider {
    id: "codex",
    vendor: "Codex",
    command: "codex",
    logged_in,
    strategies,
    finish,
    state: &STATE,
};

fn strategies() -> Vec<Strategy> {
    vec![
        Strategy {
            name: "rollout",
            step: Step::Local,
            available: Box::new(|| codex_home().is_some()),
            fetch: Box::new(|| {
                Box::pin(async {
                    let home = codex_home().ok_or(QuotaTrouble::NoLogin)?;
                    let now = jiff::Timestamp::now().as_second();
                    rollout_snapshot(&home, now).ok_or(QuotaTrouble::Unreadable)
                })
            }),
        },
        Strategy {
            name: "wham-usage",
            step: Step::Remote,
            available: Box::new(logged_in),
            fetch: Box::new(|| {
                Box::pin(async {
                    let home = codex_home().ok_or(QuotaTrouble::NoLogin)?;
                    let login = login_at(&home)?;
                    fetch(&endpoint(), &login).await
                })
            }),
        },
    ]
}

/// The plan: what the endpoint said if it said one, else what the login
/// itself names (the id token's `chatgpt_plan_type`) — no request.
fn finish(quota: &mut Quota) {
    if quota.plan.is_none() {
        quota.plan = codex_home()
            .and_then(|h| login_at(&h).ok())
            .and_then(|l| l.plan);
    }
}

/// `~/.codex`, where Codex keeps its login and its records.
///
/// **`$HOME`'s, never `$CODEX_HOME`'s.** A gate hands every process a
/// scratch `$HOME`; an environment variable pointing somewhere else would
/// walk a gate straight back into the operator's own login. A person who
/// moved Codex's home is not read yet, which is said rather than guessed.
fn codex_home() -> Option<PathBuf> {
    // **A tripwire for this crate's own tests**: they run with the
    // operator's `HOME`, and a test that got here would be reading the
    // operator's Codex login (`quota::claude`'s twin).
    #[cfg(test)]
    if super::as_the_account() {
        panic!("a unit test reached the operator's Codex login (quota::codex)");
    }
    Some(std::env::home_dir()?.join(".codex"))
}

/// Whether this machine holds a ChatGPT login for Codex — **no network**.
/// An API-key login (`auth_mode = apikey`) is not a subscription.
pub fn logged_in() -> bool {
    codex_home().is_some_and(|h| login_at(&h).is_ok())
}

/// What `auth.json` holds that this reads.
#[derive(Debug, Clone, PartialEq)]
struct Login {
    access_token: String,
    account_id: Option<String>,
    plan: Option<String>,
}

/// The ChatGPT login under `home` (a `.codex` directory).
fn login_at(home: &Path) -> Result<Login, QuotaTrouble> {
    let raw = std::fs::read_to_string(home.join("auth.json")).map_err(|_| QuotaTrouble::NoLogin)?;
    let json: Value = serde_json::from_str(&raw).map_err(|_| QuotaTrouble::NoLogin)?;
    if let Some(mode) = json.get("auth_mode").and_then(Value::as_str)
        && !mode.eq_ignore_ascii_case("chatgpt")
    {
        return Err(QuotaTrouble::NoLogin);
    }
    let tokens = json.get("tokens").ok_or(QuotaTrouble::NoLogin)?;
    let access_token = tokens
        .get("access_token")
        .and_then(Value::as_str)
        .filter(|t| !t.is_empty())
        .ok_or(QuotaTrouble::NoLogin)?
        .to_owned();
    let claims = |jwt: &str| -> Option<Value> {
        let body = jwt.split('.').nth(1)?;
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(body.trim_end_matches('='))
            .ok()?;
        serde_json::from_slice(&bytes).ok()
    };
    let id = tokens
        .get("id_token")
        .and_then(Value::as_str)
        .and_then(claims);
    let auth = |c: &Value| c.get("https://api.openai.com/auth").cloned();
    let account_id = tokens
        .get("account_id")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .or_else(|| {
            id.as_ref()
                .and_then(auth)?
                .get("chatgpt_account_id")?
                .as_str()
                .map(str::to_owned)
        });
    let plan = id
        .as_ref()
        .and_then(auth)
        .and_then(|a| a.get("chatgpt_plan_type")?.as_str().map(title_case));
    Ok(Login {
        access_token,
        account_id,
        plan,
    })
}

/// When the access token expires, from its own `exp`, where it says.
fn expires_at(token: &str) -> Option<i64> {
    let body = token.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(body.trim_end_matches('='))
        .ok()?;
    serde_json::from_slice::<Value>(&bytes)
        .ok()?
        .get("exp")?
        .as_i64()
}

async fn fetch(url: &str, login: &Login) -> Result<Quota, QuotaTrouble> {
    let now = jiff::Timestamp::now().as_second();
    if expires_at(&login.access_token).is_some_and(|exp| exp < now) {
        return Err(QuotaTrouble::Stale);
    }
    let mut ask = reqwest::Client::new()
        .get(url)
        .bearer_auth(&login.access_token)
        .timeout(Duration::from_secs(15));
    if let Some(id) = &login.account_id {
        ask = ask.header("ChatGPT-Account-Id", id);
    }
    let response = ask.send().await.map_err(|_| QuotaTrouble::Unreachable)?;
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
    parse_usage(&json, now).ok_or(QuotaTrouble::Unreadable)
}

/// A window's length, in the ring's words where it has one.
fn span_of(seconds: Option<i64>) -> QuotaWindowSpan {
    match seconds {
        Some(18_000) => QuotaWindowSpan::FiveHour,
        Some(604_800) => QuotaWindowSpan::SevenDay,
        Some(s) if s > 0 && s % 86_400 == 0 => QuotaWindowSpan::Other(format!("{}d", s / 86_400)),
        Some(s) if s > 0 && s % 3_600 == 0 => QuotaWindowSpan::Other(format!("{}h", s / 3_600)),
        Some(s) if s > 0 => QuotaWindowSpan::Other(format!("{}m", s / 60)),
        _ => QuotaWindowSpan::Other(String::new()),
    }
}

/// Shortest first, a length khor cannot order last — the order
/// `Quota::windows` promises.
fn seat(w: &QuotaWindow) -> u8 {
    match w.span {
        QuotaWindowSpan::FiveHour => 0,
        QuotaWindowSpan::SevenDay => 1,
        QuotaWindowSpan::Other(_) => 2,
    }
}

/// One window, from either shape: the endpoint's (`used_percent`,
/// `limit_window_seconds`, `reset_at` / `reset_after_seconds`) or a
/// session record's (`used_percent`, `window_minutes`, `resets_at` /
/// `resets_in_seconds`). `at` is when the relative reset was measured
/// from. A window with no readable percentage is not a window.
fn window(v: &Value, at: i64, now: i64) -> Option<QuotaWindow> {
    if v.is_null() {
        return None;
    }
    let used = v.get("used_percent").and_then(percent)?;
    let length = v
        .get("limit_window_seconds")
        .and_then(Value::as_i64)
        .or_else(|| {
            v.get("window_minutes")
                .and_then(Value::as_i64)
                .map(|m| m * 60)
        });
    let resets_at = ["reset_at", "resets_at"]
        .iter()
        .find_map(|k| v.get(*k).and_then(unix_seconds))
        .or_else(|| {
            ["reset_after_seconds", "resets_in_seconds"]
                .iter()
                .find_map(|k| v.get(*k).and_then(Value::as_i64))
                .map(|s| at + s)
        });
    let (used_pct, resets_at) = settle(used, resets_at, now);
    Some(QuotaWindow {
        span: span_of(length),
        scope: None,
        used_pct,
        resets_at,
        active: None,
        severity: None,
    })
}

fn ordered(mut windows: Vec<QuotaWindow>) -> Option<Vec<QuotaWindow>> {
    windows.sort_by_key(seat);
    (!windows.is_empty()).then_some(windows)
}

/// The endpoint's answer. `None` when no window can be read — which is
/// 「读不了」 for the row, never a zero.
fn parse_usage(v: &Value, now: i64) -> Option<Quota> {
    let limits = v.get("rate_limit").filter(|l| l.is_object())?;
    let windows = ordered(
        ["primary_window", "secondary_window"]
            .iter()
            .filter_map(|k| window(limits.get(*k)?, now, now))
            .collect(),
    )?;
    let credits = v.get("credits").filter(|c| c.is_object()).map(|c| {
        let has = c
            .get("has_credits")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        QuotaCredits {
            enabled: has,
            disabled_reason: None,
            used: None,
            balance: c.get("balance").and_then(|b| {
                b.as_f64()
                    .or_else(|| b.as_str().and_then(|s| s.trim().parse().ok()))
            }),
            monthly_limit: None,
            currency: None,
            ever_enabled: has,
        }
    });
    Some(Quota {
        windows,
        as_of: None,
        plan: v
            .get("plan_type")
            .and_then(Value::as_str)
            .filter(|p| !p.is_empty())
            .map(title_case),
        credits,
        fill: None,
        source: None,
        limit_reached: limits.get("limit_reached").and_then(Value::as_bool),
    })
}

/// The newest session record under `home/sessions` (`YYYY/MM/DD/`
/// directories, `rollout-<time>-<id>.jsonl` names that sort by time).
fn newest_rollout(home: &Path) -> Option<PathBuf> {
    let newest_in = |dir: &Path, want_dir: bool| -> Option<PathBuf> {
        let mut names: Vec<PathBuf> = std::fs::read_dir(dir)
            .ok()?
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir() == want_dir)
            .collect();
        names.sort();
        names.pop()
    };
    let mut dir = home.join("sessions");
    for _ in 0..3 {
        dir = newest_in(&dir, true)?;
    }
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("rollout-") && n.ends_with(".jsonl"))
        })
        .collect();
    files.sort();
    files.pop()
}

/// The last `rate_limits` snapshot in the newest session record, when it
/// was taken less than [`LOCAL_FRESH`] ago.
fn rollout_snapshot(home: &Path, now: i64) -> Option<Quota> {
    let path = newest_rollout(home)?;
    let mut f = std::fs::File::open(&path).ok()?;
    let len = f.metadata().ok()?.len();
    f.seek(SeekFrom::Start(len.saturating_sub(TAIL))).ok()?;
    let mut text = String::new();
    let mut bytes = Vec::new();
    f.read_to_end(&mut bytes).ok()?;
    text.push_str(&String::from_utf8_lossy(&bytes));
    for line in text.lines().rev() {
        if !line.contains("\"rate_limits\"") {
            continue;
        }
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let Some(limits) = v.get("payload").and_then(|p| p.get("rate_limits")) else {
            continue;
        };
        let windows: Vec<QuotaWindow> = ["primary", "secondary"]
            .iter()
            .filter_map(|k| limits.get(*k))
            .filter_map(|w| {
                let at = v.get("timestamp").and_then(unix_seconds).unwrap_or(now);
                window(w, at, now)
            })
            .collect();
        // An empty snapshot (`{}` — a turn with no limits reported) is
        // not a reading; an older line may still be one.
        let Some(windows) = ordered(windows) else {
            continue;
        };
        let taken = v.get("timestamp").and_then(unix_seconds)?;
        if now - taken > LOCAL_FRESH {
            return None;
        }
        return Some(Quota {
            windows,
            as_of: Some(taken),
            plan: limits
                .get("plan_type")
                .and_then(Value::as_str)
                .map(title_case),
            credits: None,
            fill: None,
            source: None,
            limit_reached: None,
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    use super::*;

    /// The endpoint's answer as it came back on 2026-09-22, numbers
    /// changed, nothing of the account in it.
    fn usage() -> Value {
        serde_json::json!({
            "plan_type": "plus",
            "rate_limit": {
                "allowed": true,
                "limit_reached": false,
                "primary_window": {"used_percent": 9, "limit_window_seconds": 18000, "reset_after_seconds": 3600, "reset_at": 4102444800i64},
                "secondary_window": {"used_percent": 31, "limit_window_seconds": 604800, "reset_after_seconds": 86400, "reset_at": 4102531200i64}
            },
            "credits": {"has_credits": false, "unlimited": false, "balance": "0"},
            "code_review_rate_limit": null,
            "additional_rate_limits": null,
            "model_usage": null,
            "promo": null
        })
    }

    #[test]
    fn the_endpoints_shape_reads() {
        let q = parse_usage(&usage(), 1_700_000_000).expect("readable");
        assert_eq!(
            q.windows
                .iter()
                .map(|w| (w.span.clone(), w.used_pct))
                .collect::<Vec<_>>(),
            vec![
                (QuotaWindowSpan::FiveHour, 9.0),
                (QuotaWindowSpan::SevenDay, 31.0)
            ]
        );
        assert_eq!(q.windows[0].resets_at, Some(4_102_444_800));
        assert_eq!(q.plan.as_deref(), Some("Plus"));
        assert_eq!(q.limit_reached, Some(false));
        assert_eq!(q.credits.unwrap().balance, Some(0.0));
    }

    /// **A shape this build cannot read is 「读不了」, never a zero and
    /// never a panic**: a field missing, fields added, a field of the
    /// wrong type.
    #[test]
    fn a_shape_it_cannot_read_is_unreadable_not_zero() {
        let now = 1_700_000_000;
        assert!(parse_usage(&serde_json::json!({}), now).is_none());
        assert!(parse_usage(&serde_json::json!({"rate_limit": null}), now).is_none());
        assert!(parse_usage(&serde_json::json!({"rate_limit": "full"}), now).is_none());
        assert!(
            parse_usage(
                &serde_json::json!({"rate_limit": {"primary_window": {"used_percent": "lots"}}}),
                now
            )
            .is_none()
        );
        assert!(
            parse_usage(
                &serde_json::json!({"rate_limit": {"primary_window": [1, 2]}}),
                now
            )
            .is_none()
        );
        // More than it knows is fine: what it knows still reads.
        let mut more = usage();
        more["rate_limit"]["tertiary_window"] = serde_json::json!({"used_percent": 1});
        more["something_new"] = serde_json::json!({"a": [1, 2, 3]});
        assert_eq!(parse_usage(&more, now).unwrap().windows.len(), 2);
        // A reset relative to now when no absolute one is given.
        let rel = serde_json::json!({"rate_limit": {"primary_window": {"used_percent": 5, "limit_window_seconds": 18000, "reset_after_seconds": 60}}});
        assert_eq!(
            parse_usage(&rel, now).unwrap().windows[0].resets_at,
            Some(now + 60)
        );
    }

    fn jwt(claims: Value) -> String {
        let b = |v: &Value| {
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(serde_json::to_vec(v).unwrap())
        };
        format!(
            "{}.{}.sig",
            b(&serde_json::json!({"alg": "none"})),
            b(&claims)
        )
    }

    fn scratch(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("khor-codex-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn the_chatgpt_login_is_read_and_an_api_key_login_is_not_a_subscription() {
        let h = scratch("login");
        let id = jwt(
            serde_json::json!({"https://api.openai.com/auth": {"chatgpt_plan_type": "pro", "chatgpt_account_id": "acc-from-id"}}),
        );
        std::fs::write(
            h.join("auth.json"),
            serde_json::json!({"auth_mode": "chatgpt", "OPENAI_API_KEY": null, "tokens": {"id_token": id, "access_token": "tok", "refresh_token": "r", "account_id": "acc-1"}}).to_string(),
        )
        .unwrap();
        let l = login_at(&h).unwrap();
        assert_eq!(l.access_token, "tok");
        assert_eq!(l.account_id.as_deref(), Some("acc-1"));
        assert_eq!(l.plan.as_deref(), Some("Pro"));
        std::fs::write(
            h.join("auth.json"),
            serde_json::json!({"auth_mode": "apikey", "OPENAI_API_KEY": "sk-x"}).to_string(),
        )
        .unwrap();
        assert_eq!(login_at(&h), Err(QuotaTrouble::NoLogin));
        let _ = std::fs::remove_dir_all(&h);
        assert_eq!(login_at(&h), Err(QuotaTrouble::NoLogin));
    }

    /// The last non-empty snapshot in the newest record, while fresh.
    #[test]
    fn a_session_records_snapshot_is_read_while_fresh() {
        let h = scratch("rollout");
        let day = h.join("sessions/2026/09/22");
        std::fs::create_dir_all(&day).unwrap();
        std::fs::create_dir_all(h.join("sessions/2026/09/06")).unwrap();
        std::fs::write(
            h.join("sessions/2026/09/06/rollout-2026-09-06T00-00-00-x.jsonl"),
            "{\"timestamp\":\"2026-09-06T00:00:00Z\",\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\",\"rate_limits\":{\"primary\":{\"used_percent\":99}}}}\n",
        )
        .unwrap();
        let line = |ts: &str, limits: Value| {
            serde_json::json!({"timestamp": ts, "type": "event_msg", "payload": {"type": "token_count", "info": null, "rate_limits": limits}}).to_string()
        };
        let mut f = std::fs::File::create(day.join("rollout-2026-09-22T10-00-00-y.jsonl")).unwrap();
        writeln!(f, "{}", line("2026-09-22T10:00:00Z", serde_json::json!({"primary": {"used_percent": 6.0, "window_minutes": 300, "resets_in_seconds": 600}, "secondary": {"used_percent": 20.0, "window_minutes": 10080, "resets_at": 4102444800i64}}))).unwrap();
        writeln!(f, "{}", line("2026-09-22T10:05:00Z", serde_json::json!({}))).unwrap();
        let taken = "2026-09-22T10:00:00Z"
            .parse::<jiff::Timestamp>()
            .unwrap()
            .as_second();
        let q = rollout_snapshot(&h, taken + 60).expect("fresh");
        assert_eq!(q.as_of, Some(taken));
        assert_eq!(q.windows[0].span, QuotaWindowSpan::FiveHour);
        assert_eq!(q.windows[0].used_pct, 6.0);
        assert_eq!(q.windows[0].resets_at, Some(taken + 600));
        assert_eq!(q.windows[1].span, QuotaWindowSpan::SevenDay);
        assert!(
            rollout_snapshot(&h, taken + LOCAL_FRESH + 1).is_none(),
            "stale is not served"
        );
        let _ = std::fs::remove_dir_all(&h);
    }

    /// Every request asked, whatever path; answers the table's one reply.
    fn serve(status: &'static str, body: String) -> (u16, Arc<Mutex<Vec<String>>>) {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let log = seen.clone();
        std::thread::spawn(move || {
            for c in l.incoming() {
                let Ok(mut c) = c else { continue };
                let mut buf = [0u8; 4096];
                let n = c.read(&mut buf).unwrap_or(0);
                log.lock()
                    .unwrap()
                    .push(String::from_utf8_lossy(&buf[..n]).to_string());
                let _ = write!(
                    c,
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
            }
        });
        (port, seen)
    }

    fn rt() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
    }

    fn login() -> Login {
        Login {
            access_token: "fake-token".into(),
            account_id: Some("acc-1".into()),
            plan: None,
        }
    }

    /// The two headers the endpoint wants, and the reading back.
    #[test]
    fn the_endpoint_is_asked_with_the_login_and_its_account() {
        let (port, seen) = serve("200 OK", usage().to_string());
        let q = rt()
            .block_on(fetch(
                &format!("http://127.0.0.1:{port}/wham/usage"),
                &login(),
            ))
            .unwrap();
        assert_eq!(q.windows.len(), 2);
        let asked = seen.lock().unwrap()[0].to_ascii_lowercase();
        assert!(
            asked.contains("authorization: bearer fake-token"),
            "{asked}"
        );
        assert!(asked.contains("chatgpt-account-id: acc-1"), "{asked}");
    }

    /// Refused is an expired login (run `codex login`), an unreadable
    /// body is 「读不了」, and an expired token is not even sent.
    #[test]
    fn refusals_and_nonsense_are_named_not_zeroed() {
        let (port, _) = serve("401 Unauthorized", "{}".into());
        let url = format!("http://127.0.0.1:{port}/");
        assert_eq!(
            rt().block_on(fetch(&url, &login())),
            Err(QuotaTrouble::Stale)
        );
        let (port, _) = serve("200 OK", "{\"rate_limit\": 7}".into());
        let url = format!("http://127.0.0.1:{port}/");
        assert_eq!(
            rt().block_on(fetch(&url, &login())),
            Err(QuotaTrouble::Unreadable)
        );
        let (port, seen) = serve("200 OK", usage().to_string());
        let old = Login {
            access_token: jwt(serde_json::json!({"exp": 1})),
            ..login()
        };
        let url = format!("http://127.0.0.1:{port}/");
        assert_eq!(rt().block_on(fetch(&url, &old)), Err(QuotaTrouble::Stale));
        assert!(
            seen.lock().unwrap().is_empty(),
            "an expired token is not sent"
        );
    }

    #[test]
    fn the_usage_endpoint_has_a_test_door_and_a_fixed_default() {
        assert_eq!(endpoint_from(None), ENDPOINT);
        assert_eq!(
            endpoint_from(Some("http://127.0.0.1:9/u".into())),
            "http://127.0.0.1:9/u"
        );
        assert_eq!(endpoint_from(Some(String::new())), ENDPOINT);
    }

    /// **No unit test reads the operator's Codex login.**
    #[test]
    #[should_panic(expected = "reached the operator's Codex login")]
    fn unit_tests_never_reach_the_login() {
        let _ = logged_in();
    }
}
