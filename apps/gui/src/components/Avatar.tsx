import { useId } from "react";

import type { Avatar } from "@/gen/bindings/Avatar";
import type { Vein } from "@/gen/bindings/Vein";
import { cn } from "@/lib/utils";

/**
 * A machine's face.
 *
 * **Not one number is computed here.** The ground color, every stroke's
 * translation / rotation / scale / color / blend, the blur radius, even
 * what shape to crop to, all arrive derived from `khor_core::avatar`.
 * This file does exactly two things: **paint what it is told**, and
 * **crop**.
 *
 * Why that has to hold: a face is worth one thing, "this face is that
 * machine". The same machine looking different on the phone than on the
 * desktop is worse than having no face at all, and one misrecognition
 * voids the convention. Any geometry computed here is geometry that
 * exists only here.
 *
 * The one exception is the two path strings ([`VEIN_PATH`]) — those are
 * *how to paint*, not parameters, so each end carries its own copy. The
 * price is that they must match character for character; the criterion
 * is on that constant.
 *
 * **The painter never reads the theme.** One machine, one face, in
 * light and dark alike — the same rule as "a rename keeps the face".
 * The only theme-aware part is the hairline edge below, which is not
 * part of the face: it is about what the face is sitting on.
 */

/* ═══════════ the paths that are written down, not derived ═══════════ */

/**
 * Marble's two paths, **copied character for character** from
 * `boringdesigners/boring-avatars`, `src/lib/components/avatar-marble.tsx`.
 *
 * ## Why they live here and not in Rust
 *
 * They are **how to paint, not parameters**: what a machine looks like
 * is the handful of numbers the core derives, while "which primitive
 * draws those numbers" differs by platform anyway (SVG's `<path d>` vs
 * SwiftUI's `Path`). Shipping the string would mean parsing SVG on iOS,
 * or emitting two dialects from Rust — the write-it-twice problem this
 * whole design avoids.
 *
 * ## The price: every end's copy must be identical
 *
 * `sweep`'s last point returns to **22.215** while its start is
 * **22.216** — upstream is off by that 0.001 and `z` closes the hair.
 * **Copy it, do not tidy it up.** One end "fixing" it is two different
 * faces, and at 0.001 on an 80 canvas nobody can see the difference, so
 * fixing it buys nothing at all.
 */
const VEIN_PATH: Record<Vein, string> = {
  shard: "M32.414 59.35L50.376 70.5H72.5v-71H33.728L26.5 13.381l19.057 27.08L32.414 59.35z",
  sweep:
    "M22.216 24L0 46.75l14.108 38.129L78 86l-3.081-59.276-22.378 4.005 12.972 20.186-23.35 27.395L22.215 24z",
};

/**
 * How many σ past the canvas the filter region reaches.
 *
 * 3.43 is not a guess: a Gaussian carries under 0.3% of its weight
 * beyond 3σ, and 128 = 80 + 2×24 is a whole number with 24 = 3.43 × 7.
 * The criterion is "does a **visible** pixel move" — only the canvas's
 * own 80×80 is visible, so a margin of 3σ is a mathematical equivalence.
 *
 * **This is a performance dial, not a visual one.** Larger does not
 * look better, it only rasterizes more area per frame; below 3σ is
 * where the picture actually changes. Leaving it unset is not an
 * option: the default region is `-10% -10% 120% 120%`, which clips
 * these paths — they are scaled past 100 — and the cut lands right in
 * the blur's falloff, where SwiftUI's `.blur()` has no region concept
 * to match.
 */
const BLUR_MARGIN = 3.43;

/* ═══════════ the brush ═══════════ */

/**
 * The canvas and its strokes. `uid` only gives the blur filter a
 * document-unique id — a screen holds dozens of faces, and a collided
 * `url(#…)` resolves every one of them to whichever appeared first.
 */
