// Ported from mandala's icon system (tokens repo, Icons.tsx + glyph.ts):
// 24-canvas stroke glyphs, one solid piece per form (data-body) with the
// rest in line — two densities, not two colors. Overlaps are cut with an
// equal-width mask gap, never broken paths. Paths are copied verbatim;
// changing one here forks the family.
//
// **One family, no exceptions.** Every mark any component draws comes
// from this file — including the ones inside vendored shadcn components,
// whose stock marks are swapped out on the way in. Two families differ in
// stroke ratio, so mixing them on one screen reads the way two typefaces
// in one paragraph read. `scripts/check-icons.mjs` fails the build on an
// import from the foreign family (it names the package), which is what
// catches a future `shadcn add` quietly bringing its own back.

import { useId, type ReactNode } from "react";

import { cn } from "@/lib/utils";

// **One line weight, as a share of the box** (2026-10-02, the owner:
// 「确保所有icon的大小一致」, and a ＋ beside 会话 in one capsule drew
// 1.69 to its 1.05). It was two ratios — 1.4 on the 24 canvas, 1.5 on the
// 16 — and a box holding one of each drew two weights. Now every stroked
// glyph draws `line` of whatever box it is in: 1.44 at a 20 box, between
// the two it replaces. The tick keeps its heavier density, `bold`, by the
// same ratio as before (1.9 to 1.5): it is read as an answer, not a
// picture.
export const STROKE = { line: 0.072, bold: 0.091 } as const;

// The family's cut: the gap a front form's line leaves either side where
// it crosses what is behind it, as a share of the box (0.8 of 24 and 0.55
// of 16 both came to it). The cut shape is the front form's outline at
// the line weight plus a gap each side.
const GAP = 0.034;

/**
 * **One live area for every mark** (2026-10-02, the owner: 「怎么不同的
 * icon比例不一呀 vscode明显看起来更圆更大」, then 「我希望做饱满一点,
 * 不要浪费空间」). A glyph's larger ink dimension fills the same share of
 * whatever box it is drawn in — 0.90, with two optical classes either
 * side: 0.88 for square masses (a rack, a plate, a page, a keyboard),
 * which read bigger than their extent, and 0.92 for round and diagonal
 * ones (a ring, a chevron, a cross), which read smaller. The ink is
 * centred on the box. So one box gives one painted size, and a row of
 * glyphs is one box size — never a smaller box for a mark that happens
 * to fill its canvas.
 *
 * **Done with the viewBox, not by redrawing.** The paths stay where the
 * family drew them on its 24 or 16 canvas; `fit` names the square of that
 * canvas the ink sits in (measured by rasterising each glyph at 512 and
 * scanning its ink — `getBBox` ignores strokes). Cropping the canvas
 * scales the drawing up, so every stroke width and cut is set from the
 * crop's side: the line is a share of the **box** (`STROKE`), and does not
 * fatten with the drawing.
 */
type Fit = { x: number; y: number; side: number };
const fit = (x: number, y: number, side: number): Fit => ({ x, y, side });
const viewBox = (f: Fit) => `${f.x} ${f.y} ${f.side} ${f.side}`;
/** The line, in a glyph's own units. */
const lineOf = (f: Fit, w: number = STROKE.line) => w * f.side;
/** The cut's width, in a glyph's own units; `extra` widens it (a share). */
const cutOf = (f: Fit, extra = 0) => (STROKE.line + 2 * GAP + extra) * f.side;

/**
 * **Bare line glyphs fit a smaller share** (2026-10-02, the owner:
 * 「"+"等简单符号，都有点太大了。但别的icon不会」). Glyphs are matched by
 * weight, not by extent: a figure that is only a few strokes reaching
 * out to its ends shows nothing but that reach, so at the reach of a
 * glyph with a body it reads a size bigger. The criterion is measured —
 * ink coverage, painted ink over the ink's bounding box at the common
 * line, rendered at 96: under 0.21 with nothing enclosed or filled is
 * **bare**, 0.72 of the box (＋ 0.15, the tick 0.18, the chevrons and
 * back / forward 0.20, × 0.21); an open figure just over it, or a bare
 * one that sits among filled marks, is **light**, 0.80 (search 0.21 with
 * its ring, reload 0.22, the terminal's `>_` 0.15 among the agents'
 * filled marks). Everything else keeps its 0.88 / 0.90 / 0.92 (link 0.26
 * and up). The line stays the common share of the box (`STROKE`): the
 * figure gets shorter, not thinner.
 */
const BARE = 0.72;
const LIGHT = 0.8;
const ROUND = 0.92;
const PLAIN = 0.9;
/** A measured live area refitted from one share of the box to another,
    about the same centre. */
const reshare = (f: Fit, from: number, to: number): Fit => {
  const side = (f.side * from) / to;
  return fit(f.x - (side - f.side) / 2, f.y - (side - f.side) / 2, side);
};

/** Each glyph's live area on its own canvas, measured (`fit`). Unmarked
    is the 0.90 share; `square` is 0.88 and `round` (round or diagonal)
    is 0.92; `reshare`d ones are bare or light (above). */
