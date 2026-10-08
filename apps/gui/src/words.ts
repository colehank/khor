// Wording helpers on top of the generated catalog. The display word is
// looked up at the last moment; unknown keys echo — an old face must
// still render a session whose kind it has never heard of.
import { avatar, category, cli, gui, hop, one, state } from "./gen/catalog";
import { type HopLeg, isDown } from "./lib/topology";
export { hopRoad } from "./lib/topology";

/**
 * **Counted words, singular where the language has one** (`[one]` in the
 * catalog; 2026-10-03, 「1 minutes ago」): every count of time the face
 * prints goes through here, the GUI's twin of `khor_catalog::count`.
 */
export const count = {
  seconds: (n: number) => (n === 1 ? one.second(n) : cli.age_seconds(n)),
  minutes: (n: number) => (n === 1 ? one.minute(n) : cli.age_minutes(n)),
  hours: (n: number) => (n === 1 ? one.hour(n) : cli.age_hours(n)),
  days: (n: number) => (n === 1 ? one.day(n) : cli.age_days(n)),
  upDays: (n: number) => (n === 1 ? one.up_day(n) : cli.uptime_days(n)),
  upHours: (n: number) => (n === 1 ? one.up_hour(n) : cli.uptime_hours(n)),
  lockIdleAfter: (n: number) => (n === 1 ? one.lock_idle_after(n) : cli.lock_idle_after(n)),
};

export function word(key: string): string {
  return (state as Record<string, string>)[key] ?? key;
}

/**
 * `/a/b/khor` → `khor`. The last real component, so a trailing slash
 * does not answer with an empty name, and a path with no component at
 * all answers with itself.
 *
 * **The same rule the node applies** when it names a workspace row
 * (`khor_node::workspace::basename`); it is spelled here too because
 * one caller has a path and no row — the workspace pane's header, whose
 * `root` is a string handed down by the shell.
 */
export function basename(path: string): string {
  return path.split("/").filter(Boolean).pop() ?? path;
}

/**
 * The word for which road a machine is being reached over
 * (`khor_core::Hop`), from the key the node sent.
 *
 * Its own lookup rather than a second use of `word` above: the keys are
 * deliberately not shared with the state table — `quiet` exists here and
 * nowhere else — precisely so that reading one in the
 * wrong table echoes the key instead of quietly finding a real word and
 * painting it. Sharing a lookup would give that mistake back.
 */
export function hopWord(key: string): string {
  return (hop as Record<string, string>)[key] ?? key;
}

/**
 * The road word with its liveness beside it, when that says something
 * (`khor_core::Link`): a machine **not connected** that was heard before
 * reads 「未连 · 上次 3 分钟前」. A connected one is heard every few
 * seconds by keepalive alone, so its age would only ever say "now" —
 * it is left off, the way `ageIfOld` leaves off a fresh reading.
 *
 * **Idle is not down** (round NET-13): `quiet` is no live wire, and the
 * next open dials it fine; 「不通」 is said only when the resident's last
 * attempt failed and nothing was heard since (`downMs`, `Link::down_ms`).
 */
export function linkWord(key: string, heardMs: number | null | undefined, downMs?: number | null): string {
  const road = hopWord(key === "quiet" && isDown(downMs) ? "down" : key);
  if (key !== "quiet" || heardMs == null) return road;
  return gui.link_last_heard(road, agoOf(heardMs));
}

/** **不通, with when it was last heard** (WORDS-1008: 「不通 · 上次 27 分钟
    前」; never 离线 or 无法获取): the map's caption under an unreachable
    seat and the overview's word, from the same key the rows use. */
export function downWord(heardMs: number | null | undefined): string {
  return heardMs == null ? hopWord("down") : gui.link_last_heard(hopWord("down"), agoOf(heardMs));
}


