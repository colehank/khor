/** Preview is local to the pill. Only a completed release changes the open tab. */
export type DeckScrubState = {
  opened: number; preview: number; position: number;
  mode: 'idle' | 'pending' | 'swipe' | 'rail' | 'settling';
  source: 'swipe' | 'rail'; direction: number; holding: boolean;
  travelMs: number; dwellMs: number;
};
export const LONG_PRESS_MS = 320, HOLD_MS = 400, RAIL_HOLD_MS = 180, SETTLE_MS = 140, RAIL_STEP = 92;
/** Design trial, not a reaction-time standard. Ordinary swipe distances scale
 * with the pill; the long-press picker scales with its visible item width. */
export function holdTiming(displacement: number, width: number) {
  const distance = Math.abs(displacement) / Math.max(1, width);
  const t = Math.max(0, Math.min(1, (distance - .24) / (.68 - .24)));
  const gain = t * t * (3 - 2 * t);
  return { travelMs: 220 - 60 * gain, dwellMs: 900 - 420 * gain };
}
export const wrapIndex = (n: number, count: number) => ((n % count) + count) % count;
/** Neighbour copies keep the strip continuous across either end of the list. */
export function nearbySlots(position: number, count: number, radius: number) {
  if (count <= 1) return [{ slot: 0, index: 0 }];
  const center = Math.floor(position);
  return Array.from({ length: radius * 2 + 1 }, (_, i) => {
    const slot = center + i - radius;
    return { slot, index: wrapIndex(slot, count) };
  });
}
type Clock = { now: () => number; request: (f: () => void) => number; cancel: (id: number) => void };
export function deckScrubber(count: number, initial: number, changed: (s: DeckScrubState) => void,
  commit: (index: number) => void, clock: Clock = {
    now: () => performance.now(), request: f => requestAnimationFrame(f), cancel: id => cancelAnimationFrame(id),
  }) {
  const wrap = (n: number) => wrapIndex(n, Math.max(1, count));
  let s: DeckScrubState = { opened: initial, preview: initial, position: initial, mode: 'idle', source: 'swipe', direction: 0, holding: false, ...holdTiming(0, 250) };
  let frame = 0, down = false, origin = { x: 0, y: 0 }, width = 250, railWidth = RAIL_STEP, base = initial;
  let began = 0, holdAt = Infinity, lastX = 0, lastMove = 0, speed = 0, holdX = 0;
  let rateControl = false, dwellAt: number | null = null;
  let travel: { from: number; target: number; at: number; duration: number } | null = null;
  let landing: { from: number; target: number; at: number } | null = null;
  const nearest = () => s.mode === 'swipe' && !rateControl && Math.abs(s.position - base) < 1
    ? base + (Math.abs(s.position - base) >= .4 ? Math.sign(s.position - base) : 0) : Math.round(s.position);
  const publish = () => { s.preview = wrap(nearest()); changed({ ...s }); };
  const stop = () => { clock.cancel(frame); frame = 0; };
  const cancel = () => {
    stop(); down = false; landing = travel = null; holdAt = Infinity; rateControl = false; dwellAt = null;
    s = { ...s, position: s.opened, preview: s.opened, mode: 'idle', holding: false, direction: 0 };
    publish();
  };
  const finish = (slot: number) => {
    const target = wrap(slot), before = s.opened;
    landing = travel = null; s.opened = target; s.position = target;
    s.mode = 'idle'; s.holding = false; s.direction = 0;
    publish(); if (target !== before) commit(target);
  };
  const tick = () => {
    frame = 0;
    const now = clock.now();
    if (landing) {
      const p = Math.min(1, (now - landing.at) / SETTLE_MS);
      s.position = landing.from + (landing.target - landing.from) * (1 - (1 - p) ** 3);
      if (p === 1) { finish(landing.target); return; }
      publish(); frame = clock.request(tick); return;
    }
    if (!down) return;
    if (s.mode === 'pending' && now - began >= LONG_PRESS_MS) {
      s.mode = 'rail'; s.source = 'rail'; publish();
    }
    if (travel) {
      const p = Math.min(1, (now - travel.at) / travel.duration);
      s.position = travel.from + (travel.target - travel.from) * (1 - (1 - p) ** 3);
      if (p === 1) { travel = null; dwellAt = now; holdAt = now + s.dwellMs; }
      publish();
    } else if (count > 1 && (s.mode === 'swipe' || s.mode === 'rail') && s.direction && now >= holdAt) {
      const nearest = Math.round(s.position);
      // First land the item already under the finger. Then move one slot per beat.
      const target = Math.abs(nearest - s.position) > .001
        ? (s.direction > 0 ? Math.ceil(s.position) : Math.floor(s.position)) : nearest + s.direction;
      travel = { from: s.position, target, at: now, duration: s.travelMs };
      rateControl = true; dwellAt = null; s.holding = true; publish();
    }
    frame = clock.request(tick);
  };
  return {
    read: () => ({ ...s }),
    arrive() { if (landing) { const target=landing.target; stop(); down=false; finish(target); } },
    start(x: number, y: number, bandWidth = 250, itemWidth = RAIL_STEP) {
      // Finish an already released selection before accepting another touch.
      if (landing) { const target = landing.target; cancel(); finish(target); } else cancel();
      down = true; origin = { x, y }; width = Math.max(1, bandWidth); railWidth = Math.max(1, itemWidth); base = s.opened;
      began = lastMove = clock.now(); lastX = holdX = x; speed = 0; s.mode = 'pending'; s.source = 'swipe'; publish();
      frame = clock.request(tick);
    },
    move(x: number, y: number) {
      if (!down) return;
      const dx = x - origin.x, dy = y - origin.y, now = clock.now();
      if (s.mode === 'pending') {
        if (Math.hypot(dx, dy) < 8) return;
        if (Math.abs(dy) >= Math.abs(dx)) { cancel(); return; }
        s.mode = 'swipe'; s.source = 'swipe';
      }
      const rail = s.mode === 'rail', scale = rail ? railWidth : width;
      const holdDelay = rail ? RAIL_HOLD_MS : HOLD_MS;
      const threshold = scale * (s.direction ? .16 : .24);
      const direction = Math.abs(dx) >= threshold ? -Math.sign(dx) : 0;
      const timing = holdTiming(dx, scale);
      if (travel) {
        // Change rate without restarting the movement or jumping its progress.
        const progress = Math.min(1, (now - travel.at) / travel.duration);
        travel.at = now - progress * timing.travelMs; travel.duration = timing.travelMs;
      }
      s.travelMs = timing.travelMs; s.dwellMs = timing.dwellMs;
      if (rateControl) {
        if (direction !== s.direction) {
          s.position = Math.round(s.position); travel = null; dwellAt = null;
          holdAt = direction ? (s.direction ? now : now + holdDelay) : Infinity;
        } else if (dwellAt !== null) holdAt = dwellAt + s.dwellMs;
        s.holding = direction !== 0;
      } else {
        s.position = count > 1 ? base - dx / scale
          : -Math.sign(dx) * .1 * (1 - 1 / (Math.abs(dx) * .55 / (width * .1) + 1));
        // Rail is already an intentional long press. Ignore small jitter
        // around the last deliberate position instead of restarting a full
        // hold on every sample. Ordinary swipes retain their existing delay.
        const moved = rail ? Math.abs(x - holdX) >= scale * .04 : Math.abs(x - lastX) > 1;
        if (moved || direction !== s.direction) { holdAt = now + holdDelay; holdX = x; }
      }
      if (Math.abs(x - lastX) > 1) {
        speed = (x - lastX) / Math.max(1, now - lastMove); lastX = x; lastMove = now;
      }
      s.direction = direction;
      if (!direction) holdAt = Infinity;
      publish();
    },
    pause() {
      s.position = Math.round(s.position); rateControl = true; dwellAt = null;
      travel = null; holdAt = Infinity; s.direction = 0; s.holding = false; publish();
    },
    release(instant = false) {
      if (!down) return 'none';
      down = false; stop(); travel = null;
      if (s.mode === 'pending') { cancel(); return 'tap'; }
      let target = nearest();
      // Preserve the short quick flick, including when crossing the cyclic seam.
      if (count > 1 && s.mode === 'swipe' && !rateControl && clock.now() - lastMove < 90 && Math.abs(speed) > .4 && Math.abs(lastX - origin.x) >= 12)
        target = Math.sign(speed) === -Math.sign(s.position-base)
          ? (Math.abs(s.position-base)<1 ? base-Math.sign(speed) : target) : base;
      if (instant) { finish(target); return wrap(target) === wrap(base) ? 'none' : 'commit'; }
      s.source = s.mode === 'rail' ? 'rail' : 'swipe'; s.mode = 'settling'; s.holding = false;
      landing = { from: s.position, target, at: clock.now() };
      publish(); frame = clock.request(tick); return wrap(target) === s.opened ? 'none' : 'commit';
    },
    cancel,
    select(index: number) { cancel(); s.opened = wrap(index); s.position = s.opened; publish(); },
  };
}
