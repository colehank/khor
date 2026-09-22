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
//!
//! # Where new-api's shapes come from, and how far that goes
//!
//! **Every `file.go:NN` below is `QuantumNous/new-api` at
//! `0c76e4dae77a279e015329b7478e6f02d6b62edd`** (its default branch on
//! 2026-09-06T14:49:02Z), read on 2026-09-07 over `gh api
//! repos/QuantumNous/new-api/contents/<path>` — remotely, nothing
//! cloned. **The revision is the anchor**: a bare line number rots
//! silently against a moving branch, and the reader of a rotted one
//! does not get an error, they get a confident pointer to the wrong
//! line.
//!
//! ⚠️ **Source, not a capture. This shape has never been checked
//! against a running gateway.** Source says what the project *means* to
//! answer; a real site may run an older build, or a fork. So every
//! field here is read with `Option` / defaults, and a khor that meets a
//! disagreeing gateway loses a figure rather than a reading.
//!
//! ⚠️ **The project's own OpenAPI document contradicts its code, in
//! two places**, and both would break an implementation written from
//! it: `docs/openapi/api.json` gives `/api/log/token` a `key` query
//! parameter and calls it unauthenticated, while the route is behind
//! `middleware.TokenAuthReadOnly` and reads the `Authorization` header
//! only (`router/api-router.go:340`, `middleware/auth.go:288-305`; the
//! `c.Query("key")` fallback lives in the *other* middleware,
//! `auth.go:390`). **The code is what runs.**
//!
//! # What reading it bought, and why no gate could have
//!
//! Two of the shapes below are traps that **cannot go red**:
//! `expires_at` carries a sentinel with two spellings (`-1` in the
//! table, rewritten to `0` in the handler), and `model_limits` keeps
//! its old contents after the limit is switched off. Neither errors,
//! neither draws a blank — **one paints a real-looking date, the other
//! a list of real model names.** A gate can only compare what is drawn
//! against what was routed, and a fixture written by the same person
//! who misread the source carries the misreading into both halves.
//! What caught these was reading the source; what keeps them caught is
//! the citations, which is why they are line-precise and pinned.

use std::path::PathBuf;
use std::time::Duration;

use khor_catalog::msg;
use khor_core::{AccountReading, Fill, Quota, QuotaTrouble, Strain};
use khor_sync::accounts::{Account, AccountKind, Provider};
use serde::{Deserialize, Serialize};

use crate::Node;
use crate::link;

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
    /// A registered subscription whose home machine has no vendor login
    /// left to read. **Registration refuses this state**
    /// (`register_account`), so reaching it means the login went away
    /// afterwards — which is a thing that happens, and a different
    /// remedy from the one below.
    pub const NO_LOGIN: &str = "no-login";
    /// The home machine's vendor login is expired or was refused. khor
    /// does not refresh it (`khor_core::QuotaTrouble::Stale`); the
    /// person runs the vendor's own command once over there.
    pub const LOGIN_STALE: &str = "login-stale";
}

/// The provider words a subscription can be registered against —
/// **derived from [`sub_reader`], never listed beside it.**
///
/// A provider belongs here exactly when this build knows which stored
/// login answers for it, and that is one fact: asking the reader means
/// the form, the refusal and the reading cannot come to disagree about
/// what is offered. A second list kept in step by hand is how a
/// provider ends up on a form that nothing can read — the registration
/// succeeds and the row arrives troubled, the same mistake as
/// registering a subscription on a machine with no login.
///
/// codex's credential is the next candidate and is deliberately absent
/// until somebody has read where it lives.
pub fn sub_provider_words() -> Vec<&'static str> {
    Provider::WORDS
        .into_iter()
        .filter(|word| {
            Provider::parse(word)
                .as_ref()
                .and_then(sub_reader)
                .is_some()
        })
        .collect()
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
    Fill {
        used: (f.clamp(0.0, 1.0) * 1000.0).round() as u64,
        total: 1000,
    }
    .strain()
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
    ///
    /// # Two kinds, one rule about `home`
    ///
    /// A billed account is homed here because **its key is here** and a
    /// secret does not travel. A subscription has no key, so that
    /// reason does not apply to it — and it is still homed here, for a
    /// different one: what reads it is the vendor's login *on this
    /// machine*, and **whether that login exists is the one thing a
    /// registering machine can prove on the spot**
    /// (`quota::logged_in`).
    ///
    /// Letting somebody register a subscription *for* another machine
    /// would put a claim into a replicated document that the writer
    /// cannot check — and it would not fail at the keyboard, it would
    /// fail minutes later as a permanently troubled row on a screen
    /// somewhere else, which the person can only delete and retype.
    /// **`home` is a field, not a constant**: `--home` is an argument
    /// somebody can add the day that trade looks worth it, and nothing
    /// here has to be redone for it.
    ///
    /// An unknown kind is refused rather than taken for a billed one,
    /// the same bargain [`Provider::parse`] makes: `AccountKind::Other`
    /// only ever *arrives* from a newer khor, it is never registered.
    pub fn register_account(
        &self,
        name: &str,
        provider: Provider,
        kind: AccountKind,
        base_url: Option<&str>,
        key: &str,
        budget: Option<f64>,
    ) -> Result<Account, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err(msg::account_name_blank().into());
        }
        match kind {
            AccountKind::Sub => {
                return self.register_subscription(name, provider, base_url, key, budget);
            }
            AccountKind::Other(word) => return Err(msg::no_such_kind(word)),
            AccountKind::Bill => {}
        }
        // A plan read through a login, with no key or bill of its own.
        if provider == Provider::Codex {
            return Err(msg::provider_sub_only(provider.word()));
        }
        let key = key.trim();
        if key.is_empty() {
            return Err(msg::account_key_blank().into());
        }
        let base_url = match (&provider, base_url.map(str::trim).filter(|u| !u.is_empty())) {
            (Provider::Newapi, None) => return Err(msg::account_needs_address().into()),
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
            return Err(msg::account_budget_bad().into());
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
            // …and nothing arrives with a unit either: until somebody
            // types one, what the endpoint says is what khor says.
            unit: None,
            kind: AccountKind::Bill,
        };
        std::fs::create_dir_all(self.keys_dir()).map_err(msg::cant_make_keys_dir)?;
        link::write_private(&self.keys_dir().join(&account.id), key.as_bytes())?;
        self.commit_account(account)
    }

    /// Registers **this machine's** vendor subscription: a name, and
    /// nothing else to fill in.
    ///
    /// Four refusals, and each one is a thing that cannot be discovered
    /// later at the same price:
    ///
    /// - **A key, an address or a budget was given.** Refused rather
    ///   than dropped. Dropping a key is the worst of the four: the
    ///   person pastes a secret, reads 「registered」, and the secret
    ///   went nowhere — the same shape `account_key_elsewhere` exists
    ///   to refuse.
    /// - **A provider with no subscription khor can read**
    ///   ([`sub_reader`] answers nothing for it).
    /// - **No login on this machine.** The one check that is neither
    ///   transient nor a guess, and the whole reason `home` is here.
    /// - **A second subscription to one vendor on this machine.** One
    ///   machine holds one login per vendor with no way to choose
    ///   between logins, so a second
    ///   registration could only ever draw a second row of the same
    ///   numbers — which is the duplication this batch removed from the
    ///   probe row, re-made by hand.
    fn register_subscription(
        &self,
        name: &str,
        provider: Provider,
        base_url: Option<&str>,
        key: &str,
        budget: Option<f64>,
    ) -> Result<Account, String> {
        let Some(reader) = sub_reader(&provider) else {
            return Err(msg::provider_has_no_sub(
                provider.word(),
                sub_provider_words().join("、"),
            ));
        };
        let given_address = base_url.map(str::trim).is_some_and(|u| !u.is_empty());
        if !key.trim().is_empty() || given_address || budget.is_some() {
            return Err(msg::account_sub_takes_nothing_else(reader.vendor));
        }
        let me = self.device_str().to_owned();
        // **Before the login check, deliberately.** Both can be true at
        // once — a machine that registered its subscription and later
        // lost the login — and 「已经登记过了」 is the more specific of
        // the two answers: it names the row that already exists, where
        // the other would send somebody to log in again for a
        // registration they would still not be allowed to make.
        if let Some(had) = self
            .accounts()?
            .into_iter()
            .find(|a| a.kind.is_sub() && a.home == me && a.provider == provider)
        {
            return Err(msg::account_sub_already_here(had.name));
        }
        if !(reader.logged_in)() {
            return Err(msg::account_sub_no_login(reader.vendor, reader.command));
        }
        self.commit_account(Account {
            id: link::fresh_hex()?,
            name: name.to_owned(),
            provider,
            // **Three fields a subscription has no use for**, and they
            // are `None` here because they were refused above rather
            // than because this is where they get thrown away.
            base_url: None,
            budget: None,
            unit: None,
            home: me,
            pinned: false,
            kind: AccountKind::Sub,
        })
    }

    /// **This machine's vendor login becomes a registration, once**
    /// (docs/SHELL.md 第七轮 第 3 条:「这些自动读取的一视同仁. 只是自动初始化
    /// 了, 但一切操作与别的list无异」). Until then a login khor found was a
    /// probed row with no menu (2026-09-07); now it is the same document
    /// `khor accounts add --kind sub` writes, so pinning, deleting and
    /// editing it are what they are for every other account.
    ///
    /// **Once per provider, remembered here and never replicated**
    /// (`.khor/subs-adopted.json`): a person who deletes it is not handed
    /// it back on the next read — they register it by hand if they want
    /// it. A provider is written down when a subscription for it is homed
    /// here, whether this made it or somebody registered it by hand
    /// first; a machine with no login writes nothing, so a login that
    /// arrives later is adopted then.
    ///
    /// **Only this machine's own login**: the registration is homed here
    /// like every subscription (`register_subscription` says why), and
    /// another machine's login is that machine's to adopt — its row
    /// arrives over sync.
    ///
    /// Answers how many it made. Called before every read of the list —
    /// the window's board and `khor quota` / `khor accounts` — so an
    /// installed machine adopts on its first read after the upgrade.
    pub fn adopt_subscriptions(&self) -> Result<usize, String> {
        self.adopt_subscriptions_with(|p| (p.logged_in)())
    }

    /// [`Node::adopt_subscriptions`] with the login check handed in: the
    /// real one reads the operator's credential store, which no test may
    /// touch (`quota::stored_credential`'s tripwire).
    pub(crate) fn adopt_subscriptions_with(
        &self,
        logged_in: impl Fn(&'static crate::quota::Provider) -> bool,
    ) -> Result<usize, String> {
        let note = self.root().join(".khor").join("subs-adopted.json");
        let mut done: std::collections::BTreeSet<String> = std::fs::read_to_string(&note)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        let me = self.device_str().to_owned();
        let mut made = 0;
        let mut changed = false;
        for word in sub_provider_words() {
            if done.contains(word) {
                continue;
            }
            let Some(provider) = Provider::parse(word) else {
                continue;
            };
            let Some(reader) = sub_reader(&provider) else {
                continue;
            };
            let here = self
                .accounts()?
                .into_iter()
                .any(|a| a.kind.is_sub() && a.home == me && a.provider == provider);
            if !here {
                if !logged_in(reader) {
                    continue;
                }
                self.commit_account(Account {
                    id: link::fresh_hex()?,
                    name: msg::sub_auto_name(reader.vendor),
                    provider,
                    base_url: None,
                    budget: None,
                    unit: None,
                    home: me.clone(),
                    pinned: false,
                    kind: AccountKind::Sub,
                })?;
                made += 1;
            }
            done.insert(word.to_owned());
            changed = true;
        }
        if changed {
            let dot = self.root().join(".khor");
            std::fs::create_dir_all(&dot).map_err(|_| msg::cant_make_dir(dot.display()))?;
            let text = serde_json::to_vec(&done).map_err(|e| e.to_string())?;
            std::fs::write(&note, text).map_err(|e| msg::cant_write(note.display(), e))?;
        }
        Ok(made)
    }

    /// Writes a fresh registration to the replicated doc. The key, when
    /// there is one, is already on disk by the time this is called —
    /// **that order matters**: a doc entry with no key beside it reads
    /// as 「the home machine holds no key for this account」, which is a
    /// sentence about a registration somebody made, not about one that
    /// half-happened.
    fn commit_account(&self, account: Account) -> Result<Account, String> {
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
        unit: Option<&str>,
    ) -> Result<(), String> {
        let loaded = self.accounts_loaded()?;
        let mut account = loaded.doc.get(id).ok_or_else(|| msg::no_such_account(id))?;
        let name = name.trim();
        if name.is_empty() {
            return Err(msg::account_name_blank().into());
        }
        // The same two checks registration makes, in the same order and
        // with the same sentences — an edit that accepted an address
        // registration refuses would be a second opinion about what a
        // gateway is.
        let base_url = match (
            &account.provider,
            base_url.map(str::trim).filter(|u| !u.is_empty()),
        ) {
            (Provider::Newapi, None) => return Err(msg::account_needs_address().into()),
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
            return Err(msg::account_budget_bad().into());
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
        // **Blank is 「use what the endpoint said」**, not an empty
        // string: the two are one thing to a person and two to a
        // reader downstream, and `None` is the one that means it.
        account.unit = unit
            .map(str::trim)
            .filter(|u| !u.is_empty())
            .map(str::to_owned);
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
        std::fs::read_to_string(self.keys_dir().join(id))
            .ok()
            .map(|k| k.trim().to_owned())
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
        let mine: Vec<Account> = self
            .accounts()?
            .into_iter()
            .filter(|a| a.home == me)
            .collect();
        let now = crate::live::now_ms();
        let mut out = Vec::with_capacity(mine.len());
        for account in mine {
            out.push(self.read_one(&account, now).await);
        }
        Ok(out)
    }

    async fn read_one(&self, account: &Account, now: i64) -> AccountReading {
        // **A subscription keeps no reading of its own on this disk.**
        // `quota::read` already holds three layers for exactly this
        // purpose — five minutes in memory, a disk cache every process
        // on the machine shares, a ten-minute cooldown after a failure
        // — so a second cache over it would add only its own staleness,
        // and `at_ms` would then say 「read now」 about a figure two
        // caches deep. The age it carries is the reading's own
        // (`Quota::as_of`), which is the axis this page is built on.
        if account.kind.is_sub() {
            return read_subscription(account, now).await;
        }
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
                self.cache_reading(
                    &account.id,
                    &Cached {
                        reading: reading.clone(),
                        failed_at_ms: None,
                    },
                );
                reading
            }
            Err(why) => {
                // The last good reading stands, aged; the failure only
                // starts the cooldown. With nothing to stand, the
                // trouble is the reading.
                let kept = cached
                    .filter(|c| c.reading.trouble.is_none())
                    .map(|c| c.reading);
                let reading = kept.unwrap_or_else(|| bare(account, Some(why), now));
                self.cache_reading(
                    &account.id,
                    &Cached {
                        reading: reading.clone(),
                        failed_at_ms: Some(now),
                    },
                );
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
        let report = PeerQuotas {
            at_ms: crate::live::now_ms(),
            readings: readings.to_vec(),
        };
        link::write_private(
            &self.quotas_dir().join(format!("peer-{device_id}.json")),
            &serde_json::to_vec(&report).map_err(|e| e.to_string())?,
        )
    }

    fn peer_quotas(&self, device_id: &str) -> Option<(Vec<AccountReading>, u64)> {
        let text =
            std::fs::read_to_string(self.quotas_dir().join(format!("peer-{device_id}.json")))
                .ok()?;
        let report: PeerQuotas = serde_json::from_str(&text).ok()?;
        Some((
            report.readings,
            crate::live::now_ms().saturating_sub(report.at_ms) as u64,
        ))
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
            // **The unit override is applied here and nowhere else.**
            // Both faces come through this function — the GUI's board
            // and `khor quota` — so a screen and the terminal cannot
            // disagree about what an account is billed in. It is a
            // *label*: the figure beside it is untouched, because the
            // gateway reported the right amount under the wrong name
            // (`Account::unit`).
            let relabel = |mut r: AccountReading| {
                if let Some(unit) = account.unit.clone() {
                    r.currency = Some(unit);
                }
                r
            };
            if account.home == me {
                if let Some(r) = mine.iter().find(|r| r.account == account.id) {
                    out.push((relabel(r.clone()), 0));
                }
                continue;
            }
            let carried = self.peer_quotas(&account.home).and_then(|(rs, age)| {
                rs.into_iter()
                    .find(|r| r.account == account.id)
                    .map(|r| (r, age))
            });
            let (reading, age) =
                carried.unwrap_or_else(|| (bare(&account, Some(trouble::NO_READING), now), 0));
            out.push((relabel(reading), age));
        }
        Ok(out)
    }
}

