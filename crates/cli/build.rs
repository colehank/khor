fn main() {
    println!("cargo::rustc-check-cfg=cfg(khor_embedded_server)");
    println!("cargo::rerun-if-env-changed=KHOR_SERVER_BINARY");
    if let Some(path) = std::env::var_os("KHOR_SERVER_BINARY") {
        let path = std::fs::canonicalize(path).expect("KHOR_SERVER_BINARY must name khor-server");
        println!("cargo::rerun-if-changed={}", path.display());
        println!("cargo::rustc-env=KHOR_SERVER_BINARY={}", path.display());
        println!("cargo::rustc-cfg=khor_embedded_server");
    }
}
