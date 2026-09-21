//! `khor config`: the preferences this machine keeps (docs/MORE.md 设置).
//!
//! # A table with a value domain, not a bag of strings
//!
//! Every key is declared here with the values it takes and the one it
//! has when nobody has said otherwise ([`KEYS`]). An unknown key and an
//! unknown value are both **refused by name** — the shape
//! `lock::set_timeout` already has, and for its reason: a value the
//! faces do not draw would be a setting somebody could reach from the
//! terminal and never see again, and a key nothing reads would be a
//! preference that silently does nothing forever.
//!
//! That is also why there is no free-text key on this table. A key whose
//! domain cannot be written down cannot be refused by name, and the
//! first thing a typo in one buys is a screen that draws no selection at
//! all while khor goes on reporting the typo back as the setting.
//!
//! # It is a local preference, not a synced document
//!
//! `.khor/config.json`, beside `lock.json` and `avatar.json`, for the
//! reason those two are local: **the choice is made here and never
//! travels**. 「分两栏的事这一轮不做（「本机的 / 全网同步的」要等网络宏图），
//! 全部是本机的」(docs/MORE.md) — and a theme that replicated would mean
//! a phone deciding what a desktop looks like.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use khor_catalog::msg;

/// A heading on one of the 「…」 pages, and which page that is.
///
/// **Here rather than in the two faces**, both halves. The pages have to
/// group these rows somehow and decide which of them draws which group;
/// written in TS that would be two tables of one fact — ones that
/// compile, run, and put 危险动作二次确认 on 设置 the day somebody adds a
/// key.
#[derive(Debug, Clone, Copy)]
pub struct Section {
    pub name: &'static str,
    /// The `data-more-page` this section is drawn under
    /// (`views/more/MorePage`'s `MorePage`).
    pub page: &'static str,
}

/// The sections, in the order the pages draw them. A list rather than an
/// order field on each key: the order is a fact about the page, and a
/// number on every key is a thing to renumber.
///
/// ⚠️ [`KEYS`] is kept in this order too, so a page can start a new
/// heading wherever the section changes between neighbouring rows and
/// never needs a list of its own.
pub const SECTIONS: &[Section] = &[
    // 外观. First, because it is the one setting on this page a person
    // sees the effect of before they have finished reading the row.
    Section {
        name: "appearance",
        page: "settings",
    },
    // 语言, under 外观: the two are what this whole page is written in,
    // and a person looking for either is looking at the top of it.
    Section {
        name: "language",
        page: "settings",
    },
    // 通用: what holds for every landing at once — the tab strips are
    // the first thing on it (docs/SHELL.md 第三轮 第 4 条).
    Section {
        name: "general",
        page: "settings",
    },
    // 会话: what a new one starts out as; 工作区 under it.
    Section {
        name: "session",
        page: "settings",
    },
    Section {
        name: "workspace",
        page: "settings",
    },
    // Drawn on 安全 rather than 设置, because that is where docs/MORE.md
    // puts it and because the sections beside it there are the other
    // things that cost something.
    Section {
        name: "confirm",
        page: "security",
    },
];

/// The section a name belongs to, if khor has one.
pub fn section(name: &str) -> Option<&'static Section> {
    SECTIONS.iter().find(|s| s.name == name)
}

/// One setting: its name, where it is drawn, what it takes, and what it
/// is when nobody has said.
#[derive(Debug, Clone, Copy)]
pub struct Key {
    pub name: &'static str,
    pub section: &'static str,
    /// Every value this key takes, in the order the face offers them.
    pub values: &'static [&'static str],
    pub default: &'static str,
}

