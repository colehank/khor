//! Registered API accounts and what they have used — the 额度 page.
//!
//! # Three rules, and the first is the shape of everything else
//!
//! **A key lives on one machine.** The registration (name, provider,
//! address, budget, and *which machine*) replicates like every other
//! table (`khor_sync::accounts`); the key is written once, under
//! `.khor/keys/`, on the machine that registered it, and never leaves.
//! That machine reads the provider and answers [`Request::Quotas`]; the
//! others cache its answer with an age, the way vitals cross. So a
//! ring on any screen is "what the home machine last read", and a
//! reading that is minutes old says so rather than pretending to be
//! now.
//!
//! **The fraction is decided here.** A provider with a ceiling of its
//! own (a new-api token's remaining quota) fills against that; one that
//! bills per token fills against the budget the person registered, and
//! with no budget there is no fraction — numbers, a track, no arc. A
//! screen that worked out its own share would be a second opinion on
//! what "full" means.
//!
//! **Read on a leash.** Every provider rate-limits its usage endpoints;
//! a reading is cached for [`CACHE_TTL_MS`] and a failure cools the
//! account for [`FAIL_COOLDOWN_MS`] before it is asked again (`quota.rs`'s
//! discipline, lifted). A screen polling every ten seconds costs the
//! provider one call per five minutes.
//!
//! # Which way this can be wrong
//!
//! The vendors' usage APIs are **organisation-level admin** calls: a
//! plain `sk-ant-api…` / project key cannot read them, and the form
//! says so. A gateway can be an old new-api without
//! `/api/usage/token/` (it arrived 2025-08-24, v0.9.0-alpha.8, and only
//! reads an exhausted token since v0.10.8, 2026-02-07); that answers
//! 404 and is said as 站太老 rather than read through the older
//! `/api/user/self`, which wants a *user* token and would refuse the
//! sk-key with a sentence that blames the key. Verified 2026-09-02
//! against the docs and new-api's source (`router/api-router.go`,
//! `controller/token.go`, `middleware/auth.go`).

use std::path::PathBuf;
use std::time::Duration;

use khor_catalog::msg;
use khor_core::{AccountReading, Fill, Strain};
use khor_sync::accounts::{Account, Provider};
use serde::{Deserialize, Serialize};

use crate::link;
use crate::Node;

/// How long a reading stands before the provider is asked again.
pub const CACHE_TTL_MS: i64 = 5 * 60 * 1000;
/// How long a failed account is left alone. Longer than the cache: a
/// provider that just refused is a provider that will refuse again for
/// a while, and asking every five minutes is how a key gets throttled.
pub const FAIL_COOLDOWN_MS: i64 = 10 * 60 * 1000;
const HTTP_TIMEOUT: Duration = Duration::from_secs(15);

/// The keys [`AccountReading::trouble`] travels under.
pub mod trouble {
    /// The provider refused the key (401/403).
    pub const KEY_REFUSED: &str = "key-refused";
    /// No answer from the address.
    pub const UNREACHABLE: &str = "unreachable";
    /// An answer this reader could not read.
    pub const UNREADABLE: &str = "unreadable";
    /// Left alone after a failure with nothing on record to stand —
    /// which a failure normally leaves, so this is the rare word.
    pub const COOLING: &str = "cooling";
    /// The home machine holds no key for this account.
    pub const NO_KEY: &str = "no-key";
    /// Another machine's account, and that machine has not answered yet.
    pub const NO_READING: &str = "no-reading";
    /// A new-api too old to have `/api/usage/token/` (before v0.9.0).
    pub const GATEWAY_TOO_OLD: &str = "gateway-too-old";
}

/// One reading as kept on disk: the reading and, when the last attempt
/// failed, when it did — so the cooldown survives a restart.
#[derive(Serialize, Deserialize)]
struct Cached {
    reading: AccountReading,
    #[serde(default)]
    failed_at_ms: Option<i64>,
}

/// A peer's whole answer, stamped with when it arrived.
#[derive(Serialize, Deserialize)]
struct PeerQuotas {
    at_ms: i64,
    readings: Vec<AccountReading>,
}

