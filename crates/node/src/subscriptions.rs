//! Discover logins on their own machine, then register only the selected one.
//! Neither discovery nor import transports a credential or calls a usage API.

use khor_catalog::msg;
use khor_sync::accounts::{Account, AccountKind, Provider};
use serde::{Deserialize, Serialize};

use crate::{Node, accounts::sub_reader};

// One resident can receive imports from several screens at once.
static IMPORT: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    Discover,
    Import { provider: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionCandidate {
    pub provider: String,
    pub vendor: String,
    /// Already registered there; it may not have synced to the asker yet.
    pub account: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Answer {
    Candidates {
        candidates: Vec<SubscriptionCandidate>,
    },
    Imported {
        #[serde(with = "account_map")]
        account: Account,
    },
}

// Account is a JSON document with omitted optional fields. MessagePack's
// positional struct encoding would shift `pinned` into `budget`; keep its
// named document shape when carrying it over either the wire or resident IPC.
mod account_map {
    use super::Account;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(account: &Account, serializer: S) -> Result<S::Ok, S::Error> {
        serde_json::to_value(account)
            .map_err(serde::ser::Error::custom)?
            .serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Account, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        serde_json::from_value(value).map_err(serde::de::Error::custom)
    }
}

impl Node {
    pub(crate) fn subscriptions_here(&self, action: Action) -> Result<Answer, String> {
        self.subscriptions_with_login(action, |p| (p.logged_in)())
    }

    pub(crate) fn subscriptions_with_login(
        &self,
        action: Action,
        logged_in: impl Fn(&'static crate::quota::Provider) -> bool,
    ) -> Result<Answer, String> {
        let _guard = matches!(&action, Action::Import { .. })
            .then(|| IMPORT.lock().unwrap_or_else(|p| p.into_inner()));
        let accounts = self.accounts()?;
        let existing = |provider: &str| {
            accounts.iter().find(|a| {
                a.home == self.device_str() && a.kind.is_sub() && a.provider.word() == provider
            })
        };
        match action {
            Action::Discover => Ok(Answer::Candidates {
                candidates: crate::quota::providers()
                    .into_iter()
                    .filter(|p| logged_in(p))
                    .map(|p| SubscriptionCandidate {
                        provider: p.id.into(),
                        vendor: p.vendor.into(),
                        account: existing(p.id).map(|a| a.id.clone()),
                    })
                    .collect(),
            }),
            Action::Import { provider } => {
                let parsed =
                    Provider::parse(&provider).ok_or_else(|| msg::no_such_provider(&provider))?;
                let reader = sub_reader(&parsed).ok_or_else(|| {
                    msg::provider_has_no_sub(
                        &provider,
                        crate::accounts::sub_provider_words().join("、"),
                    )
                })?;
                if !logged_in(reader) {
                    return Err(msg::account_sub_no_login(reader.vendor, reader.command));
                }
                // Retrying a successful import returns the same registration.
                if let Some(account) = existing(reader.id) {
                    self.adopt_subscriptions_with(|_| false)?;
                    return Ok(Answer::Imported {
                        account: account.clone(),
                    });
                }
                let account = self.commit_account(Account {
                    id: crate::link::fresh_hex()?,
                    name: msg::sub_auto_name(reader.vendor),
                    provider: parsed,
                    base_url: None,
                    budget: None,
                    unit: None,
                    home: self.device_str().into(),
                    pinned: false,
                    kind: AccountKind::Sub,
                })?;
                self.adopt_subscriptions_with(|_| false)?;
                Ok(Answer::Imported { account })
            }
        }
    }

    /// The selected row is available immediately, before the next sync round.
    /// Do not overwrite metadata already edited on this machine.
    pub(crate) fn keep_imported_subscription(
        &self,
        account: &Account,
        home: &str,
    ) -> Result<(), String> {
        if account.home != home || !account.kind.is_sub() || sub_reader(&account.provider).is_none()
        {
            return Err(msg::subscription_wrong_home().into());
        }
        let loaded = self.accounts_loaded()?;
        if loaded.doc.get(&account.id).is_none() {
            loaded.doc.set(account)?;
            let mut store = loaded.store;
            store.flush(&loaded.doc)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testdir::TempTree;

    fn imported(answer: Answer) -> Account {
        match answer {
            Answer::Imported { account } => account,
            _ => panic!("expected imported account"),
        }
    }

    #[test]
    fn discovery_is_read_only_and_import_is_explicit_and_repeatable() {
        let root = TempTree::fresh("subscription-discovery");
        let n = Node::open(root.path()).unwrap();
        let discovered = n
            .subscriptions_with_login(Action::Discover, |_| true)
            .unwrap();
        let Answer::Candidates { candidates } = discovered else {
            panic!("expected candidates")
        };
        assert_eq!(candidates.len(), 2);
        assert!(candidates.iter().all(|c| c.account.is_none()));
        assert!(
            n.accounts().unwrap().is_empty(),
            "opening discovery must not register anything"
        );
        let action = || Action::Import {
            provider: "codex".into(),
        };
        let account = imported(
            n.subscriptions_with_login(action(), |p| p.id == "codex")
                .unwrap(),
        );
        assert_eq!(account.home, n.device_str());
        let bytes = crate::proto::encode(&Answer::Imported {
            account: account.clone(),
        })
        .unwrap();
        assert_eq!(
            imported(crate::proto::decode(&bytes).unwrap()),
            account,
            "optional document fields must retain their positions across the wire"
        );
        assert_eq!(
            n.accounts().unwrap().len(),
            1,
            "only the selected provider is imported"
        );
        assert_eq!(
            imported(n.subscriptions_with_login(action(), |_| true).unwrap()).id,
            account.id
        );
        assert!(
            !root.join(".khor/keys").exists(),
            "no credential is copied or written"
        );
        assert!(
            n.subscriptions_with_login(action(), |_| false).is_err(),
            "login was removed after discovery"
        );
        assert!(
            n.subscriptions_with_login(
                Action::Import {
                    provider: "newapi".into()
                },
                |_| true
            )
            .is_err()
        );
        n.remove_account(&account.id).unwrap();
        assert_eq!(
            n.adopt_subscriptions_with(|_| true).unwrap(),
            1,
            "only the other vendor auto-adopts; a deleted imported row stays deleted"
        );
        assert_ne!(
            imported(n.subscriptions_with_login(action(), |_| true).unwrap()).id,
            account.id,
            "explicit import restores a deleted registration"
        );
    }

    #[test]
    fn imported_registration_checks_home_and_preserves_existing_edits() {
        let root = TempTree::fresh("subscription-source");
        let receiver = TempTree::fresh("subscription-receiver");
        let n = Node::open(root.path()).unwrap();
        let r = Node::open(receiver.path()).unwrap();
        let mut account = imported(
            n.subscriptions_with_login(
                Action::Import {
                    provider: "claude".into(),
                },
                |_| true,
            )
            .unwrap(),
        );
        assert!(
            r.keep_imported_subscription(&account, r.device_str())
                .is_err()
        );
        r.keep_imported_subscription(&account, n.device_str())
            .unwrap();
        r.rename_account(&account.id, "My chosen name").unwrap();
        account.name = "Old peer name".into();
        r.keep_imported_subscription(&account, n.device_str())
            .unwrap();
        assert_eq!(r.accounts().unwrap()[0].name, "My chosen name");
        assert!(!receiver.join(".khor/keys").exists());
    }

    #[test]
    fn discovery_and_import_have_different_wire_retry_rules() {
        use crate::proto::{self, Request};
        let discover = Request::Subscriptions {
            action: Action::Discover,
        };
        let import = Request::Subscriptions {
            action: Action::Import {
                provider: "codex".into(),
            },
        };
        assert!(discover.is_reading());
        assert!(!import.is_reading());
        assert!(!discover.rides_early());
        assert!(!import.rides_early());
        let bytes = proto::encode(&import).unwrap();
        assert!(bytes.windows(13).any(|w| w == b"Subscriptions"));
        assert!(matches!(
            proto::decode::<Request>(&bytes).unwrap(),
            Request::Subscriptions {
                action: Action::Import { .. }
            }
        ));
    }
}
