/**
 * **The browser's address field: the machine, then the address** (sb-a,
 * 2026-10-09, the lab approved — audit/br-lab.md; the owner: 「浏览器点击
 * 药丸, 不要展开机器, 而是一个推荐列表, 里面可以选择机器, 也可以手敲然后自动
 * 推荐, 类似目录选择. 确定机器后, 机器的头像就在左边定下来了, 点击后头像可以
 * 再选择别的机器或用删除键删掉再敲新机器. 然后敲地址. 已经确定的药丸, 再此点
 * 击, 可以点头像或改地址.」).
 *
 * One glass field and one list under it (over it on the deck):
 *
 * 1. Pressed with no machine, the list is the machines this one can reach,
 *    each its face, name and road (「直连 23 ms」); typing narrows it, the
 *    matched letters heavy; Enter or Tab takes the lit row, else the first.
 *    **Tab is the machine alone** (the owner's fix in the lab): the
 *    address stays empty, the machine's recents listed.
 * 2. The chosen machine is its face, fixed at the field's left. Pressing
 *    the face offers the machines again, the current one ticked; on an
 *    open page another reloads the same address through it at once.
 *    Backspace with the caret at the address's start takes the face off,
 *    and what was typed waits for the next machine (Enter brings it back).
 * 3. The address: the list is that machine's recent addresses, narrowed
 *    as it is typed. Enter opens the lit row, else the typed text
 *    (https:// added where it has no scheme; http:// for this network's
 *    own hosts); Tab completes without opening.
 * 4. A set field rests as face and host. Pressing the host edits the
 *    address, selected whole; pressing the face changes the machine.
 *
 * **Where the list is drawn**: in the app, on the wide face, in the app's
 * small menu window (`useFloatedMenu`), because the page's native webview
 * paints over this page's DOM; on the deck, in the seat over it
 * (`listSeat`), in the flow, so the page gives way rather than covering
 * it; elsewhere under the field. ⚠️ 墓碑 (the redesign 2026-10-02 →
 * sb-a): a menu of machines (`DropdownMenu`) on the face, and on the deck
 * a row of machine chips over it with the face out of the field (UX-42
 * row 13).
 *
 * Back, forward and reload stand in a glass capsule at the field's left,
 * only while a page is open (unchanged).
 *
 * `fixed`: the machine is the page's and not a choice (a machine's own
 * page, its 浏览 in place) — the face is shown, not offered.
 */
import { useEffect, useId, useRef, useState } from "react";
import { createPortal } from "react-dom";

import { fetchRecents, pageStep, type DeviceRow, type RecentRow } from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import { GlassCapsule, GlassItem } from "@/components/Glass";
import { IconBack, IconClose, IconForward, IconReload } from "@/components/icons";
import { OmniMenu, useFloatedMenu, type MenuRows } from "@/components/OmniMenu";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { useFieldDismiss } from "@/hooks/use-field-dismiss";
import { imeKey } from "@/lib/ime";
import { ago, reachWord } from "@/words";

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

