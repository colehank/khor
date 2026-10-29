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
// **Four parts.** Pinned at the left end, 📎, the files panel (`AttachKey`,
// the owner 2026-10-07 12:30). The caps a person chose (`lib/term-keys`,
// default the row of 2026-10-02) scroll sideways, and the last of them is
// always 「…」, **a panel in the keyboard's place** with every key, the
// row's editor and the keyboard's own settings (`KeyPanel`,
// `lib/key-panel`; UX-31 — it was a sheet, which took the focus and
// dropped the keyboard). Pinned at the right end, the cursor pad
// (`views/CursorPad`) alone. 📎 and 「…」 are one band's two panels: each
// is on while its own is up, and pressed again puts it away.
//
// **Real bytes, the terminal's own road.** A key here is handed to xterm
// as if typed (`Terminal.input`, `wasUserInput`), so it leaves through
// `onData` → `term_key` like every other keystroke and scrolls the screen
// back to the cursor the way typing does. What each key sends, and how
// the program's cursor-key mode and the latches change it, is
// `lib/term-keys`. Paste reads the clipboard and hands the text to xterm's
// own paste (`Terminal.paste`), which brackets it when the program asked
// for that (DECSET 2004) — the same path ⌘V takes on a Mac; a picture
// with no text goes down the drop's road instead (`lib/term-paste`).
//
// **Ctrl and Alt are latches.** Pressed, one waits for the next key — from
// this row, the sheet or the soft keyboard (`TerminalPane` applies them to
// what xterm hands out) — sends that key's modified form, and lets go.
// Pressed again while waiting, it lets go with nothing sent. **Pressed
// twice at once, it locks** (item 69, Blink's rule): it stays for every
// key until pressed again (`lib/term-keys` `pressLatch`). Waiting, the cap
// is the selected row's grey block in full ink; locked, the same block in
// the rail's open ink (`rail-on-ink`) — two states, two looks, no new
// colour, never the brand. Both say so (`aria-pressed`, `data-locked`).
//
// **A key never takes the focus.** Pressing a cap must not move the caret
// out of the terminal, or the keyboard would drop on every press — so a
// press is answered on `mousedown` with `preventDefault` (WebKit's tap
// focuses on its synthesized mousedown) and acted on at the click.
import { useEffect, useRef, useState, type KeyboardEvent, type RefObject } from "react";
import { createPortal } from "react-dom";
import type { Terminal } from "@xterm/xterm";

import { IconAttach, IconChevronDown, IconChevronRight, IconChevronUp, IconGrip, IconKeyboard, IconKeyRow, IconMore } from "@/components/icons";
import { GlassCircle } from "@/components/Glass";
import { Switch } from "@/components/settings/Switch";
import { Dialog, TaskOverlay, DialogTitle } from "@/components/ui/dialog";
import { SegmentedControl } from "@/components/ui/segmented-control";
import { gui } from "@/gen/catalog";
import { afterKey, GROUPS, KEYS, keyDefinition, pressLatch, setKeyRow, useKeyRow, type KeyDef, type Mods } from "@/lib/term-keys";
import { pressKeyAction } from "@/lib/term-key-codec";
import type { KeyRowEntry } from "@/lib/term-key-model";
import { keyPanelGone, toggleKeyPanel, useKeyPanel, useKeyPanelKind, type PanelKind } from "@/lib/key-panel";
import { keyboardUp, lowerKeyboard, onKeyboardChange, raiseKeyboard } from "@/lib/soft-keyboard";
import { useHeldKey } from "@/lib/key-repeat";
import { pasteInto } from "@/lib/term-paste";
import { setTermOptionMeta, useTermOptionMeta } from "@/lib/term-option-meta";
import { setTermWake, useTermWake, WAKES, type Wake } from "@/lib/term-tap";
import { cn } from "@/lib/utils";
import { CursorKey } from "@/views/CursorPad";
import { TermFiles, type FilesRoad } from "@/views/TermFiles";

/** A cap's two looks: its box is the finger's floor, the cap drawn inside
    it (`p-xxs` round a `radius-xs` cap, the key cap's corner); held, it
    wears the app's 「on」 grey. */
