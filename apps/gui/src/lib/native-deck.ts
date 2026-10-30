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
/** The native deck draws the same actual estimate as the visible DOM line. */
export type DeckProgress = {
  x: number; y: number; w: number; h: number;
  clip: { x: number; y: number; w: number; h: number; r: number };
  color: string;
  /** Actual issued navigation estimate, including its final successful 1. */
  share: number;
};

/** **The glass's rim** (UX-38 §5): `--glass-rim`, a one-pixel ring of a
    linear gradient — its CSS angle and its stops, each colour at a share
    of the line. */
export type DeckRim = { angle: number; stops: { color: string; at: number }[] };
/** The glass's drop shadow, `--glass-shadow`'s widest layer. */
export type DeckShadow = { y: number; blur: number; spread: number; color: string };

export type DeckDrawing = {
  /** Actual DOM deck bounds in the same viewport coordinates as PageBox. */
  x: number;
  y: number;
  w: number;
  h: number;
  /** The page's bottom inset: the deck row, or the strip, and the safe area. */
  inset: number;
  dark: boolean;
  /** `--glass` and `--glass-sel`, as rgba strings. */
  tint: string;
  selTint: string;
  rim: DeckRim;
  shadow: DeckShadow | null;
  glass: DeckGlass[];
  images: DeckImage[];
  labels: DeckLabel[];
  hits: DeckHit[];
  /** Absent once the load ends: the native line goes with it. */
  progress: DeckProgress | null;
};

/** The iPhone app, where pages are native views over the face. */
export const nativeDeckHere: boolean = inApp && /\b(?:iPhone|iPad|iPod)\b/.test(navigator.userAgent);

const SEATS = [
  "[data-session-deck] [data-back]",
  "[data-session-deck] [data-page-step=\"back\"]",
  "[data-session-deck] [data-page-step=\"forward\"]",
  "[data-session-deck] [data-deck-title]",
  "[data-session-deck] [data-page-step=\"reload\"]",
  // Seat 5 belonged to the removed tabs circle. Keep later wire seats stable.
  ":not(*)",
  "[data-session-bar] [data-deck-strip]",
  // A page that could not be opened (UX-22).
  "[data-session-deck] [data-page-retry]",
] as const;

/** The DOM control a native seat stands for. */
export function seatControl(screen: HTMLElement, seat: number): HTMLElement | null {
  const sel = SEATS[seat];
  return sel ? screen.querySelector<HTMLElement>(sel) : null;
}

/** The iPhone page's upward title swipe uses the DOM band's same action. */
export function openDeckOverview(screen:HTMLElement):boolean {
  const band=screen.querySelector<HTMLElement>('[data-deck-band]');
  if(!band)return false;
  band.dispatchEvent(new KeyboardEvent('keydown',{key:'ArrowUp',bubbles:true}));
  return true;
}

/** An inline SVG as a PNG at the screen's scale, its `currentColor` the
    element's own colour. */
