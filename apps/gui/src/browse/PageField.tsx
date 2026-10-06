/**
 * **The browser's address field: the machine first** (the redesign,
 * 2026-10-02, the owner: 「点击框，弹出可选机器，选择一个机器，机器的头像
 * 出现在输入框的左侧。现在完全不知道是在那台机器。然后，用户再输入地址」).
 *
 * One glass field. At its left end, inside it, the machine whose network
 * the page goes through — its face, at a tab's size, always on screen, a
 * button that offers the machines again. A page with no machine yet has
 * no caret: the field says to choose one, and pressing it opens the
 * picker (the machines this one can reach, each its face and name).
 * Choosing puts the face in the field and the caret after it; then the
 * address, and Enter. On a page already open, choosing another machine
 * reloads the same address through it.
 *
 * Back, forward and reload — the page's own steps, which the browser had
 * as ghost buttons — stand in a glass capsule at the field's left, and
 * only while a page is open (there is nothing to step through before).
 *
 * `fixed`: the machine is the page's and not a choice (a machine's own
 * page, its 浏览 in place) — the face is shown, not offered.
 */
import { useEffect, useRef, useState } from "react";

import { pageStep, type DeviceRow } from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import { GlassCapsule, GlassItem } from "@/components/Glass";
import { IconBack, IconClose, IconForward, IconReload } from "@/components/icons";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { imeKey } from "@/lib/ime";

/** The machines a page can go out through: this one, and the ones this
    one can reach now (not quiet, not paused). */
export function reachable(devices: DeviceRow[]): DeviceRow[] {
  return devices.filter((d) => d.me || (d.hop !== null && d.hop !== "quiet" && !d.paused));
}

/** **The machine a page last went out through** (UX-18): a fresh tab
    starts there. Per screen, as the tabs themselves are. */
const LAST_MACHINE = "khor.page-machine";

export function lastPageMachine(): string | null {
  try {
    return window.localStorage.getItem(LAST_MACHINE);
  } catch {
    return null;
  }
}

export function rememberPageMachine(id: string) {
  try {
    window.localStorage.setItem(LAST_MACHINE, id);
  } catch {
    // A private window keeps nothing; the next tab asks again.
  }
}