/// The whole table.
///
/// ⚠️ **A key belongs here only once something reads it.** docs/MORE.md:
/// 没有落地实现的键不画 — and the same sentence applies one layer down,
/// where a key with no reader is worse than a greyed switch: it stores
/// an answer, reports it back, and changes nothing.
pub const KEYS: &[Key] = &[
    // ── 外观 ──────────────────────────────────────────────────────
    //
    // 「现在只跟系统——加一个覆盖」(docs/MORE.md 设置). `auto` is what
    // khor did before this key existed, so the default is the behaviour
    // rather than a new opinion about it.
    //
    // **Nobody reads this on the Rust side except the window's own
    // ground** (`src-tauri`'s `paint_the_window_before_the_page`): the
    // palette is CSS, and the face resolves `auto` itself against
    // `matchMedia` because that is the only place the question can be
    // asked. Which is also why the value stored here is the *choice*
    // (`auto` included) and never the resolved theme — a store holding
    // 「dark」 because the desktop was dark that evening is a machine
    // that stops following the desktop without anybody having said so.
    Key {
        name: "theme",
        section: "appearance",
        values: &["auto", "light", "dark"],
        default: "auto",
    },
    // ── 语言 ──────────────────────────────────────────────────────
    //
    // 「自动跟随系统语言，我们支持英语与中文。失败会退英文」 (the user,
    // 2026-09-19). The fallback is not a rung on this ladder: it is what
    // `auto` resolves to when the machine says something that is not
    // zh*, and 「says nothing at all」 is that same case
    // (`khor_catalog::lang::from_env`).
    //
    // **Stored as the choice, `auto` included**, for `theme`'s reason
    // one step sharper: a store holding 「en」 because somebody was on an
    // English machine that week is a laptop that stops speaking Chinese
    // when it goes home.
    Key {
        name: "lang",
        section: "language",
        values: &["auto", "zh", "en"],
        default: "auto",
    },
    // ── 通用 ──────────────────────────────────────────────────────
    //
    // 标签页上限 (docs/SHELL.md 第三轮 第 4 条): past this many, opening
    // one more closes the one touched longest ago that is neither pinned
    // nor holding unsaved work — and when every candidate is one of
    // those, nothing is closed and the strip simply grows. `off` is
    // what the strips have always done.
    //
    // **The numbers are the values themselves**, not words: `4` means
    // four in every language, so the catalog has no entry for them and
    // `word()`'s echo is the answer (the gate in `tests.rs` knows).
    Key {
        name: "tabs-limit",
        section: "general",
        values: &[
            "off", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16", "17",
            "18", "19", "20", "21", "22", "23", "24", "25", "26", "27", "28", "29", "30", "31",
            "32", "33", "34", "35", "36", "37", "38", "39", "40", "41", "42", "43", "44", "45",
            "46", "47", "48", "49", "50",
        ],
        default: "off",
    },
    // ── 会话 ──────────────────────────────────────────────────────
    //
    // Which of the two forms the new-session wizard starts on
    // (`NewSessionDialog`). `chat` is what it has always opened as.
    Key {
        name: "session-form",
        section: "session",
        values: &["chat", "term"],
        default: "chat",
    },
    // ── 工作区 ────────────────────────────────────────────────────
    //
    // What one click on a file in the tree does: take the preview seat
    // (the next click replaces it) or open a tab of its own. Double
    // click pins either way, so `preview` is not a road with no exit —
    // and `preview` is what the tree has always done.
    Key {
        name: "files-click",
        section: "workspace",
        values: &["preview", "pin"],
        default: "preview",
    },
    // ── 危险动作二次确认 ──────────────────────────────────────────
    //
    // **Two, not the three docs/MORE.md lists, and the defaults are not
    // both 开.** 删文件 is already asked with a 「不再提醒」 tick of its
    // own, kept per face in `localStorage` (`RemoveDialog`: 「一个习惯,
    // 不是一个关于网络的决定」), and the verification side ruled on
    // 2026-09-20 that it stays there — promoting it would turn a habit
    // one browser remembers into one answer a phone and a desktop share.
    //
    // 接管会话 carries a confirm today, so `on` is the behaviour.
    // 移出工作区 does **not**: 「nothing is destroyed. The directory is
    // untouched」 (the user, 2026-09-14), so `off` is the behaviour and
    // the switch is there for somebody who wants the question anyway. A
    // default of `on` would have been this table quietly reversing a
    // ruling.
    Key {
        name: "confirm-takeover",
        section: "confirm",
        values: &["on", "off"],
        default: "on",
    },
    Key {
        name: "confirm-ws-remove",
        section: "confirm",
        values: &["on", "off"],
        default: "off",
    },
];

/// The declaration for a name, if khor has one.
pub fn key(name: &str) -> Option<&'static Key> {
    KEYS.iter().find(|k| k.name == name)
}

fn path(root: &Path) -> PathBuf {
    root.join(".khor").join("config.json")
}

/// Where the preferences are written, for the test that asserts
/// **reading writes nothing**. Not public otherwise: a caller outside
/// this module that knew the path would be a second way to write it.
#[cfg(test)]
pub fn path_for_tests(root: &Path) -> PathBuf {
    path(root)
}

