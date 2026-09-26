// shadcn/ui button (new-york), unmodified apart from the size scale
// riding our control-height tokens. Pages compose variants; they never
// restyle heights locally — that is how mandala grew four button heights.
import * as React from "react";
import { Slot } from "@radix-ui/react-slot";
import { cva, type VariantProps } from "class-variance-authority";

import { cn } from "@/lib/utils";

const buttonVariants = cva(
  // The focus ring is an **outline with an offset**, not the stock
  // `ring-1` sitting flush against the edge. Measured reason: the ring
  // and the primary button are the same colour — both are the theme's
  // ink (light) or paper (dark) — so a ring drawn tight against that
  // button contrasts 1.00 with it and cannot be seen at all. That was
  // true of the brand green it replaces too, in *both* themes, so this
  // is not a new problem introduced by the palette change; it is one the
  // palette change stopped hiding behind a colour that was unreadable
  // anyway.
  //
  // An outline rather than `ring-offset-*`: the gap an outline leaves is
  // transparent, so it shows whatever the button actually sits on. A
  // ring offset has to be *told* its colour, and a button on a card
  // would then get a gap painted in the page's colour instead of the
  // card's — right in the one place the ring is hardest to see.
  // docs/design-system components/Button (2026-09-26): `radius-sm` 6,
  // `body` 13 at 400, parts `gap-xxs` apart, 0.4 when disabled, a press
  // that gives `--press` (app.css) — and no shadow on any of them.
  "inline-flex items-center justify-center gap-xxs whitespace-nowrap rounded-sm text-body font-body focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring disabled:pointer-events-none disabled:opacity-40 [&_svg]:pointer-events-none [&_svg]:shrink-0",
  {
    variants: {
      variant: {
        // An ordinary push button (第十七轮, docs/design-system
        // components/Button): `fill-strong` under `foreground`. Since
        // 第十五轮 `--primary` is `--brand-strong`, and a default that
        // painted it made every unmarked button on every page a solid
        // olive block — the brand voltage in a place it is not allowed.
        default: "bg-fill-strong text-foreground hover:bg-fill-strong/80",
        // **The** one primary act of a dialog or a sheet, and nowhere
        // else: at most one per dialog, never two in a row. `check-face`
        // (第十七轮) goes red on a primary outside `[role=dialog]` /
        // `[data-sheet]`, and on two in one.
        primary: "bg-primary text-primary-foreground hover:bg-primary/90",
        secondary: "bg-secondary text-secondary-foreground hover:bg-fill-strong",
        // `plain`: no ground, `secondary` ink, `fill` under the pointer.
        ghost: "text-muted-foreground hover:bg-fill hover:text-foreground",
        destructive: "bg-destructive text-destructive-foreground hover:bg-destructive/90",
                link: "text-primary underline-offset-4 hover:underline",
      },
      // **None of these carries the finger floor, and that is on
      // purpose.** These three are the *look* of an inline control
      // beside text (`--ctl-*`); `--touch` is a different axis and
      // tokens.css says so in as many words — a control that a finger
      // must land on wears `min-h-touch` / `min-w-touch` at the call
      // site, and `min-height` beats `height`, so it lands even though
      // `cn()` cannot take an `h-ctl-*` off (see `auto` below for that
      // scar). Baking the floor into `sm` would regrow every small
      // button in the app — including the ones deliberately inline in a
      // sentence — to fix the handful that are targets. The rendered
      // gate (`scripts/check-face.mjs` §3) is what finds the ones whose
      // author forgot; this scale is not the place to guess.
      size: {
        default: "h-ctl-md px-row",
        sm: "h-ctl-sm px-in text-callout",
        lg: "h-ctl-lg px-base",
        icon: "h-ctl-md w-ctl-md",
        // Sized by what is inside it — for a button that stacks a mark
        // over a word, where the three fixed heights are all too short.
        //
        // **It has to be a size here rather than an `h-auto` from the
        // caller**, and that is measured, not preference: `cn()` cannot
        // take `h-ctl-md` off, because tailwind-merge does not read
        // `ctl-md` as a height and so keeps *both* classes. The one that
        // wins is then whichever the stylesheet emits last. Measured on
        // the settings screen: computed height 30px against a
        // scrollHeight of 40, which on a wrapping row laid one row's
        // words underneath the next row's marks.
        //
        // The same trap is waiting for any caller trying to override a
        // `ctl-*` or `rail`/`list`/`avatar` sized utility through
        // `className`; it only shows when something inside overflows.
        auto: "h-auto",
      },
    },
    defaultVariants: {
      variant: "default",
      size: "default",
    },
  },
);

function Button({
  className,
  variant,
  size,
  asChild = false,
  ...props
}: React.ComponentProps<"button"> &
  VariantProps<typeof buttonVariants> & {
    asChild?: boolean;
  }) {
  const Comp = asChild ? Slot : "button";
  return (
    <Comp
      data-slot="button"
      className={cn(buttonVariants({ variant, size, className }))}
      {...props}
    />
  );
}

export { Button, buttonVariants };
