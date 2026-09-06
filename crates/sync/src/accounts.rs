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
///
/// **A word this build has never heard of still comes through**, as
/// [`Provider::Other`], carrying the word. [`AccountDoc::get`] reads
/// each account with `.ok()`, so a variant this build could not decode
/// would not be an error — it would be a registration that is silently
/// not there, on exactly the machines least able to know why. An
/// account registered by a newer khor shows on an older one with its
/// word as itself, its home's reading carried like any other; only
/// *reading the provider* is beyond that build, and it says so
/// (`khor_node::accounts::trouble::UNKNOWN_PROVIDER`). `Other` is never
/// registered — [`Provider::parse`] refuses an unknown word by name —
/// it only ever arrives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Provider {
    /// A new-api gateway (github.com/QuantumNous/new-api): the token's
    /// own key reads its own quota.
    Newapi,
    /// Anthropic's API, read through an **admin** key (the usage and
    /// cost reports are organisation-level).
    Claude,
    /// OpenAI's API, likewise through an admin key.
    Openai,
    /// DeepSeek's platform: the key's own prepaid balance.
    Deepseek,
    /// Moonshot AI's global site (api.moonshot.ai, billed in USD).
    Moonshot,
    /// Moonshot AI's China site (api.moonshot.cn, billed in CNY). Two
    /// words rather than one with an address: the site decides the
    /// currency, and a currency guessed from a URL is a guess.
    MoonshotCn,
    /// OpenRouter: the key's own spend cap and usage.
    Openrouter,
    /// A word from a newer khor: carried and shown, never read here.
    Other(String),
}

impl Provider {
    /// The words `parse` accepts, spelled as `word` spells them — for
    /// the sentence that refuses one it does not, and for the usage
    /// text, so neither can list fewer than exist.
    pub const WORDS: [&'static str; 7] =
        ["newapi", "claude", "openai", "deepseek", "moonshot", "moonshot-cn", "openrouter"];

    pub fn parse(word: &str) -> Option<Provider> {
        match word.trim().to_ascii_lowercase().as_str() {
            "newapi" | "new-api" => Some(Provider::Newapi),
            "claude" | "anthropic" => Some(Provider::Claude),
            "openai" => Some(Provider::Openai),
            "deepseek" => Some(Provider::Deepseek),
            "moonshot" | "moonshotai" => Some(Provider::Moonshot),
            "moonshot-cn" | "moonshotai-cn" => Some(Provider::MoonshotCn),
            "openrouter" => Some(Provider::Openrouter),
            _ => None,
        }
    }

    pub fn word(&self) -> &str {
        match self {
            Provider::Newapi => "newapi",
            Provider::Claude => "claude",
            Provider::Openai => "openai",
            Provider::Deepseek => "deepseek",
            Provider::Moonshot => "moonshot",
            Provider::MoonshotCn => "moonshot-cn",
            Provider::Openrouter => "openrouter",
            Provider::Other(word) => word,
        }
    }
}

impl Serialize for Provider {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.word())
    }
}

impl<'de> Deserialize<'de> for Provider {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let word = String::deserialize(d)?;
        Ok(Provider::parse(&word).unwrap_or(Provider::Other(word)))
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
    /// Floated to the top of the 额度 list, everywhere.
    ///
    /// **Tail field, `serde default`, and both halves matter.** This
    /// doc replicates, so a machine running an older khor decodes an
    /// account written by a newer one — a required field would make
    /// that account fail to parse, and [`AccountDoc::get`] reads with
    /// `.ok()`, so the failure would not be an error: it would be a
    /// registration that is **silently not there**, on exactly the
    /// machines least able to work out why. Defaulted, an older build
    /// reads the account whole and only misses the pin.
    ///
    /// **The order lives with the data, not with a screen**, the same
    /// bargain the device list makes: two faces sorting the same rows
    /// by their own rules is two answers to 「what is at the top」.
    #[serde(default)]
    pub pinned: bool,
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

