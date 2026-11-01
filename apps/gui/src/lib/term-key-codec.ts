import type { Terminal } from '@xterm/xterm';
import { ctrlOf, kittyRowKey } from '@/lib/term-kitty';
import { KEY_ROW_LIMITS, validStroke, type BaseKey, type CustomKeyAction, type KeyStroke } from '@/lib/term-key-model';

const ESC = '\x1b';
type KeyModifiers = { ctrl: boolean; alt: boolean; shift?: boolean };
const modifierParameter = (mods: KeyModifiers) => 1 + Number(!!mods.shift) + 2 * Number(mods.alt) + 4 * Number(mods.ctrl);

export function typed(ch: string, mods: KeyModifiers): string {
  const value = mods.ctrl ? (ctrlOf(ch) ?? ch) : ch;
  return mods.alt ? `${ESC}${value}` : value;
}
export function cursorKey(term: Terminal, final: string, mods: KeyModifiers): string {
  const modifier = modifierParameter(mods);
  return modifier > 1 ? `${ESC}[1;${modifier}${final}` : `${ESC}${term.modes.applicationCursorKeysMode ? 'O' : '['}${final}`;
}
export function tildeKey(number: number, mods: KeyModifiers): string {
  const modifier = modifierParameter(mods);
  return `${ESC}[${number}${modifier > 1 ? `;${modifier}` : ''}~`;
}
export function ss3Key(final: string, mods: KeyModifiers): string {
  const modifier = modifierParameter(mods);
  return modifier > 1 ? `${ESC}[1;${modifier}${final}` : `${ESC}O${final}`;
}

const PRINTABLE: Record<string, [string, string, number]> = {
  Space: [' ', ' ', 32], Backquote: ['`', '~', 192], Minus: ['-', '_', 189], Equal: ['=', '+', 187],
  BracketLeft: ['[', '{', 219], BracketRight: [']', '}', 221], Backslash: ['\\', '|', 220],
  Semicolon: [';', ':', 186], Quote: ["'", '"', 222], Comma: [',', '<', 188], Period: ['.', '>', 190], Slash: ['/', '?', 191],
};
const NUMBER_SHIFT = ')!@#$%^&*(';
const NAMED_CODES: Record<string, number> = {
  Escape: 27, Tab: 9, Enter: 13, Backspace: 8, Delete: 46, Insert: 45,
  ArrowLeft: 37, ArrowRight: 39, ArrowUp: 38, ArrowDown: 40,
  Home: 36, End: 35, PageUp: 33, PageDown: 34,
};
const CURSOR: Record<string, string> = { ArrowLeft: 'D', ArrowRight: 'C', ArrowUp: 'A', ArrowDown: 'B', Home: 'H', End: 'F' };
const TILDE: Record<string, number> = { Insert: 2, Delete: 3, PageUp: 5, PageDown: 6, F5: 15, F6: 17, F7: 18, F8: 19, F9: 20, F10: 21, F11: 23, F12: 24 };
const CAPS: Record<string, string> = {
  Escape: 'Esc', Backspace: '⌫', Delete: 'Del', Insert: 'Ins', ArrowLeft: '←', ArrowRight: '→', ArrowUp: '↑', ArrowDown: '↓',
  PageUp: 'PgUp', PageDown: 'PgDn', Space: 'Space',
};

function printable(key: BaseKey): [string, string, number] | null {
  if (/^Key[A-Z]$/.test(key)) {
    const letter = key.slice(3);
    return [letter.toLowerCase(), letter, letter.charCodeAt(0)];
  }
  if (/^Digit[0-9]$/.test(key)) {
    const digit = key.slice(5);
    return [digit, NUMBER_SHIFT[Number(digit)], digit.charCodeAt(0)];
  }
  return PRINTABLE[key] ?? null;
}
export function baseKeyCaption(key: BaseKey): string {
  if (CAPS[key]) return CAPS[key];
  const text = printable(key);
  return text ? text[0].toUpperCase() : key;
}
export function strokeCaption(stroke: KeyStroke): string {
  return `${stroke.ctrl ? '⌃' : ''}${stroke.alt ? '⌥' : ''}${stroke.shift ? '⇧' : ''}${baseKeyCaption(stroke.key)}`;
}
export function actionCaption(action: CustomKeyAction): string {
  return action.kind === 'chord' ? strokeCaption(action.stroke) : action.steps.map(strokeCaption).join(' → ');
}

