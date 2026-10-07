// The session's 附栏 (UX-30): whether it is open beside this session, how
// wide it was dragged, and which seat it is on.
//
// **Open belongs to the session in front, and to nothing else** (UX-46,
// the owner 2026-10-07: 「这个侧栏应该是只属于 session，该关闭的」): one
// session at most has it open, leaving that session — another landing,
// another tab, the session closed — closes it (`App`), and coming back
// finds it closed; the press opens it again where it was left (UX-47,
// below). ⚠️ 墓碑 (UX-30 → UX-46): 「open」 remembered per session on
// this device, so a session someone had docked a phone beside came back
// docked, and the column outlived its session on every other landing.
//
// **The width is remembered per session** (`localStorage`, beside
// `use-list-width`'s and `use-day-shown`'s keys): a viewer's own
// convenience, never a fact about the session. Storage can be absent or
// throwing (a private window, blocked site data), and every touch of it
// is wrapped: unreadable reads as the token's default, and a refused
// write costs only the memory.
//
// **Where it was left is remembered per session, in memory only**
// (UX-47, the owner 2026-10-07, replacing UX-40 §3's 「每次打开都是显示
// 最近」): the seat and what was open in it (`SidePlace`), kept by the
// shell while the 附栏 is open and read back when it opens again. Never
// written down: after a restart, or a reload, it opens on 最近. ⚠️ 墓碑
// (UX-40 §3 → UX-47): every opening landing on 最近.
//
// A store outside React with `useSyncExternalStore`: the front tab's
// press (`App`'s tab faces) and the column that answers it (`DetailPane`)
// read the same session's state, and both must turn on one press.
import { useSyncExternalStore } from "react";

/** The five seats of the 附栏: the seat bar's first three, then the two
    under its 更多 (`SeatBar`, UX-45). */
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

/** Where the 工作区 seat stands inside its tree (`SideWorkspace`): the
    file open over it, and 「放进来的」 open as a tree of its own, with
    its own spot. */
export type WsSpot = { file: string | null; drops: WsSpot | null };

/** **Where the 附栏 was left** (UX-47): the seat, and what was open in
    it — the shell's own pieces of the 附栏's state, as they stood. */
export type SidePlace = {
  seat: SideSeat;
  /** The page was opened from 最近: its ‹ goes back there. */
  fromRecent: boolean;
  /** The 工作区 seat's tree: its machine (and whether that is this one),
      the directory it was pointed at, the file it was asked to open, and
      where it stood inside. */
  files: {
    machine: string;
    here: boolean;
    dir: string | null;
    start: { path: string; at: number } | null;
    spot: WsSpot | null;
  } | null;
  /** 浏览器: the page in front, or `null` for the list. */
  page: string | null;
  /** 额度: the list, or the account's page. */
  quota: "list" | "detail";
};

/** Not a store anyone renders from: written while the 附栏 is open,
    read once when it opens again. */
const places = new Map<string, SidePlace>();

export function keepSidePlace(session: string, place: SidePlace): void {
  places.set(session, place);
}

export function sidePlace(session: string): SidePlace | null {
  return places.get(session) ?? null;
}

/** The remembered thing is gone: the next opening lands on 最近. */
export function forgetSidePlace(session: string): void {
  places.delete(session);
}

/** Every session's place, for a change that may have taken one away. */
export function sidePlaces(): [string, SidePlace][] {
  return [...places];
}

/** The file a place shows, if any: inside 「放进来的」 its own. */
export function placeFile(spot: WsSpot | null): string | null {
  if (spot === null) return null;
  return spot.drops !== null ? placeFile(spot.drops) : spot.file;
}