/**
 * How this machine reaches that one, as the word a row or a card prints
 * beside the name — the one fact a person picks a machine by.
 *
 * **One derivation, every surface.** The machines list, the machine
 * card, the landing's 机器 strip, the new-session picker and the
 * place card all print this word; when it was spelled at each of them,
 * two of the five kept the old `(本机)` bracket the user had asked to
 * drop (2026-09-02:「本机不用写一个括号在机器名旁边。连接方式就是
 * 本机」) while the other three had moved on — one fact, five sources,
 * and the drift was visible on screen. This machine reaches itself over
 * nothing, so its word is 本机; a satellite has no road of its own and
 * its sponsor speaks for it; otherwise the road (`hopWord`). `null` when
 * khor cannot name the road yet — absent beats a blank.
 */
export function connectionWord(row: {
  me: boolean;
  sponsor: string | null;
  hop: string | null;
  via?: string | null;
  heard_ms?: number | null;
  down_ms?: number | null;
  paused?: boolean;
}): string | null {
  if (!row.paused && !row.me && !row.sponsor && row.hop && !(row.hop === "forwarded" && row.via)) {
    return linkWord(row.hop, row.heard_ms, row.down_ms);
  }
  return connectionKeyWord(connectionKey(row));
}

/**
 * The same fact as a key, for the machines list's filter (2026-09-29):
 * `here` for this machine, `via:<name>` for a machine reached through
 * another (a satellite's sponsor, a peer hop's middle), the hop key
 * otherwise, and `""` where `connectionWord` has no word. **The word is
 * looked up from the key** (`connectionKeyWord`), so a tick in the
 * filter and the word on the row cannot come apart — there is one
 * derivation, which is this one. The prefixes cannot collide: a hop key
 * is one kebab word (`khor_core::Hop`), never `here` and never with a
 * colon in it. An idle machine whose last attempt failed is `down`, the
 * one key here that is not a road (`khor_core::Link::DOWN_KEY`).
 * **Through another machine is one key, `via:<name>`**, worded 「通过
 * hinton」 (WORDS-1008; the lead 2026-10-09): a satellite whose sponsor
 * is hinton and a peer hop through hinton (NET-DIRECT) say the same
 * words, so they are one tick in the filter, not two that read alike.
 */
export function connectionKey(row: {
  me: boolean;
  sponsor: string | null;
  hop: string | null;
  via?: string | null;
  down_ms?: number | null;
  paused?: boolean;
}): string {
  // **Paused wins** (WORDS-1008: 已暂停, the map says it too): this
  // machine stopped talking to it, whatever its wire last did — and the
  // filter's 已暂停 is this key, so the tick and the row cannot differ.
  if (row.paused) return "paused";
  if (row.me) return "here";
  if (row.sponsor) return `via:${row.sponsor}`;
  if (row.hop === "quiet" && isDown(row.down_ms)) return "down";
  if (row.hop === "forwarded" && row.via) return `via:${row.via}`;
  return row.hop ?? "";
}

/** The word for a `connectionKey`, `null` for the empty one. */
export function connectionKeyWord(key: string): string | null {
  if (key === "paused") return gui.machine_paused;
  if (key === "here") return gui.machine_here;
  if (key.startsWith("via:")) return gui.add_step_via(key.slice("via:".length));
  if (key !== "") return hopWord(key);
  return null;
}

/** A leg's line: 「iPhone → turing 直连 26 ms」. */
export function legWord(l: HopLeg): string {
  const road = l.road ? hopWord(l.road) : gui.filter_unknown;
  return l.ms === null ? gui.hop_leg(l.from, l.to, road) : gui.hop_leg_rtt(l.from, l.to, road, latency(l.ms));
}

/**
 * The name of a machine's system family (`khor_core::SystemKind`), from
 * the key the node sent — the family a machine row's corner draws
 * (`BrandIcons.machineMark`), so a filter on it and the corner read the
 * same. An unknown key echoes, `hopWord`'s reason: a family a newer node
 * knows is still better named by its key than not named at all.
 */