function AvatarPaintLayers({ face, uid }: { face: Avatar; uid: string }) {
  const c = face.canvas / 2;
  const blurId = `av-blur-${uid}`;
  const paint = face.paint;
  return (
    <>
      {paint.variant === "marble" && (
        <defs>
          {/*
            `colorInterpolationFilters="sRGB"` has to be written out: an
            SVG filter defaults to linearRGB, which computes this blur a
            notch brighter, and the SwiftUI side picks its color mode to
            match this one.

            feFlood + feBlend are two no-ops upstream exported from Figma
            (transparent ground + normal blend = unchanged); they are
            copied so diffs against upstream stay clean. Other ends need
            only reproduce the Gaussian.
          */}
          <filter
            id={blurId}
            filterUnits="userSpaceOnUse"
            x={-paint.blur * BLUR_MARGIN}
            y={-paint.blur * BLUR_MARGIN}
            width={face.canvas + paint.blur * BLUR_MARGIN * 2}
            height={face.canvas + paint.blur * BLUR_MARGIN * 2}
            colorInterpolationFilters="sRGB"
          >
            <feFlood floodOpacity={0} result="BackgroundImageFix" />
            <feBlend in="SourceGraphic" in2="BackgroundImageFix" result="shape" />
            <feGaussianBlur stdDeviation={paint.blur} />
          </filter>
        </defs>
      )}
      {/*
        `isolation: isolate` is **insurance, not something in effect right
        now**: the ground rect below is opaque and fills the canvas, so
        the overlay stroke's backdrop is always this face's own ground,
        never the page. It stays because of what it stops later — the
        moment a ground becomes translucent, overlay blends all the way
        down to the page, and the same machine becomes three faces (a
        row that highlights on hover, a card, a list). That failure has
        no compile-time signal and this line costs nothing.
      */}
      <g style={{ isolation: "isolate" }}>
        <rect x={0} y={0} width={face.canvas} height={face.canvas} fill={face.background} />
        {paint.variant === "marble" ? (
          paint.veins.map((v, i) => (
            // Scale about the origin, then rotate about the canvas
            // center, then translate. **A transform list applies
            // right-to-left to coordinates**, so it is written
            // backwards. `scale()` is about the **origin**, not the
            // center — SwiftUI defaults to the center, which is the
            // easiest place for the two ends to diverge.
            <path
              key={i}
              d={VEIN_PATH[v.vein]}
              fill={v.color}
              filter={`url(#${blurId})`}
              transform={`translate(${v.dx} ${v.dy}) rotate(${v.rotate} ${c} ${c}) scale(${v.scale})`}
              style={v.overlay ? { mixBlendMode: "overlay" } : undefined}
            />
          ))
        ) : paint.variant === "bauhaus" ? (
          paint.shapes.map((s, i) => {
            // Hard edges: no blur, no blend. Two steps only (rotate,
            // translate) because the position is already in x/y/w/h —
            // **there is no scale about the origin here**, so this
            // transform is shorter than marble's by design.
            const t = `translate(${s.dx} ${s.dy}) rotate(${s.rotate} ${c} ${c})`;
            return s.shape === "rect" ? (
              <rect key={i} x={s.x} y={s.y} width={s.w} height={s.h} fill={s.color} transform={t} />
            ) : (
              // Upstream draws a `<circle>`; the core gives a bounding
              // box, which allows an ellipse even though this variant is
              // always round. Painting the box as an `<ellipse>` means
              // no change here the day the core sends a real ellipse.
              <ellipse
                key={i}
                cx={s.x + s.w / 2}
                cy={s.y + s.h / 2}
                rx={s.w / 2}
                ry={s.h / 2}
                fill={s.color}
                transform={t}
              />
            );
          })
        ) : (
          // beam: one wrapper (a rounded rect whose `rx` is already
          // final — always a circle or a rounded square) over the
          // ground, then a face. No blur, no blend. The literals below
          // (14 / 19 / 1.5 / 2 / 1 and the mouth paths) are **how to
          // paint, not parameters** — same treatment as VEIN_PATH.
          <>
            <rect
              x={0}
              y={0}
              width={face.canvas}
              height={face.canvas}
              rx={paint.wrapper_rx}
              fill={paint.wrapper_color}
              transform={`translate(${paint.wrapper_dx} ${paint.wrapper_dy}) rotate(${paint.wrapper_rotate} ${c} ${c}) scale(${paint.wrapper_scale})`}
            />
            <g
              transform={`translate(${paint.face_dx} ${paint.face_dy}) rotate(${paint.face_rotate} ${c} ${c})`}
            >
              {paint.mouth_open ? (
                <path
                  d={`M15 ${19 + paint.mouth_spread}c2 1 4 1 6 0`}
                  stroke={paint.face_ink}
                  fill="none"
                  strokeLinecap="round"
                />
              ) : (
                <path
                  d={`M13,${19 + paint.mouth_spread} a1,0.75 0 0,0 10,0`}
                  fill={paint.face_ink}
                />
              )}
              <rect
                x={14 - paint.eye_spread}
                y={14}
                width={1.5}
                height={2}
                rx={1}
                fill={paint.face_ink}
              />
              <rect
                x={20 + paint.eye_spread}
                y={14}
                width={1.5}
                height={2}
                rx={1}
                fill={paint.face_ink}
              />
            </g>
          </>
        )}
      </g>
    </>
  );
}

