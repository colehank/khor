// **The right side of the window, every page** (docs/SHELL.md 第十五轮
// 第 5、6 条; docs/DESIGN.md 第五节「三种页型」).
//
// Two rows, never more: **R0**, a first row at `--pane-head` — the same
// token the left block's row wears, so the two are one line across the
// window — and under it **R1, a board floating on the app's ground**
// (`--card`, `--gap-in` from the region's other three edges, `--radius-xl`
// at the corners). A page fills them; it does not add a row, take one
// away or change a height.
//
// **One component, one prop saying which of three pages this is**:
//
//   detail  A 详情 — pick one thing, see it. R0 is `PageHead`: the
//           page's name, the page's own
//           buttons at the right end. 机器, 额度, 「…」, 工作区.
//   tabs    B 标签 — open several, switch. R0 is the tab strip; with no
//           tab open it is `PageHead` with the landing's name. 会话.
//   doc     C 文档 — the row *is* a live document. R0 is its address bar.
//           浏览.
//
// The kind is written on the region's root (`data-region-kind`) so a
// gate can find every page by what it claims to be.
//
// ⚠️ 墓碑 (第十四轮 第 3 条, 2026-09-24 → 2026-09-25): 「没有标签的页没有
// 行」. A page with no tabs had no R0 and its board started at the top,
// and Windows got a thin row of its own for its three buttons. D2
// (2026-09-25) put R0 back on every page — with something in it this
// time, a way back and a name, like System Settings — so the three are
// at R0's right end again on every page and the thin row is gone.
//
// **The open tab is the board's colour and joins it** — that is the
// whole of how a tab says it is in front (`TabStrip`): its bottom edge
// has no edge, and the board is what it is continuous with.
import { createContext, useContext, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";

import { windowClose, windowMaximizeToggle, windowMinimize, windowsApp } from "@/api";
import { IconClose } from "@/components/icons";
import { WINDOW_DRAG } from "@/components/shell";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";

/**
 * **Windows' habit, drawn at the right end of R0 because Windows has no
 * buttons of its own over it** (第十轮, moved here in 第十三轮):
 * minimise, maximise, close — close turning red under the pointer, the
 * way every window on that desktop behaves. Only that platform draws
 * them: macOS has the system's at the left block's row and Linux keeps
 * its decorations.
 */
export function WindowControls() {
  if (!windowsApp) return null;
  return (
    <div data-window-controls className="flex flex-none items-center self-stretch">
      {[
        { key: "minimize", label: gui.window_minimize, run: windowMinimize, mark: "—" },
        { key: "maximize", label: gui.window_maximize, run: windowMaximizeToggle, mark: "▢" },
        { key: "close", label: gui.window_close, run: windowClose, mark: null },
      ].map((b) => (
        <button
          key={b.key}
          type="button"
          data-window-control={b.key}
          aria-label={b.label}
          onClick={() => void b.run().catch(() => {})}
          className={cn(
            "flex h-full w-12 items-center justify-center text-caption",
            b.key === "close" ? "hover:bg-destructive hover:text-on-destructive" : "hover:bg-secondary",
          )}
        >
          {b.mark ?? <IconClose />}
        </button>
      ))}
    </div>
  );
}

/**
 * Where a page's own buttons go (D9: page buttons live at R0's right
 * end). `undefined` is 「no R0 around this」 — the narrow face, where a
 * page's head is its own (`NarrowHead`) — and there the buttons stay
 * where they were written.
 */
const ActionsSeat = createContext<HTMLElement | null | undefined>(undefined);

/**
 * **A page's button, drawn in R0 by the page that owns it** (2026-09-28,
 * 安全's 「立即锁定」). A 「…」 page is built inside the board, and the
 * one row that holds page buttons is R0 above it — which the shell
 * draws. This lets the page say what its buttons are and the shell say
 * where they stand, the way `SideSeat` does for a landing's search.
 */
export function PageAction({ children }: { children: ReactNode }) {
  const seat = useContext(ActionsSeat);
  if (seat === undefined) return <>{children}</>;
  if (seat === null) return null;
  return createPortal(children, seat);
}

/** Where a page's second title line goes, under its name in R0. */
const SubtitleSeat = createContext<HTMLElement | null | undefined>(undefined);

/**
 * **A page's second title line, inside R0's 48** (2026-09-28, the open
 * workspace: branch, changed files, `机器:路径`). The macOS title-and-
 * subtitle pair — Xcode puts the branch under the project — and not a
 * row between R0 and R1, which nothing may add (10-layout). Only a page
 * with a *location* to state wears one (components/Toolbar says when).
 *
 * Drawn by the page that owns the facts, seated by the shell, the way
 * `PageAction` is. **Nothing where there is no R0**: the narrow face's
 * head is the page's own (`NarrowHead`) and says what fits there itself.
 */
export function PageSubtitle({ children }: { children: ReactNode }) {
  const seat = useContext(SubtitleSeat);
  if (!seat) return null;
  return createPortal(children, seat);
}

/** Which of the three page types a region is (docs/DESIGN.md 第五节). */
export type RegionKind = "detail" | "tabs" | "doc";

/**
 * **R0 on a page with no tabs** (D2): the page's name in `title-sm`,
 * and a slot at the right end for the page's own buttons. **No ‹ ›**
 * (2026-09-26, 「机器等非标签页左上角不用有返回与前进，就是标题就行」):
 * a detail page is reached from its list, which is always beside it,
 * so a history over what was picked walked what the list already shows.
 */
export function PageHead({
  title,
  actions,
  caption,
  seatRef,
  subRef,
}: {
  title: ReactNode;
  actions?: ReactNode;
  /** A stale or offline region's one line (第十七轮 §6, 20-states.md):
      「3 分钟前的读数」 at R0's right end, never a banner in the board. */
  caption?: ReactNode;
  /** Where the page's own buttons are seated (`PageAction`), before
      the shell's. */
  seatRef?: (el: HTMLElement | null) => void;
  /** Where the page's second title line is seated (`PageSubtitle`).
      Empty, it takes no room and the name sits alone on R0's middle. */
  subRef?: (el: HTMLElement | null) => void;
}) {
  return (
    <div data-page-head className="flex min-w-0 flex-1 items-center gap-row self-stretch px-base">
      {/* The name over its second line, the pair centred in the 48. */}
      <div className="flex min-w-0 flex-1 flex-col justify-center">
        <h1 data-page-title className="min-w-0 truncate text-title-sm font-title">
          {title}
        </h1>
        {subRef && <div ref={subRef} data-page-subtitle className="flex min-w-0 empty:hidden" />}
      </div>
      {caption && (
        <span data-region-caption className="flex-none truncate text-caption text-tertiary tabular-nums">
          {caption}
        </span>
      )}
      {(actions || seatRef) && (
        <div data-page-actions className="flex flex-none items-center gap-in">
          {seatRef && <span ref={seatRef} data-page-actions-seat className="contents" />}
          {actions}
        </div>
      )}
    </div>
  );
}

export function Region({
  kind,
  head,
  title,
  actions,
  caption,
  busy,
  flush,
  className,
  children,
}: {
  kind: RegionKind;
  /** What R0 holds where it is not a `PageHead` — the tab strip (tabs),
      the address bar (doc). **Absent, R0 is a `PageHead`** with the
      `title` below: every page has the row (D2), a page without tabs
      puts its name in it. */
  head?: ReactNode;
  /** The page's name in R0 — the landing's name when nothing is picked,
      the picked thing's name when something is. Read where `head` is
      absent. */
  title?: ReactNode;
  /** The page's own buttons at R0's right end (第十七轮 §3). */
  actions?: ReactNode;
  /** A stale region's caption at R0's right end — in the `PageHead`, or
      after `head` where R0 is a strip or an address bar. */
  caption?: ReactNode;
  /** Refreshing content already seen: a 2 px brand bar along R0's
      bottom edge, the content left as it was (第十七轮 §6). */
  busy?: boolean;
  /** **The board is the panes' own** (第十七轮 §5, D8): an open
      workspace is B inside R1 — three panes, each with a head at
      `--pane-head`, hairlines between them — so the board wears no
      margin and no corner of its own, and a hairline under R0 is the
      one edge between the two. */
  flush?: boolean;
  /** How the board lays its content out. */
  className?: string;
  children: ReactNode;
}) {
  const [seat, setSeat] = useState<HTMLElement | null>(null);
  const [sub, setSub] = useState<HTMLElement | null>(null);
  return (
    <div data-region data-region-kind={kind} className="flex min-h-0 min-w-0 flex-1 flex-col">
      <div
        data-region-head
        // R0 is one of the window's three handles, marked by the shell
        // (`shell.ts`): its blank moves the window, and what is pressable
        // inside it keeps its press by tauri's own rule.
        {...WINDOW_DRAG}
        className="relative flex h-pane-head flex-none items-end select-none"
      >
        <div className="flex min-w-0 flex-1 self-stretch">
          {head ?? (
            <PageHead title={title} actions={actions} caption={caption} seatRef={setSeat} subRef={setSub} />
          )}
        </div>
        {/* **Every R0 can say it is stale**, not only a `PageHead`: on a
            tab strip (B) or an address bar (C) the caption sits at the
            row's right end after them, outside the strip — so the strip
            measures what is left and compresses its tabs into that,
            rather than being pushed past the edge. */}
        {head && caption && (
          <span
            data-region-caption
            className="flex-none self-center truncate px-row text-caption text-tertiary tabular-nums"
          >
            {caption}
          </span>
        )}
        <WindowControls />
        {busy && <div data-region-busy aria-hidden className="absolute inset-x-0 bottom-0 h-0.5 bg-brand" />}
      </div>
      {/* The board. On narrow it is the whole screen and wears no
          margin and no corner (`app.css`, keyed on `data-narrow`).
          **A block, not a flex column**: what it holds is written
          `h-full` (a machine's card, a session's pane), and a flex
          item's `min-height: auto` lets that grow past the board to
          its content — the card's process table went out of reach of
          the face gate the first time this was a column. A block with
          a definite height is what the wrapper this replaces was.
          **Joined to R0 with nothing between** — under a strip the open
          tab and the board are one piece, and under a page's name there
          is no second line (第十四轮 第 1 条). */}
      <div
        data-board
        data-board-flush={flush ? "" : undefined}
        className={cn(
          "min-h-0 flex-1 overflow-hidden bg-card",
          flush ? "border-t" : "mx-in mb-in rounded-xl",
          className,
        )}
      >
        <ActionsSeat.Provider value={head ? undefined : seat}>
          <SubtitleSeat.Provider value={head ? undefined : sub}>{children}</SubtitleSeat.Provider>
        </ActionsSeat.Provider>
      </div>
    </div>
  );
}
