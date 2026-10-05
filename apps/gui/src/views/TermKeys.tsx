// The keys a soft keyboard lacks, and the switch for the keyboard itself
// (2026-10-02, iOS 第三步 A; the row made editable 2026-10-03).
//
// A phone's keyboard types text. A terminal also wants Esc, Ctrl, Tab
// and Shift-Tab, the arrows, and a handful of characters a shell lives
// on that sit two layers deep (`/ - | :`). Every phone terminal grows the
// same row over the keyboard; this is that row, drawn **only under a
// finger** — the pointer decides the touch things, never the width
// (docs/UX.md): a Mac window dragged narrow has a real keyboard and no
// row, an iPad on the wide face has the row under its terminal.
//
// **Three parts.** The caps a person chose (`lib/term-keys`, default the
// row of 2026-10-02) scroll sideways; after them, pinned, the cursor pad
// (`views/CursorPad`) and 「…」, a sheet with every key and the row's editor
// (`KeySheet`, `KeyRowEditor`).
//
// **Real bytes, the terminal's own road.** A key here is handed to xterm
// as if typed (`Terminal.input`, `wasUserInput`), so it leaves through
// `onData` → `term_key` like every other keystroke and scrolls the screen
// back to the cursor the way typing does. What each key sends, and how
// the program's cursor-key mode and the latches change it, is
// `lib/term-keys`. Paste reads the clipboard and hands the text to xterm's
// own paste (`Terminal.paste`), which brackets it when the program asked
// for that (DECSET 2004) — the same path ⌘V takes on a Mac.
//
// **Ctrl and Alt are latches.** Pressed, one waits for the next key — from
// this row, the sheet or the soft keyboard (`TerminalPane` applies them to
// what xterm hands out) — sends that key's modified form, and lets go.
// Pressed again while waiting, it lets go with nothing sent. It shows that
// it is waiting as a grey cap (a held key, the app's 「on」 grey — never
// the brand) and says so (`aria-pressed`).
//
// **A key never takes the focus.** Pressing a cap must not move the caret
// out of the terminal, or the keyboard would drop on every press — so a
// press is answered on `mousedown` with `preventDefault` (WebKit's tap
// focuses on its synthesized mousedown) and acted on at the click.
import { useEffect, useRef, useState, type KeyboardEvent, type PointerEvent, type RefObject } from "react";
import type { Terminal } from "@xterm/xterm";

import { IconChevronDown, IconChevronUp, IconGrip, IconKeyboard, IconKeyRow, IconMore } from "@/components/icons";
import { GlassCircle } from "@/components/Glass";
import { Switch } from "@/components/settings/Switch";
import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog";
import { gui } from "@/gen/catalog";
import { GROUPS, KEYS, KEY_BY_NAME, setKeyRow, useKeyRow, type KeyDef, type Mods, type RowSeat } from "@/lib/term-keys";
import { keyboardUp, lowerKeyboard, onKeyboardChange, raiseKeyboard } from "@/lib/soft-keyboard";
import { useHeldKey } from "@/lib/key-repeat";
import { cn } from "@/lib/utils";
import { CursorKey } from "@/views/CursorPad";

const NO_MODS: Mods = { ctrl: false, alt: false };

/** A cap's two looks: its box is the finger's floor, the cap drawn inside
    it (`p-xxs` round a `radius-xs` cap, the key cap's corner); held, it
    wears the app's 「on」 grey. */
const CAP_BOX = "flex h-key-cap min-w-key-cap-w flex-none items-stretch outline-none focus-visible:[&>span]:ring-2 focus-visible:[&>span]:ring-ring";
const capFace = (held: boolean) =>
  cn(
    "flex flex-1 items-center justify-center rounded-lg px-2.5 text-callout font-label",
    held ? "bg-fill-strong text-[var(--rail-on-ink)]" : "bg-fill text-foreground",
  );

/** Whether a latch key is the one waiting. */
const heldOf = (k: KeyDef, mods: Mods) => (k.name === "ctrl" && mods.ctrl) || (k.name === "alt" && mods.alt);

/** What pressing a key does: a latch flips, paste reads the clipboard,
    anything else is sent under the latches, which let go first —
    `TerminalPane` reads them for whatever xterm hands out, and these
    bytes are already in their modified form. */
