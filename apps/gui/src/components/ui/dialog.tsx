// One Radix frame; task intent chooses responsive presentation through
// overlay-policy. Pages supply content, never their own sheet gestures.
import * as React from "react";
import { keyboardCover } from "@/lib/viewport";
import { overlayPolicy, type OverlayOptions, type TaskPurpose } from "@/lib/overlay-policy";
import { useWindowNarrow } from "@/hooks/use-narrow";
import { useTaskOverlay } from "@/hooks/use-task-overlay";
import { gui } from "@/gen/catalog";
import * as DialogPrimitive from "@radix-ui/react-dialog";

import { Fault } from "@/components/Fault";
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
      // **A tap on the scrim has to arrive as a click on iOS** (UX-21):
      // Radix closes on a touch outside only when the click that follows
      // reaches the document, and WebKit sends no click for a tap on an
      // element nothing listens to — so every sheet on the phone ignored
      // a tap beside it. A listener of its own (React sets `onclick`)
      // makes the scrim a thing a tap clicks.
      onClick={() => {}}
      data-scrim
      // **One scrim** (UX-44 §2, the owner 2026-10-07: 「都用标签列表的深
      // 灰」): every sheet's and dialog's backdrop is `--sheet-scrim`, the
      // 附栏's included; it darkens in both themes (UX-49 §1). ⚠️ 墓碑:
      // `bg-black/50` here, and the 附栏 alone on the token.
      className={cn(
        "fixed inset-0 z-50 bg-(--sheet-scrim)",
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
  purpose = "confirmation",
  initialHeight,
  maxHeight,
  dismiss: dismissMode,
  scrollable = false,
  ...props
}: React.ComponentProps<typeof DialogPrimitive.Content> & Omit<OverlayOptions,"purpose"> & {
  purpose?: OverlayOptions["purpose"];
  /** Long tasks put their one scroller in DialogBody. */
  scrollable?: boolean;
}) {
  const narrow=useWindowNarrow();
  const policy=overlayPolicy({purpose,initialHeight,maxHeight,dismiss:dismissMode},narrow);

  // **Focus goes back to what opened it** (the audit fix plan F3.7; W20):
  // most of these are opened from a menu or by state, not by a Radix
  // trigger, so Radix had nothing to return to and focus fell to the
  // page. What held focus as the dialog came up is kept and refocused.
  // **Read when it opens** (W-V3), not when this component first drew:
  // the dialog's wrapper is drawn with the page, long before anybody
  // opens it, and what held focus then was the page itself.
  const opener = React.useRef<HTMLElement | null>(null);
  const remember = () => {
    const at = document.activeElement;
    // Opened from a menu's item: the menu is going away, so its trigger.
    const menu = at instanceof HTMLElement ? at.closest<HTMLElement>('[role="menu"]') : null;
    const trigger = menu?.id ? document.querySelector<HTMLElement>(`[aria-controls="${CSS.escape(menu.id)}"]`) : null;
    opener.current = trigger ?? (at instanceof HTMLElement && at !== document.body ? at : null);
  };
  const { onCloseAutoFocus, onOpenAutoFocus, onInteractOutside, ...rest } = props;
  const close = React.useRef<HTMLButtonElement>(null);
  // State, not a ref: the content mounts when the dialog opens, after
  // this component did, and the sheet's listeners need that moment.
  const [sheet, setSheet] = React.useState<HTMLDivElement | null>(null);
  const dismiss = React.useCallback(() => close.current?.click(), []);
  // Every dialog follows the visible viewport, including short alerts that
  // may open while a keyboard is already up. Never use innerHeight as the
  // keyboard cover on WKWebView (lib/viewport).
  React.useLayoutEffect(() => {
    if (!sheet) return;
    const viewport = window.visualViewport;
    const sync = () => {
      sheet.style.setProperty("--dialog-visible-height", `${viewport?.height ?? window.innerHeight}px`);
      sheet.style.setProperty("--dialog-visible-top", `${viewport?.offsetTop ?? 0}px`);
      sheet.style.setProperty("--sheet-kb", `${keyboardCover()}px`);
    };
    sync();
    window.addEventListener("resize", sync);
    viewport?.addEventListener("resize", sync);
    viewport?.addEventListener("scroll", sync);
    return () => {
      window.removeEventListener("resize", sync);
      viewport?.removeEventListener("resize", sync);
      viewport?.removeEventListener("scroll", sync);
    };
  }, [sheet]);
  const taskSheet=useTaskOverlay(sheet,policy,dismiss);
  return (
    <DialogPrimitive.Portal data-slot="dialog-portal">
      <DialogOverlay />
      <DialogPrimitive.Content
        ref={setSheet}
        data-slot="dialog-content"
        data-overlay-purpose={purpose}
        data-overlay-task={policy.task ? "" : undefined}
        data-overlay-presentation={policy.presentation}
        data-overlay-initial={policy.initial}
        data-overlay-max={policy.maximum}
        data-detent={policy.presentation === "sheet" ? taskSheet.detent : undefined}
        data-scrollable={scrollable ? "" : undefined}
        onOpenAutoFocus={(e) => {
          remember();
          if (!policy.focus) {
            e.preventDefault();
            (e.target instanceof HTMLElement?e.target:sheet)?.focus({preventScroll:true});
          }
          onOpenAutoFocus?.(e);
          if (policy.task && policy.focus && !e.defaultPrevented) {
            const root=e.target instanceof HTMLElement?e.target:sheet;
            const field=[...(root?.querySelectorAll<HTMLElement>('input,textarea,[contenteditable="true"]')??[])].find(field=>{
              const hidden=field.closest('[aria-hidden="true"]');
              return field.getClientRects().length>0&&!field.matches(':disabled,[type="hidden"]')&&getComputedStyle(field).visibility!=='hidden'&&(!hidden||hidden===root||!root?.contains(hidden));
            });
            if(field){e.preventDefault();field.focus();}
          }
        }}
        onCloseAutoFocus={(e) => {
          onCloseAutoFocus?.(e);
          if (e.defaultPrevented) return;
          // A task can replace another task (catalog → manage → edit).
          // The departing frame must not steal focus from its replacement.
          if ([...document.querySelectorAll('[data-slot="dialog-content"][data-state="open"]')].some(next=>next!==e.target&&next!==sheet)) {
            e.preventDefault();
            return;
          }
          const back = opener.current;
          if (back && back.isConnected) {
            e.preventDefault();
            back.focus();
          }
        }}
        // Radix asks for a description or an explicit opt-out. Our
        // dialogs opt out by default: a paragraph explaining what the
        // dialog is for is exactly the decorative copy docs/UX.md bans.
        // Callers that genuinely have one can pass their own.
        aria-describedby={undefined}
        onInteractOutside={(e) => {
          const target=e.detail.originalEvent.target;
          const scrim=target instanceof Element && !!target.closest('[data-scrim]');
          if (!policy.outside || (policy.presentation==='sheet' && !scrim)) e.preventDefault();
          onInteractOutside?.(e);
        }}
        className={cn(
          // components/Dialog: `dialog-w` 380 for a question, 32rem for a
          // form (the width the retired sheet had, which the two-column
          // forms were laid out in); `popover` under `float` and a
          // hairline, `radius-lg`, parts `gap-row` apart. **It floats, so
          // it wears what floats**, never the board's `card` (⚠️ 墓碑
          // 2026-09-28: on the dark theme `card` *is* the board, and a form
          // on it sat flush, the same colour — `check-face` 4I).
          "fixed left-1/2 z-50 grid overflow-x-hidden overflow-y-auto overscroll-contain max-w-[calc(100%-2rem)] -translate-x-1/2 gap-row rounded-lg border bg-popover p-pane shadow-(--shadow-float) outline-none",
          // **A form does not re-centre as it changes** (the audit fix plan
          // F4.5; N12): its top stays put and it grows downward in a
          // window; on the narrow face it stands on the bottom and grows
          // up, where the thumb is. A question stays centred.
          policy.task ? "top-[16vh] w-[32rem]" : "top-1/2 w-dialog -translate-y-1/2",
          policy.width === "wide" && "w-[min(56rem,calc(100%-2rem))]",
          "duration-[var(--dur-180)] ease-[var(--ease-out)]",
          "data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-95 data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95",
          className,
        )}
        {...rest}
        // **An open list inside it takes the first Escape.** Radix hears
        // Escape on the document before the focused box does, so a
        // combobox showing its suggestions (`PathField`) could not close
        // them without closing the whole dialog. `aria-expanded` is the
        // box's own statement that a list is open.
        onEscapeKeyDown={(e) => {
          if (!policy.dismissible) { e.preventDefault(); return; }
          const t = e.target instanceof Element ? e.target : null;
          if (t?.closest('[role="combobox"][aria-expanded="true"]')) {
            e.preventDefault();
            return;
          }
          props.onEscapeKeyDown?.(e);
        }}
      >
        {/* **On a phone a form is a sheet** (the audit fix plan F7.4;
            narrow P-E, `app.css` `[data-sheet-grabber]`): it rises from
            the bottom, rides above the soft keyboard, and the grabber is
            where a finger pulls it down to put it away. Drawn only on
            the narrow face. */}
        {policy.task && <button type="button" data-sheet-grabber data-resizable={taskSheet.resizable || undefined} tabIndex={taskSheet.resizable ? 0 : -1} aria-label={taskSheet.detent==='half'?gui.content_expand_sheet:gui.content_collapse_sheet} aria-hidden={!taskSheet.resizable || undefined} onClick={taskSheet.resizable?taskSheet.cycle:undefined}><span/></button> }
        {/* A dialog that fails to draw says so inside its own frame
            (`components/Fault`), and Esc still puts it away. */}
        <Fault name="dialog">{children}</Fault>
        <DialogPrimitive.Close ref={close} hidden tabIndex={-1} aria-hidden="true" />
        {/* ⚠️ 墓碑 (2026-09-30): the sheet's close mark stood here, a 16
            cross in the corner grown to a finger's box. A dialog has no
            cross (components/Dialog): its way out is 取消 left of its one
            act — the form dialogs now carry one — and Esc. */}
      </DialogPrimitive.Content>
    </DialogPrimitive.Portal>
  );
}

/** A task's public entry point: the page supplies intent and content slots;
 * the shared policy chooses dialog/sheet, focus, stops and dismissal. */
function TaskOverlay(props: Omit<React.ComponentProps<typeof DialogContent>, "purpose"> & {purpose: TaskPurpose}) {
  return <DialogContent {...props}/>;
}

/** One scrolling content region in a long dialog, between its fixed title
 * and actions. min-height:0 lets it yield to the keyboard and short windows. */
function DialogBody({ className, ...props }: React.ComponentProps<"div">) {
  return <div data-slot="dialog-body" className={cn("flex min-h-0 min-w-0 flex-1 flex-col gap-row overflow-x-hidden overflow-y-auto overscroll-contain", className)} {...props} />;
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
      // The in-content control height, as the fields beside them (W-V17):
      // 28 under a mouse, 44 under a finger (`app.css`).
      className={cn("mt-xxs flex flex-row flex-wrap justify-end gap-in [&_button]:h-ctl-md", className)}
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
  TaskOverlay,
  DialogBody,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogOverlay,
  DialogTitle,
  DialogTrigger,
};