/// Where a fraction sits on the strain scale — `Fill`'s own steps, so a
/// ring at 91% wears the ink a disk at 91% wears (one doctrine).
pub fn strain_of(fill: Option<f32>) -> Option<Strain> {
    let f = fill?;
    Fill { used: (f.clamp(0.0, 1.0) * 1000.0).round() as u64, total: 1000 }.strain()
}

impl Node {
    fn keys_dir(&self) -> PathBuf {
        self.root().join(".khor").join("keys")
    }

    fn quotas_dir(&self) -> PathBuf {
        self.root().join(".khor").join("quotas")
    }

    /// Every registered account, wherever its key lives.
    pub fn accounts(&self) -> Result<Vec<Account>, String> {
        Ok(self.accounts_loaded()?.doc.all())
    }

    /// Registers an account **here**: the description goes to every
    /// machine, the key to `.khor/keys/` on this one, and this machine
    /// becomes the account's home. Refused by name for a missing key or
    /// a gateway without an address; the provider word is validated by
    /// the caller (`Provider::parse`) so the CLI and the face refuse it
    /// in one voice.
    pub fn register_account(
        &self,
        name: &str,
        provider: Provider,
        base_url: Option<&str>,
        key: &str,
        budget: Option<f64>,
    ) -> Result<Account, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err(msg::ACCOUNT_NAME_BLANK.into());
        }
        let key = key.trim();
        if key.is_empty() {
            return Err(msg::ACCOUNT_KEY_BLANK.into());
        }
        let base_url = match (provider, base_url.map(str::trim).filter(|u| !u.is_empty())) {
            (Provider::Newapi, None) => return Err(msg::ACCOUNT_NEEDS_ADDRESS.into()),
            (Provider::Newapi, Some(u)) => {
                if !(u.starts_with("http://") || u.starts_with("https://")) {
                    return Err(msg::account_address_bad(u));
                }
                Some(u.trim_end_matches('/').to_owned())
            }
            // The two vendors have one address each; a registered one
            // would be a second opinion about where Anthropic is.
            (_, _) => None,
        };
        if let Some(b) = budget
            && !(b.is_finite() && b > 0.0)
        {
            return Err(msg::ACCOUNT_BUDGET_BAD.into());
        }
        let account = Account {
            id: link::fresh_hex()?,
            name: name.to_owned(),
            provider,
            base_url,
            budget,
            home: self.device_str().to_owned(),
        };
        std::fs::create_dir_all(self.keys_dir()).map_err(msg::cant_make_keys_dir)?;
        link::write_private(&self.keys_dir().join(&account.id), key.as_bytes())?;
        let loaded = self.accounts_loaded()?;
        loaded.doc.set(&account)?;
        let mut store = loaded.store;
        store.flush(&loaded.doc)?;
        Ok(account)
    }

    /// Forgets an account everywhere, and its key here if this is where
    /// it lived. A key on another machine stays on that disk until it
    /// is removed there: nothing here can reach it, which is the point
    /// of where it was put.
    pub fn remove_account(&self, id: &str) -> Result<(), String> {
        let loaded = self.accounts_loaded()?;
        if loaded.doc.get(id).is_none() {
            return Err(msg::no_such_account(id));
        }
        loaded.doc.remove(id)?;
        let mut store = loaded.store;
        store.flush(&loaded.doc)?;
        let _ = std::fs::remove_file(self.keys_dir().join(id));
        let _ = std::fs::remove_file(self.quotas_dir().join(format!("{id}.json")));
        Ok(())
    }

    fn key_of(&self, id: &str) -> Option<String> {
        std::fs::read_to_string(self.keys_dir().join(id)).ok().map(|k| k.trim().to_owned())
    }

    fn cached_reading(&self, id: &str) -> Option<Cached> {
        let text = std::fs::read_to_string(self.quotas_dir().join(format!("{id}.json"))).ok()?;
        serde_json::from_str(&text).ok()
    }

    fn cache_reading(&self, id: &str, cached: &Cached) {
        let _ = std::fs::create_dir_all(self.quotas_dir());
        if let Ok(bytes) = serde_json::to_vec(cached) {
            let _ = link::write_private(&self.quotas_dir().join(format!("{id}.json")), &bytes);
        }
    }

    /// This machine's reading of every account whose key it holds —
    /// what it answers [`Request::Quotas`] with. Cached and cooled per
    /// account (module head); accounts homed elsewhere are not in it.
    pub async fn quota_readings(&self) -> Result<Vec<AccountReading>, String> {
        let me = self.device_str().to_owned();
        let mine: Vec<Account> =
            self.accounts()?.into_iter().filter(|a| a.home == me).collect();
        let now = crate::live::now_ms();
        let mut out = Vec::with_capacity(mine.len());
        for account in mine {
            out.push(self.read_one(&account, now).await);
        }
        Ok(out)
    }

    async fn read_one(&self, account: &Account, now: i64) -> AccountReading {
        let cached = self.cached_reading(&account.id);
        if let Some(c) = &cached
            && c.reading.trouble.is_none()
            && now - c.reading.at_ms < CACHE_TTL_MS
        {
            return c.reading.clone();
        }
        // Cooling: the last reading stands — a number, or the trouble
        // that started the cooldown. **The trouble is kept, not
        // replaced by「cooling」**: the first cut of this swapped it on
        // the second read, so a refused key read as 钥匙被拒 for one
        // beat and 歇一会 for the next ten minutes, and the screen lost
        // the one word a person could act on (found by the smoke).
        if let Some(failed) = cached.as_ref().and_then(|c| c.failed_at_ms)
            && now - failed < FAIL_COOLDOWN_MS
        {
            return match cached {
                Some(c) => c.reading,
                None => bare(account, Some(trouble::COOLING), now),
            };
        }
        let Some(key) = self.key_of(&account.id) else {
            return bare(account, Some(trouble::NO_KEY), now);
        };
        match fetch(account, &key, now).await {
            Ok(reading) => {
                self.cache_reading(&account.id, &Cached { reading: reading.clone(), failed_at_ms: None });
                reading
            }
            Err(why) => {
                // The last good reading stands, aged; the failure only
                // starts the cooldown. With nothing to stand, the
                // trouble is the reading.
                let kept = cached.filter(|c| c.reading.trouble.is_none()).map(|c| c.reading);
                let reading = kept.unwrap_or_else(|| bare(account, Some(why), now));
                self.cache_reading(&account.id, &Cached { reading: reading.clone(), failed_at_ms: Some(now) });
                reading
            }
        }
    }

    /// Remembers a peer's answer, stamped with now (`cache_peer_vitals`'s
    /// shape: its own file, its own age).
    pub(crate) fn cache_peer_quotas(
        &self,
        device_id: &str,
        readings: &[AccountReading],
    ) -> Result<(), String> {
        if device_id.len() != 64 || !device_id.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(msg::not_a_machine_id(format_args!("{device_id:?}")));
        }
        std::fs::create_dir_all(self.quotas_dir()).map_err(msg::cant_make_vitals_dir)?;
        let report = PeerQuotas { at_ms: crate::live::now_ms(), readings: readings.to_vec() };
        link::write_private(
            &self.quotas_dir().join(format!("peer-{device_id}.json")),
            &serde_json::to_vec(&report).map_err(|e| e.to_string())?,
        )
    }

    fn peer_quotas(&self, device_id: &str) -> Option<(Vec<AccountReading>, u64)> {
        let text =
            std::fs::read_to_string(self.quotas_dir().join(format!("peer-{device_id}.json"))).ok()?;
        let report: PeerQuotas = serde_json::from_str(&text).ok()?;
        Some((report.readings, crate::live::now_ms().saturating_sub(report.at_ms) as u64))
    }

    /// One reading per registered account, wherever it lives: this
    /// machine's read now, every other machine's as last carried here
    /// (`age_ms` says how long ago). An account whose home has never
    /// answered is on the board too, as 问不出来 — a registration that
    /// vanished from the page because its machine is off would be the
    /// page lying about what is registered.
    pub async fn quota_board(&self) -> Result<Vec<(AccountReading, u64)>, String> {
        let me = self.device_str().to_owned();
        let all = self.accounts()?;
        let mine = self.quota_readings().await?;
        let now = crate::live::now_ms();
        let mut out = Vec::with_capacity(all.len());
        for account in all {
            if account.home == me {
                if let Some(r) = mine.iter().find(|r| r.account == account.id) {
                    out.push((r.clone(), 0));
                }
                continue;
            }
            let carried = self
                .peer_quotas(&account.home)
                .and_then(|(rs, age)| rs.into_iter().find(|r| r.account == account.id).map(|r| (r, age)));
            out.push(carried.unwrap_or_else(|| (bare(&account, Some(trouble::NO_READING), now), 0)));
        }
        Ok(out)
    }
}

