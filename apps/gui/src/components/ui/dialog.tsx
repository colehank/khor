// shadcn/ui dialog (new-york) over Radix. Departures from stock: imports
// name the individual `@radix-ui/react-*` package; the close mark is our
// own `IconClose` rather than the stock one; motion rides our tokens; and the
// close button's name comes from the catalog, because "Close" spelled in
// a component is a user-facing word living outside zh.toml.
import * as React from "react";
import * as DialogPrimitive from "@radix-ui/react-dialog";

import { IconClose } from "@/components/icons";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";

function Dialog({ ...props }: React.ComponentProps<typeof DialogPrimitive.Root>) {
  return <DialogPrimitive.Root data-slot="dialog" {...props} />;
}

function DialogTrigger({ ...props }: React.ComponentProps<typeof DialogPrimitive.Trigger>) {
  return <DialogPrimitive.Trigger data-slot="dialog-trigger" {...props} />;
}

function DialogClose({ ...props }: React.ComponentProps<typeof DialogPrimitive.Close>) {
  return <DialogPrimitive.Close data-slot="dialog-close" {...props} />;
}

function DialogOverlay({
  className,
  ...props
}: React.ComponentProps<typeof DialogPrimitive.Overlay>) {
  return (
    <DialogPrimitive.Overlay
      data-slot="dialog-overlay"
      className={cn(
        "fixed inset-0 z-50 bg-black/50",
        "duration-[var(--dur-180)] ease-[var(--ease-out)]",
        "data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=closed]:animate-out data-[state=closed]:fade-out-0",
        className,
      )}
      {...props}
    />
  );
}

function DialogContent({
  className,
  children,
  sheet = false,
  ...props
}: React.ComponentProps<typeof DialogPrimitive.Content> & {
  /** **A sheet, not a dialog** (docs/SHELL.md 第十七轮 §4, D10): a
      create or import form drops from R0's lower edge — its top is R0's
      bottom, its width no more than the board's, and nothing dims the
      window (no overlay). Esc and the close mark close it the way they
      close a dialog. A confirmation that destroys something stays a
      dialog. */
  sheet?: boolean;
}) {
  // The sheet hangs inside the right side (`[data-main]`, positioned by
  // the shell), so `top-pane-head` is R0's bottom whichever page is up.
  // With no right side (the narrow face) it hangs from the window's
  // first row instead, which is the same height.
  const main =
    sheet && typeof document !== "undefined" ? document.querySelector<HTMLElement>("[data-main]") : null;
  return (
    <DialogPrimitive.Portal data-slot="dialog-portal" container={main ?? undefined}>
      {!sheet && <DialogOverlay />}
      <DialogPrimitive.Content
        data-slot="dialog-content"
        data-sheet={sheet ? "" : undefined}
        // Radix asks for a description or an explicit opt-out. Our
        // dialogs opt out by default: a paragraph explaining what the
        // dialog is for is exactly the decorative copy docs/UX.md bans.
        // Callers that genuinely have one can pass their own.
        aria-describedby={undefined}
        // A sheet leaves the rest of the window live, so a press outside
        // it is not a request to close it.
        onInteractOutside={sheet ? (e) => e.preventDefault() : undefined}
        className={cn(
          sheet
            ? cn(
                "top-pane-head inset-x-in z-50 mx-auto grid max-w-lg gap-4 rounded-b-lg border border-t-0 bg-card p-6 shadow-lg outline-none",
                main ? "absolute" : "fixed",
                "duration-[var(--dur-180)] ease-[var(--ease-out)]",
                "data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=closed]:animate-out data-[state=closed]:fade-out-0",
              )
            : cn(
                // components/Dialog: `dialog-w` 380, `popover` under `float` and
                // a hairline, `radius-lg`, parts `gap-row` apart.
                "fixed top-1/2 left-1/2 z-50 grid w-dialog max-w-[calc(100%-2rem)] -translate-x-1/2 -translate-y-1/2 gap-row rounded-lg border bg-popover p-pane shadow-(--shadow-float) outline-none",
                "duration-[var(--dur-180)] ease-[var(--ease-out)]",
                "data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-95 data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95",
              ),
          className,
        )}
        {...props}
        // **An open list inside it takes the first Escape.** Radix hears
        // Escape on the document before the focused box does, so a
        // combobox showing its suggestions (`PathField`) could not close
        // them without closing the whole sheet. `aria-expanded` is the
        // box's own statement that a list is open.
        onEscapeKeyDown={(e) => {
          const t = e.target instanceof Element ? e.target : null;
          if (t?.closest('[role="combobox"][aria-expanded="true"]')) {
            e.preventDefault();
            return;
          }
          props.onEscapeKeyDown?.(e);
        }}
      >
        {children}
        {/* **The box is a finger's, the glyph stays a glyph.** This was
            the smallest target on the whole face — 16×16, the icon's own
            size — and it is the one control every sheet has. The box
            grows to `--touch` and centres the mark inside it; the inset
            drops by the same amount the box gained on each side, so the
            cross does not move a pixel from where it has always sat.
            (`top-4 right-4` around a 16 box put the mark's centre 24 in
            from both edges; the inset is `24 − box/2` so the mark stays
            there whatever the box is — `--hit` is 28 under a mouse and
            44 under a finger since 2026-09-26, and a fixed inset moved
            the cross 8 into the corner when the box shrank.)

            The feedback is on the glyph rather than a plate behind it,
            and that is not the trap tokens.css warns about: hovering
            anywhere in the grown box brightens the mark, so the whole
            pressable area does announce itself. A plate would be a
            second kind of hover mark on a surface that has none.

            **What this does not fix:** every sheet's own controls. This
            is the vendored shell, and a dialog body that draws a 24-high
            button is still a 24-high button — the face gate walks the
            sheets and says so. */}
        {sheet && (
        <DialogPrimitive.Close
          data-slot="dialog-close"
          aria-label={gui.close}
          className="absolute top-[calc(24px-var(--hit)/2)] right-[calc(24px-var(--hit)/2)] grid size-touch place-items-center rounded-sm text-muted-foreground opacity-70 transition-opacity hover:opacity-100 focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none disabled:pointer-events-none [&_svg]:pointer-events-none [&_svg]:shrink-0"
        >
          <IconClose />
        </DialogPrimitive.Close>
        )}
      </DialogPrimitive.Content>
    </DialogPrimitive.Portal>
  );
}

function DialogHeader({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="dialog-header"
      className={cn("flex flex-col gap-row text-left", className)}
      {...props}
    />
  );
}

function DialogFooter({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="dialog-footer"
      className={cn("mt-xxs flex flex-row justify-end gap-in [&_button]:h-ctl-lg", className)}
      {...props}
    />
  );
}

function DialogTitle({ className, ...props }: React.ComponentProps<typeof DialogPrimitive.Title>) {
  return (
    <DialogPrimitive.Title
      data-slot="dialog-title"
      className={cn("text-title-md font-title", className)}
      {...props}
    />
  );
}

function DialogDescription({
  className,
  ...props
}: React.ComponentProps<typeof DialogPrimitive.Description>) {
  return (
    <DialogPrimitive.Description
      data-slot="dialog-description"
      className={cn("text-body text-muted-foreground", className)}
      {...props}
    />
  );
}

export {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogOverlay,
  DialogTitle,
  DialogTrigger,
};