const CAP_BOX = "flex h-key-cap min-w-key-cap-w flex-none items-stretch outline-none focus-visible:[&>span]:ring-2 focus-visible:[&>span]:ring-ring";
const capFace = (held: boolean, locked = false) =>
  cn(
    "flex flex-1 items-center justify-center rounded-lg px-2.5 text-callout font-label",
    locked ? "bg-fill-strong text-[var(--rail-on-ink)]" : held ? "bg-fill-strong text-foreground" : "bg-fill text-foreground",
  );

/** Whether a latch key is the one waiting, and whether it is locked. */
const heldOf = (k: KeyDef, mods: Mods) => (k.name === "ctrl" && mods.ctrl) || (k.name === "alt" && mods.alt);
const lockedOf = (k: KeyDef, mods: Mods) => (k.name === "ctrl" || k.name === "alt") && !!mods.lock?.[k.name] && heldOf(k, mods);

/** What pressing a key does: a latch flips, paste reads the clipboard,
    anything else is sent under the latches, which let go first —
    `TerminalPane` reads them for whatever xterm hands out, and these
    bytes are already in their modified form. */
function pressKey(k: KeyDef, t: Terminal, mods: Mods, onMods: (m: Mods) => void, images: (files: File[]) => void) {
  if (k.action) {
    pressKeyAction(t, k.action);
    return;
  }
  if (k.name === "ctrl" || k.name === "alt") {
    onMods(pressLatch(mods, k.name, performance.now()));
    return;
  }
  if (k.name === "paste") {
    // A picture with no text goes down the drop's road (`lib/term-paste`).
    pasteInto(t, images);
    return;
  }
  const bytes = k.send?.(t, mods);
  if (!bytes) return;
  if (mods.ctrl || mods.alt) onMods(afterKey(mods));
  t.input(bytes, true);
}

/** One cap: the row's and the sheet's. A key a keyboard repeats repeats
    while held (`lib/key-repeat`), under the latches it went out with. */
function Cap({ k, mods, onPress, className }: { k: KeyDef; mods: Mods; onPress: (under: Mods) => void; className?: string }) {
  const latch = k.name === "ctrl" || k.name === "alt";
  const held = heldOf(k, mods);
  const locked = lockedOf(k, mods);
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
      data-locked={locked || undefined}
      aria-pressed={latch ? held : undefined}
      aria-label={k.label}
      // The caret stays in the terminal (the file head).
      onMouseDown={(e) => e.preventDefault()}
      {...hold}
      className={cn(CAP_BOX, k.repeats && "select-none [-webkit-touch-callout:none]", className)}
    >
      <span className={capFace(held, locked)}>{k.cap}</span>
    </button>
  );
}

export function TermKeys({
  term,
  mods,
  onMods,
  files,
}: {
  /** The terminal the keys go into. Null until it has opened. */
  term: RefObject<Terminal | null>;
  /** What 📎's panel puts files in through (`TermFiles`). */
  files: FilesRoad;
  /** The latches — held by `TerminalPane`, because a key from the soft
      keyboard has to be able to take them too. */
  mods: Mods;
  onMods: (m: Mods) => void;
}) {
  const row = useKeyRow();
  const shown = row.filter((s) => s.on).flatMap((s) => keyDefinition(s) ?? []);
  const press = (k: KeyDef, under: Mods) => {
    const t = term.current;
    // A repeat after the first key: the latches have let go already.
    if (t) pressKey(k, t, under, under === mods ? onMods : () => {}, files.dropFiles);
  };
  return (
    <div
      data-term-keys
      role="toolbar"
      aria-label={gui.term_keys}
      // The canvas's key row (the redesign, 2026-10-02): rounded caps on a
      // `fill`, a row's inset at each end. The caps scroll sideways with
      // 「…」 the last of them; 📎 stays at the left end and the cursor pad
      // at the right, outside them. ⚠️ 墓碑 (item 72, the owner
      // 2026-10-07: 「终端的更多放在软键盘里面的最后吧」, then 「进横滑区的末
      // 尾」): 「…」 was pinned at the right end beside the pad.
      className="flex h-key-row flex-none items-center gap-1.5 px-row"
    >
      <AttachKey term={term} />
      <div data-term-keys-scroll className="flex min-w-0 flex-1 items-center gap-1.5 self-stretch overflow-x-auto">
        {shown.map((k) => (
          <Cap key={k.name} k={k} mods={mods} onPress={(under) => press(k, under)} />
        ))}
        <KeyPanel term={term} mods={mods} onMods={onMods} files={files} />
      </div>
      <CursorKey term={term} mods={mods} onMods={onMods} />
    </div>
  );
}

