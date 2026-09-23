// When a browsing tab counts as still loading (docs/SHELL.md 第十二轮:
// 「浏览器这, 如果页面在加载, 左侧的侧边栏应该有状态动效」).
//
// **A file with no imports, and that is the point.** The rule has one
// part that cannot be measured through the product: a load that starts
// and never finishes has to stop breathing after half a minute, and no
// gate in this repo can produce that — the events come from the app's
// own child webview (`page:load`), and every gate drives the face in
// Chrome. So the rule lives here as plain functions over plain data with
// the clock handed in, `use-loading.ts` wires it to React and to the
// app's events, and `check-tabs.mjs` runs *this* with a clock of its own.
//
// What is therefore still 未量: that the app really emits `page:load`
// with these two words, and that the hook subscribes. Those are read as
// source, one line each, and nothing measures them running.

/**
 * How long a row may breathe with nothing more heard about it.
 *
 * **A page can load forever** — a socket that stays open, a site that
 * answers headers and then stalls — and a row that breathes forever is
 * a row that has stopped meaning anything. Half a minute is long enough
 * that a slow site finishes inside it and short enough that nobody sits
 * watching a lie. Running out is not an error and says nothing about the
 * page: the title does not change, nothing is reported, the row simply
 * goes quiet (第十二轮 第 2 条:「灭了不改标题、不报错」).
 */
export const CEILING_MS = 30_000;

/** Which tabs are loading, each with the moment it began. */
export type Loads = ReadonlyMap<string, number>;

/** Nothing is loading. */
export const NONE: Loads = new Map<string, number>();

/**
 * A load began for this tab, at `now`.
 *
 * **A second start restarts the clock** rather than being ignored: a
 * page that navigates again is a new load, and the ceiling is about how
 * long since the last thing was heard, not since the first.
 */
export function started(loads: Loads, id: string, now: number): Loads {
  const next = new Map(loads);
  next.set(id, now);
  return next;
}

/** It finished (or khor stopped waiting for it). */
export function ended(loads: Loads, id: string): Loads {
  if (!loads.has(id)) return loads;
  const next = new Map(loads);
  next.delete(id);
  return next;
}

/**
 * Whether this tab is still loading, as of `now` — the ceiling counted
 * in rather than swept, so a row goes quiet on time even if nothing has
 * swept the map yet.
 */
export function loading(loads: Loads, id: string, now: number): boolean {
  const since = loads.get(id);
  return since !== undefined && now - since < CEILING_MS;
}

/**
 * The map with everything past the ceiling dropped.
 *
 * **The same object back when nothing expired**, so a caller can hand
 * the result straight to a setter without a render every beat.
 */
export function alive(loads: Loads, now: number): Loads {
  const dead = [...loads].filter(([, since]) => now - since >= CEILING_MS);
  if (dead.length === 0) return loads;
  const next = new Map(loads);
  for (const [id] of dead) next.delete(id);
  return next;
}
