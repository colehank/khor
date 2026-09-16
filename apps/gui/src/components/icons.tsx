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

// Stroke widths are ratios in disguise: the number only means anything
// divided by its canvas. 24-canvas rail glyphs use 1.4; 16-canvas marks
// use 1.5 (optically compensated — thinner would vanish at 1x); the
// tick-and-cross density is 1.9, heavier because those two are read as
// answers, not as pictures.
export const STROKE = { rail: 1.4, sm: 1.5, bold: 1.9 } as const;

// The cut shape's width: the cut line is the front form's outline
// widened to 3, leaving a (3 - 1.4) / 2 = 0.8 gap on each side.
const CUT = 3;

type IconProps = { className?: string };

function RailGlyph({
  className,
  cut,
  children,
}: IconProps & {
  cut?: ReactNode;
  children: ReactNode | ((cutId: string) => ReactNode);
}) {
  const uid = useId().replace(/:/g, "");
  const cutId = `railcut-${uid}`;
  return (
    <svg
      className={cn("size-icon-rail", className)}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={STROKE.rail}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {cut && (
        <mask id={cutId}>
          <rect x="0" y="0" width="24" height="24" fill="white" />
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
      cut={<path d={BUBBLE_FRONT} fill="black" stroke="black" strokeWidth={CUT} />}
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
      viewBox="0 0 100 100"
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
    <RailGlyph className={className}>
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
      cut={
        <ellipse
          cx="12"
          cy="12"
          rx="8.1"
          ry="2.9"
          transform="rotate(-20 12 12)"
          fill="none"
          stroke="black"
          // 0.2 wider than `CUT`, the family's one exception: the band
          // crosses the sphere at a slant, and an equal-width cut leaves
          // a visually narrower gap along a slanted edge than a square
          // one. This is the value that makes the gap look the same as
          // the other glyphs', not a different rule.
          strokeWidth={CUT + 0.2}
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
    <RailGlyph className={className}>
      <circle cx="5.4" cy="12" r="1.55" fill="currentColor" stroke="none" />
      <circle cx="12" cy="12" r="1.55" fill="currentColor" stroke="none" />
      <circle cx="18.6" cy="12" r="1.55" fill="currentColor" stroke="none" />
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
  weight = "sm",
  children,
}: IconProps & { weight?: "sm" | "bold"; children: ReactNode }) {
  return (
    <svg
      className={cn("size-4", className)}
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth={STROKE[weight]}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {children}
    </svg>
  );
}

/** Back chevron, 16-canvas mark. */
export function IconBack({ className }: IconProps) {
  return (
    <Mark className={className}>
      <path d="M10 3.2 5.2 8l4.8 4.8" />
    </Mark>
  );
}

/** The mirror of `IconBack`, and drawn as its mirror rather than as its
    own path so the pair cannot drift apart by a pixel. */
export function IconForward({ className }: IconProps) {
  return (
    <Mark className={className}>
      <path d="M6 3.2 10.8 8 6 12.8" />
    </Mark>
  );
}

/** Reload: a circle open at the top with the arrowhead on the open end,
    which is what says it comes back round rather than merely turns. */
export function IconReload({ className }: IconProps) {
  return (
    <Mark className={className}>
      <path d="M13 8a5 5 0 1 1-1.5-3.6" />
      <path d="M13 2.6V5h-2.4" />
    </Mark>
  );
}

export function IconSearch({ className }: IconProps) {
  return (
    <Mark className={className}>
      <circle cx="7" cy="7" r="4.2" />
      <path d="M10.2 10.2L13.5 13.5" />
    </Mark>
  );
}

export function IconPlus({ className }: IconProps) {
  return (
    <Mark className={className}>
      <path d="M8 3v10M3 8h10" />
    </Mark>
  );
}

/** How to look at this list. Three sliders — a menu of choices, which is
    what separates it from `IconMore` (three dots, "there is more"). */
export function IconFilter({ className }: IconProps) {
  return (
    <Mark className={className}>
      <path d="M2.5 4.5h11M2.5 8h11M2.5 11.5h11" />
      <circle cx="5.5" cy="4.5" r="1.7" fill="currentColor" />
      <circle cx="11" cy="8" r="1.7" fill="currentColor" />
      <circle cx="7" cy="11.5" r="1.7" fill="currentColor" />
    </Mark>
  );
}

/** The tick a checkbox item shows — the family's own, not a borrowed one:
    a second icon family on one screen reads as a second typeface. */
export function IconCheck({ className }: IconProps) {
  return (
    <Mark className={className} weight="bold">
      <path d="M3 8.5l3.2 3L13 4.5" />
    </Mark>
  );
}

export function IconClose({ className }: IconProps) {
  return (
    <Mark className={className}>
      <path d="M4 4l8 8M12 4l-8 8" />
    </Mark>
  );
}

/** The two steps through a list of matches (the terminal's search).
    A chevron and not an arrow: an arrow says "go", these say "the one
    before / the one after", which is what ⇧⏎ / ⏎ do in the field. */
export function IconChevronUp({ className }: IconProps) {
  return (
    <Mark className={className}>
      <path d="M4 10l4-4 4 4" />
    </Mark>
  );
}

export function IconChevronDown({ className }: IconProps) {
  return (
    <Mark className={className}>
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
    <Mark className={className}>
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
    preview glyph the user picked. Everything else stays in `Mark`. */
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
    <Fill className={className} box="0 0 32 32">
      <path d="M30.865 3.448l-6.583-3.167c-0.766-0.37-1.677-0.214-2.276 0.385l-12.609 11.505-5.495-4.167c-0.51-0.391-1.229-0.359-1.703 0.073l-1.76 1.604c-0.583 0.526-0.583 1.443-0.005 1.969l4.766 4.349-4.766 4.349c-0.578 0.526-0.578 1.443 0.005 1.969l1.76 1.604c0.479 0.432 1.193 0.464 1.703 0.073l5.495-4.172 12.615 11.51c0.594 0.599 1.505 0.755 2.271 0.385l6.589-3.172c0.693-0.333 1.13-1.031 1.13-1.802v-21.495c0-0.766-0.443-1.469-1.135-1.802zM24.005 23.266l-9.573-7.266 9.573-7.266z" />
    </Fill>
  );
}

/** Paperclip — attach a file to the conversation. One continuous line,
    the family's density; drawn for the 16 canvas, not scaled in. */
export function IconAttach({ className }: IconProps) {
  return (
    <Mark className={className}>
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
    <Mark className={cn("origin-center", pinned && "-rotate-45", className)}>
      <path d="M9.6 1.9l4.5 4.5-1.6.5-.6 2.6-4.9-4.9 2.6-.6.4-2.1z" />
      <path d="M7 9L2.7 13.3" />
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
    <Mark className={className}>
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
    <Mark className={className}>
      <path d="M13.2 8.6c0 2.3-2.2 4.2-5 4.2-.6 0-1.2-.1-1.7-.2l-3 1.2.9-2.4c-.9-.8-1.4-1.7-1.4-2.8 0-2.3 2.2-4.2 5-4.2s5 1.9 5 4.2z" />
    </Mark>
  );
}

/** A file — the transfer rows' mark (mandala path, verbatim). */
export function IconFile({ className }: IconProps) {
  return (
    <Mark className={className}>
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
    <Fill className={className} box="0 0 1024 1024">
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
    <Mark className={className}>
      <path d="M6.6 9.4a2.6 2.6 0 0 0 3.9.3l1.7-1.7a2.6 2.6 0 0 0-3.7-3.7l-1 1" />
      <path d="M9.4 6.6a2.6 2.6 0 0 0-3.9-.3L3.8 8a2.6 2.6 0 0 0 3.7 3.7l1-1" />
    </Mark>
  );
}
