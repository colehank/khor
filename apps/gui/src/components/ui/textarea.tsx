// shadcn/ui textarea (new-york), with two changes and one omission.
//
// The changes are the same one every control here makes: the floor is a
// control-height token rather than a literal, and the ceiling is
// `--chat-box` (tokens.css) so a box that grows still stops.
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
        "min-h-ctl-md max-h-chat-box w-full min-w-0 resize-none overflow-y-auto rounded-md border border-input bg-transparent px-3 py-1.5 text-base shadow-xs transition-[color,box-shadow] outline-none selection:bg-primary selection:text-primary-foreground placeholder:text-muted-foreground disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-50 md:text-sm",
        "focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-2",
        "aria-invalid:border-destructive aria-invalid:ring-destructive/20",
        className,
      )}
      {...props}
    />
  );
}

export { Textarea };
