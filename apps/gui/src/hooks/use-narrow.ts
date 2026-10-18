// The one breakpoint, and the only place 719 is written.
//
// **The shell stamps `data-narrow` on the root from this**, and
// stylesheets key off that attribute (`app.css`, the machine card's
// part rows). A `@media` repeating 719 would be a second copy that can
// drift with nothing to catch it — and the px gate refuses it out loud,
// which is how this came back: the claim was written here before
// anything implemented it, sat false for a while, and was made true the
// day a stylesheet actually needed the number (2026-09-04).
//
// **The posture is a container's, not only the window's** (UX-30). The
// session's 附栏 is the narrow face mounted in a box beside a terminal
// on the wide face, so「am I narrow」cannot be answered by the window
// alone. `useNarrow` reads the nearest `NarrowBox` first and the window
// only outside one; the box stamps the same `data-narrow` on its own
// element, so a stylesheet keyed `[data-narrow] …` and a Tailwind
// `[[data-narrow]_&]` variant see it the way they see the root. What
// is portaled to `document.body` (dialogs, menus) stays the window's —
// a sheet from inside the column is still a dialog on a wide face —
// and those read the root attribute directly.
import { createContext, createElement, useContext, useEffect, useState, type ComponentPropsWithRef } from "react";

export const NARROW_QUERY = "(max-width: 759px)";

/** `null` = no box above: follow the window. */
const NarrowContext = createContext<boolean | null>(null);

export function useWindowNarrow(): boolean {
  const [narrow, setNarrow] = useState(() => window.matchMedia(NARROW_QUERY).matches);
  useEffect(() => {
    const m = window.matchMedia(NARROW_QUERY);
    const onChange = (e: MediaQueryListEvent) => setNarrow(e.matches);
    m.addEventListener("change", onChange);
    return () => m.removeEventListener("change", onChange);
  }, []);
  return narrow;
}

export function useNarrow(): boolean {
  const boxed = useContext(NarrowContext);
  const window_ = useWindowNarrow();
  return boxed ?? window_;
}

/**
 * The narrow face in a box. Everything inside lays itself out as on
 * the phone, whatever the window's width: `useNarrow` answers `true`
 * and the element wears `data-narrow` for the stylesheets. The box is
 * a `div` and takes a `div`'s props (`ref` among them, React 19).
 */
export function NarrowBox({ children, ...rest }: ComponentPropsWithRef<"div">) {
  return createElement(
    NarrowContext.Provider,
    { value: true },
    createElement("div", { "data-narrow": "", "data-narrow-box": "", ...rest }, children),
  );
}
