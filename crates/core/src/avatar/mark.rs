//! What a kind mark painted on a face wears: **which ink, and how much
//! veil under it** — derived per face, here, next to the face itself.
//!
//! The owner, 2026-10-02, on the first version (one black veil at 0.42
//! on every face, one white ink):「浮现这，给原始头像上了蒙板？感觉没那
//! 么好看了」. Of six ways tried on real faces they chose **按脸换墨，不够
//! 才压**: a bright face gets a dark mark, a dark face a light one, and
//! no veil at all; only a face whose centre mixes light and dark gets a
//! veil, and only as much as it needs — black under a light mark, white
//! under a dark one.
//!
//! ## Why the core, and not the brush
//!
//! The brush (`Avatar.tsx`) computes nothing, so the phone and the
//! desktop agree; a face whose mark is dark on one end and light on the
//! other is the same machine looking different again. So the core,
//! which already decides what a face looks like, also decides what sits
//! on it. To do that it needs the face's **pixels**, which it never had
//! (it emits parameters) — hence the small raster below. It paints only
//! what lies under a mark: the centre disc, at a fixed grid.
//!
//! ## The rule (searched in a lab on the browser's own raster first)
//!
//! 1. **Region**: the disc at the face's centre whose diameter is
//!    [`MARK_REGION`] of the side, inside the face's crop. A tab's mark
//!    is 12 on a 16 face — that disc exactly — and a row's 18 on 32 sits
//!    inside it.
//! 2. **Ink**: the one most of the region wants — the *median* contrast
//!    of each ink against the bare face; the dark ink wins a tie.
//!    Choosing by "whichever needs less veil" instead put dark marks on
//!    white-washed navy faces, and that looked wrong.
//! 3. **Veil**: the least alpha, in steps of [`VEIL_STEP`], of black
//!    (under the light ink) or white (under the dark ink), mixed in sRGB
//!    the way CSS mixes, at which the **worst pixel** of the region reads
//!    [`MARK_TARGET`] against the ink. Not a quantile: any tolerance let
//!    a real glyph's stroke land on the pixel it forgave and fall under
//!    3:1. Black never goes past [`MAX_BLACK_VEIL`] — the old universal
//!    veil, which already holds the light ink at 3.04 on pure white.
//!
//! The mark is **opaque** (the owner, 2026-10-02:「还是不透明吧」), so
//! the mark's pixel is simply the ink. ⚠️ 墓碑: an opacity was weighed
//! and measured — at 80% 263 of the 336 sample faces needed no veil
//! (against 269 opaque), at 60% only 95, and 10 never reached 3:1 at
//! any veil. It is not a field, not a token and not a constant.
//!
//! The rule is **glyph-blind**: it knows the region a mark can cover,
//! never which mark. Every glyph the family draws stays above 3:1
//! because the worst pixel anywhere in that region does.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::{Avatar, AvatarPaint, Shape, Vein};

/// The light ink: white. `--mark-ink-light` in the GUI's tokens.
pub const MARK_INK_LIGHT: &str = "#ffffff";

/// The dark ink: the warm near-black of the app's own ink, opaque.
/// `--mark-ink-dark` in the GUI's tokens.
pub const MARK_INK_DARK: &str = "#14140f";

/// What the worst pixel under the mark must read against the ink. Above
/// a graphic's 3:1 by a margin, because the browser's raster and this
/// one are not the same raster: the lab's worst real glyph at this
/// target was 3.16–3.22.
pub const MARK_TARGET: f64 = 3.2;

/// The region's diameter as a fraction of the face's side.
pub const MARK_REGION: f64 = 0.75;

/// The veil moves in these steps.
pub const VEIL_STEP: f64 = 0.02;

/// The most black a face is ever veiled with — the universal veil this
/// rule replaced (white under it reads 3.04 against white ink, so the
/// light ink clears 3:1 on any face at this bound).
pub const MAX_BLACK_VEIL: f64 = 0.42;

/// How far past the canvas, in σ, marble's filter region reaches — the
/// brush's `BLUR_MARGIN` (`Avatar.tsx`, its reason there). The region
/// cuts the vein before it is blurred, so it is part of the picture.
const BLUR_MARGIN: f64 = 3.43;

/// The blurred field's grid, in the vein's own units.
const FIELD_STEP: f64 = 2.0;

