// The session's 附栏 (UX-30): whether it is open beside this session, how
// wide it was dragged, and which seat it is on — one memory per session.
//
// **Per session, not per face** (the owner: 「一边 vibe 一边看交付物」 —
// the deliverable is this session's, and the column is where it is
// watched from, so a session someone docked a phone beside keeps it
// docked, and the next session starts without one). A session's seat is
// remembered the same way the old pill sheet kept its tab (`keptTab`,
// which this replaces): 最近 first, then whichever was last looked at.
//
// **Kept on this device** (`localStorage`, beside `use-list-width`'s and
// `use-day-shown`'s keys): a viewer's own convenience, never a fact about
// the session. Storage can be absent or throwing (a private window,
// blocked site data), and every touch of it is wrapped: unreadable reads
// as 「never opened」, and a refused write costs only the memory.
//
// A store outside React with `useSyncExternalStore`: the front tab's
// press (`App`'s tab faces) and the column that answers it (`DetailPane`)
// read the same session's state, and both must turn on one press.
import { useSyncExternalStore } from "react";

/** The five seats of the column's capsule, in the capsule's order. */
export type SideSeat = "recent" | "workspace" | "browser" | "machine" | "quota";
export const SIDE_SEATS: readonly SideSeat[] = ["recent", "workspace", "browser", "machine", "quota"];

export type SidePref = {
  open: boolean;
  /** The width it was dragged to; `null` = the token's default. */
  w: number | null;
  seat: SideSeat;
  /** Whether the ruler is expanded into the full day (`DayRuler`). */
  day: boolean;
};

const KEY = (session: string) => `khor.session-side.${session}`;
const CLOSED: SidePref = { open: false, w: null, seat: "recent", day: false };

function isSeat(v: unknown): v is SideSeat {
  return typeof v === "string" && (SIDE_SEATS as readonly string[]).includes(v);
}

function stored(session: string): SidePref {
  try {
    const raw = window.localStorage.getItem(KEY(session));
    if (!raw) return CLOSED;
    const p: unknown = JSON.parse(raw);
    if (typeof p !== "object" || p === null) return CLOSED;
    const { open, w, seat, day } = p as Partial<SidePref>;
    return {
      open: open === true,
      w: typeof w === "number" && Number.isFinite(w) ? w : null,
      seat: isSeat(seat) ? seat : "recent",
      day: day === true,
    };
  } catch {
    return CLOSED;
  }
}

/** What is in memory, by session: one object per session that has been
    read, so `useSyncExternalStore`'s snapshot is stable between writes. */
const live = new Map<string, SidePref>();
const heard = new Set<() => void>();

function current(session: string): SidePref {
  let p = live.get(session);
  if (!p) {
    p = stored(session);
    live.set(session, p);
  }
  return p;
}

export function setSessionSide(session: string, patch: Partial<SidePref>): void {
  const next = { ...current(session), ...patch };
  live.set(session, next);
  try {
    window.localStorage.setItem(KEY(session), JSON.stringify(next));
  } catch {
    /* the column still moves; only the memory of it is lost */
  }
  for (const tell of heard) tell();
}

function listen(tell: () => void): () => void {
  heard.add(tell);
  return () => {
    heard.delete(tell);
  };
}

/** One session's column, as remembered. `null` session (nothing open)
    reads as closed. */
export function useSessionSide(session: string | null): SidePref {
  return useSyncExternalStore(
    listen,
    () => (session === null ? CLOSED : current(session)),
    () => CLOSED,
  );
}