/// A reading with no numbers: the account named, the trouble said.
fn bare(account: &Account, trouble: Option<&str>, now: i64) -> AccountReading {
    AccountReading {
        account: account.id.clone(),
        name: account.name.clone(),
        provider: account.provider.word().to_owned(),
        home: account.home.clone(),
        fill: None,
        spent: None,
        limit: None,
        currency: None,
        unlimited: false,
        resets_at: None,
        trouble: trouble.map(str::to_owned),
        at_ms: now,
    }
}

/// Spend against a registered budget: the fraction the two vendors get,
/// since neither has a ceiling of its own to fill against.
fn against_budget(mut r: AccountReading, spent: f64, budget: Option<f64>, currency: &str) -> AccountReading {
    r.spent = Some(spent);
    r.currency = Some(currency.to_owned());
    if let Some(b) = budget {
        r.limit = Some(b);
        r.fill = Some((spent / b).clamp(0.0, 1.0) as f32);
    }
    r
}

async fn fetch(account: &Account, key: &str, now: i64) -> Result<AccountReading, &'static str> {
    match account.provider {
        Provider::Newapi => fetch_newapi(account, key, now).await,
        Provider::Claude => fetch_claude(account, key, now).await,
        Provider::Openai => fetch_openai(account, key, now).await,
    }
}