/// Samples across the face's side. 80 is one per canvas unit on marble
/// and bauhaus, two and a half per CSS pixel on a row's 32.
const GRID: usize = 80;

/// Which ink a mark on this face is drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum MarkInk {
    /// [`MARK_INK_LIGHT`], over a **black** veil when there is one.
    Light,
    /// [`MARK_INK_DARK`], over a **white** veil when there is one.
    Dark,
}

impl MarkInk {
    /// The ink as `#rrggbb`.
    pub const fn hex(self) -> &'static str {
        match self {
            MarkInk::Light => MARK_INK_LIGHT,
            MarkInk::Dark => MARK_INK_DARK,
        }
    }

    /// The veil's colour under this ink: the ink's opposite.
    pub const fn veil_hex(self) -> &'static str {
        match self {
            MarkInk::Light => "#000000",
            MarkInk::Dark => "#ffffff",
        }
    }
}

/// What a kind mark painted on this face wears (类型标记「浮现」).
///
/// **How to paint it**: when `veil` is above 0, a layer of the ink's
/// opposite colour (black under [`MarkInk::Light`], white under
/// [`MarkInk::Dark`]) at alpha `veil` over the whole face, cropped with
/// it; then the mark, opaque, in the ink. A face with no mark on it
/// ignores this entirely.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
pub struct MarkOnFace {
    pub ink: MarkInk,
    /// The veil's alpha, `0.0..=1.0` in steps of [`VEIL_STEP`]; `0` is
    /// no veil. Never above [`MAX_BLACK_VEIL`] under the light ink.
    pub veil: f64,
}

/* ═══════════ colour ═══════════ */

/// sRGB, `0.0..=255.0` per channel.
pub(crate) type Rgb = [f64; 3];

pub(crate) fn rgb(hex: &str) -> Rgb {
    let v = u32::from_str_radix(hex.get(1..7).unwrap_or("000000"), 16).unwrap_or(0);
    [
        ((v >> 16) & 0xff) as f64,
        ((v >> 8) & 0xff) as f64,
        (v & 0xff) as f64,
    ]
}

/// Each eight-bit level, linearised — the browser keeps eight bits a
/// channel, so a colour is read at the level it would be painted at.
fn lin_table() -> &'static [f64; 256] {
    static TABLE: std::sync::OnceLock<[f64; 256]> = std::sync::OnceLock::new();
    TABLE.get_or_init(|| {
        let mut t = [0.0; 256];
        for (i, v) in t.iter_mut().enumerate() {
            let s = i as f64 / 255.0;
            *v = if s <= 0.040_45 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            };
        }
        t
    })
}

/// WCAG relative luminance of an sRGB colour, each channel rounded to
/// its eight-bit level.
pub(crate) fn lum(c: Rgb) -> f64 {
    let t = lin_table();
    let at = |v: f64| t[v.round().clamp(0.0, 255.0) as usize];
    0.2126 * at(c[0]) + 0.7152 * at(c[1]) + 0.0722 * at(c[2])
}

pub(crate) fn ratio(a: f64, b: f64) -> f64 {
    let (hi, lo) = if a > b { (a, b) } else { (b, a) };
    (hi + 0.05) / (lo + 0.05)
}

/// `c` under a layer of `v` (a grey level) at alpha `a`, in sRGB — what
/// CSS does with an `rgba()` laid over a colour.
fn veiled(c: Rgb, v: f64, a: f64) -> Rgb {
    [
        c[0] * (1.0 - a) + v * a,
        c[1] * (1.0 - a) + v * a,
        c[2] * (1.0 - a) + v * a,
    ]
}

/* ═══════════ the rule ═══════════ */

/// The pair for this face.
pub(crate) fn derive(face: &Avatar) -> MarkOnFace {
    choose(&region(face))
}