export function systemWord(key: string): string {
  switch (key) {
    case "macos":
      return gui.system_macos;
    case "ios":
      return gui.system_ios;
    case "apple":
      return gui.system_apple;
    case "linux":
      return gui.system_linux;
    case "windows":
      return gui.system_windows;
    case "android":
      return gui.system_android;
    default:
      return key;
  }
}

/**
 * The word for one face option — a factory palette's id, a variant key
 * or a shape key, exactly as the node sent it.
 *
 * Unknown keys echo, same as above. It should never fire: a Rust gate
 * (`every_face_option_has_a_word_and_every_word_names_an_option`) walks
 * both directions across this section. The echo is what a *newer* node
 * talking to an older screen falls back to — a palette named `okabe` on
 * a button is ugly, a button with no name at all is unusable.
 */
export function faceWord(key: string): string {
  // Guarded on the type rather than cast through, because this section
  // is not all plain words: `axis-palette-slot` takes an argument, so a
  // straight lookup can hand back a function, and a function rendered
  // into JSX is a blank where a name belongs.
  const found: unknown = (avatar as Record<string, unknown>)[key];
  return typeof found === "string" ? found : key;
}

/**
 * A group heading, from the key the node put on the row.
 *
 * The prefix says what the rest of the key is, so this dispatches
 * instead of guessing (`khor_node::list` module head): a state key and a
 * category key are looked up, a machine name is printed as it stands.
 * Without the prefixes a machine called `busy` would come out 忙碌.
 *
 * An unknown category echoes, which is not a fallback but the design:
 * vendor names are proper nouns with no catalog entry, so echoing them
 * *is* the translation.
 */
export function groupLabel(group: string): string {
  if (group === "pin") return cli.group_pinned;
  // The khor-wide sessions' group when arranged by machine (the broadcast,
  // `khor_node::list::GROUP_KHOR`): the khor's own name.
  if (group === "khor") return category.khor;
  if (group.startsWith("state:")) return word(group.slice("state:".length));
  if (group.startsWith("cat:")) {
    const name = group.slice("cat:".length);
    const table = category as Record<string, string>;
    return name === "" ? table.unknown : (table[name] ?? name);
  }
  if (group.startsWith("at:")) {
    const path = group.slice("at:".length);
    // **The last component, which is the name people use.** The key
    // keeps the whole path — two checkouts called `gui` in different
    // trees are two groups, and they stay two — but a sidebar heading
    // is three inches wide, and the head of a path is the part nobody
    // reads. When two groups on one screen share that name,
    // `placeLabels` is the reader to use: it sees them both.
    if (path === "") return gui.group_placeless;
    return tail(path, 1);
  }
  return group.startsWith("dev:") ? group.slice("dev:".length) : group;
}

/** The last `n` components of a path, joined as they were. */
function tail(path: string, n: number): string {
  const parts = path.split("/").filter((p) => p !== "");
  return parts.length === 0 ? path : parts.slice(-n).join("/");
}

/**
 * Labels for a set of place groups, **told apart where their names
 * collide**. `groupLabel` reads one key and prints its last component;
 * two checkouts called `gui` in different trees then print two
 * identical headings over two different groups — never merged (the
 * keys differ), but indistinguishable on the screen. This reads them
 * all, and for any name shared by two keys it prints as many trailing
 * components as it takes to tell them apart (`apps/gui` and
 * `web/gui`), leaving every uncontested name as short as it was.
 *
 * Keyed by the full group key, so a caller with a heading to draw asks
 * with the key it has and never re-derives the name.
 */
