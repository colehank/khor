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
    /// A provider word from a newer khor (`Provider::Other`): carried
    /// and shown, but this build has no reader for it. Only the home
    /// machine reads, so this is the home machine having been
    /// downgraded past its own registration.
    pub const UNKNOWN_PROVIDER: &str = "unknown-provider";
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
        let base_url = match (&provider, base_url.map(str::trim).filter(|u| !u.is_empty())) {
            (Provider::Newapi, None) => return Err(msg::ACCOUNT_NEEDS_ADDRESS.into()),
            (Provider::Newapi, Some(u)) => {
                if !(u.starts_with("http://") || u.starts_with("https://")) {
                    return Err(msg::account_address_bad(u));
                }
                Some(u.trim_end_matches('/').to_owned())
            }
            // The vendors have one address each; a registered one would
            // be a second opinion about where Anthropic is.
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
            // Nothing arrives pinned: the top of the list is somebody's
            // choice, and a new row claiming it would be this code
            // making that choice for them.
            pinned: false,
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

    /// Floats an account to the top of the 額度 list, or drops it back.
    ///
    /// **Everywhere, not here**: the flag rides the replicated doc, so
    /// pinning on one machine pins on all of them — the same bargain a
    /// pinned machine makes. Refused by name for an id nobody has, so
    /// a stale row cannot silently pin nothing.
    pub fn pin_account(&self, id: &str, on: bool) -> Result<(), String> {
        let loaded = self.accounts_loaded()?;
        let mut account = loaded.doc.get(id).ok_or_else(|| msg::no_such_account(id))?;
        account.pinned = on;
        loaded.doc.set(&account)?;
        let mut store = loaded.store;
        store.flush(&loaded.doc)?;
        Ok(())
    }

    /// Edits a registered account: its name, its address, its budget,
    /// and **optionally its key**.
    ///
    /// # What cannot be edited, and why it is not an oversight
    ///
    /// **The provider.** It decides how this account is read at all —
    /// which endpoint, which header, which shape of answer — so
    /// changing it is not editing this account, it is registering a
    /// different one under an old id, with the old cached reading still
    /// on disk beside it.
    ///
    /// **The home.** The key lives on one machine and only that machine
    /// reads the account (the module head). Moving the home would mean
    /// moving a secret between machines, which is the one thing this
    /// design does not do.
    ///
    /// `key` of `None` leaves the stored key alone — an edit of the
    /// name is not a reason to make somebody paste a secret again. A
    /// key is written **only on the machine that holds it**: editing
    /// from anywhere else is refused by name rather than quietly
    /// dropping the new key, which would read as having worked.
    pub fn update_account(
        &self,
        id: &str,
        name: &str,
        base_url: Option<&str>,
        key: Option<&str>,
        budget: Option<f64>,
    ) -> Result<(), String> {
        let loaded = self.accounts_loaded()?;
        let mut account = loaded.doc.get(id).ok_or_else(|| msg::no_such_account(id))?;
        let name = name.trim();
        if name.is_empty() {
            return Err(msg::ACCOUNT_NAME_BLANK.into());
        }
        // The same two checks registration makes, in the same order and
        // with the same sentences — an edit that accepted an address
        // registration refuses would be a second opinion about what a
        // gateway is.
        let base_url = match (&account.provider, base_url.map(str::trim).filter(|u| !u.is_empty())) {
            (Provider::Newapi, None) => return Err(msg::ACCOUNT_NEEDS_ADDRESS.into()),
            (Provider::Newapi, Some(u)) => {
                if !(u.starts_with("http://") || u.starts_with("https://")) {
                    return Err(msg::account_address_bad(u));
                }
                Some(u.trim_end_matches('/').to_owned())
            }
            (_, _) => None,
        };
        if let Some(b) = budget
            && !(b.is_finite() && b > 0.0)
        {
            return Err(msg::ACCOUNT_BUDGET_BAD.into());
        }
        if let Some(key) = key.map(str::trim).filter(|k| !k.is_empty()) {
            if account.home != self.device_str() {
                return Err(msg::account_key_elsewhere(&account.name));
            }
            std::fs::create_dir_all(self.keys_dir()).map_err(msg::cant_make_keys_dir)?;
            link::write_private(&self.keys_dir().join(&account.id), key.as_bytes())?;
        }
        account.name = name.to_owned();
        account.base_url = base_url;
        account.budget = budget;
        loaded.doc.set(&account)?;
        let mut store = loaded.store;
        store.flush(&loaded.doc)?;
        // The reading on disk was taken with the old budget, and the
        // budget is the denominator — leaving it would draw a fraction
        // of a ceiling that no longer exists until the cache expires.
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
        // the second read, so a refused key read as 密钥被拒 for one
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
        remaining: None,
        expires_at: None,
        models: Vec::new(),
        days: Vec::new(),
        spent_this_month: None,
        spent_all_time: None,
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
    match &account.provider {
        Provider::Newapi => fetch_newapi(account, key, now).await,
        Provider::Claude => fetch_claude(account, key, now).await,
        Provider::Openai => fetch_openai(account, key, now).await,
        Provider::Deepseek => {
            let j = get_json(client().get("https://api.deepseek.com/user/balance").bearer_auth(key))
                .await?
                .map_err(|_| trouble::UNREACHABLE)?;
            read_deepseek(account, &j, now)
        }
        Provider::Moonshot => {
            let j = get_json(client().get("https://api.moonshot.ai/v1/users/me/balance").bearer_auth(key))
                .await?
                .map_err(|_| trouble::UNREACHABLE)?;
            read_moonshot(account, &j, now, "USD")
        }
        Provider::MoonshotCn => {
            let j = get_json(client().get("https://api.moonshot.cn/v1/users/me/balance").bearer_auth(key))
                .await?
                .map_err(|_| trouble::UNREACHABLE)?;
            read_moonshot(account, &j, now, "CNY")
        }
        Provider::Openrouter => {
            let j = get_json(client().get("https://openrouter.ai/api/v1/key").bearer_auth(key))
                .await?
                .map_err(|_| trouble::UNREACHABLE)?;
            read_openrouter(account, &j, now)
        }
        Provider::Other(_) => Err(trouble::UNKNOWN_PROVIDER),
    }
}

/// A number the provider may spell as a number or as a decimal string
/// (DeepSeek spells balances as strings).
fn decimal(v: Option<&serde_json::Value>) -> Option<f64> {
    let v = v?;
    v.as_f64().or_else(|| v.as_str().and_then(|s| s.trim().parse().ok())).filter(|n| n.is_finite())
}

/// DeepSeek: the key's own prepaid balance (`GET /user/balance`,
/// Bearer) — one `balance_infos` row per currency, amounts as decimal
/// strings. A balance is what is *left*, so it lands in `remaining`
/// and never in `spent`. The two currencies are two wallets, not one
/// number: the first with money in it is shown, USD before CNY, and an
/// empty account shows USD at zero. No spend, so no fraction and no
/// budget — there is nothing for a budget to be a fraction of.
/// Endpoint and shape as `pi-usage` and `pi-deepseek-usage` read them.
fn read_deepseek(
    account: &Account,
    j: &serde_json::Value,
    now: i64,
) -> Result<AccountReading, &'static str> {
    let rows = j.get("balance_infos").and_then(serde_json::Value::as_array).ok_or(trouble::UNREADABLE)?;
    let wallet = |currency: &str| {
        rows.iter()
            .find(|r| r.get("currency").and_then(serde_json::Value::as_str) == Some(currency))
            .and_then(|r| decimal(r.get("total_balance")))
    };
    let usd = wallet("USD");
    let cny = wallet("CNY");
    let (left, currency) = match (usd, cny) {
        (Some(u), _) if u > 0.0 => (u, "USD"),
        (_, Some(c)) if c > 0.0 => (c, "CNY"),
        (Some(u), _) => (u, "USD"),
        (None, Some(c)) => (c, "CNY"),
        (None, None) => return Err(trouble::UNREADABLE),
    };
    let mut r = bare(account, None, now);
    r.remaining = Some(left);
    r.currency = Some(currency.to_owned());
    Ok(r)
}

