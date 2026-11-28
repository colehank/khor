export type PreviewPoint = { x: number; y: number };
type Pinch = PreviewPoint & { distance: number };
export type BeginPreviewPinch = (origin: PreviewPoint, signal: AbortSignal) =>
  ((center: PreviewPoint, factor: number) => void | Promise<void>) | undefined;

export function pinchPoint(touches: TouchList): Pinch {
  const [a, b] = [touches[0], touches[1]];
  return {
    x: (a.clientX + b.clientX) / 2,
    y: (a.clientY + b.clientY) / 2,
    distance: Math.max(1, Math.hypot(a.clientX - b.clientX, a.clientY - b.clientY)),
  };
}

export function surfaceAt(root: HTMLElement, selector: string, point: PreviewPoint): HTMLElement | undefined {
  let nearest: HTMLElement | undefined;
  let distance = Infinity;
  for (const surface of root.querySelectorAll<HTMLElement>(selector)) {
    const bounds = surface.getBoundingClientRect();
    if (!bounds.width || !bounds.height) continue;
    const next = Math.hypot(Math.max(bounds.left - point.x, 0, point.x - bounds.right),
      Math.max(bounds.top - point.y, 0, point.y - bounds.bottom));
    if (next >= distance) continue;
    nearest = surface;
    distance = next;
    if (!next) break;
  }
  return nearest;
}

/** Keep a point on the document in place through a page relayout. */
export function anchorSurface(viewport: HTMLElement, surface: () => HTMLElement | undefined, origin: PreviewPoint) {
  const before = surface()?.getBoundingClientRect();
  if (!before?.width || !before.height) return;
  const x = (origin.x - before.left) / before.width;
  const y = (origin.y - before.top) / before.height;
  return (center: PreviewPoint) => {
    const after = surface()?.getBoundingClientRect();
    if (!after?.width || !after.height) return;
    viewport.scrollBy({ left: after.left + x * after.width - center.x,
      top: after.top + y * after.height - center.y, behavior: "instant" });
  };
}

/** Leave one-finger scrolling to the viewer. A worker-backed renderer can
 * finish its current scale change while only the latest new gesture step waits. */
export function bindPreviewPinch(element: HTMLElement, begin: BeginPreviewPinch, onError?: () => void) {
  const events = new AbortController();
  let gesture: AbortController | undefined;
  let accept = false;
  let distance = 1;
  let step: ReturnType<BeginPreviewPinch>;
  let pending: Pinch | undefined;
  let frame = 0;
  let busy = false;
  const schedule = () => {
    if (!frame && !busy && pending) frame = requestAnimationFrame(flush);
  };
  const flush = () => {
    frame = 0;
    const point = pending;
    const signal = gesture?.signal;
    pending = undefined;
    if (!point || !step || !signal || signal.aborted) return;
    busy = true;
    void (async () => {
      try { await step(point, point.distance / distance); }
      catch { if (!signal.aborted) onError?.(); }
      finally { busy = false; schedule(); }
    })();
  };
  const stop = () => { accept = false; };
  const cancel = () => { stop(); gesture?.abort(); pending = undefined; };
  const start = (event: TouchEvent) => {
    if (event.touches.length !== 2) { stop(); return; }
    if (![...event.touches].every(touch => touch.target instanceof Node && element.contains(touch.target))) return;
    gesture?.abort();
    gesture = new AbortController();
    pending = undefined;
    const start = pinchPoint(event.touches);
    distance = start.distance;
    step = begin(start, gesture.signal);
    accept = !!step;
    if (accept) event.preventDefault();
  };
  const move = (event: TouchEvent) => {
    if (!accept || event.touches.length !== 2) return;
    event.preventDefault();
    pending = pinchPoint(event.touches);
    schedule();
  };
  const options = { capture: true, passive: false, signal: events.signal };
  element.addEventListener("touchstart", start, options);
  element.addEventListener("touchmove", move, options);
  element.addEventListener("touchend", stop, options);
  element.addEventListener("touchcancel", cancel, options);
  return () => {
    events.abort();
    cancel();
    cancelAnimationFrame(frame);
    frame = 0;
  };
}