export function placeLabels(groups: Iterable<string>): Map<string, string> {
  const keys = [...new Set(groups)].filter((g) => g.startsWith("at:") && g !== "at:");
  const out = new Map<string, string>();
  const paths = keys.map((k) => k.slice("at:".length));
  for (let i = 0; i < keys.length; i += 1) {
    let n = 1;
    let label = tail(paths[i], n);
    // Widen until no other path prints the same, or there is nothing
    // left to widen with — two identical paths cannot be told apart
    // and would not be two keys.
    while (paths.some((p, j) => j !== i && tail(p, n) === label) && n < paths[i].split("/").length) {
      n += 1;
      label = tail(paths[i], n);
    }
    out.set(keys[i], label);
  }
  return out;
}

/**
 * The four axes a session list can be filtered on, in the node's own
 * spelling (`khor_node::list` — `GROUP_STATE` / `GROUP_DEVICE` /
 * `GROUP_CATEGORY` / `GROUP_PLACE`).
 *
 * **Nothing here is a vocabulary this app invented.** The node already
 * mints these keys to group by, `groupLabel` above already reads them,
 * and a filter is the same question a grouping answers — so a ticked
 * filter and a group heading are the same string, and there is no second
 * spelling that can drift from the first.
 */
export const AXES = ["state:", "dev:", "cat:", "at:"] as const;
export type Axis = (typeof AXES)[number];

/**
 * The 额度 page's three (2026-09-06:「组别可以有来源(机器、设置,即 khor
 * 登记的)、提供商、类型(订阅、计费)」).
 *
 * **A second list rather than four more entries in the first.** These
 * are not sessions and none of these keys can ever appear in a session's
 * store or the other way round; one flat list would let a tick from one
 * page be read on the other, and the reading would be silent nonsense
 * rather than an error.
 *
 * ⚠️ **`from:` and not `home:`, because the value is not always a
 * machine.** 来源 is 「where this row's reading comes from」: a
 * subscription row's is the machine whose Claude Code was read, an
 * account's is 「khor 登记」 — the account was typed into khor, and
 * *which machine holds its key* is a property of the account, shown in
 * the detail, not where the reading comes from. One value per row, so
 * an OR within the axis cannot select overlapping sets and a grouping
 * puts each row in exactly one group.
 *
 * ⚠️ **And not `at:`**, which on the session list means the working
 * directory. Two stores that never meet would not actually collide, but
 * re-using a spelling for a different meaning is the drift the note
 * above this one exists to prevent.
 */
export const QUOTA_AXES = ["kind:", "by:", "from:"] as const;
export type QuotaAxis = (typeof QUOTA_AXES)[number];

/**
 * Which axis a ticked key belongs to.
 *
 * **A key with no prefix is a state word**, and that rule is doing two
 * jobs at once: it is how the axis is read off a key, and it is the
 * whole of the compatibility with what an older app wrote — this
 * preference used to be a list of bare state keys, so a stored value
 * from before the other two axes existed parses as what it always meant.
 * One line instead of a migration, and it keeps working in the other
 * direction too (an older app reading a newer store sees keys it does
 * not know and, per its own rule, simply matches no rows on them).
 */
export function axisOf(key: string): Axis;
export function axisOf<T extends readonly string[]>(key: string, axes: T): T[number];
export function axisOf(key: string, axes: readonly string[] = AXES): string {
  const found = axes.find((a) => key.startsWith(a));
  // **The fallback is the sessions rule, and it is first on purpose.**
  // For that list `axes[0]` is `state:`, which is exactly the
  // compatibility described above. For any other list nothing ever
  // wrote a bare key, so an unprefixed key does not arise there — and
  // if one ever did, landing on the first axis is at least a value
  // that can be untucked rather than a crash.
  return found ?? axes[0];
}

/** The value inside a ticked key — the state word, machine name, or
    category name. An unprefixed key is the whole value (see `axisOf`). */
export function valueOf(key: string, axes: readonly string[] = AXES): string {
  const a = axes.find((x) => key.startsWith(x));
  return a ? key.slice(a.length) : key;
}

