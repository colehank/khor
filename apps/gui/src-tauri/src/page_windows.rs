//! A borrowed WebView2 page has its own environment and proxy arguments.

use std::net::{Ipv4Addr, SocketAddrV4};
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Default)]
pub struct Profiles(Mutex<Option<tempfile::TempDir>>);

impl Profiles {
    pub fn directory(&self, machine: &str, proxy: SocketAddrV4) -> Result<PathBuf, String> {
        let mut directory = self.0.lock().map_err(|error| error.to_string())?;
        if directory.is_none() {
            let mut builder = tempfile::Builder::new();
            builder.prefix("khor-borrow-");
            let created = match profile_directory() {
                Some(base) => {
                    std::fs::create_dir_all(&base).map_err(|error| error.to_string())?;
                    builder.tempdir_in(base)
                }
                None => builder.tempdir(),
            };
            *directory = Some(created.map_err(|error| error.to_string())?);
        }
        let key = khor_gui_core::web::page_store_identifier(machine);
        let key = u128::from_le_bytes(key);
        Ok(directory
            .as_ref()
            .expect("borrow profile directory initialized")
            .path()
            .join(format!("{key:032x}-{}", proxy.port())))
    }
}

pub fn profile_directory() -> Option<PathBuf> {
    std::env::var_os("KHOR_HOME").map(|home| PathBuf::from(home).join("webview"))
}

pub fn proxy_address(address: &str) -> Result<SocketAddrV4, String> {
    let proxy: SocketAddrV4 = address
        .parse()
        .map_err(|_| "invalid borrowing proxy address".to_owned())?;
    if *proxy.ip() != Ipv4Addr::LOCALHOST || proxy.port() == 0 {
        return Err("the borrowing proxy must have a local listening port".into());
    }
    Ok(proxy)
}

pub fn browser_args(proxy: SocketAddrV4) -> String {
    // Wry replaces its defaults when we set additional arguments. Retain
    // those defaults and include localhost in requests sent to this exit.
    format!(
        "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection \
         --proxy-server=http://{proxy} --proxy-bypass-list=<-loopback>"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn borrowing_uses_only_the_owned_loopback_port() {
        let proxy = proxy_address("127.0.0.1:7420").unwrap();
        let args = browser_args(proxy);
        assert!(args.contains("--proxy-server=http://127.0.0.1:7420"));
        assert!(args.contains("--proxy-bypass-list=<-loopback>"));
        for invalid in ["127.0.0.1:0", "192.0.2.1:7420", "127.0.0.1:7420 --other"] {
            assert!(proxy_address(invalid).is_err());
        }
    }

    #[test]
    fn profiles_are_shared_only_by_tabs_on_the_same_lease() {
        let profiles = Profiles::default();
        let proxy = proxy_address("127.0.0.1:7420").unwrap();
        let same = profiles.directory("a", proxy).unwrap();
        assert_eq!(same, profiles.directory("a", proxy).unwrap());
        assert_ne!(same, profiles.directory("b", proxy).unwrap());
        assert_ne!(
            same,
            profiles
                .directory("a", proxy_address("127.0.0.1:7421").unwrap())
                .unwrap()
        );
    }
}
