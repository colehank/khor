/** Background opacity belongs to paint, never to terminal data or glyphs.
 * xterm 6 / addon-webgl 0.19: keep the original RGB/ANSI palette (including
 * inverse text), replacing each background region once. DOM rows follow the
 * same rule, including their trailing blank cells. No ANSI rewriting.
 *
 * These narrowly scoped adapters reach pinned renderer internals. Reattach
 * on RenderService.setRenderer so context loss and recovery share the rule.
 */
import type { Terminal } from "@xterm/xterm";
import { windowOpacity } from "./window-surface";

type Color = { css: string; rgba: number };
type Colors = { background: Color; foreground: Color; ansi: Color[] };
type Factory = { createRow: (...args: unknown[]) => HTMLSpanElement[]; _themeService: { colors: Colors } };
type Rectangles = {
  _gl: WebGL2RenderingContext;
  _vertices: { attributes: Float32Array; count: number };
  renderBackgrounds: () => void;
};
type Renderer = {
  _rowFactory?: Factory;
  _rectangleRenderer?: { value?: Rectangles };
  _model?: {
    cursor?: { x: number; y: number; width: number; style: string };
    selection?: { isCellSelected: (term: Terminal, x: number, y: number) => boolean };
  };
};
type Service = { setRenderer: (renderer: Renderer) => void; _renderer: { value?: Renderer } };

/** Renderer-only alpha; CSS values below originate in xterm's palette. */
function translucent(color: string, alpha: number): string {
  return `color-mix(in srgb, ${color} ${alpha * 100}%, transparent)`;
}

function domBackgrounds(factory: Factory, term: Terminal): void {
  const create = factory.createRow;
  factory.createRow = function (...args) {
    const spans = create.apply(this, args);
    const alpha = windowOpacity();
    const colors = this._themeService.colors;
    for (const span of spans) {
      // Caret, selection and search marks retain their UI contrast pair.
      if (span.classList.contains("xterm-cursor-block") || span.classList.contains("xterm-decoration-top")) continue;
      const ansi = /(?:^| )xterm-bg-(\d+)(?: |$)/.exec(span.className);
      const bg = span.style.backgroundColor || (ansi ? (Number(ansi[1]) === 257 ? colors.foreground : colors.ansi[Number(ansi[1])])?.css : null) || colors.background.css;
      span.style.backgroundColor = translucent(bg, alpha);
    }
    // xterm trims cells with default backgrounds. Paint that remaining run,
    // rather than laying a second default background behind every span.
    const line = args[0] as { getNoBgTrimmedLength: () => number };
    const length = Math.min(term.cols, Math.max(line.getNoBgTrimmedLength(), args[2] ? Number(args[5]) + 1 : 0));
    if (length < term.cols) {
      const tail = document.createElement("span");
      tail.setAttribute("aria-hidden", "true");
      tail.style.width = `${(term.cols - length) * Number(args[7])}px`;
      tail.style.backgroundColor = translucent(colors.background.css, alpha);
      spans.push(tail);
    }
    return spans;
  };
}

function webglBackgrounds(rect: Rectangles, renderer: Renderer, term: Terminal): void {
  const draw = rect.renderBackgrounds;
  let scratch = new Float32Array(0);
  rect.renderBackgrounds = function () {
    const alpha = windowOpacity();
    if (alpha === 1) { draw.call(this); return; }
    const { attributes, count } = this._vertices;
    if (scratch.length !== attributes.length) scratch = new Float32Array(attributes.length);
    scratch.set(attributes);
    // The first rectangle fills the viewport; later rectangles REPLACE it.
    // Blending here would turn two 50% backgrounds into 75% opacity. Store
    // premultiplied pixels, as required by WebGL's browser compositor.
    for (let i = 0; i < count * 8; i += 8) {
      const cursor = renderer._model?.cursor;
      const caret = i > 0 && cursor?.style === "block"
        && Math.abs(scratch[i] - cursor.x / term.cols) < 0.00001
        && Math.abs(scratch[i + 1] - cursor.y / term.rows) < 0.00001
        && Math.abs(scratch[i + 2] - cursor.width / term.cols) < 0.00001;
      const selected = i > 0 && renderer._model?.selection?.isCellSelected(
        term, Math.round(scratch[i] * term.cols),
        Math.round(scratch[i + 1] * term.rows) + term.buffer.active.viewportY,
      );
      const opacity = caret || selected ? 1 : alpha;
      scratch[i + 4] *= opacity;
      scratch[i + 5] *= opacity;
      scratch[i + 6] *= opacity;
      scratch[i + 7] = opacity;
    }
    const gl = this._gl;
    const blending = gl.isEnabled(gl.BLEND);
    gl.disable(gl.BLEND);
    this._vertices.attributes = scratch;
    try { draw.call(this); }
    finally {
      this._vertices.attributes = attributes;
      if (blending) gl.enable(gl.BLEND);
      // Glyph coverage contributes alpha once, independently of RGB blending.
      gl.blendFuncSeparate(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA, gl.ONE, gl.ONE_MINUS_SRC_ALPHA);
    }
  };
}

export function keepTerminalGlass(term: Terminal, pane: HTMLElement): () => void {
  const service = (term as unknown as { _core: { _renderService: Service } })._core._renderService;
  const guarded = new WeakSet<object>();
  const attach = (renderer?: Renderer) => {
    if (!renderer || guarded.has(renderer)) return;
    guarded.add(renderer);
    if (renderer._rowFactory) domBackgrounds(renderer._rowFactory, term);
    const rect = renderer._rectangleRenderer?.value;
    if (rect?._vertices?.attributes instanceof Float32Array && rect._gl && typeof rect.renderBackgrounds === "function") webglBackgrounds(rect, renderer, term);
  };
  const set = service.setRenderer;
  service.setRenderer = function (renderer) { set.call(this, renderer); attach(renderer); };
  attach(service._renderer.value);
  pane.setAttribute("data-term-glass", "");
  const screen = term.element?.querySelector<HTMLElement>(".xterm-screen");
  // Padding owns a single palette tint; the screen owns its own cell runs.
  // Geometry changes on fit/resize, never on terminal output or history.
  const measure = () => {
    if (!screen) return;
    const p = pane.getBoundingClientRect(), s = screen.getBoundingClientRect();
    const x = s.left - p.left, y = s.top - p.top;
    pane.style.setProperty("--term-ground-clip", `path(evenodd, "M0 0H${p.width}V${p.height}H0Z M${x} ${y}H${x + s.width}V${y + s.height}H${x}Z")`);
  };
  const resized = new ResizeObserver(measure);
  resized.observe(pane);
  if (screen) resized.observe(screen);
  pane.addEventListener("scroll", measure, true);
  measure();
  return () => {
    service.setRenderer = set;
    resized.disconnect();
    pane.removeEventListener("scroll", measure, true);
    pane.removeAttribute("data-term-glass");
    pane.style.removeProperty("--term-ground-clip");
  };
}
