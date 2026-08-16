// Ported from mandala's icon system (tokens repo, Icons.tsx + glyph.ts):
// 24-canvas stroke glyphs, one solid piece per form (data-body) with the
// rest in line — two densities, not two colors. Overlaps are cut with an
// equal-width mask gap, never broken paths. Paths are copied verbatim;
// changing one here forks the family.

import { useId, type ReactNode } from "react";

import { cn } from "@/lib/utils";

// Stroke widths are ratios in disguise: the number only means anything
// divided by its canvas. 24-canvas rail glyphs use 1.4; 16-canvas marks
// use 1.5 (optically compensated — thinner would vanish at 1x).
export const STROKE = { rail: 1.4, sm: 1.5 } as const;

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

/** Back chevron, 16-canvas mark. */
export function IconBack({ className }: IconProps) {
  return (
    <svg
      className={cn("size-4", className)}
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth={STROKE.sm}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M10 3.2 5.2 8l4.8 4.8" />
    </svg>
  );
}
