/**
 * What a page last showed, kept across its unmount (docs/SHELL.md
 * 第十六轮; design-system `30-behavior.md`: 「回到一页，第一帧就是上次
 * 的内容，然后原地刷新」).
 *
 * **In memory, for this window's life, by key.** A page that comes back
 * mounts with the last answer it had and asks again in place; the new
 * answer replaces the old one without the old one ever being cleared.
 * Nothing here goes to disk: an answer kept across a restart would be a
 * reading of unknown age, and the ages on screen are the thing a person
 * reads to know how fresh a number is.
 *
 * It started as the 额度 landing's module-level `kept` (the narrow face
 * remounts that landing to slide a screen in, and a board back at null
 * sent the detail screen straight back); the machine card and the quota
 * details had the same need and no copy of it.
 */
import { useCallback, useRef, useState } from "react";

const store = new Map<string, unknown>();

/** The last value kept under `key`, or `undefined`. */
export function keptGet<T>(key: string): T | undefined {
  return store.get(key) as T | undefined;
}

/** Keeps `value` under `key`. */
export function keptSet<T>(key: string, value: T): void {
  store.set(key, value);
}

/**
 * `useState` whose value outlives the component, under `key`.
 *
 * Mounts with what was last kept under the key (else `initial`); when
 * the key changes — a different machine picked — it switches to what
 * that key kept, rather than to nothing. Every set writes through.
 */
export function useKept<T>(key: string, initial: T): [T, (next: T | ((was: T) => T)) => void] {
  const read = (k: string) => (store.has(k) ? (store.get(k) as T) : initial);
  const [value, setValue] = useState<T>(() => read(key));
  // A key change swaps in that key's own last value during render —
  // React's pattern for state derived from a prop — so no frame shows
  // one machine's answer under another's name.
  const [shownKey, setShownKey] = useState(key);
  if (shownKey !== key) {
    setShownKey(key);
    setValue(read(key));
  }
  // The key a set writes under, current at the time of the set: an
  // answer that lands after the key moved on belongs to the old key.
  const keyRef = useRef(key);
  keyRef.current = key;
  // ⚠️ **The store is written at the call, not inside a state updater.**
  // An updater runs on the next render, and a press that also swaps the
  // screen unmounts this component first — the narrow 额度 face does
  // exactly that — so the updater never ran and the pick was lost (face
  // gate: 「pressing the narrow 额度 row acc-fill drew acc-sub-here」).
  const valueRef = useRef(value);
  valueRef.current = value;
  const set = useCallback((next: T | ((was: T) => T)) => {
    const now = typeof next === "function" ? (next as (was: T) => T)(valueRef.current) : next;
    valueRef.current = now;
    store.set(keyRef.current, now);
    setValue(now);
  }, []);
  return [value, set];
}
