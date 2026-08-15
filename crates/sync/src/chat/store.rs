//! Persistence: a stack of append-only block files anyone can read or write.
//!
//! ```text
//! ~/.khor/chat/<channel>/
//!     u-<author:016x>-<seq:08x>.loro   ← increment block, immutable once written
//!     snap-<seq:08x>.loro              ← compaction product (see `compact`)
//! ```
//!
//! One file per batch, author in the name: two devices writing the same
//! directory never conflict at the filesystem level (the Maildir answer).
//! Loro increments merge in any order, so "immutable files + merge them
//! all" is the entire consistency story.
//!
//! The path is fixed rather than `config_dir()`: the writer is often on
//! another machine (sftp) and cannot know the remote OS's config dir.
//! Two path schemes would eventually disagree, and the symptom would be
//! two chat histories on one machine, each blind to the other.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use loro::VersionVector;

use super::doc::ChatDoc;
use super::plan::{Ledger, Side};

/// Location relative to the home directory. Fixed — see module head.
pub const REL_DIR: &str = ".khor/chat";

/// Block file extension.
const EXT: &str = "loro";

/// The merged-blocks ledger. No `.loro` extension, so it is neither read
/// as a block nor synced to the far side — each machine keeps its own.
const LEDGER: &str = ".merged";

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

/// One channel's stack on disk.
pub struct ChatStore {
    dir: PathBuf,
    /// Versions already on disk. `flush` writes only what came after —
    /// without this every flush rewrites all history, and "append-only"
    /// silently turns quadratic.
    on_disk: VersionVector,
    /// Blocks ever merged. Compaction deletes files, never ledger lines —
    /// see the `plan` module head.
    ledger: Ledger,
}

/// What `load` returns.
pub struct Loaded {
    pub store: ChatStore,
    pub doc: ChatDoc,
    /// Files that would not read. Counted, not swallowed: a bad block is
    /// a missing slice of conversation and the UI must be able to say so;
    /// one bad block must not kill the channel either.
    pub broken: Vec<PathBuf>,
}

impl ChatStore {
    /// Reads a channel back from disk. No directory = empty conversation,
    /// not an error.
    pub fn load(dir: &Path, peer: u64) -> Result<Loaded, String> {
        let doc = ChatDoc::new(peer)?;
        let mut broken = Vec::new();
        let mut ledger = read_ledger(dir);

        for p in blocks(dir)? {
            match fs::read(&p) {
                Ok(bytes) => {
                    if doc.merge(&bytes).is_err() {
                        broken.push(p);
                    } else if let Some(n) = file_name(&p) {
                        ledger.insert(n);
                    }
                }
                Err(_) => broken.push(p),
            }
        }
        let on_disk = doc.version();
        let mut store = ChatStore {
            dir: dir.to_path_buf(),
            on_disk,
            ledger,
        };
        // Only write the ledger if the directory exists: an empty channel
        // should leave nothing on disk.
        if dir.exists() {
            store.save_ledger()?;
        }
        Ok(Loaded { store, doc, broken })
    }

    /// My side of the books, for [`super::plan::plan`].
    pub fn side(&self) -> Result<Side, String> {
        Ok(Side::new(
            blocks(&self.dir)?
                .iter()
                .filter_map(|p| file_name(p))
                .collect::<Vec<_>>(),
            self.ledger.names().iter().cloned().collect::<Vec<_>>(),
        ))
    }

    /// Takes in a block pulled from the far side: merge, write, then
    /// ledger — in that order. Reversed, a mid-way failure marks a block
    /// merged that never was; it is never pulled again and a slice of
    /// conversation goes permanently missing, with no error anywhere.
    pub fn absorb(&mut self, doc: &ChatDoc, name: &str, bytes: &[u8]) -> Result<(), String> {
        doc.merge(bytes)?;
        make_dir(&self.dir)?;
        write_atomic(&self.dir.join(name), bytes)?;
        self.ledger.insert(name);
        self.save_ledger()?;
        self.on_disk = doc.version();
        Ok(())
    }

    /// Reads a block to push to the far side.
    pub fn read_block(&self, name: &str) -> Result<Vec<u8>, String> {
        fs::read(self.dir.join(name)).map_err(|e| format!("读不了 {name}: {e}"))
    }

