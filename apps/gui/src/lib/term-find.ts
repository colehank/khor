// **The terminal's find: regex and case** (the terminal audit, item 70;
// iTerm2's and VS Code's find bars). Two toggles beside the field, handed
// to xterm's search addon as its own `regex` and `caseSensitive`. Both
// off by default (a plain word, any case — what the bar did before).
// This device's, in its `localStorage`, as the bell setting is.
import { useState } from "react";

export type FindHow = { regex: boolean; caseSensitive: boolean };

const KEY = "khor.term-find";
const PLAIN: FindHow = { regex: false, caseSensitive: false };

export function findHow(): FindHow {
  try {
    const v = JSON.parse(window.localStorage.getItem(KEY) ?? "null") as Partial<FindHow> | null;
    return { regex: v?.regex === true, caseSensitive: v?.caseSensitive === true };
  } catch {
    return PLAIN;
  }
}

function keep(how: FindHow) {
  try {
    window.localStorage.setItem(KEY, JSON.stringify(how));
  } catch {
    // Kept for this pane only.
  }
}

/** The two toggles, read once and remembered on every change. */
export function useFindHow(): [FindHow, (how: FindHow) => void] {
  const [how, setHow] = useState(findHow);
  return [
    how,
    (next) => {
      keep(next);
      setHow(next);
    },
  ];
}

/** Whether `q` can be searched as `how` says: an unfinished pattern
    (`a(`) is no search yet, not an error. */
export function searchable(q: string, how: FindHow): boolean {
  if (!how.regex) return true;
  try {
    new RegExp(q);
    return true;
  } catch {
    return false;
  }
}
