//! Which machine this is (SHELL.md 第五轮 第 1 条): a stable kind the
//! face picks an icon from, and the name the machine gives itself.
//!
//! **Read in-process, never by running a command**, the rule the GPU
//! reading keeps (`khor_core::Gpu`): macOS answers through IOKit and
//! `sysctl`, Linux through `/sys/class/dmi/id`. And **once per
//! process**: it is fixed hardware, and vitals are sampled every few
//! seconds.
//!
//! # Where each platform keeps it
//!
//! - **Apple Silicon Macs** carry the marketing name in the device tree
//!   (`IODeviceTree:/product`, `product-name` = `Mac mini (2024)`). Their
//!   `hw.model` (`Mac16,10`) names the board and says nothing about the
//!   shape, so the name is asked first.
//! - **Intel Macs** have no such node; their `hw.model` carries the shape
//!   in its prefix (`Macmini8,1`, `MacBookPro16,1`, `iMac20,1`).
//! - **Linux** has DMI: `product_name` for the name and `chassis_type`
//!   (SMBIOS 3.x, table 7.4.1) for the shape. Boards that ship with
//!   `To Be Filled By O.E.M.` in every field (hinton's) name themselves
//!   by `board_name` instead. **No DMI at all reads as a server** (SHELL.md
//!   says so, and the machines without it are VMs and ARM boards run
//!   headless).

use khor_core::{Model, ModelKind};

/// This machine's model. Asked once per process — the answer is fixed
/// hardware — and `None` when the platform would not say.
pub fn this_machine() -> Option<Model> {
    static ONCE: std::sync::OnceLock<Option<Model>> = std::sync::OnceLock::new();
    ONCE.get_or_init(read).clone()
}

#[cfg(target_os = "macos")]
fn read() -> Option<Model> {
    let name = mac::product_name().or_else(mac::hw_model)?;
    Some(Model {
        kind: mac_kind(&name),
        name: Some(name),
    })
}

#[cfg(target_os = "linux")]
fn read() -> Option<Model> {
    let dmi = |f: &str| {
        std::fs::read_to_string(format!("/sys/class/dmi/id/{f}"))
            .ok()
            .map(|s| s.trim().to_owned())
    };
    Some(linux_model(
        dmi("product_name").as_deref(),
        dmi("board_name").as_deref(),
        dmi("chassis_type").as_deref(),
    ))
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn read() -> Option<Model> {
    None
}

/// A Mac's kind from its product name (`Mac mini (2024)`) or, on an
/// Intel Mac with no such name, its `hw.model` (`Macmini8,1`).
///
/// Prefixes rather than a table of identifiers: Apple adds identifiers
/// every year and has kept these prefixes since the first Intel Mac.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) fn mac_kind(name: &str) -> ModelKind {
    let squashed: String = name.chars().filter(|c| !c.is_whitespace()).collect();
    let s = squashed.to_ascii_lowercase();
    if s.starts_with("macmini") {
        ModelKind::MacMini
    } else if s.starts_with("macbook") {
        ModelKind::Macbook
    } else if s.starts_with("imac") {
        ModelKind::Imac
    } else if s.starts_with("macstudio") {
        ModelKind::MacStudio
    } else if s.starts_with("macpro") {
        ModelKind::Desktop
    } else {
        ModelKind::Unknown
    }
}

/// What a vendor writes into a field it did not fill.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn placeholder(s: &str) -> bool {
    let l = s.trim().to_ascii_lowercase();
    l.is_empty()
        || l.contains("to be filled")
        || l == "default string"
        || l == "system product name"
        || l == "not applicable"
        || l == "none"
        || l == "o.e.m."
}

/// A Linux machine from its DMI fields.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(crate) fn linux_model(
    product: Option<&str>,
    board: Option<&str>,
    chassis: Option<&str>,
) -> Model {
    let name = [product, board]
        .into_iter()
        .flatten()
        .find(|s| !placeholder(s))
        .map(str::to_owned);
    // SMBIOS chassis types. Desktop-shaped: desktop, low-profile, pizza
    // box, mini tower, tower, all-in-one, space-saving, lunch box, mini
    // PC, stick PC. Laptop-shaped: portable, laptop, notebook, hand
    // held, sub notebook, tablet, convertible, detachable. Everything
    // else — server chassis, rack mount, blade, "other", "unknown", or
    // no DMI at all — is a server, the shape of every headless box khor
    // is installed on over ssh.
    let kind = match chassis.and_then(|c| c.trim().parse::<u32>().ok()) {
        Some(3..=7 | 13 | 15 | 16 | 24 | 35 | 36) => ModelKind::Desktop,
        Some(8..=10 | 11 | 14 | 30..=32) => ModelKind::Unknown,
        _ => ModelKind::Server,
    };
    Model { kind, name }
}

#[cfg(target_os = "macos")]
mod mac {
    use core_foundation::base::{CFType, TCFType, kCFAllocatorDefault};
    use core_foundation::data::CFData;
    use core_foundation::string::CFString;
    use io_kit_sys::{
        IOObjectRelease, IORegistryEntryCreateCFProperty, IORegistryEntryFromPath,
        kIOMasterPortDefault,
    };

