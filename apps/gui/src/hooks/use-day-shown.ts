// Whether the open session's day (`DayStrip`) is drawn under the tabs.
//
// **Hidden until asked for** (2026-09-29, the owner:「把这个状态bar设置为，
// 点击标签后显示吧」). The tab in front is the switch — pressed once the
// day comes down under it, pressed again it goes — and on the narrow face,
// which has no tabs, the detail screen's head stands in for that tab
// (`DetailPane`). The same morning he had reported a blank band under the
// tabs: the strip's no-data branch holding an empty row open while the
// day loaded, and for good on a session with no record. Hidden by default
// is also the end of that band.
//
// **One switch for this face, not one per session.** It is a way of
// looking at sessions, not a fact about one — a person who wants the day
// wants it on the next session too — and a per-session memory would be a
// list that grows with every session ever opened.
//
// **Kept on this device** (`localStorage`, the `khor.*` keys beside it:
// `use-list-width`, `session-filter`): a habit of this screen, not a
// decision about the network. Storage can throw — a private window,
// blocked site data — and a throw reads as 「hidden」, the default: all it
// costs is one press.
//
// A store outside React with `useSyncExternalStore`, because two places
// read it at once — the tab strip's hint (`App`) and the pane that draws
// the day (`DetailPane`) — and both must turn on the same press.
import { useSyncExternalStore } from "react";

const KEY = "khor.sessions.day";

function stored(): boolean {
  try {
    return window.localStorage.getItem(KEY) === "1";
  } catch {
    return false;
  }
}

let shown = stored();
const heard = new Set<() => void>();

/** Show or hide the day, everywhere this face draws it, and remember it. */
export function setDayShown(next: boolean): void {
  shown = next;
  try {
    window.localStorage.setItem(KEY, next ? "1" : "0");
  } catch {
    /* the day still turns; only the memory of it is lost */
  }
  for (const tell of heard) tell();
}

function listen(tell: () => void): () => void {
  heard.add(tell);
  return () => {
    heard.delete(tell);
  };
}

/** Whether the day is asked for. Not whether it is drawn: a day with no
    record draws nothing either way (`dayHasRecord`). */
export function useDayShown(): boolean {
  return useSyncExternalStore(listen, () => shown, () => false);
}
