export const KEY_ROW_VERSION = 1;
export const KEY_ROW_LIMITS = {
  entries: 128,
  steps: 16,
  labelCharacters: 32,
  labelBytes: 128,
  configurationBytes: 64 * 1024,
  encodedBytes: 16 * 1024,
} as const;

const NAMED_KEYS = [
  'Escape', 'Tab', 'Enter', 'Backspace', 'Delete', 'Insert',
  'ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown',
  'Home', 'End', 'PageUp', 'PageDown',
  'F1', 'F2', 'F3', 'F4', 'F5', 'F6', 'F7', 'F8', 'F9', 'F10', 'F11', 'F12',
  'Space', 'Backquote', 'Minus', 'Equal', 'BracketLeft', 'BracketRight',
  'Backslash', 'Semicolon', 'Quote', 'Comma', 'Period', 'Slash',
] as const;
type Letter = 'A'|'B'|'C'|'D'|'E'|'F'|'G'|'H'|'I'|'J'|'K'|'L'|'M'|'N'|'O'|'P'|'Q'|'R'|'S'|'T'|'U'|'V'|'W'|'X'|'Y'|'Z';
type Digit = '0'|'1'|'2'|'3'|'4'|'5'|'6'|'7'|'8'|'9';
export type BaseKey = typeof NAMED_KEYS[number] | `Key${Letter}` | `Digit${Digit}`;
export const BASE_KEYS: readonly BaseKey[] = [
  ...NAMED_KEYS,
  ...Array.from('ABCDEFGHIJKLMNOPQRSTUVWXYZ', letter => `Key${letter}` as BaseKey),
  ...Array.from('0123456789', digit => `Digit${digit}` as BaseKey),
];
const allowedKeys = new Set<string>(BASE_KEYS);

export type KeyStroke = { key: BaseKey; ctrl: boolean; alt: boolean; shift: boolean };
type Seat = { id: string; on: boolean };
export type BuiltinKeyEntry = Seat & { kind: 'builtin'; name: string };
export type CustomKeyAction = { kind: 'chord'; stroke: KeyStroke } | { kind: 'sequence'; steps: KeyStroke[] };
export type CustomKeyEntry = Seat & { label: string } & CustomKeyAction;
export type KeyRowEntry = BuiltinKeyEntry | CustomKeyEntry;
export type KeyRowConfiguration = { version: typeof KEY_ROW_VERSION; entries: KeyRowEntry[] };
export type KeyRowProblem = 'shape' | 'version' | 'budget' | 'identity' | 'key';
export type KeyRowDecode =
  | { ok: true; entries: KeyRowEntry[]; migrated: boolean }
  | { ok: false; problem: KeyRowProblem };

const bytes = (value: string) => new TextEncoder().encode(value).length;
const record = (value: unknown): value is Record<string, unknown> =>
  value !== null && typeof value === 'object' && !Array.isArray(value);
function fields(value: Record<string, unknown>, names: readonly string[]) {
  return Object.keys(value).length === names.length && names.every(name => Object.hasOwn(value, name));
}
export function validStroke(value: unknown): value is KeyStroke {
  return record(value) && fields(value, ['key', 'ctrl', 'alt', 'shift']) &&
    typeof value.key === 'string' && allowedKeys.has(value.key) &&
    typeof value.ctrl === 'boolean' && typeof value.alt === 'boolean' && typeof value.shift === 'boolean';
}
function readEntry(value: unknown, builtins: ReadonlySet<string>): KeyRowEntry | KeyRowProblem {
  if (!record(value) || typeof value.on !== 'boolean' || typeof value.id !== 'string') return 'shape';
  if (!value.id || value.id.length > 80) return 'identity';
  if (value.kind === 'builtin') {
    if (!fields(value, ['id', 'on', 'kind', 'name'])) return 'shape';
    if (typeof value.name !== 'string' || !builtins.has(value.name) || value.id !== value.name) return 'identity';
    return { id: value.id, on: value.on, kind: 'builtin', name: value.name };
  }
  if (!/^custom:[a-zA-Z0-9_-]{1,64}$/.test(value.id) || typeof value.label !== 'string') return 'identity';
  if (value.label.length > KEY_ROW_LIMITS.labelCharacters * 2 || [...value.label].length > KEY_ROW_LIMITS.labelCharacters || bytes(value.label) > KEY_ROW_LIMITS.labelBytes) return 'budget';
  if (value.kind === 'chord') {
    if (!fields(value, ['id', 'on', 'kind', 'label', 'stroke'])) return 'shape';
    if (!validStroke(value.stroke)) return 'key';
    return { id: value.id, on: value.on, kind: 'chord', label: value.label, stroke: { ...value.stroke } };
  }
  if (value.kind === 'sequence') {
    if (!fields(value, ['id', 'on', 'kind', 'label', 'steps']) || !Array.isArray(value.steps)) return 'shape';
    if (!value.steps.length || value.steps.length > KEY_ROW_LIMITS.steps) return 'budget';
    if (!value.steps.every(validStroke)) return 'key';
    return { id: value.id, on: value.on, kind: 'sequence', label: value.label, steps: value.steps.map(step => ({ ...step })) };
  }
  return 'shape';
}