function pressKey(k: KeyDef, t: Terminal, mods: Mods, onMods: (m: Mods) => void) {
  if (k.name === "ctrl" || k.name === "alt") {
    onMods({ ...mods, [k.name]: !mods[k.name] });
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
  const bytes = k.send?.(t, mods);
  if (!bytes) return;
  if (mods.ctrl || mods.alt) onMods(NO_MODS);
  t.input(bytes, true);
}

/** One cap: the row's and the sheet's. A key a keyboard repeats repeats
    while held (`lib/key-repeat`), under the latches it went out with. */
function Cap({ k, mods, onPress, className }: { k: KeyDef; mods: Mods; onPress: (under: Mods) => void; className?: string }) {
  const latch = k.name === "ctrl" || k.name === "alt";
  const held = heldOf(k, mods);
  const under = useRef(mods);
  const hold = useHeldKey((first) => {
    if (first) under.current = mods;
    onPress(under.current);
  }, k.repeats === true);
  return (
    <button
      type="button"
      data-term-key={k.name}
      data-on={latch ? held : undefined}
      aria-pressed={latch ? held : undefined}
      aria-label={k.label}
      // The caret stays in the terminal (the file head).
      onMouseDown={(e) => e.preventDefault()}
      {...hold}
      className={cn(CAP_BOX, k.repeats && "select-none [-webkit-touch-callout:none]", className)}
    >
      <span className={capFace(held)}>{k.cap}</span>
    </button>
  );
}

export function TermKeys({
  term,
  mods,
  onMods,
}: {
  /** The terminal the keys go into. Null until it has opened. */
  term: RefObject<Terminal | null>;
  /** The latches — held by `TerminalPane`, because a key from the soft
      keyboard has to be able to take them too. */
  mods: Mods;
  onMods: (m: Mods) => void;
}) {
  const row = useKeyRow();
  const shown = row.filter((s) => s.on).flatMap((s) => KEY_BY_NAME.get(s.name) ?? []);
  const press = (k: KeyDef, under: Mods) => {
    const t = term.current;
    // A repeat after the first key: the latches have let go already.
    if (t) pressKey(k, t, under, under === mods ? onMods : () => {});
  };
  return (
    <div
      data-term-keys
      role="toolbar"
      aria-label={gui.term_keys}
      // The canvas's key row (the redesign, 2026-10-02): rounded caps on a
      // `fill`, a row's inset at each end. The caps scroll sideways; the
      // cursor pad and 「…」 stay at the right end, outside them.
      className="flex h-key-row flex-none items-center gap-1.5 pr-row"
    >
      <div data-term-keys-scroll className="flex min-w-0 flex-1 items-center gap-1.5 self-stretch overflow-x-auto pl-row">
        {shown.map((k) => (
          <Cap key={k.name} k={k} mods={mods} onPress={(under) => press(k, under)} />
        ))}
      </div>
      <CursorKey term={term} mods={mods} onMods={onMods} />
      <KeySheet term={term} mods={mods} onMods={onMods} />
    </div>
  );
}

/**
 * **「…」: every key, and the row's editor** (2026-10-03). The row's last
 * seat, pinned. A press opens a sheet with every key in four groups —
 * 移动, 修饰, 符号, 功能键 — and a press there sends the key the way the
 * row does and leaves the sheet open, so several go in a row. A press
 * outside or on the grabber puts it away. Its last line opens the row's
 * editor (`KeyRowEditor`).
 */
function KeySheet({ term, mods, onMods }: { term: RefObject<Terminal | null>; mods: Mods; onMods: (m: Mods) => void }) {
  const [open, setOpen] = useState(false);
  const [editing, setEditing] = useState(false);
  const press = (k: KeyDef, under: Mods) => {
    const t = term.current;
    if (t) pressKey(k, t, under, under === mods ? onMods : () => {});
  };
  return (
    <>
      <button
        type="button"
        data-term-key="more"
        aria-label={gui.key_more}
        aria-haspopup="dialog"
        onMouseDown={(e) => e.preventDefault()}
        onClick={() => setOpen(true)}
        className={CAP_BOX}
      >
        <span className={capFace(false)}>
          <IconMore className="size-5" />
        </span>
      </button>
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent
          form
          data-term-key-sheet
          // Unlike a form, a press outside puts this one away: there is
          // nothing typed in it to lose.
          onInteractOutside={() => {}}
          onPointerUp={(e) => onGrabber(e, () => setOpen(false))}
        >
          <DialogTitle className="sr-only">{gui.key_more}</DialogTitle>
          {GROUPS.map(({ group, label }) => (
            <section key={group} data-term-key-group={group} className="flex flex-col gap-in">
              <h3 className="text-group text-muted-foreground">{label}</h3>
              <div className="flex flex-wrap gap-1.5">
                {KEYS.filter((k) => k.group === group).map((k) => (
                  <Cap key={k.name} k={k} mods={mods} onPress={(under) => press(k, under)} />
                ))}
              </div>
            </section>
          ))}
          {/* From prompt to prompt (roadmap 2.3): the phone's ⌘↑ ⌘↓. The
              sheet goes away so the jump can be seen. */}
          <section data-term-key-group="commands" className="flex flex-col gap-in">
            <h3 className="text-group text-muted-foreground">{gui.key_group_commands}</h3>
            <div className="flex flex-wrap gap-1.5">
              {([-1, 1] as const).map((dir) => (
                <button
                  key={dir}
                  type="button"
                  data-term-key={dir < 0 ? "command-prev" : "command-next"}
                  aria-label={dir < 0 ? gui.command_prev : gui.command_next}
                  onMouseDown={(e) => e.preventDefault()}
                  onClick={() => {
                    const box = term.current?.element?.closest("[data-terminal]") as
                      | (HTMLElement & { __jump?: (d: -1 | 1) => void })
                      | null
                      | undefined;
                    box?.__jump?.(dir);
                    setOpen(false);
                  }}
                  className={CAP_BOX}
                >
                  <span className={capFace(false)}>
                    {dir < 0 ? <IconChevronUp className="size-5" /> : <IconChevronDown className="size-5" />}
                  </span>
                </button>
              ))}
              {/* The machine's commands, searched (roadmap 2.6): the
                  phone's ⌘⇧;. */}
              <button
                type="button"
                data-term-key="history"
                onMouseDown={(e) => e.preventDefault()}
                onClick={() => {
                  const box = term.current?.element?.closest("[data-terminal]") as
                    | (HTMLElement & { __history?: () => void })
                    | null
                    | undefined;
                  setOpen(false);
                  box?.__history?.();
                }}
                className={CAP_BOX}
              >
                <span className={capFace(false)}>{gui.command_history}</span>
              </button>
            </div>
          </section>
          <button
            type="button"
            data-term-keys-edit
            onClick={() => {
              setOpen(false);
              setEditing(true);
            }}
            className="flex min-h-touch items-center border-t text-body"
          >
            {gui.keys_edit_row}
          </button>
        </DialogContent>
      </Dialog>
      <KeyRowEditor open={editing} onOpenChange={setEditing} />
    </>
  );
}

/** A press on a sheet's grabber band (its top 28, as `useSheet` reads
    it) that is not on a control puts the sheet away. */
function onGrabber(e: PointerEvent<HTMLDivElement>, close: () => void) {
  const top = e.currentTarget.getBoundingClientRect().top;
  if (e.clientY - top < 28 && !(e.target instanceof Element && e.target.closest("button"))) close();
}

/** A key's spoken name in the editor: its own where it has one. */
const nameOf = (k: KeyDef) => k.label ?? k.cap;

/**
 * **The row's editor** (2026-10-03): every key the sheet offers, and 粘贴,
 * each with a switch (in the row or not) and a grip to move it — iOS's
 * own list for this, the one 外观's navigation order already is. Kept at
 * once (`lib/term-keys`); the row follows. The cursor pad and 「…」 are
 * the row's own and not listed. Opened from the sheet and from 外观 → 终端.
 */
export function KeyRowEditor({ open, onOpenChange }: { open: boolean; onOpenChange: (open: boolean) => void }) {
  const row = useKeyRow();
  const box = useRef<HTMLDivElement>(null);
  const [dragging, setDragging] = useState<string | null>(null);
  const move = (name: string, to: number) => {
    const from = row.findIndex((s) => s.name === name);
    const at = Math.max(0, Math.min(row.length - 1, to));
    if (from < 0 || from === at) return;
    const next = row.filter((s) => s.name !== name);
    next.splice(at, 0, row[from]);
    setKeyRow(next);
  };
  const toggle = (name: string) => setKeyRow(row.map((s): RowSeat => (s.name === name ? { ...s, on: !s.on } : s)));
  /** Where a row dragged to `y` lands: after every other row whose middle
      is above the pointer. */
  const slotAt = (name: string, y: number): number => {
    let slot = 0;
    for (const r of box.current?.querySelectorAll<HTMLElement>("[data-key-row-seat]") ?? []) {
      if (r.dataset.keyRowSeat === name) continue;
      const b = r.getBoundingClientRect();
      if (y > b.top + b.height / 2) slot += 1;
    }
    return slot;
  };
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        form
        data-key-row-editor
        onInteractOutside={() => {}}
        onPointerUp={(e) => onGrabber(e, () => onOpenChange(false))}
      >
        <div className="flex items-center gap-row">
          <DialogTitle className="min-w-0 flex-1">{gui.term_key_row}</DialogTitle>
          <button
            type="button"
            data-key-row-done
            onClick={() => onOpenChange(false)}
            className="flex min-h-touch flex-none items-center text-body font-label"
          >
            {gui.edit_done}
          </button>
        </div>
        <div ref={box} className="flex flex-col">
          {row.map((s) => {
            const k = KEY_BY_NAME.get(s.name);
            if (!k) return null;
            return (
              <div
                key={s.name}
                data-key-row-seat={s.name}
                data-dragging={dragging === s.name || undefined}
                className={cn("flex min-h-touch items-center gap-row border-b", dragging === s.name && "bg-fill")}
              >
                <span className="min-w-0 flex-1 truncate text-body">{k.cap}</span>
                <Switch on={s.on} label={nameOf(k)} onToggle={() => toggle(s.name)} />
                <button
                  type="button"
                  data-key-row-grip
                  data-sheet-hold
                  aria-label={nameOf(k)}
                  aria-keyshortcuts="ArrowUp ArrowDown"
                  className="grid min-h-touch min-w-touch flex-none cursor-grab touch-none place-items-center rounded-full text-tertiary [&_svg]:size-4"
                  onPointerDown={(e) => {
                    e.currentTarget.setPointerCapture(e.pointerId);
                    setDragging(s.name);
                  }}
                  onPointerMove={(e) => {
                    if (dragging === s.name) move(s.name, slotAt(s.name, e.clientY));
                  }}
                  onPointerUp={() => setDragging(null)}
                  onPointerCancel={() => setDragging(null)}
                  onKeyDown={(e: KeyboardEvent<HTMLButtonElement>) => {
                    if (e.key !== "ArrowUp" && e.key !== "ArrowDown") return;
                    e.preventDefault();
                    move(s.name, row.findIndex((r) => r.name === s.name) + (e.key === "ArrowUp" ? -1 : 1));
                  }}
                >
                  <IconGrip />
                </button>
              </div>
            );
          })}
        </div>
      </DialogContent>
    </Dialog>
  );
}

