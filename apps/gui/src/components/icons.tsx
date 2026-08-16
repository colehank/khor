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
