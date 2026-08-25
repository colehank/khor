// What the corner strip is allowed to carry, and for how long.
//
// **Two admission rules, and they are the whole design.** Everything
// else here is bookkeeping.
//
// 1. **Only process-class facts** — things that outlive the click that
//    started them and go on happening while nobody is looking. A file
//    moving between machines is one. A button press is not.
// 2. **Only facts the node already has a row for.** Every item here is
//    keyed by a session id, and its word is the word that row wears. The
//    strip never mints a state, never phrases an outcome, and cannot
//    show anything the list could not also show.
//
// **What that keeps out is the point.** A pin that would not take says
// so on the pin; a close that was refused says so on the pane that asked
// (docs/UX.md — 失败就地说, and this app has nowhere that collects
// messages). Those rulings do not move here, because the moment a
// corner box will accept "something went wrong" it becomes the place
// every failure goes, and then it is a notification centre — a surface
// with its own rules that nobody asked for. The rule is written down
// rather than merely intended, because "just this one click result" is
// how it would actually happen.
//
// **It can always go to zero.** A transfer that finishes leaves on its
// own after a beat; one that broke stays until it is taken off by hand,
// because a failure that vanishes was never reported. There is no third
// path — every item that enters has a way out.
import { useCallback, useEffect, useRef, useState } from "react";

import type { SessionRow } from "@/api";

/** One line in the corner. `word` is the row's state key, never a word
    this layer chose; `title` is what the row calls itself. */
export type StatusItem = {
  id: string;
  word: string;
  title: string;
  /** True once the row reached an ending that leaves by itself. */
  settled: boolean;
};

/** How long a finished item stays before it goes. Long enough to read a
    filename, short enough that nobody reaches to dismiss it. */
const LINGER_MS = 3_000;

/** The endings that stay put. A failure that disappeared on its own was
    not reported — it was hidden (docs/UX.md 做了但没变化 / 失败). */
const STICKS = ["errored", "failed"];

/** The words only a transfer can press onto a machine's chat row
    (传输并入对话批: the retired transfer rows' pressure lands there —
    `TransferKind::urgency`). 忙碌 = bytes flowing, 中断 = a resumable
    break. **待批 is deliberately not here**: an offer waiting for this
    machine already has the badge and the top of the list, and the strip
    only carries what is *happening* — the same judgment the retired
    shape made by never admitting a row it first met at 待批. */
const PRESS = ["busy", "errored"];

export function useStatusBar(rows: SessionRow[]): {
  items: StatusItem[];
  dismiss: (id: string) => void;
} {
  const [live, setLive] = useState<StatusItem[]>([]);
  // Ids taken off by hand. Kept so the next poll does not put a
  // dismissed failure straight back — the row is still there and still
  // says 失败, which is right for the *list* and wrong for a strip the
  // person has already answered.
  const dropped = useRef<Set<string>>(new Set());
  // The word each transfer row wore when this app last looked. A row
  // first seen is recorded and nothing more — see the entry rule below.
  const seen = useRef<Map<string, string>>(new Map());

  const dismiss = useCallback((id: string) => {
    dropped.current.add(id);
    setLive((prev) => prev.filter((i) => i.id !== id));
  }, []);

  useEffect(() => {
    // **The bookkeeping happens here, not inside the updater below.**
    // React calls a state updater twice under StrictMode, and this ref
    // is what "changed since last time" is measured against — written
    // in there, the second call would compare the new word against
    // itself, find no change, and admit nothing. Measured: with the
    // write inside the updater the strip stayed empty through a real
    // transfer, in dev only, which is the whole app.
    //
    // **Entry is a change, not a state.** Two failures fall out of that
    // one choice:
    //
    // - "enter while 忙碌" misses a transfer that finished between two
    //   polls, and a small file over a fast link finishes between two
    //   polls every time — the strip would only ever show the slow
    //   ones, which is the opposite of useful.
    // - "enter whenever the row is not resting" announces every
    //   transfer that ever happened the moment the app opens, and a
    //   corner that is full at startup is a corner nobody reads.
    //
    // A word that changed while this app was watching is exactly
    // "something happened just now"; a row first seen already finished
    // is history, and history is what the list is for.
    const moved: SessionRow[] = [];
    for (const r of rows) {
      // Rule 1 and 2 together: a process-class fact, read off a row.
      // The carrier changed in 传输并入对话批 — transfers no longer
      // stand as rows, their pressure lands on the machine's chat row
      // (`TransferKind::urgency`) — so the strip now watches chat rows
      // for **transfer-pressed movement**: a change into a PRESS word
      // (a fetch started, a break), out of one (it finished), or out
      // of 待批 (the offer was answered — the old shape's own entry
      // moment, 待批 → 完成). An ordinary message arriving or being
      // read moves between 完成/空闲 and touches neither side, so the
      // strip stays deaf to chatter — it is not a notification centre
      // (the module head's whole point).
      if (r.kind !== "chat") continue;
      const before = seen.current.get(r.id);
      seen.current.set(r.id, r.word);
      if (dropped.current.has(r.id)) continue;
      if (before === undefined || before === r.word) continue;
      const pressed = PRESS.includes(r.word);
      const was = PRESS.includes(before) || before === "blocked";
      if (pressed || was) moved.push(r);
    }
    setLive((prev) => {
      const byId = new Map(prev.map((i) => [i.id, i]));
      for (const r of moved) {
        byId.set(r.id, {
          id: r.id,
          word: r.word,
          title: r.title || r.id,
          // Out of the pressure words = this line is done saying it;
          // 中断 is inside PRESS, so a break stays until dismissed
          // (STICKS' rule: a failure that vanishes was never reported).
          settled: !PRESS.includes(r.word),
        });
      }
      // A row that left the list takes its line with it.
      const alive = new Set(rows.map((r) => r.id));
      const next = [...byId.values()].filter((i) => alive.has(i.id));
      const same =
        next.length === prev.length &&
        next.every((i, n) => {
          const was = prev[n];
          return was && was.id === i.id && was.word === i.word && was.settled === i.settled;
        });
      return same ? prev : next;
    });
  }, [rows]);

  // The beat before a finished line goes. One timer per settled item,
  // cleared on the way out so a line that changed its mind (a resumed
  // transfer turning 忙碌 again) is not removed by a stale timer.
  useEffect(() => {
    const timers = live
      .filter((i) => i.settled)
      .map((i) =>
        window.setTimeout(() => {
          setLive((prev) => prev.filter((x) => x.id !== i.id));
        }, LINGER_MS),
      );
    return () => timers.forEach((t) => window.clearTimeout(t));
  }, [live]);

  return { items: live, dismiss };
}

export { STICKS as STATUS_STICKS };