    fn save_ledger(&mut self) -> Result<(), String> {
        write_atomic(&self.dir.join(LEDGER), self.ledger.render().as_bytes())
    }

    /// Writes what the doc has beyond `on_disk` as one new block; `None`
    /// when there is nothing new.
    ///
    /// The emptiness test is version-vector equality, not
    /// `delta.is_empty()`: loro exports a non-empty header block even with
    /// zero new ops, and judging by emptiness piles up thousands of
    /// "empty" files while everything appears to work.
    pub fn flush(&mut self, doc: &ChatDoc) -> Result<Option<PathBuf>, String> {
        let now = doc.version();
        if now == self.on_disk {
            return Ok(None);
        }
        let delta = doc.changes_since(&self.on_disk)?;
        if delta.is_empty() {
            return Ok(None);
        }
        make_dir(&self.dir)?;
        let path = self.dir.join(format!(
            "u-{:016x}-{:08x}.{EXT}",
            doc.raw().peer_id(),
            self.next_seq(doc.raw().peer_id())?
        ));
        write_atomic(&path, &delta)?;
        // Own blocks enter the ledger too: the far side may push one right
        // back, and an unledgered copy would be re-pulled on every sync.
        if let Some(n) = file_name(&path) {
            self.ledger.insert(n);
        }
        self.save_ledger()?;
        self.on_disk = doc.version();
        Ok(Some(path))
    }

    /// Next unused sequence number for this author, counted from disk,
    /// not memory: two processes of one device (GUI + a CLI run) counting
    /// in memory would collide, and the collision silently overwrites a
    /// block.
    fn next_seq(&self, peer: u64) -> Result<u32, String> {
        let prefix = format!("u-{peer:016x}-");
        let mut max = None::<u32>;
        if let Ok(rd) = fs::read_dir(&self.dir) {
            for e in rd.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                let Some(rest) = name.strip_prefix(&prefix) else {
                    continue;
                };
                let Some(num) = rest.strip_suffix(&format!(".{EXT}")) else {
                    continue;
                };
                if let Ok(n) = u32::from_str_radix(num, 16) {
                    max = Some(max.map_or(n, |m: u32| m.max(n)));
                }
            }
        }
        Ok(max.map_or(0, |m| m + 1))
    }

    /// Compaction: write current state as one snapshot block, then delete
    /// the increments it covers. Write-then-delete, never reversed — a
    /// crash in between must cost a few spare KB, not the whole
    /// conversation. The ledger keeps every line, or the next sync would
    /// pull the compacted blocks right back, every time.
    ///
    /// This does not trim history: other devices syncing afterwards still
    /// receive it all (the snapshot carries it). Shallow snapshots are a
    /// different feature with product implications of their own.
    pub fn compact(&mut self, doc: &ChatDoc) -> Result<PathBuf, String> {
        make_dir(&self.dir)?;
        let old = blocks(&self.dir)?;
        let snap = doc.snapshot()?;
        let path = self
            .dir
            .join(format!("snap-{:08x}.{EXT}", self.next_seq(doc.raw().peer_id())?));
        write_atomic(&path, &snap)?;
        for p in old {
            let _ = fs::remove_file(p);
        }
        if let Some(n) = file_name(&path) {
            self.ledger.insert(n);
        }
        self.save_ledger()?;
        self.on_disk = doc.version();
        Ok(path)
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }
}

/// All blocks, sorted by name. Order doesn't affect the merge result
/// (increments commute); sorting exists so two loads behave identically
/// and failures reproduce.
fn blocks(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let Ok(rd) = fs::read_dir(dir) else {
        return Ok(Vec::new()); // no directory = empty conversation
    };
    let mut out: Vec<PathBuf> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == EXT))
        .collect();
    out.sort();
    Ok(out)
}

/// Blocks get copied to other machines and their mode travels along. A
/// block is the document itself — the bytes reconstruct the whole
/// conversation — so the create() default (644 under common umask) leaks
/// it on any shared-home machine. "Not plaintext" is no protection.
#[cfg(unix)]
const OWNER_ONLY_FILE: u32 = 0o600;
/// Directories likewise: 755 lets anyone cd in and list.
#[cfg(unix)]
const OWNER_ONLY_DIR: u32 = 0o700;