// ── Time (WORDS-1008, the owner 2026-10-08) ─────────────────────────
//
// **One way to say a time, and the direction is always said.** 「连不上
// 叫不通; MDH这个只在需要缩写的地方吧……普通的还是2秒前这样.重置如2小时13
// 分后重置」. Three shapes, and every time on the face is one of them:
//
// - **a moment in the past** — 「2 秒前」「3 分钟前」「4 小时前」 (`ago`,
//   `agoOf`): list corners, last heard, last answered, a reading's age;
// - **a moment ahead** — 「2 小时 13 分后」 (`until`): two units where the
//   second matters, never rounded down to one (a reset in 2 h 13 min was
//   「2 小时后」 and read as two hours early); `untilBare` under a column
//   head that already says which way;
//
// **Minutes** (the lead, 2026-10-08): alone they are 分钟 (「45 分钟后重
// 置」「3 分钟前」), second after hours they are 分 (「2 小时 13 分」).
// Seconds are only ever alone (「2 秒前」), never a second unit.
// - **a duration** — 「4 小时」 bare (`lasting`, `stretch`): how long a
//   state has held, how long a process ran, how long something waited.
//
// The ladder itself (`span`) does not leave this file: a bare number of
// hours handed to a page is the ambiguity the owner pointed at (a session
// row's 「4 小时」 that was 4 hours *ago*). **A catalog sentence never
// carries the direction itself** (no 「{0}前」 in `[gui]`): it takes what
// `ago` / `until` said, so 前 and 后 are written in exactly two keys
// (`time-ago`, `time-in`). The `[cli]` sentences that do carry it are the
// terminal's, and the face reaches them only through here
// (`readingAge`). `check-words` holds all of this.
//
// M / H / D are not here: they are for names and tight labels (「5H 窗
// 口」, an axis), never for an age or a deadline.

/** A moment in the past, said with its direction: 「3 分钟前」. */
export function ago(atMs: number): string {
  return agoOf(Date.now() - atMs);
}

/** An age (how long ago, in ms) said with its direction: 「3 分钟前」.
    For the answers that carry an age rather than a moment. */
export function agoOf(ageMs: number): string {
  return gui.time_ago(span(ageMs));
}

/**
 * How old an answer is — **or nothing, while that is not worth saying**.
 *
 * Takes an age rather than a moment, because the callers that have one
 * (a reading's `age_ms`) have exactly that.
 *
 * `null` for anything still measured in seconds, and **the threshold is
 * not a new judgment**: it is `span`'s own first rung. The ladder had
 * already decided that under a minute is the seconds tier, and a machine
 * that answered four seconds ago answered *now* — the star map printed
 * `0 秒前的读数` under a seat that had just replied, which reads as a
 * fault rather than as freshness. Past the rung the age starts carrying
 * something (khor has missed a round it meant to take), so it appears,
 * and it appears in the same words everything else uses for an age.
 *
 * The caller decides what silence means for its own picture; on the map
 * it means the caption is one line shorter for every machine that is
 * answering, which is most of them, most of the time.
 */
export function ageIfOld(ageMs: number): string | null {
  return ageMs < SECONDS_TIER ? null : agoOf(ageMs);
}

/** A reading's age as the map's caption says it (「3 分钟前的读数」), or
    nothing while it is fresh (`ageIfOld`). The sentence is the
    terminal's own (`[cli] vitals-taken` carries its 前), so it is said
    here and nowhere else. */
export function readingAge(ageMs: number): string | null {
  return ageMs < SECONDS_TIER ? null : cli.vitals_taken(span(ageMs));
}

/** The first rung of `span`'s ladder — where seconds become minutes. */
const SECONDS_TIER = 60_000;

/** The quiet note (词的保鲜期批): how long an unvouched live word has
    said nothing, in the catalog's sentence. The duration is the word's
    own age — the same clock the CLI reads. */
export function quietFor(atMs: number): string {
  return cli.quiet_for(span(Date.now() - atMs));
}

