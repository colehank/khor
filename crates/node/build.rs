fn main() {
    println!("cargo::rustc-check-cfg=cfg(khor_embedded_agent)");
    println!("cargo::rerun-if-env-changed=KHOR_AGENT_BUNDLE");
    let target = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if matches!(target.as_str(), "ios" | "android") {
        return;
    }
    if let Some(path) = std::env::var_os("KHOR_AGENT_BUNDLE") {
        let path = std::fs::canonicalize(path)
            .expect("KHOR_AGENT_BUNDLE must name the compiled Pi sidecar");
        println!("cargo::rerun-if-changed={}", path.display());
        println!("cargo::rustc-env=KHOR_AGENT_BUNDLE={}", path.display());
        println!("cargo::rustc-cfg=khor_embedded_agent");
    }
}
