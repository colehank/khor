/**
 * **The conversation's input: one glass capsule** (the redesign,
 * 2026-10-02, the owner after Gemini's: 「gui 的 session 会话, 输入栏现在
 * 很丑, 改成这样. 这里的 Pro 下箭头, 是权限设置. 运行中发送改为中断按钮」).
 *
 * ＋ at the left end, the text, the permission mode as a word with a
 * chevron, and at the far end send (only once there is something to
 * send) or, while a turn is running, the interrupt in the same seat.
 * Everything is inside the one capsule: `--composer-h` tall at one line,
 * its ends round at that height and kept at that radius as the text
 * grows (a rounded rectangle, not a pill of growing radius). The
 * controls stand on the last line — bottom-aligned, so typing does not
 * walk them up the screen.
 *
 * A note to a machine (`MachineChat`) uses the same capsule with no mode
 * and no interrupt.
 */
import * as DropdownMenuPrimitive from "@radix-ui/react-dropdown-menu";
import type { ReactNode } from "react";

import { IconChevronDown, IconChevronRight, IconChevronUp, IconPlus } from "@/components/icons";
import { Switch } from "@/components/settings/Switch";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";

/** Our glass menu (`FloatNav`'s 更多, the page field's machines). */
const MENU = "glass rounded-2xl border-0 bg-glass-menu p-1.5 shadow-[var(--glass-in),var(--glass-shadow)]";
const ITEM = "min-h-touch gap-row rounded-xl px-row text-body focus:glass-sel focus:text-foreground";
/** A button standing inside the capsule: one control's height less the
    rim's 6 above and below. */
const SEAT = "grid size-composer-item flex-none place-items-center rounded-full [&_svg]:size-ctl-glyph";

export function Composer({ lead, children, trail }: { lead?: ReactNode; children: ReactNode; trail?: ReactNode }) {
  return (
    <div
      data-chat-box
      className="glass flex min-h-composer min-w-0 flex-1 items-end gap-1 rounded-composer p-1.5"
    >
      {lead}
      {children}
      {trail}
    </div>
  );
}

/** The text inside the capsule: no box of its own, one line as tall as
    the seats beside it, growing to `--chat-box` (the caller measures). */
export const COMPOSER_TEXT =
  "min-h-composer-item max-h-chat-box min-w-0 flex-1 resize-none self-center border-0 bg-transparent px-1 py-[calc((var(--composer-item)-1lh)/2)] outline-none shadow-none placeholder:text-muted-foreground focus-visible:ring-0";

/** ＋ as a plain glyph button (the menu or the picker hangs off it). */
export function PlusSeat(props: React.ComponentProps<"button">) {
  return (
    <button
      type="button"
      data-slot="button"
      aria-label={gui.chat_add}
      {...props}
      className={cn(SEAT, "text-foreground hover:bg-fill data-[state=open]:glass-sel", props.className)}
    >
      <IconPlus />
    </button>
  );
}

/** Send: the brand's one solid circle. */
export function SendSeat({ onSend, disabled }: { onSend: () => void; disabled?: boolean }) {
  return (
    <button
      type="button"
      data-slot="button"
      data-chat-send
      aria-label={gui.send_it}
      disabled={disabled}
      onClick={onSend}
      className={cn(SEAT, "bg-brand-strong text-on-brand disabled:opacity-40")}
    >
      <IconChevronUp />
    </button>
  );
}

/** Interrupt, in send's seat while a turn runs: a filled rounded square
    on a round button, no word (the name is the tooltip's and a screen
    reader's). */
export function StopSeat({ onStop, tip }: { onStop: () => void; tip: boolean }) {
  const button = (
    <button
      type="button"
      data-slot="button"
      data-chat-stop
      aria-label={gui.chat_stop}
      onClick={onStop}
      className={cn(SEAT, "bg-fill-strong text-foreground")}
    >
      <span aria-hidden className="size-[calc(var(--composer-item)/3)] rounded-[3px] bg-current" />
    </button>
  );
  return tip ? (
    <Tooltip>
      <TooltipTrigger asChild>{button}</TooltipTrigger>
      <TooltipContent side="top">{gui.chat_stop}</TooltipContent>
    </Tooltip>
  ) : (
    button
  );
}

export type ModeChoice = { id: string; name: string; note: string | null };

/**
 * **The permission mode, a word with a chevron, and its menu in place.**
 *
 * ⚠️ 墓碑: it was a `PopupButton` above the box — 「a choice among values
 * is a popup button」 (components/PopupButton, 2026-09-28: the system's
 * menu, never one of our own drawing). The owner's picture overrules that
 * here (2026-10-02, Gemini's input: the mode is a word inside the
 * capsule, opening a menu of its own), and the menu is our glass one:
 * each mode's name and its description under it, the current one chosen.
 *
 * Any mode but the ordinary one wears `state-blocked`, as the marker did.
 * `choices` absent: the agent reports a mode it does not offer to change
 * — the word alone, not a button.
 */