/**
 * A moment ahead, said with its direction and to two units:
 * 「2 小时 13 分后」, 「3 天 5 小时后」, 「40 分钟后」.
 *
 * **Never rounded down to one unit** (the owner: 「重置如2小时13分后重
 * 置」): one unit floored made 23 h 59 min say 「23 小时」 and 1 day 23 h
 * say 「1 天」.
 */
export function until(atMs: number): string {
  return gui.time_in(pair(atMs - Date.now()));
}

/**
 * The same two units **without the direction** — 「2 小时 14 分」「3 天 4
 * 小时」 — where something else on screen already says which way: the
 * quota account page's 「5H　↻ 2 小时 14 分」, whose ↻ glyph carries it
 * (its aria-label keeps 「…后重置」; the owner, 2026-10-08). Never trim
 * 后 off `until`: that is a second spelling of the direction, and the
 * words gate reads for it.
 */
export function untilBare(atMs: number): string {
  return pair(atMs - Date.now());
}

/**
 * How long something has been going on or lasted — a **duration**, bare:
 * 「4 小时」. A state held in a row's bottom-right corner, a process's run
 * time, a wait. Never an age or a deadline: those say which way
 * (`ago`, `until`).
 */
export function lasting(ms: number): string {
  return span(ms);
}

/**
 * How long a stretch of the day strip lasted, to the minute: hours
 * *and* minutes, in the same words `lasting` uses.
 *
 * **Two units, where every other duration here gets one**, because this
 * one is printed beside its own two ends (`DayStrip`): 「10:05–11:55 ·
 * 1 小时」 reads as arithmetic that does not add up, and the reader is
 * holding both clocks. The ladder's one-unit rule is right for an age —
 * 「3 小时前」 needs no minutes — and wrong for a span whose endpoints
 * are on the same line. Never under a minute: a strip's smallest stretch
 * is one cell, and a cell is at least one (`use-day-strips`).
 */
export function stretch(ms: number): string {
  const m = Math.max(1, Math.round(ms / 60_000));
  if (m < 60) return count.minutes(m);
  const h = Math.floor(m / 60);
  const rest = m % 60;
  // The minutes rule above: second after hours, 分.
  return rest === 0 ? count.hours(h) : gui.time_hours_minutes(h, rest);
}

/** A duration in the coarsest unit that still says something. **Not
    exported**: a bare number of hours is the ambiguity this section
    exists to end — a page asks for `ago`, `until` or `lasting`. */
function span(ms: number): string {
  const s = Math.max(0, Math.floor(ms / 1000));
  if (s < 60) return count.seconds(s);
  const m = Math.floor(s / 60);
  if (m < 60) return count.minutes(m);
  const h = Math.floor(m / 60);
  if (h < 24) return count.hours(h);
  return count.days(Math.floor(h / 24));
}

/** A time ahead to two units where the second matters: 「2 小时 13 分」,
    「3 天 4 小时」; alone, 「45 分钟」「2 小时」「3 天」. Whole minutes,
    never under one (a reset is not said in seconds); the pair's words
    are the catalog's (`time-hours-minutes`, `time-days-hours`; the
    lead's pick, 2026-10-08, with the 额度 lab's ux-q2). */
function pair(ms: number): string {
  const m = Math.max(1, Math.floor(ms / 60_000));
  if (m < 60) return count.minutes(m);
  const h = Math.floor(m / 60);
  if (h < 24) return m % 60 === 0 ? count.hours(h) : gui.time_hours_minutes(h, m % 60);
  const d = Math.floor(h / 24);
  return h % 24 === 0 ? count.days(d) : gui.time_days_hours(d, h % 24);
}

// ── Other units (WORDS-1008) ─────────────────────────────────────────

/** A latency: always 「23 ms」, never 毫秒 (the owner, 2026-10-05). The
    one place the face words one. */
