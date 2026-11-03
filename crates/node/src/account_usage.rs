//! Project Pi's normalized balance reports; credentials and vendor HTTP stay
//! with the management process. iOS keeps its native readers without a sidecar.
use super::{Account, AccountReading, Cached, Node, PiObservation, bare, decimal, trouble};
use crate::khor_agent::{account_provider_id, runtime};
use serde_json::Value;
use std::sync::OnceLock;

pub(super) fn supports(account: &Account) -> bool {
    !account.kind.is_sub()
        && matches!(
            account_provider_id(account).as_str(),
            "deepseek" | "moonshotai" | "moonshotai-cn"
        )
}

fn process_slots() -> &'static tokio::sync::Semaphore {
    static SLOTS: OnceLock<tokio::sync::Semaphore> = OnceLock::new();
    SLOTS.get_or_init(|| tokio::sync::Semaphore::new(2))
}

impl Node {
    pub(super) async fn read_pi_balance(&self, account: &Account, now: i64) -> AccountReading {
        let Ok(_call) = runtime::enter_observer(self.root()) else {
            return bare(account, Some(trouble::UNREACHABLE), now);
        };
        // Bound SDK memory across simultaneous board requests. Native HTTP
        // readers retain their existing concurrency; these processes use two slots.
        let _slot = process_slots()
            .acquire()
            .await
            .expect("private usage slots stay open");
        let Some(fingerprint) = self.usage_credential_fingerprint(account) else {
            return bare(account, Some(trouble::NO_KEY), now);
        };
        let cached = self.cached_reading(&account.id).filter(|cached| {
            cached
                .pi
                .as_ref()
                .is_some_and(|pi| pi.credential_fingerprint == fingerprint)
        });
        if let Some(cached) = &cached
            && cached.reading.trouble.is_none()
            && cached
                .pi
                .as_ref()
                .is_some_and(|pi| pi.expires_at_ms > crate::live::now_ms())
        {
            return cached.reading.clone();
        }
        let result = self.read_pi_usage(account).await;
        if self.usage_credential_fingerprint(account).as_deref() != Some(fingerprint.as_str()) {
            return bare(account, Some(trouble::NO_READING), now);
        }
        let result = result.map_err(|_| trouble::UNREACHABLE).and_then(|value| {
            if value["status"] != "ready" {
                return Err(match value["reason"].as_str() {
                    Some("auth-unavailable") => trouble::NO_KEY,
                    Some("unsupported") => trouble::UNKNOWN_PROVIDER,
                    Some("protocol-error") => trouble::UNREADABLE,
                    Some("timeout") => trouble::UNREACHABLE,
                    _ => trouble::QUERY_FAILED,
                });
            }
            let expires_at_ms = value["expiresAt"].as_i64().ok_or(trouble::UNREADABLE)?;
            let report = value["report"].clone();
            let reading = project(account, &report)?;
            Ok((
                reading,
                PiObservation {
                    credential_fingerprint: fingerprint,
                    expires_at_ms,
                    report,
                },
            ))
        });
        match result {
            Ok((reading, observation)) => {
                self.cache_reading(
                    &account.id,
                    &Cached {
                        reading: reading.clone(),
                        failed_at_ms: None,
                        pi: Some(observation),
                    },
                );
                reading
            }
            Err(why) => cached
                .filter(|cached| cached.reading.trouble.is_none())
                .map(|cached| cached.reading)
                .unwrap_or_else(|| bare(account, Some(why), now)),
        }
    }
}

fn project(account: &Account, report: &Value) -> Result<AccountReading, &'static str> {
    let provider = account_provider_id(account);
    if report["providerId"] != provider {
        return Err(trouble::UNREADABLE);
    }
    let captured_at = report["capturedAt"].as_i64().ok_or(trouble::UNREADABLE)?;
    let metrics = report["metrics"].as_array().ok_or(trouble::UNREADABLE)?;
    let metric = |id: &str, currency: &str| {
        metrics
            .iter()
            .find(|m| m["id"] == id && m["currency"] == currency)
            .and_then(|m| decimal(m.get("value")))
    };
    let (remaining, currency) = match provider.as_str() {
        "deepseek" => match (metric("usd-total", "USD"), metric("cny-total", "CNY")) {
            (Some(usd), _) if usd > 0.0 => (usd, "USD"),
            (_, Some(cny)) if cny > 0.0 => (cny, "CNY"),
            (Some(usd), _) => (usd, "USD"),
            (_, Some(cny)) => (cny, "CNY"),
            _ => return Err(trouble::UNREADABLE),
        },
        "moonshotai" | "moonshotai-cn" => {
            let currency = if provider == "moonshotai" {
                "USD"
            } else {
                "CNY"
            };
            (
                metric("available-balance", currency).ok_or(trouble::UNREADABLE)?,
                currency,
            )
        }
        _ => return Err(trouble::UNKNOWN_PROVIDER),
    };
    let mut reading = bare(account, None, captured_at);
    reading.remaining = Some(remaining);
    reading.currency = Some(currency.into());
    Ok(reading)
}

