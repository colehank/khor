//! Recent dismissal addresses the ledger identity, even if the machine is gone.
use khor_node::{Node, PlaceKind};
use serde_json::json;
use std::{fs, path::PathBuf};

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "khor-recents-identity-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn dismissal_dispatch_accepts_device_identity_and_keeps_machine_compatibility() {
    let scratch = Scratch::new();
    let node = Node::open_as(scratch.0.clone(), "recents-test").unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let url = "https://example.test/device";
    node.recent_visit(PlaceKind::Page, node.name(), url)
        .unwrap();
    let answer = khor_gui_core::api::answer(
        &rt,
        &scratch.0,
        "forget_recent",
        &json!({
            "kind": "page", "device": node.device_str(), "place": url,
        }),
    );
    assert_eq!(
        answer,
        Ok("null".into()),
        "a list row carries its stable device id"
    );
    assert!(node.recents(PlaceKind::Page, 10).unwrap().is_empty());

    node.recent_visit(PlaceKind::Page, node.name(), url)
        .unwrap();
    assert_eq!(
        node.recents(PlaceKind::Page, 10).unwrap().len(),
        1,
        "a visit restores a dismissed row"
    );
    assert_eq!(
        khor_gui_core::api::answer(
            &rt,
            &scratch.0,
            "forget_recent",
            &json!({
                "kind": "page", "machine": node.name(), "place": url,
            })
        ),
        Ok("null".into()),
        "older callers can still address a machine by name"
    );
    assert!(node.recents(PlaceKind::Page, 10).unwrap().is_empty());

    for device in [
        "",
        "deadbeef",
        "g000000000000000000000000000000000000000000000000000000000000000",
    ] {
        assert!(
            khor_gui_core::api::answer(
                &rt,
                &scratch.0,
                "forget_recent",
                &json!({
                    "kind": "page", "device": device, "place": url,
                })
            )
            .is_err(),
            "a display name or shortened id is not a ledger identity"
        );
    }
    assert!(
        khor_gui_core::api::answer(
            &rt,
            &scratch.0,
            "forget_recent",
            &json!({
                "kind": "page", "device": 7, "place": url,
            })
        )
        .is_err(),
        "a malformed explicit identity must not fall back to another machine"
    );
}