const LIVE = {
  sessions: fit(1.7, 1.81, 20.6),
  devices: fit(1.45, 1.45, 21.11), // square
  files: fit(1.45, 1.49, 21.11), // square
  browser: fit(2.93, 2.93, 18.15), // round
  more: fit(2.92, 2.92, 18.16),
  gear: fit(2.92, 2.92, 18.17), // round
  appearance: fit(2.92, 2.92, 18.17), // round
  shield: fit(1.19, 1.33, 21.62),
  network: fit(1.44, 0.74, 21.12), // round
  pulse: fit(1.34, 1.34, 21.32), // square
  branch: fit(0.55, 0.55, 14.89),
  back: reshare(fit(1.93, 2.33, 11.33), ROUND, BARE),
  forward: reshare(fit(2.73, 2.33, 11.33), ROUND, BARE),
  reload: reshare(fit(1.78, 1.41, 12.44), ROUND, LIGHT),
  search: reshare(fit(1.84, 1.84, 12.63), ROUND, LIGHT),
  grip: fit(1.82, 1.82, 12.37), // square
  plus: reshare(fit(1.96, 1.96, 12.08), PLAIN, BARE),
  filter: fit(1, 1, 14), // square
  check: reshare(fit(1.96, 1.97, 12.07), ROUND, BARE),
  close: reshare(fit(3.28, 3.28, 9.44), ROUND, BARE),
  chevronUp: reshare(fit(3.28, 3.28, 9.44), ROUND, BARE),
  chevronDown: reshare(fit(3.28, 3.28, 9.44), ROUND, BARE),
  chevronRight: reshare(fit(3.28, 3.28, 9.44), ROUND, BARE),
  attach: fit(1.19, 0.98, 13.22), // round
  // Fitted on the larger of its two states: upright, pinned, it is taller.
  pin: fit(2.87, 1.84, 11.29), // round
  terminal: reshare(fit(2.45, 2.56, 11.1), PLAIN, LIGHT),
  chat: fit(1.6, 2.3, 12.8),
  keyboard: fit(1.17, 1.17, 13.66), // square
  file: fit(1.07, 1.07, 13.87), // square
  link: fit(2.14, 2.14, 11.73), // round
  // The cursor key's (2026-10-03): ink 11 across, centred on the 16
  // canvas; a few strokes reaching out to their ends, so bare.
  move: reshare(fit(1.36, 1.36, 13.29), PLAIN, BARE),
  arrow: reshare(fit(1.36, 1.36, 13.29), PLAIN, BARE),
} as const;

/**
 * **Pointed marks lean toward their point** (2026-10-02, the owner:
 * 「返回icon在圆形按钮下怎么怪怪的，感觉靠右侧了」). A chevron centred by
 * its ink box is not centred to the eye: its point is a single dot and its
 * two arm tips are a wide edge, so a `<` reads as pushed toward its tips.
 * These few are moved 0.09 of their box toward the point — by moving the
 * fitted viewBox, so the path and the size are untouched — and every
 * other mark stays centred by its ink. `IconForward` is `IconBack`'s exact
 * mirror on the 16 canvas. A seat that stacks two of them as one pair
 * (the popup button's ⌃⌄) asks for them `centred`, or they would part.
 */
const LEAN = 0.09;
const lean = (f: Fit, dx: number, dy: number): Fit => fit(f.x - dx * LEAN * f.side, f.y - dy * LEAN * f.side, f.side);
const mirror = (f: Fit): Fit => fit(16 - f.x - f.side, f.y, f.side);
const BACK = lean(LIVE.back, -1, 0);
const POINTED = {
  back: BACK,
  forward: mirror(BACK),
  chevronUp: lean(LIVE.chevronUp, 0, -1),
  chevronDown: lean(LIVE.chevronDown, 0, 1),
  chevronRight: lean(LIVE.chevronRight, 1, 0),
} as const;

type IconProps = { className?: string };

function RailGlyph({
  className,
  live,
  cut,
  children,
}: IconProps & {
  live: Fit;
  cut?: ReactNode;
  children: ReactNode | ((cutId: string) => ReactNode);
}) {
  const uid = useId().replace(/:/g, "");
  const cutId = `railcut-${uid}`;
  return (
    <svg
      className={cn("size-icon-rail", className)}
      viewBox={viewBox(live)}
      fill="none"
      stroke="currentColor"
      strokeWidth={lineOf(live)}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {cut && (
        <mask id={cutId}>
          <rect x={live.x} y={live.y} width={live.side} height={live.side} fill="white" />
          {cut}
        </mask>
      )}
      {typeof children === "function" ? children(cutId) : children}
    </svg>
  );
}

/** Front bubble (upper left), tail at the lower left — the solid one. */
const BUBBLE_FRONT =
  "M6.3 3.8H12.4A2.8 2.8 0 0 1 15.2 6.6V9.8A2.8 2.8 0 0 1 12.4 12.6H9L6.4 15.2V12.6A2.8 2.8 0 0 1 3.5 9.8V6.6A2.8 2.8 0 0 1 6.3 3.8Z";

/** Back bubble (lower right), a full loop — the overlap is masked out. */
const BUBBLE_BACK =
  "M11.6 9H17.7A2.8 2.8 0 0 1 20.5 11.8V15A2.8 2.8 0 0 1 17.7 17.8V20.4L15.1 17.8H11.6A2.8 2.8 0 0 1 8.8 15V11.8A2.8 2.8 0 0 1 11.6 9Z";

/** Sessions = two offset bubbles: a conversation, not a message. */
export function IconSessions({ className }: IconProps) {
  return (
    <RailGlyph
      className={className}
      live={LIVE.sessions}
      cut={<path d={BUBBLE_FRONT} fill="black" stroke="black" strokeWidth={cutOf(LIVE.sessions)} />}
    >
      {(cutId) => (
        <>
          <path mask={`url(#${cutId})`} d={BUBBLE_BACK} />
          <path data-body d={BUBBLE_FRONT} />
        </>
      )}
    </RailGlyph>
  );
}

/** 额度 = the token stack the user handed over (2026-09-02: 「token icon
    用这个……把文字去掉」— noun-project 957015, Alina Oleynik). Their art
    verbatim, the two credit `<text>` lines removed, filled in
    `currentColor` so it themes and lights up selected like every rail
    glyph. **Not redrawn in the stroke family**: it is a filled pictorial
    mark, the way the mesh door's machine face is — using the file they
    chose beats matching a house style they did not ask me to keep. */