/** The press of a panel's own key: its panel up, turned to, or put away
    (`lib/key-panel`). */
function panelKey(term: RefObject<Terminal | null>, want: PanelKind) {
  const el = term.current?.textarea;
  if (el) toggleKeyPanel(el, want);
}

/**
 * **📎, pinned at the row's left end** (the owner 2026-10-07 12:30: 「回形
 * 针常驻最左边」). The files panel (`TermFiles`, UX-41) in the keyboard's
 * band, the band 「…」 uses; on while it is up, the way 「…」 is, and
 * pressed again it puts the panel away. ⚠️ 墓碑: a key of the editable
 * row, scrolled with the rest and never lit.
 */
function AttachKey({ term }: { term: RefObject<Terminal | null> }) {
  const raised = useKeyPanel();
  const kind = useKeyPanelKind();
  const on = raised && kind === "files";
  return (
    <button
      type="button"
      data-term-key="attach"
      data-on={on || undefined}
      aria-label={gui.key_attach_name}
      aria-expanded={on}
      onMouseDown={(e) => e.preventDefault()}
      onClick={() => panelKey(term, "files")}
      className={CAP_BOX}
    >
      <span className={capFace(on, on)}>
        <IconAttach className="size-5" />
      </span>
    </button>
  );
}

/**
 * **「…」: a panel in the keyboard's place** (UX-31, the owner 2026-10-06;
 * `lib/key-panel`). The scrolled caps' last, always (item 72), and the
 * row's own: not in the editor's list. A press puts the panel
 * where the keyboard is, as tall as it, with every key in four groups —
 * 移动, 修饰, 符号, 功能键 — and the two jumps between prompts; a press
 * there sends the key the way the row does and leaves the panel up, so
 * several go in a row. Under the keys, the keyboard's own matters and
 * nothing else: 键盘唤醒, Option as Meta, and 编辑这一行, which turns the
 * panel into the row's editor in place (`KeyRowList`). 「…」 pressed
 * again puts it away: back to the keyboard it replaced, else down.
 * Nothing here takes the focus: the terminal keeps it throughout.
 * ⚠️ 墓碑 (the owner 2026-10-07 12:30: 「不要在最下面搞个键盘，用户点不
 * 到」): a 「键盘」 key at the panel's foot, by the home indicator.
 * ⚠️ 墓碑 (2026-10-03): a bottom sheet (`KeySheet`), which took the focus
 * — the keyboard dropped under it, and its drop closed the sheet.
 */
