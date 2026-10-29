// **Every key the terminal's key row can carry, and the row a person chose**
// (2026-10-03, the owner's 「…」: every key in groups, and an editor for
// the row above the keyboard).
//
// A key is a name, its cap and what it sends. What it sends is spelled the
// way xterm spells the same key from a real keyboard, so a program cannot
// tell the row from a keyboard:
//
// - the cursor and editing keys follow the program's cursor-key mode
//   (DECCKM: `ESC O H` for Home in an editor's application mode,
//   `ESC [ H` otherwise);
// - with a modifier latched, the special keys take xterm's modifier
//   parameter (`ESC [ 1 ; 5 H` for ⌃Home, `ESC [ 5 ; 3 ~` for ⌥PgUp) and a
//   character takes its control form (C → 0x03) and, for Alt, an `ESC`
//   in front — the meta form every terminal sends for ⌥.
//
// **The row is this device's** (`localStorage`, like the terminal's font
// and size in `lib/term-font` and `lib/type-size`): the keys shown and their
// order. A face that refuses storage keeps changes for this window.
import { useEffect, useState } from "react";
import type { Terminal } from "@xterm/xterm";

import { gui } from "@/gen/catalog";
import { heardFrom } from "@/lib/other-windows";
import { kittyNamed, kittyTyped, shiftEnter } from "@/lib/term-kitty";
import { actionCaption, cursorKey, tildeKey as tilde, ss3Key as ss3, typed } from "@/lib/term-key-codec";
export { cursorKey, typed } from "@/lib/term-key-codec";
import { decodeKeyRow, serializeKeyRow, type CustomKeyAction, type KeyRowEntry, type KeyRowProblem } from "@/lib/term-key-model";

export { ctrlOf } from "@/lib/term-kitty";

/** The latched modifiers the next key goes out with, and which of them
    are locked (item 69): a locked one stays after the key. */
export type Mods = { ctrl: boolean; alt: boolean; lock?: { ctrl?: boolean; alt?: boolean } };

/** How soon a second press on Ctrl or Alt locks it (iOS's double tap). */
export const LOCK_TAP_MS = 350;

/** The last press on a latch, for the double tap. */
let lastLatch: { name: "ctrl" | "alt" | null; at: number } = { name: null, at: -Infinity };

/** **A press on Ctrl or Alt** (item 69, Blink's rule): from rest it waits
    for one key; pressed again at once (within `LOCK_TAP_MS`) it locks,
    and stays for every key until pressed again; pressed again later while
    waiting, or while locked, it lets go with nothing sent. */
export function pressLatch(m: Mods, name: "ctrl" | "alt", now: number): Mods {
  const again = lastLatch.name === name && now - lastLatch.at < LOCK_TAP_MS;
  lastLatch = { name, at: now };
  const lock = { ...m.lock };
  if (m[name] && !lock[name] && again) {
    lock[name] = true;
    return { ...m, lock };
  }
  if (m[name]) {
    lock[name] = false;
    lastLatch = { name: null, at: -Infinity };
    return { ...m, [name]: false, lock };
  }
  return { ...m, [name]: true, lock };
}

/** The latches once a key went out under them: a waiting one lets go, a
    locked one stays (item 69). */
export function afterKey(m: Mods): Mods {
  return { ctrl: !!m.lock?.ctrl, alt: !!m.lock?.alt, lock: m.lock };
}

export type KeyGroup = "move" | "mod" | "symbol" | "function";

export type KeyDef = {
  /** `data-term-key`, and the row's stored name. */
  name: string;
  cap: string;
  /** The spoken name, where the cap is not one. */
  label?: string;
  group: KeyGroup | null;
  /** What the key sends under the latches; absent for a key with its own
      act (paste, the latches themselves). */
  send?: (term: Terminal, mods: Mods) => string;
  /** Held, it repeats (item 34), as a hardware keyboard repeats it: the
      characters, the editing and moving keys, Tab. Never Esc, the
      latches, paste or the function keys: a one-shot sent twice is a
      different act. */
  repeats?: true;
  action?: CustomKeyAction;
};

const SYMBOLS = ["/", "-", "|", ":", "~", "`", "^", "&", "*", "$", "#", "@", "!", "%", "(", ")", "[", "]", "{", "}", "<", ">", "=", "+", "_", ";", "'", '"', "\\", "?"];

const F_KEYS: [string, (mods: Mods) => string][] = [
  ["F1", (m) => ss3("P", m)],
  ["F2", (m) => ss3("Q", m)],
  ["F3", (m) => ss3("R", m)],
  ["F4", (m) => ss3("S", m)],
  ["F5", (m) => tilde(15, m)],
  ["F6", (m) => tilde(17, m)],
  ["F7", (m) => tilde(18, m)],
  ["F8", (m) => tilde(19, m)],
  ["F9", (m) => tilde(20, m)],
  ["F10", (m) => tilde(21, m)],
  ["F11", (m) => tilde(23, m)],
  ["F12", (m) => tilde(24, m)],
];

