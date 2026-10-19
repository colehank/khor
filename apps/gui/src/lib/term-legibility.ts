/** Viewer-only paint adapter for xterm 6 / addon-webgl 0.19.
* DIM retains a quieter colour, with the full selected contrast floor.
* Cursor colour is resolved against its cell, not the terminal's ground.
* No buffer, ANSI stream, palette query response or PTY is modified.
*/
import type { Terminal, IBufferCell } from '@xterm/xterm';
import { cursorPair, mixRgb, readableRgb, rgbCss } from './term-contrast';
type Color = {
  rgba: number;
  css: string;
};
type Colors = {
  background: Color;
  foreground: Color;
  cursor: Color;
  cursorAccent: Color;
  ansi: Color[];
};
type Cell = {
  fg: number;
  bg: number;
  isDim: () => number;
  getCode: () => number;
};
type Factory = {
  _applyMinimumContrast: (element: HTMLElement, bg: Color, fg: Color, cell: Cell, bgOverride?: Color, fgOverride?: Color) => boolean;
};
type Resolver = {
  result: {
    fg: number;
    bg: number;
    ext: number;
  };
  resolve: (cell: Cell, x: number, y: number, width: number) => void;
};
type Renderer = {
  renderRows: (start: number, end: number) => void;
  _injectCss?: (colors: Colors) => void;
  _rowFactory?: Factory;
  _cellColorResolver?: Resolver;
  _rectangleRenderer?: {
    value?: {
      _cursorFloat: Float32Array;
    };
  };
  _themeService: {
    colors: Colors;
  };
};
type Service = {
  setRenderer: (renderer: Renderer) => void;
  _renderer: {
    value?: Renderer;
  };
};
// xterm's pinned packed attribute layout; only render scratch values use it.
const COLOR_MASK = 0x3ffffff, CM_MASK = 0x3000000, CM_RGB = 0x3000000, RGB_MASK = 0xffffff, DIM = 0x8000000, INVERSE = 0x4000000, BOLD = 0x8000000;
const decorative = (code: number) => (code >= 0x2500 && code <= 0x259f) || (code >= 0xe0a4 && code <= 0xe0d6);
const asColor = (rgb: number): Color => ({ rgba: ((rgb << 8) | 255) >>> 0, css: rgbCss(rgb) });
function colorRgb(color: Color, ground: number): number { return mixRgb(color.rgba >>> 8, ground, (color.rgba & 255) / 255); }
function packedRgb(value: number, colors: Colors, ground: Color, bold = false): number {
  switch (value & CM_MASK) {
    case CM_RGB: return value & RGB_MASK;
    case 0x1000000:
    case 0x2000000: {
      let index = value & 255;
      if (bold && index < 8)
        index += 8;
      return (colors.ansi[index]?.rgba ?? 0) >>> 8;
    }
    default: return ground.rgba >>> 8;
  }
}
function surface(fg: number, bg: number, colors: Colors, boldBright: boolean): {
  fg: number;
  bg: number;
} {
  const inverse = !!(fg & INVERSE);
  const back = inverse ? packedRgb(fg, colors, colors.foreground) : packedRgb(bg, colors, colors.background);
  const front = inverse ? packedRgb(bg, colors, colors.background) : packedRgb(fg, colors, colors.foreground, boldBright && !!(fg & BOLD));
  return { fg: front, bg: back };
}
function cursorSurface(term: Terminal, colors: Colors, dom: boolean): number {
  const b = term.buffer.active, c = b.getLine(b.baseY + b.cursorY)?.getCell(Math.min(b.cursorX, term.cols - 1));
  if (!c)
    return colors.background.rgba >>> 8;
  const value = (cell: IBufferCell, front: boolean) => {
    const mode = front ? cell.getFgColorMode() : cell.getBgColorMode(), n = front ? cell.getFgColor() : cell.getBgColor();
    return mode === CM_RGB ? n : mode === 0x1000000 || mode === 0x2000000 ? (colors.ansi[n]?.rgba ?? 0) >>> 8 : dom ? colorRgb(front ? colors.foreground : colors.background, colors.background.rgba >>> 8) : (front ? colors.foreground : colors.background).rgba >>> 8;
  };
  return value(c, !!c.isInverse());
}
export function protectTerminal(term: Terminal): () => void {
  const service = (term as unknown as {
    _core: {
      _renderService: Service;
    };
  })._core._renderService;
  const restorers: (() => void)[] = [], cache = new Map<string, number>();
  let active: Renderer | undefined, stopped = false;
  const release = () => {
    for (const restore of restorers.splice(0).reverse()) restore();
    active = undefined;
  };
  const ratio = () => term.options.minimumContrastRatio ?? 1;
  const dim = (fg: number, bg: number) => {
    const key = `${fg}/${bg}/${ratio()}`;
    let value = cache.get(key);
    if (value === undefined) {
      value = readableRgb(mixRgb(fg, bg, .5), bg, ratio());
      if (cache.size >= 512)
        cache.clear();
      cache.set(key, value);
    }
    return value;
  };
  const attach = (renderer?: Renderer) => {
    if (!renderer || active === renderer)
      return;
    release();
    active = renderer;
    const factory = renderer._rowFactory, resolver = renderer._cellColorResolver;
    if (factory) {
      const apply = factory._applyMinimumContrast;
      factory._applyMinimumContrast = function (element, bg, fg, cell, bgOverride, fgOverride) {
        if (ratio() <= 1 || !cell.isDim() || decorative(cell.getCode()))
          return apply.call(this, element, bg, fg, cell, bgOverride, fgOverride);
        // A half-alpha bgOverride is xterm's DIM hint, not another surface.
        const back = colorRgb(bgOverride && (bgOverride.rgba & 255) === 255 ? bgOverride : bg, renderer._themeService.colors.background.rgba >>> 8);
        element.style.color = rgbCss(dim(colorRgb(fgOverride ?? fg, back), back));
        return true;
      };
      restorers.push(() => { factory._applyMinimumContrast = apply; });
    }
    if (resolver) {
      const resolve = resolver.resolve;
      resolver.resolve = function (cell, x, y, width) {
        resolve.call(this, cell, x, y, width);
        if (ratio() <= 1 || !(this.result.bg & DIM) || decorative(cell.getCode()))
          return;
        const colors = renderer._themeService.colors, p = surface(this.result.fg, this.result.bg, colors, !!term.options.drawBoldTextInBrightColors);
        this.result.bg = (this.result.bg & ~(COLOR_MASK | DIM)) | CM_RGB | p.bg;
        this.result.fg = (this.result.fg & ~(COLOR_MASK | INVERSE)) | CM_RGB | dim(p.fg, p.bg);
      };
      restorers.push(() => { resolver.resolve = resolve; });
    }
    // DOM keeps cursor colours in CSS, including blink keyframes. Variables
    // preserve those exact shape/blink rules without a second cursor layer.
    if (renderer._injectCss) {
      const inject = renderer._injectCss;
      renderer._injectCss = function (colors) { inject.call(this, { ...colors, cursor: { ...colors.cursor, css: 'var(--khor-term-cursor)' }, cursorAccent: { ...colors.cursorAccent, css: 'var(--khor-term-cursor-accent)' } }); };
      renderer._injectCss(renderer._themeService.colors);
      restorers.push(() => { renderer._injectCss = inject; inject.call(renderer, renderer._themeService.colors); });
    }
    const render = renderer.renderRows;
    let cursorKey = "", paintCursor = asColor(0), paintAccent = asColor(0), cursorFloat = new Float32Array(4);
    renderer.renderRows = function (start, end) {
      const colors = this._themeService.colors, originalCursor = colors.cursor, originalAccent = colors.cursorAccent;
      const pair = ratio() > 1 ? cursorPair(cursorSurface(term, colors, !!factory), originalCursor.rgba >>> 8, originalAccent.rgba >>> 8, ratio()) : { cursor: originalCursor.rgba >>> 8, accent: originalAccent.rgba >>> 8 };
      term.element?.style.setProperty('--khor-term-cursor', ratio() > 1 ? rgbCss(pair.cursor) : originalCursor.css);
      term.element?.style.setProperty('--khor-term-cursor-accent', ratio() > 1 ? rgbCss(pair.accent) : originalAccent.css);
      const rect = this._rectangleRenderer?.value, oldFloat = rect?._cursorFloat;
      // Scoped to this draw: ThemeService never receives an update, so colour
      // queries and shared glyph atlases retain the original program palette.
      if (resolver && ratio() > 1) {
        const key = `${pair.cursor}/${pair.accent}`;
        if (key !== cursorKey) {
          cursorKey = key;
          paintCursor = asColor(pair.cursor);
          paintAccent = asColor(pair.accent);
          cursorFloat = new Float32Array([pair.cursor >>> 16 & 255, pair.cursor >>> 8 & 255, pair.cursor & 255, 255].map(v => v / 255));
        }
        colors.cursor = paintCursor;
        colors.cursorAccent = paintAccent;
        if (rect)
          rect._cursorFloat = cursorFloat;
      }
      try {
        render.call(this, start, end);
      }
      finally {
        colors.cursor = originalCursor;
        colors.cursorAccent = originalAccent;
        if (rect && oldFloat)
          rect._cursorFloat = oldFloat;
      }
    };
    restorers.push(() => { renderer.renderRows = render; });
  };
  const set = service.setRenderer;
  service.setRenderer = function (renderer) {
    if (active !== renderer) release();
    set.call(this, renderer);
    attach(renderer);
  };
  attach(service._renderer.value);
  term.refresh(0, term.rows - 1);
  return () => {
    if (stopped) return;
    stopped = true;
    service.setRenderer = set;
    release();
    cache.clear();
    term.element?.style.removeProperty('--khor-term-cursor');
    term.element?.style.removeProperty('--khor-term-cursor-accent');
  };
}
