// **The kitty keyboard protocol** (roadmap 5.1; sw.kovidgoyal.net/kitty/
// keyboard-protocol). A program that asks for it — Claude Code, neovim,
// helix, fish 4 — gets keys it can tell apart: Shift+Enter is not Enter,
// Ctrl+I is not Tab, Esc is not the start of a sequence.
//
// xterm.js 6.0 does not speak it; xterm 6.1 does (xtermjs/xterm.js #5600)
// and is still a beta, so this pane carries its own, **in this one module
// so that it is dropped whole the day 6.1 ships**: the pane's key handler
// asks `kittyKey` first, the key row asks `kittyFlags`, and nothing else
// knows the protocol exists.
//
// What is kept, per the spec: a stack of enhancement flags for each screen
// (main and alternate), pushed with `CSI > flags u`, popped with
// `CSI < n u`, changed with `CSI = flags ; mode u`, asked with `CSI ? u`
// (answered `CSI ? flags u`). The alternate screen's stack goes when the
// program leaves that screen, both go on a full reset (RIS) and when the
// pane joins its session again. The host keeps the same stacks for a
// replay whose head was trimmed (`khor_node::modes`).
//
// How keys are encoded, flag by flag:
//   1 disambiguate — Esc, and a text key with Ctrl or Alt (alone or with
//     Shift), as `CSI code ; mods u`; Enter, Tab and Backspace with any
//     modifier likewise, and plain as they always were (so a person can
//     still type `reset` after a program that crashed with the flags on);
//     the arrows, Home/End, PgUp/PgDn, Ins/Del and F-keys keep xterm's own
//     forms, which the spec lists as these keys' encodings anyway;
//   2 event types — `:2` for a repeat, and a release `:3` for every key
//     that was reported as an escape code;
//   4 alternate keys — `code:shifted:base` (the shifted character, and the
//     US-layout key when the layout differs);
//   8 all keys as escape codes — text keys, Enter/Tab/Backspace, the
//     modifier keys themselves, and the functional keys in the table's
//     `CSI 1 ; mods X` / `CSI n ; mods ~` forms;
//  16 associated text — with 8, the text a key types as a third field.
//
// **And xterm's modifyOtherKeys** (`CSI > 4 ; Pv m`, xterm's ctlseqs),
// which tmux asks of its outer terminal when the person's tmux.conf turns
// `extended-keys` on (the tmux walk, 2026-10-04: tmux 3.7 sent
// `CSI > 4 ; 2 m`): with it, Shift+Enter crosses tmux as `CSI 27 ; 2 ; 13 ~`
// instead of a bare CR, and tmux hands it on to Claude Code. Level 2 sends
// every key with Ctrl or Alt that way; level 1 only those that have no
// control character of their own; both send Enter, Tab and Backspace with
// any modifier. The kitty flags, when a program set any, come first.
import { ctrlOf } from "@/lib/term-keys";
import type { Terminal } from "@xterm/xterm";

const DISAMBIGUATE = 1;
const EVENTS = 2;
const ALTERNATES = 4;
const ALL_KEYS = 8;
const TEXT = 16;

/** The spec leaves the size to the terminal; kitty keeps this many. */
const STACK_MAX = 16;

/** Functional keys: the spec's number and final. */
const FUNCTIONAL: Record<string, [number, string]> = {
  Escape: [27, "u"],
  Enter: [13, "u"],
  Tab: [9, "u"],
  Backspace: [127, "u"],
  Insert: [2, "~"],
  Delete: [3, "~"],
  ArrowLeft: [1, "D"],
  ArrowRight: [1, "C"],
  ArrowUp: [1, "A"],
  ArrowDown: [1, "B"],
  PageUp: [5, "~"],
  PageDown: [6, "~"],
  Home: [1, "H"],
  End: [1, "F"],
  F1: [1, "P"],
  F2: [1, "Q"],
  F3: [13, "~"],
  F4: [1, "S"],
  F5: [15, "~"],
  F6: [17, "~"],
  F7: [18, "~"],
  F8: [19, "~"],
  F9: [20, "~"],
  F10: [21, "~"],
  F11: [23, "~"],
  F12: [24, "~"],
};

