// **A bell rung off screen marks its session** (the terminal audit,
// item 67). iTerm2 marks a tab whose bell rang while it was not in front;
// khor's bell (a flash, a haptic) only reached a pane on screen, so a
// bell in a session behind another tab, on another page, or with the app
// hidden went unheard.
//
// The host records when the terminal last rang (`Doing.bell_ms`, one a
// second at most), so a session with no pane mounted is heard too. This
// keeps, per session, the last bell this face has seen, and answers which
// sessions rang since and are not on screen: those wear the mark on their
// tab and their row until they are on screen. The first sight of a session
// only records where it stands (a bell from before this face looked is
// not news), as `FinishedWatch` does with notices. The bell setting off
// (`termBell`) marks nothing.
import type { SessionRow } from "@/api";
import { termBell } from "@/lib/term-bell";

/** The sessions whose bell rang off screen since this face last saw them.
    `seen` is the face's own record (session → the last bell seen), kept
    up to date here: a session on screen has seen its bell. */
export function rungOffScreen(rows: readonly SessionRow[], seen: Map<string, number>, onScreen: (id: string) => boolean): Set<string> {
  const out = new Set<string>();
  const on = termBell();
  for (const r of rows) {
    const at = r.doing?.bell_ms ?? 0;
    const was = seen.get(r.id);
    if (was === undefined || !on || onScreen(r.id)) {
      seen.set(r.id, at);
      continue;
    }
    if (at > was) out.add(r.id);
  }
  return out;
}
