/** sRGB paint calculations. Values here are terminal colour data, not app tokens. */
export function mixRgb(foreground: number, background: number, alpha: number): number {
  let result = 0;
  for (const shift of [16, 8, 0])
    result |= Math.round(((foreground >>> shift) & 255) * alpha + ((background >>> shift) & 255) * (1 - alpha)) << shift;
  return result;
}
const linear = (v: number) => { const n = v / 255; return n <= .04045 ? n / 12.92 : ((n + .055) / 1.055) ** 2.4; };
const luminance = (rgb: number) => .2126 * linear(rgb >>> 16 & 255) + .7152 * linear(rgb >>> 8 & 255) + .0722 * linear(rgb & 255);
export function contrastRatio(a: number, b: number): number { const x = luminance(a), y = luminance(b); return (Math.max(x, y) + .05) / (Math.min(x, y) + .05); }
export function readableRgb(foreground: number, background: number, ratio: number): number {
  if (ratio <= 1 || contrastRatio(foreground, background) >= ratio)
    return foreground;
  // Choose the reachable pole requiring the least channel movement. Binary
  // search stays on the original hue's path; rounding is checked each step.
  let best = foreground, distance = Infinity;
  for (const pole of [0, 0xffffff]) {
    if (contrastRatio(pole, background) < ratio)
      continue;
    let lo = 0, hi = 1;
    for (let i = 0; i < 12; i++) {
      const at = (lo + hi) / 2;
      if (contrastRatio(mixRgb(pole, foreground, at), background) >= ratio)
        hi = at;
      else
        lo = at;
    }
    const candidate = mixRgb(pole, foreground, hi);
    const d = [16, 8, 0].reduce((sum, s) => sum + (((candidate >>> s) & 255) - ((foreground >>> s) & 255)) ** 2, 0);
    if (d < distance) {
      best = candidate;
      distance = d;
    }
  }
  // A mid-luminance background cannot support arbitrary ratios (e.g. 7).
  return distance < Infinity ? best : contrastRatio(0, background) > contrastRatio(0xffffff, background) ? 0 : 0xffffff;
}
export function cursorPair(background: number, cursor: number, accent: number, ratio: number): {
  cursor: number;
  accent: number;
} {
  // Keep a user's cursor colour when it is already legible. A stable pole
  // avoids slightly different grey cursors for every shade of input surface.
  const target = Math.max(3, ratio);
  const ink = contrastRatio(cursor, background) >= target ? cursor : contrastRatio(0, background) > contrastRatio(0xffffff, background) ? 0 : 0xffffff;
  const glyph = contrastRatio(accent, ink) >= Math.max(4.5, ratio) ? accent : contrastRatio(0, ink) > contrastRatio(0xffffff, ink) ? 0 : 0xffffff;
  return { cursor: ink, accent: glyph };
}
export const rgbCss = (rgb: number) => '#' + (rgb >>> 0).toString(16).padStart(6, '0');