/**
 * The soft keyboard's one switch: up when it is down, down when it is up
 * — in the bar's first row, at its right end, which is where R0 keeps the
 * open thing's verbs (`components/SessionBar`). Only under a finger: a
 * mouse has a real keyboard and nothing to raise. It raises and lowers
 * the keyboard without moving the focus (`lib/soft-keyboard`); up, it
 * wears the app's 「on」 grey.
 */
export function KeyboardSwitch({ field }: { field: RefObject<HTMLElement | null> }) {
  const [up, setUp] = useState(false);
  // "Up" is the field's own mode, not the focus: the terminal holds the
  // focus whether or not a keyboard is showing (`lib/soft-keyboard`).
  useEffect(() => {
    const sync = () => setUp(keyboardUp(field.current));
    sync();
    return onKeyboardChange(sync);
  }, [field]);
  return (
    <GlassCircle
      data-keyboard-switch
      data-on={up}
      aria-pressed={up}
      aria-label={up ? gui.keyboard_hide : gui.keyboard_show}
      onMouseDown={(e) => e.preventDefault()}
      onClick={() => {
        const el = field.current;
        if (!el) return;
        if (up) lowerKeyboard(el);
        else raiseKeyboard(el);
      }}
      className={cn(up && "text-[var(--rail-on-ink)]")}
    >
      <IconKeyboard />
    </GlassCircle>
  );
}

/**
 * **The key row's circle on the deck** (UX-12, 2026-10-04): opens and
 * closes the row under the deck (`lib/key-row-open`), with the keyboard
 * up or down. It moves no focus, so a keyboard that is up stays up; open,
 * it wears the app's 「on」 grey, as the keyboard's switch does. That one
 * (点击按钮 alone) raises the system keyboard; this one is khor's row.
 */
export function KeyRowSwitch({ open, onToggle }: { open: boolean; onToggle: () => void }) {
  return (
    <GlassCircle
      data-key-row-switch
      data-on={open}
      aria-expanded={open}
      aria-label={open ? gui.key_row_hide : gui.key_row_show}
      onMouseDown={(e) => e.preventDefault()}
      onClick={onToggle}
      className={cn(open && "text-[var(--rail-on-ink)]")}
    >
      <IconKeyRow />
    </GlassCircle>
  );
}