    /// Every account: **pinned first**, then by name, then by id — the
    /// order a list shows them in.
    ///
    /// Id last so the order is total: two accounts can share a name
    /// (nothing forbids it), and a sort that stopped at the name would
    /// put them in whatever order the map handed them, which is not the
    /// same order twice.
    pub fn all(&self) -> Vec<Account> {
        let map = self.inner.get_map(ACCOUNTS);
        let mut out: Vec<Account> = map.keys().filter_map(|k| self.get(&k)).collect();
        out.sort_by(|a, b| {
            b.pinned.cmp(&a.pinned).then(a.name.cmp(&b.name)).then(a.id.cmp(&b.id))
        });
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
            pinned: false,
        }
    }

    /// **An account written by an older khor still decodes, and it is
    /// not pinned.**
    ///
    /// The bytes are built by hand rather than by serialising a struct
    /// with the field removed — a struct cannot have the field removed
    /// without deleting it, so the only way to hold an *old* shape is
    /// to write one. **First it is asserted that the key really is
    /// absent from those bytes**: without that line this test would
    /// pass just as well against a document that has `"pinned": false`
    /// in it, which is the shape it is meant to exclude, and the whole
    /// thing would be checking that serde can read its own output.
    #[test]
    fn an_account_from_before_the_pin_reads_as_unpinned() {
        let old = r#"{"id":"k1","name":"gateway","provider":"newapi",
            "base_url":"https://gw.example","home":"aa"}"#;
        assert!(!old.contains("pinned"), "the old shape must not carry the key it predates");
        let account: Account = serde_json::from_str(old).expect("an older khor's account still reads");
        assert_eq!(account, one("k1", "gateway"), "every other field survives whole");
        assert!(!account.pinned, "absent is not pinned");
    }

    /// **Pinned floats, and the order under it is unchanged.**
    ///
    /// Three accounts, and the pinned one is the *last* by name — so an
    /// implementation that ignored the flag would answer them in a
    /// different order than this asks for, rather than accidentally
    /// agreeing with it. (Two rows, with the pinned one already first
    /// alphabetically, is a control group that cannot fail.)
    #[test]
    fn a_pinned_account_floats_and_the_rest_keep_their_order() {
        let doc = AccountDoc::new(1).unwrap();
        doc.set(&one("k1", "alpha")).unwrap();
        doc.set(&one("k2", "beta")).unwrap();
        let mut last = one("k3", "zulu");
        last.pinned = true;
        doc.set(&last).unwrap();
        let names: Vec<String> = doc.all().into_iter().map(|a| a.name).collect();
        assert_eq!(names, ["zulu", "alpha", "beta"], "pinned first, then by name");
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
        assert_eq!(Provider::parse("openai").as_ref().map(Provider::word), Some("openai"));
        assert_eq!(Provider::parse("moonshotai-cn"), Some(Provider::MoonshotCn));
        assert_eq!(Provider::parse("azure"), None);
        let json = serde_json::to_string(&one("k", "n")).unwrap();
        assert!(json.contains("\"provider\":\"newapi\""), "{json}");
        for w in Provider::WORDS {
            assert_eq!(Provider::parse(w).as_ref().map(Provider::word), Some(w), "{w} round-trips");
        }
    }

    /// **A word from a newer khor comes through, and goes back out
    /// unchanged.** The doc reads each account with `.ok()`, so a
    /// variant this build could not decode would not be an error — it
    /// would be a registration silently not there. An unknown word
    /// therefore decodes to `Other`, is spelled as itself, re-serialises
    /// to the same bytes, and sits in the doc's list like any other. The
    /// last line is the control: the same word is refused at
    /// registration, so `Other` can only ever arrive.
    #[test]
    fn a_provider_word_from_the_future_is_carried_not_dropped() {
        let json = r#"{"id":"k","name":"n","provider":"quantum-llm","home":"aa"}"#;
        let a: Account = serde_json::from_str(json).unwrap();
        assert_eq!(a.provider, Provider::Other("quantum-llm".into()));
        assert_eq!(a.provider.word(), "quantum-llm");
        let back = serde_json::to_string(&a).unwrap();
        assert!(back.contains("\"provider\":\"quantum-llm\""), "{back}");
        let d = AccountDoc::new(1).unwrap();
        d.set(&a).unwrap();
        assert_eq!(d.all(), vec![a], "listed, not dropped");
        assert_eq!(Provider::parse("quantum-llm"), None, "control: never registered here");
    }
}