/// The rule on the region's pixels.
pub(crate) fn choose(px: &[Rgb]) -> MarkOnFace {
    if px.is_empty() {
        return MarkOnFace {
            ink: MarkInk::Light,
            veil: MAX_BLACK_VEIL,
        };
    }
    let bare: Vec<f64> = px.iter().map(|&p| lum(p)).collect();
    let median = |ink: f64| {
        let mut v: Vec<f64> = bare.iter().map(|&l| ratio(ink, l)).collect();
        v.sort_by(|a, b| a.total_cmp(b));
        v[v.len() / 2]
    };
    let ink = if median(lum(rgb(MARK_INK_DARK))) >= median(lum(rgb(MARK_INK_LIGHT))) {
        MarkInk::Dark
    } else {
        MarkInk::Light
    };
    let (grey, top) = match ink {
        MarkInk::Light => (0.0, MAX_BLACK_VEIL),
        MarkInk::Dark => (255.0, 1.0),
    };
    let ink_l = lum(rgb(ink.hex()));
    let steps = (top / VEIL_STEP).round() as usize;
    // The pixel that failed last is tried first: most steps fail, and
    // fail on the same pixel.
    let mut culprit = 0;
    for k in 0..=steps {
        let a = alpha(k);
        let reads = |i: usize| ratio(ink_l, lum(veiled(px[i], grey, a))) >= MARK_TARGET;
        if !reads(culprit) {
            continue;
        }
        match (0..px.len()).find(|&i| !reads(i)) {
            None => return MarkOnFace { ink, veil: a },
            Some(i) => culprit = i,
        }
    }
    MarkOnFace {
        ink,
        veil: alpha(steps),
    }
}

/// Step `k` as a two-decimal alpha (`0.06`, not `0.060000000000000005`).
fn alpha(k: usize) -> f64 {
    (k as f64 * VEIL_STEP * 100.0).round() / 100.0
}

/* ═══════════ the raster: what lies under a mark ═══════════ */

/// The face's pixels in the region a mark can cover, sampled at the
/// centres of a [`GRID`]-square grid over the side. Paints what the
/// brush paints, in its order: the ground, then —
///
/// - marble: each vein, its polygon blurred by `blur` **in the vein's
///   own coordinates** (an SVG filter in `userSpaceOnUse` lives in the
///   space of the element that references it, transform included, so
///   the blur scales with the vein), cut to the filter region first,
///   composited normally or with `overlay`;
/// - bauhaus: each shape, hard-edged;
/// - beam: the wrapper alone — a mark leaves the features out.
pub(crate) fn region(face: &Avatar) -> Vec<Rgb> {
    let side = face.canvas;
    let c = side / 2.0;
    let reach = side * MARK_REGION / 2.0;
    let corner = face.radius_ratio * side;
    let mut points = vec![];
    for j in 0..GRID {
        for i in 0..GRID {
            let x = (i as f64 + 0.5) * side / GRID as f64;
            let y = (j as f64 + 0.5) * side / GRID as f64;
            if (x - c).powi(2) + (y - c).powi(2) <= reach * reach
                && in_rounded(x, y, 0.0, side, corner)
            {
                points.push((x, y));
            }
        }
    }
    let ground = rgb(&face.background);
    let mut px = vec![ground; points.len()];
    match &face.paint {
        AvatarPaint::Marble { blur, veins } => {
            let lo = -blur * BLUR_MARGIN;
            let hi = side + blur * BLUR_MARGIN;
            for v in veins {
                let poly = vein_polygon(v.vein);
                let local: Vec<(f64, f64)> = points
                    .iter()
                    .map(|&(x, y)| unplace(x, y, v.dx, v.dy, v.rotate, v.scale, c))
                    .collect();
                let alpha = blurred(&poly, *blur, lo, hi, &local);
                let col = rgb(&v.color);
                for (p, a) in px.iter_mut().zip(alpha) {
                    let top = if v.overlay { overlay(*p, col) } else { col };
                    *p = quantize([
                        p[0] * (1.0 - a) + top[0] * a,
                        p[1] * (1.0 - a) + top[1] * a,
                        p[2] * (1.0 - a) + top[2] * a,
                    ]);
                }
            }
        }
        AvatarPaint::Bauhaus { shapes } => {
            for s in shapes {
                let col = rgb(&s.color);
                for (p, &(x, y)) in px.iter_mut().zip(&points) {
                    let (u, w) = unplace(x, y, s.dx, s.dy, s.rotate, 1.0, c);
                    let inside = match s.shape {
                        Shape::Rect => u >= s.x && u <= s.x + s.w && w >= s.y && w <= s.y + s.h,
                        Shape::Circle => {
                            let (rx, ry) = (s.w / 2.0, s.h / 2.0);
                            ((u - s.x - rx) / rx).powi(2) + ((w - s.y - ry) / ry).powi(2) <= 1.0
                        }
                    };
                    if inside {
                        *p = col;
                    }
                }
            }
        }
        AvatarPaint::Beam {
            wrapper_rx,
            wrapper_scale,
            wrapper_rotate,
            wrapper_dx,
            wrapper_dy,
            wrapper_color,
            ..
        } => {
            let col = rgb(wrapper_color);
            for (p, &(x, y)) in px.iter_mut().zip(&points) {
                let (u, w) = unplace(
                    x,
                    y,
                    *wrapper_dx,
                    *wrapper_dy,
                    *wrapper_rotate,
                    *wrapper_scale,
                    c,
                );
                if in_rounded(u, w, 0.0, side, *wrapper_rx) {
                    *p = col;
                }
            }
        }
    }
    px
}

