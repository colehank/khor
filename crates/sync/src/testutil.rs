//! Shared scaffolding for the chat tests.

use std::path::PathBuf;

use crate::chat::{ChatDoc, Sender};

pub(crate) fn me(n: &str) -> Sender {
    Sender {
        id: format!("dev-{n}"),
        name: n.into(),
    }
}

/// A comparable flat rendering; convergence is judged on it.
pub(crate) fn render(d: &ChatDoc) -> String {
    d.messages()
        .iter()
        .map(|m| {
            format!(
                "{}|{}|{:?}|{}|{}",
                m.id, m.from.id, m.body, m.retracted, m.edited
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn tmpdir(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "khor-sync-test-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&p);
    p
}
