// **The chosen piece travels** (the audit fix plan F7.2; SYS-11's
// proposal). In a capsule of choices — the landings strip, the narrow
// bar, the tab capsule, a machine's verbs — the chosen piece slides from
// the old item to the new one instead of vanishing there and appearing
// here, so the eye is carried to where the choice went.
//
// **A stand-in does the travelling, never the items.** The page changes
// at once (nothing here delays what the press announced); the new item's
// own piece is hidden for the trip (`[data-sel-arriving]`, `app.css`) and
// a copy of the piece (`[data-sel-ghost]`, the same `glass-sel`) runs
// from the old item's box to the new one's under the items, then goes.
// A second press mid-trip starts from wherever the stand-in is. Reduced
// motion: no trip, the piece is simply where it is.
import { useLayoutEffect, useRef } from "react";

/** The chosen piece inside a host: the first shown `glass-sel`, or an
    item marked on, that is not inside another one. */
function pieceIn(host: HTMLElement): HTMLElement | null {
  const all = [...host.querySelectorAll<HTMLElement>('.glass-sel, [data-on="true"], [data-on=""]')];
  for (const e of all) {
    if (e.closest("[data-sel-ghost]")) continue;
    if (all.some((o) => o !== e && o.contains(e))) continue;
    // Not by its paint: a seat's ground may still be fading in when this
    // runs (the hover rule's transition), so the class and the mark are
    // what say which item is the chosen one.
    if (e.getClientRects().length === 0) continue;
    return e;
  }
  return null;
}

type Box = { x: number; y: number; w: number; h: number; r: string };

function boxIn(host: HTMLElement, e: Element, r?: string): Box {
  const h = host.getBoundingClientRect();
  const b = e.getBoundingClientRect();
  return {
    x: b.left - h.left - host.clientLeft + host.scrollLeft,
    y: b.top - h.top - host.clientTop + host.scrollTop,
    w: b.width,
    h: b.height,
    r: r ?? getComputedStyle(e).borderRadius,
  };
}

/** Where each named capsule's piece was last seen. **Kept outside the
    component**: a landing's strip is drawn inside that landing's head, so
    choosing another landing mounts a new strip — its first layout is the
    moment the trip starts, and a ref would have been born empty. */
const seen = new Map<string, Box>();

export function useTravel(name: string, host: () => HTMLElement | null, key: string | null) {
  const last = { get current() { return seen.get(name) ?? null; }, set current(b: Box | null) { if (b) seen.set(name, b); else seen.delete(name); } };
  const trip = useRef<{ ghost: HTMLElement; anim: Animation; to: HTMLElement } | null>(null);

  useLayoutEffect(() => {
    const h = host();
    if (!h) return;
    const to = pieceIn(h);
    const end = () => {
      const t = trip.current;
      if (!t) return;
      t.to.removeAttribute("data-sel-arriving");
      t.ghost.remove();
      trip.current = null;
    };
    // Mid-trip: start from where the stand-in is now.
    let from = last.current;
    if (trip.current) {
      from = boxIn(h, trip.current.ghost, getComputedStyle(trip.current.ghost).borderRadius);
      trip.current.anim.cancel();
      end();
    }
    const still = typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;
    if (!to) {
      last.current = null;
      return;
    }
    const at = boxIn(h, to);
    last.current = at;
    if (!from || still || (Math.abs(from.x - at.x) < 1 && Math.abs(from.y - at.y) < 1 && Math.abs(from.w - at.w) < 1)) return;

    if (getComputedStyle(h).position === "static") h.style.position = "relative";
    h.style.isolation = "isolate";
    const ghost = document.createElement("span");
    ghost.dataset.selGhost = "";
    ghost.setAttribute("aria-hidden", "true");
    ghost.className = "glass-sel";
    Object.assign(ghost.style, {
      position: "absolute",
      left: "0",
      top: "0",
      width: `${at.w}px`,
      height: `${at.h}px`,
      borderRadius: at.r,
      zIndex: "-1",
      pointerEvents: "none",
    });
    h.prepend(ghost);
    to.setAttribute("data-sel-arriving", "");
    const ease = getComputedStyle(document.documentElement).getPropertyValue("--ease-move").trim() || "ease";
    const anim = ghost.animate(
      [
        { transform: `translate(${from.x}px, ${from.y}px)`, width: `${from.w}px`, height: `${from.h}px`, borderRadius: from.r },
        { transform: `translate(${at.x}px, ${at.y}px)`, width: `${at.w}px`, height: `${at.h}px`, borderRadius: at.r },
      ],
      { duration: 240, easing: ease, fill: "forwards" },
    );
    trip.current = { ghost, anim, to };
    anim.onfinish = end;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);
}
