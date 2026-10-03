// **The two text sizes** (the redesign, 2026-10-02, 外观 → 字号).
//
// 界面字号 scales the whole type ramp through one number, `--ui-scale`
// (`tokens.css`). By default it **follows the system**: iOS tells a page
// the reader's text size through the `-apple-system-body` font keyword
// (Dynamic Type), and the scale is that size over the default body's 17.
// A platform that tells a page nothing (a desktop browser, the Mac app's
// web view at its default) answers 1. Off, the slider's step is the scale
// — iOS's seven text sizes, xS to xxxL, over the same 17.
//
// 终端字号 is the terminal's own (`--text-term`), a slider of whole
// points; open terminals take it at once (`TerminalPane` listens).
//
// Kept in this face's `localStorage`, like the navigation's order
// (`lib/nav-order`): the store has no keys for them yet. A face that
// refuses storage follows the system and the default terminal size.
import { useEffect, useState } from "react";

/** iOS's body sizes, xS…xxxL; the fourth (17) is the default. */
export const UI_STEPS = [14, 15, 16, 17, 19, 21, 23] as const;
const UI_BASE = 17;
export const TERM_MIN = 10;
export const TERM_MAX = 22;
const TERM_DEFAULT = 14;

export type TypeSize = { followSystem: boolean; uiStep: number; term: number };

const KEY = "khor.type-size";
const CHANGED = "khor:type-size";

function stored(): TypeSize {
  const fallback: TypeSize = { followSystem: true, uiStep: 3, term: TERM_DEFAULT };
  try {
    const raw = JSON.parse(window.localStorage.getItem(KEY) ?? "null") as Partial<TypeSize> | null;
    if (!raw || typeof raw !== "object") return fallback;
    return {
      followSystem: typeof raw.followSystem === "boolean" ? raw.followSystem : true,
      uiStep: clamp(Math.round(Number(raw.uiStep)), 0, UI_STEPS.length - 1, 3),
      term: clamp(Math.round(Number(raw.term)), TERM_MIN, TERM_MAX, TERM_DEFAULT),
    };
  } catch {
    return fallback;
  }
}

function clamp(n: number, lo: number, hi: number, otherwise: number): number {
  return Number.isFinite(n) ? Math.min(hi, Math.max(lo, n)) : otherwise;
}

/** The sizes as stored now (a pinch reads the terminal's from here). */
export function typeSize(): TypeSize {
  return stored();
}

/** The system's text size as a scale, where the platform says one. */
export function systemScale(): number {
  if (typeof CSS === "undefined" || !CSS.supports("font", "-apple-system-body")) return 1;
  const probe = document.createElement("span");
  probe.style.font = "-apple-system-body";
  probe.style.position = "absolute";
  probe.style.visibility = "hidden";
  document.body.appendChild(probe);
  const px = Number.parseFloat(getComputedStyle(probe).fontSize);
  probe.remove();
  return Number.isFinite(px) && px > 0 ? px / UI_BASE : 1;
}

/** Stamp the sizes on the root, where the ramp and the terminal read them. */
function wear(t: TypeSize) {
  const root = document.documentElement.style;
  const scale = t.followSystem ? systemScale() : UI_STEPS[t.uiStep] / UI_BASE;
  root.setProperty("--ui-scale", String(scale));
  root.setProperty("--text-term", `${t.term}px`);
  // The leading keeps the token's ratio (20 over 14) for the pane's own
  // text and 外观's sample line; xterm's rows take no leading
  // (`TerminalPane`'s `fontOf`).
  root.setProperty("--text-term--line-height", `${Math.round(t.term * (20 / 14))}px`);
}

/** At startup, before the first frame (`main.tsx`), and again whenever the
    page comes back to the front — the system size may have changed while
    it was away. */
export function wearTypeSize() {
  wear(stored());
  document.addEventListener("visibilitychange", () => {
    if (document.visibilityState === "visible") wear(stored());
  });
}

export function setTypeSize(t: TypeSize) {
  try {
    window.localStorage.setItem(KEY, JSON.stringify(t));
  } catch {
    // Kept for this session only.
  }
  wear(t);
  window.dispatchEvent(new CustomEvent(CHANGED, { detail: t }));
}

export function useTypeSize(): TypeSize {
  const [t, setT] = useState<TypeSize>(stored);
  useEffect(() => {
    const on = (e: Event) => setT((e as CustomEvent<TypeSize>).detail);
    window.addEventListener(CHANGED, on);
    return () => window.removeEventListener(CHANGED, on);
  }, []);
  return t;
}

/** A terminal's hook on the size: called with the new point size. */
export function onTermSize(run: (px: number) => void): () => void {
  const on = (e: Event) => run((e as CustomEvent<TypeSize>).detail.term);
  window.addEventListener(CHANGED, on);
  return () => window.removeEventListener(CHANGED, on);
}
