// **How many lines of history a terminal keeps** (item 17, Appearance →
// Terminal: the owner chose to make item 15's 10 000 a setting).
//
// xterm keeps 12 bytes a cell (`BufferLine`, three Uint32 per cell), so a
// line costs its width: measured 0.55 KB at the phone's 46 columns and
// 1.08 KB at the Mac's 90. The default follows the device — 5000 lines on
// a phone (2.7 MB a terminal), 10 000 on a Mac (10.5 MB) — and the row
// says the cost of the choice. After a re-attach a terminal holds what the
// host's ring replays (`RING` in `host.rs`, 1 MiB: 13 000–26 000 lines at
// the measured 42–80 bytes a line), so the largest choice can outrun it.
//
// This device's, in its `localStorage` like the terminal's font, size,
// cursor and tap settings. A face that refuses storage keeps the default.
import { useEffect, useState } from "react";

export const HISTORY_CHOICES = [1000, 2000, 5000, 10_000, 20_000] as const;

const KEY = "khor.term-history";
const CHANGED = "khor:term-history";

/** Whether this device is held in the hand: the phone's default and cost. */
const fingered = () => typeof window !== "undefined" && window.matchMedia("(pointer: coarse)").matches;

/** KB one line of history costs on this device (measured, see above). */
export const lineKb = () => (fingered() ? 0.55 : 1.08);

/** This session's choice once it has been set, for a face whose storage
    refuses it. */
let session: number | null = null;

export function termHistory(): number {
  if (session !== null) return session;
  const fallback = fingered() ? 5000 : 10_000;
  try {
    const n = Number(window.localStorage.getItem(KEY));
    return (HISTORY_CHOICES as readonly number[]).includes(n) ? n : fallback;
  } catch {
    return fallback;
  }
}

export function setTermHistory(lines: number) {
  session = lines;
  try {
    window.localStorage.setItem(KEY, String(lines));
  } catch {
    // Kept for this session only.
  }
  window.dispatchEvent(new CustomEvent(CHANGED, { detail: lines }));
}

/** Calls `then` with every later choice; answers the way to stop. */
export function onTermHistory(then: (lines: number) => void): () => void {
  const changed = (e: Event) => then((e as CustomEvent<number>).detail);
  window.addEventListener(CHANGED, changed);
  return () => window.removeEventListener(CHANGED, changed);
}

export function useTermHistory(): number {
  const [n, setN] = useState(termHistory);
  useEffect(() => onTermHistory(setN), []);
  return n;
}
