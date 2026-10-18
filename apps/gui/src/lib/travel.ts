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
  const all = [...host.querySelectorAll<HTMLElement>('.glass-sel, [data-on="true"], [data-on=""]')].filter(e => !e.hasAttribute("data-tab-group"));
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

type Trip = { ghost: HTMLElement; anim: Animation; to: HTMLElement; host: HTMLElement; planned: Box };

/** The stand-in's box now, in its host's coordinates. */
function ghostBox(t: Trip): Box {
  return boxIn(t.host, t.ghost, getComputedStyle(t.ghost).borderRadius);
}

/**
 * Lands a trip: **the new item's own piece is drawn in full in the same
 * frame the stand-in goes** (V08) — its ground would otherwise fade in
 * over the hover rule's transition, a blink after the slide. So its
 * transitions are off for that frame.
 */
function land(t: Trip) {
  t.to.style.transition = "none";
  t.to.removeAttribute("data-sel-arriving");
  void t.to.offsetWidth;
  t.ghost.remove();
  requestAnimationFrame(() => {
    t.to.style.transition = "";
  });
}

type TravelOptions = {
  /** A composite strip can resolve its own selectable pieces. */
  current?: (host: HTMLElement) => HTMLElement | null;
  pressed?: (host: HTMLElement, target: Element) => HTMLElement | null;
  /** When supplied, animate selection changes within this same set only. */
  members?: string;
  durationToken?: string;
  ghostClass?: string;
};

function fly(h: HTMLElement, to: HTMLElement, from: Box, at: Box, options: TravelOptions = {}): Trip {
  if (getComputedStyle(h).position === "static") h.style.position = "relative";
  h.style.isolation = "isolate";
  const ghost = document.createElement("span");
  ghost.dataset.selGhost = "";
  ghost.setAttribute("aria-hidden", "true");
  ghost.className = options.ghostClass ?? "glass-sel";
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
  const duration = options.durationToken
    ? parseFloat(getComputedStyle(h).getPropertyValue(options.durationToken)) || TRIP_MS
    : TRIP_MS;
  const anim = ghost.animate(
    [
      { transform: `translate(${from.x}px, ${from.y}px)`, width: `${from.w}px`, height: `${from.h}px`, borderRadius: from.r },
      { transform: `translate(${at.x}px, ${at.y}px)`, width: `${at.w}px`, height: `${at.h}px`, borderRadius: at.r },
    ],
    { duration, easing: ease, fill: "forwards" },
  );
  return { ghost, anim, to, host: h, planned: at };
}

const still = () => typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;
/** Reduced motion is asked for: nothing travels. */
export const stillMotion = still;
/** A trip's length, the beat anything moving with the piece keeps. */
export const TRIP_MS = 240;
const same = (a: Box, b: Box) => Math.abs(a.x - b.x) < 1 && Math.abs(a.y - b.y) < 1 && Math.abs(a.w - b.w) < 1;

export function useTravel(name: string, host: () => HTMLElement | null, key: string | null, options: TravelOptions = {}) {
  const trip = useRef<Trip | null>(null);
  const latest = useRef(options); latest.current = options;
  const previous = useRef({key, members: options.members});

  // **The slide starts on the press** (V08), before the page's work: a
  // click on an item that is not the chosen one sends the stand-in on
  // its way at once; the render that follows only confirms (or
  // retargets) it. Capture, so it runs before the app's own handler.
  useLayoutEffect(() => {
    const h = host();
    if (!h) return;
    const press = (e: MouseEvent) => {
      if (still()) return;
      const target = e.target as Element | null;
      if (!target) return;
      const opts = latest.current;
      const item = opts.pressed ? opts.pressed(h, target) : target.closest<HTMLElement>('button, [data-tab]');
      if (!item || !h.contains(item) || item.closest("[data-tab-close]")) return;
      const now = opts.current ? opts.current(h) : pieceIn(h);
      if (!now || now === item || now.contains(item)) return;
      const from = trip.current ? ghostBox(trip.current) : boxIn(h, now);
      if (trip.current) {
        trip.current.anim.cancel();
        land(trip.current);
      }
      const t = fly(h, item, from, boxIn(h, item, getComputedStyle(now).borderRadius), opts);
      trip.current = t;
      t.anim.onfinish = () => {
        if (trip.current === t) {
          land(t);
          trip.current = null;
        }
      };
    };
    h.addEventListener("click", press, true);
    return () => {
      h.removeEventListener("click", press, true);
      // A host that goes (a landing's strip, mounted in the next
      // landing's head) hands its stand-in's place to the next one.
      const t = trip.current;
      if (t) {
        seen.set(name, ghostBox(t));
        t.anim.cancel();
        land(t);
        trip.current = null;
      }
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useLayoutEffect(() => {
    const h = host();
    if (!h) return;
    const opts = latest.current;
    const before = previous.current;
    previous.current = {key, members: opts.members};
    const to = opts.current ? opts.current(h) : pieceIn(h);
    if (!to) {
      seen.delete(name);
      return;
    }
    const at = boxIn(h, to);
    const running = trip.current;
    // Adding, closing and disclosing rearrange a strip, but do not move
    // the user's selection between existing choices. Note the new box
    // without sending a stand-in towards a +, a close button or empty air.
    if (opts.members !== undefined && (before.key === key || before.members !== opts.members)) {
      if (running) { running.anim.cancel(); land(running); trip.current = null; }
      seen.set(name, at);
      return;
    }
    // Already on its way to this very item (started on the press) — and
    // to the box it had at the press. **A box that grew since** (the tab
    // in front takes its whole name, UX-25) is flown to from wherever the
    // stand-in is, so the piece lands on the item as it is now.
    if (running && running.to === to) {
      const planned = running.planned;
      if (Math.abs(planned.w - at.w) < 1 && Math.abs(planned.x - at.x) < 1) {
        seen.set(name, at);
        return;
      }
    }
    let from = seen.get(name) ?? null;
    if (running) {
      from = ghostBox(running);
      running.anim.cancel();
      land(running);
      trip.current = null;
    }
    seen.set(name, at);
    if (!from || still() || same(from, at)) return;
    const t = fly(h, to, from, at, opts);
    trip.current = t;
    t.anim.onfinish = () => {
      if (trip.current === t) {
        land(t);
        trip.current = null;
      }
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key, options.members]);
}
