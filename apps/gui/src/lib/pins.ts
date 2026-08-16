// Pinning, for every list pane in the rail.
//
// ## Why this is local, and why that is not a shortcut
//
// A pin says "this one matters *to me, here*". It is not a fact about
// the machine or the session — those live in the mesh and replicate, and
// this deliberately does not (docs/handoff 账本 has the entry, with what
// would have to be true for it to become a synced preference). Two people
// on two machines rearranging one shared list to suit themselves is the
// failure this avoids, and it is the same reason mandala keeps its
// synthetic rows' pins on the box they are looked at from.
//
// `localStorage` is the whole mechanism. Both faces the app has get it:
// the tauri window persists it in the webview's own store, the dev bridge
// in the browser's, and neither needs the node to grow a preference file
// for something the node has no opinion about.
//
// ## One store, one namespace per pane
//
// Pinning a machine on the files pane does not pin it on the devices
// pane. That is a judgment, not a technical consequence — the panes will
// stop showing the same rows as soon as files and browser grow their own
// content, and a pin that followed a machine into every pane would then
// be pinning it in places its owner never looked at. The shape says so:
// one record, keyed by pane, and nothing reads across the keys.
//
// Keys are the row's stable id, and stale ones are simply never matched —
// a pinned machine that leaves the table takes its pin out of play
// without anyone having to clean up. Storing the whole row instead would
// mean deciding what to do when the stored copy and the live one differ,
// which is a question with no good answer and no need to ask.

const STORAGE_KEY = "khor.pins";

const EMPTY: ReadonlySet<string> = new Set();

type Pins = Readonly<Record<string, ReadonlySet<string>>>;

function read(): Pins {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    const parsed: unknown = raw ? JSON.parse(raw) : null;
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) return {};
    const out: Record<string, ReadonlySet<string>> = {};
    for (const [scope, keys] of Object.entries(parsed as Record<string, unknown>)) {
      if (Array.isArray(keys)) {
        out[scope] = new Set(keys.filter((k): k is string => typeof k === "string"));
      }
    }
    return out;
  } catch {
    // Unreadable or unavailable storage means no pins this session, not
    // a broken pane: the fallback lands on the side where every list is
    // simply in its natural order.
    return {};
  }
}

let snapshot: Pins = read();
const listeners = new Set<() => void>();

function commit(next: Pins) {
  // A fresh object every time, or `useSyncExternalStore` cannot tell
  // that anything changed and the list never repaints.
  snapshot = next;
  try {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify(Object.fromEntries(Object.entries(next).map(([s, k]) => [s, [...k]]))),
    );
  } catch {
    // Out of quota or storage denied: the pin still works for this
    // session, and saying so would be an error message about the
    // browser, which is not something the user can act on.
  }
  for (const l of listeners) l();
}

export function subscribePins(fn: () => void): () => void {
  listeners.add(fn);
  return () => listeners.delete(fn);
}

/** The whole record. Stable between commits — `useSyncExternalStore`
    compares by identity and would loop on a freshly built object. */
export function pinsSnapshot(): Pins {
  return snapshot;
}

export function pinsIn(all: Pins, scope: string): ReadonlySet<string> {
  return all[scope] ?? EMPTY;
}

/** Pin or unpin `key` within one pane. */
export function togglePin(scope: string, key: string): void {
  const next = new Set(pinsIn(snapshot, scope));
  if (!next.delete(key)) next.add(key);
  commit({ ...snapshot, [scope]: next });
}

/**
 * Pinned rows first, **everything else in the order it arrived**.
 *
 * A partition, not a sort: the node has already decided what order this
 * list is in (docs/UX.md 状态呈现), and a comparator here would be a
 * second ranking quietly competing with that one. Two buckets filled in
 * one pass keep both groups internally in the node's order, which is why
 * unpinning puts a row back exactly where it was rather than somewhere
 * plausible.
 */
export function pinnedFirst<T>(
  rows: readonly T[],
  keyOf: (row: T) => string,
  pinned: ReadonlySet<string>,
): T[] {
  if (pinned.size === 0) return [...rows];
  const up: T[] = [];
  const down: T[] = [];
  for (const row of rows) (pinned.has(keyOf(row)) ? up : down).push(row);
  return [...up, ...down];
}