/// Moonshot: `GET /v1/users/me/balance` (Bearer) answers `code: 0`,
/// `status: true` and `data.available_balance` — cash plus vouchers,
/// the figure that gates requests (`cash_balance` alone can be
/// negative on credit). The site decides the currency (.ai in USD, .cn
/// in CNY), which is why the caller names it and not the answer.
fn read_moonshot(
    account: &Account,
    j: &serde_json::Value,
    now: i64,
    currency: &str,
) -> Result<AccountReading, &'static str> {
    let ok = j.get("code").and_then(serde_json::Value::as_i64) == Some(0)
        && j.get("status").and_then(serde_json::Value::as_bool) == Some(true);
    if !ok {
        return Err(trouble::UNREADABLE);
    }
    let left = decimal(j.pointer("/data/available_balance")).ok_or(trouble::UNREADABLE)?;
    let mut r = bare(account, None, now);
    r.remaining = Some(left);
    r.currency = Some(currency.to_owned());
    Ok(r)
}

/// OpenRouter: `GET /api/v1/key` (Bearer) describes the key itself —
/// `data.limit` (its spend cap in USD; `null` is no cap),
/// `data.limit_remaining`, `data.usage` (all-time) and
/// `data.usage_monthly` where the key reports one. A capped key fills
/// against its cap, the cap outranking any budget because it is what
/// actually stops requests. An uncapped key is 不限量 at the key and
/// fills against the registered budget with **this month's** usage;
/// with only the all-time figure to hand, that number is said and no
/// fraction is drawn — a lifetime spend over a monthly budget would
/// be a fraction of two different things.
fn read_openrouter(
    account: &Account,
    j: &serde_json::Value,
    now: i64,
) -> Result<AccountReading, &'static str> {
    let d = j.get("data").ok_or(trouble::UNREADABLE)?;
    let all_time = decimal(d.get("usage"));
    let monthly = decimal(d.get("usage_monthly"));
    let mut r = bare(account, None, now);
    r.currency = Some("USD".to_owned());
    // **Both figures, whichever branch the fraction takes.** They are
    // two periods, and which of them used to reach a screen depended on
    // whether the key had a cap — so 「已用」 meant this month on one
    // account and all time on another, with nothing saying which.
    r.spent_this_month = monthly;
    r.spent_all_time = all_time;
    match decimal(d.get("limit")) {
        Some(cap) => {
            let left = decimal(d.get("limit_remaining"))
                .unwrap_or_else(|| (cap - all_time.unwrap_or(0.0)).max(0.0))
                .clamp(0.0, cap.max(0.0));
            r.limit = Some(cap);
            r.remaining = Some(left);
            r.spent = Some(cap - left);
            r.fill = Some(if cap > 0.0 { ((cap - left) / cap).clamp(0.0, 1.0) as f32 } else { 0.0 });
            Ok(r)
        }
        None => {
            r.unlimited = true;
            match monthly {
                Some(m) => Ok(against_budget(r, m, account.budget, "USD")),
                None => {
                    r.spent = Some(all_time.ok_or(trouble::UNREADABLE)?);
                    Ok(r)
                }
            }
        }
    }
}

