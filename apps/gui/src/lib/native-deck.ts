// **The browser's deck, drawn natively over the page** (UX-13's last
// paragraph, the owner 2026-10-05: 「浏览器那个做原生的对吧」 — yes).
//
// On an iPhone the page is a WKWebView of its own laid over the face
// (`phone_page.rs`), and a native view always paints above the face's
// DOM, so a DOM deck can only stand *beside* the page: the page ends
// above it and nothing scrolls under the glass. The deck is therefore
// drawn again natively, over the page, and the page's scroll view gets a
// bottom inset of the deck's height so its last line rests clear of it.
//
// **One deck, measured once.** The DOM deck stays laid out where it always
// was (invisible, covered by the page), and this reads it: every glass
// shape, every glyph, the title and every place a finger can press, as
// rectangles relative to the deck, with the colours and the type the
// tokens gave them. The native side draws exactly those — so the two
// cannot drift: a token changed moves the DOM deck, and the next reading
// moves the native one (`check-face` bar r holds what is sent to the
// tokens). A press on a native seat comes back as `page:deck` with the
// seat's index, and the face clicks the same DOM control: back, the tabs
// sheet, reload, ← →, the title (the address editor, UX-18), the strip.
// The pill's sideways swipe comes back as `swipe:-1` / `swipe:1`.
//
// While the pill is the address editor the native deck is put away and
// the DOM deck, with the editor and the keyboard, is the deck (UX-18).

import { inApp } from "@/api";

/** A glass shape: a capsule or a circle, or the strip's band. */
export type DeckGlass = { x: number; y: number; w: number; h: number; r: number; sel: boolean };
/** A glyph or a face, as a PNG at the screen's scale. */
export type DeckImage = { x: number; y: number; w: number; h: number; png: string };
/** A line of text: the title, the strip's name, the tabs count. */
export type DeckLabel = { x: number; y: number; w: number; h: number; text: string; size: number; weight: number; color: string; center: boolean };
/** Where a finger lands, and which DOM control it presses. */
export type DeckHit = { x: number; y: number; w: number; h: number; seat: number; swipe: boolean };

export type DeckDrawing = {
  /** The deck's band, from its top to the screen's foot (safe area in). */
  h: number;
  /** The page's bottom inset: the deck row, or the strip, and the safe area. */
  inset: number;
  dark: boolean;
  /** `--glass`, `--glass-sel` and the rim, as rgba strings. */
  tint: string;
  selTint: string;
  rim: string;
  glass: DeckGlass[];
  images: DeckImage[];
  labels: DeckLabel[];
  hits: DeckHit[];
};

/** The iPhone app, where pages are native views over the face. */
export const nativeDeckHere: boolean = inApp && /\b(?:iPhone|iPad|iPod)\b/.test(navigator.userAgent);

const SEATS = [
  "[data-session-deck] [data-back]",
  "[data-session-deck] [data-page-step=\"back\"]",
  "[data-session-deck] [data-page-step=\"forward\"]",
  "[data-session-deck] [data-deck-title]",
  "[data-session-deck] [data-page-step=\"reload\"]",
  "[data-session-deck] [data-deck-tabs]",
  "[data-session-bar] [data-deck-strip]",
] as const;

/** The DOM control a native seat stands for. */
export function seatControl(screen: HTMLElement, seat: number): HTMLElement | null {
  const sel = SEATS[seat];
  return sel ? screen.querySelector<HTMLElement>(sel) : null;
}

/** An inline SVG as a PNG at the screen's scale, its `currentColor` the
    element's own colour. */
