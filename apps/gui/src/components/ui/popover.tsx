// shadcn/ui popover (new-york) over Radix. Departures from stock:
// imports name the individual `@radix-ui/react-*` package (what the rest
// of this app already depends on); it wears the one floating language
// (docs/design-system guidelines/20-states 浮层: `popover` + the `float`
// shadow + `radius-lg`, a hairline around it) instead of stock's
// `rounded-md shadow-md`; motion rides our tokens; and the parts this app
// has no use for yet (a close button) are left out rather than vendored
// unused. The anchor apart from the trigger came with UX-36: the day
// hangs from a session's pill, which its menu opens, not a press on it.
//
// **Non-modal, and that is Radix's default rather than a choice made
// here**: nothing behind it is inert, focus is not trapped, a press
// outside or Esc closes it, and focus goes back to the trigger that
// opened it. It holds facts to read, not a form — a trap would be the
// wrong promise.
import * as React from "react";
import * as PopoverPrimitive from "@radix-ui/react-popover";

import { cn } from "@/lib/utils";

function Popover({ ...props }: React.ComponentProps<typeof PopoverPrimitive.Root>) {
  return <PopoverPrimitive.Root data-slot="popover" {...props} />;
}

function PopoverTrigger({ ...props }: React.ComponentProps<typeof PopoverPrimitive.Trigger>) {
  return <PopoverPrimitive.Trigger data-slot="popover-trigger" {...props} />;
}

function PopoverContent({
  className,
  align = "center",
  sideOffset = 6,
  ...props
}: React.ComponentProps<typeof PopoverPrimitive.Content>) {
  return (
    <PopoverPrimitive.Portal>
      <PopoverPrimitive.Content
        data-slot="popover-content"
        align={align}
        sideOffset={sideOffset}
        className={cn(
          "z-50 w-max max-w-64 origin-(--radix-popover-content-transform-origin) rounded-lg border bg-popover px-row py-in text-popover-foreground shadow-(--shadow-float) outline-none",
          "duration-[var(--dur-120)] ease-[var(--ease-out)]",
          "data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-95 data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95",
          className,
        )}
        {...props}
      />
    </PopoverPrimitive.Portal>
  );
}

function PopoverAnchor({ ...props }: React.ComponentProps<typeof PopoverPrimitive.Anchor>) {
  return <PopoverPrimitive.Anchor data-slot="popover-anchor" {...props} />;
}

export { Popover, PopoverTrigger, PopoverAnchor, PopoverContent };