fn client() -> reqwest::Client {
    reqwest::Client::builder().timeout(HTTP_TIMEOUT).build().unwrap_or_default()
}

/// ⚠️ **A 200 is not a yes.**
///
/// new-api's whole error convention is `HTTP 200` with the failure in
/// the body: `ApiError`, `ApiErrorMsg` and `ApiErrorI18n` all answer
/// `c.JSON(http.StatusOK, {"success": false, "message": …})`
/// (`common/gin.go:199-229`, read 2026-09-07 at
/// `0c76e4da`). So 「invalid token」 arrives as a 200, this function used
/// to hand it back as a success, the reader then found no `data`, and
/// the screen said 「读不懂它的回答」 — **while the truth was 「你的密钥
/// 被拒了」**. A reading khor could not take did not become 「no value」;
/// it became the answer *next to* it, and that answer points at a thing
/// that is not broken.
///
/// **`success: false` is the one spelling checked here, and absence is
/// not falsehood.** A provider that never uses the key is unaffected;
/// new-api's *successful* usage road does not carry it either (its top
/// level is `{"code": true, "message": "ok", "data": …}` — a **bool
/// named `code`**), which is exactly why a scheme that demanded a
/// success flag would refuse a good answer. Present-and-false is the
/// provider saying no about itself; anything else is no opinion.
///
/// A provider that spells refusal some other way is judged in its own
/// reader, where the spelling is known — Moonshot's `code: 0` plus
/// `status: true` is read in [`read_moonshot`], not here.
///
/// **It lands on 「密钥被拒」 rather than a sentence of the gateway's
/// own.** The gateway's `message` is not carried: `trouble` is a *key*
/// this side turns into one of four sentences, each naming a different
/// thing a person can do about it, and a free-text channel beside it
/// would be a second wire field for a string in whatever language that
/// gateway runs in. What is lost is the *why* behind a refusal khor
/// cannot itself distinguish (an invalid token and a database error
/// both arrive this way); what is gained is that the common case stops
/// pointing at the wrong thing.
async fn get_json(
    req: reqwest::RequestBuilder,
) -> Result<Result<serde_json::Value, u16>, &'static str> {
    let response = req.send().await.map_err(|_| trouble::UNREACHABLE)?;
    let status = response.status().as_u16();
    match status {
        401 | 403 => Err(trouble::KEY_REFUSED),
        200..=299 => {
            let body: serde_json::Value =
                response.json().await.map_err(|_| trouble::UNREADABLE)?;
            if refused_in_body(&body) {
                return Err(trouble::KEY_REFUSED);
            }
            Ok(Ok(body))
        }
        code => Ok(Err(code)),
    }
}

