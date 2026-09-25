// ‹ › over what was picked in one landing (docs/SHELL.md 第十五轮 第 6
// 条; docs/DESIGN.md 第五节「A 详情」): System Settings' arrows, which
// walk the panes you opened, not a web page's history.
//
// **In memory, never stored.** A person's trail through the machines is a
// habit of this sitting; the next launch starts where it starts.
//
// **Nothing here decides what is picked** — the landing keeps its own
// state (a device id, an account, a page) and this only watches it:
// every change that did not come from these arrows is a step forward
// that drops whatever was ahead, as a browser does after going back and
// following a new link. Nothing picked (the overview) is a place too:
// going back from the first machine you opened lands on the map.
import { useCallback, useEffect, useState } from "react";

import type { PageHistory } from "@/components/Region";

export function useHistory<T>(current: T, go: (to: T) => void, key: (v: T) => string): PageHistory {
  const [trail, setTrail] = useState<{ items: T[]; at: number }>(() => ({ items: [current], at: 0 }));
  const here = key(current);
  useEffect(() => {
    setTrail((t) => {
      // Arrived by the arrows (or not moved at all): already where the
      // trail says we are.
      if (key(t.items[t.at]) === here) return t;
      const items = [...t.items.slice(0, t.at + 1), current];
      return { items, at: items.length - 1 };
    });
    // `current` is read through `here`; a new object with the same key
    // is the same place.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [here]);
  const step = useCallback(
    (by: number) => {
      const at = trail.at + by;
      if (at < 0 || at >= trail.items.length) return;
      setTrail({ items: trail.items, at });
      go(trail.items[at]);
    },
    [trail, go],
  );
  return {
    back: trail.at > 0 ? () => step(-1) : undefined,
    forward: trail.at < trail.items.length - 1 ? () => step(1) : undefined,
  };
}