fn make_dir(dir: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        // .mode() applies only to directories actually created here; a
        // pre-existing 755 directory stays 755.
        return fs::DirBuilder::new()
            .recursive(true)
            .mode(OWNER_ONLY_DIR)
            .create(dir)
            .or_else(|e| {
                if dir.is_dir() { Ok(()) } else { Err(format!("建不了目录: {e}")) }
            });
    }
    #[cfg(not(unix))]
    fs::create_dir_all(dir).map_err(|e| format!("建不了目录: {e}"))
}

/// tmp + sync + rename: readers see a whole block or none. The sync
/// matters — without it the rename can reach disk before the content, and
/// a crash leaves a right-sized block full of zeros that reads fine.
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension("tmp");
    {
        let mut opts = fs::OpenOptions::new();
        opts.create(true).write(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(OWNER_ONLY_FILE);
        }
        let mut f = opts
            .open(&tmp)
            .map_err(|e| format!("写不了 {}: {e}", tmp.display()))?;
        f.write_all(bytes).map_err(|e| format!("写不完: {e}"))?;
        f.sync_all().map_err(|e| format!("落不了盘: {e}"))?;
    }
    fs::rename(&tmp, path).map_err(|e| format!("改不了名: {e}"))
}

fn file_name(p: &Path) -> Option<String> {
    Some(p.file_name()?.to_string_lossy().to_string())
}

