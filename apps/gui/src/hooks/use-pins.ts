// React's view of the pin store (`@/lib/pins`). Kept apart from the
// store itself so the ordering rule stays a plain function that can be
// read — and reasoned about — without a component around it.
import { useCallback, useSyncExternalStore } from "react";

import { pinsIn, pinsSnapshot, subscribePins, togglePin } from "@/lib/pins";

/**
 * The pins of one pane, plus the toggle for them.
 *
 * `useSyncExternalStore` rather than component state: the store outlives
 * the pane, so switching landings and coming back shows the same list
 * rather than one that quietly reset.
 */
export function usePins(scope: string): {
  pinned: ReadonlySet<string>;
  toggle: (key: string) => void;
} {
  const all = useSyncExternalStore(subscribePins, pinsSnapshot, pinsSnapshot);
  const toggle = useCallback((key: string) => togglePin(scope, key), [scope]);
  return { pinned: pinsIn(all, scope), toggle };
}
