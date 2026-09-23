// 窗口顶上那一条 (docs/SHELL.md 第十轮, 收尾在第十一轮).
//
// **One row across the whole window, above the rail and every pane** —
// VS Code's and Slack's structure, and the reason is portability: the
// round before this one moved macOS's own three buttons back into place
// whenever AppKit re-laid them out, which was khor wrestling AppKit and
// was true on one platform. Here nothing of the system's is touched:
//
//   - macOS keeps the buttons exactly as the SDK draws them (size and
//     spacing), placed inside this row by tauri's `trafficLightPosition`
//     — with the number khor measures rather than one written down
//     (`src-tauri/src/traffic.rs`); the row leaves `--traffic` of width.
//   - Windows draws its own three at the right end (its own habit).
//   - Linux keeps the system's decorations; the row is still here.
//   - The web face has no window at all, and the row is still here —
//     what it carries is not about a window.
//
// **Three columns, and the middle one is centred on the window** (第十一
// 轮 第 1 条:「为什么搜索框没有居中？」). Left and right are `1fr` each,
// so the search sits on the row's own centre however long the crumbs
// get; macOS's strip is padding on the left column, which is what keeps
// the centre the *window's* centre rather than the content's.
//
// **The search expands in place** (第十一轮 第 2 条:「和 vscode 一样就地
// 搜索就行了」): pressing it — or ⌘K anywhere — turns the box into a
// field and hangs the hits under it. No dialog, no overlay: the window
// behind stays readable and stays clickable, and Esc or a press outside
// puts it away.
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
  const seat = useRef<HTMLDivElement>(null);

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

  // **A press outside puts it away** — the half of 「就地展开」 a dialog
  // used to get for free from its overlay, and the reason there is no
  // overlay: the window behind stays live, so the only thing that should
  // close this is a press that was not meant for it.
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
  const here = narrow ? crumbs.slice(-1) : crumbs;

  return (
    <header
      data-title-bar
      // The blank of this row is what moves the window, on every
      // platform that lets a page say so (第八轮 第 59 条's rule, in its
      // own row now rather than borrowed from each landing's head).
      data-tauri-drag-region="deep"
      // `minmax(0,1fr)` rather than `1fr`: a plain `1fr` is
      // `minmax(auto,1fr)`, so a long crumb — or macOS's own strip of
      // padding — would push the left column wider than the right and
      // the middle off the window's centre, which is the very thing
      // 第十一轮 第 1 条 is about.
      className="grid h-title-bar flex-none grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)] items-center gap-2 border-b bg-card select-none"
    >
      <nav
        data-title-crumbs
        className="flex min-w-0 items-center gap-1 overflow-hidden text-xs"
        // macOS's three buttons stand in this column's left end; the
        // width is the window's own measurement (`--traffic`).
        style={{ paddingLeft: "var(--traffic)" }}
      >
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
      {/* The middle column: the seat is `relative` so the hits hang from
          it, and the box inside is the fixed width the ruling names —
          360, down to 240 before it gives way. */}
      <div ref={seat} data-search-seat className="relative flex justify-center">
        {open ? (
          <div className="w-search-box">
            <Input
              ref={box}
              data-search-box
              value={word}
              placeholder={gui.find_hint}
              aria-label={gui.find_everything}
              role="combobox"
              aria-expanded={shown.length > 0}
              aria-controls="khor-search-rows"
              className="h-7 w-full"
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
            {/* Hung from the seat, not from the page: the row is the
                anchor, and what is under it stays under it however the
                window moves. */}
            {word.trim() !== "" && (
              <div
                id="khor-search-rows"
                data-search-rows
                role="listbox"
                className="absolute top-full right-0 left-0 z-50 mt-1 flex max-h-80 flex-col overflow-y-auto rounded-md border bg-card shadow-md"
              >
                {shown.length === 0 && (
                  <span data-search-none className="px-2 py-2 text-sm text-muted-foreground">
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
                      "flex min-h-touch min-w-0 items-center gap-2 px-2 text-left text-sm hover:bg-muted/50",
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
            )}
          </div>
        ) : (
          <Button
            variant="ghost"
            size={narrow ? "icon" : "sm"}
            data-global-search
            aria-label={gui.find_everything}
            onClick={() => setOpen(true)}
            className={cn(
              "min-h-touch flex-none justify-start gap-2",
              narrow ? "min-w-touch" : "h-7 w-search-box border text-muted-foreground",
            )}
          >
            <IconSearch />
            {!narrow && (
              <>
                <span className="min-w-0 flex-1 truncate text-left text-xs">{gui.find_everything}</span>
                <span data-search-hint className="text-xs tabular-nums">
                  ⌘K
                </span>
              </>
            )}
          </Button>
        )}
      </div>
      <div className="flex min-w-0 items-center justify-end gap-1">
        {!narrow && (
          <button
            type="button"
            data-net-line
            onClick={onNetwork}
            className="min-h-touch min-w-0 truncate rounded-sm px-2 text-xs text-muted-foreground hover:bg-secondary"
          >
            {netLine(devices)}
          </button>
        )}
        {windowsApp && (
          // **Windows' habit, drawn here because Windows has no buttons
          // of its own over this row**: minimise, maximise, close —
          // close turning red under the pointer, the way every window on
          // that desktop behaves.
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
      </div>
    </header>
  );
}