fn client() -> reqwest::Client {
    reqwest::Client::builder().timeout(HTTP_TIMEOUT).build().unwrap_or_default()
}

/// One GET with the provider's headers, sorted into the three troubles
/// a person can act on differently.
async fn get_json(
    req: reqwest::RequestBuilder,
) -> Result<Result<serde_json::Value, u16>, &'static str> {
    let response = req.send().await.map_err(|_| trouble::UNREACHABLE)?;
    let status = response.status().as_u16();
    match status {
        401 | 403 => Err(trouble::KEY_REFUSED),
        200..=299 => response.json().await.map(Ok).map_err(|_| trouble::UNREADABLE),
        code => Ok(Err(code)),
    }
}

/// new-api: the token's own key reads its own quota
/// (`GET {base}/api/usage/token/`, `Authorization: Bearer sk-…`), in the
/// gateway's raw units (`total_used`, `total_available`,
/// `unlimited_quota`); `GET {base}/api/status` says how many units make
/// a dollar (`data.quota_per_unit`, 500000 by default). The window is
/// the token's whole life — there is no reset, only `expires_at`. An
/// older gateway answers 404 on the token road and is said so.
async fn fetch_newapi(account: &Account, key: &str, now: i64) -> Result<AccountReading, &'static str> {
    let base = account.base_url.as_deref().ok_or(trouble::UNREADABLE)?;
    let c = client();
    let per_unit = get_json(c.get(format!("{base}/api/status")))
        .await
        .ok()
        .and_then(Result::ok)
        .and_then(|j| j.pointer("/data/quota_per_unit").and_then(serde_json::Value::as_f64))
        .filter(|u| *u > 0.0)
        .unwrap_or(500_000.0);
    let answer = get_json(c.get(format!("{base}/api/usage/token/")).bearer_auth(key)).await?;
    let (used, available, unlimited) = match answer {
        Ok(j) => {
            let d = j.get("data").ok_or(trouble::UNREADABLE)?;
            (
                d.get("total_used").and_then(serde_json::Value::as_f64).unwrap_or(0.0),
                d.get("total_available").and_then(serde_json::Value::as_f64),
                d.get("unlimited_quota").and_then(serde_json::Value::as_bool).unwrap_or(false),
            )
        }
        Err(404) => return Err(trouble::GATEWAY_TOO_OLD),
        Err(_) => return Err(trouble::UNREACHABLE),
    };
    let mut r = bare(account, None, now);
    r.currency = Some("USD".to_owned());
    r.spent = Some(used / per_unit);
    r.unlimited = unlimited;
    if !unlimited && let Some(avail) = available {
        let total = used + avail;
        r.limit = Some(total / per_unit);
        r.fill = Some(if total > 0.0 { (used / total).clamp(0.0, 1.0) as f32 } else { 0.0 });
    }
    Ok(r)
}