/// Which vendor answers for a provider's subscription — **the one
/// place that decides it, and everything else asks here.**
///
/// Registration, the login check and the reading all go through this,
/// and [`sub_provider_words`] is derived from it rather than listed
/// beside it. **The vendors are `crate::quota`'s descriptions** (one
/// per vendor, a chain of strategies each — docs/SHELL.md 第九轮), so
/// adding a vendor is a description there, not an arm here.
pub fn sub_reader(provider: &Provider) -> Option<&'static crate::quota::Provider> {
    crate::quota::provider(provider.word())
}

/// This machine's subscription reading for one registered account.
async fn read_subscription(account: &Account, now: i64) -> AccountReading {
    let Some(reader) = sub_reader(&account.provider) else {
        // Registration refuses a provider nothing can read, so reaching
        // this means the home machine has been downgraded past its own
        // registration — the state `UNKNOWN_PROVIDER` already names.
        return bare(account, Some(trouble::UNKNOWN_PROVIDER), now);
    };
    subscription_reading(account, reader.read().await, now)
}

/// A [`Quota`] as one row of the 额度 board — **the seam the reading is
/// tested at.**
///
/// Split out because the step above it reads the machine's real stored
/// credential and can leave over HTTP: a test that called it would be
/// asking a person's keychain what a mapping does.
fn subscription_reading(
    account: &Account,
    read: Result<Quota, QuotaTrouble>,
    now: i64,
) -> AccountReading {
    let quota = match read {
        Ok(quota) => quota,
        Err(why) => return bare(account, Some(sub_trouble(&why)), now),
    };
    AccountReading {
        // **The share is the node's, and it is the same number the
        // subscription's own reading carries** (`Quota::fill`) — one
        // rule, one function, so a probed row and a registered row
        // cannot draw different arcs for one login.
        fill: quota.fill,
        // The reading's own age, not this call's: `None` is 「taken
        // just now」 and anything else is when it was really taken.
        at_ms: quota.as_of.map_or(now, |seconds| seconds * 1000),
        windows: quota.windows,
        plan: quota.plan,
        credits: quota.credits,
        ..bare(account, None, now)
    }
}

/// Why a subscription has no numbers, in the account board's words.
///
/// **`Cooling`'s minutes do not survive**, and that is a real loss said
/// out loud rather than papered over: the board's vocabulary is keys,
/// one sentence each, and there is nowhere in a key to put a number.
/// The two new words are the ones worth having — 「no login」 and
/// 「expired login」 have different remedies, and telling them apart is
/// the whole value of saying anything.
pub fn sub_trouble(why: &QuotaTrouble) -> &'static str {
    match why {
        QuotaTrouble::NoLogin => trouble::NO_LOGIN,
        QuotaTrouble::Stale => trouble::LOGIN_STALE,
        QuotaTrouble::Cooling { .. } => trouble::COOLING,
        QuotaTrouble::Unreadable => trouble::UNREADABLE,
        QuotaTrouble::Unreachable => trouble::UNREACHABLE,
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
        model_days: Vec::new(),
        log_capped: None,
        windows: Vec::new(),
        plan: None,
        credits: None,
        model_tokens: Vec::new(),
        //   **No buckets, so no zone that cut them.** `bare` is the
        //   reading with no numbers in it; the field is filled by
        //   whoever fills the lists, beside the pass that cut them.
        zone: None,
    }
}

/// Spend against a registered budget: the fraction the two vendors get,
/// since neither has a ceiling of its own to fill against.
fn against_budget(
    mut r: AccountReading,
    spent: f64,
    budget: Option<f64>,
    currency: &str,
) -> AccountReading {
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
            let j = get_json(
                client()
                    .get("https://api.deepseek.com/user/balance")
                    .bearer_auth(key),
            )
            .await?
            .map_err(|_| trouble::UNREACHABLE)?;
            read_deepseek(account, &j, now)
        }
        Provider::Moonshot => {
            let j = get_json(
                client()
                    .get("https://api.moonshot.ai/v1/users/me/balance")
                    .bearer_auth(key),
            )
            .await?
            .map_err(|_| trouble::UNREACHABLE)?;
            read_moonshot(account, &j, now, "USD")
        }
        Provider::MoonshotCn => {
            let j = get_json(
                client()
                    .get("https://api.moonshot.cn/v1/users/me/balance")
                    .bearer_auth(key),
            )
            .await?
            .map_err(|_| trouble::UNREACHABLE)?;
            read_moonshot(account, &j, now, "CNY")
        }
        Provider::Openrouter => {
            let j = get_json(
                client()
                    .get("https://openrouter.ai/api/v1/key")
                    .bearer_auth(key),
            )
            .await?
            .map_err(|_| trouble::UNREACHABLE)?;
            read_openrouter(account, &j, now)
        }
        // A subscription only: registration refuses a billed one, so
        // this is a doc written by hand or by a newer khor.
        Provider::Codex | Provider::Other(_) => Err(trouble::UNKNOWN_PROVIDER),
    }
}