#[cfg(test)]
mod tests {
    use super::*;
    use khor_sync::accounts::{AccountKind, Inference, Provider};
    use serde_json::json;

    fn account() -> Account {
        Account {
            id: "usage-fixture".into(),
            name: "balance".into(),
            provider: Provider::Deepseek,
            home: "fixture".into(),
            kind: AccountKind::Bill,
            base_url: None,
            budget: None,
            pinned: false,
            unit: None,
            inference: Some(Inference {
                provider: "deepseek".into(),
                base_url: None,
                default_model: String::new(),
                auth_type: "api_key".into(),
            }),
        }
    }
    fn report(usd: Option<&str>, cny: Option<&str>) -> Value {
        let mut metrics = Vec::new();
        for (currency, amount) in [("USD", usd), ("CNY", cny)] {
            if let Some(amount) = amount {
                metrics.push(json!({"id":format!("{}-total", currency.to_lowercase()),"currency":currency,"value":amount}));
            }
        }
        json!({"providerId":"deepseek","capturedAt":1234,"metrics":metrics})
    }

    #[test]
    fn balance_projection_keeps_zero_currency_and_capture_time_without_inventing_spend() {
        let account = account();
        let zero = project(&account, &report(Some("0.00"), None)).unwrap();
        assert_eq!(zero.remaining, Some(0.0));
        assert_eq!(zero.currency.as_deref(), Some("USD"));
        assert_eq!(zero.at_ms, 1234);
        assert_eq!(zero.spent, None);
        assert_eq!(zero.fill, None);
        let cny = project(&account, &report(Some("0"), Some("17"))).unwrap();
        assert_eq!(cny.remaining, Some(17.0));
        assert_eq!(cny.currency.as_deref(), Some("CNY"));
        assert!(project(&account, &report(None, None)).is_err());
        let mut other = report(Some("42"), None);
        other["providerId"] = json!("moonshotai");
        assert!(project(&account, &other).is_err());
    }

    #[tokio::test]
    async fn model_credentials_reach_the_formal_board_without_a_second_usage_key() {
        let root = std::env::temp_dir().join(format!(
            "khor-pi-observation-{}",
            crate::link::fresh_hex().unwrap()
        ));
        let node = Node::open_as(root.clone(), "usage-test").unwrap();
        let mut account = account();
        account.home = node.device_str().into();
        node.commit_account(account.clone()).unwrap();
        let auth = root
            .join(".khor/agent/credentials")
            .join(&account.id)
            .join("auth.json");
        std::fs::create_dir_all(auth.parent().unwrap()).unwrap();
        std::fs::write(&auth, br#"{"deepseek":{"type":"api_key","key":"fixture"}}"#).unwrap();
        let now = crate::live::now_ms();
        let normalized = report(Some("0"), Some("17"));
        let reading = project(&account, &normalized).unwrap();
        node.cache_reading(
            &account.id,
            &Cached {
                reading: reading.clone(),
                failed_at_ms: None,
                pi: Some(PiObservation {
                    credential_fingerprint: node.usage_credential_fingerprint(&account).unwrap(),
                    expires_at_ms: now + 60_000,
                    report: normalized,
                }),
            },
        );
        assert!(!root.join(".khor/keys").join(&account.id).exists());
        assert_eq!(node.quota_readings().await.unwrap(), vec![reading.clone()]);
        assert_eq!(node.quota_readings_held().await.unwrap(), vec![reading]);
        std::fs::write(&auth, br#"{"deepseek":{"type":"api_key","key":"rotated"}}"#).unwrap();
        assert!(
            node.quota_readings_held().await.unwrap().is_empty(),
            "a changed credential cannot reuse the old observation"
        );
        assert_eq!(
            Node::shutdown_agent_runtime(&root).await.unwrap(),
            crate::khor_agent::RuntimeClose::Closed
        );
        assert_eq!(
            node.quota_readings().await.unwrap()[0].trouble.as_deref(),
            Some(trouble::UNREACHABLE)
        );
        drop(node);
        std::fs::remove_dir_all(root).unwrap();
    }
}
