// The first row of the left block (docs/SHELL.md 第十三轮).
//
// **One block down the left, one row across its top.** The rail and the
// list are one material now (`--side`), and the row over both holds the
// three things the user put there: macOS's own buttons at the left end
// (placed by the window, `src-tauri/src/traffic.rs`; this row only
// keeps the width they stand in clear), **the one search box**, and the
// landing's 「+」. The window's own 38 band of 第十轮 is gone — 「不喜欢
// 现在的最上方搜索和状态. 浪费空间了」 — and what survived it is the
// search that expands in place with its hits hung under the box.
//
// **The box is the landing's, and it stands in the shell's row.** Every
// landing already draws its column's head — `PaneBar`: the box that
// narrows the list, the filter, the 「+」 — from inside its own column,
// where the gates and the smoke reach for it by the column's name. That
// head now *seats itself here* (`SideSeat`, a portal) rather than being
// rebuilt by the shell from state the shell does not hold: 額度's bar is
// built from a dozen things only 額度 knows. The shell owns the seat and
// what is global — ⌘K, the widening, the hits — and the landing owns
// what the box does to its column.
//
// **Two ranges, one box** (第 4 条): typing narrows this column, which
// is what the landing's box has always done; ⌘K anywhere, or ⇥ in the
// box, widens the same seat to everything (`Node::find`, `khor find` —
// one matching, one order), and a small word in the box says which
// range it is on. Esc, a press outside, or a pick puts the column's box
// back.
import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useId,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { createPortal } from "react-dom";

import { find as findAll, type FindHit } from "@/api";
import { IconSearch } from "@/components/icons";
import { Input } from "@/components/ui/input";
import { gui } from "@/gen/catalog";
import { useNarrow } from "@/hooks/use-narrow";
import { cn } from "@/lib/utils";

/**
 * Where a landing's head and its list column go: two seats the shell
 * holds, and a count of who is sitting in the first — the row draws a
 * search of its own only while nobody is.
 */
type Seats = {
  head: HTMLDivElement | null;
  list: HTMLDivElement | null;
  claim: () => () => void;
  /** The narrow detail screen's head that holds the row now — the
      deepest of those that asked (`NarrowHead`), or none. */
  top: string | null;
  claimHead: (id: string, depth: number) => () => void;
};

const SeatsContext = createContext<Seats | null>(null);

/** The seats, made once by the shell and handed down as context. */
export function useSideSeats() {
  const [head, setHead] = useState<HTMLDivElement | null>(null);
  const [list, setList] = useState<HTMLDivElement | null>(null);
  const [claims, setClaims] = useState(0);
  const claim = useCallback(() => {
    setClaims((n) => n + 1);
    return () => setClaims((n) => n - 1);
  }, []);
  const [heads, setHeads] = useState<{ id: string; depth: number; at: number }[]>([]);
  const order = useRef(0);
  const claimHead = useCallback((id: string, depth: number) => {
    order.current += 1;
    const at = order.current;
    setHeads((h) => [...h.filter((x) => x.id !== id), { id, depth, at }]);
    return () => setHeads((h) => h.filter((x) => x.id !== id));
  }, []);
  // The deepest wins; between two at one depth, the later one.
  const top =
    heads.length === 0
      ? null
      : heads.reduce((a, b) => (b.depth > a.depth || (b.depth === a.depth && b.at > a.at) ? b : a)).id;
  const seats = useMemo<Seats>(() => ({ head, list, claim, top, claimHead }), [head, list, claim, top, claimHead]);
  return { seats, setHead, setList, claimed: claims > 0 };
}

export const SideSeatsProvider = SeatsContext.Provider;

/**
 * A landing's head, seated in the first row. **Written where the head
 * is written** (inside the landing's `ListPane`), drawn where the row
 * is. With no shell around it — a head drawn on its own somewhere —
 * it stays where it was written.
 */
export function SideSeat({ children }: { children: ReactNode }) {
  const seats = useContext(SeatsContext);
  useLayoutEffect(() => seats?.claim(), [seats]);
  if (!seats) return <>{children}</>;
  // A narrow detail screen's head holds the row (`NarrowHead`): the
  // landing's box has no job on a screen that is not its list.
  if (!seats.head || seats.top !== null) return null;
  return createPortal(children, seats.head);
}

/**
 * How deep a narrow head sits: a screen inside a screen (a session
 * opened in a workspace's column) is one deeper than the workspace's
 * own, and the deeper head is the one the row shows.
 */
const HeadDepth = createContext(0);

/** Everything under this is one screen deeper than what wraps it. */
export function DeeperHeads({ children }: { children: ReactNode }) {
  const depth = useContext(HeadDepth);
  return <HeadDepth.Provider value={depth + 1}>{children}</HeadDepth.Provider>;
}

