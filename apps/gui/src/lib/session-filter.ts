// What a pane full of sessions can be searched, filtered and arranged
// by — **built in one place because two panes draw it now** (工作区批⑩).
//
// The sessions landing had all of this inside `App`, which was fine
// while it was the only list of sessions in the app. 「工作区的 session
// list 是全功能的,和 session 页的 list 一致,可以搜索、新建和 filter」
// (the user, 2026-09-15) makes it two, and a second gathering of the
// same options is a second gathering that can come back with something
// else — the failure `PaneBar`'s own head names one control over.
//
// **Nothing here decides what states exist.** The options are whatever
// the node put on the rows; this layer only groups them and hands them
// their words (docs/UX.md 状态呈现).
import { type DeviceRow, type SessionRow } from "@/api";
import { type OmniAxis } from "@/components/Omnibox";
import { type FilterOption } from "@/components/PaneBar";
import { gui } from "@/gen/catalog";
import { AXES, type Axis, axisOf, groupLabel, placeLabels, valueOf, word } from "@/words";

/**
 * How to arrange the session list. The keys are the node's
 * (`khor_node::list::Arrange`) and the CLI's `--by` values — spelled
 * once there, echoed here, so the two faces cannot drift into different
 * vocabularies for one setting.
 */
/// ⚠️ **A function, not a `const`** (批 D3): the labels are catalog
/// reads, and a module-level constant would keep whichever language the
/// page loaded in for the life of the tab.
export const arrange = () =>
  [
    { key: "recent", label: gui.by_recent },
    { key: "category", label: gui.by_category },
    { key: "device", label: gui.by_device },
    { key: "state", label: gui.by_state },
    { key: "place", label: gui.by_place },
  ] as const;

/** The keys on their own, for everything that does not paint a label.
    Separate so a lookup by key costs no wording at all. */
export const ARRANGE_KEYS = ["recent", "category", "device", "state", "place"] as const;

/** WeChat's opening: the thing you touched last is the thing you want. */
export const ARRANGE_DEFAULT = "recent";
const ARRANGE_KEY = "khor.sessions.arrange";

/**
 * The chosen arrangement, remembered on this device.
 *
 * **Local on purpose, unlike a pin.** A pin is a property of the session
 * it is stuck to, so it travels the network; how one screen sorts is
 * that screen's posture, and a phone held in a queue does not want the
 * desktop's answer (docs/handoff 置顶与分类 — the same split that keeps
 * collapse state local).
 *
 * An unreadable or unknown stored value falls back to the default rather
 * than being passed on: the backend refuses a mode it does not know, so
 * a stale key would leave the list empty with an error nobody asked for.
 */
export function storedArrange(): string {
  try {
    const saved = window.localStorage.getItem(ARRANGE_KEY);
    return ARRANGE_KEYS.some((k) => k === saved) ? saved! : ARRANGE_DEFAULT;
  } catch {
    return ARRANGE_DEFAULT;
  }
}

/** Remember it, and go on applying it either way: storage refusing (a
    private window, a full quota) costs the memory of the choice and
    nothing else. */
export function rememberArrange(key: string): void {
  try {
    window.localStorage.setItem(ARRANGE_KEY, key);
  } catch {
    /* the list still arranges; only the memory of it is lost */
  }
}

/**
 * The ticked filter words, remembered on this device — **one entry per
 * pane**, which is why the key carries the pane's name.
 *
 * Same mechanism and same reasoning as the arrangement above: a
 * preference the user set once should still be set the next time
 * (docs/handoff — 所有偏好保留一次设置).
 *
 * **Not validated against a list of known words.** This layer does not
 * get to say which states exist (docs/UX.md 状态呈现) — the options come
 * from what the node sent — so all that can be checked here is the
 * shape. A key that no longer means anything simply matches no rows,
 * which the pane says out loud (`no_matches`) with the filter control
 * lit, and one tick undoes it.
 */
const FILTER_KEY = "khor.filter";
export const filterKey = (pane: string) => `${FILTER_KEY}.${pane}`;

