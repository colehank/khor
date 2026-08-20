// Whether anybody is looking at this window.
//
// **Slowed, not stopped, and the reason is what happens when the wake-up
// is missed.** A loop that stops entirely depends on one event to start
// again, and if that event never arrives — a listener torn down by a
// re-render, a tab restored from memory, a browser that fires it
// differently — the pane never updates again. A pane that never updates
// is indistinguishable from the app being broken, and it is the kind of
// failure nobody reports because it looks like nothing happened. A slow
// beat repairs itself within one interval and gives up almost the same
// work: the terminal's twenty polls a second become one every ten.
//
// The wake-up is still wired, because ten seconds of a stale screen on
// return is its own kind of wrong. It is the optimisation, not the
// mechanism — the difference being that when it fails the app is merely
// late instead of dead.
import { useEffect, useState } from "react";

/** The beat every poll in this app falls back to while it is not being
    looked at. One number, so "how much does a hidden window cost" has a
    single answer rather than one per pane. */
export const HIDDEN_MS = 10_000;

export function useHidden(): boolean {
  const [hidden, setHidden] = useState(() => document.hidden);
  useEffect(() => {
    const on = () => setHidden(document.hidden);
    document.addEventListener("visibilitychange", on);
    // Read once on the way in as well: this mounts after the event may
    // already have happened (a pane opened while the window was in the
    // background), and a state initialised at first render is not
    // re-read by anything else.
    on();
    return () => document.removeEventListener("visibilitychange", on);
  }, []);
  return hidden;
}
