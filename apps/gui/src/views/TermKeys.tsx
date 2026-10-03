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
// (`CursorKey`) and 「…」, a sheet with every key and the row's editor
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

import { IconArrow, IconGrip, IconKeyboard, IconMore, IconMove } from "@/components/icons";
import { GlassCircle } from "@/components/Glass";
import { Switch } from "@/components/settings/Switch";
import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog";
import { gui } from "@/gen/catalog";
import { GROUPS, KEYS, KEY_BY_NAME, cursorKey, setKeyRow, useKeyRow, type KeyDef, type Mods, type RowSeat } from "@/lib/term-keys";
import { cn } from "@/lib/utils";

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

/** One cap: the row's and the sheet's. */
function Cap({ k, mods, onPress, className }: { k: KeyDef; mods: Mods; onPress: () => void; className?: string }) {
  const latch = k.name === "ctrl" || k.name === "alt";
  const held = heldOf(k, mods);
  return (
    <button
      type="button"
      data-term-key={k.name}
      data-on={latch ? held : undefined}
      aria-pressed={latch ? held : undefined}
      aria-label={k.label}
      // The caret stays in the terminal (the file head).
      onMouseDown={(e) => e.preventDefault()}
      onClick={onPress}
      className={cn(CAP_BOX, className)}
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
  const press = (k: KeyDef) => {
    const t = term.current;
    if (t) pressKey(k, t, mods, onMods);
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
          <Cap key={k.name} k={k} mods={mods} onPress={() => press(k)} />
        ))}
      </div>
      <CursorKey term={term} mods={mods} onMods={onMods} />
      <KeySheet term={term} mods={mods} onMods={onMods} />
    </div>
  );
}

type Way = "left" | "right" | "up" | "down";
const LETTER: Record<Way, "A" | "B" | "C" | "D"> = { up: "A", down: "B", right: "C", left: "D" };

/** Travel before anything happens: a tap, or a finger settling. */
export const PAD_DEAD = 10;
/** Travel per arrow, on either axis. */
export const PAD_STEP = 16;
/** A deflected finger at rest this long starts the repeat. */
export const PAD_HOLD_MS = 300;
/** Arrows per second while it repeats. */
export const PAD_RATE = 12;
/** Movement below this is a resting finger's tremor, not a move. */
const PAD_STILL = 3;

/** One gesture on the pad, from the finger's landing. */
type Gesture = {
  pointer: number;
  /** Where the finger landed: deflection is measured from here. */
  x0: number;
  y0: number;
  /** Where the step count starts — the landing, or the point the finger
      rested at before it moved on from a repeat. */
  base: number;
  axis: "x" | "y" | null;
  /** Steps sent since `base`, signed: coming back takes them back. */
  sent: number;
  /** Where the finger last was, for the rest timer. */
  still: number;
  rest: number;
  repeat: number;
  /** The latches as they were at the landing: the whole gesture uses them. */
  mods: Mods;
  freed: boolean;
};

/**
 * **The cursor pad: drag for steps, hold for repeat** (2026-10-03, the
 * owner on item 7's one-per-cell trackpad: 「触控板有点太灵敏了，而且位置有点
 * 不好按」). Pinned second from the right, so there is room to drag both
 * ways. A finger lands on it and the whole screen is its pad until it
 * lifts (pointer capture).
 *
 * - Nothing until the finger has travelled `PAD_DEAD`; a tap sends
 *   nothing (it is the most reachable spot on the row, and → would accept
 *   a shell's suggestion).
 * - Then the axis that dominates is the gesture's until the finger lifts.
 * - Along it, one arrow per `PAD_STEP` of travel from the landing, both
 *   axes the same; coming back sends the opposite arrow.
 * - A finger deflected past the dead zone and still for `PAD_HOLD_MS`
 *   repeats the arrow of its deflection at `PAD_RATE` a second until it
 *   moves (steps then count from where it rested) or lifts.
 *
 * Arrows go through `cursorKey`, so an editor's application mode and a
 * latched Ctrl or Alt (word jumps) hold; the latches let go at the first
 * arrow, as on every key. While a gesture is locked the cap shows that
 * way's arrow in the held grey; repeating, the arrow wears the accent.
 */
