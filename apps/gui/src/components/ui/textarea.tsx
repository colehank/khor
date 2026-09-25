// shadcn/ui textarea (new-york), with three changes and one omission.
//
// Two of the changes are the one every control here makes: the floor is
// a control-height token rather than a literal, and the ceiling is
// `--chat-box` (tokens.css) so a box that grows still stops.
//
// **The third is that it no longer names a size at all.** Stock ships
// `text-body md:text-callout`, a step down at the wide breakpoint, and this
// app is the wrong home for it twice over:
//
//   - **The step buys nothing it was designed to buy.** Upstream it is
//     an iOS guard — mobile Safari zooms the page when a field under 16
//     takes focus, and stock's `text-body` *is* 16. This app's scale
//     redefines that token to 14 (tokens.css), so the narrow branch was
//     already under the threshold and the zoom it exists to prevent was
//     never being prevented.
//   - **It made a person write in a size two steps under the one they
//     were reading.** Measured on the conversation before this: the
//     column read at 15 while the box computed 12.5 — and the box had
//     already been brought into line with the column's *width*, which
//     is what made the difference in size the louder of the two.
//
// **Overriding it from the caller was not an option, which is the part
// worth writing down.** `cn()` cannot beat `md:text-callout`: tailwind-merge
// files a bare `text-read` and a `md:`-prefixed size in different
// groups, so both survive into the markup and the breakpoint one wins
// above 768 — the same family of silent loss as the custom spacing
// classes. A caller's override would have looked right in a narrow
// window and done nothing in the posture the problem lives in.
//
// So the size is inherited now, which is also the honest thing for a
// textarea to do: it writes in whatever size it was put inside. The
// conversation's composer sits in the reading column and gets 15 with
// the leading that size was chosen with; anywhere else it gets the
// shell's base, the way every other unstyled run does.
//
// The omission is `field-sizing: content`, which is how stock shadcn
// grows this box. **It is not available where this app actually runs**:
// the desktop shell is a WKWebView, and a growth rule the browser
// silently ignores would leave the box one line tall in the app while
// looking correct in the Chrome the verification drives. The caller
// grows it by measuring instead (`ChatView`).
import * as React from "react";

import { cn } from "@/lib/utils";

function Textarea({ className, ...props }: React.ComponentProps<"textarea">) {
  return (
    <textarea
      data-slot="textarea"
      className={cn(
        "min-h-ctl-md max-h-chat-box w-full min-w-0 resize-none overflow-y-auto rounded-md border border-input bg-transparent px-3 py-1.5 shadow-xs transition-[color,box-shadow] outline-none selection:bg-primary selection:text-primary-foreground placeholder:text-muted-foreground disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-50",
        "focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-2",
        "aria-invalid:border-destructive aria-invalid:ring-destructive/20",
        className,
      )}
      {...props}
    />
  );
}

export { Textarea };
