// 窗口顶上那一条 (docs/SHELL.md 第十轮, 2026-09-23:「我们还要支持多系统。
// 是不是应该不去动它，但是在那一行作出真正有信心的内容？」).
//
// **One row across the whole window, above the rail and every pane** —
// VS Code's and Slack's structure, and the reason is portability: the
// round before this one moved macOS's own three buttons back into place
// whenever AppKit re-laid them out, which was khor wrestling AppKit and
// was true on one platform. Here nothing of the system's is touched:
//
//   - macOS keeps the buttons exactly as the SDK draws them (size and
//     spacing), placed inside this row by tauri's `trafficLightPosition`;
//     the row leaves `--traffic` of width for them.
//   - Windows draws its own three at the right end (its own habit).
//   - Linux keeps the system's decorations; the row is still here.
//   - The web face has no window at all, and the row is still here —
//     what it carries is not about a window.
//
// **What it carries is the global half of the app**: where you are, one
// search over everything, and the network in a sentence. Each of the
// three is a thing that is true of the whole app rather than of one
// pane, which is exactly what a pane's own head cannot say.
import { useEffect, useMemo, useRef, useState } from "react";

import {
  find as findAll,
  windowClose,
  windowMaximizeToggle,
  windowMinimize,
  windowsApp,
  type DeviceRow,
  type FindHit,
} from "@/api";
import { IconClose, IconSearch } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";

/** One step of 你在哪 — the machine, the page, the thing open in it. */
export type Crumb = { key: string; label: string; onPress?: () => void };

/**
 * The network in one line: how many machines, and what is wrong where
 * anything is. **Counted from the same device rows the machines landing
 * draws**, so the number cannot disagree with the list.
 */
function netLine(devices: DeviceRow[]): string {
  if (devices.length === 0) return gui.net_line_alone;
  const paused = devices.filter((d) => d.paused).length;
  // No road known is the honest reading of 「连不上」 here: `hop` is
  // what the device row says about how it is reached, and a satellite
  // rides its sponsor's.
  const quiet = devices.filter((d) => !d.me && d.hop === null && d.sponsor === null).length;
  if (paused > 0) return gui.net_line_paused(devices.length, paused);
  if (quiet > 0) return gui.net_line_quiet(devices.length, quiet);
  return gui.net_line_all(devices.length);
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

export function TitleBar({
  narrow,
  crumbs,
  devices,
  onJump,
  onNetwork,
}: {
  narrow: boolean;
  /** 你在哪, left to right. Narrow draws only the last one. */
  crumbs: Crumb[];
  devices: DeviceRow[];
  /** A search hit was chosen: go there. */
  onJump: (hit: FindHit) => void;
  onNetwork: () => void;
}) {
  const [open, setOpen] = useState(false);
  const [word, setWord] = useState("");
  const [hits, setHits] = useState<FindHit[]>([]);
  const [at, setAt] = useState(0);
  const box = useRef<HTMLInputElement>(null);

  // ⌘K anywhere in the app — the one shortcut this row owns. Bound on
  // the window rather than on the button: a person who is typing in a
  // pane still means 「search everything」 by it.
  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      if (e.key.toLowerCase() !== "k" || !(e.metaKey || e.ctrlKey)) return;
      e.preventDefault();
      setOpen(true);
    };
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, []);

  useEffect(() => {
    if (!open) return;
    setWord("");
    setHits([]);
    setAt(0);
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
  const here = narrow ? crumbs.slice(-1) : crumbs;

  return (
    <header
      data-title-bar
      // The blank of this row is what moves the window, on every
      // platform that lets a page say so (第八轮 第 59 条's rule, in its
      // own row now rather than borrowed from each landing's head).
      data-tauri-drag-region="deep"
      className="flex h-title-bar flex-none items-center gap-2 border-b bg-card pr-2 select-none"
      // macOS puts its three buttons inside this row; the width they
      // stand in is the token, and it is zero everywhere else.
      style={{ paddingLeft: "var(--traffic)" }}
    >
      <nav data-title-crumbs className="flex min-w-0 flex-1 items-center gap-1 text-xs">
        {here.map((c, i) => (
          <span key={c.key} className="flex min-w-0 items-center gap-1">
            {i > 0 && <span className="flex-none text-muted-foreground">›</span>}
            <button
              type="button"
              data-crumb-step={c.key}
              disabled={c.onPress === undefined}
              onClick={c.onPress}
              className={cn(
                // The row's whole height, which is the most a 38-tall
                // band can give a target (`UNDER_FLOOR` in check-face
                // carries why that is allowed to be under the floor).
                "h-title-bar min-w-0 truncate rounded-sm px-1 text-left",
                c.onPress === undefined ? "text-foreground" : "text-muted-foreground hover:bg-secondary",
              )}
            >
              {c.label}
            </button>
          </span>
        ))}
      </nav>
      <Button
        variant="ghost"
        size={narrow ? "icon" : "sm"}
        data-global-search
        aria-label={gui.find_everything}
        onClick={() => setOpen(true)}
        className="min-h-touch min-w-touch flex-none"
      >
        <IconSearch />
        {!narrow && (
          <>
            <span className="text-xs text-muted-foreground">{gui.find_everything}</span>
            <span data-search-hint className="text-xs tabular-nums text-muted-foreground">
              ⌘K
            </span>
          </>
        )}
      </Button>
      {!narrow && (
        <button
          type="button"
          data-net-line
          onClick={onNetwork}
          className="min-h-touch flex-none rounded-sm px-2 text-xs text-muted-foreground hover:bg-secondary"
        >
          {netLine(devices)}
        </button>
      )}
      {windowsApp && (
        // **Windows' habit, drawn here because Windows has no buttons of
        // its own over this row**: minimise, maximise, close — close
        // turning red under the pointer, the way every window on that
        // desktop behaves.
        <div data-window-controls className="flex flex-none items-center">
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
                "flex h-title-bar w-12 items-center justify-center text-xs",
                b.key === "close" ? "hover:bg-destructive hover:text-white" : "hover:bg-secondary",
              )}
            >
              {b.mark ?? <IconClose />}
            </button>
          ))}
        </div>
      )}
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent data-search-dialog>
          <DialogHeader>
            <DialogTitle>{gui.find_everything}</DialogTitle>
          </DialogHeader>
          <div className="flex min-w-0 flex-col gap-2">
            <Input
              ref={box}
              autoFocus
              data-search-box
              value={word}
              placeholder={gui.find_hint}
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
                }
              }}
            />
            {word.trim() !== "" && shown.length === 0 && (
              <span data-search-none className="text-sm text-muted-foreground">
                {gui.find_nothing}
              </span>
            )}
            <div data-search-rows className="flex max-h-72 min-w-0 flex-col overflow-y-auto">
              {shown.map((h, i) => (
                <button
                  key={`${h.kind}:${h.id}`}
                  type="button"
                  data-search-row={h.id}
                  data-search-kind={h.kind}
                  data-on={i === at}
                  onClick={() => go(h)}
                  className={cn(
                    "flex min-h-touch min-w-0 items-center gap-2 rounded-md px-2 text-left text-sm hover:bg-muted/50",
                    i === at && "bg-accent",
                  )}
                >
                  <span className="min-w-0 flex-1 truncate">{h.label}</span>
                  {h.note !== null && (
                    <span className="min-w-0 flex-1 truncate text-xs text-muted-foreground">{h.note}</span>
                  )}
                  <span className="flex-none text-xs text-muted-foreground">{kindWord(h.kind)}</span>
                </button>
              ))}
            </div>
          </div>
        </DialogContent>
      </Dialog>
    </header>
  );
}
