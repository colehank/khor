// The right side of the window (docs/SHELL.md 第十三轮 第 5 条): a first
// row at `--pane-head` — the same token the left block's row wears, so
// the two are one line across the window — and under it **a board
// floating on the app's ground**: `--card`, `--gap-in` from the region's
// edges, `--radius` at the corners. The row holds the tabs where a
// landing has them — **and where a landing has none there is no row**
// (第十四轮 第 3 条): the board starts at the top, `--gap-in` from it
// like every other side. The blank of the row is where the window is
// dragged by and, on Windows, where its own three buttons live; a page
// with no row gives those three a thin row of their own instead.
//
// **The open tab is the board's colour and joins it** — that is the
// whole of how a tab says it is in front (`TabStrip`): its bottom edge
// has no edge, and the board is what it is continuous with.
import type { ReactNode } from "react";

import { windowClose, windowMaximizeToggle, windowMinimize, windowsApp } from "@/api";
import { IconClose } from "@/components/icons";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";

/**
 * **Windows' habit, drawn at the right end of the row because Windows
 * has no buttons of its own over it** (第十轮, moved here in 第十三轮):
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
            "flex h-full w-12 items-center justify-center text-xs",
            b.key === "close" ? "hover:bg-destructive hover:text-white" : "hover:bg-secondary",
          )}
        >
          {b.mark ?? <IconClose />}
        </button>
      ))}
    </div>
  );
}

export function Region({
  head,
  className,
  children,
}: {
  /** What the first row holds — a tab strip, an address bar.
      **Absent, there is no row**: the board starts at the top
      (第十四轮 第 3 条). */
  head?: ReactNode;
  /** How the board lays its content out. */
  className?: string;
  children: ReactNode;
}) {
  return (
    <>
      {/* **The row exists only where something heads it** (第十四轮 第
          3 条, 2026-09-24:「现在机器、额度和浏览器的顶部都空了一截出来,
          这应该属于是非标签页的抽象没做好」). 第十三轮 第 5 条 said the
          right side has a first row at `--pane-head`, and that was read
          as「always」— so a machine page, a 额度 page, 「…」 and a
          sessions page with nothing open each wore an empty band as
          tall as a tab strip. A band is the *tabs'* row; a page with no
          tabs has no row, and its board starts at the top. */}
      {head !== undefined && (
        <div
          data-region-head
          // The blank of this row moves the window (第八轮 第 59 条);
          // the strip inside says the same on its own scroller.
          data-tauri-drag-region="deep"
          className="flex h-pane-head flex-none items-end select-none"
        >
          <div className="flex min-w-0 flex-1 self-stretch">{head}</div>
          <WindowControls />
        </div>
      )}
      {/* **Windows still needs somewhere to put its three.** macOS has
          the system's buttons over the left block and Linux keeps its
          decorations, so only that desktop is left without a home for
          them when the row above is gone. It gets a thin row of its
          own — the three and nothing else, above the board's top
          corner — and every other platform gets nothing at all, which
          is a height of zero (第十四轮 第 3 条). */}
      {head === undefined && windowsApp && (
        <div
          data-window-row
          data-tauri-drag-region="deep"
          className="flex h-ctl-lg flex-none items-stretch justify-end select-none"
        >
          <WindowControls />
        </div>
      )}
      {/* The board. On narrow it is the whole screen and wears no
          margin and no corner (`app.css`, keyed on `data-narrow`).
          **A block, not a flex column**: what it holds is written
          `h-full` (a machine's card, a session's pane), and a flex
          item's `min-height: auto` lets that grow past the board to
          its content — the card's process table went out of reach of
          the face gate the first time this was a column. A block with
          a definite height is what the wrapper this replaces was. */}
      <div
        data-board
        className={cn(
          "mx-in mb-in min-h-0 flex-1 overflow-hidden rounded-lg bg-card",
          // **Only where nothing heads it.** Under a tab strip the board
          // is joined to the open tab and any gap there would be a seam
          // through the one thing that says which tab is in front
          // (`TabStrip`); with no strip above, the top is an edge like
          // the other three and wears the same `--gap-in`.
          head === undefined && "mt-in",
          className,
        )}
      >
        {children}
      </div>
    </>
  );
}