export function PageField({
  tab,
  machine,
  devices,
  at,
  onGo,
  onMachine,
  fixed = false,
  steps = true,
  autoFocus = false,
  onLeave,
  inPill = false,
}: {
  /** The page this field steps (back, forward, reload). */
  tab: string;
  machine: DeviceRow | null;
  devices: DeviceRow[];
  /** Where the page is, `""` before anything has opened. */
  at: string;
  onGo: (place: string) => void;
  onMachine?: (machine: DeviceRow) => void;
  fixed?: boolean;
  /** Draw ← → ↻ beside the field — not on the phone's deck, whose pill
      carries them (round UX-3). */
  steps?: boolean;
  /** Put the caret in the address at once: the deck opened the field to
      be typed in (round UX-3). */
  autoFocus?: boolean;
  /** The caret left the address: the deck puts the field away. */
  onLeave?: () => void;
  /** **The deck's pill is the editor** (UX-18): the field is drawn inside
      the pill's own glass, not as a glass field of its own. */
  inPill?: boolean;
}) {
  const shell = inPill
    ? "flex h-full min-w-0 flex-1 items-center rounded-full"
    : "glass flex h-glass-btn min-w-0 flex-1 items-center rounded-full";
  const [draft, setDraft] = useState(at);
  const [picking, setPicking] = useState(false);
  const input = useRef<HTMLInputElement>(null);
  const editing = useRef(false);
  // The page moved (a link, a step): the field follows unless somebody is
  // typing in it.
  useEffect(() => {
    if (!editing.current) setDraft(at);
  }, [at]);

  // The caret goes in once the face is there, ready for the address —
  // which may be a round trip later (the tab learns its machine from the
  // node), so it is asked for and taken when the field has a machine.
  const caret = useRef(false);
  const choose = (d: DeviceRow) => {
    setPicking(false);
    caret.current = true;
    onMachine?.(d);
  };
  useEffect(() => {
    if (!caret.current || !machine) return;
    caret.current = false;
    requestAnimationFrame(() => input.current?.focus());
  }, [machine]);
  /** **The picker does not cover the tabs** (the audit fix plan F4.6; N22).
      On the narrow face the field is the session bar's second row, under
      the tabs and 「‹」, and a menu opening upward from it lay over both:
      there it opens above the whole bar, 8 clear of it. Measured as it
      opens; elsewhere (the wide head) it opens where it always did. */
  const row = useRef<HTMLDivElement>(null);
  const [lift, setLift] = useState<number | null>(null);
  const pick = (open: boolean) => {
    if (open) {
      const at = row.current;
      const trigger = at?.querySelector("[data-page-machine-face], [data-page-field-empty]")?.getBoundingClientRect();
      const bar = at?.closest("[data-session-screen]")?.querySelector("[data-session-bar]")?.getBoundingClientRect();
      setLift(trigger && bar && bar.top < trigger.top ? trigger.top - bar.top + 8 : null);
    }
    setPicking(open);
  };
  const menu = (
    <DropdownMenuContent
      align="start"
      {...(lift === null ? { sideOffset: 8 } : { side: "top" as const, sideOffset: lift, avoidCollisions: false })}
      data-page-machines
      onCloseAutoFocus={(e) => e.preventDefault()}
      className="min-w-nav-menu"
    >
      {reachable(devices).map((d) => (
        <DropdownMenuItem
          key={d.id}
          data-page-machine={d.name}
          data-on={machine?.id === d.id || undefined}
          onSelect={() => choose(d)}
          className={cn(machine?.id === d.id && "glass-sel")}
        >
          <MachineAvatar face={d.face} className="size-tab-face flex-none" />
          <span className="min-w-0 truncate">{d.shown}</span>
        </DropdownMenuItem>
      ))}
    </DropdownMenuContent>
  );

  const open = machine !== null && at !== "";
  /** **One machine to go through is not a choice** (the audit fix plan
      F4.6; N22): the face is shown, not offered, and nothing opens. */
  const only = reachable(devices).length <= 1;
  /** Whether the caret is in the address: the whole address then; at
      rest, the host first (Safari's), cut with an ellipsis. */
  const [typing, setTyping] = useState(false);
  const host = (() => {
    try {
      // An address typed without a scheme is a web address: `new URL`
      // reads 「localhost:1458/x」 as the scheme `localhost:` and drops it,
      // and the field said 「1458/x」.
      const u = new URL(/^[a-z][a-z\d+.-]*:\/\//i.test(at) ? at : `http://${at}`);
      return u.host + (u.pathname === "/" ? "" : u.pathname);
    } catch {
      return at;
    }
  })();
  return (
    <div ref={row} data-page-field-row className="flex min-w-0 flex-1 items-center gap-in">
      {open && steps && (
        <GlassCapsule>
          {(
            [
              ["back", gui.page_back, IconBack],
              ["forward", gui.page_forward, IconForward],
              ["reload", gui.page_reload, IconReload],
            ] as const
          ).map(([way, word, Icon]) => (
            <GlassItem key={way} aria-label={word} data-page-step={way} onClick={() => void pageStep(tab, way).catch(() => {})}>
              <Icon />
            </GlassItem>
          ))}
        </GlassCapsule>
      )}
      <DropdownMenu open={picking} onOpenChange={pick}>
        {machine === null ? (
          // No machine yet: the whole field is the way to choose one, and
          // there is no caret to type into. With one machine there is
          // nothing to choose: the press takes it.
          only ? (
            <button
              type="button"
              data-page-field
              data-page-field-empty
              onClick={() => {
                const d = reachable(devices)[0];
                if (d) choose(d);
              }}
              className={cn(shell, "px-row text-left text-body text-tertiary")}
            >
              {gui.page_choose_machine}
            </button>
          ) : (
            <DropdownMenuTrigger asChild>
              <button
                type="button"
                data-page-field
                data-page-field-empty
                className={cn(shell, "px-row text-left text-body text-tertiary")}
              >
                {gui.page_choose_machine}
              </button>
            </DropdownMenuTrigger>
          )
        ) : (
          <label data-page-field className={cn(shell, "gap-in ps-1 pe-row")}>
            {fixed || only ? (
              <span data-page-machine-face className="grid size-glass-item-h flex-none place-items-center">
                <MachineAvatar face={machine.face} className="size-tab-face" />
              </span>
            ) : (
              <DropdownMenuTrigger asChild>
                <button
                  type="button"
                  data-page-machine-face
                  aria-label={machine.shown}
                  title={gui.page_machine}
                  data-glass-hit
                  className="grid size-glass-item-h flex-none place-items-center rounded-full"
                >
                  <MachineAvatar face={machine.face} className="size-tab-face" />
                </button>
              </DropdownMenuTrigger>
            )}
            {/* **Safari's address field** (UX-38 §3): the keyboard for an
                address with Go on its return key, nothing capitalised or
                corrected, the whole address selected as the pill opens it
                (typing replaces it, a tap places the caret), the text
                scrolling rather than cut while it is edited, and a clear ×
                at its end. */}
            <input
              ref={input}
              data-page-address
              aria-label={gui.address_page}
              placeholder={gui.page_address}
              value={typing ? draft : host}
              enterKeyHint="go"
              inputMode="url"
              autoCapitalize="none"
              autoCorrect="off"
              spellCheck={false}
              onFocus={(e) => {
                editing.current = true;
                setTyping(true);
                if (inPill) {
                  const field = e.currentTarget;
                  requestAnimationFrame(() => field.select());
                }
              }}
              autoFocus={autoFocus}
              onBlur={() => {
                editing.current = false;
                setTyping(false);
                setDraft(at);
                onLeave?.();
                // At rest the address reads from its host (the line above);
                // the field kept the scroll the typing left it at, so a long
                // address showed its tail, its start cut with no ellipsis.
                requestAnimationFrame(() => {
                  if (input.current) input.current.scrollLeft = 0;
                });
              }}
              onChange={(e) => setDraft(e.target.value)}
              onKeyDown={(e) => {
                if (imeKey(e)) return;
                if (e.key === "Enter" && draft.trim()) {
                  e.preventDefault();
                  editing.current = false;
                  onGo(draft.trim());
                  input.current?.blur();
                }
                if (e.key === "Escape") {
                  setDraft(at);
                  input.current?.blur();
                }
              }}
              className={cn("min-w-0 flex-1 bg-transparent text-body outline-none placeholder:text-tertiary", !typing && "truncate")}
            />
            {inPill && typing && draft !== "" && (
              <button
                type="button"
                data-page-address-clear
                aria-label={gui.search_clear}
                // The caret stays in the field.
                onPointerDown={(e) => e.preventDefault()}
                onMouseDown={(e) => e.preventDefault()}
                onClick={() => {
                  setDraft("");
                  input.current?.focus();
                }}
                className="grid size-6 flex-none place-items-center rounded-full bg-fill-strong text-muted-foreground"
              >
                <IconClose className="size-affordance" />
              </button>
            )}
          </label>
        )}
        {menu}
      </DropdownMenu>
    </div>
  );
}
