/** Terminal palettes belong to the viewer, like its font and cursor.
 * A selection changes paint, never the PTY, agent argv or remote config.
 * Presets and user themes go through the same xterm ITheme resolver. */
import { useEffect, useState } from "react";
import type { ITheme } from "@xterm/xterm";
import { heardFrom } from "@/lib/other-windows";
import presets from "@/lib/term-palettes.json";

export const TERM_THEME_KEY = "khor.term-theme";
const CHANGED = "khor:term-theme";
export const THEME_FILE_MAX = 100_000;
export const ANSI_KEYS = ["black", "red", "green", "yellow", "blue", "magenta", "cyan", "white", "brightBlack", "brightRed", "brightGreen", "brightYellow", "brightBlue", "brightMagenta", "brightCyan", "brightWhite"] as const;
export const BASE_KEYS = ["background", "foreground", "cursor", "cursorAccent", "selectionBackground", "selectionForeground"] as const;
export type ColorKey = typeof ANSI_KEYS[number] | typeof BASE_KEYS[number];
export type Palette = Record<ColorKey, string> & { extendedAnsi?: string[] };
export type UserPalette = { id: string; name: string; theme: Palette };
export type ThemeChoice = { selected: string; customs: UserPalette[] };
export type ThemeError = "json" | "colors" | "extended" | "large" | "full";
export const DEFAULT_THEME: ThemeChoice = { selected: "follow", customs: [] };

/** Existing khor ANSI16, preserved for Follow. This is a terminal color
 * table, not app chrome. 16–255 retain xterm's standard cube and ramp. */
export const KHOR_ANSI = ["#000000", "#cc0000", "#4e9a06", "#c4a000", "#3465a4", "#75507b", "#06989a", "#d3d7cf", "#555753", "#ef2929", "#8ae234", "#fce94f", "#729fcf", "#ad7fa8", "#34e2e2", "#eeeeec"];

export const PRESETS = Object.entries(presets).map(([id, p]) => ({ id, ...p, theme: p.theme as Palette }));
const validId = (id: string, customs: UserPalette[]) => id === "follow" || PRESETS.some(p => p.id === id) || customs.some(p => p.id === id);
/** HEX files are portable and need no CSS evaluator. Short hex expands;
 * alpha is allowed on the selection, as in xterm's theme API. */
