// 外观's two sections of the redesign (2026-10-02): 字号 and 底部导航.
// Both are this face's own settings, kept in its storage
// (`lib/type-size`, `lib/nav-order`), not node config rows — the store
// has no keys for them yet.
import { useRef, useState, type KeyboardEvent, type PointerEvent, type ReactNode } from "react";

import { IconBrowser, IconChevronDown, IconChevronUp, IconClose, IconDevices, IconFiles, IconGrip, IconQuota, IconSessions } from "@/components/icons";
import { Slider } from "@/components/settings/Slider";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { SwitchRow } from "@/components/settings/SwitchRow";
import { gui } from "@/gen/catalog";
import { useNarrow } from "@/hooks/use-narrow";
import { NAV_SHOWN, rememberNavOrder, useNavOrder, type NavKey } from "@/lib/nav-order";
import { FONT_MAX_MB, addFontFile, fontsHere, removeFont, setTermFont, stackOf, useTermFont } from "@/lib/term-font";
import { TERM_MAX, TERM_MIN, UI_STEPS, setTypeSize, useTypeSize } from "@/lib/type-size";
import { cn } from "@/lib/utils";
import { MoreSection } from "@/views/more/MorePage";

const ROW = "flex items-center gap-row px-row py-in";

/** 界面字号: follows the system, or a slider of iOS's seven sizes. */
export function TypeSizeSection() {
  const t = useTypeSize();
  return (
    <MoreSection name="type-size" label={gui.type_size}>
      <SwitchRow
        name="ui-size-system"
        label={gui.ui_size_system}
        on={t.followSystem}
        onToggle={() => setTypeSize({ ...t, followSystem: !t.followSystem })}
      />
      {/* Only while the switch is off (the owner: a control that cannot
          be used now is not drawn, never dimmed). */}
      {!t.followSystem && (
        <div data-setting-row="slider" data-setting-row-name="ui-size" className={ROW}>
          <Slider
            label={gui.ui_size}
            min={0}
            max={UI_STEPS.length - 1}
            value={t.uiStep}
            ticks={UI_STEPS.length}
            onChange={(uiStep) => setTypeSize({ ...t, uiStep })}
            small={<span className="text-caption">A</span>}
            large={<span className="text-title-lg font-body">A</span>}
          />
        </div>
      )}
    </MoreSection>
  );
}

/**
 * **终端** (F7.6): the terminal's font and its size, previewed in one
 * line drawn in the choice. The font row is the settings picker — the
 * name and the two chevrons — opening the one menu: 系统默认, the
 * monospace faces this device has (each name drawn in itself), the
 * fonts added here (a heading only once there is one, each with its
 * 移除), and 添加字体文件…. Per device; terminals only.
 */