/** An address as a row says it: no scheme, no trailing slash. */
function bareOf(url: string): string {
  return url.replace(/^[a-z][a-z\d+.-]*:\/\//i, "").replace(/\/$/, "");
}

/** **What Enter opens** (the brief: 「adding https:// when missing」): a
    typed address without a scheme is a web address over https — except
    this network's own hosts (localhost, an IP, `.local`, `.lan`), whose
    dev servers speak plain http and would only fail the other way. */
export function addressOf(raw: string): string {
  const t = raw.trim();
  if (/^[a-z][a-z\d+.-]*:\/\//i.test(t)) return t;
  const host = t.replace(/[/?#].*$/, "").replace(/:\d+$/, "").toLowerCase();
  const local =
    host === "localhost" ||
    host.endsWith(".localhost") ||
    host.endsWith(".local") ||
    host.endsWith(".lan") ||
    /^\d{1,3}(\.\d{1,3}){3}$/.test(host) ||
    host.startsWith("[");
  return `${local ? "http" : "https"}://${t}`;
}

/** Where the typed letters stand in a label, for the list to draw them
    heavy (`Candidate.mark`). */
function markOf(label: string, q: string): [number, number] | undefined {
  if (!q) return undefined;
  const i = label.toLowerCase().indexOf(q.toLowerCase());
  return i < 0 ? undefined : [i, q.length];
}

/** How many of a machine's addresses the list offers. */
const RECENTS = 6;

export function PageField({
  tab,
  machine,
  devices,
  at,
  onGo,
  onDraft,
  onEditingChange,
  machinePrompt,
  onMachine,
  fixed = false,
  steps = true,
  autoFocus = false,
  onLeave,
  inPill = false,
  listSeat,
  first = null,
}: {
  /** The page this field steps (back, forward, reload). */
  tab: string;
  machine: DeviceRow | null;
  devices: DeviceRow[];
  /** Where the page is, `""` before anything has opened. */
  at: string;
  /** Open this address through this machine. */
  onGo: (place: string, machine: DeviceRow) => void;
  /** Preserve a draft without reading input events before the field handles them. */
  onDraft?: (text: string) => void;
  /** The surrounding pill gives its identity room to the active editor. */
  onEditingChange?: (editing: boolean) => void;
  /** A historical page with no exit asks the person to choose one. */
  machinePrompt?: number;
  /** Another machine chosen for the page that is open: it reloads the
      same address through it. A field with no page yet keeps the face to
      itself until Enter. */
  onMachine?: (machine: DeviceRow) => void;
  fixed?: boolean;
  /** Draw ← → ↻ beside the field — not on the phone's deck, whose pill
      carries them (round UX-3). */
  steps?: boolean;
  /** Put the caret in the field at once: the deck opened it to be typed
      in (round UX-3), or a blank tab (UX-45 §2). */
  autoFocus?: boolean;
  /** The caret left the field: the deck puts the editor away. */
  onLeave?: () => void;
  /** **The deck's pill is the editor** (UX-18): the field is drawn inside
      the pill's own glass, not as a glass field of its own. */
  inPill?: boolean;
  /** The seat over the deck the list is drawn in (the deck's editor); the
      list hangs under the field where there is none. */
  listSeat?: HTMLElement | null;
  /** The machine listed first, by name: the session's, in its 附栏
      (UX-42 row 13's order); else this one leads. */
  first?: string | null;
}) {
  const offered = (() => {
    const all = reachable(devices);
    const lead = (d: DeviceRow) => ((d.id === first || d.name === first) ? 0 : d.me ? 1 : 2);
    return [...all].sort((a, b) => lead(a) - lead(b));
  })();
  /** **One machine to go through is not a choice** (the audit fix plan
      F4.6; N22): the face is shown, not offered. */
  const only = offered.length <= 1;
  const still = fixed || only;
  const open = machine !== null && at !== "";

  /** The face in the field: the machine the next address goes through. */
  const [token, setToken] = useState<DeviceRow | null>(machine ?? (only ? (offered[0] ?? null) : null));
  const [editing, setEditing] = useState(machinePrompt !== undefined || autoFocus);
  /** What is in the field: a machine's name being typed, or an address. */
  const [text, setText] = useState(at);
  const draft = (value: string) => { setText(value); onDraft?.(value); };
  /** The face was pressed: the list is the machines, the address stays. */
  const [switching, setSwitching] = useState(machinePrompt !== undefined);
  /** The address the face's removal put aside. */
  const [kept, setKept] = useState("");
  /** The row the arrows lit, `-1` for none. */
  const [hi, setHi] = useState(-1);
  const [recents, setRecents] = useState<RecentRow[]>([]);
  const input = useRef<HTMLInputElement>(null);
  const inputId = useId();
  const editingListener = useRef(onEditingChange);
  editingListener.current = onEditingChange;
  useEffect(() => { editingListener.current?.(editing); }, [editing]);
  useEffect(() => {
    if (machinePrompt === undefined) return;
    setEditing(true); setSwitching(true); setHi(-1); input.current?.focus(); input.current?.select();
  }, [machinePrompt]);
  const field = useRef<HTMLLabelElement>(null);
  const menu = useRef<HTMLDivElement>(null);

  // The page moved (a link, a step, another machine): the field follows
  // unless somebody is typing in it.
  useEffect(() => {
    if (editing) return;
    setText(at);
    if (machine) setToken(machine);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [at, machine?.id]);
  // A fresh tab learns its machine a round trip after the caret went in
  // (the last one used, UX-18): an empty face takes it.
  useEffect(() => {
    if (machine && token === null && editing) setToken(machine);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [machine?.id]);
  // One machine, and the tab had none yet: it is the face.
  useEffect(() => {
    if (!token && only && offered[0]) setToken(offered[0]);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [only, offered[0]?.id]);
  // The machines' recent addresses, read again each time the field is
  // taken: the list is a glance, one read per visit.
  useEffect(() => {
    if (!editing) return;
    let live = true;
    fetchRecents("page", 60).then(
      (r) => live && setRecents(r),
      () => {},
    );
    return () => {
      live = false;
    };
  }, [editing]);
  useEffect(() => {
    if (autoFocus) {
      const frame = requestAnimationFrame(() => { input.current?.focus(); if (open) input.current?.select(); });
      return () => cancelAnimationFrame(frame);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const pickingMachine = editing && !fixed && (token === null || switching);
  /** The whole address just selected is not a query: all of them. */
  const query = (pickingMachine ? (token === null ? text : "") : open && text === at ? "" : text).trim();
  const machineRows = offered.filter((d) => d.shown.toLowerCase().includes(token === null ? query.toLowerCase() : ""));
  const addressRows = (() => {
    if (!editing || pickingMachine || !token) return [];
    const seen = new Set<string>();
    return recents
      .filter((r) => r.device === token.id && !seen.has(r.place) && seen.add(r.place) && bareOf(r.place).toLowerCase().includes(query.toLowerCase()))
      .slice(0, RECENTS);
  })();
  const n = pickingMachine ? machineRows.length : addressRows.length;
  /** A name being narrowed lights its first row: what Enter takes. */
  const lit = hi >= 0 ? hi : pickingMachine && query !== "" && n > 0 ? 0 : -1;

  const rows: MenuRows | null = !editing
    ? null
    : pickingMachine
      ? {
          groups: [
            {
              key: "machines",
              label: gui.devices_tab,
              empty: gui.no_matches,
              candidates: machineRows.map((d) => ({
                key: `m:${d.id}`,
                label: d.shown,
                face: d.face,
                mark: token === null ? markOf(d.shown, query) : undefined,
                aside: reachWord(d, devices) ?? undefined,
                on: token?.id === d.id || undefined,
              })),
            },
          ],
          at: lit,
        }
      : addressRows.length > 0 && token
        ? {
            groups: [
              {
                key: "recent",
                label: gui.page_recent_on(token.shown),
                candidates: addressRows.map((r) => ({
                  key: `u:${r.place}`,
                  label: bareOf(r.place),
                  glyph: "page" as const,
                  mark: markOf(bareOf(r.place), query),
                  aside: ago(r.at_ms),
                })),
              },
            ],
            at: lit,
          }
        : null;

  /** Set as the field sends the page somewhere: the blur that follows
      leaves the face and the address for the page's answer to replace,
      rather than putting back the ones it is leaving. */
  const went = useRef(false);
  /** Back to what is open (or to what was being made), the face with it. */
  const rest = () => {
    setEditing(false);
    setSwitching(false);
    setHi(-1);
    if (open && !went.current) {
      setToken(machine);
      setText(at);
    }
    went.current = false;
  };
  /** Out of the field: its blur puts it to rest (`onBlur`). */
  const leave = () => {
    if (input.current && document.activeElement === input.current) input.current.blur();
    else rest();
  };
  useFieldDismiss(editing, () => [field.current, menu.current], leave);
  const go = (raw: string) => {
    if (!token || raw.trim() === "") return;
    const place = addressOf(raw);
    rememberPageMachine(token.id);
    onGo(place, token);
    setText(place);
    went.current = true;
    leave();
  };
  const pick = (d: DeviceRow, bare = false) => {
    // The face pressed on an open page: it reloads at once through the
    // new machine.
    if (open && switching) {
      setToken(d);
      if (d.id !== machine?.id || machinePrompt !== undefined) {
        rememberPageMachine(d.id);
        onMachine?.(d);
        went.current = true;
      }
      leave();
      return;
    }
    setToken(d);
    setSwitching(false);
    draft(bare ? "" : kept);
    setKept("");
    setHi(-1);
    input.current?.focus();
  };
  const press = (key: string) => {
    if (key.startsWith("m:")) {
      const d = offered.find((x) => x.id === key.slice(2));
      if (d) pick(d);
    } else if (key.startsWith("u:")) go(key.slice(2));
  };
  const floated = useFloatedMenu(field, listSeat === undefined ? rows : null, press, listSeat === undefined);

  const host = (() => {
    try {
      // An address typed without a scheme is a web address: `new URL`
      // reads 「localhost:1458/x」 as the scheme `localhost:` and drops it.
      const u = new URL(/^[a-z][a-z\d+.-]*:\/\//i.test(at) ? at : `http://${at}`);
      return u.host + (u.pathname === "/" ? "" : u.pathname);
    } catch {
      return at;
    }
  })();

  const list = rows && (
    <OmniMenu
      menuRef={menu}
      rows={rows}
      onPick={press}
      className={listSeat ? "mx-row" : "absolute inset-x-0 top-full z-20 mt-2"}
    />
  );
  const shell = inPill
    ? "flex h-full min-w-0 flex-1 items-center rounded-full"
    : "glass flex h-glass-btn min-w-0 flex-1 items-center rounded-full";
  return (
    <div data-page-field-row className="flex min-w-0 flex-1 items-center gap-in">
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
      <div className="relative flex min-w-0 flex-1">
        <label
          ref={field}
          data-page-field
          data-state={editing ? "editing" : token ? "set" : "empty"}
          // **A label for the address**: a press anywhere on the glass is
          // a press on the address, by the platform's own rule (the face
          // and the × are buttons of their own, which a label leaves be).
          htmlFor={inputId}
          className={cn(shell, "gap-in pe-row", token ? "ps-1" : "ps-row")}
        >
          {token &&
            (still ? (
              <span data-page-machine-face title={token.shown} className="grid size-glass-item-h flex-none place-items-center">
                <MachineAvatar face={token.face} className="size-tab-face" />
              </span>
            ) : (
              <button
                type="button"
                data-page-machine-face
                aria-label={`${gui.page_machine}：${token.shown}`}
                aria-expanded={pickingMachine}
                title={token.shown}
                data-glass-hit
                // The caret stays in the field; the face only changes
                // what the list offers.
                onPointerDown={(e) => {
                  e.preventDefault();
                  e.stopPropagation();
                  if (!editing) input.current?.focus();
                  setSwitching((s) => !(editing && s));
                  setHi(-1);
                }}
                onClick={(e) => {
                  // A keyboard's press (Space / Enter on the face).
                  if (e.detail === 0) {
                    input.current?.focus();
                    setSwitching((s) => !s);
                  }
                }}
                className="grid size-glass-item-h flex-none place-items-center rounded-full"
              >
                <MachineAvatar face={token.face} className="size-tab-face" />
              </button>
            ))}
          {/* **Safari's address field** (UX-38 §3): the keyboard for an
              address with Go on its return key, nothing capitalised or
              corrected, the whole address selected as it is taken (typing
              replaces it, a tap places the caret), the text scrolling
              rather than cut while it is edited, and a clear × at its end. */}
          <input
            ref={input}
            id={inputId}
            data-page-address
            aria-label={gui.address_page}
            aria-expanded={rows !== null}
            placeholder={token ? gui.page_address : gui.page_choose_machine}
            value={editing || !open ? text : host}
            enterKeyHint={token ? "go" : "next"}
            inputMode={token ? "url" : "text"}
            autoCapitalize="none"
            autoCorrect="off"
            spellCheck={false}
            onFocus={(e) => {
              if (editing) return;
              setEditing(true);
              setHi(-1);
              if (open) {
                setText(at);
                const el = e.currentTarget;
                requestAnimationFrame(() => el.select());
              }
            }}
            onBlur={() => {
              rest();
              onLeave?.();
              // At rest the address reads from its host; the field kept
              // the scroll the typing left it at.
              requestAnimationFrame(() => {
                if (input.current) input.current.scrollLeft = 0;
              });
            }}
            onChange={(e) => {
              draft(e.target.value);
              setSwitching(false);
              setHi(-1);
            }}
            onKeyDown={(e) => {
              if (imeKey(e)) return;
              if ((e.key === "ArrowDown" || e.key === "ArrowUp") && n > 0) {
                e.preventDefault();
                if (e.key === "ArrowDown") setHi(lit < 0 ? 0 : (lit + 1) % n);
                else setHi(lit < 0 ? n - 1 : (lit - 1 + n) % n);
              } else if (e.key === "Escape") {
                e.preventDefault();
                if (switching) setSwitching(false);
                else leave();
              } else if (e.key === "Enter" || e.key === "Tab") {
                if (pickingMachine) {
                  const d = machineRows[lit >= 0 ? lit : 0];
                  if (d) {
                    e.preventDefault();
                    pick(d, e.key === "Tab");
                  }
                } else if (e.key === "Enter") {
                  e.preventDefault();
                  if (lit >= 0 && addressRows[lit]) go(addressRows[lit].place);
                  else go(text);
                } else if (addressRows.length > 0) {
                  // Tab completes without opening, as a shell's does.
                  e.preventDefault();
                  draft(addressRows[lit >= 0 ? lit : 0].place);
                  setHi(-1);
                }
              } else if (
                e.key === "Backspace" &&
                token &&
                !still &&
                !switching &&
                e.currentTarget.selectionStart === 0 &&
                e.currentTarget.selectionEnd === 0
              ) {
                // Nothing left of the caret but the face: it comes off
                // whole, and what was typed waits for the next machine.
                e.preventDefault();
                setKept(text);
                setText("");
                setToken(null);
                setHi(-1);
              }
            }}
            // The glass's whole height is the address's to press.
            className={cn("min-w-0 flex-1 self-stretch bg-transparent text-body outline-none placeholder:text-tertiary", !editing && "truncate")}
          />
          {editing && text !== "" && !switching && (
            <button
              type="button"
              data-page-address-clear
              aria-label={gui.search_clear}
              // The caret stays in the field.
              onPointerDown={(e) => e.preventDefault()}
              onMouseDown={(e) => e.preventDefault()}
              onClick={() => {
                draft("");
                setHi(-1);
                input.current?.focus();
              }}
              className="grid size-6 flex-none place-items-center rounded-full bg-fill-strong text-muted-foreground"
            >
              <IconClose className="size-affordance" />
            </button>
          )}
        </label>
        {!floated && listSeat === undefined && list}
      </div>
      {listSeat && list && createPortal(<div data-page-list>{list}</div>, listSeat)}
    </div>
  );
}