/// A number the provider may spell as a number or as a decimal string
/// (DeepSeek spells balances as strings).
fn decimal(v: Option<&serde_json::Value>) -> Option<f64> {
    let v = v?;
    v.as_f64()
        .or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))
        .filter(|n| n.is_finite())
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
    let rows = j
        .get("balance_infos")
        .and_then(serde_json::Value::as_array)
        .ok_or(trouble::UNREADABLE)?;
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
            r.fill = Some(if cap > 0.0 {
                ((cap - left) / cap).clamp(0.0, 1.0) as f32
            } else {
                0.0
            });
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
    reqwest::Client::builder()
        .timeout(HTTP_TIMEOUT)
        .build()
        .unwrap_or_default()
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
            let body: serde_json::Value = response.json().await.map_err(|_| trouble::UNREADABLE)?;
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
async fn fetch_newapi(
    account: &Account,
    key: &str,
    now: i64,
) -> Result<AccountReading, &'static str> {
    let base = account.base_url.as_deref().ok_or(trouble::UNREADABLE)?;
    let c = client();
    let per_unit = get_json(c.get(format!("{base}/api/status")))
        .await
        .ok()
        .and_then(Result::ok)
        .and_then(|j| {
            j.pointer("/data/quota_per_unit")
                .and_then(serde_json::Value::as_f64)
        })
        .filter(|u| *u > 0.0)
        .unwrap_or(500_000.0);
    let answer = get_json(c.get(format!("{base}/api/usage/token/")).bearer_auth(key)).await?;
    let (used, available, unlimited, expires_at, models) = match answer {
        Ok(j) => {
            let d = j.get("data").ok_or(trouble::UNREADABLE)?;
            (
                d.get("total_used")
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.0),
                d.get("total_available").and_then(serde_json::Value::as_f64),
                d.get("unlimited_quota")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
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
    // **A second request, and it is allowed to fail on its own.** The
    // log is the chart; the figures above are the reading. A gateway
    // too old for `/api/log/token`, or one that refuses it, must not
    // turn a good quota reading into 问不出来 — so this is read for
    // what it gives and dropped when it gives nothing.
    if let Ok(Ok(j)) = get_json(c.get(format!("{base}/api/log/token")).bearer_auth(key)).await {
        let log = read_log(&j, per_unit);
        r.days = log.days;
        r.model_days = log.model_days;
        r.model_tokens = log.model_tokens;
        r.log_capped = log.capped;
        //   **This road's buckets are khor's own, cut at this node's
        //   local midnight** — `read_log` groups per-request records
        //   with `TimeZone::system()`, because a person reading a
        //   gateway's log means their own day. The zone travels with
        //   the buckets so a face can name that day instead of
        //   guessing: reading these instants as UTC printed the day
        //   before for every account east of Greenwich (measured on
        //   real accounts, 2026-09-09, seven rows out of seven).
        r.zone = zone_name(&jiff::tz::TimeZone::system());
    }
    if !unlimited && let Some(avail) = available {
        let total = used + avail;
        r.limit = Some(total / per_unit);
        r.fill = Some(if total > 0.0 {
            (used / total).clamp(0.0, 1.0) as f32
        } else {
            0.0
        });
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
    d.get("expires_at")
        .and_then(serde_json::Value::as_i64)
        .filter(|at| *at > 0)
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
    if d.get("model_limits_enabled")
        .and_then(serde_json::Value::as_bool)
        != Some(true)
    {
        return Vec::new();
    }
    let Some(map) = d.get("model_limits").and_then(serde_json::Value::as_object) else {
        return Vec::new();
    };
    let mut out: Vec<String> = map.keys().cloned().collect();
    out.sort();
    out
}

/// How many records new-api's log road answers at most
/// (`common.MaxRecentItems`, `common/constants.go:60`, read 2026-09-07
/// at `0c76e4da`).
///
/// **It is a ceiling on the answer, not a page size**: the road takes
/// no time window and no page (`model/log.go:140-148`), so this many
/// records **is** everything khor can see, newest first. A busy key
/// spends it in a day.
///
/// Named here rather than in the reader so that the number and the
/// sentence about it live together, and so a gateway that answers more
/// than this some day shows up as a changed constant rather than as a
/// chart that quietly stopped being cut.
const NEWAPI_LOG_MAX: usize = 1000;

/// A new-api token's recent consumption, cut into days and into models.
///
/// **Only `type == 2` is spending** (`LogTypeConsume`,
/// `model/log.go:87`); the other kinds are top-ups, admin actions,
/// system notes, errors and refunds, and summing them would bill a
/// person for their own refund. **`quota` is in the gateway's raw
/// units**, the same ones `total_used` is in, so it takes the same
/// divisor.
///
/// Days are cut in **this machine's** zone, which is a choice and not
/// an accident: the endpoint states an instant per record and no day at
/// all, so somebody has to cut them, and the person reading the chart
/// is here (`khor_core::UsageDay` carries the ruling).
///
/// `capped` is the ceiling — see [`NEWAPI_LOG_MAX`] and
/// `AccountReading::log_capped`.
///
/// # The tokens were already on the wire
///
/// **`prompt_tokens` and `completion_tokens` sit on every one of these
/// records** (`model/log.go:59-81`), beside the `quota` this reader was
/// already taking — so the token account costs **no extra request and
/// no extra key**, only the two fields this function used to walk past.
///
/// ⚠️ **Two numbers, not four, and khor cannot say what the first one
/// counts.** The row publishes no cache read and no cache write, so
/// those two ride as `cache_split: false` — 「not reported」, which a
/// screen must not draw as zero. And **whether `prompt_tokens` already
/// nets out a cached part is not stated anywhere khor has read**: the
/// column is the gateway's own, gateways front many upstreams, and
/// there is no per-record total here to referee it the way
/// `khor_node::usage::reconciled` referees a transcript. So it is
/// carried across as the gateway's `input` and **nothing is subtracted
/// on a guess** — being wrong that way makes khor's input column too
/// large by whatever was cached, which is visible beside a
/// `cache_split: false` that says khor does not know; subtracting on a
/// guess would invent a cache figure that was never reported.
/// The IANA name of a zone, **or nothing at all**.
///
/// ⚠️ **This was `unwrap_or("UTC")` for six hours on 2026-09-09, and
/// that is the bug this function exists to name.** A zone whose name
/// khor cannot read is not UTC; it is a zone khor cannot name. Filling
/// in `"UTC"` turned 「I do not know」 into a confident wrong answer —
/// **and into exactly the wrong answer**, since reading these buckets
/// as UTC is the day-early bug the `zone` field was added to fix. The
/// old shape was worse than no field at all: a `None` leaves the face
/// knowing it is falling back, while a `"UTC"` tells it not to.
///
/// **How it happened, because the mechanism is not obvious**: `jiff`
/// was built here with `default-features = false`, which leaves out
/// `tzdb-zoneinfo`. Without it `TimeZone::system()` reads
/// `/etc/localtime` as raw TZif and hands back
/// `TimeZone(TZif("Local"))` — **the offset is right (+08:00 measured
/// on this machine) and the name is `None`**. With the feature on, the
/// same call returns `Some("Asia/Singapore")`. Both readings are from
/// this Mac, 2026-09-09; the feature is now on in the workspace
/// manifest.
///
/// **What is deliberately not done**: falling back to the numeric
/// offset (`"+08:00"`, which `Intl.DateTimeFormat` does accept). It is
/// right only while that offset holds — a bucket from the other side of
/// a daylight-saving change is cut at a local midnight the fixed offset
/// puts at 23:00 the day before, **which is the same day-shift this is
/// fixing, just rarer and seasonal**. A rarer version of the bug is a
/// worse bug. If a real machine turns up with no tzdb at all, that is
/// the rung to consider, and this is the note to read first.
fn zone_name(tz: &jiff::tz::TimeZone) -> Option<String> {
    tz.iana_name().map(str::to_owned)
}

fn read_log(j: &serde_json::Value, per_unit: f64) -> LogRead {
    let Some(rows) = j.get("data").and_then(serde_json::Value::as_array) else {
        return LogRead::default();
    };
    let zone = jiff::tz::TimeZone::system();
    // Keyed by the day's start so the two lists cannot disagree about
    // which bucket a record fell in: one pass, one decision per record.
    let mut per_day: std::collections::BTreeMap<i64, f64> = std::collections::BTreeMap::new();
    let mut per_model: std::collections::BTreeMap<(i64, String), f64> =
        std::collections::BTreeMap::new();
    // Keyed the same way, off the same records, in the same pass — so
    // 「which day did this request fall in」 is decided once and the
    // money list and the token list cannot answer it differently.
    let mut per_model_tokens: std::collections::BTreeMap<(i64, String), khor_core::Tokens> =
        std::collections::BTreeMap::new();
    for row in rows {
        if row.get("type").and_then(serde_json::Value::as_i64) != Some(2) {
            continue;
        }
        let Some(at) = row
            .get("created_at")
            .and_then(serde_json::Value::as_i64)
            .filter(|t| *t > 0)
        else {
            continue;
        };
        let Some(start) = jiff::Timestamp::from_second(at)
            .ok()
            .and_then(|t| t.to_zoned(zone.clone()).start_of_day().ok())
            .map(|d| d.timestamp().as_second())
        else {
            continue;
        };
        let spent = row
            .get("quota")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0)
            / per_unit;
        *per_day.entry(start).or_default() += spent;
        let model = row
            .get("model_name")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned();
        *per_model.entry((start, model.clone())).or_default() += spent;
        let n = |k: &str| row.get(k).and_then(serde_json::Value::as_u64).unwrap_or(0);
        per_model_tokens
            .entry((start, model))
            .or_default()
            .add(khor_core::Tokens {
                input: n("prompt_tokens"),
                cached_input: 0,
                cache_write: 0,
                output: n("completion_tokens"),
            });
    }
    let days = per_day
        .into_iter()
        .map(|(at, spent)| khor_core::DaySpend { at, spent })
        .collect();
    let model_days = per_model
        .into_iter()
        .map(|((at, model), spent)| khor_core::ModelDay { at, model, spent })
        .collect();
    let model_tokens = per_model_tokens
        .into_iter()
        // A request that billed money and reported no tokens is a row
        // of two zeroes, which reads as 「this model ran for free」.
        // Nothing else here has to be dropped: a record with no spend
        // never entered the map.
        .filter(|(_, t)| !t.is_zero())
        .map(|((at, model), tokens)| khor_core::ModelTokenDay {
            at,
            // Blank is 「the record named none」, not a model called "".
            model: (!model.is_empty()).then_some(model),
            tokens,
            cache_split: false,
        })
        .collect();
    // **`>=`, not `==`.** A gateway with a raised ceiling would answer
    // more, and「exactly a thousand」would then read as「not cut」on the
    // one build where being cut matters most.
    let capped = (rows.len() >= NEWAPI_LOG_MAX).then_some(rows.len() as u32);
    LogRead {
        days,
        model_days,
        model_tokens,
        capped,
    }
}

/// What one pass over new-api's log answers: three lists cut off the
/// same records, and whether the records were all of them.
///
/// A struct rather than a tuple because it stopped being two things.
#[derive(Default)]
struct LogRead {
    days: Vec<khor_core::DaySpend>,
    model_days: Vec<khor_core::ModelDay>,
    model_tokens: Vec<khor_core::ModelTokenDay>,
    capped: Option<u32>,
}

/// One day-and-model bucket, as both vendors' token reports shape them.
///
/// Keyed rather than pushed, because a report grouped by more than one
/// thing answers several rows per (day, model) — khor asks for one
/// grouping, and folding is what makes a second one arrive as a sum
/// instead of as duplicate rows.
type TokenRows = std::collections::BTreeMap<(i64, Option<String>), khor_core::Tokens>;

/// The folded buckets as the wire shape, oldest day first and the
/// unnamed model last within a day.
///
/// **Unnamed last, not first**, which is what `None`'s own ordering
/// would do: it is the row a reader wants after the ones that say
/// something (`khor_core::UsageModelDay` makes the same call).
fn token_rows(rows: TokenRows, cache_split: bool) -> Vec<khor_core::ModelTokenDay> {
    let mut out: Vec<khor_core::ModelTokenDay> = rows
        .into_iter()
        // A bucket the provider reported with nothing in it is not a
        // row: every one of these reports answers a bucket per day
        // whether or not anything happened in it, and a day of zeroes
        // reads as 「this model ran and cost nothing」.
        .filter(|(_, t)| !t.is_zero())
        .map(|((at, model), tokens)| khor_core::ModelTokenDay {
            at,
            model,
            tokens,
            cache_split,
        })
        .collect();
    out.sort_by(|a, b| {
        a.at.cmp(&b.at)
            .then_with(|| a.model.is_none().cmp(&b.model.is_none()))
            .then_with(|| a.model.cmp(&b.model))
    });
    out
}

/// The model a result names, or `None` where it names none.
///
/// Both reports answer `model: null` for a result that was not grouped
/// by model — which is what arrives if the grouping ask is not honoured
/// — and that is a **coarser** answer, never a wrong one: the row then
/// carries the day's whole total under 「nobody said which model」.
fn result_model(r: &serde_json::Value) -> Option<String> {
    let name = r.get("model")?.as_str()?.trim();
    (!name.is_empty()).then(|| name.to_owned())
}

/// One page of Anthropic's messages usage report, folded in.
///
/// ⚠️ **Shape from the vendor's own reference**, `GET
/// /v1/organizations/usage_report/messages` at
/// `platform.claude.com/docs/en/api/admin-api/usage-cost/get-messages-usage-report`,
/// read 2026-09-07. **Documentation, not a capture** — no khor has met
/// this endpoint with a real admin key yet, so every field is read
/// through `Option` and a page that disagrees loses figures rather than
/// the reading.
///
/// The four names land without arithmetic, which is the reason this
/// endpoint is worth a request of its own: `uncached_input_tokens` is
/// already `khor_core::Tokens::input`'s promise (input never including
/// what came out of a cache), and the cache write is the **sum of the
/// two ephemeral lifetimes** — 1 hour and 5 minutes are two prices for
/// one act, and khor's column is the act.
fn claude_token_page(j: &serde_json::Value, out: &mut TokenRows) {
    for bucket in j
        .get("data")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
    {
        // A bucket khor cannot place in time gets no row rather than
        // one at the epoch — `instant_of`'s ruling, for the reason it
        // states: a bar at 1970 is a wrong bar, not a missing one.
        let Some(at) = instant_of(bucket, "starting_at") else {
            continue;
        };
        for r in bucket
            .get("results")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
        {
            let n = |k: &str| r.get(k).and_then(serde_json::Value::as_u64).unwrap_or(0);
            let ephemeral = |k: &str| {
                r.pointer(&format!("/cache_creation/{k}"))
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0)
            };
            out.entry((at, result_model(r)))
                .or_default()
                .add(khor_core::Tokens {
                    input: n("uncached_input_tokens"),
                    cached_input: n("cache_read_input_tokens"),
                    cache_write: ephemeral("ephemeral_1h_input_tokens")
                        .saturating_add(ephemeral("ephemeral_5m_input_tokens")),
                    output: n("output_tokens"),
                });
        }
    }
}