/// **Unreadable and unparseable both fall back to the defaults**, the
/// same direction `lock::stored` chooses and for the same reason: the
/// defaults are what khor did before any of these keys existed, so a
/// file nobody can read costs a person their preferences and never their
/// machine.
fn stored(root: &Path) -> BTreeMap<String, String> {
    std::fs::read_to_string(path(root))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn save(root: &Path, map: &BTreeMap<String, String>) -> Result<(), String> {
    let dot = root.join(".khor");
    std::fs::create_dir_all(&dot).map_err(|_| msg::cant_make_dir(dot.display()))?;
    let text = serde_json::to_vec_pretty(map).map_err(|e| e.to_string())?;
    let p = path(root);
    std::fs::write(&p, text).map_err(|e| msg::cant_write(p.display(), e))
}

/// What a key is set to — its default when nobody has said, and **its
/// default again when the file holds a value this build no longer
/// takes**.
///
/// That second case is the one worth the sentence: a store written by a
/// newer khor, or edited by hand, can hold `theme = "sepia"`. Handing
/// that back would put a value on screen that no face can draw and no
/// `set` would accept — so it is read as unset, which is a state every
/// face already knows how to paint.
fn value_of(map: &BTreeMap<String, String>, k: &Key) -> &'static str {
    map.get(k.name)
        .and_then(|v| k.values.iter().find(|w| *w == v))
        .copied()
        .unwrap_or(k.default)
}

/// One key's value. Refused by name for a key khor does not have.
pub fn get(root: &Path, name: &str) -> Result<&'static str, String> {
    let k = key(name).ok_or_else(|| msg::config_no_such_key(name))?;
    Ok(value_of(&stored(root), k))
}

/// Sets one key. Refused by name for an unknown key **and for a value
/// outside that key's ladder** — the two refusals are different
/// sentences because they send a person to two different places.
pub fn set(root: &Path, name: &str, value: &str) -> Result<&'static str, String> {
    let k = key(name).ok_or_else(|| msg::config_no_such_key(name))?;
    let value = k.values.iter().find(|w| **w == value).ok_or_else(|| {
        msg::config_no_such_value(
            name,
            value,
            k.values.join(khor_catalog::cli::name_separator()),
        )
    })?;
    let mut map = stored(root);
    map.insert(k.name.to_owned(), (*value).to_owned());
    save(root, &map)?;
    Ok(value)
}

/// Every key and what it is set to, in [`KEYS`] order.
///
/// **Every key, never only the ones somebody has touched.** A listing of
/// what is in the file would answer 「什么都没设过」 with nothing at all
/// — a command that prints an empty page is a command that looks broken.
pub fn list(root: &Path) -> Vec<(&'static Key, &'static str)> {
    let map = stored(root);
    KEYS.iter().map(|k| (k, value_of(&map, k))).collect()
}

/// Resolves 语言 and tells the catalog, for **this whole process**
/// (docs/MORE.md 语言).
///
/// **The one place the store and the wording meet.** `khor_catalog` has
/// no dependencies and cannot read a config file; this crate can, and
/// every binary — the CLI, the app, the bridge, the resident — calls
/// this once at startup so that the first word it prints is already in
/// the right language.
///
/// A stored choice wins; `auto` and a store that cannot be read both
/// fall through to the environment, which falls through to English.
/// That chain is the ruling 「显式设过 lang 的以它为准 … 读不到回退英文」
/// with nothing in between for anybody to get wrong.
pub fn apply_lang(root: &Path) {
    use khor_catalog::lang::Lang;
    let chosen = match get(root, "lang") {
        Ok("zh") => Lang::Zh,
        Ok("en") => Lang::En,
        _ => khor_catalog::lang::from_env(),
    };
    khor_catalog::lang::set(chosen);
}

/// 标签页上限 as a number, or `None` for `off` (and for a store that
/// cannot be read, which reads as the default).
pub fn tabs_limit(root: &Path) -> Option<usize> {
    get(root, "tabs-limit").ok().and_then(|v| v.parse().ok())
}

/// Whether a switch-shaped key is on. The two `confirm-*` keys' readers
/// want a bool, and spelling `== "on"` at each call site would be the
/// place the third one spells it `"true"`.
pub fn on(root: &Path, name: &str) -> bool {
    get(root, name).map(|v| v == "on").unwrap_or(false)
}
