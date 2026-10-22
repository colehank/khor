//! A per-machine request so a slow peer does not hold up the other results.
use khor_node::{
    Node,
    subscriptions::{Action, Answer},
};
use serde::Serialize;
use std::path::Path;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, TS)]
pub struct SubscriptionCandidate {
    pub provider: String,
    pub vendor: String,
    pub account: Option<String>,
}

pub async fn discover(root: &Path, machine: &str) -> Result<Vec<SubscriptionCandidate>, String> {
    match Node::open(root.to_path_buf())?
        .subscriptions_on(machine, Action::Discover)
        .await?
    {
        Answer::Candidates { candidates } => Ok(candidates
            .into_iter()
            .map(|c| SubscriptionCandidate {
                provider: c.provider,
                vendor: c.vendor,
                account: c.account,
            })
            .collect()),
        other => Err(khor_catalog::msg::peer_non_answer(format_args!(
            "{other:?}"
        ))),
    }
}

pub async fn import(root: &Path, machine: &str, provider: &str) -> Result<String, String> {
    match Node::open(root.to_path_buf())?
        .subscriptions_on(
            machine,
            Action::Import {
                provider: provider.into(),
            },
        )
        .await?
    {
        Answer::Imported { account } => Ok(account.id),
        other => Err(khor_catalog::msg::peer_non_answer(format_args!(
            "{other:?}"
        ))),
    }
}
