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
// order. ⚠️ In this branch only: the real store, after sign-off, is a file
// under khor's home served to the app and the web face alike. A face that
// refuses storage keeps the default row for the session.
import { useEffect, useState } from "react";
import type { Terminal } from "@xterm/xterm";

import { gui } from "@/gen/catalog";
import { kittyNamed, kittyTyped, shiftEnter } from "@/lib/term-kitty";

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

/** A typed character under the latches: its control form for Ctrl, an
    `ESC` before it for Alt. */
export function typed(ch: string, mods: Mods): string {
  const c = mods.ctrl ? (ctrlOf(ch) ?? ch) : ch;
  return mods.alt ? `\x1b${c}` : c;
}

/** xterm's modifier parameter: 1 + Shift 1 + Alt 2 + Ctrl 4. */
function modParam(mods: Mods): number {
  return 1 + (mods.alt ? 2 : 0) + (mods.ctrl ? 4 : 0);
}

/** A key whose plain form is `ESC [ <letter>` or, in application mode,
    `ESC O <letter>` — the arrows, Home and End. */
export function cursorKey(term: Terminal, letter: string, mods: Mods): string {
  const m = modParam(mods);
  if (m > 1) return `\x1b[1;${m}${letter}`;
  return term.modes.applicationCursorKeysMode ? `\x1bO${letter}` : `\x1b[${letter}`;
}

/** A key spelled `ESC [ <n> ~` — PgUp, Ins, F5 and up. */
function tilde(n: number, mods: Mods): string {
  const m = modParam(mods);
  return m > 1 ? `\x1b[${n};${m}~` : `\x1b[${n}~`;
}

/** F1–F4: `ESC O P`…`S`, and the CSI form with a modifier. */
function ss3(letter: string, mods: Mods): string {
  const m = modParam(mods);
  return m > 1 ? `\x1b[1;${m}${letter}` : `\x1bO${letter}`;
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

/** One place in the row: a key and whether it is drawn. */
export type RowSeat = { name: string; on: boolean };

/** The row as it was before it could be edited (2026-10-02): the escape
    and the modifier, what moves through a form, paste, then the
    characters a shell lives on. Every other key follows, off. */
const DEFAULT_ON = ["esc", "ctrl", "tab", "backtab", "paste", "/", "-", "|", ":"];

/** **Keys that came after a row was stored, drawn on** (UX-41): a key
    appended off is a key nobody finds. Each goes in on, before the key
    named (inside the row's first screenful), or last. Empty since 📎
    left the row for its own seat. */
const LATER_ON: Record<string, string> = {};

export function defaultRow(): RowSeat[] {
  const on = DEFAULT_ON.map((name) => ({ name, on: true }));
  const rest = KEYS.filter((k) => !DEFAULT_ON.includes(k.name)).map((k) => ({ name: k.name, on: false }));
  return [...on, ...rest];
}

const STORE = "khor.term-key-row";
const CHANGED = "khor:term-key-row";

/** This session's row once it has been set, for a face whose storage
    refuses it. */
let session: RowSeat[] | null = null;

/** The stored row, cleaned: unknown names dropped, each key once, and a
    key the store does not name yet (a later version's) appended off. */
export function keyRow(): RowSeat[] {
  if (session) return session;
  let raw: unknown = null;
  try {
    raw = JSON.parse(window.localStorage.getItem(STORE) ?? "null");
  } catch {
    raw = null;
  }
  if (!Array.isArray(raw)) return defaultRow();
  const seen = new Set<string>();
  const row: RowSeat[] = [];
  for (const s of raw) {
    const name = (s as RowSeat)?.name;
    if (typeof name !== "string" || !KEY_BY_NAME.has(name) || seen.has(name)) continue;
    seen.add(name);
    row.push({ name, on: (s as RowSeat).on === true });
  }
  for (const k of KEYS) {
    if (seen.has(k.name)) continue;
    const before = LATER_ON[k.name];
    if (before === undefined) {
      row.push({ name: k.name, on: false });
      continue;
    }
    const at = row.findIndex((s) => s.name === before);
    row.splice(at < 0 ? row.length : at, 0, { name: k.name, on: true });
  }
  return row;
}

export function setKeyRow(row: RowSeat[]) {
  session = row;
  try {
    window.localStorage.setItem(STORE, JSON.stringify(row));
  } catch {
    // Kept for this session only.
  }
  window.dispatchEvent(new CustomEvent(CHANGED, { detail: row }));
}

export function useKeyRow(): RowSeat[] {
  const [row, setRow] = useState(keyRow);
  useEffect(() => {
    const changed = (e: Event) => setRow((e as CustomEvent<RowSeat[]>).detail);
    window.addEventListener(CHANGED, changed);
    return () => window.removeEventListener(CHANGED, changed);
  }, []);
  return row;
}