export function TerminalSection() {
  const t = useTypeSize();
  const f = useTermFont();
  const [here] = useState(() => fontsHere());
  const [refused, setRefused] = useState<string | null>(null);
  const picker = useRef<HTMLInputElement>(null);
  const name = f.choice || gui.term_font_system;
  const pick = (file: File | undefined) => {
    if (!file) return;
    setRefused(null);
    void addFontFile(file).then((why) => {
      if (why === "too-big") setRefused(gui.term_font_too_big(FONT_MAX_MB));
      else if (why === "bad") setRefused(gui.term_font_bad(file.name));
    });
  };
  const item = (family: string, label: string, after?: ReactNode) => (
    <DropdownMenuItem
      key={family || "system"}
      data-term-font-option={family || "system"}
      data-on={f.choice === family || undefined}
      onSelect={() => setTermFont(family)}
      className={cn(f.choice === family && "glass-sel")}
    >
      <span className="min-w-0 flex-1 truncate" style={{ fontFamily: stackOf(family) }}>
        {label}
      </span>
      {after}
    </DropdownMenuItem>
  );
  return (
    <MoreSection name="terminal" label={gui.view_terminal}>
      <div data-setting-row="popup" data-setting-row-name="term-font" className={ROW}>
        <span className="min-w-0 flex-1 text-body">{gui.term_font}</span>
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <button
              type="button"
              data-term-font
              aria-label={`${gui.term_font}: ${name}`}
              className="-mr-xxs flex h-ctl-md min-w-0 items-center gap-in rounded-sm pr-xxs pl-in text-body hover:bg-fill"
            >
              <span className="min-w-0 truncate">{name}</span>
              <span
                data-setting-trail
                className="flex h-4.5 w-4 flex-none flex-col items-center justify-center rounded-xs bg-fill-strong text-muted-foreground"
              >
                <IconChevronUp centred className="-mb-[3px] size-2.5" />
                <IconChevronDown centred className="size-2.5" />
              </span>
            </button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" data-term-font-menu className="max-h-[60vh] min-w-56 overflow-y-auto">
            {item("", gui.term_font_system)}
            {here.map((n) => item(n, n))}
            {f.added.length > 0 && (
              <>
                <DropdownMenuSeparator />
                <DropdownMenuLabel>{gui.term_font_added}</DropdownMenuLabel>
                {f.added.map((a) =>
                  item(
                    a.family,
                    a.family,
                    <button
                      type="button"
                      data-term-font-remove={a.family}
                      aria-label={`${gui.remove} ${a.family}`}
                      title={gui.remove}
                      onPointerDown={(e) => e.stopPropagation()}
                      onClick={(e) => {
                        e.preventDefault();
                        e.stopPropagation();
                        void removeFont(a.family);
                      }}
                      className="grid size-6 flex-none place-items-center rounded-full text-muted-foreground hover:bg-fill-strong"
                    >
                      <IconClose className="size-3.5" />
                    </button>,
                  ),
                )}
              </>
            )}
            <DropdownMenuSeparator />
            <DropdownMenuItem data-term-font-add onSelect={() => picker.current?.click()}>
              {gui.term_font_add}
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
        <input
          ref={picker}
          type="file"
          hidden
          accept=".ttf,.otf,.woff,.woff2"
          onChange={(e) => {
            pick(e.target.files?.[0]);
            e.target.value = "";
          }}
        />
      </div>
      {refused && (
        <div data-term-font-refused className="px-row text-callout text-destructive">
          {refused}
        </div>
      )}
      <div data-setting-row="value" data-setting-row-name="term-size" className={ROW}>
        <span className="min-w-0 flex-1 text-body">{gui.term_size}</span>
        <span className="text-body text-muted-foreground tabular-nums">{t.term}</span>
      </div>
      <div data-setting-row="slider" data-setting-row-name="term-size-slider" className={ROW}>
        <Slider
          label={gui.term_size}
          min={TERM_MIN}
          max={TERM_MAX}
          value={t.term}
          ticks={(TERM_MAX - TERM_MIN) / 2 + 1}
          onChange={(term) => setTypeSize({ ...t, term })}
          small={<span className="font-term text-caption">A</span>}
          large={<span className="font-term text-title-md font-body">A</span>}
        />
      </div>
      <div
        data-term-size-sample
        className="truncate px-row text-muted-foreground"
        // The live variable, not the `font-term` utility: the theme
        // inlines the token's value there, and the sample must wear the
        // choice the root now carries (`lib/term-font`).
        style={{ fontFamily: "var(--font-term)", fontSize: "var(--text-term)", lineHeight: "var(--text-term--line-height)" }}
      >
        {gui.term_size_sample}
      </div>
    </MoreSection>
  );
}

const NAV_GLYPH: Record<NavKey, () => ReactNode> = {
  devices: () => <IconDevices />,
  sessions: () => <IconSessions />,
  quota: () => <IconQuota />,
  workspaces: () => <IconFiles />,
  browser: () => <IconBrowser />,
};

const navName = (key: NavKey): string =>
  ({
    devices: gui.devices_tab,
    sessions: gui.sessions_tab,
    quota: gui.accounts_tab,
    workspaces: gui.workspaces_tab,
    browser: gui.browser_tab,
  })[key];

