// The node is back from the background, and has caught up
// (docs/IOS.md 第三步).
//
// A phone's app is suspended seconds after it leaves the screen, and its
// node hears nothing while it is. On the way back the shell gives the
// node new ears and runs a sync round at once (`resume_the_node`), then
// says `node:woken` — the moment what the others wrote meanwhile is on
// this disk. A list that re-asks then shows it without waiting out its
// beat.
//
// **A count, used as an effect's dependency** — the same shape `hidden`
// already has in the effects that poll (`App`): coming back re-runs the
// effect, and the effect asks once before it sets its interval, so there
// is nothing else to wire and nothing to clear. The page's own
// `visibilitychange` re-asks too, earlier; that answer is what the node
// had before its round, and this one is what it has after.
import { useEffect, useState } from "react";

import { hear } from "@/api";

export function useWoken(): number {
  const [woken, setWoken] = useState(0);
  useEffect(() => hear<number>("node:woken", () => setWoken((n) => n + 1)), []);
  return woken;
}