/// The first instant of this month, UTC, unix seconds — the window the
/// two vendors' cost reports are asked over.
fn month_start(now_ms: i64) -> i64 {
    let ts = jiff::Timestamp::from_millisecond(now_ms).unwrap_or(jiff::Timestamp::UNIX_EPOCH);
    let zoned = ts.to_zoned(jiff::tz::TimeZone::UTC);
    zoned
        .first_of_month()
        .ok()
        .and_then(|d| d.start_of_day().ok())
        .map(|d| d.timestamp().as_second())
        .unwrap_or(now_ms / 1000)
}

/// Anthropic: the organisation's cost report through an **admin** key
/// (`GET /v1/organizations/cost_report`, `x-api-key`,
/// `anthropic-version: 2023-06-01`), summed for the month and set against
/// the registered budget. Amounts come back in cents as decimal strings.
async fn fetch_claude(account: &Account, key: &str, now: i64) -> Result<AccountReading, &'static str> {
    let start = jiff::Timestamp::from_second(month_start(now))
        .map_err(|_| trouble::UNREADABLE)?
        .to_string();
    let mut url = format!(
        "https://api.anthropic.com/v1/organizations/cost_report?starting_at={start}&bucket_width=1d&limit=31"
    );
    let mut cents = 0.0;
    let c = client();
    loop {
        let j = get_json(
            c.get(&url).header("x-api-key", key).header("anthropic-version", "2023-06-01"),
        )
        .await?
        .map_err(|_| trouble::UNREACHABLE)?;
        for bucket in j.get("data").and_then(serde_json::Value::as_array).into_iter().flatten() {
            for r in bucket.get("results").and_then(serde_json::Value::as_array).into_iter().flatten() {
                if let Some(a) = r.get("amount") {
                    cents += a.as_f64().or_else(|| a.as_str().and_then(|s| s.parse().ok())).unwrap_or(0.0);
                }
            }
        }
        match j.get("next_page").and_then(serde_json::Value::as_str) {
            Some(page) if j.get("has_more").and_then(serde_json::Value::as_bool) == Some(true) => {
                url = format!(
                    "https://api.anthropic.com/v1/organizations/cost_report?starting_at={start}&bucket_width=1d&limit=31&page={page}"
                );
            }
            _ => break,
        }
    }
    Ok(against_budget(bare(account, None, now), cents / 100.0, account.budget, "USD"))
}