/// Whether a 200 body is the provider saying no. See [`get_json`].
fn refused_in_body(body: &serde_json::Value) -> bool {
    body.get("success") == Some(&serde_json::Value::Bool(false))
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
    let (used, available, unlimited, expires_at, models) = match answer {
        Ok(j) => {
            let d = j.get("data").ok_or(trouble::UNREADABLE)?;
            (
                d.get("total_used").and_then(serde_json::Value::as_f64).unwrap_or(0.0),
                d.get("total_available").and_then(serde_json::Value::as_f64),
                d.get("unlimited_quota").and_then(serde_json::Value::as_bool).unwrap_or(false),
                expiry_of(d),
                model_limits_of(d),
            )
        }
        Err(404) => return Err(trouble::GATEWAY_TOO_OLD),
        Err(_) => return Err(trouble::UNREACHABLE),
    };
    let mut r = bare(account, None, now);
    r.currency = Some("USD".to_owned());
    r.spent = Some(used / per_unit);
    r.unlimited = unlimited;
    r.expires_at = expires_at;
    r.models = models;
    if !unlimited && let Some(avail) = available {
        let total = used + avail;
        r.limit = Some(total / per_unit);
        r.fill = Some(if total > 0.0 { (used / total).clamp(0.0, 1.0) as f32 } else { 0.0 });
    }
    Ok(r)
}

/// When a new-api token stops working, or `None` for 「never / not
/// said」.
///
/// ⚠️ **`<= 0` is a sentinel with two spellings, and both would draw as
/// a real date.** The token stores `-1` for 「never expired」
/// (`model/token.go:22`) and the handler rewrites that to `0` before
/// answering (`controller/token.go:256-259`) — so the wire carries `0`
/// today and `-1` is what anyone reading the schema would expect. Both
/// are turned away here, and so is any other non-positive number: a
/// timestamp at or before the epoch is not a fact about when this key
/// stops working.
fn expiry_of(d: &serde_json::Value) -> Option<i64> {
    d.get("expires_at").and_then(serde_json::Value::as_i64).filter(|at| *at > 0)
}

