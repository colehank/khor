// Presentation-only deadlines for Khor's own errands. Native navigation
// snapshots below have explicit outcomes; a deadline never means success.
import type { PageNavigationState } from "@/gen/bindings/PageNavigationState";

/** Stop a silent Khor errand's status after 30 seconds. This is only a
 * presentation deadline: it does not finish a native navigation or invent
 * an error, title, or progress estimate for a page. */
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

// Native snapshots are issued by the view/navigation owner. Delivery and
// initial snapshot queries may cross, so acceptance is by issued identity,
// never by matching URL or by an arrival-time loading heuristic.
export type NativeLoads = ReadonlyMap<string, PageNavigationState>;
const decimal = (value: string): bigint | null => /^[1-9][0-9]{0,19}$/.test(value) ? BigInt(value) : null;
const terminal = (phase: PageNavigationState["phase"]): boolean => phase === "finished" || phase === "failed" || phase === "cancelled" || phase === "settled";

export function nativeReceived(loads: NativeLoads, state: PageNavigationState): NativeLoads {
  const revision = decimal(state.revision), view = decimal(state.view);
  if (revision === null || view === null || (state.navigation !== "" && decimal(state.navigation) === null)) return loads;
  const was = loads.get(state.tab);
  if (was) {
    if (revision <= BigInt(was.revision) || view < BigInt(was.view)) return loads;
    if (state.view === was.view && state.navigation !== "" && was.navigation !== "") {
      if (BigInt(state.navigation) < BigInt(was.navigation)) return loads;
      if (state.navigation === was.navigation && terminal(was.phase)) return loads;
    }
  }
  if (state.estimate !== null && (!Number.isFinite(state.estimate) || state.estimate < 0 || state.estimate > 1)) return loads;
  if (state.phase === "finished" ? state.estimate !== 1 : state.estimate === 1) return loads;
  const next = new Map(loads); next.set(state.tab, state); return next;
}

export function nativeBusy(state: PageNavigationState | undefined | null): boolean {
  return state?.phase === "opening" || state?.phase === "started" || state?.phase === "committed";
}
