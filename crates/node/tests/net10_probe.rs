//! NET-10 probe: read a copied `.khor` as a foreign store and print what
//! its device table and roster say. Run by hand:
//! `NET10_STORE=<dir holding devices/ and root.pub> cargo test -p khor-node --test net10_probe -- --ignored --nocapture`

use khor_sync::devices::DeviceDoc;
use khor_sync::store;

#[test]
#[ignore]
fn print_a_copied_store() {
    let dir = std::path::PathBuf::from(std::env::var("NET10_STORE").expect("NET10_STORE"));
    let loaded: store::Loaded<DeviceDoc> = store::load(&dir.join("devices"), 7).unwrap();
    println!("broken: {:?}", loaded.broken);
    let doc = &loaded.doc;
    let short = |s: &str| s.chars().take(8).collect::<String>();
    for d in doc.all() {
        println!(
            "row {} name={} label={:?} hosts={} cert={}",
            short(&d.id),
            d.name,
            d.label,
            d.hosts,
            doc.cert_of(&d.id).is_some()
        );
    }
    for id in doc.gone_ids() {
        println!("gone {} {:?}", short(&id), doc.name_of(&id));
    }
    let text = std::fs::read_to_string(dir.join("root.pub")).unwrap();
    let anchor =
        iroh::PublicKey::from_bytes(&khor_net::trust::id_from_hex(text.trim()).unwrap()).unwrap();
    for (k, b) in doc.rosters() {
        match khor_net::trust::verify_roster(&b, &anchor) {
            Ok(r) => println!(
                "roster {k}: seq={} issued={} certs={:?} revoked={:?}",
                r.seq,
                r.issued_ms,
                r.certs
                    .iter()
                    .map(|c| short(&khor_net::trust::id_hex(&c.subject)))
                    .collect::<Vec<_>>(),
                r.revoked
                    .iter()
                    .map(|c| short(&khor_net::trust::id_hex(c)))
                    .collect::<Vec<_>>()
            ),
            Err(e) => println!("roster {k}: refused {e:?}"),
        }
    }
    println!("version: {:?}", khor_sync::store::Doc::version(doc));
}
