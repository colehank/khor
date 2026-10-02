// The keys a soft keyboard lacks, and the switch for the keyboard itself
// (2026-10-02, iOS 第三步 A).
//
// A phone's keyboard types text. A terminal also wants Esc, Ctrl, Tab
// and Shift-Tab, the arrows, and a handful of characters a shell lives
// on that sit two layers deep (`/ - | :`). Every phone terminal grows the
// same row over the keyboard; this is that row, drawn **only under a
// finger** — the pointer decides the touch things, never the width
// (docs/UX.md): a Mac window dragged narrow has a real keyboard and no
// row, an iPad on the wide face has the row under its terminal.
//
// **Real bytes, the terminal's own road.** A key here is handed to xterm
// as if typed (`Terminal.input`, `wasUserInput`), so it leaves through
// `onData` → `term_key` like every other keystroke and scrolls the screen
// back to the cursor the way typing does. The arrows ask xterm which
// cursor-key mode the program set (DECCKM: `ESC O A` in an editor's
// application mode, `ESC [ A` otherwise). Paste reads the clipboard and
// hands the text to xterm's own paste (`Terminal.paste`), which brackets
// it when the program asked for that (DECSET 2004) — the same path ⌘V
// takes on a Mac.
//
// **Ctrl is a latch.** Pressed, it waits for the next key — from this row
// or from the soft keyboard (`TerminalPane` applies it to what xterm hands
// out) — sends that key's control form (C → 0x03), and lets go. Pressed
// again while waiting, it lets go with nothing sent. It shows that it is
// waiting as a grey cap (a held key, the app's 「on」 grey — never the
// brand) and says so (`aria-pressed`).
//
// **A key never takes the focus.** Pressing a cap must not move the caret
// out of the terminal, or the keyboard would drop on every press — so a
// press is answered on `mousedown` with `preventDefault` (WebKit's tap
// focuses on its synthesized mousedown) and acted on at the click.
import { useEffect, useState, type RefObject } from "react";
import type { Terminal } from "@xterm/xterm";

import { IconKeyboard } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";

/** The control form of one typed character, or `null` where Ctrl does
    not change it. The table every terminal agrees on: `@`–`_` and the
    letters fold to 0x00–0x1f, `?` is DEL, space is NUL, `/` is 0x1f (the
    key `_` shares on a US keyboard, which is what xterm sends for ⌃/). */
export function ctrlOf(ch: string): string | null {
  if (ch.length !== 1) return null;
  if (ch === "?") return "\x7f";
  if (ch === " ") return "\x00";
  if (ch === "/") return "\x1f";
  const up = ch.toUpperCase().charCodeAt(0);
  if (up >= 0x40 && up <= 0x5f) return String.fromCharCode(up & 0x1f);
  return null;
}

/** An arrow, as the program's cursor-key mode spells it — and with Ctrl
    held, the modifier form (`ESC [ 1 ; 5 A`) xterm sends for ⌃↑. */
function arrow(term: Terminal, letter: "A" | "B" | "C" | "D", ctrl: boolean): string {
  if (ctrl) return `\x1b[1;5${letter}`;
  return term.modes.applicationCursorKeysMode ? `\x1bO${letter}` : `\x1b[${letter}`;
}

type Key = {
  /** `data-term-key`, what a gate presses. */
  name: string;
  cap: string;
  label?: string;
  /** What the key sends, Ctrl latched or not. `null` from `send` is a
      key with its own act (paste, Ctrl itself). */
  send?: (term: Terminal, ctrl: boolean) => string;
};

/** The row, in the order a hand reaches: the escape and the modifier
    first, then what moves through a form, then the characters a shell
    lives on, then the arrows together at the end. */
const KEYS: Key[] = [
  { name: "esc", cap: gui.key_esc, label: gui.key_esc_name, send: () => "\x1b" },
  { name: "ctrl", cap: gui.key_ctrl, label: gui.key_ctrl_name },
  { name: "tab", cap: gui.key_tab, label: gui.key_tab_name, send: () => "\t" },
  { name: "backtab", cap: gui.key_backtab, label: gui.key_backtab_name, send: () => "\x1b[Z" },
  { name: "paste", cap: gui.key_paste },
  ...["/", "-", "|", ":"].map((ch) => ({
    name: ch,
    cap: ch,
    send: (_: Terminal, ctrl: boolean) => (ctrl ? (ctrlOf(ch) ?? ch) : ch),
  })),
  { name: "left", cap: "←", label: gui.key_left, send: (t, c) => arrow(t, "D", c) },
  { name: "up", cap: "↑", label: gui.key_up, send: (t, c) => arrow(t, "A", c) },
  { name: "down", cap: "↓", label: gui.key_down, send: (t, c) => arrow(t, "B", c) },
  { name: "right", cap: "→", label: gui.key_right, send: (t, c) => arrow(t, "C", c) },
];

