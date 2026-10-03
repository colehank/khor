// **A screen that takes the whole window on the narrow face** (round UX-3):
// the window's first row and the landings' bar step aside, as they do for
// a session or a browsing page (`App`'s `fullScreen`). Those two the shell
// knows from its own state; a file opened inside a workspace or a machine
// page is the pane's own state, so the pane claims it here while it shows.
import { createContext, useContext, useLayoutEffect } from "react";

type Claim = () => () => void;

const Claims = createContext<Claim | null>(null);

export const FullScreenProvider = Claims.Provider;

/** Holds the narrow face full screen while `on` (and while mounted). */
export function useFullScreen(on: boolean): void {
  const claim = useContext(Claims);
  useLayoutEffect(() => {
    if (!on || claim === null) return;
    return claim();
  }, [on, claim]);
}