/// The browser keeps eight bits per channel between layers.
fn quantize(c: Rgb) -> Rgb {
    [
        c[0].round().clamp(0.0, 255.0),
        c[1].round().clamp(0.0, 255.0),
        c[2].round().clamp(0.0, 255.0),
    ]
}

/// `overlay(backdrop, source)` per channel (CSS Compositing: hard-light
/// with the layers swapped).
fn overlay(b: Rgb, s: Rgb) -> Rgb {
    let one = |cb: f64, cs: f64| {
        let (cb, cs) = (cb / 255.0, cs / 255.0);
        255.0
            * if cb <= 0.5 {
                2.0 * cb * cs
            } else {
                1.0 - 2.0 * (1.0 - cb) * (1.0 - cs)
            }
    };
    [one(b[0], s[0]), one(b[1], s[1]), one(b[2], s[2])]
}

/// Whether `(x, y)` is inside the square `lo..hi` with corners rounded
/// by `r`.
fn in_rounded(x: f64, y: f64, lo: f64, hi: f64, r: f64) -> bool {
    if x < lo || x > hi || y < lo || y > hi {
        return false;
    }
    let r = r.min((hi - lo) / 2.0);
    let cx = x.clamp(lo + r, hi - r);
    let cy = y.clamp(lo + r, hi - r);
    (x - cx).powi(2) + (y - cy).powi(2) <= r * r
}

/// A canvas point taken back through
/// `translate(dx dy) rotate(deg c c) scale(s)` into the element's own
/// coordinates. y points down; the rotation is clockwise on screen.
fn unplace(x: f64, y: f64, dx: f64, dy: f64, deg: f64, s: f64, c: f64) -> (f64, f64) {
    let (sin, cos) = (-deg).to_radians().sin_cos();
    let (x, y) = (x - dx - c, y - dy - c);
    ((c + x * cos - y * sin) / s, (c + x * sin + y * cos) / s)
}

/* ═══════════ marble's two polygons ═══════════ */

/// Marble's two paths, **character for character** the brush's
/// (`apps/gui/src/components/Avatar.tsx` `VEIN_PATH`; a test reads that
/// file). Both are straight segments only, so a polygon each.
pub(crate) const VEIN_PATH: [(Vein, &str); 2] = [
    (
        Vein::Shard,
        "M32.414 59.35L50.376 70.5H72.5v-71H33.728L26.5 13.381l19.057 27.08L32.414 59.35z",
    ),
    (
        Vein::Sweep,
        "M22.216 24L0 46.75l14.108 38.129L78 86l-3.081-59.276-22.378 4.005 12.972 20.186-23.35 27.395L22.215 24z",
    ),
];

fn vein_polygon(v: Vein) -> Vec<(f64, f64)> {
    let d = VEIN_PATH
        .iter()
        .find(|(k, _)| *k == v)
        .map(|(_, d)| *d)
        .unwrap_or("");
    polygon(d)
}