/**
 * The crop, as a fraction of the side. A percentage rather than a pixel
 * radius so one face is the same shape at every size — a fixed radius
 * looks like a ball when small and a brick when large. The ratio itself
 * comes from Rust with the face; this only spells it as CSS.
 */
function crop(ratio: number) {
  return { borderRadius: `${ratio * 100}%` };
}

/**
 * **A pure brush**: it paints what it is given, and takes no size of its
 * own — the caller sizes it with a utility, so the numbers stay in the
 * theme.
 */
export function AvatarFace({ face, className }: { face: Avatar; className?: string }) {
  const uid = useId().replace(/:/g, "");
  return (
    <span
      aria-hidden="true"
      // Verification anchors on this, not on a class name: classes are
      // styling and get renamed, and a selector that quietly matches
      // nothing reads in a report as "the thing isn't there".
      data-face=""
      className={cn("relative inline-block flex-none overflow-hidden", className)}
      style={crop(face.radius_ratio)}
    >
      <svg
        viewBox={`0 0 ${face.canvas} ${face.canvas}`}
        className="block size-full"
        // The machine's name is always written beside a face; reading
        // the picture out too would just say the row twice.
        aria-hidden="true"
      >
        <AvatarPaintLayers face={face} uid={uid} />
      </svg>
      {/*
        The hairline edge. **Not decoration — the one stroke this design
        cannot go without.**

        The factory palettes are all tuned for dark editors: every slot
        has to read against one dark ground, which leaves them
        contrasting only 1.12–1.56 against a *light* card. Without an
        edge, half the faces dissolve into the paper in light mode and
        the row looks like it failed to paint one. No choice of palette
        fixes that, so it is handled once, here.

        **This is the one part that reads the theme, and it is not part
        of the face**: `--avatar-edge` is the current theme's ink, which
        covers both directions at once — dark on light gives bright
        tiles an outline, light on dark gives dark ones one, and on the
        half that already stands out the edge simply does not show.
        Every derived number stays theme-independent.

        A layer on top rather than an inset shadow: an inset shadow
        paints above the background but **below the content**, and the
        content here is a rect filling the whole canvas.
      */}
      <span
        className="pointer-events-none absolute inset-0 border border-avatar-edge"
        style={crop(face.radius_ratio)}
      />
    </span>
  );
}

/**
 * A blank the size of a face.
 *
 * It stands for two different things and looking identical is right:
 * the data has not arrived (hold the space, or a list jumps the frame
 * it lands), or this machine has no identity to seed a face with. **A
 * blank is not a gray face, it is no face** — inventing one would get
 * believed, and it would not be that machine's.
 */
export function AvatarBlank({ className }: { className?: string }) {
  return (
    <span
      aria-hidden="true"
      data-face-blank=""
      className={cn("inline-block flex-none rounded-full bg-muted", className)}
    />
  );
}

/**
 * **What screens should use.** Hand it the `face` off a row; a row
 * without one gets the blank.
 */
export function MachineAvatar({
  face,
  className,
}: {
  face: Avatar | null;
  className?: string;
}) {
  return face ? (
    <AvatarFace face={face} className={className} />
  ) : (
    <AvatarBlank className={className} />
  );
}
