// **The two text sizes** (the redesign, 2026-10-02, 外观 → 字号).
//
// 界面字号 scales the whole type ramp through one number, `--ui-scale`
// (`tokens.css`). By default it **follows the system**: WebKit tells a
// page the reader's text size through the `-apple-system-body` font
// keyword, and the scale is that size over **the platform's own default
// body** — 17 on iOS (Dynamic Type's Large), 13 on the Mac. A platform
// that tells a page nothing (a desktop browser) answers 1. Off, the
// slider's step is the scale — iOS's seven text sizes, xS to xxxL, over
// 17.
//
// ⚠️ It divided by 17 everywhere until UX-9: the Mac's web view answers
// 13, so the whole Mac face came up at 13/17 of its tokens — row names at
// 9.9 px — on every store that had never touched the slider.
//
// 终端字号 is the terminal's own (`--text-term`), a slider of whole
// points; open terminals take it at once (`TerminalPane` listens).
//
// Kept in this face's `localStorage`, like the navigation's order
// (`lib/nav-order`): the store has no keys for them yet. A face that
// refuses storage follows the system and the default terminal size.
import { useEffect, useState } from "react";

import { heardFrom } from "@/lib/other-windows";

/** iOS's body sizes, xS…xxxL; the fourth (17) is the default. */
export const UI_STEPS = [14, 15, 16, 17, 19, 21, 23] as const;
const UI_BASE = 17;

/** **The body size the system answers at its default, on this platform**
    — read once from the platform, never inferred from the answer (a Mac
    whose reader raised the text size answers more than 13, and that is
    the size to follow). iOS 17; the Mac 13. An iPad that asks for the
    desktop site says Macintosh too, and touch is what tells it apart. */
const SYSTEM_BODY =
  typeof navigator !== "undefined" && /Mac/.test(navigator.platform) && navigator.maxTouchPoints === 0 ? 13 : UI_BASE;
export const TERM_MIN = 10;
export const TERM_MAX = 22;
export const TERM_DEFAULT = 14;

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
  return Number.isFinite(px) && px > 0 ? px / SYSTEM_BODY : 1;
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
  // **Set in another window of this app** (fix-c, 2026-10-08, the owner:
  // 字号「改不动」). On the Mac 外观 is the settings window, a page of its
  // own: its slider wrote the store and wore the size on itself, and the
  // main window heard nothing — the changed event is a window's own, and
  // a window beside it is never hidden, so `visibilitychange` never came
  // either. The size took only at the next launch. The store says so to
  // every other page of this origin (`lib/other-windows`); this one wears
  // it and tells its terminals and rows, as if the slider were its own.
  heardFrom(KEY, () => {
    const t = stored();
    wear(t);
    window.dispatchEvent(new CustomEvent(CHANGED, { detail: t }));
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