export function storedWords(pane: string): string[] {
  try {
    const raw = window.localStorage.getItem(filterKey(pane));
    if (!raw) return [];
    const parsed: unknown = JSON.parse(raw);
    return Array.isArray(parsed) && parsed.every((k) => typeof k === "string")
      ? (parsed as string[])
      : [];
  } catch {
    return [];
  }
}

export function rememberWords(pane: string, words: string[]): void {
  try {
    window.localStorage.setItem(filterKey(pane), JSON.stringify(words));
  } catch {
    /* the list still filters; only the memory of it is lost */
  }
}

/** Every axis there is, which is what the sessions landing offers.
    A workspace's column offers `["state:"]` alone — see `sessionOptions`. */
export const ALL_AXES = AXES;

/**
 * The filter's choices: the keys the node actually sent, in the order it
 * sent them — which is its ranking, so they come out in the order that
 * matters. Enumerating the six words here instead would be this layer
 * deciding what states exist, and it does not get to (docs/UX.md
 * 状态呈现).
 *
 * **A ticked key stays on the list after its last row leaves**, or the
 * tick becomes impossible to undo — which also covers a tick restored
 * from storage before any row has arrived, the one case where the ticked
 * key is guaranteed to be missing from `rows`.
 *
 * **The axes are the node's own** (`khor_node::list` groups by exactly
 * these). A ticked key is a group key, so a filter and a heading are the
 * same string and cannot drift apart.
 *
 * The machine axis reads `home` rather than `source`: `source` is the
 * offline axis and is absent on rows that live here, so an axis built on
 * it would be missing this machine — the one people reach for first
 * (`SessionRow::home` says the rest).
 */
export function sessionOptions(
  rows: SessionRow[],
  ticked: string[],
  axes: readonly Axis[] = ALL_AXES,
): FilterOption[] {
  // The place chips, told apart where two checkouts share a name — the
  // same reader the list's headings use, so a chip and the heading it
  // filters to always print the same words.
  const placeNames = placeLabels(rows.map((r) => `at:${r.place ?? ""}`));
  const pick: Record<Axis, (r: SessionRow) => string> = {
    "state:": (r) => r.word,
    "dev:": (r) => r.home,
    "cat:": (r) => r.category ?? "",
    "at:": (r) => r.place ?? "",
  };
  const label: Record<Axis, (v: string) => string> = {
    "state:": word,
    "dev:": (v) => v,
    "cat:": (v) => groupLabel(`cat:${v}`),
    "at:": (v) => placeNames.get(`at:${v}`) ?? groupLabel(`at:${v}`),
  };
  const heading: Record<Axis, string> = {
    "state:": gui.axis_state,
    "dev:": gui.devices_tab,
    "cat:": gui.axis_category,
    "at:": gui.axis_place,
  };
  const out: FilterOption[] = [];
  for (const axis of axes) {
    const values: string[] = [];
    for (const r of rows) {
      const v = pick[axis](r);
      if (!values.includes(v)) values.push(v);
    }
    for (const key of ticked) {
      if (axisOf(key) === axis && !values.includes(valueOf(key))) values.push(valueOf(key));
    }
    for (const v of values) {
      out.push({ key: `${axis}${v}`, label: label[axis](v), axis: heading[axis] });
    }
  }
  return out;
}

/**
 * The same options, grouped into the axes the search box offers.
 * **Derived from the options rather than gathered again**: the chips and
 * the menu ticks are then not merely kept in step, they are one list
 * read two ways, and there is no second gathering that can come back
 * with something else.
 */
export function sessionAxes(options: FilterOption[], devices: DeviceRow[] = []): OmniAxis[] {
  const byAxis = new Map<string, OmniAxis>();
  for (const o of options) {
    const axis = axisOf(o.key);
    const seat = byAxis.get(axis) ?? { key: axis, label: o.axis ?? "", candidates: [] };
    seat.candidates.push({
      key: o.key,
      label: o.label,
      // A machine wears its face here as it does everywhere else; the
      // other axes have no face to wear, and an invented one is worse
      // than none (Avatar.tsx's rule).
      face:
        axis === "dev:"
          ? (devices.find((d) => d.name === valueOf(o.key))?.face ?? null)
          : undefined,
    });
    byAxis.set(axis, seat);
  }
  return [...byAxis.values()];
}
