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
import { overlapsNativePage } from "@/lib/native-page-yield";

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

function TooltipTrigger({ onFocus, ...props }: React.ComponentProps<typeof TooltipPrimitive.Trigger>) {
  return (
    <TooltipPrimitive.Trigger
      data-slot="tooltip-trigger"
      // **Focus opens a tooltip only when the keyboard put it there**
      // (the owner, 2026-10-12: press ＋, press elsewhere to fold its menu,
      // and ＋ shows its label). Radix opens on any focus that no pointer
      // press on the trigger caused — and a menu closing hands focus back
      // to its trigger programmatically, which is exactly that. The root
      // already says which hand is on the app (`lib/modality`); under a
      // pointer the focus is a side effect, not a request, so the open
      // is declined (Radix skips its own handler once the event is
      // `defaultPrevented`). Tab onto the control still opens it.
      onFocus={(e) => {
        onFocus?.(e);
        if (document.documentElement.dataset.modality !== "keyboard") e.preventDefault();
      }}
      {...props}
    />
  );
}

function TooltipContent({
  className,
  sideOffset = 6,
  children,
  ref: forwardedRef,
  style,
  ...props
}: React.ComponentProps<typeof TooltipPrimitive.Content>) {
  const element = React.useRef<HTMLDivElement | null>(null);
  const [node, setNode] = React.useState<HTMLDivElement | null>(null);
  const [suppressed, setSuppressed] = React.useState(true);
  const attach = React.useCallback((el: HTMLDivElement | null) => {
    element.current = el; setNode(el);
    if (typeof forwardedRef === "function") forwardedRef(el);
    else if (forwardedRef) forwardedRef.current = el;
  }, [forwardedRef]);
  const measure = React.useCallback(() => {
    if (element.current) setSuppressed(overlapsNativePage(element.current));
  }, []);
  React.useLayoutEffect(() => {
    const el = element.current;
    if (!el) return;
    measure();
    const resized = new ResizeObserver(measure);
    resized.observe(el);
    const changed = new MutationObserver(measure);
    // Radix positions the wrapper after its first layout. Native page
    // restoration can also make a previously safe hint overlap again.
    changed.observe(document.body, {subtree:true, childList:true, attributes:true,
      attributeFilter:["style","data-native-page-visible"]});
    window.addEventListener("resize", measure);
    document.addEventListener("scroll", measure, true);
    return () => { resized.disconnect(); changed.disconnect(); window.removeEventListener("resize", measure); document.removeEventListener("scroll", measure, true); };
  }, [node, measure]);
  return (
    <TooltipPrimitive.Portal>
      <TooltipPrimitive.Content
        ref={attach}
        style={{...style, ...(suppressed ? {visibility:"hidden" as const} : {})}}
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