export function IconQuota({ className }: IconProps) {
  return (
    <svg
      className={cn("size-icon-rail", className)}
      // Its own 100 canvas cropped to the one live area (`LIVE`, round).
      viewBox="9.63 11.53 77.04 77.04"
      fill="currentColor"
      aria-hidden="true"
    >
      <path d="M62.356,39.278c0.007-0.184,0.018-0.368,0.018-0.553c0-0.053-0.007-0.104-0.008-0.157v-5.475c0.001-0.053,0.008-0.104,0.008-0.157c0-10.127-10.449-18.336-23.338-18.336s-23.338,8.209-23.338,18.336c0,0.071,0.01,0.14,0.011,0.211v5.368c-0.001,0.071-0.011,0.14-0.011,0.211c0,5.26,2.823,9.999,7.339,13.343c-1.438,3.03-2.246,6.418-2.246,9.995c0,12.889,10.449,23.338,23.338,23.338c1.119,0,2.218-0.084,3.295-0.236c1.077,0.152,2.176,0.236,3.295,0.236s2.218-0.084,3.295-0.236c1.077,0.152,2.176,0.236,3.295,0.236c12.889,0,23.338-10.449,23.338-23.338C80.647,50.907,72.818,41.585,62.356,39.278z M19.122,32.936c0-8.641,8.916-15.646,19.914-15.646S58.95,24.295,58.95,32.936s-8.916,15.646-19.914,15.646S19.122,41.577,19.122,32.936z M44.129,81.977c-10.998,0-19.914-8.916-19.914-19.914c0-2.904,0.636-5.656,1.754-8.144c0.916,0.487,1.88,0.923,2.882,1.306c-0.937,2.089-1.47,4.399-1.47,6.837c0,9.249,7.498,16.747,16.747,16.747s16.747-7.498,16.747-16.747c0-4.324-1.653-8.253-4.343-11.224c0.803-0.716,1.544-1.475,2.199-2.283c3.286,3.551,5.311,8.287,5.311,13.507C64.043,73.061,55.127,81.977,44.129,81.977z M58.651,80.325c5.37-4.276,8.816-10.865,8.816-18.262c0-6.423-2.6-12.235-6.8-16.454c0.104-0.201,0.207-0.401,0.302-0.605c5.787,3.483,9.665,9.813,9.665,17.059C70.633,70.24,65.7,77.259,58.651,80.325z M65.241,80.325c5.37-4.276,8.816-10.865,8.816-18.262S70.611,48.076,65.241,43.8c7.049,3.066,11.982,10.085,11.982,18.262C77.223,70.24,72.29,77.259,65.241,80.325z" />
      <ellipse cx="39.036" cy="32.936" rx="16.747" ry="13.158" />
    </svg>
  );
}

/** Devices = a rack: two drawers, each with a punched-through lamp. */
export function IconDevices({ className }: IconProps) {
  return (
    <RailGlyph
      className={className}
      live={LIVE.devices}
      cut={
        <>
          <circle cx="7.2" cy="7.8" r="1.15" fill="black" />
          <circle cx="7.2" cy="16.2" r="1.15" fill="black" />
        </>
      }
    >
      {(cutId) => (
        <g mask={`url(#${cutId})`}>
          <rect data-body x="3.5" y="4.6" width="17" height="6.4" rx="2.6" />
          <rect data-body x="3.5" y="13" width="17" height="6.4" rx="2.6" />
        </g>
      )}
    </RailGlyph>
  );
}

/**
 * Files = a folder in two pieces, the front one solid. Copied verbatim
 * from mandala's `IconRailFiles`, including the two things about it that
 * look like mistakes next to its neighbours and are not: its bounding box
 * is one notch shorter (4.6–19.5 rather than 17 high) because a wide form
 * reads bigger than a tall one at equal area, and its corner radii
 * (1.1–1.7) never joined the 2.6/2.8 family the other glyphs share.
 */
export function IconFiles({ className }: IconProps) {
  return (
    <RailGlyph className={className} live={LIVE.files}>
      {/* Back piece: an open path, its right and bottom left to the front. */}
      <path d="M3.5 16.9V6.3a1.7 1.7 0 0 1 1.7-1.7h4a1.2 1.2 0 0 1 1 .5l1.2 1.8h6.2a1.7 1.7 0 0 1 1.7 1.7v1.6" />
      {/* Front piece: a trapezoid leaning up to the right. */}
      <path
        data-body
        d="M6.6 10.2h12.8a1.1 1.1 0 0 1 1.1 1.4l-1.6 6.6a1.7 1.7 0 0 1-1.7 1.3H5.2a1.1 1.1 0 0 1-1.1-1.4l1.4-6.6a1.4 1.4 0 0 1 1.1-1.3z"
      />
    </RailGlyph>
  );
}

/**
 * Browser = a sphere with an orbit cutting across it, verbatim from
 * mandala's `IconRailBrowser`. A level band would read as an equator (a
 * globe); the tilt is what makes it an orbit, and this landing is about
 * going out through someone else's network, not about the earth.
 *
 * The numbers are computed, not chosen: the band's half-width is
 * `rx·cos20° + ry·sin20° = 8.1×0.940 + 2.9×0.342 = 8.60`, which puts its
 * ends at 3.4 / 20.6 and makes the band — not the sphere — the thing
 * setting this glyph's bounding box. The sphere sits at r = 7.2 so the
 * band genuinely reaches past it; a band contained inside the sphere is
 * an equator again.
 */