/// A path of `M L H V Z` (either case) as its vertices. Anything else is
/// not a vein path and ends the polygon where it stands.
pub(crate) fn polygon(d: &str) -> Vec<(f64, f64)> {
    let mut out = vec![];
    let (mut x, mut y) = (0.0, 0.0);
    let mut cmd = 'M';
    let mut nums: Vec<f64> = vec![];
    let flush =
        |cmd: char, nums: &mut Vec<f64>, x: &mut f64, y: &mut f64, out: &mut Vec<(f64, f64)>| {
            let rel = cmd.is_ascii_lowercase();
            match cmd.to_ascii_uppercase() {
                'M' | 'L' => {
                    for p in nums.chunks(2).filter(|p| p.len() == 2) {
                        if rel {
                            *x += p[0];
                            *y += p[1];
                        } else {
                            *x = p[0];
                            *y = p[1];
                        }
                        out.push((*x, *y));
                    }
                }
                'H' => {
                    for &n in nums.iter() {
                        *x = if rel { *x + n } else { n };
                        out.push((*x, *y));
                    }
                }
                'V' => {
                    for &n in nums.iter() {
                        *y = if rel { *y + n } else { n };
                        out.push((*x, *y));
                    }
                }
                _ => {}
            }
            nums.clear();
        };
    let bytes = d.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i] as char;
        if b.is_ascii_alphabetic() {
            flush(cmd, &mut nums, &mut x, &mut y, &mut out);
            if !"MmLlHhVvZz".contains(b) {
                break;
            }
            cmd = b;
            i += 1;
        } else if b == '-' || b == '.' || b.is_ascii_digit() {
            let start = i;
            i += 1;
            let mut dot = b == '.';
            while i < bytes.len() {
                let n = bytes[i] as char;
                if n.is_ascii_digit() || (n == '.' && !dot) {
                    dot |= n == '.';
                    i += 1;
                } else {
                    break;
                }
            }
            nums.push(d[start..i].parse().unwrap_or(0.0));
        } else {
            i += 1;
        }
    }
    flush(cmd, &mut nums, &mut x, &mut y, &mut out);
    // `z` closes on the start; a last point equal to the first is the
    // same polygon either way.
    out
}

/* ═══════════ a Gaussian-blurred polygon ═══════════ */

/// The polygon's coverage (nonzero rule), cut to the square `lo..hi`
/// (the filter region), blurred by `sigma`, read at each of `at`.
///
/// Exact across each scanline (the integral of a Gaussian over an
/// interval is two `erf`s), a midpoint sum down rows one unit apart,
/// evaluated on a grid [`FIELD_STEP`] units square around the points and
/// read off it bilinearly. The field is a blur a σ of seven units wide,
/// so at that spacing it is flat: a test holds the grid to the exact
/// integral.
fn blurred(poly: &[(f64, f64)], sigma: f64, lo: f64, hi: f64, at: &[(f64, f64)]) -> Vec<f64> {
    if at.is_empty() || poly.len() < 3 {
        return vec![0.0; at.len()];
    }
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for &(x, y) in at {
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x);
        y1 = y1.max(y);
    }
    let g = FIELD_STEP;
    let (gx0, gy0) = (x0 - g, y0 - g);
    let nx = ((x1 - gx0) / g).ceil() as usize + 2;
    let ny = ((y1 - gy0) / g).ceil() as usize + 2;
    let cols_x: Vec<f64> = (0..nx).map(|i| gx0 + i as f64 * g).collect();
    // Rows of the polygon, one unit apart, as far as 4σ past the grid.
    let h = 1.0;
    let reach = 4.0 * sigma;
    let r0 = (gy0 - reach).max(lo);
    let r1 = (gy0 + ny as f64 * g + reach).min(hi);
    let mut rows: Vec<(f64, Vec<f64>)> = vec![];
    let mut v = r0 + h / 2.0;
    while v < r1 {
        let spans = scan(poly, v, lo, hi);
        if !spans.is_empty() {
            let cols = cols_x
                .iter()
                .map(|&x| {
                    spans
                        .iter()
                        .map(|&(a, b)| phi((b - x) / sigma) - phi((a - x) / sigma))
                        .sum()
                })
                .collect();
            rows.push((v, cols));
        }
        v += h;
    }
    let norm = h / (sigma * (2.0 * std::f64::consts::PI).sqrt());
    let mut grid = vec![0.0; nx * ny];
    for (j, line) in grid.chunks_mut(nx).enumerate() {
        let y = gy0 + j as f64 * g;
        for (v, cols) in &rows {
            let t = (y - v) / sigma;
            if t.abs() > 4.0 {
                continue;
            }
            let w = norm * (-0.5 * t * t).exp();
            for (cell, c) in line.iter_mut().zip(cols) {
                *cell += w * c;
            }
        }
    }
    at.iter()
        .map(|&(x, y)| {
            let (fx, fy) = ((x - gx0) / g, (y - gy0) / g);
            let (i, j) = (
                (fx.floor() as usize).min(nx - 2),
                (fy.floor() as usize).min(ny - 2),
            );
            let (tx, ty) = (fx - i as f64, fy - j as f64);
            let at = |i: usize, j: usize| grid[j * nx + i];
            let top = at(i, j) * (1.0 - tx) + at(i + 1, j) * tx;
            let bot = at(i, j + 1) * (1.0 - tx) + at(i + 1, j + 1) * tx;
            (top * (1.0 - ty) + bot * ty).clamp(0.0, 1.0)
        })
        .collect()
}