/// The models a new-api token is limited to, or empty for 「not
/// limited」.
///
/// ⚠️ **The flag decides, not the list.** `DisableModelLimits` switches
/// `model_limits_enabled` off and **leaves `model_limits` populated**
/// (`model/token.go:354-360`), so a token that stopped being limited
/// still answers with its old models. Reading the list without the flag
/// paints 「limited to these three」 over 「not limited at all」 — two
/// opposite sentences, both spelled with real model names, and nothing
/// downstream can tell them apart.
///
/// **The shape is a map, not an array**: `GetModelLimitsMap()` answers
/// `{"gpt-4": true, …}` (`model/token.go:345-352`), so the models are
/// its *keys*. Sorted, because a JSON object has no order and this list
/// is drawn on a screen — the same two models must not swap places
/// between two readings.
///
/// **「Which group is this token in」 is not here and cannot be.** The
/// `Token` row has a `Group` (`model/token.go:29`) and this endpoint
/// does not answer it; the `group` on each *log* line is which group
/// that one request went through, which is a different fact. A token's
/// own group is an admin-key road, and it is not this one.
fn model_limits_of(d: &serde_json::Value) -> Vec<String> {
    if d.get("model_limits_enabled").and_then(serde_json::Value::as_bool) != Some(true) {
        return Vec::new();
    }
    let Some(map) = d.get("model_limits").and_then(serde_json::Value::as_object) else {
        return Vec::new();
    };
    let mut out: Vec<String> = map.keys().cloned().collect();
    out.sort();
    out
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
    let mut days: Vec<khor_core::DaySpend> = Vec::new();
    let c = client();
    loop {
        let j = get_json(
            c.get(&url).header("x-api-key", key).header("anthropic-version", "2023-06-01"),
        )
        .await?
        .map_err(|_| trouble::UNREACHABLE)?;
        for bucket in j.get("data").and_then(serde_json::Value::as_array).into_iter().flatten() {
            // **The bucket's own total, kept as well as summed.** The
            // day was always in this answer — it was the loop that threw
            // it away. Amounts come back in cents as decimal strings.
            let mut in_bucket = 0.0;
            for r in bucket.get("results").and_then(serde_json::Value::as_array).into_iter().flatten() {
                if let Some(a) = r.get("amount") {
                    in_bucket +=
                        a.as_f64().or_else(|| a.as_str().and_then(|s| s.parse().ok())).unwrap_or(0.0);
                }
            }
            cents += in_bucket;
            if let Some(at) = instant_of(bucket, "starting_at") {
                days.push(khor_core::DaySpend { at, spent: in_bucket / 100.0 });
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
    let mut r = against_budget(bare(account, None, now), cents / 100.0, account.budget, "USD");
    r.days = days;
    Ok(r)
}

/// A bucket's start instant, unix seconds, however the vendor spelled
/// it: a number, or an RFC 3339 string.
///
/// **Two spellings because the two vendors disagree** — OpenAI's
/// `start_time` is a unix integer and Anthropic's `starting_at` is a
/// timestamp string. A bucket khor cannot place in time gets **no
/// entry** rather than one at the epoch: a bar at 1970 is not a
/// missing bar, it is a wrong one, and it would drag every chart's
/// axis with it.
fn instant_of(bucket: &serde_json::Value, key: &str) -> Option<i64> {
    let v = bucket.get(key)?;
    if let Some(n) = v.as_i64() {
        return (n > 0).then_some(n);
    }
    let s = v.as_str()?;
    s.parse::<jiff::Timestamp>().ok().map(|t| t.as_second())
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
    let mut days: Vec<khor_core::DaySpend> = Vec::new();
    loop {
        let j = get_json(c.get(&url).bearer_auth(key)).await?.map_err(|_| trouble::UNREACHABLE)?;
        for bucket in j.get("data").and_then(serde_json::Value::as_array).into_iter().flatten() {
            let mut in_bucket = 0.0;
            for r in bucket.get("results").and_then(serde_json::Value::as_array).into_iter().flatten() {
                in_bucket += r.pointer("/amount/value").and_then(serde_json::Value::as_f64).unwrap_or(0.0);
            }
            dollars += in_bucket;
            // Already dollars here, where Anthropic's are cents.
            if let Some(at) = instant_of(bucket, "start_time") {
                days.push(khor_core::DaySpend { at, spent: in_bucket });
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
    let mut r = against_budget(bare(account, None, now), dollars, limit_usd.or(account.budget), "USD");
    r.days = days;
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **A 200 with `success: false` is a refusal, and a 200 without
    /// the key is not.**
    ///
    /// Three arms, and the third is the one a careless fix breaks:
    /// new-api's *successful* usage road carries no `success` at all
    /// (`{"code": true, …}`), so a rule that demanded a success flag
    /// would refuse every good answer this provider gives. The second
    /// arm is the control for the first — same shape, opposite value —
    /// and without it 「refuses everything」 passes.
    #[test]
    fn a_two_hundred_that_says_no_is_a_refusal_and_a_silent_one_is_not() {
        let no = serde_json::json!({"success": false, "message": "无效的令牌"});
        assert!(refused_in_body(&no), "the gateway said no in a 200");
        let yes = serde_json::json!({"success": true, "message": "", "data": []});
        assert!(!refused_in_body(&yes), "the same key, the other value");
        // The shape `GetTokenUsage` actually answers with on success —
        // a **bool named `code`**, and no `success` anywhere.
        let usage = serde_json::json!({
            "code": true, "message": "ok",
            "data": {"object": "token_usage", "total_used": 450000},
        });
        assert!(!refused_in_body(&usage), "absent is not false");
    }

    /// **The expiry sentinel, both of its spellings, and a real date.**
    ///
    /// Four arms. The last one is the control: without a positive value
    /// that survives, 「turns everything into None」 passes the first
    /// three — and that implementation loses a real expiry silently,
    /// which is the failure this pair of fields exists to avoid in the
    /// other direction.
    #[test]
    fn a_never_expiring_token_gets_no_date_whichever_way_it_says_so() {
        let at = |v: serde_json::Value| expiry_of(&serde_json::json!({"expires_at": v}));
        assert_eq!(at(serde_json::json!(0)), None, "0 is what the endpoint rewrites -1 into");
        assert_eq!(at(serde_json::json!(-1)), None, "-1 is what the row itself stores");
        assert_eq!(expiry_of(&serde_json::json!({})), None, "and a gateway that says nothing");
        assert_eq!(at(serde_json::json!(1_800_000_000i64)), Some(1_800_000_000), "a real instant survives");
    }

    /// **A switched-off model limit keeps its old list, and must not be
    /// drawn.**
    ///
    /// The first arm is the trap: `model_limits` is full while
    /// `model_limits_enabled` is false, which is exactly what a token
    /// answers after `DisableModelLimits`. The second is the control —
    /// same list, flag on — so 「always empty」 cannot pass. Sorted,
    /// because a JSON object has no order and this is drawn on a screen.
    #[test]
    fn a_disabled_model_limit_is_not_a_list_of_models() {
        let full = serde_json::json!({"gpt-5": true, "claude-opus-4-6": true});
        let off = serde_json::json!({"model_limits_enabled": false, "model_limits": full});
        assert!(model_limits_of(&off).is_empty(), "the flag is off, so the list means nothing");
        let on = serde_json::json!({"model_limits_enabled": true, "model_limits": full});
        assert_eq!(
            model_limits_of(&on),
            vec!["claude-opus-4-6".to_owned(), "gpt-5".to_owned()],
            "the flag is on: the map's keys, sorted",
        );
        let silent = serde_json::json!({"total_used": 1});
        assert!(model_limits_of(&silent).is_empty(), "a gateway that says neither");
    }

    /// **The refusal check is wired into `get_json`, not merely
    /// written.**
    ///
    /// ⚠️ **This test exists because the one above does not cover it.**
    /// That one calls `refused_in_body` directly, so it proves the
    /// judgment is *correct* and says nothing about whether anything
    /// *asks* it — measured 2026-09-07 by deleting the call from
    /// `get_json` and leaving the function in place: **every test
    /// stayed green**. An assertion about a rule and an assertion about
    /// the rule being applied are two different axes, and the first one
    /// is the cheaper and more tempting to stop at.
    ///
    /// So this one speaks HTTP: a socket that answers a real
    /// `200 + {"success": false}` — the shape new-api uses for every
    /// refusal — and the answer has to come back as 密钥被拒 rather than
    /// as a body for somebody downstream to fail to understand.
    #[tokio::test]
    async fn a_gateway_refusing_inside_a_two_hundred_reaches_the_reader_as_refused() {
        use tokio::io::AsyncWriteExt as _;
        let serve = |body: &'static str| async move {
            let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let port = l.local_addr().unwrap().port();
            tokio::spawn(async move {
                let (mut sock, _) = l.accept().await.unwrap();
                let head = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                    body.len(),
                );
                let _ = sock.write_all(head.as_bytes()).await;
                let _ = sock.write_all(body.as_bytes()).await;
                let _ = sock.shutdown().await;
            });
            port
        };
        let no = serve(r#"{"success":false,"message":"invalid token"}"#).await;
        let said = get_json(client().get(format!("http://127.0.0.1:{no}/"))).await;
        assert_eq!(said.err(), Some(trouble::KEY_REFUSED), "a 200 that says no is a refusal");

        // The control arm: the same status, the same road, a body that
        // does not say no — it must come back as an answer, or this
        // check is 「refuse every 200」 wearing the right words.
        let ok = serve(r#"{"code":true,"message":"ok","data":{"total_used":1}}"#).await;
        let said = get_json(client().get(format!("http://127.0.0.1:{ok}/"))).await;
        assert!(said.is_ok(), "and a 200 with no `success` key is not one");
    }

    /// **A bucket's start, in either vendor's spelling, and none at all
    /// where it cannot be placed.**
    ///
    /// OpenAI's `start_time` is a unix integer, Anthropic's
    /// `starting_at` an RFC 3339 string — one function, two shapes,
    /// because two readers doing it separately is how they come to
    /// disagree about a bar's position. The last two arms are the
    /// control: a bucket khor cannot place gets **no entry**, never one
    /// at the epoch, because a bar at 1970 is not a missing bar — it is
    /// a wrong one, and it drags the whole axis with it.
    #[test]
    fn a_bucket_is_placed_by_either_vendors_spelling_or_not_at_all() {
        let at = |k: &str, v: serde_json::Value| instant_of(&serde_json::json!({k: v}), k);
        assert_eq!(at("start_time", serde_json::json!(1_757_000_000i64)), Some(1_757_000_000));
        // **The two spellings of one instant must land on one number**,
        // asserted against each other rather than against a constant.
        // The first draft wrote the epoch seconds out by hand and they
        // were wrong by twelve days — a number nobody had computed,
        // which is the same failure as quoting a hash from memory. This
        // way the test cannot be wrong about what 2026-09-01 *is*; it
        // can only be wrong about the two readings disagreeing, which
        // is the thing it is for.
        let as_text = at("starting_at", serde_json::json!("2026-09-01T00:00:00Z"))
            .expect("an RFC 3339 instant parses");
        assert_eq!(
            at("start_time", serde_json::json!(as_text)),
            Some(as_text),
            "an RFC 3339 instant lands on the same axis as an integer",
        );
        assert!(as_text > 1_700_000_000, "…and it is this decade, not the epoch");
        assert_eq!(at("start_time", serde_json::json!(0)), None, "the epoch is not a bucket");
        assert_eq!(at("starting_at", serde_json::json!("not a time")), None);
        assert_eq!(instant_of(&serde_json::json!({}), "start_time"), None, "and a silent bucket");
    }

    /// **OpenRouter states two periods and both survive, whichever
    /// branch the fraction takes.**
    ///
    /// Two arms, capped and uncapped, because *which* figure used to
    /// reach a screen depended on exactly that — so 「已用」 meant this
    /// month on one account and all time on another, with nothing on
    /// the glass saying which. An implementation that kept only one
    /// passes one arm and fails the other.
    #[test]
    fn openrouter_keeps_its_month_and_its_lifetime_apart() {
        let capped = serde_json::json!({
            "data": {"limit": 20.0, "limit_remaining": 5.0, "usage": 115.0, "usage_monthly": 15.0}
        });
        let r = read_openrouter(&wallet(Provider::Openrouter, None), &capped, 0).unwrap();
        assert_eq!(r.spent_this_month, Some(15.0));
        assert_eq!(r.spent_all_time, Some(115.0));
        assert_eq!(r.spent, Some(15.0), "and `spent` still means what the fraction is made of");

        let uncapped = serde_json::json!({
            "data": {"limit": null, "usage": 115.0, "usage_monthly": 15.0}
        });
        let r = read_openrouter(&wallet(Provider::Openrouter, Some(100.0)), &uncapped, 0).unwrap();
        assert_eq!(r.spent_this_month, Some(15.0), "the month survives the other branch too");
        assert_eq!(r.spent_all_time, Some(115.0), "…and so does the lifetime");
    }

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
            pinned: false,
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

    fn wallet(provider: Provider, budget: Option<f64>) -> Account {
        Account {
            id: "a".into(),
            name: "n".into(),
            provider,
            base_url: None,
            budget,
            home: "h".into(),
            pinned: false,
        }
    }

    /// **A balance is what is left, and it is never written into
    /// `spent`.** DeepSeek's shape (decimal strings, a row per
    /// currency): the wallet with money in it is the one shown, USD
    /// before CNY; an empty account still reads, at zero; a body with
    /// no rows is unreadable rather than zero.
    #[test]
    fn a_deepseek_balance_lands_in_remaining_and_picks_the_wallet_with_money() {
        let acc = wallet(Provider::Deepseek, None);
        let both = serde_json::json!({"is_available": true, "balance_infos": [
            {"currency": "CNY", "total_balance": "12.50", "granted_balance": "0.00", "topped_up_balance": "12.50"},
            {"currency": "USD", "total_balance": "17.35", "granted_balance": "0.00", "topped_up_balance": "17.35"}]});
        let r = read_deepseek(&acc, &both, 0).unwrap();
        assert_eq!((r.remaining, r.currency.as_deref(), r.spent, r.fill), (Some(17.35), Some("USD"), None, None));

        let cny_only_has_money = serde_json::json!({"is_available": true, "balance_infos": [
            {"currency": "USD", "total_balance": "0.00"}, {"currency": "CNY", "total_balance": "3.00"}]});
        let r = read_deepseek(&acc, &cny_only_has_money, 0).unwrap();
        assert_eq!((r.remaining, r.currency.as_deref()), (Some(3.0), Some("CNY")));

        let empty = serde_json::json!({"is_available": false, "balance_infos": [
            {"currency": "USD", "total_balance": "0.00"}, {"currency": "CNY", "total_balance": "0.00"}]});
        let r = read_deepseek(&acc, &empty, 0).unwrap();
        assert_eq!((r.remaining, r.currency.as_deref()), (Some(0.0), Some("USD")), "empty is a reading");

        let none = serde_json::json!({"is_available": true, "balance_infos": []});
        assert_eq!(read_deepseek(&acc, &none, 0).unwrap_err(), trouble::UNREADABLE);
    }

    /// Moonshot answers `code`/`status` around the figure; a refusal
    /// dressed as 200 (`code != 0`) is unreadable, not zero.
    #[test]
    fn a_moonshot_balance_is_read_in_the_sites_currency() {
        let acc = wallet(Provider::MoonshotCn, None);
        let j = serde_json::json!({"code": 0, "status": true, "scode": "0x0",
            "data": {"available_balance": 49.58, "voucher_balance": 40.0, "cash_balance": 9.58}});
        let r = read_moonshot(&acc, &j, 0, "CNY").unwrap();
        assert_eq!((r.remaining, r.currency.as_deref(), r.spent), (Some(49.58), Some("CNY"), None));
        let refused = serde_json::json!({"code": 1, "status": false, "data": {"available_balance": 49.58}});
        assert_eq!(read_moonshot(&acc, &refused, 0, "CNY").unwrap_err(), trouble::UNREADABLE);
    }

    /// A capped OpenRouter key fills against its cap and the cap
    /// outranks a budget; an uncapped one is 不限量 and fills against
    /// the budget with the month's usage — and with only an all-time
    /// figure, says the number and draws no fraction.
    #[test]
    fn an_openrouter_key_fills_against_its_cap_or_the_budget_never_against_all_time() {
        let capped = serde_json::json!({"data": {"label": "k", "limit": 20.0, "limit_remaining": 5.0,
            "usage": 115.0, "usage_monthly": 15.0, "is_free_tier": false}});
        let r = read_openrouter(&wallet(Provider::Openrouter, Some(100.0)), &capped, 0).unwrap();
        assert_eq!((r.limit, r.remaining, r.spent), (Some(20.0), Some(5.0), Some(15.0)));
        assert_eq!(r.fill, Some(0.75));
        assert!(!r.unlimited);

        let uncapped = serde_json::json!({"data": {"limit": null, "limit_remaining": null,
            "usage": 115.0, "usage_monthly": 15.0}});
        let r = read_openrouter(&wallet(Provider::Openrouter, Some(100.0)), &uncapped, 0).unwrap();
        assert!(r.unlimited);
        assert_eq!((r.spent, r.limit, r.fill), (Some(15.0), Some(100.0), Some(0.15)));

        let all_time_only = serde_json::json!({"data": {"limit": null, "usage": 115.0}});
        let r = read_openrouter(&wallet(Provider::Openrouter, Some(100.0)), &all_time_only, 0).unwrap();
        assert_eq!((r.spent, r.limit, r.fill), (Some(115.0), None, None), "no fraction of two different things");
    }

    /// The sentence that refuses a provider word, and the usage text,
    /// must name every word `parse` accepts — two lists a person reads
    /// to learn what exists, kept honest against the one that decides.
    #[test]
    fn every_provider_word_is_named_where_a_person_would_look() {
        let refusal = msg::no_such_provider("x");
        for w in Provider::WORDS {
            assert!(refusal.contains(w), "no-such-provider does not name {w}: {refusal}");
            assert!(khor_catalog::cli::USAGE.contains(w), "the usage text does not name {w}");
        }
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