export function IconBrowser({ className }: IconProps) {
  return (
    <RailGlyph
      className={className}
      live={LIVE.browser}
      cut={
        <ellipse
          cx="12"
          cy="12"
          rx="8.1"
          ry="2.9"
          transform="rotate(-20 12 12)"
          fill="none"
          stroke="black"
          // 0.2 of 24 wider than the family's cut, its one exception: the band
          // crosses the sphere at a slant, and an equal-width cut leaves
          // a visually narrower gap along a slanted edge than a square
          // one. This is the value that makes the gap look the same as
          // the other glyphs', not a different rule.
          strokeWidth={cutOf(LIVE.browser, 0.2 / 24)}
        />
      }
    >
      {(cutId) => (
        <>
          <circle data-body mask={`url(#${cutId})`} cx="12" cy="12" r="7.2" />
          {/* The orbit in front, drawn whole — this glyph's line half. */}
          <ellipse cx="12" cy="12" rx="8.1" ry="2.9" transform="rotate(-20 12 12)" />
        </>
      )}
    </RailGlyph>
  );
}

/** More = three dots; lighter than the formed glyphs on purpose. */
export function IconMore({ className }: IconProps) {
  return (
    <RailGlyph className={className} live={LIVE.more}>
      <circle cx="5.4" cy="12" r="1.55" fill="currentColor" stroke="none" />
      <circle cx="12" cy="12" r="1.55" fill="currentColor" stroke="none" />
      <circle cx="18.6" cy="12" r="1.55" fill="currentColor" stroke="none" />
    </RailGlyph>
  );
}

/**
 * The three doors behind 「…」 (docs/MORE.md), as 24-canvas rail glyphs.
 *
 * They are drawn at rail size although they only ever appear in a menu:
 * the「…」menu on the narrow bar already puts a landing's own rail glyph
 * beside its name, and three marks at a second size in one list would be
 * the「two typefaces in one paragraph」this file's head is about. The
 * menu sizes them down from the outside.
 *
 * None of the three carries a `data-body` piece. That rule paints the
 * *open* landing solid, and none of these is a landing — a page behind
 * 「…」 is opened and closed, never「where you are」 in the rail's sense.
 */

/** 设置 = a gear. Eight teeth as one dashed ring, so the tooth pitch is
    the dash pattern rather than eight hand-placed paths that can drift. */
export function IconGear({ className }: IconProps) {
  return (
    <RailGlyph className={className} live={LIVE.gear}>
      {/* The dash pattern CLOSES. At r 7.7 the circumference is 48.38,
          so eight teeth are 3.02 on and 3.02 off, turned half a tooth
          so one sits at twelve o'clock. The previous pattern (1.9 /
          2.85 on r 8.6) came to 11.4 teeth — the seam where it failed
          to close is what read as unfinished. The teeth overlap the rim
          by 0.2 so the two merge into one cog profile. */}
      <circle cx="12" cy="12" r="7.7" strokeDasharray="3.02 3.02" transform="rotate(-101.24 12 12)" />
      <circle cx="12" cy="12" r="6.5" />
      <circle data-body cx="12" cy="12" r="2.8" />
    </RailGlyph>
  );
}

/** 外观 = light and dark: a ring, and the half of it that is dark as the
    family's one solid piece (2026-09-27, for the 「…」 list — the one
    page there that had no glyph). The half disc sits inside the ring
    with the family's 0.8 gap on its round side (r 7.7 − 0.7 − 0.8 =
    6.2), so the two read as a frame and a fill rather than one blot. */
export function IconAppearance({ className }: IconProps) {
  return (
    <RailGlyph className={className} live={LIVE.appearance}>
      <circle cx="12" cy="12" r="7.7" />
      <path data-body d="M12 5.8a6.2 6.2 0 0 1 0 12.4Z" />
    </RailGlyph>
  );
}

/** 安全 = a shield. The plate a lock hangs on everywhere else, and the
    one mark of the three that says「kept out」 rather than「looked at」. */
export function IconShield({ className }: IconProps) {
  return (
    <RailGlyph
      className={className}
      live={LIVE.shield}
      cut={
        <>
          <circle cx="12" cy="10.5" r="1.5" fill="black" />
          <path d="M12 11.4v3.2" stroke="black" strokeWidth="2.1" />
        </>
      }
    >
      {(cutId) => (
        // The keyhole is PUNCHED THROUGH the plate, the way the rack's
        // lamps are. It was a 3.4-long stub floating on an empty
        // outline, which is the shape of a drawing that stopped early.
        <path
          data-body
          mask={`url(#${cutId})`}
          d="M12 3.2 4.7 6.0v5.5c0 4.3 2.9 7.8 7.3 9.6 4.4-1.8 7.3-5.3 7.3-9.6V6.0Z"
        />
      )}
    </RailGlyph>
  );
}

/** 网络 = points joined by edges, which is exactly what the page draws
    (docs/MORE.md 第二轮: 网络整页是地图). **Not `IconDevices`**, which the
    机器 landing already wears: one glyph standing for two rows in one
    rail is two places to press to find out which. */
export function IconNetwork({ className }: IconProps) {
  return (
    <RailGlyph
      className={className}
      live={LIVE.network}
      cut={
        <>
          <circle cx="12" cy="5.4" r="2.45" fill="black" stroke="black" strokeWidth={cutOf(LIVE.network)} />
          <circle cx="5.5" cy="17.2" r="2.45" fill="black" stroke="black" strokeWidth={cutOf(LIVE.network)} />
          <circle cx="18.5" cy="17.2" r="2.45" fill="black" stroke="black" strokeWidth={cutOf(LIVE.network)} />
        </>
      }
    >
      {(cutId) => (
        <>
          {/* The edges are drawn WHOLE, centre to centre, and the nodes
              cut them — so every gap is the family's 0.8 instead of
              three pairs of endpoints placed by hand. The old ones sat
              0.5, 0.6 and 1.1 apart, which is what made the joins look
              untidy without looking obviously wrong. */}
          <path mask={`url(#${cutId})`} d="M12 5.4 5.5 17.2M12 5.4l6.5 11.8M5.5 17.2h13" />
          {/* The hub is the solid one: this machine. */}
          <circle data-body cx="12" cy="5.4" r="2.45" />
          <circle cx="5.5" cy="17.2" r="2.45" />
          <circle cx="18.5" cy="17.2" r="2.45" />
        </>
      )}
    </RailGlyph>
  );
}