export function colorHex(v: unknown, alpha = false): string | null {
  if (typeof v !== "string") return null;
  const s = v.trim().toLowerCase();
  if (/^#[\da-f]{3}$/.test(s)) return "#" + [...s.slice(1)].map(c => c + c).join("");
  if (/^#[\da-f]{6}$/.test(s) || alpha && /^#[\da-f]{8}$/.test(s)) return s;
  if (alpha) {
    const m = s.match(/^rgba?\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)(?:\s*,\s*(0?(?:\.\d+)|1(?:\.0+)?|0))?\s*\)$/);
    if (m && m.slice(1, 4).every(n => Number(n) <= 255)) return "#" + m.slice(1, 4).map(n => Number(n).toString(16).padStart(2, "0")).join("") + (m[4] === undefined ? "" : Math.round(Number(m[4]) * 255).toString(16).padStart(2, "0"));
  }
  return null;
}
export function readPalette(raw: unknown, base: Palette): Palette {
  if (!raw || typeof raw !== "object" || Array.isArray(raw)) throw new Error("colors" satisfies ThemeError);
  const source = raw as Record<string, unknown>;
  const theme = { ...base };
  let count = 0;
  for (const key of [...BASE_KEYS, ...ANSI_KEYS]) {
    if (!Object.hasOwn(source, key)) continue;
    const value = colorHex(source[key], key === "selectionBackground" || key === "selectionForeground");
    if (!value) throw new Error("colors" satisfies ThemeError);
    theme[key] = value;
    count++;
  }
  if (Object.hasOwn(source, "extendedAnsi")) {
    const extra = source.extendedAnsi;
    if (!Array.isArray(extra) || extra.length > 240 || extra.some(c => !colorHex(c))) throw new Error("extended" satisfies ThemeError);
    theme.extendedAnsi = extra.map(c => colorHex(c)!);
    count++;
  }
  if (!count) throw new Error("colors" satisfies ThemeError);
  return theme;
}
function sanitize(raw: unknown): ThemeChoice {
  if (!raw || typeof raw !== "object") return DEFAULT_THEME;
  const v = raw as Partial<ThemeChoice>;
  const customs: UserPalette[] = [];
  if (Array.isArray(v.customs)) for (const p of v.customs.slice(0, 50)) {
    try {
      if (p && typeof p.id === "string" && p.id.startsWith("custom-") && p.id.length < 100 && typeof p.name === "string" && p.name.trim() && !customs.some(o => o.id === p.id)) customs.push({ id: p.id, name: p.name.trim().slice(0, 48), theme: readPalette(p.theme, PRESETS[0].theme) });
    } catch { /* One broken palette does not hide the others. */ }
  }
  return { selected: typeof v.selected === "string" && validId(v.selected, customs) ? v.selected : "follow", customs };
}
let cached: ThemeChoice | null = null;
export function termTheme(): ThemeChoice {
  if (cached) return cached;
  try { return sanitize(JSON.parse(window.localStorage.getItem(TERM_THEME_KEY) ?? "null")); } catch { return DEFAULT_THEME; }
}
export function setTermTheme(next: ThemeChoice): boolean {
  cached = sanitize(next);
  let persisted = true;
  try { window.localStorage.setItem(TERM_THEME_KEY, JSON.stringify(cached)); } catch { persisted = false; }
  wearTermPalette();
  window.dispatchEvent(new Event(CHANGED));
  return persisted;
}
export function onTermTheme(run: () => void): () => void {
  window.addEventListener(CHANGED, run);
  return () => window.removeEventListener(CHANGED, run);
}
heardFrom(TERM_THEME_KEY, () => { cached = null; wearTermPalette(); window.dispatchEvent(new Event(CHANGED)); });
export function useTermTheme(): ThemeChoice {
  const [value, setValue] = useState(termTheme);
  useEffect(() => onTermTheme(() => setValue(termTheme())), []);
  return value;
}
export function chosenPalette(choice = termTheme()): Palette | null {
  return PRESETS.find(p => p.id === choice.selected)?.theme ?? choice.customs.find(p => p.id === choice.selected)?.theme ?? null;
}
export function wearTermPalette(): void {
  const p = chosenPalette();
  const root = document.documentElement;
  for (const [key, value] of [["--term-palette-bg", p?.background], ["--term-palette-fg", p?.foreground]] as const) {
    if (value) root.style.setProperty(key, value); else root.style.removeProperty(key);
  }
}
export function terminalTheme(el: HTMLElement, choice = termTheme()): ITheme {
  const p = chosenPalette(choice);
  if (p) return { ...p, extendedAnsi: p.extendedAnsi?.slice() };
  const css = getComputedStyle(el);
  const background = getComputedStyle(document.documentElement).getPropertyValue("--canvas-solid").trim();
  const foreground = css.color;
  return { background, foreground, cursor: foreground, cursorAccent: background, ...Object.fromEntries(ANSI_KEYS.map((k, i) => [k, KHOR_ANSI[i]])) };
}
/** Resolve Follow to an opaque, portable HEX palette for cloning/export. */
export function clonePalette(el: HTMLElement, choice = termTheme()): Palette {
  const chosen = chosenPalette(choice);
  if (chosen) return structuredClone(chosen);
  const t = terminalTheme(el, choice);
  const hex = (s: string) => {
    const direct = colorHex(s); if (direct) return direct;
    const rgb = s.match(/[\d.]+/g)?.slice(0, 3).map(Number);
    if (!rgb || rgb.length !== 3) return PRESETS[0].theme.foreground;
    return "#" + rgb.map(n => Math.round(n).toString(16).padStart(2, "0")).join("");
  };
  const p = Object.fromEntries([...ANSI_KEYS, ...BASE_KEYS].map(k => [k, hex(t[k] ?? (k === "selectionBackground" ? KHOR_ANSI[4] : k === "selectionForeground" ? t.background! : t.foreground!))])) as unknown as Palette;
  if (t.extendedAnsi) p.extendedAnsi = t.extendedAnsi.slice();
  return p;
}
export function importPalette(text: string, base: Palette): { name: string; theme: Palette } {
  if (new TextEncoder().encode(text).length > THEME_FILE_MAX) throw new Error("large" satisfies ThemeError);
  let raw: unknown;
  try { raw = JSON.parse(text); } catch { throw new Error("json" satisfies ThemeError); }
  const obj = raw as { name?: unknown; theme?: unknown } | null;
  return { name: typeof obj?.name === "string" ? obj.name.trim().slice(0, 48) : "", theme: readPalette(obj?.theme ?? raw, base) };
}
export function savePalette(name: string, theme: Palette, id?: string): string {
  const choice = termTheme();
  if (!choice.customs.some(p => p.id === id) && choice.customs.length >= 50) throw new Error("full" satisfies ThemeError);
  const picked = id && choice.customs.some(p => p.id === id) ? id : `custom-${crypto.randomUUID()}`;
  const next = { id: picked, name: name.trim().slice(0, 48), theme: readPalette(theme, PRESETS[0].theme) };
  if (!setTermTheme({ selected: picked, customs: [...choice.customs.filter(p => p.id !== picked), next] })) throw new Error("storage");
  return picked;
}
export function removePalette(id: string): boolean {
  const c = termTheme();
  return setTermTheme({ selected: c.selected === id ? "follow" : c.selected, customs: c.customs.filter(p => p.id !== id) });
}