/// One page of OpenAI's completions usage report, folded in.
///
/// ⚠️ **Shape from the vendor's own OpenAPI document**,
/// `GET /v1/organization/usage/completions` — `openai/openai-openapi` at
/// `b61ced96515cb6e73794ff459e9e12ca57596c72`, `openapi.yaml:14399`
/// (the route) and `:65381` (`UsageCompletionsResult`), read 2026-09-07
/// over `gh api`, nothing cloned. **Specification, not a capture**: no
/// khor has met this endpoint with a real admin key yet.
///
/// ⚠️⚠️ **`input_tokens` is the trap and it does not announce itself.**
/// The document says it is 「the aggregated number of input tokens used,
/// **including cached and cache-write tokens**」, while
/// `input_uncached_tokens` is the one that excludes them — and
/// `khor_core::Tokens::input` promises the second. Reading the obvious
/// name would double-count every cached token into two columns, and
/// **nothing would say so**: the figures stay plausible, the totals
/// stay positive, and only a person who knew both names would ever
/// notice. Same shape as the disagreement `khor_node::usage`'s module
/// head records between claude's and codex's transcripts, met again at
/// a different door.
///
/// The subtraction is the fallback and not the rule: a build that stops
/// sending `input_uncached_tokens` still has the same fact spelled as
/// `input_tokens` minus the two cache columns, per the same document.
fn openai_token_page(j: &serde_json::Value, out: &mut TokenRows) {
    for bucket in j
        .get("data")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(at) = instant_of(bucket, "start_time") else {
            continue;
        };
        for r in bucket
            .get("results")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
        {
            let n = |k: &str| r.get(k).and_then(serde_json::Value::as_u64).unwrap_or(0);
            let cached = n("input_cached_tokens");
            let cache_write = n("input_cache_write_tokens");
            let uncached = r
                .get("input_uncached_tokens")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or_else(|| {
                    n("input_tokens")
                        .saturating_sub(cached)
                        .saturating_sub(cache_write)
                });
            out.entry((at, result_model(r)))
                .or_default()
                .add(khor_core::Tokens {
                    input: uncached,
                    cached_input: cached,
                    cache_write,
                    output: n("output_tokens"),
                });
        }
    }
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
async fn fetch_claude(
    account: &Account,
    key: &str,
    now: i64,
) -> Result<AccountReading, &'static str> {
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
            c.get(&url)
                .header("x-api-key", key)
                .header("anthropic-version", "2023-06-01"),
        )
        .await?
        .map_err(|_| trouble::UNREACHABLE)?;
        for bucket in j
            .get("data")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
        {
            // **The bucket's own total, kept as well as summed.** The
            // day was always in this answer — it was the loop that threw
            // it away. Amounts come back in cents as decimal strings.
            let mut in_bucket = 0.0;
            for r in bucket
                .get("results")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
            {
                if let Some(a) = r.get("amount") {
                    in_bucket += a
                        .as_f64()
                        .or_else(|| a.as_str().and_then(|s| s.parse().ok()))
                        .unwrap_or(0.0);
                }
            }
            cents += in_bucket;
            if let Some(at) = instant_of(bucket, "starting_at") {
                days.push(khor_core::DaySpend {
                    at,
                    spent: in_bucket / 100.0,
                });
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
    let mut r = against_budget(
        bare(account, None, now),
        cents / 100.0,
        account.budget,
        "USD",
    );
    r.days = days;
    // **A second request, and it is allowed to fail on its own** — the
    // same bargain new-api's log road strikes: the money above is the
    // reading, the tokens are a layer beside it, and an admin key that
    // can read costs but not usage must not turn a good reading into
    // 问不出来. Cost is one call per five minutes per account
    // (`CACHE_TTL_MS`); this makes it two, which is the price of the
    // only figures either vendor states in tokens at all.
    r.model_tokens = claude_model_tokens(&c, key, &start).await;
    //   **The provider's own buckets, and they are UTC** (`starting_at`
    //   on the cost report, `bucket_width=1d`). Said rather than left
    //   empty: a face that has to ask 「which road is this」 before it
    //   can format a date is a face with a branch in it, and the branch
    //   is what went wrong on the road next door.
    r.zone = Some("UTC".to_owned());
    Ok(r)
}

/// How many pages either token report is followed for.
///
/// **One is already enough** — both are asked at `bucket_width=1d` with
/// `limit=31` over a single month, so 31 buckets is the whole answer —
/// and the cap exists for the other reason: a paging loop that trusts
/// the server's own `has_more` is a loop whose end condition is written
/// by somebody else. The cost of being wrong here is a figure missing
/// from a chart; the cost of the unbounded form is a resident that
/// never comes back.
const TOKEN_PAGES: usize = 4;

/// Anthropic's **token** report, beside the cost report the reading is
/// built from: `GET /v1/organizations/usage_report/messages`, grouped
/// by model, over the same month.
///
/// **The same admin key and the same headers as the cost report** — a
/// sibling road on one API, so this asks nothing new of the person.
///
/// ⚠️ **The header is inherited rather than verified**: the vendor's
/// reference spells its example with `Authorization: Bearer`, while
/// khor reaches this API with `x-api-key` and that is what its cost
/// report answers to today. Both are documented ways to carry an admin
/// key; if this one is the wrong one here, the road answers 401 and is
/// dropped whole — a missing layer, never a wrong number.
///
/// Returns an empty list for every way this can go wrong. See
/// `khor_core::AccountReading::model_tokens`: empty is 「no token report
/// from this provider」, which is also the ordinary answer for five of
/// the seven.
async fn claude_model_tokens(
    c: &reqwest::Client,
    key: &str,
    start: &str,
) -> Vec<khor_core::ModelTokenDay> {
    let ask = |page: Option<&str>| {
        let mut url = format!(
            "https://api.anthropic.com/v1/organizations/usage_report/messages?starting_at={start}&bucket_width=1d&limit=31&group_by[]=model"
        );
        if let Some(p) = page {
            url.push_str(&format!("&page={p}"));
        }
        url
    };
    let mut rows = TokenRows::new();
    let mut page: Option<String> = None;
    for _ in 0..TOKEN_PAGES {
        let Ok(Ok(j)) = get_json(
            c.get(ask(page.as_deref()))
                .header("x-api-key", key)
                .header("anthropic-version", "2023-06-01"),
        )
        .await
        else {
            break;
        };
        claude_token_page(&j, &mut rows);
        match j.get("next_page").and_then(serde_json::Value::as_str) {
            Some(next) if j.get("has_more").and_then(serde_json::Value::as_bool) == Some(true) => {
                page = Some(next.to_owned())
            }
            _ => break,
        }
    }
    token_rows(rows, true)
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
async fn fetch_openai(
    account: &Account,
    key: &str,
    now: i64,
) -> Result<AccountReading, &'static str> {
    let start = month_start(now);
    let c = client();
    let limit_usd = get_json(
        c.get("https://api.openai.com/v1/organization/spend_limit")
            .bearer_auth(key),
    )
    .await
    .ok()
    .and_then(Result::ok)
    .and_then(|j| {
        let cents = j
            .get("threshold_amount")
            .and_then(serde_json::Value::as_f64)?;
        (j.get("interval").and_then(serde_json::Value::as_str) == Some("month") && cents > 0.0)
            .then_some(cents / 100.0)
    });
    let mut url = format!(
        "https://api.openai.com/v1/organization/costs?start_time={start}&bucket_width=1d&limit=31"
    );
    let mut dollars = 0.0;
    let mut days: Vec<khor_core::DaySpend> = Vec::new();
    loop {
        let j = get_json(c.get(&url).bearer_auth(key))
            .await?
            .map_err(|_| trouble::UNREACHABLE)?;
        for bucket in j
            .get("data")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
        {
            let mut in_bucket = 0.0;
            for r in bucket
                .get("results")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
            {
                in_bucket += r
                    .pointer("/amount/value")
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.0);
            }
            dollars += in_bucket;
            // Already dollars here, where Anthropic's are cents.
            if let Some(at) = instant_of(bucket, "start_time") {
                days.push(khor_core::DaySpend {
                    at,
                    spent: in_bucket,
                });
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
    let mut r = against_budget(
        bare(account, None, now),
        dollars,
        limit_usd.or(account.budget),
        "USD",
    );
    r.days = days;
    // The token road, on the same terms Anthropic's is on: a layer
    // beside the reading, dropped whole if it cannot be had.
    r.model_tokens = openai_model_tokens(&c, key, start).await;
    //   OpenAI's `start_time` buckets, likewise UTC. See the note on
    //   the Anthropic road for why this is stated rather than omitted.
    r.zone = Some("UTC".to_owned());
    Ok(r)
}

/// OpenAI's **token** report: `GET /v1/organization/usage/completions`,
/// grouped by model, over the same month and on the same admin key the
/// costs road already uses.
///
/// `group_by=model` is the document's own serialisation (a `form`-style
/// array); Anthropic's sibling wants `group_by[]=model`. **Neither
/// spelling can make this wrong** — a grouping ask that is not honoured
/// comes back as one result per bucket with `model: null`, which lands
/// as one unnamed row per day carrying that day's whole total
/// (`result_model` says so where it is read).
async fn openai_model_tokens(
    c: &reqwest::Client,
    key: &str,
    start: i64,
) -> Vec<khor_core::ModelTokenDay> {
    let ask = |page: Option<&str>| {
        let mut url = format!(
            "https://api.openai.com/v1/organization/usage/completions?start_time={start}&bucket_width=1d&limit=31&group_by=model"
        );
        if let Some(p) = page {
            url.push_str(&format!("&page={p}"));
        }
        url
    };
    let mut rows = TokenRows::new();
    let mut page: Option<String> = None;
    for _ in 0..TOKEN_PAGES {
        let Ok(Ok(j)) = get_json(c.get(ask(page.as_deref())).bearer_auth(key)).await else {
            break;
        };
        openai_token_page(&j, &mut rows);
        match j.get("next_page").and_then(serde_json::Value::as_str) {
            Some(next) if j.get("has_more").and_then(serde_json::Value::as_bool) == Some(true) => {
                page = Some(next.to_owned())
            }
            _ => break,
        }
    }
    token_rows(rows, true)
}

#[cfg(test)]
mod tests {

    /// **A zone khor cannot name must not come back named UTC.**
    ///
    /// This is the 2026-09-09 regression written as a test rather than
    /// as a promise: `zone_name` filled `"UTC"` where the name was
    /// unreadable, and a screen took that at its word and printed every
    /// new-api day one early — the exact bug the field was added to
    /// fix, now wearing a value that says it is not a guess.
    ///
    /// ⚠️ **It is asserted against the offset, not against a name**,
    /// which is what lets it run anywhere: a zone at +08:00 calling
    /// itself UTC is wrong on any machine, while 「this Mac says
    /// Asia/Singapore」 is a fact about this Mac. **Before the fix this
    /// went red here** (name `"UTC"`, offset `+08:00`); on a machine
    /// genuinely on UTC it passes by agreeing, which is correct — there
    /// is nothing wrong to catch there.
    #[test]
    fn a_zone_that_is_not_utc_is_never_reported_as_utc() {
        let tz = jiff::tz::TimeZone::system();
        let now = jiff::Timestamp::now();
        if zone_name(&tz).as_deref() == Some("UTC") {
            assert_eq!(
                tz.to_offset(now),
                jiff::tz::Offset::UTC,
                "the system zone reports itself as UTC while sitting at {:?} — a name khor could not read must come back \
                 as None, never as the one zone that would make every bucket a day early",
                tz.to_offset(now),
            );
        }
        //   And the other arm: a zone that *is* named must keep its
        //   name, or the field is empty everywhere and says nothing.
        //
        //   ⚠️ **Skipped rather than unwrapped where there is no tzdb
        //   to look a name up in.** The first draft unwrapped, and on
        //   the very build this test is for — `jiff` without
        //   `tzdb-zoneinfo` — it panicked *here*, before reaching the
        //   arm above: **a red pointing at the wrong line, on the one
        //   configuration the test was written to catch.**
        match jiff::tz::TimeZone::get("Asia/Shanghai") {
            Ok(named) => assert_eq!(zone_name(&named).as_deref(), Some("Asia/Shanghai")),
            Err(_) => println!(
                "no tzdb on this build, so 「a named zone keeps its name」 was not evaluated"
            ),
        }
    }
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
        assert_eq!(
            at(serde_json::json!(0)),
            None,
            "0 is what the endpoint rewrites -1 into"
        );
        assert_eq!(
            at(serde_json::json!(-1)),
            None,
            "-1 is what the row itself stores"
        );
        assert_eq!(
            expiry_of(&serde_json::json!({})),
            None,
            "and a gateway that says nothing"
        );
        assert_eq!(
            at(serde_json::json!(1_800_000_000i64)),
            Some(1_800_000_000),
            "a real instant survives"
        );
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
        assert!(
            model_limits_of(&off).is_empty(),
            "the flag is off, so the list means nothing"
        );
        let on = serde_json::json!({"model_limits_enabled": true, "model_limits": full});
        assert_eq!(
            model_limits_of(&on),
            vec!["claude-opus-4-6".to_owned(), "gpt-5".to_owned()],
            "the flag is on: the map's keys, sorted",
        );
        let silent = serde_json::json!({"total_used": 1});
        assert!(
            model_limits_of(&silent).is_empty(),
            "a gateway that says neither"
        );
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
        assert_eq!(
            said.err(),
            Some(trouble::KEY_REFUSED),
            "a 200 that says no is a refusal"
        );

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
        assert_eq!(
            at("start_time", serde_json::json!(1_757_000_000i64)),
            Some(1_757_000_000)
        );
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
        assert!(
            as_text > 1_700_000_000,
            "…and it is this decade, not the epoch"
        );
        assert_eq!(
            at("start_time", serde_json::json!(0)),
            None,
            "the epoch is not a bucket"
        );
        assert_eq!(at("starting_at", serde_json::json!("not a time")), None);
        assert_eq!(
            instant_of(&serde_json::json!({}), "start_time"),
            None,
            "and a silent bucket"
        );
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
        assert_eq!(
            r.spent,
            Some(15.0),
            "and `spent` still means what the fraction is made of"
        );

        let uncapped = serde_json::json!({
            "data": {"limit": null, "usage": 115.0, "usage_monthly": 15.0}
        });
        let r = read_openrouter(&wallet(Provider::Openrouter, Some(100.0)), &uncapped, 0).unwrap();
        assert_eq!(
            r.spent_this_month,
            Some(15.0),
            "the month survives the other branch too"
        );
        assert_eq!(r.spent_all_time, Some(115.0), "…and so does the lifetime");
    }

    /// **The log becomes days and models, and the two lists agree.**
    ///
    /// Four claims, and each has something in the sample that would
    /// break it alone:
    ///
    /// - **only `type == 2` is spending** — the sample carries a top-up
    ///   (`type: 1`) worth more than everything else put together, so an
    ///   implementation that summed every row is off by a mile rather
    ///   than by a rounding;
    /// - **`quota` is raw units** — divided by the same `per_unit` the
    ///   quota figures use;
    /// - **the day totals equal the model totals**, asserted against
    ///   each other: they come out of one pass, and a face drawing a
    ///   stack taller than its own column would be two readings of one
    ///   number;
    /// - **a short log is not cut** — the control for `log_capped`,
    ///   without which 「always says cut」 passes.
    #[test]
    fn a_token_log_becomes_days_and_models_that_add_up() {
        let day = 86_400;
        let t = 1_800_000_000i64;
        // **The token columns ride the same rows**, because they arrive
        // in the same records and the whole point of reading them is
        // that no second request was made. Their sizes are chosen so
        // that a reader taking the wrong rows is off by a lot: the
        // top-up alone carries eighteen million.
        let rows = serde_json::json!({"success": true, "data": [
            {"type": 2, "created_at": t,           "model_name": "gpt-5",  "quota": 500_000,
             "prompt_tokens": 100, "completion_tokens": 10},
            {"type": 2, "created_at": t + 60,      "model_name": "gpt-5",  "quota": 250_000,
             "prompt_tokens": 20,  "completion_tokens": 2},
            {"type": 2, "created_at": t + 60,      "model_name": "opus",   "quota": 1_000_000,
             "prompt_tokens": 300, "completion_tokens": 30},
            {"type": 2, "created_at": t + day * 2, "model_name": "opus",   "quota": 750_000,
             "prompt_tokens": 40,  "completion_tokens": 4},
            // Not spending: a top-up, and it dwarfs the rest on purpose.
            {"type": 1, "created_at": t + 120,     "model_name": "",       "quota": 99_000_000,
             "prompt_tokens": 9_000_000, "completion_tokens": 9_000_000},
            // Unplaceable in time — dropped rather than stacked at the epoch.
            {"type": 2, "created_at": 0,           "model_name": "gpt-5",  "quota": 500_000,
             "prompt_tokens": 7, "completion_tokens": 7},
        ]});
        let log = read_log(&rows, 500_000.0);
        let (days, models) = (&log.days, &log.model_days);

        let total: f64 = days.iter().map(|d| d.spent).sum();
        assert!(
            (total - 5.0).abs() < 1e-9,
            "1+0.5+2+1.5 = 5 dollars, and no top-up in it: {total}"
        );
        assert_eq!(
            days.len(),
            2,
            "two days had spending, and the epoch row is not a third"
        );

        let by_day: f64 = days.iter().map(|d| d.spent).sum();
        let by_model: f64 = models.iter().map(|m| m.spent).sum();
        assert!(
            (by_day - by_model).abs() < 1e-9,
            "the two lists are one pass over one answer"
        );
        assert_eq!(
            models.len(),
            3,
            "gpt-5 and opus on the first day, opus on the third"
        );

        assert_eq!(log.capped, None, "six rows is not a thousand");

        // **The tokens are cut on the same records as the money.** Sums
        // rather than per-day figures, so this says the same thing in
        // every time zone — the day boundary is the money half's
        // business and is asserted there.
        let tok = &log.model_tokens;
        assert_eq!(
            tok.len(),
            3,
            "the same three (day, model) buckets the money made"
        );
        let input: u64 = tok.iter().map(|m| m.tokens.input).sum();
        let output: u64 = tok.iter().map(|m| m.tokens.output).sum();
        assert_eq!(
            (input, output),
            (460, 46),
            "100+20+300+40 and 10+2+30+4 — the top-up's eighteen million is not in it, \
             and neither are the seven the epoch row carries"
        );
        assert!(
            tok.iter().all(|m| !m.cache_split),
            "this gateway publishes no cache columns, and 「not reported」 is not zero"
        );
        assert!(
            tok.iter()
                .all(|m| m.tokens.cached_input == 0 && m.tokens.cache_write == 0),
            "…so the two it does not report stay empty rather than invented"
        );
    }

    /// **A row with no token columns makes no token row, and a row with
    /// no model name makes an unnamed one.**
    ///
    /// Two claims, each with its own pair differing in **one field**.
    /// The money arm is the control for both: every row here bills the
    /// same `quota`, so anything that changes between the two answers
    /// is the field that was changed and nothing else.
    #[test]
    fn a_log_row_without_tokens_makes_no_row_and_one_without_a_model_makes_an_unnamed_one() {
        let t = 1_800_000_000i64;
        let log = |extra: serde_json::Value| {
            let mut row = serde_json::json!({"type": 2, "created_at": t, "quota": 500_000});
            let obj = row.as_object_mut().unwrap();
            for (k, v) in extra.as_object().unwrap() {
                obj.insert(k.clone(), v.clone());
            }
            read_log(&serde_json::json!({"data": [row]}), 500_000.0)
        };

        // One field apart: `prompt_tokens` present or absent.
        let with = log(serde_json::json!({"model_name": "m", "prompt_tokens": 5}));
        let without = log(serde_json::json!({"model_name": "m"}));
        assert_eq!(with.days.len(), 1, "both rows bill money");
        assert_eq!(without.days.len(), 1);
        assert_eq!(
            with.model_tokens.len(),
            1,
            "the one that reported tokens gets a token row"
        );
        assert_eq!(with.model_tokens[0].tokens.input, 5);
        assert!(
            without.model_tokens.is_empty(),
            "and a request that reported none is not a model that ran for free"
        );

        // One field apart: `model_name` filled or blank.
        let named = log(serde_json::json!({"model_name": "m", "prompt_tokens": 5}));
        let blank = log(serde_json::json!({"model_name": "", "prompt_tokens": 5}));
        assert_eq!(named.model_tokens[0].model.as_deref(), Some("m"));
        assert_eq!(
            blank.model_tokens[0].model, None,
            "blank is 「the record named none」, never a model whose name is the empty string"
        );
    }

    /// **A log at the ceiling says so, and one under it does not.**
    ///
    /// The control is the whole test: `log_capped` decides whether a
    /// chart gets to keep calling itself a week, so an implementation
    /// that never sets it and one that always sets it are both wrong in
    /// ways only the other arm catches.
    #[test]
    fn a_log_cut_at_the_ceiling_carries_the_count() {
        let row = serde_json::json!({"type": 2, "created_at": 1_800_000_000i64, "model_name": "m", "quota": 0});
        let full = serde_json::json!({"data": vec![row.clone(); NEWAPI_LOG_MAX]});
        assert_eq!(
            read_log(&full, 1.0).capped,
            Some(NEWAPI_LOG_MAX as u32),
            "at the ceiling: cut"
        );
        let short = serde_json::json!({"data": vec![row; NEWAPI_LOG_MAX - 1]});
        assert_eq!(read_log(&short, 1.0).capped, None, "one under it: not cut");
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
            unit: None,
            kind: AccountKind::Bill,
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
            .register_account(
                "gw",
                Provider::Newapi,
                AccountKind::Bill,
                Some("http://127.0.0.1:1"),
                "sk-x",
                None,
            )
            .unwrap();
        let now = crate::live::now_ms();
        let mut refused = bare(&a, Some(trouble::KEY_REFUSED), now - 1_000);
        refused.at_ms = now - 1_000;
        n.cache_reading(
            &a.id,
            &Cached {
                reading: refused,
                failed_at_ms: Some(now - 1_000),
            },
        );
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let got = rt.block_on(n.quota_readings()).unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(
            got[0].trouble.as_deref(),
            Some(trouble::KEY_REFUSED),
            "the reason stands"
        );

        let mut fine = bare(&a, None, now - 1_000);
        fine.fill = Some(0.45);
        n.cache_reading(
            &a.id,
            &Cached {
                reading: fine,
                failed_at_ms: Some(now - 1_000),
            },
        );
        let got = rt.block_on(n.quota_readings()).unwrap();
        assert_eq!(
            got[0].fill,
            Some(0.45),
            "control: a good reading on record stands too"
        );
        assert_eq!(got[0].trouble, None);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **The unit override relabels the board, and moves no figure.**
    /// 2026-09-07,用户:「他说 usd,但可能是 rmb。同样的数字,单位不同
    /// 而已。」
    ///
    /// Three arms, and none of them is redundant:
    ///
    /// - **before** — the endpoint's own word comes through untouched,
    ///   which is what makes the second arm mean something. Without it
    ///   an implementation that always wrote `RMB` would pass.
    /// - **after** — the override wins, **and `spent` is compared
    ///   against the same number**: this is the assertion that a later
    ///   reader turning this into a conversion has to break. A test
    ///   that only read `currency` would let a rate through silently.
    /// - **blank** — an emptied box is 「use what the endpoint said」,
    ///   not an override to the empty string. Those two are one thing
    ///   to the person typing and two things to everything downstream,
    ///   so the stored value is read as well as the board's.
    #[test]
    fn the_unit_a_person_typed_relabels_the_board_and_moves_no_figure() {
        let root = std::env::temp_dir().join(format!("khor-accounts-unit-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let n = Node::open(root.clone()).unwrap();
        let a = n
            .register_account(
                "gw",
                Provider::Newapi,
                AccountKind::Bill,
                Some("http://127.0.0.1:1"),
                "sk-x",
                None,
            )
            .unwrap();
        // **Re-seeded before every read, with the identical reading.**
        // An edit drops the cached reading on purpose (the budget is a
        // denominator, see `update_account`), so without this the arms
        // after an edit would be measuring a failed live read rather
        // than the override — and they would differ in two things at
        // once.
        let seed = || {
            let mut read = bare(&a, None, crate::live::now_ms());
            read.spent = Some(123.75);
            read.currency = Some("USD".to_owned());
            n.cache_reading(
                &a.id,
                &Cached {
                    reading: read,
                    failed_at_ms: None,
                },
            );
        };
        seed();
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        let before = rt.block_on(n.quota_board()).unwrap();
        assert_eq!(before.len(), 1);
        assert_eq!(
            before[0].0.currency.as_deref(),
            Some("USD"),
            "with no override, the endpoint's word stands"
        );

        n.update_account(
            &a.id,
            "gw",
            Some("http://127.0.0.1:1"),
            None,
            None,
            Some("RMB"),
        )
        .unwrap();
        seed();
        let after = rt.block_on(n.quota_board()).unwrap();
        assert_eq!(
            after[0].0.currency.as_deref(),
            Some("RMB"),
            "the person's word wins over the endpoint's"
        );
        assert_eq!(
            after[0].0.spent, before[0].0.spent,
            "a label changed, not a rate: the figure is the one the endpoint reported"
        );

        n.update_account(
            &a.id,
            "gw",
            Some("http://127.0.0.1:1"),
            None,
            None,
            Some("   "),
        )
        .unwrap();
        seed();
        let cleared = rt.block_on(n.quota_board()).unwrap();
        assert_eq!(
            cleared[0].0.currency.as_deref(),
            Some("USD"),
            "an emptied box hands the naming back"
        );
        assert_eq!(
            n.accounts().unwrap()[0].unit,
            None,
            "…and stores nothing, rather than an override to the empty string"
        );
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
            unit: None,
            kind: AccountKind::Bill,
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
        assert_eq!(
            (r.remaining, r.currency.as_deref(), r.spent, r.fill),
            (Some(17.35), Some("USD"), None, None)
        );

        let cny_only_has_money = serde_json::json!({"is_available": true, "balance_infos": [
            {"currency": "USD", "total_balance": "0.00"}, {"currency": "CNY", "total_balance": "3.00"}]});
        let r = read_deepseek(&acc, &cny_only_has_money, 0).unwrap();
        assert_eq!(
            (r.remaining, r.currency.as_deref()),
            (Some(3.0), Some("CNY"))
        );

        let empty = serde_json::json!({"is_available": false, "balance_infos": [
            {"currency": "USD", "total_balance": "0.00"}, {"currency": "CNY", "total_balance": "0.00"}]});
        let r = read_deepseek(&acc, &empty, 0).unwrap();
        assert_eq!(
            (r.remaining, r.currency.as_deref()),
            (Some(0.0), Some("USD")),
            "empty is a reading"
        );

        let none = serde_json::json!({"is_available": true, "balance_infos": []});
        assert_eq!(
            read_deepseek(&acc, &none, 0).unwrap_err(),
            trouble::UNREADABLE
        );
    }

    /// Moonshot answers `code`/`status` around the figure; a refusal
    /// dressed as 200 (`code != 0`) is unreadable, not zero.
    #[test]
    fn a_moonshot_balance_is_read_in_the_sites_currency() {
        let acc = wallet(Provider::MoonshotCn, None);
        let j = serde_json::json!({"code": 0, "status": true, "scode": "0x0",
            "data": {"available_balance": 49.58, "voucher_balance": 40.0, "cash_balance": 9.58}});
        let r = read_moonshot(&acc, &j, 0, "CNY").unwrap();
        assert_eq!(
            (r.remaining, r.currency.as_deref(), r.spent),
            (Some(49.58), Some("CNY"), None)
        );
        let refused =
            serde_json::json!({"code": 1, "status": false, "data": {"available_balance": 49.58}});
        assert_eq!(
            read_moonshot(&acc, &refused, 0, "CNY").unwrap_err(),
            trouble::UNREADABLE
        );
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
        assert_eq!(
            (r.limit, r.remaining, r.spent),
            (Some(20.0), Some(5.0), Some(15.0))
        );
        assert_eq!(r.fill, Some(0.75));
        assert!(!r.unlimited);

        let uncapped = serde_json::json!({"data": {"limit": null, "limit_remaining": null,
            "usage": 115.0, "usage_monthly": 15.0}});
        let r = read_openrouter(&wallet(Provider::Openrouter, Some(100.0)), &uncapped, 0).unwrap();
        assert!(r.unlimited);
        assert_eq!(
            (r.spent, r.limit, r.fill),
            (Some(15.0), Some(100.0), Some(0.15))
        );

        let all_time_only = serde_json::json!({"data": {"limit": null, "usage": 115.0}});
        let r = read_openrouter(
            &wallet(Provider::Openrouter, Some(100.0)),
            &all_time_only,
            0,
        )
        .unwrap();
        assert_eq!(
            (r.spent, r.limit, r.fill),
            (Some(115.0), None, None),
            "no fraction of two different things"
        );
    }

    /// The sentence that refuses a provider word, and the usage text,
    /// must name every word `parse` accepts — two lists a person reads
    /// to learn what exists, kept honest against the one that decides.
    #[test]
    fn every_provider_word_is_named_where_a_person_would_look() {
        let refusal = msg::no_such_provider("x");
        for w in Provider::WORDS {
            assert!(
                refusal.contains(w),
                "no-such-provider does not name {w}: {refusal}"
            );
            assert!(
                khor_catalog::cli::usage().contains(w),
                "the usage text does not name {w}"
            );
        }
    }

    /// The month window starts at the first instant of the month, UTC.
    #[test]
    fn the_window_is_the_month_so_far() {
        // 2026-09-02T00:30:00Z
        let now_ms = 1_788_309_000_000;
        let start = month_start(now_ms);
        let z = jiff::Timestamp::from_second(start)
            .unwrap()
            .to_zoned(jiff::tz::TimeZone::UTC);
        assert_eq!((z.year(), z.month(), z.day(), z.hour()), (2026, 9, 1, 0));
    }

    fn subscription(home: &str) -> Account {
        Account {
            id: "s1".into(),
            name: "max".into(),
            provider: Provider::Claude,
            base_url: None,
            budget: None,
            home: home.to_owned(),
            pinned: false,
            unit: None,
            kind: AccountKind::Sub,
        }
    }

    fn a_window(
        span: khor_core::QuotaWindowSpan,
        scope: Option<&str>,
        used: f32,
    ) -> khor_core::QuotaWindow {
        khor_core::QuotaWindow {
            span,
            scope: scope.map(str::to_owned),
            used_pct: used,
            resets_at: None,
            active: None,
            severity: None,
        }
    }

    /// **A subscription's reading is that machine's quota carried
    /// whole** — and the two fields that are *not* copies are what this
    /// is really about.
    ///
    /// `fill` is taken from the quota rather than worked out again
    /// here: it is the same number a probed subscription draws with
    /// (`Quota::fill`), which is what makes 「the registered row and the
    /// probed row show the same arc」 impossible to break rather than a
    /// thing to keep true. And `at_ms` is the **reading's** age, not
    /// this call's — the quota is served out of caches up to five
    /// minutes deep, so stamping it with now would make every reading
    /// claim to be fresh.
    ///
    /// The money fields stay empty on purpose: a subscription has no
    /// 「spent」, and a figure written into `spent` would draw money on
    /// a row that has none.
    #[test]
    fn a_subscription_reading_carries_the_quota_and_keeps_its_own_age() {
        use khor_core::QuotaWindowSpan::{FiveHour, SevenDay};
        let acc = subscription("h");
        let taken = 1_788_309_000;
        let quota = Quota {
            windows: vec![
                a_window(FiveHour, None, 4.0),
                a_window(SevenDay, Some("Fable"), 71.0),
            ],
            as_of: Some(taken),
            plan: Some("Max 20x".into()),
            credits: None,
            fill: Some(0.39),
            source: None,
            limit_reached: None,
        };
        let r = subscription_reading(&acc, Ok(quota), taken * 1000 + 600_000);
        assert_eq!(r.trouble, None);
        assert_eq!(r.account, "s1");
        assert_eq!(
            r.fill,
            Some(0.39),
            "the node's one fraction, not a second opinion"
        );
        assert_eq!(
            r.at_ms,
            taken * 1000,
            "the reading's own age, not the moment it was asked for"
        );
        assert_eq!(
            r.windows.len(),
            2,
            "every window crosses, including a model's slice"
        );
        assert_eq!(r.plan.as_deref(), Some("Max 20x"));
        assert_eq!(
            (r.spent, r.limit, r.remaining),
            (None, None, None),
            "a subscription has no spend"
        );
    }

    /// **Every way a subscription can be unreadable says which way it
    /// was**, and the two new ones are the reason this matters: 「no
    /// login」 and 「the login expired」 send a person to two different
    /// actions, and there is nothing else on the row to tell them
    /// apart.
    ///
    /// The keys' *texts* cannot collide — `khor_catalog`'s build
    /// refuses two names with one text — so what is left to check here
    /// is that each cause reaches its own key, and that a trouble
    /// leaves no numbers behind it.
    #[test]
    fn a_subscription_that_cannot_be_read_says_which_kind_of_cannot() {
        let acc = subscription("h");
        for (why, key) in [
            (QuotaTrouble::NoLogin, trouble::NO_LOGIN),
            (QuotaTrouble::Stale, trouble::LOGIN_STALE),
            (QuotaTrouble::Cooling { minutes: 3 }, trouble::COOLING),
            (QuotaTrouble::Unreachable, trouble::UNREACHABLE),
            (QuotaTrouble::Unreadable, trouble::UNREADABLE),
        ] {
            let r = subscription_reading(&acc, Err(why.clone()), 7);
            assert_eq!(r.trouble.as_deref(), Some(key), "{why:?}");
            assert_eq!(r.fill, None, "{why:?}: no share behind a trouble");
            assert!(r.windows.is_empty(), "{why:?}: no windows behind a trouble");
            assert_eq!(r.name, "max", "…and the row still knows what it is");
        }
    }

    /// **A subscription takes a name and nothing else — and the extras
    /// are refused, never dropped.**
    ///
    /// Dropping is the failure worth spending a test on: a person
    /// pastes a key, reads 「registered」, and the key went nowhere.
    /// Each arm is one field on its own, so a refusal that fired for
    /// the wrong reason would show as the wrong sentence.
    ///
    /// **The control is the last line of the first half**: nothing was
    /// registered by any of the four refusals. An implementation that
    /// wrote the account first and refused afterwards passes every
    /// `unwrap_err` above it.
    ///
    /// The 「already registered here」 arm seeds the doc directly rather
    /// than registering twice, so that this test says nothing about
    /// whether *this* machine happens to be logged into Claude Code —
    /// which is a fact about the person running it, not about the code.
    #[test]
    fn a_subscription_refuses_everything_it_has_no_use_for() {
        let root = std::env::temp_dir().join(format!("khor-accounts-sub-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let n = Node::open(root.clone()).unwrap();
        let add = |p: Provider, url: Option<&str>, key: &str, budget: Option<f64>| {
            n.register_account("s", p, AccountKind::Sub, url, key, budget)
                .unwrap_err()
        };
        let no_use = msg::account_sub_takes_nothing_else("Claude Code");
        assert_eq!(add(Provider::Claude, None, "sk-x", None), no_use, "a key");
        assert_eq!(
            add(Provider::Claude, Some("https://gw"), "", None),
            no_use,
            "an address"
        );
        assert_eq!(
            add(Provider::Claude, None, "", Some(20.0)),
            no_use,
            "a budget"
        );
        assert_eq!(
            add(Provider::Newapi, None, "", None),
            msg::provider_has_no_sub("newapi", sub_provider_words().join("、")),
            "a gateway has no login to read"
        );
        assert!(
            n.accounts().unwrap().is_empty(),
            "control: a refusal registers nothing"
        );

        n.commit_account(subscription(n.device_str())).unwrap();
        assert_eq!(
            n.register_account(
                "another",
                Provider::Claude,
                AccountKind::Sub,
                None,
                "",
                None
            )
            .unwrap_err(),
            msg::account_sub_already_here("max"),
            "one machine holds one login, so a second row could only repeat the first"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The list a person is offered and the list something can read are
    /// **one list**, and this pins what is in it today.
    ///
    /// It is meant to go red the day a second vendor's login is read —
    /// that is the moment somebody has to look at the form, the
    /// refusal and the reader together, and a green test would let the
    /// first two stay behind.
    #[test]
    fn only_the_vendors_whose_login_khor_reads_can_be_subscribed_to() {
        assert_eq!(sub_provider_words(), ["claude", "codex"]);
        assert!(sub_reader(&Provider::Claude).is_some());
        assert!(sub_reader(&Provider::Codex).is_some());
        assert!(
            sub_reader(&Provider::Openai).is_none(),
            "OpenAI's API is billed; the plan is Codex's"
        );
        assert!(sub_reader(&Provider::Other("quantum-llm".into())).is_none());
        // Every word the refusal offers is a word `parse` accepts —
        // otherwise it would name a kind nobody can type.
        let refusal = msg::no_such_kind("x");
        for w in AccountKind::WORDS {
            assert!(
                refusal.contains(w),
                "no-such-kind does not name {w}: {refusal}"
            );
        }
    }
    /// ⚠️ **The one field on OpenAI's usage report that is a trap, and
    /// the fallback for the build that stops sending it.**
    ///
    /// `input_tokens` **includes** the cached and cache-write parts;
    /// `input_uncached_tokens` is the one `khor_core::Tokens::input`
    /// promises. The sample is built so the two answers cannot be
    /// confused: reading the wrong name gives 1000 where the right one
    /// gives 150, and **nothing else in the row changes** — no error,
    /// no zero, no negative, just an input column six times too big
    /// with a plausible number in it.
    ///
    /// The second arm is one field apart from the first: the same
    /// result with `input_uncached_tokens` **removed**. The subtraction
    /// has to land on the same 150, or a provider that drops the field
    /// silently starts double-counting its own cache.
    #[test]
    fn openai_input_is_the_uncached_column_and_not_the_one_that_includes_the_cache() {
        let result = |extra: serde_json::Value| {
            let mut r = serde_json::json!({
                "model": "gpt-5",
                "input_tokens": 1000,
                "input_cached_tokens": 800,
                "input_cache_write_tokens": 50,
                "output_tokens": 20,
            });
            let obj = r.as_object_mut().unwrap();
            for (k, v) in extra.as_object().unwrap() {
                obj.insert(k.clone(), v.clone());
            }
            let page =
                serde_json::json!({"data": [{"start_time": 1_800_000_000i64, "results": [r]}]});
            let mut rows = TokenRows::new();
            openai_token_page(&page, &mut rows);
            token_rows(rows, true)
        };

        let stated = result(serde_json::json!({"input_uncached_tokens": 150}));
        assert_eq!(stated.len(), 1);
        assert_eq!(
            stated[0].tokens,
            khor_core::Tokens {
                input: 150,
                cached_input: 800,
                cache_write: 50,
                output: 20
            },
            "the uncached column, never the one that includes the cache"
        );
        assert_ne!(
            stated[0].tokens.input, 1000,
            "…and 1000 is what reading `input_tokens` gives"
        );

        let derived = result(serde_json::json!({}));
        assert_eq!(
            derived[0].tokens, stated[0].tokens,
            "with the column absent, 1000 - 800 - 50 is the same fact spelled the other way"
        );
    }

    /// **Anthropic's cache write is two lifetimes added, and its four
    /// names land without arithmetic.**
    ///
    /// One hour and five minutes are two prices for one act, and khor's
    /// column is the act — so a reader taking either alone loses the
    /// other silently. The control is that the two are different
    /// numbers: with `1h == 5m` a sum, a maximum and either one alone
    /// would all be wrong in ways this test could not tell apart.
    #[test]
    fn a_claude_usage_bucket_adds_both_cache_lifetimes_into_one_column() {
        let page = serde_json::json!({"data": [{
            "starting_at": "2027-01-15T00:00:00Z",
            "results": [{
                "model": "claude-opus-5",
                "uncached_input_tokens": 11,
                "cache_read_input_tokens": 22,
                "cache_creation": {"ephemeral_1h_input_tokens": 300, "ephemeral_5m_input_tokens": 44},
                "output_tokens": 55,
            }],
        }]});
        let mut rows = TokenRows::new();
        claude_token_page(&page, &mut rows);
        let out = token_rows(rows, true);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].model.as_deref(), Some("claude-opus-5"));
        assert_eq!(
            out[0].tokens,
            khor_core::Tokens {
                input: 11,
                cached_input: 22,
                cache_write: 344,
                output: 55
            },
            "300 + 44 — not 300, not 44, and not the larger of the two"
        );
        assert!(
            out[0].cache_split,
            "this vendor states the cache, so the two columns mean something"
        );
    }

    /// **A report that did not honour the grouping is coarser, never
    /// wrong.**
    ///
    /// Neither vendor's array-parameter spelling is verified against a
    /// live endpoint, so the question that matters is what happens if
    /// the ask is ignored: the answer comes back with `model: null` and
    /// one result per bucket, and it has to land as **one unnamed row
    /// carrying that day's whole total** — the day figure still true,
    /// the breakdown honestly absent.
    ///
    /// The control is the second arm, one field apart: the same bucket
    /// with a model named. Without it, 「files everything under None」
    /// would pass.
    #[test]
    fn an_ungrouped_token_report_lands_as_one_unnamed_row_that_still_totals() {
        let bucket = |model: serde_json::Value| {
            let page = serde_json::json!({"data": [{
                "start_time": 1_800_000_000i64,
                "results": [{
                    "model": model,
                    "input_uncached_tokens": 7,
                    "input_cached_tokens": 0,
                    "input_cache_write_tokens": 0,
                    "output_tokens": 3,
                }],
            }]});
            let mut rows = TokenRows::new();
            openai_token_page(&page, &mut rows);
            token_rows(rows, true)
        };
        let ungrouped = bucket(serde_json::Value::Null);
        assert_eq!(ungrouped.len(), 1);
        assert_eq!(ungrouped[0].model, None, "nobody said which model");
        assert_eq!(
            (ungrouped[0].tokens.input, ungrouped[0].tokens.output),
            (7, 3),
            "and the day's total is still the day's total"
        );
        let grouped = bucket(serde_json::json!("gpt-5"));
        assert_eq!(
            grouped[0].model.as_deref(),
            Some("gpt-5"),
            "the control: a named row is named"
        );
        assert_eq!(
            grouped[0].tokens, ungrouped[0].tokens,
            "same tokens, one field apart"
        );
    }

    /// **An empty bucket is not a row, and the unnamed row sorts last.**
    ///
    /// Both reports answer a bucket per day whether or not anything
    /// happened in it, so 「a day of zeroes」 arrives on its own and
    /// reads as 「this model ran and cost nothing」. And `None`'s own
    /// ordering would put the one row khor cannot name at the **top**
    /// of every day, which is the opposite of useful.
    #[test]
    fn an_empty_token_bucket_is_dropped_and_the_unnamed_row_comes_last() {
        let page = serde_json::json!({"data": [{
            "start_time": 1_800_000_000i64,
            "results": [
                {"model": serde_json::Value::Null, "input_uncached_tokens": 1, "output_tokens": 0},
                {"model": "zzz", "input_uncached_tokens": 1, "output_tokens": 0},
                {"model": "aaa", "input_uncached_tokens": 1, "output_tokens": 0},
                {"model": "empty", "input_uncached_tokens": 0, "output_tokens": 0},
            ],
        }]});
        let mut rows = TokenRows::new();
        openai_token_page(&page, &mut rows);
        let out = token_rows(rows, true);
        assert_eq!(
            out.iter().map(|m| m.model.as_deref()).collect::<Vec<_>>(),
            vec![Some("aaa"), Some("zzz"), None],
            "named first and in order, the unnamed one last, and the empty bucket not at all"
        );
    }

    /// **Somebody actually asks for the tokens** — not just 「the reader
    /// is written correctly」.
    ///
    /// The parsers above prove the shapes are read right. They would go
    /// on passing with the line that fills `model_tokens` deleted from
    /// `fetch_newapi`, because a test that calls a function directly
    /// never asks whether anything else does. So this one walks the
    /// whole road against a real socket: `fetch_newapi` makes its three
    /// requests, and the tokens have to come out the far end.
    ///
    /// **It also proves the tokens cost no second road**: the log
    /// answered here is the same one the money already came from, and
    /// both figures land off it.
    ///
    /// ⚠️ **The two vendors have no equivalent and cannot have one
    /// here**: their hosts are literals in `claude_model_tokens` and
    /// `openai_model_tokens`, so there is nowhere to point a local
    /// socket. Their call sites are ungated, and that is stated rather
    /// than papered over.
    #[tokio::test]
    async fn a_gateways_tokens_travel_the_whole_road_and_not_only_the_parser() {
        use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = l.local_addr().unwrap().port();
        tokio::spawn(async move {
            for _ in 0..6 {
                let Ok((mut sock, _)) = l.accept().await else {
                    break;
                };
                let mut buf = [0u8; 2048];
                let n = sock.read(&mut buf).await.unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                let body = if req.contains("/api/status") {
                    r#"{"data":{"quota_per_unit":500000}}"#.to_owned()
                } else if req.contains("/api/log/token") {
                    r#"{"data":[{"type":2,"created_at":1800000000,"model_name":"gpt-5","quota":500000,"prompt_tokens":123,"completion_tokens":45}]}"#.to_owned()
                } else {
                    r#"{"code":true,"data":{"total_used":500000,"total_available":500000,"unlimited_quota":false,"expires_at":0}}"#.to_owned()
                };
                let head = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                    body.len(),
                );
                let _ = sock.write_all(head.as_bytes()).await;
                let _ = sock.write_all(body.as_bytes()).await;
                let _ = sock.shutdown().await;
            }
        });

        let mut account = wallet(Provider::Newapi, None);
        account.base_url = Some(format!("http://127.0.0.1:{port}"));
        let r = fetch_newapi(&account, "sk-x", 0)
            .await
            .expect("the gateway answered");

        assert_eq!(
            r.days.len(),
            1,
            "the money came off the log, as it did before"
        );
        assert_eq!(
            r.model_tokens.len(),
            1,
            "and the tokens came off the same log"
        );
        assert_eq!(r.model_tokens[0].model.as_deref(), Some("gpt-5"));
        assert_eq!(
            (
                r.model_tokens[0].tokens.input,
                r.model_tokens[0].tokens.output
            ),
            (123, 45),
            "the figures the gateway actually sent, through the reader that is actually called"
        );
        assert!(
            !r.model_tokens[0].cache_split,
            "a gateway log has no cache columns to state"
        );
    }

    /// **A reading *file* written before the token report still
    /// parses.**
    ///
    /// `.khor/quotas/<id>.json` is JSON with named fields, and **there
    /// are such files on real machines right now**, written by the khor
    /// that ran before this field existed. The old text is spelled out
    /// by hand rather than round-tripped, because serialising the
    /// current struct and deleting a key is asking the new code to
    /// describe the old one.
    ///
    /// **Its own test, not one half of a bigger one.** The wire surface
    /// below is a different codec with a different way of spelling 「the
    /// field is absent」, and two surfaces sharing a test means the
    /// second assertion is never seen to fail — the first one always
    /// gets there first.
    #[test]
    fn a_reading_file_written_before_the_token_report_still_parses() {
        let r: AccountReading =
            serde_json::from_str(OLD_READING_JSON).expect("the file on disk still parses");
        assert_eq!(r.spent, Some(1.5), "what it did record survives whole");
        assert_eq!(r.days.len(), 1);
        assert!(
            r.model_tokens.is_empty(),
            "a reading from before the token report says so by being empty, not by being wrong"
        );
    }

    /// One `.khor/quotas/<id>.json` as the khor before 2026-09-07 wrote
    /// it: every field that existed then, and none that did not.
    const OLD_READING_JSON: &str = r#"{"account":"a","name":"n","provider":"newapi","home":"h",
        "fill":null,"spent":1.5,"limit":null,"currency":"USD","unlimited":false,"resets_at":null,
        "trouble":null,"at_ms":7,"remaining":null,"expires_at":null,"models":[],
        "days":[{"at":1800000000,"spent":1.5}],"spent_this_month":null,"spent_all_time":null,
        "model_days":[],"log_capped":null,"windows":[],"plan":null,"credits":null}"#;

    /// **A reading *frame* from before the token report still decodes.**
    ///
    /// The wire is positional msgpack, so 「the field is absent」 is a
    /// **shorter array** — and the only honest way to produce one is to
    /// write the struct that produces it, which is what `OldReading`
    /// is. The length is asserted for the same reason `khor_node::proto`
    /// asserts it on its own old frames: a struct that quietly still
    /// carried the field would round-trip and prove nothing.
    #[test]
    fn a_reading_frame_from_before_the_token_report_still_decodes() {
        #[derive(serde::Serialize)]
        struct OldReading {
            account: String,
            name: String,
            provider: String,
            home: String,
            fill: Option<f32>,
            spent: Option<f64>,
            limit: Option<f64>,
            currency: Option<String>,
            unlimited: bool,
            resets_at: Option<i64>,
            trouble: Option<String>,
            at_ms: i64,
            remaining: Option<f64>,
            expires_at: Option<i64>,
            models: Vec<String>,
            days: Vec<khor_core::DaySpend>,
            spent_this_month: Option<f64>,
            spent_all_time: Option<f64>,
            model_days: Vec<khor_core::ModelDay>,
            log_capped: Option<u32>,
            windows: Vec<khor_core::QuotaWindow>,
            plan: Option<String>,
            credits: Option<khor_core::QuotaCredits>,
        }
        let mut want = bare(&wallet(Provider::Newapi, None), None, 7);
        want.spent = Some(1.5);
        want.currency = Some("USD".to_owned());
        want.days = vec![khor_core::DaySpend {
            at: 1_800_000_000,
            spent: 1.5,
        }];

        let old_wire = rmp_serde::to_vec(&OldReading {
            account: want.account.clone(),
            name: want.name.clone(),
            provider: want.provider.clone(),
            home: want.home.clone(),
            fill: want.fill,
            spent: want.spent,
            limit: want.limit,
            currency: want.currency.clone(),
            unlimited: want.unlimited,
            resets_at: want.resets_at,
            trouble: want.trouble.clone(),
            at_ms: want.at_ms,
            remaining: want.remaining,
            expires_at: want.expires_at,
            models: want.models.clone(),
            days: want.days.clone(),
            spent_this_month: want.spent_this_month,
            spent_all_time: want.spent_all_time,
            model_days: want.model_days.clone(),
            log_capped: want.log_capped,
            windows: want.windows.clone(),
            plan: want.plan.clone(),
            credits: want.credits.clone(),
        })
        .unwrap();
        let fresh = rmp_serde::to_vec(&want).unwrap();
        assert!(
            old_wire.len() < fresh.len(),
            "the old frame really is shorter: it lacks the field"
        );

        let back: AccountReading = rmp_serde::from_slice(&old_wire).unwrap();
        assert!(
            back.model_tokens.is_empty(),
            "an older peer's answer carries no token list"
        );
        assert_eq!(back, want, "and everything it did carry arrives unchanged");
    }
}