/// OpenAI: the organisation's costs through an **admin** key
/// (`GET /v1/organization/costs?start_time=…&bucket_width=1d`,
/// `amount.value` in dollars), summed for the month and set against the
/// organisation's own monthly spend limit where one is set
/// (`GET /v1/organization/spend_limit`, `threshold_amount` in **cents**,
/// `interval: "month"` — public since 2026-08-21; the answer carries no
/// spend of its own, which is why the costs are summed here) and
/// otherwise against the registered budget. What an unset limit answers
/// is undocumented, so anything but a threshold is read as "none".
async fn fetch_openai(account: &Account, key: &str, now: i64) -> Result<AccountReading, &'static str> {
    let start = month_start(now);
    let c = client();
    let limit_usd = get_json(c.get("https://api.openai.com/v1/organization/spend_limit").bearer_auth(key))
        .await
        .ok()
        .and_then(Result::ok)
        .and_then(|j| {
            let cents = j.get("threshold_amount").and_then(serde_json::Value::as_f64)?;
            (j.get("interval").and_then(serde_json::Value::as_str) == Some("month") && cents > 0.0)
                .then_some(cents / 100.0)
        });
    let mut url =
        format!("https://api.openai.com/v1/organization/costs?start_time={start}&bucket_width=1d&limit=31");
    let mut dollars = 0.0;
    loop {
        let j = get_json(c.get(&url).bearer_auth(key)).await?.map_err(|_| trouble::UNREACHABLE)?;
        for bucket in j.get("data").and_then(serde_json::Value::as_array).into_iter().flatten() {
            for r in bucket.get("results").and_then(serde_json::Value::as_array).into_iter().flatten() {
                dollars += r.pointer("/amount/value").and_then(serde_json::Value::as_f64).unwrap_or(0.0);
            }
        }
        match j.get("next_page").and_then(serde_json::Value::as_str) {
            Some(page) if j.get("has_more").and_then(serde_json::Value::as_bool) == Some(true) => {
                url = format!(
                    "https://api.openai.com/v1/organization/costs?start_time={start}&bucket_width=1d&limit=31&page={page}"
                );
            }
            _ => break,
        }
    }
    // The organisation's own ceiling outranks a registered budget: it
    // is the one that actually stops requests (429 past it).
    Ok(against_budget(bare(account, None, now), dollars, limit_usd.or(account.budget), "USD"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The fraction and its strain are this side's judgment, on
    /// `Fill`'s own steps: 91% of a budget wears what 91% of a disk
    /// wears, and no budget is no fraction rather than zero.
    #[test]
    fn a_budget_makes_a_fraction_and_no_budget_makes_none() {
        let acc = Account {
            id: "a".into(),
            name: "n".into(),
            provider: Provider::Openai,
            base_url: None,
            budget: Some(100.0),
            home: "h".into(),
        };
        let r = against_budget(bare(&acc, None, 0), 91.0, acc.budget, "USD");
        assert_eq!(r.fill, Some(0.91));
        assert_eq!(strain_of(r.fill), Some(Strain::Tight));
        let r = against_budget(bare(&acc, None, 0), 91.0, None, "USD");
        assert_eq!(r.fill, None, "no denominator, no fraction");
        assert_eq!(r.spent, Some(91.0), "but the number is still said");
        assert_eq!(strain_of(None), None);
    }

    /// **A cooled account keeps the reason it failed.** The cache path
    /// alone, no HTTP: a refused key is on record with its failure time,
    /// and the read inside the cooldown must hand that reading back —
    /// not a fresh「cooling」that hides which key was refused. The
    /// second arm is the control: a *good* reading on record survives
    /// the cooldown the same way.
    #[test]
    fn a_cooled_account_keeps_the_trouble_it_failed_with() {
        let root = std::env::temp_dir().join(format!("khor-accounts-cool-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let n = Node::open(root.clone()).unwrap();
        let a = n
            .register_account("gw", Provider::Newapi, Some("http://127.0.0.1:1"), "sk-x", None)
            .unwrap();
        let now = crate::live::now_ms();
        let mut refused = bare(&a, Some(trouble::KEY_REFUSED), now - 1_000);
        refused.at_ms = now - 1_000;
        n.cache_reading(&a.id, &Cached { reading: refused, failed_at_ms: Some(now - 1_000) });
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let got = rt.block_on(n.quota_readings()).unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].trouble.as_deref(), Some(trouble::KEY_REFUSED), "the reason stands");

        let mut fine = bare(&a, None, now - 1_000);
        fine.fill = Some(0.45);
        n.cache_reading(&a.id, &Cached { reading: fine, failed_at_ms: Some(now - 1_000) });
        let got = rt.block_on(n.quota_readings()).unwrap();
        assert_eq!(got[0].fill, Some(0.45), "control: a good reading on record stands too");
        assert_eq!(got[0].trouble, None);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The month window starts at the first instant of the month, UTC.
    #[test]
    fn the_window_is_the_month_so_far() {
        // 2026-09-02T00:30:00Z
        let now_ms = 1_788_309_000_000;
        let start = month_start(now_ms);
        let z = jiff::Timestamp::from_second(start).unwrap().to_zoned(jiff::tz::TimeZone::UTC);
        assert_eq!((z.year(), z.month(), z.day(), z.hour()), (2026, 9, 1, 0));
    }
}