/**
 * 底部导航: the five landings in the bar's order, the first three over a
 * 「更多」 rule and the rest under it. A row is dragged by its grip (a
 * pointer of any kind); the grip also takes ↑ and ↓ from the keyboard.
 * Each move is kept at once and the bar follows (`useNavOrder`).
 */
export function NavOrderSection() {
  const narrow = useNarrow();
  const order = useNavOrder();
  const box = useRef<HTMLDivElement>(null);
  const [dragging, setDragging] = useState<NavKey | null>(null);

  const move = (key: NavKey, to: number) => {
    const from = order.indexOf(key);
    const at = Math.max(0, Math.min(order.length - 1, to));
    if (from < 0 || from === at) return;
    const next = order.filter((k) => k !== key);
    next.splice(at, 0, key);
    rememberNavOrder(next);
  };

  /** Where a row dragged to `y` lands: after every other row whose
      middle is above the pointer. */
  const slotAt = (key: NavKey, y: number): number => {
    let slot = 0;
    for (const r of box.current?.querySelectorAll<HTMLElement>("[data-nav-order-row]") ?? []) {
      if (r.dataset.navOrderRow === key) continue;
      const b = r.getBoundingClientRect();
      if (y > b.top + b.height / 2) slot += 1;
    }
    return slot;
  };

  const grip = (key: NavKey) => ({
    onPointerDown: (e: PointerEvent<HTMLButtonElement>) => {
      e.currentTarget.setPointerCapture(e.pointerId);
      setDragging(key);
    },
    onPointerMove: (e: PointerEvent<HTMLButtonElement>) => {
      if (dragging !== key) return;
      move(key, slotAt(key, e.clientY));
    },
    onPointerUp: () => setDragging(null),
    onPointerCancel: () => setDragging(null),
    onKeyDown: (e: KeyboardEvent<HTMLButtonElement>) => {
      if (e.key !== "ArrowUp" && e.key !== "ArrowDown") return;
      e.preventDefault();
      move(key, order.indexOf(key) + (e.key === "ArrowUp" ? -1 : 1));
      // The grip moved with its row; keep the caret on it.
      requestAnimationFrame(() =>
        box.current?.querySelector<HTMLElement>(`[data-nav-order-row="${key}"] [data-nav-grip]`)?.focus(),
      );
    },
  });

  return (
    <MoreSection name="nav-order" label={narrow ? gui.nav_order : gui.nav}>
      {/* The sentence and the 更多 rule are about the narrow bar, which
          seats three and folds the rest; the wide strip seats every
          landing, and there the rows only set its order (the owner). */}
      {narrow && (
        <div data-setting-note className="text-callout text-muted-foreground">
          {gui.nav_order_note}
        </div>
      )}
      <div ref={box} data-nav-order className="flex flex-col">
        {order.map((key, i) => {
          const Glyph = NAV_GLYPH[key];
          return (
            <div key={key} className="flex flex-col">
              {narrow && i === NAV_SHOWN && (
                <div data-nav-order-fold className="flex items-center gap-2.5 px-row pt-in pb-xxs text-caption text-tertiary">
                  <span>{gui.rail_more}</span>
                  <span className="h-px flex-1 bg-border" />
                </div>
              )}
              <div
                data-nav-order-row={key}
                data-dragging={dragging === key || undefined}
                className={cn(
                  // A row of the card (`app.css` `[data-nav-order-row]`):
                  // its inset and its hairline, so the glyph stands on
                  // the column's line and the line ends on its edge.
                  "flex min-h-touch items-center gap-row [&_svg]:size-nav-glyph",
                  dragging === key && "bg-fill",
                )}
              >
                <Glyph />
                <span className="min-w-0 flex-1 text-body">{navName(key)}</span>
                <button
                  type="button"
                  data-nav-grip
                  aria-label={navName(key)}
                  aria-keyshortcuts="ArrowUp ArrowDown"
                  className="grid min-h-touch min-w-touch flex-none cursor-grab touch-none place-items-center text-tertiary [&_svg]:size-4"
                  {...grip(key)}
                >
                  <IconGrip />
                </button>
              </div>
            </div>
          );
        })}
      </div>
    </MoreSection>
  );
}