/// Where the line `y = v` is inside the polygon (nonzero winding), cut
/// to `lo..hi`.
fn scan(poly: &[(f64, f64)], v: f64, lo: f64, hi: f64) -> Vec<(f64, f64)> {
    let mut hits: Vec<(f64, i32)> = vec![];
    for k in 0..poly.len() {
        let (a, b) = (poly[k], poly[(k + 1) % poly.len()]);
        if a.1 == b.1 {
            continue;
        }
        let (top, bottom) = if a.1 < b.1 { (a, b) } else { (b, a) };
        if v >= top.1 && v < bottom.1 {
            let x = a.0 + (v - a.1) * (b.0 - a.0) / (b.1 - a.1);
            hits.push((x, if a.1 < b.1 { 1 } else { -1 }));
        }
    }
    hits.sort_by(|p, q| p.0.total_cmp(&q.0));
    let mut out = vec![];
    let mut wind = 0;
    let mut from = 0.0;
    for (x, d) in hits {
        let was = wind;
        wind += d;
        if was == 0 && wind != 0 {
            from = x;
        } else if was != 0 && wind == 0 {
            let (a, b) = (from.max(lo), x.min(hi));
            if b > a {
                out.push((a, b));
            }
        }
    }
    out
}

/// The standard normal's cumulative distribution.
fn phi(z: f64) -> f64 {
    0.5 * (1.0 + erf(z / std::f64::consts::SQRT_2))
}

/// `erf`, to 1.2e-7 (Numerical Recipes' `erfc` by Chebyshev fit).
fn erf(x: f64) -> f64 {
    let z = x.abs();
    let t = 1.0 / (1.0 + 0.5 * z);
    let r = t
        * (-z * z - 1.265_512_23
            + t * (1.000_023_68
                + t * (0.374_091_96
                    + t * (0.096_784_18
                        + t * (-0.186_288_06
                            + t * (0.278_868_07
                                + t * (-1.135_203_98
                                    + t * (1.488_515_87
                                        + t * (-0.822_152_23 + t * 0.170_872_77)))))))))
            .exp();
    if x >= 0.0 { 1.0 - r } else { r - 1.0 }
}

#[cfg(test)]
mod tests {
    use super::super::{AvatarSeed, AvatarStyle, FaceShape, PRESETS, Variant, avatar};
    use super::*;

    /// A spread of real ids — a seed is a device id, and nothing about
    /// these means anything except that they differ.
    fn seeds(n: u64) -> Vec<AvatarSeed> {
        (0..n)
            .map(|i| {
                let mut state = 0x2545_f491_4f6c_dd1du64.wrapping_mul(i + 7);
                let mut hex = String::new();
                while hex.len() < 64 {
                    state ^= state << 13;
                    state ^= state >> 7;
                    state ^= state << 17;
                    hex.push_str(&format!("{state:016x}"));
                }
                AvatarSeed::from_id_hex(&hex[..64]).unwrap()
            })
            .collect()
    }

    /// The worst the ink reads anywhere under the mark, on this face, with
    /// this pair.
    fn worst(px: &[Rgb], m: MarkOnFace) -> f64 {
        let ink = lum(rgb(m.ink.hex()));
        let grey = rgb(m.ink.veil_hex())[0];
        px.iter()
            .map(|&p| ratio(ink, lum(veiled(p, grey, m.veil))))
            .fold(f64::MAX, f64::min)
    }