async function svgPng(svg: SVGSVGElement, w: number, h: number, color: string, alpha = 1): Promise<string | null> {
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
  g.globalAlpha = alpha;
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

/** An `<svg>` or `<img>` decoded as a drawable, or null. */
async function drawable(el: SVGSVGElement | HTMLImageElement, w: number, h: number, color: string): Promise<HTMLImageElement | null> {
  const img = new Image();
  if (el instanceof HTMLImageElement) {
    img.crossOrigin = "anonymous";
    img.src = el.currentSrc || el.src;
  } else {
    const copy = el.cloneNode(true) as SVGSVGElement;
    copy.setAttribute("xmlns", "http://www.w3.org/2000/svg");
    copy.setAttribute("width", String(w));
    copy.setAttribute("height", String(h));
    copy.style.color = color;
    img.src = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(new XMLSerializer().serializeToString(copy))}`;
  }
  try {
    await img.decode();
  } catch {
    return null;
  }
  return img;
}

/** **The whole face seat as one picture** (UX-34): under UX-32's
    assembly a seat is the machine's face, the kind mark on it or on a
    corner punch-out, or the mark as the base with the small face on the
    corner — painted here in DOM order, each piece at its own box: a
    ground (a punch-out, the neutral disc) as its rounded fill, a face's
    svg clipped round the way the DOM crops it, a glyph in its own ink, a
    favicon as it is. ⚠️ 墓碑: the deck rasterised `[data-face]` alone and
    the glyphs on their own, so the 角标 · 机器 way put the 10-px corner
    face where the seat is. */
async function seatPng(seat: HTMLElement, w: number, h: number): Promise<string | null> {
  const k = window.devicePixelRatio || 1;
  const c = document.createElement("canvas");
  c.width = Math.round(w * k);
  c.height = Math.round(h * k);
  const g = c.getContext("2d");
  if (!g) return null;
  const o = seat.getBoundingClientRect();
  const at = (el: Element) => {
    const b = el.getBoundingClientRect();
    return { x: (b.left - o.left) * k, y: (b.top - o.top) * k, w: b.width * k, h: b.height * k };
  };
  const radius = (el: Element, b: { w: number; h: number }) => {
    const r = parseFloat(getComputedStyle(el).borderTopLeftRadius) || 0;
    return Math.min(r * k, b.w / 2, b.h / 2);
  };
  let drew = 0;
  for (const el of [seat, ...seat.querySelectorAll<Element>("*")]) {
    // A svg's own children are the svg's.
    if (el instanceof SVGElement && !(el instanceof SVGSVGElement)) continue;
    if (el.getClientRects().length === 0) continue;
    const b = at(el);
    if (b.w <= 0 || b.h <= 0) continue;
    const cs = getComputedStyle(el);
    // Drawn at its own opacity, and cut by the face it lies on (UX-38 §6:
    // the face's veil, white at 0.1, was painted solid and square over
    // the face — the native deck showed a white tile for every page).
    let alpha = 1;
    for (let at: Element | null = el; at && seat.contains(at); at = at.parentElement) alpha *= Number(getComputedStyle(at).opacity) || 0;
    const face = el.closest<HTMLElement>("[data-face]");
    const cut = (from: Element) => {
      if (!face || face === from) return;
      const cb = at(face);
      g.beginPath();
      g.roundRect(cb.x, cb.y, cb.w, cb.h, radius(face, cb) || cb.w / 2);
      g.clip();
    };
    const bg = cs.backgroundColor;
    if (bg && !/rgba\(\s*\d+,\s*\d+,\s*\d+,\s*0\s*\)/.test(bg) && bg !== "transparent") {
      g.save();
      g.globalAlpha = alpha;
      cut(el);
      g.beginPath();
      g.roundRect(b.x, b.y, b.w, b.h, radius(el, b));
      g.fillStyle = bg;
      g.fill();
      g.restore();
      drew += 1;
    }
    if (el instanceof SVGSVGElement || el instanceof HTMLImageElement) {
      const img = await drawable(el, b.w / k, b.h / k, cs.color);
      if (!img) continue;
      // A face's svg is cropped by its box (`[data-face]`, round or
      // rounded); a glyph is drawn as it is.
      g.save();
      g.globalAlpha = alpha;
      cut(el);
      try {
        g.drawImage(img, b.x, b.y, b.w, b.h);
        drew += 1;
      } catch {
        // A favicon from another origin: nothing to draw, the rest stands.
      }
      g.restore();
    }
  }
  if (drew === 0) return null;
  try {
    return c.toDataURL("image/png").slice("data:image/png;base64,".length);
  } catch {
    return null;
  }
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

/** `top` laid over the opaque `under`, as `#rrggbb` (both as `plain`
    says them). */
function over(top: string, under: string): string {
  const rgba = (c: string) => {
    const hex = /^#([0-9a-f]{6})$/i.exec(c)?.[1];
    if (hex) return [0, 2, 4].map((i) => parseInt(hex.slice(i, i + 2), 16)).concat(1);
    const n = /rgba?\(([^)]*)\)/.exec(c)?.[1].split(",").map(Number) ?? [0, 0, 0, 1];
    return [n[0], n[1], n[2], n[3] ?? 1];
  };
  const [t, u] = [rgba(top), rgba(under)];
  const a = t[3];
  return `#${[0, 1, 2].map((i) => Math.round(t[i] * a + u[i] * (1 - a)).toString(16).padStart(2, "0")).join("")}`;
}

/** A CSS colour token's colours in order — `#hex`, `rgb()`, `rgba()`,
    the way a gradient or a shadow lists them. */
