// The session's 附栏 (UX-30): whether it is open beside this session, how
// wide it was dragged, and which seat it is on.
//
// **Open belongs to the session in front, and to nothing else** (UX-46,
// the owner 2026-10-07: 「这个侧栏应该是只属于 session，该关闭的」): one
// session at most has it open, leaving that session — another landing,
// another tab, the session closed — closes it (`App`), and coming back
// finds it closed; the press opens it again, on 最近 (UX-40 §3). ⚠️ 墓碑
// (UX-30 → UX-46): 「open」 remembered per session on this device, so a
// session someone had docked a phone beside came back docked, and the
// column outlived its session on every other landing.
//
// **The width is remembered per session** (`localStorage`, beside
// `use-list-width`'s and `use-day-shown`'s keys): a viewer's own
// convenience, never a fact about the session. Storage can be absent or
// throwing (a private window, blocked site data), and every touch of it
// is wrapped: unreadable reads as the token's default, and a refused
// write costs only the memory. The seat lives as long as the app does
// and every opening resets it (UX-40 §3), so it is not written down.
//
// A store outside React with `useSyncExternalStore`: the front tab's
// press (`App`'s tab faces) and the column that answers it (`DetailPane`)
// read the same session's state, and both must turn on one press.
import { useSyncExternalStore } from "react";

/** The five seats of the 附栏, in the order its title's menu lists them
    (`SeatTitle`, UX-39 §2). */
export type SideSeat = "recent" | "workspace" | "browser" | "machine" | "quota";
export const SIDE_SEATS: readonly SideSeat[] = ["recent", "workspace", "browser", "machine", "quota"];

export type SidePref = {
  open: boolean;
  /** The width it was dragged to; `null` = the token's default. */
  w: number | null;
  seat: SideSeat;
};

const KEY = (session: string) => `khor.session-side.${session}`;
const CLOSED: SidePref = { open: false, w: null, seat: "recent" };

function stored(session: string): SidePref {
  try {
    const raw = window.localStorage.getItem(KEY(session));
    if (!raw) return CLOSED;
    const p: unknown = JSON.parse(raw);
    if (typeof p !== "object" || p === null) return CLOSED;
    // An `open` or a `seat` written before UX-46 is read past: closed, 最近.
    const { w } = p as Partial<SidePref>;
    return { ...CLOSED, w: typeof w === "number" && Number.isFinite(w) ? w : null };
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

/** The one session whose 附栏 is open, if any. */
let openFor: string | null = null;

export function setSessionSide(session: string, patch: Partial<SidePref>): void {
  // Opening one closes any other: the 附栏 is the session in front's.
  if (patch.open === true && openFor !== null && openFor !== session) {
    live.set(openFor, { ...current(openFor), open: false });
  }
  if (patch.open !== undefined) openFor = patch.open ? session : openFor === session ? null : openFor;
  const next = { ...current(session), ...patch };
  live.set(session, next);
  if (patch.w !== undefined) {
    try {
      window.localStorage.setItem(KEY(session), JSON.stringify({ w: next.w }));
    } catch {
      /* the column still moves; only the memory of it is lost */
    }
  }
  for (const tell of heard) tell();
}

/** The session whose 附栏 is open, or `null`. */
export function sessionSideOpen(): string | null {
  return openFor;
}

/** Closes the 附栏, whichever session it was open beside (`App`, when
    that session is left). */
export function closeSessionSide(): void {
  if (openFor !== null) setSessionSide(openFor, { open: false });
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