    /// **The rule, on every factory palette × variant × crop and a spread
    /// of machines, on this raster**: the ink is the one most of the
    /// region wants; the veil is a whole step, its worst pixel reads 3.2
    /// (or, where black would have to pass 0.42, the light ink stops at
    /// 0.42 and still reads 3:1), and one step less would not have done.
    #[test]
    fn every_factory_face_wears_the_least_veil_that_reads() {
        let (mut faces, mut bare, mut dark, mut most) = (0, 0, 0, 0.0f64);
        for p in PRESETS.iter() {
            for variant in Variant::ALL {
                for shape in FaceShape::ALL {
                    let style = AvatarStyle {
                        palette: p.palette(),
                        variant,
                        shape,
                    };
                    for seed in seeds(12) {
                        let face = avatar(&seed, &style);
                        let m = face.mark;
                        let px = region(&face);
                        let at = format!("{} {} {} {:?}", p.id, variant.key(), shape.key(), m);
                        assert!(!px.is_empty(), "{at}: nothing under the mark");
                        assert_eq!(choose(&px), m, "{at}: the record is not the rule's");
                        assert!(
                            (m.veil / VEIL_STEP - (m.veil / VEIL_STEP).round()).abs() < 1e-9,
                            "{at}: not a whole step"
                        );
                        let w = worst(&px, m);
                        if m.ink == MarkInk::Light {
                            assert!(m.veil <= MAX_BLACK_VEIL + 1e-9, "{at}: black past 0.42");
                        }
                        if m.ink == MarkInk::Light && m.veil >= MAX_BLACK_VEIL - 1e-9 {
                            assert!(w >= 3.0, "{at}: capped at 0.42 and reads {w:.2}");
                        } else {
                            assert!(w >= MARK_TARGET, "{at}: the worst pixel reads {w:.2}");
                        }
                        if m.veil > 0.0 {
                            let less = MarkOnFace {
                                veil: alpha((m.veil / VEIL_STEP).round() as usize - 1),
                                ..m
                            };
                            assert!(
                                worst(&px, less) < MARK_TARGET,
                                "{at}: one step less already read"
                            );
                        }
                        faces += 1;
                        bare += usize::from(m.veil == 0.0);
                        dark += usize::from(m.ink == MarkInk::Dark);
                        most = most.max(m.veil);
                    }
                }
            }
        }
        println!("{faces} faces: {bare} bare, {dark} dark ink, the most veil {most}");
        // The point of the rule: most faces wear no veil at all.
        assert!(bare * 10 >= faces * 7, "{bare} of {faces} faces bare");
    }

    #[test]
    fn the_mark_is_the_face_s_alone() {
        for seed in seeds(4) {
            let style = AvatarStyle::default();
            let a = avatar(&seed, &style);
            assert_eq!(a.mark, avatar(&seed, &style).mark);
            assert_eq!(a.mark, derive(&a));
        }
    }

    /// The grid the blurred field is read off holds to the integral done
    /// the slow way, on the real vein at a real place.
    #[test]
    fn the_field_holds_to_the_integral() {
        let poly = vein_polygon(Vein::Sweep);
        let (lo, hi) = (-7.0 * BLUR_MARGIN, 80.0 + 7.0 * BLUR_MARGIN);
        let at: Vec<(f64, f64)> = (0..60)
            .map(|k| (10.0 + (k % 10) as f64 * 6.1, 12.0 + (k / 10) as f64 * 9.3))
            .collect();
        let got = blurred(&poly, 7.0, lo, hi, &at);
        for (&(x, y), g) in at.iter().zip(got) {
            let mut sum = 0.0;
            let mut v = lo + 0.05;
            while v < hi {
                let t = (y - v) / 7.0;
                let w = 0.1 * (-0.5 * t * t).exp() / (7.0 * (2.0 * std::f64::consts::PI).sqrt());
                for (a, b) in scan(&poly, v, lo, hi) {
                    sum += w * (phi((b - x) / 7.0) - phi((a - x) / 7.0));
                }
                v += 0.1;
            }
            assert!((g - sum).abs() < 0.004, "at {x},{y}: {g} against {sum}");
        }
    }

    #[test]
    fn the_vein_paths_are_the_brush_s_character_for_character() {
        let brush = include_str!("../../../../apps/gui/src/components/Avatar.tsx");
        for (v, d) in VEIN_PATH {
            assert!(
                brush.contains(&format!("\"{d}\"")),
                "{v:?}'s path is not the brush's VEIN_PATH"
            );
        }
    }

