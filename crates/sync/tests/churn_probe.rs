//! Which writer changed what in a copied document: every change, by
//! peer, as loro's JSON updates. Run by hand:
//! `CHURN_DIR=<a document's directory> cargo test -p khor-sync --test churn_probe -- --ignored --nocapture`

#[test]
#[ignore]
fn print_who_changed_what() {
    let dir = std::path::PathBuf::from(std::env::var("CHURN_DIR").expect("CHURN_DIR"));
    let doc = loro::LoroDoc::new();
    let mut names: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .collect();
    names.sort();
    for p in names
        .iter()
        .filter(|p| p.extension().is_some_and(|x| x == "loro"))
    {
        let _ = doc.import(&std::fs::read(p).unwrap());
    }
    let json = doc.export_json_updates(&Default::default(), &doc.oplog_vv());
    let v = serde_json::to_value(&json).unwrap();
    for ch in v["changes"].as_array().unwrap() {
        let ops: Vec<String> = ch["ops"]
            .as_array()
            .unwrap()
            .iter()
            .map(|o| {
                let c = o["content"].to_string();
                format!(
                    "{}:{}",
                    o["container"]
                        .as_str()
                        .unwrap_or("?")
                        .chars()
                        .take(24)
                        .collect::<String>(),
                    c.chars().take(90).collect::<String>()
                )
            })
            .collect();
        println!(
            "{} t={} n={} {}",
            ch["id"],
            ch["timestamp"],
            ops.len(),
            ops.join(" | ")
        );
    }
}