/** Every key, in the sheet's order within each group. */
export const KEYS: KeyDef[] = [
  { name: "home", cap: "Home", group: "move", repeats: true, send: (t, m) => cursorKey(t, "H", m) },
  { name: "end", cap: "End", group: "move", repeats: true, send: (t, m) => cursorKey(t, "F", m) },
  { name: "pgup", cap: "PgUp", group: "move", repeats: true, send: (_, m) => tilde(5, m) },
  { name: "pgdn", cap: "PgDn", group: "move", repeats: true, send: (_, m) => tilde(6, m) },
  { name: "ins", cap: "Ins", group: "move", send: (_, m) => tilde(2, m) },
  { name: "del", cap: "Del", group: "move", repeats: true, send: (_, m) => tilde(3, m) },
  // ⌫ as a keyboard sends it: DEL, ⌃⌫ as ^H, ⌥⌫ with ESC in front; under
  // the kitty protocol, as the program asked.
  {
    name: "backspace",
    cap: "⌫",
    label: gui.key_backspace_name,
    group: "move",
    repeats: true,
    send: (t, m) => kittyNamed(t, "Backspace", false, m.alt, m.ctrl) ?? (m.ctrl ? "\x08" : m.alt ? "\x1b\x7f" : "\x7f"),
  },
  { name: "ctrl", cap: gui.key_ctrl, label: gui.key_ctrl_name, group: "mod" },
  { name: "alt", cap: gui.key_alt, group: "mod" },
  // Under the kitty keyboard protocol (roadmap 5.1) these four are spelled
  // as the program asked; otherwise as xterm spells them.
  ...SYMBOLS.map((ch): KeyDef => ({ name: ch, cap: ch, group: "symbol", repeats: true, send: (t, m) => kittyTyped(t, ch, m.ctrl, m.alt) ?? typed(ch, m) })),
  {
    name: "esc",
    cap: gui.key_esc,
    label: gui.key_esc_name,
    group: "function",
    send: (t, m) => kittyNamed(t, "Escape", false, m.alt) ?? (m.alt ? "\x1b\x1b" : "\x1b"),
  },
  {
    name: "tab",
    cap: gui.key_tab,
    label: gui.key_tab_name,
    group: "function",
    repeats: true,
    send: (t, m) => kittyNamed(t, "Tab", false, m.alt) ?? (m.alt ? "\x1b\t" : "\t"),
  },
  { name: "backtab", cap: gui.key_backtab, label: gui.key_backtab_name, group: "function", repeats: true, send: (t) => kittyNamed(t, "Tab", true, false) ?? "\x1b[Z" },
  // Shift+Enter (item 40): the soft keyboard has none, and it is Claude
  // Code's newline. Off on the row by default; in the sheet.
  { name: "newline", cap: gui.key_newline, label: gui.key_newline_name, group: "function", repeats: true, send: (t) => shiftEnter(t) },
  ...F_KEYS.map(([cap, send]): KeyDef => ({ name: cap.toLowerCase(), cap, group: "function", send: (_, m) => send(m) })),
  { name: "paste", cap: gui.key_paste, group: null },
  // ⚠️ 墓碑 (the owner 2026-10-07 12:30): 📎 as a key of the row, scrolled
  // with the rest and listed in the editor. It is the row's own seat now,
  // pinned at its left end (`views/TermKeys` `AttachKey`); a stored row
  // naming it drops the name as any unknown one.
];

export const KEY_BY_NAME = new Map(KEYS.map((k) => [k.name, k]));

export const GROUPS: { group: KeyGroup; label: string }[] = [
  { group: "move", label: gui.keys_group_move },
  { group: "mod", label: gui.keys_group_mod },
  { group: "symbol", label: gui.keys_group_symbol },
  { group: "function", label: gui.keys_group_function },
];

const DEFAULT_ON = new Set(["esc", "ctrl", "tab", "backtab", "paste", "/", "-", "|", ":"]);
const STORE = "khor.term-key-row";
const CHANGED = "khor:term-key-row";
const names = KEYS.map(key => key.name);

export function defaultRow(): KeyRowEntry[] {
  const first = [...DEFAULT_ON].filter(name => KEY_BY_NAME.has(name));
  const rest = names.filter(name => !DEFAULT_ON.has(name));
  return [...first, ...rest].map(name => ({ id: name, kind: "builtin", name, on: DEFAULT_ON.has(name) }));
}
export function keyDefinition(entry: KeyRowEntry): KeyDef | undefined {
  if (entry.kind === "builtin") return KEY_BY_NAME.get(entry.name);
  return { name: entry.id, cap: entry.label || actionCaption(entry), group: null, action: entry };
}

export type KeyRowState = { entries: KeyRowEntry[]; problem: KeyRowProblem | null; temporary: boolean };
let current: KeyRowState | null = null;
function rowState(): KeyRowState {
  if (current) return current;
  let raw: string | null;
  try { raw = window.localStorage.getItem(STORE); }
  catch { return current = { entries: defaultRow(), problem: null, temporary: true }; }
  if (raw === null) return current = { entries: defaultRow(), problem: null, temporary: false };
  const decoded = decodeKeyRow(raw, names);
  return current = decoded.ok
    ? { entries: decoded.entries, problem: null, temporary: false }
    : { entries: defaultRow(), problem: decoded.problem, temporary: false };
}
export function keyRow(): KeyRowEntry[] { return rowState().entries; }
export function setKeyRow(entries: KeyRowEntry[]): void {
  const raw = serializeKeyRow(entries, names);
  const normalized = decodeKeyRow(raw, names);
  if (!normalized.ok) throw new Error("invalid key row");
  let temporary = false;
  try { window.localStorage.setItem(STORE, raw); }
  catch { temporary = true; }
  current = { entries: normalized.entries, problem: null, temporary };
  window.dispatchEvent(new CustomEvent(CHANGED));
}
heardFrom(STORE, () => {
  current = null;
  window.dispatchEvent(new CustomEvent(CHANGED));
});
export function useKeyRowState(): KeyRowState {
  const [state, setState] = useState(rowState);
  useEffect(() => {
    const changed = () => setState(rowState());
    window.addEventListener(CHANGED, changed);
    return () => window.removeEventListener(CHANGED, changed);
  }, []);
  return state;
}
export function useKeyRow(): KeyRowEntry[] { return useKeyRowState().entries; }
