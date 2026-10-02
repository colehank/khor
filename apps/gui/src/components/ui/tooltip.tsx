// shadcn/ui tooltip (new-york) over Radix, with four departures from
// stock: imports name the individual `@radix-ui/react-*` package (what
// the rest of this app already depends on); the enter/exit motion is fed
// by our tokens rather than tw-animate-css's defaults (`duration-*` and
// `ease-*` set the very variables `animate-in` reads); it wears the
// popover surface instead of inverted ink, so it belongs to the same
// paper as every other floating thing here; and it carries no arrow —
// the rail label is a name, not a callout.
//
// **A tooltip waits, then follows** (the audit fix plan F6.1,
// 2026-10-03): about 600ms before the first name appears, as a Mac does,
// so a pointer passing over the window does not shower it with labels;
// once one is showing, its neighbours answer at once for 300ms
// (`skipDelayDuration`), so reading along a row of glyphs is not a wait
// per glyph. ⚠️ 墓碑: `delayDuration = 0` — every pass of the pointer
// lit a label.
import * as React from "react";
import * as TooltipPrimitive from "@radix-ui/react-tooltip";

import { cn } from "@/lib/utils";

function TooltipProvider({
  delayDuration = 600,
  skipDelayDuration = 300,
  ...props
}: React.ComponentProps<typeof TooltipPrimitive.Provider>) {
  return (
    <TooltipPrimitive.Provider
      data-slot="tooltip-provider"
      delayDuration={delayDuration}
      skipDelayDuration={skipDelayDuration}
      {...props}
    />
  );
}

function Tooltip({ ...props }: React.ComponentProps<typeof TooltipPrimitive.Root>) {
  return <TooltipPrimitive.Root data-slot="tooltip" {...props} />;
}

function TooltipTrigger({ ...props }: React.ComponentProps<typeof TooltipPrimitive.Trigger>) {
  return <TooltipPrimitive.Trigger data-slot="tooltip-trigger" {...props} />;
}

function TooltipContent({
  className,
  sideOffset = 6,
  children,
  ...props
}: React.ComponentProps<typeof TooltipPrimitive.Content>) {
  return (
    <TooltipPrimitive.Portal>
      <TooltipPrimitive.Content
        data-slot="tooltip-content"
        sideOffset={sideOffset}
        className={cn(
          // docs/design-system components/Tooltip: as wide as its word
          // (`w-max`, at most 256), `caption` on `popover` with a hairline,
          // `radius-sm`, and the `float` shadow — it is a thing that floats.
          "z-50 w-max max-w-64 origin-(--radix-tooltip-content-transform-origin) rounded-sm border bg-popover px-in py-xxs text-caption text-balance text-popover-foreground shadow-(--shadow-float)",
          "duration-[var(--dur-120)] ease-[var(--ease-out)]",
          "animate-in fade-in-0 zoom-in-95 data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95",
          className,
        )}
        {...props}
      >
        {children}
      </TooltipPrimitive.Content>
    </TooltipPrimitive.Portal>
  );
}

export { Tooltip, TooltipTrigger, TooltipContent, TooltipProvider };
