/**
 * **Glass controls** (the redesign, 2026-10-02: iOS 26's floating
 * controls, in CSS because this is a web view). The material is the
 * `glass` utility (`tokens.css`); these are its two shapes:
 *
 * - `GlassCircle`: one round button — search, back, the keyboard.
 * - `GlassCapsule` holding `GlassItem`s: a few buttons that belong
 *   together in one pill — filter and ＋, a page's verbs.
 *
 * The size is the pointer's (`--glass-btn`: 32 under a mouse, 44 under a
 * finger), never the width's. Both forward their ref and props, so a
 * Radix trigger can wear them (`asChild`).
 */
import { forwardRef, type ComponentProps } from "react";

import { cn } from "@/lib/utils";

const RING = "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring";

/** `head`: a detail head's size (`--head-ctl`), the avatar's — the
    back circle and the verbs capsule there are exactly as tall as the
    face beside them (the owner, 2026-10-02). */
type Size = { size?: "head" };

export const GlassCircle = forwardRef<HTMLButtonElement, ComponentProps<"button"> & Size>(function GlassCircle(
  { className, type = "button", size, ...props },
  ref,
) {
  return (
    <button
      ref={ref}
      type={type}
      data-slot="button"
      data-glass-circle
      className={cn(
        "glass grid flex-none place-items-center rounded-full text-foreground",
        size === "head" ? "size-head-ctl [&_svg]:size-head-glyph" : "size-glass-btn [&_svg]:size-glass-icon",
        "disabled:opacity-40 [&_svg]:pointer-events-none [&_svg]:shrink-0",
        RING,
        className,
      )}
      {...props}
    />
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
export const GlassItem = forwardRef<HTMLButtonElement, ComponentProps<"button"> & { on?: boolean }>(
  function GlassItem({ className, type = "button", on, ...props }, ref) {
    return (
      <button
        ref={ref}
        type={type}
        data-slot="button"
        data-glass-item
        data-on={on || undefined}
        className={cn(
          "grid h-glass-item-h w-glass-item-w place-items-center rounded-full text-foreground",
          // Inside a head-sized capsule, the head's button and glyph.
          "group-data-[size=head]/cap:h-head-item-h group-data-[size=head]/cap:w-head-item-w group-data-[size=head]/cap:[&_svg]:size-head-glyph",
          "hover:bg-fill data-[on]:bg-fill-strong data-[on]:text-[var(--rail-on-ink)] data-[state=open]:bg-fill",
          "disabled:opacity-40 [&_svg]:pointer-events-none [&_svg]:size-glass-icon [&_svg]:shrink-0",
          RING,
          className,
        )}
        {...props}
      />
    );
  },
);
