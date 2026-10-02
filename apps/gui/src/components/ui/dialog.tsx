// shadcn/ui dialog (new-york) over Radix. Departures from stock: imports
// name the individual `@radix-ui/react-*` package; there is no close mark
// (a dialog's way out is 取消 and Esc, components/Dialog); motion rides
// our tokens; and it is the one kind of window a press opens — centred,
// over the scrim — since the R0 sheet was retired (2026-09-30).
import * as React from "react";
import * as DialogPrimitive from "@radix-ui/react-dialog";

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
  form = false,
  ...props
}: React.ComponentProps<typeof DialogPrimitive.Content> & {
  /** **A dialog that holds a form** — 新会话, 导入, 加入网络, 登记,
      选择目录, the relay and the PIN. Two things differ from a question
      dialog and nothing else: it is wider, because a form is two columns
      (the questions right-aligned against their answers, `ui/form-grid`),
      and **a press outside it does not close it** — what was typed is not
      thrown away by a stray click; Esc, 取消 and its own button do. Same
      place, same surface, same scrim (components/Dialog). */
  form?: boolean;
}) {
  // ⚠️ 墓碑 (2026-09-30): a `sheet` variant stood here from 第十七轮 (D10)
  // — a create form dropped from R0's lower edge, portaled into the right
  // side, no scrim, only its lower corners rounded, and the one close mark
  // in the app. The owner: 「整个 app 所有点击的弹窗，比如新建 session，
  // 是要来一个弹窗，而不是现在这样从标题下长出来的奇怪东西——我说的弹窗类似
  // 机器页点 ＋ 后的邀请设备弹窗」. **One kind of window a press opens**:
  // centred, over the scrim, `popover` under `float`. Retired whole, no
  // switch left (`check-face` §13 (b) reads `src/` for it).
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
  const { onCloseAutoFocus, onOpenAutoFocus, ...rest } = props;
  const close = React.useRef<HTMLButtonElement>(null);
  // State, not a ref: the content mounts when the dialog opens, after
  // this component did, and the sheet's listeners need that moment.
  const [sheet, setSheet] = React.useState<HTMLDivElement | null>(null);
  const dismiss = React.useCallback(() => close.current?.click(), []);
  useSheet(form, sheet, dismiss);
  return (
    <DialogPrimitive.Portal data-slot="dialog-portal">
      <DialogOverlay />
      <DialogPrimitive.Content
        ref={setSheet}
        data-slot="dialog-content"
        data-form={form ? "" : undefined}
        onOpenAutoFocus={(e) => {
          remember();
          onOpenAutoFocus?.(e);
        }}
        onCloseAutoFocus={(e) => {
          onCloseAutoFocus?.(e);
          if (e.defaultPrevented) return;
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
        onInteractOutside={form ? (e) => e.preventDefault() : undefined}
        className={cn(
          // components/Dialog: `dialog-w` 380 for a question, 32rem for a
          // form (the width the retired sheet had, which the two-column
          // forms were laid out in); `popover` under `float` and a
          // hairline, `radius-lg`, parts `gap-row` apart. **It floats, so
          // it wears what floats**, never the board's `card` (⚠️ 墓碑
          // 2026-09-28: on the dark theme `card` *is* the board, and a form
          // on it sat flush, the same colour — `check-face` 4I).
          "fixed left-1/2 z-50 grid max-w-[calc(100%-2rem)] -translate-x-1/2 gap-row rounded-lg border bg-popover p-pane shadow-(--shadow-float) outline-none",
          // **A form does not re-centre as it changes** (the audit fix plan
          // F4.5; N12): its top stays put and it grows downward in a
          // window; on the narrow face it stands on the bottom and grows
          // up, where the thumb is. A question stays centred.
          form
            ? "top-[16vh] w-[32rem] [[data-narrow]_&]:top-auto [[data-narrow]_&]:bottom-[calc(var(--safe-bottom)+1rem)]"
            : "top-1/2 w-dialog -translate-y-1/2",
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
        {form && <div data-sheet-grabber aria-hidden="true" />}
        {children}
        {form && <DialogPrimitive.Close ref={close} hidden tabIndex={-1} aria-hidden="true" />}
        {/* ⚠️ 墓碑 (2026-09-30): the sheet's close mark stood here, a 16
            cross in the corner grown to a finger's box. A dialog has no
            cross (components/Dialog): its way out is 取消 left of its one
            act — the form dialogs now carry one — and Esc. */}
      </DialogPrimitive.Content>
    </DialogPrimitive.Portal>
  );
}

/**
 * The narrow sheet's two behaviours (F7.4): it rides the soft keyboard
 * (`--sheet-kb`, the part of the layout viewport the keyboard covers, from
 * `visualViewport`), and a pull down from its top quarter that goes past a
 * quarter of its height, or is thrown, puts it away; anything less springs
 * back. Inert on the wide face, where a form is a centred dialog.
 */
function useSheet(on: boolean, el: HTMLDivElement | null, dismiss: () => void) {
  React.useEffect(() => {
    if (!on || !el) return;
    const narrow = () => document.documentElement.hasAttribute("data-narrow");
    const vv = window.visualViewport;
    const ride = () => {
      if (!vv) return;
      const kb = Math.max(0, window.innerHeight - vv.height - vv.offsetTop);
      el.style.setProperty("--sheet-kb", `${kb}px`);
    };
    ride();
    vv?.addEventListener("resize", ride);
    vv?.addEventListener("scroll", ride);
    // Touch, not pointer, events: a pointer on a scrolling sheet is taken
    // by the browser's pan (`pointercancel`) before it has moved far; a
    // touch move that is prevented is the sheet's.
    let from: number | null = null;
    let dy = 0;
    let speed = 0;
    let lastY = 0;
    let lastT = 0;
    const down = (e: TouchEvent) => {
      if (!narrow() || e.touches.length !== 1) return;
      const y = e.touches[0].clientY;
      // From the grabber's band only: the rest of the sheet is a form.
      if (y - el.getBoundingClientRect().top > 28) return;
      from = y;
      lastY = y;
      lastT = e.timeStamp;
      dy = 0;
      speed = 0;
      el.setAttribute("data-sheet-pulling", "");
    };
    const move = (e: TouchEvent) => {
      if (from === null) return;
      e.preventDefault();
      const y = e.touches[0].clientY;
      dy = Math.max(0, y - from);
      const dt = e.timeStamp - lastT;
      if (dt > 0) speed = (y - lastY) / dt;
      lastY = y;
      lastT = e.timeStamp;
      el.style.setProperty("--sheet-pull", `${dy}px`);
    };
    const up = () => {
      if (from === null) return;
      from = null;
      el.removeAttribute("data-sheet-pulling");
      const go = dy > el.getBoundingClientRect().height / 4 || speed > 0.5;
      el.style.setProperty("--sheet-pull", "0px");
      if (go) dismiss();
    };
    el.addEventListener("touchstart", down, { passive: true });
    el.addEventListener("touchmove", move, { passive: false });
    el.addEventListener("touchend", up);
    el.addEventListener("touchcancel", up);
    return () => {
      vv?.removeEventListener("resize", ride);
      vv?.removeEventListener("scroll", ride);
      el.removeEventListener("touchstart", down);
      el.removeEventListener("touchmove", move);
      el.removeEventListener("touchend", up);
      el.removeEventListener("touchcancel", up);
    };
  }, [on, el, dismiss]);
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