function eventFor(stroke: KeyStroke): KeyboardEvent {
  const text = printable(stroke.key);
  const functionNumber = /^F([0-9]+)$/.exec(stroke.key);
  return {
    type: 'keydown', key: text ? text[stroke.shift ? 1 : 0] : stroke.key,
    code: stroke.key, keyCode: text ? text[2] : functionNumber ? 111 + Number(functionNumber[1]) : NAMED_CODES[stroke.key],
    ctrlKey: stroke.ctrl, altKey: stroke.alt, shiftKey: stroke.shift,
    metaKey: false, repeat: false, isComposing: false,
  } as KeyboardEvent;
}
function legacyStroke(term: Terminal, stroke: KeyStroke, event: KeyboardEvent): string {
  const cursor = CURSOR[stroke.key];
  if (cursor) return cursorKey(term, cursor, stroke);
  const tilde = TILDE[stroke.key];
  if (tilde) return tildeKey(tilde, stroke);
  if (/^F[1-4]$/.test(stroke.key)) {
    const final = 'PQRS'[Number(stroke.key.slice(1)) - 1];
    return ss3Key(final, stroke);
  }
  if (stroke.key === 'Escape') return stroke.alt ? ESC + ESC : ESC;
  if (stroke.key === 'Tab') return stroke.shift ? `${ESC}[Z` : `${stroke.alt ? ESC : ''}\t`;
  if (stroke.key === 'Enter') return `${stroke.alt ? ESC : ''}\r`;
  if (stroke.key === 'Backspace') return `${stroke.alt ? ESC : ''}${stroke.ctrl ? '\x08' : '\x7f'}`;
  const plain = printable(stroke.key);
  if (!plain) throw new Error('unsupported terminal key');
  const control = stroke.ctrl ? ctrlOf(event.key) : event.key;
  if (control === null) throw new Error('this terminal protocol cannot encode the combination');
  return typed(event.key, stroke);
}

export function encodeStroke(term: Terminal, stroke: KeyStroke): string {
  if (!validStroke(stroke)) throw new Error('invalid terminal key');
  const event = eventFor(stroke);
  const down = kittyRowKey(term, event, false) ?? legacyStroke(term, stroke, event);
  return down + (kittyRowKey(term, event, true) ?? '');
}
export function encodeKeyAction(term: Terminal, action: CustomKeyAction): string {
  if (!action || (action.kind !== 'chord' && action.kind !== 'sequence')) throw new Error('invalid terminal key action');
  const steps = action.kind === 'chord' ? [action.stroke] : action.steps;
  if (!Array.isArray(steps) || !steps.length || steps.length > KEY_ROW_LIMITS.steps || !steps.every(validStroke)) throw new Error('invalid terminal key sequence');
  const chunks = steps.map(stroke => encodeStroke(term, stroke));
  const encoded = chunks.join('');
  if (new TextEncoder().encode(encoded).length > KEY_ROW_LIMITS.encodedBytes) throw new Error('terminal key sequence exceeds its byte budget');
  return encoded;
}

const encodedInputs = new WeakMap<Terminal, number>();
export function isEncodedKeyInput(term: Terminal): boolean {
  return (encodedInputs.get(term) ?? 0) > 0;
}
export function pressKeyAction(term: Terminal, action: CustomKeyAction): void {
  const encoded = encodeKeyAction(term, action);
  const depth = encodedInputs.get(term) ?? 0;
  encodedInputs.set(term, depth + 1);
  try { term.input(encoded, true); }
  finally {
    if (depth) encodedInputs.set(term, depth);
    else encodedInputs.delete(term);
  }
}
