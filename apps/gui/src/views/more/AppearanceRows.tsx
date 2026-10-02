// 外观's two sections of the redesign (2026-10-02): 字号 and 底部导航.
// Both are this face's own settings, kept in its storage
// (`lib/type-size`, `lib/nav-order`), not node config rows — the store
// has no keys for them yet.
import { useRef, useState, type KeyboardEvent, type PointerEvent, type ReactNode } from "react";

import { IconBrowser, IconDevices, IconFiles, IconGrip, IconQuota, IconSessions } from "@/components/icons";
import { Slider } from "@/components/settings/Slider";
import { SwitchRow } from "@/components/settings/SwitchRow";
import { gui } from "@/gen/catalog";
import { NAV_SHOWN, rememberNavOrder, useNavOrder, type NavKey } from "@/lib/nav-order";
import { TERM_MAX, TERM_MIN, UI_STEPS, setTypeSize, useTypeSize } from "@/lib/type-size";
import { cn } from "@/lib/utils";
import { MoreSection } from "@/views/more/MorePage";

const ROW = "flex items-center gap-row px-row py-in";

/** 界面字号 (follows the system, or a slider of iOS's seven sizes) and
    终端字号 (a slider of whole points, previewed in the terminal's face). */
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
        className="truncate font-term text-muted-foreground"
        style={{ fontSize: "var(--text-term)", lineHeight: "var(--text-term--line-height)" }}
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
    <MoreSection name="nav-order" label={gui.nav_order}>
      <div data-setting-note className="text-callout text-muted-foreground">
        {gui.nav_order_note}
      </div>
      <div ref={box} data-nav-order className="flex flex-col">
        {order.map((key, i) => {
          const Glyph = NAV_GLYPH[key];
          return (
            <div key={key} className="flex flex-col">
              {i === NAV_SHOWN && (
                <div data-nav-order-fold className="flex items-center gap-2.5 px-row pt-in pb-xxs text-caption text-tertiary">
                  <span>{gui.rail_more}</span>
                  <span className="h-px flex-1 bg-border" />
                </div>
              )}
              <div
                data-nav-order-row={key}
                data-dragging={dragging === key || undefined}
                className={cn(
                  "flex min-h-touch items-center gap-row border-b px-row [&_svg]:size-nav-glyph",
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
