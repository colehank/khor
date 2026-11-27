import { createContext, useContext, useEffect, useRef } from "react";
import { GlassCapsule, GlassItem } from "@/components/Glass";
import { IconMinus, IconPlus } from "@/components/icons";
import { gui } from "@/gen/catalog";
import { useCoarse } from "@/hooks/use-coarse";
import { groundUnder, inkOver } from "@/lib/deck-fold";

// Use the same adaptive ink for the glass hover fill and the glyph.
const previewButton = "[--fill:color-mix(in_srgb,currentColor_20%,transparent)] group-data-[ink=dark]/preview-zoom:text-(--ink-on-light) group-data-[ink=light]/preview-zoom:text-(--ink-on-dark)";

/** The open file can outlive the mounted renderer when another tab is selected. */
export const PreviewZoomIntroScope = createContext<object | null>(null);
const introduced = new WeakSet<object>();

function previewGround(lens: DOMRect, panel: HTMLElement, pixel: CanvasRenderingContext2D | null): string {
  const x = lens.left + lens.width / 2, y = lens.top + lens.height / 2;
  const raster = document.elementsFromPoint(x, y).find((element): element is HTMLCanvasElement | HTMLImageElement =>
    !!panel.parentElement?.contains(element) && (element instanceof HTMLCanvasElement || element instanceof HTMLImageElement));
  if (raster && pixel) {
    const box = raster.getBoundingClientRect();
    const width = raster instanceof HTMLImageElement ? raster.naturalWidth : raster.width;
    const height = raster instanceof HTMLImageElement ? raster.naturalHeight : raster.height;
    const left = Math.max(lens.left, box.left), right = Math.min(lens.right, box.right);
    const top = Math.max(lens.top, box.top), bottom = Math.min(lens.bottom, box.bottom);
    if (width && height && right > left && bottom > top) {
      const sx = width / box.width, sy = height / box.height;
      try {
        pixel.clearRect(0, 0, 1, 1);
        pixel.drawImage(raster, (left - box.left) * sx, (top - box.top) * sy, (right - left) * sx, (bottom - top) * sy, 0, 0, 1, 1);
        const [red, green, blue, alpha] = pixel.getImageData(0, 0, 1, 1).data;
        if (alpha > 128) return `${red},${green},${blue}`;
      } catch {
        pixel.canvas.width = 1;
      }
    }
  }
  return groundUnder(x, y, panel);
}

export function PreviewZoom({ onZoom }: { onZoom: (step: -1 | 1) => void }) {
  return useCoarse() ? null : <HoverZoom onZoom={onZoom}/>;
}

/** The preview owns the hover area so invisible controls leave the page interactive. */
function HoverZoom({ onZoom }: { onZoom: (step: -1 | 1) => void }) {
  const area = useRef<HTMLDivElement>(null);
  const ownScope = useRef({});
  const introScope = useContext(PreviewZoomIntroScope) ?? ownScope.current;
  useEffect(() => {
    const panel = area.current;
    const preview = panel?.parentElement;
    const lens = panel?.firstElementChild;
    if (!panel || !preview || !lens) return;

    const sampler = document.createElement("canvas");
    sampler.width = sampler.height = 1;
    const pixel = sampler.getContext("2d", { willReadFrequently: true });
    const sampleInk = () => {
      panel.dataset.ink = inkOver(previewGround(lens.getBoundingClientRect(), panel, pixel));
    };
    let hovered = false;
    let introTimer = 0;
    const stopIntro = () => {
      window.clearTimeout(introTimer);
      introTimer = 0;
    };
    const updateVisibility = () => {
      const visible = hovered || introTimer !== 0;
      if (visible && !panel.hasAttribute("data-shown")) sampleInk();
      panel.toggleAttribute("data-shown", visible);
    };
    const mouse = (event: PointerEvent) => {
      if (event.pointerType === "touch") return;
      const {left, right, top, bottom} = panel.getBoundingClientRect();
      hovered = event.clientX >= left && event.clientX <= right && event.clientY >= top && event.clientY <= bottom;
      if (hovered) stopIntro();
      updateVisibility();
    };
    const leave = (event: PointerEvent) => {
      if (event.pointerType === "touch") return;
      hovered = false;
      updateVisibility();
    };

    let inView = false;
    let presented = false;
    const syncPresentation = () => {
      const visible = inView && !document.hidden;
      if (visible === presented) return;
      presented = visible;
      if (!visible) {
        stopIntro();
        hovered = false;
      } else if (!introduced.has(introScope)) {
        introduced.add(introScope);
        if (!hovered) {
          introTimer = window.setTimeout(() => {
            introTimer = 0;
            hovered = lens.matches(":hover");
            updateVisibility();
          }, 2000);
        }
      }
      updateVisibility();
    };
    const introObserver = window.matchMedia("(any-hover: hover)").matches
      ? new IntersectionObserver(([entry]) => {
          inView = entry.isIntersecting;
          syncPresentation();
        })
      : null;
    if (introObserver) {
      introObserver.observe(panel);
      document.addEventListener("visibilitychange", syncPresentation);
    }
    let frame = 0;
    const scroll = () => {
      if (frame || !panel.hasAttribute("data-shown")) return;
      frame = requestAnimationFrame(() => { frame = 0; sampleInk(); });
    };

    preview.addEventListener("pointermove", mouse, true);
    preview.addEventListener("pointerleave", leave);
    preview.addEventListener("scroll", scroll, true);
    panel.addEventListener("focusin", sampleInk);
    panel.addEventListener("click", scroll);
    return () => {
      preview.removeEventListener("pointermove", mouse, true);
      preview.removeEventListener("pointerleave", leave);
      preview.removeEventListener("scroll", scroll, true);
      panel.removeEventListener("focusin", sampleInk);
      panel.removeEventListener("click", scroll);
      cancelAnimationFrame(frame);
      introObserver?.disconnect();
      if (introObserver) document.removeEventListener("visibilitychange", syncPresentation);
      stopIntro();
    };
  }, [introScope]);

  return <div ref={area} data-preview-zoom
    className="group/preview-zoom pointer-events-none absolute inset-x-0 bottom-row z-20 mx-auto flex w-fit min-w-40 justify-center px-page pt-row pb-row">
    <GlassCapsule role="group" aria-label={gui.office_tool_zoom}
      className="[--glass-btn:calc(var(--touch)+2*var(--glass-pad))] [--glass-item-h:var(--touch)] [--glass-item-w:var(--touch)] [--ctl-glyph:var(--icon-rail)] pointer-events-none opacity-0 transition-opacity duration-(--dur-180) group-data-[shown]/preview-zoom:pointer-events-auto group-data-[shown]/preview-zoom:opacity-100 group-data-[shown]/preview-zoom:duration-0 has-[:focus-visible]:opacity-100 has-[:focus-visible]:duration-0 motion-reduce:transition-none">
      <GlassItem className={previewButton} data-preview-zoom-out aria-label={gui.file_size_down} tip={false} onClick={() => onZoom(-1)}><IconMinus/></GlassItem>
      <GlassItem className={previewButton} data-preview-zoom-in aria-label={gui.file_size_up} tip={false} onClick={() => onZoom(1)}><IconPlus/></GlassItem>
    </GlassCapsule>
  </div>;
}