/** The row a seated head is laid out as: the way back at its left,
    the title taking what is left, the screen's own buttons at the
    right. `self-stretch` so the whole row's blank is the head's. */
export const HEAD_IN_ROW = "flex min-w-0 flex-1 items-center gap-in self-stretch pr-1";

/**
 * **A narrow detail screen's head, seated in the window's first row**
 * (10-layout 窄档: 「列表一屏 ⇄ 详情一屏，R0 左上角是返回」; 2026-09-28).
 *
 * On the narrow face a detail screen used to draw two bars: L0 holding
 * a magnifier with nothing to search, then its own `‹ title` bar under
 * it. The search has no job on a detail screen, and L0 is the row that
 * already does everything a head needs done — it stands clear of
 * macOS's three buttons (`--traffic`) and it moves the window (the
 * shell marks it once, `shell.ts`). So the head sits **in** it, the way
 * a landing's search box does on its list screen (`SideSeat`).
 *
 * `children` is told whether it is seated, because the same head is
 * drawn in place wherever there is no narrow shell around it — the wide
 * face's workspace column keeps its session's bar inside the column.
 * The deepest head asking wins the row (`DeeperHeads`).
 */
export function NarrowHead({ children }: { children: (seated: boolean) => ReactNode }) {
  const seats = useContext(SeatsContext);
  const narrow = useNarrow();
  const depth = useContext(HeadDepth);
  const id = useId();
  const seated = seats !== null && narrow;
  const claim = seats?.claim;
  const claimHead = seats?.claimHead;
  useLayoutEffect(() => {
    if (!seated || !claim || !claimHead) return;
    const off = claim();
    const offHead = claimHead(id, depth);
    return () => {
      off();
      offHead();
    };
  }, [seated, claim, claimHead, id, depth]);
  if (!seated) return <>{children(false)}</>;
  if (!seats.head || seats.top !== id) return null;
  return createPortal(
    <div data-screen-head className={HEAD_IN_ROW}>
      {children(true)}
    </div>,
    seats.head,
  );
}

/**
 * A landing's list column, seated in the block under the row — beside
 * the rail, on the block's ground — wherever the landing wrote it.
 */
export function SideListSeat({ children }: { children: ReactNode }) {
  const seats = useContext(SeatsContext);
  if (!seats) return <>{children}</>;
  if (!seats.list) return null;
  return createPortal(children, seats.list);
}

/**
 * 「Search everything」, asked for from inside a seated head — the
 * magnifier a column's box shrinks to when the block is too narrow for
 * it (`PaneBar`, 第十五轮 第 4 条). An event on the window rather than a
 * callback down the tree: the head is the landing's, written inside the
 * landing, and the one who can widen is this row.
 */
export const FIND_ALL = "khor:find-all";

/**
 * The ⌘K hint at a seated search field's right end (components/
 * SearchField): `caption` in `tertiary` on a `radius-xs` keycap. **The
 * first thing to go when the block is narrowed** — `@container/bar`
 * is the row the box stands in, so it hides before the placeholder's
 * words are cut, and on the narrow face (a phone has no ⌘) it is not
 * drawn at all.
 */
export function SearchHint({ children = "⌘K" }: { children?: ReactNode }) {
  return (
    <kbd
      data-search-scope
      className="pointer-events-none flex-none rounded-xs border px-xxs text-caption text-tertiary @max-3xs/bar:hidden"
    >
      {children}
    </kbd>
  );
}

/** The word a hit's kind is shown by — never the key itself. */
function kindWord(kind: string): string {
  switch (kind) {
    case "session":
      return gui.sessions_tab;
    case "machine":
      return gui.devices_tab;
    case "workspace":
      return gui.workspaces_tab;
    case "tab":
      return gui.browser_tab;
    default:
      return gui.settings;
  }
}

/**
 * Where the row's controls start: after macOS's three buttons where
 * there are any (`--traffic`, measured by the window), and otherwise on
 * the rail's own glyph box (`--side-inset`) — so the search box starts
 * where the rail's line is (第 2 条), on every face.
 */
const START = "max(var(--traffic), var(--side-inset))";

