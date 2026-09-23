// The right side of the window (docs/SHELL.md 第十三轮 第 5 条): a first
// row at `--pane-head` — the same token the left block's row wears, so
// the two are one line across the window — and under it **a board
// floating on the app's ground**: `--card`, `--gap-in` from the region's
// edges, `--radius` at the corners. The row holds the tabs where a
// landing has them, and stays as a blank band where it does not, because
// it is also where Windows' own three buttons live and where the
// window is dragged by.
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
  narrow = false,
  className,
  children,
}: {
  /** What the first row holds — a tab strip, an address bar. Absent,
      the row is a blank band on the wide face and nothing on the
      narrow one (第 8 条:「窄档…其余不变」). */
  head?: ReactNode;
  narrow?: boolean;
  /** How the board lays its content out. */
  className?: string;
  children: ReactNode;
}) {
  return (
    <>
      {(head !== undefined || !narrow) && (
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
        className={cn("mx-in mb-in min-h-0 flex-1 overflow-hidden rounded-lg bg-card", className)}
      >
        {children}
      </div>
    </>
  );
}
