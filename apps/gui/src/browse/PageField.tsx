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
import { IconBack, IconForward, IconReload } from "@/components/icons";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";

/** The machines a page can go out through: this one, and the ones this
    one can reach now (not quiet, not paused). */
export function reachable(devices: DeviceRow[]): DeviceRow[] {
  return devices.filter((d) => d.me || (d.hop !== null && d.hop !== "quiet" && !d.paused));
}


export function PageField({
  tab,
  machine,
  devices,
  at,
  onGo,
  onMachine,
  fixed = false,
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
}) {
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
  const menu = (
    <DropdownMenuContent
      align="start"
      sideOffset={8}
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
      const u = new URL(at);
      return u.host + (u.pathname === "/" ? "" : u.pathname);
    } catch {
      return at;
    }
  })();
  return (
    <div data-page-field-row className="flex min-w-0 flex-1 items-center gap-in">
      {open && (
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
      <DropdownMenu open={picking} onOpenChange={setPicking}>
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
              className="glass flex h-glass-btn min-w-0 flex-1 items-center rounded-full px-row text-left text-body text-tertiary"
            >
              {gui.page_choose_machine}
            </button>
          ) : (
            <DropdownMenuTrigger asChild>
              <button
                type="button"
                data-page-field
                data-page-field-empty
                className="glass flex h-glass-btn min-w-0 flex-1 items-center rounded-full px-row text-left text-body text-tertiary"
              >
                {gui.page_choose_machine}
              </button>
            </DropdownMenuTrigger>
          )
        ) : (
          <label data-page-field className="glass flex h-glass-btn min-w-0 flex-1 items-center gap-in rounded-full ps-1 pe-row">
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
            <input
              ref={input}
              data-page-address
              aria-label={gui.address_page}
              placeholder={gui.page_address}
              value={typing ? draft : host}
              onFocus={() => {
                editing.current = true;
                setTyping(true);
              }}
              onBlur={() => {
                editing.current = false;
                setTyping(false);
                setDraft(at);
              }}
              onChange={(e) => setDraft(e.target.value)}
              onKeyDown={(e) => {
                if (e.nativeEvent.isComposing) return;
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
              className="min-w-0 flex-1 truncate bg-transparent text-body outline-none placeholder:text-tertiary"
            />
          </label>
        )}
        {menu}
      </DropdownMenu>
    </div>
  );
}