/** The reading, drawn once: it is both the front form and the shape
    that cuts the plate behind it, and two copies would drift. */
const PULSE_TRACE = "M6.6 12.2h2.5l1.9-4.6 2.6 9 1.8-4.4h2.8";

/** 诊断 = a pulse across a frame: a list of readings taken, which is what
    the page is. Not a stethoscope — khor checks itself, nobody holds it. */
export function IconPulse({ className }: IconProps) {
  return (
    <RailGlyph
      className={className}
      live={LIVE.pulse}
      cut={<path d={PULSE_TRACE} stroke="black" strokeWidth={cutOf(LIVE.pulse)} />}
    >
      {(cutId) => (
        <>
          {/* Same construction as the conversation and the sphere: the
              reading is the front form and the plate is masked by it,
              so the trace runs in its own channel rather than being
              drawn on top of a box. The old swing (−3.6 / +7.2) was too
              small to read at rail size; this one reaches the plate's
              own inset. */}
          <rect data-body mask={`url(#${cutId})`} x="3.4" y="5" width="17.2" height="14" rx="2.8" />
          <path d={PULSE_TRACE} />
        </>
      )}
    </RailGlyph>
  );
}

/**
 * The 16-canvas marks' common plate. Canvas and stroke are written once
 * here for the same reason `RailGlyph` writes them once: a width spelled
 * out per icon drifts, and mandala ended up with six of them, each right
 * on its own and no two alike. `weight` names the two densities — the
 * numbers behind them stay out of reach.
 */
function Mark({
  className,
  weight = "line",
  icon,
  live,
  cut,
  children,
}: IconProps & {
  weight?: keyof typeof STROKE;
  icon?: string;
  live: Fit;
  cut?: ReactNode;
  children: ReactNode | ((cutId: string) => ReactNode);
}) {
  const uid = useId().replace(/:/g, "");
  const cutId = `markcut-${uid}`;
  return (
    <svg
      // Which mark this is, for the few gates that must find one by what
      // it *is* (docs/SHELL.md 第四轮: no pin anywhere on the browse page).
      data-icon={icon}
      className={cn("size-4", className)}
      viewBox={viewBox(live)}
      fill="none"
      stroke="currentColor"
      strokeWidth={lineOf(live, STROKE[weight])}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {cut && (
        <mask id={cutId}>
          <rect x={live.x} y={live.y} width={live.side} height={live.side} fill="white" />
          {cut}
        </mask>
      )}
      {typeof children === "function" ? children(cutId) : children}
    </svg>
  );
}

/** A git branch: two commits on a trunk and one on a side road joined
    back in — the corner of a workspace that is a repository
    (docs/SHELL.md 第四轮 第 2 条:「文件夹, 是 git 仓库时换分支」). */
export function IconBranch({ className }: IconProps) {
  return (
    <Mark
      className={className}
      live={LIVE.branch}
      cut={
        <>
          <circle cx="4.6" cy="3.4" r="1.55" fill="black" stroke="black" strokeWidth={cutOf(LIVE.branch)} />
          <circle cx="4.6" cy="12.6" r="1.55" fill="black" stroke="black" strokeWidth={cutOf(LIVE.branch)} />
          <circle cx="11.4" cy="4.8" r="1.55" fill="black" stroke="black" strokeWidth={cutOf(LIVE.branch)} />
        </>
      }
    >
      {(cutId) => (
        <>
          {/* Trunk and side road are drawn whole and the commits cut
              them. The old side road never actually met the trunk — it
              stopped 1.9 short and pointed at it, which is the one thing
              a merge may not do. */}
          <path
            mask={`url(#${cutId})`}
            d="M4.6 3.4v9.2M11.4 4.8v2.8a3.1 3.1 0 0 1-3.1 3.1H4.6"
          />
          <circle cx="4.6" cy="3.4" r="1.55" />
          <circle cx="4.6" cy="12.6" r="1.55" />
          {/* The branch head is the solid commit. */}
          <circle data-body cx="11.4" cy="4.8" r="1.55" />
        </>
      )}
    </Mark>
  );
}

/** Back chevron, 16-canvas mark. */
export function IconBack({ className }: IconProps) {
  return (
    <Mark className={className} live={POINTED.back}>
      <path d="M10 3.2 5.2 8l4.8 4.8" />
    </Mark>
  );
}

/** The mirror of `IconBack`, and drawn as its mirror rather than as its
    own path so the pair cannot drift apart by a pixel. */
export function IconForward({ className }: IconProps) {
  return (
    <Mark className={className} live={POINTED.forward}>
      <path d="M6 3.2 10.8 8 6 12.8" />
    </Mark>
  );
}

/** Reload: a circle open at the top with the arrowhead on the open end,
    which is what says it comes back round rather than merely turns. */
export function IconReload({ className }: IconProps) {
  return (
    <Mark className={className} live={LIVE.reload}>
      {/* 306° of ring, and a head that is a real arrowhead. The old one
          made its head from two strokes meeting at a right angle, which
          at 16px reads as a corner, not as a point. The arc stops under
          the triangle so its round cap has nowhere to show. */}
      <path d="M11.15 4.25A4.9 4.9 0 1 1 6.82 3.25" />
      <path d="M8.48 2.61 6.87 4.9 5.77 1.9Z" fill="currentColor" stroke="none" />
    </Mark>
  );
}