function CursorKey({ term, mods, onMods }: { term: RefObject<Terminal | null>; mods: Mods; onMods: (m: Mods) => void }) {
  const g = useRef<Gesture | null>(null);
  const [way, setWay] = useState<Way | null>(null);
  const [repeating, setRepeating] = useState(false);
  const stopTimers = (s: Gesture) => {
    window.clearTimeout(s.rest);
    window.clearInterval(s.repeat);
    s.repeat = 0;
  };
  const end = () => {
    if (g.current) stopTimers(g.current);
    g.current = null;
    setWay(null);
    setRepeating(false);
  };
  useEffect(() => end, []);
  const send = (s: Gesture, w: Way) => {
    const t = term.current;
    if (!t) return;
    setWay(w);
    if (!s.freed && (s.mods.ctrl || s.mods.alt)) {
      s.freed = true;
      onMods(NO_MODS);
    }
    t.input(cursorKey(t, LETTER[w], s.mods), true);
  };
  const wayOf = (axis: "x" | "y", sign: number): Way =>
    axis === "x" ? (sign > 0 ? "right" : "left") : sign > 0 ? "down" : "up";
  const down = (e: PointerEvent<HTMLButtonElement>) => {
    if (!term.current) return;
    e.currentTarget.setPointerCapture(e.pointerId);
    g.current = {
      pointer: e.pointerId,
      x0: e.clientX,
      y0: e.clientY,
      base: 0,
      axis: null,
      sent: 0,
      still: 0,
      rest: 0,
      repeat: 0,
      mods,
      freed: false,
    };
  };
  const move = (e: PointerEvent<HTMLButtonElement>) => {
    const s = g.current;
    if (!s || e.pointerId !== s.pointer) return;
    const dx = e.clientX - s.x0;
    const dy = e.clientY - s.y0;
    if (!s.axis) {
      if (Math.hypot(dx, dy) < PAD_DEAD) return;
      s.axis = Math.abs(dx) >= Math.abs(dy) ? "x" : "y";
      s.still = s.axis === "x" ? dx : dy;
      setWay(wayOf(s.axis, s.still));
    }
    const at = s.axis === "x" ? dx : dy;
    if (Math.abs(at - s.still) < PAD_STILL && (s.rest || s.repeat)) return;
    // It moved: a repeat ends and the steps count from where it rested.
    if (s.repeat) {
      stopTimers(s);
      setRepeating(false);
      s.base = s.still;
      s.sent = 0;
    }
    s.still = at;
    const want = Math.trunc((at - s.base) / PAD_STEP);
    while (s.sent !== want) {
      const forward = want > s.sent;
      s.sent += forward ? 1 : -1;
      send(s, wayOf(s.axis, forward ? 1 : -1));
    }
    window.clearTimeout(s.rest);
    s.rest = window.setTimeout(() => {
      s.rest = 0;
      if (g.current !== s || !s.axis || Math.abs(s.still) < PAD_DEAD) return;
      const w = wayOf(s.axis, Math.sign(s.still));
      setRepeating(true);
      send(s, w);
      s.repeat = window.setInterval(() => send(s, w), 1000 / PAD_RATE);
    }, PAD_HOLD_MS);
  };
  return (
    <button
      type="button"
      data-term-key="cursor"
      data-way={way ?? undefined}
      data-repeating={repeating || undefined}
      aria-label={gui.key_cursor}
      onMouseDown={(e) => e.preventDefault()}
      onPointerDown={down}
      onPointerMove={move}
      onPointerUp={end}
      onPointerCancel={end}
      onLostPointerCapture={end}
      // A pad, not a cap: wider, the same fill. The finger owns it: no
      // page scroll, no sideways scroll of the row under it.
      className="flex h-key-cap w-key-pad-w flex-none touch-none items-stretch outline-none focus-visible:[&>span]:ring-2 focus-visible:[&>span]:ring-ring"
    >
      <span
        className={cn(
          "flex flex-1 items-center justify-center rounded-lg",
          way ? "bg-fill-strong" : "bg-fill",
          repeating ? "text-[var(--rail-on-ink)]" : "text-foreground",
        )}
      >
        {way ? <IconArrow to={way} className="size-5" /> : <IconMove className="size-5" />}
      </span>
    </button>
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
  const press = (k: KeyDef) => {
    const t = term.current;
    if (t) pressKey(k, t, mods, onMods);
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
                  <Cap key={k.name} k={k} mods={mods} onPress={() => press(k)} />
                ))}
              </div>
            </section>
          ))}
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
    <GlassCircle
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
      className={cn(up && "text-[var(--rail-on-ink)]")}
    >
      <IconKeyboard />
    </GlassCircle>
  );
}