const COLOURS = /#[0-9a-f]{3,8}\b|rgba?\([^)]*\)/gi;

/** `--glass-rim` read as a gradient: `linear-gradient(<angle>, <colour>
    <at>%, …)`. A rim that is not one is the plain ring at its first
    colour. */
export function rimOf(css: string): DeckRim {
  const angle = Number(/(-?[\d.]+)deg/.exec(css)?.[1] ?? 180);
  const stops: DeckRim["stops"] = [];
  for (const m of css.matchAll(new RegExp(`(${COLOURS.source})\\s+(-?[\\d.]+)%`, "gi"))) {
    stops.push({ color: plain(m[1]), at: Number(m[2]) / 100 });
  }
  if (stops.length === 0) {
    const one = css.match(COLOURS)?.[0];
    if (one) stops.push({ color: plain(one), at: 0 }, { color: plain(one), at: 1 });
  }
  return { angle, stops };
}

/** `--glass-shadow`'s widest layer: `<x> <y> <blur> [<spread>] <colour>`. */
export function shadowOf(css: string): DeckShadow | null {
  let best: DeckShadow | null = null;
  for (const layer of css.split(/,(?![^(]*\))/)) {
    if (/\binset\b/.test(layer)) continue;
    const color = layer.match(COLOURS)?.[0];
    const n = layer.replace(COLOURS, "").trim().split(/\s+/).map((v) => parseFloat(v));
    if (!color || n.length < 3 || n.some((v) => !Number.isFinite(v))) continue;
    const at = { y: n[1], blur: n[2], spread: n[3] ?? 0, color: plain(color) };
    if (!best || at.blur > best.blur) best = at;
  }
  return best;
}

function rel(r: DOMRect, o: DOMRect) {
  return { x: r.left - o.left, y: r.top - o.top, w: r.width, h: r.height };
}

/** **An element's box as the band shows it** (UX-35 §3): inside the deck
    band (`[data-deck-band]`) a name is as wide as it is long — the band
    clips it, a reading would not — so the box is cut to the band's
    before it is sent and `UILabel` truncates the tail there. ⚠️ 墓碑: the
    title's whole width went out and the native name ran past the
    capsule. */