export function IconSearch({ className }: IconProps) {
  return (
    <Mark className={className} live={LIVE.search}>
      <circle cx="7" cy="7" r="4.2" />
      <path d="M10.2 10.2L13.5 13.5" />
    </Mark>
  );
}

/** A row's handle for dragging it into another place (外观's navigation
    order, the redesign 2026-10-02): three rules, the system's 「≡」. */
export function IconGrip({ className }: IconProps) {
  return (
    <Mark className={className} live={LIVE.grip}>
      <path d="M3 5.5h10M3 8h10M3 10.5h10" />
    </Mark>
  );
}

export function IconPlus({ className }: IconProps) {
  return (
    <Mark className={className} live={LIVE.plus}>
      <path d="M8 3v10M3 8h10" />
    </Mark>
  );
}

/** How to look at this list. Three sliders — a menu of choices, which is
    what separates it from `IconMore` (three dots, "there is more"). */
export function IconFilter({ className }: IconProps) {
  return (
    <Mark
      className={className}
      live={LIVE.filter}
      cut={
        <>
          <circle cx="5.4" cy="4" r="2.4" fill="black" />
          <circle cx="10.6" cy="8" r="2.4" fill="black" />
          <circle cx="7.2" cy="12" r="2.4" fill="black" />
        </>
      }
    >
      {(cutId) => (
        <>
          {/* The rails are drawn full width and the knobs cut them, so a
              rail stops 0.55 short of its knob instead of running under
              it. The knobs take the family's body density rather than
              full ink — three discs at full ink read as three holes. */}
          <path mask={`url(#${cutId})`} d="M2.6 4h10.8M2.6 8h10.8M2.6 12h10.8" />
          <circle data-body cx="5.4" cy="4" r="1.65" />
          <circle data-body cx="10.6" cy="8" r="1.65" />
          <circle data-body cx="7.2" cy="12" r="1.65" />
        </>
      )}
    </Mark>
  );
}

/** The tick a checkbox item shows — the family's own, not a borrowed one:
    a second icon family on one screen reads as a second typeface. */
export function IconCheck({ className }: IconProps) {
  return (
    <Mark className={className} live={LIVE.check} weight="bold">
      <path d="M3 8.5l3.2 3L13 4.5" />
    </Mark>
  );
}

export function IconClose({ className }: IconProps) {
  return (
    <Mark className={className} live={LIVE.close}>
      <path d="M4 4l8 8M12 4l-8 8" />
    </Mark>
  );
}

/** The two steps through a list of matches (the terminal's search).
    A chevron and not an arrow: an arrow says "go", these say "the one
    before / the one after", which is what ⇧⏎ / ⏎ do in the field. */
export function IconChevronUp({ className, centred = false }: IconProps & { centred?: boolean }) {
  return (
    <Mark className={className} live={centred ? LIVE.chevronUp : POINTED.chevronUp}>
      <path d="M4 10l4-4 4 4" />
    </Mark>
  );
}

export function IconChevronDown({ className, centred = false }: IconProps & { centred?: boolean }) {
  return (
    <Mark className={className} live={centred ? LIVE.chevronDown : POINTED.chevronDown}>
      <path d="M4 6l4 4 4-4" />
    </Mark>
  );
}

/** The twisty on a directory row (工作区批⑦): 「目录有类似 `>` 的符号表示
    可展开」. **The same chevron turned**, not a second glyph — one form
    rotating is what says the open row and the shut row are one control
    in two positions; two drawings would have to be learned as two. */
export function IconChevronRight({ className }: IconProps) {
  return (
    <Mark className={className} live={POINTED.chevronRight}>
      <path d="M6 4l4 4-4 4" />
    </Mark>
  );
}

/** An editor — the way to VS Code, on the workspace's tab strip
    (工作区批⑧: 「vscode 换成 icon 显示」).
    **Angle brackets and a slash, not Microsoft's mark.** A vendor's logo
    is their drawing, at their stroke ratio, and this family has one
    (icons.tsx's head); what the button opens is said out loud by its
    name, which a screen reader gets and a hover gives. */
/** A filled mark from outside the 16-canvas family: the shape comes
    whole, in its own box, and takes the current colour. **Two icons
    wear this and both were handed over as files** (2026-09-16:「VSCODE
    的 icon 用这个…预览的 ICON 用这个」) — the VS Code logo, which is
    somebody else's mark and not ours to redraw as strokes, and the
    preview glyph the user picked. Everything else stays in `Mark`.
    **`box` is not the file's canvas**: both filled theirs edge to edge,
    so each is padded out to the one live area (`LIVE`, square: 0.88) —
    the shape untouched, scaled and centred. */
function Fill({
  className,
  box,
  children,
}: IconProps & { box: string; children: ReactNode }) {
  return (
    <svg
      className={cn("size-4", className)}
      viewBox={box}
      fill="currentColor"
      aria-hidden="true"
    >
      {children}
    </svg>
  );
}

/** The VS Code logo, as the switch that opens a workspace in it.
    Traced from the file the user handed over (svgrepo's copy of the
    official mark); the two strokes it replaced said 「code」 in general
    and the switch is about one editor in particular. */
