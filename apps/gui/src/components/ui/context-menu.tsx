// shadcn/ui context-menu (new-york) over Radix, vendored the same way
// `dropdown-menu.tsx` was and departing from stock in the same places:
// imports name the individual `@radix-ui/react-*` package, motion rides
// our tokens, and the parts this app has no use for (submenus, checkbox
// and radio items, shortcut) are left out rather than vendored unused.
//
// **Why a second menu primitive at all**, when the dropdown is already
// here: the trigger is the whole difference. A dropdown opens from a
// button you can see; this one opens from `contextmenu` — and, on a
// touch screen, from a 700ms press with the callout suppressed, which
// is the half a dropdown has no answer for. Both sit on the same
// `@radix-ui/react-menu` underneath, so the menu itself — roving focus,
// typeahead, escape, dismiss, collision flipping — is one implementation
// wearing two triggers, not two.
import * as React from "react";
import * as ContextMenuPrimitive from "@radix-ui/react-context-menu";

import { cn } from "@/lib/utils";

function ContextMenu({ ...props }: React.ComponentProps<typeof ContextMenuPrimitive.Root>) {
  return <ContextMenuPrimitive.Root data-slot="context-menu" {...props} />;
}

function ContextMenuTrigger({
  ...props
}: React.ComponentProps<typeof ContextMenuPrimitive.Trigger>) {
  return <ContextMenuPrimitive.Trigger data-slot="context-menu-trigger" {...props} />;
}

function ContextMenuContent({
  className,
  ...props
}: React.ComponentProps<typeof ContextMenuPrimitive.Content>) {
  return (
    <ContextMenuPrimitive.Portal>
      <ContextMenuPrimitive.Content
        data-slot="context-menu-content"
        className={cn(
          "z-50 max-h-(--radix-context-menu-content-available-height) max-w-64 min-w-40 origin-(--radix-context-menu-content-transform-origin) overflow-x-hidden overflow-y-auto rounded-md border bg-popover p-1 text-popover-foreground shadow-md",
          "duration-[var(--dur-120)] ease-[var(--ease-out)]",
          "data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-95 data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95",
          className,
        )}
        {...props}
      />
    </ContextMenuPrimitive.Portal>
  );
}

function ContextMenuItem({
  className,
  ...props
}: React.ComponentProps<typeof ContextMenuPrimitive.Item>) {
  return (
    <ContextMenuPrimitive.Item
      data-slot="context-menu-item"
      className={cn(
        "relative flex cursor-default items-center gap-2 rounded-sm px-2 py-1.5 text-callout outline-hidden select-none focus:bg-accent focus:text-accent-foreground data-[disabled]:pointer-events-none data-[disabled]:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0",
        className,
      )}
      {...props}
    />
  );
}

function ContextMenuLabel({
  className,
  ...props
}: React.ComponentProps<typeof ContextMenuPrimitive.Label>) {
  return (
    <ContextMenuPrimitive.Label
      data-slot="context-menu-label"
      // Departs from stock in two places, both because this label carries
      // a sentence rather than a heading: no weight of its own (a heading
      // is what stock uses it for), and it wraps — a warning truncated to
      // one line is a warning nobody reads.
      className={cn("px-2 py-1.5 text-callout whitespace-normal", className)}
      {...props}
    />
  );
}

function ContextMenuSeparator({
  className,
  ...props
}: React.ComponentProps<typeof ContextMenuPrimitive.Separator>) {
  return (
    <ContextMenuPrimitive.Separator
      data-slot="context-menu-separator"
      className={cn("-mx-1 my-1 h-px bg-border", className)}
      {...props}
    />
  );
}

export {
  ContextMenu,
  ContextMenuTrigger,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuLabel,
  ContextMenuSeparator,
};
