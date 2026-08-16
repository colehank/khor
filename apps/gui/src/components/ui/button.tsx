// shadcn/ui button (new-york), unmodified apart from the size scale
// riding our control-height tokens. Pages compose variants; they never
// restyle heights locally — that is how mandala grew four button heights.
import * as React from "react";
import { Slot } from "@radix-ui/react-slot";
import { cva, type VariantProps } from "class-variance-authority";

import { cn } from "@/lib/utils";

const buttonVariants = cva(
  "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0",
  {
    variants: {
      variant: {
        default: "bg-primary text-primary-foreground shadow hover:bg-primary/90",
        secondary: "bg-secondary text-secondary-foreground shadow-sm hover:bg-secondary/80",
        ghost: "hover:bg-secondary hover:text-foreground",
        destructive: "bg-destructive text-destructive-foreground shadow-sm hover:bg-destructive/90",
        outline: "border border-border bg-transparent shadow-sm hover:bg-secondary",
        link: "text-primary underline-offset-4 hover:underline",
      },
      size: {
        default: "h-ctl-md px-4 py-2",
        sm: "h-ctl-sm rounded-md px-3 text-xs",
        lg: "h-ctl-lg rounded-md px-8",
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
