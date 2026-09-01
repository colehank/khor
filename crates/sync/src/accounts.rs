//! Registered API accounts: whose usage the 额度 page draws a ring for.
//!
//! **The key is not in here, and that is the whole design.** This doc
//! replicates to every machine the way the agent registry does, so a
//! secret written into it would be a secret on every disk in the mesh —
//! mandala's phone-sync of secrets is the counter-example khor refuses
//! (`agents.rs` says the same about commands). What crosses is the
//! *description* of an account: its name, its provider, where it
//! points, a budget if the person set one, and **which machine holds
//! the key** (`home`). Only that machine ever reads the account; every
//! other machine reads that machine's reading, the way vitals cross.
//!
//! Adding a provider is adding a variant here and a fetcher in the
//! node — nothing else knows the list.

use loro::{LoroDoc, LoroValue, VersionVector};
use serde::{Deserialize, Serialize};

use crate::glue;
use crate::store::Doc;

pub const REL_DIR: &str = ".khor/accounts";

pub fn accounts_dir(home: &std::path::Path) -> std::path::PathBuf {
    home.join(REL_DIR)
}

const ACCOUNTS: &str = "accounts";

/// Who answers for this account's usage. Spelled as the words a person
/// registers with (`khor accounts add --provider newapi`), and carried
/// on the wire as those words.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    /// A new-api gateway (github.com/QuantumNous/new-api): the token's
    /// own key reads its own quota.
    Newapi,
    /// Anthropic's API, read through an **admin** key (the usage and
    /// cost reports are organisation-level).
    Claude,
    /// OpenAI's API, likewise through an admin key.
    Openai,
}

impl Provider {
    pub fn parse(word: &str) -> Option<Provider> {
        match word.trim().to_ascii_lowercase().as_str() {
            "newapi" | "new-api" => Some(Provider::Newapi),
            "claude" | "anthropic" => Some(Provider::Claude),
            "openai" => Some(Provider::Openai),
            _ => None,
        }
    }

    pub fn word(self) -> &'static str {
        match self {
            Provider::Newapi => "newapi",
            Provider::Claude => "claude",
            Provider::Openai => "openai",
        }
    }
}

/// One registered account, as every machine sees it. No secret.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub name: String,
    pub provider: Provider,
    /// Where the provider is reached. Required for new-api (every
    /// gateway has its own address); the two vendors have a fixed one
    /// and carry `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    /// What "full" means for a provider that has no hard limit of its
    /// own (the vendors bill per token): month-to-date spend against
    /// this, in the provider's currency. `None` draws a track and a
    /// number, never a fraction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget: Option<f64>,
    /// The device (hex id) that holds the key and reads the account.
    pub home: String,
}

pub struct AccountDoc {
    inner: LoroDoc,
}

impl AccountDoc {
    pub fn new(peer: u64) -> Result<Self, String> {
        Ok(Self { inner: glue::with_peer(peer)? })
    }

    pub fn get(&self, id: &str) -> Option<Account> {
        let value = self.inner.get_map(ACCOUNTS).get(id)?.into_value().ok()?;
        let LoroValue::String(json) = value else { return None };
        serde_json::from_str(&json).ok()
    }

    pub fn set(&self, account: &Account) -> Result<(), String> {
        if self.get(&account.id).as_ref() == Some(account) {
            return Ok(());
        }
        let json = serde_json::to_string(account).map_err(|e| e.to_string())?;
        self.inner.get_map(ACCOUNTS).insert(&account.id, json).map_err(|e| e.to_string())?;
        self.inner.commit();
        Ok(())
    }

    pub fn remove(&self, id: &str) -> Result<(), String> {
        if self.get(id).is_none() {
            return Ok(());
        }
        self.inner.get_map(ACCOUNTS).delete(id).map_err(|e| e.to_string())?;
        self.inner.commit();
        Ok(())
    }

    /// Every account, by name then id — the order a list shows them in.
    pub fn all(&self) -> Vec<Account> {
        let map = self.inner.get_map(ACCOUNTS);
        let mut out: Vec<Account> = map.keys().filter_map(|k| self.get(&k)).collect();
        out.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id)));
        out
    }
}

impl Doc for AccountDoc {
    fn open(peer: u64) -> Result<Self, String> {
        AccountDoc::new(peer)
    }

    fn peer_id(&self) -> u64 {
        self.inner.peer_id()
    }

    fn version(&self) -> VersionVector {
        self.inner.oplog_vv()
    }

    fn changes_since(&self, theirs: &VersionVector) -> Result<Vec<u8>, String> {
        glue::changes_since(&self.inner, theirs)
    }

    fn snapshot(&self) -> Result<Vec<u8>, String> {
        glue::snapshot(&self.inner)
    }

    fn merge(&self, bytes: &[u8]) -> Result<(), String> {
        glue::merge(&self.inner, bytes)
    }

    fn items(&self) -> usize {
        self.all().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(id: &str, name: &str) -> Account {
        Account {
            id: id.to_owned(),
            name: name.to_owned(),
            provider: Provider::Newapi,
            base_url: Some("https://gw.example".to_owned()),
            budget: None,
            home: "aa".to_owned(),
        }
    }

    /// The doc carries the description and nothing else: what a merge
    /// hands another machine has no field a key could be in — asserted
    /// on the bytes a peer would receive, because that is what leaves
    /// this machine.
    #[test]
    fn an_account_crosses_whole_and_carries_no_secret() {
        let a = AccountDoc::new(1).unwrap();
        a.set(&one("k1", "gateway")).unwrap();
        let bytes = a.snapshot().unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains("sk-"), "no key shape in the bytes");
        let b = AccountDoc::new(2).unwrap();
        b.merge(&bytes).unwrap();
        assert_eq!(b.all(), vec![one("k1", "gateway")]);
        b.remove("k1").unwrap();
        a.merge(&b.changes_since(&a.version()).unwrap()).unwrap();
        assert!(a.all().is_empty(), "a removal crosses too");
    }

    #[test]
    fn a_provider_is_spelled_the_way_it_is_registered() {
        assert_eq!(Provider::parse("new-api"), Some(Provider::Newapi));
        assert_eq!(Provider::parse("Anthropic"), Some(Provider::Claude));
        assert_eq!(Provider::parse("openai").map(Provider::word), Some("openai"));
        assert_eq!(Provider::parse("azure"), None);
        let json = serde_json::to_string(&one("k", "n")).unwrap();
        assert!(json.contains("\"provider\":\"newapi\""), "{json}");
    }
}
