/** One desktop background plane. Terminal surfaces cut out their own
 * rectangle and paint their chosen palette once, so 85% never becomes
 * 97.75% by stacking two backgrounds. Output/scroll does not remeasure it. */
import { useEffect, useState } from "react";
import { inApp, isMenuFace, settingsFace, windowAppearanceSupport, setWindowAppearance, type ConfigRow } from "@/api";

declare global { interface Window { __KHOR_WINDOW_SURFACE__?: boolean; __KHOR_WINDOW_PLATFORM__?: string; __KHOR_WINDOW_PREF__?: WindowChoice } }
export type WindowMaterial = "solid" | "clear" | "frost";
export type WindowChoice = { material: WindowMaterial; opacity: number; outline?: boolean };
export const WINDOW_MIRROR = "khor.window-surface";
/** Zero is a valid saved preference; absent/non-finite values stay solid. */
export function normalizeWindowOpacity(value: unknown): number {
  const n = typeof value === "number" || typeof value === "string" && value.trim() !== "" ? Number(value) : NaN;
  return Number.isFinite(n) ? Math.max(0, Math.min(100, n)) : 100;
}
let choice: WindowChoice = { material: "solid", opacity: 100 };
let nativeMaterial: WindowMaterial | null = null;
const main = () => inApp && window.__KHOR_WINDOW_SURFACE__ === true && !settingsFace && !isMenuFace;
export function windowChoice(rows: ConfigRow[] | null): WindowChoice | null {
  const m = rows?.find(r => r.name === "window-material")?.value;
  const o = rows?.find(r => r.name === "window-opacity")?.value;
  return m === "solid" || m === "clear" || m === "frost" ? { material: m, opacity: normalizeWindowOpacity(o), outline: rows?.find(r => r.name === "window-outline")?.value !== "off" } : null;
}
export function windowOpacity(): number { return main() && choice.material !== "solid" ? choice.opacity / 100 : 1; }
export function wearWindowChoice(next: WindowChoice, native = true): void {
  choice = next;
  if (!main()) return;
  const root = document.documentElement;
  root.toggleAttribute("data-window-transparent", next.material !== "solid");
  root.toggleAttribute("data-window-outline", next.outline !== false);
  if(window.__KHOR_WINDOW_PLATFORM__)root.dataset.windowPlatform=window.__KHOR_WINDOW_PLATFORM__;
  root.style.setProperty("--window-opacity", `${windowOpacity() * 100}%`);
  try { window.localStorage.setItem(WINDOW_MIRROR, JSON.stringify(next)); } catch { /* Only the next launch's mirror is lost. */ }
  if (native && nativeMaterial !== next.material) {
    nativeMaterial = next.material;
    void setWindowAppearance(next.material).catch(() => { nativeMaterial = null; });
  }
  queueGround();
  window.dispatchEvent(new Event("khor:window-surface"));
}
export function onWindowSurface(run: () => void): () => void {
  window.addEventListener("khor:window-surface", run);
  return () => window.removeEventListener("khor:window-surface", run);
}
export function useWindowSupport(): { supported: boolean; blur: boolean } {
  const [support, setSupport] = useState({ supported: false, blur: false });
  useEffect(() => {
    let alive = true;
    if (inApp) void windowAppearanceSupport().then(s => { if (alive) setSupport(s); }, () => {});
    return () => { alive = false; };
  }, []);
  return support;
}
let frame = 0;
let armed = false;
const surfaceSelector = "[data-terminal], [data-term-ended-screen]";
function queueGround() {
  if (!main() || frame) return;
  frame = requestAnimationFrame(() => { frame = 0; measureGround(); });
}
let resize: ResizeObserver | null = null;
const watched = new Set<Element>();
function measureGround() {
  const root = document.getElementById("root");
  if (!root) return;
  const surfaces = [...root.querySelectorAll<HTMLElement>(surfaceSelector)];
  for (const el of watched) if (!surfaces.includes(el as HTMLElement)) { resize?.unobserve(el); watched.delete(el); }
  if (windowOpacity() === 1) { root.style.removeProperty("--window-ground-clip"); return; }
  const rect = root.getBoundingClientRect();
  let path = `M0 0H${rect.width}V${rect.height}H0Z`;
  for (const el of surfaces) {
    if (!watched.has(el)) { watched.add(el); resize?.observe(el); }
    // DetailPane parks other tabs at full size using visibility:hidden.
    // An even-odd path counts every rectangle: including those invisible
    // twins would fill the active terminal's hole again and double its tint.
    if (getComputedStyle(el).visibility !== "visible") continue;
    const r = el.getBoundingClientRect();
    if (!r.width || !r.height || r.right <= rect.left || r.left >= rect.right || r.bottom <= rect.top || r.top >= rect.bottom) continue;
    const x = Math.max(0, r.left - rect.left), y = Math.max(0, r.top - rect.top);
    const right = Math.min(rect.width, r.right - rect.left), bottom = Math.min(rect.height, r.bottom - rect.top);
    path += `M${x} ${y}H${right}V${bottom}H${x}Z`;
  }
  root.style.setProperty("--window-ground-clip", `path(evenodd, "${path}")`);
}
export function followWindowSurface(): void {
  if (!main() || armed) return;
  armed = true;
  try {
    const v = window.__KHOR_WINDOW_PREF__ ?? JSON.parse(window.localStorage.getItem(WINDOW_MIRROR) ?? "null") as WindowChoice | null;
    if (v && ["solid", "clear", "frost"].includes(v.material)) wearWindowChoice({ material: v.material, opacity: normalizeWindowOpacity(v.opacity), outline: v.outline !== false }, false);
  } catch { /* A missing mirror remains solid. */ }
  resize = new ResizeObserver(queueGround);
  const root = document.getElementById("root");
  if (!root) return;
  resize.observe(root);
  // Ignore xterm's cell output. A retained tab can become visible without
  // inserting/removing a node or changing size, so watch its ancestors too.
  // Root styles include our own clip path; observing that write would loop.
  new MutationObserver(records => {
    if (records.some(r => {
      const target = r.target;
      if (!(target instanceof Element)) return false;
      if (r.type === "attributes") return target !== root && (target.matches(surfaceSelector)
        || !target.closest(surfaceSelector) && !!target.querySelector(surfaceSelector));
      return !target.closest(surfaceSelector) && [...r.addedNodes, ...r.removedNodes].some(n => n instanceof Element && (n.matches(surfaceSelector) || n.querySelector(surfaceSelector)));
    })) queueGround();
  }).observe(root, { childList: true, attributes: true, attributeFilter: ["class", "style", "hidden", "aria-hidden"], subtree: true });
  queueGround();
}