export function ModeSeat({
  current,
  word,
  choices,
  onPick,
}: {
  current: string;
  word: string;
  choices?: ModeChoice[];
  onPick?: (id: string) => void;
}) {
  const ink = current === "default" ? "text-muted-foreground" : "text-state-blocked";
  if (!choices || !onPick) {
    return (
      <span data-chat-mode data-mode-id={current} className={cn("flex h-composer-item flex-none items-center px-2.5 text-callout", ink)}>
        {word}
      </span>
    );
  }
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <button
          type="button"
          data-slot="button"
          data-chat-mode-menu
          data-mode-id={current}
          aria-label={`${gui.chat_mode}: ${word}`}
          className={cn(
            "flex h-composer-item flex-none items-center gap-1 rounded-full ps-2.5 pe-1.5 text-callout whitespace-nowrap hover:bg-fill",
            "data-[state=open]:glass-sel data-[state=open]:pe-2.5",
            ink,
          )}
        >
          {word}
          <IconChevronDown className="size-affordance" />
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent
        side="top"
        align="end"
        // Out to the capsule's own end and 10 above it: the seat stands 6
        // in from its rim.
        alignOffset={-6}
        sideOffset={16}
        data-chat-mode-list
        onCloseAutoFocus={(e) => e.preventDefault()}
        className={cn(MENU, "w-[232px] rounded-[18px]")}
      >
        {choices.map((m) => (
          <DropdownMenuItem
            key={m.id}
            data-mode-option={m.id}
            data-on={m.id === current || undefined}
            onSelect={() => onPick(m.id)}
            className={cn(
              "min-h-12 flex-col items-start justify-center gap-0 rounded-xl px-3 py-1.5 text-foreground focus:glass-sel focus:text-foreground",
              m.id === current && "glass-sel",
            )}
          >
            <span className={cn("text-body", m.id === current && "font-title")}>{m.name}</span>
            {m.note && <span className="text-caption text-muted-foreground">{m.note}</span>}
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/** One of the agent's own settings, as the ＋ menu draws it. */
export type AddOption =
  | { kind: "switch"; id: string; name: string; note: string | null; on: boolean; onToggle: () => void }
  | {
      kind: "values";
      id: string;
      name: string;
      note: string | null;
      current: string;
      values: { value: string; name: string; note: string | null; heading: string | null }[];
      onPick: (value: string) => void;
    };

/**
 * **＋: what can go along with the line, then the agent's settings.**
 *
 * With nothing but attaching, ＋ is the file picker itself (`onAttach`);
 * with settings it opens a menu — attaching first, a hairline, then each
 * setting as it was drawn above the box: a switch, or a submenu of its
 * values. Nothing to attach and no settings: ＋ is not drawn.
 */
export function AddSeat({ onAttach, options }: { onAttach?: () => void; options: AddOption[] }) {
  if (!onAttach && options.length === 0) return null;
  if (options.length === 0) return <PlusSeat data-chat-add onClick={onAttach} />;
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <PlusSeat data-chat-add />
      </DropdownMenuTrigger>
      <DropdownMenuContent
        side="top"
        align="start"
        alignOffset={-6}
        sideOffset={16}
        data-chat-add-menu
        onCloseAutoFocus={(e) => e.preventDefault()}
        className={cn(MENU, "min-w-nav-menu")}
      >
        {onAttach && (
          <DropdownMenuItem data-chat-attach onSelect={onAttach} className={ITEM}>
            {gui.chat_attach}
          </DropdownMenuItem>
        )}
        {onAttach && <DropdownMenuSeparator />}
        {options.map((o) =>
          o.kind === "switch" ? (
            <DropdownMenuItem
              key={o.id}
              title={o.note ?? undefined}
              // The row is the switch's label: pressing anywhere on it
              // toggles, and the menu stays open on the answer.
              onSelect={(e) => {
                e.preventDefault();
                o.onToggle();
              }}
              className={cn(ITEM, "justify-between")}
            >
              <span>{o.name}</span>
              <Switch on={o.on} label={o.name} onToggle={o.onToggle} mark={{ "data-chat-option": o.id }} className="-my-3" />
            </DropdownMenuItem>
          ) : (
            <DropdownMenuPrimitive.Sub key={o.id}>
              <DropdownMenuPrimitive.SubTrigger
                data-chat-option={o.id}
                data-value={o.current}
                title={o.note ?? undefined}
                className={cn(
                  ITEM,
                  "relative flex cursor-default items-center outline-hidden select-none data-[state=open]:glass-sel",
                )}
              >
                <span className="min-w-0 flex-1">{o.name}</span>
                <span className="text-muted-foreground">{o.values.find((v) => v.value === o.current)?.name ?? o.current}</span>
                <IconChevronRight className="size-affordance text-muted-foreground" />
              </DropdownMenuPrimitive.SubTrigger>
              <DropdownMenuPrimitive.Portal>
                <DropdownMenuPrimitive.SubContent sideOffset={8} className={cn(MENU, "z-50 min-w-nav-menu")}>
                  {o.values.map((v, i) => (
                    <div key={v.value}>
                      {v.heading && v.heading !== o.values[i - 1]?.heading && (
                        <DropdownMenuLabel className="px-row text-caption text-muted-foreground">{v.heading}</DropdownMenuLabel>
                      )}
                      <DropdownMenuItem
                        data-option-value={v.value}
                        title={v.note ?? undefined}
                        onSelect={() => o.onPick(v.value)}
                        className={cn(ITEM, v.value === o.current && "glass-sel")}
                      >
                        {v.name}
                      </DropdownMenuItem>
                    </div>
                  ))}
                </DropdownMenuPrimitive.SubContent>
              </DropdownMenuPrimitive.Portal>
            </DropdownMenuPrimitive.Sub>
          ),
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