function seen(el: Element, o: DOMRect) {
  const b = el.getBoundingClientRect();
  const band = el.closest("[data-deck-band]");
  if (!band) return rel(b, o);
  const c = band.getBoundingClientRect();
  const left = Math.max(b.left, c.left);
  const right = Math.min(b.right, c.right);
  return { x: left - o.left, y: b.top - o.top, w: Math.max(0, right - left), h: b.height };
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
  // Laid out, and not clipped away: the band's neighbours (UX-33) stand
  // outside its box until a drag pulls them in.
  const shown = (el: Element) => {
    if (el.getClientRects().length === 0) return false;
    const clip = el.closest("[data-deck-band]");
    if (!clip) return true;
    const c = clip.getBoundingClientRect();
    const b = el.getBoundingClientRect();
    return b.left < c.right && b.right > c.left;
  };
  // ⚠️ 墓碑 (UX-13 → UX-29): the folded strip stood on a band of glass to
  // the screen's foot. The folded deck is Safari's capsule now — a `.glass`
  // shape like the rest, read below, its words in the ink the face chose
  // from the page's ground (`page:ground`).
  for (const el of bar.querySelectorAll<HTMLElement>(".glass")) {
    if (!shown(el)) continue;
    const b = el.getBoundingClientRect();
    glass.push({ ...rel(b, o), r: Math.min(b.width, b.height) / 2, sel: el.classList.contains("glass-sel") });
  }
  // Face seats (UX-32's assembly), each as one picture (UX-34).
  for (const seat of bar.querySelectorAll<HTMLElement>("[data-side-face]")) {
    if (!shown(seat)) continue;
    const b = seat.getBoundingClientRect();
    const png = await seatPng(seat, b.width, b.height);
    if (png) images.push({ ...rel(b, o), png });
  }
  // Glyphs: every svg in the deck that is not a face or in a seat, in its own ink.
  for (const svg of bar.querySelectorAll<SVGSVGElement>("svg")) {
    if (!shown(svg) || svg.closest("[data-face], [data-tabs-glyph], [data-side-face]")) continue;
    const b = svg.getBoundingClientRect();
    const color = getComputedStyle(svg).color;
    // At the opacity the DOM shows it: a step with nowhere to go is
    // dimmed there (UX-38 audit: the native ← was full ink on a first page).
    let alpha = 1;
    for (let at: Element | null = svg; at && bar.contains(at); at = at.parentElement) alpha *= Number(getComputedStyle(at).opacity) || 0;
    const png = await svgPng(svg, b.width, b.height, color, alpha);
    if (png) images.push({ ...rel(b, o), png });
  }
  // A face standing outside a seat, cropped as the DOM crops it.
  for (const face of bar.querySelectorAll<HTMLElement>("[data-face]")) {
    if (!shown(face) || face.closest("[data-side-face]")) continue;
    const b = face.getBoundingClientRect();
    const png = await facePng(face, b.width, b.height);
    if (png) images.push({ ...rel(b, o), png });
  }
  const tabs = bar.querySelector<HTMLElement>("[data-tabs-glyph]");
  if (tabs && shown(tabs)) {
    const b = tabs.getBoundingClientRect();
    // The squares wear the glass's tint over the ground (`--tabs-fill`, a
    // background image the computed colour leaves out).
    const fill = over(plain(root.getPropertyValue("--glass").trim()), plain(getComputedStyle(tabs.querySelector("[data-tabs-front]") ?? tabs).backgroundColor));
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
  // A caption holding the failure is drawn as its two words (UX-22).
  const said = "[data-deck-title], [data-deck-strip] > span:first-child, [data-region-caption]:not(:has([data-page-failed])), [data-page-failed-word], [data-page-retry]";
  for (const el of bar.querySelectorAll<HTMLElement>(said)) {
    if (!shown(el)) continue;
    const cs = getComputedStyle(el);
    labels.push({
      ...seen(el, o),
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
    // The title is pressed anywhere on the band's current segment, its
    // face included, as the DOM band takes it (UX-35 §1).
    const at = seen(el.closest('[data-band-seg="cur"]') ?? el, o);
    hits.push({ ...at, seat, swipe: false });
  });
  // The pill's progress line, read from the DOM's own (`PillProgress`)
  // while it is there: the DOM pill stays the source of truth.
  let progress: DeckProgress | null = null;
  const line = [...bar.querySelectorAll<HTMLElement>("[data-pill-progress],[data-deck-strip-progress]")].find(line => shown(line) && (line.dataset.pillProgress ?? line.dataset.deckStripProgress ?? "") !== "");
  const run = line?.firstElementChild;
  const glassOf = line?.closest<HTMLElement>("[data-deck-pill],[data-deck-strip]");
  const share = Number(line?.dataset.pillProgress ?? line?.dataset.deckStripProgress);
  if (line && run && glassOf && Number.isFinite(share) && share >= 0 && share <= 1) {
    const cs = getComputedStyle(run), g = glassOf.getBoundingClientRect();
    progress = {
      ...rel(line.getBoundingClientRect(), o),
      clip: { ...rel(g, o), r: Math.min(g.width, g.height) / 2 },
      color: plain(cs.backgroundColor), share,
    };
  }
  // The pill swipes sideways between tabs, as the DOM pill does.
  const pill = bar.querySelector<HTMLElement>("[data-deck-pill]");
  if (pill && shown(pill)) hits.push({ ...rel(pill.getBoundingClientRect(), o), seat: -1, swipe: true });
  return {
    x: o.left,
    y: o.top,
    w: o.width,
    h: o.height,
    // BrowserPane clips the native frame above this deck. A second
    // contentInset would shorten/pad the document twice.
    inset: 0,
    dark: document.documentElement.getAttribute("data-theme") === "dark" ||
      (document.documentElement.getAttribute("data-theme") !== "light" && matchMedia("(prefers-color-scheme: dark)").matches),
    tint: plain(root.getPropertyValue("--glass").trim()),
    selTint: plain(root.getPropertyValue("--glass-sel").trim()),
    rim: rimOf(root.getPropertyValue("--glass-rim")),
    shadow: shadowOf(root.getPropertyValue("--glass-shadow")),
    glass,
    images,
    labels,
    hits,
    progress,
  };
}
