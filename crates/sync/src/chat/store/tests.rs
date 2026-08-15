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
        st.doc.tell(&me("a"), &format!("第 {i} 句")).unwrap();
    }
    let first = st.store.flush(&st.doc).unwrap().expect("该写出一个块");
    let big = fs::metadata(&first).unwrap().len();

    st.doc.tell(&me("a"), "又一句").unwrap();
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
    st.doc.tell(&me("a"), "一").unwrap();
    st.doc.tell(&me("a"), "二").unwrap();
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
    st.doc.tell(&me("a"), "好的那句").unwrap();
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
        st.doc.tell(&me("a"), &format!("第 {i} 句")).unwrap();
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

    one.doc.tell(&me("a"), "来自进程一").unwrap();
    let p1 = one.store.flush(&one.doc).unwrap().unwrap();
    two.doc.tell(&me("a"), "来自进程二").unwrap();
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
        .tell(&Sender { id: "a".into(), name: "A".into() }, "一句话")
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