async function svgPng(svg: SVGSVGElement, w: number, h: number, color: string): Promise<string | null> {
  const copy = svg.cloneNode(true) as SVGSVGElement;
  copy.setAttribute("xmlns", "http://www.w3.org/2000/svg");
  copy.setAttribute("width", String(w));
  copy.setAttribute("height", String(h));
  copy.style.color = color;
  const text = new XMLSerializer().serializeToString(copy);
  const img = new Image();
  img.src = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(text)}`;
  try {
    await img.decode();
  } catch {
    return null;
  }
  const k = window.devicePixelRatio || 1;
  const c = document.createElement("canvas");
  c.width = Math.round(w * k);
  c.height = Math.round(h * k);
  const g = c.getContext("2d");
  if (!g) return null;
  g.drawImage(img, 0, 0, c.width, c.height);
  return c.toDataURL("image/png").slice("data:image/png;base64,".length);
}

/** A face (an avatar's SVG in its cropped box) as a PNG, cropped round
    the way the DOM crops it. */
async function facePng(box: HTMLElement, w: number, h: number): Promise<string | null> {
  const svg = box.querySelector("svg");
  if (!svg) return null;
  const inner = await svgPng(svg, w, h, getComputedStyle(box).color);
  if (inner === null) return null;
  const img = new Image();
  img.src = `data:image/png;base64,${inner}`;
  await img.decode().catch(() => {});
  const k = window.devicePixelRatio || 1;
  const c = document.createElement("canvas");
  c.width = Math.round(w * k);
  c.height = Math.round(h * k);
  const g = c.getContext("2d");
  if (!g) return null;
  const r = parseFloat(getComputedStyle(box).borderTopLeftRadius) || w / 2;
  g.beginPath();
  g.roundRect(0, 0, c.width, c.height, r * k);
  g.clip();
  g.drawImage(img, 0, 0, c.width, c.height);
  return c.toDataURL("image/png").slice("data:image/png;base64,".length);
}

/** Two pages, the front one holding the count (`SessionBar`'s
    `TabsGlyph`), drawn from its own laid-out squares. */
function tabsSvg(glyph: HTMLElement, color: string, fill: string): SVGSVGElement | null {
  const back = glyph.querySelector<HTMLElement>("[data-tabs-back]");
  const front = glyph.querySelector<HTMLElement>("[data-tabs-front]");
  if (!back || !front) return null;
  const o = glyph.getBoundingClientRect();
  const ns = "http://www.w3.org/2000/svg";
  const svg = document.createElementNS(ns, "svg");
  svg.setAttribute("viewBox", `0 0 ${o.width} ${o.height}`);
  for (const el of [back, front]) {
    const b = el.getBoundingClientRect();
    const cs = getComputedStyle(el);
    const sw = parseFloat(cs.borderTopWidth) || 1;
    const rect = document.createElementNS(ns, "rect");
    rect.setAttribute("x", String(b.left - o.left + sw / 2));
    rect.setAttribute("y", String(b.top - o.top + sw / 2));
    rect.setAttribute("width", String(b.width - sw));
    rect.setAttribute("height", String(b.height - sw));
    rect.setAttribute("rx", String(parseFloat(cs.borderTopLeftRadius) || 0));
    rect.setAttribute("fill", fill);
    rect.setAttribute("stroke", color);
    rect.setAttribute("stroke-width", String(sw));
    svg.appendChild(rect);
  }
  return svg;
}

/** A CSS colour as `#rrggbb` or `rgba(r, g, b, a)`, whatever it was
    written in (an `oklch()` token, a `color-mix()`): a canvas says it
    back in one of those two. */
let pen: CanvasRenderingContext2D | null = null;
function plain(color: string): string {
  pen ??= document.createElement("canvas").getContext("2d");
  if (!pen) return color;
  pen.fillStyle = "#000000";
  pen.fillStyle = color;
  return String(pen.fillStyle);
}

function rel(r: DOMRect, o: DOMRect) {
  return { x: r.left - o.left, y: r.top - o.top, w: r.width, h: r.height };
}

/**
 * Reads the DOM deck under `screen` (`[data-session-screen]`) into what
 * the native side draws. `null` when there is no deck to read.
 */
