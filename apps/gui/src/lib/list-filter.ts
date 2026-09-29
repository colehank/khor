// What any list in a sidebar can be narrowed by: the ticks in the pane
// bar's filter menu, the chips in its box, and which rows they leave
// standing — **one mechanism, every list** (2026-09-29, the owner:
// 「所有的session list应该都可以filter吧？」).
//
// The sessions landing had this to itself, spelled twice (the options
// in `session-filter`, the sieve in `SessionsList`), and the 额度 landing
// grew a third copy beside its own rows. A fourth and a fifth for the
// machines and the workspaces would have been five gatherings of one
// idea, each free to come back with something the others do not — the
// failure `PaneBar`'s head names one control over. So the part that is
// the same for every list lives here, and a list only says **what its
// axes are**: their prefix, their heading, what a row answers on each,
// and the word for each answer.
//
// Every list with a filter reads it from here: 会话, 机器, 工作区 and
// 额度. 额度 was the last (2026-09-29): `QuotaLanding`'s `visibleQuota`
// and its options carried their own copy of the same two loops until
// then, behaving the same, so the move changed nothing on screen.
//
// **Nothing here decides what values exist.** The options are whatever
// the rows answer, in the order the rows came — which is the node's
// order — and a list's `pick` reads a fact the node sent, never a word
// (docs/UX.md 状态呈现).
import { type OmniAxis } from "@/components/Omnibox";
import { type FilterOption } from "@/components/PaneBar";
import { type Avatar } from "@/gen/bindings/Avatar";
import { axisOf, valueOf } from "@/words";

/**
 * One axis a list can be narrowed by.
 *
 * `key` is the prefix every ticked key on this axis carries (`state:`,
 * `dev:` …), so a stored tick says which axis it is on by itself.
 *
 * `pick` is what a row answers on this axis, **as the node's key** —
 * the state key, the machine, the system family — never the word on the
 * glass. `null` is 「this row says nothing on this axis」, which is not
 * the same as an empty answer: an empty string is a value (the node's
 * own key for 「could not tell」, `cat:` with nothing after it) and gets
 * an option; `null` gets none, and a row that answers `null` is never
 * kept by a tick on that axis. It is what lets an axis with one
 * meaningful value (a machine that is paused) appear only when a row
 * has it.
 */
export type ListAxis<R> = {
  key: string;
  /** The heading over this axis's options — in the menu and in the box. */
  heading: string;
  pick: (row: R) => string | null;
  /** The word for one value, looked up at paint time. */
  label: (value: string) => string;
};

/**
 * Which ticked values each axis wants, from the keys as stored.
 *
 * `keys` is every prefix the list's store can hold, which is **not**
 * always the axes on offer: the workspace column's session list offers
 * the state axis alone and still reads its store against all four, so
 * a `dev:` key in it can never be mistaken for a state word. A key with
 * no known prefix lands on the first (`words.axisOf` says why).
 */
function wantedOf(ticked: readonly string[], keys: readonly string[]): Map<string, Set<string>> {
  const wanted = new Map<string, Set<string>>();
  for (const key of ticked) {
    const axis = axisOf(key, keys);
    const set = wanted.get(axis) ?? new Set<string>();
    set.add(valueOf(key, keys));
    wanted.set(axis, set);
  }
  return wanted;
}

/**
 * The rows the ticks leave standing. **AND across axes, OR within one.**
 * Both halves are decisions, not defaults: ticking 忙碌 and 待批 means
 * 「either of these two」, while ticking 忙碌 and a machine means 「busy
 * ones on that machine」. The same on every list, so a hand that learned
 * it on one knows it on the next.
 *
 * Matched on the key the row answers (`pick`), never on the word drawn.
 * The box's typed words are not this function's: each list reads them
 * against what its rows say out loud (`SideList`'s `find`).
 */
export function sieve<R>(
  rows: readonly R[],
  ticked: readonly string[],
  axes: readonly Pick<ListAxis<R>, "key" | "pick">[],
  keys: readonly string[] = axes.map((a) => a.key),
): R[] {
  if (ticked.length === 0) return [...rows];
  const wanted = wantedOf(ticked, keys);
  const pickOf = new Map(axes.map((a) => [a.key, a.pick]));
  return rows.filter((r) => {
    for (const [axis, set] of wanted) {
      const pick = pickOf.get(axis);
      const on = pick ? pick(r) : "";
      if (on === null || !set.has(on)) return false;
    }
    return true;
  });
}

/**
 * The filter menu's choices: every value the rows answer, axis by axis,
 * in the order the rows came.
 *
 * **A ticked key stays on the list after its last row leaves**, or the
 * tick becomes impossible to undo — which also covers a tick restored
 * from storage before any row has arrived, the one case where the
 * ticked key is guaranteed to be missing from `rows`.
 */
export function listOptions<R>(
  rows: readonly R[],
  ticked: readonly string[],
  axes: readonly ListAxis<R>[],
  keys: readonly string[] = axes.map((a) => a.key),
): FilterOption[] {
  const out: FilterOption[] = [];
  for (const axis of axes) {
    const values: string[] = [];
    for (const r of rows) {
      const v = axis.pick(r);
      if (v !== null && !values.includes(v)) values.push(v);
    }
    for (const key of ticked) {
      if (axisOf(key, keys) === axis.key && !values.includes(valueOf(key, keys))) values.push(valueOf(key, keys));
    }
    for (const v of values) {
      out.push({ key: `${axis.key}${v}`, label: axis.label(v), axis: axis.heading });
    }
  }
  return out;
}

/**
 * The same options, grouped into the axes the search box offers.
 * **Derived from the options rather than gathered again**: the chips and
 * the menu's ticks are then not merely kept in step, they are one list
 * read two ways, and there is no second gathering that can come back
 * with something else.
 *
 * `face` gives a candidate the face it wears — a machine's, where the
 * value is a machine. `undefined` is 「this axis has no faces」 and
 * `null` is 「a machine with no face yet」; an invented one is worse
 * than either (Avatar.tsx's rule).
 */
export function listAxes(
  options: readonly FilterOption[],
  keys: readonly string[],
  face?: (key: string) => Avatar | null | undefined,
): OmniAxis[] {
  const byAxis = new Map<string, OmniAxis>();
  for (const o of options) {
    const axis = axisOf(o.key, keys);
    const seat = byAxis.get(axis) ?? { key: axis, label: o.axis ?? "", candidates: [] };
    seat.candidates.push({ key: o.key, label: o.label, face: face?.(o.key) });
    byAxis.set(axis, seat);
  }
  return [...byAxis.values()];
}
