// TextField (docs/design-system components/TextField, 2026-09-28): the
// one box a person types into, **the search field's family** — `fill`
// ground, no border and no shadow, `radius-sm`, `ctl-md` tall, `body` in
// `ink` with a `tertiary` placeholder, and a 2 px `ring` drawn flush
// outside it while it has focus (the switch's and the segmented control's
// ring). A search field is this box with a magnifier at its left
// (`pl-7`, components/SearchField).
//
// It was shadcn's input as shipped: a `rounded-md` corner, an `input`
// border, Tailwind's own `shadow-xs`, a 3 px half-strength glow for focus
// and a step down to `callout` at the wide breakpoint — the one control
// in a sheet that did not wear the `fill` ground its neighbours (the
// popup button's field, the segmented control's track, the search box)
// all do.
//
// Disabled is the push button's 0.4 and **the arrow stays**: AppKit does
// not swap the cursor over a disabled control (`check-face` §11b).
// `min-h-touch` is the hit floor (`--hit`: the control height with a
// mouse, the finger's 44 with a finger); the look does not change.
import * as React from "react";

import { cn } from "@/lib/utils";

function Input({ className, type, ...props }: React.ComponentProps<"input">) {
  return (
    <input
      type={type}
      data-slot="input"
      className={cn(
        "h-ctl-md min-h-touch w-full min-w-0 rounded-sm border-0 bg-fill px-in text-body text-foreground placeholder:text-tertiary disabled:pointer-events-none disabled:opacity-40",
        "focus-visible:outline-2 focus-visible:outline-offset-0 focus-visible:outline-ring",
        "aria-invalid:outline-2 aria-invalid:outline-offset-0 aria-invalid:outline-destructive",
        className,
      )}
      {...props}
    />
  );
}

export { Input };