export function IconCode({ className }: IconProps) {
  return (
    <Fill className={className} box="-2.17 -2.17 36.34 36.34">
      <path d="M30.865 3.448l-6.583-3.167c-0.766-0.37-1.677-0.214-2.276 0.385l-12.609 11.505-5.495-4.167c-0.51-0.391-1.229-0.359-1.703 0.073l-1.76 1.604c-0.583 0.526-0.583 1.443-0.005 1.969l4.766 4.349-4.766 4.349c-0.578 0.526-0.578 1.443 0.005 1.969l1.76 1.604c0.479 0.432 1.193 0.464 1.703 0.073l5.495-4.172 12.615 11.51c0.594 0.599 1.505 0.755 2.271 0.385l6.589-3.172c0.693-0.333 1.13-1.031 1.13-1.802v-21.495c0-0.766-0.443-1.469-1.135-1.802zM24.005 23.266l-9.573-7.266 9.573-7.266z" />
    </Fill>
  );
}

/** Paperclip — attach a file to the conversation. One continuous line,
    the family's density; drawn for the 16 canvas, not scaled in. */
export function IconAttach({ className }: IconProps) {
  return (
    <Mark className={className} live={LIVE.attach}>
      <path d="M12.9 7.1l-5.3 5.3a2.9 2.9 0 0 1-4.1-4.1l5.7-5.7a2 2 0 0 1 2.9 2.9l-5.4 5.4a1.1 1.1 0 0 1-1.6-1.6l4.9-4.9" />
    </Mark>
  );
}

/**
 * Pin a row to the top. Ported from mandala's `IconPin`, paths verbatim,
 * and so is the judgment behind it:
 *
 * **The two states must be two shapes, not one shape in two colors.** A
 * pin carries its own metaphor — tilted means resting on the surface,
 * upright means driven in — so the button reports its own state instead
 * of making you check whether the row jumped to the top. That check is
 * position speaking on the control's behalf, and it fails exactly when
 * the row was already at the top.
 *
 * Drawn tilted and rotated 45° when pinned, which is a difference the
 * eye catches without looking for it. `origin-center` is not decoration:
 * an SVG element's `transform-origin` defaults to (0, 0), not to the
 * centre the way a normal HTML element's does, and without it the pin
 * swings out of its own box.
 */
export function IconPin({ className, pinned = false }: IconProps & { pinned?: boolean }) {
  return (
    <Mark className={cn("origin-center", pinned && "-rotate-45", className)} icon="pin" live={LIVE.pin}>
      {/* A real tack — head, collar, shaft, needle — drawn UPRIGHT
          inside a group turned 45°, so `pinned`'s `-rotate-45` above
          takes it back to upright rather than to some third angle. The
          old one was a quadrilateral and a line: a shape with no tack
          in it. The head carries the body density. */}
      <g transform="rotate(45 8 8)">
        <path
          data-body
          d="M6.2 3h3.6a1 1 0 0 1 1 1v.9a1 1 0 0 1-1 1h-.4l.58 2.9h.92a.85.85 0 0 1 0 1.7H5.1a.85.85 0 0 1 0-1.7h.92l.58-2.9H6.2a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1Z"
        />
        <path d="M8 10.6v2.6" />
      </g>
    </Mark>
  );
}

/**
 * A terminal: prompt and cursor. Ported from mandala's own-drawn one
 * (BrandIcons.tsx there, path verbatim): a session with no agent is not
 * "missing an agent", it *is* a terminal — the mark states that rather
 * than negating something.
 */
export function IconTerminal({ className }: IconProps) {
  return (
    <Mark className={className} live={LIVE.terminal}>
      <path d="M3.4 4.6L6.6 8l-3.2 3.4" />
      <path d="M8.2 11.6h4.4" />
    </Mark>
  );
}

/**
 * A conversation: one bubble, not two — two stacked bubbles blur into a
 * blob at this size, and this one shape means the same thing in every
 * app (mandala's judgment, path verbatim). The lower-left point is the
 * whole information content: without it this is a rounded box.
 */
export function IconChat({ className }: IconProps) {
  return (
    <Mark className={className} live={LIVE.chat}>
      {/* **This IS the rail's front bubble**, `BUBBLE_FRONT` taken to
          the 16 canvas (×0.906, shifted −0.47 / +0.1): same corner
          radius, same tail, same corner where the tail turns. One thing
          drawn twice was the whole problem — the hand-made version had
          a rounded nub where this has a point, and at 16px a rounded
          nub is just a bulge. It carries the body density like every
          other closed form in the family. */}
      <path
        data-body
        d="M5.24 3.54H10.76A2.54 2.54 0 0 1 13.3 6.08V8.98A2.54 2.54 0 0 1 10.76 11.52H7.68L5.33 13.87V11.52A2.54 2.54 0 0 1 2.7 8.98V6.08A2.54 2.54 0 0 1 5.24 3.54Z"
      />
    </Mark>
  );
}

/**
 * A keyboard — the soft keyboard's switch on the narrow session page's
 * bar (2026-10-02, iOS 第三步 A): up when it is down, down when it is up.
 * Drawn for the 16 canvas, the 32nd glyph. **The case is the solid
 * piece** and the keys are punched through it with the family's cut — a
 * row of three keys and the space bar, the shield's keyhole done four
 * times — rather than drawn on top: keys drawn on a filled case would be
 * ink on ink, and keys drawn as an outline's contents read at 16 as a
 * grille.
 */
export function IconKeyboard({ className }: IconProps) {
  return (
    <Mark
      className={className}
      live={LIVE.keyboard}
      icon="keyboard"
      cut={
        <path
          stroke="black"
          strokeWidth={lineOf(LIVE.keyboard)}
          strokeLinecap="round"
          d="M5.4 6.9h.01M8 6.9h.01M10.6 6.9h.01M5.4 9.5h5.2"
        />
      }
    >
      {(cutId) => (
        <path
          data-body
          mask={`url(#${cutId})`}
          d="M4.1 4h7.8a1.6 1.6 0 0 1 1.6 1.6v4.8a1.6 1.6 0 0 1-1.6 1.6H4.1a1.6 1.6 0 0 1-1.6-1.6V5.6A1.6 1.6 0 0 1 4.1 4Z"
        />
      )}
    </Mark>
  );
}