/** The modifier keys themselves, by `KeyboardEvent.code` (reported only
    with flag 8). */
const MODIFIER_KEYS: Record<string, number> = {
  ShiftLeft: 57441,
  ControlLeft: 57442,
  AltLeft: 57443,
  MetaLeft: 57444,
  ShiftRight: 57447,
  ControlRight: 57448,
  AltRight: 57449,
  MetaRight: 57450,
  CapsLock: 57358,
};

/** The un-shifted character of a US-layout key, by `code`: the key code a
    text key is reported with is always the un-shifted one. */
const US: Record<string, string> = {
  Backquote: "`",
  Minus: "-",
  Equal: "=",
  BracketLeft: "[",
  BracketRight: "]",
  Backslash: "\\",
  Semicolon: ";",
  Quote: "'",
  Comma: ",",
  Period: ".",
  Slash: "/",
  Space: " ",
};
function usKey(code: string): string | null {
  if (/^Key[A-Z]$/.test(code)) return code.slice(3).toLowerCase();
  if (/^Digit[0-9]$/.test(code)) return code.slice(5);
  return US[code] ?? null;
}

type State = { normal: number[]; alternate: number[]; otherKeys: number };
const states = new WeakMap<Terminal, State>();

/** The flags in force on the screen the program is on (0: none). */
export function kittyFlags(term: Terminal): number {
  const s = states.get(term);
  if (!s) return 0;
  const stack = s[term.buffer.active.type];
  return stack[stack.length - 1] ?? 0;
}

/** Starts keeping the stacks for `term`; `answer` sends a reply to the
    program. Returns what stops it, and what forgets the stacks. */
export function kittyProtocol(term: Terminal, answer: (bytes: string) => void): { dispose: () => void; reset: () => void } {
  const state: State = { normal: [], alternate: [], otherKeys: 0 };
  states.set(term, state);
  const stack = () => state[term.buffer.active.type];
  const num = (p: (number | number[])[], i: number, d: number) => {
    const v = p[i];
    const n = Array.isArray(v) ? v[0] : v;
    return n === undefined || n === 0 ? d : n;
  };
  const handlers = [
    term.parser.registerCsiHandler({ prefix: ">", final: "u" }, (p) => {
      const s = stack();
      if (s.length >= STACK_MAX) s.shift();
      s.push(num(p, 0, 0));
      return true;
    }),
    term.parser.registerCsiHandler({ prefix: "<", final: "u" }, (p) => {
      const s = stack();
      s.splice(Math.max(0, s.length - num(p, 0, 1)));
      return true;
    }),
    term.parser.registerCsiHandler({ prefix: "=", final: "u" }, (p) => {
      const s = stack();
      const f = num(p, 0, 0);
      const mode = num(p, 1, 1);
      const was = s.pop() ?? 0;
      s.push(mode === 2 ? was | f : mode === 3 ? was & ~f : f);
      return true;
    }),
    term.parser.registerCsiHandler({ prefix: "?", final: "u" }, () => {
      answer(`\x1b[?${kittyFlags(term)}u`);
      return true;
    }),
    // modifyOtherKeys: `CSI > 4 ; Pv m` sets it, `CSI > 4 m` resets it.
    // Other `CSI > … m` (xterm's modifyCursorKeys and kin) are xterm's.
    term.parser.registerCsiHandler({ prefix: ">", final: "m" }, (p) => {
      const which = Array.isArray(p[0]) ? p[0][0] : p[0];
      if (which !== 4) return false;
      const v = p.length > 1 ? (Array.isArray(p[1]) ? p[1][0] : p[1]) : 0;
      state.otherKeys = v === 1 || v === 2 ? v : 0;
      return true;
    }),
    // RIS forgets both; xterm goes on to reset everything else.
    term.parser.registerEscHandler({ final: "c" }, () => {
      state.normal = [];
      state.alternate = [];
      state.otherKeys = 0;
      return false;
    }),
    // Leaving the alternate screen leaves its flags behind.
    term.buffer.onBufferChange((b) => {
      if (b.type === "normal") state.alternate = [];
    }),
  ];
  return {
    dispose: () => {
      for (const h of handlers) h.dispose();
      states.delete(term);
    },
    reset: () => {
      state.normal = [];
      state.alternate = [];
      state.otherKeys = 0;
    },
  };
}