export function latency(ms: number): string {
  return gui.row_rtt(Math.round(ms));
}

/**
 * **Money, symbol first** (WORDS-1008: 「$9.21 / $20」「$457 / $1,000」
 * 「¥12K」), the one formatter for every figure the face shows in money.
 *
 * `unit` is whatever its caller holds and is **not interpreted**: the mark
 * an account's owner typed, or the code an endpoint reported (the quota
 * page's rule — a label, never a rate; nothing here converts). A mark
 * that is a symbol stands before the figure, touching it (`$9.21`); one
 * that starts with a letter is a word and follows it with a space
 * (`124 USDT`), since 「USDT124」 reads as one token. No unit, the bare
 * figure.
 *
 * The figure: whole amounts whole (`$20`), others to the cent under 100
 * (`$9.21`) and to the unit from 100 (`$457`), thousands grouped
 * (`$1,000`).
 */
export function money(n: number, unit: string | null): string {
  return dress(amount(n), unit);
}

/**
 * **An endpoint's ISO 4217 code, as the mark `money` wears** (WORDS-1008,
 * 「钱的符号在前」): `USD` → `$`, `CNY` → `¥`, `EUR` → `€`, `GBP` → `£`,
 * `JPY` → `¥`. Any other code stays the code, and `money` puts a mark that
 * starts with a letter after the figure (`12.40 CHF`). For a currency the
 * provider names (the subscription's extra usage, `QuotaCredits.currency`);
 * an account's own unit is the person's mark and never passes through here.
 * ⚠️ 墓碑 (ux-q3): the code itself as the mark, 「12.40 USD / 50 USD」.
 */
export function markOfCode(code: string | null): string | null {
  if (!code) return null;
  return CODE_MARK[code.trim().toUpperCase()] ?? code;
}

const CODE_MARK: Record<string, string> = { USD: "$", CNY: "¥", EUR: "€", GBP: "£", JPY: "¥" };

/**
 * Money in at most `room` characters — a ring's middle (`RING_CHARS`): the
 * shortest true form that fits. Whole units (`$124`), then the token
 * count's K / M / B (`¥12K`, `$1.2K`; `lib/tokens`' letters), then without
 * its decimal (`$1K`) — and when even that does not fit, **the mark goes,
 * never the digits**: a free-text mark (`USDT`) can be longer than the
 * whole room, and a reading of 124 must not be drawn as zero to keep it.
 */
export function moneyIn(n: number, unit: string | null, room: number): string {
  const whole = Math.abs(n) < 1000 ? `${Math.round(n)}` : null;
  const forms = [
    ...(whole === null ? [] : [dress(whole, unit), whole]),
    dress(glance(n, 1), unit),
    dress(glance(n, 0), unit),
    glance(n, 1),
    glance(n, 0),
  ];
  return forms.find((f) => f.length <= room) ?? forms[forms.length - 1];
}

function dress(figure: string, unit: string | null): string {
  if (!unit || unit.trim() === "") return figure;
  return /^\p{L}/u.test(unit) ? `${figure} ${unit}` : `${unit}${figure}`;
}

function amount(n: number): string {
  const whole = Number.isInteger(n) || Math.abs(n) >= 100;
  return n.toLocaleString("en-US", whole ? { maximumFractionDigits: 0 } : { minimumFractionDigits: 2, maximumFractionDigits: 2 });
}

/** K / M / B past a thousand, `d` decimals under ten of them. */
function glance(n: number, d: number): string {
  const steps = [["B", 1e9], ["M", 1e6], ["K", 1e3]] as const;
  for (const [mark, size] of steps) {
    if (Math.abs(n) >= size) {
      const v = n / size;
      return `${Math.abs(v) < 10 && d > 0 ? v.toFixed(d).replace(/\.0+$/, "") : Math.round(v)}${mark}`;
    }
  }
  return `${Math.round(n)}`;
}
