// **The file page's 字号** (round UX-3, its 「…」): how large a file reads,
// apart from the interface's size (`lib/type-size`) — a lock file read on
// a phone wants to be smaller than the app, a long README larger. Five
// steps around the interface's own, per device, in this browser's storage
// like the other type sizes (not a node setting: it is about this screen).
//
// Worn as the two text sizes a file view uses (`--text-mono` for code,
// `--text-read` for rendered Markdown), each a multiple of the root's own
// value (`--fs-mono`, `--fs-read`), so no size is written twice.
import { useEffect, useState, type CSSProperties } from "react";

export const FILE_STEPS = [0.85, 1, 1.15, 1.3, 1.5] as const;
const BASE = 1;

const KEY = "khor.file-size";
const CHANGED = "khor:file-size";

function stored(): number {
  try {
    const n = Number(window.localStorage.getItem(KEY));
    return Number.isInteger(n) && n >= 0 && n < FILE_STEPS.length && window.localStorage.getItem(KEY) !== null ? n : BASE;
  } catch {
    return BASE;
  }
}

/** The step and a way to move it; every open file page follows. */
export function useFileSize(): { step: number; set: (step: number) => void } {
  const [step, setStep] = useState(stored);
  useEffect(() => {
    const again = () => setStep(stored());
    window.addEventListener(CHANGED, again);
    return () => window.removeEventListener(CHANGED, again);
  }, []);
  const set = (next: number) => {
    const n = Math.max(0, Math.min(FILE_STEPS.length - 1, next));
    try {
      window.localStorage.setItem(KEY, String(n));
    } catch {
      // A private window keeps it for this page only.
    }
    setStep(n);
    window.dispatchEvent(new Event(CHANGED));
  };
  return { step, set };
}

/** The sizes a file view reads, scaled by `step`; nothing at the base. */
export function fileSizeStyle(step: number): CSSProperties | undefined {
  const k = FILE_STEPS[step] ?? 1;
  if (k === 1) return undefined;
  return {
    "--text-mono": `calc(var(--fs-mono) * ${k})`,
    "--text-mono--line-height": `calc(var(--lh-mono) * ${k})`,
    "--text-read": `calc(var(--fs-read) * ${k})`,
    "--text-read--line-height": `calc(var(--lh-read) * ${k})`,
  } as CSSProperties;
}
