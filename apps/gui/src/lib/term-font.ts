// **The terminal's font** (the audit fix plan F7.6; N17, widened by the
// owner 2026-10-03: 「这个作为兜底，用户可以在终端设置里选择字体，如 ubuntu
// mono，Nerd font 等，并可自己上传字体」).
//
// - **A symbols face is always the last family** before the generic one
//   (`khor-symbols`, Nerd Fonts' "Symbols Only" mono face, bundled under
//   `assets/fonts` with its licence): a prompt's powerline and Nerd glyphs
//   draw in the Mac app, the web face and on a phone, whatever font is
//   chosen. It is the only font khor ships.
// - **The choice is this device's** (`localStorage`), not synced:
//   devices have different fonts. Terminals only; the code view keeps
//   its own.
// - **Fonts on this device are measured, never assumed**: a candidate is
//   present when a test string drawn in it differs in width from the same
//   string in two different generic families (`document.fonts.check`
//   alone answers yes for fonts that are not there in some engines).
// - **Added fonts** are files the person picked, registered with the
//   FontFace API under their own family name and kept on this device in
//   IndexedDB. ⚠️ In this branch only: the real store, after sign-off, is
//   a file under khor's home served to the app and the web face alike.
import { useEffect, useState } from "react";

/** Today's stack, before a choice: the system's monospace faces. */
const SYSTEM = `"SF Mono", SFMono-Regular, Menlo, Consolas`;
/** The bundled symbols face (`app.css` `@font-face`). */
export const SYMBOLS = "khor-symbols";
/** The largest font file taken, in MB. */
export const FONT_MAX_MB = 10;

/** Common terminal faces, looked for by measurement. */
const KNOWN = [
  "SF Mono",
  "Menlo",
  "Monaco",
  "JetBrains Mono",
  "Fira Code",
  "Ubuntu Mono",
  "Cascadia Code",
  "Hack",
  "Source Code Pro",
  "IBM Plex Mono",
  "Iosevka",
  "MesloLGS NF",
  "JetBrainsMono Nerd Font",
  "JetBrainsMono Nerd Font Mono",
  "FiraCode Nerd Font",
  "FiraCode Nerd Font Mono",
  "UbuntuMono Nerd Font",
  "Hack Nerd Font",
  "MesloLGS Nerd Font",
  "CaskaydiaCove Nerd Font",
  "SauceCodePro Nerd Font",
  "BlexMono Nerd Font",
  "Iosevka Nerd Font",
];

const KEY = "khor.term-font";
const CHANGED = "khor:term-font";

export type AddedFont = { family: string; file: string };
type Stored = AddedFont & { bytes: ArrayBuffer };

/** The full stack for a choice (`""` is the system's). */
export function stackOf(choice: string): string {
  const head = choice ? `"${choice.replace(/"/g, "")}", ` : "";
  return `${head}${SYSTEM}, "${SYMBOLS}", monospace`;
}

function storedChoice(): string {
  try {
    return window.localStorage.getItem(KEY) ?? "";
  } catch {
    return "";
  }
}

function wear(choice: string) {
  document.documentElement.style.setProperty("--font-term", stackOf(choice));
}

// ── the device's fonts ───────────────────────────────────────────────

let present: string[] | null = null;

/** The known faces this device has, by measurement. */
export function fontsHere(): string[] {
  if (present) return present;
  const ctx = document.createElement("canvas").getContext("2d");
  if (!ctx) return (present = []);
  const probe = "mmmmmmmmmmlli1O0WW@#";
  const width = (font: string) => {
    ctx.font = font;
    return ctx.measureText(probe).width;
  };
  const bases = ["monospace", "serif"].map((g) => [g, width(`72px ${g}`)] as const);
  present = KNOWN.filter((name) => bases.every(([g, w]) => Math.abs(width(`72px "${name}", ${g}`) - w) > 0.5));
  return present;
}

// ── added fonts (IndexedDB) ──────────────────────────────────────────

const faces = new Map<string, FontFace>();

function db(): Promise<IDBDatabase> {
  return new Promise((ok, no) => {
    const open = indexedDB.open("khor-fonts", 1);
    open.onupgradeneeded = () => open.result.createObjectStore("fonts", { keyPath: "family" });
    open.onsuccess = () => ok(open.result);
    open.onerror = () => no(open.error);
  });
}

async function all(): Promise<Stored[]> {
  try {
    const d = await db();
    return await new Promise((ok, no) => {
      const q = d.transaction("fonts").objectStore("fonts").getAll();
      q.onsuccess = () => ok(q.result as Stored[]);
      q.onerror = () => no(q.error);
    });
  } catch {
    return [];
  }
}

async function put(f: Stored): Promise<void> {
  const d = await db();
  await new Promise<void>((ok, no) => {
    const t = d.transaction("fonts", "readwrite");
    t.objectStore("fonts").put(f);
    t.oncomplete = () => ok();
    t.onerror = () => no(t.error);
  });
}

async function drop(family: string): Promise<void> {
  const d = await db();
  await new Promise<void>((ok, no) => {
    const t = d.transaction("fonts", "readwrite");
    t.objectStore("fonts").delete(family);
    t.oncomplete = () => ok();
    t.onerror = () => no(t.error);
  });
}

