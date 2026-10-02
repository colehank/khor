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
import { useEffect, useLayoutEffect, useRef, useState, type ComponentType, type ReactNode } from "react";

import { IconChevronDown, IconChevronRight, IconChevronUp, IconPlus } from "@/components/icons";
import { arc, ARC0, SPAN } from "@/components/Face";
import { Switch } from "@/components/settings/Switch";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
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
import { gaugeColor } from "@/lib/vitals";

/** Our glass menu (`FloatNav`'s 更多, the page field's machines). */
/** Our glass menu, as a popover wears it (the who panel): the one menu's
    material and geometry (`ui/dropdown-menu`). */
const MENU = "glass rounded-menu border-0 bg-glass-menu p-menu-inset text-foreground shadow-[var(--glass-in),var(--glass-shadow)]";
/** A button standing inside the capsule: one control's height less the
    rim's 6 above and below. */
const SEAT = "grid size-composer-item flex-none place-items-center rounded-full [&_svg]:size-ctl-glyph";

/** What the text keeps while typing before the who seat gives way. */
const TEXT_FLOOR_SEAT = 120;

/**
 * The capsule. One of its parts is decided by **its own width**, not the
 * window's: the who seat (`[data-composer-seat]`), which gives way while
 * there is text to send (`seatYields`) and the text would keep less
 * than `TEXT_FLOOR_SEAT` — on a 390 phone with the longest mode word.
 *
 * **No model is written inside the capsule** (the owner, 2026-10-03, on
 * a Codex session: 「这里不要显示这个model水印」). The model is the who
 * seat's panel's and its tooltip's. ⚠️ 墓碑: `WhoInline`, the agent's
 * mark and the model id before the mode wherever the capsule had room —
 * read as a watermark, and before usage was known it stood the same
 * mark twice side by side with the seat's.
 */
export function Composer({
  lead,
  children,
  trail,
  seatYields = false,
}: {
  lead?: ReactNode;
  children: ReactNode;
  trail?: ReactNode;
  seatYields?: boolean;
}) {
  const cap = useRef<HTMLDivElement>(null);
  const [seatGone, setSeatGone] = useState(false);
  const seatW = useRef(0);
  const measure = useRef(() => {});
  measure.current = () => {
    const el = cap.current;
    if (!el) return;
    const cs = getComputedStyle(el);
    const gap = parseFloat(cs.columnGap) || 0;
    const inner = el.clientWidth - (parseFloat(cs.paddingLeft) || 0) - (parseFloat(cs.paddingRight) || 0);
    const seat = el.querySelector<HTMLElement>("[data-composer-seat]");
    if (seat && seat.offsetWidth > 0) seatW.current = seat.offsetWidth;
    // Everything but the text, as drawn now.
    const others = [...el.children].filter(
      (k): k is HTMLElement => k instanceof HTMLElement && !k.hasAttribute("data-composer-text") && k.offsetWidth > 0,
    );
    const free = inner - others.reduce((n, k) => n + k.offsetWidth + gap, 0);
    // With the seat back in, what the text would keep.
    const withSeat = seatGone ? free - seatW.current - gap : free;
    const gone = seatYields && seatW.current > 0 && withSeat < TEXT_FLOOR_SEAT;
    if (gone !== seatGone) setSeatGone(gone);
  };
  useLayoutEffect(() => measure.current());
  useEffect(() => {
    const el = cap.current;
    if (!el) return;
    const ro = new ResizeObserver(() => measure.current());
    ro.observe(el);
    return () => ro.disconnect();
  }, []);
  return (
    <div
      ref={cap}
      data-chat-box
      data-seat-gone={seatGone || undefined}
      className="glass flex min-h-composer min-w-0 flex-1 items-end gap-1 rounded-composer p-1.5 data-[seat-gone]:[&_[data-composer-seat]]:hidden"
    >
      {lead}
      {children}
      {trail}
    </div>
  );
}

/**
 * **The capsule's text grows with what is typed** (V05): as tall as its
 * lines, up to `--chat-box`, measured — `field-sizing` would say it in
 * CSS and the Mac app's WKWebView does not have it. The height is
 * cleared first, or a tall box would never shrink back. One rule for
 * every capsule (the agent conversation's, a machine's notes).
 */