export function SideHead({
  narrow,
  bare,
  seatRef,
  claimed,
  onJump,
}: {
  narrow: boolean;
  /** Nothing to seat and no room to draw a search: a 「…」 page beside
      a rail alone, or a list put away. ⌘K still widens over it. */
  bare: boolean;
  seatRef: (el: HTMLDivElement | null) => void;
  /** Whether a landing's head is sitting in the seat. */
  claimed: boolean;
  /** A hit was chosen: go there. */
  onJump: (hit: FindHit) => void;
}) {
  const [open, setOpen] = useState(false);
  const [word, setWord] = useState("");
  const [hits, setHits] = useState<FindHit[]>([]);
  const [at, setAt] = useState(0);
  const box = useRef<HTMLInputElement>(null);
  const seat = useRef<HTMLDivElement>(null);

  const widen = useCallback((from: string) => {
    setWord(from);
    setAt(0);
    setOpen(true);
  }, []);

  // ⌘K anywhere in the app — the one shortcut this row owns. Bound on
  // the window rather than on the box: a person typing in a pane still
  // means 「search everything」 by it. **Esc on the window too**: the
  // box's own Esc is the ordinary way out, and this is the one that
  // still works when focus has wandered off the box (the smoke found
  // the hits still up over the list after an Esc that went elsewhere).
  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setOpen(false);
        return;
      }
      if (e.key.toLowerCase() !== "k" || !(e.metaKey || e.ctrlKey)) return;
      e.preventDefault();
      widen("");
    };
    const all = () => widen("");
    window.addEventListener("keydown", key);
    window.addEventListener(FIND_ALL, all);
    return () => {
      window.removeEventListener("keydown", key);
      window.removeEventListener(FIND_ALL, all);
    };
  }, [widen]);

  // ⇥ in the column's box widens it to everything, carrying the word
  // (第 4 条). **A DOM listener on the seat, not a React `onKeyDown`**:
  // the box is a portal's child here, and a portal's events bubble up
  // the React tree — to the landing that wrote the box — never through
  // this element. Bubble phase, so a box that took the ⇥ for itself
  // (the omnibox completing a candidate) has said so first.
  const [seatEl, setSeatEl] = useState<HTMLDivElement | null>(null);
  useEffect(() => {
    if (!seatEl) return;
    const key = (e: KeyboardEvent) => {
      if (e.key !== "Tab" || e.shiftKey || e.defaultPrevented) return;
      const t = e.target;
      if (!(t instanceof HTMLInputElement) || !t.matches("[data-pane-search]")) return;
      e.preventDefault();
      widen(t.value);
    };
    seatEl.addEventListener("keydown", key);
    return () => seatEl.removeEventListener("keydown", key);
  }, [seatEl, widen]);

  // **A press outside puts it away** — the half of 「就地展开」 a dialog
  // used to get free from its overlay, and the reason there is none:
  // the window behind stays live, so the only thing that should close
  // this is a press that was not meant for it.
  useEffect(() => {
    if (!open) return;
    const away = (e: PointerEvent) => {
      if (seat.current && !seat.current.contains(e.target as Node)) setOpen(false);
    };
    window.addEventListener("pointerdown", away);
    return () => window.removeEventListener("pointerdown", away);
  }, [open]);

  useEffect(() => {
    if (!open) {
      setWord("");
      setHits([]);
      setAt(0);
      return;
    }
    box.current?.focus();
  }, [open]);

  // The node answers the whole list; the order is its own, so this face
  // paints the rows and decides nothing (`Node::find`, `khor find`).
  useEffect(() => {
    if (!open) return;
    let live = true;
    findAll(word)
      .then((rows) => {
        if (!live) return;
        setHits(rows);
        setAt(0);
      })
      .catch(() => live && setHits([]));
    return () => {
      live = false;
    };
  }, [open, word]);

  const go = (hit: FindHit | undefined) => {
    if (!hit) return;
    setOpen(false);
    onJump(hit);
  };

  const shown = useMemo(() => hits.slice(0, 20), [hits]);

  return (
    <div
      data-side-head
      // `min-w-0`: the row is the block's width and never sets it
      // (第十五轮 第 4 条) — what is seated in it gives way instead.
      className="relative flex min-h-pane-head min-w-0 flex-none items-center bg-side select-none"
      style={{ paddingLeft: START }}
    >
      {/* The landing's head sits here. `hidden` while the box is widened
          — the same seat, the other range — and while there is nothing
          to seat; the element stays so the portal has somewhere to go. */}
      <div
        ref={(el) => {
          seatRef(el);
          setSeatEl(el);
        }}
        data-side-seat
        // …and while nobody sits in it: an empty seat is still `flex-1`,
        // and it took half the row from the 「搜索全部」 box beside it
        // (the narrow 更多 list, 2026-09-28).
        hidden={open || bare || !claimed}
        className="flex min-w-0 flex-1 items-center"
      />
      {!claimed && !open && !bare && (
        // Nobody is seated (a screen with no column): the row still has
        // the one search, at its wide range.
        // **A box, never a magnifier button** (components/SearchField,
        // 2026-09-27「搜索应该是个筐而非按钮」; 2026-09-28 it was still a
        // bare magnifier on the narrow face). The one field look —
        // `fill`, `radius-sm`, `ctl-md`, the magnifier at the left and
        // ⌘K at the right — pressed, it widens into the real box.
        <div className="@container/bar flex min-w-0 flex-1 items-center pr-1">
          <button
            type="button"
            data-global-search
            data-search-field
            aria-label={gui.find_everything}
            onClick={() => widen("")}
            className="flex h-ctl-md min-h-touch min-w-0 flex-1 cursor-text items-center gap-in rounded-sm bg-fill px-2 text-left text-tertiary"
          >
            <IconSearch className="flex-none" />
            <span className="min-w-0 flex-1 truncate text-body">{gui.find_everything}</span>
            {!narrow && <SearchHint />}
          </button>
        </div>
      )}
      {open && (
        // The widened box, in the row's own place. `absolute` with a
        // floor on its width, so a block narrowed to the rail alone
        // (a 「…」 page) still gets a box a word fits in — it reaches
        // over the ground beside, which is the one thing here allowed
        // to. **The floor is the token `--list-w-min`**, not `--list-w`:
        // that one is the block's live width now (第十五轮 第 4 条),
        // which is zero exactly where this floor is needed.
        <div
          ref={seat}
          data-search-seat
          className="absolute inset-y-0 z-50 flex min-w-list-min items-center bg-side pr-1"
          style={{ left: START, right: 0 }}
        >
          <div className="relative flex min-w-0 flex-1 items-center">
            <IconSearch className="pointer-events-none absolute top-1/2 left-2 -translate-y-1/2 text-tertiary" />
            <Input
              ref={box}
              data-search-box
              data-search-field
              data-scope="all"
              value={word}
              placeholder={gui.find_hint}
              aria-label={gui.find_everything}
              role="combobox"
              aria-expanded={shown.length > 0}
              aria-controls="khor-search-rows"
              className="min-h-touch rounded-sm border-0 bg-fill pr-12 pl-7 text-body shadow-none placeholder:text-tertiary focus-visible:ring-0 md:text-body"
              onChange={(e) => setWord(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "ArrowDown") {
                  e.preventDefault();
                  setAt((i) => Math.min(i + 1, shown.length - 1));
                } else if (e.key === "ArrowUp") {
                  e.preventDefault();
                  setAt((i) => Math.max(i - 1, 0));
                } else if (e.key === "Enter") {
                  e.preventDefault();
                  go(shown[at]);
                } else if (e.key === "Escape") {
                  e.preventDefault();
                  setOpen(false);
                }
              }}
            />
            {/* Which range the box is on, in one small word (第 4 条). */}
            <kbd
              data-search-scope
              className="pointer-events-none absolute top-1/2 right-2 -translate-y-1/2 rounded-xs border px-xxs text-caption text-tertiary"
            >
              {gui.search_all}
            </kbd>
            {/* Hung from the box, not from the page: the row is the
                anchor, and what is under it stays under it however the
                window moves. */}
            {word.trim() !== "" && (
              <div
                id="khor-search-rows"
                data-search-rows
                role="listbox"
                // Search results float (20-states 浮层): `popover` under
                // `float`, which on the dark theme is lighter than what
                // it hangs over — `card` was the board's own colour.
                className="absolute top-full right-0 z-50 mt-1 flex max-h-80 flex-col overflow-y-auto rounded-lg border bg-popover shadow-(--shadow-float)"
                // From the list column's edge, not the box's: hung
                // straight down they would lie over the rail's glyphs
                // (tokens.css `--seat-menu-inset`).
                style={{ left: "var(--seat-menu-inset)" }}
              >
                {shown.length === 0 && (
                  <span data-search-none className="px-2 py-2 text-callout text-muted-foreground">
                    {gui.find_nothing}
                  </span>
                )}
                {shown.map((h, i) => (
                  <button
                    key={`${h.kind}:${h.id}`}
                    type="button"
                    role="option"
                    aria-selected={i === at}
                    data-search-row={h.id}
                    data-search-kind={h.kind}
                    data-on={i === at}
                    onClick={() => go(h)}
                    className={cn(
                      "flex min-h-touch min-w-0 items-center gap-2 px-2 text-left text-callout hover:bg-muted/50",
                      i === at && "bg-accent",
                    )}
                  >
                    <span className="min-w-0 flex-1 truncate">{h.label}</span>
                    {h.note !== null && (
                      <span className="min-w-0 flex-1 truncate text-caption text-muted-foreground">{h.note}</span>
                    )}
                    <span className="flex-none text-caption text-muted-foreground">{kindWord(h.kind)}</span>
                  </button>
                ))}
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
