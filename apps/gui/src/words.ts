// Wording helpers on top of the generated catalog. The display word is
// looked up at the last moment; unknown keys echo — an old face must
// still render a session whose kind it has never heard of.
import { avatar, category, cli, gui, hop, one, state } from "./gen/catalog";

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
 * deliberately not shared with the state table — `quiet` and `unsure`
 * exist here and nowhere else — precisely so that reading one in the
 * wrong table echoes the key instead of quietly finding a real word and
 * painting it. Sharing a lookup would give that mistake back.
 */
export function hopWord(key: string): string {
  return (hop as Record<string, string>)[key] ?? key;
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
export function connectionWord(row: { me: boolean; sponsor: string | null; hop: string | null }): string | null {
  return connectionKeyWord(connectionKey(row));
}

/**
 * The same fact as a key, for the machines list's filter (2026-09-29):
 * `here` for this machine, `via:<sponsor>` for a satellite, the hop key
 * otherwise, and `""` where `connectionWord` has no word. **The word is
 * looked up from the key** (`connectionKeyWord`), so a tick in the
 * filter and the word on the row cannot come apart — there is one
 * derivation, which is this one. The prefixes cannot collide: a hop key
 * is one kebab word (`khor_core::Hop`), never `here` and never with a
 * colon in it.
 */
export function connectionKey(row: { me: boolean; sponsor: string | null; hop: string | null }): string {
  if (row.me) return "here";
  if (row.sponsor) return `via:${row.sponsor}`;
  return row.hop ?? "";
}

/** The word for a `connectionKey`, `null` for the empty one. */
export function connectionKeyWord(key: string): string | null {
  if (key === "here") return gui.machine_here;
  if (key.startsWith("via:")) return cli.via_sponsor(key.slice("via:".length));
  if (key !== "") return hopWord(key);
  return null;
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

/** How long ago, in the catalog's units. */
export function ago(ms: number): string {
  return span(Date.now() - ms);
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
 * and it appears in the same words everything else uses for a duration.
 *
 * The caller decides what silence means for its own picture; on the map
 * it means the caption is one line shorter for every machine that is
 * answering, which is most of them, most of the time.
 */
export function ageIfOld(ageMs: number): string | null {
  return ageMs < SECONDS_TIER ? null : span(ageMs);
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
 * How long until a moment, in the same words `ago` uses for how long
 * since one.
 *
 * **The ladder is shared rather than written twice.** Two copies would
 * be two places to change 「分钟」, and the direction is the only thing
 * that differs — which is the argument, not the vocabulary.
 */
export function until(ms: number): string {
  return span(ms - Date.now());
}

/**
 * How long something has been going on — the same ladder `ago` and
 * `until` use, with no direction attached.
 *
 * **Shared rather than written a fourth time.** 「分钟」 lives in one
 * place; what differs between these callers is which two moments they
 * subtract, and that is the argument.
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
  return rest === 0 ? count.hours(h) : `${count.hours(h)} ${count.minutes(rest)}`;
}

/** A duration in the coarsest unit that still says something. */
function span(ms: number): string {
  const s = Math.max(0, Math.floor(ms / 1000));
  if (s < 60) return count.seconds(s);
  const m = Math.floor(s / 60);
  if (m < 60) return count.minutes(m);
  const h = Math.floor(m / 60);
  if (h < 24) return count.hours(h);
  return count.days(Math.floor(h / 24));
}