export function decodeKeyRow(raw: string, names: readonly string[]): KeyRowDecode {
  if (raw.length > KEY_ROW_LIMITS.configurationBytes || bytes(raw) > KEY_ROW_LIMITS.configurationBytes) return { ok: false, problem: 'budget' };
  let value: unknown;
  try { value = JSON.parse(raw); } catch { return { ok: false, problem: 'shape' }; }
  const builtins = new Set(names);
  const migrated = Array.isArray(value);
  let values: unknown[];
  if (migrated) {
    // Existing preferences are data to migrate, never a second execution format.
    if (!Array.isArray(value) || value.length > KEY_ROW_LIMITS.entries) return { ok: false, problem: 'budget' };
    values = [];
    for (const seat of value) {
      if (!record(seat) || typeof seat.name !== 'string' || typeof seat.on !== 'boolean') return { ok: false, problem: 'shape' };
      if (builtins.has(seat.name)) values.push({ id: seat.name, kind: 'builtin', name: seat.name, on: seat.on });
    }
  } else {
    if (!record(value) || !fields(value, ['version', 'entries'])) return { ok: false, problem: 'shape' };
    if (value.version !== KEY_ROW_VERSION) return { ok: false, problem: 'version' };
    if (!Array.isArray(value.entries)) return { ok: false, problem: 'shape' };
    values = value.entries;
  }
  if (values.length > KEY_ROW_LIMITS.entries) return { ok: false, problem: 'budget' };
  const entries: KeyRowEntry[] = [], ids = new Set<string>();
  for (const value of values) {
    const entry = readEntry(value, builtins);
    if (typeof entry === 'string') return { ok: false, problem: entry };
    if (ids.has(entry.id)) return { ok: false, problem: 'identity' };
    ids.add(entry.id); entries.push(entry);
  }
  for (const name of names) {
    if (!ids.has(name)) entries.push({ id: name, kind: 'builtin', name, on: false });
  }
  if (entries.length > KEY_ROW_LIMITS.entries) return { ok: false, problem: 'budget' };
  const configuration: KeyRowConfiguration = { version: KEY_ROW_VERSION, entries };
  if (bytes(JSON.stringify(configuration)) > KEY_ROW_LIMITS.configurationBytes) return { ok: false, problem: 'budget' };
  return { ok: true, entries, migrated };
}

export function serializeKeyRow(entries: readonly KeyRowEntry[], names: readonly string[]): string {
  if (!Array.isArray(entries) || entries.length > KEY_ROW_LIMITS.entries) throw new Error('invalid key row: budget');
  const builtins = new Set(names);
  const bounded = entries.map(entry => {
    const checked = readEntry(entry, builtins);
    if (typeof checked === 'string') throw new Error(`invalid key row: ${checked}`);
    return checked;
  });
  const raw = JSON.stringify({ version: KEY_ROW_VERSION, entries: bounded });
  const decoded = decodeKeyRow(raw, names);
  if (!decoded.ok) throw new Error(`invalid key row: ${decoded.problem}`);
  return JSON.stringify({ version: KEY_ROW_VERSION, entries: decoded.entries });
}