    /// `IODeviceTree:/product`'s `product-name` — Apple Silicon only.
    pub(super) fn product_name() -> Option<String> {
        let path = std::ffi::CString::new("IODeviceTree:/product").ok()?;
        // SAFETY: the path is a live NUL-terminated string for the call;
        // the entry is released on every path out; the property comes
        // back under the create rule and is wrapped as such, and its
        // type is checked before it is read as data.
        unsafe {
            let entry = IORegistryEntryFromPath(kIOMasterPortDefault, path.as_ptr() as *mut _);
            if entry == 0 {
                return None;
            }
            let key = CFString::new("product-name");
            let raw = IORegistryEntryCreateCFProperty(
                entry,
                key.as_concrete_TypeRef(),
                kCFAllocatorDefault,
                0,
            );
            IOObjectRelease(entry);
            if raw.is_null() {
                return None;
            }
            let property = CFType::wrap_under_create_rule(raw);
            let data = property.downcast::<CFData>()?;
            let text = String::from_utf8_lossy(data.bytes());
            let text = text.trim_end_matches('\0').trim();
            (!text.is_empty()).then(|| text.to_owned())
        }
    }

    /// `sysctl hw.model`: `Macmini8,1` on Intel, `Mac16,10` on Apple
    /// Silicon.
    pub(super) fn hw_model() -> Option<String> {
        let name = std::ffi::CString::new("hw.model").ok()?;
        let mut buf = [0u8; 128];
        let mut len = buf.len();
        // SAFETY: `buf` and `len` describe a writable buffer that
        // outlives the call; sysctl writes at most `len` bytes.
        let rc = unsafe {
            libc::sysctlbyname(
                name.as_ptr(),
                buf.as_mut_ptr().cast(),
                &mut len,
                std::ptr::null_mut(),
                0,
            )
        };
        if rc != 0 {
            return None;
        }
        let text = String::from_utf8_lossy(&buf[..len.min(buf.len())]);
        let text = text.trim_end_matches('\0').trim();
        (!text.is_empty()).then(|| text.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mac_is_known_by_its_product_name_or_its_identifier() {
        for (name, kind) in [
            // Apple Silicon: the device tree's product-name.
            ("Mac mini (2024)", ModelKind::MacMini),
            ("MacBook Pro (14-inch, M3, 2023)", ModelKind::Macbook),
            ("MacBook Air (13-inch, M2, 2022)", ModelKind::Macbook),
            ("iMac (24-inch, 2023)", ModelKind::Imac),
            ("Mac Studio (2023)", ModelKind::MacStudio),
            // Intel: hw.model is all there is.
            ("Macmini8,1", ModelKind::MacMini),
            ("MacBookPro16,1", ModelKind::Macbook),
            ("iMac20,1", ModelKind::Imac),
            // A Mac Pro is a desktop; an Apple Silicon identifier with no
            // name to go with it says nothing about the shape.
            ("Mac Pro (2023)", ModelKind::Desktop),
            ("MacPro7,1", ModelKind::Desktop),
            ("Mac16,10", ModelKind::Unknown),
        ] {
            assert_eq!(mac_kind(name), kind, "{name}");
        }
    }

    /// The three Linux machines of this fleet, as their DMI read on
    /// 2026-09-22 (`/sys/class/dmi/id`).
    #[test]
    fn the_fleet_reads_as_what_its_dmi_says() {
        // turing: chassis 17, "Main Server Chassis".
        assert_eq!(
            linux_model(Some("MZ72-HB0-00"), Some("MZ72-HB0-00"), Some("17")),
            Model {
                kind: ModelKind::Server,
                name: Some("MZ72-HB0-00".into())
            }
        );
        // lecun: a Supermicro GPU server whose chassis says "Other" (1).
        assert_eq!(
            linux_model(Some("SYS-4028GR-TR"), Some("X10DRG-O+-CPU"), Some("1")),
            Model {
                kind: ModelKind::Server,
                name: Some("SYS-4028GR-TR".into())
            }
        );
        // hinton: a workstation board in a desktop case (3), and a
        // product name nobody filled in — the board is the name.
        assert_eq!(
            linux_model(
                Some("To Be Filled By O.E.M."),
                Some("EP2C621D12 WS"),
                Some("3")
            ),
            Model {
                kind: ModelKind::Desktop,
                name: Some("EP2C621D12 WS".into())
            }
        );
    }

    #[test]
    fn no_dmi_reads_as_a_server_and_a_laptop_as_unknown() {
        assert_eq!(
            linux_model(None, None, None),
            Model {
                kind: ModelKind::Server,
                name: None
            }
        );
        assert_eq!(
            linux_model(Some("ThinkPad X1"), None, Some("10")).kind,
            ModelKind::Unknown
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn this_mac_says_what_it_is() {
        let m = this_machine().expect("a Mac always has a model");
        assert!(m.name.is_some(), "{m:?}");
        // Every Mac shape has a kind; only an Apple Silicon identifier
        // read without its product name would be Unknown, and that is
        // the fallback this reading must not have needed here.
        assert_ne!(m.kind, ModelKind::Unknown, "{m:?}");
    }
}