fn read_ledger(dir: &Path) -> Ledger {
    // Unreadable = empty ledger, not an error: the cost is one idempotent
    // re-pull, while erroring here would brick a fresh channel.
    fs::read_to_string(dir.join(LEDGER))
        .map(|t| Ledger::parse(&t))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::chat::doc::Sender;
    use crate::chat::testutil::{me, render, tmpdir};

    /// Flush writes only what disk lacks. Without this, "append-only"
    /// silently becomes quadratic — the Nth flush writes N histories,
    /// and everything still works.
    #[test]
    fn flush_writes_only_what_is_not_on_disk_yet() {
        let dir = tmpdir("flush");
        let mut st = ChatStore::load(&dir, 1).unwrap();
        for i in 0..50 {
            st.doc.say(&me("a"), &format!("第 {i} 句")).unwrap();
        }
        let first = st.store.flush(&st.doc).unwrap().expect("该写出一个块");
        let big = fs::metadata(&first).unwrap().len();

        st.doc.say(&me("a"), "又一句").unwrap();
        let second = st.store.flush(&st.doc).unwrap().expect("该再写一个块");
        let small = fs::metadata(&second).unwrap().len();

        assert!(
            small * 5 < big,
            "第二块该只装那一句(第一块 {big} 字节,第二块 {small} 字节)"
        );
        // No new content, no file — not even an empty one.
        assert!(st.store.flush(&st.doc).unwrap().is_none(), "没有新东西不该写文件");
        let _ = fs::remove_dir_all(&dir);
    }

    /// What is flushed comes back, identical.
    #[test]
    fn what_is_flushed_comes_back() {
        let dir = tmpdir("roundtrip");
        let mut st = ChatStore::load(&dir, 1).unwrap();
        st.doc.say(&me("a"), "一").unwrap();
        st.doc.say(&me("a"), "二").unwrap();
        st.store.flush(&st.doc).unwrap();
        let want = render(&st.doc);

        let back = ChatStore::load(&dir, 1).unwrap();
        assert!(back.broken.is_empty(), "不该有坏块");
        assert_eq!(render(&back.doc), want);
        let _ = fs::remove_dir_all(&dir);
    }

    /// One broken block neither kills the channel nor disappears
    /// silently: it must be counted, so the UI can say a slice is
    /// missing.
    #[test]
    fn one_broken_block_does_not_kill_the_channel() {
        let dir = tmpdir("broken");
        let mut st = ChatStore::load(&dir, 1).unwrap();
        st.doc.say(&me("a"), "好的那句").unwrap();
        st.store.flush(&st.doc).unwrap();
        fs::write(dir.join("u-000000000000ffff-00000000.loro"), b"not a loro block").unwrap();

        let back = ChatStore::load(&dir, 1).unwrap();
        assert_eq!(back.broken.len(), 1, "坏块要被数出来,不能静默吞掉");
        assert_eq!(back.doc.messages().len(), 1, "好的那句还得在");
        let _ = fs::remove_dir_all(&dir);
    }

    /// Compaction loses not a word, and the old blocks are gone.
    #[test]
    fn compacting_keeps_every_word() {
        let dir = tmpdir("compact");
        let mut st = ChatStore::load(&dir, 1).unwrap();
        for i in 0..20 {
            st.doc.say(&me("a"), &format!("第 {i} 句")).unwrap();
            st.store.flush(&st.doc).unwrap();
        }
        let want = render(&st.doc);
        // Count blocks only, not the ledger (`.merged` shares the
        // directory): otherwise this measures "files in dir" when it
        // asks "blocks left".
        let count_blocks = |d: &PathBuf| {
            fs::read_dir(d)
                .unwrap()
                .flatten()
                .filter(|e| e.file_name().to_string_lossy().ends_with(".loro"))
                .count()
        };
        let before = count_blocks(&dir);
        assert!(before >= 20, "压实前该有一堆块,实测 {before}");

        st.store.compact(&st.doc).unwrap();
        let after = count_blocks(&dir);
        assert_eq!(after, 1, "压实后只该剩那个快照,实测 {after}");

        let back = ChatStore::load(&dir, 1).unwrap();
        assert_eq!(render(&back.doc), want, "压实不许丢任何一句");
        let _ = fs::remove_dir_all(&dir);
    }

    /// Two writers on one device, each with its own peer: both lines
    /// survive and the block names don't collide (`next_seq` counts from
    /// disk). "GUI open plus a CLI run" is a real scene.
    #[test]
    fn two_writers_on_the_same_device_do_not_collide() {
        let dir = tmpdir("collide");
        // Same directory, same device, different peers — ChatDoc::new's
        // contract.
        let mut one = ChatStore::load(&dir, 0xA1).unwrap();
        let mut two = ChatStore::load(&dir, 0xA2).unwrap();

        one.doc.say(&me("a"), "来自进程一").unwrap();
        let p1 = one.store.flush(&one.doc).unwrap().unwrap();
        two.doc.say(&me("a"), "来自进程二").unwrap();
        let p2 = two.store.flush(&two.doc).unwrap().unwrap();

        assert_ne!(p1, p2, "两个块不许同名");
        let back = ChatStore::load(&dir, 0xA3).unwrap();
        let text = render(&back.doc);
        assert!(
            text.contains("来自进程一") && text.contains("来自进程二"),
            "两句都得在:\n{text}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

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

    /// What lands on disk is readable only by me. Blocks get copied to
    /// other machines and the mode travels; a block is the document
    /// itself, so "not plaintext" protects nothing. This measures the
    /// mode bits, not which API was called — the latter stays green when
    /// broken.
    #[cfg(unix)]
    #[test]
    fn what_lands_on_disk_is_readable_only_by_me() {
        use std::os::unix::fs::PermissionsExt;

        let home = tmpdir("perms");
        let dir = channel_dir(&home, "turing").expect("频道名该是合法的");
        let mut l = ChatStore::load(&dir, 7).expect("空频道要开得起来");
        l.doc
            .say(&Sender { id: "a".into(), name: "A".into() }, "一句话")
            .expect("说不出话");
        l.store.flush(&l.doc).expect("落不了盘");

        let mode = |p: &std::path::Path| {
            std::fs::metadata(p).expect("读不到属性").permissions().mode() & 0o777
        };
        assert_eq!(mode(&dir), 0o700, "频道目录不许让别人 cd 进来");

        let mut n = 0;
        for e in std::fs::read_dir(&dir).expect("列不了目录") {
            let p = e.expect("读不到条目").path();
            assert_eq!(mode(&p), 0o600, "{} 不该让别人读得到", p.display());
            n += 1;
        }
        // Prove the round wrote anything at all: a write-nothing
        // implementation idles through the loop above and every
        // assertion passes. At least two files: one block plus the
        // ledger.
        assert!(n >= 2, "这一趟该写出块和账本,实测只有 {n} 个文件");
    }
}