    #[test]
    fn the_paths_parse_to_their_vertices() {
        let shard = polygon(VEIN_PATH[0].1);
        assert_eq!(shard.len(), 8);
        assert_eq!(shard[0], (32.414, 59.35));
        assert_eq!(shard[2], (72.5, 70.5)); // H72.5
        assert_eq!(shard[3], (72.5, -0.5)); // v-71
        assert!((shard[6].0 - 45.557).abs() < 1e-9 && (shard[6].1 - 40.461).abs() < 1e-9); // l19.057 27.08
        let sweep = polygon(VEIN_PATH[1].1);
        assert_eq!(sweep.len(), 9);
        // `l-3.081-59.276-22.378 4.005 …`: one command, three pairs, the
        // signs splitting the numbers.
        assert!((sweep[4].0 - 74.919).abs() < 1e-9 && (sweep[4].1 - 26.724).abs() < 1e-9);
        assert!((sweep[5].0 - 52.541).abs() < 1e-9 && (sweep[5].1 - 30.729).abs() < 1e-9);
        assert_eq!(sweep[8], (22.215, 24.0));
    }

    #[test]
    fn erf_is_erf() {
        assert!(erf(0.0).abs() < 1e-7);
        assert!((erf(1.0) - 0.842_700_79).abs() < 2e-7);
        assert!((erf(-2.0) + 0.995_322_27).abs() < 2e-7);
    }

    /// A half-plane blurred is ½ on its edge and the normal CDF off it;
    /// a polygon far bigger than the blur is whole at its middle.
    #[test]
    fn a_blur_is_a_gaussian() {
        let half = [(0.0, -500.0), (500.0, -500.0), (500.0, 500.0), (0.0, 500.0)];
        let got = blurred(
            &half,
            7.0,
            -1000.0,
            1000.0,
            &[(0.0, 0.0), (7.0, 3.0), (-7.0, -2.0), (14.0, 0.0)],
        );
        assert!((got[0] - 0.5).abs() < 1e-3, "{got:?}");
        assert!((got[1] - phi(1.0)).abs() < 1e-3, "{got:?}");
        assert!((got[2] - phi(-1.0)).abs() < 1e-3, "{got:?}");
        assert!((got[3] - phi(2.0)).abs() < 3e-3, "{got:?}");
        // …and the filter region cuts the source before the blur.
        let cut = blurred(&half, 7.0, -1000.0, 0.0, &[(0.0, 0.0)]);
        assert!(cut[0] < 1e-6, "{cut:?}");
    }

    #[test]
    fn the_rule_on_plain_grounds() {
        let white = choose(&[[255.0; 3]; 10]);
        assert_eq!(
            white,
            MarkOnFace {
                ink: MarkInk::Dark,
                veil: 0.0
            }
        );
        let black = choose(&[[0.0; 3]; 10]);
        assert_eq!(
            black,
            MarkOnFace {
                ink: MarkInk::Light,
                veil: 0.0
            }
        );
        // Mostly black with one white pixel: the light ink most of it
        // wants, and black enough to bring the white one down — capped
        // at 0.42, where white reads 3.04.
        let mut mixed = vec![[0.0; 3]; 9];
        mixed.push([255.0; 3]);
        assert_eq!(
            choose(&mixed),
            MarkOnFace {
                ink: MarkInk::Light,
                veil: MAX_BLACK_VEIL
            }
        );
        // Any one flat colour reads 3.2 against one ink or the other
        // bare; a face that mixes them is what gets veiled — here white
        // mostly, so the dark ink, and white over the black pixels.
        let mut split = vec![[255.0; 3]; 6];
        split.extend([[0.0; 3]; 4]);
        let got = choose(&split);
        assert_eq!(got.ink, MarkInk::Dark);
        assert!(got.veil > 0.3 && got.veil <= 0.42, "{got:?}");
    }

    #[test]
    fn the_veil_steps_are_two_decimals() {
        assert_eq!(alpha(3), 0.06);
        assert_eq!(alpha(21), 0.42);
        assert_eq!((MAX_BLACK_VEIL / VEIL_STEP).round() as usize, 21);
    }
}
