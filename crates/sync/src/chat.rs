//! Chat: one CRDT document per channel, persisted through the generic
//! block store. A channel is a machine's window — one machine, one
//! channel, every device writes into the same conversation.

pub mod doc;

use std::path::{Path, PathBuf};

pub use doc::{ChatDoc, FileRef, Message, MsgBody, Sender};

use crate::store::{load, Loaded};

/// Location relative to the home directory. Fixed rather than
/// `config_dir()`: the writer is often on another machine (sftp) and
/// cannot know the remote OS's config dir. Two path schemes would
/// eventually disagree, and the symptom would be two chat histories on
/// one machine, each blind to the other.
pub const REL_DIR: &str = ".khor/chat";

/// Opens one channel's document and store.
pub fn open_channel(dir: &Path, peer: u64) -> Result<Loaded<ChatDoc>, String> {
    load(dir, peer)
}

/// Channel names travel through a remote shell (ssh/sftp path building),
/// so this whitelist blocks command injection, not just bad filenames.
/// `.` passes because channel names are usually machine names.
pub fn valid_channel(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name != "."
        && name != ".."
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
}

/// A channel's directory; `None` for invalid names. Never "clean" a name
/// here: two devices cleaning differently would split one channel into
/// two directories. Cleaning has exactly one implementation,
/// [`channel_of_machine`].
pub fn channel_dir(home: &Path, channel: &str) -> Option<PathBuf> {
    valid_channel(channel).then(|| home.join(REL_DIR).join(channel))
}

/// The channel name of a machine. One machine, one channel: every device
/// writes into the same window, so a third machine joins the same
/// conversation instead of spawning pairwise ones.
///
/// `name` must be the machine's **self-reported** hostname — not a
/// user-editable display name, not an ssh alias. Two devices naming one
/// machine differently means two directories that never converge, with
/// no error anywhere.
///
/// Cleaning: clean names pass through unchanged; a changed name gets a
/// fingerprint of the **original** appended. So a cleaned name can never
/// collide with a real one (`a b` → `a-b-<fp>`, distinct from a machine
/// truly named `a-b`), and two originals that clean to the same stem stay
/// apart. Pure function — two machines must compute the same result.
/// `None` when nothing usable remains: report that, don't invent a name
/// someone else could also get.
pub fn channel_of_machine(name: &str) -> Option<String> {
    let name = name.trim().trim_end_matches(".local");
    if name.is_empty() {
        return None;
    }
    if valid_channel(name) {
        return Some(name.to_string());
    }
    // Replace per char, then collapse runs of `-`: a mostly-CJK name
    // should not become a row of dashes.
    let mut cleaned = String::new();
    for c in name.chars() {
        let ok = c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.';
        if ok {
            cleaned.push(c);
        } else if !cleaned.ends_with('-') {
            cleaned.push('-');
        }
    }
    let cleaned = cleaned.trim_matches(['-', '.']).to_string();
    let stem: String = cleaned.chars().take(40).collect();
    let stem = stem.trim_end_matches(['-', '.']);
    // The fingerprint hashes the original, not the cleaned form: that is
    // what keeps two different originals with identical stems apart.
    let out = format!("{}-{}", if stem.is_empty() { "m" } else { stem }, fingerprint(name));
    valid_channel(&out).then_some(out)
}

/// FNV-1a. This needs determinism across machines, not collision
/// resistance — no hash crate.
fn fingerprint(s: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{:08x}", (h ^ (h >> 32)) as u32)
}