export function useGrowingText(box: React.RefObject<HTMLTextAreaElement | null>, ...deps: unknown[]) {
  useLayoutEffect(() => {
    const el = box.current;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = `${el.scrollHeight}px`;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);
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
      data-glass-hit
      className={cn(SEAT, "text-foreground data-[state=open]:glass-sel aria-busy:animate-pulse motion-reduce:aria-busy:animate-none", props.className)}
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
      aria-busy={disabled || undefined}
      className={cn(SEAT, "bg-brand-strong text-on-brand aria-busy:animate-pulse motion-reduce:aria-busy:animate-none")}
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

/** **Each agent's ordinary mode**, the one that wears no warning ink:
    claude's `default` (asks before changes) and codex's `workspace` (free
    inside the workspace, asks past it — `codexagent::PRESETS`). Any
    other mode is a session that will do more, or less, than usual. */
export const ORDINARY_MODES = new Set(["default", "workspace"]);

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
  const ink = ORDINARY_MODES.has(current) ? "text-muted-foreground" : "text-state-blocked";
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
          data-glass-hit
          data-mode-id={current}
          aria-label={`${gui.chat_mode}: ${word}`}
          className={cn(
            "flex h-composer-item flex-none items-center gap-1 rounded-full ps-2.5 pe-1.5 text-callout whitespace-nowrap",
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
        className="w-[232px]"
      >
        {choices.map((m) => (
          <DropdownMenuItem
            key={m.id}
            data-mode-option={m.id}
            data-on={m.id === current || undefined}
            onSelect={() => onPick(m.id)}
            className={cn(
              "flex-col items-start justify-center gap-0 py-1.5",
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
        className="min-w-nav-menu"
      >
        {onAttach && (
          <DropdownMenuItem data-chat-attach onSelect={onAttach}>
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
              className="justify-between"
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
                className="relative flex min-h-menu-item cursor-default items-center gap-row rounded-menu-item px-row text-body outline-hidden select-none data-[highlighted]:glass-sel data-[state=open]:glass-sel"
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
                        className={cn(v.value === o.current && "glass-sel")}
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

/** Who is answering: the agent's mark and name, the model it runs on,
    and how full its context is. Each absent when it is not known. */
export type Who = {
  Mark: ComponentType<{ className?: string }>;
  name: string | null;
  model: string | null;
  usage: { used: number; size: number } | null;
};

/** `82k`, `1.2M`: a token count a person reads at a glance. */
function compact(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(n >= 10_000_000 ? 0 : 1).replace(/\.0$/, "")}M`;
  if (n >= 1_000) return `${Math.round(n / 1_000)}k`;
  return String(n);
}

/**
 * **The context as a dial** (the owner, 2026-10-02: 「上下文用环表示,
 * 也就是我们额度那种环. 里面写数字」): the quota and machine dials' own
 * geometry (`components/Face`: `ARC0`, `SPAN`, `arc`), its gap at the
 * bottom, track and value with round caps, and the share as a bare number
 * inside. From 90 up the value and the number take the strain ink, as
 * those dials do (`gaugeColor`).
 */
function CtxDial({ pct }: { pct: number }) {
  const S = 28;
  const W = 2.5;
  const r = S / 2 - W;
  // The 210° arc reaches r above its centre and r·sin 15° below it:
  // centred on that extent, not on the circle it is cut from.
  const below = Math.sin((ARC0 + SPAN) * (Math.PI / 180));
  const cy = S / 2 + (r * (1 - below)) / 2;
  const ink = gaugeColor(pct >= 90 ? "critical" : null);
  const end = ARC0 + (Math.min(100, Math.max(0, pct)) / 100) * SPAN;
  return (
    <span className="relative grid size-[calc(var(--composer-h)*0.585)] place-items-center">
      <svg viewBox={`0 0 ${S} ${S}`} aria-hidden="true" className="absolute inset-0 size-full">
        <path d={arc(S / 2, cy, r, ARC0, ARC0 + SPAN)} fill="none" stroke="currentColor" strokeOpacity={0.2} strokeWidth={W} strokeLinecap="round" />
        {pct > 0 && (
          <path d={arc(S / 2, cy, r, ARC0, end)} fill="none" stroke={ink} strokeWidth={W} strokeLinecap="round" />
        )}
      </svg>
      <span
        className="absolute left-1/2 -translate-x-1/2 -translate-y-1/2 text-[10px] leading-3 font-title tabular-nums"
        // In the arc's bowl: the box's middle, a pixel low, where the
        // open bottom leaves it room.
        style={{ color: ink, top: `${((S / 2 + 1) / S) * 100}%` }}
      >
        {pct}
      </span>
    </span>
  );
}

/** The three facts, as rows of a label and a value; a row whose value
    is unknown is absent. The panel and the tooltip both read these. */
function whoRows(who: Who) {
  const rows: { key: string; label: string; value: ReactNode }[] = [];
  if (who.name)
    rows.push({
      key: "agent",
      label: gui.new_session_agent,
      value: (
        <>
          <who.Mark className="size-[18px] flex-none" />
          <span className="font-title">{who.name}</span>
        </>
      ),
    });
  if (who.model)
    rows.push({ key: "model", label: gui.accounts_models, value: <span className="min-w-0 truncate font-mono text-callout">{who.model}</span> });
  if (who.usage) {
    const pct = Math.round((who.usage.used / who.usage.size) * 100);
    rows.push({
      key: "context",
      label: gui.chat_who_context,
      value: (
        <>
          <span className="tabular-nums">{pct}%</span>
          <span className="text-callout text-muted-foreground tabular-nums">
            {compact(who.usage.used)} / {compact(who.usage.size)}
          </span>
        </>
      ),
    });
  }
  return rows;
}

/**
 * **Who is answering, inside the capsule** (the owner, 2026-10-02: the
 * line above it 「占用了窄屏的空间」 — 「有没有什么办法, 让这些信息显示在
 * 这个框里面? 比如窄屏是一个 i 的 icon, 点击后显示」). One seat between
 * the mode and send: the context dial once usage is known, the agent's
 * mark before. Pressing it opens a small glass panel in place with the
 * three facts — nothing in it is pressable; it closes on an outside
 * press, Escape, or the seat again. Under a mouse the same facts are the
 * seat's tooltip. No cost anywhere in the composer (「右边不写美元」).
 */
export function WhoSeat({ who, tip }: { who: Who; tip: boolean }) {
  const [open, setOpen] = useState(false);
  const pct = who.usage ? Math.round((who.usage.used / who.usage.size) * 100) : null;
  const rows = whoRows(who);
  const seat = (
    <button
      type="button"
      data-slot="button"
      data-composer-seat
      data-glass-hit
      data-chat-who
      data-chat-usage={who.usage ? "" : undefined}
      data-usage-used={who.usage?.used}
      data-usage-size={who.usage?.size}
      aria-label={pct === null ? gui.chat_who : `${gui.chat_who}: ${gui.chat_context(String(pct))}`}
      className={cn(
        "grid size-composer-item flex-none place-items-center rounded-full text-muted-foreground data-[state=open]:glass-sel",
      )}
    >
      {pct === null ? <who.Mark className="size-[calc(var(--composer-item)*0.55)]" /> : <CtxDial pct={pct} />}
    </button>
  );
  return (
    <Popover open={open} onOpenChange={setOpen}>
      {tip && !open && rows.length > 0 ? (
        <Tooltip>
          <TooltipTrigger asChild>
            <PopoverTrigger asChild>{seat}</PopoverTrigger>
          </TooltipTrigger>
          <TooltipContent side="top" data-chat-who-tip className="flex flex-col gap-0.5">
            {rows.map((r) => (
              <span key={r.key} className="flex items-center gap-1.5">
                <span className="opacity-70">{r.label}</span>
                {r.value}
              </span>
            ))}
          </TooltipContent>
        </Tooltip>
      ) : (
        <PopoverTrigger asChild>{seat}</PopoverTrigger>
      )}
      <PopoverContent
        side="top"
        align="end"
        sideOffset={16}
        alignOffset={-6}
        data-chat-who-panel
        onOpenAutoFocus={(e) => e.preventDefault()}
        onCloseAutoFocus={(e) => e.preventDefault()}
        className={cn(MENU, "flex w-[272px] max-w-none flex-col")}
      >
        {rows.map((r, i) => (
          <div key={r.key}>
            {i > 0 && <div className="mx-3 h-px bg-border" />}
            <div data-who-row={r.key} className="flex min-h-10 items-center gap-3 px-3 py-1">
              <span className="w-[52px] flex-none text-callout text-muted-foreground">{r.label}</span>
              <span className="flex min-w-0 flex-1 items-center gap-1.5 text-body text-foreground">{r.value}</span>
            </div>
          </div>
        ))}
      </PopoverContent>
    </Popover>
  );
}