function KeyPanel({
  term,
  mods,
  onMods,
  files,
}: {
  term: RefObject<Terminal | null>;
  mods: Mods;
  onMods: (m: Mods) => void;
  files: FilesRoad;
}) {
  const raised = useKeyPanel();
  const kind = useKeyPanelKind();
  /** 「…」 is on only while the keys are the panel up (📎 has the files). */
  const up = raised && kind === "keys";
  const [editing, setEditing] = useState(false);
  const press = (k: KeyDef, under: Mods) => {
    const t = term.current;
    if (t) pressKey(k, t, under, under === mods ? onMods : () => {}, files.dropFiles);
  };
  // The row gone (collapsed, the page left) takes the panel with it.
  useEffect(() => () => keyPanelGone(), []);
  useEffect(() => {
    if (!up) setEditing(false);
  }, [up]);
  return (
    <>
      <button
        type="button"
        data-term-key="more"
        data-on={up || undefined}
        aria-label={gui.key_more}
        aria-expanded={up}
        onMouseDown={(e) => e.preventDefault()}
        onClick={() => panelKey(term, "keys")}
        className={CAP_BOX}
      >
        <span className={capFace(up, up)}>
          <IconMore className="size-5" />
        </span>
      </button>
      {raised &&
        createPortal(
          <div
            data-key-panel
            role="group"
            aria-label={gui.key_more}
            // Nothing on the panel takes the focus from the terminal (the
            // caps' own rule, for the switches and the editor too).
            onMouseDown={(e) => e.preventDefault()}
            // **In the keyboard's band**: fixed to the layout viewport's
            // foot, as tall as the foot the panel holds (`--kb`,
            // `lib/viewport`), under the shell that ends above it. The
            // home indicator's room is its own, as a keyboard's is.
            className="fixed inset-x-0 bottom-0 z-40 flex h-[var(--kb)] flex-col border-t bg-background pb-safe-bottom font-sans text-foreground"
          >
            {kind === "files" ? (
              <TermFiles road={files} />
            ) : editing ? (
              <>
                <div className="flex flex-none items-center gap-row px-row">
                  <span className="min-w-0 flex-1 text-body font-label">{gui.term_key_row}</span>
                  <button
                    type="button"
                    data-key-row-done
                    onClick={() => setEditing(false)}
                    className="flex min-h-touch flex-none items-center text-body font-label"
                  >
                    {gui.edit_done}
                  </button>
                </div>
                <div data-key-panel-scroll className="min-h-0 flex-1 touch-pan-y overflow-y-auto overscroll-contain px-row">
                  <KeyRowList />
                </div>
              </>
            ) : (
              <div data-key-panel-scroll className="flex min-h-0 flex-1 touch-pan-y flex-col gap-base overflow-y-auto overscroll-contain px-row pt-in">
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
                {/* From prompt to prompt (roadmap 2.3): the phone's ⌘↑ ⌘↓;
                    and the machine's commands, searched (roadmap 2.6), the
                    phone's ⌘⇧;. */}
                <section data-term-key-group="commands" className="flex flex-col gap-in">
                  <h3 className="text-group text-muted-foreground">{gui.key_group_commands}</h3>
                  <div className="flex flex-wrap gap-1.5">
                    {([-1, 1] as const).map((dir) => (
                      <button
                        key={dir}
                        type="button"
                        data-term-key={dir < 0 ? "command-prev" : "command-next"}
                        aria-label={dir < 0 ? gui.command_prev : gui.command_next}
                        onClick={() => {
                          const box = term.current?.element?.closest("[data-terminal]") as
                            | (HTMLElement & { __jump?: (d: -1 | 1) => void })
                            | null
                            | undefined;
                          box?.__jump?.(dir);
                        }}
                        className={CAP_BOX}
                      >
                        <span className={capFace(false)}>
                          {dir < 0 ? <IconChevronUp className="size-5" /> : <IconChevronDown className="size-5" />}
                        </span>
                      </button>
                    ))}
                    <button
                      type="button"
                      data-term-key="history"
                      onClick={() => {
                        const box = term.current?.element?.closest("[data-terminal]") as
                          | (HTMLElement & { __history?: () => void })
                          | null
                          | undefined;
                        box?.__history?.();
                      }}
                      className={CAP_BOX}
                    >
                      <span className={capFace(false)}>{gui.command_history}</span>
                    </button>
                  </div>
                </section>
                <KeyboardMatters onEdit={() => setEditing(true)} />
              </div>
            )}
          </div>,
          document.body,
        )}
    </>
  );
}

/**
 * **The keyboard's own matters, under the keys** (UX-31): 键盘唤醒 (自动 /
 * 点击屏幕 / 点击按钮, `lib/term-tap`), Option as Meta (`lib/term-option-meta`,
 * a hardware keyboard's) and the way into the row's editor. The same
 * settings 外观 → 终端 keeps; this is where a person is when they want
 * them.
 */