export async function readDeck(screen: HTMLElement): Promise<DeckDrawing | null> {
  const bar = screen.querySelector<HTMLElement>("[data-session-bar]");
  if (!bar) return null;
  const o = bar.getBoundingClientRect();
  const root = getComputedStyle(document.documentElement);
  const ink = getComputedStyle(bar).color;
  const glass: DeckGlass[] = [];
  const images: DeckImage[] = [];
  const labels: DeckLabel[] = [];
  const hits: DeckHit[] = [];
  const shown = (el: Element) => el.getClientRects().length > 0;
  // **The folded strip stands on a band of glass** to the screen's foot:
  // in the DOM it sits on the app's ground under the page; over the page
  // its words need a ground of their own.
  const folded = bar.querySelector<HTMLElement>("[data-deck-strip]");
  if (folded && shown(folded)) {
    const b = folded.getBoundingClientRect();
    glass.push({ x: 0, y: b.top - o.top, w: o.width, h: o.bottom - b.top, r: 0, sel: false });
  }
  for (const el of bar.querySelectorAll<HTMLElement>(".glass")) {
    if (!shown(el)) continue;
    const b = el.getBoundingClientRect();
    glass.push({ ...rel(b, o), r: Math.min(b.width, b.height) / 2, sel: el.classList.contains("glass-sel") });
  }
  // Glyphs: every svg in the deck that is not a face, in its own ink.
  for (const svg of bar.querySelectorAll<SVGSVGElement>("svg")) {
    if (!shown(svg) || svg.closest("[data-face], [data-tabs-glyph]")) continue;
    const b = svg.getBoundingClientRect();
    const color = getComputedStyle(svg).color;
    const png = await svgPng(svg, b.width, b.height, color);
    if (png) images.push({ ...rel(b, o), png });
  }
  for (const face of bar.querySelectorAll<HTMLElement>("[data-face]")) {
    if (!shown(face)) continue;
    const b = face.getBoundingClientRect();
    const png = await facePng(face, b.width, b.height);
    if (png) images.push({ ...rel(b, o), png });
  }
  const tabs = bar.querySelector<HTMLElement>("[data-tabs-glyph]");
  if (tabs && shown(tabs)) {
    const b = tabs.getBoundingClientRect();
    const fill = getComputedStyle(tabs.querySelector("[data-tabs-front]") ?? tabs).backgroundColor;
    const svg = tabsSvg(tabs, ink, fill);
    const png = svg ? await svgPng(svg, b.width, b.height, ink) : null;
    if (png) images.push({ ...rel(b, o), png });
    const front = tabs.querySelector<HTMLElement>("[data-tabs-front]");
    if (front) {
      const cs = getComputedStyle(front);
      labels.push({
        ...rel(front.getBoundingClientRect(), o),
        text: front.textContent ?? "",
        size: parseFloat(cs.fontSize),
        weight: Number(cs.fontWeight) || 600,
        color: plain(cs.color),
        center: true,
      });
    }
  }
  for (const el of bar.querySelectorAll<HTMLElement>("[data-deck-title], [data-deck-strip] > span:first-child, [data-region-caption]")) {
    if (!shown(el)) continue;
    const cs = getComputedStyle(el);
    labels.push({
      ...rel(el.getBoundingClientRect(), o),
      text: el.textContent ?? "",
      size: parseFloat(cs.fontSize),
      weight: Number(cs.fontWeight) || 400,
      color: plain(cs.color),
      center: el.closest("[data-deck-strip]") !== null,
    });
  }
  SEATS.forEach((sel, seat) => {
    const el = screen.querySelector<HTMLElement>(sel);
    if (!el || !shown(el) || el.getAttribute("aria-disabled") === "true") return;
    hits.push({ ...rel(el.getBoundingClientRect(), o), seat, swipe: false });
  });
  // The pill swipes sideways between tabs, as the DOM pill does.
  const pill = bar.querySelector<HTMLElement>("[data-deck-pill]");
  if (pill && shown(pill)) hits.push({ ...rel(pill.getBoundingClientRect(), o), seat: -1, swipe: true });
  const strip = bar.querySelector<HTMLElement>("[data-deck-strip]");
  const row = strip && shown(strip) ? strip : bar.querySelector<HTMLElement>("[data-session-deck]");
  const safe = parseFloat(getComputedStyle(bar).paddingBottom) || 0;
  return {
    h: o.height,
    inset: (row?.getBoundingClientRect().height ?? 0) + safe,
    dark: document.documentElement.getAttribute("data-theme") === "dark" ||
      (document.documentElement.getAttribute("data-theme") !== "light" && matchMedia("(prefers-color-scheme: dark)").matches),
    tint: plain(root.getPropertyValue("--glass").trim()),
    selTint: plain(root.getPropertyValue("--glass-sel").trim()),
    rim: "rgba(255, 255, 255, 0.5)",
    glass,
    images,
    labels,
    hits,
  };
}