/** The modifier field: 1 + Shift 1 + Alt 2 + Ctrl 4 + Super 8. */
function modsOf(ev: KeyboardEvent, altIsMod: boolean): number {
  return 1 + (ev.shiftKey ? 1 : 0) + (ev.altKey && altIsMod ? 2 : 0) + (ev.ctrlKey ? 4 : 0) + (ev.metaKey ? 8 : 0);
}

/** `CSI keys ; mods:event ; text final`, with every field the spec lets
    go left out. */
function csi(keys: string, mods: number, event: number, text: string, final: string): string {
  let out = keys;
  if (mods > 1 || event > 1 || text) out += `;${mods}${event > 1 ? `:${event}` : ""}`;
  if (text) out += `;${text}`;
  if (final !== "u" && out === "1") out = "";
  return `\x1b[${out}${final}`;
}

/**
 * **What a key sends under the program's flags**, or `null` when the flags
 * leave it to xterm (no flags at all; a plain text key; a key xterm already
 * spells unambiguously). `release` is a keyup.
 */
export function kittyKey(term: Terminal, ev: KeyboardEvent, release: boolean): string | null {
  const flags = kittyFlags(term);
  if (!(flags & (DISAMBIGUATE | ALL_KEYS))) return otherKey(term, ev, release);
  // An IME's keys are the composition's, and what it commits is text.
  if (ev.isComposing || ev.keyCode === 229) return null;
  const mac = /Mac|iPhone|iPad/.test(navigator.platform);
  // ⌥ on a Mac types a character unless the pane makes it Meta.
  const altIsMod = !mac || !!term.options.macOptionIsMeta;
  const mods = modsOf(ev, altIsMod);
  const all = (flags & ALL_KEYS) !== 0;
  const event = flags & EVENTS ? (release ? 3 : ev.repeat ? 2 : 1) : 1;
  if (release && !(flags & EVENTS)) return null;

  // The modifier keys themselves.
  const modifier = MODIFIER_KEYS[ev.code];
  if (modifier !== undefined) return all ? csi(String(modifier), mods, event, "", "u") : null;

  const fn = FUNCTIONAL[ev.key];
  if (fn) {
    const [n, final] = fn;
    const legacyTriple = ev.key === "Enter" || ev.key === "Tab" || ev.key === "Backspace";
    if (!all) {
      // Enter, Tab and Backspace stay as they were when plain, and send no
      // release; the rest of the table keeps xterm's own spelling.
      if (legacyTriple && (mods === 1 || release)) return null;
      if (ev.key !== "Escape" && !legacyTriple) return release ? csi(String(n), mods, event, "", final) : null;
    }
    return csi(String(n), mods, event, "", final);
  }

  // A text key: one character on its cap.
  if ([...ev.key].length !== 1) return null;
  const base = usKey(ev.code);
  const key = ev.shiftKey ? (base ?? ev.key.toLowerCase()) : ev.key.toLowerCase();
  const others = mods - 1 - (ev.shiftKey ? 1 : 0);
  // Plain or shifted text stays text unless every key is asked for.
  if (!all && others === 0) return null;
  // ⌘ is the app's, never the program's, short of flag 8.
  if (ev.metaKey && !all) return null;
  const cp = key.codePointAt(0)!;
  let keys = String(cp);
  if (flags & ALTERNATES) {
    const shifted = ev.shiftKey && ev.key !== key ? String(ev.key.codePointAt(0)) : "";
    const layout = base && base !== key ? String(base.codePointAt(0)) : "";
    if (shifted || layout) keys += `:${shifted}${layout ? `:${layout}` : ""}`;
  }
  const text = all && flags & TEXT && !release && !ev.ctrlKey && others === 0 ? String(ev.key.codePointAt(0)) : "";
  return csi(keys, mods, event, text, "u");
}