#[cfg(test)]
mod integration;

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    /// The channel-name whitelist blocks command injection, not just bad
    /// filenames — the name travels through a remote shell.
    #[test]
    fn channel_names_are_whitelisted() {
        for ok in ["turing", "mac-mini.local", "a_b-1.2"] {
            assert!(valid_channel(ok), "{ok} 该放行");
        }
        for bad in ["", ".", "..", "a/b", "a b", "a;rm -rf /", "a$(x)", "a\nb"] {
            assert!(!valid_channel(bad), "{bad:?} 该挡住");
        }
        // Invalid names return None — never "cleaned" here, because two
        // devices cleaning differently would split the channel.
        assert!(channel_dir(&PathBuf::from("/home/x"), "a/b").is_none());
        assert_eq!(
            channel_dir(&PathBuf::from("/home/x"), "turing").unwrap(),
            PathBuf::from("/home/x/.khor/chat/turing")
        );
    }

    /// Clean machine names pass through unchanged — and match what the
    /// ssh side sees. Pure function: two machines must compute the same
    /// name.
    #[test]
    fn a_clean_machine_name_is_the_channel_name_unchanged() {
        for n in ["turing", "mac-mini", "gpu.01", "aliyun_2", "A1"] {
            assert_eq!(channel_of_machine(n).as_deref(), Some(n), "{n} 该原样通过");
        }
        // `.local` stripped: one machine self-reports both spellings
        // depending on the entry point.
        assert_eq!(channel_of_machine("zgh-mac.local").as_deref(), Some("zgh-mac"));
        assert_eq!(channel_of_machine("  turing  ").as_deref(), Some("turing"));
    }

    /// A cleaned name never collides with a clean one — the whole reason
    /// the fingerprint exists. Control: the machine truly named `a-b`
    /// passes unchanged, or an all-names-get-suffixed implementation is
    /// green too.
    #[test]
    fn a_cleaned_name_never_collides_with_a_clean_one() {
        let dirty = channel_of_machine("a b").expect("该算得出来");
        let clean = channel_of_machine("a-b").expect("该算得出来");
        assert_eq!(clean, "a-b", "干净的必须原样(对照组:不是所有名字都加后缀)");
        assert_ne!(dirty, clean, "清洗过的不许撞上真名,实测 {dirty} vs {clean}");
        assert!(dirty.starts_with("a-b-"), "清洗过的该带指纹,实测 {dirty}");

        // Two different originals with the same cleaned stem stay apart —
        // the fingerprint hashes the original.
        let x = channel_of_machine("a b").unwrap();
        let y = channel_of_machine("a/b").unwrap();
        assert_ne!(x, y, "原名不同就不许同名,实测 {x} vs {y}");
    }

    /// Same original, same answer, every time.
    #[test]
    fn the_channel_name_is_a_pure_function_of_the_machine_name() {
        for n in ["张三的电脑", "turing", "a b", "服务器-01"] {
            let a = channel_of_machine(n);
            let b = channel_of_machine(n);
            assert_eq!(a, b, "{n} 算两次该一样");
            if let Some(c) = a {
                assert!(valid_channel(&c), "{n} → {c} 必须是合法频道名");
            }
        }
    }

    /// Non-ASCII machine names still get a channel; a name with nothing
    /// usable gets `None`, not an invented name.
    #[test]
    fn a_non_ascii_machine_name_still_gets_a_channel() {
        let c = channel_of_machine("张三的电脑").expect("该算得出一个名字");
        assert!(valid_channel(&c), "{c}");
        assert!(!c.is_empty());
        // Mixed names keep their readable part — people `ls` these dirs.
        let m = channel_of_machine("张三的 MacBook").expect("该算得出来");
        assert!(m.contains("MacBook"), "可读的那一截该留着,实测 {m}");

        assert_eq!(channel_of_machine(""), None);
        assert_eq!(channel_of_machine("   "), None);
        // Not None: zero legal chars still yields placeholder +
        // fingerprint, which is deterministic.
        let all_bad = channel_of_machine("。、·").expect("该有个兜底名");
        assert!(valid_channel(&all_bad), "{all_bad}");
    }

    /// Long names are bounded, and two sharing a long prefix don't
    /// collide. The truncation is safe only because the fingerprint
    /// hashes the full original — this measures exactly that.
    #[test]
    fn a_very_long_machine_name_is_bounded_without_colliding() {
        for long in ["机器".repeat(200), "a".repeat(200)] {
            let c = channel_of_machine(&long).expect("超长也该算得出一个名字");
            assert!(valid_channel(&c), "长度 {} : {c}", c.len());
        }
        // Identical first 128 chars, one differing tail char: two
        // channels.
        let a = channel_of_machine(&format!("{}X", "a".repeat(150))).unwrap();
        let b = channel_of_machine(&format!("{}Y", "a".repeat(150))).unwrap();
        assert_ne!(a, b, "只差最后一个字的两台机器不许并成一场:{a} vs {b}");
    }
}