/** The cursor key at rest (2026-10-03, `views/TermKeys`): four arrows
    from one centre — it moves the cursor any way it is dragged. */
export function IconMove({ className }: IconProps) {
  return (
    <Mark className={className} live={LIVE.move}>
      <path d="M8 2.5v11M2.5 8h11M6.2 4.3 8 2.5l1.8 1.8M6.2 11.7 8 13.5l1.8-1.8M4.3 6.2 2.5 8l1.8 1.8M11.7 6.2 13.5 8l-1.8 1.8" />
    </Mark>
  );
}

const ARROW = {
  left: "M13.5 8h-11M6.5 4l-4 4 4 4",
  right: "M2.5 8h11M9.5 4l4 4-4 4",
  up: "M8 13.5v-11M4 6.5l4-4 4 4",
  down: "M8 2.5v11M4 9.5l4 4 4-4",
} as const;

/** One arrow — the cursor key while a drag is locked to that way. */
export function IconArrow({ className, to }: IconProps & { to: keyof typeof ARROW }) {
  return (
    <Mark className={className} live={LIVE.arrow}>
      <path d={ARROW[to]} />
    </Mark>
  );
}

/** A file — the transfer rows' mark (mandala path, verbatim). */
export function IconFile({ className }: IconProps) {
  return (
    <Mark className={className} live={LIVE.file}>
      <path d="M9 2.4H4.6c-.7 0-1.2.5-1.2 1.2v8.8c0 .7.5 1.2 1.2 1.2h6.8c.7 0 1.2-.5 1.2-1.2V5.8L9 2.4z" />
      <path d="M8.9 2.6v3.1h3.3" />
    </Mark>
  );
}

/**
 * An eye — 预览 on the tab strip's far end (「如果可预览,就是一个预览的
 * icon,再次点击回到源码」— the user, 2026-09-16). The same mark VS Code
 * puts on 打开预览, and the reason it can be a picture at all is that it
 * switches a *face* rather than a column: the two words it replaced said
 * which of them you were looking at, and `data-on` says that now.
 *
 * Two strokes in the 16 family: the almond, and the pupil that keeps it
 * from reading as a leaf.
 */
/** 预览 — an eye inside a viewfinder's four corners, the file the user
    handed over for the source/preview switch (its export metadata
    stripped, its ink made the current colour). */
export function IconEye({ className }: IconProps) {
  return (
    <Fill className={className} box="-69.5 -69.5 1163.01 1163.01">
      <path d="M703.956003 63.996a31.998 31.998 0 1 1 0-63.996h186.932316C964.419724 0 1023.936004 59.51628 1023.936004 133.047685V319.980001a31.998 31.998 0 1 1-63.996 0V133.047685C959.940004 94.906068 929.029936 63.996 890.888319 63.996H703.956003z m255.984001 624.792951a31.998 31.998 0 1 1 63.996 0v202.099368C1023.936004 964.419724 964.419724 1023.936004 890.888319 1023.936004h-126.328104a31.998 31.998 0 1 1 0-63.996h126.328104c38.141616 0 69.051684-30.910068 69.051685-69.051685v-202.099368zM335.147053 959.940004a31.998 31.998 0 1 1 0 63.996H133.047685A133.047685 133.047685 0 0 1 0 890.888319V703.956003a31.998 31.998 0 1 1 63.996 0v186.932316C63.996 929.029936 94.906068 959.940004 133.047685 959.940004h202.099368zM63.996 319.980001a31.998 31.998 0 0 1-63.996 0V133.047685C0 59.51628 59.51628 0 133.047685 0H319.980001a31.998 31.998 0 0 1 0 63.996H133.047685C94.906068 63.996 63.996 94.906068 63.996 133.047685V319.980001zM511.968002 671.958003c109.625148 0 211.634773-56.892444 308.332729-175.989001C723.602775 376.872445 621.59315 319.980001 511.968002 319.980001c-109.625148 0-211.634773 56.892444-308.332729 175.989001C300.333229 615.065558 402.342854 671.958003 511.968002 671.958003z m0 63.996c-130.615837 0-249.968377-66.55584-357.993625-199.667521a63.996 63.996 0 0 1 0-80.63496C261.999625 322.539841 381.352165 255.984001 511.968002 255.984001c130.615837 0 249.968377 66.55584 357.993625 199.667521a63.996 63.996 0 0 1 0 80.63496C761.936379 669.398163 642.583839 735.954003 511.968002 735.954003z" />
      <path d="M511.968002 575.964002a79.995 79.995 0 1 0 0-159.99A79.995 79.995 0 0 0 511.968002 575.964002z m0 63.996a143.991001 143.991001 0 1 1 0-287.982001A143.991001 143.991001 0 0 1 511.968002 639.960002z" />
    </Fill>
  );
}

/**
 * A link — the borrow rows' mark: a borrowed pipe to another machine's
 * network (mandala path, verbatim; there it meant a symlink, here the
 * same shape reads as "connected through").
 */
export function IconLink({ className }: IconProps) {
  return (
    <Mark className={className} live={LIVE.link}>
      <path d="M6.6 9.4a2.6 2.6 0 0 0 3.9.3l1.7-1.7a2.6 2.6 0 0 0-3.7-3.7l-1 1" />
      <path d="M9.4 6.6a2.6 2.6 0 0 0-3.9-.3L3.8 8a2.6 2.6 0 0 0 3.7 3.7l1-1" />
    </Mark>
  );
}
