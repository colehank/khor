type Box = Pick<DOMRect, "left" | "top" | "right" | "bottom" | "width" | "height">;

function withHole(frame: Box, area: Box, radius: number): string {
  const n = (value: number) => value.toFixed(2);
  const x = area.left - frame.left, y = area.top - frame.top;
  const r = area.right - frame.left, b = area.bottom - frame.top;
  const c = Math.min(radius, area.width / 2, area.height / 2);
  const a = `A ${n(c)} ${n(c)} 0 0 1`;
  return `path(evenodd, "M 0 0 H ${n(frame.width)} V ${n(frame.height)} H 0 Z M ${n(x + c)} ${n(y)} H ${n(r - c)} ${a} ${n(r)} ${n(y + c)} V ${n(b - c)} ${a} ${n(r - c)} ${n(b)} H ${n(x + c)} ${a} ${n(x)} ${n(b - c)} V ${n(y + c)} ${a} ${n(x + c)} ${n(y)} Z")`;
}

function sheetPaperBox(sheet: HTMLElement): Box {
  const frame = sheet.getBoundingClientRect();
  const style = getComputedStyle(sheet);
  // The absolute ::before fills the padding box. Measuring from the
  // outside of the sheet shifts its opening by the border at the top.
  const left = frame.left + parseFloat(style.borderLeftWidth);
  const top = frame.top + parseFloat(style.borderTopWidth);
  const right = frame.right - parseFloat(style.borderRightWidth);
  const bottom = frame.bottom - parseFloat(style.borderBottomWidth);
  return { left, top, right, bottom, width: right - left, height: bottom - top };
}

/** A windowed native scanner paints beneath the WebView. Clear only the
 * rounded finder region in the app, scrim and sheet backgrounds. The
 * modal and its buttons retain their ordinary paint and hit testing. */
export function revealPairingCamera(area: HTMLElement): () => void {
  const sheet = area.closest<HTMLElement>("[data-join-sheet]");
  const app = document.getElementById("root");
  const scrim = sheet?.previousElementSibling;
  if (!sheet || !app || !(scrim instanceof HTMLElement) || !scrim.matches("[data-scrim]")) return () => {};

  const layers = [app, scrim, sheet];
  const update = () => {
    const hole = area.getBoundingClientRect();
    const radius = parseFloat(getComputedStyle(area).borderTopLeftRadius) || 0;
    for (const layer of layers) {
      const frame = layer === sheet ? sheetPaperBox(sheet) : layer.getBoundingClientRect();
      const clip = withHole(frame, hole, radius);
      if (layer.style.getPropertyValue("--pairing-camera-clip") !== clip) {
        layer.style.setProperty("--pairing-camera-clip", clip);
      }
    }
  };
  let frame = 0;
  const schedule = () => {
    if (frame) return;
    frame = requestAnimationFrame(() => {
      frame = 0;
      update();
      if (sheet.getAnimations().some(animation => animation.playState === "running")) schedule();
    });
  };
  const observed = new ResizeObserver(schedule);
  for (const element of [area, ...layers]) observed.observe(element);
  const motion = new MutationObserver(schedule);
  motion.observe(sheet, { attributes: true, attributeFilter: ["style", "data-detent", "data-sheet-pulling"] });
  const viewport = window.visualViewport;
  viewport?.addEventListener("resize", schedule);
  viewport?.addEventListener("scroll", schedule);
  update();
  schedule();
  document.documentElement.setAttribute("data-pairing-camera", "");
  sheet.setAttribute("data-camera-through", "");
  return () => {
    cancelAnimationFrame(frame);
    observed.disconnect();
    motion.disconnect();
    viewport?.removeEventListener("resize", schedule);
    viewport?.removeEventListener("scroll", schedule);
    document.documentElement.removeAttribute("data-pairing-camera");
    sheet.removeAttribute("data-camera-through");
    for (const layer of layers) layer.style.removeProperty("--pairing-camera-clip");
  };
}
