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
        let bytes = std::fs::read(&path).expect("read the compiled Pi sidecar");
        println!(
            "cargo::rustc-env=KHOR_AGENT_BUNDLE_BLAKE3={}",
            blake3::hash(&bytes).to_hex()
        );
        println!("cargo::rustc-env=KHOR_AGENT_BUNDLE={}", path.display());
        println!("cargo::rustc-cfg=khor_embedded_agent");
    } else {
        // **A build without the runtime is a build whose khor conversation
        // cannot answer**, and that failure is silent until somebody tries
        // one — which is how "the local agent was removed" came to be
        // reported (2026-11-18) when in fact no build this round had
        // embedded it. The warning is the cheap half; the gate is
        // `scripts/windows-agent-check.ps1`, which reads the built binary's
        // `_runtime_info` and refuses it when `portable` is false.
        //
        // Not a hard error here on purpose: `cargo test`, `cargo check` and
        // a CLI-only build all compile this crate, and none of them is a
        // shippable desktop app.
        println!(
            "cargo::warning=the Khor agent runtime is NOT embedded in this build; \
             set KHOR_AGENT_BUNDLE to apps/agent/dist/khor-agent and check with \
             scripts/windows-agent-check.ps1 (see docs/windows-migration/W9-NEXT-STAGE-PLAN.md T0-2)"
        );
    }
}
