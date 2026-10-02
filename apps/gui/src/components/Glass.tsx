/**
 * **Glass controls** (the redesign, 2026-10-02: iOS 26's floating
 * controls, in CSS because this is a web view). The material is the
 * `glass` utility (`tokens.css`); these are its two shapes:
 *
 * - `GlassCircle`: one round button — search, back, the keyboard.
 * - `GlassCapsule` holding `GlassItem`s: a few buttons that belong
 *   together in one pill — filter and ＋, a page's verbs.
 *
 * The size is the pointer's (`--glass-btn`: 36 under a mouse, 44 under a
 * finger), never the width's; the glyph in either is `--ctl-glyph`, set by
 * `app.css` on the seat, and so are the hover, the press and the focus
 * ring (one answer for every glass control). Both forward their ref and
 * props, so a Radix trigger can wear them (`asChild`).
 *
 * **Every one names itself under the pointer** (the audit fix plan F1.7;
 * SYS-08, W11): a glyph-only control carries a tooltip, its accessible
 * name unless it says otherwise (`tip`), `false` where a caller draws its
 * own. On macOS every glyph-only toolbar button has a help tag.
 */
import { forwardRef, type ComponentProps, type ReactElement } from "react";

import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";

/** The tooltip a glyph-only control wears: its `tip`, else its
    accessible name; none for `false` or for a control with no name. */
type Tip = { tip?: string | false; tipSide?: "top" | "bottom" };
function tipped(button: ReactElement, tip: string | false | undefined, label: unknown, side: "top" | "bottom" = "bottom") {
  const words = tip === false ? null : (tip ?? (typeof label === "string" ? label : null));
  if (!words) return button;
  return (
    <Tooltip>
      <TooltipTrigger asChild>{button}</TooltipTrigger>
      <TooltipContent side={side}>{words}</TooltipContent>
    </Tooltip>
  );
}

const RING = "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring";

/** `head`: a detail head's size (`--head-ctl`), the avatar's — the
    back circle and the verbs capsule there are exactly as tall as the
    face beside them (the owner, 2026-10-02). */
type Size = { size?: "head" };

export const GlassCircle = forwardRef<HTMLButtonElement, ComponentProps<"button"> & Size & Tip>(function GlassCircle(
  { className, type = "button", size, tip, tipSide, ...props },
  ref,
) {
  return tipped(
    <button
      ref={ref}
      type={type}
      data-slot="button"
      data-glass-circle
      className={cn(
        "glass grid flex-none place-items-center rounded-full text-foreground",
        size === "head" ? "size-head-ctl" : "size-glass-btn",
        "disabled:pointer-events-none aria-busy:animate-pulse motion-reduce:aria-busy:animate-none [&_svg]:pointer-events-none [&_svg]:shrink-0",
        RING,
        className,
      )}
      {...props}
    />,
    tip,
    props["aria-label"],
    tipSide,
  );
});

export function GlassCapsule({ className, size, ...props }: ComponentProps<"div"> & Size) {
  return (
    <div
      data-glass-capsule
      data-size={size}
      className={cn(
        "glass flex flex-none items-center rounded-full px-glass-pad",
        size === "head" ? "group/cap h-head-ctl" : "h-glass-btn",
        className,
      )}
      {...props}
    />
  );
}

/** One button inside a capsule: no material of its own, a fill when it
    is the one that is on (`aria-pressed`, a menu's open state). */
export const GlassItem = forwardRef<HTMLButtonElement, ComponentProps<"button"> & { on?: boolean } & Tip>(
  function GlassItem({ className, type = "button", on, tip, tipSide, ...props }, ref) {
    return tipped(
      <button
        ref={ref}
        type={type}
        data-slot="button"
        data-glass-item
        data-on={on || undefined}
        className={cn(
          "grid h-glass-item-h w-glass-item-w place-items-center rounded-full text-foreground",
          // Inside a head-sized capsule, the head's button and glyph.
          "group-data-[size=head]/cap:h-head-item-h group-data-[size=head]/cap:w-head-item-w",
          "data-[on]:glass-sel data-[on]:text-[var(--rail-on-ink)] data-[state=open]:glass-sel",
          "disabled:pointer-events-none aria-busy:animate-pulse motion-reduce:aria-busy:animate-none [&_svg]:pointer-events-none [&_svg]:shrink-0",
          RING,
          className,
        )}
        {...props}
      />,
      tip,
      props["aria-label"],
      tipSide,
    );
  },
);
