//! One wire-name registry shared by inventory, transfer and ordinary loads.

pub(crate) const ROUND_DOCS: [&str; 11] = [
    "devices",
    "seen",
    "pins",
    "roster",
    "dirpins",
    "workspaces",
    "webpins",
    "recents",
    "agents",
    "accounts",
    "titles",
];

pub(crate) fn authorized_name(name: &str) -> bool {
    ROUND_DOCS.contains(&name) || khor_sync::chat::wide_name(name).is_some()
}

macro_rules! with_doc {
    ($context:expr, $name:expr, |$loaded:ident| $body:expr) => {{
        let name: &str = $name;
        let root = $context.root();
        let peer = $context.writer_peer();
        match name {
            "devices" => {
                let mut l = khor_sync::store::load_pile::<khor_sync::devices::DeviceDoc>(
                    &khor_sync::devices::devices_dir(root),
                    peer,
                )?;
                let $loaded = &mut l;
                $body
            }
            "seen" => {
                let mut l = khor_sync::store::load_pile::<khor_sync::seen::SeenDoc>(
                    &khor_sync::seen::seen_dir(root),
                    peer,
                )?;
                let $loaded = &mut l;
                $body
            }
            "pins" => {
                let mut l = khor_sync::store::load_pile::<khor_sync::pins::PinDoc>(
                    &khor_sync::pins::pins_dir(root),
                    peer,
                )?;
                let $loaded = &mut l;
                $body
            }
            "roster" => {
                let mut l = khor_sync::store::load_pile::<khor_sync::roster::RosterDoc>(
                    &khor_sync::roster::roster_dir(root),
                    peer,
                )?;
                let $loaded = &mut l;
                $body
            }
            "dirpins" => {
                let mut l = khor_sync::store::load_pile::<khor_sync::dirpins::DirPinDoc>(
                    &khor_sync::dirpins::dirpins_dir(root),
                    peer,
                )?;
                let $loaded = &mut l;
                $body
            }
            "workspaces" => {
                let mut l = khor_sync::store::load_pile::<khor_sync::workspaces::WorkspaceDoc>(
                    &khor_sync::workspaces::workspaces_dir(root),
                    peer,
                )?;
                let $loaded = &mut l;
                $body
            }
            "webpins" => {
                let mut l = khor_sync::store::load_pile::<khor_sync::webpins::WebPinDoc>(
                    &khor_sync::webpins::webpins_dir(root),
                    peer,
                )?;
                let $loaded = &mut l;
                $body
            }
            "recents" => {
                let mut l = khor_sync::store::load_pile::<khor_sync::recents::RecentDoc>(
                    &khor_sync::recents::recents_dir(root),
                    peer,
                )?;
                let $loaded = &mut l;
                $body
            }
            "agents" => {
                let mut l = khor_sync::store::load_pile::<khor_sync::agents::AgentDoc>(
                    &khor_sync::agents::agents_dir(root),
                    peer,
                )?;
                let $loaded = &mut l;
                $body
            }
            "accounts" => {
                let mut l = khor_sync::store::load_pile::<khor_sync::accounts::AccountDoc>(
                    &khor_sync::accounts::accounts_dir(root),
                    peer,
                )?;
                let $loaded = &mut l;
                $body
            }
            "titles" => {
                let mut l = khor_sync::store::load_pile::<khor_sync::titles::TitleDoc>(
                    &khor_sync::titles::titles_dir(root),
                    peer,
                )?;
                let $loaded = &mut l;
                $body
            }
            _ => match khor_sync::chat::wide_dir(root, name) {
                Some(dir) => {
                    let mut l = khor_sync::chat::open_channel(&dir, peer)?;
                    let $loaded = &mut l;
                    $body
                }
                None => Err(khor_catalog::msg::unknown_doc(name).into()),
            },
        }
    }};
}
pub(crate) use with_doc;