function KeyboardMatters({ onEdit }: { onEdit: () => void }) {
  const wake = useTermWake();
  const optionMeta = useTermOptionMeta();
  const row = "flex min-h-touch items-center gap-row border-t";
  return (
    <section data-key-panel-matters className="flex flex-col">
      <div className={row} data-setting-row-name="term-wake">
        <span className="min-w-0 flex-1 text-body">{gui.term_wake}</span>
        <SegmentedControl
          label={gui.term_wake}
          value={wake}
          onChange={(w) => setTermWake(w as Wake)}
          mark={{ "data-key-panel-wake": true }}
          options={WAKES.map((w) => ({
            key: w,
            label: { auto: gui.term_wake_auto, screen: gui.term_wake_screen, button: gui.term_wake_button }[w],
          }))}
        />
      </div>
      <div className={row} data-setting-row-name="term-option-meta">
        <span className="min-w-0 flex-1 text-body">{gui.term_option_meta}</span>
        <Switch on={optionMeta} label={gui.term_option_meta} onToggle={() => setTermOptionMeta(!optionMeta)} />
      </div>
      <button type="button" data-term-keys-edit onClick={onEdit} className={cn(row, "w-full text-left")}>
        <span className="min-w-0 flex-1 text-body">{gui.keys_edit_row}</span>
        <IconChevronRight className="size-affordance text-tertiary" />
      </button>
    </section>
  );
}

/** A key's spoken name in the editor: its own where it has one. */
const nameOf = (k: KeyDef) => k.label ?? k.cap;

/**
 * **The row's editor** (2026-10-03): every key the panel offers, and 粘贴,
 * each with a switch (in the row or not) and a grip to move it — iOS's
 * own list for this, the one 外观's navigation order already is. Kept at
 * once (`lib/term-keys`); the row follows. 📎, the cursor pad and 「…」
 * are the row's own and not listed. A sheet from 外观 → 终端; in place inside
 * the key panel (`KeyRowList`, UX-31).
 */
export function KeyRowEditor({ open, onOpenChange }: { open: boolean; onOpenChange: (open: boolean) => void }) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <TaskOverlay
        purpose="edit" initialHeight="medium" maxHeight="large" dismiss="outside"
        data-key-row-editor
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
        <KeyRowList />
      </TaskOverlay>
    </Dialog>
  );
}

/** The editor's list: the seats, their switches and grips. */
function KeyRowList() {
  const row = useKeyRow();
  const box = useRef<HTMLDivElement>(null);
  const [dragging, setDragging] = useState<string | null>(null);
  const move = (name: string, to: number) => {
    const from = row.findIndex((s) => s.id === name);
    const at = Math.max(0, Math.min(row.length - 1, to));
    if (from < 0 || from === at) return;
    const next = row.filter((s) => s.id !== name);
    next.splice(at, 0, row[from]);
    setKeyRow(next);
  };
  const toggle = (name: string) => setKeyRow(row.map((s): KeyRowEntry => (s.id === name ? { ...s, on: !s.on } : s)));
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
    <div ref={box} data-key-row-list className="flex flex-col">
      {row.map((s) => {
        const k = keyDefinition(s);
        if (!k) return null;
        return (
          <div
            key={s.id}
            data-key-row-seat={s.id}
            data-dragging={dragging === s.id || undefined}
            className={cn("flex min-h-touch items-center gap-row border-b", dragging === s.id && "bg-fill")}
          >
            <span className="min-w-0 flex-1 truncate text-body">{k.cap}</span>
            <Switch on={s.on} label={nameOf(k)} onToggle={() => toggle(s.id)} />
            <button
              type="button"
              data-key-row-grip
              data-sheet-hold
              aria-label={nameOf(k)}
              aria-keyshortcuts="ArrowUp ArrowDown"
              className="grid min-h-touch min-w-touch flex-none cursor-grab touch-none place-items-center rounded-full text-tertiary [&_svg]:size-4"
              onPointerDown={(e) => {
                e.currentTarget.setPointerCapture(e.pointerId);
                setDragging(s.id);
              }}
              onPointerMove={(e) => {
                if (dragging === s.id) move(s.id, slotAt(s.id, e.clientY));
              }}
              onPointerUp={() => setDragging(null)}
              onPointerCancel={() => setDragging(null)}
              onKeyDown={(e: KeyboardEvent<HTMLButtonElement>) => {
                if (e.key !== "ArrowUp" && e.key !== "ArrowDown") return;
                e.preventDefault();
                move(s.id, row.findIndex((r) => r.id === s.id) + (e.key === "ArrowUp" ? -1 : 1));
              }}
            >
              <IconGrip />
            </button>
          </div>
        );
      })}
    </div>
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