/** The modifyOtherKeys level in force (0: off). */
export function otherKeysLevel(term: Terminal): number {
  return states.get(term)?.otherKeys ?? 0;
}

/** A key under xterm's modifyOtherKeys: `CSI 27 ; mods ; code ~`, or
    `null` for xterm's own spelling. */
function otherKey(term: Terminal, ev: KeyboardEvent, release: boolean): string | null {
  const level = otherKeysLevel(term);
  if (!level || release || ev.isComposing || ev.keyCode === 229 || ev.metaKey) return null;
  const mac = /Mac|iPhone|iPad/.test(navigator.platform);
  const altIsMod = !mac || !!term.options.macOptionIsMeta;
  const mods = modsOf(ev, altIsMod);
  if (mods === 1) return null;
  const triple: Record<string, number> = { Enter: 13, Tab: 9, Backspace: 127 };
  if (ev.key in triple) return `\x1b[27;${mods};${triple[ev.key]}~`;
  if ([...ev.key].length !== 1) return null;
  const ctrl = ev.ctrlKey;
  const alt = ev.altKey && altIsMod;
  if (!ctrl && !alt) return null;
  const base = usKey(ev.code) ?? ev.key.toLowerCase();
  // Level 1 leaves the combinations every terminal agrees on (⌃A, ⌥x
  // without Shift) as they were.
  if (level === 1 && !ev.shiftKey && (!ctrl || ctrlOf(base) !== null)) return null;
  return `\x1b[27;${mods};${(ev.shiftKey ? ev.key : base).codePointAt(0)}~`;
}

/** A character the key row types under its latches (`lib/term-keys`), as
    the protocol spells it, or `null` to keep the legacy form. */
export function kittyTyped(term: Terminal, ch: string, ctrl: boolean, alt: boolean): string | null {
  const flags = kittyFlags(term);
  if (!(flags & (DISAMBIGUATE | ALL_KEYS)) || (!ctrl && !alt && !(flags & ALL_KEYS))) return null;
  const mods = 1 + (alt ? 2 : 0) + (ctrl ? 4 : 0);
  return csi(String(ch.toLowerCase().codePointAt(0)), mods, 1, "", "u");
}

/** Esc, Tab, Shift+Tab and Backspace from the key row, as the protocol
    spells them, or `null` to keep the legacy form. */
export function kittyNamed(term: Terminal, key: "Escape" | "Tab" | "Backspace" | "Enter", shift: boolean, alt: boolean, ctrl = false): string | null {
  const flags = kittyFlags(term);
  if (!(flags & (DISAMBIGUATE | ALL_KEYS))) return null;
  const mods = 1 + (shift ? 1 : 0) + (alt ? 2 : 0) + (ctrl ? 4 : 0);
  // Tab and Backspace without a modifier keep their legacy bytes unless
  // every key is to be reported (the spec's "disambiguate" leaves them).
  if (key !== "Escape" && mods === 1 && !(flags & ALL_KEYS)) return null;
  return csi(key === "Escape" ? "27" : key === "Tab" ? "9" : key === "Enter" ? "13" : "127", mods, 1, "", "u");
}

/** Shift+Enter, for the key row's 换行 cap (item 40): what a keyboard's
    Shift+Enter sends in the current mode — the kitty form when the
    program set flags, xterm's modifyOtherKeys form when it (or tmux)
    asked for that, else the CR it has always been. */
export function shiftEnter(term: Terminal): string {
  return kittyNamed(term, "Enter", true, false) ?? (otherKeysLevel(term) ? "\x1b[27;2;13~" : "\r");
}
