// The session's 附栏 (UX-30): whether it is open beside this session, how
// wide it was dragged, and which seat it is on.
//
// **Open is remembered per session, for the app's lifetime** (fix-e, the
// owner 2026-10-08: 「要记住每一个药丸的侧边展开情况, 从一个没有展开的到
// 展开的, 要自动展开」): each pill keeps its own open or closed, in memory
// only — the same lifetime as where it was left (UX-47, below). The column
// is drawn for the session in front, on the sessions landing, when that
// session's own memory says open (`App`): another tab in front shows that
// tab's memory, and another landing draws none, leaving each memory as it
// was. The press, the seam and closing the tab are what close one (the tab
// closed forgets it, `keepSidesOf`). ⚠️ 墓碑 (UX-46 → fix-e): one session
// at most had it open, and leaving that session — another landing,
// another tab — closed it, so coming back found it closed. ⚠️ 墓碑 (UX-30
// → UX-46): 「open」 written to this device's storage, so a session came
// back docked after a restart.
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

/** The six seats of the 附栏: the seat bar's first three, then the three
    under its 更多 (`SeatBar`, UX-45; 存储, STORE-BRIEF phase 1). */
export type SideSeat = "recent" | "workspace" | "browser" | "machine" | "quota" | "store";
export const SIDE_SEATS: readonly SideSeat[] = ["recent", "workspace", "browser", "machine", "quota", "store"];

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

export function setSessionSide(session: string, patch: Partial<SidePref>): void {
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

/** **A pill closed takes its 附栏's open with it** (`App`, as the tabs
    change): every session remembered open that is not among `tabs` is
    closed, so the same session opened again as a tab starts closed. Where
    it was left (`sidePlace`) stays, as it always has. */
export function keepSidesOf(tabs: readonly string[]): void {
  const gone = [...live].filter(([s, p]) => p.open && !tabs.includes(s)).map(([s]) => s);
  for (const s of gone) live.set(s, { ...current(s), open: false });
  if (gone.length) for (const tell of heard) tell();
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
    file open over it. ⚠️ 墓碑 (STORE-BRIEF phase 1): `drops`, 「放进来的」
    open as a tree of its own — received files are 存储's now. */
export type WsSpot = { file: string | null };

/** Where 存储 stands (`StorePage`): the machine shown, by its API name,
    and the file open over its list. */
export type StoreSpot = { machine: string; file: string | null };

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
  /** 存储: the machine and the file open, as it last said. */
  store: StoreSpot | null;
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

/** The file a place shows, if any. */
export function placeFile(spot: WsSpot | null): string | null {
  return spot === null ? null : spot.file;
}