async function register(f: Stored): Promise<void> {
  const face = new FontFace(f.family, f.bytes.slice(0));
  await face.load();
  document.fonts.add(face);
  faces.set(f.family, face);
}

/**
 * A font file's own family name, from its `name` table (ID 16, the
 * typographic family, else ID 1). Read for TrueType and OpenType; a WOFF
 * or WOFF2 file is compressed, so its file name stands in.
 */
export function familyOf(bytes: ArrayBuffer, file: string): string {
  const stem = file.replace(/\.(ttf|otf|woff2?)$/i, "").replace(/[-_](Regular|Bold|Italic|Medium|Light|Book)$/i, "");
  try {
    const v = new DataView(bytes);
    const tag = v.getUint32(0);
    if (tag !== 0x00010000 && tag !== 0x4f54544f /* OTTO */ && tag !== 0x74727565 /* true */) return stem;
    const tables = v.getUint16(4);
    let nameAt = -1;
    for (let i = 0; i < tables; i++) {
      const rec = 12 + i * 16;
      if (v.getUint32(rec) === 0x6e616d65 /* name */) nameAt = v.getUint32(rec + 8);
    }
    if (nameAt < 0) return stem;
    const count = v.getUint16(nameAt + 2);
    const strings = nameAt + v.getUint16(nameAt + 4);
    const found = new Map<number, string>();
    for (let i = 0; i < count; i++) {
      const r = nameAt + 6 + i * 12;
      const platform = v.getUint16(r);
      const id = v.getUint16(r + 6);
      if (id !== 1 && id !== 16) continue;
      const len = v.getUint16(r + 8);
      const off = strings + v.getUint16(r + 10);
      let s = "";
      if (platform === 3 || platform === 0) {
        for (let k = 0; k < len; k += 2) s += String.fromCharCode(v.getUint16(off + k));
      } else if (platform === 1) {
        for (let k = 0; k < len; k++) s += String.fromCharCode(v.getUint8(off + k));
      }
      if (s && !found.has(id)) found.set(id, s);
    }
    return (found.get(16) ?? found.get(1) ?? stem).trim() || stem;
  } catch {
    return stem;
  }
}

// ── the choice ───────────────────────────────────────────────────────

export type TermFont = { choice: string; added: AddedFont[] };

let added: AddedFont[] = [];

function tell() {
  window.dispatchEvent(new CustomEvent(CHANGED, { detail: { choice: storedChoice(), added } }));
}

/** At startup (`main.tsx`): the choice on the root, the symbols face
    asked for so the first terminal does not draw before it, and the
    added fonts registered. */
export function wearTermFont() {
  wear(storedChoice());
  void document.fonts?.load(`14px "${SYMBOLS}"`, "").catch(() => {});
  void all().then(async (fonts) => {
    for (const f of fonts) {
      try {
        await register(f);
      } catch {
        // A file that stopped parsing is simply not offered.
        continue;
      }
    }
    added = fonts.filter((f) => faces.has(f.family)).map(({ family, file }) => ({ family, file }));
    tell();
  });
}

export function setTermFont(choice: string) {
  try {
    if (choice) window.localStorage.setItem(KEY, choice);
    else window.localStorage.removeItem(KEY);
  } catch {
    // Kept for this session only.
  }
  wear(choice);
  tell();
}

/** Takes a picked file: refused by size, refused when it does not parse,
    else registered, kept and chosen. Answers why it was refused. */
export async function addFontFile(file: File): Promise<"too-big" | "bad" | null> {
  if (file.size > FONT_MAX_MB * 1024 * 1024) return "too-big";
  const bytes = await file.arrayBuffer();
  const f: Stored = { family: familyOf(bytes, file.name), file: file.name, bytes };
  try {
    await register(f);
  } catch {
    return "bad";
  }
  try {
    await put(f);
  } catch {
    // Not kept past this session, but usable now.
  }
  added = [...added.filter((a) => a.family !== f.family), { family: f.family, file: f.file }];
  setTermFont(f.family);
  return null;
}

export async function removeFont(family: string) {
  const face = faces.get(family);
  if (face) document.fonts.delete(face);
  faces.delete(family);
  try {
    await drop(family);
  } catch {
    // Gone from this session either way.
  }
  added = added.filter((a) => a.family !== family);
  if (storedChoice() === family) setTermFont("");
  else tell();
}

export function useTermFont(): TermFont {
  const [t, setT] = useState<TermFont>(() => ({ choice: storedChoice(), added }));
  useEffect(() => {
    const on = (e: Event) => setT((e as CustomEvent<TermFont>).detail);
    window.addEventListener(CHANGED, on);
    return () => window.removeEventListener(CHANGED, on);
  }, []);
  return t;
}

/** A terminal's hook on the font: called with the new stack. */
export function onTermFont(run: (stack: string) => void): () => void {
  const on = (e: Event) => run(stackOf((e as CustomEvent<TermFont>).detail.choice));
  window.addEventListener(CHANGED, on);
  return () => window.removeEventListener(CHANGED, on);
}
