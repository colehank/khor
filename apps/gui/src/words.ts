// Wording helpers on top of the generated catalog. The display word is
// looked up at the last moment; unknown keys echo — an old face must
// still render a session whose kind it has never heard of.
import { avatar, category, cli, gui, hop, state } from "./gen/catalog";

export function word(key: string): string {
  return (state as Record<string, string>)[key] ?? key;
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
    // reads.
    if (path === "") return gui.group_placeless;
    const cut = path.lastIndexOf("/");
    return cut < 0 || cut === path.length - 1 ? path : path.slice(cut + 1);
  }
  return group.startsWith("dev:") ? group.slice("dev:".length) : group;
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
export function axisOf(key: string): Axis {
  const found = AXES.find((a) => key.startsWith(a));
  return found ?? "state:";
}

/** The value inside a ticked key — the state word, machine name, or
    category name. An unprefixed key is the whole value (see `axisOf`). */
export function valueOf(key: string): string {
  const a = AXES.find((x) => key.startsWith(x));
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

/** A duration in the coarsest unit that still says something. */
function span(ms: number): string {
  const s = Math.max(0, Math.floor(ms / 1000));
  if (s < 60) return cli.age_seconds(s);
  const m = Math.floor(s / 60);
  if (m < 60) return cli.age_minutes(m);
  const h = Math.floor(m / 60);
  if (h < 24) return cli.age_hours(h);
  return cli.age_days(Math.floor(h / 24));
}
