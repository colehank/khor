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
//   detail  A 详情 — pick one thing, see it. R0 is `PageHead`: ‹ › over
//           this landing's selections, the page's name, the page's own
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
import { useEffect, type ReactNode } from "react";

import { windowClose, windowMaximizeToggle, windowMinimize, windowsApp } from "@/api";
import { IconBack, IconClose } from "@/components/icons";
import { WINDOW_DRAG } from "@/components/shell";
import { Button } from "@/components/ui/button";
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

/** Which of the three page types a region is (docs/DESIGN.md 第五节). */
export type RegionKind = "detail" | "tabs" | "doc";

/**
 * ‹ › over **what was picked in this landing** — not a web page's
 * history (that is the address bar's, `AddressBar`) but the list's, the
 * way System Settings walks the panes you opened. Each half is absent
 * where there is nowhere to go, and then its button is not drawn.
 */
export type PageHistory = { back?: () => void; forward?: () => void };

/**
 * **R0 on a page with no tabs** (D2): ‹ › where there is a history, the
 * page's name in `title-sm`, and a slot at the right end for the page's
 * own buttons (第十七轮 moves them in; this round leaves the slot).
 *
 * ⌘[ and ⌘] walk the same history — bound while this head is on screen,
 * which is while its page is.
 */
export function PageHead({
  title,
  history,
  actions,
}: {
  title: ReactNode;
  history?: PageHistory;
  actions?: ReactNode;
}) {
  const back = history?.back;
  const forward = history?.forward;
  useEffect(() => {
    if (!back && !forward) return;
    const key = (e: KeyboardEvent) => {
      if (!(e.metaKey || e.ctrlKey) || e.altKey || e.shiftKey) return;
      if (e.key === "[" && back) {
        e.preventDefault();
        back();
      } else if (e.key === "]" && forward) {
        e.preventDefault();
        forward();
      }
    };
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, [back, forward]);
  return (
    <div data-page-head className="flex min-w-0 flex-1 items-center gap-1 self-stretch px-in">
      {back && (
        <Button
          variant="ghost"
          size="icon"
          data-history-back
          aria-label={gui.history_back}
          onClick={back}
          className="min-h-touch min-w-touch flex-none"
        >
          <IconBack />
        </Button>
      )}
      {forward && (
        <Button
          variant="ghost"
          size="icon"
          data-history-forward
          aria-label={gui.history_forward}
          onClick={forward}
          className="min-h-touch min-w-touch flex-none"
        >
          <IconBack className="rotate-180" />
        </Button>
      )}
      <h1 data-page-title className="min-w-0 flex-1 truncate px-1 text-title-sm font-title">
        {title}
      </h1>
      {actions && <div className="flex flex-none items-center gap-1">{actions}</div>}
    </div>
  );
}

export function Region({
  kind,
  head,
  title,
  history,
  actions,
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
  history?: PageHistory;
  /** The page's own buttons at R0's right end (第十七轮 fills this). */
  actions?: ReactNode;
  /** How the board lays its content out. */
  className?: string;
  children: ReactNode;
}) {
  return (
    <div data-region data-region-kind={kind} className="flex min-h-0 min-w-0 flex-1 flex-col">
      <div
        data-region-head
        // R0 is one of the window's three handles, marked by the shell
        // (`shell.ts`): its blank moves the window, and what is pressable
        // inside it keeps its press by tauri's own rule.
        {...WINDOW_DRAG}
        className="flex h-pane-head flex-none items-end select-none"
      >
        <div className="flex min-w-0 flex-1 self-stretch">
          {head ?? <PageHead title={title} history={history} actions={actions} />}
        </div>
        <WindowControls />
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
      <div data-board className={cn("mx-in mb-in min-h-0 flex-1 overflow-hidden rounded-xl bg-card", className)}>
        {children}
      </div>
    </div>
  );
}
