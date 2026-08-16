// shadcn/ui tooltip (new-york) over Radix, with four departures from
// stock: imports name the individual `@radix-ui/react-*` package (what
// the rest of this app already depends on); the enter/exit motion is fed
// by our tokens rather than tw-animate-css's defaults (`duration-*` and
// `ease-*` set the very variables `animate-in` reads); it wears the
// popover surface instead of inverted ink, so it belongs to the same
// paper as every other floating thing here; and it carries no arrow —
// the rail label is a name, not a callout.
//
// `delayDuration = 0` is a product decision, not a default worth losing:
// the rail is icon-only, so the name is the only thing that says where a
// glyph goes. A tooltip that waits is a name that is not there.
import * as React from "react";
import * as TooltipPrimitive from "@radix-ui/react-tooltip";

import { cn } from "@/lib/utils";

function TooltipProvider({
  delayDuration = 0,
  ...props
}: React.ComponentProps<typeof TooltipPrimitive.Provider>) {
  return (
    <TooltipPrimitive.Provider
      data-slot="tooltip-provider"
      delayDuration={delayDuration}
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
          "z-50 w-fit origin-(--radix-tooltip-content-transform-origin) rounded-md border bg-popover px-2 py-1 text-xs text-balance text-popover-foreground shadow-md",
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
