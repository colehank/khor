// Which browsing tabs are loading right now (docs/SHELL.md 第十二轮), for
// the one thing that draws it: the breath around that row's face in the
// sidebar (`FaceMark`'s `[data-halo]`, the ring a 忙碌 session wears).
//
// **Two sources, because the two faces know different things.**
//
//   - The app has the page itself. Its child webview reports `page:load`
//     with `started` and `finished` for every tab, foreground or not
//     (`src-tauri/src/lib.rs`), which is the fact exactly — heard here
//     for every tab at once rather than per pane, so a tab that loads
//     while you are looking at another one still breathes in the list.
//   - The web face has no page at all: browsing borrows a machine's
//     network into a webview the app owns, and a browser tab cannot be
//     handed one. What that face can honestly say is what **khor** is
//     doing for the tab — opening the borrow, fetching the site's icon
//     through it — so the body reports its own errands and they stand in
//     (`BrowserPane`'s `onBusy`). It lights on the same press and goes
//     out when khor's errand to that site comes back, which is the
//     closest true statement available there.
//
// **In the app the errands do not put it out**, only the page does:
// `open_page` resolves as soon as the webview exists, long before the
// site has sent anything, and letting that end the breath would put the
// row out while the page is still arriving.
//
// The ceiling over both is `loading-rule.ts`, which is where the whole
// rule lives and where it is measured.
//
// **One pulse from a load's start to its end** (UX-22, the owner
// 2026-10-05: 「the status light flashes wildly」): a page that redirects,
// or moves itself with `location =`, is several loads to WebKit and each
// one reported its own start and finish, so the ring went out and came
// back — and restarted its breath — on every hop. A finish is held for
// `SETTLE_MS` and a start inside that time carries on the same pulse.
import { useCallback, useEffect, useRef, useState } from "react";

import { hear, inApp } from "@/api";
import { alive, ended, loading, NONE, started, type Loads } from "@/browse/loading-rule";

/** How often the ceiling is swept — the delay between a load running out
    of time and the row going quiet, which nobody can see. */
const BEAT_MS = 1_000;

/** How long a load's finish is held before the light goes out: a hop of
    a redirect chain arrives well inside it. */
export const SETTLE_MS = 600;

export type Loading = {
  /** Is this tab loading? Ask at paint: the ceiling is counted from the
      clock, not from a sweep. */
  busy: (id: string) => boolean;
  /** The body says what khor itself is doing for this tab. `true` lights
      it on every face; `false` puts it out only where there are no page
      events to hear (see the head). */
  errand: (id: string, busy: boolean) => void;
};

export function useLoading(): Loading {
  const [loads, setLoads] = useState<Loads>(NONE);
  /** Each tab's finish, held (`SETTLE_MS`). */
  const settling = useRef(new Map<string, ReturnType<typeof setTimeout>>());
  // The app's own report, for every tab. Subscribed once for the life of
  // the landing: this is not per tab, and a subscription per row would
  // be the same event heard n times.
  useEffect(
    () =>
      hear<{ tab: string; text: string }>("page:load", (said) => {
        const held = settling.current.get(said.tab);
        if (held !== undefined) {
          clearTimeout(held);
          settling.current.delete(said.tab);
        }
        if (said.text === "started") {
          // Carried on: a start inside the hold is the same pulse, and
          // the ceiling's clock is not restarted by a hop.
          setLoads((was) => (held !== undefined && was.has(said.tab) ? was : started(was, said.tab, Date.now())));
          return;
        }
        settling.current.set(
          said.tab,
          setTimeout(() => {
            settling.current.delete(said.tab);
            setLoads((was) => ended(was, said.tab));
          }, SETTLE_MS),
        );
      }),
    [],
  );
  // The ceiling, swept **only while something is loading** — an app that
  // is not browsing anything runs no timer at all.
  const any = loads.size > 0;
  useEffect(() => {
    if (!any) return;
    const beat = setInterval(() => setLoads((was) => alive(was, Date.now())), BEAT_MS);
    return () => clearInterval(beat);
  }, [any]);
  // Read at paint, so the row goes quiet the moment the ceiling passes
  // even between two beats.
  const now = Date.now();
  const at = useRef(now);
  at.current = now;
  const busy = useCallback((id: string) => loading(loads, id, at.current), [loads]);
  const errand = useCallback((id: string, going: boolean) => {
    if (going) setLoads((was) => started(was, id, Date.now()));
    else if (!inApp) setLoads((was) => ended(was, id));
  }, []);
  return { busy, errand };
}