export function TermKeys({
  term,
  ctrl,
  onCtrl,
}: {
  /** The terminal the keys go into. Null until it has opened. */
  term: RefObject<Terminal | null>;
  /** Whether Ctrl is latched — held by `TerminalPane`, because a key
      from the soft keyboard has to be able to take it too. */
  ctrl: boolean;
  onCtrl: (on: boolean) => void;
}) {
  const press = (k: Key) => {
    const t = term.current;
    if (!t) return;
    if (k.name === "ctrl") {
      onCtrl(!ctrl);
      return;
    }
    if (k.name === "paste") {
      // The clipboard can refuse (no permission, a WebKit paste callout
      // dismissed); then nothing is typed, which is the true answer.
      void navigator.clipboard
        ?.readText()
        .then((text) => {
          if (text) t.paste(text);
        })
        .catch(() => {});
      return;
    }
    const bytes = k.send?.(t, ctrl);
    if (!bytes) return;
    // Let go of the latch before the bytes go: `TerminalPane` reads it for
    // whatever xterm hands out, and these are already in their Ctrl form.
    if (ctrl) onCtrl(false);
    t.input(bytes, true);
  };
  return (
    <div
      data-term-keys
      role="toolbar"
      aria-label={gui.term_keys}
      className="flex h-key-row flex-none items-stretch overflow-x-auto bg-background px-xxs"
    >
      {KEYS.map((k) => {
        const held = k.name === "ctrl" && ctrl;
        return (
          <button
            key={k.name}
            type="button"
            data-term-key={k.name}
            data-on={k.name === "ctrl" ? held : undefined}
            aria-pressed={k.name === "ctrl" ? held : undefined}
            aria-label={k.label}
            // The caret stays in the terminal (the file head).
            onMouseDown={(e) => e.preventDefault()}
            onClick={() => press(k)}
            // The box is the finger's floor, the cap is drawn inside it:
            // `p-xxs` round a `radius-xs` cap, the key cap's corner.
            className="flex min-w-touch flex-none items-stretch p-xxs outline-none focus-visible:[&>span]:ring-2 focus-visible:[&>span]:ring-ring"
          >
            <span
              className={cn(
                "flex flex-1 items-center justify-center rounded-xs px-1 text-callout text-foreground",
                held ? "bg-fill-strong" : "bg-card",
              )}
            >
              {k.cap}
            </span>
          </button>
        );
      })}
    </div>
  );
}

/**
 * The soft keyboard's one switch: up when it is down, down when it is up
 * — in the bar's first row, at its right end, which is where R0 keeps the
 * open thing's verbs (`components/SessionBar`). Only under a finger: a
 * mouse has a real keyboard and nothing to raise. It reads 「up」 as the
 * field having the focus, which is what raises a phone's keyboard; up, it
 * wears the app's 「on」 grey.
 */
export function KeyboardSwitch({ field }: { field: RefObject<HTMLElement | null> }) {
  const [up, setUp] = useState(false);
  useEffect(() => {
    const sync = () => setUp(field.current !== null && document.activeElement === field.current);
    // `focusout` runs before the focus has gone anywhere; read after it.
    const later = () => queueMicrotask(sync);
    sync();
    document.addEventListener("focusin", sync);
    document.addEventListener("focusout", later);
    return () => {
      document.removeEventListener("focusin", sync);
      document.removeEventListener("focusout", later);
    };
  }, [field]);
  return (
    <Button
      variant="ghost"
      size="icon"
      data-keyboard-switch
      data-on={up}
      aria-pressed={up}
      aria-label={up ? gui.keyboard_hide : gui.keyboard_show}
      onMouseDown={(e) => e.preventDefault()}
      onClick={() => {
        const el = field.current;
        if (!el) return;
        if (up) el.blur();
        else el.focus({ preventScroll: true });
      }}
      className={cn("min-h-touch min-w-touch flex-none", up && "bg-fill-strong")}
    >
      <IconKeyboard />
    </Button>
  );
}
