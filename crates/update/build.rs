fn main() {
    for name in [
        "KHOR_BUILD_SOURCE",
        "KHOR_RELEASE_TAG",
        "KHOR_RELEASE_BUILD",
    ] {
        println!("cargo::rerun-if-env-changed={name}");
        let value = std::env::var(name).unwrap_or_default();
        assert!(!value.contains(['\n', '\r']), "invalid build identity");
        println!("cargo::rustc-env={name}={value}");
    }
}
